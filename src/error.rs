//! Crypto Arbitrage Engine — Error Definitions

use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("WebSocket error: {0}")]
    WebSocket(#[from] tokio_tungstenite::tungstenite::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Malformed response: {0}")]
    MalformedResponse(String),

    #[error("Rate limited; retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Upstream API error: HTTP {status}")]
    UpstreamStatus { status: u16 },

    #[error("No exchanges returned data")]
    NoExchanges,

    #[allow(dead_code)]
    #[error("Config error: {0}")]
    Config(String),

    #[allow(dead_code)]
    #[error("Network error: {0}")]
    Network(String),
}

pub type Result<T> = std::result::Result<T, Error>;