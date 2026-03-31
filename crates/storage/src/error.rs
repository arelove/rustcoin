use thiserror::Error;

/// Errors that can occur in storage operations.
#[derive(Debug, Error)]
pub enum StorageError {
    /// Failed to open the database.
    #[error("failed to open database: {0}")]
    Open(String),
    /// A write operation failed.
    #[error("write error: {0}")]
    Write(String),
    /// A read operation failed.
    #[error("read error: {0}")]
    Read(String),
    /// Failed to serialize or deserialize a value.
    #[error("serialization error: {0}")]
    Serialization(String),
    /// The database contains corrupt or unexpected data.
    #[error("database corruption: {0}")]
    Corruption(String),
    /// The account has insufficient balance for the operation.
    #[error("insufficient balance")]
    InsufficientBalance,
    /// An arithmetic overflow occurred when handling an amount.
    #[error("amount overflow")]
    Overflow,
    /// No block found for the given hash or height.
    #[error("block not found: {0}")]
    BlockNotFound(String),
}
