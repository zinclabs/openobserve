use chrono::{DateTime, Utc};
use config::meta::websocket::SearchEventReq;
use serde::{Deserialize, Serialize};

use crate::handler::http::request::websocket::utils::{TimeOffset, WsClientEvents, WsServerEvents};

pub type SessionId = String;
pub type ClientId = String;
pub type QuerierName = String;
pub type TraceId = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub trace_id: String,
    pub message_type: MessageType,
    pub payload: serde_json::Value,
    #[serde(skip)]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageType {
    Search(Box<SearchEventReq>),
    SearchResponse,
    #[cfg(feature = "enterprise")]
    Cancel,
    #[cfg(feature = "enterprise")]
    CancelResponse,
    Error,
    End,
    Benchmark,
}

impl Message {
    pub fn new(trace_id: String, message_type: MessageType, payload: serde_json::Value) -> Self {
        Self {
            trace_id,
            message_type,
            payload,
            timestamp: Utc::now(),
        }
    }

    pub fn from_client_event(event: WsClientEvents) -> Self {
        match event {
            WsClientEvents::Search(req) => {
                let trace_id = req.trace_id.clone();
                let payload = serde_json::to_value(&req).unwrap_or_default();
                Self::new(trace_id, MessageType::Search(req), payload)
            }
            #[cfg(feature = "enterprise")]
            WsClientEvents::Cancel { trace_id } => Self::new(
                MessageType::Cancel,
                serde_json::json!({"trace_id": trace_id}),
            ),
            WsClientEvents::Benchmark { id } => Self::new(
                id.clone(),
                MessageType::Benchmark,
                serde_json::json!({"id": id}),
            ),
        }
    }

    pub fn to_server_event(&self) -> Option<WsServerEvents> {
        match self.message_type {
            MessageType::SearchResponse => {
                let response: Box<config::meta::search::Response> =
                    serde_json::from_value(self.payload.clone()).ok()?;
                Some(WsServerEvents::SearchResponse {
                    trace_id: self.trace_id.clone(),
                    results: response,
                    time_offset: TimeOffset {
                        start_time: 0,
                        end_time: 0,
                    },
                    streaming_aggs: false,
                })
            }
            MessageType::Error => {
                let error: ErrorPayload = serde_json::from_value(self.payload.clone()).ok()?;
                Some(WsServerEvents::Error {
                    code: error.code,
                    message: error.message,
                    error_detail: error.error_detail,
                    trace_id: Some(self.trace_id.clone()),
                    request_id: None,
                })
            }
            MessageType::End => Some(WsServerEvents::End {
                trace_id: Some(self.trace_id.clone()),
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct ErrorPayload {
    code: u16,
    message: String,
    error_detail: Option<String>,
}
