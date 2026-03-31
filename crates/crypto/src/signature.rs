//! Ed25519 подпись (64 байта).

use ed25519_dalek::Signature as DalekSignature;
use serde::{Deserialize, Serialize};

/// Ed25519 подпись
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature(pub(crate) DalekSignature);

impl Signature {
    /// Сырые байты подписи (64 байта)
    #[must_use]
    pub fn as_bytes(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }

    /// Создать из байт
    /// # Errors
    /// Returns `Err` if the bytes are not a valid 64-byte Ed25519 signature.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, crate::CryptoError> {
        let arr: &[u8; 64] = bytes
            .try_into()
            .map_err(|_| crate::CryptoError::InvalidSignature)?;
        Ok(Self(DalekSignature::from_bytes(arr)))
    }

    /// Конвертация в Vec<u8> для хранения в Transaction
    #[must_use]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_bytes().to_vec()
    }
}

impl Serialize for Signature {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(&self.0.to_bytes())
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let bytes: Vec<u8> = Deserialize::deserialize(d)?;
        Self::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}
