// Copyright 2024 Zinc Labs Inc.
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
use config::{
    meta::{
        function::VRLResultResolver,
        pipeline::{components::NodeData, Pipeline},
        stream::StreamParams,
    },
    utils::{flatten, json::Value},
};
use futures::future::try_join_all;
use tokio::sync::mpsc::{channel, Receiver, Sender};

use super::execution::PipelineExt;
use crate::service::ingestion::apply_vrl_fn;

const PIPELINE_NODE_BUFFER: usize = 100;

#[derive(Debug)]
pub struct ExecutablePipeline {
    source_node_id: String,
    sorted_nodes: Vec<String>,
    vrl_map: HashMap<String, VRLResultResolver>,
    node_map: HashMap<String, ExecutableNode>,
}

#[derive(Debug, Clone)]
pub struct ExecutableNode {
    node_data: NodeData,
    children: Vec<String>,
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

        let vrl_map = pipeline.register_functions().await?;
        let sorted_nodes = topological_sort(&node_map)?;
        let source_node_id = sorted_nodes[0].to_owned();

        Ok(Self {
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
    ) -> Result<HashMap<StreamParams, Value>> {
        let (result_sender, mut result_receiver) =
            channel::<(StreamParams, Value)>(PIPELINE_NODE_BUFFER);

        let mut node_senders = HashMap::new();
        let mut node_receivers = HashMap::new();

        for node_id in &self.sorted_nodes {
            let (sender, receiver) = channel::<(Value, bool)>(PIPELINE_NODE_BUFFER);
            node_senders.insert(node_id.to_string(), sender);
            node_receivers.insert(node_id.to_string(), receiver);
        }

        // Spawn tasks for each node
        let mut node_tasks = Vec::new();
        for node_id in &self.sorted_nodes {
            let org_id_cp = org_id.to_string();
            let node = self.node_map.get(node_id).unwrap().clone();
            let node_receiver = node_receivers.remove(node_id).unwrap();
            let child_senders: Vec<_> = node
                .children
                .iter()
                .map(|child| node_senders.get(child).unwrap().clone())
                .collect();
            let result_sender_cp = (!node.children.is_empty()).then_some(result_sender.clone());
            let vrl_runtime = self.vrl_map.get(node_id).cloned();

            let task = tokio::spawn(async move {
                process_node(
                    org_id_cp,
                    node,
                    node_receiver,
                    child_senders,
                    vrl_runtime,
                    result_sender_cp,
                )
                .await
            });
            node_tasks.push(task);
        }

        // Send records to the source node to begin processing
        let source_sender = node_senders.remove(&self.source_node_id).unwrap();
        for record in records {
            if let Err(send_err) = source_sender.send((record, false)).await {
                log::error!(
                    "[Pipeline]: Error sending original records into source Node for {send_err}"
                );
                break;
            }
        }
        drop(source_sender); // Close the source channel

        // Collect results
        let mut results = HashMap::new();
        while let Some((stream_params, record)) = result_receiver.recv().await {
            results.insert(stream_params, record);
        }

        // Wait for all node tasks to complete
        if let Err(e) = try_join_all(node_tasks).await {
            log::error!("[Pipeline] node processing jobs failed: {}", e);
        }

        Ok(results)
    }
}

async fn process_node(
    org_id: String,
    node: ExecutableNode,
    mut receiver: Receiver<(Value, bool)>,
    mut child_senders: Vec<Sender<(Value, bool)>>,
    vrl_runtime: Option<VRLResultResolver>,
    result_sender: Option<Sender<(StreamParams, Value)>>,
) -> Result<()> {
    let cfg = config::get_config();
    match &node.node_data {
        NodeData::Stream(stream_params) => {
            if node.children.is_empty() {
                // leaf node: `result_sender` guaranteed to be Some()
                // send received results directly via `result_sender` for collection
                let result_sender = result_sender.unwrap();
                while let Some((mut record, flattened)) = receiver.recv().await {
                    if !flattened {
                        record =
                            flatten::flatten_with_level(record, cfg.limit.ingest_flatten_level)
                                .map_err(|e| anyhow!("LeafNode error with flattening: {}", e))?;
                    }
                    if let Err(send_err) = result_sender.send((stream_params.clone(), record)).await
                    {
                        log::error!(
                            "[Pipeline]: Leaf node errors sending result for collection caused by: {send_err}"
                        );
                        break;
                    }
                }
                drop(result_sender);
            } else {
                // source stream node: send received record to all its children
                while let Some(item) = receiver.recv().await {
                    send_to_children(&mut child_senders, item, "StreamNode").await;
                }
            }
        }
        NodeData::Condition(condition_params) => {
            while let Some((mut record, mut flattened)) = receiver.recv().await {
                // value must be flattened before condition params can take effect
                if !flattened {
                    record = flatten::flatten_with_level(record, cfg.limit.ingest_flatten_level)
                        .map_err(|e| anyhow!("ConditionNode error with flattening: {}", e))?;
                    flattened = true;
                }
                // only send to children when passing all condition evaluations
                if condition_params
                    .conditions
                    .iter()
                    .all(|cond| cond.evaluate(record.as_object().unwrap()))
                {
                    send_to_children(&mut child_senders, (record, flattened), "QueryNode").await;
                }
            }
        }
        NodeData::Function(func_params) => {
            let mut runtime = crate::service::ingestion::init_functions_runtime();
            let vrl_runtime = vrl_runtime.unwrap();
            while let Some((mut record, mut flattened)) = receiver.recv().await {
                if func_params.after_flatten && !flattened {
                    record = flatten::flatten_with_level(record, cfg.limit.ingest_flatten_level)
                        .map_err(|e| anyhow!("FunctionNode error with flattening: {}", e))?;
                    flattened = true;
                }
                record = apply_vrl_fn(
                    &mut runtime,
                    &vrl_runtime,
                    &record,
                    &org_id,
                    &["pipeline".to_string()],
                );
                send_to_children(&mut child_senders, (record, flattened), "FunctionNode").await;
            }
        }
        NodeData::Query(_) => {
            // source node for Scheduled pipeline. Directly send to children nodes
            while let Some(item) = receiver.recv().await {
                send_to_children(&mut child_senders, item, "QueryNode").await;
            }
        }
    }
    drop(child_senders);

    Ok(())
}

async fn send_to_children(
    child_senders: &mut [Sender<(Value, bool)>],
    item: (Value, bool),
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
