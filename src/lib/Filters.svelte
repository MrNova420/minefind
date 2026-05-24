<script>
  let { filters = {}, onChange = () => {} } = $props();

  let search = $state(filters.search ?? "");
  let category = $state(filters.category ?? "all");
  let whitelist = $state(filters.whitelist ?? "all");
  let sortBy = $state(filters.sortBy ?? "players");

  $effect(() => {
    onChange({ search, category, whitelist, sortBy });
  });
</script>

<div class="filters">
  <input
    type="text"
    placeholder="Search MOTD or IP..."
    bind:value={search}
  />
  <select bind:value={category}>
    <option value="all">All Categories</option>
    <option value="vanilla_survival">Vanilla Survival</option>
    <option value="modded">Modded</option>
    <option value="plugin_heavy">Plugin Heavy</option>
    <option value="creative">Creative</option>
    <option value="minigame">Minigame</option>
    <option value="anarchy">Anarchy</option>
    <option value="private_group">Private Group</option>
    <option value="idle">Idle</option>
  </select>
  <select bind:value={whitelist}>
    <option value="all">Whitelist: Any</option>
    <option value="no">Not Whitelisted</option>
    <option value="yes">Whitelisted</option>
    <option value="unknown">Unknown</option>
  </select>
  <select bind:value={sortBy}>
    <option value="players">Sort: Players</option>
    <option value="ping">Sort: Ping</option>
    <option value="name">Sort: Name</option>
  </select>
</div>
