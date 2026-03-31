//! Основной интерфейс к RocksDB.

use crate::{account::AccountState, error::StorageError, keys::Keys};
use rc_primitives::{
    block::{Block, BlockHeader},
    hash::Hash,
    transaction::{Transaction, TxId},
    types::{Address, BlockHeight},
};
use rocksdb::{ColumnFamilyDescriptor, Options, DB};
use std::path::Path;
use std::sync::Arc;

/// Имена Column Families
const CF_BLOCKS: &str = "blocks";
const CF_HEADERS: &str = "headers";
const CF_HEIGHTS: &str = "heights";
const CF_TXS: &str = "txs";
const CF_STATE: &str = "state";
const CF_META: &str = "meta";

/// Основной объект базы данных.
/// Обёрнут в `Arc` для sharing между потоками.
#[derive(Clone)]
pub struct Database {
    inner: Arc<DB>,
}

impl Database {
    /// Открыть (или создать) базу данных по указанному пути
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);
        opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        opts.increase_parallelism(num_cpus());
        opts.set_max_background_jobs(4);
        // Write buffer: 64MB (буферизуем записи в памяти перед flush на диск)
        opts.set_write_buffer_size(64 * 1024 * 1024);

        let cfs = [CF_BLOCKS, CF_HEADERS, CF_HEIGHTS, CF_TXS, CF_STATE, CF_META]
            .iter()
            .map(|name| {
                let mut cf_opts = Options::default();
                // Bloom filter: ускоряет point lookups в 10x
                cf_opts.set_bloom_locality(1);
                ColumnFamilyDescriptor::new(*name, cf_opts)
            })
            .collect::<Vec<_>>();

        let db = DB::open_cf_descriptors(&opts, path, cfs)
            .map_err(|e| StorageError::Open(e.to_string()))?;

        Ok(Self {
            inner: Arc::new(db),
        })
    }

    /// Открыть временную БД в `/tmp` (для тестов)
    #[cfg(test)]
    pub fn open_temp() -> Result<Self, StorageError> {
        let dir = tempfile::tempdir().expect("tempdir");
        Self::open(dir.path())
    }

    // ─── Блоки ──────────────────────────────────────────────────────────────

    /// Сохранить блок (атомарно: блок + заголовок + индекс по высоте)
    pub fn put_block(&self, block: &Block) -> Result<(), StorageError> {
        let hash = block.hash();
        let mut batch = rocksdb::WriteBatch::default();

        // Полный блок
        let cf_blocks = self.cf(CF_BLOCKS)?;
        batch.put_cf(cf_blocks, Keys::block(&hash), self.serialize(block)?);

        // Только заголовок (для light clients)
        let cf_headers = self.cf(CF_HEADERS)?;
        batch.put_cf(
            cf_headers,
            Keys::hash(&hash),
            self.serialize(&block.header)?,
        );

        // Индекс: высота → хэш
        let cf_heights = self.cf(CF_HEIGHTS)?;
        batch.put_cf(cf_heights, Keys::height(block.height()), hash.as_bytes());

        self.inner
            .write(batch)
            .map_err(|e| StorageError::Write(e.to_string()))
    }

    /// Получить блок по хэшу
    pub fn get_block(&self, hash: &Hash) -> Result<Option<Block>, StorageError> {
        let cf = self.cf(CF_BLOCKS)?;
        match self
            .inner
            .get_cf(cf, Keys::block(hash))
            .map_err(|e| StorageError::Read(e.to_string()))?
        {
            Some(bytes) => Ok(Some(self.deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    /// Получить блок по высоте
    pub fn get_block_at(&self, height: BlockHeight) -> Result<Option<Block>, StorageError> {
        let cf_heights = self.cf(CF_HEIGHTS)?;
        let hash_bytes = match self
            .inner
            .get_cf(cf_heights, Keys::height(height))
            .map_err(|e| StorageError::Read(e.to_string()))?
        {
            Some(b) => b,
            None => return Ok(None),
        };

        let mut arr = [0u8; 32];
        if hash_bytes.len() != 32 {
            return Err(StorageError::Corruption("invalid hash length".into()));
        }
        arr.copy_from_slice(&hash_bytes);
        let hash = Hash::from_bytes(arr);
        self.get_block(&hash)
    }

    /// Получить заголовок блока (без транзакций — быстро)
    pub fn get_header(&self, hash: &Hash) -> Result<Option<BlockHeader>, StorageError> {
        let cf = self.cf(CF_HEADERS)?;
        match self
            .inner
            .get_cf(cf, Keys::hash(hash))
            .map_err(|e| StorageError::Read(e.to_string()))?
        {
            Some(bytes) => Ok(Some(self.deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    // ─── Транзакции ─────────────────────────────────────────────────────────

    /// Сохранить транзакцию
    pub fn put_tx(&self, tx: &Transaction) -> Result<(), StorageError> {
        let cf = self.cf(CF_TXS)?;
        let txid = tx.tx_id();
        self.inner
            .put_cf(cf, Keys::hash(&txid), self.serialize(tx)?)
            .map_err(|e| StorageError::Write(e.to_string()))
    }

    /// Получить транзакцию по TxId
    pub fn get_tx(&self, txid: &TxId) -> Result<Option<Transaction>, StorageError> {
        let cf = self.cf(CF_TXS)?;
        match self
            .inner
            .get_cf(cf, Keys::hash(txid))
            .map_err(|e| StorageError::Read(e.to_string()))?
        {
            Some(bytes) => Ok(Some(self.deserialize(&bytes)?)),
            None => Ok(None),
        }
    }

    // ─── Account State ───────────────────────────────────────────────────────

    /// Получить состояние аккаунта (баланс, nonce)
    pub fn get_account(&self, address: &Address) -> Result<AccountState, StorageError> {
        let cf = self.cf(CF_STATE)?;
        match self
            .inner
            .get_cf(cf, address.as_bytes())
            .map_err(|e| StorageError::Read(e.to_string()))?
        {
            Some(bytes) => self.deserialize(&bytes),
            None => Ok(AccountState::default()), // новый аккаунт
        }
    }

    /// Обновить состояние аккаунта
    pub fn put_account(&self, address: &Address, state: &AccountState) -> Result<(), StorageError> {
        let cf = self.cf(CF_STATE)?;
        self.inner
            .put_cf(cf, address.as_bytes(), self.serialize(state)?)
            .map_err(|e| StorageError::Write(e.to_string()))
    }

    // ─── Meta ────────────────────────────────────────────────────────────────

    /// Сохранить метаданные (например, текущий best tip)
    pub fn put_meta(&self, key: &str, value: &[u8]) -> Result<(), StorageError> {
        let cf = self.cf(CF_META)?;
        self.inner
            .put_cf(cf, key.as_bytes(), value)
            .map_err(|e| StorageError::Write(e.to_string()))
    }

    /// Получить метаданные
    pub fn get_meta(&self, key: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let cf = self.cf(CF_META)?;
        self.inner
            .get_cf(cf, key.as_bytes())
            .map_err(|e| StorageError::Read(e.to_string()))
    }

    // ─── Helpers ─────────────────────────────────────────────────────────────

    fn cf(&self, name: &str) -> Result<&rocksdb::ColumnFamily, StorageError> {
        self.inner
            .cf_handle(name)
            .ok_or_else(|| StorageError::Corruption(format!("CF '{name}' not found")))
    }

    pub(crate) fn serialize<T: serde::Serialize>(
        &self,
        value: &T,
    ) -> Result<Vec<u8>, StorageError> {
        serde_json::to_vec(value).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    pub(crate) fn deserialize<T: serde::de::DeserializeOwned>(
        &self,
        bytes: &[u8],
    ) -> Result<T, StorageError> {
        serde_json::from_slice(bytes).map_err(|e| StorageError::Serialization(e.to_string()))
    }

    /// Получить Column Family handle по имени (pub для state_machine).
    pub(crate) fn cf_handle(&self, name: &str) -> Result<&rocksdb::ColumnFamily, StorageError> {
        self.inner
            .cf_handle(name)
            .ok_or_else(|| StorageError::Corruption(format!("CF '{name}' not found")))
    }

    /// Записать WriteBatch атомарно.
    pub(crate) fn write_batch(&self, batch: rocksdb::WriteBatch) -> Result<(), StorageError> {
        self.inner
            .write(batch)
            .map_err(|e| StorageError::Write(e.to_string()))
    }
}

fn num_cpus() -> i32 {
    std::thread::available_parallelism()
        .map(|n| n.get() as i32)
        .unwrap_or(2)
}
