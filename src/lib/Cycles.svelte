<script>
  let { cycleData = { summary: {}, history: [], checkpoint: null } } = $props();

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
      <div class="card-label">Total Servers Found</div>
      <div class="card-value green">{formatNum(cycleData.summary?.total_servers_found ?? 0)}</div>
    </div>
  </div>

  {#if cycleData.checkpoint}
    <div class="checkpoint-banner">
      <span>Saved checkpoint exists: {cycleLabel(cycleData.checkpoint.cycle_type)} at {formatNum(cycleData.checkpoint.scanned_ips)} IPs ({cycleData.checkpoint.found_servers} servers found)</span>
      <span class="note">Will auto-resume when scan starts</span>
    </div>
  {/if}

  <div class="table-wrap">
    <div class="table-header">
      <span>#</span><span>Type</span><span>Scanned</span><span>Found</span><span>Started</span><span>Finished</span>
    </div>
    {#if cycleData.history?.length > 0}
      {#each cycleData.history as c}
        <div class="table-row">
          <span class="cycle-num">{c.cycle}</span>
          <span class="cycle-type">{cycleLabel(c.cycle_type)}</span>
          <span class="mono">{formatNum(c.targets_scanned)}</span>
          <span class="found">{c.servers_found ?? 0}</span>
          <span class="small">{timeAgo(c.started_at)}</span>
          <span class="small">{timeAgo(c.finished_at)}</span>
        </div>
      {/each}
    {:else}
      <div class="empty">No cycles completed yet. Start a scan to begin gathering data.</div>
    {/if}
  </div>
</div>

<style>
  .cycles { max-width: 1000px; margin: 0 auto; }

  h2 { font-size: 16px; font-weight: 600; margin-bottom: 16px; }

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

  .table-wrap {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    overflow: hidden;
  }
  .table-header, .table-row {
    display: grid;
    grid-template-columns: 50px 140px 120px 80px 100px 1fr;
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

  .cycle-num { font-weight: 600; color: var(--accent2); }
  .cycle-type { font-size: 11px; }
  .found { color: var(--green); font-weight: 500; }
  .mono { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; }
  .small { font-size: 11px; color: var(--text-dim); }
  .empty { padding: 32px; text-align: center; color: var(--text-dim); font-size: 13px; }
</style>
