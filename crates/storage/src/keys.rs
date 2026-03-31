//! Схема ключей для RocksDB.
//!
//! Централизуем всю логику формирования ключей в одном месте.

use rc_primitives::{hash::Hash, types::BlockHeight};

/// Namespace for RocksDB key construction helpers.
pub struct Keys;

impl Keys {
    /// Ключ для блока: префикс "b" + 32 байта хэша
    pub fn block(hash: &Hash) -> Vec<u8> {
        let mut key = Vec::with_capacity(33);
        key.push(b'b');
        key.extend_from_slice(hash.as_bytes());
        key
    }

    /// Ключ для произвольного хэша (заголовки, txid)
    pub fn hash(hash: &Hash) -> [u8; 32] {
        *hash.as_bytes()
    }

    /// Ключ для индекса высота→хэш: big-endian u64
    /// Big-endian важен! RocksDB сортирует лексикографически,
    /// big-endian u64 сортируется так же как числа.
    pub fn height(height: BlockHeight) -> [u8; 8] {
        height.0.to_be_bytes()
    }
}
