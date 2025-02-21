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

use std::{collections::HashMap, io::Error};

use actix_web::{get, http::StatusCode, post, web, HttpRequest, HttpResponse};
use arrow_schema::Schema;
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        search::{SearchEventType, SearchHistoryHitResponse},
        self_reporting::usage::{RequestStats, UsageType, USAGE_STREAM},
        sql::resolve_stream_names,
        stream::StreamType,
    },
    metrics,
    utils::{base64, json},
    DISTINCT_FIELDS, TIMESTAMP_COL_NAME,
};
use infra::{cache::stats, errors};
use tracing::{Instrument, Span};
#[cfg(feature = "enterprise")]
use utils::check_stream_permissions;

#[cfg(feature = "enterprise")]
use crate::service::search::sql::get_cipher_key_names;
use crate::{
    common::{
        meta::{self, http::HttpResponse as MetaHttpResponse},
        utils::{
            functions,
            http::{
                get_or_create_trace_id, get_search_event_context_from_request,
                get_search_type_from_request, get_stream_type_from_request,
                get_use_cache_from_request, get_work_group,
            },
            stream::get_settings_max_query_range,
        },
    },
    service::{
        metadata::distinct_values::DISTINCT_STREAM_PREFIX,
        search as SearchService,
        self_reporting::{http_report_metrics, report_request_usage_stats},
    },
};

pub mod multi_streams;
#[cfg(feature = "enterprise")]
pub mod query_manager;
pub mod saved_view;
#[cfg(feature = "enterprise")]
pub mod search_job;
#[cfg(feature = "enterprise")]
pub(crate) mod utils;

async fn can_use_distinct_stream(
    org: &str,
    stream_name: &str,
    stream_type: StreamType,
    fields: &[String],
    query_sql: &str,
    start_time: i64,
) -> bool {
    if !matches!(stream_type, StreamType::Logs | StreamType::Traces) {
        return false;
    }

    let stream_settings = infra::schema::get_settings(org, stream_name, stream_type)
        .await
        .unwrap_or_default();

    // all fields which are requested must be in the distinct stream
    let all_fields_distinct = fields.iter().all(|f| {
        if DISTINCT_FIELDS.contains(f) {
            return true;
        }
        stream_settings
            .distinct_value_fields
            .iter()
            .any(|entry| entry.name == *f && entry.added_ts <= start_time)
    });

    // all the fields used in the query sent must be in the distinct stream
    let query_fields: Vec<_> = match config::meta::sql::Sql::new(query_sql) {
        // if sql is invalid, we let it follow the original search and fail
        Err(_) => return false,
        Ok(sql) => sql
            .fields
            .into_iter()
            .filter(|f| f != "_timestamp")// _timestamp is hardcoded in queries
            .collect(),
    };

    let all_query_fields_distinct = query_fields.iter().all(|f| {
        if DISTINCT_FIELDS.contains(f) {
            return true;
        }
        stream_settings
            .distinct_value_fields
            .iter()
            .any(|entry| entry.name == *f && entry.added_ts <= start_time)
    });

    all_fields_distinct && all_query_fields_distinct
}

/// SearchStreamData
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchSQL",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = SearchRequest, description = "Search query", content_type = "application/json", example = json!({
        "query": {
            "sql": "select * from k8s ",
            "start_time": 1675182660872049i64,
            "end_time": 1675185660872049i64,
            "from": 0,
            "size": 10
        }
    })),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "hits": [
                {
                    "_p": "F",
                    "_timestamp": 1674213225158000i64,
                    "kubernetes": {
                        "container_hash": "dkr.ecr.us-west-2.amazonaws.com/openobserve@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
                        "container_image": "dkr.ecr.us-west-2.amazonaws.com/openobserve:v0.0.3",
                        "container_name": "openobserve",
                        "docker_id": "eb0983bdb9ff9360d227e6a0b268fe3b24a0868c2c2d725a1516c11e88bf5789",
                        "host": "ip.us-east-2.compute.internal",
                        "namespace_name": "openobserve",
                        "pod_id": "35a0421f-9203-4d73-9663-9ff0ce26d409",
                        "pod_name": "openobserve-ingester-0"
                    },
                    "log": "[2023-01-20T11:13:45Z INFO  actix_web::middleware::logger] 10.2.80.192 \"POST /api/demo/_bulk HTTP/1.1\" 200 68",
                    "stream": "stderr"
                }
            ],
            "total": 27179431,
            "from": 0,
            "size": 1,
            "scan_size": 28943
        })),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/_search")]
pub async fn search(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let org_id = org_id.into_inner();
    let mut range_error = String::new();
    let http_span = if cfg.common.tracing_search_enabled || cfg.common.tracing_enabled {
        tracing::info_span!("/api/{org_id}/_search", org_id = org_id.clone())
    } else {
        Span::none()
    };

    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let use_cache = cfg.common.result_cache_enabled && get_use_cache_from_request(&query);
    // handle encoding for query and aggs
    let mut req: config::meta::search::Request = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };
    if let Err(e) = req.decode() {
        return Ok(MetaHttpResponse::bad_request(e));
    }
    req.use_cache = Some(use_cache);

    // set search event type
    if req.search_type.is_none() {
        req.search_type = match get_search_type_from_request(&query) {
            Ok(v) => v,
            Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
        };
    };
    if req.search_event_context.is_none() {
        req.search_event_context = req
            .search_type
            .as_ref()
            .and_then(|event_type| get_search_event_context_from_request(event_type, &query));
    }

    // get stream name
    let stream_names = match resolve_stream_names(&req.query.sql) {
        Ok(v) => v.clone(),
        Err(e) => {
            return Ok(
                HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    e.to_string(),
                )),
            );
        }
    };

    // get stream settings
    for stream_name in stream_names {
        if let Some(settings) =
            infra::schema::get_settings(&org_id, &stream_name, stream_type).await
        {
            let max_query_range =
                get_settings_max_query_range(settings.max_query_range, &org_id, Some(&user_id))
                    .await;
            if max_query_range > 0
                && (req.query.end_time - req.query.start_time) > max_query_range * 3600 * 1_000_000
            {
                req.query.start_time = req.query.end_time - max_query_range * 3600 * 1_000_000;
                range_error = format!(
                    "Query duration is modified due to query range restriction of {} hours",
                    max_query_range
                );
            }
        }

        // Check permissions on stream
        #[cfg(feature = "enterprise")]
        if let Some(res) =
            check_stream_permissions(&stream_name, &org_id, &user_id, &stream_type).await
        {
            return Ok(res);
        }
    }

    #[cfg(feature = "enterprise")]
    {
        let keys_used = match get_cipher_key_names(&req.query.sql) {
            Ok(v) => v,
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().json(
                    meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        e.to_string(),
                    ),
                ));
            }
        };
        if !keys_used.is_empty() {
            log::info!("keys used : {:?}", keys_used);
        }
        for key in keys_used {
            // Check permissions on keys
            {
                use o2_openfga::meta::mapping::OFGA_MODELS;

                use crate::common::{
                    infra::config::USERS,
                    utils::auth::{is_root_user, AuthExtractor},
                };

                if !is_root_user(&user_id) {
                    let user: meta::user::User =
                        USERS.get(&format!("{org_id}/{}", user_id)).unwrap().clone();

                    if !crate::handler::http::auth::validator::check_permissions(
                        &user_id,
                        AuthExtractor {
                            auth: "".to_string(),
                            method: "GET".to_string(),
                            o2_type: format!(
                                "{}:{}",
                                OFGA_MODELS
                                    .get("cipher_keys")
                                    .map_or("cipher_keys", |model| model.key),
                                key
                            ),
                            org_id: org_id.clone(),
                            bypass_check: false,
                            parent_id: "".to_string(),
                        },
                        user.role,
                        user.is_external,
                    )
                    .await
                    {
                        return Ok(MetaHttpResponse::forbidden("Unauthorized Access to key"));
                    }
                    // Check permissions on key ends
                }
            }
        }
    }

    // run search with cache
    let res = SearchService::cache::search(
        &trace_id,
        &org_id,
        stream_type,
        Some(user_id),
        &req,
        range_error,
    )
    .instrument(http_span)
    .await;
    match res {
        Ok(res) => Ok(HttpResponse::Ok().json(res)),
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, "", "500", "_search");
            log::error!("[trace_id {trace_id}] search error: {}", err);
            Ok(match err {
                errors::Error::ErrorCode(code) => match code {
                    errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                        .json(meta::http::HttpResponse::error_code_with_trace_id(
                            code,
                            Some(trace_id),
                        )),
                    _ => HttpResponse::InternalServerError().json(
                        meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                    ),
                },
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            })
        }
    }
}

/// SearchAround
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchAround",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "stream_name name"),
        ("key" = i64, Query, description = "around key"),
        ("size" = i64, Query, description = "around size"),
        ("regions" = Option<String>, Query, description = "regions, split by comma"),
        ("timeout" = Option<i64>, Query, description = "timeout, seconds"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "hits": [
                {
                    "_p": "F",
                    "_timestamp": 1674213225158000i64,
                    "kubernetes": {
                        "container_hash": "dkr.ecr.us-west-2.amazonaws.com/openobserve@sha256:3dbbb0dc1eab2d5a3b3e4a75fd87d194e8095c92d7b2b62e7cdbd07020f54589",
                        "container_image": "dkr.ecr.us-west-2.amazonaws.com/openobserve:v0.0.3",
                        "container_name": "openobserve",
                        "docker_id": "eb0983bdb9ff9360d227e6a0b268fe3b24a0868c2c2d725a1516c11e88bf5789",
                        "host": "ip.us-east-2.compute.internal",
                        "namespace_name": "openobserve",
                        "pod_id": "35a0421f-9203-4d73-9663-9ff0ce26d409",
                        "pod_name": "openobserve-ingester-0"
                    },
                    "log": "[2023-01-20T11:13:45Z INFO  actix_web::middleware::logger] 10.2.80.192 \"POST /api/demo/_bulk HTTP/1.1\" 200 68",
                    "stream": "stderr"
                }
            ],
            "total": 10,
            "from": 0,
            "size": 10,
            "scan_size": 28943
        })),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_name}/_around")]
pub async fn around(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let started_at = Utc::now().timestamp_micros();
    let (org_id, stream_name) = path.into_inner();
    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!(
            "/api/{org_id}/{stream_name}/_around",
            org_id = org_id.clone(),
            stream_name = stream_name.clone()
        )
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);
    let user_id = in_req
        .headers()
        .get("user_id")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let mut uses_fn = false;
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let around_key = match query.get("key") {
        Some(v) => v.parse::<i64>().unwrap_or(0),
        None => return Ok(MetaHttpResponse::bad_request("around key is empty")),
    };
    let mut query_fn = query
        .get("query_fn")
        .and_then(|v| base64::decode_url(v).ok());
    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let default_sql = format!("SELECT * FROM \"{}\" ", stream_name);
    let around_sql = match query.get("sql") {
        None => default_sql,
        Some(v) => match base64::decode_url(v) {
            Err(_) => default_sql,
            Ok(sql) => {
                uses_fn = functions::get_all_transform_keys(&org_id)
                    .await
                    .iter()
                    .any(|fn_name| sql.contains(&format!("{}(", fn_name)));
                if uses_fn {
                    sql
                } else {
                    default_sql
                }
            }
        },
    };

    let around_size = query
        .get("size")
        .map_or(10, |v| v.parse::<i64>().unwrap_or(10));

    let regions = query.get("regions").map_or(vec![], |regions| {
        regions
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let clusters = query.get("clusters").map_or(vec![], |clusters| {
        clusters
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .inc();
    // get a local search queue lock
    #[cfg(not(feature = "enterprise"))]
    let locker = SearchService::QUEUE_LOCKER.clone();
    #[cfg(not(feature = "enterprise"))]
    let locker = locker.lock().await;
    #[cfg(not(feature = "enterprise"))]
    if !cfg.common.feature_query_queue_enabled {
        drop(locker);
    }
    #[cfg(not(feature = "enterprise"))]
    let took_wait = start.elapsed().as_millis() as usize;
    #[cfg(feature = "enterprise")]
    let took_wait = 0;
    log::info!(
        "http search around API wait in queue took: {} ms",
        took_wait
    );
    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .dec();

    let timeout = query
        .get("timeout")
        .map_or(0, |v| v.parse::<i64>().unwrap_or(0));
    let around_start_time = around_key
        - Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();
    let around_end_time = around_key
        + Duration::try_seconds(900)
            .unwrap()
            .num_microseconds()
            .unwrap();

    // search forward
    let fw_sql = SearchService::sql::check_or_add_order_by_timestamp(&around_sql, false)
        .unwrap_or(around_sql.to_string());
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: fw_sql,
            from: 0,
            size: around_size / 2,
            start_time: around_start_time,
            end_time: around_key,
            quick_mode: false,
            query_type: "".to_string(),
            track_total_hits: false,
            uses_zo_fn: uses_fn,
            query_fn: query_fn.clone(),
            action_id: None,
            skip_wal: false,
            streaming_output: false,
            streaming_id: None,
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions: regions.clone(),
        clusters: clusters.clone(),
        timeout,
        search_type: Some(SearchEventType::UI),
        search_event_context: None,
        use_cache: None,
    };
    let search_res = SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
        .instrument(http_span.clone())
        .await;

    let resp_forward = match search_res {
        Ok(res) => res,
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, &stream_name, "500", "_around");
            log::error!("search around error: {:?}", err);
            return Ok(match err {
                errors::Error::ErrorCode(code) => match code {
                    errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                        .json(meta::http::HttpResponse::error_code_with_trace_id(
                            code,
                            Some(trace_id),
                        )),
                    _ => HttpResponse::InternalServerError().json(
                        meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                    ),
                },
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            });
        }
    };

    // search backward
    let bw_sql = SearchService::sql::check_or_add_order_by_timestamp(&around_sql, true)
        .unwrap_or(around_sql.to_string());
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: bw_sql,
            from: 0,
            size: around_size / 2,
            start_time: around_key,
            end_time: around_end_time,
            quick_mode: false,
            query_type: "".to_string(),
            track_total_hits: false,
            uses_zo_fn: uses_fn,
            query_fn: query_fn.clone(),
            action_id: None,
            skip_wal: false,
            streaming_output: false,
            streaming_id: None,
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions,
        clusters,
        timeout,
        search_type: Some(SearchEventType::UI),
        search_event_context: None,
        use_cache: None,
    };
    let search_res = SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
        .instrument(http_span)
        .await;

    let resp_backward = match search_res {
        Ok(res) => res,
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, &stream_name, "500", "_around");
            log::error!("search around error: {:?}", err);
            return Ok(match err {
                errors::Error::ErrorCode(code) => match code {
                    errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                        .json(meta::http::HttpResponse::error_code_with_trace_id(
                            code,
                            Some(trace_id),
                        )),
                    _ => HttpResponse::InternalServerError().json(
                        meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                    ),
                },
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            });
        }
    };

    // merge
    let mut resp = config::meta::search::Response::default();
    let hits_num = resp_backward.hits.len();
    for i in 0..hits_num {
        resp.hits
            .push(resp_backward.hits[hits_num - 1 - i].to_owned());
    }
    let hits_num = resp_forward.hits.len();
    for i in 0..hits_num {
        resp.hits.push(resp_forward.hits[i].to_owned());
    }
    resp.total = resp.hits.len();
    resp.size = around_size;
    resp.scan_size = resp_forward.scan_size + resp_backward.scan_size;
    resp.took = resp_forward.took + resp_backward.took;
    resp.cached_ratio = (resp_forward.cached_ratio + resp_backward.cached_ratio) / 2;

    let time = start.elapsed().as_secs_f64();
    http_report_metrics(start, &org_id, stream_type, &stream_name, "200", "_around");

    let req_stats = RequestStats {
        records: resp.hits.len() as i64,
        response_time: time,
        size: resp.scan_size as f64,
        request_body: Some(req.query.sql),
        user_email: user_id,
        min_ts: Some(around_start_time),
        max_ts: Some(around_end_time),
        cached_ratio: Some(resp.cached_ratio),
        trace_id: Some(trace_id),
        took_wait_in_queue: match (
            resp_forward.took_detail.as_ref(),
            resp_backward.took_detail.as_ref(),
        ) {
            (Some(forward_took), Some(backward_took)) => {
                Some(forward_took.cluster_wait_queue + backward_took.cluster_wait_queue)
            }
            (Some(forward_took), None) => Some(forward_took.cluster_wait_queue),
            (None, Some(backward_took)) => Some(backward_took.cluster_wait_queue),
            _ => None,
        },
        work_group: get_work_group(vec![
            resp_forward.work_group.clone(),
            resp_backward.work_group.clone(),
        ]),
        ..Default::default()
    };
    let num_fn = req.query.query_fn.is_some() as u16;
    report_request_usage_stats(
        req_stats,
        &org_id,
        &stream_name,
        StreamType::Logs,
        UsageType::SearchAround,
        num_fn,
        started_at,
    )
    .await;

    Ok(HttpResponse::Ok().json(resp))
}

/// SearchTopNValues
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchValues",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_name" = String, Path, description = "stream_name name"),
        ("fields" = String, Query, description = "fields, split by comma"),
        ("filter" = Option<String>, Query, description = "filter, eg: a=b"),
        ("keyword" = Option<String>, Query, description = "keyword, eg: abc"),
        ("size" = i64, Query, description = "size"), // topN
        ("start_time" = i64, Query, description = "start time"),
        ("end_time" = i64, Query, description = "end time"),
        ("regions" = Option<String>, Query, description = "regions, split by comma"),
        ("timeout" = Option<i64>, Query, description = "timeout, seconds"),
        ("no_count" = Option<bool>, Query, description = "no need count, true of false"),
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "values": [
                {
                    "field": "field1",
                    "values": ["value1", "value2"]
                }
            ]
        })),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[get("/{org_id}/{stream_name}/_values")]
pub async fn values(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let (org_id, stream_name) = path.into_inner();
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();

    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let http_span = if config::get_config().common.tracing_search_enabled {
        tracing::info_span!(
            "/api/{org_id}/{stream_name}/_values",
            org_id = org_id.clone(),
            stream_name = stream_name.clone()
        )
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);

    // originally there was v1 which would to a full stream search
    // and v2 which would do search on a distinct values stream iff
    // the queried fields configured accordingly.
    // Now we simply check if the fields in query are in the distinct stream or not,
    // and change the search stream to the distinct stream, so we don't need any separate
    // v2 fucntion.
    values_v1(
        &org_id,
        stream_type,
        &stream_name,
        &query,
        &user_id,
        trace_id,
        http_span,
    )
    .await
}

// If all fields requested in the query AND fields from the
// sql query in the query are stored in distinct stream,
// this will search on the distinct stream, otherwise
// just search on the original data
async fn values_v1(
    org_id: &str,
    stream_type: StreamType,
    stream_name: &str,
    query: &web::Query<HashMap<String, String>>,
    user_id: &str,
    trace_id: String,
    http_span: Span,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();
    let cfg = get_config();

    let mut uses_fn = false;
    let fields = match query.get("fields") {
        Some(v) => v.split(',').map(|s| s.to_string()).collect::<Vec<_>>(),
        None => return Ok(MetaHttpResponse::bad_request("fields is empty")),
    };
    let mut query_fn = query
        .get("query_fn")
        .and_then(|v| base64::decode_url(v).ok());
    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let default_sql = format!("SELECT {} FROM \"{stream_name}\"", TIMESTAMP_COL_NAME);
    let mut query_sql = match query.get("filter") {
        None => default_sql,
        Some(v) => {
            if v.is_empty() {
                default_sql
            } else {
                let columns = v.splitn(2, '=').collect::<Vec<_>>();
                if columns.len() < 2 {
                    return Ok(MetaHttpResponse::bad_request("Invalid filter format"));
                }
                let vals = columns[1].split(',').collect::<Vec<_>>().join("','");
                format!("{} WHERE {} IN ('{}')", default_sql, columns[0], vals)
            }
        }
    };

    let keyword = match query.get("keyword") {
        None => "".to_string(),
        Some(v) => v.trim().to_string(),
    };
    let no_count = match query.get("no_count") {
        None => false,
        Some(v) => {
            let v = v.to_lowercase();
            v == "true" || v == "1"
        }
    };

    if let Some(v) = query.get("sql") {
        if let Ok(sql) = base64::decode_url(v) {
            uses_fn = functions::get_all_transform_keys(org_id)
                .await
                .iter()
                .any(|fn_name| sql.contains(&format!("{}(", fn_name)));
            query_sql = sql;
        }
    };

    // pick up where clause from sql
    let where_str = match SearchService::sql::pickup_where(&query_sql, None) {
        Ok(v) => v.unwrap_or_default(),
        Err(e) => {
            return Err(Error::other(e));
        }
    };

    let size = query
        .get("size")
        .map_or(10, |v| v.parse::<i64>().unwrap_or(10));
    // If this is a enrichment table, we need to get the start_time and end_time from the stats
    let stats = if stream_type.eq(&StreamType::EnrichmentTables) {
        Some(stats::get_stream_stats(org_id, stream_name, stream_type))
    } else {
        None
    };
    let start_time = if stream_type.eq(&StreamType::EnrichmentTables) {
        stats.as_ref().unwrap().doc_time_min
    } else {
        query
            .get("start_time")
            .map_or(0, |v| v.parse::<i64>().unwrap_or(0))
    };

    if start_time == 0 {
        return Ok(MetaHttpResponse::bad_request("start_time is empty"));
    }
    let end_time = if stream_type.eq(&StreamType::EnrichmentTables) {
        stats.as_ref().unwrap().doc_time_max
    } else {
        query
            .get("end_time")
            .map_or(0, |v| v.parse::<i64>().unwrap_or(0))
    };
    if end_time == 0 {
        return Ok(MetaHttpResponse::bad_request("end_time is empty"));
    }
    let (start_time, end_time) = if start_time == end_time {
        (start_time - 1, end_time + 1)
    } else {
        (start_time, end_time)
    };

    // check if we can use the distinct stream for this query
    let use_distinct_stream = can_use_distinct_stream(
        org_id,
        stream_name,
        stream_type,
        &fields,
        &query_sql,
        start_time,
    )
    .await;

    let regions = query.get("regions").map_or(vec![], |regions| {
        regions
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });
    let clusters = query.get("clusters").map_or(vec![], |clusters| {
        clusters
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect::<Vec<_>>()
    });

    let timeout = query
        .get("timeout")
        .map_or(0, |v| v.parse::<i64>().unwrap_or(0));

    // search
    let use_cache = cfg.common.result_cache_enabled && get_use_cache_from_request(query);
    let req = config::meta::search::Request {
        query: config::meta::search::Query {
            sql: query_sql,
            from: 0,
            size: config::meta::sql::MAX_LIMIT,
            start_time,
            end_time,
            uses_zo_fn: uses_fn,
            query_fn: query_fn.clone(),
            ..Default::default()
        },
        encoding: config::meta::search::RequestEncoding::Empty,
        regions,
        clusters,
        timeout,
        search_type: Some(SearchEventType::Values),
        search_event_context: None,
        use_cache: Some(use_cache),
    };

    // skip fields which aren't part of the schema
    let schema = infra::schema::get(org_id, stream_name, stream_type)
        .await
        .unwrap_or(Schema::empty());

    let mut query_results = Vec::with_capacity(fields.len());
    let sql_where = if where_str.is_empty() {
        "".to_string()
    } else {
        format!("WHERE {}", where_str)
    };
    for field in &fields {
        let http_span = http_span.clone();
        // skip values for field which aren't part of the schema
        if schema.field_with_name(field).is_err() {
            continue;
        }
        let sql_where = if !sql_where.is_empty() && !keyword.is_empty() {
            format!("{sql_where} AND {field} ILIKE '%{keyword}%'")
        } else if !keyword.is_empty() {
            format!("WHERE {field} ILIKE '%{keyword}%'")
        } else {
            sql_where.clone()
        };

        let distinct_prefix;
        let count_fn;
        let actual_stream_type;

        if use_distinct_stream {
            distinct_prefix = format!("{}_{}_", DISTINCT_STREAM_PREFIX, stream_type.as_str());
            // if we are using distinct stream, we have already partially aggregated
            // the counts, so we need to sum over that field
            count_fn = "SUM(count)";
            // distinct_values_* stream is metadata
            actual_stream_type = StreamType::Metadata;
        } else {
            distinct_prefix = "".to_owned();
            // for non-distinct fields, we need the actual count
            count_fn = "COUNT(*)";
            actual_stream_type = stream_type;
        }

        let sql = if no_count {
            format!(
                "SELECT histogram(_timestamp) AS zo_sql_time, \"{field}\" AS zo_sql_key FROM \"{distinct_prefix}{stream_name}\" {sql_where} GROUP BY zo_sql_time, zo_sql_key ORDER BY zo_sql_time ASC, zo_sql_key ASC"
            )
        } else {
            format!(
                "SELECT histogram(_timestamp) AS zo_sql_time, \"{field}\" AS zo_sql_key, {count_fn} AS zo_sql_num FROM \"{distinct_prefix}{stream_name}\" {sql_where} GROUP BY zo_sql_time, zo_sql_key ORDER BY zo_sql_time ASC, zo_sql_num DESC"
            )
        };
        let mut req = req.clone();
        req.query.sql = sql;

        let search_res = SearchService::cache::search(
            &trace_id,
            org_id,
            actual_stream_type,
            Some(user_id.to_string()),
            &req,
            "".to_string(),
        )
        .instrument(http_span)
        .await;
        let resp_search = match search_res {
            Ok(res) => res,
            Err(err) => {
                http_report_metrics(start, org_id, stream_type, stream_name, "500", "_values/v1");
                log::error!("search values error: {:?}", err);
                return Ok(match err {
                    errors::Error::ErrorCode(code) => match code {
                        errors::ErrorCodes::SearchCancelQuery(_) => HttpResponse::TooManyRequests()
                            .json(meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            )),
                        _ => HttpResponse::InternalServerError().json(
                            meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            ),
                        ),
                    },
                    _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        err.to_string(),
                    )),
                });
            }
        };
        query_results.push((field.to_string(), resp_search));
    }

    let mut resp = config::meta::search::Response::default();
    let mut hit_values: Vec<json::Value> = Vec::new();
    let mut work_group_set = Vec::with_capacity(query_results.len());
    for (key, ret) in query_results {
        let mut top_hits: HashMap<String, i64> = HashMap::default();
        for row in ret.hits {
            let key = row
                .get("zo_sql_key")
                .map(json::get_string_value)
                .unwrap_or("".to_string());
            let num = row
                .get("zo_sql_num")
                .map(|v| v.as_i64().unwrap_or(0))
                .unwrap_or(0);
            let key_num = top_hits.entry(key).or_insert(0);
            *key_num += num;
        }
        let mut top_hits = top_hits.into_iter().collect::<Vec<_>>();
        if no_count {
            top_hits.sort_by(|a, b| a.0.cmp(&b.0));
        } else {
            top_hits.sort_by(|a, b| b.1.cmp(&a.1));
        }
        let top_hits = top_hits
            .into_iter()
            .take(size as usize)
            .map(|(k, v)| {
                let mut item = json::Map::new();
                item.insert("zo_sql_key".to_string(), json::Value::String(k));
                item.insert("zo_sql_num".to_string(), json::Value::Number(v.into()));
                json::Value::Object(item)
            })
            .collect::<Vec<_>>();

        let mut field_value: json::Map<String, json::Value> = json::Map::new();
        field_value.insert("field".to_string(), json::Value::String(key));
        field_value.insert("values".to_string(), json::Value::Array(top_hits));
        hit_values.push(json::Value::Object(field_value));
        resp.scan_size = std::cmp::max(resp.scan_size, ret.scan_size);
        resp.scan_records = std::cmp::max(resp.scan_records, ret.scan_records);
        resp.cached_ratio = std::cmp::max(resp.cached_ratio, ret.cached_ratio);
        resp.result_cache_ratio = std::cmp::max(resp.result_cache_ratio, ret.result_cache_ratio);
        work_group_set.push(ret.work_group);
    }
    resp.total = fields.len();
    resp.hits = hit_values;
    resp.size = size;
    resp.took = start.elapsed().as_millis() as usize;

    let time = start.elapsed().as_secs_f64();
    http_report_metrics(start, org_id, stream_type, stream_name, "200", "_values/v1");

    let req_stats = RequestStats {
        records: resp.hits.len() as i64,
        response_time: time,
        size: resp.scan_size as f64,
        request_body: Some(req.query.sql),
        user_email: Some(user_id.to_string()),
        min_ts: Some(start_time),
        max_ts: Some(end_time),
        cached_ratio: Some(resp.cached_ratio),
        search_type: Some(SearchEventType::Values),
        trace_id: Some(trace_id),
        took_wait_in_queue: if resp.took_detail.is_some() {
            let resp_took = resp.took_detail.as_ref().unwrap();
            // Consider only the cluster wait queue duration
            Some(resp_took.cluster_wait_queue)
        } else {
            None
        },
        work_group: get_work_group(work_group_set),
        ..Default::default()
    };
    let num_fn = req.query.query_fn.is_some() as u16;
    report_request_usage_stats(
        req_stats,
        org_id,
        stream_name,
        stream_type,
        UsageType::SearchTopNValues,
        num_fn,
        started_at,
    )
    .await;

    Ok(HttpResponse::Ok().json(resp))
}

/// SearchStreamPartition
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchPartition",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
    ),
    request_body(content = SearchRequest, description = "Search query", content_type = "application/json", example = json!({
        "sql": "select * from k8s ",
        "start_time": 1675182660872049i64,
        "end_time": 1675185660872049i64
    })),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json!({
            "took": 155,
            "file_num": 10,
            "original_size": 10240,
            "compressed_size": 1024,
            "partitions": [
                [1674213225158000i64, 1674213225158000i64],
                [1674213225158000i64, 1674213225158000i64],
            ]
        })),
        (status = 400, description = "Failure", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Failure", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/_search_partition")]
pub async fn search_partition(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!("/api/{org_id}/_search_partition", org_id = org_id.clone(),)
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);

    let org_id = org_id.into_inner();
    let user_id = in_req
        .headers()
        .get("user_id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let mut req: config::meta::search::SearchPartitionRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => return Ok(MetaHttpResponse::bad_request(e)),
    };
    if let Err(e) = req.decode() {
        return Ok(MetaHttpResponse::bad_request(e));
    }

    let search_res = SearchService::search_partition(
        &trace_id,
        &org_id,
        Some(&user_id),
        stream_type,
        &req,
        false,
    )
    .instrument(http_span)
    .await;

    // do search
    match search_res {
        Ok(res) => {
            http_report_metrics(start, &org_id, stream_type, "", "200", "_search_partition");
            Ok(HttpResponse::Ok().json(res))
        }
        Err(err) => {
            http_report_metrics(start, &org_id, stream_type, "", "500", "_search_partition");
            log::error!("search error: {:?}", err);
            Ok(match err {
                errors::Error::ErrorCode(code) => HttpResponse::InternalServerError().json(
                    meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                ),
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            })
        }
    }
}

/// Search History
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchHistory",
    security(
        ("Authorization" = [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization ID"),
    ),
    request_body(
        content = SearchHistoryRequest,
        description = "Search history request parameters",
        content_type = "application/json",
        example = json!({
            "stream_name": "default",
            "stream_type": "logs",
            "min_ts": 1632960000,
            "max_ts": 1633046400,
            "trace_id": "7f7898fd19424c47ba830a6fa9b25e1f",
            "size": 100
        })
    ),
    responses(
        (status = 200, description = "Success", content_type = "application/json", body = SearchResponse, example = json ! ({
            "took": 40,
            "took_detail": {
                "total": 0,
                "idx_took": 0,
                "wait_queue": 0,
                "cluster_total": 40,
                "cluster_wait_queue": 0
            },
            "hits": [
                {
                "cached_ratio": 0,
                "end_time": 15,
                "org_id": "default",
                "scan_records": 1,
                "scan_size": 7.0,
                "sql": "SELECT COUNT(*) from \"default\"",
                "start_time": 0,
                "stream_name": "default",
                "stream_type": "logs",
                "took": 0.056222333,
                "trace_id": "7f7898fd19424c47ba830a6fa9b25e1f",
                "function": ".",
                },
            ],
            "total": 3,
            "from": 0,
            "size": 20,
            "cached_ratio": 0,
            "scan_size": 0,
            "idx_scan_size": 0,
            "scan_records": 3,
            "trace_id": "2lsPBWjwZxUJ5ugvZ4jApESZEpk",
            "is_partial": false,
            "result_cache_ratio": 0
        })),
        (status = 400, description = "Bad Request - Invalid parameters or body", content_type = "application/json", body = HttpResponse),
        (status = 500, description = "Internal Server Error", content_type = "application/json", body = HttpResponse),
    )
)]
#[post("/{org_id}/_search_history")]
pub async fn search_history(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();
    let org_id = org_id.into_inner();
    let cfg = get_config();
    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!("/api/{org_id}/_search_history", org_id = org_id.clone())
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);
    let user_id = in_req
        .headers()
        .get("user_id")
        .map(|v| v.to_str().unwrap_or("").to_string());

    let mut req: config::meta::search::SearchHistoryRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    // restrict history only to path org_id
    req.org_id = Some(org_id.clone());
    // restrict history only to requested user_id
    req.user_email = user_id.clone();

    // Search
    let stream_name = USAGE_STREAM;
    let search_query_req = match req.to_query_req(stream_name) {
        Ok(r) => r,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };

    // increment query queue
    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .inc();

    // handle search queue lock and timing
    #[cfg(not(feature = "enterprise"))]
    let locker = SearchService::QUEUE_LOCKER.clone();
    #[cfg(not(feature = "enterprise"))]
    let locker = locker.lock().await;
    #[cfg(not(feature = "enterprise"))]
    if !cfg.common.feature_query_queue_enabled {
        drop(locker);
    }
    #[cfg(not(feature = "enterprise"))]
    let took_wait = start.elapsed().as_millis() as usize;
    #[cfg(feature = "enterprise")]
    let took_wait = 0;

    log::info!(
        "http search history API wait in queue took: {} ms",
        took_wait
    );

    metrics::QUERY_PENDING_NUMS
        .with_label_values(&[&org_id])
        .dec();

    let history_org_id = &cfg.common.usage_org;
    let stream_type = StreamType::Logs;
    let search_res = SearchService::search(
        &trace_id,
        history_org_id,
        stream_type,
        user_id.clone(),
        &search_query_req,
    )
    .instrument(http_span)
    .await;

    let mut search_res = match search_res {
        Ok(res) => res,
        Err(err) => {
            http_report_metrics(
                start,
                &org_id,
                stream_type,
                stream_name,
                "500",
                "_search_history",
            );
            log::error!("[trace_id {}] Search history error : {:?}", trace_id, err);
            return Ok(match err {
                errors::Error::ErrorCode(code) => HttpResponse::InternalServerError().json(
                    meta::http::HttpResponse::error_code_with_trace_id(code, Some(trace_id)),
                ),
                _ => HttpResponse::InternalServerError().json(meta::http::HttpResponse::error(
                    StatusCode::INTERNAL_SERVER_ERROR.into(),
                    err.to_string(),
                )),
            });
        }
    };

    search_res.hits = search_res
        .hits
        .into_iter()
        .filter_map(|hit| match SearchHistoryHitResponse::try_from(hit) {
            Ok(response) => match serde_json::to_value(response) {
                Ok(json_value) => Some(json_value),
                Err(e) => {
                    log::error!("[trace_id {}] Serialization error: {:?}", trace_id, e);
                    None
                }
            },
            Err(e) => {
                log::error!("[trace_id {}] Deserialization error: {:?}", trace_id, e);
                None
            }
        })
        .collect::<Vec<_>>();

    search_res.trace_id = trace_id.clone();

    // report http metrics
    http_report_metrics(
        start,
        &org_id,
        stream_type,
        stream_name,
        "200",
        "_search_history",
    );

    // prepare usage metrics
    let time_taken = start.elapsed().as_secs_f64();
    let took_wait_in_queue = if search_res.took_detail.is_some() {
        let resp_took = search_res.took_detail.as_ref().unwrap();
        Some(resp_took.cluster_wait_queue)
    } else {
        None
    };
    let req_stats = RequestStats {
        records: search_res.hits.len() as i64,
        response_time: time_taken,
        size: search_res.scan_size as f64,
        request_body: Some(search_query_req.query.sql),
        user_email: user_id,
        min_ts: Some(req.start_time),
        max_ts: Some(req.end_time),
        cached_ratio: Some(search_res.cached_ratio),
        search_type: Some(SearchEventType::Other),
        trace_id: Some(trace_id),
        took_wait_in_queue,
        work_group: search_res.work_group.clone(),
        ..Default::default()
    };
    let num_fn = search_query_req.query.query_fn.is_some() as u16;
    report_request_usage_stats(
        req_stats,
        history_org_id,
        stream_name,
        StreamType::Logs,
        UsageType::SearchHistory,
        num_fn,
        started_at,
    )
    .await;

    Ok(HttpResponse::Ok().json(search_res))
}
