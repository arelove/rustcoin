//! SHA-256 хэш — основа цепочки блоков.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// 32-байтовый SHA-256 хэш
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Нулевой хэш (используется как `previous_hash` в genesis-блоке)
    pub const ZERO: Self = Self([0u8; 32]);

    /// Вычислить SHA-256 от произвольных байт
    #[must_use]
    pub fn sha256(data: &[u8]) -> Self {
        let digest = Sha256::digest(data);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&digest);
        Self(bytes)
    }

    /// Двойное SHA-256 (`SHA256d`) — используется в Bitcoin
    #[must_use]
    pub fn sha256d(data: &[u8]) -> Self {
        let first = Sha256::digest(data);
        let second = Sha256::digest(first);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&second);
        Self(bytes)
    }

    /// Returns the raw 32-byte representation of this hash.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Constructs a `Hash` from a raw 32-byte array.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Конвертация в hex-строку
    #[must_use]
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Парсинг из hex-строки
    /// # Errors
    /// Returns `Err` if the string is not valid hex or wrong length.
    pub fn from_hex(s: &str) -> Result<Self, hex::FromHexError> {
        let bytes = hex::decode(s)?;
        if bytes.len() != 32 {
            // Возвращаем ошибку типа hex::FromHexError через InvalidStringLength
            return Err(hex::FromHexError::InvalidStringLength);
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }

    /// Проверить, соответствует ли хэш заданной сложности (difficulty)
    /// Сложность N означает: первые N байт должны быть нулями
    #[must_use]
    pub fn meets_difficulty(&self, difficulty: u32) -> bool {
        let bytes_to_check = (difficulty / 8) as usize;
        let bits_remaining = (difficulty % 8) as u8;

        // Проверяем полные нулевые байты
        if self.0[..bytes_to_check].iter().any(|&b| b != 0) {
            return false;
        }

        // Проверяем оставшиеся биты
        if bits_remaining > 0 && bytes_to_check < 32 {
            let mask = 0xFFu8 << (8 - bits_remaining);
            if self.0[bytes_to_check] & mask != 0 {
                return false;
            }
        }

        true
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hash({}...{})",
            &self.to_hex()[..8],
            &self.to_hex()[56..]
        )
    }
}
