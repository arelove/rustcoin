//! # rc-crypto
//!
//! Криптографические примитивы блокчейна.
//!
//! ## Что внутри
//! - [`Keypair`] — пара ключей Ed25519
//! - [`PublicKey`] — публичный ключ (верификация подписей)
//! - [`PrivateKey`] — приватный ключ (создание подписей, zeroize при drop)
//! - [`Signature`] — Ed25519 подпись
//!
//! ## Почему Ed25519, а не secp256k1?
//! - **Быстрее**: Ed25519 подписывает в 2-3x быстрее secp256k1
//! - **Безопаснее**: нет ситуаций с плохим randomness (детерминированный)
//! - **Компактнее**: ключи 32 байта, подписи 64 байта
//! - **Современнее**: используется в Solana, NEAR, Cosmos
//!   (secp256k1 — Bitcoin/Ethereum, тоже есть в `k256` крейте)

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod error;
pub mod keypair;
pub mod signature;

pub use error::CryptoError;
pub use keypair::{Keypair, PrivateKey, PublicKey};
pub use signature::Signature;
