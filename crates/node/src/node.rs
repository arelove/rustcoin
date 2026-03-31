//! Главный объект ноды — оркестрирует все компоненты.

use std::sync::Arc;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

use rc_consensus::miner::Miner;
use rc_mempool::Mempool;
use rc_p2p::{Network, NetworkConfig, NetworkEvent};
use rc_primitives::{block::Block, hash::Hash};
use rc_rpc::{RpcServer, RpcServerConfig};
use rc_storage::Database;

use crate::{config::NodeConfig, error::NodeError, state::ChainState};

/// Событие внутренней шины ноды
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// Новый блок добавлен в цепочку
    NewBlock(Box<Block>),
    /// Новая транзакция в mempool
    NewTransaction(rc_primitives::transaction::TxId),
    /// Смена best tip
    ChainReorg {
        /// The hash of the previous chain tip before the reorg.
        old_tip: Hash,
        /// The hash of the new chain tip after the reorg.
        new_tip: Hash,
    },
    /// Нода завершает работу
    Shutdown,
}

/// Полная нода блокчейна
pub struct Node {
    config: NodeConfig,
    db: Database,
    mempool: Arc<Mempool>,
    chain_state: Arc<ChainState>,
    shutdown: CancellationToken,
}

impl Node {
    /// Инициализировать ноду (открыть БД, загрузить состояние)
    pub async fn new(config: NodeConfig) -> Result<Self, NodeError> {
        // Создаём директорию с данными
        std::fs::create_dir_all(&config.data_dir)
            .map_err(|e| NodeError::Init(format!("cannot create data dir: {e}")))?;

        // Открываем БД
        let db =
            Database::open(&config.db_path()).map_err(|e| NodeError::Storage(e.to_string()))?;

        // Проверяем genesis блок
        Self::ensure_genesis(&db)?;

        let mempool = Arc::new(Mempool::new());
        let shutdown = CancellationToken::new();

        // Восстанавливаем ChainState из БД (переживает рестарт ноды)
        let chain_state = Self::restore_chain_state(&db)?;

        info!(
            network  = %config.network,
            data_dir = %config.data_dir.display(),
            tip      = %chain_state.tip().hash,
            height   = %chain_state.tip().height,
            "Node initialized"
        );

        Ok(Self {
            config,
            db,
            mempool,
            chain_state,
            shutdown,
        })
    }

    /// Запустить ноду (запускает все компоненты, блокирует до shutdown)
    pub async fn run(self) -> Result<(), NodeError> {
        let (event_tx, _) = broadcast::channel::<NodeEvent>(256);

        // ── 1. P2P сеть ──────────────────────────────────────────────────────
        let network = Network::start(NetworkConfig {
            listen_port: self.config.p2p.port,
            bootstrap_peers: self
                .config
                .p2p
                .bootstrap_peers
                .iter()
                .filter_map(|s| s.parse().ok())
                .collect(),
            max_peers: self.config.p2p.max_peers,
            network_name: self.config.network.clone(),
        })
        .await
        .map_err(|e| NodeError::Network(e.to_string()))?;

        info!(peer_id = %network.local_peer_id, "P2P started");

        // ── 2. RPC сервер ─────────────────────────────────────────────────────
        if self.config.rpc.enabled {
            let rpc_addr = format!("{}:{}", self.config.rpc.host, self.config.rpc.port)
                .parse()
                .map_err(|_| NodeError::Init("invalid rpc addr".into()))?;

            let rpc_server = RpcServer::new(
                RpcServerConfig {
                    bind_addr: rpc_addr,
                    enable_cors: self.config.rpc.cors,
                },
                self.db.clone(),
                self.mempool.clone(),
            );

            let shutdown = self.shutdown.clone();
            tokio::spawn(async move {
                tokio::select! {
                    res = rpc_server.run() => {
                        if let Err(e) = res { error!("RPC error: {e}"); }
                    }
                    _ = shutdown.cancelled() => {}
                }
            });
        }

        // ── 3. Майнинг ────────────────────────────────────────────────────────
        if let Some(mining_config) = &self.config.mining {
            let coinbase_addr =
                rc_primitives::types::Address::from_base58(&mining_config.coinbase_address)
                    .map_err(|_| NodeError::Init("invalid coinbase address".into()))?;

            let miner = Arc::new(Miner::new(coinbase_addr, self.shutdown.clone()));
            let db = self.db.clone();
            let mempool = self.mempool.clone();
            let chain_state = self.chain_state.clone();
            let event_tx = event_tx.clone();
            let shutdown = self.shutdown.clone();

            tokio::spawn(async move {
                Self::mining_loop(miner, db, mempool, chain_state, event_tx, shutdown).await;
            });
        }

        // ── 4. Network event loop ─────────────────────────────────────────────
        let db = self.db.clone();
        let mempool = self.mempool.clone();
        let chain_state = self.chain_state.clone();
        let event_tx2 = event_tx.clone();
        let shutdown = self.shutdown.clone();

        tokio::spawn(async move {
            Self::network_event_loop(network, db, mempool, chain_state, event_tx2, shutdown).await;
        });

        // ── 5. Ждём сигнала завершения ────────────────────────────────────────
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                info!("Received Ctrl-C, shutting down...");
                self.shutdown.cancel();
            }
            _ = self.shutdown.cancelled() => {}
        }

        info!("Node stopped");
        Ok(())
    }

    /// Проверить или создать genesis блок
    fn ensure_genesis(db: &Database) -> Result<(), NodeError> {
        if db
            .get_block_at(0.into())
            .map_err(|e| NodeError::Storage(e.to_string()))?
            .is_none()
        {
            let genesis = Block::genesis();
            // Применяем genesis через state machine (coinbase → запись балансов)
            db.apply_block(&genesis)
                .map_err(|e| NodeError::Storage(e.to_string()))?;
            info!("Genesis block created and applied: {}", genesis.hash());
        }
        Ok(())
    }

    /// Восстановить ChainState из БД после рестарта.
    ///
    /// Если `best_tip` есть в метаданных — берём оттуда.
    /// Иначе — genesis (height 0).
    fn restore_chain_state(db: &Database) -> Result<Arc<ChainState>, NodeError> {
        let chain_state = ChainState::new(1);

        if let Some((hash, height)) = db
            .get_best_tip()
            .map_err(|e| NodeError::Storage(e.to_string()))?
        {
            chain_state.update_tip(hash, height);
            info!(hash = %hash, height = %height, "Chain state restored from DB");
        }

        Ok(chain_state)
    }

    /// Цикл майнинга
    async fn mining_loop(
        miner: Arc<Miner>,
        db: Database,
        mempool: Arc<Mempool>,
        chain_state: Arc<ChainState>,
        event_tx: broadcast::Sender<NodeEvent>,
        shutdown: CancellationToken,
    ) {
        info!("Mining started");

        loop {
            if shutdown.is_cancelled() {
                break;
            }

            let tip = chain_state.tip();
            let height = rc_primitives::types::BlockHeight(tip.height.0 + 1);
            let txs = mempool.select_for_block(500);

            match miner.mine_async(tip.hash, height, txs, 20, 1).await {
                Ok(result) => {
                    let block = result.block;
                    let hash = block.hash();

                    // apply_block атомарно: сохраняет блок + txs + обновляет балансы + best_tip
                    match db.apply_block(&block) {
                        Ok(apply_result) => {
                            info!(
                                height       = %height,
                                hash         = %hash,
                                txs_applied  = apply_result.txs_applied,
                                txs_skipped  = apply_result.txs_skipped,
                                "✓ Block mined and applied"
                            );
                        }
                        Err(e) => {
                            error!(height = %height, hash = %hash, err = %e, "Failed to apply mined block");
                            continue;
                        }
                    }

                    // Удаляем подтверждённые транзакции из mempool
                    mempool.remove_confirmed(
                        &block
                            .transactions
                            .iter()
                            .map(|t| t.tx_id())
                            .collect::<Vec<_>>(),
                    );

                    // Обновляем in-memory tip
                    chain_state.update_tip(hash, height);

                    let _ = event_tx.send(NodeEvent::NewBlock(Box::new(block)));
                }
                Err(rc_consensus::error::ConsensusError::MiningCancelled) => {
                    // Нормальная ситуация — получили блок от сети, начинаем заново
                }
                Err(e) => {
                    warn!("Mining error: {e}");
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        }
    }

    /// Цикл обработки сетевых событий
    async fn network_event_loop(
        mut network: Network,
        db: Database,
        mempool: Arc<Mempool>,
        chain_state: Arc<ChainState>,
        event_tx: broadcast::Sender<NodeEvent>,
        shutdown: CancellationToken,
    ) {
        loop {
            tokio::select! {
                Some(event) = network.rx.recv() => {
                    match event {
                        NetworkEvent::PeerConnected(peer)    => info!(%peer, "Peer connected"),
                        NetworkEvent::PeerDisconnected(peer) => info!(%peer, "Peer disconnected"),

                        NetworkEvent::NewBlock { block, .. } => {
                            let hash   = block.hash();
                            let height = block.height();

                            // apply_block идемпотентен — безопасно вызывать
                            // даже если блок уже есть (например, мы сами его майнили)
                            match db.apply_block(&block) {
                                Ok(apply_result) => {
                                    let tip = chain_state.tip();
                                    if height.0 > tip.height.0 {
                                        chain_state.update_tip(hash, height);
                                        let _ = event_tx.send(NodeEvent::NewBlock(block));
                                        info!(
                                            height      = %height,
                                            hash        = %hash,
                                            txs_applied = apply_result.txs_applied,
                                            "Incoming block applied"
                                        );
                                    }
                                }
                                Err(e) => {
                                    warn!(hash = %hash, height = %height, err = %e,
                                          "Failed to apply incoming block");
                                }
                            }
                        }

                        NetworkEvent::NewTransaction { tx, .. } => {
                            if let Ok(txid) = mempool.add(*tx) {
                                let _ = event_tx.send(NodeEvent::NewTransaction(txid));
                            }
                        }

                        NetworkEvent::SyncRequest { from, start_height } => {
                            // TODO: отправить блоки начиная с start_height
                            info!(%from, start_height, "Sync request received");
                        }
                    }
                }
                _ = shutdown.cancelled() => break,
            }
        }
    }
}
