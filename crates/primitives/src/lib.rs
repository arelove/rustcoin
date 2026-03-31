//! # rc-primitives
//!
//! Core blockchain types shared across all crates.
//! This crate has **zero** async dependencies — everything here is pure data.
//!
//! ## Types
//! - [`Hash`] — 32-byte SHA-256 digest
//! - [`Address`] — 20-byte wallet address (`Base58Check` encoded)
//! - [`Block`] — blockchain block with header + transactions
//! - [`Transaction`] — signed value transfer
//! - [`BlockHeader`] — block metadata (separable for light clients)

#![forbid(unsafe_code)] // запрет unsafe во всём крейте
#![deny(missing_docs)]
// все публичные элементы должны быть задокументированы
#![deny(clippy::all, clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]

pub mod block;
pub mod error;
pub mod hash;
pub mod transaction;
pub mod types;

// Реэкспорт для удобного импорта: `use rc_primitives::Block;`
pub use block::{Block, BlockHeader};
pub use error::PrimitivesError;
pub use hash::Hash;
pub use transaction::{Transaction, TxId};
pub use types::{Address, Amount, BlockHeight, Nonce, Timestamp};
