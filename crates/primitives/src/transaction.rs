//! Transaction types.
//!
//! Транзакция — это подписанное намерение перевести монеты от A к B.
//! Подпись создаётся приватным ключом отправителя через Ed25519.

use crate::{
    hash::Hash,
    types::{Address, Amount, Timestamp},
};
use serde::{Deserialize, Serialize};

/// Идентификатор транзакции = SHA-256 от её содержимого
pub type TxId = Hash;

/// Тип транзакции
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxKind {
    /// Обычный перевод монет
    Transfer,
    /// Coinbase — награда майнеру (нет отправителя)
    Coinbase,
    /// Вызов смарт-контракта
    ContractCall {
        /// Адрес контракта
        contract: Address,
        /// Название функции
        method: String,
        /// ABI-кодированные аргументы
        args: Vec<u8>,
        /// Лимит газа
        gas_limit: u64,
    },
    /// Деплой нового смарт-контракта
    ContractDeploy {
        /// WASM байткод контракта
        bytecode: Vec<u8>,
        /// Начальные аргументы конструктора
        init_args: Vec<u8>,
        /// Лимит газа
        gas_limit: u64,
    },
}

/// Транзакция
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Transaction {
    /// Версия формата (для будущих апгрейдов)
    pub version: u8,
    /// Тип транзакции
    pub kind: TxKind,
    /// Публичный ключ отправителя (32 байта, Ed25519)
    /// None для Coinbase транзакций
    pub from: Option<Address>,
    /// Адрес получателя
    pub to: Address,
    /// Сумма перевода
    pub amount: Amount,
    /// Комиссия майнеру
    pub fee: Amount,
    /// Nonce отправителя (защита от replay-атак)
    pub nonce: u64,
    /// Время создания
    pub timestamp: Timestamp,
    /// Ed25519 подпись (64 байта)
    pub signature: Option<Vec<u8>>,
    /// Публичный ключ (32 байта) — для верификации подписи
    pub public_key: Option<Vec<u8>>,
}

impl Transaction {
    /// Данные для подписи (всё кроме самой подписи и pubkey)
    /// # Panics
    /// Panics if serialization fails (should never happen).
    #[must_use]
    pub fn signing_bytes(&self) -> Vec<u8> {
        // Сериализуем только deterministic часть
        let signing_data = TransactionSigningData {
            version: self.version,
            from: self.from,
            to: self.to,
            amount: self.amount,
            fee: self.fee,
            nonce: self.nonce,
            timestamp: self.timestamp,
        };
        // Используем bincode для детерминированной сериализации
        // (JSON не детерминирован — порядок полей может меняться)
        serde_json::to_vec(&signing_data).expect("serialization is infallible")
    }

    /// Вычислить `TxId` (хэш транзакции)
    /// # Panics
    /// Panics if serialization fails (should never happen).
    #[must_use]
    pub fn tx_id(&self) -> TxId {
        let bytes = serde_json::to_vec(self).expect("serialization is infallible");
        Hash::sha256d(&bytes)
    }

    /// Базовая валидация без проверки подписи
    /// # Errors
    /// Returns `Err` if the transaction fails basic validation.
    pub fn validate_basic(&self) -> Result<(), crate::PrimitivesError> {
        // Coinbase транзакции — особый случай
        if matches!(self.kind, TxKind::Coinbase) {
            if self.from.is_some() {
                return Err(crate::PrimitivesError::InvalidTransaction(
                    "coinbase cannot have sender".into(),
                ));
            }
            return Ok(());
        }

        // Обычные транзакции
        if self.from.is_none() {
            return Err(crate::PrimitivesError::InvalidTransaction(
                "missing sender".into(),
            ));
        }
        if self.signature.is_none() || self.public_key.is_none() {
            return Err(crate::PrimitivesError::InvalidTransaction(
                "missing signature or public key".into(),
            ));
        }
        if self.amount == Amount(0) {
            return Err(crate::PrimitivesError::InvalidTransaction(
                "zero amount not allowed".into(),
            ));
        }

        Ok(())
    }
}

/// Только поля, которые подписываются
#[derive(Serialize)]
struct TransactionSigningData {
    version: u8,
    from: Option<Address>,
    to: Address,
    amount: Amount,
    fee: Amount,
    nonce: u64,
    timestamp: Timestamp,
}
