use thiserror::Error;

/// Errors that can occur in the P2P network layer.
#[derive(Debug, Error)]
pub enum P2pError {
    /// Failed to initialize the network or a sub-protocol.
    #[error("failed to initialize network: {0}")]
    Init(String),

    /// The provided multiaddr string is invalid.
    #[error("invalid multiaddr")]
    InvalidAddr,

    /// The internal message channel has been closed.
    #[error("channel closed")]
    ChannelClosed,

    /// Failed to encode or decode a network message.
    #[error("encode/decode error: {0}")]
    Codec(String),
}
