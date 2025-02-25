use std::sync::Arc;

use dashmap::DashMap;
use tokio::sync::mpsc;

use super::{connection::Connection, error::*, pool::*, session::*, types::*};

pub struct RouterMessageBus {
    session_manager: Arc<RouterSessionManager>,
    connection_pool: Arc<QuerierConnectionPool>,
    client_channels: DashMap<SessionId, mpsc::Sender<Message>>,
}

impl RouterMessageBus {
    pub fn new(
        session_manager: Arc<RouterSessionManager>,
        connection_pool: Arc<QuerierConnectionPool>,
    ) -> Self {
        Self {
            session_manager,
            connection_pool,
            client_channels: DashMap::new(),
        }
    }

    pub async fn register_client(
        &self,
        session_id: SessionId,
        sender: mpsc::Sender<Message>,
    ) -> WsResult<()> {
        self.client_channels.insert(session_id, sender);
        Ok(())
    }

    pub async fn unregister_client(&self, session_id: &SessionId) -> WsResult<()> {
        self.client_channels.remove(session_id);
        Ok(())
    }

    pub async fn forward_to_querier(
        &self,
        session_id: &SessionId,
        message: Message,
    ) -> WsResult<()> {
        let start = std::time::Instant::now();

        // Get session info
        let _session = self
            .session_manager
            .get_session(session_id)
            .await
            .ok_or_else(|| WsError::SessionError("Session not found".into()))?;

        // Get or assign querier for this trace
        let querier_id = if let Some(querier_id) = self
            .session_manager
            .get_querier_for_trace(&message.trace_id)
            .await
        {
            querier_id
        } else {
            // Use consistent hashing to select querier
            let querier_id = self.select_querier(&message.trace_id).await?;
            self.session_manager
                .set_querier_for_trace(message.trace_id.clone(), querier_id.clone())
                .await?;
            querier_id
        };

        // Get connection to querier
        let conn = self.connection_pool.get_connection(&querier_id).await?;

        // Send message
        conn.send_message(message).await?;

        Ok(())
    }

    pub async fn forward_to_client(
        &self,
        session_id: &SessionId,
        message: Message,
    ) -> WsResult<()> {
        let start = std::time::Instant::now();

        if let Some(sender) = self.client_channels.get(session_id) {
            sender
                .send(message)
                .await
                .map_err(|e| WsError::MessageError(e.to_string()))?;
        }

        Ok(())
    }

    async fn select_querier(&self, trace_id: &str) -> WsResult<QuerierId> {
        use crate::common::infra::cluster;

        // Get online querier nodes
        let _nodes = cluster::get_cached_online_querier_nodes(Some(
            config::meta::cluster::RoleGroup::Interactive,
        ))
        .await
        .ok_or_else(|| WsError::QuerierNotAvailable("No queriers available".into()))?;

        // Use consistent hashing to select querier
        let node = cluster::get_node_from_consistent_hash(
            trace_id,
            &config::meta::cluster::Role::Querier,
            None,
        )
        .await
        .ok_or_else(|| WsError::QuerierNotAvailable("Failed to select querier".into()))?;

        Ok(node)
    }

    pub async fn get_all_clients(&self) -> Vec<SessionId> {
        self.client_channels
            .iter()
            .map(|r| r.key().clone())
            .collect()
    }

    pub async fn close_all(&self) -> WsResult<()> {
        for client in self.client_channels.iter() {
            let _ = self.unregister_client(client.key()).await;
        }
        Ok(())
    }
}
