#!/usr/bin/env bash
# check.sh — запускает все проверки качества кода
# Используй перед коммитом!

set -euo pipefail

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

ok()   { echo -e "${GREEN}✓${NC} $1"; }
fail() { echo -e "${RED}✗${NC} $1"; exit 1; }
info() { echo -e "${YELLOW}→${NC} $1"; }

echo "══════════════════════════════════════"
echo "  RustCoin Code Quality Check"
echo "══════════════════════════════════════"
echo ""

info "1. Format check (rustfmt)..."
cargo fmt --all -- --check && ok "Formatting OK" || fail "Run: cargo fmt --all"

info "2. Lint (clippy)..."
cargo clippy --workspace --all-targets --all-features -- -D warnings \
    && ok "Clippy OK" || fail "Fix clippy warnings above"

info "3. Tests..."
cargo test --workspace --all-features \
    && ok "All tests passed" || fail "Tests failed"

info "4. Documentation..."
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --quiet \
    && ok "Docs OK" || fail "Fix doc warnings"

info "5. Security audit..."
if command -v cargo-audit &> /dev/null; then
    cargo audit && ok "No vulnerabilities found"
else
    echo "   (skip — install with: cargo install cargo-audit)"
fi

info "6. Dependency check..."
if command -v cargo-deny &> /dev/null; then
    cargo deny check && ok "Dependency check OK"
else
    echo "   (skip — install with: cargo install cargo-deny)"
fi

echo ""
echo "══════════════════════════════════════"
echo -e "${GREEN}All checks passed! Ready to commit.${NC}"
echo "══════════════════════════════════════"
