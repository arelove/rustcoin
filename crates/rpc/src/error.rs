use thiserror::Error;

/// Errors that can occur in RPC server operations.
#[derive(Debug, Error)]
pub enum RpcError {
    /// The requested JSON-RPC method does not exist.
    #[error("method not found: {0}")]
    MethodNotFound(String),
    /// One or more parameters are invalid or missing.
    #[error("invalid params: {0}")]
    InvalidParams(String),
    /// An unexpected internal server error occurred.
    #[error("internal error: {0}")]
    Internal(String),
    /// Failed to bind the server to the configured address.
    #[error("failed to bind address: {0}")]
    Bind(String),
    /// The server encountered an error while serving requests.
    #[error("server error: {0}")]
    Serve(String),
}

impl RpcError {
    /// JSON-RPC 2.0 код ошибки
    pub fn code(&self) -> i32 {
        match self {
            Self::MethodNotFound(_) => -32601,
            Self::InvalidParams(_) => -32602,
            Self::Internal(_) => -32603,
            _ => -32000,
        }
    }
}
