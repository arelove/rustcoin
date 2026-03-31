//! # rc-mempool
//!
//! Пул ожидающих транзакций (mempool).
//!
//! ## Приоритизация
//!
//! Транзакции сортируются по **fee per byte** (комиссия / размер).
//! Майнер берёт транзакции с наибольшей комиссией — это максимизирует его доход.
//!
//! ## Защиты
//!
//! - Максимальный размер пула: `MAX_MEMPOOL_TXS`
//! - Дублирующиеся TxId отклоняются
//! - Проверяется подпись и базовые правила транзакции
//! - Nonce-проверка против replay-атак

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]

use dashmap::DashMap;
use rc_primitives::{
    transaction::{Transaction, TxId},
    types::Amount,
};
use tracing::{debug, warn};

/// Error types for mempool operations.
pub mod error;
pub use error::MempoolError;

/// Максимальное число транзакций в пуле
const MAX_MEMPOOL_TXS: usize = 5_000;

/// Запись в пуле
#[derive(Debug, Clone)]
struct MempoolEntry {
    tx: Transaction,
    /// Fee-per-byte для сортировки
    fee_per_byte: u64,
    /// Время добавления (для TTL и порядка при равной комиссии)
    added_at: std::time::Instant,
}

/// Потокобезопасный Mempool
///
/// Использует `DashMap` для O(1) lookup по TxId
/// и `BTreeMap` для сортировки по приоритету.
pub struct Mempool {
    /// Быстрый lookup: TxId → Entry
    entries: DashMap<TxId, MempoolEntry>,
    /// Максимальное количество транзакций
    max_size: usize,
}

impl Mempool {
    /// Создать с параметрами по умолчанию
    pub fn new() -> Self {
        Self::with_capacity(MAX_MEMPOOL_TXS)
    }

    /// Создать с заданной ёмкостью
    pub fn with_capacity(max_size: usize) -> Self {
        Self {
            entries: DashMap::with_capacity(max_size),
            max_size,
        }
    }

    /// Добавить транзакцию в пул
    pub fn add(&self, tx: Transaction) -> Result<TxId, MempoolError> {
        // Базовая валидация
        tx.validate_basic()
            .map_err(|e| MempoolError::InvalidTransaction(e.to_string()))?;

        let txid = tx.tx_id();

        // Дубликат?
        if self.entries.contains_key(&txid) {
            return Err(MempoolError::DuplicateTransaction(txid));
        }

        // Пул переполнен?
        if self.entries.len() >= self.max_size {
            // Вытесняем транзакцию с самой низкой комиссией
            self.evict_lowest_fee()?;
        }

        // Вычисляем fee-per-byte для приоритизации
        let tx_size = serde_json::to_vec(&tx).map(|v| v.len() as u64).unwrap_or(1);
        let fee_per_byte = tx.fee.0 / tx_size.max(1);

        debug!(
            txid = %txid,
            fee = %tx.fee,
            fee_per_byte = fee_per_byte,
            "tx added to mempool"
        );

        self.entries.insert(
            txid,
            MempoolEntry {
                tx,
                fee_per_byte,
                added_at: std::time::Instant::now(),
            },
        );

        Ok(txid)
    }

    /// Извлечь транзакции для включения в блок.
    ///
    /// Возвращает не более `max_count` транзакций,
    /// отсортированных по убыванию fee-per-byte.
    pub fn select_for_block(&self, max_count: usize) -> Vec<Transaction> {
        // В select_for_block замени кортеж на включение fee:
        let mut entries: Vec<_> = self
            .entries
            .iter()
            .map(|e| (e.fee_per_byte, e.tx.fee.0, e.tx.clone(), e.added_at))
            .collect();

        // Сортировка: сначала по fee_per_byte, потом по абсолютной fee
        entries.sort_by(|a, b| {
            b.0.cmp(&a.0)
                .then_with(|| b.1.cmp(&a.1)) // при равном fee_per_byte — выше абсолютная fee
                .then_with(|| a.3.cmp(&b.3)) // при равной fee — старше добавленная
        });

        entries
            .into_iter()
            .take(max_count)
            .map(|(_, _, tx, _)| tx)
            .collect()
    }

    /// Удалить транзакции, включённые в подтверждённый блок
    pub fn remove_confirmed(&self, txids: &[TxId]) {
        for txid in txids {
            self.entries.remove(txid);
        }
    }

    /// Получить транзакцию по TxId
    pub fn get(&self, txid: &TxId) -> Option<Transaction> {
        self.entries.get(txid).map(|e| e.tx.clone())
    }

    /// Текущий размер пула
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Пуст ли пул?
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Суммарные ожидающие комиссии
    pub fn total_fees(&self) -> Amount {
        self.entries
            .iter()
            .fold(Amount(0), |acc, e| acc.checked_add(e.tx.fee).unwrap_or(acc))
    }

    /// Вытеснить транзакцию с наименьшей комиссией
    fn evict_lowest_fee(&self) -> Result<(), MempoolError> {
        let lowest = self
            .entries
            .iter()
            .min_by_key(|e| e.fee_per_byte)
            .map(|e| *e.key());

        if let Some(txid) = lowest {
            warn!(txid = %txid, "evicting low-fee tx from mempool");
            self.entries.remove(&txid);
            Ok(())
        } else {
            Err(MempoolError::Full)
        }
    }
}

impl Default for Mempool {
    fn default() -> Self {
        Self::new()
    }
}
