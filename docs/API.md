# MineFind API Reference

All endpoints at `http://localhost:8765/api/`.

---

## Scan Control

### `POST /scan/start`
Start a new scan cycle.

**Query Parameters:**
| Param | Type | Default | Description |
|-------|------|---------|-------------|
| `concurrency` | int | 6000 | Parallel connections (100-10000) |
| `probe_whitelist` | 0/1 | 1 | Detects whitelist status per server |
| `force_proxy` | 0/1 | 0 | Route all traffic through proxy |
| `proxy` | string | — | SOCKS5 proxy address |
| `cycle_type` | string | — | Start specific cycle type: `ipv4_fast`, `ipv6_targeted`, `ipv4_deep`, `ipv6_deep` |
| `fresh` | 0/1 | 0 | 1 = ignore checkpoint, start fresh |

**Response:**
```json
{
  "ok": true,
  "message": "scan started (multi-cycle mode)",
  "cycle": 1,
  "probe_whitelist": true,
  "concurrency": 6000,
  "resuming": false
}
```

### `POST /scan/cancel`
Stop the current scan. Saves checkpoint for resume.

**Response:** `{"ok": true}`

### `GET /scan/status`
Live scan progress.

**Response:**
```json
{
  "running": true,
  "cycle_type": "ipv6_targeted",
  "cycle": 1,
  "status": "scanning",
  "total_ips": 1048576,
  "scanned_ips": 425591,
  "lifetime_scanned": 53500591,
  "found_servers": 1,
  "current_range": "2a01:4f8::/32",
  "elapsed_secs": 140
}
```

### `GET /scan/cycles`
Cycle history and summary.

**Response:**
```json
{
  "summary": {
    "cycles": 0,
    "total_servers_found": 0,
    "total_targets_scanned": 0,
    "actual_servers": 4433
  },
  "history": [...],
  "checkpoint": null
}
```

### `POST /scan/concurrency`
Change concurrency during scan.
**Query:** `?n=8000`

### `POST /scan/reset`
Clear all cycle history, checkpoints, and range tracking. Keeps server data.

**Response:** `{"ok": true}`

---

## Servers

### `GET /servers`
All discovered servers, sorted by online players descending.

**Response:** Array of ServerInfo objects:
```json
[{
  "ip": "51.81.0.222",
  "port": 25565,
  "motd": "Velocity Proxy",
  "version": "Velocity 1.7.2-26.1.2",
  "protocol": 767,
  "online_players": 34,
  "max_players": 1337,
  "ping_ms": 15,
  "modded": false,
  "whitelisted": false,
  "category": "vanilla_survival",
  "tags": ["vanilla_survival", "medium", "survival", "1.21"],
  "last_seen": "2026-05-29T01:00:00Z",
  "first_seen": "2026-05-28T12:00:00Z"
}]
```

### `GET /servers/count`
Total server count. **Response:** `{"count": 4433}`

### `GET /stats`
Aggregated statistics.

**Response:**
```json
{
  "total": 4433,
  "whitelisted": 0,
  "not_whitelisted": 68,
  "unknown_whitelist": 4365,
  "modded": 120,
  "total_players": 15234,
  "categories": {
    "vanilla_survival": 3200,
    "modded": 120,
    ...
  },
  "versions": {
    "1.21": 2800,
    "1.20": 800,
    ...
  }
}
```

### `POST /servers/reverify-wl`
Re-probe all servers for whitelist status. Background task.

### `GET /servers/reverify-wl/status`
WL re-verify progress. **Response:** `{"running": true, "total": 4433, "done": 1200}`

### `POST /servers/dedup`
Check for duplicate (ip, port) pairs.

**Response:**
```json
{
  "total": 4433,
  "unique": 4433,
  "duplicates": 0,
  "duplicate_pairs": []
}
```

---

## Settings

### `GET /settings`
All current settings.

**Response:**
```json
{
  "concurrency": 6000,
  "rescan_all": true,
  "force_proxy": false,
  "probe_whitelist": true,
  "cycle_ipv4_fast": true,
  "cycle_ipv6_targeted": true,
  "cycle_ipv4_deep": true,
  "cycle_ipv6_deep": true,
  "has_ipv6": true
}
```

### `POST /settings/rescan`
Toggle re-scan all ranges. **Query:** `?on=0/1`

### `POST /settings/cycle`
Enable/disable a cycle type. **Query:** `?cycle=ipv4_fast&on=0/1`

### `POST /settings/force-proxy`
Toggle force proxy. **Query:** `?on=0/1`

### `POST /settings/probe-wl`
Toggle whitelist probing. **Query:** `?on=0/1`

---

## KittyScan Blocklist

### `POST /kitty/sync`
Fetch latest IP list from `LillySchramm/KittyScanBlocklist` on GitHub.

**Response:** `{"ok": true, "added": 5, "removed": 0, "db_total": 1217}`

### `POST /kitty/verify`
Ping all blocklist IPs for Minecraft servers. Background task.

### `GET /kitty/list`
All blocklist IPs with verification results.

### `GET /kitty/stats`
Blocklist statistics.

**Response:** `{"total": 1217, "verified": 9, "online": 3, "syncing": false, "verifying": false}`

### `GET /kitty/status`
Sync/verify progress.

**Response:** `{"syncing": false, "verifying": true, "verify_total": 1217, "verify_done": 600, "verify_found": 5}`

---

## Server List Sources

### `POST /serverlist/seed`
Seed the server list database from existing discovered servers.

**Response:** `{"ok": true, "seeded": 500, "added": 0, "total": 470}`

### `GET /serverlist/stats`
Server list database statistics. **Response:** `{"total": 470}`

---

## SRV Records

### `GET /srv/{domain}`
Resolve Minecraft SRV records for a domain.

**Example:** `GET /srv/mc.hypixel.net`

**Response:**
```json
{
  "domain": "mc.hypixel.net",
  "records": []
}
```

---

## Database Export

### `POST /db/push`
Push databases to `minefind-database` GitHub repo. Background task.

### `GET /db/push/status`
Push progress. **Response:** `{"running": false, "status": "pushed at 2026-05-29 — 4433 servers"}`

---

## Proxy

### `GET /proxy/status`
Current proxy configuration. **Response:** `{"proxy": null, "force_proxy": false}`

### `POST /proxy/detect`
Auto-detect SOCKS5 proxy on common ports (9050, 9150, 1080, 1088).

---
