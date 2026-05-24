#![allow(dead_code)]

mod scanner;
mod db;
mod proxy;
mod kitty;

use axum::{extract::State, http::StatusCode, Json, routing::{get, post}, Router};
use db::Database;
use scanner::ServerInfo;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

#[derive(Clone)]
struct ScanProgress {
    cycle_type: String,
    cycle: u64,
    total_ips: u64,
    scanned_ips: u64,
    found_servers: u64,
    current_range: String,
    status: String,
    start_time: Instant,
}

struct AppCtx {
    db: std::sync::Mutex<Option<Database>>,
    kitty_db: std::sync::Mutex<Option<kitty::KittyDatabase>>,
    scan_cancel: Arc<AtomicBool>,
    scan_running: Arc<AtomicBool>,
    scan_probe_wl: Arc<AtomicBool>,
    scan_proxy: std::sync::Mutex<Option<String>>,
    scan_force_proxy: Arc<AtomicBool>,
    scan_concurrency: Arc<AtomicU64>,
    scan_progress: std::sync::Mutex<ScanProgress>,
    stats_cache: std::sync::Mutex<Option<serde_json::Value>>,
    kitty_ctx: kitty::KittyCtx,
    db_push_running: AtomicBool,
    db_push_status: std::sync::Mutex<String>,
    wl_reverify_running: AtomicBool,
    wl_reverify_total: AtomicU64,
    wl_reverify_done: AtomicU64,
    rescan_all: AtomicBool,
}

#[derive(Clone, PartialEq)]
enum CycleType {
    Ipv4Fast,
    Ipv6Targeted,
    Ipv4Deep,
    Ipv6Deep,
}

impl CycleType {
    fn name(&self) -> &'static str {
        match self {
            CycleType::Ipv4Fast => "ipv4_fast",
            CycleType::Ipv6Targeted => "ipv6_targeted",
            CycleType::Ipv4Deep => "ipv4_deep",
            CycleType::Ipv6Deep => "ipv6_deep",
        }
    }

    fn label(&self) -> &'static str {
        match self {
            CycleType::Ipv4Fast => "IPv4 Fast (port 25565)",
            CycleType::Ipv6Targeted => "IPv6 Targeted (port 25565)",
            CycleType::Ipv4Deep => "IPv4 Deep (16 ports)",
            CycleType::Ipv6Deep => "IPv6 Deep (27 ports)",
        }
    }

    fn ports(&self) -> Vec<u16> {
        match self {
            CycleType::Ipv4Fast | CycleType::Ipv6Targeted => vec![25565],
            CycleType::Ipv4Deep | CycleType::Ipv6Deep => {
                let mut p: Vec<u16> = (25560..=25575).collect();
                p.extend(19130..=19140);
                p
            }
        }
    }

    fn is_v6(&self) -> bool {
        matches!(self, CycleType::Ipv6Targeted | CycleType::Ipv6Deep)
    }

    fn cycle_order() -> [CycleType; 4] {
        [
            CycleType::Ipv4Fast,
            CycleType::Ipv6Targeted,
            CycleType::Ipv4Deep,
            CycleType::Ipv6Deep,
        ]
    }
}

const SAVE_CHECKPOINT_EVERY: u64 = 100000;
const DENSITY_EMPTY_SKIP_AFTER: i64 = 3;

fn detect_or_start_proxy() -> Option<String> {
    for port in &[9050u16, 9150, 1080, 1088] {
        if std::net::TcpStream::connect_timeout(
            &"127.0.0.1".parse().unwrap(),
            Duration::from_secs(1),
        ).is_ok() {
            return Some(format!("127.0.0.1:{}", port));
        }
    }
    let tor_paths = ["/usr/bin/tor", "/usr/sbin/tor", "/usr/local/bin/tor", "/opt/homebrew/bin/tor"];
    for tp in &tor_paths {
        if std::path::Path::new(tp).exists() {
            log::info!("Tor found at {} but not running. Start it: `sudo systemctl start tor` or `tor &`", tp);
            break;
        }
    }
    None
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let ctx = Arc::new(AppCtx {
        db: std::sync::Mutex::new(None),
        kitty_db: std::sync::Mutex::new(None),
        scan_cancel: Arc::new(AtomicBool::new(false)),
        scan_running: Arc::new(AtomicBool::new(false)),
        scan_probe_wl: Arc::new(AtomicBool::new(true)),
        scan_proxy: std::sync::Mutex::new(None),
        scan_force_proxy: Arc::new(AtomicBool::new(false)),
        scan_concurrency: Arc::new(AtomicU64::new(4000)),
        scan_progress: std::sync::Mutex::new(ScanProgress {
            cycle_type: "stopped".into(),
            cycle: 0,
            total_ips: 0,
            scanned_ips: 0,
            found_servers: 0,
            current_range: String::new(),
            status: "stopped".to_string(),
            start_time: Instant::now(),
        }),
        stats_cache: std::sync::Mutex::new(None),
        kitty_ctx: kitty::KittyCtx::new(),
        db_push_running: AtomicBool::new(false),
        db_push_status: std::sync::Mutex::new(String::new()),
        wl_reverify_running: AtomicBool::new(false),
        wl_reverify_total: AtomicU64::new(0),
        wl_reverify_done: AtomicU64::new(0),
        rescan_all: AtomicBool::new(false),
    });

    log::info!("Direct connections only. Enable proxy (Tor) in Settings for privacy.");

    let data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("minefind");
    std::fs::create_dir_all(&data_dir).ok();

    match Database::new(data_dir.join("servers.db").to_str().unwrap()) {
        Ok(db) => {
            let count = db.get_server_count().unwrap_or(0);
            log::info!("Servers DB: {} servers", count);
            *ctx.db.lock().unwrap() = Some(db);
        }
        Err(e) => log::error!("Servers DB init: {}", e),
    }

    match kitty::KittyDatabase::new(data_dir.join("kitty.db").to_str().unwrap()) {
        Ok(kdb) => {
            log::info!("Kitty DB ready");
            *ctx.kitty_db.lock().unwrap() = Some(kdb);
        }
        Err(e) => log::error!("Kitty DB init: {}", e),
    }

    let app = Router::new()
        .route("/api/init", post(api_init))
        .route("/api/servers", get(api_servers))
        .route("/api/servers/count", get(api_count))
        .route("/api/stats", get(api_stats))
        .route("/api/scan/start", post(api_scan_start))
        .route("/api/scan/cancel", post(api_scan_cancel))
        .route("/api/scan/status", get(api_scan_status))
        .route("/api/scan/cycles", get(api_scan_cycles))
        .route("/api/scan/concurrency", post(api_set_concurrency))
        .route("/api/proxy/status", get(api_proxy_status))
        .route("/api/proxy/detect", post(api_proxy_detect))
        .route("/api/cache/clear", post(api_cache_clear))
        .route("/api/kitty/sync", post(kitty::api_kitty_sync))
        .route("/api/kitty/verify", post(kitty::api_kitty_verify))
        .route("/api/kitty/list", get(kitty::api_kitty_list))
        .route("/api/kitty/stats", get(kitty::api_kitty_stats))
        .route("/api/kitty/status", get(kitty::api_kitty_status))
        .route("/api/db/push", post(api_db_push))
        .route("/api/db/push/status", get(api_db_push_status))
        .route("/api/servers/reverify-wl", post(api_reverify_wl))
        .route("/api/servers/reverify-wl/status", get(api_reverify_wl_status))
        .route("/api/settings/rescan", post(api_set_rescan))
        .layer(CorsLayer::permissive())
        .with_state(ctx);

    let frontend_path = std::env::var("MINEFIND_FRONTEND")
        .unwrap_or_else(|_| {
            let p = std::path::PathBuf::from("../dist");
            if p.exists() { p.to_str().unwrap().to_string() } else { "dist".to_string() }
        });
    let app = app
        .route_service("/", ServeDir::new(&frontend_path))
        .route_service("/{*path}", ServeDir::new(&frontend_path));

    let addr = "0.0.0.0:8765";
    log::info!("MineFind -> http://localhost:8765");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

fn get_db(ctx: &AppCtx) -> Result<std::sync::MutexGuard<'_, Option<Database>>, String> {
    let g = ctx.db.lock().map_err(|e| format!("lock: {}", e))?;
    if g.is_none() { Err("DB not ready".into()) } else { Ok(g) }
}

fn ok_json(v: serde_json::Value) -> (StatusCode, Json<serde_json::Value>) {
    (StatusCode::OK, Json(v))
}

// --- API handlers (unchanged) ---

async fn api_init(State(ctx): State<Arc<AppCtx>>) -> (StatusCode, Json<serde_json::Value>) {
    let d = dirs_next::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("minefind");
    std::fs::create_dir_all(&d).ok();
    match Database::new(d.join("servers.db").to_str().unwrap()) {
        Ok(db) => {
            let c = db.get_server_count().unwrap_or(0);
            *ctx.db.lock().unwrap() = Some(db);
            ok_json(serde_json::json!({"ok":true,"count":c}))
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error":e.to_string()}))),
    }
}

async fn api_servers(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    let g = match get_db(&ctx) {
        Ok(g) => g,
        Err(e) => return Json(serde_json::json!({"error":e})),
    };
    match g.as_ref().unwrap().get_all_servers() {
        Ok(s) => Json(serde_json::json!(s)),
        Err(e) => Json(serde_json::json!({"error":e.to_string()})),
    }
}

async fn api_count(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    let g = match get_db(&ctx) {
        Ok(g) => g,
        Err(e) => return Json(serde_json::json!({"error":e})),
    };
    Json(serde_json::json!({"count": g.as_ref().unwrap().get_server_count().unwrap_or(0)}))
}

async fn api_reverify_wl(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    if ctx.wl_reverify_running.load(Ordering::SeqCst) {
        return Json(serde_json::json!({"error": "already running"}));
    }
    ctx.wl_reverify_running.store(true, Ordering::SeqCst);
    ctx.wl_reverify_total.store(0, Ordering::SeqCst);
    ctx.wl_reverify_done.store(0, Ordering::SeqCst);

    let ctx2 = ctx.clone();
    tokio::spawn(async move {
        let servers = get_db(&ctx2).ok()
            .and_then(|g| g.as_ref().unwrap().get_all_servers().ok())
            .unwrap_or_default();

        let total = servers.len() as u64;
        if total == 0 {
            ctx2.wl_reverify_running.store(false, Ordering::SeqCst);
            return;
        }

        ctx2.wl_reverify_total.store(total, Ordering::SeqCst);
        log::info!("WL re-verify: probing {} servers", total);

        let sem = Arc::new(tokio::sync::Semaphore::new(200));
        let proxy = ctx2.scan_proxy.lock().unwrap().clone();

        for server in &servers {
            let permit = sem.clone().acquire_owned().await.unwrap();
            let info = server.clone();
            let px = proxy.clone();
            let ctx3 = ctx2.clone();

            tokio::spawn(async move {
                let _held = permit;
                let px_ref: Option<&str> = px.as_deref();
                let wl = scanner::probe::check_whitelist(&info, px_ref).await;
                if let Ok(db_guard) = get_db(&ctx3) {
                    let mut updated = info;
                    updated.whitelisted = wl;
                    if let Err(e) = db_guard.as_ref().unwrap().upsert_server(&updated) {
                        log::error!("WL re-verify upsert {}:{} - {}", updated.ip, updated.port, e);
                    }
                }
                ctx3.wl_reverify_done.fetch_add(1, Ordering::SeqCst);
            });
        }

        while ctx2.wl_reverify_done.load(Ordering::SeqCst) < ctx2.wl_reverify_total.load(Ordering::SeqCst) {
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        ctx2.wl_reverify_running.store(false, Ordering::SeqCst);
        log::info!("WL re-verify complete: {} servers checked", total);
    });

    Json(serde_json::json!({"ok": true, "total": 0}))
}

async fn api_reverify_wl_status(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "running": ctx.wl_reverify_running.load(Ordering::SeqCst),
        "total": ctx.wl_reverify_total.load(Ordering::SeqCst),
        "done": ctx.wl_reverify_done.load(Ordering::SeqCst),
    }))
}

async fn api_set_rescan(
    State(ctx): State<Arc<AppCtx>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let on = params.get("on").map(|v| v == "1").unwrap_or(false);
    ctx.rescan_all.store(on, Ordering::SeqCst);
    Json(serde_json::json!({"rescan_all": on}))
}

async fn api_stats(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    if let Some(cached) = ctx.stats_cache.lock().unwrap().as_ref() {
        return Json(cached.clone());
    }
    let g = match get_db(&ctx) {
        Ok(g) => g,
        Err(e) => return Json(serde_json::json!({"error":e})),
    };
    let servers = match g.as_ref().unwrap().get_all_servers() {
        Ok(s) => s,
        Err(e) => return Json(serde_json::json!({"error":e.to_string()})),
    };
    let stats = compute_stats(&servers);
    *ctx.stats_cache.lock().unwrap() = Some(stats.clone());
    Json(stats)
}

fn compute_stats(servers: &[ServerInfo]) -> serde_json::Value {
    let total = servers.len();
    let wl = servers.iter().filter(|s| s.whitelisted == Some(true)).count();
    let nwl = servers.iter().filter(|s| s.whitelisted == Some(false)).count();
    let uwl = servers.iter().filter(|s| s.whitelisted.is_none()).count();
    let modded = servers.iter().filter(|s| s.modded).count();
    let tp: i32 = servers.iter().map(|s| s.online_players).sum();

    let mut cats = serde_json::Map::new();
    for c in &["vanilla_survival","modded","plugin_heavy","creative","minigame","anarchy","private_group","idle","unknown"] {
        cats.insert(c.to_string(), serde_json::json!(servers.iter().filter(|s| s.category.as_str() == *c).count()));
    }

    let mut vers: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for s in servers {
        let v = if s.version.contains("1.21") {"1.21"} else if s.version.contains("1.20") {"1.20"}
               else if s.version.contains("1.19") {"1.19"} else if s.version.contains("1.18") {"1.18"}
               else if s.version.contains("1.17") {"1.17"} else if s.version.contains("1.16") {"1.16"}
               else if s.version.contains("1.12") {"1.12"} else if s.version.contains("1.8") {"1.8"}
               else {"other"};
        *vers.entry(v.to_string()).or_insert(0) += 1;
    }

    serde_json::json!({"total":total,"whitelisted":wl,"not_whitelisted":nwl,"unknown_whitelist":uwl,
        "modded":modded,"total_players":tp,"categories":cats,"versions":vers})
}

// --- Scan API ---

async fn api_scan_start(
    State(ctx): State<Arc<AppCtx>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> (StatusCode, Json<serde_json::Value>) {
    if ctx.scan_running.load(Ordering::SeqCst) {
        return (StatusCode::CONFLICT, Json(serde_json::json!({"error":"already running"})));
    }

    let probe_wl = params.get("probe_whitelist").map(|v| v == "1").unwrap_or(true);
    let concurrency = params.get("concurrency").and_then(|v| v.parse::<u64>().ok()).unwrap_or(4000).max(100).min(10000);

    let explicit_proxy = params.get("proxy").filter(|s| !s.is_empty()).cloned();
    let stored_proxy = ctx.scan_proxy.lock().unwrap().clone();
    let force_proxy = params.get("force_proxy").map(|v| v == "1").unwrap_or(false);

    let proxy = if force_proxy {
        explicit_proxy.clone().or(stored_proxy)
    } else {
        explicit_proxy.clone()
    };

    if force_proxy && proxy.is_none() {
        return (StatusCode::BAD_REQUEST, Json(serde_json::json!({
            "error": "No proxy configured. Set proxy in Settings or disable Force Proxy."
        })));
    }

    if let Some(ref p) = explicit_proxy {
        *ctx.scan_proxy.lock().unwrap() = Some(p.clone());
    }

    ctx.scan_probe_wl.store(probe_wl, Ordering::SeqCst);
    ctx.scan_force_proxy.store(force_proxy, Ordering::SeqCst);
    ctx.scan_concurrency.store(concurrency, Ordering::SeqCst);
    ctx.scan_running.store(true, Ordering::SeqCst);
    ctx.scan_cancel.store(false, Ordering::SeqCst);

    if let Some(ref p) = proxy {
        log::info!("Proxy: {}", p);
    } else {
        log::info!("Direct connections");
    }

    let cancel = ctx.scan_cancel.clone();
    let running = ctx.scan_running.clone();
    let ctx2 = ctx.clone();

    tokio::spawn(async move { scan_loop(ctx2, cancel, running).await; });

    let cp = {
        let db_ref = get_db(&ctx).ok();
        db_ref.and_then(|g| g.as_ref().unwrap().load_checkpoint().ok()).flatten()
    };

    let mut resp = serde_json::json!({
        "ok":true, "message":"scan started (multi-cycle mode)",
        "cycle": ctx.scan_progress.lock().unwrap().cycle + 1,
        "probe_whitelist":probe_wl,
        "concurrency": concurrency,
        "proxy":proxy, "force_proxy":force_proxy,
    });
    if let Some(ref cp) = cp {
        resp["resuming"] = serde_json::json!(true);
        resp["resume_type"] = serde_json::json!(&cp.cycle_type);
        resp["resume_at"] = serde_json::json!(cp.scanned_ips);
    }
    ok_json(resp)
}

async fn scan_loop(ctx: Arc<AppCtx>, cancel: Arc<AtomicBool>, running: Arc<AtomicBool>) {
    let db_dir = dirs_next::data_dir().unwrap_or_else(|| std::path::PathBuf::from(".")).join("minefind");
    std::fs::create_dir_all(&db_dir).ok();
    let db = match Database::new(db_dir.join("servers.db").to_str().unwrap()) {
        Ok(d) => Arc::new(d),
        Err(e) => { log::error!("scan db: {}", e); running.store(false, Ordering::SeqCst); return; }
    };

    // Batch DB write channel — single writer avoids Mutex contention
    let (db_tx, mut db_rx) = tokio::sync::mpsc::channel::<ServerInfo>(10000);
    let writer_db = db.clone();
    tokio::spawn(async move {
        while let Some(info) = db_rx.recv().await {
            if let Err(e) = writer_db.upsert_server(&info) {
                log::error!("DB upsert {}:{} - {}", info.ip, info.port, e);
            }
        }
    });

    let cycle_order = CycleType::cycle_order();
    let mut cycle_order_idx: usize = 0;
    let mut global_cycle: u64 = 0;

    // Check for resume
    let mut resume_from = db.load_checkpoint().ok().flatten();

    while !cancel.load(Ordering::SeqCst) {
        let ct = cycle_order[cycle_order_idx].clone();
        let cycle_name = ct.name();
        let cycle_label = ct.label();

        global_cycle += 1;
        let cycle_start = Instant::now();
        let cycle_started_at = chrono::Utc::now().to_rfc3339();
        let found_count = Arc::new(AtomicU64::new(0));

        let probe_wl = ctx.scan_probe_wl.load(Ordering::SeqCst);
        let proxy = ctx.scan_proxy.lock().unwrap().clone();
        let force_proxy = ctx.scan_force_proxy.load(Ordering::SeqCst);
        let effective_proxy: Option<String> = if force_proxy { proxy.clone() } else { None };
        let concurrency = ctx.scan_concurrency.load(Ordering::SeqCst).max(100) as usize;

        let ports = ct.ports();
        let port_count = ports.len();

        let ranges = if ct.is_v6() {
            vec![] // IPv6 not iterable for now, skip
        } else {
            build_ipv4_ranges(&db, cycle_name, ctx.rescan_all.load(Ordering::SeqCst))
        };

        let total_ips: u64 = ranges.iter().map(|r| {
            let s = scanner::ranges::ip_to_u32(r.start);
            let e = scanner::ranges::ip_to_u32(r.end);
            (e.saturating_sub(s) + 1) as u64
        }).sum();

        let mut scanned_ips: u64 = 0;
        let mut start_from_ip: Option<u32> = None;

        // Resume logic
        if let Some(ref cp) = resume_from {
            if cp.cycle_type == cycle_name {
                log::info!("Resuming {} at ip_u32={} ({} scanned, {} found)", cycle_name, cp.ip_u32, cp.scanned_ips, cp.found_servers);
                start_from_ip = Some(cp.ip_u32 as u32);
                scanned_ips = cp.scanned_ips;
                found_count.store(cp.found_servers, Ordering::SeqCst);
                global_cycle = cp.cycle_num;
            } else {
                log::info!("Checkpoint was for {}, starting fresh {}", cp.cycle_type, cycle_name);
            }
            resume_from = None;
        }

        {
            let mut p = ctx.scan_progress.lock().unwrap();
            p.cycle_type = cycle_name.to_string();
            p.cycle = global_cycle;
            p.total_ips = total_ips;
            p.scanned_ips = scanned_ips;
            p.found_servers = 0;
            p.current_range = cycle_label.to_string();
            p.status = "scanning".to_string();
            p.start_time = cycle_start;
        }

        let effective_total = total_ips.saturating_sub(scanned_ips);
        log::info!("=== Cycle {} ({}) - {} IPs across {} ranges ({} ports) ===", global_cycle, cycle_label, total_ips, ranges.len(), port_count);
        if scanned_ips > 0 {
            log::info!("Resuming at {} scanned, {} remaining", scanned_ips, effective_total);
        }

        let sem = Arc::new(tokio::sync::Semaphore::new(concurrency));
        let mut handles: Vec<tokio::task::JoinHandle<()>> = Vec::new();
        let mut last_checkpoint_at: u64 = scanned_ips;

        for range in &ranges {
            if cancel.load(Ordering::SeqCst) { break; }

            let range_start = scanner::ranges::ip_to_u32(range.start);
            let range_end = scanner::ranges::ip_to_u32(range.end);
            let range_name = format!("{} ({})", range.name, range.mask);

            // Skip ranges before resume point
            let ip_start = if let Some(resume_ip) = start_from_ip {
                if resume_ip > range_end {
                    // Already past this range, skip entirely
                    scanned_ips += (range_end.saturating_sub(range_start) + 1) as u64;
                    continue;
                } else if resume_ip >= range_start {
                    // Resume within this range
                    start_from_ip = None;
                    resume_ip
                } else {
                    // Before resume point, skip
                    range_start
                }
            } else {
                range_start
            };

            {
                let mut p = ctx.scan_progress.lock().unwrap();
                p.current_range = range_name.clone();
            }

            let range_found_before = found_count.load(Ordering::SeqCst);

            for ip_u32 in ip_start..=range_end {
                if cancel.load(Ordering::SeqCst) { break; }
                if scanner::ranges::is_reserved(ip_u32) {
                    scanned_ips += 1;
                    continue;
                }

                let permit = sem.clone().acquire_owned().await.unwrap();
                let c = cancel.clone();
                let a_str = scanner::ranges::u32_to_ip(ip_u32).to_string();
                let proxy_for_task = effective_proxy.clone();
                let fc = found_count.clone();
                let do_probe = probe_wl;
                let task_ports = ports.clone();
                let task_tx = db_tx.clone();

                handles.push(tokio::spawn(async move {
                    let _held = permit;
                    let mut found_this_ip: u64 = 0;
                    let proxy_ref: Option<String> = proxy_for_task;

                    for &port in &task_ports {
                        if c.load(Ordering::SeqCst) { break; }
                        let px: Option<&str> = proxy_ref.as_deref();
                        let r = if let Some(pr) = px {
                            scanner::ping::ping_server_via_proxy(&a_str, port, Some(pr)).await
                        } else {
                            scanner::ping::ping_server(&a_str, port).await
                        };
                        if let Ok(mut info) = r {
                            found_this_ip += 1;
                            log::info!("Found server: {}:{} v={}", info.ip, port, info.version);
                            if do_probe {
                                info.whitelisted = scanner::probe::check_whitelist(&info, px).await;
                            }
                            info.category = categorize_from_info(&info);
                            info.tags = crate::scanner::ping::generate_tags_info(&info);
                            let _ = task_tx.send(info).await;
                        }
                    }

                    if found_this_ip > 0 {
                        fc.fetch_add(found_this_ip, Ordering::SeqCst);
                    }
                }));

                scanned_ips += 1;

                if scanned_ips % 200 == 0 {
                    handles.retain(|h| !h.is_finished());
                    {
                        let mut p = ctx.scan_progress.lock().unwrap();
                        p.scanned_ips = scanned_ips;
                        p.found_servers = found_count.load(Ordering::SeqCst);
                    }
                    *ctx.stats_cache.lock().unwrap() = None;
                    tokio::task::yield_now().await;
                }

                if scanned_ips.saturating_sub(last_checkpoint_at) >= SAVE_CHECKPOINT_EVERY {
                    if let Err(e) = db.save_checkpoint(cycle_name, global_cycle, ip_u32 as u64, scanned_ips, found_count.load(Ordering::SeqCst)) {
                        log::error!("Checkpoint save failed: {}", e);
                    }
                    last_checkpoint_at = scanned_ips;
                }
            }

            // Record density for this /8
            let range_found = found_count.load(Ordering::SeqCst).saturating_sub(range_found_before);
            let _ = db.record_range_density(&range_name, range_found as i64, cycle_name);
        }

        // Drain remaining handles
        for h in handles.drain(..) {
            if let Err(e) = h.await {
                log::error!("Task panic: {:?}", e);
            }
        }

        let cycle_duration = cycle_start.elapsed();
        let total_found = found_count.load(Ordering::SeqCst);
        let cycle_secs = cycle_duration.as_secs();

        log::info!("=== {} complete: {} servers in {:.0}s ===", cycle_label, total_found, cycle_duration.as_secs_f64());

        if let Err(e) = db.record_cycle(cycle_name, global_cycle, scanned_ips, total_found, &cycle_started_at, cycle_secs) {
            log::error!("record_cycle: {}", e);
        }
        // Fallback lifetime counter — survives DB failures
        save_lifetime(scanned_ips);
        let _ = db.clear_checkpoint();

        {
            let mut p = ctx.scan_progress.lock().unwrap();
            p.scanned_ips = scanned_ips;
            p.found_servers = total_found;
            p.current_range = format!("{} complete ({:.0}s)", cycle_label, cycle_duration.as_secs_f64());
            p.status = "waiting".to_string();
        }
        *ctx.stats_cache.lock().unwrap() = None;

        // Advance to next cycle type
        cycle_order_idx = (cycle_order_idx + 1) % cycle_order.len();

        // Between cycles: poll cancel every 5s for 30s
        if !cancel.load(Ordering::SeqCst) {
            for _ in 0..6 {
                if cancel.load(Ordering::SeqCst) { break; }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    running.store(false, Ordering::SeqCst);
    {
        let mut p = ctx.scan_progress.lock().unwrap();
        p.status = "stopped".to_string();
        p.current_range = format!("Stopped after {} cycles", global_cycle);
    }
    log::info!("Scanner stopped after {} cycles", global_cycle);
}

fn build_ipv4_ranges(db: &Database, cycle_type: &str, rescan_all: bool) -> Vec<scanner::ranges::CidrRange> {
    let skipped_prefixes: std::collections::HashSet<String> = db
        .get_skipped_prefixes(DENSITY_EMPTY_SKIP_AFTER)
        .unwrap_or_default()
        .into_iter()
        .collect();

    let already_scanned: std::collections::HashSet<String> = if rescan_all {
        std::collections::HashSet::new()
    } else {
        db.get_already_scanned_prefixes(cycle_type)
            .unwrap_or_default()
            .into_iter()
            .collect()
    };

    let priority = scanner::ranges::get_priority_ranges();
    let full = scanner::ranges::get_full_ipv4_ranges();

    // Filter priority ranges first (they always get scanned)
    let priority_set: std::collections::HashSet<String> = priority
        .iter()
        .map(|r| format!("{}.0.0.0/8", r.start.octets()[0]))
        .collect();

    let mut all: Vec<scanner::ranges::CidrRange> = Vec::new();
    for r in &priority {
        let prefix = format!("{}.0.0.0/8", r.start.octets()[0]);
        if !skipped_prefixes.contains(&prefix) && !already_scanned.contains(&prefix) {
            all.push(r.clone());
        }
    }

    // Add full ranges, skipping priority duplicates, empty /8s, and already-scanned
    for r in &full {
        let prefix = format!("{}.0.0.0/8", r.start.octets()[0]);
        if priority_set.contains(&prefix) { continue; }
        if skipped_prefixes.contains(&prefix) { continue; }
        if already_scanned.contains(&prefix) { continue; }
        all.push(r.clone());
    }

    all
}

fn categorize_from_info(info: &scanner::ServerInfo) -> scanner::ServerCategory {
    use scanner::ServerCategory::*;
    let lower = info.motd.to_lowercase();

    if info.online_players == 0 && info.whitelisted != Some(false) {
        return Idle;
    }
    if lower.contains("anarchy") || lower.contains("2b2t") || lower.contains("norules") {
        return Anarchy;
    }
    if lower.contains("minigame") || lower.contains("skywars") || lower.contains("bedwars")
        || lower.contains("kitpvp") || lower.contains("prison") || lower.contains("hunger")
    {
        return Minigame;
    }
    if lower.contains("creative") || lower.contains("plot") || lower.contains("build") {
        return Creative;
    }
    if info.modded {
        return Modded;
    }
    if info.online_players <= 5 {
        if info.whitelisted == Some(false) || lower.contains("private") || lower.contains("friends")
            || lower.contains("family") || lower.contains("small") || lower.contains("group")
        {
            return PrivateGroup;
        }
        return VanillaSurvival;
    }
    if lower.contains("survival") || lower.contains("vanilla") || lower.contains("smp") {
        return VanillaSurvival;
    }
    if info.whitelisted == Some(false) && info.online_players <= 10 {
        return PrivateGroup;
    }
    Unknown
}

async fn api_scan_cancel(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    ctx.scan_cancel.store(true, Ordering::SeqCst);
    ctx.scan_running.store(false, Ordering::SeqCst);
    {
        let mut p = ctx.scan_progress.lock().unwrap();
        p.status = "stopped".to_string();
    }
    Json(serde_json::json!({"ok":true}))
}

async fn api_scan_status(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    let running = ctx.scan_running.load(Ordering::SeqCst);
    let p = ctx.scan_progress.lock().unwrap();
    let elapsed = p.start_time.elapsed().as_secs();

    let lifetime_scanned = get_db(&ctx).ok()
        .and_then(|g| g.as_ref().unwrap().get_cycle_summary().ok())
        .and_then(|v| v.get("total_targets_scanned").and_then(|x| x.as_u64()))
        .filter(|&x| x > 0)
        .unwrap_or_else(|| load_lifetime().max(p.scanned_ips));

    Json(serde_json::json!({
        "running": running,
        "cycle_type": p.cycle_type,
        "cycle": p.cycle,
        "status": p.status,
        "total_ips": p.total_ips,
        "scanned_ips": p.scanned_ips,
        "lifetime_scanned": lifetime_scanned,
        "found_servers": p.found_servers,
        "current_range": p.current_range,
        "elapsed_secs": elapsed,
    }))
}

async fn api_set_concurrency(
    State(ctx): State<Arc<AppCtx>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Json<serde_json::Value> {
    let n = params.get("n").and_then(|v| v.parse::<u64>().ok()).unwrap_or(4000).max(100).min(10000);
    ctx.scan_concurrency.store(n, Ordering::SeqCst);
    log::info!("Scan concurrency set to {}", n);
    Json(serde_json::json!({"concurrency": n}))
}

async fn api_scan_cycles(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    match get_db(&ctx) {
        Ok(g) => match g.as_ref().unwrap().get_cycle_summary() {
            Ok(v) => Json(v),
            Err(e) => Json(serde_json::json!({"error": e.to_string()})),
        },
        Err(e) => Json(serde_json::json!({"error": e})),
    }
}

async fn api_proxy_status(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    let proxy = ctx.scan_proxy.lock().unwrap().clone();
    let fp = ctx.scan_force_proxy.load(Ordering::SeqCst);
    Json(serde_json::json!({
        "proxy": proxy,
        "force_proxy": fp,
        "blocked": fp && proxy.is_none(),
    }))
}

async fn api_proxy_detect(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    let detected = detect_or_start_proxy();
    if let Some(ref p) = detected {
        *ctx.scan_proxy.lock().unwrap() = Some(p.clone());
        log::info!("Proxy detected at {}", p);
        Json(serde_json::json!({"proxy": p, "ok": true}))
    } else {
        Json(serde_json::json!({"proxy": null, "ok": false, "error": "No SOCKS5 proxy found on common ports"}))
    }
}

async fn api_cache_clear(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    *ctx.stats_cache.lock().unwrap() = None;
    Json(serde_json::json!({"ok":true}))
}

fn lifetime_counter_path() -> std::path::PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("minefind")
        .join("lifetime_counter.txt")
}

fn save_lifetime(count: u64) {
    if let Ok(s) = std::fs::read_to_string(lifetime_counter_path()) {
        let old: u64 = s.trim().parse().unwrap_or(0);
        if count > old {
            std::fs::write(lifetime_counter_path(), count.to_string()).ok();
        }
    } else {
        std::fs::write(lifetime_counter_path(), count.to_string()).ok();
    }
}

fn load_lifetime() -> u64 {
    std::fs::read_to_string(lifetime_counter_path())
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0)
}

const DB_REPO_URL: &str = "https://github.com/MrNova420/minefind-database.git";

fn db_repo_cache_dir() -> std::path::PathBuf {
    dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("minefind")
        .join("db-repo-cache")
}

async fn api_db_push(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    if ctx.db_push_running.load(Ordering::SeqCst) {
        return Json(serde_json::json!({"error": "push already running"}));
    }
    ctx.db_push_running.store(true, Ordering::SeqCst);
    *ctx.db_push_status.lock().unwrap() = "cloning...".into();

    let ctx2 = ctx.clone();
    tokio::spawn(async move {
        let _ = run_db_push(&ctx2).await;
        ctx2.db_push_running.store(false, Ordering::SeqCst);
    });

    Json(serde_json::json!({"ok": true, "message": "push started"}))
}

async fn api_db_push_status(State(ctx): State<Arc<AppCtx>>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "running": ctx.db_push_running.load(Ordering::SeqCst),
        "status": ctx.db_push_status.lock().unwrap().clone(),
    }))
}

async fn run_db_push(ctx: &AppCtx) -> Result<(), String> {
    let data_dir = dirs_next::data_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("minefind");
    let cache = db_repo_cache_dir();
    let servers_db = data_dir.join("servers.db");
    let kitty_db = data_dir.join("kitty.db");

    let set_status = |s: &str| { *ctx.db_push_status.lock().unwrap() = s.to_string(); };

    // Clone or pull
    if cache.join(".git").exists() {
        set_status("pulling latest...");
        let out = tokio::process::Command::new("git")
            .args(["-C", cache.to_str().unwrap(), "pull", "--rebase"])
            .output().await.map_err(|e| format!("git pull: {}", e))?;
        if !out.status.success() {
            log::warn!("git pull: {}", String::from_utf8_lossy(&out.stderr));
        }
    } else {
        set_status("cloning db repo...");
        std::fs::create_dir_all(&cache).ok();
        let _ = std::fs::remove_dir_all(&cache);
        let out = tokio::process::Command::new("git")
            .args(["clone", DB_REPO_URL, cache.to_str().unwrap()])
            .output().await.map_err(|e| format!("git clone: {}", e))?;
        if !out.status.success() {
            set_status(&format!("clone failed: {}", String::from_utf8_lossy(&out.stderr)));
            return Err("clone failed".into());
        }
    }

    // Copy DB files
    set_status("copying database files...");
    for (src, dst_name) in &[(&servers_db, "servers.db"), (&kitty_db, "kitty.db")] {
        if src.exists() {
            std::fs::copy(src, cache.join(dst_name)).map_err(|e| format!("copy: {}", e))?;
        }
    }

    // Export servers as JSON
    set_status("exporting servers.json...");
    {
        let db_guard = ctx.db.lock().map_err(|e| format!("lock: {}", e))?;
        if let Some(ref db) = *db_guard {
            let servers = db.get_all_servers().unwrap_or_default();
            let kitty_db_guard = ctx.kitty_db.lock().map_err(|e| format!("lock: {}", e))?;
            let kitty_list = kitty_db_guard.as_ref()
                .map(|kdb| kdb.get_all().unwrap_or_default())
                .unwrap_or_default();
            let export = serde_json::json!({
                "exported_at": chrono::Utc::now().to_rfc3339(),
                "total_servers": servers.len(),
                "total_kitty_ips": kitty_list.len(),
                "cycle_summary": db.get_cycle_summary().unwrap_or(serde_json::json!({})),
                "servers": servers,
                "kitty_ips": kitty_list,
            });
            std::fs::write(cache.join("servers.json"),
                serde_json::to_string_pretty(&export).unwrap_or_default())
                .map_err(|e| format!("write json: {}", e))?;
        }
    }

    // Create README if not exists
    let readme_path = cache.join("README.md");
    if !readme_path.exists() {
        std::fs::write(&readme_path, format!("# MineFind Database\n\nAuto-updated Minecraft server database from MineFind scanner.\n\nLast sync: {}\n", chrono::Utc::now().to_rfc3339())).ok();
    }

    // Git add, commit, push
    set_status("committing...");
    let _ = tokio::process::Command::new("git")
        .args(["-C", cache.to_str().unwrap(), "add", "-A"])
        .output().await;

    let ts = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S").to_string();
    let servers_count = get_db(ctx).ok().and_then(|g| g.as_ref().unwrap().get_server_count().ok()).unwrap_or(0);
    let _ = tokio::process::Command::new("git")
        .args(["-C", cache.to_str().unwrap(), "commit", "-m", &format!("Auto-sync: {} servers", servers_count)])
        .output().await;

    set_status("pushing...");
    let out = tokio::process::Command::new("git")
        .args(["-C", cache.to_str().unwrap(), "push"])
        .output().await.map_err(|e| format!("git push: {}", e))?;

    if out.status.success() {
        set_status(&format!("pushed at {} — {} servers", ts, servers_count));
        log::info!("DB pushed to minefind-database: {} servers, {} kitty IPs", servers_count,
            ctx.kitty_db.lock().ok().and_then(|g| g.as_ref().map(|k| k.get_all().unwrap_or_default().len())).unwrap_or(0));
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr);
        set_status(&format!("push failed: {}", stderr));
        log::error!("git push failed: {}", stderr);
        return Err("push failed".into());
    }

    Ok(())
}
