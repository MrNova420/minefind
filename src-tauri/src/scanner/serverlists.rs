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
/// Uses regex to find every valid IPv4:port combination in the page source.
pub async fn fetch_serverlist(url: &str) -> Result<Vec<(String, u16)>, String> {
    let output = tokio::process::Command::new("curl")
        .args(["-s", "-L", "--max-time", "30",
              "-H", "User-Agent: Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36",
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
    let re = Regex::new(r"\b(\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}):(\d{1,5})\b")
        .map_err(|e| format!("regex: {}", e))?;

    let mut seen = HashSet::new();
    let mut results = Vec::new();

    for cap in re.captures_iter(html) {
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

    Ok(results)
}

/// Seed from known servers (kept for backwards compat but NOT recommended)
pub fn seed_from_existing(db_path: &str) -> Result<Vec<(String, u16)>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT ip, port FROM servers ORDER BY last_seen DESC LIMIT 500")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16))
    }).map_err(|e| e.to_string())?;
    Ok(rows.filter_map(|r| r.ok()).collect())
}
