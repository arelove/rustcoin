use rc_primitives::transaction::TxId;
use thiserror::Error;

/// Errors that can occur in mempool operations.
#[derive(Debug, Error)]
pub enum MempoolError {
    /// A transaction with this TxId already exists in the pool.
    #[error("duplicate transaction: {0}")]
    DuplicateTransaction(TxId),
    /// The transaction failed basic validation.
    #[error("invalid transaction: {0}")]
    InvalidTransaction(String),
    /// The mempool is full and the transaction could not be evicted.
    #[error("mempool is full")]
    Full,
    /// No transaction found for the given TxId.
    #[error("transaction not found")]
    NotFound,
}
