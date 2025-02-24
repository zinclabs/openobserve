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

use std::str::FromStr;

use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_ws::Message;
use config::get_config;
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderName, HeaderValue};
use tokio_tungstenite::{connect_async, tungstenite};
use url::Url;

/// WebSocket proxy that manages bidirectional communication using two concurrent tasks.
///
/// # Architecture Overview
/// ```text
/// +--------+     +-----------------Router Process----------------+     +---------+
/// |        |     |  +-------------+        +-------------+       |     |         |
/// |        |     |  |    Task 1   |        |    Task 2   |       |     |         |
/// |        |     |  |client_to_bkd|        |backend_to_cl|       |     |         |
/// |        |     |  +-------------+        +-------------+       |     |         |
/// |        |     |    ↑    |               |     ↓               |     |         |
/// |Client  |<--->| msg_stream  ws_sink   ws_stream  session      |<--->| Backend |
/// |        |     |                                               |     |         |
/// +--------+     +-----------------------------------------------+     +---------+
/// ```
///
/// # Stream Flow
/// ```text
/// 1. Client -> Backend (Task 1: client_to_backend)
///    client_msg_stream -> convert format -> backend_ws_sink
///
/// 2. Backend -> Client (Task 2: backend_to_client)
///    backend_ws_stream -> convert format -> session.send
/// ```
///
/// # Key Features
/// - Two independent tasks handle each direction
/// - No shared state between tasks
/// - Tasks complete naturally on connection close
/// - Automatic message format conversion between protocols
///
/// # Example Message Flow
/// ```text
/// Client Message:
///   1. Client sends message
///   2. Task 1 receives via client_msg_stream
///   3. Task 1 converts format (actix -> tungstenite)
///   4. Task 1 sends to backend via ws_sink
///
/// Backend Message:
///   1. Backend sends message
///   2. Task 2 receives via ws_stream
///   3. Task 2 converts format (tungstenite -> actix)
///   4. Task 2 sends to client via session
/// ```
pub async fn ws_proxy(
    req: HttpRequest,
    payload: web::Payload,
    ws_base_url: &str,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();
    let node_role = cfg.common.node_role.clone();

    // Session 1: Client<->Router WebSocket connection
    let (response, mut session, mut client_msg_stream) = actix_ws::handle(&req, payload)?;

    // Prepare backend connection request
    let ws_req = match convert_actix_to_tungstenite_request(&req, ws_base_url) {
        Ok(req) => req,
        Err(e) => {
            log::error!(
                "[WS_PROXY] Failed to convert Actix request to Tungstenite request: {:?}",
                e
            );
            return Err(actix_web::error::ErrorInternalServerError(
                "Failed to convert actix request to tungstenite request",
            ));
        }
    };

    // Session 2: Router<->Backend WebSocket connection
    let (backend_ws_stream, _) = connect_async(ws_req).await.map_err(|e| {
        log::error!(
            "[WS_PROXY] Node Role: {} Failed to connect to backend WebSocket service, error: {:?}",
            node_role,
            e
        );
        actix_web::error::ErrorInternalServerError("Failed to connect to backend websocket service")
    })?;
    let (mut backend_ws_sink, mut backend_ws_stream) = backend_ws_stream.split();

    // Task 1: Forward messages from Client to Backend
    let client_to_backend = async move {
        while let Some(msg_result) = client_msg_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    let ws_msg = from_actix_message(msg);
                    match ws_msg {
                        tungstenite::protocol::Message::Close(reason) => {
                            // Forward close to backend
                            log::info!("[WS_PROXY] Client -> Router close");
                            let close_msg = tungstenite::protocol::Message::Close(reason.clone());
                            if let Err(e) = backend_ws_sink.send(close_msg).await {
                                log::error!("[WS_PROXY] Failed to forward close: {}", e);
                            }
                            log::info!("[WS_PROXY] Client -> Router close completed");
                            break;
                        }
                        _ => {
                            if backend_ws_sink.send(ws_msg).await.is_err() {
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    log::error!("[WS_PROXY] Client error: {:?}", e);
                    break;
                }
            }
        }
    };

    // Task 2: Forward messages from Backend to Client
    let backend_to_client = async move {
        while let Some(msg_result) = backend_ws_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    let ws_msg = from_tungstenite_msg_to_actix_msg(msg);
                    match ws_msg {
                        Message::Close(reason) => {
                            // This handles both:
                            // 1. Backend initiated close
                            // 2. Backend's acknowledgment of client's close
                            log::info!("[WS_PROXY] Backend -> Router close");
                            if let Err(e) = session.close(reason).await {
                                log::error!("[WS_PROXY] Failed to close client: {}", e);
                            }
                            log::info!("[WS_PROXY] Backend -> Router close completed");
                            break;
                        }
                        Message::Text(text) => {
                            if session.text(text).await.is_err() {
                                break;
                            }
                        }
                        Message::Binary(bin) => {
                            if session.binary(bin).await.is_err() {
                                break;
                            }
                        }
                        Message::Ping(ping) => {
                            if session.ping(&ping).await.is_err() {
                                break;
                            }
                        }
                        Message::Pong(pong) => {
                            if session.pong(&pong).await.is_err() {
                                break;
                            }
                        }
                        _ => log::warn!("[WS_PROXY] Unsupported message type: {:?}", ws_msg),
                    }
                }
                Err(e) => {
                    log::error!("[WS_PROXY] Backend error: {:?}", e);
                    break;
                }
            }
        }
    };

    // Spawn tasks
    rt::spawn(async move {
        let _ = tokio::join!(rt::spawn(client_to_backend), rt::spawn(backend_to_client));
        log::info!("[WS_PROXY] WebSocket proxy completed");
    });

    Ok(response)
}

/// Convert actix-web WebSocket message to tungstenite message format
fn from_actix_message(msg: Message) -> tungstenite::protocol::Message {
    match msg {
        Message::Text(text) => tungstenite::protocol::Message::Text(text.to_string()),
        Message::Binary(bin) => tungstenite::protocol::Message::Binary(bin.to_vec()),
        Message::Ping(msg) => tungstenite::protocol::Message::Ping(msg.to_vec()),
        Message::Pong(msg) => tungstenite::protocol::Message::Pong(msg.to_vec()),
        Message::Close(reason) => {
            log::info!(
                "[WS_PROXY] Received a Message::Close with reason: {:#?}, closing connection to proxied server",
                reason
            );
            tungstenite::protocol::Message::Close(reason.map(|r| {
                tungstenite::protocol::CloseFrame {
                    code: u16::from(r.code).into(),
                    reason: r.description.unwrap_or_default().into(),
                }
            }))
        }
        _ => {
            log::info!("[WS_PROXY] Unsupported message type {:?}", msg);
            tungstenite::protocol::Message::Close(None)
        }
    }
}

/// Convert tungstenite WebSocket message to actix-web message format
fn from_tungstenite_msg_to_actix_msg(msg: tungstenite::protocol::Message) -> Message {
    match msg {
        tungstenite::protocol::Message::Text(text) => Message::Text(text.into()),
        tungstenite::protocol::Message::Binary(bin) => Message::Binary(bin.into()),
        tungstenite::protocol::Message::Ping(msg) => Message::Ping(msg.into()),
        tungstenite::protocol::Message::Pong(msg) => Message::Pong(msg.into()),
        tungstenite::protocol::Message::Close(reason) => {
            Message::Close(reason.map(|r| actix_ws::CloseReason {
                code: u16::from(r.code).into(),
                description: Some(r.reason.to_string()),
            }))
        }
        _ => {
            log::info!("[WS_PROXY] Unsupported message type {:?}", msg);
            Message::Close(None)
        }
    }
}

/// Helper function to convert an HTTP/HTTPS URL to a WebSocket URL
pub fn convert_to_websocket_url(url_str: &str) -> Result<String, String> {
    let mut parsed_url = match Url::parse(url_str) {
        Ok(url) => url,
        Err(e) => {
            return Err(format!("Failed to parse URL: {}", e));
        }
    };

    // Check the scheme and update it accordingly
    match parsed_url.scheme() {
        "http" => {
            parsed_url
                .set_scheme("ws")
                .map_err(|_| "Failed to set scheme to ws")?;
        }
        "https" => {
            parsed_url
                .set_scheme("wss")
                .map_err(|_| "Failed to set scheme to wss")?;
        }
        _ => {
            return Err(format!("Unsupported URL scheme: {}", parsed_url.scheme()));
        }
    }

    Ok(parsed_url.to_string())
}

/// Helper function to convert an Actix HttpRequest to a Tungstenite WebSocket Request
pub fn convert_actix_to_tungstenite_request(
    actix_req: &HttpRequest,
    ws_base_url: &str,
) -> Result<tungstenite::http::Request<()>, Box<dyn std::error::Error>> {
    let mut url = Url::parse(ws_base_url)?;
    let query_string = actix_req.query_string();
    if !query_string.is_empty() {
        url.set_query(Some(query_string));
    }

    let uri: tungstenite::http::Uri = url.as_str().parse()?;

    let method = reqwest::Method::from_str(actix_req.method().as_str()).unwrap();
    let mut headers = tokio_tungstenite::tungstenite::http::HeaderMap::new();
    for (key, value) in actix_req.headers().iter() {
        if let Ok(header_str) = value.to_str() {
            if let Ok(header_name) = HeaderName::from_str(key.as_str()) {
                if let Ok(header_value) = HeaderValue::from_str(header_str) {
                    headers.insert(header_name, header_value);
                }
            }
        }
    }

    // insert headers for websockets, connection upgrade and upgrade to websocket
    headers.insert(
        HeaderName::from_static("connection"),
        HeaderValue::from_static("upgrade"),
    );
    headers.insert(
        HeaderName::from_static("upgrade"),
        HeaderValue::from_static("websocket"),
    );

    // Build the WebSocket request using the extracted method, URI, and headers
    let mut request_builder = tungstenite::http::Request::builder()
        .method(method)
        .uri(uri);

    for (key, value) in headers.iter() {
        request_builder = request_builder.header(key, value);
    }

    let ws_request = request_builder.body(())?;

    Ok(ws_request)
}
