use actix_ws::{MessageStream, Session};
use config::{
    get_config,
    meta::{
        search::{RequestEncoding, Response, SearchEventType, SearchPartitionRequest},
        sql::resolve_stream_names,
        stream::StreamType,
    },
    utils::{base64, sql::is_aggregate_query},
};
use futures::StreamExt;
use infra::errors::Error;
use proto::cluster_rpc::SearchQuery;
use tracing::Instrument;

use crate::{
    handler::http::request::websocket::utils::{sessions_cache_utils, WSClientMessage},
    service::{
        search as SearchService,
        search::{cache::cacher::get_ts_col_order_by, sql::Sql, RESULT_ARRAY},
    },
};

pub struct SessionHandler {
    session: Session,
    msg_stream: MessageStream,
    user_id: String,
    request_id: String,
    org_id: String,
    stream_type: StreamType,
    use_cache: bool,
    search_type: String,
}

impl SessionHandler {
    pub fn new(
        session: Session,
        msg_stream: MessageStream,
        user_id: &str,
        request_id: &str,
        org_id: &str,
        stream_type: StreamType,
        use_cache: bool,
        search_type: &str,
    ) -> Self {
        Self {
            session,
            msg_stream,
            user_id: user_id.to_string(),
            request_id: request_id.to_string(),
            org_id: org_id.to_string(),
            stream_type,
            use_cache,
            search_type: search_type.to_string(),
        }
    }

    // Main handler method to run the session
    pub async fn run(mut self) {
        let mut close_reason: Option<actix_ws::CloseReason> = None;

        loop {
            tokio::select! {
                Some(msg) = self.msg_stream.next() => {
                    match msg {
                        Ok(actix_ws::Message::Ping(bytes)) => {
                            if self.session.pong(&bytes).await.is_err() {
                                log::info!("[WEBSOCKET]: Failed to send pong, closing session for request_id: {}", self.request_id);
                                break;
                            }
                        }
                        Ok(actix_ws::Message::Text(msg)) => {
                            log::info!("[WEBSOCKET]: Got text message for request_id: {}: {}", self.request_id, msg);
                            self.handle_text_message(msg.into()).await;
                        }
                        Ok(actix_ws::Message::Close(reason)) => {
                            close_reason = reason;
                            log::info!("[WEBSOCKET]: Session closed for request_id: {}", self.request_id);
                            break;
                        }
                        Ok(actix_ws::Message::Continuation(_)) => {
                            close_reason = None;
                            log::info!("[WEBSOCKET]: Continuation message received, closing session for request_id: {}", self.request_id);
                            break;
                        }
                        _ => (),
                    }
                }
            }
        }

        // Clean up the session when the loop breaks
        self.cleanup().await;

        // Close the session once, after the loop ends
        if let Err(e) = self.session.close(close_reason).await {
            log::error!(
                "[WEBSOCKET]: Error closing session for request_id {}: {:?}",
                self.request_id,
                e
            );
        }
    }

    async fn handle_text_message(&mut self, msg: String) {
        match serde_json::from_str::<WSClientMessage>(&msg) {
            Ok(client_msg) => {
                log::debug!(
                    "[WEBSOCKET]: Received trace registrations msg: {:?}",
                    client_msg
                );
                match client_msg {
                    WSClientMessage::Search { query } => {
                        match self.handle_search_request(query).await {
                            Ok(_) => {}
                            Err(e) => {
                                log::error!(
                                    "[WEBSOCKET]: Failed to get search result for request_id: {}, error: {:?}",
                                    self.request_id,
                                    e
                                );
                            }
                        };
                    }
                    WSClientMessage::Cancel { .. } => {
                        // TODO
                    }
                }
            }
            Err(e) => {
                log::error!(
                    "Failed to parse maessage incoming from ws client: {:?}, {:?}",
                    msg,
                    e
                );
            }
        }
    }

    // Cleanup the session when it ends
    async fn cleanup(&self) {
        sessions_cache_utils::remove_session(&self.request_id);
        log::info!(
            "[WEBSOCKET]: Cleaning up session for request_id: {}, session_cache_len: {}",
            self.request_id,
            sessions_cache_utils::len_sessions()
        );
    }

    async fn handle_search_request(
        &mut self,
        req: config::meta::search::Request,
    ) -> Result<(), Error> {
        let mut response = vec![];

        // create the parent trace_id
        let trace_id = config::ider::uuid();

        if self.is_partition_request(&req).await {
            let partitions = self.get_partitions(&req, &trace_id).await;

            if partitions.is_empty() {
                return Ok(());
            }

            for [start_time, end_time] in partitions {
                let mut req = req.clone();
                req.query.start_time = start_time;
                req.query.end_time = end_time;
                let search_res = self.do_search(req, trace_id.clone()).await?;
                response.push(search_res);
            }
        } else {
            // call search directly
            let search_res = self.do_search(req, trace_id).await?;
            response.push(search_res);
        }

        // send the search result for every response
        let response = serde_json::json!({
            "payload": response,
        });
        if self.session.text(response.to_string()).await.is_err() {
            log::error!(
                "[WEBSOCKET]: Failed to send search response for request_id: {}",
                self.request_id
            );
        }

        Ok(())
    }

    async fn is_partition_request(&self, req: &config::meta::search::Request) -> bool {
        let cfg = get_config();

        let query = SearchQuery {
            start_time: req.query.start_time,
            end_time: req.query.end_time,
            sql: req.query.sql.clone(),
            ..Default::default()
        };

        let sql = match Sql::new(&query, &self.org_id, self.stream_type).await {
            Ok(s) => s,
            Err(e) => {
                log::error!("[WEBSOCKET] Failed to create SQL query: {:?}", e);
                return false;
            }
        };

        // check for vrl
        let apply_over_hits = match req.query.query_fn.as_ref() {
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

        // if there is no _timestamp field in the query, return single partition
        let is_aggregate = is_aggregate_query(&req.query.sql).unwrap_or_default();
        let res_ts_column = get_ts_col_order_by(&sql, &cfg.common.column_timestamp, is_aggregate);
        let ts_column = res_ts_column.map(|(v, _)| v);
        let skip_get_file_list = ts_column.is_none() || apply_over_hits;

        skip_get_file_list
    }

    async fn get_partitions(
        &self,
        req: &config::meta::search::Request,
        trace_id: &str,
    ) -> Vec<[i64; 2]> {
        let search_partition_req = SearchPartitionRequest {
            sql: req.query.sql.clone(),
            start_time: req.query.start_time,
            end_time: req.query.end_time,
            encoding: req.encoding.clone(),
            regions: req.regions.clone(),
            clusters: req.clusters.clone(),
            query_fn: req.query.query_fn.clone(),
        };

        let res = SearchService::search_partition(
            &trace_id,
            &self.org_id,
            self.stream_type,
            &search_partition_req,
        )
        .instrument(tracing::info_span!("search_partition"))
        .await;

        // get the list of partitions
        let partitions = match res {
            Ok(res) => res.partitions,
            Err(e) => {
                log::error!(
                    "[WEBSOCKET]: Failed to get partitions for request_id: {}, error: {:?}",
                    self.request_id,
                    e
                );
                return vec![];
            }
        };

        partitions
    }

    async fn do_search(
        &mut self,
        query: config::meta::search::Request,
        trace_id: String,
    ) -> Result<Response, infra::errors::Error> {
        SearchService::cache::search(
            &trace_id,
            &self.org_id,
            self.stream_type,
            Some("root@example.com".to_string()),
            &query,
            self.use_cache,
        )
        .instrument(tracing::info_span!("search"))
        .await
    }

    // TODO: Remove this method
    async fn _handle_search_request(&mut self, query: SearchPartitionRequest) {
        // create the parent trace_id
        let trace_id = config::ider::uuid();
        // call the search partition service
        let search_partition_res =
            SearchService::search_partition(&trace_id, &self.org_id, self.stream_type, &query)
                .instrument(tracing::info_span!("search_partition"))
                .await;

        // get the list of partitions
        let partitions = match search_partition_res {
            Ok(res) => res.partitions,
            Err(e) => {
                log::error!(
                    "[WEBSOCKET]: Failed to get partitions for request_id: {}, error: {:?}",
                    self.request_id,
                    e
                );
                return;
            }
        };

        // respond to the client with the parent trace_id
        // for reference, to call cancel query if required
        let response = serde_json::json!({
            "trace_id": trace_id,
            "partitions": partitions,
        });

        if self.session.text(response.to_string()).await.is_err() {
            log::error!(
                "[WEBSOCKET]: Failed to send search partition response for request_id: {}",
                self.request_id
            );
        }

        let cfg = get_config();
        let use_cache = false;

        // for each partition, call the search service
        // TODO: What does `size`, `from` do?
        for [start_time, end_time] in partitions {
            let mut query = config::meta::search::Query {
                sql: query.sql.clone(),
                start_time,
                end_time,
                size: 100,
                from: 0,
                quick_mode: false,
                ..Default::default()
            };
            let stream_names =
                resolve_stream_names(&query.sql).expect("Failed to resolve stream names");

            // get stream settings
            for stream_name in stream_names {
                if let Some(settings) =
                    infra::schema::get_settings(&self.org_id, &stream_name, self.stream_type).await
                {
                    let max_query_range = settings.max_query_range;
                    if max_query_range > 0
                        && (query.end_time.clone() - query.start_time.clone())
                            > max_query_range * 3600 * 1_000_000
                    {
                        query.start_time =
                            query.end_time.clone() - max_query_range * 3600 * 1_000_000;
                        let error = format!(
                            "Query duration is modified due to query range restriction of {} hours",
                            max_query_range
                        );
                        log::warn!("[WEBSOCKET]: {} for request_id: {}", error, self.request_id);
                    }
                }

                let req: config::meta::search::Request = config::meta::search::Request {
                    query: query.clone(),
                    search_type: Some(SearchEventType::UI),
                    search_event_context: None,
                    index_type: cfg.common.inverted_index_search_format.to_string(),
                    encoding: RequestEncoding::default(),
                    regions: vec![],
                    clusters: vec![],
                    timeout: 0,
                };
                dbg!(
                    &trace_id,
                    &self.org_id,
                    &self.stream_type,
                    &self.user_id,
                    &req,
                    &use_cache
                );
                let search_res = SearchService::cache::search(
                    &trace_id,
                    &self.org_id,
                    self.stream_type,
                    Some("root@example.com".to_string()),
                    &req,
                    use_cache,
                )
                .instrument(tracing::info_span!("search"))
                .await;

                // send the search result for every response
                match search_res {
                    Ok(res) => {
                        let response = serde_json::json!({
                            "search_res": res,
                        });

                        if self.session.text(response.to_string()).await.is_err() {
                            log::error!(
                                "[WEBSOCKET]: Failed to send search response for request_id: {}",
                                self.request_id
                            );
                        }
                    }
                    Err(e) => {
                        log::error!(
                            "[WEBSOCKET]: Failed to get search result for request_id: {}, error: {:?}",
                            self.request_id,
                            e
                        );
                    }
                }
            }
        }
    }
}