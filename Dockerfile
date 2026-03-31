# ╔══════════════════════════════════════════════════════════╗
# ║  RustCoin Dockerfile — multi-stage build                ║
# ║                                                         ║
# ║  Stage 1 (builder): компилирует всё                     ║
# ║  Stage 2 (runtime): минимальный образ только с бинарём  ║
# ║  Stage 3 (dev):     полная среда разработки             ║
# ╚══════════════════════════════════════════════════════════╝

# ─── Stage 1: Builder ────────────────────────────────────────────────────────
FROM rust:1.85-bookworm AS builder

# Зависимости для RocksDB
RUN apt-get update && apt-get install -y \
    librocksdb-dev \
    clang \
    llvm \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Сначала копируем только Cargo.toml файлы — это позволяет Docker
# закэшировать слой зависимостей и не перекачивать их при каждом изменении кода
COPY Cargo.toml ./
COPY crates/primitives/Cargo.toml  crates/primitives/
COPY crates/crypto/Cargo.toml      crates/crypto/
COPY crates/consensus/Cargo.toml   crates/consensus/
COPY crates/storage/Cargo.toml     crates/storage/
COPY crates/mempool/Cargo.toml     crates/mempool/
COPY crates/p2p/Cargo.toml         crates/p2p/
COPY crates/vm/Cargo.toml          crates/vm/
COPY crates/rpc/Cargo.toml         crates/rpc/
COPY crates/wallet/Cargo.toml      crates/wallet/
COPY crates/node/Cargo.toml        crates/node/
COPY crates/cli/Cargo.toml         crates/cli/

# Создаём пустые src/lib.rs для всех крейтов чтобы cargo мог скачать зависимости
RUN for crate in primitives crypto consensus storage mempool p2p vm rpc wallet node; do \
    mkdir -p crates/$crate/src && \
    echo "pub fn placeholder() {}" > crates/$crate/src/lib.rs; \
    done && \
    mkdir -p crates/cli/src && \
    echo "fn main() {}" > crates/cli/src/main.rs

# Скачиваем и компилируем все зависимости (кэшируется если Cargo.toml не менялся)
RUN cargo build --release 2>&1 | tail -5

# Теперь копируем реальный код
COPY crates/ crates/
COPY rustfmt.toml clippy.toml deny.toml ./

# Сбрасываем timestamp чтобы Cargo пересобрал только наши крейты
RUN find crates -name "*.rs" | xargs touch

# Финальная сборка
RUN cargo build --release --bin quench

# ─── Stage 2: Runtime (минимальный образ для запуска) ────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y \
    librocksdb7.8 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Создаём пользователя (не запускаем от root)
RUN useradd -ms /bin/bash quench

WORKDIR /home/quench

# Копируем только бинарник из builder
COPY --from=builder /app/target/release/quench /usr/local/bin/quench

# Директория для данных ноды
RUN mkdir -p /data && chown quench:quench /data

USER quench

VOLUME ["/data"]

EXPOSE 8333 8545

ENTRYPOINT ["quench"]
CMD ["node", "start", "--p2p-port", "8333", "--rpc-port", "8545"]

# ─── Stage 3: Dev (полная среда для разработки) ──────────────────────────────
FROM rust:1.85-bookworm AS dev

RUN apt-get update && apt-get install -y \
    librocksdb-dev \
    clang \
    llvm \
    pkg-config \
    curl \
    git \
    jq \
    vim \
    && rm -rf /var/lib/apt/lists/*

# Rust инструменты — версии совместимые с Rust 1.82
RUN rustup component add rustfmt clippy && \
    rustup target add wasm32-unknown-unknown && \
    cargo install --locked cargo-watch@8.5.2 && \
    cargo install --locked cargo-audit@0.21.1 && \
    cargo install --locked cargo-deny@0.16.2

WORKDIR /app

# Порты: P2P и RPC
EXPOSE 8333 8545

CMD ["bash"]
