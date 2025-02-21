// Copyright 2025 OpenObserve Inc.
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

use std::sync::Arc;

use config::{
    meta::{
        cluster::{IntoArcVec, RoleGroup},
        search::{ScanStats, SearchEventType},
        sql::TableReferenceExt,
        stream::{FileKey, StreamType},
    },
    utils::time::BASE_TIME,
};
use datafusion::{
    common::{tree_node::TreeNode, TableReference},
    physical_plan::ExecutionPlan,
    prelude::SessionContext,
};
use datafusion_proto::bytes::physical_plan_from_bytes_with_extension_codec;
use infra::{
    errors::{Error, Result},
    file_list::FileId,
    schema::get_stream_setting_index_fields,
};
use proto::cluster_rpc::{KvItem, SearchQuery};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use crate::service::search::{
    cluster::flight::{
        check_work_group, get_inverted_index_file_list, get_online_querier_nodes,
        partition_filt_list,
    },
    datafusion::{
        distributed_plan::{
            codec::{ComposedPhysicalExtensionCodec, EmptyExecPhysicalExtensionCodec},
            empty_exec::NewEmptyExec,
            node::{RemoteScanNode, SearchInfos},
            remote_scan::RemoteScanExec,
            NewEmptyExecVisitor,
        },
        exec::{prepare_datafusion_context, register_udf},
    },
    generate_filter_from_equal_items,
    request::{FlightSearchRequest, Request},
    utils::AsyncDefer,
};

/// in cluster search function only single stream take part in
// 1. get nodes
// 2. get file list
// 3. partition file list
// 4. get physical plan
// 5. add remote scan to physical plan
// 6. execute physical plan to get stream
#[tracing::instrument(name = "service:search:grpc:flight:follower:search", skip_all)]
pub async fn search(
    trace_id: &str,
    flight_request: &FlightSearchRequest,
) -> Result<(
    SessionContext,
    Arc<dyn ExecutionPlan>,
    AsyncDefer,
    ScanStats,
)> {
    let start = std::time::Instant::now();
    let cfg = config::get_config();
    let mut req: Request = (*flight_request).clone().into();
    let trace_id = trace_id.to_string();

    // create datafusion context, just used for decode plan, the params can use default
    let mut ctx =
        prepare_datafusion_context(req.work_group.clone(), vec![], false, cfg.limit.cpu_num)
            .await?;

    // register udf
    register_udf(&ctx, &req.org_id)?;
    datafusion_functions_json::register_all(&mut ctx)?;

    // Decode physical plan from bytes
    let proto = ComposedPhysicalExtensionCodec {
        codecs: vec![Arc::new(EmptyExecPhysicalExtensionCodec {})],
    };
    let mut physical_plan = physical_plan_from_bytes_with_extension_codec(
        &flight_request.search_info.plan,
        &ctx,
        &proto,
    )?;

    // replace empty table to real table
    let mut visitor = NewEmptyExecVisitor::default();
    if physical_plan.visit(&mut visitor).is_err() || visitor.get_data().is_none() {
        return Err(Error::Message(
            "flight->follower_leader: physical plan visit error: there is no EmptyTable"
                .to_string(),
        ));
    }
    let empty_exec = visitor
        .get_data()
        .unwrap()
        .as_any()
        .downcast_ref::<NewEmptyExec>()
        .unwrap();

    // get stream name
    let stream = TableReference::from(empty_exec.name());
    let stream_name = stream.stream_name();
    let stream_type = stream.get_stream_type(req.stream_type);

    // 1. get file id list
    let file_id_list = get_file_id_lists(&req.org_id, stream_type, &stream, req.time_range).await?;

    let file_id_list_vec = file_id_list.iter().collect::<Vec<_>>();
    let file_id_list_took = start.elapsed().as_millis() as usize;
    log::info!(
        "[trace_id {trace_id}] flight->follower_leader: get file_list time_range: {:?}, num: {}, took: {} ms",
        req.time_range,
        file_id_list_vec.len(),
        file_id_list_took,
    );
    let mut scan_stats = ScanStats {
        files: file_id_list_vec.len() as i64,
        original_size: file_id_list_vec.iter().map(|v| v.original_size).sum(),
        ..Default::default()
    };

    // 2. get inverted index file list
    let (use_ttv_inverted_index, idx_file_list, idx_scan_size, _idx_took) =
        get_inverted_index_file_lists(
            &trace_id,
            &req,
            &stream_name, // for inverted index search, only have on stream
            &flight_request.index_info.equal_keys,
            &flight_request.index_info.match_all_keys,
        )
        .await?;
    scan_stats.idx_scan_size = idx_scan_size as i64;
    req.set_use_inverted_index(use_ttv_inverted_index);

    // get nodes
    let node_group = req
        .search_event_type
        .as_ref()
        .map(|v| {
            SearchEventType::try_from(v.as_str())
                .ok()
                .map(RoleGroup::from)
        })
        .unwrap_or(None);
    let nodes = get_online_querier_nodes(&trace_id, node_group).await?;
    let querier_num = nodes.iter().filter(|node| node.is_querier()).count();
    if querier_num == 0 {
        log::error!("no querier node online");
        return Err(Error::Message("no querier node online".to_string()));
    }

    // check work group
    let (_took_wait, work_group_str, work_group) = check_work_group(
        &req,
        &trace_id,
        &nodes,
        &file_id_list_vec,
        start,
        file_id_list_took,
    )
    .await?;
    // add work_group
    req.add_work_group(Some(work_group_str));

    // release work_group in flight follow search
    let user_id = req.user_id.clone();
    let trace_id_move = trace_id.to_string();
    let defer = AsyncDefer::new({
        async move {
            let _ = work_group
                .as_ref()
                .unwrap()
                .done(&trace_id_move, user_id.as_deref())
                .await
                .map_err(|e| {
                    log::error!(
                        "[trace_id {trace_id_move}] release work_group in flight follow search error: {e}",
                    );
                    e.to_string();
                });
            log::info!("[trace_id {trace_id_move}] release work_group in flight follow search");
        }
    });

    // partition file list
    let partition_file_lists = partition_filt_list(file_id_list, &nodes, node_group).await?;

    // update search session scan stats
    super::super::SEARCH_SERVER
        .add_file_stats(
            &trace_id,
            scan_stats.files,
            scan_stats.records,
            scan_stats.original_size + scan_stats.idx_scan_size,
            scan_stats.compressed_size,
        )
        .await;

    let search_infos = SearchInfos {
        plan: vec![],
        file_id_list: partition_file_lists.clone(),
        idx_file_list,
        start_time: req.time_range.as_ref().map(|x| x.0).unwrap_or(0),
        end_time: req.time_range.as_ref().map(|x| x.1).unwrap_or(0),
        timeout: req.timeout as u64,
    };

    let context = tracing::Span::current().context();
    let mut remote_scan_node = RemoteScanNode::from_flight_search_request(
        flight_request,
        search_infos,
        nodes.into_arc_vec(),
        context,
    );
    remote_scan_node.set_is_super_cluster(false);

    // add sort preserving merge node to preserving the order
    if physical_plan.name() == "SortPreservingMergeExec" {
        let top_merge_node = physical_plan.clone();
        let remote_scan_exec = Arc::new(RemoteScanExec::new(physical_plan, remote_scan_node)?);
        physical_plan = top_merge_node.with_new_children(vec![remote_scan_exec])?;
    } else {
        physical_plan = Arc::new(RemoteScanExec::new(physical_plan, remote_scan_node)?);
    }

    log::info!("[trace_id {trace_id}] flight->follower_leader: generate physical plan finish",);

    Ok((ctx, physical_plan, defer, scan_stats))
}

#[tracing::instrument(
    name = "service:search:super_cluster:follower:get_file_id_lists",
    skip_all
)]
pub async fn get_file_id_lists(
    org_id: &str,
    stream_type: StreamType,
    stream: &TableReference,
    mut time_range: Option<(i64, i64)>,
) -> Result<Vec<FileId>> {
    let stream_name = stream.stream_name();
    let stream_type = stream.get_stream_type(stream_type);
    // if stream is enrich, rewrite the time_range
    if let Some(schema) = stream.schema() {
        if schema == "enrich" || schema == "enrichment_tables" {
            let start = BASE_TIME.timestamp_micros();
            let end = chrono::Utc::now().timestamp_micros();
            time_range = Some((start, end));
        }
    }
    let file_id_list =
        crate::service::file_list::query_ids(org_id, stream_type, &stream_name, time_range).await?;
    Ok(file_id_list)
}

#[tracing::instrument(
    name = "service:search:super_cluster:follower:get_inverted_index_file_lists",
    skip_all
)]
async fn get_inverted_index_file_lists(
    trace_id: &str,
    req: &Request,
    stream_name: &str,
    equal_terms: &[KvItem],
    match_terms: &[String],
) -> Result<(bool, Vec<FileKey>, usize, usize)> {
    let cfg = config::get_config();
    #[allow(deprecated)]
    let inverted_index_type = cfg.common.inverted_index_search_format.clone();
    let use_inverted_index = req.use_inverted_index;
    let use_parquet_inverted_index = use_inverted_index && inverted_index_type == "parquet";
    let use_ttv_inverted_index = use_inverted_index && inverted_index_type == "tantivy";
    log::info!(
        "[trace_id {trace_id}] flight->follower_leader: use_inverted_index with parquet format {}",
        use_parquet_inverted_index
    );

    if !use_parquet_inverted_index {
        return Ok((use_ttv_inverted_index, vec![], 0, 0));
    }

    // construct partition filters
    let equal_terms: Vec<(String, String)> = equal_terms
        .iter()
        .map(|v| (v.key.to_string(), v.value.to_string()))
        .collect::<Vec<_>>();
    // filter equal_items with index_fields
    let schema = infra::schema::get(&req.org_id, stream_name, req.stream_type).await?;
    let stream_settings = infra::schema::unwrap_stream_settings(&schema);
    let index_fields = get_stream_setting_index_fields(&stream_settings);
    let index_terms = super::super::filter_index_fields(&equal_terms, &index_fields);

    // construct SearchQuery for inverted index search
    let (start_time, end_time) = req.time_range.unwrap_or((0, 0));
    let query = SearchQuery {
        start_time,
        end_time,
        ..Default::default()
    };

    // use inverted index to filter file list
    let index_terms = generate_filter_from_equal_items(&index_terms);
    let mut index_req = req.clone();
    index_req.trace_id = trace_id.to_string(); // reset trace_id for follower
    let (idx_file_list, idx_scan_size, idx_took) =
        get_inverted_index_file_list(index_req, query, stream_name, match_terms, &index_terms)
            .await?;

    log::info!(
        "[trace_id {trace_id}] flight->follower_leader: get file_list from inverted index time_range: {:?}, num: {}, scan_size: {}, took: {} ms",
        req.time_range,
        idx_file_list.len(),
        idx_scan_size,
        idx_took,
    );

    Ok((
        use_ttv_inverted_index,
        idx_file_list,
        idx_scan_size,
        idx_took,
    ))
}
