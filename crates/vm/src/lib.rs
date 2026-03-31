//! # rc-vm
//!
//! WebAssembly смарт-контракт VM на базе `wasmtime`.
//!
//! ## Архитектура
//!
//! ```text
//! Transaction (ContractCall)
//!       │
//!       ▼
//!   Executor  ──── загружает байткод из Storage
//!       │
//!       ▼
//!   wasmtime Engine
//!       │
//!       ├── Imports (Host Functions) ◄── контракт вызывает
//!       │     ├── storage_get(key) → value
//!       │     ├── storage_set(key, value)
//!       │     ├── get_caller() → Address
//!       │     ├── get_balance(addr) → u64
//!       │     ├── transfer(to, amount)
//!       │     └── emit_event(name, data)
//!       │
//!       └── Exports (Contract Functions) ◄── нода вызывает
//!             ├── call(method, args) → result
//!             └── init(args)         ← вызывается при деплое
//! ```
//!
//! ## Газ
//!
//! Каждая WASM инструкция стоит определённое количество газа.
//! Wasmtime поддерживает fuel-based execution — контракт
//! автоматически прерывается при исчерпании газа.
//!
//! ## Изоляция
//!
//! Каждый вызов контракта выполняется в отдельном `Store` —
//! полная изоляция памяти между вызовами и контрактами.

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod context;
/// Error types for VM operations.
pub mod error;
pub mod executor;
pub mod gas;
pub mod host;

pub use context::ExecutionContext;
pub use error::VmError;
pub use executor::{ExecutionResult, Executor};
