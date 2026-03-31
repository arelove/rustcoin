//! Построитель транзакций — удобный fluent API.
//!
//! Пример:
//! ```rust,ignore
//! let tx = TransactionBuilder::new()
//!     .from(my_address)
//!     .to(recipient)
//!     .amount(Amount::ONE)
//!     .fee(Amount(1000))
//!     .nonce(current_nonce)
//!     .sign(&keypair)?;
//! ```

use crate::error::WalletError;
use rc_crypto::keypair::Keypair;
use rc_primitives::{
    transaction::{Transaction, TxKind},
    types::{Address, Amount, Timestamp},
};

/// Построитель транзакции
#[derive(Default)]
pub struct TransactionBuilder {
    from: Option<Address>,
    to: Option<Address>,
    amount: Option<Amount>,
    fee: Option<Amount>,
    nonce: Option<u64>,
    kind: Option<TxKind>,
}

impl TransactionBuilder {
    /// Создать новый построитель
    pub fn new() -> Self {
        Self::default()
    }

    /// Адрес отправителя
    pub fn from(mut self, addr: Address) -> Self {
        self.from = Some(addr);
        self
    }

    /// Адрес получателя
    pub fn to(mut self, addr: Address) -> Self {
        self.to = Some(addr);
        self
    }

    /// Сумма перевода
    pub fn amount(mut self, amount: Amount) -> Self {
        self.amount = Some(amount);
        self
    }

    /// Комиссия майнеру
    pub fn fee(mut self, fee: Amount) -> Self {
        self.fee = Some(fee);
        self
    }

    /// Nonce отправителя (должен совпадать с текущим nonce аккаунта в сети)
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Переопределить тип транзакции (по умолчанию — Transfer)
    pub fn kind(mut self, kind: TxKind) -> Self {
        self.kind = Some(kind);
        self
    }

    /// Подписать транзакцию и вернуть готовый объект
    pub fn sign(self, keypair: &Keypair) -> Result<Transaction, WalletError> {
        let from = self.from.ok_or(WalletError::MissingField("from"))?;
        let to = self.to.ok_or(WalletError::MissingField("to"))?;
        let amount = self.amount.ok_or(WalletError::MissingField("amount"))?;
        let fee = self.fee.unwrap_or(Amount(0));
        let nonce = self.nonce.ok_or(WalletError::MissingField("nonce"))?;
        let kind = self.kind.unwrap_or(TxKind::Transfer);

        let mut tx = Transaction {
            version: 1,
            kind,
            from: Some(from),
            to,
            amount,
            fee,
            nonce,
            timestamp: Timestamp::now(),
            signature: None,
            public_key: None,
        };

        // Подписываем
        let signing_bytes = tx.signing_bytes();
        let signature = keypair.sign(&signing_bytes);

        tx.signature = Some(signature.to_vec());
        tx.public_key = Some(keypair.public.as_bytes().to_vec());

        Ok(tx)
    }
}
