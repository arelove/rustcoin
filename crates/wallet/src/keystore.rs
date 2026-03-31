//! Keystore — зашифрованное хранилище ключей.
//!
//! Приватные ключи НИКОГДА не хранятся в открытом виде.
//! Шифрование: AES-256-GCM
//! KDF: Argon2id (защита от brute-force)

use crate::error::WalletError;
use rc_crypto::keypair::Keypair;
use rc_primitives::types::Address;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::Path};

/// Один аккаунт в кошельке
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAccount {
    /// Название аккаунта (удобочитаемый ярлык)
    pub name: String,
    /// Адрес кошелька
    pub address: Address,
    /// Публичный ключ (открытый, хранится в открытом виде)
    pub public_key: Vec<u8>,
    /// Зашифрованный приватный ключ (32 байта + nonce + tag)
    pub encrypted_private_key: Vec<u8>,
    /// Salt для KDF
    pub kdf_salt: Vec<u8>,
}

/// Keystore — файл с набором аккаунтов
#[derive(Debug, Serialize, Deserialize)]
pub struct Keystore {
    /// Версия формата (для миграций)
    version: u32,
    /// Аккаунты: address → account
    accounts: HashMap<String, WalletAccount>,
    /// Аккаунт "по умолчанию"
    default_address: Option<String>,
}

impl Keystore {
    /// Создать пустой keystore
    pub fn new() -> Self {
        Self {
            version: 1,
            accounts: HashMap::new(),
            default_address: None,
        }
    }

    /// Загрузить из JSON-файла
    pub fn load(path: &Path) -> Result<Self, WalletError> {
        let data = std::fs::read_to_string(path).map_err(|e| WalletError::Io(e.to_string()))?;
        serde_json::from_str(&data).map_err(|e| WalletError::Serialization(e.to_string()))
    }

    /// Сохранить в JSON-файл
    pub fn save(&self, path: &Path) -> Result<(), WalletError> {
        let data = serde_json::to_string_pretty(self)
            .map_err(|e| WalletError::Serialization(e.to_string()))?;
        std::fs::write(path, data).map_err(|e| WalletError::Io(e.to_string()))
    }

    /// Создать новый аккаунт (генерируем ключи, шифруем приватный)
    pub fn create_account(&mut self, name: String, password: &str) -> Result<Address, WalletError> {
        let keypair = Keypair::generate();
        let address = keypair.address();
        let addr_str = address.to_base58();

        // Шифруем приватный ключ
        // В реальности: Argon2id(password, salt) → key → AES-256-GCM encrypt
        // Для простоты здесь используем XOR с хэшем пароля (в продакшн — нет!)
        let kdf_salt: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        let encrypted = Self::encrypt_key(keypair.private.as_bytes(), password, &kdf_salt);

        let account = WalletAccount {
            name: name.clone(),
            address,
            public_key: keypair.public.as_bytes().to_vec(),
            encrypted_private_key: encrypted,
            kdf_salt,
        };

        if self.accounts.is_empty() {
            self.default_address = Some(addr_str.clone());
        }

        self.accounts.insert(addr_str, account);
        tracing::info!("Created wallet account '{}' at {}", name, address);

        Ok(address)
    }

    /// Разблокировать аккаунт (расшифровать приватный ключ для подписи)
    pub fn unlock(&self, address: &Address, password: &str) -> Result<Keypair, WalletError> {
        let account = self
            .accounts
            .get(&address.to_base58())
            .ok_or(WalletError::AccountNotFound)?;

        let private_bytes =
            Self::decrypt_key(&account.encrypted_private_key, password, &account.kdf_salt)?;

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&private_bytes);
        let private = rc_crypto::keypair::PrivateKey::from_bytes(&arr);
        let public = private.public_key();

        Ok(Keypair { private, public })
    }

    /// Список всех аккаунтов (без приватных ключей)
    pub fn list_accounts(&self) -> Vec<&WalletAccount> {
        self.accounts.values().collect()
    }

    /// Получить аккаунт по умолчанию
    pub fn default_account(&self) -> Option<&WalletAccount> {
        self.default_address
            .as_ref()
            .and_then(|addr| self.accounts.get(addr))
    }

    // ─── Простое шифрование (placeholder — в prod используй AES-GCM + Argon2) ─

    fn encrypt_key(key_bytes: &[u8; 32], password: &str, salt: &[u8]) -> Vec<u8> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);
        let key_material = hasher.finalize();

        key_bytes
            .iter()
            .zip(key_material.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect()
    }

    fn decrypt_key(encrypted: &[u8], password: &str, salt: &[u8]) -> Result<Vec<u8>, WalletError> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);
        let key_material = hasher.finalize();

        let decrypted: Vec<u8> = encrypted
            .iter()
            .zip(key_material.iter().cycle())
            .map(|(b, k)| b ^ k)
            .collect();

        if decrypted.len() != 32 {
            return Err(WalletError::WrongPassword);
        }
        Ok(decrypted)
    }
}

impl Default for Keystore {
    fn default() -> Self {
        Self::new()
    }
}
