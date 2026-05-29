use std::time::Duration;
use std::sync::Arc;
use tokio::net::UdpSocket;
use tokio::time::timeout;

const BEDROCK_TIMEOUT: Duration = Duration::from_secs(2);

// RakNet unconnected ping magic bytes
const MAGIC: [u8; 16] = [
    0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE,
    0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78,
];

#[derive(Debug, Clone)]
pub struct BedrockInfo {
    pub motd: String,
    pub version: String,
    pub protocol: i32,
    pub online_players: i32,
    pub max_players: i32,
    pub game_mode: String,
    pub edition: String,
    pub sub_motd: String,
    pub ping_ms: i64,
}

pub async fn ping_bedrock(ip: &str, port: u16) -> Result<BedrockInfo, String> {
    ping_bedrock_inner(ip, port, None).await
}

pub async fn ping_bedrock_with_sem(ip: &str, port: u16, cs: Option<Arc<tokio::sync::Semaphore>>) -> Result<BedrockInfo, String> {
    ping_bedrock_inner(ip, port, cs).await
}

async fn ping_bedrock_inner(ip: &str, port: u16, conn_sem: Option<Arc<tokio::sync::Semaphore>>) -> Result<BedrockInfo, String> {
    let start = std::time::Instant::now();
    let addr = format!("{}:{}", ip, port);
    let _cp = match conn_sem {
        Some(ref s) => Some(s.clone().acquire_owned().await.unwrap()),
        None => None,
    };

    let socket = timeout(BEDROCK_TIMEOUT, UdpSocket::bind("0.0.0.0:0"))
        .await
        .map_err(|_| "failed to bind UDP socket".to_string())?
        .map_err(|e| format!("socket bind error: {}", e))?;

    let _ = socket.connect(&addr).await.map_err(|e| format!("connect error: {}", e))?;

    // Build unconnected ping packet
    let mut packet = Vec::new();
    packet.push(0x01); // packet ID: unconnected ping
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;
    packet.extend_from_slice(&ts.to_be_bytes());
    packet.extend_from_slice(&MAGIC);
    packet.extend_from_slice(&0u64.to_be_bytes()); // client GUID

    timeout(BEDROCK_TIMEOUT, socket.send(&packet))
        .await
        .map_err(|_| format!("timeout sending bedrock ping to {}", addr))?
        .map_err(|e| format!("send error to {}: {}", addr, e))?;

    let mut buf = [0u8; 4096];
    let len = timeout(BEDROCK_TIMEOUT, socket.recv(&mut buf))
        .await
        .map_err(|_| format!("timeout reading bedrock response from {}", addr))?
        .map_err(|e| format!("recv error from {}: {}", addr, e))?;

    if len == 0 {
        return Err(format!("empty response from {}", addr));
    }

    let ping_ms = start.elapsed().as_millis() as i64;
    parse_bedrock_response(&buf[..len], ping_ms)
}

fn parse_bedrock_response(data: &[u8], ping_ms: i64) -> Result<BedrockInfo, String> {
    if data.is_empty() || data[0] != 0x1C {
        return Err(format!("unexpected packet ID: {}", data.first().unwrap_or(&0)));
    }

    let mut pos = 1;
    // Skip timestamp (8 bytes)
    if pos + 8 > data.len() { return Err("response too short".into()); }
    pos += 8;
    // Skip server GUID (8 bytes)
    if pos + 8 > data.len() { return Err("response too short".into()); }
    pos += 8;
    // Skip magic (16 bytes)
    if pos + 16 > data.len() { return Err("response too short".into()); }
    pos += 16;

    // Read server ID string (2 bytes length BE + data)
    if pos + 2 > data.len() { return Err("response too short".into()); }
    let server_id_len = u16::from_be_bytes([data[pos], data[pos+1]]) as usize;
    pos += 2;
    if pos + server_id_len > data.len() { return Err("response too short".into()); }
    let server_id = String::from_utf8_lossy(&data[pos..pos+server_id_len]).to_string();

    // Split server ID: "MCPE;MOTD;PROTO;VERSION;PLAYERS;MAX;GUID;GAMEMODE;..."
    let parts: Vec<&str> = server_id.split(';').collect();
    if parts.len() < 8 {
        return Err(format!("invalid server ID format: {} parts", parts.len()));
    }

    Ok(BedrockInfo {
        motd: parts.get(1).unwrap_or(&"").to_string(),
        version: parts.get(3).unwrap_or(&"").to_string(),
        protocol: parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0),
        online_players: parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(0),
        max_players: parts.get(5).and_then(|s| s.parse().ok()).unwrap_or(0),
        game_mode: parts.get(7).unwrap_or(&"").to_string(),
        ping_ms,
        edition: if server_id.contains("MCEE") { "Education".into() } else { "Bedrock".into() },
        sub_motd: parts.get(8).unwrap_or(&"").to_string(),
    })
}
