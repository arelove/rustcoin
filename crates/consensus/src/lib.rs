//! # rc-consensus
//!
//! Движок консенсуса Proof-of-Work.
//!
//! ## Как работает PoW
//!
//! Майнер снова и снова перебирает `nonce`, вычисляя хэш заголовка блока,
//! пока не найдёт хэш, начинающийся с нужного количества нулевых бит.
//! Количество нулей задаётся `difficulty` (сложностью).
//!
//! ```text
//! hash(header || nonce) < target
//! ```
//!
//! ## Adjustable Difficulty (как в Bitcoin)
//!
//! Каждые `DIFFICULTY_ADJUSTMENT_INTERVAL` блоков сложность пересчитывается,
//! чтобы среднее время блока стремилось к `TARGET_BLOCK_TIME_SECS`.
//!
//! ## Форк-выбор
//!
//! Правило: **цепочка с наибольшей суммарной сложностью побеждает** (not just longest chain).
//! Это корректнее — короткая цепочка с высокой сложностью может быть "тяжелее".

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

pub mod difficulty;
/// Error types for consensus operations.
pub mod error;
pub mod fork_choice;
pub mod miner;

pub use difficulty::{compute_target, DifficultyAdjuster};
pub use error::ConsensusError;
pub use fork_choice::ForkChoice;
pub use miner::{MineResult, Miner};
