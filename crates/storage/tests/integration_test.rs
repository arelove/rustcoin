use rc_primitives::{
    block::Block,
    hash::Hash,
    transaction::{Transaction, TxKind},
    types::{Address, Amount, BlockHeight, Timestamp},
};
use rc_storage::{AccountState, Database};

fn make_test_db() -> Database {
    let dir = tempfile::tempdir().unwrap();
    Database::open(dir.path()).expect("open test db")
}

fn make_tx(nonce: u64) -> Transaction {
    Transaction {
        version: 1,
        kind: TxKind::Transfer,
        from: Some(Address::from_bytes([1u8; 20])),
        to: Address::from_bytes([2u8; 20]),
        amount: Amount(1000),
        fee: Amount(100),
        nonce,
        timestamp: Timestamp(0),
        signature: Some(vec![0u8; 64]),
        public_key: Some(vec![0u8; 32]),
    }
}

// ─── Block Storage ────────────────────────────────────────────────────────────

#[test]
fn test_put_and_get_block() {
    let db = make_test_db();
    let genesis = Block::genesis();
    let hash = genesis.hash();

    db.put_block(&genesis).expect("put block");
    let retrieved = db
        .get_block(&hash)
        .expect("get block")
        .expect("block exists");
    assert_eq!(genesis, retrieved);
}

#[test]
fn test_get_nonexistent_block_returns_none() {
    let db = make_test_db();
    let hash = Hash::sha256(b"nonexistent");
    let result = db.get_block(&hash).expect("no error");
    assert!(result.is_none());
}

#[test]
fn test_get_block_by_height() {
    let db = make_test_db();
    let genesis = Block::genesis();

    db.put_block(&genesis).expect("put block");
    let by_height = db
        .get_block_at(BlockHeight(0))
        .expect("no error")
        .expect("block at height 0");

    assert_eq!(genesis, by_height);
}

#[test]
fn test_get_header_only() {
    let db = make_test_db();
    let genesis = Block::genesis();
    let hash = genesis.hash();

    db.put_block(&genesis).expect("put block");
    let header = db
        .get_header(&hash)
        .expect("get header")
        .expect("header exists");
    assert_eq!(genesis.header, header);
}

// ─── Transaction Storage ──────────────────────────────────────────────────────

#[test]
fn test_put_and_get_tx() {
    let db = make_test_db();
    let tx = make_tx(0);
    let txid = tx.tx_id();

    db.put_tx(&tx).expect("put tx");
    let retrieved = db.get_tx(&txid).expect("get tx").expect("tx exists");
    assert_eq!(tx.tx_id(), retrieved.tx_id());
}

// ─── Account State ────────────────────────────────────────────────────────────

#[test]
fn test_new_account_has_zero_balance() {
    let db = make_test_db();
    let addr = Address::from_bytes([42u8; 20]);
    let acc = db.get_account(&addr).expect("get account");
    assert_eq!(acc.balance, Amount(0));
    assert_eq!(acc.nonce, 0);
}

#[test]
fn test_update_account_balance() {
    let db = make_test_db();
    let addr = Address::from_bytes([7u8; 20]);

    let mut state = AccountState::default();
    state.credit(Amount(1_000_000)).expect("credit");
    db.put_account(&addr, &state).expect("put account");

    let loaded = db.get_account(&addr).expect("load account");
    assert_eq!(loaded.balance, Amount(1_000_000));
}

#[test]
fn test_account_debit_insufficient_returns_err() {
    let mut state = AccountState::default();
    state.balance = Amount(100);
    let result = state.debit(Amount(200));
    assert!(result.is_err(), "should fail on insufficient balance");
}

#[test]
fn test_account_nonce_increment() {
    let mut state = AccountState::default();
    assert_eq!(state.nonce, 0);
    state.increment_nonce();
    assert_eq!(state.nonce, 1);
    state.increment_nonce();
    assert_eq!(state.nonce, 2);
}

// ─── Meta ─────────────────────────────────────────────────────────────────────

#[test]
fn test_meta_roundtrip() {
    let db = make_test_db();
    let val = b"best_tip_hash_here";

    db.put_meta("best_tip", val).expect("put meta");
    let loaded = db.get_meta("best_tip").expect("get meta").expect("exists");
    assert_eq!(loaded.as_slice(), val);
}

#[test]
#[ignore] // запускать только с живой БД: cargo test -- --ignored check_live_balance
fn check_live_balance() {
    let db = Database::open(std::path::Path::new("/app/data/db")).expect("open db");
    let addr_bytes: [u8; 20] = [
        208, 68, 2, 44, 210, 88, 86, 164, 103, 190, 157, 251, 132, 67, 190, 220, 208, 209, 99, 41,
    ];
    let addr = rc_primitives::types::Address::from_bytes(addr_bytes);
    println!("Address: {}", addr);
    let account = db.get_account(&addr).expect("get account");
    println!(
        "Balance: {} rustoshi ({} RSC)",
        account.balance.0,
        account.balance.0 / 100_000_000
    );
    println!("Nonce:   {}", account.nonce);
    assert!(account.balance.0 > 0, "balance should be > 0 after mining");
}
