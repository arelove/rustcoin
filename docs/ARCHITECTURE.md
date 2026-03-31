# Architecture

## Crate Dependency Graph

```
                        ┌─────────────────┐
                        │   rc-cli (bin)  │  ← точка входа
                        └────────┬────────┘
                                 │
                        ┌────────▼────────┐
                        │    rc-node      │  ← оркестрация
                        └────────┬────────┘
              ┌──────────────────┼──────────────────────┐
              │                  │                      │
   ┌──────────▼──────┐  ┌───────▼───────┐  ┌──────────▼──────┐
   │   rc-consensus  │  │    rc-p2p     │  │    rc-rpc       │
   └──────────┬──────┘  └───────┬───────┘  └──────────┬──────┘
              │                  │                      │
              └──────────────────┼──────────────────────┘
                                 │
              ┌──────────────────┼──────────────────────┐
              │                  │                      │
   ┌──────────▼──────┐  ┌───────▼───────┐  ┌──────────▼──────┐
   │   rc-storage    │  │   rc-mempool  │  │    rc-vm        │
   └──────────┬──────┘  └───────┬───────┘  └──────────┬──────┘
              │                  │                      │
              └──────────────────┼──────────────────────┘
                                 │
              ┌──────────────────┼──────────────────────┐
              │                                         │
   ┌──────────▼──────┐                       ┌──────────▼──────┐
   │   rc-crypto     │                       │  rc-primitives  │
   └──────────┬──────┘                       └─────────────────┘
              │
   ┌──────────▼──────┐
   │  rc-primitives  │
   └─────────────────┘
```

## Компоненты

| Крейт | Назначение | Ключевые типы |
|-------|-----------|---------------|
| `rc-primitives` | Базовые типы, без зависимостей | `Block`, `Transaction`, `Hash`, `Address` |
| `rc-crypto` | Ed25519 подписи, генерация ключей | `Keypair`, `Signature`, `PublicKey` |
| `rc-consensus` | PoW майнинг, difficulty adjustment, fork choice | `Miner`, `DifficultyAdjuster`, `ForkChoice` |
| `rc-storage` | RocksDB хранилище | `Database`, `AccountState` |
| `rc-mempool` | Пул транзакций с fee-приоритизацией | `Mempool` |
| `rc-p2p` | libp2p сеть (Gossipsub, Kademlia, mDNS) | `Network`, `NetworkMessage`, `NetworkEvent` |
| `rc-vm` | WebAssembly смарт-контракт VM | `Executor`, `ExecutionContext` |
| `rc-rpc` | JSON-RPC + REST API сервер | `RpcServer` |
| `rc-wallet` | Keystore, подписание транзакций | `Keystore`, `TransactionBuilder` |
| `rc-node` | Оркестрация всех компонентов | `Node`, `NodeConfig` |
| `rc-cli` | CLI (clap) — точка входа | `rustcoin` binary |

## Поток транзакции

```
Пользователь (CLI/кошелёк)
        │
        │  1. TransactionBuilder.sign(keypair)
        ▼
   Подписанная Transaction
        │
        │  2. POST /  (JSON-RPC tx_send) | P2P NetworkMessage::NewTransaction
        ▼
   RPC Server / P2P Layer
        │
        │  3. Mempool.add(tx)  — валидация + fee-приоритизация
        ▼
   Mempool
        │
        │  4. Miner.select_for_block(500)
        ▼
   Miner (PoW mining loop)
        │
        │  5. mine_async() → Block с Coinbase + транзакциями
        ▼
   Новый блок
        │
        ├──▶ 6a. Database.put_block()
        ├──▶ 6b. Mempool.remove_confirmed()
        ├──▶ 6c. ChainState.update_tip()
        └──▶ 6d. P2P broadcast NetworkMessage::NewBlock
```

## Формат блока

```
Block {
  header: BlockHeader {
    version:       u32        (4B)
    previous_hash: Hash       (32B)
    merkle_root:   Hash       (32B)   ← SHA256d дерево TxId'ов
    timestamp:     Timestamp  (8B)
    bits:          u32        (4B)    ← сложность (компактный формат)
    nonce:         Nonce      (8B)
    height:        BlockHeight(8B)
  }                           ──────
                              96B
  transactions: Vec<Transaction>
    [0] Coinbase              ← награда майнеру + все комиссии
    [1..] Transfer/Contract
}
```

## P2P протокол

Узлы общаются через Gossipsub топики:

| Топик | Сообщения |
|-------|----------|
| `rustcoin/blocks/1` | `NewBlock`, `GetBlock`, `GetBlocks`, `Blocks` |
| `rustcoin/txs/1` | `NewTransaction` |
| `rustcoin/headers/1` | `Headers` (для light clients) |
| `rustcoin/control/1` | `Status`, `Inventory`, `Ping`, `Pong` |

## Смарт-контракты

```
Rust source (.rs)
       │
       │ cargo build --target wasm32-unknown-unknown
       ▼
WebAssembly (.wasm)
       │
       │ Transaction::ContractDeploy { bytecode }
       ▼
Blockchain storage (key: contract_address)
       │
       │ Transaction::ContractCall { contract, method, args }
       ▼
rc-vm Executor
  ├── wasmtime Engine (compile + instantiate)
  ├── Host functions (storage, transfer, events)
  ├── Gas metering (fuel-based)
  └── Isolated Store per call
```
