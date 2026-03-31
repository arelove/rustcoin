//! Block types.
//!
//! Блок состоит из заголовка (`BlockHeader`) и списка транзакций.
//! Разделение важно для light clients — им нужны только заголовки.

use crate::{
    hash::Hash,
    transaction::Transaction,
    types::{BlockHeight, Nonce, Timestamp},
};
use serde::{Deserialize, Serialize};

/// Заголовок блока (80 байт, как в Bitcoin)
/// Хэшируется для Proof-of-Work
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Версия блока (для soft/hard fork сигнализации)
    pub version: u32,
    /// Хэш предыдущего блока (вот почему это "цепочка")
    pub previous_hash: Hash,
    /// Корень Merkle-дерева транзакций
    pub merkle_root: Hash,
    /// Время создания блока
    pub timestamp: Timestamp,
    /// Текущая сложность (биты, как в Bitcoin)
    pub bits: u32,
    /// Nonce для `PoW`
    pub nonce: Nonce,
    /// Высота блока
    pub height: BlockHeight,
    /// Хэш текущего блока (вычисляется, не хранится в сети)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<Hash>,
}

impl BlockHeader {
    /// Вычислить хэш заголовка (SHA-256d, как в Bitcoin)
    /// # Panics
    /// Panics if header serialization fails (should never happen).
    #[must_use]
    pub fn compute_hash(&self) -> Hash {
        // Создаём копию без поля hash, чтобы не включать его в хэш
        let data_for_hashing = BlockHeaderHashData {
            version: self.version,
            previous_hash: self.previous_hash,
            merkle_root: self.merkle_root,
            timestamp: self.timestamp,
            bits: self.bits,
            nonce: self.nonce,
            height: self.height,
        };
        let bytes =
            serde_json::to_vec(&data_for_hashing).expect("header serialization is infallible");
        Hash::sha256d(&bytes)
    }

    /// Проверить, соответствует ли блок сложности
    #[must_use]
    pub fn meets_difficulty(&self) -> bool {
        let hash = self.compute_hash();
        hash.meets_difficulty(self.bits)
    }
}

/// Данные заголовка для хэширования (без поля hash)
#[derive(Serialize)]
struct BlockHeaderHashData {
    version: u32,
    previous_hash: Hash,
    merkle_root: Hash,
    timestamp: Timestamp,
    bits: u32,
    nonce: Nonce,
    height: BlockHeight,
}

/// Полный блок = заголовок + транзакции
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Block {
    /// Заголовок блока
    pub header: BlockHeader,
    /// Список транзакций (первая всегда Coinbase)
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Создать genesis-блок (первый блок сети)
    #[must_use]
    pub fn genesis() -> Self {
        Self {
            header: BlockHeader {
                version: 1,
                previous_hash: Hash::ZERO,
                merkle_root: Hash::ZERO,
                timestamp: Timestamp(0),
                bits: 20, // начальная сложность: 20 бит
                nonce: Nonce(0),
                height: BlockHeight(0),
                hash: Some(Hash::ZERO),
            },
            transactions: vec![],
        }
    }

    /// Вычислить Merkle Root из транзакций
    ///
    /// Merkle Tree позволяет доказать принадлежность транзакции блоку
    /// за O(log n) шагов (без скачки всего блока).
    /// # Panics
    /// Panics if the transaction list is empty and `hashes.last()` is called on empty vec.
    #[must_use]
    pub fn compute_merkle_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return Hash::ZERO;
        }

        let mut hashes: Vec<Hash> = transactions.iter().map(Transaction::tx_id).collect();

        // Строим дерево снизу вверх
        while hashes.len() > 1 {
            // Если нечётное число — дублируем последний
            if hashes.len() % 2 != 0 {
                hashes.push(*hashes.last().unwrap());
            }

            hashes = hashes
                .chunks(2)
                .map(|pair| {
                    let mut combined = pair[0].as_bytes().to_vec();
                    combined.extend_from_slice(pair[1].as_bytes());
                    Hash::sha256d(&combined)
                })
                .collect();
        }

        hashes[0]
    }

    /// Получить высоту блока
    #[must_use]
    pub fn height(&self) -> BlockHeight {
        self.header.height
    }

    /// Получить хэш блока
    #[must_use]
    pub fn hash(&self) -> Hash {
        self.header.compute_hash()
    }

    /// Получить хэш предыдущего блока
    #[must_use]
    pub fn previous_hash(&self) -> Hash {
        self.header.previous_hash
    }

    /// Валидация структуры блока (без проверки транзакций)
    /// # Errors
    /// Returns `Err` if the block structure is invalid.
    pub fn validate_structure(&self) -> Result<(), crate::PrimitivesError> {
        // Merkle root должен совпадать
        let expected_merkle = Self::compute_merkle_root(&self.transactions);
        if self.header.merkle_root != expected_merkle {
            return Err(crate::PrimitivesError::InvalidMerkleRoot);
        }

        // Блок должен соответствовать сложности
        if !self.header.meets_difficulty() {
            return Err(crate::PrimitivesError::InsufficientProofOfWork);
        }

        // Первая транзакция должна быть Coinbase
        if self.header.height.0 > 0 {
            let first_tx =
                self.transactions
                    .first()
                    .ok_or(crate::PrimitivesError::InvalidBlock(
                        "no transactions".into(),
                    ))?;
            if !matches!(first_tx.kind, crate::transaction::TxKind::Coinbase) {
                return Err(crate::PrimitivesError::InvalidBlock(
                    "first transaction must be coinbase".into(),
                ));
            }
        }

        Ok(())
    }

    /// Количество байт в блоке (для лимита размера)
    #[must_use]
    pub fn size_bytes(&self) -> usize {
        serde_json::to_vec(self).map(|v| v.len()).unwrap_or(0)
    }
}
