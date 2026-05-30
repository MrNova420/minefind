use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use crate::scanner::{ServerInfo, PlayerSample, ServerCategory};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

fn should_log_ping_fail() -> bool {
    static LAST_LOG: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    LAST_LOG.swap(now, Ordering::Relaxed) < now
}

const PING_TIMEOUT_FAST: Duration = Duration::from_secs(4);
const PING_TIMEOUT_DEEP: Duration = Duration::from_secs(6);

/// Check additional ports on the same IP when a server is found (progressive discovery)
pub async fn discover_nearby_ports(ip: &str, found_port: u16, deep: bool) -> Vec<ServerInfo> {
    let extra_ports: Vec<u16> = if found_port == 25565 {
        vec![25566, 25567, 25568, 19132, 19133]
    } else if found_port == 19132 {
        vec![25565, 19133, 19134]
    } else if (19130..=19140).contains(&found_port) {
        vec![25565, 19132]
    } else {
        vec![25565, 19132]
    };

    let mut results = Vec::new();
    for port in extra_ports {
        let r = if deep {
            ping_server_deep(ip, port).await
        } else {
            ping_server(ip, port).await
        };
        if let Ok(info) = r { results.push(info); }
    }
    results
}

// Protocol versions to try, ordered from newest to oldest
const PROTOCOL_VERSIONS: &[(i32, &str)] = &[
    (767, "1.21"),
    (766, "1.20.6"),
    (765, "1.20.4"),
    (764, "1.20.2"),
    (763, "1.20.1"),
    (762, "1.19.4"),
    (761, "1.19.3"),
    (760, "1.19.2"),
    (759, "1.19"),
    (758, "1.18.2"),
    (757, "1.18"),
    (756, "1.17.1"),
    (755, "1.17"),
    (754, "1.16.5"),
    (736, "1.16.3"),
    (735, "1.16"),
    (578, "1.15.2"),
    (498, "1.14.4"),
    (404, "1.13.2"),
    (340, "1.12.2"),
    (316, "1.11.2"),
    (210, "1.10"),
    (110, "1.9.4"),
    (47, "1.8.9"),
    (5, "1.7.10"),
];

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        if value & !0x7F == 0 {
            buf.push(value as u8);
            return;
        }
        buf.push((value as u8 & 0x7F) | 0x80);
        value = (value >> 7) & (i32::MAX >> 6);
    }
}

pub fn read_varint(reader: &mut &[u8]) -> Option<(i32, usize)> {
    let mut value: i32 = 0;
    let mut bytes_read = 0;
    loop {
        if bytes_read >= 5 || reader.is_empty() {
            return None;
        }
        let byte = reader[0];
        value |= ((byte & 0x7F) as i32) << (bytes_read * 7);
        bytes_read += 1;
        *reader = &reader[1..];
        if byte & 0x80 == 0 {
            return Some((value, bytes_read));
        }
    }
}

fn build_ping_packet(host: &str, port: u16, protocol: i32) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, protocol);
    write_varint(&mut body, host.len() as i32);
    body.extend_from_slice(host.as_bytes());
    body.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut body, 1); // next state: status

    let total = 1_i32 + body.len() as i32;
    let mut result = Vec::new();
    write_varint(&mut result, total);
    result.push(0x00); // packet ID: handshake
    result.extend_from_slice(&body);
    result
}

fn build_status_request() -> Vec<u8> {
    let mut result = Vec::new();
    write_varint(&mut result, 1);
    result.push(0x00);
    result
}

fn build_ping_request() -> Vec<u8> {
    let mut result = Vec::new();
    write_varint(&mut result, 1); // 1 byte follows
    result.push(0x01); // packet ID: ping
    result
}

pub async fn ping_server(ip: &str, port: u16) -> Result<ServerInfo, String> {
    ping_server_with_timeout(ip, port, PING_TIMEOUT_FAST).await
}

pub async fn ping_server_deep(ip: &str, port: u16) -> Result<ServerInfo, String> {
    ping_server_with_timeout(ip, port, PING_TIMEOUT_DEEP).await
}

pub async fn ping_server_via_proxy(ip: &str, port: u16, proxy: Option<&str>) -> Result<ServerInfo, String> {
    ping_server_via_proxy_with_timeout(ip, port, proxy, PING_TIMEOUT_FAST).await
}

pub async fn ping_server_with_sem(ip: &str, port: u16, cs: Option<Arc<tokio::sync::Semaphore>>) -> Result<ServerInfo, String> {
    ping_server_inner_with_sem(ip, port, None, PING_TIMEOUT_FAST, cs).await
}

pub async fn ping_server_deep_with_sem(ip: &str, port: u16, cs: Option<Arc<tokio::sync::Semaphore>>) -> Result<ServerInfo, String> {
    ping_server_inner_with_sem(ip, port, None, PING_TIMEOUT_DEEP, cs).await
}

#[allow(dead_code)]
pub async fn ping_server_via_proxy_with_sem(ip: &str, port: u16, proxy: Option<&str>, cs: Option<Arc<tokio::sync::Semaphore>>) -> Result<ServerInfo, String> {
    ping_server_inner_with_sem(ip, port, proxy, PING_TIMEOUT_FAST, cs).await
}

async fn ping_server_with_timeout(ip: &str, port: u16, timeout_dur: Duration) -> Result<ServerInfo, String> {
    ping_server_via_proxy_with_timeout(ip, port, None, timeout_dur).await
}

pub async fn ping_server_via_proxy_with_timeout(ip: &str, port: u16, proxy: Option<&str>, timeout_dur: Duration) -> Result<ServerInfo, String> {
    let result = ping_server_via_proxy_inner(ip, port, proxy, timeout_dur, None).await;
    match &result {
        Ok(info) => log::info!("PING OK {}:{} v={}", ip, port, info.version),
        Err(e) => {
            if log::log_enabled!(log::Level::Debug) && should_log_ping_fail() {
                log::debug!("PING FAIL {}:{} — {}", ip, port, e);
            }
        }
    }
    result
}

async fn ping_server_inner_with_sem(ip: &str, port: u16, proxy: Option<&str>, pto: Duration, cs: Option<Arc<tokio::sync::Semaphore>>) -> Result<ServerInfo, String> {
    let result = ping_server_via_proxy_inner(ip, port, proxy, pto, cs).await;
    match &result {
        Ok(info) => log::info!("PING OK {}:{} v={}", ip, port, info.version),
        Err(e) => {
            if log::log_enabled!(log::Level::Debug) && should_log_ping_fail() {
                log::debug!("PING FAIL {}:{} — {}", ip, port, e);
            }
        }
    }
    result
}

async fn ping_server_via_proxy_inner(ip: &str, port: u16, proxy: Option<&str>, pto: Duration, conn_sem: Option<Arc<tokio::sync::Semaphore>>) -> Result<ServerInfo, String> {
    let start = std::time::Instant::now();
    let _cp = match conn_sem {
        Some(ref s) => Some(s.clone().acquire_owned().await.unwrap()),
        None => None,
    };

    let stream = if let Some(proxy_addr) = proxy {
        match timeout(pto, crate::proxy::connect_through_socks5(proxy_addr, ip, port)).await {
            Ok(Ok(s)) => s,
            Ok(Err(e)) => return Err(format!("proxy connection failed to {}:{}: {}", ip, port, e)),
            Err(_) => return Err(format!("proxy timeout to {}:{}", ip, port)),
        }
    } else {
        timeout(pto, TcpStream::connect((ip, port)))
            .await
            .map_err(|_| format!("timeout connecting to {}:{}", ip, port))?
            .map_err(|e| format!("connection failed to {}:{}: {}", ip, port, e))?
    };

    let _ = stream.set_nodelay(true);
    let (mut reader, mut writer) = stream.into_split();

    // Try multiple protocol versions
    let mut last_err = "no protocols tried".to_string();
    for &(proto, _ver_label) in PROTOCOL_VERSIONS {
        let ping_packet = build_ping_packet(ip, port, proto);
        let mut send_buf = Vec::new();
        send_buf.extend_from_slice(&ping_packet);
        send_buf.extend_from_slice(&build_status_request());

        if timeout(pto, writer.write_all(&send_buf)).await.is_err() {
            last_err = format!("write error with proto {}", proto);
            continue;
        }

        // Read VarInt packet length
        let mut len_raw = Vec::new();
        let mut read_failed = false;
        loop {
            let mut byte = [0u8; 1];
            match timeout(pto, reader.read_exact(&mut byte)).await {
                Ok(Ok(_)) => {}
                _ => { read_failed = true; break; }
            }
            len_raw.push(byte[0]);
            if byte[0] & 0x80 == 0 { break; }
            if len_raw.len() > 5 { read_failed = true; break; }
        }
        if read_failed {
            last_err = format!("timeout reading packet length with proto {}", proto);
            continue;
        }

        let mut len_slice = &len_raw[..];
        let (packet_len, _) = match read_varint(&mut len_slice) {
            Some(v) => v,
            None => { last_err = format!("failed to parse VarInt with proto {}", proto); continue; }
        };

        if packet_len <= 0 || packet_len > 1_048_576 {
            last_err = format!("invalid packet len {} with proto {}", packet_len, proto);
            continue;
        }

        let mut packet_data = vec![0u8; packet_len as usize];
        if timeout(pto, reader.read_exact(&mut packet_data)).await.is_err() {
            last_err = format!("timeout reading packet data with proto {}", proto);
            continue;
        }

        let mut response = Vec::with_capacity(len_raw.len() + packet_data.len());
        response.extend_from_slice(&len_raw);
        response.extend_from_slice(&packet_data);

        match parse_ping_response(&response, ip, port, start.elapsed().as_millis() as i64) {
            Ok(mut info) => {
                // True ping measurement: send ping (0x01) after status
                let ping_start = std::time::Instant::now();
                if timeout(pto, writer.write_all(&build_ping_request())).await.is_ok() {
                    let mut pong_buf = [0u8; 4];
                    match timeout(pto, reader.read_exact(&mut pong_buf)).await {
                        Ok(Ok(_)) => {
                            info.ping_ms = ping_start.elapsed().as_millis() as i64;
                        }
                        _ => {}
                    }
                }
                info.version = enrich_version(&info.version, &info.motd);
                return Ok(info);
            }
            Err(e) => {
                // If this was a protocol mismatch, try next version
                if e.contains("unexpected packet ID") || e.contains("failed to read") || e.contains("JSON") {
                    last_err = format!("proto {} rejected: {}", proto, e);
                    continue;
                }
                // Not a protocol issue — don't retry
                return Err(e);
            }
        }
    }

    // Try legacy ping (pre-1.7) as final fallback
    if let Ok(legacy) = ping_legacy_fallback(ip, port, start).await {
        return Ok(legacy);
    }

    Err(last_err)
}

/// Legacy ping with fresh TCP connection (pre-1.7 Minecraft)
async fn ping_legacy_fallback(ip: &str, port: u16, start: std::time::Instant) -> Result<ServerInfo, String> {
    let tcp = match timeout(PING_TIMEOUT_FAST, TcpStream::connect((ip, port))).await {
        Ok(Ok(s)) => s,
        _ => return Err("legacy connect failed".into()),
    };
    let _ = tcp.set_nodelay(true);
    let (mut reader, mut writer) = tcp.into_split();

    // Send 0xFE byte (legacy server list ping)
    if timeout(PING_TIMEOUT_FAST, writer.write_all(&[0xFEu8])).await.is_err() {
        return Err("legacy write failed".into());
    }

    // Read header: 0xFF + 2-byte BE length
    let mut header = [0u8; 3];
    if timeout(PING_TIMEOUT_FAST, reader.read_exact(&mut header)).await.is_err() {
        return Err("legacy read header failed".into());
    }
    if header[0] != 0xFF {
        return Err(format!("legacy packet ID: {}", header[0]));
    }
    let str_len = ((header[1] as usize) << 8) | (header[2] as usize);
    if str_len == 0 || str_len > 4096 { return Err("legacy len invalid".into()); }

    let mut str_data = vec![0u8; str_len * 2];
    if timeout(PING_TIMEOUT_FAST, reader.read_exact(&mut str_data)).await.is_err() {
        return Err("legacy read str failed".into());
    }

    let utf16: Vec<u16> = str_data.chunks(2).map(|c| ((c[0] as u16) << 8) | (c[1] as u16)).collect();
    let resp = String::from_utf16(&utf16).map_err(|_| "legacy utf16")?;
    let parts: Vec<&str> = resp.split('§').collect();

    let ping_ms = start.elapsed().as_millis() as i64;
    let now = chrono::Utc::now().to_rfc3339();

    Ok(ServerInfo {
        ip: ip.to_string(), port,
        motd: parts.first().unwrap_or(&"").to_string(),
        version: "legacy".to_string(), protocol: -1,
        online_players: parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(0),
        max_players: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        ping_ms, modded: false, mod_list: vec![],
        whitelisted: None,
        category: ServerCategory::from_str("unknown"),
        tags: vec!["legacy".to_string()],
        player_sample: vec![], last_seen: now.clone(), first_seen: now,
    })
}

fn parse_ping_response(
    data: &[u8],
    ip: &str,
    port: u16,
    ping_ms: i64,
) -> Result<ServerInfo, String> {
    let mut slice = data;
    let (_packet_len, _) = read_varint(&mut slice).ok_or("failed to read packet length")?;
    let (packet_id, _) = read_varint(&mut slice).ok_or("failed to read packet ID")?;

    if packet_id != 0x00 {
        return Err(format!("unexpected packet ID: {}", packet_id));
    }

    let (json_len, _) = read_varint(&mut slice).ok_or("failed to read JSON length")?;
    if json_len as usize > slice.len() {
        return Err("JSON length exceeds remaining data".to_string());
    }

    let json_str = std::str::from_utf8(&slice[..json_len as usize])
        .map_err(|e| format!("invalid UTF-8 in response: {}", e))?;

    parse_status_json(json_str, ip, port, ping_ms)
}

fn enrich_version(version: &str, motd: &str) -> String {
    let lower_v = version.to_lowercase();
    let lower_m = motd.to_lowercase();

    // Detect proxy software from version string
    let mut enriched = version.to_string();
    if lower_v.contains("velocity") {
        if !enriched.contains("Velocity") { enriched = format!("{} (Velocity)", enriched); }
    } else if lower_v.contains("bungee") || lower_v.contains("waterfall") {
        if !enriched.contains("Bungee") { enriched = format!("{} (BungeeCord)", enriched); }
    } else if lower_v.contains("nullcord") {
        if !enriched.contains("NullCord") { enriched = format!("{} (NullCord)", enriched); }
    }

    // Detect Geyser (crossplay)
    if lower_v.contains("geyser") || lower_m.contains("geyser") ||
       lower_v.contains("floodgate") || lower_m.contains("floodgate") {
        enriched.push_str(" +Geyser");
    }

    // Detect ViaVersion
    if lower_v.contains("viaversion") {
        enriched.push_str(" +ViaVersion");
    }

    enriched
}

fn parse_status_json(
    json_str: &str,
    ip: &str,
    port: u16,
    ping_ms: i64,
) -> Result<ServerInfo, String> {
    let parsed: serde_json::Value =
        serde_json::from_str(json_str).map_err(|e| format!("JSON parse error: {}", e))?;

    let version_obj = parsed.get("version");
    let protocol = version_obj
        .and_then(|v| v.get("protocol"))
        .and_then(|v| v.as_i64())
        .unwrap_or(-1) as i32;
    let version = version_obj
        .and_then(|v| v.get("name"))
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    let players_obj = parsed.get("players");
    let online_players = players_obj
        .and_then(|p| p.get("online"))
        .and_then(|p| p.as_i64())
        .unwrap_or(0) as i32;
    let max_players = players_obj
        .and_then(|p| p.get("max"))
        .and_then(|p| p.as_i64())
        .unwrap_or(0) as i32;

    let player_sample = players_obj
        .and_then(|p| p.get("sample"))
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|s| {
                    Some(PlayerSample {
                        name: s.get("name")?.as_str()?.to_string(),
                        id: s.get("id")?.as_str()?.to_string(),
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    let description = parsed.get("description");
    let motd = description.map(|d| extract_motd(d)).unwrap_or_default();

    let modded = parsed
        .get("modinfo")
        .and_then(|m| m.get("type"))
        .and_then(|m| m.as_str())
        .map(|t| t == "FML" || t == "FORGE" || t == "LITE")
        .unwrap_or(false);

    let mod_list: Vec<String> = parsed
        .get("modinfo")
        .and_then(|m| m.get("modList"))
        .and_then(|m| m.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|m| {
                    let id = m.get("modid").and_then(|m| m.as_str()).unwrap_or("?");
                    let v = m.get("version").and_then(|m| m.as_str()).unwrap_or("");
                    if v.is_empty() { Some(id.to_string()) } else { Some(format!("{}@{}", id, v)) }
                })
                .collect()
        })
        .unwrap_or_default();

    let modded = modded || !mod_list.is_empty();
    let modded = modded || version.to_lowercase().contains("fabric") || version.to_lowercase().contains("quilt") || version.to_lowercase().contains("forge");

    let category = categorize_server(&motd, online_players, modded);
    let tags = generate_tags(&category, &version, online_players, modded, &motd);

    let now = chrono::Utc::now().to_rfc3339();

    Ok(ServerInfo {
        ip: ip.to_string(),
        port,
        motd,
        protocol,
        version,
        online_players,
        max_players,
        player_sample,
        ping_ms,
        modded,
        mod_list,
        whitelisted: None,
        category,
        tags,
        last_seen: now.clone(),
        first_seen: now,
    })
}

fn extract_motd(description: &serde_json::Value) -> String {
    match description {
        serde_json::Value::String(s) => strip_motd_formatting(s),
        serde_json::Value::Object(obj) => {
            if let Some(text) = obj.get("text").and_then(|t| t.as_str()) {
                let mut result = text.to_string();
                if let Some(extra) = obj.get("extra").and_then(|e| e.as_array()) {
                    for part in extra {
                        if let Some(t) = part.get("text").and_then(|t| t.as_str()) {
                            result.push_str(t);
                        }
                    }
                }
                strip_motd_formatting(&result)
            } else if let Some(translate) = obj.get("translate").and_then(|t| t.as_str()) {
                translate.to_string()
            } else {
                "unknown".to_string()
            }
        }
        _ => "unknown".to_string(),
    }
}

fn strip_motd_formatting(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        if chars[i] == '§' && i + 1 < chars.len() {
            i += 2;
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result.trim().to_string()
}

fn categorize_server(motd: &str, players: i32, modded: bool) -> ServerCategory {
    use super::ServerCategory::*;
    let lower = motd.to_lowercase();

    if players == 0 {
        return Idle;
    }

    if lower.contains("anarchy") || lower.contains("2b2t") || lower.contains("norules") {
        return Anarchy;
    }

    if lower.contains("minigame") || lower.contains("skywars") || lower.contains("bedwars")
        || lower.contains("kitpvp") || lower.contains("prison")
    {
        return Minigame;
    }

    if lower.contains("creative") || lower.contains("plot") || lower.contains("build") {
        return Creative;
    }

    if modded {
        return Modded;
    }

    if players <= 5 {
        if lower.contains("private") || lower.contains("friends") || lower.contains("family")
            || lower.contains("whitelist") || lower.contains("small")
        {
            return PrivateGroup;
        }
        return VanillaSurvival;
    }

    if lower.contains("survival") || lower.contains("vanilla") || lower.contains("smp") {
        return VanillaSurvival;
    }

    VanillaSurvival
}

pub fn generate_tags_info(info: &super::ServerInfo) -> Vec<String> {
    generate_tags(&info.category, &info.version, info.online_players, info.modded, &info.motd)
}

fn generate_tags(cat: &ServerCategory, version: &str, players: i32, modded: bool, motd: &str) -> Vec<String> {
    let mut tags = Vec::new();
    tags.push(cat.as_str().to_string());

    if players <= 5 {
        tags.push("small".to_string());
    } else if players <= 20 {
        tags.push("medium".to_string());
    } else {
        tags.push("large".to_string());
    }

    let lower_motd = motd.to_lowercase();
    if lower_motd.contains("survival") { tags.push("survival".to_string()); }
    if lower_motd.contains("pvp") { tags.push("pvp".to_string()); }
    if lower_motd.contains("economy") || lower_motd.contains("shop") { tags.push("economy".to_string()); }
    if lower_motd.contains("rpg") || lower_motd.contains("mmo") || lower_motd.contains("dungeon") { tags.push("rpg".to_string()); }
    if lower_motd.contains("crossplay") || lower_motd.contains("bedrock") { tags.push("crossplay".to_string()); }
    if lower_motd.contains("lobby") || lower_motd.contains("hub") { tags.push("lobby".to_string()); }
    if lower_motd.contains("1.21") || version.contains("1.21") { tags.push("1.21".to_string()); }
    if lower_motd.contains("1.20") || version.contains("1.20") { tags.push("1.20".to_string()); }
    if lower_motd.contains("1.8") || version.contains("1.8") { tags.push("1.8".to_string()); }
    if modded { tags.push("modded".to_string()); }

    tags
}
