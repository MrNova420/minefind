use std::collections::HashSet;
use std::sync::Mutex;
use rusqlite::{Connection, Result as SqlResult, params};
use regex::Regex;

pub struct ServerListDB {
    conn: Mutex<Connection>,
}

impl ServerListDB {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = ServerListDB { conn: Mutex::new(conn) };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS serverlist_ips (
                ip TEXT, port INTEGER DEFAULT 25565,
                source TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                PRIMARY KEY (ip, port)
            );"
        )?;
        Ok(())
    }

    pub fn merge_ips(&self, ips: &[(String, u16)], source: &str) -> SqlResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        let mut added: i64 = 0;

        let existing: HashSet<(String, u16)> = {
            let mut stmt = conn.prepare("SELECT ip, port FROM serverlist_ips")?;
            let rows = stmt.query_map([], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16))
            })?;
            rows.filter_map(|r| r.ok()).collect()
        };

        for (ip, port) in ips {
            if !existing.contains(&(ip.clone(), *port)) {
                conn.execute(
                    "INSERT OR IGNORE INTO serverlist_ips (ip, port, source, first_seen, last_seen) VALUES (?1, ?2, ?3, ?4, ?5)",
                    params![ip, *port as i64, source, now, now],
                )?;
                added += 1;
            }
        }

        // Update last_seen for existing
        for (ip, port) in ips {
            conn.execute(
                "UPDATE serverlist_ips SET last_seen = ?3 WHERE ip = ?1 AND port = ?2",
                params![ip, *port as i64, now],
            )?;
        }

        Ok((added, existing.len() as i64))
    }

    pub fn count(&self) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM serverlist_ips", [], |r| r.get(0))
    }

    pub fn get_all_with_source(&self) -> SqlResult<Vec<(String, u16, String)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT ip, port, source FROM serverlist_ips ORDER BY last_seen DESC LIMIT 5000")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16, row.get::<_, String>(2).unwrap_or_default()))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }

    #[allow(dead_code)]
    pub fn get_all(&self) -> SqlResult<Vec<(String, u16)>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT ip, port FROM serverlist_ips ORDER BY last_seen DESC")?;
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16))
        })?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }
}

/// Fetch a server list website and extract ALL ip:port pairs from the HTML.
pub async fn fetch_serverlist(url: &str) -> Result<Vec<(String, u16)>, String> {
    let agents = [
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.4.1 Safari/605.1.15",
        "Mozilla/5.0 (X11; Linux x86_64; rv:126.0) Gecko/20100101 Firefox/126.0",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/125.0.0.0 Safari/537.36 Edg/125.0.0.0",
    ];
    let ua = agents[url.len() % agents.len()];

    let output = tokio::process::Command::new("curl")
        .args(["-s", "-L", "--max-time", "30", "--compressed",
              "-H", &format!("User-Agent: {}", ua),
              "-H", "Accept: text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
              "-H", "Accept-Language: en-US,en;q=0.5"])
        .arg(url)
        .output()
        .await
        .map_err(|e| format!("curl: {}", e))?;

    if !output.status.success() {
        return Err("curl failed".into());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    extract_ips_from_html(&body)
}

/// Extract ALL IPv4:port pairs from raw HTML using regex.
/// No filtering — every valid IP:port found goes in.
fn extract_ips_from_html(html: &str) -> Result<Vec<(String, u16)>, String> {
    let re_ip_port = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})[:](\d{1,5})\b")
        .map_err(|e| format!("regex: {}", e))?;
    let re_ip_alone = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3})\b")
        .map_err(|e| format!("regex: {}", e))?;

    let mut seen = HashSet::new();
    let mut results = Vec::new();

    // First pass: ip:port pairs
    for cap in re_ip_port.captures_iter(html) {
        let ip = cap.get(1).unwrap().as_str();
        let port_str = cap.get(2).unwrap().as_str();
        if let Ok(port) = port_str.parse::<u16>() {
            if port > 0 {
                let key = format!("{}:{}", ip, port);
                if !seen.contains(&key) {
                    seen.insert(key);
                    results.push((ip.to_string(), port));
                }
            }
        }
    }

    // Second pass: bare IPs (use port 25565)
    let reserved: HashSet<&str> = ["0.0.0.0", "127.0.0.1", "255.255.255.255"].iter().copied().collect();
    for cap in re_ip_alone.captures_iter(html) {
        let ip = cap.get(1).unwrap().as_str();
        if reserved.contains(ip) { continue; }
        let parts: Vec<&str> = ip.split('.').collect();
        let ok = parts.iter().all(|p| {
            if let Ok(n) = p.parse::<u32>() { n <= 255 } else { false }
        });
        if ok {
            let key = format!("{}:25565", ip);
            if !seen.contains(&key) {
                seen.insert(key);
                results.push((ip.to_string(), 25565));
            }
        }
    }

    Ok(results)
}

/// Seed from known servers (kept for backwards compat but NOT recommended)
#[allow(dead_code)]
pub fn seed_from_existing(db_path: &str) -> Result<Vec<(String, u16)>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT ip, port FROM servers ORDER BY last_seen DESC LIMIT 500")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16))
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
