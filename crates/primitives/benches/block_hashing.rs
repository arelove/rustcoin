use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rc_primitives::{
    block::Block,
    hash::Hash,
    transaction::{Transaction, TxKind},
    types::{Address, Amount, Timestamp},
};

fn bench_sha256(c: &mut Criterion) {
    let data = vec![0u8; 1024];
    c.bench_function("Hash::sha256 (1KB)", |b| {
        b.iter(|| Hash::sha256(black_box(&data)))
    });
}

fn bench_sha256d(c: &mut Criterion) {
    let data = vec![0u8; 80]; // типичный размер заголовка блока
    c.bench_function("Hash::sha256d (80B header)", |b| {
        b.iter(|| Hash::sha256d(black_box(&data)))
    });
}

fn bench_merkle_root(c: &mut Criterion) {
    let tx = Transaction {
        version: 1,
        kind: TxKind::Transfer,
        from: Some(Address::from_bytes([1u8; 20])),
        to: Address::from_bytes([2u8; 20]),
        amount: Amount(1000),
        fee: Amount(100),
        nonce: 0,
        timestamp: Timestamp(0),
        signature: Some(vec![0u8; 64]),
        public_key: Some(vec![0u8; 32]),
    };

    let mut group = c.benchmark_group("merkle_root");
    for size in [1, 10, 100, 500, 1000].iter() {
        let txs = vec![tx.clone(); *size];
        group.bench_with_input(BenchmarkId::from_parameter(size), &txs, |b, txs| {
            b.iter(|| Block::compute_merkle_root(black_box(txs)))
        });
    }
    group.finish();
}

fn bench_block_hash(c: &mut Criterion) {
    let block = Block::genesis();
    c.bench_function("Block::hash (genesis)", |b| {
        b.iter(|| black_box(block.hash()))
    });
}

criterion_group!(
    benches,
    bench_sha256,
    bench_sha256d,
    bench_merkle_root,
    bench_block_hash
);
criterion_main!(benches);
