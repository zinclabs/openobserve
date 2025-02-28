mod config;
mod connection;
mod error;
mod handler;
mod message;
mod pool;
mod session;
mod types;

use std::sync::Arc;

pub use config::WsConfig;
pub use error::WsResult;
pub use handler::WsHandler;
pub use message::RouterMessageBus;
pub use pool::QuerierConnectionPool;
pub use session::RouterSessionManager;

pub async fn init() -> WsResult<Arc<WsHandler>> {
    let config = WsConfig::default();
    let session_manager = Arc::new(RouterSessionManager::new());
    let connection_pool = Arc::new(QuerierConnectionPool::new(config.clone()));

    let message_bus = Arc::new(RouterMessageBus::new(
        session_manager,
        connection_pool.clone(),
    ));

    let handler = Arc::new(WsHandler::new(message_bus));

    // Start connection maintenance task
    tokio::spawn(async move {
        connection_pool.maintain_connections().await;
    });

    Ok(handler)
}
