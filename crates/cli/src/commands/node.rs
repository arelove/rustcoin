use anyhow::Result;
use clap::Subcommand;
use rc_node::{Node, NodeConfig};

#[derive(Subcommand)]
pub enum NodeCmd {
    /// Запустить полную ноду
    Start {
        /// Включить майнинг
        #[arg(long)]
        mine: bool,
        /// Адрес кошелька для награды майнера
        #[arg(long)]
        coinbase: Option<String>,
        /// P2P порт (по умолчанию 8333)
        #[arg(long, default_value = "8333")]
        p2p_port: u16,
        /// RPC порт (по умолчанию 8545)
        #[arg(long, default_value = "8545")]
        rpc_port: u16,
        /// RPC хост (0.0.0.0 для Docker)
        #[arg(long, default_value = "0.0.0.0")]
        rpc_host: String,
        /// Начальные пиры (можно указать несколько)
        #[arg(long)]
        peer: Vec<String>,
    },
    /// Показать статус ноды
    Status,
}

pub async fn run(cmd: NodeCmd, config_path: &str) -> Result<()> {
    match cmd {
        NodeCmd::Start {
            mine,
            coinbase,
            p2p_port,
            rpc_port,
            rpc_host,
            peer,
        } => {
            let mut config = load_or_default_config(config_path);
            config.p2p.port = p2p_port;
            config.rpc.port = rpc_port;
            config.rpc.host = rpc_host;
            config.p2p.bootstrap_peers = peer;

            if mine {
                let addr = coinbase
                    .ok_or_else(|| anyhow::anyhow!("--coinbase required when --mine is set"))?;
                config.mining = Some(rc_node::config::MiningConfig {
                    coinbase_address: addr,
                    threads: 0,
                });
            }

            tracing::info!("Starting quench node...");
            let node = match Node::new(config).await {
                Ok(n) => n,
                Err(e) => {
                    eprintln!("FATAL: Failed to initialize node: {e}");
                    tracing::error!("Failed to initialize node: {e}");
                    return Err(e.into());
                }
            };
            match node.run().await {
                Ok(_) => {}
                Err(e) => {
                    eprintln!("FATAL: Node crashed: {e}");
                    tracing::error!("Node crashed: {e}");
                    return Err(e.into());
                }
            }
        }
        NodeCmd::Status => {
            println!("Node status: checking RPC at http://127.0.0.1:8545/health");
        }
    }
    Ok(())
}

fn load_or_default_config(path: &str) -> NodeConfig {
    let p = std::path::Path::new(path);
    if p.exists() {
        let s = std::fs::read_to_string(p).expect("read config");
        toml::from_str(&s).expect("parse config")
    } else {
        NodeConfig::default()
    }
}
