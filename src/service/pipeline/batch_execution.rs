// Copyright 2024 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::Utc;
use config::{
    meta::{
        function::{Transform, VRLResultResolver},
        pipeline::{
            components::{ExternalIngestionTask, NodeData},
            Pipeline,
        },
        self_reporting::error::{ErrorData, ErrorSource, PipelineError},
        stream::{StreamParams, StreamType},
    },
    utils::{
        flatten,
        json::{get_string_value, Value},
    },
};
use futures::future::try_join_all;
use once_cell::sync::Lazy;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use crate::{
    common::infra::config::QUERY_FUNCTIONS,
    service::{
        ingestion::{apply_vrl_fn, compile_vrl_function},
        self_reporting::publish_error,
    },
};

static DYNAMIC_STREAM_NAME_PATTERN: Lazy<regex::Regex> =
    Lazy::new(|| regex::Regex::new(r"\{([^}]+)\}").unwrap());

#[async_trait]
pub trait PipelineExt: Sync + Send + 'static {
    /// Registers the function of all the FunctionNode of this pipeline once for execution.
    /// Returns a map of node_id -> VRLResultResolver for quick lookup
    async fn register_functions(&self) -> Result<HashMap<String, VRLResultResolver>>;
}

#[async_trait]
impl PipelineExt for Pipeline {
    async fn register_functions(&self) -> Result<HashMap<String, VRLResultResolver>> {
        let mut vrl_map = HashMap::new();
        for node in &self.nodes {
            if let NodeData::Function(func_params) = &node.data {
                let transform = get_transforms(&self.org, &func_params.name).await?;
                let vrl_runtime_config = compile_vrl_function(&transform.function, &self.org)?;
                let registry = vrl_runtime_config
                    .config
                    .get_custom::<vector_enrichment::TableRegistry>()
                    .unwrap();
                registry.finish_load();
                vrl_map.insert(
                    node.get_node_id(),
                    VRLResultResolver {
                        program: vrl_runtime_config.program,
                        fields: vrl_runtime_config.fields,
                    },
                );
            }
        }
        Ok(vrl_map)
    }
}

#[derive(Debug, Clone)]
pub struct ExecutablePipeline {
    id: String,
    name: String,
    source_node_id: String,
    sorted_nodes: Vec<String>,
    vrl_map: HashMap<String, VRLResultResolver>,
    node_map: HashMap<String, ExecutableNode>,
}

#[derive(Debug, Clone)]
pub struct ExecutableNode {
    id: String,
    node_data: NodeData,
    children: Vec<String>,
}

#[derive(Debug)]
pub struct ExecutablePipelineBulkInputs {
    records: Vec<Value>,
    doc_ids: Vec<Option<String>>,
    originals: Vec<Option<String>>,
}

#[derive(Debug)]
pub struct ExecutablePipelineTraceInputs {
    records: Vec<Value>,
    timestamps: Vec<i64>,
    services: Vec<String>,
    span_names: Vec<String>,
    span_status_for_spanmetrics: Vec<String>,
    span_kinds: Vec<String>,
    span_durations: Vec<f64>,
}

impl ExecutablePipeline {
    pub async fn new(pipeline: &Pipeline) -> Result<Self> {
        let node_map = pipeline
            .nodes
            .iter()
            .map(|node| {
                (
                    node.get_node_id(),
                    ExecutableNode {
                        id: node.get_node_id(),
                        node_data: node.get_node_data(),
                        children: pipeline
                            .edges
                            .iter()
                            .filter(|edge| edge.source == node.id)
                            .map(|edge| edge.target.clone())
                            .collect(),
                    },
                )
            })
            .collect();

        let vrl_map = match pipeline.register_functions().await {
            Ok(vrl_map) => vrl_map,
            Err(e) => {
                let pipeline_error = PipelineError {
                    pipeline_id: pipeline.id.to_string(),
                    pipeline_name: pipeline.name.to_string(),
                    error: Some(format!("Init error: failed to compile function: {e}")),
                    node_errors: HashMap::new(),
                };
                publish_error(ErrorData {
                    _timestamp: Utc::now().timestamp_micros(),
                    stream_params: pipeline.get_source_stream_params(),
                    error_source: ErrorSource::Pipeline(pipeline_error),
                })
                .await;
                return Err(e);
            }
        };
        let sorted_nodes = match topological_sort(&node_map) {
            Ok(sorted) => sorted,
            Err(e) => {
                let pipeline_error = PipelineError {
                    pipeline_id: pipeline.id.to_string(),
                    pipeline_name: pipeline.name.to_string(),
                    error: Some(
                        "Init error: failed to sort pipeline nodes for execution".to_string(),
                    ),
                    node_errors: HashMap::new(),
                };
                publish_error(ErrorData {
                    _timestamp: Utc::now().timestamp_micros(),
                    stream_params: pipeline.get_source_stream_params(),
                    error_source: ErrorSource::Pipeline(pipeline_error),
                })
                .await;
                return Err(e);
            }
        };
        let source_node_id = sorted_nodes[0].to_owned();

        Ok(Self {
            id: pipeline.id.to_string(),
            name: pipeline.name.to_string(),
            source_node_id,
            node_map,
            sorted_nodes,
            vrl_map,
        })
    }

    pub async fn process_batch(
        &self,
        org_id: &str,
        records: Vec<Value>,
    ) -> Result<HashMap<StreamParams, Vec<(usize, Value)>>> {
        let batch_size = records.len();
        log::debug!("[Pipeline]: process batch of size {}", batch_size);

        // result_channel
        let (result_sender, mut result_receiver) =
            channel::<(usize, StreamParams, Value)>(batch_size);

        // error_channel
        let (error_sender, mut error_receiver) = channel::<(String, String, String)>(batch_size);

        let mut node_senders = HashMap::new();
        let mut node_receivers = HashMap::new();

        for node_id in &self.sorted_nodes {
            let (sender, receiver) = channel::<(usize, Value, bool)>(batch_size);
            node_senders.insert(node_id.to_string(), sender);
            node_receivers.insert(node_id.to_string(), receiver);
        }

        // Spawn tasks for each node
        let mut node_tasks = Vec::new();
        for (idx, node_id) in self.sorted_nodes.iter().enumerate() {
            let pl_id_cp = self.id.to_string();
            let org_id_cp = org_id.to_string();
            let node = self.node_map.get(node_id).unwrap().clone();
            let node_receiver = node_receivers.remove(node_id).unwrap();
            let child_senders: Vec<_> = node
                .children
                .iter()
                .map(|child| node_senders.get(child).unwrap().clone())
                .collect();
            let result_sender_cp = node.children.is_empty().then_some(result_sender.clone());
            let error_sender_cp = error_sender.clone();
            let vrl_runtime = self.vrl_map.get(node_id).cloned();

            let task = tokio::spawn(async move {
                process_node(
                    pl_id_cp,
                    idx,
                    org_id_cp,
                    node,
                    node_receiver,
                    child_senders,
                    vrl_runtime,
                    result_sender_cp,
                    error_sender_cp,
                )
                .await
            });
            node_tasks.push(task);
        }

        // task to collect results
        let result_task = tokio::spawn(async move {
            log::debug!("[Pipeline]: starts result collecting job");
            let mut count: usize = 0;
            let mut results = HashMap::new();
            while let Some((idx, stream_params, record)) = result_receiver.recv().await {
                results
                    .entry(stream_params)
                    .or_insert(Vec::new())
                    .push((idx, record));
                count += 1;
            }
            log::debug!("[Pipeline]: collected {count} records");
            results
        });

        // task to collect errors
        let mut pipeline_error = PipelineError::new(&self.id, &self.name);
        let error_task = tokio::spawn(async move {
            log::debug!("[Pipeline]: starts error collecting job");
            let mut count = 0;
            while let Some((node_id, node_type, error)) = error_receiver.recv().await {
                pipeline_error.add_node_error(node_id, node_type, error);
                count += 1;
            }
            log::debug!("[Pipeline]: collected {count} errors");
            if count > 0 {
                Some(pipeline_error)
            } else {
                None
            }
        });

        // Send records to the source node to begin processing
        let flattened = {
            let source_node = self.node_map.get(&self.source_node_id).unwrap();
            matches!(&source_node.node_data, NodeData::Stream(stream_params) if stream_params.stream_type == StreamType::Metrics)
        };
        let source_sender = node_senders.remove(&self.source_node_id).unwrap();
        for (idx, record) in records.into_iter().enumerate() {
            if let Err(send_err) = source_sender.send((idx, record, flattened)).await {
                log::error!(
                    "[Pipeline]: Error sending original records into source Node for {send_err}"
                );
                break;
            }
        }
        drop(source_sender);
        drop(result_sender);
        drop(error_sender);
        drop(node_senders);
        log::debug!("[Pipeline]: All records send into pipeline for processing");

        // Wait for all node tasks to complete
        if let Err(e) = try_join_all(node_tasks).await {
            log::error!("[Pipeline] node processing jobs failed: {}", e);
        }

        // Publish errors if received any
        if let Some(pipeline_errors) = error_task.await.map_err(|e| {
            log::error!("[Pipeline] error collecting job failed: {}", e);
            anyhow!("[Pipeline] error collecting job failed: {}", e)
        })? {
            let stream_params = self.get_source_stream_params();
            let error_data = ErrorData {
                _timestamp: Utc::now().timestamp_micros(),
                stream_params,
                error_source: ErrorSource::Pipeline(pipeline_errors),
            };
            log::debug!("[Pipeline]: execution errors occurred and published");
            publish_error(error_data).await;
        }

        let results = result_task.await.map_err(|e| {
            log::error!("[Pipeline] result collecting job failed: {}", e);
            anyhow!("[Pipeline] result collecting job failed: {}", e)
        })?;

        Ok(results)
    }

    pub fn get_all_destination_streams(&self) -> Vec<StreamParams> {
        self.node_map
            .values()
            .filter_map(|exec_node| {
                if exec_node.children.is_empty() {
                    if let NodeData::Stream(stream_params) = &exec_node.node_data {
                        Some(stream_params.clone())
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn num_of_func(&self) -> usize {
        self.node_map
            .values()
            .filter(|exec_node| matches!(exec_node.node_data, NodeData::Function(_)))
            .count()
    }

    pub fn get_pipeline_id(&self) -> &str {
        &self.id
    }

    fn get_source_stream_params(&self) -> StreamParams {
        // source_node_id must exist in node_map
        match &self.node_map.get(&self.source_node_id).unwrap().node_data {
            NodeData::Stream(stream_params) => stream_params.clone(),
            NodeData::Query(derived_stream) => StreamParams::new(
                &derived_stream.org_id,
                "DerivedStream",
                derived_stream.stream_type,
            ),
            _ => unreachable!(), // SourceNode can only be of type Stream of Query
        }
    }
}

impl ExecutableNode {
    pub fn node_type(&self) -> String {
        self.to_string()
    }
}

impl std::fmt::Display for ExecutableNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.node_data {
            NodeData::Stream(_) => write!(f, "stream"),
            NodeData::Query(_) => write!(f, "query"),
            NodeData::Function(_) => write!(f, "function"),
            NodeData::Condition(_) => write!(f, "condition"),
            NodeData::Http(_) => write!(f, "http"),
        }
    }
}

impl ExecutablePipelineBulkInputs {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            doc_ids: Vec::new(),
            originals: Vec::new(),
        }
    }

    pub fn add_input(
        &mut self,
        record: Value,
        doc_id: Option<String>,
        original_data: Option<String>,
    ) {
        self.records.push(record);
        self.doc_ids.push(doc_id);
        self.originals.push(original_data);
    }

    pub fn into_parts(self) -> (Vec<Value>, Vec<Option<String>>, Vec<Option<String>>) {
        (self.records, self.doc_ids, self.originals)
    }
}

impl Default for ExecutablePipelineBulkInputs {
    fn default() -> Self {
        Self::new()
    }
}

impl ExecutablePipelineTraceInputs {
    pub fn new() -> Self {
        Self {
            records: Vec::new(),
            timestamps: Vec::new(),
            services: Vec::new(),
            span_names: Vec::new(),
            span_status_for_spanmetrics: Vec::new(),
            span_kinds: Vec::new(),
            span_durations: Vec::new(),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn add_input(
        &mut self,
        record: Value,
        ts: i64,
        service: String,
        span_name: String,
        span_status_for_spanmetric: String,
        span_kind: String,
        duration: f64,
    ) {
        self.records.push(record);
        self.timestamps.push(ts);
        self.services.push(service);
        self.span_names.push(span_name);
        self.span_status_for_spanmetrics
            .push(span_status_for_spanmetric);
        self.span_kinds.push(span_kind);
        self.span_durations.push(duration);
    }

    #[allow(clippy::type_complexity)]
    pub fn into_parts(
        self,
    ) -> (
        Vec<Value>,
        Vec<i64>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<String>,
        Vec<f64>,
    ) {
        (
            self.records,
            self.timestamps,
            self.services,
            self.span_names,
            self.span_status_for_spanmetrics,
            self.span_kinds,
            self.span_durations,
        )
    }
}

impl Default for ExecutablePipelineTraceInputs {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(clippy::too_many_arguments)]
async fn process_node(
    pipeline_id: String,
    node_id: usize,
    org_id: String,
    node: ExecutableNode,
    mut receiver: Receiver<(usize, Value, bool)>,
    mut child_senders: Vec<Sender<(usize, Value, bool)>>,
    vrl_runtime: Option<VRLResultResolver>,
    result_sender: Option<Sender<(usize, StreamParams, Value)>>,
    error_sender: Sender<(String, String, String)>,
) -> Result<()> {
    let cfg = config::get_config();
    let mut count: usize = 0;
    match &node.node_data {
        NodeData::Stream(stream_params) => {
            if node.children.is_empty() {
                log::debug!("[Pipeline]: Leaf node {node_id} starts processing");
                // leaf node: `result_sender` guaranteed to be Some()
                // send received results directly via `result_sender` for collection
                let result_sender = result_sender.unwrap();
                while let Some((idx, mut record, flattened)) = receiver.recv().await {
                    if !flattened {
                        record = match flatten::flatten_with_level(
                            record,
                            cfg.limit.ingest_flatten_level,
                        ) {
                            Ok(flattened) => flattened,
                            Err(e) => {
                                let err_msg = format!("LeafNode error with flattening: {}", e);
                                if let Err(send_err) = error_sender
                                    .send((node.id.to_string(), node.node_type(), err_msg))
                                    .await
                                {
                                    log::error!(
                                        "[Pipeline]: LeafNode failed sending errors for collection caused by: {send_err}"
                                    );
                                    break;
                                }
                                continue;
                            }
                        };
                    }

                    let mut destination_stream = stream_params.clone();
                    if destination_stream.stream_name.contains("{") {
                        match resolve_stream_name(&destination_stream.stream_name, &record) {
                            Ok(stream_name) if !stream_name.is_empty() => {
                                destination_stream.stream_name = stream_name.into();
                            }
                            resolve_res => {
                                let err_msg = if let Err(e) = resolve_res {
                                    format!("Dynamic stream name detected in destination, but failed to resolve due to {e}. Record dropped")
                                } else {
                                    "Dynamic Stream Name resolved to empty. Record dropped"
                                        .to_string()
                                };
                                log::warn!("{err_msg}");
                                if let Err(send_err) = error_sender
                                    .send((node.id.to_string(), node.node_type(), err_msg))
                                    .await
                                {
                                    log::error!(
                                        "[Pipeline]: LeafNode failed sending errors for collection caused by: {send_err}"
                                    );
                                    break;
                                }
                                continue;
                            }
                        }
                    }

                    if let Err(send_err) =
                        result_sender.send((idx, destination_stream, record)).await
                    {
                        log::error!(
                            "[Pipeline]: LeafNode errors sending result for collection caused by: {send_err}"
                        );
                        break;
                    }
                    count += 1;
                }
                log::debug!("[Pipeline]: LeafNode {node_id} done processing {count} records");
            } else {
                log::debug!("[Pipeline]: source node {node_id} starts processing");
                // source stream node: send received record to all its children
                while let Some(item) = receiver.recv().await {
                    send_to_children(&mut child_senders, item, "StreamNode").await;
                    count += 1;
                }
                log::debug!("[Pipeline]: source node {node_id} done processing {count} records");
            }
        }
        NodeData::Condition(condition_params) => {
            log::debug!("[Pipeline]: cond node {node_id} starts processing");
            while let Some((idx, mut record, mut flattened)) = receiver.recv().await {
                // value must be flattened before condition params can take effect
                if !flattened {
                    record = match flatten::flatten_with_level(
                        record,
                        cfg.limit.ingest_flatten_level,
                    ) {
                        Ok(flattened) => flattened,
                        Err(e) => {
                            let err_msg = format!("ConditionNode error with flattening: {}", e);
                            if let Err(send_err) = error_sender
                                .send((node.id.to_string(), node.node_type(), err_msg))
                                .await
                            {
                                log::error!(
                                    "[Pipeline]: ConditionNode failed sending errors for collection caused by: {send_err}"
                                );
                                break;
                            }
                            continue;
                        }
                    };
                    flattened = true;
                }
                // only send to children when passing all condition evaluations
                if condition_params
                    .conditions
                    .iter()
                    .all(|cond| cond.evaluate(record.as_object().unwrap()))
                {
                    send_to_children(
                        &mut child_senders,
                        (idx, record, flattened),
                        "ConditionNode",
                    )
                    .await;
                    count += 1;
                }
            }
            log::debug!("[Pipeline]: cond node {node_id} done processing {count} records");
        }
        NodeData::Function(func_params) => {
            log::debug!("[Pipeline]: func node {node_id} starts processing");
            let mut runtime = crate::service::ingestion::init_functions_runtime();
            while let Some((idx, mut record, mut flattened)) = receiver.recv().await {
                if let Some(vrl_runtime) = &vrl_runtime {
                    if func_params.after_flatten && !flattened {
                        record = match flatten::flatten_with_level(
                            record,
                            cfg.limit.ingest_flatten_level,
                        ) {
                            Ok(flattened) => flattened,
                            Err(e) => {
                                let err_msg = format!("FunctionNode error with flattening: {}", e);
                                if let Err(send_err) = error_sender
                                    .send((node.id.to_string(), node.node_type(), err_msg))
                                    .await
                                {
                                    log::error!(
                                        "[Pipeline]: FunctionNode failed sending errors for collection caused by: {send_err}"
                                    );
                                    break;
                                }
                                continue;
                            }
                        };
                    }
                    record = match apply_vrl_fn(
                        &mut runtime,
                        vrl_runtime,
                        record,
                        &org_id,
                        &["pipeline".to_string()],
                    ) {
                        (res, None) => res,
                        (res, Some(error)) => {
                            let err_msg = format!("FunctionNode error: {}", error);
                            if let Err(send_err) = error_sender
                                .send((node.id.to_string(), node.node_type(), err_msg))
                                .await
                            {
                                log::error!(
                                    "[Pipeline]: FunctionNode failed sending errors for collection caused by: {send_err}"
                                );
                                break;
                            }
                            res
                        }
                    };
                    flattened = false; // since apply_vrl_fn can produce unflattened data
                }
                send_to_children(&mut child_senders, (idx, record, flattened), "FunctionNode")
                    .await;
                count += 1;
            }
            log::debug!("[Pipeline]: func node {node_id} done processing {count} records");
        }
        NodeData::Query(_) => {
            // source node for Scheduled pipeline. Directly send to children nodes
            log::debug!("[Pipeline]: query node {node_id} starts processing");
            while let Some(item) = receiver.recv().await {
                send_to_children(&mut child_senders, item, "QueryNode").await;
                count += 1;
            }
            log::debug!("[Pipeline]: query node {node_id} done processing {count} records");
        }
        NodeData::Http(dests) => {
            let mut records = vec![];
            log::debug!("[Pipeline]: Destination node {node_id} starts processing");
            while let Some((_, record, _)) = receiver.recv().await {
                // External destinations will automatically flatten the payload, hence
                // no need to flatten the records here
                records.push(record);
                count += 1;
            }

            let external_task = ExternalIngestionTask::new(pipeline_id, dests.clone(), records);
            if let Err(e) = external_task.persist_data().await {
                let err_msg = format!(
                    "DestinationNode error persisting data to be ingested externally: {}",
                    e
                );
                if let Err(send_err) = error_sender
                    .send((node.id.to_string(), node.node_type(), err_msg))
                    .await
                {
                    log::error!(
                        "[Pipeline]: DestinationNode failed sending errors for collection caused by: {send_err}"
                    );
                }
            }

            log::debug!("[Pipeline]: DestinationNode {node_id} done processing {count} records");
        }
    }

    // all cloned senders dropped when function goes out of scope -> close the channel

    Ok(())
}

async fn send_to_children(
    child_senders: &mut [Sender<(usize, Value, bool)>],
    item: (usize, Value, bool),
    node_type: &str,
) {
    if child_senders.len() == 1 {
        // HACK to avoid cloning
        if let Err(send_err) = child_senders[0].send(item).await {
            log::error!(
                "[Pipeline]: {node_type} errors sending record to its children caused by: {send_err}"
            );
        }
    } else {
        for child_sender in child_senders.iter_mut() {
            if let Err(send_err) = child_sender.send(item.clone()).await {
                log::error!(
                    "[Pipeline]: {node_type} errors sending record to its children caused by: {send_err}"
                );
                break;
            }
        }
    }
}

fn topological_sort(node_map: &HashMap<String, ExecutableNode>) -> Result<Vec<String>> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut temp = HashSet::new();

    let graph: HashMap<String, Vec<String>> = node_map
        .iter()
        .map(|(k, v)| (k.clone(), v.children.clone()))
        .collect();

    for node in node_map.keys() {
        if let Err(e) = dfs(node, &graph, &mut visited, &mut temp, &mut result) {
            return Err(anyhow!(
                "[Pipeline] Error to topological sort the nodes in preparing execution: {}",
                e
            ));
        }
    }

    result.reverse();
    Ok(result)
}

fn dfs(
    current_node_id: &str,
    graph: &HashMap<String, Vec<String>>,
    visited: &mut HashSet<String>,
    temp: &mut HashSet<String>,
    result: &mut Vec<String>,
) -> Result<()> {
    if temp.contains(current_node_id) {
        return Err(anyhow!("Cyclical pipeline detected."));
    }
    if visited.contains(current_node_id) {
        return Ok(());
    }
    temp.insert(current_node_id.to_string());
    if let Some(neighbors) = graph.get(current_node_id) {
        for neighbor in neighbors {
            dfs(neighbor, graph, visited, temp, result)?;
        }
    }
    temp.remove(current_node_id);
    visited.insert(current_node_id.to_string());
    result.push(current_node_id.to_string());
    Ok(())
}

async fn get_transforms(org_id: &str, fn_name: &str) -> Result<Transform> {
    let func_key = format!("{org_id}/{fn_name}");
    if let Some(trans) = QUERY_FUNCTIONS.get(&func_key) {
        return Ok(trans.value().clone());
    }
    // get from database
    crate::service::db::functions::get(org_id, fn_name).await
}

fn resolve_stream_name(haystack: &str, record: &Value) -> Result<String> {
    // Fast path: if it's a complete pattern like "{field}", avoid regex
    if haystack.starts_with("{") && haystack.ends_with("}") {
        let field_name = &haystack[1..haystack.len() - 1];
        return match record.get(field_name) {
            Some(stream_name) => Ok(get_string_value(stream_name)),
            None => Err(anyhow!("Field name {field_name} not found in record")),
        };
    }

    // Slow path: handle partial matches using regex
    let mut result = String::with_capacity(haystack.len());
    let mut last_match = 0;
    for cap in DYNAMIC_STREAM_NAME_PATTERN.captures_iter(haystack) {
        let field_name = cap.get(1).unwrap().as_str();
        let full_match = cap.get(0).unwrap();

        // Add the text between the last match and this one
        result.push_str(&haystack[last_match..full_match.start()]);

        // Get and validate the field value
        match record.get(field_name) {
            Some(value) => result.push_str(&get_string_value(value)),
            None => return Err(anyhow!("Field name {field_name} not found in record")),
        }

        last_match = full_match.end();
    }
    result.push_str(&haystack[last_match..]);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use config::utils::json;

    use super::resolve_stream_name;

    #[test]
    fn test_my_regex() {
        let record = json::json!({
            "app_name": "o2",
            "container_name": "compactor",
            "value": 123
        });
        let ok_cases = vec![
            ("container_name", "container_name"),
            ("{container_name}", "compactor"),
            ("abc-{container_name}", "abc-compactor"),
            ("abc-{container_name}-xyz", "abc-compactor-xyz"),
            ("abc-{value}-xyz", "abc-123-xyz"),
            ("{value}", "123"),
        ];
        for (test, expected) in ok_cases {
            let result = resolve_stream_name(test, &record);
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), expected);
        }
        let err1 = resolve_stream_name("{{value}}", &record);
        assert!(err1.is_err());
        let err1 = resolve_stream_name("{{eulav}}", &record);
        assert!(err1.is_err());
    }
}
