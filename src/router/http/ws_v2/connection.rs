use std::sync::Arc;

use async_trait::async_trait;
use futures_util::{SinkExt, StreamExt};
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{
    connect_async, tungstenite::protocol::Message as WsMessage, MaybeTlsStream, WebSocketStream,
};

use crate::router::http::ws_v2::{error::*, types::*};

type WsStreamType = WebSocketStream<MaybeTlsStream<TcpStream>>;

#[async_trait]
pub trait Connection: Send + Sync {
    async fn connect(&self) -> WsResult<()>;
    async fn disconnect(&self) -> WsResult<()>;
    async fn send_message(&self, message: Message) -> WsResult<()>;
    async fn is_connected(&self) -> bool;
    fn get_name(&self) -> &QuerierName;
}

pub struct QuerierConnection {
    name: QuerierName,
    stream: Arc<Mutex<Option<WsStreamType>>>,
    url: String,
    last_active: std::sync::atomic::AtomicI64,
}

impl QuerierConnection {
    pub async fn new(name: QuerierName, url: String) -> WsResult<Self> {
        let conn = Self {
            name,
            stream: Arc::new(Mutex::new(None)),
            url,
            last_active: std::sync::atomic::AtomicI64::new(chrono::Utc::now().timestamp_micros()),
        };
        conn.connect().await?;
        Ok(conn)
    }

    fn update_last_active(&self) {
        self.last_active.store(
            chrono::Utc::now().timestamp_micros(),
            std::sync::atomic::Ordering::SeqCst,
        );
    }
}

#[async_trait]
impl Connection for QuerierConnection {
    async fn connect(&self) -> WsResult<()> {
        let mut stream_guard = self.stream.lock().await;
        if stream_guard.is_none() {
            let (ws_stream, _) = connect_async(&self.url)
                .await
                .map_err(|e| WsError::ConnectionError(e.to_string()))?;
            *stream_guard = Some(ws_stream);
        }
        Ok(())
    }

    async fn disconnect(&self) -> WsResult<()> {
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            stream
                .close(None)
                .await
                .map_err(|e| WsError::ConnectionError(e.to_string()))?;
            *stream_guard = None;
        }
        Ok(())
    }

    async fn send_message(&self, message: Message) -> WsResult<()> {
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            let ws_message = WsMessage::Text(
                serde_json::to_string(&message)
                    .map_err(|e| WsError::MessageError(e.to_string()))?,
            );
            stream
                .send(ws_message)
                .await
                .map_err(|e| WsError::MessageError(e.to_string()))?;
            self.update_last_active();
            Ok(())
        } else {
            Err(WsError::ConnectionError("Not connected".into()))
        }
    }

    async fn is_connected(&self) -> bool {
        let stream_guard = self.stream.lock().await;
        stream_guard.is_some()
    }

    fn get_name(&self) -> &QuerierName {
        &self.name
    }
}

impl QuerierConnection {
    pub async fn receive_message(&self) -> WsResult<Option<Message>> {
        let mut stream_guard = self.stream.lock().await;
        if let Some(stream) = stream_guard.as_mut() {
            match stream.next().await {
                Some(Ok(WsMessage::Text(text))) => serde_json::from_str(&text)
                    .map(Some)
                    .map_err(|e| WsError::MessageError(e.to_string())),
                Some(Ok(WsMessage::Close(_))) => Ok(None),
                Some(Err(e)) => Err(WsError::MessageError(e.to_string())),
                _ => Ok(None),
            }
        } else {
            Err(WsError::ConnectionError("Not connected".into()))
        }
    }
}
