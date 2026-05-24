<script>
  let { servers = [] } = $props();

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
        if (categoryFilter !== "all" && s.category !== categoryFilter) return false;
        if (wlFilter === "yes" && s.whitelisted !== true) return false;
        if (wlFilter === "no" && s.whitelisted !== false) return false;
        if (wlFilter === "unknown" && s.whitelisted !== null && s.whitelisted !== undefined) return false;
        return true;
      })
      .sort((a, b) => {
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
    <span class="count">{filtered.length} servers</span>
  </div>

  <div class="table">
    <div class="table-header">
      <span class="col-ip">IP:Port</span>
      <span class="col-motd">MOTD</span>
      <span class="col-players">Players</span>
      <span class="col-version">Version</span>
      <span class="col-category">Category</span>
      <span class="col-wl">WL</span>
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
        </span>
        <span class="col-wl">
          {#if server.whitelisted === true}
            <span class="wl-dot red"></span>
          {:else if server.whitelisted === false}
            <span class="wl-dot green"></span>
          {:else}
            <span class="wl-dot dim"></span>
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
    margin-bottom: 16px;
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

  .table-header,
  .table-row {
    display: grid;
    grid-template-columns: 160px 1fr 100px 80px 110px 40px 60px;
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
  }

  .wl-dot {
    display: inline-block;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .wl-dot.green {
    background: var(--green);
  }
  .wl-dot.red {
    background: var(--red);
  }
  .wl-dot.dim {
    background: var(--text-dim);
  }

  .empty {
    text-align: center;
    padding: 48px;
    color: var(--text-dim);
  }
</style>
