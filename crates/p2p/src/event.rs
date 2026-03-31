//! События, которые сетевой слой посылает в основной цикл ноды.

use libp2p::PeerId;
use rc_primitives::{block::Block, transaction::Transaction};

/// Событие от сетевого слоя
#[derive(Debug)]
pub enum NetworkEvent {
    /// Подключился новый пир
    PeerConnected(PeerId),
    /// Пир отключился
    PeerDisconnected(PeerId),
    /// Получен новый блок от пира
    NewBlock {
        /// The peer the block was received from.
        from: PeerId,
        /// The received block.
        block: Box<Block>,
    },
    /// Получена новая транзакция от пира
    NewTransaction {
        /// The peer the transaction was received from.
        from: PeerId,
        /// The received transaction.
        tx: Box<Transaction>,
    },
    /// Пир запросил синхронизацию
    SyncRequest {
        /// The peer requesting synchronization.
        from: PeerId,
        /// The block height the peer wants to start syncing from.
        start_height: u64,
    },
}
