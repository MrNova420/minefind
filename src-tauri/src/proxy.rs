use tokio::net::TcpStream;
use tokio_socks::tcp::socks5::Socks5Stream;

pub async fn connect_through_socks5(
    proxy_addr: &str,
    target_host: &str,
    target_port: u16,
) -> Result<TcpStream, String> {
    let parts: Vec<&str> = proxy_addr.split(':').collect();
    if parts.len() != 2 {
        return Err(format!("invalid proxy: {}", proxy_addr));
    }
    let stream = Socks5Stream::connect(
        (parts[0], parts[1].parse().map_err(|e| format!("bad port: {}", e))?),
        (target_host, target_port),
    )
    .await
    .map_err(|e| format!("SOCKS5 error: {}", e))?;

    let inner = stream.into_inner();
    let _ = inner.set_nodelay(true);
    Ok(inner)
}
