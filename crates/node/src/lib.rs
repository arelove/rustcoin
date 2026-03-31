//! # rc-node
//!
//! Полная нода блокчейна — оркестрирует все крейты.
//!
//! ## Архитектура (Actor-like)
//!
//! ```text
//!                    ┌─────────────────────────────────────────┐
//!                    │                  Node                    │
//!                    │                                         │
//!   Network ◄────────┤  P2P Layer     ←→  Event Bus           │
//!                    │      │                  │               │
//!   RPC clients ◄───┤  RPC Server         Consensus Engine    │
//!                    │                         │               │
//!                    │  Mempool  ←─────────────┤               │
//!                    │      │                  │               │
//!                    │  VM Executor            │               │
//!                    │      │                  │               │
//!                    │  Storage (RocksDB) ◄────┘               │
//!                    └─────────────────────────────────────────┘
//! ```
//!
//! Все компоненты общаются через tokio channels (mpsc/broadcast).
//! Нет глобального состояния — всё передаётся явно.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::too_many_arguments)]

/// Node configuration types.
pub mod config;
/// Error types for node operations.
pub mod error;
/// Main node orchestration logic.
pub mod node;
/// Shared chain state accessible by all node components.
pub mod state;

pub use config::NodeConfig;
pub use error::NodeError;
pub use node::Node;
