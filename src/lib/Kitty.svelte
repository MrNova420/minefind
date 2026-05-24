<script>
  let { servers = [], stats = {}, verifyProgress = { verify_total: 0, verify_done: 0, verify_found: 0 }, onSync = () => {}, onVerify = () => {} } = $props();

  function timeAgo(ts) {
    if (!ts) return "—";
    const secs = Math.floor((Date.now() - new Date(ts + "Z").getTime()) / 1000);
    if (secs < 60) return "now";
    if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
    return `${Math.floor(secs / 86400)}d ago`;
  }

  let openCount = $derived(servers.filter(s => s.verified).length);
  let playerCount = $derived(servers.reduce((sum, s) => sum + (s.online_players || 0), 0));
</script>

<div class="kitty">
  <div class="header">
    <div class="title-row">
      <h2>KittyScan Blocklist</h2>
      <span class="source">from <a href="https://github.com/LillySchramm/KittyScanBlocklist" target="_blank" rel="noopener">LillySchramm/KittyScanBlocklist</a></span>
    </div>
    <div class="actions">
      <button class="action-btn" onclick={onSync} disabled={stats?.syncing}>
        {stats?.syncing ? "Syncing..." : "Sync from GitHub"}
      </button>
      <button class="action-btn verify" onclick={onVerify} disabled={stats?.verifying || stats?.syncing}>
        {stats?.verifying
          ? `Verifying ${verifyProgress.verify_done}/${verifyProgress.verify_total}`
          : "Verify All"}
      </button>
    </div>
  </div>

  <div class="grid">
    <div class="card">
      <div class="card-label">Total IPs</div>
      <div class="card-value">{stats?.total ?? 0}</div>
    </div>
    <div class="card accent">
      <div class="card-label">Verified (has server)</div>
      <div class="card-value big">{stats?.verified ?? 0}</div>
      <div class="card-sub">
        {stats?.total > 0 ? Math.round((stats?.verified ?? 0) / (stats?.total ?? 1) * 100) : 0}% of total
      </div>
    </div>
    <div class="card">
      <div class="card-label">Online Players</div>
      <div class="card-value">{playerCount}</div>
    </div>
    <div class="card">
      <div class="card-label">Unverified</div>
      <div class="card-value dim">{(stats?.total ?? 0) - (stats?.verified ?? 0)}</div>
      <div class="card-sub">
        {#if stats?.last_sync}
          Last sync: {timeAgo(stats?.last_sync)}
        {:else}
          Not synced yet
        {/if}
      </div>
    </div>
  </div>

  {#if stats?.verifying && verifyProgress.verify_total > 0}
    <div class="verify-progress">
      <div class="verify-track">
        <div class="verify-fill" style="width: {(verifyProgress.verify_done / verifyProgress.verify_total) * 100}%"></div>
      </div>
      <div class="verify-info">
        <span>Verifying {verifyProgress.verify_done} / {verifyProgress.verify_total}</span>
        <span class="verify-dot">·</span>
        <span>{verifyProgress.verify_found} servers found</span>
        <span class="verify-dot">·</span>
        <span>{Math.round((verifyProgress.verify_done / verifyProgress.verify_total) * 100)}%</span>
      </div>
    </div>
  {/if}

  <div class="table-wrap">
    <div class="table-header">
      <span>Status</span><span>IP</span><span>Port</span><span>MOTD</span><span>Players</span><span>Version</span><span>Ping</span><span>Last Seen</span>
    </div>
    {#if servers.length === 0}
      <div class="empty">
        {stats?.syncing ? "Downloading list from GitHub..." : "No data. Click 'Sync from GitHub' to fetch the KittyScan blocklist."}
      </div>
    {:else}
      {#each servers as s}
        <div class="table-row" class:verified={s.verified}>
          <span class="status-cell">{s.verified ? "✅" : "❌"}</span>
          <span class="mono">{s.ip}</span>
          <span>25565</span>
          <span class="motd">{s.motd ? s.motd.slice(0, 50) : "—"}</span>
          <span>{s.online_players ?? 0}/{s.max_players ?? "?"}</span>
          <span class="small">{s.version ?? "—"}</span>
          <span class="small">{s.ping_ms != null ? `${s.ping_ms}ms` : "—"}</span>
          <span class="small">{timeAgo(s.last_seen)}</span>
        </div>
      {/each}
    {/if}
  </div>
</div>

<style>
  .kitty { max-width: 1400px; margin: 0 auto; }

  .header {
    display: flex; align-items: center; justify-content: space-between;
    margin-bottom: 16px; flex-wrap: wrap; gap: 8px;
  }
  .title-row { display: flex; align-items: baseline; gap: 8px; }
  h2 { font-size: 16px; font-weight: 600; margin: 0; }
  .source { font-size: 11px; color: var(--text-dim); }
  .source a { color: var(--accent2); text-decoration: none; }
  .source a:hover { text-decoration: underline; }

  .actions { display: flex; gap: 6px; }
  .action-btn {
    padding: 6px 14px; font-size: 12px; background: var(--bg3); color: var(--text);
    border: 1px solid var(--border);
  }
  .action-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .action-btn.verify { background: var(--accent2); color: white; border-color: var(--accent2); }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px; margin-bottom: 16px;
  }

  .card {
    background: var(--bg2); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 14px;
  }
  .card.accent { border-color: var(--green); background: rgba(74, 222, 128, 0.05); }
  .card-label { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .card-value { font-size: 22px; font-weight: 700; margin-top: 4px; }
  .card-value.big { font-size: 30px; color: var(--green); }
  .card-value.dim { color: var(--text-dim); }
  .card-sub { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

  .table-wrap {
    background: var(--bg2); border: 1px solid var(--border);
    border-radius: var(--radius); overflow: hidden;
  }
  .table-header, .table-row {
    display: grid;
    grid-template-columns: 48px 150px 50px 1fr 80px 80px 65px 80px;
    gap: 8px; padding: 6px 12px; font-size: 12px; align-items: center;
  }
  .table-header {
    color: var(--text-dim); font-size: 10px; text-transform: uppercase; background: var(--bg3);
  }
  .table-row { border-top: 1px solid var(--border); transition: background 0.15s; }
  .table-row:hover { background: var(--bg3); }
  .table-row.verified { background: rgba(74, 222, 128, 0.03); }
  .status-cell { text-align: center; font-size: 14px; }
  .mono { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; }
  .motd { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .small { font-size: 11px; color: var(--text-dim); }
  .empty { padding: 32px; text-align: center; color: var(--text-dim); font-size: 13px; }

  .verify-progress {
    background: var(--bg2); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 10px 14px; margin-bottom: 12px;
  }
  .verify-track {
    height: 6px; background: var(--bg3); border-radius: 3px; overflow: hidden; margin-bottom: 6px;
  }
  .verify-fill {
    height: 100%; background: var(--accent2); border-radius: 3px; transition: width 0.5s ease;
  }
  .verify-info {
    display: flex; align-items: center; gap: 6px; font-size: 12px; color: var(--text);
  }
  .verify-dot { color: var(--border); }
</style>
