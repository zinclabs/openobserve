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

use actix_web::http::StatusCode;
use config::meta::websocket::SearchEventReq;
use infra::{errors, errors::Error};
use serde::{Deserialize, Serialize};

pub mod enterprise_utils {
    #[allow(unused_imports)]
    use config::meta::stream::StreamType;

    #[allow(unused_imports)]
    use crate::common::meta;

    #[cfg(feature = "enterprise")]
    pub async fn check_permissions(
        stream_name: &str,
        stream_type: StreamType,
        user_id: &str,
        org_id: &str,
    ) -> Result<(), String> {
        use o2_openfga::meta::mapping::OFGA_MODELS;

        use crate::common::{
            infra::config::USERS,
            utils::auth::{is_root_user, AuthExtractor},
        };

        // Check if the user is a root user (has all permissions)
        if is_root_user(user_id) {
            return Ok(());
        }

        // Get user details from the USERS cache
        let user: meta::user::User = USERS
            .get(&format!("{}/{}", org_id, user_id))
            .ok_or_else(|| "User not found".to_string())?
            .clone();

        // If the user is external, check permissions
        let stream_type_str = stream_type.as_str();
        let o2_type = format!(
            "{}:{}",
            OFGA_MODELS
                .get(stream_type_str)
                .map_or(stream_type_str, |model| model.key),
            stream_name
        );

        let auth_extractor = AuthExtractor {
            auth: "".to_string(),
            method: "GET".to_string(),
            o2_type,
            org_id: org_id.to_string(),
            bypass_check: false,
            parent_id: "".to_string(),
        };

        let has_permission = crate::handler::http::auth::validator::check_permissions(
            user_id,
            auth_extractor,
            user.role,
            user.is_external,
        )
        .await;

        if !has_permission {
            return Err("Unauthorized Access".to_string());
        };

        Ok(())
    }
}

pub mod sessions_cache_utils {
    use actix_ws::{CloseCode, CloseReason};
    use config::get_config;
    use futures::FutureExt;

    use super::search_registry_utils::SearchState;
    use crate::{
        common::infra::config::WS_SESSIONS,
        handler::http::request::websocket::session::{WsSession, SEARCH_REGISTRY},
    };

    pub async fn run_gc_ws_sessions() {
        log::info!("[WS_GC] Running garbage collector for websocket sessions");
        let cfg = get_config();
        let interval_secs = cfg.websocket.session_gc_interval_secs;

        let mut interval =
            tokio::time::interval(std::time::Duration::from_secs(interval_secs as u64));

        tokio::spawn(async move {
            loop {
                interval.tick().await;
                log::info!("[WS_GC] Running garbage collector for websocket sessions");

                // Use catch_unwind to prevent task from crashing
                if let Err(e) = std::panic::AssertUnwindSafe(cleanup_expired_sessions())
                    .catch_unwind()
                    .await
                {
                    log::error!("[WS_GC] Panic in cleanup: {:?}", e);
                    // Add delay before next attempt
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    continue;
                }

                // Add delay between runs if too many sessions
                if WS_SESSIONS.len() > 1000 {
                    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                }
            }
        });
    }

    async fn cleanup_expired_sessions() {
        let expired: Vec<String> = WS_SESSIONS
            .iter()
            .filter(|entry| entry.value().is_expired())
            .map(|entry| entry.key().clone())
            .collect();

        for session_id in expired {
            // Clean up associated searches first
            cleanup_searches_for_session(&session_id);

            // Close and remove session
            if let Some(mut session) = get_mut_session(&session_id) {
                log::info!("[WS_GC] Closing expired session: {}", session_id);

                if let Err(e) = session
                    .close(Some(CloseReason {
                        code: CloseCode::Normal,
                        description: Some("Session expired".to_string()),
                    }))
                    .await
                {
                    log::warn!("[WS_GC] Error closing session {}: {}", session_id, e);
                }
            }

            remove_session(&session_id);
            log::info!("[WS_GC] Removed expired session: {}", session_id);
        }

        log::info!("[WS_GC] Remaining active sessions: {}", len_sessions());
    }

    fn cleanup_searches_for_session(session_id: &str) {
        let searches_to_remove: Vec<String> = SEARCH_REGISTRY
            .iter()
            .filter_map(|entry| {
                if entry.value().get_req_id() == session_id {
                    Some(entry.key().clone())
                } else {
                    None
                }
            })
            .collect();

        for trace_id in searches_to_remove {
            if let Some((_, state)) = SEARCH_REGISTRY.remove(&trace_id) {
                match state {
                    SearchState::Running { cancel_tx, req_id } => {
                        let _ = cancel_tx.try_send(());
                        log::info!(
                            "[WS_GC] Cancelled running search: {} for session: {}",
                            trace_id,
                            req_id
                        );
                    }
                    _ => {
                        log::debug!(
                            "[WS_GC] Removed search: {} for session: {}",
                            trace_id,
                            session_id
                        );
                    }
                }
            }
        }
    }

    /// Insert a new session into the cache
    pub fn insert_session(session_id: &str, session: WsSession) {
        WS_SESSIONS.insert(session_id.to_string(), session);
    }

    /// Remove a session from the cache
    pub fn remove_session(session_id: &str) {
        WS_SESSIONS.remove(session_id);
    }

    // Return a mutable reference to the session
    pub fn get_mut_session(
        session_id: &str,
    ) -> Option<dashmap::mapref::one::RefMut<'_, String, WsSession>> {
        WS_SESSIONS.get_mut(session_id)
    }

    /// Check if a session exists in the cache
    pub fn contains_session(session_id: &str) -> bool {
        WS_SESSIONS.contains_key(session_id)
    }

    /// Get the number of sessions in the cache
    pub fn len_sessions() -> usize {
        WS_SESSIONS.len()
    }
}

pub mod search_registry_utils {
    use tokio::sync::mpsc;

    use crate::handler::http::request::websocket::session::SEARCH_REGISTRY;

    #[derive(Debug)]
    pub enum SearchState {
        Running {
            cancel_tx: mpsc::Sender<()>,
            req_id: String,
        },
        Cancelled {
            req_id: String,
        },
        Completed {
            req_id: String,
        },
    }

    impl SearchState {
        pub fn get_req_id(&self) -> &str {
            match self {
                SearchState::Running { req_id, .. } => req_id,
                SearchState::Cancelled { req_id } => req_id,
                SearchState::Completed { req_id } => req_id,
            }
        }
    }

    // Add this function to check if a search is cancelled
    pub fn is_cancelled(trace_id: &str) -> Option<bool> {
        SEARCH_REGISTRY
            .get(trace_id)
            .map(|state| matches!(*state, SearchState::Cancelled { .. }))
    }
}

/// Represents the different types of WebSocket client messages that can be sent.
///
/// The `WSClientMessage` enum is used to represent the different types of messages that can be sent
/// from a WebSocket client to the server. It includes two variants:
///
/// - `Search`: Represents a request to search for data, containing a trace ID, a query payload, and
///   a query type.
/// - `Cancel`: Represents a request to cancel a previous search, containing a trace ID.
///
/// The `WSQueryPayload` struct is used to encapsulate the details of a search query, including the
/// SQL query, start time, and end time.
///
/// This enum is serialized and deserialized using the `serde` crate, with the `type` field
/// indicating the variant and the `content` field containing the associated data.
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WsClientEvents {
    Search(Box<SearchEventReq>),
    #[cfg(feature = "enterprise")]
    Cancel {
        trace_id: String,
    },
    Benchmark {
        id: String,
    },
}

impl WsClientEvents {
    pub fn get_type(&self) -> String {
        match self {
            WsClientEvents::Search(_) => "search",
            #[cfg(feature = "enterprise")]
            WsClientEvents::Cancel { .. } => "cancel",
            WsClientEvents::Benchmark { .. } => "benchmark",
        }
        .to_string()
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WsClientEvents")
    }
}

/// To represent the query start and end time based of partition or cache
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct TimeOffset {
    pub start_time: i64,
    pub end_time: i64,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(
    tag = "type",
    content = "content",
    rename_all(serialize = "snake_case", deserialize = "snake_case")
)]
pub enum WsServerEvents {
    SearchResponse {
        trace_id: String,
        results: Box<config::meta::search::Response>,
        time_offset: TimeOffset,
        streaming_aggs: bool,
    },
    #[cfg(feature = "enterprise")]
    CancelResponse {
        trace_id: String,
        is_success: bool,
    },
    Error {
        code: u16,
        message: String,
        error_detail: Option<String>,
        trace_id: Option<String>,
        request_id: Option<String>,
    },
    End {
        trace_id: Option<String>,
    },
}

impl WsServerEvents {
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WsServerEvents")
    }

    pub fn error_response(
        err: Error,
        request_id: Option<String>,
        trace_id: Option<String>,
    ) -> Self {
        match err {
            errors::Error::ErrorCode(code) => Self::Error {
                code: code.get_code(),
                message: code.get_message(),
                error_detail: Some(code.get_error_detail()),
                trace_id: trace_id.clone(),
                request_id: request_id.clone(),
            },
            _ => Self::Error {
                code: StatusCode::INTERNAL_SERVER_ERROR.into(),
                message: err.to_string(),
                error_detail: None,
                trace_id,
                request_id,
            },
        }
    }
}
