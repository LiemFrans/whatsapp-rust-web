#!/usr/bin/env bash
# ──────────────────────────────────────────────────────────────────────────────
# run.sh — Start the WhatsApp Web Clone (backend + frontend)
# ──────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
BACKEND_PORT="${PORT:-8080}"
FRONTEND_PORT=3030

# ── Colours ───────────────────────────────────────────────────────────────────
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${CYAN}══════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  WhatsApp Web Clone — Starting Services${NC}"
echo -e "${CYAN}══════════════════════════════════════════════════${NC}"
echo ""

# ── Prerequisites check ──────────────────────────────────────────────────────
echo -e "${YELLOW}Checking prerequisites…${NC}"

if ! command -v cargo &>/dev/null; then
    echo "❌  Rust / Cargo not found. Install from https://rustup.rs"
    exit 1
fi

if ! command -v node &>/dev/null; then
    echo "❌  Node.js not found. Install from https://nodejs.org"
    exit 1
fi

if ! command -v npm &>/dev/null; then
    echo "❌  npm not found. Comes with Node.js."
    exit 1
fi

# On Debian/Ubuntu you may need: sudo apt install libsqlite3-dev pkg-config
echo -e "${GREEN}✓ cargo, node, npm found${NC}"
echo ""

# ── Install frontend dependencies ────────────────────────────────────────────
echo -e "${YELLOW}→ Installing frontend dependencies…${NC}"
cd "$ROOT/frontend"
npm install --no-audit --no-fund
echo -e "${GREEN}✓ Frontend dependencies installed${NC}"
echo ""

# ── Build Rust backend (first build can take 5-10 min) ───────────────────────
echo -e "${YELLOW}→ Building Rust backend (first build may take several minutes)…${NC}"
cd "$ROOT/backend"
cargo build --release 2>&1
echo -e "${GREEN}✓ Backend built${NC}"
echo ""

# ── Start backend ────────────────────────────────────────────────────────────
echo -e "${YELLOW}→ Starting backend on port ${BACKEND_PORT}…${NC}"
cd "$ROOT/backend"
PORT="$BACKEND_PORT" cargo run --release &
BACKEND_PID=$!

# Wait for backend to be ready
echo -n "  Waiting for backend "
for _ in $(seq 1 60); do
    if curl -s "http://localhost:${BACKEND_PORT}/api/auth/status" >/dev/null 2>&1; then
        break
    fi
    echo -n "."
    sleep 1
done
echo ""
echo -e "${GREEN}  ✅ Backend ready${NC}"
echo ""

# ── Start frontend ───────────────────────────────────────────────────────────
echo -e "${YELLOW}→ Starting Next.js frontend on port ${FRONTEND_PORT}…${NC}"
cd "$ROOT/frontend"
npm run dev -- -p "$FRONTEND_PORT" &
FRONTEND_PID=$!

# Wait a moment for Next.js dev server
sleep 3
echo -e "${GREEN}  ✅ Frontend starting${NC}"
echo ""

echo -e "${CYAN}══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  🟢  All services running!${NC}"
echo -e "  Frontend:  ${GREEN}http://localhost:${FRONTEND_PORT}${NC}"
echo -e "  Backend:   ${GREEN}http://localhost:${BACKEND_PORT}${NC}"
echo -e "${CYAN}══════════════════════════════════════════════════${NC}"
echo ""
echo -e "  Press ${YELLOW}Ctrl+C${NC} to stop all services."
echo ""

# ── Cleanup on exit ──────────────────────────────────────────────────────────
cleanup() {
    echo ""
    echo "Stopping services…"
    kill "$BACKEND_PID" 2>/dev/null || true
    kill "$FRONTEND_PID" 2>/dev/null || true
    wait "$BACKEND_PID" 2>/dev/null || true
    wait "$FRONTEND_PID" 2>/dev/null || true
    echo "Done."
}
trap cleanup EXIT INT TERM

wait
