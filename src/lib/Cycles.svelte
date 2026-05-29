<script>
  let { cycleData = { summary: {}, history: [], checkpoint: null }, progress = {}, onStartCycle = () => {} } = $props();

  function formatNum(n) {
    if (n == null) return "—";
    return Intl.NumberFormat().format(n);
  }

  function timeAgo(ts) {
    if (!ts) return "—";
    const secs = Math.floor((Date.now() - new Date(ts + "Z").getTime()) / 1000);
    if (secs < 60) return "now";
    if (secs < 3600) return `${Math.floor(secs / 60)}m ago`;
    if (secs < 86400) return `${Math.floor(secs / 3600)}h ago`;
    return `${Math.floor(secs / 86400)}d ago`;
  }

  function cycleLabel(type) {
    const labels = {
      ipv4_fast: "IPv4 Fast",
      ipv6_targeted: "IPv6 Targeted",
      ipv4_deep: "IPv4 Deep",
      ipv6_deep: "IPv6 Deep",
    };
    return labels[type] || type || "—";
  }

  const allTypes = ["ipv4_fast", "ipv6_targeted", "ipv4_deep", "ipv6_deep"];

  let typeSummary = $derived(
    allTypes.map(t => {
      const matches = (cycleData.history || []).filter(c => c.cycle_type === t);
      // Include checkpoint data if it matches this type
      const cp = cycleData.checkpoint;
      const cpMatch = cp && cp.cycle_type === t;
      return {
        type: t,
        count: matches.length + (cpMatch ? 1 : 0),
        scanned: matches.reduce((s, c) => s + (c.targets_scanned || 0), 0) + (cpMatch ? (cp.scanned_ips || 0) : 0),
        found: matches.reduce((s, c) => s + (c.servers_found || 0), 0) + (cpMatch ? (cp.found_servers || 0) : 0),
        last: matches.length ? matches.reduce((a, c) => (c.finished_at && c.finished_at > a) ? c.finished_at : a, "") : (cpMatch ? "Active" : null),
        active: cpMatch,
      };
    })
  );
</script>

<div class="cycles">
  <h2>Scan Cycles</h2>

  <div class="grid">
    <div class="card accent">
      <div class="card-label">Total Cycles</div>
      <div class="card-value">{cycleData.summary?.cycles ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Total Scanned</div>
      <div class="card-value">{formatNum(cycleData.summary?.total_targets_scanned ?? 0)}</div>
    </div>
    <div class="card">
      <div class="card-label">Unique Servers</div>
      <div class="card-value green">{cycleData.summary?.actual_servers ?? 0}</div>
    </div>
  </div>

  {#if cycleData.checkpoint}
    <div class="checkpoint-banner">
      <span>Saved checkpoint: {cycleLabel(cycleData.checkpoint.cycle_type)} at {formatNum(cycleData.checkpoint.scanned_ips)} IPs ({cycleData.checkpoint.found_servers} found)</span>
      <span class="note">Auto-resumes on next Scan</span>
    </div>
  {/if}

  <h3>Continue by Type</h3>
  {#if progress?.running && progress.cycle_type}
    <div class="active-cycle-banner">
      <span class="active-dot"></span>
      <span>Active: <strong>{cycleLabel(progress.cycle_type)}</strong> — {formatNum(progress.scanned_ips)} IPs</span>
      <span class="active-pct">{progress.total_ips > 0 ? Math.round(progress.scanned_ips / progress.total_ips * 100) : 0}%</span>
    </div>
  {/if}
  <div class="table-wrap">
    <div class="table-header type-header">
      <span>Cycle Type</span><span>Runs</span><span>Total Scanned</span><span>Total Found</span><span>Last Run</span><span></span>
    </div>
    {#each typeSummary as t}
      <div class="table-row type-row">
        <span class="cycle-type">{cycleLabel(t.type)}</span>
        <span>{t.count}</span>
        <span class="mono">{formatNum(t.scanned)}</span>
        <span class="found">{t.found}</span>
        <span class="small">
          {t.active ? "Active" : timeAgo(t.last)}
        </span>
        <span>
          <button class="continue-btn" onclick={() => onStartCycle(t.type)}>Continue</button>
        </span>
      </div>
    {/each}
  </div>

  <h3 style="margin-top: 24px">Full History</h3>
  <div class="table-wrap">
    <div class="table-header hist-header">
      <span>#</span><span>Type</span><span>Scanned</span><span>Found</span><span>Started</span><span>Finished</span><span></span>
    </div>
    {#if cycleData.history?.length > 0}
      {#each cycleData.history as c}
        <div class="table-row hist-row">
          <span class="cycle-num">{c.cycle}</span>
          <span class="cycle-type">{cycleLabel(c.cycle_type)}</span>
          <span class="mono">{formatNum(c.targets_scanned)}</span>
          <span class="found">{c.servers_found ?? 0}</span>
          <span class="small">{timeAgo(c.started_at)}</span>
          <span class="small">{timeAgo(c.finished_at)}</span>
          <span>
            <button class="continue-btn" onclick={() => onStartCycle(c.cycle_type)}>Continue</button>
          </span>
        </div>
      {/each}
    {:else if cycleData.checkpoint}
      <div class="empty" style="color:var(--accent2)">No completed cycles — checkpoint paused at {cycleLabel(cycleData.checkpoint.cycle_type)} ({formatNum(cycleData.checkpoint.scanned_ips)} IPs, {cycleData.checkpoint.found_servers} found). Use Continue or Resume to keep going.</div>
    {:else}
      <div class="empty">No cycles completed yet. Start a scan to begin.</div>
    {/if}
  </div>
</div>

<style>
  .cycles { max-width: 1000px; margin: 0 auto; }

  h2 { font-size: 16px; font-weight: 600; margin-bottom: 16px; }
  h3 { font-size: 14px; font-weight: 600; margin-bottom: 10px; color: var(--text-dim); }

  .grid {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 12px;
    margin-bottom: 16px;
  }

  .card {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 14px;
  }
  .card.accent { border-color: var(--accent2); }
  .card-label { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .card-value { font-size: 26px; font-weight: 700; margin-top: 4px; }
  .card-value.green { color: var(--green); }

  .checkpoint-banner {
    background: rgba(251, 191, 36, 0.08);
    border: 1px solid rgba(251, 191, 36, 0.2);
    border-radius: var(--radius);
    padding: 10px 14px;
    margin-bottom: 16px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    font-size: 12px;
  }
  .checkpoint-banner .note { color: var(--text-dim); font-size: 11px; }

  .active-cycle-banner {
    background: rgba(96, 165, 250, 0.08);
    border: 1px solid rgba(96, 165, 250, 0.2);
    border-radius: var(--radius);
    padding: 10px 14px;
    margin-bottom: 16px;
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 13px;
  }
  .active-dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: var(--accent2);
    animation: pulse 1.5s ease-in-out infinite;
  }
  @keyframes pulse {
    0%, 100% { opacity: 1; }
    50% { opacity: 0.4; }
  }
  .active-pct {
    margin-left: auto; font-weight: 700; color: var(--accent2); font-size: 14px;
  }

  .table-wrap {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
    margin-bottom: 16px;
  }
  .table-header, .table-row {
    display: grid;
    gap: 8px;
    padding: 6px 12px;
    font-size: 12px;
    align-items: center;
  }
  .table-header {
    color: var(--text-dim);
    font-size: 10px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    background: var(--bg3);
  }
  .table-row { border-top: 1px solid var(--border); }
  .table-row:hover { background: var(--bg3); }

  .type-header { grid-template-columns: 140px 50px 120px 80px 100px 90px; }
  .type-row { grid-template-columns: 140px 50px 120px 80px 100px 90px; }
  .hist-header { grid-template-columns: 50px 130px 120px 80px 100px 1fr 90px; }
  .hist-row { grid-template-columns: 50px 130px 120px 80px 100px 1fr 90px; }

  .cycle-num { font-weight: 600; color: var(--accent2); }
  .cycle-type { font-size: 11px; }
  .found { color: var(--green); font-weight: 500; }
  .mono { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; }
  .small { font-size: 11px; color: var(--text-dim); }
  .empty { padding: 32px; text-align: center; color: var(--text-dim); font-size: 13px; }

  .continue-btn {
    padding: 3px 10px; font-size: 11px;
    background: var(--accent); color: white;
    border: 1px solid var(--accent); border-radius: 4px; cursor: pointer;
  }
  .continue-btn:hover { background: var(--accent2); }
</style>
