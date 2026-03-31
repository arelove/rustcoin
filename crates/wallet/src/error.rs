use thiserror::Error;

/// Errors that can occur in wallet operations.
#[derive(Debug, Error)]
pub enum WalletError {
    /// No account found for the given address.
    #[error("account not found")]
    AccountNotFound,

    /// The provided password is incorrect.
    #[error("wrong password")]
    WrongPassword,

    /// A required field is missing.
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// An I/O error occurred.
    #[error("io error: {0}")]
    Io(String),

    /// Failed to serialize or deserialize data.
    #[error("serialization error: {0}")]
    Serialization(String),

    /// An underlying cryptographic error occurred.
    #[error("crypto error: {0}")]
    Crypto(String),
}
