use std::sync::Arc;

use actix_web::{web, Error, HttpRequest, HttpResponse};
use futures_util::StreamExt;
use serde_json::json;

use super::{error::*, message::*, types::*};

pub struct WsHandler {
    message_bus: Arc<RouterMessageBus>,
}

impl WsHandler {
    pub fn new(message_bus: Arc<RouterMessageBus>) -> Self {
        Self { message_bus }
    }

    pub async fn handle_connection(
        &self,
        req: HttpRequest,
        stream: web::Payload,
        client_id: ClientId,
    ) -> Result<HttpResponse, Error> {
        let (response, ws_session, mut msg_stream) = actix_ws::handle(&req, stream)?;

        // Create session
        let session_info = self
            .message_bus
            .session_manager
            .create_session(client_id)
            .await
            .map_err(Error::from)?;

        // Setup message channel
        let (tx, mut rx) = tokio::sync::mpsc::channel(32);
        self.message_bus
            .register_client(session_info.session_id.clone(), tx)
            .await
            .map_err(Error::from)?;

        let session_id = session_info.session_id.clone();
        let message_bus = self.message_bus.clone();

        // Spawn message handling tasks
        actix_web::rt::spawn(async move {
            let mut ws_session = ws_session;

            // Handle incoming messages
            let handle_incoming = async {
                while let Some(msg) = msg_stream.next().await {
                    match msg {
                        Ok(actix_ws::Message::Text(text)) => {
                            if let Ok(message) = serde_json::from_str::<Message>(&text) {
                                if let Err(e) =
                                    message_bus.forward_to_querier(&session_id, message).await
                                {
                                    log::error!("Error forwarding message: {}", e);
                                }
                            }
                        }
                        Ok(actix_ws::Message::Close(_)) => break,
                        _ => {}
                    }
                }
            };

            // Handle outgoing messages
            let handle_outgoing = async {
                while let Some(message) = rx.recv().await {
                    if let Err(e) = ws_session.text(serde_json::to_string(&message)?).await {
                        log::error!("Error sending message to client: {}", e);
                        break;
                    }
                }
                Ok::<_, Error>(())
            };

            // Run both tasks concurrently
            let _ = tokio::join!(handle_incoming, handle_outgoing);

            // Cleanup
            message_bus.unregister_client(&session_id).await.ok();
            message_bus
                .session_manager
                .remove_session(&session_id)
                .await
                .ok();
        });

        Ok(response)
    }

    pub async fn shutdown(&self) -> WsResult<()> {
        for client in self.message_bus.get_all_clients().await {
            let message = Message::new(
                "system".into(),
                MessageType::Error,
                json!({ "error": "Server shutting down" }),
            );
            let _ = self.message_bus.forward_to_client(&client, message).await;
        }
        self.message_bus.close_all().await
    }
}
