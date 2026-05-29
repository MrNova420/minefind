# MineFind Detection System

## Overview

MineFind detects Minecraft servers through multiple methods:

1. **Modern TCP Ping** — Minecraft Java Edition 1.7+
2. **Legacy Ping** — Minecraft Java Edition pre-1.7
3. **Bedrock UDP Ping** — Minecraft Bedrock Edition
4. **SRV Record Resolution** — DNS-based discovery
5. **Server List Scraping** — Public API feeds

## 1. Modern TCP Ping (Java 1.7+)

### Protocol Flow

```
Client                               Server
  |                                     |
  |--- [Handshake: protocol=767] ------->|
  |--- [Status Request: 0x00] ---------->|
  |                                     |
  |<-- [Status Response: JSON] ----------|
  |                                     |
  |--- [Ping: 0x01, time=now] --------->|
  |<-- [Pong: 0x01, time=now] ----------|
```

### Protocol Versions Tried (in order)

| Protocol | Minecraft Version | Era |
|----------|-------------------|-----|
| 767 | 1.21 | 2024 |
| 766 | 1.20.6 | 2024 |
| 765 | 1.20.4 | 2024 |
| 763 | 1.20.1 | 2023 |
| 757 | 1.18 | 2021 |
| 754 | 1.16.5 | 2021 |
| 735 | 1.16 | 2020 |
| 578 | 1.15.2 | 2020 |
| 498 | 1.14.4 | 2019 |
| 404 | 1.13.2 | 2018 |
| 340 | 1.12.2 | 2017 |
| 316 | 1.11.2 | 2016 |
| 210 | 1.10 | 2016 |
| 110 | 1.9.4 | 2016 |
| 47 | 1.8.9 | 2015 |
| 5 | 1.7.10 | 2014 |

If one protocol version fails (unexpected packet ID, timeout, parse error), the next version is tried automatically. This ensures servers running any version from 1.7 through 1.21 are detected.

### True Ping Measurement

After receiving the status response, MineFind sends a proper ping packet (0x01) to measure round-trip time. This gives accurate latency measurements, not just the total connection time.

### Packet Format

```
[VarInt: packet_length] [VarInt: 0x00] [VarInt: protocol_version] [VarInt: host_length] [host_string] [u16: port] [VarInt: 1]
```

## 2. Legacy Ping (pre-1.7)

For Minecraft versions before the modern handshake protocol.

### Protocol Flow

```
Client                               Server
  |                                     |
  |--- [0xFE] ------------------------->|
  |                                     |
  |<-- [0xFF][2-byte len][UTF-16BE] ----|
```

### Response Format

```
[0xFF][u16: string_length][UTF-16BE: "MOTD§PLAYERS§MAX"]
```

Tried as a fallback after ALL 16 modern protocol versions fail.

## 3. Bedrock UDP Ping

Minecraft Bedrock Edition uses the RakNet protocol over UDP, completely different from Java Edition TCP.

### Protocol Flow

```
Client                               Server
  |                                     |
  |--- [0x01][timestamp][magic][GUID] ->| (Unconnected Ping)
  |                                     |
  |<-- [0x1C][ts][GUID][magic][str] ----| (Unconnected Pong)
```

### Response String Format

```
MCPE;MOTD;PROTOCOL;VERSION;PLAYERS;MAX;GUID;LEVEL;GAMEMODE;SUB_MOTD;...
```

Fields extracted:
- MOTD (server name/description)
- Protocol version
- Version string
- Online players
- Max players
- Server GUID (unique identity)
- Game mode (Survival, Creative, Adventure)
- Sub-MOTD (second line)

### Ports Scanned

- Fast scan: port 19132
- Deep scan: ports 19130-19140 (11 ports)

## 4. SRV Record Resolution

Minecraft servers can advertise their real address via DNS SRV records.

### Record Format

```
_minecraft._tcp.example.com.  IN SRV  0 5 25566 server.example.com.
```

This tells clients to connect to `server.example.com:25566` instead of `example.com:25565`.

### Implementation

- Uses `hickory-resolver` for async DNS lookups
- Endpoint: `GET /api/srv/:domain`
- Returns list of (host, port) targets
- Used by the server list scraper to discover real IPs behind proxies

## 5. Whitelist Detection

### Protocol Flow

```
Client                               Server
  |                                     |
  |--- [Handshake: next_state=2] ------>|
  |--- [Login Start: username] -------->|
  |                                     |
  |<-- [Possible responses] ------------|

    0x00 = Disconnect (JSON reason)
         Contains "whitelist"? → WHITELISTED ✓
         Other reason → NOT WHITELISTED
    0x01 = Encryption Request → UNKNOWN (online-mode, can't determine)
    0x02 = Login Success → NOT WHITELISTED (we got in!)
    Timeout → NOT WHITELISTED (server accepted, kept connection open)
```

### Probe Timeout

- TCP Connect: 4 seconds
- Read Response: 4 seconds
- Encryption requests correctly return `None` (not `Some(false)`) since we can't authenticate

## 6. Progressive Discovery

When a server is found on a primary port (e.g., 25565), MineFind automatically probes nearby ports on the same IP:

- Java server found → probes 25566, 25567, 25568, 19132, 19133
- Bedrock server found → probes 25565, 19133, 19134
- Runs in background, doesn't block main scanning

## 7. Server Fingerprinting

Each server gets a unique fingerprint hash computed from:
- MOTD text
- Version string
- Online/max player counts

This allows tracking server identity across IP changes and detecting when the same server moves to a different address.
