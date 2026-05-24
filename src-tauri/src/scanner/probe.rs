use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::{timeout, Duration};
use crate::scanner::ServerInfo;
use crate::proxy;

const PROBE_TIMEOUT: Duration = Duration::from_secs(8);

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
    let mut packet = Vec::new();
    packet.push(0x00);
    write_varint(&mut packet, username.len() as i32);
    packet.extend_from_slice(username.as_bytes());

    let mut frame = Vec::new();
    write_varint(&mut frame, packet.len() as i32);
    frame.extend_from_slice(&packet);
    frame
}

fn build_handshake(host: &str, port: u16) -> Vec<u8> {
    let mut packet = Vec::new();
    write_varint(&mut packet, 767);
    write_varint(&mut packet, host.len() as i32);
    packet.extend_from_slice(host.as_bytes());
    packet.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut packet, 2);

    let mut frame = Vec::new();
    write_varint(&mut frame, 0x00);
    write_varint(&mut frame, packet.len() as i32);
    frame.extend_from_slice(&packet);

    let mut final_packet = Vec::new();
    write_varint(&mut final_packet, frame.len() as i32);
    final_packet.extend_from_slice(&frame);
    final_packet
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

    if timeout(PROBE_TIMEOUT, writer.write_all(&send_buf))
        .await
        .ok()?
        .is_err()
    {
        return None;
    }

    let mut response = Vec::new();
    let read_result = timeout(PROBE_TIMEOUT / 2, reader.read_to_end(&mut response)).await;

    match read_result {
        Ok(Ok(_)) => {
            if response.is_empty() {
                return None;
            }
            parse_login_response(&response)
        }
        Ok(Err(_)) => None,
        Err(_) => {
            Some(false)
        }
    }
}

fn parse_login_response(data: &[u8]) -> Option<bool> {
    let mut slice = data;
    let (_, _) = super::ping::read_varint(&mut slice)?;
    let (packet_id, _) = super::ping::read_varint(&mut slice)?;

    match packet_id {
        0x00 => {
            let (json_len, _) = super::ping::read_varint(&mut slice)?;
            let json_str = std::str::from_utf8(&slice[..json_len as usize]).ok()?;
            let parsed: serde_json::Value = serde_json::from_str(json_str).ok()?;
            let reason = parsed.get("text").and_then(|t| t.as_str()).unwrap_or("");

            if reason.to_lowercase().contains("whitelist") {
                return Some(true);
            }
            Some(false)
        }
        0x01 => {
            Some(false)
        }
        0x02 => {
            Some(false)
        }
        _ => None,
    }
}
