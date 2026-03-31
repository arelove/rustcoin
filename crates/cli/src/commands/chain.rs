use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum ChainCmd {
    /// Информация о текущем состоянии цепочки
    Info,
    /// Получить блок по хэшу или высоте
    Block {
        #[arg(long, conflicts_with = "height")]
        hash: Option<String>,
        #[arg(long, conflicts_with = "hash")]
        height: Option<u64>,
    },
}

pub async fn run(cmd: ChainCmd, _config_path: &str) -> Result<()> {
    let rpc = "http://127.0.0.1:8545";

    match cmd {
        ChainCmd::Info => {
            println!("Fetching chain info from {rpc}/api/v1/chain ...");
            // TODO: HTTP GET request
            println!("  Network:    quench-mainnet");
            println!("  Best height: (connect a node first)");
        }
        ChainCmd::Block { hash, height } => {
            if let Some(h) = hash {
                println!("Fetching block {h} from {rpc}/api/v1/blocks/{h}");
            } else if let Some(n) = height {
                println!("Fetching block at height {n} from {rpc}/api/v1/blocks/height/{n}");
            } else {
                println!("Provide --hash or --height");
            }
        }
    }
    Ok(())
}
