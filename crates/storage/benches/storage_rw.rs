use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use rc_primitives::types::Address;
use rc_storage::{AccountState, Database};

fn make_db() -> Database {
    Database::open_temp().expect("open temp db")
}

fn make_address(i: u64) -> Address {
    let mut bytes = [0u8; 20];
    bytes[..8].copy_from_slice(&i.to_le_bytes());
    Address::from(bytes)
}

// ─── Account write ────────────────────────────────────────────────────────────

fn bench_account_write(c: &mut Criterion) {
    let db = make_db();
    let state = AccountState {
        balance: 1_000_000,
        nonce: 0,
        code_hash: None,
        storage_root: None,
    };

    let mut group = c.benchmark_group("account_write");
    for &n in &[1u64, 10, 100] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                for i in 0..n {
                    db.put_account(&make_address(i), &state).unwrap();
                }
            });
        });
    }
    group.finish();
}

// ─── Account read ─────────────────────────────────────────────────────────────

fn bench_account_read(c: &mut Criterion) {
    let db = make_db();
    let state = AccountState {
        balance: 42,
        nonce: 1,
        code_hash: None,
        storage_root: None,
    };
    for i in 0..100u64 {
        db.put_account(&make_address(i), &state).unwrap();
    }

    let mut group = c.benchmark_group("account_read");
    for &n in &[1u64, 10, 100] {
        group.throughput(Throughput::Elements(n));
        group.bench_with_input(BenchmarkId::from_parameter(n), &n, |b, &n| {
            b.iter(|| {
                for i in 0..n {
                    let _ = db.get_account(&make_address(i));
                }
            });
        });
    }
    group.finish();
}

// ─── Meta (raw bytes) ─────────────────────────────────────────────────────────

fn bench_meta_rw(c: &mut Criterion) {
    let db = make_db();
    let value = vec![0xABu8; 64];

    let mut group = c.benchmark_group("meta_rw");
    group.bench_function("put", |b| {
        b.iter(|| db.put_meta("bench_key", &value).unwrap());
    });
    group.bench_function("get", |b| {
        db.put_meta("bench_key", &value).unwrap();
        b.iter(|| db.get_meta("bench_key").unwrap());
    });
    group.finish();
}

criterion_group!(
    benches,
    bench_account_write,
    bench_account_read,
    bench_meta_rw
);
criterion_main!(benches);
