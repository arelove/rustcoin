use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rc_primitives::types::{Address, Amount, BlockHeight, Timestamp};
use rc_vm::{ExecutionContext, Executor};
use std::collections::HashMap;

/// Minimal valid WASM module (no-op): exports nothing, just validates & instantiates.
/// Generated from: (module)
const NOOP_WASM: &[u8] = &[
    0x00, 0x61, 0x73, 0x6d, // magic: \0asm
    0x01, 0x00, 0x00, 0x00, // version: 1
];

fn make_address(seed: u8) -> Address {
    Address::from([seed; 20])
}

fn make_ctx(method: &str) -> ExecutionContext {
    ExecutionContext {
        contract_address: make_address(1),
        caller: make_address(2),
        origin: make_address(2),
        value: 0,
        block_height: 1,
        block_timestamp: 0,
        gas_limit: 1_000_000,
        method: method.to_string(),
        args: vec![],
    }
}

// ─── Deploy benchmark ────────────────────────────────────────────────────────

fn bench_deploy(c: &mut Criterion) {
    let executor = Executor::new().expect("create executor");

    let mut group = c.benchmark_group("contract_deploy");
    group.bench_function("noop_wasm", |b| {
        b.iter(|| {
            let _ = executor.deploy(NOOP_WASM, make_ctx("init"), HashMap::new());
        });
    });
    group.finish();
}

// ─── Call benchmark ───────────────────────────────────────────────────────────

fn bench_call(c: &mut Criterion) {
    let executor = Executor::new().expect("create executor");

    let mut group = c.benchmark_group("contract_call");
    group.bench_function("noop_wasm", |b| {
        b.iter(|| {
            // Calling a non-existent method returns an ExecutionResult with success=false;
            // that is the expected path for a no-op contract in benchmarks.
            let _ = executor.call(NOOP_WASM, make_ctx("transfer"), HashMap::new());
        });
    });
    group.finish();
}

criterion_group!(benches, bench_deploy, bench_call);
criterion_main!(benches);
