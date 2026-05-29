#!/bin/bash
# MineFind VPS Deploy Script
# Deploys the scanner to a remote Linux VPS and runs it as a systemd service.
# Free forever: Oracle Cloud Always Free tier (4 ARM cores, 24GB RAM)

set -e
REMOTE="${1:-root@YOUR_VPS_IP}"
PROJECT="minefind"

echo "=== MineFind VPS Deploy ==="
echo "Deploying to: $REMOTE"
echo ""

# Check SSH
echo "Checking SSH connection..."
if ! ssh -o ConnectTimeout=5 -o BatchMode=yes "$REMOTE" "echo OK" &>/dev/null; then
    echo "Cannot SSH to $REMOTE"
    echo "Install your SSH key first: ssh-copy-id $REMOTE"
    echo ""
    echo "Or use password auth by editing this script to remove '-o BatchMode=yes'"
    exit 1
fi
echo "SSH OK"

# Install dependencies on VPS
echo ""
echo "Installing dependencies on VPS..."
ssh "$REMOTE" "
    # Install Rust
    if ! command -v rustc &>/dev/null; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
        source \$HOME/.cargo/env
    fi
    # Install Node.js
    if ! command -v node &>/dev/null; then
        curl -fsSL https://deb.nodesource.com/setup_20.x | bash -
        apt-get install -y nodejs
    fi
    # Install git
    apt-get install -y git 2>/dev/null || true
"

# Clone repo on VPS
echo ""
echo "Deploying project..."
ssh "$REMOTE" "
    rm -rf $PROJECT
    git clone https://github.com/MrNova420/$PROJECT.git
    cd $PROJECT
    chmod +x setup.sh run.sh
"

# Run setup
echo ""
echo "Building on VPS (this takes 5-10 minutes)..."
ssh "$REMOTE" "
    cd $PROJECT
    source \$HOME/.cargo/env 2>/dev/null || true
    bash setup.sh
"

# Create systemd service
echo ""
echo "Creating systemd service..."
ssh "$REMOTE" "cat > /etc/systemd/system/minefind.service << 'SERVICE'
[Unit]
Description=MineFind Minecraft Server Scanner
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=/root/minefind
Environment=RUST_LOG=info
Environment=MINEFIND_FRONTEND=/root/minefind/dist
ExecStart=/root/minefind/src-tauri/target/release/minefind
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

[Install]
WantedBy=multi-user.target
SERVICE"

# Enable and start
ssh "$REMOTE" "systemctl daemon-reload && systemctl enable minefind && systemctl restart minefind"

echo ""
echo "========================================="
echo "  Deploy Complete!"
echo "========================================="
echo ""
echo "  Dashboard: http://$(echo $REMOTE | cut -d@ -f2):8765"
echo ""
echo "  Commands:"
echo "    ssh $REMOTE systemctl status minefind  # Check status"
echo "    ssh $REMOTE systemctl restart minefind  # Restart"
echo "    ssh $REMOTE journalctl -u minefind -f   # Live logs"
echo "    ssh $REMOTE systemctl stop minefind     # Stop"
echo ""
echo "  Tunnel dashboard locally:"
echo "    ssh -L 8765:localhost:8765 $REMOTE"
echo "    Then open http://localhost:8765"
echo ""
