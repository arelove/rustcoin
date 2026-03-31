//! # rc-wallet
//!
//! Управление ключами и создание транзакций.
//!
//! ## Возможности
//! - Генерация и хранение ключевых пар (Ed25519)
//! - Подпись транзакций
//! - Шифрованное хранение keystore (AES-256-GCM + Argon2 KDF)
//! - Базовый HD wallet (иерархические детерминированные ключи, BIP-32 идея)

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod builder;
/// Error types for wallet operations.
pub mod error;
pub mod keystore;

pub use builder::TransactionBuilder;
pub use error::WalletError;
pub use keystore::{Keystore, WalletAccount};
