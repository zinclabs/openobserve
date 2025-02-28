use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::Mutex;

use super::{
    config::*,
    connection::{Connection, QuerierConnection},
    error::*,
    types::*,
};
use crate::common::infra::cluster;

pub struct QuerierConnectionPool {
    connections: DashMap<QuerierName, Arc<QuerierConnection>>,
    config: WsConfig,
}

impl QuerierConnectionPool {
    pub fn new(config: WsConfig) -> Self {
        Self {
            connections: DashMap::new(),
            config,
        }
    }

    pub async fn get_connection(
        &self,
        querier_name: &QuerierName,
    ) -> WsResult<Arc<QuerierConnection>> {
        if let Some(conn) = self.connections.get(querier_name) {
            return Ok(conn.clone());
        }

        // Create new connection
        let conn = self.create_connection(querier_name).await?;
        self.connections.insert(querier_name.clone(), conn.clone());
        Ok(conn)
    }

    async fn create_connection(
        &self,
        querier_name: &QuerierName,
    ) -> WsResult<Arc<QuerierConnection>> {
        // Get querier info from cluster
        let node = cluster::get_cached_node_by_name(querier_name)
            .await
            .ok_or_else(|| WsError::QuerierNotAvailable(querier_name.clone()))?;

        // Convert HTTP URL to WebSocket URL
        let ws_url = crate::router::http::ws::convert_to_websocket_url(&node.http_addr)
            .map_err(|e| WsError::ConnectionError(e))?;

        let conn = QuerierConnection::new(querier_name.clone(), ws_url).await?;
        Ok(Arc::new(conn))
    }

    pub async fn remove_connection(&self, querier_name: &QuerierName) -> WsResult<()> {
        if let Some((_, conn)) = self.connections.remove(querier_name) {
            conn.disconnect().await?;
        }
        Ok(())
    }

    pub async fn maintain_connections(&self) {
        loop {
            let mut to_remove = Vec::new();

            for conn_ref in self.connections.iter() {
                let querier_name = conn_ref.key().clone();
                let conn = conn_ref.value();

                if !conn.is_connected().await {
                    // Try to reconnect
                    if let Err(e) = conn.connect().await {
                        log::error!("Failed to reconnect to querier {}: {}", conn.get_name(), e);
                        to_remove.push(querier_name);
                    }
                }
            }

            // Remove connections that failed to reconnect
            for querier in to_remove {
                log::warn!("Removing disconnected connection to querier: {}", querier);
                self.connections.remove(&querier);
            }

            tokio::time::sleep(std::time::Duration::from_secs(
                self.config.health_check_config.interval_secs,
            ))
            .await;
        }
    }
}
