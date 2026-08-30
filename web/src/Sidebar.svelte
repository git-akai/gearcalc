<script lang="ts">
  import { workspace, trains, library } from "./state.svelte";
  import { exportLibrary, t, languages, language, setLanguage } from "./core";

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
  <!-- The application's name, which is not the same string as the heading over
       the gear tabs below even though both once read "Gears". One key for two
       jobs meant renaming either renamed both. -->
  <h1>{t("ui.app_name")}</h1>

  <!-- Under the title rather than at the foot of the pane. It sat below the tab
       lists, which grow: past a dozen tabs it was pushed off the bottom and the
       one control a reader needs *before* they can read anything was the one
       they had to scroll to find.

       Each language is named in itself, because a reader looking for theirs is
       looking for the word they call it by — a list that says "German" is a
       list for people who already read English.

       Only once the core has answered: the list comes from Rust, and an empty
       select is a blank box rather than a language. -->
  {#if languages().length}
    <label class="language">
      <span class="visually-hidden">{t("ui.sidebar_language")}</span>
      <select value={language()} onchange={(e) => setLanguage(e.currentTarget.value)}>
        {#each languages() as l (l.code)}
          <option value={l.code}>
            {l.name}{l.name === l.english ? "" : ` (${l.english})`}
          </option>
        {/each}
      </select>
    </label>
  {/if}

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
      {t("ui.sidebar_material_count", { count: String(library.materials.material.length) })}{library.origin
          ? ` · ${library.origin}`
          : ""}
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
    <button class="add" onclick={() => workspace.create()}>{t("ui.sidebar_new_gear")}</button>
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
            <span class="teeth">
              {t(
                tab.train.stages.length === 1 ? "ui.sidebar_stage_count_one" : "ui.sidebar_stage_count",
                { count: String(tab.train.stages.length) },
              )}
            </span>
          </button>
        </li>
      {/each}
    </ul>
    <button class="add" onclick={() => trains.create()}>{t("ui.sidebar_new_geartrain")}</button>
  </section>

  {#if version}
    <p class="version">{t("ui.sidebar_core_version", { version })}</p>
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
  /* A `select` sizes itself to its longest option unless told not to, and a
     flex item will not shrink below that intrinsic width — so without the
     `min-width` the picker pushed itself past the edge of the pane. */
  .language {
    min-width: 0;
  }
  .language select {
    max-width: 100%;
    font: inherit;
    font-size: 0.75rem;
    width: 100%;
    padding: 0.25rem 0.4rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    /* Named rather than left transparent: an unpainted `select` shows the
       browser's own control behind the page's text, and the two do not agree
       in a dark theme. */
    background: var(--bg);
    /* Read, not skimmed: this is a control someone uses when they cannot read
       the rest of the page, so it is the last thing that should be set in the
       muted grey the *notes* use. The affordance moves to the border. */
    color: var(--fg);
    cursor: pointer;
  }
  .language select:hover,
  .language select:focus {
    border-color: var(--muted);
  }
  /* The picker needs no visible label — a list of language names in their own
     scripts says what it is — but it still needs one to be announced. */
  .visually-hidden {
    position: absolute;
    width: 1px;
    height: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }
  .version {
    margin-top: auto;
    font-size: 0.7rem;
    color: var(--muted);
    padding-left: 0.25rem;
  }
</style>
