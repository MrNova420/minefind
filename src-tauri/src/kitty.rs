use axum::{extract::State, Json};
use crate::scanner;
use rusqlite::{Connection, Result as SqlResult, params};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;
use tokio::sync::Semaphore;

const KITTY_URL: &str = "https://raw.githubusercontent.com/LillySchramm/KittyScanBlocklist/main/ips.txt";

pub struct KittyDatabase {
    conn: Mutex<Connection>,
}

impl KittyDatabase {
    pub fn new(path: &str) -> SqlResult<Self> {
        let conn = Connection::open(path)?;
        let db = KittyDatabase { conn: Mutex::new(conn) };
        db.init()?;
        Ok(db)
    }

    fn init(&self) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS kitty_ips (
                ip TEXT PRIMARY KEY,
                verified INTEGER NOT NULL DEFAULT 0,
                motd TEXT,
                version TEXT,
                online_players INTEGER DEFAULT 0,
                max_players INTEGER DEFAULT 0,
                ping_ms INTEGER DEFAULT 0,
                first_seen TEXT NOT NULL,
                last_seen TEXT NOT NULL
            );"
        )?;
        Ok(())
    }

    pub fn upsert(&self, ip: &str, verified: bool, motd: Option<&str>, version: Option<&str>,
                  online_players: Option<i32>, max_players: Option<i32>, ping_ms: Option<i64>) -> SqlResult<()> {
        let conn = self.conn.lock().unwrap();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO kitty_ips (ip, verified, motd, version, online_players, max_players, ping_ms, first_seen, last_seen)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(ip) DO UPDATE SET
             verified=excluded.verified, motd=COALESCE(excluded.motd, kitty_ips.motd),
             version=COALESCE(excluded.version, kitty_ips.version),
             online_players=COALESCE(excluded.online_players, kitty_ips.online_players),
             max_players=COALESCE(excluded.max_players, kitty_ips.max_players),
             ping_ms=COALESCE(excluded.ping_ms, kitty_ips.ping_ms),
             last_seen=excluded.last_seen",
            params![ip, verified as i32, motd, version, online_players, max_players, ping_ms, now, now],
        )?;
        Ok(())
    }

    pub fn sync_ips(&self, ips: &[String]) -> SqlResult<(i64, i64)> {
        let conn = self.conn.lock().unwrap();
        let mut new_count: i64 = 0;
        let mut removed_count: i64 = 0;

        let existing: std::collections::HashSet<String> = {
            let mut stmt = conn.prepare("SELECT ip FROM kitty_ips")?;
            let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
            rows.filter_map(|r| r.ok()).collect()
        };

        let incoming: std::collections::HashSet<String> = ips.iter().cloned().collect();
        let now = chrono::Utc::now().to_rfc3339();

        for ip in &incoming {
            if !existing.contains(ip.as_str()) {
                conn.execute(
                    "INSERT INTO kitty_ips (ip, verified, ping_ms, first_seen, last_seen) VALUES (?1, 0, NULL, ?2, ?3)",
                    params![ip, now, now],
                )?;
                new_count += 1;
            }
        }

        for ip in &existing {
            if !incoming.contains(ip.as_str()) {
                conn.execute("DELETE FROM kitty_ips WHERE ip = ?1", params![ip])?;
                removed_count += 1;
            }
        }

        Ok((new_count, removed_count))
    }

    pub fn get_all(&self) -> SqlResult<Vec<serde_json::Value>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT ip, verified, motd, version, online_players, max_players, ping_ms, first_seen, last_seen
             FROM kitty_ips ORDER BY verified DESC, online_players DESC"
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(serde_json::json!({
                "ip": row.get::<_, String>(0)?,
                "verified": row.get::<_, i32>(1)? != 0,
                "motd": row.get::<_, Option<String>>(2)?,
                "version": row.get::<_, Option<String>>(3)?,
                "online_players": row.get::<_, i32>(4)?,
                "max_players": row.get::<_, i32>(5)?,
                "ping_ms": row.get::<_, Option<i64>>(6)?,
                "first_seen": row.get::<_, String>(7)?,
                "last_seen": row.get::<_, String>(8)?,
            }))
        })?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(v) = r { result.push(v); }
        }
        Ok(result)
    }

    pub fn stats(&self) -> SqlResult<serde_json::Value> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row("SELECT COUNT(*) FROM kitty_ips", [], |row| row.get(0))?;
        let verified: i64 = conn.query_row("SELECT COUNT(*) FROM kitty_ips WHERE verified=1", [], |row| row.get(0))?;
        let online: i64 = conn.query_row("SELECT COUNT(*) FROM kitty_ips WHERE online_players > 0", [], |row| row.get(0))?;
        let last_sync: Option<String> = conn.query_row(
            "SELECT MAX(last_seen) FROM kitty_ips", [], |row| row.get(0)
        ).ok();
        Ok(serde_json::json!({
            "total": total, "verified": verified, "online": online, "last_sync": last_sync,
        }))
    }

    pub fn get_unverified(&self) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT ip FROM kitty_ips WHERE verified=0 AND (ping_ms IS NULL OR ping_ms = 0)")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(ip) = r { result.push(ip); }
        }
        Ok(result)
    }

    pub fn count(&self) -> SqlResult<i64> {
        let conn = self.conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM kitty_ips", [], |row| row.get(0))
    }

    pub fn get_all_ips(&self) -> SqlResult<Vec<String>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT ip FROM kitty_ips")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut result = Vec::new();
        for r in rows {
            if let Ok(ip) = r { result.push(ip); }
        }
        Ok(result)
    }
}

pub struct KittyCtx {
    pub sync_running: AtomicBool,
    pub verify_running: AtomicBool,
    pub verify_total: AtomicU64,
    pub verify_done: AtomicU64,
    pub verify_found: AtomicU64,
    pub last_sync: std::sync::Mutex<Option<String>>,
}

impl KittyCtx {
    pub fn new() -> Self {
        KittyCtx {
            sync_running: AtomicBool::new(false),
            verify_running: AtomicBool::new(false),
            verify_total: AtomicU64::new(0),
            verify_done: AtomicU64::new(0),
            verify_found: AtomicU64::new(0),
            last_sync: std::sync::Mutex::new(None),
        }
    }
}

pub type AppState = Arc<crate::AppCtx>;

fn with_kitty_db<F, R>(ctx: &crate::AppCtx, f: F) -> Result<R, String>
where
    F: FnOnce(&KittyDatabase) -> Result<R, String>,
{
    let guard = ctx.kitty_db.lock().map_err(|e| format!("lock: {}", e))?;
    match guard.as_ref() {
        Some(db) => f(db),
        None => Err("Kitty DB not ready".into()),
    }
}

async fn fetch_ips() -> Result<Vec<String>, String> {
    let output = tokio::process::Command::new("curl")
        .arg("-s")
        .arg("--max-time")
        .arg("30")
        .arg(KITTY_URL)
        .output()
        .await
        .map_err(|e| format!("curl exec: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", stderr));
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let ips: Vec<String> = body
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();

    if ips.is_empty() {
        return Err("empty response from GitHub".into());
    }

    Ok(ips)
}

async fn verify_one_ip(ctx: &crate::AppCtx, ip: &str) {
    if let Ok(info) = scanner::ping::ping_server(ip, 25565).await {
        ctx.kitty_ctx.verify_found.fetch_add(1, Ordering::SeqCst);
        let _ = with_kitty_db(ctx, |db| {
            db.upsert(ip, true, Some(&info.motd), Some(&info.version),
                Some(info.online_players), Some(info.max_players), Some(info.ping_ms))
                .map_err(|e| e.to_string())
        });
        return;
    }
    if let Ok(info) = scanner::ping::ping_server(ip, 19132).await {
        ctx.kitty_ctx.verify_found.fetch_add(1, Ordering::SeqCst);
        let _ = with_kitty_db(ctx, |db| {
            db.upsert(ip, true, Some(&info.motd), Some(&info.version),
                Some(info.online_players), Some(info.max_players), Some(info.ping_ms))
                .map_err(|e| e.to_string())
        });
        return;
    }
    let _ = with_kitty_db(ctx, |db| {
        db.upsert(ip, false, None, None, None, None, Some(-1))
            .map_err(|e| e.to_string())
    });
}

pub async fn api_kitty_sync(State(ctx): State<AppState>) -> Json<serde_json::Value> {
    let kc = &ctx.kitty_ctx;
    if kc.sync_running.load(Ordering::SeqCst) {
        return Json(serde_json::json!({"error": "sync already running"}));
    }
    kc.sync_running.store(true, Ordering::SeqCst);

    let ips = match fetch_ips().await {
        Ok(ips) => ips,
        Err(e) => {
            kc.sync_running.store(false, Ordering::SeqCst);
            return Json(serde_json::json!({"error": e}));
        }
    };

    let result = with_kitty_db(&ctx, |db| db.sync_ips(&ips).map_err(|e| e.to_string()));

    match result {
        Ok((added, removed)) => {
            let now = chrono::Utc::now().to_rfc3339();
            *kc.last_sync.lock().unwrap() = Some(now.clone());
            kc.sync_running.store(false, Ordering::SeqCst);
            let total = with_kitty_db(&ctx, |db| db.count().map_err(|e| e.to_string())).unwrap_or(0);
            Json(serde_json::json!({
                "ok": true, "total_ips": ips.len(), "added": added,
                "removed": removed, "db_total": total, "synced_at": now
            }))
        }
        Err(e) => {
            kc.sync_running.store(false, Ordering::SeqCst);
            Json(serde_json::json!({"error": e}))
        }
    }
}

pub async fn api_kitty_verify(State(ctx): State<AppState>) -> Json<serde_json::Value> {
    let kc = &ctx.kitty_ctx;
    if kc.verify_running.load(Ordering::SeqCst) {
        return Json(serde_json::json!({"error": "verify already running"}));
    }
    kc.verify_running.store(true, Ordering::SeqCst);

    let ctx_clone = ctx.clone();
    tokio::spawn(async move {
        let kc = &ctx_clone.kitty_ctx;
        kc.verify_total.store(0, Ordering::SeqCst);
        kc.verify_done.store(0, Ordering::SeqCst);
        kc.verify_found.store(0, Ordering::SeqCst);

        let all_ips = match with_kitty_db(&ctx_clone, |db| db.get_all_ips().map_err(|e| e.to_string())) {
            Ok(ips) => ips,
            Err(_) => {
                kc.verify_running.store(false, Ordering::SeqCst);
                return;
            }
        };

        let total = all_ips.len();
        if total == 0 {
            kc.verify_running.store(false, Ordering::SeqCst);
            return;
        }

        kc.verify_total.store(total as u64, Ordering::SeqCst);

        log::info!("Kitty verify: pinging {} IPs", total);
        let sem = Arc::new(Semaphore::new(200));

        for ip in &all_ips {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let ctx2 = ctx_clone.clone();
            let ip2 = ip.clone();
            tokio::spawn(async move {
                let _held = permit;
                verify_one_ip(&ctx2, &ip2).await;
                ctx2.kitty_ctx.verify_done.fetch_add(1, Ordering::SeqCst);
            });
        }

        // Wait for all spawned tasks to finish
        while kc.verify_done.load(Ordering::SeqCst) < kc.verify_total.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        kc.verify_running.store(false, Ordering::SeqCst);

        let verified = with_kitty_db(&ctx_clone, |db| {
            let s = db.stats().map_err(|e| e.to_string())?;
            Ok(s.get("verified").and_then(|v| v.as_i64()).unwrap_or(0))
        }).unwrap_or(0);

        let found = kc.verify_found.load(Ordering::SeqCst);
        log::info!("Kitty verify done: {} verified ({} found online)", verified, found);
    });

    Json(serde_json::json!({"ok": true, "message": "verify started in background", "total": 0}))
}

pub async fn api_kitty_list(State(ctx): State<AppState>) -> Json<serde_json::Value> {
    let guard = match ctx.kitty_db.lock() {
        Ok(g) => g,
        Err(_) => return Json(serde_json::json!({"error": "lock error"})),
    };
    let db = match guard.as_ref() {
        Some(d) => d,
        None => return Json(serde_json::json!({"error": "Kitty DB not ready"})),
    };
    match db.get_all() {
        Ok(list) => Json(serde_json::json!(list)),
        Err(e) => Json(serde_json::json!({"error": e.to_string()})),
    }
}

pub async fn api_kitty_stats(State(ctx): State<AppState>) -> Json<serde_json::Value> {
    let guard = match ctx.kitty_db.lock() {
        Ok(g) => g,
        Err(_) => return Json(serde_json::json!({"error": "lock error"})),
    };
    let db = match guard.as_ref() {
        Some(d) => d,
        None => return Json(serde_json::json!({"error": "Kitty DB not ready"})),
    };

    let mut stats = match db.stats() {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"error": e.to_string()})),
    };

    let kc = &ctx.kitty_ctx;
    if let Some(obj) = stats.as_object_mut() {
        obj.insert("syncing".into(), serde_json::json!(kc.sync_running.load(Ordering::SeqCst)));
        obj.insert("verifying".into(), serde_json::json!(kc.verify_running.load(Ordering::SeqCst)));
        obj.insert("verify_total".into(), serde_json::json!(kc.verify_total.load(Ordering::SeqCst)));
        obj.insert("verify_done".into(), serde_json::json!(kc.verify_done.load(Ordering::SeqCst)));
        obj.insert("verify_found".into(), serde_json::json!(kc.verify_found.load(Ordering::SeqCst)));
        obj.insert("last_sync".into(), serde_json::to_value(kc.last_sync.lock().unwrap().clone()).unwrap_or(serde_json::Value::Null));
    }

    Json(stats)
}

pub async fn api_kitty_status(State(ctx): State<AppState>) -> Json<serde_json::Value> {
    let kc = &ctx.kitty_ctx;
    Json(serde_json::json!({
        "syncing": kc.sync_running.load(Ordering::SeqCst),
        "verifying": kc.verify_running.load(Ordering::SeqCst),
        "verify_total": kc.verify_total.load(Ordering::SeqCst),
        "verify_done": kc.verify_done.load(Ordering::SeqCst),
        "verify_found": kc.verify_found.load(Ordering::SeqCst),
        "last_sync": *kc.last_sync.lock().unwrap(),
    }))
}
