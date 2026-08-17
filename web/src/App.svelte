<script lang="ts">
  import { onMount } from "svelte";
  import { loadCore, coreVersion } from "./core";
  import { workspace, trains, library } from "./state.svelte";
  import Sidebar from "./Sidebar.svelte";
  import GearPanel from "./GearPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";

  let loaded = $state(false);
  let failed = $state<string | null>(null);

  onMount(async () => {
    try {
      await loadCore();
      // The shipped materials live in the core, so they cannot be read before
      // it is up.
      library.loadDefaults();
      loaded = true;
    } catch (e) {
      failed = e instanceof Error ? e.message : String(e);
    }
  });
</script>

<div class="shell">
  <Sidebar version={loaded ? coreVersion() : null} />
  <main>
    {#if failed}
      <p class="error">The calculation core failed to load: {failed}</p>
    {:else if !loaded}
      <p class="muted">Loading core…</p>
    {:else}
      {#if trains.active === "train"}
        {#key trains.selected.id}
          <TrainPanel tab={trains.selected} />
        {/key}
      {:else}
        {#key workspace.selected.id}
          <GearPanel tab={workspace.selected} />
        {/key}
      {/if}
    {/if}
  </main>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 16rem 1fr;
    height: 100vh;
  }
  main {
    padding: 1rem 1.25rem 2rem;
    overflow-y: auto;
    min-width: 0;
  }
  .error {
    color: var(--warn);
  }
  .muted {
    color: var(--muted);
  }
</style>
