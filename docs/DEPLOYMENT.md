# MineFind Deployment Guide

## Local Machine

```bash
git clone https://github.com/MrNova420/minefind.git
cd minefind
chmod +x setup.sh run.sh
./setup.sh    # Install deps, build, copy DBs (~5 min)
./run.sh      # Launch → http://localhost:8765
```

## VPS Deployment

### One-Command Deploy
```bash
./deploy.sh root@YOUR_VPS_IP
```

This script:
1. SSH into the VPS
2. Installs Rust + Node.js if missing
3. Clones the repo from GitHub
4. Runs setup.sh (builds everything)
5. Creates systemd service `minefind.service`
6. Enables auto-start on boot + auto-restart on crash
7. Starts the scanner

Access at `http://VPS_IP:8765` or tunnel:
```bash
ssh -L 8765:localhost:8765 root@YOUR_VPS_IP
# Open http://localhost:8765
```

### Free Forever VPS Options

**Oracle Cloud Always Free** (best option):
- 4 ARM Ampere A1 cores
- 24 GB RAM
- 4 Gbps network
- 200 GB storage
- IPv6 included
- No credit card for ARM instances in some regions
- Sign up: https://cloud.oracle.com

**Google Cloud Free Tier**:
- 1 e2-micro instance
- 1 GB RAM
- 30 GB storage
- Limited but sufficient for low concurrency

**AWS Free Tier** (12 months only):
- 1 t2.micro instance
- 1 GB RAM
- 30 GB EBS

### Managing the VPS Scanner

```bash
# Check status
ssh root@vps systemctl status minefind

# View live logs
ssh root@vps journalctl -u minefind -f

# Restart scanner
ssh root@vps systemctl restart minefind

# Stop scanner
ssh root@vps systemctl stop minefind

# Start scanner
ssh root@vps systemctl start minefind

# Pull DB back to local
scp root@vps:~/.local/share/minefind/servers.db ~/.local/share/minefind/
```

### Syncing Databases

The scanner runs on the VPS. To keep your local machine in sync:

1. **VPS → GitHub** (Dashboard "Push to GitHub")
2. **GitHub → Local** (pull + copy DBs)
3. **Local → VPS** (push DBs, VPS picks up new servers)

Or use `scp` directly for one-time transfers.

### Firewall

If you can't access port 8765:
```bash
# On the VPS
sudo ufw allow 8765/tcp

# Or for Oracle Cloud: add Ingress rule in Security List
# Source: 0.0.0.0/0, Port: 8765, Protocol: TCP
```

### Performance Notes

- Oracle Cloud ARM: Run at 10k concurrency
- Google Cloud e2-micro: Run at 500 concurrency
- Local desktop: 4k-6k concurrency
- Adjust in Settings to match your machine's capability
