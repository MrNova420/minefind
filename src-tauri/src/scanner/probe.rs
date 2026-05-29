use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use crate::scanner::ServerInfo;
use crate::proxy;

const PROBE_TIMEOUT: Duration = Duration::from_secs(4);

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

fn build_login_start(username: &str) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, username.len() as i32);
    body.extend_from_slice(username.as_bytes());

    let total = 1_i32 + body.len() as i32;
    let mut result = Vec::new();
    write_varint(&mut result, total);
    result.push(0x00); // packet ID 0x00 = login start
    result.extend_from_slice(&body);
    result
}

fn build_handshake(host: &str, port: u16) -> Vec<u8> {
    let mut body = Vec::new();
    write_varint(&mut body, 767);
    write_varint(&mut body, host.len() as i32);
    body.extend_from_slice(host.as_bytes());
    body.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut body, 2); // next state: login

    let total = 1_i32 + body.len() as i32;
    let mut result = Vec::new();
    write_varint(&mut result, total);
    result.push(0x00); // packet ID 0x00 = handshake
    result.extend_from_slice(&body);
    result
}

/// Probe a server to check if it's whitelisted.
/// Uses proxy if available, direct if not.
/// Returns Some(true) if whitelisted, Some(false) if not, None if inconclusive.
pub async fn check_whitelist(
    info: &ServerInfo,
    proxy_addr: Option<&str>,
) -> Option<bool> {
    let username = "MineFindProbe";
    let host = &info.ip;
    let port = info.port;

    let tcp_stream = if let Some(px) = proxy_addr {
        match timeout(PROBE_TIMEOUT, proxy::connect_through_socks5(px, host, port)).await {
            Ok(Ok(s)) => s,
            _ => return None,
        }
    } else {
        match timeout(PROBE_TIMEOUT, TcpStream::connect((host.as_str(), port))).await {
            Ok(Ok(s)) => s,
            _ => return None,
        }
    };

    let _ = tcp_stream.set_nodelay(true);
    let (mut reader, mut writer) = tcp_stream.into_split();

    let handshake = build_handshake(host, port);
    let login = build_login_start(username);

    let mut send_buf = Vec::new();
    send_buf.extend_from_slice(&handshake);
    send_buf.extend_from_slice(&login);

    if timeout(PROBE_TIMEOUT, writer.write_all(&send_buf)).await.ok().is_none() {
        return None;
    }

    // Read packet length using VarInt (same as ping)
    let mut len_raw = Vec::new();
    loop {
        let mut byte = [0u8; 1];
        match timeout(PROBE_TIMEOUT, reader.read_exact(&mut byte)).await {
            Ok(Ok(_)) => {}
            _ => {
                // Can't read — server might have accepted but kept connection open
                // or disconnected without sending packet
                return Some(false);
            }
        }
        len_raw.push(byte[0]);
        if byte[0] & 0x80 == 0 { break; }
        if len_raw.len() > 5 { return None; }
    }

    let mut len_slice = &len_raw[..];
    let (packet_len, _) = super::ping::read_varint(&mut len_slice)?;

    if packet_len <= 0 || packet_len > 262144 { return None; }

    let mut packet_data = vec![0u8; packet_len as usize];
    if timeout(PROBE_TIMEOUT, reader.read_exact(&mut packet_data)).await.is_err() {
        // Timed out reading packet — server accepted login, kept connection open
        return Some(false);
    }

    let mut response = Vec::with_capacity(len_raw.len() + packet_data.len());
    response.extend_from_slice(&len_raw);
    response.extend_from_slice(&packet_data);

    parse_login_response(&response)
}

fn parse_login_response(data: &[u8]) -> Option<bool> {
    let mut slice = data;
    let (_, _) = super::ping::read_varint(&mut slice)?;
    let (packet_id, _) = super::ping::read_varint(&mut slice)?;

    match packet_id {
        0x00 => {
            // Disconnect packet — server rejected us
            let (json_len, _) = super::ping::read_varint(&mut slice)?;
            let json_str = std::str::from_utf8(&slice[..json_len as usize]).ok()?;
            let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let reason = parsed.get("text").and_then(|t| t.as_str()).unwrap_or("");

            if reason.to_lowercase().contains("whitelist") {
                return Some(true); // whitelisted
            }
            Some(false) // rejected for other reason (full, banned, etc.)
        }
        0x01 => {
            // Encryption Request — server is online-mode
            // We can't authenticate, so we don't know if it's whitelisted
            None
        }
        0x02 => {
            // Login Success — we got in! Server is open
            Some(false)
        }
        _ => None,
    }
}
