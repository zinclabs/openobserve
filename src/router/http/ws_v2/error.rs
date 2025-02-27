use actix_web::{error::ResponseError, http::StatusCode};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WsError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Session error: {0}")]
    SessionError(String),

    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Message error: {0}")]
    MessageError(String),

    #[error("Querier not available: {0}")]
    QuerierNotAvailable(String),

    #[error("Circuit breaker open for querier: {0}")]
    CircuitBreakerOpen(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

impl ResponseError for WsError {
    fn status_code(&self) -> StatusCode {
        match self {
            WsError::ConnectionError(_) => StatusCode::SERVICE_UNAVAILABLE,
            WsError::SessionError(_) => StatusCode::BAD_REQUEST,
            WsError::SessionNotFound(_) => StatusCode::BAD_REQUEST,
            WsError::MessageError(_) => StatusCode::BAD_REQUEST,
            WsError::QuerierNotAvailable(_) => StatusCode::SERVICE_UNAVAILABLE,
            WsError::CircuitBreakerOpen(_) => StatusCode::SERVICE_UNAVAILABLE,
            WsError::Timeout(_) => StatusCode::REQUEST_TIMEOUT,
            WsError::Other(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type WsResult<T> = Result<T, WsError>;
