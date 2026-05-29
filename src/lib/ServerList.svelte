<script>
  let { servers = [], wlReverify = { running: false, total: 0, done: 0 }, onReverifyWL = () => {}, onDedup = () => {} } = $props();

  let dedupResult = $state(null);
  let portFilter = $state("all");

  let search = $state("");
  let categoryFilter = $state("all");
  let wlFilter = $state("all");
  let sortBy = $state("players");

  let categories = $derived(
    [...new Set(servers.map((s) => s.category || "unknown"))]
  );

  let filtered = $derived(
    servers
      .filter((s) => {
        if (search && !s.motd?.toLowerCase().includes(search.toLowerCase()) &&
            !s.ip?.includes(search)) return false;
        if (categoryFilter !== "all" && s.category !== categoryFilter)         return false;
        if (portFilter === "25565" && (s.port || 25565) !== 25565) return false;
        if (portFilter === "19132" && s.port !== 19132) return false;
        if (wlFilter === "yes" && s.whitelisted !== true) return false;
        if (wlFilter === "no" && s.whitelisted !== false) return false;
        if (wlFilter === "unknown" && s.whitelisted !== null && s.whitelisted !== undefined) return false;
        return true;
      })
      .toSorted((a, b) => {
        if (sortBy === "players") return (b.online_players || 0) - (a.online_players || 0);
        if (sortBy === "ping") return (a.ping_ms || 0) - (b.ping_ms || 0);
        if (sortBy === "motd") return (a.motd || "").localeCompare(b.motd || "");
        return 0;
      })
  );

  function categoryBadge(cat) {
    const colors = {
      vanilla_survival: "#4ade80",
      modded: "#fbbf24",
      plugin_heavy: "#60a5fa",
      creative: "#f472b6",
      minigame: "#a78bfa",
      anarchy: "#f87171",
      private_group: "#34d399",
      idle: "#9ca3af",
      unknown: "#6b7280",
    };
    return colors[cat] || "#6b7280";
  }

  let wlPct = $derived(wlReverify.total > 0 ? Math.round((wlReverify.done / wlReverify.total) * 100) : 0);
</script>

<div class="server-list">
  <div class="filters">
    <input
      type="text"
      placeholder="Search MOTD or IP..."
      bind:value={search}
    />
    <select bind:value={categoryFilter}>
      <option value="all">All Categories</option>
      {#each categories as cat}
        <option value={cat}>{cat}</option>
      {/each}
    </select>
    <select bind:value={wlFilter}>
      <option value="all">Whitelist: Any</option>
      <option value="no">Not Whitelisted</option>
      <option value="yes">Whitelisted</option>
      <option value="unknown">Unknown</option>
    </select>
    <select bind:value={sortBy}>
      <option value="players">Sort: Players</option>
      <option value="ping">Sort: Ping</option>
      <option value="motd">Sort: Name</option>
    </select>
    <select bind:value={portFilter}>
      <option value="all">Port: All</option>
      <option value="25565">Port: 25565</option>
      <option value="19132">Port: 19132</option>
    </select>
    <button class="export-btn" onclick={() => { const blob = new Blob([JSON.stringify(servers, null, 2)], {type: 'application/json'}); const u = URL.createObjectURL(blob); const a = document.createElement('a'); a.href = u; a.download = 'minefind-servers.json'; a.click(); }}>Export JSON</button>
    <button class="reverify-btn" onclick={onReverifyWL} disabled={wlReverify.running}>
      {wlReverify.running ? `Reverifying ${wlReverify.done}/${wlReverify.total}...` : "Re-verify WL"}
    </button>
    <button class="dedup-btn" onclick={async () => { const r = await fetch('/api/servers/dedup', {method:'POST'}); dedupResult = await r.json(); }}>
      Check Dupes
    </button>
    <input type="text" class="srv-input" placeholder="mc.example.com" id="srvDomain" />
    <button class="srv-btn" onclick={async () => {
      const domain = document.getElementById('srvDomain').value.trim();
      if (!domain) return;
      const r = await fetch(`/api/srv/${domain}`);
      const d = await r.json();
      if (d.records?.length > 0) {
        let added = 0;
        for (const rec of d.records) {
          const r2 = await fetch(`/api/ping/${rec.host}?port=${rec.port}`);
          const d2 = await r2.json();
          if (d2?.found) added++;
        }
        alert(`Resolved ${d.records.length} records, ${added} servers found`);
      } else {
        alert(`No SRV records found for ${domain}`);
      }
    }}>
      SRV Lookup
    </button>
    <span class="count">{filtered.length} servers</span>
  </div>

  {#if wlReverify.running}
    <div class="reverify-bar">
      <div class="reverify-fill" style="width: {wlPct}%"></div>
      <span class="reverify-text">{wlReverify.done}/{wlReverify.total} servers checked · {wlPct}%</span>
    </div>
  {/if}

  {#if dedupResult}
    <div class="dedup-result" class:clean={!dedupResult.duplicates}>
      {#if dedupResult.duplicates > 0}
        Found {dedupResult.duplicates} duplicate pairs across {dedupResult.total} rows ({dedupResult.unique} unique)
      {:else}
        Clean — {dedupResult.total} rows, {dedupResult.unique} unique, 0 duplicates
      {/if}
    </div>
  {/if}

  <div class="table">
    <div class="table-header">
      <span class="col-ip">IP:Port</span>
      <span class="col-motd">MOTD</span>
      <span class="col-players">Players</span>
      <span class="col-version">Version</span>
      <span class="col-category">Category / WL</span>
      <span class="col-ping">Ping</span>
    </div>

    {#each filtered as server}
      <div class="table-row">
        <span class="col-ip mono">{server.ip}:{server.port ?? 25565}</span>
        <span class="col-motd">{server.motd?.slice(0, 60)}</span>
        <span class="col-players">
          {server.online_players ?? 0}/{server.max_players ?? "?"}
        </span>
        <span class="col-version small">{server.version}</span>
        <span class="col-category">
          <span
            class="badge"
            style="background: {categoryBadge(server.category)}22; color: {categoryBadge(server.category)}; border-color: {categoryBadge(server.category)}44"
          >
            {server.category}
          </span>
          {#if server.whitelisted === false}
            <span class="tag tag-open">Open</span>
          {:else if server.whitelisted === true}
            <span class="tag tag-wl">WL</span>
          {:else}
            <span class="tag tag-unknown">?</span>
          {/if}
        </span>
        <span class="col-ping small">{server.ping_ms ?? "?"}ms</span>
      </div>
    {/each}
  </div>

  {#if filtered.length === 0}
    <div class="empty">No servers found matching filters</div>
  {/if}
</div>

<style>
  .server-list {
    max-width: 1400px;
    margin: 0 auto;
  }

  .filters {
    display: flex;
    gap: 8px;
    margin-bottom: 12px;
    flex-wrap: wrap;
    align-items: center;
  }

  .filters input {
    flex: 1;
    min-width: 200px;
  }

  .count {
    font-size: 13px;
    color: var(--text-dim);
    margin-left: auto;
  }

  .reverify-btn {
    padding: 6px 14px;
    font-size: 12px;
    background: var(--accent2);
    color: white;
    border: 1px solid var(--accent2);
    border-radius: 4px;
    cursor: pointer;
    white-space: nowrap;
  }
  .reverify-btn:disabled { opacity: 0.6; cursor: not-allowed; }

  .dedup-btn {
    padding: 6px 14px; font-size: 12px; background: var(--bg3); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; cursor: pointer; white-space: nowrap;
  }
  .export-btn {
    padding: 6px 14px; font-size: 12px; background: var(--bg3); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; cursor: pointer; white-space: nowrap;
  }
  .srv-input {
    width: 160px; padding: 5px 8px; font-size: 11px;
    font-family: "SF Mono", "Fira Code", monospace;
  }
  .srv-btn {
    padding: 6px 14px; font-size: 12px; background: var(--accent2); color: white;
    border: 1px solid var(--accent2); border-radius: 4px; cursor: pointer; white-space: nowrap;
  }
  .dedup-result {
    padding: 8px 12px; margin-bottom: 12px; border-radius: var(--radius);
    font-size: 11px; background: rgba(248, 113, 113, 0.08); color: var(--red);
    border: 1px solid rgba(248, 113, 113, 0.15);
  }
  .dedup-result.clean {
    background: rgba(74, 222, 128, 0.06); color: var(--green);
    border-color: rgba(74, 222, 128, 0.15);
  }

  .reverify-bar {
    height: 22px;
    background: var(--bg3);
    border-radius: 4px;
    position: relative;
    margin-bottom: 12px;
    overflow: hidden;
  }
  .reverify-fill {
    height: 100%;
    background: var(--accent2);
    border-radius: 4px;
    transition: width 0.5s ease;
  }
  .reverify-text {
    position: absolute;
    top: 0; left: 0; right: 0; bottom: 0;
    display: flex; align-items: center; justify-content: center;
    font-size: 11px; color: white; font-weight: 500;
    text-shadow: 0 1px 2px rgba(0,0,0,0.4);
  }

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 160px 1fr 100px 80px 190px 70px;
    gap: 8px;
    align-items: center;
    padding: 8px 12px;
    font-size: 13px;
  }

  .table-header {
    color: var(--text-dim);
    font-size: 11px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    background: var(--bg);
  }

  .table-row {
    border-bottom: 1px solid var(--border);
    transition: background 0.15s;
  }

  .table-row:hover {
    background: var(--bg2);
  }

  .mono {
    font-family: "SF Mono", "Fira Code", monospace;
    font-size: 12px;
  }

  .small {
    font-size: 12px;
    color: var(--text-dim);
  }

  .badge {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 11px;
    border: 1px solid;
    font-weight: 500;
    margin-right: 6px;
  }

  .tag {
    display: inline-block;
    padding: 2px 6px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .tag-open {
    background: rgba(74, 222, 128, 0.15);
    color: var(--green);
  }
  .tag-wl {
    background: rgba(248, 113, 113, 0.15);
    color: var(--red);
  }
  .tag-unknown {
    background: rgba(156, 163, 175, 0.15);
    color: var(--text-dim);
  }

  .empty {
    text-align: center;
    padding: 48px;
    color: var(--text-dim);
  }
</style>
