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

use actix_ws::Session;
use config::{
    get_config,
    meta::{
        search::{Response, SearchPartitionRequest, SearchPartitionResponse},
        sql::{resolve_stream_names, OrderBy},
        stream::StreamType,
        websocket::{SearchEventReq, SearchResultType},
    },
    utils::sql::is_aggregate_query,
};
use infra::errors::Error;
use proto::cluster_rpc::SearchQuery;
use tracing::Instrument;

#[allow(unused_imports)]
use crate::handler::http::request::websocket::utils::enterprise_utils;
use crate::{
    common::{
        meta::search::{CachedQueryResponse, MultiCachedQueryResponse, QueryDelta},
        utils::websocket::{
            calc_queried_range, calc_result_cache_ratio, get_search_type_from_ws_req,
            update_histogram_interval_in_query,
        },
    },
    handler::http::request::websocket::{session::send_message, utils::WsServerEvents},
    service::search::{
        cache,
        cache::cacher::get_ts_col_order_by,
        sql::Sql,
        {self as SearchService},
    },
};

#[cfg(feature = "enterprise")]
pub async fn handle_cancel(trace_id: &str, org_id: &str) -> WsServerEvents {
    match crate::service::search::cancel_query(org_id, trace_id).await {
        Ok(ret) => {
            log::info!(
                "[WS_HANDLER]: Cancel search for trace_id: {}, is_success: {}",
                ret.trace_id,
                ret.is_success
            );
            WsServerEvents::CancelResponse {
                trace_id: ret.trace_id,
                is_success: ret.is_success,
            }
        }
        Err(e) => {
            log::error!(
                "[WS_HANDLER]: Failed to cancel search for trace_id: {}, error: {:?}",
                trace_id,
                e
            );
            WsServerEvents::CancelResponse {
                trace_id: trace_id.to_string(),
                is_success: false,
            }
        }
    }
}

pub async fn handle_search_request(
    session: &mut Session,
    accumulated_results: &mut Vec<SearchResultType>,
    org_id: &str,
    user_id: &str,
    mut req: SearchEventReq,
) -> Result<(), Error> {
    let cfg = get_config();
    let trace_id = req.trace_id.clone();
    let stream_type = req.stream_type;
    let start_time = req.payload.query.start_time;
    let end_time = req.payload.query.end_time;

    log::info!(
        "[WS_SEARCH] trace_id: {} Received search request, start_time: {}, end_time: {}",
        trace_id,
        start_time,
        end_time
    );

    // check and append search event type
    if req.payload.search_type.is_none() {
        req.payload.search_type = Some(req.search_type);
    }

    // get stream name
    #[allow(unused_variables)]
    let stream_names = match resolve_stream_names(&req.payload.query.sql) {
        Ok(v) => v.clone(),
        Err(e) => {
            let err_res =
                WsServerEvents::error_response(Error::Message(e.to_string()), Some(trace_id), None);
            send_message(session, err_res.to_json().to_string()).await?;
            return Ok(());
        }
    };

    // Check permissions for each stream
    #[cfg(feature = "enterprise")]
    for stream_name in stream_names.iter() {
        if let Err(e) =
            enterprise_utils::check_permissions(stream_name, stream_type, user_id, org_id).await
        {
            let err_res = WsServerEvents::error_response(Error::Message(e), Some(trace_id), None);
            send_message(session, err_res.to_json().to_string()).await?;
            return Ok(());
        }
    }

    // handle search result size
    let req_size = if req.payload.query.size == 0 {
        cfg.limit.query_default_limit
    } else {
        req.payload.query.size
    };

    // set search event context
    if req.payload.search_event_context.is_none() && req.search_event_context.is_some() {
        req.payload.search_event_context = get_search_type_from_ws_req(
            &req.search_type,
            req.search_event_context.clone().unwrap(),
        );
    }

    // create new sql query with histogram interval
    let sql = Sql::new(&req.payload.query.clone().into(), org_id, stream_type).await?;
    if let Some(interval) = sql.histogram_interval {
        // modify the sql query statement to include the histogram interval
        let updated_query = update_histogram_interval_in_query(&sql.sql, interval)?;
        req.payload.query.sql = updated_query;
        log::info!(
            "[WS_SEARCH] trace_id: {}; Updated query {}; with histogram interval: {}",
            trace_id,
            req.payload.query.sql,
            interval
        );
    }
    let order_by = sql.order_by.first().map(|v| v.1).unwrap_or_default();

    // start search
    // get the max query range from the stream settings
    let stream_name = if let Some(stream) = stream_names.first() {
        stream
    } else {
        let err_res = WsServerEvents::error_response(
            Error::Message("No stream name found in the query".to_string()),
            Some(trace_id),
            None,
        );
        send_message(session, err_res.to_json().to_string()).await?;
        return Ok(());
    };
    let stream_settings = infra::schema::get_settings(org_id, stream_name, stream_type)
        .await
        .unwrap();
    // TODO: revisit `max_query_range` to handle the below if condition in a better way
    // `max_query_range` is used initialize `remaining_query_range`
    // set max_query_range to i64::MAX if it is 0, to ensure unlimited query range
    let max_query_range = if stream_settings.max_query_range == 0 {
        i64::MAX
    } else {
        stream_settings.max_query_range
    }; // hours

    if is_partition_request(&req.payload, stream_type, org_id).await {
        log::info!(
            "[WS_SEARCH] trace_id: {}, Searching Cache, req_size: {}",
            req.trace_id,
            req_size
        );
        // search cache
        let c_resp =
            cache::check_cache_v2(&trace_id, org_id, stream_type, &req.payload, req.use_cache)
                .await?;
        let local_c_resp = c_resp.clone();
        let cached_resp = local_c_resp.cached_response;
        let mut deltas = local_c_resp.deltas;
        deltas.sort();
        deltas.dedup();

        let cached_hits = cached_resp
            .iter()
            .fold(0, |acc, c| acc + c.cached_response.hits.len());

        let c_start_time = cached_resp
            .first()
            .map(|c| c.response_start_time)
            .unwrap_or_default();

        let c_end_time = cached_resp
            .last()
            .map(|c| c.response_end_time)
            .unwrap_or_default();

        log::info!(
            "[WS_SEARCH] trace_id: {}, found cache responses len:{}, with hits: {}, cache_start_time: {:#?}, cache_end_time: {:#?}",
            trace_id,
            cached_resp.len(),
            cached_hits,
            c_start_time,
            c_end_time
        );

        // handle cache responses and deltas
        if !cached_resp.is_empty() && cached_hits > 0 {
            handle_cache_responses_and_deltas(
                session,
                &req,
                trace_id.clone(),
                req_size,
                cached_resp,
                deltas,
                accumulated_results,
                org_id,
                user_id,
                max_query_range,
                &order_by,
            )
            .await?;
        } else {
            // no caches found process req directly
            log::info!(
                "[WS_SEARCH] trace_id: {} No cache found, processing search request",
                trace_id
            );
            do_partitioned_search(
                session,
                &req,
                &trace_id,
                req_size,
                org_id,
                user_id,
                accumulated_results,
                max_query_range,
            )
            .await?;
        }
        write_results_to_file(c_resp, start_time, end_time, accumulated_results).await?;
    } else {
        // Single search (non-partitioned) for aggregate queries
        log::info!(
            "[WS_SEARCH] trace_id: {} Non-partitioned search",
            req.trace_id
        );
        let end_time = req.payload.query.end_time;
        let search_res = do_search(&req, org_id, user_id).await?;

        let ws_search_res = WsServerEvents::SearchResponse {
            trace_id: trace_id.clone(),
            results: Box::new(search_res.clone()),
            time_offset: end_time,
        };
        log::info!(
            "[WS_SEARCH] trace_id: {} Sending single search response, hits: {}",
            trace_id,
            search_res.hits.len()
        );
        send_message(session, ws_search_res.to_json().to_string()).await?;
    }

    // Once all searches are complete, write the accumulated results to a file
    log::info!("[WS_SEARCH] trace_id {} all searches completed", trace_id);

    Ok(())
}

async fn do_search(req: &SearchEventReq, org_id: &str, user_id: &str) -> Result<Response, Error> {
    SearchService::cache::search(
        &req.trace_id,
        org_id,
        req.stream_type,
        Some(user_id.to_string()),
        &req.payload,
        req.use_cache,
    )
    .instrument(tracing::info_span!(
        "src::handler::http::request::websocket::search::do_search"
    ))
    .await
}

async fn is_partition_request(
    req: &config::meta::search::Request,
    stream_type: StreamType,
    org_id: &str,
) -> bool {
    let cfg = get_config();

    let query = SearchQuery {
        start_time: req.query.start_time,
        end_time: req.query.end_time,
        sql: req.query.sql.clone(),
        ..Default::default()
    };

    let sql = match Sql::new(&query, org_id, stream_type).await {
        Ok(s) => s,
        Err(e) => {
            log::error!("[WS_HANDLER] Failed to create SQL query: {:?}", e);
            return false;
        }
    };

    // if there is no _timestamp field in the query, return single partition
    let is_aggregate = is_aggregate_query(&req.query.sql).unwrap_or_default();
    let res_ts_column = get_ts_col_order_by(&sql, &cfg.common.column_timestamp, is_aggregate);
    let ts_column = res_ts_column.map(|(v, _)| v);

    ts_column.is_some()
}

#[allow(clippy::too_many_arguments)]
async fn handle_cache_responses_and_deltas(
    session: &mut Session,
    req: &SearchEventReq,
    trace_id: String,
    req_size: i64,
    cached_resp: Vec<CachedQueryResponse>,
    mut deltas: Vec<QueryDelta>,
    accumulated_results: &mut Vec<SearchResultType>,
    org_id: &str,
    user_id: &str,
    max_query_range: i64,
    order_by: &OrderBy,
) -> Result<(), Error> {
    // reverse the deltas if order_by is descending
    if let OrderBy::Desc = order_by {
        deltas.reverse();
    }

    // Initialize iterators for deltas and cached responses
    let mut delta_iter = deltas.iter().peekable();
    let mut cached_resp_iter = cached_resp.iter().peekable();
    let mut curr_res_size = 0; // number of records

    let mut remaining_query_range = max_query_range as f64; // hours
    let mut is_search_err = false;

    log::info!(
        "[WS_SEARCH] trace_id: {}, Handling cache response and deltas, curr_res_size: {}, deltas_len: {}, cache_start_time: {}, cache_end_time: {}, deltas: {:?}",
        trace_id,
        curr_res_size,
        deltas.len(),
        cached_resp.first().map(|c| c.response_start_time).unwrap_or_default(),
        cached_resp.last().map(|c| c.response_end_time).unwrap_or_default(),
        deltas
    );

    // Process cached responses and deltas in sorted order
    while cached_resp_iter.peek().is_some() || delta_iter.peek().is_some() {
        if let (Some(&delta), Some(cached)) = (delta_iter.peek(), cached_resp_iter.peek()) {
            // If the delta is before the current cached response time, fetch partitions
            log::info!(
                "[WS_SEARCH] checking delta: {:?} with cached start_time: {:?}, end_time:{}",
                delta,
                cached.response_start_time,
                cached.response_end_time,
            );
            // Compare delta and cached response based on the order
            let process_delta_first = match order_by {
                OrderBy::Asc => delta.delta_end_time <= cached.response_start_time,
                OrderBy::Desc => delta.delta_start_time >= cached.response_end_time,
            };

            if process_delta_first {
                log::info!(
                    "[WS_SEARCH] trace_id: {} Processing delta before cached response, order_by: {:#?}",
                    trace_id,
                    order_by
                );
                let delta = process_delta(
                    session,
                    req,
                    trace_id.clone(),
                    delta,
                    req_size,
                    accumulated_results,
                    &mut curr_res_size,
                    org_id,
                    user_id,
                    &mut remaining_query_range,
                )
                .await;

                if delta.is_err() {
                    is_search_err = true;
                    break;
                }
                delta_iter.next(); // Move to the next delta after processing
            } else {
                // Send cached response
                send_cached_responses(
                    session,
                    &trace_id,
                    req_size,
                    cached,
                    accumulated_results,
                    &mut curr_res_size,
                )
                .await?;
                cached_resp_iter.next();
            }
        } else if let Some(&delta) = delta_iter.peek() {
            // Process remaining deltas
            log::info!(
                "[WS_SEARCH] trace_id: {} Processing remaining delta",
                trace_id
            );
            let delta = process_delta(
                session,
                req,
                trace_id.clone(),
                delta,
                req_size,
                accumulated_results,
                &mut curr_res_size,
                org_id,
                user_id,
                &mut remaining_query_range,
            )
            .await;
            if delta.is_err() {
                is_search_err = true;
                break;
            }
            delta_iter.next(); // Move to the next delta after processing
        } else if let Some(cached) = cached_resp_iter.next() {
            // Process remaining cached responses
            send_cached_responses(
                session,
                &trace_id,
                req_size,
                cached,
                accumulated_results,
                &mut curr_res_size,
            )
            .await?;
        }

        // Stop if reached the requested result size
        if req_size != -1 && curr_res_size >= req_size {
            log::info!(
                "[WS_SEARCH] trace_id: {} Reached requested result size: {}, stopping search",
                trace_id,
                req_size
            );
            break;
        }
    }

    if is_search_err {
        log::info!(
            "[WS_SEARCH] trace_id: {} Search error occurred, stopping search",
            trace_id
        );
        if let Err(e) = send_partial_search_resp(session, &trace_id).await {
            log::error!(
                "[WS_SEARCH] trace_id: {} Failed to send partial search response: {:?}",
                trace_id,
                e
            );
        }
    }

    Ok(())
}

// Process a single delta (time range not covered by cache)
#[allow(clippy::too_many_arguments)]
async fn process_delta(
    session: &mut Session,
    req: &SearchEventReq,
    trace_id: String,
    delta: &QueryDelta,
    req_size: i64,
    accumulated_results: &mut Vec<SearchResultType>,
    curr_res_size: &mut i64,
    org_id: &str,
    user_id: &str,
    remaining_query_range: &mut f64,
) -> Result<(), Error> {
    log::info!(
        "[WS_SEARCH]: Processing delta for trace_id: {}, delta: {:?}",
        trace_id,
        delta
    );
    let mut req = req.clone();
    req.payload.query.start_time = delta.delta_start_time;
    req.payload.query.end_time = delta.delta_end_time;

    let partition_resp = get_partitions(&req, org_id).await?;
    let partitions = partition_resp.partitions;

    if partitions.is_empty() {
        return Ok(());
    }

    log::info!(
        "[WS_SEARCH] Found {} partitions for trace_id: {}",
        partitions.len(),
        trace_id
    );

    for &[start_time, end_time] in partitions.iter() {
        let mut req = req.clone();
        req.payload.query.start_time = start_time;
        req.payload.query.end_time = end_time;

        if req_size != -1 {
            req.payload.query.size -= *curr_res_size;
        }

        let mut search_res = do_search(&req, org_id, user_id).await?;
        *curr_res_size += search_res.hits.len() as i64;

        log::info!(
            "[WS_SEARCH]: Found {} hits, hits: {:#?}, for trace_id: {}",
            search_res.hits.len(),
            search_res.hits,
            trace_id
        );

        if !search_res.hits.is_empty() {
            // for every partition, compute the queried range omitting the result cache ratio
            let queried_range =
                calc_queried_range(start_time, end_time, search_res.result_cache_ratio);
            *remaining_query_range -= queried_range;

            // Accumulate the result
            accumulated_results.push(SearchResultType::Search(search_res.clone()));

            // calc `result_cache_ratio`
            let result_cache_ratio = calc_result_cache_ratio(accumulated_results);
            search_res.result_cache_ratio = result_cache_ratio;

            let ws_search_res = WsServerEvents::SearchResponse {
                trace_id: trace_id.clone(),
                results: Box::new(search_res.clone()),
                time_offset: end_time,
            };
            log::info!(
                "[WS_SEARCH]: Processing deltas for trace_id: {}, hits: {:?}",
                trace_id,
                search_res.hits
            );
            log::info!(
                "[WS_SEARCH]: Sending search response for trace_id: {}, delta: {:?}, hits len: {}, result_cache_ratio: {}, accumulated_results len: {}",
                trace_id,
                delta,
                search_res.hits.len(),
                result_cache_ratio,
                accumulated_results.len()
            );
            send_message(session, ws_search_res.to_json().to_string()).await?;
        }

        // Stop if `remaining_query_range` is less than 0
        if *remaining_query_range <= 0.00 {
            log::info!(
                "[WS_SEARCH]: trace_id: {} Remaining query range is less than 0, stopping search",
                trace_id
            );
            break;
        }

        // Stop if reached the request result size
        if req_size != -1 && *curr_res_size >= req_size {
            log::info!(
                "[WS_SEARCH]: Reached requested result size ({}), stopping search",
                req_size
            );
            break;
        }
    }

    Ok(())
}

async fn get_partitions(
    req: &SearchEventReq,
    org_id: &str,
) -> Result<SearchPartitionResponse, Error> {
    let search_payload = req.payload.clone();
    let search_partition_req = SearchPartitionRequest {
        sql: search_payload.query.sql.clone(),
        start_time: search_payload.query.start_time,
        end_time: search_payload.query.end_time,
        encoding: search_payload.encoding,
        regions: search_payload.regions.clone(),
        clusters: search_payload.clusters.clone(),
        query_fn: search_payload.query.query_fn.clone(),
    };

    let res = SearchService::search_partition(
        &req.trace_id,
        org_id,
        req.stream_type,
        &search_partition_req,
    )
    .instrument(tracing::info_span!(
        "src::handler::http::request::websocket::search::get_partitions"
    ))
    .await?;

    Ok(res)
}

async fn send_cached_responses(
    session: &mut Session,
    trace_id: &str,
    req_size: i64,
    cached: &CachedQueryResponse,
    accumulated_results: &mut Vec<SearchResultType>,
    curr_res_size: &mut i64,
) -> Result<(), Error> {
    log::info!(
        "[WS_SEARCH]: Processing cached response for trace_id: {}",
        trace_id
    );

    let mut cached = cached.clone();

    // add cache hits to `curr_res_size`
    *curr_res_size += cached.cached_response.hits.len() as i64;

    // truncate hits if `curr_res_size` is greater than `req_size`
    if req_size != -1 && *curr_res_size > req_size {
        let excess_hits = *curr_res_size - req_size;
        let total_hits = cached.cached_response.hits.len() as i64;
        if total_hits > excess_hits {
            let cache_hits: usize = (total_hits - excess_hits) as usize;
            cached.cached_response.hits.truncate(cache_hits);
            cached.cached_response.total = cache_hits;
        }
    }

    // Accumulate the result
    accumulated_results.push(SearchResultType::Cached(cached.cached_response.clone()));

    // calc `result_cache_ratio`
    let result_cache_ratio = calc_result_cache_ratio(accumulated_results);
    cached.cached_response.result_cache_ratio = result_cache_ratio;

    // Send the cached response
    let ws_search_res = WsServerEvents::SearchResponse {
        trace_id: trace_id.to_string(),
        results: Box::new(cached.cached_response.clone()),
        time_offset: cached.response_end_time,
    };
    log::info!(
        "[WS_SEARCH]: Sending cached search response for trace_id: {}, hits: {}, result_cache_ratio: {}, accumulated_result len: {}",
        trace_id,
        cached.cached_response.hits.len(),
        result_cache_ratio,
        accumulated_results.len()
    );
    send_message(session, ws_search_res.to_json().to_string()).await?;

    Ok(())
}

// Do partitioned search without cache
#[allow(clippy::too_many_arguments)]
async fn do_partitioned_search(
    session: &mut Session,
    req: &SearchEventReq,
    trace_id: &str,
    req_size: i64,
    org_id: &str,
    user_id: &str,
    accumulated_results: &mut Vec<SearchResultType>,
    max_query_range: i64, // hours
) -> Result<(), Error> {
    let partitions_resp = get_partitions(req, org_id).await?;
    let partitions = partitions_resp.partitions;

    let mut remaining_query_range = max_query_range as f64; // hours

    if partitions.is_empty() {
        return Ok(());
    }

    let mut curr_res_size = 0;

    log::info!(
        "[WS_SEARCH] Found {} partitions for trace_id: {}, partitions: {:#?}",
        partitions.len(),
        trace_id,
        &partitions
    );

    for &[start_time, end_time] in partitions.iter() {
        let mut req = req.clone();
        req.payload.query.start_time = start_time;
        req.payload.query.end_time = end_time;

        if req_size != -1 {
            req.payload.query.size -= curr_res_size;
        }

        let search_res = do_search(&req, org_id, user_id).await?;
        curr_res_size += search_res.hits.len() as i64;

        if !search_res.hits.is_empty() {
            // for every partition, compute the queried range omitting the result cache ratio
            let queried_range =
                calc_queried_range(start_time, end_time, search_res.result_cache_ratio);
            remaining_query_range -= queried_range;

            // Accumulate the result
            accumulated_results.push(SearchResultType::Search(search_res.clone()));

            // Send the cached response
            let ws_search_res = WsServerEvents::SearchResponse {
                trace_id: trace_id.to_string(),
                results: Box::new(search_res.clone()),
                time_offset: end_time,
            };
            send_message(session, ws_search_res.to_json().to_string()).await?;
        }

        // Stop if `remaining_query_range` is less than 0
        if remaining_query_range < 0.00 {
            log::info!(
                "[WS_SEARCH]: trace_id: {} Remaining query range is less than 0, stopping search",
                trace_id
            );
            break;
        }

        // Stop if reached the requested result size
        if req_size != -1 && curr_res_size >= req_size {
            log::info!(
                "[WS_SEARCH]: Reached requested result size ({}), stopping search",
                req_size
            );
            break;
        }
    }

    if curr_res_size == 0 {
        log::info!(
            "[WS_SEARCH]: No hits found for trace_id: {}, partitions: {:#?}",
            trace_id,
            &partitions
        );
        // send empty response
        let ws_search_res = WsServerEvents::SearchResponse {
            trace_id: trace_id.to_string(),
            results: Box::new(Response::default()),
            time_offset: req.payload.query.end_time,
        };
        send_message(session, ws_search_res.to_json().to_string()).await?;
    }

    Ok(())
}

async fn send_partial_search_resp(session: &mut Session, trace_id: &str) -> Result<(), Error> {
    let s_resp = Response {
        is_partial: true,
        ..Default::default()
    };

    let ws_search_res = WsServerEvents::SearchResponse {
        trace_id: trace_id.to_string(),
        results: Box::new(s_resp),
        time_offset: 0,
    };
    log::info!(
        "[WS_SEARCH]: trace_id: {} Sending partial search response",
        trace_id
    );

    send_message(session, ws_search_res.to_json().to_string()).await?;

    Ok(())
}

async fn write_results_to_file(
    c_resp: MultiCachedQueryResponse,
    start_time: i64,
    end_time: i64,
    accumulated_results: &mut Vec<SearchResultType>,
) -> Result<(), Error> {
    if accumulated_results.is_empty() {
        return Ok(());
    }

    log::info!(
            "[WS_SEARCH]: Writing results to file for trace_id: {}, file_path: {}, accumulated_results len: {}",
            c_resp.trace_id,
            c_resp.file_path,
            accumulated_results.len()
        );

    let mut cached_responses = Vec::new();
    let mut search_responses = Vec::new();

    for result in accumulated_results {
        match result {
            SearchResultType::Cached(resp) => cached_responses.push(resp.clone()),
            SearchResultType::Search(resp) => search_responses.push(resp.clone()),
        }
    }

    let merged_response = cache::merge_response_v2(
        &c_resp.trace_id,
        &mut cached_responses,
        &mut search_responses,
        &c_resp.ts_column,
        c_resp.limit,
        c_resp.is_descending,
        c_resp.took,
    );

    cache::write_results_v2(
        &c_resp.trace_id,
        &c_resp.ts_column,
        start_time,
        end_time,
        &merged_response,
        c_resp.file_path.clone(),
        c_resp.is_aggregate,
        c_resp.is_descending,
    )
    .await;

    log::info!(
        "[WS_SEARCH]: Results written to file for trace_id: {}, file_path: {}",
        c_resp.trace_id,
        c_resp.file_path,
    );

    Ok(())
}