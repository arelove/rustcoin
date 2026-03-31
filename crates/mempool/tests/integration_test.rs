use rc_mempool::Mempool;
use rc_primitives::{
    transaction::{Transaction, TxKind},
    types::{Address, Amount, Timestamp},
};

fn make_tx(nonce: u64, fee: u64) -> Transaction {
    Transaction {
        version: 1,
        kind: TxKind::Transfer,
        from: Some(Address::from_bytes([1u8; 20])),
        to: Address::from_bytes([2u8; 20]),
        amount: Amount(1000),
        fee: Amount(fee),
        nonce,
        timestamp: Timestamp(0),
        signature: Some(vec![0u8; 64]),
        public_key: Some(vec![0u8; 32]),
    }
}

#[test]
fn test_add_and_get_tx() {
    let pool = Mempool::new();
    let tx = make_tx(0, 500);
    let txid = pool.add(tx.clone()).expect("add tx");
    let got = pool.get(&txid).expect("get tx");
    assert_eq!(got.nonce, tx.nonce);
}

#[test]
fn test_duplicate_rejected() {
    let pool = Mempool::new();
    let tx = make_tx(0, 500);
    pool.add(tx.clone()).expect("first add ok");
    let err = pool.add(tx);
    assert!(matches!(
        err,
        Err(rc_mempool::MempoolError::DuplicateTransaction(_))
    ));
}

#[test]
fn test_select_for_block_ordered_by_fee() {
    let pool = Mempool::new();

    // Добавляем транзакции с разными комиссиями
    pool.add(make_tx(0, 100)).unwrap();
    pool.add(make_tx(1, 500)).unwrap();
    pool.add(make_tx(2, 300)).unwrap();

    let selected = pool.select_for_block(10);
    assert_eq!(selected.len(), 3);

    // Первая транзакция должна иметь самую высокую комиссию
    assert_eq!(selected[0].fee, Amount(500));
    assert_eq!(selected[1].fee, Amount(300));
    assert_eq!(selected[2].fee, Amount(100));
}

#[test]
fn test_select_respects_max_count() {
    let pool = Mempool::new();
    for i in 0..10u64 {
        pool.add(make_tx(i, i * 100 + 1)).unwrap();
    }
    let selected = pool.select_for_block(3);
    assert_eq!(selected.len(), 3, "should respect max_count");
}

#[test]
fn test_remove_confirmed() {
    let pool = Mempool::new();
    let tx1 = make_tx(0, 100);
    let tx2 = make_tx(1, 200);
    let id1 = pool.add(tx1).unwrap();
    let id2 = pool.add(tx2).unwrap();

    pool.remove_confirmed(&[id1]);
    assert_eq!(pool.len(), 1);
    assert!(pool.get(&id1).is_none());
    assert!(pool.get(&id2).is_some());
}

#[test]
fn test_eviction_on_full_pool() {
    let pool = Mempool::with_capacity(3);
    let low_fee = 10u64;
    let hi_fee = 1000u64;

    pool.add(make_tx(0, low_fee)).unwrap();
    pool.add(make_tx(1, hi_fee)).unwrap();
    pool.add(make_tx(2, hi_fee)).unwrap();

    // Пул полон — добавляем ещё одну, должна вытеснить самую дешёвую
    pool.add(make_tx(3, hi_fee)).unwrap();

    assert_eq!(pool.len(), 3, "pool size should stay at capacity");
    // Проверяем что транзакция с низкой комиссией была вытеснена
    let remaining = pool.select_for_block(10);
    assert!(
        remaining.iter().all(|tx| tx.fee.0 >= hi_fee),
        "low-fee tx should have been evicted"
    );
}

#[test]
fn test_total_fees() {
    let pool = Mempool::new();
    pool.add(make_tx(0, 100)).unwrap();
    pool.add(make_tx(1, 200)).unwrap();
    pool.add(make_tx(2, 300)).unwrap();

    let total = pool.total_fees();
    assert_eq!(total, Amount(600));
}

#[test]
fn test_zero_amount_tx_rejected() {
    let pool = Mempool::new();
    let mut tx = make_tx(0, 100);
    tx.amount = Amount(0);
    assert!(pool.add(tx).is_err());
}
