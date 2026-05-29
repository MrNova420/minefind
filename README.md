# MineFind

**High-performance Minecraft server discovery engine.** Scans the entire public IPv4 and IPv6 hosting space to find and catalog every Minecraft server on planet Earth.

<p align="center">
  <img src="https://img.shields.io/badge/rust-1.85+-orange?logo=rust" alt="Rust">
  <img src="https://img.shields.io/badge/svelte-5-red?logo=svelte" alt="Svelte 5">
  <img src="https://img.shields.io/badge/sqlite-3-blue?logo=sqlite" alt="SQLite">
  <img src="https://img.shields.io/badge/tokio-async-purple" alt="Tokio">
</p>

---

## Features

### Core Scanner
- **Full IPv4 world scan** — 2.78 billion public IPs, every address on the internet checked
- **IPv6 hosting scan** — 16 hosting provider /32 prefixes, 1M addresses per cycle
- **4-cycle rotation** — ordered by speed, auto-advancing:
  - Cycle 1: `IPv6 Targeted` — 16 hosting v6 prefixes, 2 ports (~5 min)
  - Cycle 2: `IPv6 Deep` — extended v6 with 27 ports (~2 hours)
  - Cycle 3: `IPv4 Fast` — 2.78B IPs, Java (25565) + Bedrock (19132) (~8 days)
  - Cycle 4: `IPv4 Deep` — 2.78B IPs, 27 ports, parallel batching (~16 days)
- **Total duration**: ~24 days at 6K concurrency. Health pauses add ~3 days = ~27 days.
- **Configurable cycle toggles** — enable/disable each cycle type in Settings
- **Checkpoint & Resume** — saves every 50K IPs, auto-resumes on restart or cancel
- **Concurrent scanning** — Configurable 100–10,000 parallel connections via semaphore pool
- **PC health pauses** — 15 minute cooldown every 2 hours to keep your PC usable

### Detection
- **16 protocol versions** — 767 (1.21) → 766 → 765 → 763 → 757 → 754 → 735 → 578 → 498 → 404 → 340 → 316 → 210 → 110 → 47 → 5 (1.7.10)
- **Legacy ping fallback** — 0xFE byte ping for pre-1.7 Minecraft servers (UTF-16BE §-delimited)
- **Bedrock RakNet ping** — UDP unconnected ping for Minecraft Bedrock Edition
- **True ping measurement** — sends proper ping packet (0x01) after status for accurate RTT
- **Dynamic timeouts** — 2s for fast cycles, 3s for deep cycles
- **Whitelist probe** — Login attempt detection with proper packet framing, encryption request = inconclusive
- **SRV record resolution** — DNS SRV lookups for `_minecraft._tcp.{domain}` to discover servers on non-standard ports
- **Progressive discovery** — probes nearby ports on same IP when a server is found
- **Server fingerprinting** — hash-based identity tracking: MOTD + version + player counts
- **Server categorization** — Auto-tags: Vanilla Survival, Modded, Plugin Heavy, Creative, Minigame, Anarchy, Private Group, Idle
- **Proxy detection** — Auto-detects Velocity, BungeeCord, NullCord, Geyser from version strings
- **Mod tracking** — Forge/FML/Fabric/Quilt detection, mod list with versions
- **MOTD parsing** — Strips Minecraft formatting codes (§), extracts text + extras from JSON chat components

### Server List Sources
- **Seed from existing DB** — Pulls known server IPs into a separate `serverlists.db` for tracking and analysis
- **DNS SRV resolution** — `GET /api/srv/:domain` resolves Minecraft SRV records
- **Multi-source architecture** — Server lists, DNS, and direct scanning all feed the same database

### KittyScan Blocklist Verification
- **Separate database** — `kitty.db` completely isolated from the main `servers.db`
- **Sync from GitHub** — Pulls IP list from `LillySchramm/KittyScanBlocklist`
- **Verify all** — Pings every IP on ports 25565 + 19132, records server info
- **Live progress** — Real-time verification progress bar with found-servers counter
- **Re-verify** — Always re-pings all IPs for fresh results

### Database & Export
- **SQLite WAL mode** — Concurrent read/write with no lock contention
- **Batch DB writes** — mpsc channel to single writer task, no Mutex contention
- **Fallback lifetime counter** — File-based counter survives DB failures
- **Push to GitHub** — One-click dashboard button pushes DBs + `servers.json` to `minefind-database` repo
- **Cycle history** — Every cycle recorded with type, timestamps, IPs scanned, servers found
- **No duplicates** — `UNIQUE(ip, port)` constraint prevents duplicate server entries

### Web Dashboard
- **Live progress** — IPs scanned counter, cycle type, ETA, scan rate, current range
- **EDA display** — Estimated completion time based on current scan rate
- **Cooling indicator** — Yellow progress bar during PC health pauses
- **Server browser** — Search, category filter, whitelist filter, port filter, sort, export JSON
- **WL re-verify** — One-click whitelist probe of all servers with live progress
- **Duplicates check** — Verify database integrity, zero duplicates guaranteed
- **Settings panel** — Concurrency slider, 4 cycle toggles, re-scan all, WL probe, proxy config
- **Cycles panel** — Continue by type, full cycle history, checkpoint info, active cycle indicator

### Proxy Support
- **SOCKS5 proxy** — Route scan traffic through Tor or any SOCKS5 proxy
- **Off by default** — Direct connections for maximum speed
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
│           ├── mod.rs         # ServerInfo, ServerCategory structs
│           ├── ping.rs        # Multi-protocol ping (8 versions), true ping, enrichment
│           ├── probe.rs       # Whitelist detection (login attempt)
│           ├── bedrock.rs     # Bedrock RakNet UDP ping
│           └── ranges.rs      # IPv4 /8 ranges, priority ranges, IPv6 prefixes
│
├── src/                       # Svelte 5 frontend
│   ├── main.js                # Entry point
│   ├── app.css                # Global styles (dark theme)
│   ├── App.svelte             # Main app shell: nav, settings, scan controls, progress
│   └── lib/
│       ├── Dashboard.svelte   # Stats, progress, scan rate/ETA cards, server table
│       ├── ServerList.svelte  # Filterable server browser with WL tags, export
│       ├── Cycles.svelte      # Cycle types, continue buttons, history, active indicator
│       ├── Kitty.svelte       # KittyScan blocklist tab
│       ├── Map.svelte         # Geographic map view
│       └── Filters.svelte     # Category/version filter controls
│
├── data/                      # Bundled database files (committed)
│   ├── servers.db             # ~4,400 discovered servers
│   ├── kitty.db               # ~1,200 KittyScan blocklist IPs
│   └── lifetime_counter.txt   # Total IPs scanned across all time
│
├── setup.sh                   # Full auto-install: checks deps, builds, copies DBs
├── run.sh                     # Quick launch: rebuilds if stale, starts server
├── index.html                 # SPA entry
├── package.json               # Node dependencies
├── vite.config.js             # Vite build config
├── svelte.config.js           # Svelte compiler config
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
- **git** — For database push feature

### Fresh Install

```bash
git clone https://github.com/MrNova420/minefind.git
cd minefind
chmod +x setup.sh run.sh
./setup.sh    # One-time: checks deps, installs, builds, copies DBs
./run.sh      # Launches server on http://localhost:8765
```

The setup script automatically:
1. Checks Rust, Node.js, npm are installed
2. Installs npm dependencies
3. Builds the Svelte frontend
4. Compiles the Rust backend
5. Copies bundled databases from `data/` to `~/.local/share/minefind/`
6. Configures git identity

Open `http://localhost:8765` in your browser.

### Manual Build

```bash
# Frontend
cd minefind && npm install && npm run build

# Backend
cd src-tauri && cargo build --release

# Run (from minefind/ directory)
MINEFIND_FRONTEND=dist RUST_LOG=info ./src-tauri/target/release/minefind
```

---

## Usage

### Scanning

1. Open `http://localhost:8765`
2. Configure in **Settings** (⚙):
   - **Concurrency** — 500–10,000 (default: 4,000)
   - **Cycle toggles** — Enable/disable each cycle type
   - **WL Probe** — Detect whitelist status per server (default: on)
   - **Proxy** — SOCKS5 address + Force Proxy toggle (default: off)
3. Click **Start Fresh** to begin multi-cycle world scan
4. Watch live stats: IPs scanned, scan rate, ETA, servers found
5. Click **Stop** — checkpoint saves, use **Resume** to continue
6. Between cycles: 15 min PC health pause every 2 hours (yellow bar)

### Multi-Cycle Flow

Ordered by ETA (shortest first), configurable via toggles:

| # | Type | Ports | Scope | Est. Duration (6K conc) |
|---|------|-------|-------|------|
| 1 | IPv6 Fast | 25565 + 19132 | 1M IPv6 IPs (16 prefixes) | ~5 min |
| 2 | IPv6 Deep | 27 ports (25560-75 + 19130-40) | 4M IPv6 IPs | ~2 hours |
| 3 | IPv4 Fast | 25565 + 19132 | 2.78B IPv4 IPs | ~8 days |
| 4 | IPv4 Deep | 27 ports, parallel batching | 2.78B IPv4 IPs | ~16 days |

**Total**: ~24 days continuous + health pauses ≈ ~27 days at 6K concurrency.

**Deep cycle ports**: 25560-25575 (Java) + 19130-19140 (Bedrock) = 27 ports per IP, probed concurrently.

### Server Browser

- **Search** by MOTD or IP
- **Filters**: Category, Whitelist, Port (25565/19132)
- **Sort** by players, ping, or name
- **Export JSON** — downloads the full server list
- **Check Dupes** — verify database integrity
- **Re-verify WL** — re-probe all servers for whitelist status
- **WL tags**: Green "Open" (can join), Red "WL" (whitelist), Gray "?" (unknown)

### Dashboard

- **Stats cards**: Not Whitelisted, Whitelisted, Unknown WL, Total Servers, Players Online, Modded
- **Progress**: Current Cycle IPs, Scan Rate (IPs/sec), ETA, Lifetime Scanned
- **Categories**: Bar chart showing server distribution across types
- **Versions**: Bar chart showing Minecraft version breakdown
- **Server table**: Top servers by player count with live last-seen timestamps
- **Push to GitHub**: Export entire database with one click

### Cycles Panel

- **Continue by Type**: One button per cycle type, instant resume
- **Full History**: Every completed cycle with type, IPs scanned, servers found, timestamps
- **Active indicator**: Pulsing dot shows which cycle is currently scanning
- **Checkpoint**: Shows saved progress if a cycle was paused

### KittyScan Blocklist

1. Navigate to **Kitty** tab
2. Click **Sync from GitHub** — downloads from `LillySchramm/KittyScanBlocklist`
3. Click **Verify All** — pings every IP on ports 25565 + 19132
4. Live progress bar shows verification status
5. Verified servers appear with server details

### Database Export

Click **Push to GitHub** in Dashboard to sync to `minefind-database` repo:
- Clones/pulls the repo
- Copies both `servers.db` and `kitty.db`
- Exports full `servers.json` with cycle stats
- Commits with timestamp and pushes

### Git Authentication

```bash
git config --global credential.helper store
# Generate token at https://github.com/settings/tokens (scope: repo)
# First push caches credentials for both code and DB repos
```

---

## API Reference

All endpoints at `http://localhost:8765/api/`.

### Scan

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/scan/start?concurrency=N&cycle_type=X` | Start scan, optionally pin to a cycle type |
| `POST` | `/scan/cancel` | Stop scan (saves checkpoint) |
| `GET` | `/scan/status` | Live progress: type, cycle, scanned, found, ETA, lifetime |
| `GET` | `/scan/cycles` | History + summary + checkpoint info |
| `POST` | `/scan/concurrency?n=N` | Change concurrency during scan |
| `POST` | `/scan/reset` | Clear cycles/checkpoints, keep servers |

### Servers

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/servers` | All discovered servers (sorted by players DESC) |
| `GET` | `/servers/count` | Total server count |
| `GET` | `/stats` | Aggregated stats (categories, versions, whitelist breakdown) |
| `POST` | `/servers/reverify-wl` | Re-probe all servers for whitelist status |
| `GET` | `/servers/reverify-wl/status` | WL re-verify progress |
| `POST` | `/servers/dedup` | Check for duplicate (ip, port) pairs |

### Settings

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/settings` | All settings state |
| `POST` | `/settings/rescan?on=0/1` | Re-scan all ranges toggle |
| `POST` | `/settings/cycle?cycle=X&on=0/1` | Enable/disable cycle type |
| `POST` | `/settings/force-proxy?on=0/1` | Force proxy toggle |
| `POST` | `/settings/probe-wl?on=0/1` | Whitelist probe toggle |

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
cycles_scanned INTEGER, last_scanned_at TEXT, last_cycle_type TEXT
```

### `kitty_ips` (separate `kitty.db`)
```sql
ip TEXT PRIMARY KEY, verified INTEGER, motd TEXT,
version TEXT, online_players INTEGER, max_players INTEGER,
ping_ms INTEGER, first_seen TEXT, last_seen TEXT
```

---

## Performance Tuning

- **Concurrency**: 4,000 default. 6,000 for faster scanning. 10,000 for maximum speed.
- **File descriptors**: `ulimit -n 100000` if needed
- **Bandwidth**: ~300-800 Mbps at 6,000 concurrency
- **CPU**: Rust async I/O — minimal CPU usage even at 10K concurrent
- **Disk**: SQLite WAL mode handles concurrent read/write efficiently
- **Memory**: ~300MB at 6K concurrency (JoinHandle tracking + channel buffer)
- **IPv6**: Requires IPv6 connectivity. If unavailable, disable in Settings.

---

## Development

```bash
# Frontend dev server (hot reload)
npm install && npm run dev

# Backend build
cd src-tauri && cargo build --release

# Run with debug logging
RUST_LOG=debug ./src-tauri/target/release/minefind

# Run with warn-level (connection failures)
RUST_LOG=warn ./src-tauri/target/release/minefind
```

### Adding Protocol Versions
Edit `src-tauri/src/scanner/ping.rs` — `PROTOCOL_VERSIONS` array.

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
