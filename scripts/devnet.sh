#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# devnet.sh — запускает локальную тестовую сеть из 3 нод
# Использование: ./scripts/devnet.sh
# ──────────────────────────────────────────────────────────────────────────────

set -euo pipefail

BINARY="./target/release/quench"
LOG_LEVEL="debug"

# Сначала соберём бинарник
echo "🔨 Building quench..."
cargo build --release --bin quench

echo "🧹 Cleaning old devnet data..."
rm -rf /tmp/quench-devnet-{1,2,3}

echo "🪙 Creating wallets..."
# Нода 1 — майнер
mkdir -p /tmp/quench-devnet-1
$BINARY --log-level debug wallet create \
    --name "miner" \
    --password "devnet123" 2>/dev/null || true

MINER_ADDR=$(cat /tmp/quench-devnet-1/keystore.json 2>/dev/null | \
    python3 -c "import json,sys; d=json.load(sys.stdin); print(list(d['accounts'].values())[0]['address'])" 2>/dev/null || \
    echo "1111111111111111111111111111")

echo "💰 Miner address: $MINER_ADDR"

echo ""
echo "🚀 Starting 3 nodes..."

# Нода 1 — майнер, порт 8333/8545
$BINARY \
    --log-level $LOG_LEVEL \
    node start \
    --p2p-port 8333 \
    --rpc-port 8545 \
    --mine \
    --coinbase "$MINER_ADDR" \
    > /tmp/quench-devnet-1/node.log 2>&1 &
PID1=$!
echo "  Node 1 (miner)  PID=$PID1  P2P=8333 RPC=8545"

sleep 1

# Нода 2 — обычная, подключается к ноде 1
$BINARY \
    --log-level $LOG_LEVEL \
    node start \
    --p2p-port 8334 \
    --rpc-port 8546 \
    --peer "/ip4/127.0.0.1/tcp/8333" \
    > /tmp/quench-devnet-2/node.log 2>&1 &
PID2=$!
echo "  Node 2          PID=$PID2  P2P=8334 RPC=8546"

# Нода 3 — обычная, подключается к ноде 1
$BINARY \
    --log-level $LOG_LEVEL \
    node start \
    --p2p-port 8335 \
    --rpc-port 8547 \
    --peer "/ip4/127.0.0.1/tcp/8333" \
    > /tmp/quench-devnet-3/node.log 2>&1 &
PID3=$!
echo "  Node 3          PID=$PID3  P2P=8335 RPC=8547"

echo ""
echo "✅ Devnet running! Logs:"
echo "   tail -f /tmp/quench-devnet-1/node.log"
echo "   tail -f /tmp/quench-devnet-2/node.log"
echo ""
echo "🌐 RPC endpoints:"
echo "   http://127.0.0.1:8545  (miner node)"
echo "   http://127.0.0.1:8546"
echo "   http://127.0.0.1:8547"
echo ""
echo "📊 Check chain info:"
echo "   curl http://127.0.0.1:8545/api/v1/chain | jq"
echo ""
echo "Press Ctrl+C to stop all nodes"

# Ждём Ctrl+C и убиваем все процессы
trap "echo ''; echo 'Stopping devnet...'; kill $PID1 $PID2 $PID3 2>/dev/null; exit 0" INT
wait
