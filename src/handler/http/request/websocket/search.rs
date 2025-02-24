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

use config::{
    get_config,
    meta::{
        search::{
            Response, SearchEventType, SearchPartitionRequest, SearchPartitionResponse,
            PARTIAL_ERROR_RESPONSE_MESSAGE,
        },
        sql::{resolve_stream_names, OrderBy},
        websocket::{SearchEventReq, SearchResultType, MAX_QUERY_RANGE_LIMIT_ERROR_MESSAGE},
    },
};
use infra::errors::{Error, ErrorCodes};
use tracing::Instrument;

use super::sort::order_search_results;
#[allow(unused_imports)]
use crate::handler::http::request::websocket::utils::enterprise_utils;
use crate::{
    common::{
        meta::search::{CachedQueryResponse, MultiCachedQueryResponse, QueryDelta},
        utils::{
            stream::get_max_query_range,
            websocket::{
                calc_queried_range, get_search_type_from_ws_req, update_histogram_interval_in_query,
            },
        },
    },
    handler::http::request::websocket::{
        session::send_message,
        utils::{search_registry_utils, TimeOffset, WsServerEvents},
    },
    service::search::{
        self as SearchService, cache, datafusion::distributed_plan::streaming_aggs_exec, sql::Sql,
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
                is_success: true,
            }
        }
    }
}

pub async fn handle_search_request(
    req_id: &str,
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
    let stream_names = match resolve_stream_names(&req.payload.query.sql) {
        Ok(v) => v.clone(),
        Err(e) => {
            let err_res = WsServerEvents::error_response(
                Error::Message(e.to_string()),
                Some(req_id.to_string()),
                Some(trace_id),
            );
            send_message(req_id, err_res.to_json().to_string()).await?;
            return Ok(());
        }
    };

    // Check permissions for each stream
    #[cfg(feature = "enterprise")]
    for stream_name in stream_names.iter() {
        if let Err(e) =
            enterprise_utils::check_permissions(stream_name, stream_type, user_id, org_id).await
        {
            let err_res = WsServerEvents::error_response(
                Error::Message(e),
                Some(req_id.to_string()),
                Some(trace_id),
            );
            send_message(req_id, err_res.to_json().to_string()).await?;
            return Ok(());
        }
    }

    // handle search result size
    let req_size = if req.payload.query.size == 0 {
        req.payload.query.size = cfg.limit.query_default_limit;
        req.payload.query.size
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
        let updated_query = update_histogram_interval_in_query(&req.payload.query.sql, interval)?;
        req.payload.query.sql = updated_query;
        log::info!(
            "[WS_SEARCH] trace_id: {}; Updated query {}; with histogram interval: {}",
            trace_id,
            req.payload.query.sql,
            interval
        );
    }
    let order_by = sql.order_by.first().map(|v| v.1).unwrap_or_default();

    // Search start
    log::info!(
        "[WS_SEARCH] trace_id: {}, Searching Cache, req_size: {}",
        req.trace_id,
        req_size
    );
    // Step 1: Search result cache
    if req.payload.query.from == 0 {
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
            // `max_query_range` is used initialize `remaining_query_range`
            // set max_query_range to i64::MAX if it is 0, to ensure unlimited query range
            // for cache only search
            let max_query_range =
                get_max_query_range(&stream_names, org_id, user_id, stream_type).await; // hours
            let remaining_query_range = if max_query_range == 0 {
                i64::MAX
            } else {
                max_query_range
            }; // hours

            // Step 1(a): handle cache responses & query the deltas
            handle_cache_responses_and_deltas(
                req_id,
                &req,
                trace_id.clone(),
                req_size,
                cached_resp,
                deltas,
                accumulated_results,
                org_id,
                user_id,
                max_query_range,
                remaining_query_range,
                &order_by,
            )
            .await?;
        } else {
            // Step 2: Search without cache
            // no caches found process req directly
            log::info!(
                "[WS_SEARCH] trace_id: {} No cache found, processing search request",
                trace_id
            );
            let max_query_range =
                get_max_query_range(&stream_names, org_id, user_id, stream_type).await; // hours

            do_partitioned_search(
                req_id,
                &mut req,
                &trace_id,
                req_size,
                org_id,
                user_id,
                accumulated_results,
                max_query_range,
            )
            .await?;
        }
        // Step 3: Write to results cache
        // cache only if from is 0 and is not an aggregate_query
        if req.payload.query.from == 0 {
            write_results_to_cache(c_resp, start_time, end_time, accumulated_results)
                .await
                .map_err(|e| {
                    log::error!(
                        "[WS_SEARCH] trace_id: {}, Error writing results to cache: {:?}",
                        trace_id,
                        e
                    );
                    Error::Message(e.to_string())
                })?;
        }
    } else {
        // Step 4: Search without cache for req with from > 0
        let max_query_range =
            get_max_query_range(&stream_names, org_id, user_id, stream_type).await; // hours

        do_partitioned_search(
            req_id,
            &mut req,
            &trace_id,
            req_size,
            org_id,
            user_id,
            accumulated_results,
            max_query_range,
        )
        .await?;
    }

    // Once all searches are complete, write the accumulated results to a file
    log::info!("[WS_SEARCH] trace_id {} all searches completed", trace_id);
    let end_res = WsServerEvents::End {
        trace_id: Some(trace_id.clone()),
    };
    send_message(req_id, end_res.to_json().to_string()).await?;

    Ok(())
}

async fn do_search(
    req: &SearchEventReq,
    org_id: &str,
    user_id: &str,
    use_cache: bool,
) -> Result<Response, Error> {
    let span = tracing::info_span!(
        "src::handler::http::request::websocket::search::do_search",
        trace_id = %req.trace_id,
        org_id = %org_id,
    );

    let mut req = req.clone();
    req.payload.use_cache = Some(use_cache);
    let res = SearchService::cache::search(
        &req.trace_id,
        org_id,
        req.stream_type,
        Some(user_id.to_string()),
        &req.payload,
        "".to_string(),
    )
    .instrument(span)
    .await;

    res.map(handle_partial_response)
}

fn handle_partial_response(mut res: Response) -> Response {
    if res.is_partial {
        res.function_error = if res.function_error.is_empty() {
            PARTIAL_ERROR_RESPONSE_MESSAGE.to_string()
        } else {
            format!(
                "{} \n {}",
                PARTIAL_ERROR_RESPONSE_MESSAGE, res.function_error
            )
        };
    }
    res
}

#[allow(clippy::too_many_arguments)]
async fn handle_cache_responses_and_deltas(
    req_id: &str,
    req: &SearchEventReq,
    trace_id: String,
    req_size: i64,
    mut cached_resp: Vec<CachedQueryResponse>,
    mut deltas: Vec<QueryDelta>,
    accumulated_results: &mut Vec<SearchResultType>,
    org_id: &str,
    user_id: &str,
    max_query_range: i64,
    remaining_query_range: i64,
    mut order_by: &OrderBy,
) -> Result<(), Error> {
    // Force set order_by to desc for dashboards & histogram
    // so that deltas are processed in the reverse order
    if req.search_type == SearchEventType::Dashboards || req.payload.query.size == -1 {
        order_by = &OrderBy::Desc;
    }

    // sort both deltas and cache by order_by
    match order_by {
        OrderBy::Desc => {
            deltas.sort_by(|a, b| b.delta_start_time.cmp(&a.delta_start_time));
            cached_resp.sort_by(|a, b| b.response_start_time.cmp(&a.response_start_time));
        }
        OrderBy::Asc => {
            deltas.sort_by(|a, b| a.delta_start_time.cmp(&b.delta_start_time));
            cached_resp.sort_by(|a, b| a.response_start_time.cmp(&b.response_start_time));
        }
    }

    // Initialize iterators for deltas and cached responses
    let mut delta_iter = deltas.iter().peekable();
    let mut cached_resp_iter = cached_resp.iter().peekable();
    let mut curr_res_size = 0; // number of records

    let mut remaining_query_range = remaining_query_range as f64; // hours
    let cache_start_time = cached_resp
        .first()
        .map(|c| c.response_start_time)
        .unwrap_or_default();
    let cache_end_time = cached_resp
        .last()
        .map(|c| c.response_end_time)
        .unwrap_or_default();
    let cache_duration = cache_end_time - cache_start_time; // microseconds
    let cached_search_duration = cache_duration + (max_query_range * 3600 * 1_000_000); // microseconds

    log::info!(
        "[WS_SEARCH] trace_id: {}, Handling cache response and deltas, curr_res_size: {}, cached_search_duration: {}, remaining_query_duration: {}, deltas_len: {}, cache_start_time: {}, cache_end_time: {}, deltas: {:?}",
        trace_id,
        curr_res_size,
        cached_search_duration,
        remaining_query_range,
        deltas.len(),
        cache_start_time,
        cache_end_time,
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
                process_delta(
                    req_id,
                    req,
                    trace_id.clone(),
                    delta,
                    req_size,
                    accumulated_results,
                    &mut curr_res_size,
                    org_id,
                    user_id,
                    &mut remaining_query_range,
                    cached_search_duration,
                )
                .await?;
                delta_iter.next(); // Move to the next delta after processing
            } else {
                // Send cached response
                send_cached_responses(
                    req_id,
                    &trace_id,
                    req_size,
                    cached,
                    accumulated_results,
                    &mut curr_res_size,
                    req.fallback_order_by_col.clone(),
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
            process_delta(
                req_id,
                req,
                trace_id.clone(),
                delta,
                req_size,
                accumulated_results,
                &mut curr_res_size,
                org_id,
                user_id,
                &mut remaining_query_range,
                cached_search_duration,
            )
            .await?;
            delta_iter.next(); // Move to the next delta after processing
        } else if let Some(cached) = cached_resp_iter.next() {
            // Process remaining cached responses
            send_cached_responses(
                req_id,
                &trace_id,
                req_size,
                cached,
                accumulated_results,
                &mut curr_res_size,
                req.fallback_order_by_col.clone(),
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

    Ok(())
}

// Process a single delta (time range not covered by cache)
#[allow(clippy::too_many_arguments)]
async fn process_delta(
    req_id: &str,
    req: &SearchEventReq,
    trace_id: String,
    delta: &QueryDelta,
    req_size: i64,
    accumulated_results: &mut Vec<SearchResultType>,
    curr_res_size: &mut i64,
    org_id: &str,
    user_id: &str,
    remaining_query_range: &mut f64,
    cache_req_duration: i64,
) -> Result<(), Error> {
    log::info!(
        "[WS_SEARCH]: Processing delta for trace_id: {}, delta: {:?}",
        trace_id,
        delta
    );
    let mut req = req.clone();
    let _original_req_start_time = req.payload.query.start_time;
    let original_req_end_time = req.payload.query.end_time;
    req.payload.query.start_time = delta.delta_start_time;
    req.payload.query.end_time = delta.delta_end_time;

    let partition_resp = get_partitions(&req, org_id, user_id).await?;
    let mut partitions = partition_resp.partitions;

    if partitions.is_empty() {
        return Ok(());
    }

    // check if `streaming_aggs` is supported
    let is_streaming_aggs = partition_resp.streaming_aggs;
    if is_streaming_aggs {
        req.payload.query.streaming_output = true;
        req.payload.query.streaming_id = partition_resp.streaming_id.clone();
    }

    log::info!(
        "[WS_SEARCH] Found {} partitions for trace_id: {}",
        partitions.len(),
        trace_id
    );

    // for dashboards & histograms
    if req.search_type == SearchEventType::Dashboards || req.payload.query.size == -1 {
        // sort partitions by timestamp in desc
        partitions.sort_by(|a, b| b[0].cmp(&a[0]));
    }

    for (idx, &[start_time, end_time]) in partitions.iter().enumerate() {
        // Check if the cancellation flag is set
        if let Some(is_cancelled) = search_registry_utils::is_cancelled(&trace_id) {
            if is_cancelled {
                // Search is cancelled, stop processing
                return Ok(());
            }
        } else {
            // Search not found in registry, stop processing
            return Ok(());
        }

        let mut req = req.clone();
        req.payload.query.start_time = start_time;
        req.payload.query.end_time = end_time;

        if req_size != -1 {
            req.payload.query.size -= *curr_res_size;
        }

        // use cache for delta search
        let mut search_res = do_search(&req, org_id, user_id, true).await?;
        *curr_res_size += search_res.hits.len() as i64;

        log::info!(
            "[WS_SEARCH]: Found {} hits, hits: {:#?}, for trace_id: {}",
            search_res.hits.len(),
            search_res.hits,
            trace_id
        );

        if !search_res.hits.is_empty() {
            search_res = order_search_results(search_res, req.fallback_order_by_col);
            // for every partition, compute the queried range omitting the result cache ratio
            let queried_range =
                calc_queried_range(start_time, end_time, search_res.result_cache_ratio);
            *remaining_query_range -= queried_range;

            // when searching with limit queries
            // the limit in sql takes precedence over the requested size
            // hence, the search result needs to be trimmed when the req limit is reached
            if *curr_res_size > req_size {
                let excess_hits = *curr_res_size - req_size;
                let total_hits = search_res.hits.len() as i64;
                if total_hits > excess_hits {
                    let cache_hits: usize = (total_hits - excess_hits) as usize;
                    search_res.hits.truncate(cache_hits);
                    search_res.total = cache_hits;
                }
            }

            // Accumulate the result
            if is_streaming_aggs {
                // Only accumulate the results of the last partition
                if idx == partitions.len() - 1 {
                    accumulated_results.push(SearchResultType::Search(search_res.clone()));
                }
            } else {
                accumulated_results.push(SearchResultType::Search(search_res.clone()));
            }

            // `result_cache_ratio` will be 0 for delta search
            let result_cache_ratio = search_res.result_cache_ratio;

            let ws_search_res = WsServerEvents::SearchResponse {
                trace_id: trace_id.clone(),
                results: Box::new(search_res.clone()),
                time_offset: TimeOffset {
                    start_time,
                    end_time,
                },
                streaming_aggs: is_streaming_aggs,
            };
            log::info!(
                "[WS_SEARCH]: Processing deltas for trace_id: {}, hits: {:?}",
                trace_id,
                search_res.hits
            );
            log::debug!(
                "[WS_SEARCH]: Sending search response for trace_id: {}, delta: {:?}, hits len: {}, result_cache_ratio: {}, accumulated_results len: {}",
                trace_id,
                delta,
                search_res.hits.len(),
                result_cache_ratio,
                accumulated_results.len()
            );
            send_message(req_id, ws_search_res.to_json().to_string()).await?;
        }

        // Stop if `remaining_query_range` is less than 0
        if *remaining_query_range <= 0.00 {
            log::info!(
                "[WS_SEARCH]: trace_id: {} Remaining query range is less than 0, stopping search",
                trace_id
            );
            let (new_start_time, new_end_time) = (
                original_req_end_time - cache_req_duration,
                original_req_end_time,
            );
            // passs original start_time and end_time partition end time
            let _ = send_partial_search_resp(
                req_id,
                &trace_id,
                MAX_QUERY_RANGE_LIMIT_ERROR_MESSAGE,
                new_start_time,
                new_end_time,
                search_res.order_by,
                is_streaming_aggs,
            )
            .await;
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

    // Remove the streaming_aggs cache
    if is_streaming_aggs && partition_resp.streaming_id.is_some() {
        streaming_aggs_exec::remove_cache(&partition_resp.streaming_id.unwrap())
    }

    Ok(())
}

async fn get_partitions(
    req: &SearchEventReq,
    org_id: &str,
    user_id: &str,
) -> Result<SearchPartitionResponse, Error> {
    let search_payload = req.payload.clone();
    let search_partition_req = SearchPartitionRequest {
        sql: search_payload.query.sql.clone(),
        start_time: search_payload.query.start_time,
        end_time: search_payload.query.end_time,
        encoding: search_payload.encoding,
        regions: search_payload.regions.clone(),
        clusters: search_payload.clusters.clone(),
        // vrl is not required for _search_partition
        query_fn: Default::default(),
        streaming_output: true,
    };

    let res = SearchService::search_partition(
        &req.trace_id,
        org_id,
        Some(user_id),
        req.stream_type,
        &search_partition_req,
        false,
    )
    .instrument(tracing::info_span!(
        "src::handler::http::request::websocket::search::get_partitions"
    ))
    .await?;

    Ok(res)
}

async fn send_cached_responses(
    req_id: &str,
    trace_id: &str,
    req_size: i64,
    cached: &CachedQueryResponse,
    accumulated_results: &mut Vec<SearchResultType>,
    curr_res_size: &mut i64,
    fallback_order_by_col: Option<String>,
) -> Result<(), Error> {
    if let Some(is_cancelled) = search_registry_utils::is_cancelled(trace_id) {
        if is_cancelled {
            log::info!(
                "[WS_SEARCH]: Cancellation detected for trace_id: {}, stopping cached response",
                trace_id
            );
            return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!(
                "Search cancel detected for trace_id: {}",
                trace_id
            ))));
        };
    } else {
        // Search not found in registry, stop processing
        return Err(Error::ErrorCode(ErrorCodes::SearchCancelQuery(format!(
            "Search cancel detected for trace_id: {}",
            trace_id
        ))));
    }

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

    cached.cached_response = order_search_results(cached.cached_response, fallback_order_by_col);

    // Accumulate the result
    accumulated_results.push(SearchResultType::Cached(cached.cached_response.clone()));

    // `result_cache_ratio` for cached response is 100
    cached.cached_response.result_cache_ratio = 100;
    // `scan_size` is 0, as it is not used for cached responses
    cached.cached_response.scan_size = 0;

    // Send the cached response
    let ws_search_res = WsServerEvents::SearchResponse {
        trace_id: trace_id.to_string(),
        results: Box::new(cached.cached_response.clone()),
        time_offset: TimeOffset {
            start_time: cached.response_start_time,
            end_time: cached.response_end_time,
        },
        streaming_aggs: false, // streaming aggs is not supported for cached responses
    };
    log::info!(
        "[WS_SEARCH]: Sending cached search response for trace_id: {}, hits: {}, result_cache_ratio: {}, accumulated_result len: {}",
        trace_id,
        cached.cached_response.hits.len(),
        cached.cached_response.result_cache_ratio,
        accumulated_results.len()
    );
    send_message(req_id, ws_search_res.to_json().to_string()).await?;

    Ok(())
}

// Do partitioned search without cache
#[allow(clippy::too_many_arguments)]
async fn do_partitioned_search(
    req_id: &str,
    req: &mut SearchEventReq,
    trace_id: &str,
    req_size: i64,
    org_id: &str,
    user_id: &str,
    accumulated_results: &mut Vec<SearchResultType>,
    max_query_range: i64, // hours
) -> Result<(), Error> {
    // limit the search by max_query_range
    let mut range_error = String::new();
    if max_query_range > 0
        && (req.payload.query.end_time - req.payload.query.start_time)
            > max_query_range * 3600 * 1_000_000
    {
        req.payload.query.start_time =
            req.payload.query.end_time - max_query_range * 3600 * 1_000_000;
        log::info!(
            "[WS_SEARCH] Query duration is modified due to query range restriction of {} hours, new start_time: {}",
            max_query_range,
            req.payload.query.start_time
        );
        range_error = format!(
            "Query duration is modified due to query range restriction of {} hours",
            max_query_range
        );
    }
    // for new_start_time & new_end_time
    let modified_start_time = req.payload.query.start_time;
    let modified_end_time = req.payload.query.end_time;

    let partition_resp = get_partitions(req, org_id, user_id).await?;
    let mut partitions = partition_resp.partitions;

    if partitions.is_empty() {
        return Ok(());
    }

    // check if `streaming_aggs` is supported
    let is_streaming_aggs = partition_resp.streaming_aggs;
    if is_streaming_aggs {
        req.payload.query.streaming_output = true;
        req.payload.query.streaming_id = partition_resp.streaming_id.clone();
    }

    // sort partitions in desc by _timestamp for dashboards & histograms
    if req.search_type == SearchEventType::Dashboards || req.payload.query.size == -1 {
        partitions.sort_by(|a, b| b[0].cmp(&a[0]));
    }

    let mut curr_res_size = 0;

    log::info!(
        "[WS_SEARCH] Found {} partitions for trace_id: {}, partitions: {:#?}",
        partitions.len(),
        trace_id,
        &partitions
    );

    for (idx, &[start_time, end_time]) in partitions.iter().enumerate() {
        // Check if the cancellation flag is set
        if let Some(is_cancelled) = search_registry_utils::is_cancelled(trace_id) {
            if is_cancelled {
                log::info!(
                    "[WS_SEARCH]: Cancellation detected for trace_id: {}, stopping partitioned search",
                    trace_id
                );
                return Ok(());
            }
        } else {
            // Search not found in registry, stop processing
            return Ok(());
        }

        let mut req = req.clone();
        req.payload.query.start_time = start_time;
        req.payload.query.end_time = end_time;

        if req_size != -1 {
            req.payload.query.size -= curr_res_size;
        }

        // do not use cache for partitioned search without cache
        let mut search_res = do_search(&req, org_id, user_id, false).await?;
        curr_res_size += search_res.hits.len() as i64;

        if !search_res.hits.is_empty() {
            search_res = order_search_results(search_res, req.fallback_order_by_col);

            // check range error
            if !range_error.is_empty() {
                search_res.is_partial = true;
                search_res.function_error = if search_res.function_error.is_empty() {
                    range_error.clone()
                } else {
                    format!("{} \n {}", range_error, search_res.function_error)
                };
                search_res.new_start_time = Some(modified_start_time);
                search_res.new_end_time = Some(modified_end_time);
            }

            // Accumulate the result
            if is_streaming_aggs {
                // Only accumulate the results of the last partition
                if idx == partitions.len() - 1 {
                    accumulated_results.push(SearchResultType::Search(search_res.clone()));
                }
            } else {
                accumulated_results.push(SearchResultType::Search(search_res.clone()));
            }

            // Send the cached response
            let ws_search_res = WsServerEvents::SearchResponse {
                trace_id: trace_id.to_string(),
                results: Box::new(search_res.clone()),
                time_offset: TimeOffset {
                    start_time,
                    end_time,
                },
                streaming_aggs: is_streaming_aggs,
            };
            send_message(req_id, ws_search_res.to_json().to_string()).await?;
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
            time_offset: TimeOffset {
                start_time: req.payload.query.start_time,
                end_time: req.payload.query.end_time,
            },
            streaming_aggs: is_streaming_aggs,
        };
        send_message(req_id, ws_search_res.to_json().to_string()).await?;
    }

    // Remove the streaming_aggs cache
    if is_streaming_aggs && partition_resp.streaming_id.is_some() {
        streaming_aggs_exec::remove_cache(&partition_resp.streaming_id.unwrap())
    }
    Ok(())
}

async fn send_partial_search_resp(
    req_id: &str,
    trace_id: &str,
    error: &str,
    new_start_time: i64,
    new_end_time: i64,
    order_by: Option<OrderBy>,
    is_streaming_aggs: bool,
) -> Result<(), Error> {
    let error = if error.is_empty() {
        PARTIAL_ERROR_RESPONSE_MESSAGE.to_string()
    } else {
        format!("{} \n {}", PARTIAL_ERROR_RESPONSE_MESSAGE, error)
    };
    let s_resp = Response {
        is_partial: true,
        function_error: error,
        new_start_time: Some(new_start_time),
        new_end_time: Some(new_end_time),
        order_by,
        trace_id: trace_id.to_string(),
        ..Default::default()
    };

    let ws_search_res = WsServerEvents::SearchResponse {
        trace_id: trace_id.to_string(),
        results: Box::new(s_resp),
        time_offset: TimeOffset {
            start_time: new_start_time,
            end_time: new_end_time,
        },
        streaming_aggs: is_streaming_aggs,
    };
    log::info!(
        "[WS_SEARCH]: trace_id: {} Sending partial search response",
        trace_id
    );

    send_message(req_id, ws_search_res.to_json().to_string()).await?;

    Ok(())
}

async fn write_results_to_cache(
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

    let cfg = get_config();
    let mut cached_responses = Vec::new();
    let mut search_responses = Vec::new();

    for result in accumulated_results {
        match result {
            SearchResultType::Cached(resp) => cached_responses.push(resp.clone()),
            SearchResultType::Search(resp) => search_responses.push(resp.clone()),
        }
    }

    let merged_response = cache::merge_response(
        &c_resp.trace_id,
        &mut cached_responses,
        &mut search_responses,
        &c_resp.ts_column,
        c_resp.limit,
        c_resp.is_descending,
        c_resp.took,
    );

    // There are 3 types of partial responses:
    // 1. VRL error
    // 2. Super cluster error
    // 3. Range error (max_query_limit)
    // Cache partial results only if there is a range error
    let skip_cache_results = (merged_response.is_partial
        && (merged_response.new_start_time.is_none() || merged_response.new_end_time.is_none()))
        || (!merged_response.function_error.is_empty()
            && merged_response.function_error.contains("vrl"));

    if cfg.common.result_cache_enabled && !skip_cache_results {
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
    }

    Ok(())
}
