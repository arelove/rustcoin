use criterion::{black_box, criterion_group, criterion_main, Criterion};
use rc_crypto::keypair::Keypair;

fn bench_keygen(c: &mut Criterion) {
    c.bench_function("Keypair::generate (Ed25519)", |b| {
        b.iter(Keypair::generate)
    });
}

fn bench_sign(c: &mut Criterion) {
    let kp  = Keypair::generate();
    let msg = b"benchmark signing message -- 64 bytes of typical tx data!!!!!";
    c.bench_function("sign message (Ed25519)", |b| {
        b.iter(|| kp.sign(black_box(msg)))
    });
}

fn bench_verify(c: &mut Criterion) {
    let kp  = Keypair::generate();
    let msg = b"benchmark verify message -- typical tx data length here!!!!!";
    let sig = kp.sign(msg);
    c.bench_function("verify signature (Ed25519)", |b| {
        b.iter(|| kp.public.verify(black_box(msg), black_box(&sig)))
    });
}

fn bench_address_derivation(c: &mut Criterion) {
    let kp = Keypair::generate();
    c.bench_function("PublicKey → Address (SHA256+RIPEMD160)", |b| {
        b.iter(|| kp.public.to_address())
    });
}

criterion_group!(benches, bench_keygen, bench_sign, bench_verify, bench_address_derivation);
criterion_main!(benches);
