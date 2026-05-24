use rusqlite::{Connection, Result, params};
use crate::scanner::ServerInfo;
use std::sync::Mutex;

pub struct Database {
    conn: Mutex<Connection>,
}

pub struct Checkpoint {
    pub cycle_type: String,
    pub cycle_num: u64,
    pub ip_u32: u64,
    pub scanned_ips: u64,
    pub found_servers: u64,
}

#[derive(Clone)]
pub struct RangeDensity {
    pub ip_prefix: String,
    pub servers_found: i64,
    pub cycles_scanned: i64,
}

impl Database {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Database { conn: Mutex::new(conn) };
        db.init_tables()?;
        Ok(db)
    }

    fn init_tables(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS servers (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                ip TEXT NOT NULL,
                port INTEGER NOT NULL DEFAULT 25565,
                motd TEXT,
                protocol INTEGER,
                version TEXT,
                online_players INTEGER DEFAULT 0,
                max_players INTEGER DEFAULT 0,
                ping_ms INTEGER DEFAULT 0,
                modded INTEGER DEFAULT 0,
                mod_list TEXT,
                whitelisted INTEGER,
                category TEXT,
                tags TEXT,
                player_sample TEXT,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL,
                UNIQUE(ip, port)
            );
            CREATE INDEX IF NOT EXISTS idx_servers_cat ON servers(category);
            CREATE INDEX IF NOT EXISTS idx_servers_ver ON servers(version);
            CREATE INDEX IF NOT EXISTS idx_servers_wl ON servers(whitelisted);
            CREATE INDEX IF NOT EXISTS idx_servers_ip_port ON servers(ip, port);
            CREATE TABLE IF NOT EXISTS scan_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                started_at TEXT NOT NULL,
                finished_at TEXT,
                targets_scanned INTEGER DEFAULT 0,
                servers_found INTEGER DEFAULT 0,
                ranges TEXT,
                cycle_type TEXT DEFAULT ''
            );
            CREATE TABLE IF NOT EXISTS scanner_checkpoint (
                id INTEGER PRIMARY KEY CHECK (id = 1),
                cycle_type TEXT NOT NULL,
                cycle_num INTEGER NOT NULL DEFAULT 1,
                ip_u32 INTEGER NOT NULL DEFAULT 0,
                scanned_ips INTEGER NOT NULL DEFAULT 0,
                found_servers INTEGER NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS range_density (
                ip_prefix TEXT PRIMARY KEY,
                servers_found INTEGER DEFAULT 0,
                cycles_scanned INTEGER DEFAULT 0,
                last_scanned_at TEXT
            );
            CREATE TABLE IF NOT EXISTS activity_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                server_id INTEGER REFERENCES servers(id),
                recorded_at TEXT NOT NULL,
                online_players INTEGER,
                ping_ms INTEGER
            );"
        )?;
        let _ = conn.execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_rd_prefix ON range_density(ip_prefix);
             CREATE INDEX IF NOT EXISTS idx_rd_found ON range_density(servers_found);
             ALTER TABLE scan_history ADD COLUMN cycle_type TEXT DEFAULT '';"
        );
        let _ = conn.execute("ALTER TABLE range_density ADD COLUMN last_cycle_type TEXT DEFAULT ''", []);
        Ok(())
    }

    pub fn upsert_server(&self, info: &ServerInfo) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO servers (ip, port, motd, protocol, version, online_players, max_players,
             ping_ms, modded, mod_list, whitelisted, category, tags, player_sample, first_seen, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)
             ON CONFLICT(ip, port) DO UPDATE SET
             motd=excluded.motd, protocol=excluded.protocol, version=excluded.version,
             online_players=excluded.online_players, max_players=excluded.max_players,
             ping_ms=excluded.ping_ms, modded=excluded.modded, mod_list=excluded.mod_list,
             whitelisted=COALESCE(excluded.whitelisted, servers.whitelisted),
             category=excluded.category, tags=excluded.tags,
             player_sample=excluded.player_sample, last_seen=excluded.last_seen",
            params![
                info.ip, info.port, info.motd, info.protocol, info.version,
                info.online_players, info.max_players, info.ping_ms,
                info.modded as i32,
                serde_json::to_string(&info.mod_list).unwrap_or_default(),
                info.whitelisted, info.category.as_str(),
                serde_json::to_string(&info.tags).unwrap_or_default(),
                serde_json::to_string(&info.player_sample).unwrap_or_default(),
                info.first_seen, info.last_seen,
            ],
        )?;
        Ok(())
    }

    pub fn get_all_servers(&self) -> Result<Vec<ServerInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ip, port, motd, protocol, version, online_players, max_players,
             ping_ms, modded, mod_list, whitelisted, category, tags, player_sample, first_seen, last_seen
             FROM servers ORDER BY online_players DESC"
        )?;

        let servers = stmt.query_map([], |row| {
            Ok(ServerInfo {
                ip: row.get(0)?,
                port: row.get(1)?,
                motd: row.get(2)?,
                protocol: row.get(3)?,
                version: row.get(4)?,
                online_players: row.get(5)?,
                max_players: row.get(6)?,
                ping_ms: row.get(7)?,
                modded: row.get::<_, i32>(8)? != 0,
                mod_list: serde_json::from_str(&row.get::<_, String>(9)?).unwrap_or_default(),
                whitelisted: row.get(10)?,
                category: crate::scanner::ServerCategory::from_str(&row.get::<_, String>(11)?),
                tags: serde_json::from_str(&row.get::<_, String>(12)?).unwrap_or_default(),
                player_sample: serde_json::from_str(&row.get::<_, String>(13)?).unwrap_or_default(),
                first_seen: row.get(14)?,
                last_seen: row.get(15)?,
            })
        })?;

        let mut result = Vec::new();
        for s in servers {
            if let Ok(s) = s { result.push(s); }
        }
        Ok(result)
    }

    pub fn get_server_count(&self) -> Result<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM servers", [], |row| row.get(0))
    }

    pub fn record_cycle(&self, cycle_type: &str, _cycle: u64, scanned: u64, found: u64, started_at: &str, _duration_secs: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO scan_history (started_at, finished_at, targets_scanned, servers_found, cycle_type)
             VALUES (?4, datetime('now'), ?1, ?2, ?3)",
            params![scanned, found, cycle_type, started_at],
        )?;
        Ok(())
    }

    pub fn get_cycle_count(&self) -> Result<u64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM scan_history", [], |row| row.get(0))
    }

    pub fn get_cycle_summary(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let count: u64 = conn.query_row("SELECT COUNT(*) FROM scan_history", [], |row| row.get(0))?;
        let total_servers: u64 = conn.query_row("SELECT COALESCE(SUM(servers_found), 0) FROM scan_history", [], |row| row.get(0))?;
        let total_scanned: u64 = conn.query_row("SELECT COALESCE(SUM(targets_scanned), 0) FROM scan_history", [], |row| row.get(0))?;
        let actual_servers: i64 = conn.query_row("SELECT COUNT(*) FROM servers", [], |row| row.get(0))?;
        Ok(serde_json::json!({
            "cycles": count,
            "total_servers_found": total_servers,
            "total_targets_scanned": total_scanned,
            "actual_servers": actual_servers,
        }))
    }

    pub fn get_cycle_history(&self) -> Result<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT rowid, started_at, finished_at, targets_scanned, servers_found, cycle_type
             FROM scan_history ORDER BY rowid DESC LIMIT 50"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "cycle": row.get::<_, i64>(0)?,
                "started_at": row.get::<_, String>(1)?,
                "finished_at": row.get::<_, String>(2)?,
                "targets_scanned": row.get::<_, i64>(3)?,
                "servers_found": row.get::<_, i64>(4)?,
                "cycle_type": row.get::<_, String>(5).unwrap_or_default(),
            }))
        })?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(v) = r { result.push(v); }
        }
        Ok(result)
    }

    // --- Checkpoint ---

    pub fn save_checkpoint(&self, cycle_type: &str, cycle_num: u64, ip_u32: u64, scanned: u64, found: u64) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO scanner_checkpoint (id, cycle_type, cycle_num, ip_u32, scanned_ips, found_servers, updated_at)
             VALUES (1, ?1, ?2, ?3, ?4, ?5, datetime('now'))",
            params![cycle_type, cycle_num, ip_u32 as i64, scanned as i64, found as i64],
        )?;
        Ok(())
    }

    pub fn load_checkpoint(&self) -> Result<Option<Checkpoint>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT cycle_type, cycle_num, ip_u32, scanned_ips, found_servers FROM scanner_checkpoint WHERE id = 1"
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            Ok(Some(Checkpoint {
                cycle_type: row.get(0)?,
                cycle_num: row.get(1)?,
                ip_u32: row.get::<_, i64>(2)? as u64,
                scanned_ips: row.get::<_, i64>(3)? as u64,
                found_servers: row.get::<_, i64>(4)? as u64,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn clear_checkpoint(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute("DELETE FROM scanner_checkpoint WHERE id = 1", [])?;
        Ok(())
    }

    // --- Range Density ---

    pub fn record_range_density(&self, prefix: &str, servers: i64, cycle_type: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO range_density (ip_prefix, servers_found, cycles_scanned, last_scanned_at, last_cycle_type)
             VALUES (?1, ?2, 1, datetime('now'), ?3)
             ON CONFLICT(ip_prefix) DO UPDATE SET
             servers_found = servers_found + excluded.servers_found,
             cycles_scanned = cycles_scanned + 1,
             last_scanned_at = excluded.last_scanned_at,
             last_cycle_type = excluded.last_cycle_type",
            params![prefix, servers, cycle_type],
        )?;
        Ok(())
    }

    pub fn get_dense_ranges(&self, limit: i64, skip_empty_after: i64) -> Result<Vec<RangeDensity>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ip_prefix, servers_found, cycles_scanned
             FROM range_density
             WHERE servers_found > 0 OR cycles_scanned <= ?2
             ORDER BY servers_found DESC, ip_prefix ASC
             LIMIT ?1"
        )?;
        let rows = stmt.query_map(params![limit, skip_empty_after], |row| {
            Ok(RangeDensity {
                ip_prefix: row.get(0)?,
                servers_found: row.get(1)?,
                cycles_scanned: row.get(2)?,
            })
        })?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(d) = r { result.push(d); }
        }
        Ok(result)
    }

    pub fn get_skipped_prefixes(&self, empty_after: i64) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ip_prefix FROM range_density WHERE servers_found = 0 AND cycles_scanned > ?1"
        )?;
        let rows = stmt.query_map(params![empty_after], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(p) = r { result.push(p); }
        }
        Ok(result)
    }

    pub fn get_already_scanned_prefixes(&self, cycle_type: &str) -> Result<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ip_prefix FROM range_density WHERE last_cycle_type = ?1"
        )?;
        let rows = stmt.query_map(params![cycle_type], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(p) = r { result.push(p); }
        }
        Ok(result)
    }

    pub fn reset_scanner_memory(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "DELETE FROM scan_history;
             DELETE FROM range_density;
             DELETE FROM scanner_checkpoint;"
        )?;
        Ok(())
    }

    pub fn dedup_check(&self) -> Result<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM servers", [], |r| r.get(0))?;
        let unique: i64 = conn.query_row("SELECT COUNT(DISTINCT ip || ':' || port) FROM servers", [], |r| r.get(0))?;
        let mut stmt = conn.prepare(
            "SELECT ip || ':' || port, COUNT(*) as cnt FROM servers GROUP BY ip, port HAVING cnt > 1"
        )?;
        let dupes: Vec<serde_json::Value> = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "key": row.get::<_, String>(0)?,
                "count": row.get::<_, i64>(1)?,
            }))
        })?.filter_map(|r| r.ok()).collect();
        Ok(serde_json::json!({
            "total": total,
            "unique": unique,
            "duplicates": dupes.len(),
            "duplicate_pairs": dupes,
        }))
    }
}
