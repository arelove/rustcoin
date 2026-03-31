//! Применение блоков к стейту — «state transition function».
//!
//! Это сердце блокчейна: именно здесь блок превращается в изменения балансов.
//!
//! ## Порядок операций для каждого блока
//!
//! 1. Проверить, что блок не применялся ранее (идемпотентность)
//! 2. Для каждой транзакции в блоке:
//!    - `Coinbase` → зачислить `amount` на адрес `to` (майнеру)
//!    - `Transfer`  → списать `amount + fee` с `from`, зачислить `amount` на `to`
//!    - `ContractCall` / `ContractDeploy` → пропустить (будущая работа)
//! 3. Все изменения применяются атомарно через `WriteBatch`
//! 4. Сохранить транзакции в CF `txs` для последующего поиска
//! 5. Обновить метаданные `best_tip`

use crate::{account::AccountState, db::Database, error::StorageError};
use rc_primitives::{
    block::Block,
    hash::Hash,
    transaction::TxKind,
    types::{Address, BlockHeight},
};
use tracing::{debug, warn};

/// Метаданные лучшего tip-а — сохраняются в CF `meta`
const META_BEST_TIP_HASH: &str = "best_tip_hash";
const META_BEST_TIP_HEIGHT: &str = "best_tip_height";

/// Результат применения блока
#[derive(Debug)]
pub struct ApplyResult {
    /// Количество успешно обработанных транзакций
    pub txs_applied: usize,
    /// Количество пропущенных транзакций (контракты, ошибки)
    pub txs_skipped: usize,
}

impl Database {
    /// Применить блок к состоянию цепочки.
    ///
    /// Атомарно обновляет балансы аккаунтов, сохраняет транзакции
    /// и обновляет `best_tip` метаданные.
    ///
    /// Возвращает `Ok(ApplyResult)` если блок успешно применён,
    /// или `Err` если блок уже существует / произошла ошибка БД.
    pub fn apply_block(&self, block: &Block) -> Result<ApplyResult, StorageError> {
        let hash = block.hash();
        let height = block.height();

        // ── 1. Идемпотентность: уже применяли этот блок? ────────────────────
        if self.get_block(&hash)?.is_some() {
            return Ok(ApplyResult {
                txs_applied: 0,
                txs_skipped: 0,
            });
        }

        // ── 2. Собираем изменения аккаунтов в памяти ─────────────────────────
        // HashMap: Address → AccountState (загружаем лениво, один раз)
        let mut accounts: std::collections::HashMap<Address, AccountState> =
            std::collections::HashMap::new();

        let mut txs_applied = 0usize;
        let mut txs_skipped = 0usize;

        for tx in &block.transactions {
            match &tx.kind {
                // ── Coinbase: зачислить награду майнеру ──────────────────────
                TxKind::Coinbase => {
                    let miner = accounts
                        .entry(tx.to)
                        .or_insert_with(|| self.get_account(&tx.to).unwrap_or_default());

                    if let Err(e) = miner.credit(tx.amount) {
                        warn!(txid = %tx.tx_id(), err = %e, "Coinbase credit failed");
                        txs_skipped += 1;
                        continue;
                    }
                    txs_applied += 1;
                }

                // ── Transfer: списать с отправителя, зачислить получателю ────
                TxKind::Transfer => {
                    let from_addr = match tx.from {
                        Some(a) => a,
                        None => {
                            warn!(txid = %tx.tx_id(), "Transfer missing sender, skipping");
                            txs_skipped += 1;
                            continue;
                        }
                    };

                    // Загружаем отправителя (если ещё не в кеше)
                    if !accounts.contains_key(&from_addr) {
                        let state = self.get_account(&from_addr)?;
                        accounts.insert(from_addr, state);
                    }

                    // total = amount + fee
                    let total = tx
                        .amount
                        .checked_add(tx.fee)
                        .ok_or(StorageError::Overflow)?;

                    // Проверяем nonce (защита от replay)
                    let sender = accounts.get_mut(&from_addr).unwrap();
                    if tx.nonce != sender.nonce {
                        warn!(
                            txid    = %tx.tx_id(),
                            expected = sender.nonce,
                            got      = tx.nonce,
                            "Nonce mismatch, skipping"
                        );
                        txs_skipped += 1;
                        continue;
                    }

                    // Списываем с отправителя
                    if let Err(e) = sender.debit(total) {
                        warn!(txid = %tx.tx_id(), err = %e, "Insufficient balance, skipping");
                        txs_skipped += 1;
                        continue;
                    }
                    sender.increment_nonce();

                    // Зачисляем получателю
                    if !accounts.contains_key(&tx.to) {
                        let state = self.get_account(&tx.to)?;
                        accounts.insert(tx.to, state);
                    }
                    let recipient = accounts.get_mut(&tx.to).unwrap();
                    if let Err(e) = recipient.credit(tx.amount) {
                        warn!(txid = %tx.tx_id(), err = %e, "Credit failed, skipping");
                        txs_skipped += 1;
                        continue;
                    }

                    txs_applied += 1;
                }

                // ── ContractCall / ContractDeploy: VM не реализована ─────────
                TxKind::ContractCall { .. } | TxKind::ContractDeploy { .. } => {
                    debug!(txid = %tx.tx_id(), "Contract tx skipped (VM not wired)");
                    txs_skipped += 1;
                }
            }
        }

        // ── 3. Атомарный WriteBatch ───────────────────────────────────────────
        // Записываем всё за один fsync: блок + заголовок + высота + txs + accounts + meta
        self.apply_block_batch(block, &hash, height, &accounts, &block.transactions)?;

        Ok(ApplyResult {
            txs_applied,
            txs_skipped,
        })
    }

    /// Внутренний метод: формирует и записывает WriteBatch.
    ///
    /// Вынесен отдельно, чтобы `apply_block` оставался читаемым.
    fn apply_block_batch(
        &self,
        block: &Block,
        hash: &Hash,
        height: BlockHeight,
        accounts: &std::collections::HashMap<Address, AccountState>,
        txs: &[rc_primitives::transaction::Transaction],
    ) -> Result<(), StorageError> {
        use crate::keys::Keys;

        let mut batch = rocksdb::WriteBatch::default();

        // ── Блок ─────────────────────────────────────────────────────────────
        {
            let cf = self.cf_handle("blocks")?;
            batch.put_cf(cf, Keys::block(hash), self.serialize(block)?);
        }
        {
            let cf = self.cf_handle("headers")?;
            batch.put_cf(cf, Keys::hash(hash), self.serialize(&block.header)?);
        }
        {
            let cf = self.cf_handle("heights")?;
            batch.put_cf(cf, Keys::height(height), hash.as_bytes());
        }

        // ── Транзакции ───────────────────────────────────────────────────────
        {
            let cf = self.cf_handle("txs")?;
            for tx in txs {
                let txid = tx.tx_id();
                batch.put_cf(cf, Keys::hash(&txid), self.serialize(tx)?);
            }
        }

        // ── Аккаунты ─────────────────────────────────────────────────────────
        {
            let cf = self.cf_handle("state")?;
            for (addr, state) in accounts {
                batch.put_cf(cf, addr.as_bytes(), self.serialize(state)?);
            }
        }

        // ── Best tip metadata ────────────────────────────────────────────────
        {
            let cf = self.cf_handle("meta")?;
            batch.put_cf(cf, META_BEST_TIP_HASH.as_bytes(), hash.as_bytes());
            batch.put_cf(cf, META_BEST_TIP_HEIGHT.as_bytes(), &height.0.to_be_bytes());
        }

        self.write_batch(batch)
    }

    /// Получить текущий best tip из метаданных БД.
    ///
    /// Используется при старте ноды для восстановления состояния после рестарта.
    pub fn get_best_tip(&self) -> Result<Option<(Hash, BlockHeight)>, StorageError> {
        let hash_bytes = match self.get_meta(META_BEST_TIP_HASH)? {
            Some(b) => b,
            None => return Ok(None),
        };
        let height_bytes = match self.get_meta(META_BEST_TIP_HEIGHT)? {
            Some(b) => b,
            None => return Ok(None),
        };

        if hash_bytes.len() != 32 || height_bytes.len() != 8 {
            return Err(StorageError::Corruption("invalid best_tip metadata".into()));
        }

        let mut arr = [0u8; 32];
        arr.copy_from_slice(&hash_bytes);
        let hash = Hash::from_bytes(arr);

        let mut h = [0u8; 8];
        h.copy_from_slice(&height_bytes);
        let height = BlockHeight(u64::from_be_bytes(h));

        Ok(Some((hash, height)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rc_primitives::{
        block::{Block, BlockHeader},
        transaction::{Transaction, TxKind},
        types::{Address, Amount, BlockHeight, Nonce, Timestamp},
    };

    fn make_db() -> Database {
        Database::open_temp().expect("open temp db")
    }

    fn addr(seed: u8) -> Address {
        Address::from_bytes([seed; 20])
    }

    /// Строим блок с coinbase вручную (genesis пустой — без транзакций)
    fn make_block_with_coinbase(
        prev_hash: rc_primitives::hash::Hash,
        height: BlockHeight,
        miner: Address,
        reward: Amount,
        extra_txs: Vec<Transaction>,
    ) -> Block {
        let coinbase = Transaction {
            version: 1,
            kind: TxKind::Coinbase,
            from: None,
            to: miner,
            amount: reward,
            fee: Amount(0),
            nonce: height.0,
            timestamp: Timestamp(0),
            signature: None,
            public_key: None,
        };
        let mut txs = vec![coinbase];
        txs.extend(extra_txs);

        let merkle = Block::compute_merkle_root(&txs);
        let mut header = BlockHeader {
            version: 1,
            previous_hash: prev_hash,
            merkle_root: merkle,
            timestamp: Timestamp(height.0),
            bits: u32::MAX, // difficulty = 0: любой хэш подходит
            nonce: Nonce(0),
            height,
            hash: None,
        };
        let h = header.compute_hash();
        header.hash = Some(h);
        Block {
            header,
            transactions: txs,
        }
    }

    // Genesis (пустой блок) применяется без ошибок, best_tip обновляется
    #[test]
    fn test_apply_genesis_ok() {
        let db = make_db();
        let genesis = Block::genesis();
        let result = db.apply_block(&genesis).expect("apply genesis");

        // genesis пустой — нет транзакций, нечего применять
        assert_eq!(result.txs_applied, 0);
        assert_eq!(result.txs_skipped, 0);

        let (hash, height) = db.get_best_tip().unwrap().unwrap();
        assert_eq!(hash, genesis.hash());
        assert_eq!(height.0, 0);
    }

    // Идемпотентность: повторное применение не меняет стейт
    #[test]
    fn test_apply_block_idempotent() {
        let db = make_db();
        let genesis = Block::genesis();
        db.apply_block(&genesis).expect("first apply");

        let result2 = db
            .apply_block(&genesis)
            .expect("second apply — should be no-op");
        assert_eq!(result2.txs_applied, 0);
        assert_eq!(result2.txs_skipped, 0);

        // best_tip не изменился
        let (hash, height) = db.get_best_tip().unwrap().unwrap();
        assert_eq!(hash, genesis.hash());
        assert_eq!(height.0, 0);
    }

    // Coinbase: мнер получает награду
    #[test]
    fn test_coinbase_credits_miner() {
        let db = make_db();
        let miner = addr(1);
        let reward = Amount(50 * 100_000_000);

        // Применяем genesis чтобы best_tip не был пустым
        db.apply_block(&Block::genesis()).unwrap();

        let block = make_block_with_coinbase(
            Block::genesis().hash(),
            BlockHeight(1),
            miner,
            reward,
            vec![],
        );
        db.apply_block(&block).expect("apply block 1");

        let account = db.get_account(&miner).unwrap();
        assert_eq!(account.balance, reward, "miner must receive block reward");
    }

    // best_tip обновляется после каждого блока
    #[test]
    fn test_best_tip_persisted() {
        let db = make_db();
        let miner = addr(2);

        db.apply_block(&Block::genesis()).unwrap();
        let block1 = make_block_with_coinbase(
            Block::genesis().hash(),
            BlockHeight(1),
            miner,
            Amount(5_000_000_000),
            vec![],
        );
        db.apply_block(&block1).unwrap();

        let (hash, height) = db.get_best_tip().unwrap().unwrap();
        assert_eq!(hash, block1.hash());
        assert_eq!(height.0, 1);
    }

    // Transfer: баланс списывается с отправителя и зачисляется получателю
    #[test]
    fn test_transfer_updates_balances() {
        let db = make_db();
        let from_addr = addr(10);
        let to_addr = addr(11);
        let miner = addr(99);

        // Пополняем баланс отправителя напрямую
        let mut sender_state = AccountState::default();
        sender_state.credit(Amount(1_000_000)).unwrap();
        db.put_account(&from_addr, &sender_state).unwrap();

        db.apply_block(&Block::genesis()).unwrap();

        let transfer = Transaction {
            version: 1,
            kind: TxKind::Transfer,
            from: Some(from_addr),
            to: to_addr,
            amount: Amount(400_000),
            fee: Amount(100),
            nonce: 0,
            timestamp: Timestamp(0),
            signature: Some(vec![0u8; 64]),
            public_key: Some(vec![0u8; 32]),
        };

        let block = make_block_with_coinbase(
            Block::genesis().hash(),
            BlockHeight(1),
            miner,
            Amount(5_000_000_000),
            vec![transfer],
        );
        db.apply_block(&block).expect("apply block with transfer");

        let sender = db.get_account(&from_addr).unwrap();
        let recipient = db.get_account(&to_addr).unwrap();

        assert_eq!(sender.balance, Amount(1_000_000 - 400_000 - 100));
        assert_eq!(sender.nonce, 1, "nonce must increment after transfer");
        assert_eq!(recipient.balance, Amount(400_000));
    }

    // Недостаточный баланс: транзакция пропускается, стейт не меняется
    #[test]
    fn test_transfer_insufficient_balance_skipped() {
        let db = make_db();
        let from_addr = addr(20);
        let to_addr = addr(21);
        let miner = addr(99);

        // У отправителя 0 монет
        db.apply_block(&Block::genesis()).unwrap();

        let transfer = Transaction {
            version: 1,
            kind: TxKind::Transfer,
            from: Some(from_addr),
            to: to_addr,
            amount: Amount(999_999),
            fee: Amount(100),
            nonce: 0,
            timestamp: Timestamp(0),
            signature: Some(vec![0u8; 64]),
            public_key: Some(vec![0u8; 32]),
        };

        let block = make_block_with_coinbase(
            Block::genesis().hash(),
            BlockHeight(1),
            miner,
            Amount(5_000_000_000),
            vec![transfer],
        );
        let result = db.apply_block(&block).unwrap();

        assert_eq!(result.txs_skipped, 1, "transfer must be skipped");
        assert_eq!(db.get_account(&to_addr).unwrap().balance, Amount(0));
    }

    // Неверный nonce: транзакция пропускается
    #[test]
    fn test_transfer_wrong_nonce_skipped() {
        let db = make_db();
        let from_addr = addr(30);
        let to_addr = addr(31);
        let miner = addr(99);

        let mut sender_state = AccountState::default();
        sender_state.credit(Amount(1_000_000)).unwrap();
        db.put_account(&from_addr, &sender_state).unwrap();

        db.apply_block(&Block::genesis()).unwrap();

        let transfer = Transaction {
            version: 1,
            kind: TxKind::Transfer,
            from: Some(from_addr),
            to: to_addr,
            amount: Amount(100_000),
            fee: Amount(100),
            nonce: 5, // неверный nonce (ожидается 0)
            timestamp: Timestamp(0),
            signature: Some(vec![0u8; 64]),
            public_key: Some(vec![0u8; 32]),
        };

        let block = make_block_with_coinbase(
            Block::genesis().hash(),
            BlockHeight(1),
            miner,
            Amount(5_000_000_000),
            vec![transfer],
        );
        let result = db.apply_block(&block).unwrap();

        assert_eq!(result.txs_skipped, 1, "wrong nonce must be skipped");
        assert_eq!(
            db.get_account(&from_addr).unwrap().balance,
            Amount(1_000_000),
            "balance unchanged"
        );
    }
}
