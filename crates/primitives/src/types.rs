//! Primitive type aliases and newtypes.
//!
//! Использование newtypes (например, `BlockHeight(u64)` вместо просто `u64`)
//! позволяет компилятору ловить ошибки — нельзя случайно передать
//! `BlockHeight` туда, где ожидается Timestamp.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Высота блока в цепочке (genesis = 0)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct BlockHeight(pub u64);

impl fmt::Display for BlockHeight {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<u64> for BlockHeight {
    fn from(v: u64) -> Self {
        Self(v)
    }
}

impl From<BlockHeight> for u64 {
    fn from(h: BlockHeight) -> Self {
        h.0
    }
}

/// Unix-время в миллисекундах
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Timestamp(pub u64);

impl Timestamp {
    /// Текущее время
    /// # Panics
    /// Panics if the system clock is set before the Unix epoch.
    #[must_use]
    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock before epoch")
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        Self(ms)
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Nonce для Proof-of-Work
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Nonce(pub u64);

impl Nonce {
    /// Increments the nonce by 1, wrapping on overflow.
    pub fn increment(&mut self) {
        self.0 = self.0.wrapping_add(1);
    }
}

/// Количество монет в наименьших единицах (аналог сатоши в BTC)
/// 1 RSC = `100_000_000` rustoshi
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Default,
)]
pub struct Amount(pub u64);

impl Amount {
    /// 1 RSC
    pub const ONE: Self = Self(100_000_000);
    /// Максимальная эмиссия: 21 миллион RSC
    pub const MAX_SUPPLY: Self = Self(21_000_000 * 100_000_000);

    /// Checked addition; returns `None` on overflow.
    pub fn checked_add(self, rhs: Self) -> Option<Self> {
        self.0.checked_add(rhs.0).map(Self)
    }

    /// Checked subtraction; returns `None` on underflow.
    pub fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Показываем как RSC с 8 знаками после запятой
        #[allow(clippy::cast_precision_loss)]
        let display = self.0 as f64 / 100_000_000.0;
        write!(f, "{display:.8} RSC")
    }
}

/// Адрес кошелька (20 байт, как в Ethereum)
/// Внешнее представление — `Base58Check` строка
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Address([u8; 20]);

impl Address {
    /// Нулевой адрес (burn address)
    pub const ZERO: Self = Self([0u8; 20]);

    /// Constructs an `Address` from a raw 20-byte array.
    #[must_use]
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Returns the raw 20-byte representation of this address.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Кодируем в `Base58Check` с префиксом версии
    #[must_use]
    #[allow(clippy::items_after_statements)]
    pub fn to_base58(&self) -> String {
        let mut payload = vec![0x00u8]; // version byte
        payload.extend_from_slice(&self.0);

        // Checksum: SHA256(SHA256(payload))[0..4]
        use sha2::{Digest, Sha256};
        let first = Sha256::digest(&payload);
        let second = Sha256::digest(first);
        payload.extend_from_slice(&second[..4]);

        bs58::encode(payload).into_string()
    }

    /// Декодируем из `Base58Check`
    /// # Errors
    /// Returns `Err` if the string is not valid `Base58Check`.
    #[allow(clippy::items_after_statements)]
    pub fn from_base58(s: &str) -> Result<Self, crate::PrimitivesError> {
        let decoded = bs58::decode(s)
            .into_vec()
            .map_err(|_| crate::PrimitivesError::InvalidAddress(s.to_string()))?;

        if decoded.len() != 25 {
            return Err(crate::PrimitivesError::InvalidAddress(s.to_string()));
        }

        // Проверяем checksum
        use sha2::{Digest, Sha256};
        let payload = &decoded[..21];
        let checksum = &decoded[21..];
        let first = Sha256::digest(payload);
        let second = Sha256::digest(first);

        if &second[..4] != checksum {
            return Err(crate::PrimitivesError::InvalidAddress(s.to_string()));
        }

        let mut bytes = [0u8; 20];
        bytes.copy_from_slice(&decoded[1..21]);
        Ok(Self(bytes))
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

impl fmt::Debug for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Address({})", self.to_base58())
    }
}
