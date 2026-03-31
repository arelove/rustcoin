use anyhow::Result;
use clap::Subcommand;
use rc_primitives::types::{Address, Amount};
use rc_wallet::{keystore::Keystore, TransactionBuilder};

#[derive(Subcommand)]
pub enum WalletCmd {
    /// Создать новый аккаунт
    Create {
        /// Имя аккаунта
        #[arg(short, long)]
        name: String,
        /// Пароль (будет запрошен интерактивно если не указан)
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Список всех аккаунтов
    List,
    /// Баланс аккаунта
    Balance {
        #[arg(short, long)]
        address: String,
    },
    /// Отправить транзакцию
    Send {
        #[arg(long)]
        to: String,
        /// Сумма в RSC (например, 1.5)
        #[arg(long)]
        amount: f64,
        /// Комиссия в RSC
        #[arg(long, default_value = "0.0001")]
        fee: f64,
        /// Адрес отправителя (по умолчанию — первый аккаунт)
        #[arg(long)]
        from: Option<String>,
        /// Пароль кошелька
        #[arg(short, long)]
        password: Option<String>,
    },
    /// Экспорт приватного ключа (осторожно!)
    Export {
        #[arg(short, long)]
        address: String,
        #[arg(short, long)]
        password: Option<String>,
    },
}

pub async fn run(cmd: WalletCmd, _config_path: &str) -> Result<()> {
    let keystore_path = std::path::PathBuf::from("keystore.json");
    let mut keystore = if keystore_path.exists() {
        Keystore::load(&keystore_path)?
    } else {
        Keystore::new()
    };

    match cmd {
        WalletCmd::Create { name, password } => {
            let password = get_password(password, "Enter password: ")?;
            let address = keystore.create_account(name.clone(), &password)?;
            keystore.save(&keystore_path)?;

            println!("✓ Account created!");
            println!("  Name:    {name}");
            println!("  Address: {address}");
        }

        WalletCmd::List => {
            let accounts = keystore.list_accounts();
            if accounts.is_empty() {
                println!("No accounts found. Run: quench wallet create --name main");
                return Ok(());
            }
            println!("{:<20} {}", "Name", "Address");
            println!("{}", "-".repeat(70));
            for acc in accounts {
                println!("{:<20} {}", acc.name, acc.address);
            }
        }

        WalletCmd::Balance { address } => {
            // В реальности делаем запрос к RPC: account_getBalance(address)
            println!("Address: {address}");
            println!("Balance: (connect to node with: quench node start)");
        }

        WalletCmd::Send {
            to,
            amount,
            fee,
            from,
            password,
        } => {
            let password = get_password(password, "Enter wallet password: ")?;
            let to_addr = Address::from_base58(&to)
                .map_err(|_| anyhow::anyhow!("Invalid recipient address"))?;

            // Находим аккаунт отправителя
            let account = if let Some(ref from_addr) = from {
                let addr = Address::from_base58(from_addr)?;
                keystore
                    .list_accounts()
                    .into_iter()
                    .find(|a| a.address == addr)
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("Account not found"))?
            } else {
                keystore
                    .default_account()
                    .cloned()
                    .ok_or_else(|| anyhow::anyhow!("No default account. Create one first."))?
            };

            let keypair = keystore.unlock(&account.address, &password)?;

            let amount_rustoshi = (amount * 100_000_000.0) as u64;
            let fee_rustoshi = (fee * 100_000_000.0) as u64;

            let tx = TransactionBuilder::new()
                .from(account.address)
                .to(to_addr)
                .amount(Amount(amount_rustoshi))
                .fee(Amount(fee_rustoshi))
                .nonce(0) // TODO: получить из ноды через RPC
                .sign(&keypair)?;

            let txid = tx.tx_id();

            // TODO: отправить через RPC
            println!("✓ Transaction signed!");
            println!("  TxId:   {txid}");
            println!("  From:   {}", account.address);
            println!("  To:     {to}");
            println!("  Amount: {amount} RSC");
            println!("  Fee:    {fee} RSC");
            println!("\nTo broadcast: connect to a running node.");
        }

        WalletCmd::Export { address, password } => {
            println!("⚠️  WARNING: Never share your private key!");
            let password = get_password(password, "Enter wallet password: ")?;
            let addr = Address::from_base58(&address)?;
            let keypair = keystore.unlock(&addr, &password)?;
            let hex_key = hex::encode(keypair.private.as_bytes());
            println!("Private key (hex): {hex_key}");
        }
    }

    Ok(())
}

fn get_password(password: Option<String>, prompt: &str) -> Result<String> {
    if let Some(p) = password {
        return Ok(p);
    }
    // Интерактивный ввод без echo
    print!("{prompt}");
    use std::io::Write;
    std::io::stdout().flush()?;
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;
    Ok(input.trim().to_string())
}
