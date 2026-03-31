use criterion::{criterion_group, criterion_main, Criterion};
use rc_consensus::miner::Miner;
use rc_primitives::{hash::Hash, types::BlockHeight};
use tokio_util::sync::CancellationToken;

fn bench_mine_difficulty_16(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let addr = rc_primitives::types::Address::from_bytes([1u8; 20]);

    c.bench_function("mine block difficulty=16", |b| {
        b.iter(|| {
            rt.block_on(async {
                let cancel = CancellationToken::new();
                let miner = Miner::new(addr, cancel);
                miner
                    .mine_async(Hash::ZERO, BlockHeight(1), vec![], 16, 1)
                    .await
                    .unwrap()
            })
        })
    });
}

criterion_group!(benches, bench_mine_difficulty_16);
criterion_main!(benches);
