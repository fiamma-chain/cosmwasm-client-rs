use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("gRPC error: {0}")]
    GrpcError(String),

    #[error("WebSocket error: {0}")]
    WebSocketError(String),

    #[error("Encoding error: {0}")]
    EncodingError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Event error: {0}")]
    EventError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Timeout error: {0}")]
    TimeoutError(String),

    #[error("Signing error: {0}")]
    SigningError(String),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;
