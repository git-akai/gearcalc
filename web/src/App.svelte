<script lang="ts">
  import { onMount } from "svelte";
  import { loadCore, coreVersion, t } from "./core";
  import { workspace, trains, library } from "./state.svelte";
  import Sidebar from "./Sidebar.svelte";
  import GearPanel from "./GearPanel.svelte";
  import TrainPanel from "./TrainPanel.svelte";

  let loaded = $state(false);
  let failed = $state<string | null>(null);

  // The tab's title is the application's **name**, read from the same catalogue
  // entry the sidebar heading uses — so the name is written down once.
  //
  // Reactive, and it has to be: the catalogue arrives with the core, and `t`
  // renders a missing message as its own key, so a plain assignment at mount
  // would put the literal `ui.app_name` in the tab and never correct it. That is
  // the "a module-level variable is not reactive" fault the sidebar hit once
  // already (`docs/corrections.md`), and the guard below is what keeps the key
  // itself off the screen while the core is still loading — `index.html` holds
  // what the tab says until then.
  $effect(() => {
    const name = t("ui.app_name");
    if (name !== "ui.app_name") document.title = name;
  });

  onMount(async () => {
    try {
      await loadCore();
      // The shipped materials and the defaults for a fresh tab both live in
      // the core, so neither can be read before it is up. That is deliberate:
      // it is what stops a default from being written down twice (docs/corrections.md).
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
  /* **The shell is a frame, and the two panes are the only things that scroll.**
     Whenever a pane's content grew past the window, a third scrollbar appeared
     on the document itself — at the far right, beside the one `main` already
     had, and scrolling nothing but blank space. `overflow: hidden` here is what
     makes that impossible rather than merely unlikely: a grid row that outgrows
     a fixed-height container overflows it, and an overflow the container does
     not handle is passed up to the viewport. Nothing is lost by clipping it,
     because both children scroll inside themselves.

     `dvh` before `vh` for the same reason, one level down: `100vh` is the
     viewport *ignoring* any horizontal scrollbar, so a page that grows one is
     suddenly a scrollbar's-height too tall — which is exactly a strip of blank
     at the bottom and a bar to scroll to it. */
  .shell {
    display: grid;
    grid-template-columns: 16rem 1fr;
    height: 100vh;
    height: 100dvh;
    overflow: hidden;
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
