//! Алгоритм выбора форка (fork-choice rule).
//!
//! При конкурирующих цепочках (форках) нода выбирает
//! цепочку с наибольшей **суммарной сложностью** (total work),
//! а не просто самую длинную.
//!
//! Это предотвращает атаки, когда злоумышленник создаёт длинную цепочку
//! с низкой сложностью.

use rc_primitives::{block::BlockHeader, hash::Hash};
use std::collections::HashMap;

/// Узел в дереве цепочек
pub struct ChainNode {
    /// The block header stored at this node.
    pub header: BlockHeader,
    /// Cumulative proof-of-work from genesis to this node.
    pub total_work: u128,
}

/// Движок выбора форка
///
/// Хранит все известные заголовки блоков и позволяет
/// определить "лучшую" цепочку в любой момент.
pub struct ForkChoice {
    /// Все известные блоки: hash → node
    nodes: HashMap<Hash, ChainNode>,
    /// Текущий tip (голова) лучшей цепи
    best_tip: Option<Hash>,
}

impl ForkChoice {
    /// Creates a new empty `ForkChoice` with no known chain tip.
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            best_tip: None,
        }
    }

    /// Добавить новый блок-кандидат
    ///
    /// Возвращает `true`, если этот блок стал новым best tip.
    pub fn add_block(&mut self, header: BlockHeader) -> bool {
        let hash = header.compute_hash();

        // Вычисляем суммарную сложность
        let parent_work = header
            .height
            .0
            .checked_sub(1)
            .and_then(|_| self.nodes.get(&header.previous_hash))
            .map(|n| n.total_work)
            .unwrap_or(0);

        // Work для этого блока ≈ 2^bits (упрощённо)
        let block_work: u128 = 1u128 << header.bits.min(127);
        let total_work = parent_work.saturating_add(block_work);

        self.nodes.insert(hash, ChainNode { header, total_work });

        // Проверяем, лучше ли эта цепочка текущей
        let is_new_best = match self.best_tip {
            None => true,
            Some(tip_hash) => {
                let tip_work = self.nodes[&tip_hash].total_work;
                total_work > tip_work
            }
        };

        if is_new_best {
            self.best_tip = Some(hash);
        }

        is_new_best
    }

    /// Хэш текущего best tip
    pub fn best_tip(&self) -> Option<Hash> {
        self.best_tip
    }

    /// Получить цепочку хэшей от tip до genesis
    pub fn ancestry(&self, tip: Hash) -> Vec<Hash> {
        let mut chain = Vec::new();
        let mut current = tip;

        loop {
            chain.push(current);
            match self.nodes.get(&current) {
                Some(node) if node.header.height.0 > 0 => {
                    current = node.header.previous_hash;
                }
                _ => break,
            }
        }

        chain.reverse();
        chain
    }
}

impl Default for ForkChoice {
    fn default() -> Self {
        Self::new()
    }
}
