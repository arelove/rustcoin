use thiserror::Error;

/// Errors that can occur in consensus operations.
#[derive(Debug, Error)]
pub enum ConsensusError {
    /// Mining was cancelled, typically because a new block arrived from the network.
    #[error("mining was cancelled (new block received from network)")]
    MiningCancelled,
    /// The block does not meet the required proof-of-work target.
    #[error("insufficient proof of work")]
    InsufficientWork,
    /// The block references a parent that is not known to this node.
    #[error("unknown parent block: {0}")]
    UnknownParent(String),
    /// An unexpected internal error occurred.
    #[error("internal consensus error: {0}")]
    Internal(String),
}
