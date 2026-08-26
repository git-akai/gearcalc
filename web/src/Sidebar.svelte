<script lang="ts">
  import { workspace, trains, library } from "./state.svelte";
  import { exportLibrary, t } from "./core";

  let { version }: { version: string | null } = $props();

  let picker: HTMLInputElement;

  async function onPicked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    library.import(await file.text(), file.name);
    // Clear it, or re-picking the same file after fixing it fires nothing.
    input.value = "";
  }

  function saveLibrary() {
    const r = exportLibrary(library.materials);
    if ("error" in r) {
      library.error = r.error;
      return;
    }
    const url = URL.createObjectURL(new Blob([r.ok], { type: "text/plain" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = "materials.toml";
    a.click();
    URL.revokeObjectURL(url);
  }
</script>

<aside>
  <h1>{t("ui.sidebar_gears")}</h1>

  <section class="library">
    <h2>{t("ui.sidebar_materials")}</h2>
    <div class="row">
      <button onclick={() => picker.click()}>{t("ui.sidebar_import_library")}</button>
      <button onclick={saveLibrary} disabled={library.materials.material.length === 0}>{t("ui.sidebar_export")}</button>
    </div>
    <input
      bind:this={picker}
      type="file"
      accept=".toml,text/plain"
      onchange={onPicked}
      hidden
    />
    <p class="detail">
      {library.materials.material.length} materials{library.origin ? ` · ${library.origin}` : ""}
    </p>
    {#if library.error}
      <p class="err">{library.error}</p>
    {/if}
  </section>

  <section>
    <h2>{t("ui.sidebar_gears")}</h2>
    <ul>
      {#each workspace.tabs as tab (tab.id)}
        <li>
          <button
            class="tab"
            class:selected={tab.id === workspace.selectedId && trains.active === "gear"}
            onclick={() => workspace.select(tab.id)}
          >
            <span class="name">{tab.name || "Unnamed"}</span>
            <span class="teeth">z{tab.params.teeth}</span>
          </button>
        </li>
      {/each}
    </ul>
    <button class="add" onclick={() => workspace.create()}>+ New gear</button>
  </section>

  <section>
    <h2>{t("ui.sidebar_geartrains")}</h2>
    <ul>
      {#each trains.tabs as tab (tab.id)}
        <li>
          <button
            class="tab"
            class:selected={tab.id === trains.selectedId && trains.active === "train"}
            onclick={() => trains.select(tab.id)}
          >
            <span class="name">{tab.name || "Unnamed"}</span>
            <span class="teeth">{tab.train.stages.length} stage{tab.train.stages.length === 1 ? "" : "s"}</span>
          </button>
        </li>
      {/each}
    </ul>
    <button class="add" onclick={() => trains.create()}>+ New geartrain</button>
  </section>

  {#if version}
    <p class="version">core v{version}</p>
  {/if}
</aside>

<style>
  aside {
    display: flex;
    flex-direction: column;
    gap: 1.25rem;
    padding: 1rem 0.75rem;
    border-right: 1px solid var(--rule);
    background: var(--panel);
    overflow-y: auto;
  }
  h1 {
    font-size: 1rem;
    margin: 0 0 0 0.25rem;
    letter-spacing: 0.06em;
    text-transform: uppercase;
    color: var(--muted);
  }
  h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 0 0 0.4rem 0.25rem;
  }
  ul {
    list-style: none;
    margin: 0 0 0.4rem;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .tab {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 0.85rem;
    padding: 0.3rem 0.5rem;
    border: 1px solid transparent;
    border-radius: 3px;
    background: none;
    color: var(--fg);
    cursor: pointer;
  }
  .tab:hover {
    background: var(--hover);
  }
  .tab.selected {
    background: var(--selected);
    border-color: var(--rule);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .teeth {
    color: var(--muted);
    font-size: 0.75rem;
    font-variant-numeric: tabular-nums;
  }
  .add {
    font: inherit;
    font-size: 0.8rem;
    width: 100%;
    text-align: left;
    padding: 0.3rem 0.5rem;
    border: 1px dashed var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .add:hover {
    color: var(--fg);
    border-color: var(--muted);
  }
  .row {
    display: flex;
    gap: 0.35rem;
  }
  .row button {
    flex: 1;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.25rem 0.5rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--fg);
    cursor: pointer;
  }
  .row button:hover:not(:disabled) {
    background: var(--hover);
  }
  .row button:disabled {
    color: var(--muted);
    cursor: default;
  }
  .library .detail {
    font-size: 0.7rem;
    color: var(--muted);
    margin: 0.35rem 0 0 0.25rem;
  }
  .library .err {
    font-size: 0.7rem;
    color: var(--warn);
    margin: 0.35rem 0 0 0.25rem;
  }
  .version {
    margin-top: auto;
    font-size: 0.7rem;
    color: var(--muted);
    padding-left: 0.25rem;
  }
</style>
