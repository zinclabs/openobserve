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
use chrono::{Duration, Utc};
use config::{
    get_config,
    meta::{
        function::VRLResultResolver,
        search::{self, PARTIAL_ERROR_RESPONSE_MESSAGE},
        self_reporting::usage::{RequestStats, UsageType},
        sql::resolve_stream_names,
        stream::StreamType,
    },
    metrics,
    utils::{base64, json},
    TIMESTAMP_COL_NAME,
};
use infra::errors;
use tracing::{Instrument, Span};

#[cfg(feature = "enterprise")]
use crate::service::search::sql::get_cipher_key_names;
use crate::{
    common::{
        meta::{self, http::HttpResponse as MetaHttpResponse},
        utils::{
            functions,
            http::{
                get_or_create_trace_id, get_search_event_context_from_request,
                get_search_type_from_request, get_stream_type_from_request, get_work_group,
            },
            stream::get_settings_max_query_range,
        },
    },
    service::{search as SearchService, self_reporting::report_request_usage_stats},
};

/// SearchStreamData
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchSQL",
    params(("org_id" = String, Path, description = "Organization name")),
    request_body(
        content = SearchRequest,
        description = "Search query",
        content_type = "application/json",
        example = json!({
        "query": {
            "sql": "select * from k8s ",
            "start_time": 1675182660872049i64,
            "end_time": 1675185660872049i64,
            "from": 0,
            "size": 10
        }
    })
    ),
    responses(
        (
            status = 200,
            description = "Success",
            content_type = "application/json",
            body = SearchResponse,
            example = json!({
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
        }),
        ),
        (
            status = 400,
            description = "Failure",
            content_type = "application/json",
            body = HttpResponse,
        ),
        (
            status = 500,
            description = "Failure",
            content_type = "application/json",
            body = HttpResponse,
        )
    )
)]
#[post("/{org_id}/_search_multi")]
pub async fn search_multi(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let org_id = org_id.into_inner();
    let started_at = Utc::now().timestamp_micros();
    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!("/api/{org_id}/_search_multi", org_id = org_id.clone())
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);

    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let search_type = match get_search_type_from_request(&query) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };
    let search_event_context = search_type
        .as_ref()
        .and_then(|event_type| get_search_event_context_from_request(event_type, &query));

    // handle encoding for query and aggs
    let multi_req: search::MultiStreamRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };

    let mut query_fn = multi_req
        .query_fn
        .as_ref()
        .and_then(|v| base64::decode_url(v).ok());

    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let mut range_error = String::new();

    let user_id = in_req.headers().get("user_id").unwrap().to_str().unwrap();
    let mut queries = multi_req.to_query_req();
    let mut multi_res = search::Response::new(multi_req.from, multi_req.size);

    let per_query_resp = multi_req.per_query_response;

    // Before making any rpc requests, first check the sql expressions can be decoded correctly
    for req in queries.iter_mut() {
        if let Err(e) = req.decode() {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    }
    let queries_len = queries.len();
    let mut vrl_stream_name = "".to_string();
    let mut sqls = vec![];

    for mut req in queries {
        sqls.push(req.query.sql.clone());
        let stream_name = match resolve_stream_names(&req.query.sql) {
            Ok(v) => v[0].clone(),
            Err(e) => {
                return Ok(HttpResponse::InternalServerError().json(
                    meta::http::HttpResponse::error(
                        StatusCode::INTERNAL_SERVER_ERROR.into(),
                        e.to_string(),
                    ),
                ));
            }
        };
        vrl_stream_name = stream_name.clone();

        // get stream settings
        if let Some(settings) =
            infra::schema::get_settings(&org_id, &stream_name, stream_type).await
        {
            let max_query_range =
                get_settings_max_query_range(settings.max_query_range, &org_id, Some(user_id))
                    .await;
            if max_query_range > 0
                && (req.query.end_time - req.query.start_time) > max_query_range * 3600 * 1_000_000
            {
                req.query.start_time = req.query.end_time - max_query_range * 3600 * 1_000_000;
                range_error = format!(
                    "{} Query duration for stream {} is modified due to query range restriction of {} hours",
                    range_error, &stream_name, max_query_range
                );

                if multi_res.new_start_time.is_none() {
                    multi_res.new_start_time = Some(req.query.start_time);
                    multi_res.new_end_time = Some(req.query.end_time);
                }
            }
        }

        // Check permissions on stream
        #[cfg(feature = "enterprise")]
        {
            use o2_openfga::meta::mapping::OFGA_MODELS;

            use crate::common::{
                infra::config::USERS,
                utils::auth::{is_root_user, AuthExtractor},
            };

            if !is_root_user(user_id) {
                let user: meta::user::User =
                    USERS.get(&format!("{org_id}/{user_id}")).unwrap().clone();
                let stream_type_str = stream_type.as_str();

                if !crate::handler::http::auth::validator::check_permissions(
                    user_id,
                    AuthExtractor {
                        auth: "".to_string(),
                        method: "GET".to_string(),
                        o2_type: format!(
                            "{}:{}",
                            OFGA_MODELS
                                .get(stream_type_str)
                                .map_or(stream_type_str, |model| model.key),
                            stream_name
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
                    return Ok(MetaHttpResponse::forbidden("Unauthorized Access"));
                }
            }

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
            // Check permissions on stream ends
            // Check permissions on keys
            for key in keys_used {
                if !is_root_user(user_id) {
                    let user: meta::user::User =
                        USERS.get(&format!("{org_id}/{}", user_id)).unwrap().clone();

                    if !crate::handler::http::auth::validator::check_permissions(
                        user_id,
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

        if !per_query_resp {
            req.query.query_fn = query_fn.clone();
        }
        for fn_name in functions::get_all_transform_keys(&org_id).await {
            if req.query.sql.contains(&format!("{}(", fn_name)) {
                req.query.uses_zo_fn = true;
                break;
            }
        }

        // add search type to request
        req.search_type = search_type;

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
        log::info!("http search multi API wait in queue took: {}", took_wait);
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&org_id])
            .dec();

        let trace_id = trace_id.clone();
        // do search
        let search_res = SearchService::search(
            &trace_id,
            &org_id,
            stream_type,
            Some(user_id.to_string()),
            &req,
        )
        .instrument(http_span.clone())
        .await;

        match search_res {
            Ok(mut res) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_search_multi",
                        "200",
                        &org_id,
                        "",
                        stream_type.as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_search_multi",
                        "200",
                        &org_id,
                        "",
                        stream_type.as_str(),
                    ])
                    .inc();
                res.set_trace_id(trace_id);
                res.set_local_took(start.elapsed().as_millis() as usize, took_wait);

                let req_stats = RequestStats {
                    records: res.hits.len() as i64,
                    response_time: time,
                    size: res.scan_size as f64,
                    request_body: Some(req.query.sql),
                    user_email: Some(user_id.to_string()),
                    min_ts: Some(req.query.start_time),
                    max_ts: Some(req.query.end_time),
                    cached_ratio: Some(res.cached_ratio),
                    search_type,
                    search_event_context: search_event_context.clone(),
                    trace_id: Some(res.trace_id.clone()),
                    took_wait_in_queue: if res.took_detail.is_some() {
                        let resp_took = res.took_detail.as_ref().unwrap();
                        // Consider only the cluster wait queue duration
                        Some(resp_took.cluster_wait_queue)
                    } else {
                        None
                    },
                    work_group: res.work_group,
                    ..Default::default()
                };
                let num_fn = req.query.query_fn.is_some() as u16;

                report_request_usage_stats(
                    req_stats,
                    &org_id,
                    &stream_name,
                    StreamType::Logs,
                    UsageType::Search,
                    num_fn,
                    started_at,
                )
                .await;

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

                if per_query_resp {
                    multi_res.hits.push(serde_json::Value::Array(res.hits));
                } else {
                    multi_res.hits.extend(res.hits);
                }

                if res.is_partial {
                    multi_res.is_partial = true;
                    multi_res.function_error = if res.function_error.is_empty() {
                        PARTIAL_ERROR_RESPONSE_MESSAGE.to_string()
                    } else {
                        format!(
                            "{} \n {}",
                            PARTIAL_ERROR_RESPONSE_MESSAGE, res.function_error
                        )
                    };
                }
                if multi_res.histogram_interval.is_none() && res.histogram_interval.is_some() {
                    multi_res.histogram_interval = res.histogram_interval;
                }
            }
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_search_multi",
                        "500",
                        &org_id,
                        "",
                        stream_type.as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_search_multi",
                        "500",
                        &org_id,
                        "",
                        stream_type.as_str(),
                    ])
                    .inc();

                log::error!("search error: {:?}", err);
                multi_res.function_error = format!("{};{:?}", multi_res.function_error, err);
                if let errors::Error::ErrorCode(code) = err {
                    if let errors::ErrorCodes::SearchCancelQuery(_) = code {
                        return Ok(HttpResponse::TooManyRequests().json(
                            meta::http::HttpResponse::error_code_with_trace_id(
                                code,
                                Some(trace_id),
                            ),
                        ));
                    }
                }
            }
        }
    }

    let mut report_function_usage = false;
    multi_res.hits = if query_fn.is_some() && per_query_resp {
        // compile vrl function & apply the same before returning the response
        let mut input_fn = query_fn.unwrap().trim().to_string();

        let apply_over_hits = SearchService::RESULT_ARRAY.is_match(&input_fn);
        if apply_over_hits {
            input_fn = SearchService::RESULT_ARRAY
                .replace(&input_fn, "")
                .to_string();
        }
        let mut runtime = crate::common::utils::functions::init_vrl_runtime();
        let program = match crate::service::ingestion::compile_vrl_function(&input_fn, &org_id) {
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
                    let (ret_val, _) = crate::service::ingestion::apply_vrl_fn(
                        &mut runtime,
                        &VRLResultResolver {
                            program: program.program.clone(),
                            fields: program.fields.clone(),
                        },
                        json::Value::Array(multi_res.hits),
                        &org_id,
                        &[vrl_stream_name.clone()],
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
                            let (ret_val, _) = crate::service::ingestion::apply_vrl_fn(
                                &mut runtime,
                                &VRLResultResolver {
                                    program: program.program.clone(),
                                    fields: program.fields.clone(),
                                },
                                hit,
                                &org_id,
                                &[vrl_stream_name.clone()],
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

    if !range_error.is_empty() {
        multi_res.is_partial = true;
        multi_res.function_error = if multi_res.function_error.is_empty() {
            range_error
        } else {
            format!("{} \n {}", range_error, multi_res.function_error)
        };
    }

    let column_timestamp = TIMESTAMP_COL_NAME.to_string();
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
            search_event_context: multi_req.search_event_context.clone(),
            ..Default::default()
        };
        report_request_usage_stats(
            req_stats,
            &org_id,
            &vrl_stream_name,
            stream_type,
            UsageType::Functions,
            0, // The request stats already contains function event
            started_at,
        )
        .await;
    }
    Ok(HttpResponse::Ok().json(multi_res))
}

/// SearchMultiStreamPartition
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchPartitionMulti",
    params(("org_id" = String, Path, description = "Organization name")),
    request_body(
        content = SearchRequest,
        description = "Search query",
        content_type = "application/json",
        example = json!({
        "sql": "select * from k8s ",
        "start_time": 1675182660872049i64,
        "end_time": 1675185660872049i64
    })
    ),
    responses(
        (
            status = 200,
            description = "Success",
            content_type = "application/json",
            body = SearchResponse,
            example = json!({
            "took": 155,
            "file_num": 10,
            "original_size": 10240,
            "compressed_size": 1024,
            "partitions": [
                [1674213225158000i64, 1674213225158000i64],
                [1674213225158000i64, 1674213225158000i64],
            ]
        }),
        ),
        (
            status = 400,
            description = "Failure",
            content_type = "application/json",
            body = HttpResponse,
        ),
        (
            status = 500,
            description = "Failure",
            content_type = "application/json",
            body = HttpResponse,
        )
    )
)]
#[post("/{org_id}/_search_partition_multi")]
pub async fn _search_partition_multi(
    org_id: web::Path<String>,
    in_req: HttpRequest,
    body: web::Bytes,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let cfg = get_config();

    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!(
            "/api/{org_id}/_search_partition_multi",
            org_id = org_id.clone()
        )
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

    let req: search::MultiSearchPartitionRequest = match json::from_slice(&body) {
        Ok(v) => v,
        Err(e) => {
            return Ok(MetaHttpResponse::bad_request(e));
        }
    };

    let search_fut =
        SearchService::search_partition_multi(&trace_id, &org_id, &user_id, stream_type, &req);
    let search_res = if !cfg.common.tracing_enabled && cfg.common.tracing_search_enabled {
        search_fut.instrument(http_span).await
    } else {
        search_fut.await
    };
    // do search
    match search_res {
        Ok(res) => {
            let time = start.elapsed().as_secs_f64();
            metrics::HTTP_RESPONSE_TIME
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "200",
                    &org_id,
                    "",
                    stream_type.as_str(),
                ])
                .observe(time);
            metrics::HTTP_INCOMING_REQUESTS
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "200",
                    &org_id,
                    "",
                    stream_type.as_str(),
                ])
                .inc();
            Ok(HttpResponse::Ok().json(res))
        }
        Err(err) => {
            let time = start.elapsed().as_secs_f64();
            metrics::HTTP_RESPONSE_TIME
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "500",
                    &org_id,
                    "",
                    stream_type.as_str(),
                ])
                .observe(time);
            metrics::HTTP_INCOMING_REQUESTS
                .with_label_values(&[
                    "/api/org/_search_partition_multi",
                    "500",
                    &org_id,
                    "",
                    stream_type.as_str(),
                ])
                .inc();
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

/// SearchAround
#[utoipa::path(
    context_path = "/api",
    tag = "Search",
    operation_id = "SearchAroundMulti",
    security(
        ("Authorization"= [])
    ),
    params(
        ("org_id" = String, Path, description = "Organization name"),
        ("stream_names" = String, Path, description = "base64 encoded comma separated stream names"),
        ("key" = i64, Query, description = "around key"),
        ("size" = i64, Query, description = "around size"),
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
#[get("/{org_id}/{stream_names}/_around_multi")]
pub async fn around_multi(
    path: web::Path<(String, String)>,
    in_req: HttpRequest,
) -> Result<HttpResponse, Error> {
    let start = std::time::Instant::now();
    let started_at = Utc::now().timestamp_micros();
    let cfg = get_config();

    let (org_id, stream_names) = path.into_inner();
    let http_span = if cfg.common.tracing_search_enabled {
        tracing::info_span!(
            "/api/{org_id}/{stream_names}/_around_multi",
            org_id = org_id.clone(),
            stream_names = stream_names.clone()
        )
    } else {
        Span::none()
    };
    let trace_id = get_or_create_trace_id(in_req.headers(), &http_span);

    let stream_names = base64::decode_url(&stream_names)?;
    let mut uses_fn = false;
    let query = web::Query::<HashMap<String, String>>::from_query(in_req.query_string()).unwrap();
    let stream_type = get_stream_type_from_request(&query).unwrap_or_default();

    let around_key = match query.get("key") {
        Some(v) => v.parse::<i64>().unwrap_or(0),
        None => {
            return Ok(MetaHttpResponse::bad_request("around key is empty"));
        }
    };
    let mut query_fn = query
        .get("query_fn")
        .and_then(|v| base64::decode_url(v).ok());
    if let Some(vrl_function) = &query_fn {
        if !vrl_function.trim().ends_with('.') {
            query_fn = Some(format!("{} \n .", vrl_function));
        }
    }

    let mut around_sqls = stream_names
        .split(',')
        .collect::<Vec<&str>>()
        .iter()
        .map(|name| format!("SELECT * FROM \"{}\" ", name))
        .collect::<Vec<String>>();
    if let Some(v) = query.get("sql") {
        let sqls = v.split(',').collect::<Vec<&str>>();
        for (i, sql) in sqls.into_iter().enumerate() {
            uses_fn = functions::get_all_transform_keys(&org_id)
                .await
                .iter()
                .any(|fn_name| v.contains(&format!("{}(", fn_name)));
            if uses_fn {
                if let Ok(sql) = base64::decode_url(sql) {
                    around_sqls[i] = sql;
                }
            }
        }
    }

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

    let mut multi_resp = search::Response {
        size: around_size,
        ..Default::default()
    };

    let user_id = in_req
        .headers()
        .get("user_id")
        .unwrap()
        .to_str()
        .ok()
        .map(|v| v.to_string());

    for around_sql in around_sqls.iter() {
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
            "http search around multi API wait in queue took: {}",
            took_wait
        );
        metrics::QUERY_PENDING_NUMS
            .with_label_values(&[&org_id])
            .dec();

        // search forward
        let fw_sql = SearchService::sql::check_or_add_order_by_timestamp(around_sql, false)
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
            search_type: Some(search::SearchEventType::UI),
            search_event_context: None,
            use_cache: None,
        };
        let search_res =
            SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
                .instrument(http_span.clone())
                .await;

        let resp_forward = match search_res {
            Ok(res) => res,
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_around_multi",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_around_multi",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.as_str(),
                    ])
                    .inc();
                log::error!("multi search around error: {:?}", err);
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

        // search backward
        let bw_sql = SearchService::sql::check_or_add_order_by_timestamp(around_sql, true)
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
            regions: regions.clone(),
            clusters: clusters.clone(),
            timeout,
            search_type: Some(search::SearchEventType::UI),
            search_event_context: None,
            use_cache: None,
        };
        let search_res =
            SearchService::search(&trace_id, &org_id, stream_type, user_id.clone(), &req)
                .instrument(http_span.clone())
                .await;

        let resp_backward = match search_res {
            Ok(res) => res,
            Err(err) => {
                let time = start.elapsed().as_secs_f64();
                metrics::HTTP_RESPONSE_TIME
                    .with_label_values(&[
                        "/api/org/_around_multi",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.as_str(),
                    ])
                    .observe(time);
                metrics::HTTP_INCOMING_REQUESTS
                    .with_label_values(&[
                        "/api/org/_around_multi",
                        "500",
                        &org_id,
                        &stream_names,
                        stream_type.as_str(),
                    ])
                    .inc();
                log::error!("multi search around error: {:?}", err);
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

        let hits_num_backward = resp_backward.hits.len();
        for i in 0..hits_num_backward {
            multi_resp
                .hits
                .push(resp_backward.hits[hits_num_backward - 1 - i].to_owned());
        }
        let hits_num_forward = resp_forward.hits.len();
        for i in 0..hits_num_forward {
            multi_resp.hits.push(resp_forward.hits[i].to_owned());
        }
        let total_hits = hits_num_forward + hits_num_backward;
        let total_scan_size = resp_forward.scan_size + resp_backward.scan_size;
        multi_resp.total += total_hits;
        multi_resp.scan_size += total_scan_size;
        multi_resp.took += resp_forward.took + resp_backward.took;
        let cached_ratio_avg = (resp_forward.cached_ratio + resp_backward.cached_ratio) / 2;

        let time = start.elapsed().as_secs_f64();
        metrics::HTTP_RESPONSE_TIME
            .with_label_values(&[
                "/api/org/_around_multi",
                "200",
                &org_id,
                &stream_names,
                stream_type.as_str(),
            ])
            .observe(time);
        metrics::HTTP_INCOMING_REQUESTS
            .with_label_values(&[
                "/api/org/_around_multi",
                "200",
                &org_id,
                &stream_names,
                stream_type.as_str(),
            ])
            .inc();

        let user_id = match &user_id {
            Some(v) => v,
            None => "",
        };
        let req_stats = RequestStats {
            records: total_hits as i64,
            response_time: time,
            size: multi_resp.scan_size as f64,
            request_body: Some(around_sql.to_string()),
            user_email: Some(user_id.to_string()),
            min_ts: Some(around_start_time),
            max_ts: Some(around_end_time),
            cached_ratio: Some(cached_ratio_avg),
            trace_id: Some(trace_id.clone()),
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
        let num_fn = query_fn.is_some() as u16;
        report_request_usage_stats(
            req_stats,
            &org_id,
            &stream_names,
            StreamType::Logs,
            UsageType::SearchAround,
            num_fn,
            started_at,
        )
        .await;
    }

    multi_resp.hits.sort_by(|a, b| {
        let a_ts = a.get("_timestamp").unwrap().as_i64().unwrap();
        let b_ts = b.get("_timestamp").unwrap().as_i64().unwrap();
        b_ts.cmp(&a_ts)
    });
    Ok(HttpResponse::Ok().json(multi_resp))
}
