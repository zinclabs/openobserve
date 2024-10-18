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

use std::{cmp::max, collections::HashSet, sync::Arc};

use arrow_schema::{DataType, Field, Schema};
use cache::cacher::get_ts_col_order_by;
use chrono::{Duration, Utc};
use config::{
    get_config, ider,
    meta::{
        cluster::RoleGroup,
        search,
        sql::OrderBy,
        stream::{FileKey, StreamType},
        usage::{RequestStats, UsageType},
    },
    metrics,
    utils::{base64, json, sql::is_aggregate_query, str::find},
    FxIndexSet,
};
use hashbrown::HashMap;
use infra::{
    errors::{Error, ErrorCodes},
    schema::{unwrap_partition_time_level, unwrap_stream_settings},
};
use once_cell::sync::Lazy;
use opentelemetry::trace::TraceContextExt;
use proto::cluster_rpc;
use regex::Regex;
#[cfg(not(feature = "enterprise"))]
use tokio::sync::Mutex;
use tracing::Instrument;
use tracing_opentelemetry::OpenTelemetrySpanExt;
#[cfg(feature = "enterprise")]
use {
    crate::service::grpc::get_cached_channel,
    o2_enterprise::enterprise::{
        common::infra::config::get_config as get_o2_config, search::TaskStatus, search::WorkGroup,
    },
    tonic::{codec::CompressionEncoding, metadata::MetadataValue, Request},
    tracing::info_span,
};

use super::usage::report_request_usage_stats;
use crate::{
    common::{
        infra::cluster as infra_cluster,
        meta::{self, stream::StreamParams},
        utils::functions,
    },
    handler::grpc::request::search::Searcher,
    service::format_partition_key,
};

pub(crate) mod cache;
pub(crate) mod cluster;
pub(crate) mod datafusion;
pub(crate) mod grpc;
pub(crate) mod sql;

// Checks for #ResultArray#
pub static RESULT_ARRAY: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^#[ \s]*Result[ \s]*Array[ \s]*#").unwrap());

// search manager
pub static SEARCH_SERVER: Lazy<Searcher> = Lazy::new(Searcher::new);

#[cfg(not(feature = "enterprise"))]
pub(crate) static QUEUE_LOCKER: Lazy<Arc<Mutex<bool>>> =
    Lazy::new(|| Arc::new(Mutex::const_new(false)));

/// Returns Error if the first query is failed, otherwise returns the partial results.
/// In case one query fails, the remaining queries are not executed.
#[tracing::instrument(name = "service:search_multi:enter", skip(multi_req))]
pub async fn search_multi(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    multi_req: &search::MultiStreamRequest,
) -> Result<search::Response, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();
    let cfg = get_config();
    let trace_id = if trace_id.is_empty() {
        if cfg.common.tracing_enabled || cfg.common.tracing_search_enabled {
            let ctx = tracing::Span::current().context();
            ctx.span().span_context().trace_id().to_string()
        } else {
            ider::uuid()
        }
    } else {
        trace_id.to_string()
    };

    let mut per_query_resp = multi_req.per_query_response;

    let mut query_fn = multi_req
        .query_fn
        .as_ref()
        .and_then(|v| base64::decode_url(v).ok());

    if let Some(vrl_function) = &query_fn {
        if RESULT_ARRAY.is_match(vrl_function) {
            // The query function expects results array as input
            // Hence, per_query_resp should be true
            per_query_resp = true;
        }
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let mut queries = multi_req.to_query_req();
    log::info!(
        "search_multi: trace_id: {}, queries.len(): {}",
        trace_id,
        queries.len()
    );
    let mut multi_res = search::Response::new(multi_req.from, multi_req.size);
    // Before making any rpc requests, first check the sql expressions can be decoded correctly
    for req in queries.iter_mut() {
        if let Err(e) = req.decode() {
            return Err(Error::Message(format!("decode sql error: {:?}", e)));
        }
    }
    let queries_len = queries.len();
    let mut stream_name = "".to_string();
    let mut sqls = vec![];
    let mut index = 0;

    for mut req in queries {
        stream_name = match config::meta::sql::Sql::new(&req.query.sql) {
            Ok(v) => v.source.to_string(),
            Err(e) => {
                log::error!("report_usage: parse sql error: {:?}", e);
                "".to_string()
            }
        };
        sqls.push(req.query.sql.clone());
        if !per_query_resp {
            req.query.query_fn = query_fn.clone();
        }

        for fn_name in functions::get_all_transform_keys(org_id).await {
            if req.query.sql.contains(&format!("{}(", fn_name)) {
                req.query.uses_zo_fn = true;
                break;
            }
        }

        let res = search(&trace_id, org_id, stream_type, user_id.clone(), &req).await;

        match res {
            Ok(res) => {
                index += 1;
                multi_res.took += res.took;

                if res.total > multi_res.total {
                    multi_res.total = res.total;
                }
                multi_res.from = res.from;
                multi_res.size += res.size;
                multi_res.file_count += res.file_count;
                multi_res.scan_size += res.scan_size;
                multi_res.scan_records += res.scan_records;
                multi_res.columns.extend(res.columns);

                multi_res.response_type = res.response_type;
                multi_res.trace_id = res.trace_id;
                multi_res.cached_ratio = res.cached_ratio;
                log::debug!(
                    "search_multi: res.hits.len() for query timerange to {} from {} : {}",
                    req.query.end_time,
                    req.query.start_time,
                    res.hits.len()
                );

                if per_query_resp {
                    multi_res.hits.push(serde_json::Value::Array(res.hits));
                } else {
                    multi_res.hits.extend(res.hits);
                }
            }
            Err(e) => {
                log::error!("search_multi: search error: {:?}", e);
                if index == 0 {
                    // Error in the first query, return the error
                    return Err(e); // TODO: return partial results
                } else {
                    // Error in subsequent queries, add the error to the response and break
                    // No need to run the remaining queries
                    multi_res.function_error = format!("{};{:?}", multi_res.function_error, e);
                    multi_res.is_partial = true;
                    break;
                }
            }
        }
    }

    let mut report_function_usage = false;
    multi_res.hits = if query_fn.is_some() && !multi_res.hits.is_empty() && !multi_res.is_partial {
        // compile vrl function & apply the same before returning the response
        let mut input_fn = query_fn.unwrap().trim().to_string();

        let apply_over_hits = RESULT_ARRAY.is_match(&input_fn);
        if apply_over_hits {
            input_fn = RESULT_ARRAY.replace(&input_fn, "").to_string();
        }
        let mut runtime = crate::common::utils::functions::init_vrl_runtime();
        let program = match crate::service::ingestion::compile_vrl_function(&input_fn, org_id) {
            Ok(program) => {
                let registry = program
                    .config
                    .get_custom::<vector_enrichment::TableRegistry>()
                    .unwrap();
                registry.finish_load();
                Some(program)
            }
            Err(err) => {
                log::error!("[trace_id {trace_id}] search->vrl: compile err: {:?}", err);
                multi_res.function_error = format!("{};{:?}", multi_res.function_error, err);
                None
            }
        };
        match program {
            Some(program) => {
                report_function_usage = true;
                if apply_over_hits {
                    let ret_val = crate::service::ingestion::apply_vrl_fn(
                        &mut runtime,
                        &meta::functions::VRLResultResolver {
                            program: program.program.clone(),
                            fields: program.fields.clone(),
                        },
                        &json::Value::Array(multi_res.hits),
                        org_id,
                        &stream_name,
                    );
                    ret_val
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter_map(|v| {
                            if per_query_resp {
                                let flattened_array = v
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|item| {
                                        config::utils::flatten::flatten(item.clone()).unwrap()
                                    })
                                    .collect::<Vec<_>>();
                                Some(serde_json::Value::Array(flattened_array))
                            } else {
                                (!v.is_null())
                                    .then_some(config::utils::flatten::flatten(v.clone()).unwrap())
                            }
                        })
                        .collect()
                } else {
                    multi_res
                        .hits
                        .into_iter()
                        .filter_map(|hit| {
                            let ret_val = crate::service::ingestion::apply_vrl_fn(
                                &mut runtime,
                                &meta::functions::VRLResultResolver {
                                    program: program.program.clone(),
                                    fields: program.fields.clone(),
                                },
                                &hit,
                                org_id,
                                &stream_name,
                            );
                            (!ret_val.is_null())
                                .then_some(config::utils::flatten::flatten(ret_val).unwrap())
                        })
                        .collect()
                }
            }
            None => multi_res.hits,
        }
    } else {
        multi_res.hits
    };
    log::debug!("multi_res len after applying vrl: {}", multi_res.hits.len());
    let column_timestamp = get_config().common.column_timestamp.to_string();
    multi_res.cached_ratio /= queries_len;
    multi_res.hits.sort_by(|a, b| {
        if a.get(&column_timestamp).is_none() || b.get(&column_timestamp).is_none() {
            return std::cmp::Ordering::Equal;
        }
        let a_ts = a.get(&column_timestamp).unwrap().as_i64().unwrap();
        let b_ts = b.get(&column_timestamp).unwrap().as_i64().unwrap();
        b_ts.cmp(&a_ts)
    });
    let time = start.elapsed().as_secs_f64();

    if report_function_usage {
        let req_stats = RequestStats {
            // For functions, records = records * num_function, in this case num_function = 1
            records: multi_res.total as i64,
            response_time: time,
            size: multi_res.scan_size as f64,
            request_body: Some(json::to_string(&sqls).unwrap()),
            user_email: None,
            min_ts: None,
            max_ts: None,
            cached_ratio: None,
            trace_id: None,
            // took_wait_in_queue: multi_res.t,
            search_type: multi_req.search_type,
            ..Default::default()
        };
        report_request_usage_stats(
            req_stats,
            org_id,
            &stream_name,
            stream_type,
            UsageType::Functions,
            0, // The request stats already contains function event
            started_at,
        )
        .await;
    }
    Ok(multi_res)
}

#[tracing::instrument(name = "service:search:enter", skip_all)]
pub async fn search(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    user_id: Option<String>,
    in_req: &search::Request,
) -> Result<search::Response, Error> {
    let start = std::time::Instant::now();
    let started_at = chrono::Utc::now().timestamp_micros();
    let cfg = get_config();
    let trace_id = if trace_id.is_empty() {
        if cfg.common.tracing_enabled || cfg.common.tracing_search_enabled {
            let ctx = tracing::Span::current().context();
            ctx.span().span_context().trace_id().to_string()
        } else {
            ider::uuid()
        }
    } else {
        trace_id.to_string()
    };

    #[cfg(feature = "enterprise")]
    {
        let sql = Some(in_req.query.sql.clone());
        let start_time = Some(in_req.query.start_time);
        let end_time = Some(in_req.query.end_time);
        // set search task
        SEARCH_SERVER
            .insert(
                trace_id.clone(),
                TaskStatus::new(
                    vec![],
                    true,
                    user_id.clone(),
                    Some(org_id.to_string()),
                    Some(stream_type.to_string()),
                    sql,
                    start_time,
                    end_time,
                ),
            )
            .await;
    }

    #[cfg(feature = "enterprise")]
    let req_regions = in_req.regions.clone();
    #[cfg(feature = "enterprise")]
    let req_clusters = in_req.clusters.clone();
    #[cfg(feature = "enterprise")]
    let local_cluster_search = req_regions == vec!["local"]
        && !req_clusters.is_empty()
        && (req_clusters == vec!["local"] || req_clusters == vec![config::get_cluster_name()]);

    let mut req: cluster_rpc::SearchRequest = in_req.to_owned().into();
    req.job.as_mut().unwrap().trace_id = trace_id.clone();
    req.org_id = org_id.to_string();
    req.stype = cluster_rpc::SearchType::Cluster as _;
    req.stream_type = stream_type.to_string();
    req.user_id = user_id.clone();

    let req_query = req.clone().query.unwrap();

    let span = tracing::span::Span::current();
    let handle = tokio::task::spawn(
        async move {
            #[cfg(feature = "enterprise")]
            if get_o2_config().super_cluster.enabled && !local_cluster_search {
                cluster::super_cluster::search(req, req_regions, req_clusters).await
            } else {
                cluster::http::search(req).await
            }
            #[cfg(not(feature = "enterprise"))]
            {
                cluster::http::search(req).await
            }
        }
        .instrument(span),
    );
    let res = match handle.await {
        Ok(Ok(res)) => Ok(res),
        Ok(Err(e)) => Err(e),
        Err(e) => Err(Error::Message(e.to_string())),
    };
    log::info!("[trace_id {trace_id}] in leader task finish");

    // remove task because task if finished
    let mut _work_group = None;
    #[cfg(feature = "enterprise")]
    {
        if let Some(status) = SEARCH_SERVER.remove(&trace_id, false).await {
            if let Some((_, stat)) = status.first() {
                match stat.work_group.as_ref() {
                    Some(WorkGroup::Short) => _work_group = Some("short".to_string()),
                    Some(WorkGroup::Long) => _work_group = Some("long".to_string()),
                    None => _work_group = None,
                }
            }
        };
    }

    metrics::QUERY_RUNNING_NUMS
        .with_label_values(&[org_id])
        .dec();

    // do this because of clippy warning
    match res {
        Ok(mut res) => {
            res.set_work_group(_work_group.clone());
            let time = start.elapsed().as_secs_f64();
            let (report_usage, search_type) = match in_req.search_type {
                Some(search_type) => match search_type {
                    search::SearchEventType::UI => (false, None),
                    search::SearchEventType::Dashboards => (true, in_req.search_type),
                    search::SearchEventType::Reports => (true, in_req.search_type),
                    search::SearchEventType::Alerts => (true, in_req.search_type),
                    search::SearchEventType::DerivedStream => (true, in_req.search_type),
                    search::SearchEventType::RUM => (true, in_req.search_type),
                    search::SearchEventType::Values => (false, None),
                    search::SearchEventType::Other => (false, None),
                },
                None => (false, None),
            };

            if report_usage {
                let stream_name = match config::meta::sql::Sql::new(&req_query.sql) {
                    Ok(v) => v.source.to_string(),
                    Err(e) => {
                        log::error!("report_usage: parse sql error: {:?}", e);
                        "".to_string()
                    }
                };
                let req_stats = RequestStats {
                    records: res.hits.len() as i64,
                    response_time: time,
                    size: res.scan_size as f64,
                    request_body: Some(req_query.sql.clone()),
                    function: if req_query.query_fn.is_empty() {
                        None
                    } else {
                        Some(req_query.query_fn.clone())
                    },
                    user_email: user_id,
                    min_ts: Some(req_query.start_time),
                    max_ts: Some(req_query.end_time),
                    cached_ratio: Some(res.cached_ratio),
                    search_type,
                    trace_id: Some(trace_id),
                    took_wait_in_queue: if res.took_detail.is_some() {
                        let resp_took = res.took_detail.as_ref().unwrap();
                        // Consider only the cluster wait queue duration
                        Some(resp_took.cluster_wait_queue)
                    } else {
                        None
                    },
                    work_group: _work_group,
                    is_partial: res.is_partial,
                    ..Default::default()
                };
                let num_fn = if req_query.query_fn.is_empty() { 0 } else { 1 };
                report_request_usage_stats(
                    req_stats,
                    org_id,
                    &stream_name,
                    StreamType::Logs,
                    UsageType::Search,
                    num_fn,
                    started_at,
                )
                .await;
            }
            Ok(res)
        }
        Err(e) => Err(e),
    }
}

#[tracing::instrument(name = "service:search_partition", skip(req))]
pub async fn search_partition(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    req: &search::SearchPartitionRequest,
) -> Result<search::SearchPartitionResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let query = cluster_rpc::SearchQuery {
        start_time: req.start_time,
        end_time: req.end_time,
        sql: req.sql.to_string(),
        ..Default::default()
    };
    let search_req = cluster_rpc::SearchRequest {
        org_id: org_id.to_string(),
        stream_type: stream_type.to_string(),
        query: Some(query),
        ..Default::default()
    };
    let meta = sql::Sql::new(&search_req).await?;

    let stream_settings = unwrap_stream_settings(&meta.schema).unwrap_or_default();
    let partition_time_level =
        unwrap_partition_time_level(stream_settings.partition_time_level, stream_type);
    let max_query_range = stream_settings.max_query_range * 1000 * 1000 * 60 * 60;
    let files = cluster::get_file_list(
        trace_id,
        &meta,
        stream_type,
        partition_time_level,
        &stream_settings.partition_keys,
    )
    .await;

    let file_list_took = start.elapsed().as_millis() as usize;
    log::info!(
        "[trace_id {trace_id}] search_partition: get file_list time_range: {:?}, num: {}, took: {} ms",
        meta.meta.time_range,
        files.len(),
        file_list_took,
    );

    let nodes = infra_cluster::get_cached_online_querier_nodes(Some(RoleGroup::Interactive))
        .await
        .unwrap_or_default();
    if nodes.is_empty() {
        log::error!("no querier node online");
        return Err(Error::Message("no querier node online".to_string()));
    }
    let cpu_cores = nodes.iter().map(|n| n.cpu_num).sum::<u64>() as usize;

    let (records, original_size, compressed_size) =
        files
            .iter()
            .fold((0, 0, 0), |(records, original_size, compressed_size), f| {
                (
                    records + f.meta.records,
                    original_size + f.meta.original_size,
                    compressed_size + f.meta.compressed_size,
                )
            });
    let mut resp = search::SearchPartitionResponse {
        trace_id: trace_id.to_string(),
        file_num: files.len(),
        records: records as usize,
        original_size: original_size as usize,
        compressed_size: compressed_size as usize,
        histogram_interval: meta.histogram_interval,
        max_query_range: stream_settings.max_query_range,
        partitions: vec![],
        order_by: OrderBy::Desc,
    };

    // check for vrl
    let apply_over_hits = match req.query_fn.as_ref() {
        None => false,
        Some(v) => {
            if v.is_empty() {
                false
            } else {
                let v = base64::decode_url(v).unwrap_or(v.to_string());
                RESULT_ARRAY.is_match(&v)
            }
        }
    };

    // if there is no _timestamp field in the query, return single partitions
    let is_aggregate = is_aggregate_query(&req.sql).unwrap_or(false);
    let res_ts_column = get_ts_col_order_by(&meta.meta, &cfg.common.column_timestamp, is_aggregate);
    let ts_column = res_ts_column.map(|(v, _)| v);
    if ts_column.is_none() || apply_over_hits {
        resp.partitions.push([req.start_time, req.end_time]);
        return Ok(resp);
    };

    let mut min_step = Duration::try_seconds(1)
        .unwrap()
        .num_microseconds()
        .unwrap();
    if is_aggregate && ts_column.is_some() {
        min_step *= meta.histogram_interval.unwrap_or(1);
    }

    let mut total_secs = resp.original_size / cfg.limit.query_group_base_speed / cpu_cores;
    if total_secs * cfg.limit.query_group_base_speed * cpu_cores < resp.original_size {
        total_secs += 1;
    }
    let mut part_num = max(1, total_secs / cfg.limit.query_partition_by_secs);
    if part_num * cfg.limit.query_partition_by_secs < total_secs {
        part_num += 1;
    }
    // if the partition number is too large, we limit it to 1000
    if part_num > 1000 {
        part_num = 1000;
    }
    let mut step = (req.end_time - req.start_time) / part_num as i64;
    // step must be times of min_step
    if step < min_step {
        step = min_step;
    }
    if step % min_step > 0 {
        step = step - step % min_step;
    }
    // this is to ensure we create partitions less than max_query_range
    if max_query_range > 0 && step > max_query_range {
        step = if min_step < max_query_range {
            max_query_range - max_query_range % min_step
        } else {
            max_query_range
        };
    }

    // Generate partitions by DESC order
    let mut partitions = Vec::with_capacity(part_num);
    let mut end = req.end_time;
    let mut last_partition_step = end % min_step;
    let duration = req.end_time - req.start_time;
    while end > req.start_time {
        let mut start = max(end - step, req.start_time);
        if last_partition_step > 0 && duration > min_step && part_num > 1 {
            partitions.push([end - last_partition_step, end]);
            start -= last_partition_step;
            end -= last_partition_step;
        } else {
            start = max(start - last_partition_step, req.start_time);
        }
        partitions.push([start, end]);
        end = start;
        last_partition_step = 0;
    }
    if partitions.is_empty() {
        partitions.push([req.start_time, req.end_time]);
    }

    // We need to reverse partitions if query is ASC order
    if let Some((field, order_by)) = meta.meta.order_by.first() {
        if field == &ts_column.unwrap() && order_by == &OrderBy::Asc {
            resp.order_by = OrderBy::Asc;
            partitions.reverse();
        }
    }

    resp.partitions = partitions;
    Ok(resp)
}

#[cfg(feature = "enterprise")]
pub async fn query_status() -> Result<search::QueryStatusResponse, Error> {
    // get nodes from cluster
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("query_status: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let nodes = nodes;

    // make cluster request
    let mut tasks = Vec::new();
    for node in nodes.iter().cloned() {
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:grpc_query_status",
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );

        let task = tokio::task::spawn(
            async move {
                let cfg = get_config();
                let mut request = tonic::Request::new(proto::cluster_rpc::QueryStatusRequest {});
                request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));

                opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(
                        &tracing::Span::current().context(),
                        &mut MetadataMap(request.metadata_mut()),
                    )
                });

                let token: MetadataValue<_> = infra_cluster::get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = get_cached_channel(&node_addr).await.map_err(|err| {
                    log::error!(
                        "search->grpc: node: {}, connect err: {:?}",
                        &node.grpc_addr,
                        err
                    );
                    server_internal_error("connect search node error")
                })?;
                let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
                    channel,
                    move |mut req: Request<()>| {
                        req.metadata_mut().insert("authorization", token.clone());
                        Ok(req)
                    },
                );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                let response = match client.query_status(request).await {
                    Ok(res) => res.into_inner(),
                    Err(err) => {
                        log::error!(
                            "search->grpc: node: {}, search err: {:?}",
                            &node.grpc_addr,
                            err
                        );
                        if err.code() == tonic::Code::Internal {
                            let err = ErrorCodes::from_json(err.message())?;
                            return Err(Error::ErrorCode(err));
                        }
                        return Err(server_internal_error("search node error"));
                    }
                };
                Ok(response)
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok(res) => results.push(res),
                Err(err) => {
                    return Err(err);
                }
            },
            Err(e) => {
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    e.to_string(),
                )));
            }
        }
    }

    let mut status = vec![];
    let mut set = HashSet::new();
    for result in results.into_iter().flat_map(|v| v.status.into_iter()) {
        if set.contains(&result.trace_id) {
            continue;
        } else {
            set.insert(result.trace_id.clone());
        }
        let query = result.query.as_ref().map(|query| search::QueryInfo {
            sql: query.sql.clone(),
            start_time: query.start_time,
            end_time: query.end_time,
        });
        let scan_stats = result
            .scan_stats
            .as_ref()
            .map(|scan_stats| search::ScanStats {
                files: scan_stats.files,
                records: scan_stats.records,
                original_size: scan_stats.original_size / 1024 / 1024, // change to MB
                compressed_size: scan_stats.compressed_size / 1024 / 1024, // change to MB
                querier_files: scan_stats.querier_files,
                querier_memory_cached_files: scan_stats.querier_memory_cached_files,
                querier_disk_cached_files: scan_stats.querier_disk_cached_files,
                idx_scan_size: scan_stats.idx_scan_size / 1024 / 1024, // change to MB
                idx_took: scan_stats.idx_took,
            });
        let query_status = if result.is_queue {
            "waiting"
        } else {
            "processing"
        };
        let work_group = if result.work_group.is_none() {
            "Unknown"
        } else if *result.work_group.as_ref().unwrap() == 0 {
            "Short"
        } else {
            "Long"
        };
        status.push(search::QueryStatus {
            trace_id: result.trace_id,
            created_at: result.created_at,
            started_at: result.started_at,
            status: query_status.to_string(),
            user_id: result.user_id,
            org_id: result.org_id,
            stream_type: result.stream_type,
            query,
            scan_stats,
            work_group: work_group.to_string(),
        });
    }

    Ok(search::QueryStatusResponse { status })
}

#[cfg(feature = "enterprise")]
pub async fn cancel_query(
    _org_id: &str,
    trace_id: &str,
) -> Result<search::CancelQueryResponse, Error> {
    // get nodes from cluster
    let mut nodes = match infra_cluster::get_cached_online_query_nodes(None).await {
        Some(nodes) => nodes,
        None => {
            log::error!("cancel_query: no querier node online");
            return Err(server_internal_error("no querier node online"));
        }
    };
    // sort nodes by node_id this will improve hit cache ratio
    nodes.dedup_by(|a, b| a.grpc_addr == b.grpc_addr);
    let nodes = nodes;

    // make cluster request
    let mut tasks = Vec::new();
    for node in nodes.iter().cloned() {
        let node_addr = node.grpc_addr.clone();
        let grpc_span = info_span!(
            "service:search:cluster:grpc_cancel_query",
            node_id = node.id,
            node_addr = node_addr.as_str(),
        );

        let trace_id = trace_id.to_string();
        let task = tokio::task::spawn(
            async move {
                let cfg = get_config();
                let mut request =
                    tonic::Request::new(proto::cluster_rpc::CancelQueryRequest { trace_id });
                request.set_timeout(std::time::Duration::from_secs(cfg.limit.query_timeout));
                opentelemetry::global::get_text_map_propagator(|propagator| {
                    propagator.inject_context(
                        &tracing::Span::current().context(),
                        &mut MetadataMap(request.metadata_mut()),
                    )
                });

                let token: MetadataValue<_> = infra_cluster::get_internal_grpc_token()
                    .parse()
                    .map_err(|_| Error::Message("invalid token".to_string()))?;
                let channel = get_cached_channel(&node_addr).await.map_err(|err| {
                    log::error!(
                        "search->cancel_query: node: {}, connect err: {:?}",
                        &node.grpc_addr,
                        err
                    );
                    server_internal_error("connect search node error")
                })?;
                let mut client = cluster_rpc::search_client::SearchClient::with_interceptor(
                    channel,
                    move |mut req: Request<()>| {
                        req.metadata_mut().insert("authorization", token.clone());
                        Ok(req)
                    },
                );
                client = client
                    .send_compressed(CompressionEncoding::Gzip)
                    .accept_compressed(CompressionEncoding::Gzip)
                    .max_decoding_message_size(cfg.grpc.max_message_size * 1024 * 1024)
                    .max_encoding_message_size(cfg.grpc.max_message_size * 1024 * 1024);
                let response: cluster_rpc::CancelQueryResponse =
                    match client.cancel_query(request).await {
                        Ok(res) => res.into_inner(),
                        Err(err) => {
                            log::error!(
                                "grpc_cancel_query: node: {}, search err: {:?}",
                                &node.grpc_addr,
                                err
                            );
                            if err.code() == tonic::Code::Internal {
                                let err = ErrorCodes::from_json(err.message())?;
                                return Err(Error::ErrorCode(err));
                            }
                            return Err(server_internal_error("search node error"));
                        }
                    };
                Ok(response)
            }
            .instrument(grpc_span),
        );
        tasks.push(task);
    }

    let mut results = Vec::new();
    for task in tasks {
        match task.await {
            Ok(res) => match res {
                Ok(res) => results.push(res),
                Err(err) => {
                    return Err(err);
                }
            },
            Err(e) => {
                return Err(Error::ErrorCode(ErrorCodes::ServerInternalError(
                    e.to_string(),
                )));
            }
        }
    }

    let mut is_success = false;
    for res in results {
        if res.is_success {
            is_success = true;
            break;
        }
    }

    Ok(search::CancelQueryResponse {
        trace_id: trace_id.to_string(),
        is_success,
    })
}

/// match a source is a valid file or not
pub async fn match_source(
    stream: StreamParams,
    time_range: Option<(i64, i64)>,
    filters: &[(&str, Vec<String>)],
    source: &FileKey,
    is_wal: bool,
    match_min_ts_only: bool,
) -> bool {
    if stream.stream_type.eq(&StreamType::Metrics)
        && source.key.starts_with(
            format!(
                "files/{}/{}/{}/",
                stream.org_id, stream.stream_type, stream.org_id
            )
            .as_str(),
        )
    {
        return true;
    }

    // match org_id & table
    if !source.key.starts_with(
        format!(
            "files/{}/{}/{}/",
            stream.org_id, stream.stream_type, stream.stream_name
        )
        .as_str(),
    ) {
        return false;
    }

    // check partition key
    if !filter_source_by_partition_key(&source.key, filters) {
        return false;
    }

    if is_wal {
        return true;
    }

    // check time range
    if source.meta.min_ts == 0 || source.meta.max_ts == 0 {
        return true;
    }
    log::trace!(
        "time range: {:?}, file time: {}-{}, {}",
        time_range,
        source.meta.min_ts,
        source.meta.max_ts,
        source.key
    );

    // match partition clause
    if let Some((time_min, time_max)) = time_range {
        if match_min_ts_only && time_min > 0 {
            return source.meta.min_ts >= time_min && source.meta.min_ts < time_max;
        }
        if time_min > 0 && time_min > source.meta.max_ts {
            return false;
        }
        if time_max > 0 && time_max < source.meta.min_ts {
            return false;
        }
    }
    true
}

/// match a source is a needed file or not, return true if needed
fn filter_source_by_partition_key(source: &str, filters: &[(&str, Vec<String>)]) -> bool {
    !filters.iter().any(|(k, v)| {
        let field = format_partition_key(&format!("{k}="));
        find(source, &format!("/{field}"))
            && !v.iter().any(|v| {
                let value = format_partition_key(&format!("{k}={v}"));
                find(source, &format!("/{value}/"))
            })
    })
}

pub fn server_internal_error(error: impl ToString) -> Error {
    Error::ErrorCode(ErrorCodes::ServerInternalError(error.to_string()))
}

#[tracing::instrument(name = "service:search_partition_multi", skip(req))]
pub async fn search_partition_multi(
    trace_id: &str,
    org_id: &str,
    stream_type: StreamType,
    req: &search::MultiSearchPartitionRequest,
) -> Result<search::SearchPartitionResponse, Error> {
    let mut res = search::SearchPartitionResponse::default();
    let mut total_rec = 0;
    for query in &req.sql {
        match search_partition(
            trace_id,
            org_id,
            stream_type,
            &search::SearchPartitionRequest {
                start_time: req.start_time,
                end_time: req.end_time,
                sql: query.to_string(),
                encoding: req.encoding,
                regions: req.regions.clone(),
                clusters: req.clusters.clone(),
                query_fn: req.query_fn.clone(),
            },
        )
        .await
        {
            Ok(resp) => {
                if resp.partitions.len() > res.partitions.len() {
                    total_rec += resp.records;
                    res = resp;
                }
            }
            Err(err) => {
                log::error!("search_partition_multi error: {:?}", err);
            }
        };
    }
    res.records = total_rec;
    Ok(res)
}

pub struct MetadataMap<'a>(pub &'a mut tonic::metadata::MetadataMap);

impl<'a> opentelemetry::propagation::Injector for MetadataMap<'a> {
    /// Set a key and value in the MetadataMap.  Does nothing if the key or
    /// value are not valid inputs
    fn set(&mut self, key: &str, value: String) {
        if let Ok(key) = tonic::metadata::MetadataKey::from_bytes(key.as_bytes()) {
            if let Ok(val) = tonic::metadata::MetadataValue::try_from(&value) {
                self.0.insert(key, val);
            }
        }
    }
}

// generate parquet file search schema
fn generate_search_schema(
    sql: &Arc<sql::Sql>,
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
) -> Result<(Arc<Schema>, HashMap<String, DataType>), Error> {
    // cacluate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();
    let mut new_fields = Vec::new();

    for field in generate_used_fields_in_query(sql).iter() {
        let group_field = schema.field_with_name(field).ok();
        let latest_field = schema_latest_map.get(field).map(|f| f.as_ref());

        match (group_field, latest_field) {
            // When group_field is None and latest_field is Some, clone latest_field
            (None, Some(field)) => new_fields.push(Arc::new(field.clone())),

            // When both group_field and latest_field are Some, compare their data types
            (Some(group_field), Some(latest_field)) => {
                if group_field.data_type() != latest_field.data_type() {
                    diff_fields.insert(field.to_string(), latest_field.data_type().clone());
                }
                new_fields.push(Arc::new(group_field.clone()));
            }

            // should we return error
            _ => {}
        }
    }

    for (field, alias) in sql.meta.field_alias.iter() {
        if let Some(v) = diff_fields.get(field) {
            diff_fields.insert(alias.to_string(), v.clone());
        }
    }

    let mut schema = Schema::new(new_fields);
    let timestamp = &get_config().common.column_timestamp;
    if schema.field_with_name(timestamp).is_err() {
        // self add timestamp column if no exist
        let field = Arc::new(Field::new(timestamp, DataType::Int64, false));
        schema = Schema::try_merge(vec![Schema::new(vec![field]), schema])?;
    }

    Ok((Arc::new(schema), diff_fields))
}

// generate parquet file search schema
fn generate_select_start_search_schema(
    sql: &Arc<sql::Sql>,
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
    defined_schema_fields: &[String],
) -> Result<(Arc<Schema>, HashMap<String, DataType>), Error> {
    let schema_fields_map = schema
        .fields()
        .iter()
        .map(|f| (f.name(), f))
        .collect::<HashMap<_, _>>();
    // cacluate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();
    for field in schema.fields().iter() {
        if let Some(f) = schema_latest_map.get(field.name()) {
            if f.data_type() != field.data_type() {
                diff_fields.insert(field.name().clone(), f.data_type().clone());
            }
        }
    }
    for (field, alias) in sql.meta.field_alias.iter() {
        if let Some(v) = diff_fields.get(field) {
            diff_fields.insert(alias.to_string(), v.clone());
        }
    }
    // add not exists field in group schema but used in sql
    let mut new_fields = Vec::new();
    let used_fields = generate_used_fields_in_query(sql);
    for field in used_fields.iter() {
        if schema_fields_map.get(field).is_none() {
            if let Some(field) = schema_latest_map.get(field) {
                new_fields.push(Arc::new(field.as_ref().clone()));
            }
        }
    }
    let cfg = get_config();
    let mut new_schema_fields = if !defined_schema_fields.is_empty() {
        let mut fields: HashSet<String> = defined_schema_fields.iter().cloned().collect();
        if !fields.contains(&cfg.common.column_timestamp) {
            fields.insert(cfg.common.column_timestamp.to_string());
        }
        if !fields.contains(config::ID_COL_NAME) {
            fields.insert(config::ID_COL_NAME.to_string());
        }
        if !cfg.common.feature_query_exclude_all && !fields.contains(&cfg.common.column_all) {
            fields.insert(cfg.common.column_all.to_string());
        }
        let new_fields = fields
            .iter()
            .filter_map(|f| match schema_fields_map.get(f) {
                Some(f) => Some((*f).clone()),
                None => schema_latest_map.get(f).map(|f| (*f).clone()),
            })
            .collect::<Vec<_>>();
        new_fields
    } else if !new_fields.is_empty() {
        let mut merged_fields = schema.fields().to_vec();
        // can extend directly b/c none of the `new_fields` is present in schema, checked above
        merged_fields.extend(new_fields);
        merged_fields
    } else {
        schema.fields().to_vec()
    };

    // skip selecting "_original" column if `SELECT * ...`
    if !used_fields.contains(config::ORIGINAL_DATA_COL_NAME) {
        new_schema_fields.retain(|field| field.name() != config::ORIGINAL_DATA_COL_NAME);
    }

    Ok((Arc::new(Schema::new(new_schema_fields)), diff_fields))
}

fn generate_used_fields_in_query(sql: &Arc<sql::Sql>) -> FxIndexSet<String> {
    let alias_map: HashSet<&String> = sql.meta.field_alias.iter().map(|(_, v)| v).collect();

    // note field name maybe equal to alias name
    sql.meta
        .group_by
        .iter()
        .chain(sql.meta.order_by.iter().map(|(f, _)| f))
        .filter(|f| !alias_map.contains(*f))
        .chain(&sql.meta.fields)
        .cloned()
        .collect()
}

// generate parquet file search schema
fn generate_search_schema_diff(
    schema: &Schema,
    schema_latest_map: &HashMap<&String, &Arc<Field>>,
) -> Result<HashMap<String, DataType>, Error> {
    // cacluate the diff between latest schema and group schema
    let mut diff_fields = HashMap::new();

    for field in schema.fields().iter() {
        if let Some(latest_field) = schema_latest_map.get(field.name()) {
            if field.data_type() != latest_field.data_type() {
                diff_fields.insert(field.name().clone(), latest_field.data_type().clone());
            }
        }
    }

    Ok(diff_fields)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_matches_by_partition_key_with_str() {
        let path = "files/default/logs/gke-fluentbit/2023/04/14/08/kuberneteshost=gke-dev1/kubernetesnamespacename=ziox-dev/7052558621820981249.parquet";
        let filters = vec![
            (vec![], true),
            (vec![("kuberneteshost", vec!["gke-dev1".to_string()])], true),
            (
                vec![("kuberneteshost", vec!["gke-dev2".to_string()])],
                false,
            ),
            (
                vec![(
                    "kuberneteshost",
                    vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                )],
                true,
            ),
            (
                vec![("some_other_key", vec!["no-matter".to_string()])],
                true,
            ),
            (
                vec![
                    ("kuberneteshost", vec!["gke-dev1".to_string()]),
                    ("kubernetesnamespacename", vec!["ziox-dev".to_string()]),
                ],
                true,
            ),
            (
                vec![
                    ("kuberneteshost", vec!["gke-dev1".to_string()]),
                    ("kubernetesnamespacename", vec!["abcdefg".to_string()]),
                ],
                false,
            ),
            (
                vec![
                    ("kuberneteshost", vec!["gke-dev2".to_string()]),
                    ("kubernetesnamespacename", vec!["ziox-dev".to_string()]),
                ],
                false,
            ),
            (
                vec![
                    ("kuberneteshost", vec!["gke-dev2".to_string()]),
                    ("kubernetesnamespacename", vec!["abcdefg".to_string()]),
                ],
                false,
            ),
            (
                vec![
                    (
                        "kuberneteshost",
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    ("kubernetesnamespacename", vec!["ziox-dev".to_string()]),
                ],
                true,
            ),
            (
                vec![
                    (
                        "kuberneteshost",
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    ("kubernetesnamespacename", vec!["abcdefg".to_string()]),
                ],
                false,
            ),
            (
                vec![
                    (
                        "kuberneteshost",
                        vec!["gke-dev1".to_string(), "gke-dev2".to_string()],
                    ),
                    ("some_other_key", vec!["no-matter".to_string()]),
                ],
                true,
            ),
        ];
        for (filter, expected) in filters {
            assert_eq!(filter_source_by_partition_key(path, &filter), expected);
        }
    }

    #[test]
    fn test_matches_by_partition_key_with_sql() {
        use config::meta::sql;
        let path = "files/default/logs/gke-fluentbit/2023/04/14/08/kuberneteshost=gke-dev1/kubernetesnamespacename=ziox-dev/7052558621820981249.parquet";
        let sqls = vec![
            ("SELECT * FROM tbl", true),
            ("SELECT * FROM tbl WHERE kuberneteshost='gke-dev1'", true),
            ("SELECT * FROM tbl WHERE kuberneteshost='gke-dev2'", false),
            ("SELECT * FROM tbl WHERE some_other_key = 'no-matter'", true),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' AND kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' AND kubernetesnamespacename='ziox-dev'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev2' OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost='gke-dev1' OR kuberneteshost='gke-dev2'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1')",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev2')",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2')",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') AND kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') AND kubernetesnamespacename='abcdefg'",
                false,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR kubernetesnamespacename='ziox-dev'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR kubernetesnamespacename='abcdefg'",
                true,
            ),
            (
                "SELECT * FROM tbl WHERE kuberneteshost IN ('gke-dev1', 'gke-dev2') OR some_other_key='abcdefg'",
                true,
            ),
        ];
        for (tsql, expected) in sqls {
            let meta = sql::Sql::new(tsql).unwrap();
            let filter = super::sql::generate_filter_from_quick_text(&meta.quick_text);
            assert_eq!(filter_source_by_partition_key(path, &filter), expected);
        }
    }
}
