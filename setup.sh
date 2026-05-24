#!/bin/bash
set -e
DIR="$(cd "$(dirname "$0")" && pwd)"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

echo -e "${CYAN}"
echo "  ⬡  MineFind — Setup & Install"
echo "  ================================="
echo -e "${NC}"
echo ""

# ─── Check prerequisites ───
check_cmd() {
    if ! command -v "$1" &>/dev/null; then
        echo -e "${RED}[FAIL]${NC} $1 not found"
        return 1
    else
        echo -e "${GREEN}[OK]${NC}   $1 ($($1 --version 2>&1 | head -1 | cut -c1-50))"
        return 0
    fi
}

echo "Checking prerequisites..."
echo ""

FAIL=0
check_cmd rustc || {
    echo -e "${YELLOW}Install Rust: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh${NC}"
    FAIL=1
}
check_cmd cargo || FAIL=1
check_cmd node || {
    echo -e "${YELLOW}Install Node.js: https://nodejs.org or nvm install 18${NC}"
    FAIL=1
}
check_cmd npm || FAIL=1

if [ "$FAIL" -gt 0 ]; then
    echo ""
    echo -e "${RED}Missing prerequisites. Install them above and re-run.${NC}"
    exit 1
fi

echo ""

# ─── Create data directory ───
DATA_DIR="$HOME/.local/share/minefind"
mkdir -p "$DATA_DIR"
echo -e "${GREEN}[OK]${NC}   Data directory: $DATA_DIR"

# ─── Install npm dependencies ───
echo ""
echo "Installing frontend dependencies..."
cd "$DIR"
if [ ! -d "node_modules" ]; then
    npm install --silent
    echo -e "${GREEN}[OK]${NC}   npm packages installed"
else
    echo -e "${GREEN}[OK]${NC}   Already installed, updating..."
    npm install --silent 2>/dev/null
fi

# ─── Build frontend ───
echo ""
echo "Building frontend (Svelte 5)..."
npm run build --silent
FRONTEND_SIZE=$(du -sh "$DIR/dist" 2>/dev/null | cut -f1)
echo -e "${GREEN}[OK]${NC}   Frontend built ($FRONTEND_SIZE)"

# ─── Build backend ───
echo ""
echo "Building backend (Rust)..."
cd "$DIR/src-tauri"
cargo build --release 2>&1 | tail -5
BIN="$DIR/src-tauri/target/release/minefind"
if [ -f "$BIN" ]; then
    BIN_SIZE=$(du -sh "$BIN" | cut -f1)
    echo -e "${GREEN}[OK]${NC}   Backend built ($BIN_SIZE)"
else
    echo -e "${RED}[FAIL]${NC} Backend binary not found"
    exit 1
fi

# ─── Setup git ───
echo ""
echo "Checking git configuration..."
if ! git config user.name &>/dev/null 2>&1; then
    echo -e "${YELLOW}[!]${NC}   Git user.name not set"
    echo "    Run: git config --global user.name \"YOUR_NAME\""
fi
if ! git config user.email &>/dev/null 2>&1; then
    echo -e "${YELLOW}[!]${NC}   Git user.email not set"
    echo "    Run: git config --global user.email \"YOUR_EMAIL\""
fi
if git config user.name &>/dev/null 2>&1 && git config user.email &>/dev/null 2>&1; then
    echo -e "${GREEN}[OK]${NC}   Git identity: $(git config user.name) <$(git config user.email)>"
fi

# ─── GitHub auth hint ───
echo ""
echo "GitHub authentication (for DB push feature):"
echo "  1. Create token: https://github.com/settings/tokens (scope: repo)"
echo "  2. Run: git config --global credential.helper store"
echo "  3. First push will cache your token"
echo ""

# ─── Done ───
echo -e "${GREEN}========================================="
echo -e "  Setup complete!"
echo -e "=========================================${NC}"
echo ""
echo "  Launch: ${CYAN}./run.sh${NC}"
echo "  Open:   ${CYAN}http://localhost:8765${NC}"
echo ""
echo "  Files:"
echo "    Frontend → $DIR/dist/"
echo "    Backend  → $BIN"
echo "    Data     → $DATA_DIR/"
echo ""
