<script lang="ts">
  import { onMount } from "svelte";
  import { loadCore, coreVersion, t } from "./core";
  import { workspace, trains, library } from "./state.svelte";
  import Sidebar from "./Sidebar.svelte";
  import GearPanel from "./GearPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";

  let loaded = $state(false);
  let failed = $state<string | null>(null);

  onMount(async () => {
    try {
      await loadCore();
      // The shipped materials and the defaults for a fresh tab both live in
      // the core, so neither can be read before it is up. That is deliberate:
      // it is what stops a default from being written down twice (DESIGN §12).
      library.loadDefaults();
      workspace.initialise();
      trains.initialise();
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
      <p class="muted">{t("ui.app_loading_core")}</p>
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
