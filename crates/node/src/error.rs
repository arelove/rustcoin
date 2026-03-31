use thiserror::Error;

/// Errors that can occur in node operations.
#[derive(Debug, Error)]
pub enum NodeError {
    /// Failed to initialize a node component.
    #[error("init error: {0}")]
    Init(String),
    /// A storage operation failed.
    #[error("storage error: {0}")]
    Storage(String),
    /// A network operation failed.
    #[error("network error: {0}")]
    Network(String),
    /// A consensus operation failed.
    #[error("consensus error: {0}")]
    Consensus(String),
    /// An RPC operation failed.
    #[error("rpc error: {0}")]
    Rpc(String),
}
