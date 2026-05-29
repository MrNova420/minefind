<script>
  let { servers = [] } = $props();

  function detectProvider(ip) {
    const o = parseInt((ip || "").split(".")[0]);
    // Hetzner
    if (o === 5 || o === 49 || o === 65 || o === 88 || o === 95 || o === 116 || o === 136 || o === 142 || o === 144 || o === 148 || o === 157 || o === 167 || o === 168 || o === 176 || o === 188 || o === 195 || o === 213) return "Hetzner";
    // OVH
    if (o === 51 || o === 54 || o === 141 || o === 145) return "OVH";
    // AWS
    if (o === 3 || o === 13 || o === 15 || o === 18 || o === 35 || o === 43 || o === 44 || o === 52 || o === 54 || o === 63 || o === 64 || o === 75 || o === 96 || o === 99 || o === 107 || o === 140 || o === 150 || o === 157 || o === 176 || o === 184 || o === 185 || o === 204 || o === 216) return "AWS";
    // DigitalOcean
    if (o === 137 || o === 142 || o === 157 || o === 159 || o === 161 || o === 165 || o === 167 || o === 170) return "DigitalOcean";
    // Vultr
    if (o === 45 || o === 108 || o === 149 || o === 155 || o === 199 || o === 207) return "Vultr";
    // Linode
    if (o === 23 || o === 45 || o === 50 || o === 72 || o === 96 || o === 139 || o === 172) return "Linode";
    // Contabo
    if (o === 161 || o === 173 || o === 178 || o === 185) return "Contabo";
    return "Other";
  }

  let grouped = $derived(
    (servers || []).reduce((acc, s) => {
      const provider = detectProvider(s.ip);
      if (!acc[provider]) acc[provider] = { servers: [], players: 0, modded: 0, openWL: 0 };
      acc[provider].servers.push(s);
      acc[provider].players += s.online_players || 0;
      if (s.modded) acc[provider].modded++;
      if (s.whitelisted === false) acc[provider].openWL++;
      return acc;
    }, {})
  );

  let sorted = $derived(
    Object.entries(grouped).sort((a, b) => b[1].servers.length - a[1].servers.length)
  );

  let maxServers = $derived(Math.max(...sorted.map(([, d]) => d.servers.length), 1));
</script>

<div class="map-view">
  <h2>Server Distribution by Provider</h2>
  <div class="grid">
    {#each sorted as [provider, data]}
      <div class="provider-card">
        <div class="prov-header">
          <span class="prov-name">{provider}</span>
          <span class="prov-count">{data.servers.length}</span>
        </div>
        <div class="prov-bar">
          <div class="prov-fill" style="width: {(data.servers.length / maxServers) * 100}%"></div>
        </div>
        <div class="prov-stats">
          <span>{data.players.toLocaleString()} players</span>
          <span class="dot">·</span>
          <span>{data.openWL} open</span>
          <span class="dot">·</span>
          <span>{data.modded} modded</span>
        </div>
        <div class="mini-list">
          {#each data.servers.slice(0, 3) as s}
            <div class="mini-row">
              <span class="ip">{s.ip}:{s.port ?? 25565}</span>
              <span class="players">{s.online_players ?? 0}/{s.max_players ?? "?"}</span>
              <span class="version">{s.version?.slice(0, 12)}</span>
            </div>
          {/each}
          {#if data.servers.length > 3}
            <div class="more">+{data.servers.length - 3} more servers</div>
          {/if}
        </div>
      </div>
    {/each}
  </div>
</div>

<style>
  .map-view { max-width: 1200px; margin: 0 auto; }
  h2 { font-size: 16px; font-weight: 600; margin-bottom: 16px; }
  .grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(280px, 1fr)); gap: 14px; }

  .provider-card {
    background: var(--bg2); border: 1px solid var(--border);
    border-radius: var(--radius); padding: 14px;
  }
  .prov-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 6px; }
  .prov-name { font-weight: 600; font-size: 14px; }
  .prov-count { background: var(--bg3); padding: 2px 8px; border-radius: 10px; font-size: 12px; color: var(--text-dim); }

  .prov-bar { height: 6px; background: var(--bg3); border-radius: 3px; overflow: hidden; margin-bottom: 8px; }
  .prov-fill { height: 100%; background: var(--accent); border-radius: 3px; transition: width 0.5s; min-width: 2px; }

  .prov-stats { display: flex; gap: 6px; font-size: 11px; color: var(--text-dim); margin-bottom: 10px; }
  .dot { color: var(--border); }

  .mini-list { display: flex; flex-direction: column; gap: 3px; }
  .mini-row { display: flex; gap: 6px; font-size: 11px; font-family: "SF Mono", "Fira Code", monospace; color: var(--text-dim); }
  .mini-row .ip { color: var(--text); flex: 1; }
  .more { font-size: 11px; color: var(--accent); margin-top: 4px; }
</style>
