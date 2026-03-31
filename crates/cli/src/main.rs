//! quench CLI — главная точка входа.
//!
//! ## Использование
//!
//! ```bash
//! # Запустить ноду
//! quench node start
//! quench node start --config ./node.toml
//!
//! # Кошелёк
//! quench wallet create --name "main"
//! quench wallet list
//! quench wallet balance --address <ADDR>
//! quench wallet send --to <ADDR> --amount 1.5 --fee 0.001
//!
//! # Информация о цепочке
//! quench chain info
//! quench chain block --hash <HASH>
//! quench chain block --height 1000
//!
//! # Транзакции
//! quench tx get --txid <TXID>
//! quench tx pending
//! ```

use anyhow::Result;
use clap::{Parser, Subcommand};

mod commands;

/// quench — блокчейн на Rust
#[derive(Parser)]
#[command(
    name    = "quench",
    version = env!("CARGO_PKG_VERSION"),
    about   = "quench blockchain node and wallet",
    long_about = None,
)]
struct Cli {
    /// Путь к конфигурационному файлу
    #[arg(short, long, global = true, default_value = "node.toml")]
    config: String,

    /// Уровень логирования (trace/debug/info/warn/error)
    #[arg(short, long, global = true, env = "RUST_LOG", default_value = "info")]
    log_level: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Управление нодой
    Node {
        #[command(subcommand)]
        cmd: commands::node::NodeCmd,
    },
    /// Управление кошельком
    Wallet {
        #[command(subcommand)]
        cmd: commands::wallet::WalletCmd,
    },
    /// Информация о цепочке
    Chain {
        #[command(subcommand)]
        cmd: commands::chain::ChainCmd,
    },
    /// Работа с транзакциями
    Tx {
        #[command(subcommand)]
        cmd: commands::tx::TxCmd,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Инициализируем логирование
    init_tracing(&cli.log_level);

    match cli.command {
        Commands::Node { cmd } => commands::node::run(cmd, &cli.config).await,
        Commands::Wallet { cmd } => commands::wallet::run(cmd, &cli.config).await,
        Commands::Chain { cmd } => commands::chain::run(cmd, &cli.config).await,
        Commands::Tx { cmd } => commands::tx::run(cmd, &cli.config).await,
    }
}

fn init_tracing(level: &str) {
    use tracing_subscriber::{fmt, prelude::*, EnvFilter};

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(level));

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt::layer().with_target(true))
        .init();
}
