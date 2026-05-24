#!/bin/bash
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"

echo "=== MineFind ==="

# Build frontend if stale
FRONTEND_SRC="$DIR/src"
FRONTEND_DIST="$DIR/dist"
if [ ! -d "$FRONTEND_DIST" ] || [ "$(find "$FRONTEND_SRC" -newer "$FRONTEND_DIST/index.html" 2>/dev/null)" ]; then
    echo "Building frontend..."
    cd "$DIR" && npm run build --silent
fi

# Build backend if missing
BIN="$DIR/src-tauri/target/release/minefind"
if [ ! -f "$BIN" ]; then
    echo "Building backend..."
    cd "$DIR/src-tauri" && cargo build --release 2>&1 | tail -1
fi

export MINEFIND_FRONTEND="$FRONTEND_DIST"
export RUST_LOG=info

echo ""
echo "========================================="
echo "  MineFind — Minecraft Server Discovery"
echo "  http://localhost:8765"
echo "========================================="
echo "  Scanning direct by default (fast)."
echo "  Enable proxy (Tor) in Settings for IP"
echo "  privacy."
echo "========================================="
echo ""

exec "$BIN"
