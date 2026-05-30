<script>
  import Dashboard from "./lib/Dashboard.svelte";
  import ServerList from "./lib/ServerList.svelte";
  import MapView from "./lib/Map.svelte";
  import Kitty from "./lib/Kitty.svelte";
  import Cycles from "./lib/Cycles.svelte";
  import Sources from "./lib/Sources.svelte";

  let currentView = $state("dashboard");
  let stats = $state({});
  let servers = $state([]);
  let dbReady = $state(false);
  let scanRunning = $state(false);
  let probeWhitelist = $state(true);
  let proxyAddr = $state("");
  let proxyAvailable = $state(false);
  let forceProxy = $state(false);
  let showSettings = $state(false);
  let concurrency = $state(4000);
  let rescanAll = $state(true);
  let cycleToggles = $state({ ipv4_fast: true, ipv6_targeted: true, ipv4_hot_deep: true, ipv4_deep: true, ipv6_deep: true });
  let cpuLimitPct = $state(45);
  let hasIpv6 = $state(true);
  let cycleStats = $state({ cycles: 0, total_servers_found: 0, total_targets_scanned: 0 });
  let progress = $state({ scanned_ips: 0, total_ips: 0, found_servers: 0, current_range: "", elapsed_secs: 0, cycle: 0, cycle_type: "", status: "stopped", lifetime_scanned: 0 });
  let kittyServers = $state([]);
  let kittyStats = $state({ total: 0, verified: 0, syncing: false, verifying: false });
  let kittyVerifyProgress = $state({ verify_total: 0, verify_done: 0, verify_found: 0 });
  let dbPushStatus = $state({ running: false, status: "" });
  let wlReverify = $state({ running: false, total: 0, done: 0 });
  let cycleData = $state({ summary: {}, history: [], checkpoint: null });
  let sourceStats = $state({ total: 0 });

  let lifetimeScanned = $derived(
    progress.lifetime_scanned || cycleStats.total_targets_scanned || 0
  );

  const API = "/api";

  async function api(path, options = {}) {
    try {
      const res = await fetch(`${API}${path}`, {
        headers: { "Content-Type": "application/json" },
        ...options,
      });
      return await res.json();
    } catch (e) {
      console.error(`API ${path} failed:`, e);
      return null;
    }
  }

  async function refreshAll() {
    const [s, sv] = await Promise.all([api("/stats"), api("/servers")]);
    if (s) stats = s;
    if (sv) servers = sv;
  }

  async function kittySync() {
    await api("/kitty/sync", { method: "POST" });
    await kittyRefresh();
  }

  async function kittyVerify() {
    await api("/kitty/verify", { method: "POST" });
    await kittyRefresh();
    await pollKittyVerify();
  }

  async function pollKittyVerify() {
    while (kittyStats.verifying) {
      await new Promise((r) => setTimeout(r, 1000));
      const st = await api("/kitty/status");
      if (st) {
        kittyVerifyProgress = { verify_total: st.verify_total || 0, verify_done: st.verify_done || 0, verify_found: st.verify_found || 0 };
        if (!st.verifying) break;
      }
    }
    await kittyRefresh();
  }

  async function kittyRefresh() {
    const [list, st] = await Promise.all([api("/kitty/list"), api("/kitty/stats")]);
    if (list) kittyServers = list;
    if (st) kittyStats = st;
  }

  async function pushDb() {
    const res = await api("/db/push", { method: "POST" });
    if (res?.ok) {
      dbPushStatus = { running: true, status: "starting..." };
      pollPushStatus();
    } else {
      dbPushStatus = { running: false, status: res?.error || "Failed to start push" };
    }
  }

  async function reverifyWL() {
    const res = await api("/servers/reverify-wl", { method: "POST" });
    if (res?.ok) {
      wlReverify = { running: true, total: 0, done: 0 };
      pollWLReverify();
    }
  }

  async function pollWLReverify() {
    while (wlReverify.running) {
      await new Promise((r) => setTimeout(r, 1000));
      const st = await api("/servers/reverify-wl/status");
      if (st) {
        wlReverify = { running: st.running || false, total: st.total || 0, done: st.done || 0 };
        if (!st.running) break;
      }
    }
    await refreshAll();
  }

  async function pollPushStatus() {
    while (dbPushStatus.running) {
      await new Promise((r) => setTimeout(r, 2000));
      const st = await api("/db/push/status");
      if (st) {
        dbPushStatus = { running: st.running || false, status: st.status || "" };
        if (!st.running && st.status?.includes("pushed")) break;
      }
    }
    if (!dbPushStatus.running && !dbPushStatus.status?.includes("pushed")) {
      dbPushStatus = { running: false, status: dbPushStatus.status || "push completed" };
    }
  }

  async function checkProxy() {
    const info = await api("/proxy/status");
    if (info) {
      proxyAvailable = !!info.proxy;
      proxyAddr = info.proxy || "";
      forceProxy = info.force_proxy === true;
    }
  }

  async function detectProxy() {
    await api("/proxy/detect", { method: "POST" });
    await checkProxy();
  }

  async function toggleForceProxy() {
    const newVal = !forceProxy;
    forceProxy = newVal;
    await api(`/settings/force-proxy?on=${newVal ? "1" : "0"}`, { method: "POST" });
  }

  async function toggleWLProbe() {
    const newVal = !probeWhitelist;
    probeWhitelist = newVal;
    await api(`/settings/probe-wl?on=${newVal ? "1" : "0"}`, { method: "POST" });
  }

  async function toggleRescan() {
    const newVal = !rescanAll;
    rescanAll = newVal;
    await api(`/settings/rescan?on=${newVal ? "1" : "0"}`, { method: "POST" });
  }

  async function toggleCycle(cycle) {
    const newVal = !cycleToggles[cycle];
    cycleToggles[cycle] = newVal;
    await api(`/settings/cycle?cycle=${cycle}&on=${newVal ? "1" : "0"}`, { method: "POST" });
  }

  async function setCpuLimit(pct) {
    cpuLimitPct = pct;
    await api(`/settings/cpu-limit?pct=${pct}`, { method: "POST" });
  }

  async function resetScanner() {
    if (!confirm("Reset all scanner memory? Keeps your servers, clears all cycle history and checkpoints.")) return;
    const res = await api("/scan/reset", { method: "POST" });
    if (res?.ok) {
      cycleData = { summary: {}, history: [], checkpoint: null };
      cycleStats = { cycles: 0, total_servers_found: 0, total_targets_scanned: 0, actual_servers: 0 };
    } else {
      alert(res?.error || "Reset failed");
    }
  }

  async function toggleScan() {
    if (scanRunning) {
      await api("/scan/cancel", { method: "POST" });
      // Wait for backend to actually stop (up to 30s)
      while (scanRunning) {
        await new Promise((r) => setTimeout(r, 500));
        const st = await api("/scan/status");
        if (st) {
          if (!st.running) scanRunning = false;
          progress = st;
        }
      }
    } else {
      await startCycle();
    }
  }

  async function startCycle(cycleType = null) {
    // First check if backend says running
    const st = await api("/scan/status");
    if (st?.running) {
      await api("/scan/cancel", { method: "POST" });
      // Wait for stop (up to 30s)
      for (let i = 0; i < 60; i++) {
        await new Promise((r) => setTimeout(r, 500));
        const s = await api("/scan/status");
        if (!s?.running) break;
      }
    }
    const params = new URLSearchParams({
      probe_whitelist: probeWhitelist ? "1" : "0",
      force_proxy: forceProxy ? "1" : "0",
      concurrency: String(concurrency),
    });
    if (cycleType) params.set("cycle_type", cycleType);
    else if (!cycleData?.checkpoint) params.set("fresh", "1");
    if (forceProxy && proxyAddr.trim()) params.set("proxy", proxyAddr.trim());
    const res = await api(`/scan/start?${params}`, { method: "POST" });
    if (res?.ok) {
      scanRunning = true;
      pollScan();
      pollCycleStats();
    } else {
      alert(res?.error || "Failed to start scan");
    }
  }

  async function pollCycleStats() {
    const cs = await api("/scan/cycles");
    if (cs) {
      cycleStats = cs.summary || cs;
      cycleData = cs;
    }
  }

  async function pollScan() {
    while (scanRunning) {
      const p = await api("/scan/status");
      if (!p?.running) {
        scanRunning = false;
        await refreshAll();
        await api("/cache/clear", { method: "POST" });
        progress = { scanned_ips: 0, total_ips: 0, found_servers: 0, current_range: "", elapsed_secs: 0, cycle: 0, cycle_type: "", status: "stopped", lifetime_scanned: 0 };
        break;
      }
      progress = p;
      const delay = p?.cycle_type?.includes("deep") ? 5000 : 2000;
      await new Promise((r) => setTimeout(r, delay));
      const s = await api("/stats");
      if (s) stats = s;
    }
  }

  import { onMount } from "svelte";
  onMount(async () => {
    const [init, settings] = await Promise.all([
      api("/init", { method: "POST" }),
      checkProxy(),
    ]);
    if (init) {
      dbReady = true;
      await refreshAll();
    }
    pollCycleStats();
    // Load saved settings
    const stg = await api("/settings");
    if (stg) {
      rescanAll = stg.rescan_all === true;
      forceProxy = stg.force_proxy === true;
      probeWhitelist = stg.probe_whitelist !== false;
      if (stg.concurrency) concurrency = stg.concurrency;
      cycleToggles = {
        ipv4_fast: stg.cycle_ipv4_fast !== false,
        ipv6_targeted: stg.cycle_ipv6_targeted !== false,
        ipv4_hot_deep: stg.cycle_ipv4_hot_deep !== false,
        ipv4_deep: stg.cycle_ipv4_deep !== false,
        ipv6_deep: stg.cycle_ipv6_deep !== false,
      };
      hasIpv6 = stg.has_ipv6 !== false;
      cpuLimitPct = stg.cpu_limit_pct ?? 45;
    }
    // Resume live polling if scan is already running
    const status = await api("/scan/status");
    if (status?.running) {
      scanRunning = true;
      pollScan();
    }
  });

  let pct = $derived(progress.total_ips > 0 ? Math.round((progress.scanned_ips / progress.total_ips) * 100) : 0);
  let elapsedStr = $derived(
    progress.elapsed_secs >= 3600
      ? `${Math.floor(progress.elapsed_secs / 3600)}h${Math.floor((progress.elapsed_secs % 3600) / 60)}m`
      : progress.elapsed_secs >= 60
        ? `${Math.floor(progress.elapsed_secs / 60)}m${progress.elapsed_secs % 60}s`
        : `${progress.elapsed_secs}s`
  );

  let scanRate = $derived(
    progress.elapsed_secs > 0 && progress.scanned_ips > 0
      ? Math.round(progress.scanned_ips / progress.elapsed_secs)
      : 0
  );

  let etaSeconds = $derived(
    scanRate > 0 && progress.total_ips > progress.scanned_ips
      ? Math.round((progress.total_ips - progress.scanned_ips) / scanRate)
      : 0
  );

  let etaStr = $derived(
    etaSeconds >= 86400
      ? `~${Math.floor(etaSeconds / 86400)}d ${Math.floor((etaSeconds % 86400) / 3600)}h`
      : etaSeconds >= 3600
        ? `~${Math.floor(etaSeconds / 3600)}h ${Math.floor((etaSeconds % 3600) / 60)}m`
        : etaSeconds >= 60
          ? `~${Math.floor(etaSeconds / 60)}m`
          : etaSeconds > 0 ? `~${etaSeconds}s` : ""
  );
</script>

<div class="app">
  <header>
    <div class="logo">
      <span class="icon">⬡</span>
      <h1>MineFind</h1>
    </div>
    <nav>
      <button
        class="nav-btn"
        class:active={currentView === "dashboard"}
        onclick={() => { currentView = "dashboard"; refreshAll(); pollCycleStats(); }}
      >
        Dashboard
      </button>
      <button
        class="nav-btn"
        class:active={currentView === "servers"}
        onclick={() => { currentView = "servers"; refreshAll(); if (wlReverify.running) pollWLReverify(); }}
      >
        Servers {servers.length ? `(${servers.length})` : ""}
      </button>
      <button
        class="nav-btn"
        class:active={currentView === "map"}
        onclick={() => (currentView = "map")}
      >
        Map
      </button>
      <button
        class="nav-btn"
        class:active={currentView === "kitty"}
        onclick={() => { currentView = "kitty"; kittyRefresh(); }}
      >
        Kitty
      </button>
      <button
        class="nav-btn"
        class:active={currentView === "cycles"}
        onclick={() => { currentView = "cycles"; pollCycleStats(); }}
      >
        Cycles
      </button>
      <button
        class="nav-btn"
        class:active={currentView === "sources"}
        onclick={() => { currentView = "sources"; api("/serverlist/stats").then(s => { if (s) sourceStats = s; }); }}
      >
        Sources
      </button>
    </nav>
    <div class="actions">
      <span class="proxy-badge" class:active={proxyAvailable} title={proxyAvailable ? "Proxy available" : "No proxy"}>
        {proxyAvailable ? "🔒 Proxy" : "🌐 Direct"}
      </span>
      <button class="settings-btn" onclick={() => (showSettings = !showSettings)}>
        ⚙
      </button>
      <span class="status-badge" class:scanning={scanRunning}>
        {scanRunning
          ? progress.status === "waiting"
            ? `Waiting (${progress.cycle_type})`
            : `Cycle ${progress.cycle} · ${progress.cycle_type}`
          : cycleStats.cycles > 0 ? `${cycleStats.cycles} cycles done` : "Idle"}
      </span>
      <button
        class="scan-btn"
        class:scanning={scanRunning}
        onclick={toggleScan}
      >
        {scanRunning ? "Stop" : (cycleData?.checkpoint ? "Resume" : "Start Fresh")}
      </button>
    </div>
  </header>

  {#if scanRunning}
    <div class="progress-bar-track" class:waiting={progress.status === "waiting"} class:cooling={progress.status === "cooling"}>
      <div class="progress-bar-fill" class:cooling={progress.status === "cooling"} style="width: {progress.status === "waiting" || progress.status === "cooling" ? 100 : pct}%"></div>
      <div class="progress-info">
        {#if progress.status === "cooling"}
          <span>PC health pause</span>
          <span>14m remaining</span>
        {:else if progress.status === "waiting"}
          <span>Cycle {progress.cycle} complete ({progress.cycle_type})</span>
          <span>Next in ~30s</span>
        {:else}
          <span>{progress.cycle_type}</span>
          <span>{pct}%</span>
          <span>{progress.found_servers} found</span>
          <span>{progress.current_range}</span>
          <span>{elapsedStr}</span>
          {#if etaStr}
            <span>ETA: {etaStr}</span>
          {/if}
        {/if}
      </div>
    </div>
    <div class="scan-counter">
      <div class="counter-main">
        <span class="counter-num">{Intl.NumberFormat().format(progress?.scanned_ips ?? 0)}</span>
        <span class="counter-label">IPs scanned {#if scanRate > 0}({Intl.NumberFormat().format(scanRate)}/s){/if}</span>
      </div>
      <div class="counter-secondary">
        <span>{Intl.NumberFormat().format(progress?.total_ips ?? 0)} total</span>
        <span class="counter-dot">·</span>
        <span>{progress?.found_servers ?? 0} servers found</span>
        {#if scanRunning && etaStr}
          <span class="counter-dot">·</span>
          <span>ETA: {etaStr}</span>
        {/if}
      </div>
    </div>
  {/if}

  {#if showSettings}
    <div class="settings-panel">
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Proxy (SOCKS5)</span>
          <span class="setting-desc">Route scan traffic through a proxy</span>
        </div>
        <div class="proxy-controls">
          <input
            class="proxy-input"
            type="text"
            placeholder="127.0.0.1:9050"
            bind:value={proxyAddr}
            disabled={scanRunning}
          />
          <button class="detect-btn" onclick={detectProxy} disabled={scanRunning}>
            Auto
          </button>
        </div>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Force Proxy</span>
          <span class="setting-desc">Block scanning if no proxy configured</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={forceProxy} onchange={toggleForceProxy} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">WL Probe</span>
          <span class="setting-desc">Detect whitelist status per server</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={probeWhitelist} onchange={toggleWLProbe} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Concurrency</span>
          <span class="setting-desc">Parallel connections: {concurrency} — higher = faster, uses more bandwidth</span>
        </div>
        <input type="range" min="500" max="10000" step="100" bind:value={concurrency} disabled={scanRunning} style="width:120px;accent-color:var(--accent);" />
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Max CPU: {cpuLimitPct}%</span>
          <span class="setting-desc">Auto-throttles scan if CPU exceeds this limit</span>
        </div>
        <input type="range" min="10" max="100" step="5" bind:value={cpuLimitPct} oninput={() => setCpuLimit(cpuLimitPct)} style="width:120px;accent-color:var(--accent);" />
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Re-scan all ranges</span>
          <span class="setting-desc">Rescan already-scanned /8 ranges (by default skips them)</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={rescanAll} onchange={toggleRescan} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">IPv4 Fast</span>
          <span class="setting-desc">Java (25565) + Bedrock (19132)</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={cycleToggles.ipv4_fast} onchange={() => toggleCycle('ipv4_fast')} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">IPv6 Targeted</span>
          <span class="setting-desc">Hosting provider v6 prefixes</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={cycleToggles.ipv6_targeted} onchange={() => toggleCycle('ipv6_targeted')} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">IPv4 Hot Deep</span>
          <span class="setting-desc">65K ports on server-hosting IPs only</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={cycleToggles.ipv4_hot_deep} onchange={() => toggleCycle('ipv4_hot_deep')} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">IPv4 Deep</span>
          <span class="setting-desc">27 ports, full sweep</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={cycleToggles.ipv4_deep} onchange={() => toggleCycle('ipv4_deep')} disabled={scanRunning} />
        </label>
      </div>
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">IPv6 Deep</span>
          <span class="setting-desc">27 ports, full v6 sweep</span>
        </div>
        <label class="toggle">
          <input type="checkbox" checked={cycleToggles.ipv6_deep} onchange={() => toggleCycle('ipv6_deep')} disabled={scanRunning} />
        </label>
      </div>
      {#if !hasIpv6}
        <div class="warning">
          IPv6 not detected on this network. IPv6 cycles auto-disabled.
          Enable IPv6 on your router or use a proxy with IPv6 support.
        </div>
      {/if}
      <div class="setting-row">
        <div class="setting-info">
          <span class="setting-label">Reset Scanner Memory</span>
          <span class="setting-desc">Clears cycle history, checkpoints, range tracking. Keeps your servers.</span>
        </div>
        <button class="reset-btn" onclick={resetScanner} disabled={scanRunning}>Reset</button>
      </div>
      {#if forceProxy && !proxyAvailable}
        <div class="warning">
          ⚠ Force Proxy is ON but no proxy configured. Scanning will fail.
          Enter a proxy address or click Auto.
        </div>
      {/if}
      {#if proxyAvailable && !proxyAddr}
        <div class="info">
          ℹ Proxy detected automatically at 127.0.0.1:9050
        </div>
      {/if}
    </div>
  {/if}

  <main>
    {#if !dbReady}
      <div class="loading">Connecting...</div>
    {:else if currentView === "dashboard"}
      <Dashboard {stats} {servers} {cycleStats} {progress} {lifetimeScanned} {scanRate} {etaStr} {dbPushStatus} onRefresh={refreshAll} onPushDb={pushDb} />
    {:else if currentView === "servers"}
      <ServerList {servers} {wlReverify} onReverifyWL={reverifyWL} />
    {:else if currentView === "map"}
      <MapView {servers} />
    {:else if currentView === "kitty"}
      <Kitty servers={kittyServers} stats={kittyStats} verifyProgress={kittyVerifyProgress} onSync={kittySync} onVerify={kittyVerify} />
    {:else if currentView === "cycles"}
      <Cycles {cycleData} {progress} onStartCycle={(type) => startCycle(type)} />
    {:else if currentView === "sources"}
      <Sources {sourceStats} onRefresh={() => { api("/serverlist/stats").then(s => { if (s) sourceStats = s; }); }} />
    {/if}
  </main>
</div>

<style>
  .app { min-height: 100vh; display: flex; flex-direction: column; }

  header {
    display: flex; align-items: center; gap: 12px;
    padding: 8px 16px; background: var(--bg2);
    border-bottom: 1px solid var(--border);
    position: sticky; top: 0; z-index: 100;
  }

  .logo { display: flex; align-items: center; gap: 6px; }
  .icon { font-size: 20px; color: var(--accent); }
  h1 { font-size: 16px; font-weight: 600; }
  nav { display: flex; gap: 4px; flex: 1; }

  .nav-btn {
    background: transparent; color: var(--text-dim);
    padding: 5px 10px; font-size: 12px;
  }
  .nav-btn.active { background: var(--bg3); color: var(--text); }

  .actions { display: flex; align-items: center; gap: 6px; }

  .proxy-badge {
    font-size: 10px; font-weight: 700; letter-spacing: 0.5px;
    padding: 2px 8px; border-radius: 4px;
    background: rgba(248, 113, 113, 0.12); color: var(--red);
  }
  .proxy-badge.active {
    background: rgba(74, 222, 128, 0.12); color: var(--green);
  }

  .settings-btn {
    background: transparent; color: var(--text-dim);
    font-size: 16px; padding: 3px 6px;
  }
  .settings-btn:hover { color: var(--text); }

  .status-badge {
    font-size: 10px; padding: 2px 6px; border-radius: 8px;
    background: var(--bg3); color: var(--text-dim);
  }
  .status-badge.scanning { background: rgba(74, 222, 128, 0.15); color: var(--green); }

  .scan-btn {
    background: var(--accent); color: white;
    padding: 5px 12px; font-size: 12px;
  }
  .scan-btn.scanning { background: var(--red); }

  .progress-bar-track {
    background: var(--bg3); height: 24px; position: relative;
    border-bottom: 1px solid var(--border);
  }
  .progress-bar-track.waiting {
    background: rgba(74, 222, 128, 0.08);
  }
  .progress-bar-track.cooling {
    background: rgba(251, 191, 36, 0.08);
  }
  .progress-bar-fill.cooling {
    background: var(--yellow);
  }
  .progress-bar-fill {
    height: 100%; background: var(--accent);
    transition: width 0.5s ease; border-radius: 0 3px 3px 0;
    min-width: 2px;
  }
  .progress-info {
    position: absolute; top: 0; left: 0; right: 0; bottom: 0;
    display: flex; align-items: center; justify-content: center;
    gap: 16px; font-size: 11px; color: white; font-weight: 500;
    text-shadow: 0 1px 2px rgba(0,0,0,0.5);
  }

  .scan-counter {
    display: flex; align-items: center; justify-content: center; gap: 20px;
    padding: 8px 16px; background: var(--bg2);
    border-bottom: 1px solid var(--border);
  }
  .counter-main { display: flex; align-items: baseline; gap: 8px; }
  .counter-num { font-size: 28px; font-weight: 700; color: var(--accent2); font-variant-numeric: tabular-nums; }
  .counter-label { font-size: 12px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .counter-secondary { display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text-dim); }
  .counter-dot { color: var(--border); }

  .settings-panel {
    background: var(--bg2);
    border-bottom: 1px solid var(--border);
    padding: 10px 20px;
    display: flex; flex-direction: column; gap: 8px;
  }

  .setting-row {
    display: flex; align-items: center;
    justify-content: space-between; gap: 12px;
  }
  .setting-info { display: flex; flex-direction: column; gap: 1px; }
  .setting-label { font-size: 12px; font-weight: 500; }
  .setting-desc { font-size: 10px; color: var(--text-dim); }

  .proxy-controls { display: flex; gap: 4px; align-items: center; }
  .proxy-input {
    width: 150px; font-size: 11px; padding: 4px 8px;
    font-family: "SF Mono", "Fira Code", monospace;
  }
  .detect-btn {
    background: var(--bg3); color: var(--text);
    padding: 4px 10px; font-size: 11px;
  }
  .reset-btn {
    background: rgba(248,113,113,0.12); color: var(--red);
    padding: 5px 12px; font-size: 11px; border: 1px solid rgba(248,113,113,0.25);
    border-radius: 4px; cursor: pointer;
  }
  .reset-btn:hover { background: rgba(248,113,113,0.2); }
  .reset-btn:disabled { opacity: 0.4; cursor: not-allowed; }

  .toggle input { accent-color: var(--accent); width: 16px; height: 16px; cursor: pointer; }

  .warning {
    font-size: 11px; color: var(--yellow);
    padding: 6px 10px; background: rgba(251, 191, 36, 0.08);
    border: 1px solid rgba(251, 191, 36, 0.2); border-radius: var(--radius);
  }
  .info {
    font-size: 11px; color: var(--green);
    padding: 6px 10px; background: rgba(74, 222, 128, 0.08);
    border: 1px solid rgba(74, 222, 128, 0.2); border-radius: var(--radius);
  }

  main { flex: 1; padding: 16px; }
  .loading { display: flex; align-items: center; justify-content: center; height: 50vh; color: var(--text-dim); }
</style>
