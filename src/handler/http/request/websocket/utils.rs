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
        use o2_enterprise::enterprise::openfga::meta::mapping::OFGA_MODELS;

        use crate::{
            common::utils::auth::{is_root_user, AuthExtractor},
            service::users::get_user,
        };

        // Check if the user is a root user (has all permissions)
        if is_root_user(user_id) {
            return Ok(());
        }

        // Get user details from the USERS cache
        let user: config::meta::user::User = get_user(Some(org_id), user_id)
            .await
            .ok_or_else(|| "User not found".to_string())?;

        // If the user is external, check permissions
        if user.is_external {
            let stream_type_str = stream_type.to_string();
            let o2_type = format!(
                "{}:{}",
                OFGA_MODELS
                    .get(stream_type_str.as_str())
                    .map_or(stream_type_str.as_str(), |model| model.key),
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
            }
        }

        Ok(())
    }
}

pub mod sessions_cache_utils {
    use crate::{
        common::infra::config::WS_SESSIONS, handler::http::request::websocket::session::WsSession,
    };

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

pub mod cancellation_registry_cache_utils {
    use crate::handler::http::request::websocket::session::CANCELLATION_FLAGS;

    /// Add a new cancellation flag for the given trace_id
    pub fn add_cancellation_flag(trace_id: &str) {
        CANCELLATION_FLAGS.insert(trace_id.to_string(), false);
        log::info!(
            "[WS_CANCEL]: Added cancellation flag for trace_id: {}",
            trace_id
        );
    }

    /// Set the cancellation flag for the given trace_id
    pub fn set_cancellation_flag(trace_id: &str) {
        if let Some(mut flag) = CANCELLATION_FLAGS.get_mut(trace_id) {
            *flag = true; // Set the flag to `true`
            log::info!(
                "[WS_CANCEL]: Cancellation flag set for trace_id: {}",
                trace_id
            );
        } else {
            log::warn!(
                "[WS_CANCEL]: No cancellation flag found for trace_id: {}",
                trace_id
            );
        }
    }

    /// Remove the cancellation flag for the given trace_id
    pub fn remove_cancellation_flag(trace_id: &str) {
        if CANCELLATION_FLAGS.remove(trace_id).is_some() {
            log::info!(
                "[WS_CANCEL]: Cancellation flag removed for trace_id: {}",
                trace_id
            );
        } else {
            log::warn!(
                "[WS_CANCEL]: No cancellation flag found to remove for trace_id: {}",
                trace_id
            );
        }
    }

    /// Check if a cancellation flag is set for the given trace_id
    pub fn is_cancelled(trace_id: &str) -> bool {
        CANCELLATION_FLAGS
            .get(trace_id)
            .map(|flag| *flag)
            .unwrap_or(false)
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
    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("Failed to serialize WsClientEvents")
    }

    pub fn get_type(&self) -> String {
        match self {
            WsClientEvents::Search(_) => "search",
            #[cfg(feature = "enterprise")]
            WsClientEvents::Cancel { .. } => "cancel",
            WsClientEvents::Benchmark { .. } => "benchmark",
        }
        .to_string()
    }
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
        time_offset: i64,
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
