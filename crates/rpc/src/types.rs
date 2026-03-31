//! Типы запросов и ответов JSON-RPC.

use rc_primitives::{
    hash::Hash,
    types::{Address, Amount, BlockHeight},
};
use serde::{Deserialize, Serialize};
// ─── Ответы ──────────────────────────────────────────────────────────────────

/// Информация о цепочке
#[derive(Debug, Serialize, Deserialize)]
pub struct ChainInfo {
    /// Human-readable network name (e.g. "quench-mainnet").
    pub network_name: String,
    /// Height of the current best chain tip.
    pub best_height: BlockHeight,
    /// Hash of the current best chain tip.
    pub best_hash: Hash,
    /// Total circulating supply in rustoshi.
    pub total_supply: Amount,
    /// Number of currently connected peers.
    pub peer_count: usize,
    /// Whether the node is actively syncing the chain.
    pub is_syncing: bool,
}

/// Информация о пире
#[derive(Debug, Serialize, Deserialize)]
pub struct PeerInfo {
    /// Libp2p peer identifier.
    pub peer_id: String,
    /// Network address of the peer.
    pub address: String,
    /// Protocol version string reported by the peer.
    pub version: String,
    /// Best block height known to the peer.
    pub best_height: BlockHeight,
    /// Round-trip latency to the peer in milliseconds.
    pub latency_ms: u64,
}

/// Квитанция транзакции (после включения в блок)
#[derive(Debug, Serialize, Deserialize)]
pub struct TxReceipt {
    /// The transaction identifier.
    pub tx_id: Hash,
    /// Hash of the block that included this transaction.
    pub block_hash: Hash,
    /// Height of the block that included this transaction.
    pub block_height: BlockHeight,
    /// Index of this transaction within the block.
    pub tx_index: u32,
    /// Whether the transaction executed successfully.
    pub success: bool,
    /// Использованный газ (для контрактных вызовов)
    pub gas_used: Option<u64>,
    /// Логи/события от контракта
    pub logs: Vec<EventLog>,
}

/// Лог события от смарт-контракта
#[derive(Debug, Serialize, Deserialize)]
pub struct EventLog {
    /// Address of the contract that emitted this event.
    pub contract: Address,
    /// Name of the emitted event.
    pub name: String,
    /// Hex-encoded event data payload.
    pub data: String,
}

// ─── Запросы ─────────────────────────────────────────────────────────────────

/// Запрос на отправку транзакции
#[derive(Debug, Deserialize)]
pub struct SendTransactionRequest {
    /// Hex-encoded сериализованная транзакция
    pub raw: String,
}
