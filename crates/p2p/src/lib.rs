//! # rc-p2p
//!
//! P2P сетевой слой на базе `libp2p`.
//!
//! ## Протоколы
//!
//! | Протокол         | Назначение                                     |
//! |------------------|------------------------------------------------|
//! | **Noise**        | Шифрование соединений (как TLS, но легче)      |
//! | **Yamux**        | Мультиплексирование стримов поверх одного TCP  |
//! | **Identify**     | Узлы сообщают друг другу свои адреса и версии  |
//! | **Kademlia DHT** | Поиск других узлов в сети (peer discovery)     |
//! | **mDNS**         | Локальный поиск узлов в той же сети (LAN)      |
//! | **Gossipsub**    | Рассылка новых блоков и транзакций             |
//! | **Ping**         | Keepalive и измерение latency                  |
//!
//! ## Сообщения (Gossipsub topics)
//!
//! - `quench/blocks/1` — новые блоки
//! - `quench/txs/1` — новые транзакции
//! - `quench/headers/1` — только заголовки (для light clients)

#![forbid(unsafe_code)]
#![deny(missing_docs, clippy::all, clippy::pedantic)]
#![allow(clippy::large_enum_variant)]

pub mod behaviour;
/// Error types for the P2P layer.
pub mod error;
pub mod event;
pub mod message;
pub mod network;

pub use error::P2pError;
pub use event::NetworkEvent;
pub use message::NetworkMessage;
pub use network::{Network, NetworkConfig};
