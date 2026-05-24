# MineFind — Minecraft Server Discovery & Dashboard

## Goal
Discover non-whitelisted, small/private Minecraft servers at scale,
categorize them, and browse via a desktop dashboard.

## Architecture
Tauri (Rust backend + Svelte frontend) → SQLite

## Scanner Design
- Async TCP scanner using Tokio
- Minecraft Server List Ping protocol (no login needed for basic info)
- Login probe through Tor SOCKS5 for whitelist detection
- Scan targets: hosting CIDRs → residential ranges → Shodan/Censys

## Categorization
- By version, player count, MOTD keywords, mod/plugin indicators
- Tags: vanilla, modded, anarchy, private, idle, creative, minigame

## Privacy
- Ping-only mode: completely anonymous
- Login probe: Tor-routed, IP never exposed
- Configurable rate limits

## Output
Tauri app with web dashboard showing live scan progress + results browser

## Project Structure
```
minefind/
├── docs/PLAN.md
├── src-tauri/
│   ├── src/
│   │   ├── main.rs
│   │   ├── scanner/
│   │   │   ├── mod.rs
│   │   │   ├── ping.rs
│   │   │   ├── probe.rs
│   │   │   └── ranges.rs
│   │   ├── detect/
│   │   │   ├── mod.rs
│   │   │   └── categorize.rs
│   │   ├── db/
│   │   │   ├── mod.rs
│   │   │   └── schema.rs
│   │   └── proxy/
│   │       ├── mod.rs
│   │       └── tor.rs
│   ├── Cargo.toml
│   └── tauri.conf.json
├── src/
│   ├── App.svelte
│   ├── main.js
│   ├── lib/
│   │   ├── Dashboard.svelte
│   │   ├── ServerList.svelte
│   │   ├── Filters.svelte
│   │   └── Map.svelte
├── public/
├── package.json
├── vite.config.js
└── svelte.config.js
```
