<script lang="ts">
  import { workspace } from "./state.svelte";

  let { version }: { version: string | null } = $props();
</script>

<aside>
  <h1>Gears</h1>

  <section>
    <h2>Gears</h2>
    <ul>
      {#each workspace.tabs as tab (tab.id)}
        <li>
          <button
            class="tab"
            class:selected={tab.id === workspace.selectedId}
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

  <section class="pending">
    <h2>Geartrains</h2>
    <p>Arrives in a later milestone.</p>
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
  .pending p {
    font-size: 0.75rem;
    color: var(--muted);
    margin: 0 0 0 0.25rem;
  }
  .version {
    margin-top: auto;
    font-size: 0.7rem;
    color: var(--muted);
    padding-left: 0.25rem;
  }
</style>
