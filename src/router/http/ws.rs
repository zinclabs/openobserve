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
use tokio::sync::oneshot;
use tokio_tungstenite::{connect_async, tungstenite};
use url::Url;

/// WebSocket proxy handler
pub async fn ws_proxy(
    req: HttpRequest,
    payload: web::Payload,
    ws_base_url: &str,
) -> Result<HttpResponse, Error> {
    // log node role
    let cfg = get_config();
    let node_role = cfg.common.node_role.clone();

    // Upgrade the client connection to a WebSocket
    let (response, mut session, mut client_msg_stream) = actix_ws::handle(&req, payload)?;

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

    // Connect to the backend WebSocket service
    let (backend_ws_stream, _) = connect_async(ws_req).await.map_err(|e| {
        log::error!(
            "[WS_PROXY] Node Role: {} Failed to connect to backend WebSocket service, error: {:?}",
            node_role,
            e
        );
        actix_web::error::ErrorInternalServerError("Failed to connect to backend websocket service")
    })?;

    // Split backend Websocket stream into sink and stream
    let (mut backend_ws_sink, mut backend_ws_stream) = backend_ws_stream.split();

    // Add coordination channels for graceful shutdown
    let (client_kill_tx, client_kill_rx) = oneshot::channel::<()>();
    let (backend_kill_tx, backend_kill_rx) = oneshot::channel::<()>();

    let client_to_backend = {
        let mut client_kill_rx = client_kill_rx;
        async move {
            loop {
                tokio::select! {
                    Some(msg_result) = client_msg_stream.next() => {
                        match msg_result {
                            Ok(msg) => {
                                let ws_msg = from_actix_message(msg);
                                match ws_msg {
                                    tungstenite::protocol::Message::Close(reason) => {
                                        log::info!("[WS_PROXY] Client initiated close ");
                                        let close_msg = tungstenite::protocol::Message::Close(reason.clone());
                                        if let Err(e) = backend_ws_sink.send(close_msg).await {
                                            log::error!("[WS_PROXY] Failed to forward close to backend: {}", e);
                                        }
                                        let _ = backend_kill_tx.send(());
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
                    _ = &mut client_kill_rx => {
                        log::info!("[WS_PROXY] Client task received shutdown signal");
                        break;
                    }
                }
            }
        }
    };

    let backend_to_client = {
        let mut backend_kill_rx = backend_kill_rx;
        let mut closed_normally = false;
        async move {
            loop {
                tokio::select! {
                    Some(msg_result) = backend_ws_stream.next() => {
                        match msg_result {
                            Ok(msg) => {
                                let ws_msg = from_tungstenite_msg_to_actix_msg(msg);
                                match ws_msg {
                                    Message::Close(reason) => {
                                        log::info!("[WS_PROXY] Backend initiated close: {:?}", reason);
                                        // Send close to client
                                        let _ = session.close(reason).await;
                                        // Then signal client to shutdown
                                        let _ = client_kill_tx.send(());
                                        closed_normally = true;
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
                                    Message::Continuation(_) => {
                                        log::warn!("[WS_PROXY] Unsupported message type from backend: {:?}", ws_msg);
                                    }
                                    Message::Nop => {
                                        log::warn!("[WS_PROXY] Unsupported message type from backend: Nop");
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("[WS_PROXY] Backend message error: {:?}", e);
                                break;
                            }
                        }
                    }
                    _ = &mut backend_kill_rx => {
                        log::info!("[WS_PROXY] Backend task received shutdown signal");
                        closed_normally = true;
                        break;
                    }
                }
            }

            if !closed_normally {
                log::warn!("[WS_PROXY] Backend connection closed unexpectedly");
            } else {
                log::info!("[WS_PROXY] Backend connection closed normally");
            }
        }
    };

    // Spawn tasks with cleanup
    let backend_handle = rt::spawn(backend_to_client);
    let client_handle = rt::spawn(client_to_backend);

    // Join the tasks and log when they complete
    rt::spawn(async move {
        let _ = tokio::join!(backend_handle, client_handle);
        log::info!("[WS_PROXY] backend and client tasks completed");
    });

    // Return the WebSocket handshake response
    Ok(response)
}

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
