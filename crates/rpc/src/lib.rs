//! # rc-rpc
//!
//! JSON-RPC 2.0 сервер — HTTP API для взаимодействия с нодой.
//!
//! ## Методы
//!
//! ### Chain
//! - `chain_getBlock(hash)`          → Block
//! - `chain_getBlockByHeight(height)` → Block
//! - `chain_getBestBlock()`           → BlockHeader
//! - `chain_getInfo()`                → ChainInfo
//!
//! ### Transactions
//! - `tx_send(raw_tx_hex)`           → TxId
//! - `tx_get(txid)`                  → Transaction
//! - `tx_getReceipt(txid)`           → TxReceipt
//!
//! ### Account
//! - `account_getBalance(address)`   → Amount
//! - `account_getNonce(address)`     → u64
//! - `account_getState(address)`     → AccountState
//!
//! ### Mempool
//! - `mempool_getPending()`          → Vec<TxId>
//! - `mempool_getSize()`             → usize
//!
//! ### Network
//! - `net_getPeers()`                → Vec<PeerInfo>
//! - `net_getPeerCount()`            → usize

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

/// Error types for RPC operations.
pub mod error;
/// Internal module — not part of the public API.
pub mod handlers;
/// RPC server and shared application state.
pub mod server;
/// Request and response types for the JSON-RPC API.
pub mod types;

pub use error::RpcError;
pub use server::{RpcServer, RpcServerConfig};
