//! Разделяемое состояние ноды.
//!
//! Все компоненты держат `Arc<ChainState>` — это потокобезопасный
//! доступ к текущему состоянию цепочки без глобальных переменных.

use parking_lot::RwLock;
use rc_primitives::{hash::Hash, types::BlockHeight};
use std::sync::Arc;

/// Текущее состояние цепочки
#[derive(Debug, Default, Clone)]
pub struct ChainTip {
    /// Хэш текущего лучшего блока
    pub hash: Hash,
    /// Высота текущего лучшего блока
    pub height: BlockHeight,
}

/// Разделяемое состояние — доступно из всех компонентов ноды
pub struct ChainState {
    /// Текущий tip (голова) лучшей цепочки
    tip: RwLock<ChainTip>,
    /// Флаг синхронизации
    is_syncing: RwLock<bool>,
    /// Версия сети
    pub network_version: u32,
}

impl ChainState {
    /// Creates a new `ChainState` wrapped in an `Arc`, ready for shared access.
    pub fn new(network_version: u32) -> Arc<Self> {
        Arc::new(Self {
            tip: RwLock::new(ChainTip::default()),
            is_syncing: RwLock::new(false),
            network_version,
        })
    }

    /// Получить текущий tip
    pub fn tip(&self) -> ChainTip {
        self.tip.read().clone()
    }

    /// Обновить tip после добавления нового блока
    pub fn update_tip(&self, hash: Hash, height: BlockHeight) {
        let mut tip = self.tip.write();
        tip.hash = hash;
        tip.height = height;
    }

    /// Идёт ли синхронизация с сетью?
    pub fn is_syncing(&self) -> bool {
        *self.is_syncing.read()
    }

    /// Sets the syncing flag — `true` while the node is catching up to the network.
    pub fn set_syncing(&self, syncing: bool) {
        *self.is_syncing.write() = syncing;
    }
}
