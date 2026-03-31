#![allow(missing_docs)]
//! Составное поведение libp2p — объединяет все протоколы.
//!
//! libp2p использует паттерн "Behaviour" — каждый протокол
//! реализует трейт `NetworkBehaviour`, и мы объединяем их через derive-макрос.

use libp2p::{gossipsub, identify, kad, mdns, ping};
use libp2p_swarm_derive::NetworkBehaviour;

/// Составное поведение ноды quench
///
/// `#[derive(NetworkBehaviour)]` автоматически генерирует код,
/// который маршрутизирует события в нужный субпротокол.
#[derive(NetworkBehaviour)]
pub struct QuenchBehaviour {
    /// Шифрование + идентификация (автоматически, встроено в транспорт)
    pub identify: identify::Behaviour,
    /// Kademlia DHT — поиск пиров
    pub kademlia: kad::Behaviour<kad::store::MemoryStore>,
    /// mDNS — обнаружение пиров в локальной сети (удобно для тестов)
    pub mdns: mdns::tokio::Behaviour,
    /// Gossipsub — pub/sub рассылка блоков и транзакций
    pub gossipsub: gossipsub::Behaviour,
    /// Ping — keepalive
    pub ping: ping::Behaviour,
}
