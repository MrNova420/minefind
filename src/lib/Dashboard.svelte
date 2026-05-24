<script>
  let { stats = {}, servers = [], cycleStats = { cycles: 0, total_servers_found: 0, total_targets_scanned: 0 }, progress = {}, lifetimeScanned = 0, onRefresh = () => {} } = $props();

  function categoryLabel(cat) {
    const labels = {
      vanilla_survival: "Vanilla Survival",
      modded: "Modded",
      plugin_heavy: "Plugin Heavy",
      creative: "Creative",
      minigame: "Minigame",
      anarchy: "Anarchy",
      private_group: "Private Group",
      idle: "Idle",
      unknown: "Unknown",
    };
    return labels[cat] || cat;
  }

  function timeAgo(ts) {
    if (!ts) return "—";
    const secs = Math.floor((Date.now() - new Date(ts + "Z").getTime()) / 1000);
    if (secs < 60) return "now";
    if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
    return `${Math.floor(secs / 86400)}d ago`;
  }

  let pctNotWl = $derived(
    stats.total > 0
      ? Math.round((stats.not_whitelisted / stats.total) * 100)
      : 0
  );
  let pctWl = $derived(
    stats.total > 0
      ? Math.round((stats.whitelisted / stats.total) * 100)
      : 0
  );

  let allServers = $derived(servers || []);
</script>

<div class="dashboard">
  <div class="grid">
    <div class="card accent">
      <div class="card-label">Not Whitelisted</div>
      <div class="card-value big">{stats.not_whitelisted ?? 0}</div>
      <div class="card-sub">{pctNotWl}% of known</div>
    </div>
    <div class="card">
      <div class="card-label">Whitelisted</div>
      <div class="card-value red">{stats.whitelisted ?? 0}</div>
      <div class="card-sub">{pctWl}% of known</div>
    </div>
    <div class="card">
      <div class="card-label">Unknown WL</div>
      <div class="card-value dim">{stats.unknown_whitelist ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Total Servers</div>
      <div class="card-value">{stats.total ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Players Online</div>
      <div class="card-value">{stats.total_players ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Modded</div>
      <div class="card-value yellow">{stats.modded ?? 0}</div>
    </div>
    <div class="card scan-progress">
      <div class="card-label">Lifetime Scanned IPs</div>
      <div class="card-value big" style="font-size:38px">{Intl.NumberFormat().format(lifetimeScanned)}</div>
      <div class="card-sub">
        across {cycleStats.cycles ?? 0} cycles
        · {progress?.found_servers ?? 0} servers this cycle
      </div>
    </div>
    <div class="card">
      <div class="card-label">Current Cycle</div>
      <div class="card-value">{Intl.NumberFormat().format(progress?.scanned_ips ?? 0)}</div>
      <div class="mini-progress-track">
        <div class="mini-progress-fill" style="width: {progress.total_ips > 0 ? (progress.scanned_ips / progress.total_ips * 100) : 0}%"></div>
      </div>
      <div class="card-sub">
        {progress.total_ips > 0 ? Math.round(progress.scanned_ips / progress.total_ips * 100) : 0}%
        of {Intl.NumberFormat().format(progress?.total_ips ?? 0)}
        {#if progress.cycle > 0}
          · {progress.found_servers ?? 0} servers
        {/if}
      </div>
    </div>
  </div>

  <div class="cycle-grid">
    <div class="card">
      <div class="card-label">Scan Cycles</div>
      <div class="card-value">{cycleStats.cycles ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Total Found (all cycles)</div>
      <div class="card-value green">{cycleStats.total_servers_found ?? 0}</div>
    </div>
  </div>

  <div class="section">
    <h2>All Servers</h2>
    {#if allServers.length > 0}
      <div class="mini-table">
        <div class="mt-header">
          <span>WL</span><span>IP:Port</span><span>MOTD</span><span>Players</span><span>Version</span><span>Category</span><span>Last Seen</span>
        </div>
        {#each allServers.sort((a, b) => (b.online_players || 0) - (a.online_players || 0)).slice(0, 100) as s}
          <div class="mt-row">
            <span class="wl-cell">
              {#if s.whitelisted === false}✅{:else if s.whitelisted === true}🔒{:else}❓{/if}
            </span>
            <span class="mono">{s.ip}:{s.port ?? 25565}</span>
            <span class="motd">{s.motd?.slice(0, 50)}</span>
            <span>{s.online_players ?? 0}/{s.max_players ?? "?"}</span>
            <span class="small">{s.version}</span>
            <span class="small">{s.category}</span>
            <span class="small">{timeAgo(s.last_seen)}</span>
          </div>
        {/each}
      </div>
    {:else}
      <div class="empty">No servers found. Run a scan to discover servers.</div>
    {/if}
  </div>

  <div class="sections">
    <div class="section">
      <h2>Categories</h2>
      <div class="bar-chart">
        {#each Object.entries(stats?.categories ?? {}) as [cat, count]}
          {@const max = Math.max(...Object.values(stats?.categories ?? {}), 1)}
          <div class="bar-row">
            <span class="bar-label">{categoryLabel(cat)}</span>
            <div class="bar-track">
              <div class="bar-fill" style="width: {(count / max) * 100}%"></div>
            </div>
            <span class="bar-count">{count}</span>
          </div>
        {/each}
      </div>
    </div>
    <div class="section">
      <h2>Versions</h2>
      <div class="bar-chart">
        {#each Object.entries(stats?.versions ?? {}) as [ver, count]}
          {@const max = Math.max(...Object.values(stats?.versions ?? {}), 1)}
          <div class="bar-row">
            <span class="bar-label">{ver}</span>
            <div class="bar-track">
              <div class="bar-fill ver" style="width: {(count / max) * 100}%"></div>
            </div>
            <span class="bar-count">{count}</span>
          </div>
        {/each}
      </div>
    </div>
  </div>
</div>

<style>
  .dashboard { max-width: 1400px; margin: 0 auto; }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(150px, 1fr));
    gap: 12px;
    margin-bottom: 24px;
  }

  .card {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
  }
  .card.accent { border-color: var(--green); background: rgba(74, 222, 128, 0.05); }
  .card-label { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .card-value { font-size: 24px; font-weight: 700; margin-top: 4px; }
  .card-value.big { font-size: 32px; color: var(--green); }
  .card-value.red { color: var(--red); }
  .card-value.yellow { color: var(--yellow); }
  .card-value.dim { color: var(--text-dim); }
  .card-sub { font-size: 12px; color: var(--text-dim); margin-top: 2px; }
  .card-value .sub-text { font-size: 14px; font-weight: 400; color: var(--text-dim); }

  .scan-progress { border-color: var(--accent2); }
  .mini-progress-track {
    height: 4px; background: var(--bg3); border-radius: 2px; margin: 6px 0; overflow: hidden;
  }
  .mini-progress-fill {
    height: 100%; background: var(--accent2); border-radius: 2px; transition: width 0.5s ease;
  }

  h2 { font-size: 15px; font-weight: 600; margin-bottom: 12px; color: var(--text-dim); }

  .sections { display: grid; grid-template-columns: 1fr 1fr; gap: 24px; margin-top: 24px; }
  .section { margin-top: 24px; }

  .bar-chart { display: flex; flex-direction: column; gap: 6px; }
  .bar-row { display: flex; align-items: center; gap: 12px; }
  .bar-label { width: 130px; font-size: 13px; text-align: right; flex-shrink: 0; }
  .bar-track { flex: 1; height: 20px; background: var(--bg3); border-radius: 4px; overflow: hidden; }
  .bar-fill { height: 100%; background: var(--accent); border-radius: 4px; transition: width 0.5s ease; min-width: 2px; }
  .bar-fill.ver { background: var(--accent2); }
  .bar-count { width: 40px; font-size: 13px; color: var(--text-dim); text-align: right; }

  .mini-table { background: var(--bg2); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .cycle-grid { display: grid; grid-template-columns: 1fr 1fr; gap: 12px; margin-bottom: 12px; }
  .cycle-grid .card { padding: 10px; }
  .cycle-grid .card-value.green { color: var(--green); }

  .mt-header, .mt-row {
    display: grid;
    grid-template-columns: 32px 155px 1fr 75px 70px 85px 70px;
    gap: 6px;
    padding: 6px 12px;
    font-size: 12px;
    align-items: center;
  }
  .mt-header { color: var(--text-dim); font-size: 10px; text-transform: uppercase; background: var(--bg3); }
  .mt-row { border-top: 1px solid var(--border); transition: background 0.15s; }
  .mt-row:hover { background: var(--bg3); }
  .wl-cell { text-align: center; font-size: 13px; }
  .mono { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; }
  .motd { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .small { font-size: 11px; color: var(--text-dim); }
  .empty { padding: 24px; text-align: center; color: var(--text-dim); font-size: 13px; background: var(--bg2); border-radius: var(--radius); border: 1px solid var(--border); }
</style>
