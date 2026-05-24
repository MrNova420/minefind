# MineFind

**High-performance Minecraft server discovery engine.** Scans the entire public IPv4 address space across multiple cycles to find and catalog every Minecraft server worldwide.

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.85+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/svelte-5-red?logo=svelte" alt="Svelte 5">
  <img src="https://img.shields.io/badge/sqlite-3-blue?logo=sqlite" alt="SQLite">
  <img src="https://img.shields.io/badge/tokio-async-purple" alt="Tokio">
</p>

---

## Features

### Core Scanner
- **Full IPv4 world scan** — 2.78 billion public IPs across 220+ /8 ranges
- **Multi-cycle rotation** — 4 cycle types auto-advancing:
  - Cycle 1: `IPv4 Fast` — port 25565 only, fastest results
  - Cycle 2: `IPv6 Targeted` — known hosting v6 prefixes
  - Cycle 3: `IPv4 Deep` — 27 ports (25560-25575 + 19130-19140)
  - Cycle 4: `IPv6 Deep` — 27 ports on v6 targets
- **Smart density tracking** — `/8` ranges sorted by server yield, empty ranges skipped after 3 cycles
- **Priority range ordering** — Hosting providers (OVH, Hetzner, AWS, DO, Vultr, Linode, Contabo) scanned first for immediate results
- **Resume-on-crash** — Checkpoint saved every 100k IPs; auto-resumes from saved position on restart
- **Concurrent scanning** — Configurable 100–10,000 parallel connections via semaphore pool
- **2-second timeout** — Optimized TCP connect timeout for fast full-world scanning

### Detection
- **Minecraft Java protocol ping** — Handshake + status request with proper VarInt-length framed packets
- **Whitelist probe** — Login attempt detection to classify servers as whitelisted, open, or unknown
- **Server categorization** — Auto-tags: Vanilla Survival, Modded, Plugin Heavy, Creative, Minigame, Anarchy, Private Group, Idle
- **Mod detection** — Forge/FML/Fabric/Quilt detection via modinfo and version strings
- **MOTD parsing** — Strips Minecraft formatting codes (§), extracts text + extras from JSON chat components

### KittyScan Blocklist Verification
- **Separate database** — `kitty.db` completely isolated from the main `servers.db`
- **Sync from GitHub** — Pulls IP list from `LillySchramm/KittyScanBlocklist` (IPs caught scanning for Minecraft servers)
- **Verify all** — Pings every IP on ports 25565 + 19132, records server info (MOTD, version, players, ping)
- **Live progress** — Real-time verification progress bar with found-servers counter
- **Re-verify** — Always re-pings all IPs on each click for fresh results

### Database & Export
- **SQLite + WAL mode** — Concurrent read/write with no lock contention
- **Batch DB writes** — Single writer task consuming an mpsc channel, no Mutex contention on upserts
- **Fallback lifetime counter** — File-based counter survives DB failures across restarts
- **Push to GitHub** — One-click dashboard button clones `minefind-database` repo, copies both DBs + exports `servers.json`, commits, and pushes
- **Cycle history** — Every scan cycle recorded with timestamps, targets scanned, and servers found

### Web Dashboard
- **Real-time progress** — IPS scanned counter, cycle type, current range, ETA, found servers
- **Server browser** — Filterable table with whitelist status, MOTD, version, player counts, last-seen time
- **Stats & charts** — Category distribution bar chart, version breakdown, density heatmap
- **Settings panel** — Concurrency slider (500–10000), Whitelist probe toggle, Force Proxy toggle, Proxy address
- **Map view** — Geographic distribution of discovered servers
- **Dark theme** — Clean dark UI with green accent colors

### Proxy Support
- **SOCKS5 proxy** — Route scan traffic through Tor or any SOCKS5 proxy
- **Off by default** — Direct connections by default for maximum speed
- **Auto-detect** — Scans common SOCKS5 ports (9050, 9150, 1080, 1088)

---

## Architecture

```
minefind/
├── src-tauri/                 # Rust backend
│   ├── Cargo.toml             # Rust dependencies
│   ├── Cargo.lock             # Locked dependency versions
│   └── src/
│       ├── main.rs            # HTTP server, scan orchestrator, multi-cycle loop
│       ├── db.rs              # SQLite schema, checkpoint, density, cycle tracking
│       ├── kitty.rs           # KittyScan blocklist sync/verify, separate kitty.db
│       ├── proxy.rs           # SOCKS5 connector
│       └── scanner/
│           ├── mod.rs         # ServerInfo, ServerCategory, ScanTarget structs
│           ├── ping.rs        # Minecraft protocol ping (handshake + status)
│           ├── probe.rs       # Whitelist detection (login attempt probe)
│           └── ranges.rs      # IPv4 /8 ranges, priority ranges, IPv6 prefixes
│
├── src/                       # Svelte 5 frontend
│   ├── main.js                # Entry point
│   ├── app.css                # Global styles (dark theme)
│   ├── App.svelte             # Main app shell: nav, settings, scan controls
│   └── lib/
│       ├── Dashboard.svelte   # Stats, progress, server table, DB push button
│       ├── ServerList.svelte  # Filterable server browser
│       ├── Kitty.svelte       # KittyScan blocklist tab
│       ├── Map.svelte         # Geographic map view
│       └── Filters.svelte     # Category/version filter controls
│
├── index.html                 # SPA entry
├── package.json               # Node dependencies
├── vite.config.js             # Vite build config
├── svelte.config.js           # Svelte compiler config
├── run.sh                     # Build + launch script
└── README.md                  # This file
```

**Data directory** (`~/.local/share/minefind/`):
- `servers.db` — Main scanner database (servers, scan_history, scanner_checkpoint, range_density)
- `kitty.db` — KittyScan blocklist database (completely independent)
- `lifetime_counter.txt` — Fallback lifetime scanned counter
- `db-repo-cache/` — Cloned copy of `minefind-database` repo for push-to-GitHub

---

## Quick Start

### Prerequisites
- **Rust** 1.85+ — `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
- **Node.js** 18+ — `https://nodejs.org` or `nvm install 18`
- **npm** 9+ (comes with Node.js)

### One-Command Launch

```bash
git clone https://github.com/MrNova420/minefind.git
cd minefind
chmod +x run.sh
./run.sh
```

The script automatically:
1. Installs npm dependencies (`npm install` — first run only)
2. Builds the Svelte frontend (`npm run build` — if stale)
3. Compiles the Rust backend (`cargo build --release` — first run only)
4. Launches the server on `http://localhost:8765`

Open `http://localhost:8765` in your browser.

### Manual Build

```bash
# Frontend
cd minefind
npm install
npm run build

# Backend
cd src-tauri
cargo build --release

# Run (from minefind/ directory)
MINEFIND_FRONTEND=dist RUST_LOG=info ./src-tauri/target/release/minefind
```

---

## Usage

### Scanning

1. Open `http://localhost:8765`
2. Click the **Settings** gear (⚙) to configure:
   - **Concurrency** — 500–10,000 (default: 4000). Higher = faster scan, more bandwidth.
   - **WL Probe** — Detect whitelist status per server (default: on)
   - **Proxy** — SOCKS5 address + Force Proxy toggle (default: off, direct connections)
3. Click **Scan** to start the multi-cycle world scan
4. Watch the progress bar fill with live stats: scanned IPs, found servers, current range
5. Click **Stop** to cancel — the checkpoint saves your position

### Multi-Cycle Flow

The scanner automatically rotates through 4 cycle types:

| Cycle | Type | Ports | Scope | Est. Duration (10k concurrency) |
|-------|------|-------|-------|------|
| 1 | IPv4 Fast | 25565 | 2.78B public IPv4 | ~3 days |
| 2 | IPv6 Targeted | 25565 | 16 hosting v6 prefixes | ~hours |
| 3 | IPv4 Deep | 27 ports (25560-25575 + 19130-19140) | 2.78B IPv4 | ~2 weeks |
| 4 | IPv6 Deep | 27 ports | 16 v6 prefixes | ~days |
| → | Repeat from Cycle 1 | | | |

**Density tracking**: After 3 full cycles, `/8` ranges with 0 servers are skipped. Re-scanned every 10th cycle to catch new servers.

### KittyScan Blocklist

1. Navigate to the **Kitty** tab
2. Click **Sync from GitHub** — downloads the latest IP list from `LillySchramm/KittyScanBlocklist`
3. Click **Verify All** — pings every IP on ports 25565 + 19132, recording server info
4. Watch the live progress bar — "Verifying X/1217 · Y servers found"
5. Verified servers appear with ✅, unverified with ❌
6. Click Verify again anytime for fresh results

### Database Export

Click the **Push to GitHub** button on the Dashboard to export the entire database:
- Clones/pulls your `MrNova420/minefind-database` repo
- Copies both `servers.db` and `kitty.db`
- Exports `servers.json` (complete server list + cycle stats + kitty IPs)
- Commits with timestamp and pushes
- Requires git authentication (see below)

### Git Authentication

For the Push to GitHub feature:

```bash
# Store credentials for future pushes
git config --global credential.helper store

# Create a Personal Access Token at https://github.com/settings/tokens
# (check 'repo' scope)

# Then either:
# - Do one manual git push to cache the token
# - Or run git push and enter the token as password
```

The main code repo push works the same way. Both use the same git credential store.

---

## API Reference

All endpoints at `http://localhost:8765/api/`.

### Scan

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/scan/start?concurrency=N&probe_whitelist=0/1` | Start multi-cycle scan |
| `POST` | `/scan/cancel` | Stop scan (saves checkpoint) |
| `GET` | `/scan/status` | Live progress: cycle, scanned, found, range, lifetime |
| `GET` | `/scan/cycles` | Cycle history summary |
| `POST` | `/scan/concurrency?n=N` | Change concurrency during scan |

### Servers

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/servers` | All discovered servers (sorted by players DESC) |
| `GET` | `/servers/count` | Total server count |
| `GET` | `/stats` | Aggregated stats (categories, versions, whitelist breakdown) |

### KittyScan

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/kitty/sync` | Fetch blocklist from GitHub, sync DB |
| `POST` | `/kitty/verify` | Ping all IPs for Minecraft servers |
| `GET` | `/kitty/list` | All blocklist IPs with verification results |
| `GET` | `/kitty/stats` | Total/verified/online counts |
| `GET` | `/kitty/status` | Sync/verify progress |

### Database

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/db/push` | Push databases to GitHub repo |
| `GET` | `/db/push/status` | Push progress |

### Proxy

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/proxy/status` | Current proxy configuration |
| `POST` | `/proxy/detect` | Auto-detect SOCKS5 proxy |

---

## Database Schema

### `servers`
```sql
ip TEXT NOT NULL, port INTEGER, motd TEXT, protocol INTEGER,
version TEXT, online_players INTEGER, max_players INTEGER,
ping_ms INTEGER, modded INTEGER, mod_list TEXT,
whitelisted INTEGER, category TEXT, tags TEXT,
player_sample TEXT, first_seen TEXT, last_seen TEXT,
UNIQUE(ip, port)
```

### `scan_history`
```sql
started_at TEXT, finished_at TEXT,
targets_scanned INTEGER, servers_found INTEGER,
cycle_type TEXT
```

### `scanner_checkpoint`
```sql
cycle_type TEXT, cycle_num INTEGER, ip_u32 INTEGER,
scanned_ips INTEGER, found_servers INTEGER, updated_at TEXT
```

### `range_density`
```sql
ip_prefix TEXT PRIMARY KEY, servers_found INTEGER,
cycles_scanned INTEGER, last_scanned_at TEXT
```

### `kitty_ips` (separate `kitty.db`)
```sql
ip TEXT PRIMARY KEY, verified INTEGER, motd TEXT,
version TEXT, online_players INTEGER, max_players INTEGER,
ping_ms INTEGER, first_seen TEXT, last_seen TEXT
```

---

## Performance Tuning

- **Concurrency**: 4000 is a good default. 10000 for maximum speed (requires good bandwidth and open file limits).
- **File descriptors**: Increase ulimit if you hit "too many open files": `ulimit -n 100000`
- **Bandwidth**: ~200-500 Mbps at 10000 concurrency with 2s timeouts
- **CPU**: Rust handles 10k concurrent with minimal CPU (mostly async I/O)
- **Disk**: SQLite WAL mode handles concurrent read/write efficiently
- **Memory**: ~200MB at 10k concurrency (JoinHandle tracking + channel buffer)

---

## Development

```bash
# Frontend dev server (hot reload)
npm install && npm run dev

# Backend build
cd src-tauri && cargo build --release

# Run with debug logging
RUST_LOG=debug ./src-tauri/target/release/minefind

# Run with warn-level (shows connection failures)
RUST_LOG=warn ./src-tauri/target/release/minefind
```

### Adding Ranges

Edit `src-tauri/src/scanner/ranges.rs`:
- `get_hosting_ranges()` — Add provider subnets
- `get_full_ipv4_ranges()` — Modify reserved exclusions
- `get_ipv6_ranges()` — Add v6 prefixes

### Adding Cycle Types

Edit `src-tauri/src/main.rs` — `CycleType` enum:
```rust
impl CycleType {
    fn ports(&self) -> Vec<u16> { ... }
    fn label(&self) -> &'static str { ... }
}
```

---

## License

Private repository. All rights reserved.

---

## Acknowledgments

- [KittyScanBlocklist](https://github.com/LillySchramm/KittyScanBlocklist) — IP blocklist of Minecraft server scanners
- Built with [Rust](https://rust-lang.org), [Tokio](https://tokio.rs), [Axum](https://github.com/tokio-rs/axum), [Svelte 5](https://svelte.dev), [SQLite](https://sqlite.org)
