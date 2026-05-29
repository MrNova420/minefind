# MineFind Cycle System

## Overview

MineFind scans in **4-cycle rotations**, ordered by estimated completion time (ETA). Each cycle type has a specific purpose: fast discovery, followed by thorough deep scanning.

## Cycle Order

| # | Type | Ports | Scope | Est. Duration (6K) | Purpose |
|---|------|-------|-------|--------------------|---------|
| 1 | IPv6 Targeted | 25565, 19132 | 1M v6 IPs | ~5 min | Quick v6 server discovery |
| 2 | IPv6 Deep | 27 ports | 4M v6 IPs | ~2 hours | Thorough v6 sweep |
| 3 | IPv4 Fast | 25565, 19132 | 2.78B v4 IPs | ~8 days | Bulk Java + Bedrock discovery |
| 4 | IPv4 Deep | 27 ports | 2.78B v4 IPs | ~16 days | Final sweep — every server on Earth |

**Total**: ~24 days continuous + PC health pauses (~3 days) ≈ ~27 days

## Fast Cycle Details

### IPv4 Fast
- **Ports**: 25565 (Java), 19132 (Bedrock)
- **Scope**: All ~2.78 billion public IPv4 addresses
- **Timeout**: 2 seconds per connection
- **Protocol**: Tries 16 versions + legacy fallback
- **Rate**: ~3,000 IPs/sec at 6K concurrency
- **Duration**: ~8 days
- **Priority**: Hosting provider ranges scanned first for immediate results

### IPv6 Targeted
- **Ports**: 25565, 19132
- **Scope**: 16 /32 hosting provider prefixes, 1 IP per /48 subnet
- **Rate**: Very fast (~200K IPs/sec, all local fail)
- **Duration**: ~5 minutes
- **Requires IPv6 connectivity**

## Deep Cycle Details

### IPv4 Deep
- **Ports**: 27 ports per IP
  - Java: 25560-25575 (16 ports)
  - Bedrock: 19130-19140 (11 ports)
- **Parallel Ports**: All 27 ports probed concurrently per IP via `futures::join_all`
- **Timeout**: 3 seconds per port (extra time for thoroughness)
- **Rate**: ~2,000 IPs/sec at 6K concurrency
- **Duration**: ~16 days
- **Purpose**: Find servers on ANY port, not just defaults

### IPv6 Deep
- **Same as IPv4 Deep but on IPv6 ranges**
- 4 IPs per /48 subnet (::1 through ::4)
- ~4 million total IPs across 16 prefixes
- Duration: ~2 hours

## Cycle Toggles

All 4 cycles can be enabled/disabled in Settings. Useful scenarios:

- **No IPv6**: Disable both v6 cycles → scanner loops v4 Fast ↔ v4 Deep
- **Fast only**: Disable deep cycles → just v4 Fast looping
- **Custom order**: Disable all except one → scanner runs that single cycle repeatedly

Disabled cycles are automatically skipped in the rotation.

## Checkpoint & Resume

### How It Works
1. Scanner periodically saves its position to `scanner_checkpoint` table
2. Saves every 50,000 IPs during scanning
3. Saves immediately on scan start
4. Saves final position on cancel/stop

### Resume Behavior
- **Stop → Resume**: Continues from exact last position
- **Crash → Restart**: Auto-loads checkpoint, resumes
- **Complete cycle → Next cycle**: Checkpoint cleared, fresh start
- **Settings "Start Fresh"**: Ignores checkpoint, starts from beginning

### Clicking "Continue" in Cycles Tab
- Starts the selected cycle type
- If checkpoint exists for that type → resumes from saved position
- If no checkpoint → starts fresh (all ranges from beginning)

## PC Health Pause

Every 2 hours of continuous scanning, MineFind pauses for 15 minutes.

- **Display**: Yellow progress bar with "PC health pause" text
- **Purpose**: Prevents overheating, reduces network load, keeps PC usable
- **During pause**: Cancel check every 5 seconds, can stop immediately
- **Checkpoint**: Saved before and after pause

## Density Tracking

After each /8 range completes, MineFind records:
- Servers found in that range
- Which cycle type scanned it
- How many times it's been scanned

This data is used to:
- Show server density per range
- Skip ranges with 0 servers after 3+ cycles (configurable)
- But only if "Re-scan all ranges" is OFF in Settings

## Range Organization

### Priority Ranges (scanned first)
Hosting providers known for high Minecraft server density:
- OVH (6 ranges)
- Hetzner (18 ranges)
- AWS EC2 (28 ranges)
- DigitalOcean (10 ranges)
- Vultr (6 ranges)
- Linode (10 ranges)
- Contabo (4 ranges)

### Full /8 Ranges
All remaining public IPv4 /8 blocks (~220 ranges), each containing ~16.8 million IPs.

Reserved/private ranges automatically skipped:
- 0.0.0.0/8 (Local Identification)
- 10.0.0.0/8 (Private)
- 100.64.0.0/10 (CGNAT)
- 127.0.0.0/8 (Loopback)
- 169.254.0.0/16 (Link-Local)
- 172.16.0.0/12 (Private)
- 192.168.0.0/16 (Private)
- 198.18.0.0/15 (Benchmarking)
- 224.0.0.0/4 (Multicast)
- 240.0.0.0/4 (Reserved)

## IPv6 Ranges

### Hosting Provider Prefixes
16 /32 prefixes from major hosting providers:

| Provider | Prefixes |
|----------|----------|
| Hetzner | `2a01:4f8::/32`, `2a01:4f9::/32`, `2a03:4000::/29` |
| OVH | `2001:41d0::/32`, `2001:41d1::/32` |
| DigitalOcean | `2604:a880::/32`, `2400:6180::/32` |
| Vultr | `2a04:3540::/32`, `2001:19f0::/32` |
| Linode | `2600:3c00::/32`, `2a01:7e00::/32` |
| AWS | `2400:cb00::/32`, `2a05:d014::/32`, `2600:1f18::/32` |
| Contabo | `2a02:7aa0::/32`, `2a02:c206::/32` |

Each /32 contains 65,536 /48 subnets.
- Fast v6: 1 IP per /48 (1M IPs total)
- Deep v6: 4 IPs per /48 (4M IPs total)
