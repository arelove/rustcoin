//! Ошибки primitives крейта.
//!
//! Используем `thiserror` — он генерирует impl Display и impl Error автоматически.

use thiserror::Error;

/// Все возможные ошибки в primitives
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum PrimitivesError {
    /// Invalid address format or value.
    #[error("invalid address: {0}")]
    InvalidAddress(String),

    /// Transaction failed validation.
    #[error("invalid transaction: {0}")]
    InvalidTransaction(String),

    /// Block failed validation.
    #[error("invalid block: {0}")]
    InvalidBlock(String),

    /// Merkle root does not match the transactions.
    #[error("invalid merkle root")]
    InvalidMerkleRoot,

    /// Block does not meet the required proof-of-work target.
    #[error("insufficient proof of work")]
    InsufficientProofOfWork,

    /// Failed to serialize or deserialize data.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Arithmetic overflow when handling amounts.
    #[error("overflow in amount calculation")]
    AmountOverflow,
}
