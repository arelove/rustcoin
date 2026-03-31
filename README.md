# RustCoin (RSC)

A blockchain node implemented from scratch in Rust. Built as a portfolio project to demonstrate systems-level Rust and Web3 engineering — full stack from cryptographic primitives to a running P2P network.

[![CI](https://github.com/arelove/rustcoin/actions/workflows/ci.yml/badge.svg)](https://github.com/arelove/rustcoin/actions)
[![Rust](https://img.shields.io/badge/rust-1.82%2B-orange.svg)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

---

## What's implemented

This is not a tutorial clone. Every component listed below is working code you can run:

**Consensus**
- Proof-of-Work mining with SHA-256d, cancellable via `CancellationToken` when a peer broadcasts a new block
- Bitcoin-style difficulty adjustment every 2016 blocks with a ±4x cap per period
- Fork choice by total accumulated work, not chain length — avoids low-difficulty long-chain attacks
- Halving schedule: block reward starts at 50 RSC and halves every 210,000 blocks

**Cryptography**
- Ed25519 key pairs via `ed25519-dalek`; addresses derived with SHA-256 → RIPEMD-160 (Bitcoin-style)
- `PrivateKey` implements `Drop` with `zeroize` — key material is zeroed on deallocation
- `Debug` is manually implemented to print `PrivateKey(***)`, preventing accidental log leaks
- secp256k1 (`k256`) present for Ethereum compatibility

**Storage**
- RocksDB backend with typed column families: blocks, transactions, accounts, metadata
- Account state machine: `credit`, `debit` (checked arithmetic), `increment_nonce`
- Genesis block written on first startup, idempotent on subsequent restarts

**Mempool**
- Concurrent `DashMap`-backed pool, thread-safe without a global lock
- Transactions selected for blocks sorted by fee-per-byte
- Eviction of lowest-fee transaction when the pool hits `MAX_MEMPOOL_TXS = 5000`
- Duplicate and basic validity checks on insertion

**P2P Network**
- `libp2p 0.53` stack: TCP + Noise encryption + Yamux multiplexing
- Gossipsub for block and transaction propagation across four topics (`blocks`, `txs`, `headers`, `control`)
- Kademlia DHT for peer routing; mDNS for local-network discovery (tested in Docker devnet)
- The log excerpt in this repo shows a real 4-node network forming in under 200ms

**Smart Contract VM**
- `wasmtime`-based executor with fuel metering (`consume_fuel`) — contracts cannot loop forever
- Host function ABI: `storage_get`, `storage_set`, `get_caller`, `emit_event`
- ERC-20-compatible token contract written in `no_std` Rust, compiled to WASM
- Gas cost table: storage write = 5000, storage read = 500, transfer = 2000, event = 300

**RPC / API**
- `axum 0.7` server with REST endpoints and a JSON-RPC 2.0 dispatcher on `POST /`
- Endpoints: `/health`, `/api/v1/chain`, `/api/v1/blocks/:hash`, `/api/v1/blocks/height/:h`, `/api/v1/tx/:txid`, `/api/v1/account/:addr`, `/api/v1/mempool`
- CORS middleware; structured `tracing` spans per request

**Wallet**
- `TransactionBuilder` with a fluent API, signs with Ed25519 and embeds the public key for verification
- `Keystore`: encrypted JSON keystore saved to disk, password-protected key derivation

**Tooling**
- Multi-stage Dockerfile: `builder` compiles, `runtime` is a minimal Debian image (~30MB), `dev` includes the full toolchain
- `docker-compose.yml` for a 3-node local devnet — nodes discover each other via mDNS automatically
- CI pipeline: `rustfmt`, `clippy -D warnings`, tests on Ubuntu + macOS, `cargo-audit`, `cargo-deny`, rustdoc
- Release workflow: cross-compiled binaries for `x86_64` and `aarch64` (Linux + macOS) on every tag
- `cargo deny` enforces license allowlist and bans `openssl`

---

## Architecture

The codebase is a Cargo workspace with 10 crates. Dependency flow is strict — lower layers have no knowledge of higher layers.

```
rc-cli (binary)
    └── rc-node          (orchestration, main event loop)
            ├── rc-consensus   (PoW mining, difficulty, fork choice)
            ├── rc-p2p         (libp2p network)
            ├── rc-rpc         (axum HTTP/JSON-RPC server)
            ├── rc-storage     (RocksDB)
            ├── rc-mempool     (transaction pool)
            └── rc-vm          (wasmtime executor)
                    └── rc-crypto      (Ed25519, address derivation)
                            └── rc-primitives  (Block, Transaction, Hash, Address)
```

`rc-primitives` has zero external runtime dependencies — it compiles in under a second and is the shared vocabulary for the entire system.

---

## Getting started

### Prerequisites

```bash
# Rust 1.82+
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# RocksDB (Ubuntu/Debian)
sudo apt-get install librocksdb-dev clang

# RocksDB (macOS)
brew install rocksdb
```

### Build

```bash
git clone https://github.com/arelove/rustcoin
cd rustcoin
cargo build --release
```

### Run a single node

```bash
# Create a wallet
./target/release/rustcoin wallet create --name main

# Start a mining node
./target/release/rustcoin node start \
  --p2p-port 8333 \
  --rpc-port 8545 \
  --mine \
  --coinbase <YOUR_ADDRESS>
```

### Local devnet (3 nodes)

The quickest way to see the P2P layer in action:

```bash
# Option A: shell script
chmod +x scripts/devnet.sh
./scripts/devnet.sh

# Option B: Docker Compose (nodes discover each other via mDNS)
docker compose up node1 node2 node3
```

Nodes connect and begin exchanging blocks within seconds. You can watch it:

```bash
curl http://127.0.0.1:8545/api/v1/chain | jq
curl http://127.0.0.1:8545/api/v1/account/<ADDRESS> | jq
```

### CLI reference

```bash
rustcoin wallet create --name main
rustcoin wallet list
rustcoin wallet balance --address <ADDRESS>
rustcoin wallet send --to <ADDRESS> --amount 1.5 --fee 0.0001

rustcoin chain info
rustcoin chain block --height 100
rustcoin chain block --hash <HASH>

rustcoin tx get --txid <TXID>
rustcoin tx pending
```

### RPC reference

```bash
# Health
curl http://127.0.0.1:8545/health

# Chain state
curl http://127.0.0.1:8545/api/v1/chain | jq

# Block by height
curl http://127.0.0.1:8545/api/v1/blocks/height/1 | jq

# Account balance
curl http://127.0.0.1:8545/api/v1/account/<BASE58_ADDRESS> | jq

# Submit transaction (JSON-RPC 2.0)
curl -X POST http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"tx_send","params":["<hex>"],"id":1}'
```

---

## Testing

```bash
# All tests
cargo test --workspace

# With stdout
cargo test --workspace -- --nocapture

# Benchmarks (Criterion, generates HTML report)
cargo bench --workspace
open target/criterion/report/index.html
```

Test coverage across the workspace:

| Crate | Tests |
|-------|-------|
| `rc-crypto` | Key generation uniqueness, deterministic signing, tampered-message rejection, roundtrip serialization |
| `rc-storage` | Block apply, account credit/debit, nonce increment, best-tip persistence |
| `rc-mempool` | Add, duplicate rejection, fee ordering, eviction |
| `rc-consensus` | Mining at various difficulties, difficulty adjustment algorithm |
| `rc-wallet` | Account create/unlock, wrong-password handling, `TransactionBuilder` signing and verification |
| `rc-node` | Genesis idempotency, best-tip recovery after restart |

Property-based tests (`proptest`) and benchmarks (`criterion`) are set up in `rc-crypto`, `rc-consensus`, `rc-storage`, and `rc-vm`.

---

## Smart contracts

Contracts are `no_std` Rust compiled to WASM. The token contract implements the ERC-20 interface.

```bash
rustup target add wasm32-unknown-unknown
cargo install wasm-opt

cd contracts/token
cargo build --target wasm32-unknown-unknown --release
wasm-opt -Oz \
  target/wasm32-unknown-unknown/release/rustcoin_token.wasm \
  -o token_opt.wasm
```

The VM loads the bytecode, resolves host imports, and calls the exported function. Storage writes are buffered in `ExecutionState` and committed atomically only on success — no partial state on revert.

---

## Security notes

- `#![forbid(unsafe_code)]` in all library crates
- `cargo-audit` runs in CI on every push
- `cargo-deny` enforces license policy and bans `openssl`
- Private keys use `zeroize` on drop; `Debug` impl is manually overridden to prevent logging
- Transactions include a nonce checked against account state — replay attacks across the same chain are rejected
- The keystore encryption in the current implementation uses XOR-with-SHA256 as a placeholder; production use would require AES-256-GCM + Argon2

---

## Codebase stats

| Metric | Value |
|--------|-------|
| Crates | 10 (+ 1 WASM contract) |
| Language | Rust 1.82 |
| Async runtime | tokio |
| Lines of code | ~5,000 (excl. generated/vendor) |
| CI targets | Ubuntu, macOS (x86\_64 + aarch64) |

---

## Roadmap

- [x] Core primitives — Block, Transaction, Hash, Address
- [x] Ed25519 cryptography with zeroize
- [x] Proof of Work — mining, difficulty adjustment, fork choice
- [x] RocksDB storage with typed column families
- [x] Mempool with fee-per-byte prioritization
- [x] libp2p P2P — Gossipsub, Kademlia, mDNS
- [x] wasmtime VM with gas metering
- [x] REST + JSON-RPC 2.0 API (axum)
- [x] Encrypted keystore and TransactionBuilder
- [x] Docker + docker-compose devnet
- [x] CI (fmt, clippy, tests, audit, deny, docs) on Ubuntu + macOS
- [ ] AES-256-GCM + Argon2 keystore encryption
- [ ] Light client (header-only sync)
- [ ] Block explorer UI
- [ ] State snapshots / fast sync
- [ ] Fuzz testing (`cargo-fuzz`)

---

## License

- [MIT License](LICENSE-MIT)
- [Apache License 2.0](LICENSE-APACHE)