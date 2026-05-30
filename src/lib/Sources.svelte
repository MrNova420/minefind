<script>
  let { onRefresh = () => {}, sourceStats = { total: 0 } } = $props();
  let sourceList = $state([]);
  let scraping = $state(false);
  let scrapeResult = $state(null);

  async function doScrape() {
    scraping = true;
    const r = await fetch('/api/serverlist/scrape', { method: 'POST' });
    scrapeResult = await r.json();
    scraping = false;
  }

  async function loadList() {
    const r = await fetch('/api/serverlist/list');
    sourceList = await r.json() || [];
    onRefresh();
  }

  import { onMount } from "svelte";
  onMount(() => { loadList(); });
</script>

<div class="sources">
  <h2>Server List Sources</h2>
  <p class="desc">Scraped from public Minecraft server directories. Completely separate from your scanner database.</p>

  <div class="grid">
    <div class="card accent">
      <div class="card-label">IPs Scraped</div>
      <div class="card-value">{sourceStats.total ?? 0}</div>
    </div>
    <div class="card">
      <div class="card-label">Sources</div>
      <div class="card-value dim">{scrapeResult?.results?.length ?? 18}</div>
      <div class="card-sub">18 server lists</div>
    </div>
    <div class="card push-card">
      <div class="card-label">Scrape All Sources</div>
      <button class="scrape-btn" onclick={doScrape} disabled={scraping}>
        {scraping ? "Scraping..." : "Scrape All Lists"}
      </button>
      {#if scrapeResult}
        <div class="scrape-result">
          {#if scrapeResult.ok}
            +{scrapeResult.total_added} new · {scrapeResult.db_total} total
          {:else}
            Failed
          {/if}
        </div>
      {/if}
    </div>
    {#if scrapeResult?.results}
      <div class="card" style="grid-column: 1 / -1;">
        <div class="card-label">Latest Scrape Results</div>
        <div class="results-grid">
          {#each scrapeResult.results as r}
            <div class="result-row">
              <span class="src-name">{r.source}</span>
              <span class="src-stat">{r.found ?? 0} found</span>
              <span class="src-added">{r.error ? 'err' : ('+' + (r.added ?? 0))}</span>
            </div>
          {/each}
        </div>
      </div>
    {/if}
  </div>

  <div class="table-wrap">
    <div class="table-header">
      <span>IP</span><span>Port</span><span>Source</span>
    </div>
    {#if sourceList.length > 0}
      {#each sourceList as s}
        <div class="table-row">
          <span class="mono">{s.ip}</span>
          <span>{s.port ?? 25565}</span>
          <span class="small">{s.source ?? "manual"}</span>
        </div>
      {/each}
    {:else}
      <div class="empty">No scraped servers yet. Click "Scrape Server Lists" to fetch from public directories.</div>
    {/if}
  </div>
</div>

<style>
  .sources { max-width: 1000px; margin: 0 auto; }
  h2 { font-size: 16px; font-weight: 600; margin-bottom: 4px; }
  .desc { font-size: 12px; color: var(--text-dim); margin-bottom: 16px; }

  .grid { display: grid; grid-template-columns: repeat(3, 1fr); gap: 12px; margin-bottom: 16px; }
  .card { background: var(--bg2); border: 1px solid var(--border); border-radius: var(--radius); padding: 14px; }
  .card.accent { border-color: var(--accent2); }
  .card-label { font-size: 11px; color: var(--text-dim); text-transform: uppercase; letter-spacing: 0.5px; }
  .card-value { font-size: 26px; font-weight: 700; margin-top: 4px; }
  .card-value.dim { color: var(--text-dim); font-size: 18px; }
  .card-sub { font-size: 11px; color: var(--text-dim); margin-top: 2px; }

  .scrape-btn { margin-top: 4px; padding: 5px 12px; font-size: 11px; background: var(--accent2); color: white; border: 1px solid var(--accent2); border-radius: 4px; cursor: pointer; width: 100%; }
  .scrape-btn:disabled { opacity: 0.5; cursor: not-allowed; }
  .scrape-result { font-size: 11px; color: var(--green); margin-top: 6px; }

  .table-wrap { background: var(--bg2); border: 1px solid var(--border); border-radius: var(--radius); overflow: hidden; }
  .table-header, .table-row { display: grid; grid-template-columns: 1fr 80px 1fr; gap: 8px; padding: 6px 12px; font-size: 12px; align-items: center; }
  .table-header { color: var(--text-dim); font-size: 10px; text-transform: uppercase; letter-spacing: 0.5px; background: var(--bg3); }
  .table-row { border-top: 1px solid var(--border); }
  .table-row:hover { background: var(--bg3); }
  .mono { font-family: "SF Mono", "Fira Code", monospace; font-size: 11px; }
  .small { font-size: 11px; color: var(--text-dim); }
  .empty { padding: 32px; text-align: center; color: var(--text-dim); font-size: 13px; }

  .results-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(180px, 1fr)); gap: 4px 12px; margin-top: 8px; }
  .result-row { display: flex; gap: 6px; align-items: center; font-size: 11px; }
  .src-name { color: var(--text-dim); flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .src-stat { color: var(--text); min-width: 40px; }
  .src-added { color: var(--green); min-width: 30px; font-weight: 600; }
  .result-row:has(.src-added:contains("err")) .src-added { color: var(--red); }
</style>
