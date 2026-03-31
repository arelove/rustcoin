//! Контекст выполнения смарт-контракта.
//!
//! Содержит всю информацию, доступную контракту во время выполнения:
//! кто вызвал, сколько монет приложено, текущая высота блока и т.д.

use rc_primitives::types::{Address, Amount, BlockHeight, Timestamp};

/// Событие, эмитированное контрактом (аналог Ethereum logs)
#[derive(Debug, Clone)]
pub struct ContractEvent {
    /// Адрес контракта, который эмитировал событие
    pub contract: Address,
    /// Название события (например, "Transfer", "Approval")
    pub name: String,
    /// Данные события (JSON)
    pub data: Vec<u8>,
}

/// Контекст вызова контракта
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    /// Адрес контракта
    pub contract_address: Address,
    /// Адрес вызывающего (кошелёк или другой контракт)
    pub caller: Address,
    /// Первоначальный инициатор транзакции (всегда кошелёк)
    pub origin: Address,
    /// Монеты, приложенные к вызову
    pub value: Amount,
    /// Текущая высота блока
    pub block_height: BlockHeight,
    /// Текущий timestamp
    pub block_timestamp: Timestamp,
    /// Лимит газа для этого вызова
    pub gas_limit: u64,
    /// Название вызываемого метода
    pub method: String,
    /// ABI-кодированные аргументы
    pub args: Vec<u8>,
}

/// Мутабельное состояние, накапливаемое в процессе выполнения
#[derive(Debug, Default)]
pub struct ExecutionState {
    /// Изменения storage контракта (применяются атомарно после успеха)
    pub storage_writes: Vec<(Vec<u8>, Vec<u8>)>,
    /// Переводы монет (применяются после успеха)
    pub transfers: Vec<(Address, Amount)>,
    /// Эмитированные события
    pub events: Vec<ContractEvent>,
    /// Использованный газ
    pub gas_used: u64,
}
