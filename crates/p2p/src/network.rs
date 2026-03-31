//! Главный объект сети — инициализация swarm и event loop.

use crate::{
    behaviour::QuenchBehaviour, error::P2pError, event::NetworkEvent, message::NetworkMessage,
};
use libp2p::{
    gossipsub, identify, kad, mdns, noise, ping, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId,
    SwarmBuilder,
};
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

/// Конфигурация сети
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Порт для входящих соединений
    pub listen_port: u16,
    /// Начальные пиры для бутстрапа (адреса известных нод)
    pub bootstrap_peers: Vec<Multiaddr>,
    /// Максимальное число пиров
    pub max_peers: usize,
    /// Название сети (отличает mainnet / testnet)
    pub network_name: String,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            listen_port: 8333,
            bootstrap_peers: vec![],
            max_peers: 50,
            network_name: "quench-mainnet".into(),
        }
    }
}

/// Хэндл для взаимодействия с сетевым слоем из других частей ноды
pub struct Network {
    /// Канал для отправки сообщений в сеть
    pub tx: mpsc::Sender<NetworkMessage>,
    /// Канал для получения событий из сети
    pub rx: mpsc::Receiver<NetworkEvent>,
    /// Наш PeerId
    pub local_peer_id: PeerId,
}

impl Network {
    /// Инициализировать сеть и запустить event loop в фоне
    pub async fn start(config: NetworkConfig) -> Result<Self, P2pError> {
        let (event_tx, event_rx) = mpsc::channel::<NetworkEvent>(256);
        let (msg_tx, msg_rx) = mpsc::channel::<NetworkMessage>(256);

        // Gossipsub конфигурация
        let gossipsub_config = gossipsub::ConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10))
            .validation_mode(gossipsub::ValidationMode::Strict)
            .max_transmit_size(4 * 1024 * 1024) // 4MB макс. размер сообщения
            .build()
            .map_err(|e| P2pError::Init(e.to_string()))?;

        // Строим swarm через новый builder API (libp2p 0.53+)
        let mut swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| P2pError::Init(e.to_string()))?
            .with_behaviour(|key| {
                let peer_id = key.public().to_peer_id();

                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )
                .expect("gossipsub init");

                let mut kademlia =
                    kad::Behaviour::new(peer_id, kad::store::MemoryStore::new(peer_id));
                kademlia.set_mode(Some(kad::Mode::Server));

                Ok(QuenchBehaviour {
                    identify: identify::Behaviour::new(identify::Config::new(
                        format!("/quench/1.0"),
                        key.public(),
                    )),
                    kademlia,
                    mdns: mdns::tokio::Behaviour::new(mdns::Config::default(), peer_id)
                        .expect("mdns init"),
                    gossipsub,
                    ping: ping::Behaviour::new(
                        ping::Config::new().with_interval(Duration::from_secs(30)),
                    ),
                })
            })
            .map_err(|e| P2pError::Init(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let local_peer_id = *swarm.local_peer_id();

        // Подписываемся на все топики
        for topic_str in &[
            "quench/blocks/1",
            "quench/txs/1",
            "quench/headers/1",
            "quench/control/1",
        ] {
            let topic = gossipsub::IdentTopic::new(*topic_str);
            swarm
                .behaviour_mut()
                .gossipsub
                .subscribe(&topic)
                .map_err(|e| P2pError::Init(e.to_string()))?;
        }

        // Начинаем слушать
        let listen_addr: Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", config.listen_port)
            .parse()
            .map_err(|_| P2pError::InvalidAddr)?;
        swarm
            .listen_on(listen_addr)
            .map_err(|e| P2pError::Init(e.to_string()))?;

        // Коннектимся к bootstrap пирам
        for addr in &config.bootstrap_peers {
            swarm.dial(addr.clone()).ok();
        }

        info!(peer_id = %local_peer_id, port = config.listen_port, "P2P network started");

        // Запускаем event loop в фоновой задаче
        tokio::spawn(async move {
            Self::run_event_loop(swarm, event_tx, msg_rx).await;
        });

        Ok(Self {
            tx: msg_tx,
            rx: event_rx,
            local_peer_id,
        })
    }

    /// Broadcast сообщение всем подписчикам топика
    pub async fn broadcast(&self, msg: NetworkMessage) -> Result<(), P2pError> {
        self.tx.send(msg).await.map_err(|_| P2pError::ChannelClosed)
    }

    /// Основной цикл обработки событий swarm
    async fn run_event_loop(
        mut swarm: libp2p::Swarm<QuenchBehaviour>,
        event_tx: mpsc::Sender<NetworkEvent>,
        mut msg_rx: mpsc::Receiver<NetworkMessage>,
    ) {
        use futures::StreamExt;

        loop {
            tokio::select! {
                // Событие от libp2p
                event = swarm.select_next_some() => {
                    Self::handle_swarm_event(event, &event_tx, &mut swarm).await;
                }
                // Сообщение для отправки в сеть
                Some(msg) = msg_rx.recv() => {
                    if let Ok(data) = msg.encode() {
                        let topic = gossipsub::IdentTopic::new(msg.topic());
                        if let Err(e) = swarm.behaviour_mut().gossipsub.publish(topic, data) {
                            warn!("gossipsub publish error: {e}");
                        }
                    }
                }
            }
        }
    }

    async fn handle_swarm_event(
        event: SwarmEvent<crate::behaviour::QuenchBehaviourEvent>,
        event_tx: &mpsc::Sender<NetworkEvent>,
        swarm: &mut libp2p::Swarm<QuenchBehaviour>,
    ) {
        match event {
            SwarmEvent::NewListenAddr { address, .. } => {
                info!("Listening on {address}");
            }
            SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                debug!(%peer_id, "peer connected");
                let _ = event_tx.send(NetworkEvent::PeerConnected(peer_id)).await;
            }
            SwarmEvent::ConnectionClosed { peer_id, .. } => {
                debug!(%peer_id, "peer disconnected");
                let _ = event_tx.send(NetworkEvent::PeerDisconnected(peer_id)).await;
            }
            SwarmEvent::Behaviour(crate::behaviour::QuenchBehaviourEvent::Gossipsub(
                gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                },
            )) => match NetworkMessage::decode(&message.data) {
                Ok(NetworkMessage::NewBlock(block)) => {
                    let _ = event_tx
                        .send(NetworkEvent::NewBlock {
                            from: propagation_source,
                            block,
                        })
                        .await;
                }
                Ok(NetworkMessage::NewTransaction(tx)) => {
                    let _ = event_tx
                        .send(NetworkEvent::NewTransaction {
                            from: propagation_source,
                            tx,
                        })
                        .await;
                }
                Ok(other) => debug!("received control message: {:?}", other),
                Err(e) => warn!("failed to decode message: {e}"),
            },
            SwarmEvent::Behaviour(crate::behaviour::QuenchBehaviourEvent::Mdns(
                mdns::Event::Discovered(peers),
            )) => {
                for (peer_id, addr) in peers {
                    info!(%peer_id, %addr, "mDNS discovered peer");
                    swarm.behaviour_mut().kademlia.add_address(&peer_id, addr);
                    swarm.dial(peer_id).ok();
                }
            }
            _ => {}
        }
    }
}
