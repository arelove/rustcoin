//! AccountState — состояние аккаунта в блокчейне.

use rc_primitives::types::Amount;
use serde::{Deserialize, Serialize};

/// Состояние аккаунта (Account Model, как в Ethereum)
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct AccountState {
    /// Баланс в rustoshi (1 RSC = 100_000_000 rustoshi)
    pub balance: Amount,
    /// Nonce: количество отправленных транзакций.
    /// Защищает от replay-атак: каждая транзакция должна иметь nonce = текущий nonce аккаунта.
    pub nonce: u64,
    /// Если аккаунт является контрактом — хэш его кода
    pub code_hash: Option<String>,
    /// Хранилище контракта (key-value, используется для state смарт-контрактов)
    pub storage_root: Option<String>,
}

impl AccountState {
    /// Применить дебет (списание)
    pub fn debit(&mut self, amount: Amount) -> Result<(), crate::StorageError> {
        self.balance = self
            .balance
            .checked_sub(amount)
            .ok_or(crate::StorageError::InsufficientBalance)?;
        Ok(())
    }

    /// Применить кредит (зачисление)
    pub fn credit(&mut self, amount: Amount) -> Result<(), crate::StorageError> {
        self.balance = self
            .balance
            .checked_add(amount)
            .ok_or(crate::StorageError::Overflow)?;
        Ok(())
    }

    /// Увеличить nonce (вызывается после каждой исходящей транзакции)
    pub fn increment_nonce(&mut self) {
        self.nonce += 1;
    }

    /// Является ли аккаунт смарт-контрактом?
    pub fn is_contract(&self) -> bool {
        self.code_hash.is_some()
    }
}
