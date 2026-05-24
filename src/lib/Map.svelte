<script>
  let { servers = [] } = $props();

  let grouped = $derived(
    (servers || []).reduce((acc, s) => {
      const firstOctet = (s.ip || "").split(".")[0];
      const region = ipRegion(firstOctet);
      if (!acc[region]) acc[region] = [];
      acc[region].push(s);
      return acc;
    }, {})
  );

  function ipRegion(first) {
    const n = parseInt(first);
    if (n >= 1 && n <= 9) return "US East";
    if (n >= 13 && n <= 56) return "US West";
    if (n >= 62 && n <= 95) return "Europe";
    if (n >= 88 && n <= 95) return "Europe";
    if (n >= 130 && n <= 140) return "US East";
    if (n >= 141 && n <= 195) return "Europe";
    if (n >= 196 && n <= 200) return "South America";
    if (n >= 201 && n <= 223) return "Asia";
    return "Unknown";
  }
</script>

<div class="map-view">
  <h2>Server Distribution by Region</h2>
  <div class="grid">
    {#each Object.entries(grouped) as [region, regionServers]}
      <div class="region-card">
        <div class="region-header">
          <span class="region-name">{region}</span>
          <span class="region-count">{regionServers.length}</span>
        </div>
        <div class="region-stats">
          <div>
            Online:
            {regionServers.reduce((s, v) => s + (v.online_players || 0), 0)}
          </div>
          <div>
            Not WL: {regionServers.filter((s) => s.whitelisted === false).length}
          </div>
          <div>
            Modded: {regionServers.filter((s) => s.modded).length}
          </div>
        </div>
        <div class="mini-list">
          {#each regionServers.slice(0, 5) as s}
            <div class="mini-row">
              <span class="ip">{s.ip}:{s.port ?? 25565}</span>
              <span class="players">{s.online_players ?? 0}/{s.max_players ?? "?"}</span>
              <span class="version">{s.version?.slice(0, 8)}</span>
            </div>
          {/each}
          {#if regionServers.length > 5}
            <div class="more">+{regionServers.length - 5} more</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .map-view {
    max-width: 1200px;
    margin: 0 auto;
  }

  h2 {
    font-size: 16px;
    font-weight: 600;
    margin-bottom: 16px;
    color: var(--text-dim);
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 16px;
  }

  .region-card {
    background: var(--bg2);
    border: 1px solid var(--border);
    border-radius: var(--radius);
    padding: 16px;
  }

  .region-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
  }

  .region-name {
    font-weight: 600;
    font-size: 15px;
  }

  .region-count {
    background: var(--bg3);
    padding: 2px 8px;
    border-radius: 10px;
    font-size: 12px;
    color: var(--text-dim);
  }

  .region-stats {
    display: flex;
    gap: 12px;
    font-size: 12px;
    color: var(--text-dim);
    margin-bottom: 12px;
  }

  .mini-list {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .mini-row {
    display: flex;
    gap: 8px;
    font-size: 11px;
    font-family: "SF Mono", "Fira Code", monospace;
    color: var(--text-dim);
  }

  .mini-row .ip {
    color: var(--text);
    flex: 1;
  }

  .more {
    font-size: 11px;
    color: var(--accent);
    margin-top: 4px;
  }
</style>
