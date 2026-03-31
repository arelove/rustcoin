//! Конфигурация ноды — загружается из файла `node.toml` и env-переменных.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Полная конфигурация ноды
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Путь к директории с данными (БД, логи, keystore)
    pub data_dir: PathBuf,

    /// Название сети: "mainnet" | "testnet" | "devnet"
    pub network: String,

    /// Настройки P2P сети
    pub p2p: P2pConfig,

    /// Настройки RPC сервера
    pub rpc: RpcConfig,

    /// Настройки майнинга (None = нода не майнит)
    pub mining: Option<MiningConfig>,

    /// Настройки логирования
    pub log: LogConfig,
}

/// P2P network configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pConfig {
    /// TCP port to listen for incoming P2P connections.
    pub port: u16,
    /// Addresses of known bootstrap peers for initial connection.
    pub bootstrap_peers: Vec<String>,
    /// Maximum number of simultaneous peer connections.
    pub max_peers: usize,
}

/// RPC server configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Whether the RPC server is enabled.
    pub enabled: bool,
    /// Host address to bind the RPC server to.
    pub host: String,
    /// Port to bind the RPC server to.
    pub port: u16,
    /// Whether to enable CORS headers.
    pub cors: bool,
}

/// Mining configuration. If absent, the node does not mine.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Wallet address that receives block rewards.
    pub coinbase_address: String,
    /// Number of mining threads (0 = auto-detect CPU count).
    pub threads: usize,
}
/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogConfig {
    /// Log level: `"trace"`, `"debug"`, `"info"`, `"warn"`, or `"error"`.
    pub level: String,
    /// Log format: `"pretty"`, `"json"`, or `"compact"`.
    pub format: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: PathBuf::from("./data"),
            network: "devnet".into(),
            p2p: P2pConfig {
                port: 8333,
                bootstrap_peers: vec![],
                max_peers: 50,
            },
            rpc: RpcConfig {
                enabled: true,
                host: "0.0.0.0".into(), // было "127.0.0.1"
                port: 8545,
                cors: true,
            },
            mining: None,
            log: LogConfig {
                level: "info".into(),
                format: "pretty".into(),
            },
        }
    }
}

impl NodeConfig {
    /// Путь к директории с данными БД
    pub fn db_path(&self) -> PathBuf {
        self.data_dir.join("db")
    }

    /// Путь к keystore файлу
    pub fn keystore_path(&self) -> PathBuf {
        self.data_dir.join("keystore.json")
    }
}
