use anyhow::Result;
use clap::Subcommand;

#[derive(Subcommand)]
pub enum TxCmd {
    /// Получить транзакцию по TxId
    Get {
        #[arg(long)]
        txid: String,
    },
    /// Список ожидающих транзакций в mempool
    Pending,
}

pub async fn run(cmd: TxCmd, _config_path: &str) -> Result<()> {
    let rpc = "http://127.0.0.1:8545";

    match cmd {
        TxCmd::Get { txid } => {
            println!("Fetching tx {txid} from {rpc}/api/v1/tx/{txid}");
        }
        TxCmd::Pending => {
            println!("Fetching mempool from {rpc}/api/v1/mempool");
        }
    }
    Ok(())
}
