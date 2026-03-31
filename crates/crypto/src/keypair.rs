//! Ed25519 key pair implementation.

use crate::{error::CryptoError, signature::Signature};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rc_primitives::types::Address;
use ripemd::Ripemd160;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Публичный ключ Ed25519 (32 байта)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PublicKey(pub(crate) VerifyingKey);

impl PublicKey {
    /// Получить адрес кошелька из публичного ключа
    ///
    /// Алгоритм (Bitcoin-style):
    /// 1. SHA-256(pubkey bytes)
    /// 2. RIPEMD-160(результат шага 1)
    /// 3. Берём первые 20 байт → Address
    #[must_use]
    pub fn to_address(&self) -> Address {
        let pub_bytes = self.0.as_bytes();

        let sha_digest = Sha256::digest(pub_bytes);
        let ripemd_digest = Ripemd160::digest(sha_digest);

        let mut addr_bytes = [0u8; 20];
        addr_bytes.copy_from_slice(&ripemd_digest);

        Address::from_bytes(addr_bytes)
    }

    /// Верифицировать подпись
    /// # Errors
    /// Returns `Err` if the signature does not match.
    pub fn verify(&self, message: &[u8], signature: &Signature) -> Result<(), CryptoError> {
        use ed25519_dalek::Verifier;
        self.0
            .verify(message, &signature.0)
            .map_err(|_| CryptoError::SignatureVerificationFailed)
    }

    /// Сырые байты публичного ключа
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Создать из байт
    /// # Errors
    /// Returns `Err` if the bytes are not a valid Ed25519 public key.
    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, CryptoError> {
        VerifyingKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| CryptoError::InvalidPublicKey)
    }
}

/// Приватный ключ Ed25519 (32 байта)
/// Автоматически зануляется в памяти при drop (zeroize)
pub struct PrivateKey(SigningKey);

impl Drop for PrivateKey {
    fn drop(&mut self) {
        use zeroize::Zeroize;
        self.0.to_bytes().zeroize(); // zeroize a copy of the raw bytes
    }
}

impl PrivateKey {
    /// Получить соответствующий публичный ключ
    #[must_use]
    pub fn public_key(&self) -> PublicKey {
        PublicKey(self.0.verifying_key())
    }

    /// Подписать сообщение
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        use ed25519_dalek::Signer;
        let sig = self.0.sign(message);
        Signature(sig)
    }

    /// Сырые байты (осторожно! использовать только для экспорта/хранения)
    #[must_use]
    pub fn as_bytes(&self) -> &[u8; 32] {
        self.0.as_bytes()
    }

    /// Создать из байт
    #[must_use]
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(bytes))
    }
}

// Не выводим Debug для PrivateKey — случайно не залогируется
impl std::fmt::Debug for PrivateKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PrivateKey(***)")
    }
}

/// Пара ключей: приватный + публичный
pub struct Keypair {
    /// Приватный ключ
    pub private: PrivateKey,
    /// Публичный ключ
    pub public: PublicKey,
}

impl Keypair {
    /// Сгенерировать новую пару ключей с помощью CSPRNG ОС
    pub fn generate() -> Self {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut OsRng, &mut bytes);
        let signing_key = SigningKey::from_bytes(&bytes);
        let public = PublicKey(signing_key.verifying_key());
        let private = PrivateKey(signing_key);
        Self { private, public }
    }

    /// Получить адрес кошелька
    #[must_use]
    pub fn address(&self) -> Address {
        self.public.to_address()
    }

    /// Подписать транзакцию
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> Signature {
        self.private.sign(message)
    }
}
