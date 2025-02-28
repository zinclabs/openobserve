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

use std::{str::FromStr, sync::Arc};

use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_ws::{CloseReason, Message};
use config::get_config;
use futures_util::{SinkExt, StreamExt};
use reqwest::header::{HeaderName, HeaderValue};
use tokio::sync::{mpsc, oneshot, Mutex};
use tokio_tungstenite::{connect_async, tungstenite};
use url::Url;

#[derive(Debug)]
pub enum WebSocketMessage {
    Data(String),
    Close(Option<CloseReason>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Binary(Vec<u8>),
}

pub struct WsProxySession {
    session: Option<actix_ws::Session>,
    tx: mpsc::Sender<WebSocketMessage>,
    rx: Option<mpsc::Receiver<WebSocketMessage>>,
    shutdown: Option<oneshot::Sender<()>>,
}

impl WsProxySession {
    pub fn new(session: actix_ws::Session) -> Self {
        let (tx, rx) = mpsc::channel(32);
        let (shutdown_tx, _) = oneshot::channel();

        Self {
            session: Some(session),
            tx,
            rx: Some(rx),
            shutdown: Some(shutdown_tx),
        }
    }

    pub fn start(&mut self) -> Result<(), Error> {
        let rx = self
            .rx
            .take()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("Session already started"))?;

        let session = self
            .session
            .take()
            .ok_or_else(|| actix_web::error::ErrorInternalServerError("Session not available"))?;

        let shutdown_rx = oneshot::channel().1;

        tokio::spawn(async move {
            Self::process_messages(rx, session, shutdown_rx).await;
        });

        Ok(())
    }

    async fn process_messages(
        mut rx: mpsc::Receiver<WebSocketMessage>,
        mut session: actix_ws::Session,
        mut shutdown_rx: oneshot::Receiver<()>,
    ) {
        loop {
            tokio::select! {
                Some(msg) = rx.recv() => {
                    let result = match msg {
                        WebSocketMessage::Data(text) => session.text(text).await,
                        WebSocketMessage::Close(reason) => {
                            match session.close(reason).await {
                                Ok(_) => {
                                    break;
                                }
                                Err(e) => {
                                    log::error!("[WS_PROXY] Failed to close session: {}", e);
                                    break;
                                }
                            };
                        },
                        WebSocketMessage::Ping(bytes) => session.ping(&bytes).await,
                        WebSocketMessage::Pong(bytes) => session.pong(&bytes).await,
                        WebSocketMessage::Binary(data) => session.binary(data).await,
                    };

                    if result.is_err() {
                        break;
                    }
                }
                _ = &mut shutdown_rx => {
                    log::info!("[WS_PROXY] Shutting down message processor");
                    break;
                }
            }
        }
    }

    pub async fn send_message(&self, msg: WebSocketMessage) -> Result<(), Error> {
        self.tx.send(msg).await.map_err(|e| {
            log::error!("[WS_PROXY] Failed to queue message: {:?}", e);
            actix_web::error::ErrorInternalServerError("Message queue full")
        })
    }
}

// When dropping WsProxySession, trigger shutdown
impl Drop for WsProxySession {
    fn drop(&mut self) {
        if let Some(shutdown) = self.shutdown.take() {
            let _ = shutdown.send(());
        }
    }
}

/// WebSocket proxy that manages bidirectional communication using two concurrent tasks.
///
/// # Architecture Overview
/// ```text
/// +--------+     +-----------------Router Process----------------+     +---------+
/// |        |     |  +-------------+        +-------------+       |     |         |
/// |        |     |  |    Task 1   |        |    Task 2   |       |     |         |
/// |Client  |<--->|  |client_to_bkd|  <-->  |backend_to_cl|       |<--->| Backend |
/// |        |     |  +-------------+        +-------------+       |     |         |
/// +--------+     +-----------------------------------------------+     +---------+
/// ```
///
/// # Close Sequence Flows
/// ```text
/// 1. Client Initiates Close:
///    Client -> Task1 -> Backend
///                    -> Send Close Frame
///                    -> Break Task1
///    Backend -> Task2 -> Client
///                    -> Send Close Ack to Backend
///                    -> Close Sink
///                    -> Break Task2
///
/// 2. Backend Initiates Close:
///    Backend -> Task2 -> Client
///                    -> Send Close Ack to Backend
///                    -> Close Sink
///                    -> Break Task2
///    Client -> Task1 -> Break Task1
/// ```
///
/// # Implementation Details
/// - Uses Arc<Mutex> for shared sink access between tasks
/// - Each task handles one direction of message flow
/// - Close sequence ensures proper WebSocket protocol shutdown
/// - Automatic resource cleanup when tasks complete
///
/// # Error Handling
/// - Connection errors trigger cleanup in both directions
/// - Timeouts prevent resource leaks
/// - Automatic task termination on connection close
///
/// # Message Flow Example
/// ```text
/// Normal Message:
///   Client -> Task1 -> convert format -> send to Backend
///   Backend -> Task2 -> convert format -> send to Client
///
/// Close Message:
///   1. Receive close frame
///   2. Forward to other endpoint
///   3. Send acknowledgment
///   4. Clean up resources
/// ```
pub async fn ws_proxy(
    req: HttpRequest,
    payload: web::Payload,
    ws_base_url: &str,
) -> Result<HttpResponse, Error> {
    let cfg = get_config();
    let node_role = cfg.common.node_role.clone();

    // Session 1: Client<->Router WebSocket connection
    let (response, session, mut client_msg_stream) = actix_ws::handle(&req, payload)?;
    let mut proxy_session = WsProxySession::new(session);
    proxy_session.start()?;

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

    // Split the stream and wrap sink in Arc<Mutex>
    let (backend_ws_sink, mut backend_ws_stream) = backend_ws_stream.split();
    // Create a new sink for task 1
    let backend_ws_sink = Arc::new(Mutex::new(backend_ws_sink));
    // Create a new sink for task 2
    let backend_ws_sink2 = backend_ws_sink.clone();

    // Task 1: Client to Backend
    let client_to_backend = async move {
        while let Some(msg_result) = client_msg_stream.next().await {
            match msg_result {
                Ok(msg) => {
                    let ws_msg = from_actix_message(msg);
                    match ws_msg {
                        tungstenite::protocol::Message::Close(reason) => {
                            let mut sink = backend_ws_sink.lock().await;
                            // Send close frame through sink
                            let close_msg = tungstenite::protocol::Message::Close(reason);
                            if let Err(e) = sink.send(close_msg).await {
                                log::error!("[WS_PROXY] Failed to forward close: {}", e);
                            }
                            break;
                        }
                        _ => {
                            let mut sink = backend_ws_sink.lock().await;
                            if sink.send(ws_msg).await.is_err() {
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

    // Task 2: Backend to Client
    let backend_to_client = async move {
        tokio::select! {
            _ = async {
                while let Some(msg_result) = backend_ws_stream.next().await {
                    match msg_result {
                        Ok(msg) => {
                            let ws_msg = from_tungstenite_msg_to_actix_msg(msg);
                            match ws_msg {
                                Message::Text(text) => {
                                    if proxy_session.send_message(WebSocketMessage::Data(text.to_string())).await.is_err() {
                                        break;
                                    }
                                }
                                Message::Binary(bin) => {
                                    if proxy_session.send_message(WebSocketMessage::Binary(bin.to_vec())).await.is_err() {
                                        break;
                                    }
                                }
                                Message::Ping(ping) => {
                                    if proxy_session.send_message(WebSocketMessage::Ping(ping.to_vec())).await.is_err() {
                                        break;
                                    }
                                }
                                Message::Pong(pong) => {
                                    if proxy_session.send_message(WebSocketMessage::Pong(pong.to_vec())).await.is_err() {
                                        break;
                                    }
                                }
                                Message::Close(reason) => {
                                    log::info!("[WS_PROXY] Backend -> Router close");

                                    // First send close through the message queue
                                    if let Err(e) = proxy_session.send_message(WebSocketMessage::Close(reason)).await {
                                        log::error!("[WS_PROXY] Failed to queue close message: {}", e);
                                    }

                                    // Then close the backend sink
                                    let mut sink = backend_ws_sink2.lock().await;
                                    if let Err(e) = sink.close().await {
                                        log::error!("[WS_PROXY] Failed to close backend sink: {}", e);
                                    }
                                    break;
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
            } => {}
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
