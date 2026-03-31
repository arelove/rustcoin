//! # rc-storage
//!
//! Персистентное хранилище на базе RocksDB.
//!
//! ## Структура Column Families (CF)
//!
//! RocksDB поддерживает "column families" — независимые пространства ключей.
//! Мы используем их для разделения типов данных:
//!
//! | Column Family | Ключ              | Значение            |
//! |---------------|-------------------|---------------------|
//! | `blocks`      | `Hash` (32 байта) | `Block` (JSON)      |
//! | `headers`     | `Hash` (32 байта) | `BlockHeader` (JSON)|
//! | `heights`     | `u64` (big-endian)| `Hash` (32 байта)   |
//! | `txs`         | `TxId` (32 байта) | `Transaction` (JSON)|
//! | `state`       | `Address` (20 байт)| `AccountState` (JSON)|
//! | `meta`        | UTF-8 строка      | произвольный JSON   |
//!
//! ## Почему big-endian для высоты?
//! RocksDB хранит ключи в лексикографическом порядке.
//! Big-endian u64 сортируется так же как числа → можно эффективно
//! делать range scan по высотам блоков.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod account;
pub mod db;
/// Error types for storage operations.
pub mod error;
pub mod keys;
/// State transition function — применение блоков к стейту.
pub mod state_machine;

pub use account::AccountState;
pub use db::Database;
pub use error::StorageError;
pub use state_machine::ApplyResult;
