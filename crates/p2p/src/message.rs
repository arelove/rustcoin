//! Сетевые сообщения — что узлы говорят друг другу.

use rc_primitives::{
    block::{Block, BlockHeader},
    hash::Hash,
    transaction::Transaction,
    types::BlockHeight,
};
use serde::{Deserialize, Serialize};

/// Сетевое сообщение (сериализуется в JSON для Gossipsub)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NetworkMessage {
    // ─── Блоки ──────────────────────────────────────────────────────────────
    /// Новый добытый блок (рассылается майнером)
    NewBlock(Box<Block>),

    /// Request a block by hash (unicast via request-response).
    GetBlock {
        /// The hash of the block being requested.
        hash: Hash,
    },

    /// Request a range of blocks for synchronization.
    GetBlocks {
        /// The first block height to fetch (inclusive).
        start_height: BlockHeight,
        /// The last block height to fetch (inclusive).
        end_height: BlockHeight,
    },

    /// Ответ с блоками
    Blocks(Vec<Block>),

    /// Только заголовки (для light clients — без тела блока)
    Headers(Vec<BlockHeader>),

    // ─── Транзакции ─────────────────────────────────────────────────────────
    /// Новая транзакция (рассылается от пользователя/кошелька)
    NewTransaction(Box<Transaction>),

    // ─── Синхронизация ──────────────────────────────────────────────────────
    /// Status-сообщение: узел сообщает о своей текущей вершине цепи
    Status {
        /// Версия протокола
        version: u32,
        /// Хэш текущего best tip
        best_hash: Hash,
        /// Высота best tip
        best_height: BlockHeight,
        /// Хэш genesis блока (для проверки совместимости сети)
        genesis_hash: Hash,
    },

    /// Ответ на запрос синхронизации (список хэшей с самой высокой высоты)
    Inventory(Vec<Hash>),

    /// Keepalive ping message.
    Ping {
        /// Random nonce to match against the corresponding Pong.
        nonce: u64,
    },
    /// Keepalive pong reply.
    Pong {
        /// Nonce echoed from the corresponding Ping.
        nonce: u64,
    },
}

impl NetworkMessage {
    /// Gossipsub topic для данного типа сообщения
    pub fn topic(&self) -> &'static str {
        match self {
            Self::NewBlock(_)
            | Self::GetBlock { .. }
            | Self::GetBlocks { .. }
            | Self::Blocks(_) => "quench/blocks/1",

            Self::Headers(_) => "quench/headers/1",

            Self::NewTransaction(_) => "quench/txs/1",

            Self::Status { .. } | Self::Inventory(_) | Self::Ping { .. } | Self::Pong { .. } => {
                "quench/control/1"
            }
        }
    }

    /// Сериализовать в байты для отправки по сети
    pub fn encode(&self) -> Result<Vec<u8>, serde_json::Error> {
        serde_json::to_vec(self)
    }

    /// Десериализовать из байт
    pub fn decode(bytes: &[u8]) -> Result<Self, serde_json::Error> {
        serde_json::from_slice(bytes)
    }
}
