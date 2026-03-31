//! Алгоритм корректировки сложности.
//!
//! Каждые `ADJUSTMENT_INTERVAL` блоков пересчитываем сложность,
//! чтобы среднее время блока стремилось к `TARGET_BLOCK_TIME_MS`.
//!
//! ## Алгоритм (упрощённый Bitcoin)
//!
//! ```text
//! new_bits = old_bits * (actual_time / expected_time)
//! ```
//! С ограничением: сложность может изменяться не более чем в 4 раза за период.

use rc_primitives::types::{BlockHeight, Timestamp};

/// Целевое время между блоками: 10 минут (как в Bitcoin)
pub const TARGET_BLOCK_TIME_MS: u64 = 10 * 60 * 1_000;

/// Интервал корректировки: каждые 2016 блоков (как в Bitcoin)
pub const ADJUSTMENT_INTERVAL: u64 = 2_016;

/// Минимальная сложность (биты)
pub const MIN_DIFFICULTY: u32 = 16;

/// Максимальная сложность (биты)
pub const MAX_DIFFICULTY: u32 = 64;

/// Вычислить новую сложность после окончания периода корректировки
///
/// # Аргументы
/// - `current_bits` — текущая сложность
/// - `period_start_time` — timestamp первого блока периода
/// - `period_end_time` — timestamp последнего блока периода
pub fn compute_target(
    current_bits: u32,
    period_start_time: Timestamp,
    period_end_time: Timestamp,
) -> u32 {
    let expected_time_ms = ADJUSTMENT_INTERVAL * TARGET_BLOCK_TIME_MS;
    let actual_time_ms = period_end_time.0.saturating_sub(period_start_time.0).max(1); // не делим на 0

    // Новая сложность: current * (expected / actual)
    // Если блоки шли быстрее — сложность растёт, медленнее — падает
    let new_bits =
        (current_bits as u128 * expected_time_ms as u128 / actual_time_ms as u128) as u32;

    // Ограничиваем диапазон изменений (не более 4x за период)
    let new_bits = new_bits.max(current_bits / 4).min(current_bits * 4);

    // Абсолютные ограничения
    new_bits.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
}

/// Хранит историю блоков для корректировки сложности
pub struct DifficultyAdjuster {
    period_start: Option<(BlockHeight, Timestamp)>,
    current_bits: u32,
}

impl DifficultyAdjuster {
    /// Creates a new `DifficultyAdjuster` with the given initial target bits.
    pub fn new(initial_bits: u32) -> Self {
        Self {
            period_start: None,
            current_bits: initial_bits,
        }
    }

    /// Уведомить о новом блоке; возвращает новую сложность если произошла корректировка
    pub fn on_new_block(&mut self, height: BlockHeight, timestamp: Timestamp) -> Option<u32> {
        // Запоминаем начало периода
        if self.period_start.is_none() {
            self.period_start = Some((height, timestamp));
        }

        // Корректировка на кратных высотах
        if height.0 > 0 && height.0 % ADJUSTMENT_INTERVAL == 0 {
            if let Some((_, start_time)) = self.period_start {
                let new_bits = compute_target(self.current_bits, start_time, timestamp);
                self.current_bits = new_bits;
                self.period_start = Some((height, timestamp)); // новый период
                return Some(new_bits);
            }
        }

        None
    }
    /// Returns the current target bits.
    pub fn current_bits(&self) -> u32 {
        self.current_bits
    }
}
