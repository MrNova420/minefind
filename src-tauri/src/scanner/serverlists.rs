use std::collections::HashSet;
use std::sync::Mutex;
use rusqlite::{Connection, Result as SqlResult, params};

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

/// Fetch server list from a public API and extract IP:port pairs.
/// Popular Minecraft server list APIs that return JSON:
/// - minecraft-server-list.com has an API endpoint
/// - Some are rate-limited, so use sparingly
pub async fn fetch_serverlist(url: &str) -> Result<Vec<(String, u16)>, String> {
    // Try using curl to fetch the list
    let output = tokio::process::Command::new("curl")
        .args(["-s", "--max-time", "30", "-L", url])
        .output()
        .await
        .map_err(|e| format!("curl: {}", e))?;

    if !output.status.success() {
        return Err("curl failed".into());
    }

    let body = String::from_utf8_lossy(&output.stdout);
    parse_serverlist_json(&body)
}

fn parse_serverlist_json(body: &str) -> Result<Vec<(String, u16)>, String> {
    let parsed: Vec<serde_json::Value> = serde_json::from_str(body).unwrap_or_default();
    let mut results = Vec::new();

    for entry in &parsed {
        let ip = entry.get("ip").and_then(|v| v.as_str()).unwrap_or("");
        let port = entry.get("port").and_then(|v| v.as_u64()).unwrap_or(25565) as u16;
        if !ip.is_empty() && port > 0 {
            results.push((ip.to_string(), port));
        }
    }

    // If JSON parse failed, try line-by-line (some APIs return plain text)
    if results.is_empty() {
        for line in body.lines() {
            let trimmed = line.trim();
            if let Some((ip, port_str)) = trimmed.split_once(':') {
                if let Ok(port) = port_str.parse::<u16>() {
                    results.push((ip.to_string(), port));
                }
            }
        }
    }

    Ok(results)
}

/// Seed from known server IPs (bootstrap discovery)
pub fn seed_from_existing(db_path: &str) -> Result<Vec<(String, u16)>, String> {
    let conn = Connection::open(db_path).map_err(|e| e.to_string())?;
    let mut stmt = conn.prepare("SELECT ip, port FROM servers ORDER BY last_seen DESC LIMIT 500")
        .map_err(|e| e.to_string())?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)? as u16))
    }).map_err(|e| e.to_string())?;

    Ok(rows.filter_map(|r| r.ok()).collect())
}
