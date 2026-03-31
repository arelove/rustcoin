//! Газ — ограничитель вычислительных ресурсов.
//!
//! Без лимита газа смарт-контракт мог бы запустить бесконечный цикл
//! и заблокировать всю ноду. Газ решает эту проблему.

/// Стоимость базовых операций в единицах газа
pub struct GasCost;

impl GasCost {
    /// Базовая стоимость вызова контракта
    pub const CONTRACT_CALL_BASE: u64 = 1_000;
    /// Стоимость деплоя (за байт байткода)
    pub const DEPLOY_PER_BYTE: u64 = 200;
    /// Запись в storage (дорого — меняет состояние блокчейна)
    pub const STORAGE_WRITE: u64 = 5_000;
    /// Чтение из storage
    pub const STORAGE_READ: u64 = 500;
    /// Перевод монет внутри контракта
    pub const TRANSFER: u64 = 2_000;
    /// Эмит события
    pub const EMIT_EVENT: u64 = 300;
    /// За каждый байт данных аргументов
    pub const DATA_PER_BYTE: u64 = 10;

    /// Максимальный газ в одном блоке
    pub const BLOCK_GAS_LIMIT: u64 = 10_000_000;

    /// Минимальная цена газа (в rustoshi)
    pub const MIN_GAS_PRICE: u64 = 1;
}

/// Вычислить стоимость газа для вызова
pub fn call_gas_cost(args_len: usize) -> u64 {
    GasCost::CONTRACT_CALL_BASE + (args_len as u64 * GasCost::DATA_PER_BYTE)
}

/// Вычислить стоимость деплоя
pub fn deploy_gas_cost(bytecode_len: usize) -> u64 {
    GasCost::CONTRACT_CALL_BASE + (bytecode_len as u64 * GasCost::DEPLOY_PER_BYTE)
}
