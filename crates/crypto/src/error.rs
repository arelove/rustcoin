//! Ошибки crypto крейта.

use thiserror::Error;

/// Errors that can occur in cryptographic operations.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum CryptoError {
    /// Signature verification failed — the signature does not match the message or public key.
    #[error("signature verification failed")]
    SignatureVerificationFailed,

    /// The provided bytes do not represent a valid Ed25519 public key.
    #[error("invalid public key bytes")]
    InvalidPublicKey,

    /// The provided bytes do not represent a valid Ed25519 signature.
    #[error("invalid signature bytes")]
    InvalidSignature,

    /// The provided bytes do not represent a valid Ed25519 private key.
    #[error("invalid private key bytes")]
    InvalidPrivateKey,
}
