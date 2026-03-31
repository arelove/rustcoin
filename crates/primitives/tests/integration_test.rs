//! Интеграционные тесты для rc-primitives.
//!
//! Тесты проверяют взаимодействие типов — хэширование, Merkle-дерево,
//! сериализацию/десериализацию, Proof-of-Work.

use rc_primitives::{
    block::Block,
    hash::Hash,
    transaction::{Transaction, TxKind},
    types::{Address, Amount, Timestamp},
};

// ─── Hash Tests ───────────────────────────────────────────────────────────────

#[test]
fn test_hash_sha256_deterministic() {
    let h1 = Hash::sha256(b"hello world");
    let h2 = Hash::sha256(b"hello world");
    assert_eq!(h1, h2, "same input must produce same hash");
}

#[test]
fn test_hash_different_inputs_differ() {
    let h1 = Hash::sha256(b"foo");
    let h2 = Hash::sha256(b"bar");
    assert_ne!(h1, h2);
}

#[test]
fn test_hash_hex_roundtrip() {
    let original = Hash::sha256(b"test");
    let hex = original.to_hex();
    let decoded = Hash::from_hex(&hex).expect("valid hex");
    assert_eq!(original, decoded);
}

#[test]
fn test_hash_meets_difficulty() {
    // Hash::ZERO начинается с 256 нулевых бит
    assert!(Hash::ZERO.meets_difficulty(0), "difficulty 0 always passes");
    assert!(Hash::ZERO.meets_difficulty(32), "ZERO meets any difficulty");
    assert!(
        Hash::ZERO.meets_difficulty(64),
        "ZERO meets high difficulty"
    );
}

// ─── Address Tests ────────────────────────────────────────────────────────────

#[test]
fn test_address_base58_roundtrip() {
    let bytes = [1u8; 20];
    let addr = Address::from_bytes(bytes);
    let encoded = addr.to_base58();
    let decoded = Address::from_base58(&encoded).expect("valid base58");
    assert_eq!(addr, decoded);
}

#[test]
fn test_address_zero_is_distinct() {
    let zero = Address::ZERO;
    let other = Address::from_bytes([1u8; 20]);
    assert_ne!(zero, other);
}

#[test]
fn test_address_invalid_base58_fails() {
    let result = Address::from_base58("not_a_valid_address!!!");
    assert!(result.is_err());
}

// ─── Amount Tests ─────────────────────────────────────────────────────────────

#[test]
fn test_amount_checked_add_overflow() {
    let max = Amount(u64::MAX);
    let one = Amount(1);
    let result = max.checked_add(one);
    assert!(result.is_none(), "overflow must return None");
}

#[test]
fn test_amount_checked_sub_underflow() {
    let small = Amount(5);
    let large = Amount(10);
    let result = small.checked_sub(large);
    assert!(result.is_none(), "underflow must return None");
}

#[test]
fn test_amount_one_rsc() {
    assert_eq!(Amount::ONE.0, 100_000_000);
}

// ─── Transaction Tests ────────────────────────────────────────────────────────

fn make_tx(amount: u64) -> Transaction {
    Transaction {
        version: 1,
        kind: TxKind::Transfer,
        from: Some(Address::from_bytes([1u8; 20])),
        to: Address::from_bytes([2u8; 20]),
        amount: Amount(amount),
        fee: Amount(1000),
        nonce: 0,
        timestamp: Timestamp(0),
        signature: Some(vec![0u8; 64]),
        public_key: Some(vec![0u8; 32]),
    }
}

#[test]
fn test_tx_id_deterministic() {
    let tx = make_tx(1000);
    let id1 = tx.tx_id();
    let id2 = tx.tx_id();
    assert_eq!(id1, id2);
}

#[test]
fn test_tx_id_differs_with_different_amount() {
    let tx1 = make_tx(1000);
    let tx2 = make_tx(2000);
    assert_ne!(tx1.tx_id(), tx2.tx_id());
}

#[test]
fn test_coinbase_validate_basic_ok() {
    let coinbase = Transaction {
        version: 1,
        kind: TxKind::Coinbase,
        from: None,
        to: Address::from_bytes([9u8; 20]),
        amount: Amount(50 * 100_000_000),
        fee: Amount(0),
        nonce: 0,
        timestamp: Timestamp(0),
        signature: None,
        public_key: None,
    };
    assert!(coinbase.validate_basic().is_ok());
}

#[test]
fn test_zero_amount_transfer_rejected() {
    let mut tx = make_tx(0);
    tx.amount = Amount(0);
    assert!(tx.validate_basic().is_err());
}

// ─── Block Tests ──────────────────────────────────────────────────────────────

#[test]
fn test_merkle_root_empty() {
    let root = Block::compute_merkle_root(&[]);
    assert_eq!(root, Hash::ZERO);
}

#[test]
fn test_merkle_root_single_tx() {
    let txs = vec![make_tx(1000)];
    let root = Block::compute_merkle_root(&txs);
    assert_eq!(root, txs[0].tx_id(), "single tx: root = tx hash");
}

#[test]
fn test_merkle_root_changes_with_tx() {
    let txs1 = vec![make_tx(1000)];
    let txs2 = vec![make_tx(2000)];
    assert_ne!(
        Block::compute_merkle_root(&txs1),
        Block::compute_merkle_root(&txs2)
    );
}

#[test]
fn test_merkle_root_odd_number_of_txs() {
    // При нечётном числе последний дублируется
    let txs = vec![make_tx(1), make_tx(2), make_tx(3)];
    let root = Block::compute_merkle_root(&txs);
    // Просто проверяем что не паникует и не равен ZERO
    assert_ne!(root, Hash::ZERO);
}

#[test]
fn test_genesis_block() {
    let genesis = Block::genesis();
    assert_eq!(genesis.height().0, 0);
    assert_eq!(genesis.previous_hash(), Hash::ZERO);
}

#[test]
fn test_block_serialization_roundtrip() {
    let block = Block::genesis();
    let json = serde_json::to_string(&block).expect("serialize");
    let back: Block = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(block, back);
}

// ─── Property-based Tests (proptest) ─────────────────────────────────────────

#[cfg(test)]
mod proptest_tests {
    use proptest::prelude::*;
    use rc_primitives::{hash::Hash, types::Amount};

    proptest! {
        #[test]
        fn hash_sha256_never_panics(data: Vec<u8>) {
            let _ = Hash::sha256(&data);
        }

        #[test]
        fn amount_add_commutative(a in 0u64..1_000_000, b in 0u64..1_000_000) {
            let sum1 = Amount(a).checked_add(Amount(b));
            let sum2 = Amount(b).checked_add(Amount(a));
            prop_assert_eq!(sum1, sum2);
        }

        #[test]
        fn hash_hex_roundtrip(data: Vec<u8>) {
            let hash  = Hash::sha256(&data);
            let hex   = hash.to_hex();
            let back  = Hash::from_hex(&hex).unwrap();
            prop_assert_eq!(hash, back);
        }
    }
}
