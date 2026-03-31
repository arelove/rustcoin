//! Proof-of-Work майнер.
//!
//! Майнинг — это перебор nonce до нахождения нужного хэша.
//! Реализуем два варианта:
//! 1. Однопоточный (простой, для тестов)
//! 2. Многопоточный через `rayon` (для реального использования)

use crate::error::ConsensusError;
use rc_primitives::{
    block::{Block, BlockHeader},
    hash::Hash,
    transaction::{Transaction, TxKind},
    types::{Amount, BlockHeight, Nonce, Timestamp},
};
use tracing::{debug, info};

/// Награда за блок (убывает каждые HALVING_INTERVAL блоков, как в Bitcoin)
pub const BLOCK_REWARD_INITIAL: Amount = Amount(50 * 100_000_000); // 50 RSC
/// Блоков между халвингами
pub const HALVING_INTERVAL: u64 = 210_000;

/// Результат успешного майнинга
#[derive(Debug)]
pub struct MineResult {
    /// Найденный блок
    pub block: Block,
    /// Сколько итераций потребовалось
    pub attempts: u64,
    /// Время майнинга в миллисекундах
    pub elapsed_ms: u64,
}

/// Майнер Proof-of-Work
pub struct Miner {
    /// Адрес кошелька майнера для получения награды
    miner_address: rc_primitives::types::Address,
    /// Флаг отмены — устанавливается при получении нового блока от сети
    cancel_token: tokio_util::sync::CancellationToken,
}

impl Miner {
    /// Создать нового майнера
    pub fn new(
        miner_address: rc_primitives::types::Address,
        cancel_token: tokio_util::sync::CancellationToken,
    ) -> Self {
        Self {
            miner_address,
            cancel_token,
        }
    }

    /// Вычислить награду за блок на заданной высоте
    pub fn block_reward(height: BlockHeight) -> Amount {
        let halvings = height.0 / HALVING_INTERVAL;
        if halvings >= 64 {
            return Amount(0); // все монеты выпущены
        }
        Amount(BLOCK_REWARD_INITIAL.0 >> halvings)
    }

    /// Создать Coinbase транзакцию (награда майнеру)
    fn create_coinbase(&self, height: BlockHeight, fee_total: Amount) -> Transaction {
        let reward = Self::block_reward(height);
        let total = reward.checked_add(fee_total).unwrap_or(reward);

        Transaction {
            version: 1,
            kind: TxKind::Coinbase,
            from: None,
            to: self.miner_address,
            amount: total,
            fee: Amount(0),
            nonce: height.0, // nonce = высота (для уникальности)
            timestamp: Timestamp::now(),
            signature: None,
            public_key: None,
        }
    }

    /// Синхронный майнинг (однопоточный).
    ///
    /// Подходит для тестов и небольших сложностей.
    /// Для production используй `mine_async`.
    pub fn mine_sync(
        &self,
        previous_hash: Hash,
        height: BlockHeight,
        transactions: Vec<Transaction>,
        bits: u32,
        version: u32,
    ) -> Result<MineResult, ConsensusError> {
        let start = std::time::Instant::now();

        // Coinbase всегда первая транзакция
        let fee_total: Amount = transactions
            .iter()
            .fold(Amount(0), |acc, tx| acc.checked_add(tx.fee).unwrap_or(acc));

        let coinbase = self.create_coinbase(height, fee_total);
        let mut all_txs = vec![coinbase];
        all_txs.extend(transactions);

        let merkle_root = Block::compute_merkle_root(&all_txs);

        let mut header = BlockHeader {
            version,
            previous_hash,
            merkle_root,
            timestamp: Timestamp::now(),
            bits,
            nonce: Nonce(0),
            height,
            hash: None,
        };

        let mut attempts = 0u64;

        loop {
            attempts += 1; // ← сюда, до compute_hash

            // Проверяем отмену каждые 10_000 итераций
            if attempts % 10_000 == 0 && self.cancel_token.is_cancelled() {
                return Err(ConsensusError::MiningCancelled);
            }

            let hash = header.compute_hash();

            if hash.meets_difficulty(bits) {
                header.hash = Some(hash);
                info!(
                    height = %height,
                    hash = %hash,
                    attempts = attempts,
                    elapsed_ms = start.elapsed().as_millis(),
                    "⛏  Block mined!"
                );

                let block = Block {
                    header,
                    transactions: all_txs,
                };

                return Ok(MineResult {
                    block,
                    attempts,
                    elapsed_ms: start.elapsed().as_millis() as u64,
                });
            }

            header.nonce.increment();

            // Nonce переполнился — обновляем timestamp и пробуем снова
            if header.nonce.0 == 0 {
                debug!("nonce exhausted, updating timestamp");
                header.timestamp = Timestamp::now();
            }
        }
    }

    /// Асинхронный майнинг — не блокирует tokio runtime.
    ///
    /// Запускает майнинг в отдельном `tokio::task::spawn_blocking` потоке,
    /// что позволяет runtime продолжать обрабатывать сетевые события.
    pub async fn mine_async(
        &self,
        previous_hash: Hash,
        height: BlockHeight,
        transactions: Vec<Transaction>,
        bits: u32,
        version: u32,
    ) -> Result<MineResult, ConsensusError> {
        let miner_address = self.miner_address;
        let cancel_token = self.cancel_token.clone();

        tokio::task::spawn_blocking(move || {
            let miner = Miner::new(miner_address, cancel_token);
            miner.mine_sync(previous_hash, height, transactions, bits, version)
        })
        .await
        .map_err(|e| ConsensusError::Internal(e.to_string()))?
    }
}
