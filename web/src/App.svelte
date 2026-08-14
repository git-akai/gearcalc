<script lang="ts">
  import { onMount } from "svelte";
  import { loadCore, coreVersion, solve, profile, defaultParams, type GearParams } from "./core";
  import Viewport from "./Viewport.svelte";

  let loaded = $state(false);
  let params = $state<GearParams>({ ...defaultParams });

  // Recompute on every input change. A solve is microseconds, so there is
  // nothing to cache and no state to invalidate — outputs cannot drift from
  // inputs because they are never stored.
  const result = $derived(loaded ? solve(params) : null);
  const outline = $derived(loaded && result && "ok" in result ? profile(params, 400) : null);

  onMount(async () => {
    await loadCore();
    loaded = true;
  });

  const fields: { key: keyof GearParams; label: string; unit: string; step: number }[] = [
    { key: "module", label: "Normal module", unit: "mm", step: 0.1 },
    { key: "pressure_angle", label: "Pressure angle", unit: "°", step: 0.5 },
    { key: "teeth", label: "Tooth count", unit: "", step: 1 },
    { key: "helix_angle", label: "Helix angle", unit: "°", step: 1 },
    { key: "profile_shift", label: "Profile shift", unit: "m", step: 0.05 },
    { key: "addendum", label: "Addendum", unit: "m", step: 0.05 },
    { key: "dedendum", label: "Dedendum", unit: "m", step: 0.05 },
    { key: "root_radius", label: "Root radius coeff.", unit: "m", step: 0.01 },
    { key: "thickness_mod", label: "Tooth thickness mod.", unit: "", step: 0.05 },
  ];

  const outputs: { label: string; get: (s: any) => string }[] = [
    { label: "Pitch radius", get: (s) => s.pitch_radius.toFixed(4) + " mm" },
    { label: "Base radius", get: (s) => s.base_radius.toFixed(4) + " mm" },
    { label: "Tip radius", get: (s) => s.tip_radius.toFixed(4) + " mm" },
    { label: "Root radius", get: (s) => s.root_radius.toFixed(4) + " mm" },
    { label: "Tooth thickness", get: (s) => s.tooth_thickness.toFixed(4) + " mm" },
    { label: "Fillet radius", get: (s) => s.fillet_radius.toFixed(4) + " mm" },
    { label: "Transverse PA", get: (s) => s.transverse_pressure_angle.toFixed(4) + " °" },
    { label: "Cutter tip width", get: (s) => s.cutter_tip_width.toFixed(4) + " mm" },
  ];
</script>

<div class="shell">
  <aside>
    <h1>Gears</h1>
    <p class="scaffold">Scaffold — milestone 0. Sidebar, tabs and the geartrain side arrive in later milestones.</p>
    {#if loaded}<p class="version">core v{coreVersion()}</p>{/if}
  </aside>

  <main>
    {#if !loaded}
      <p>Loading core…</p>
    {:else}
      <section class="params">
        {#each fields as f (f.key)}
          <label>
            <span>{f.label}</span>
            <input type="number" step={f.step} bind:value={params[f.key]} />
            <em>{f.unit}</em>
          </label>
        {/each}
      </section>

      {#if result && "error" in result}
        <p class="error">{result.error}</p>
      {:else if result}
        <section class="outputs">
          {#each outputs as o (o.label)}
            <div><span>{o.label}</span><b>{o.get(result.ok)}</b></div>
          {/each}
        </section>

        {#if result.ok.undercut || result.ok.severed || result.ok.clamps.length}
          <ul class="notes">
            {#if result.ok.undercut}<li>Undercut.</li>{/if}
            {#if result.ok.severed}<li>Tooth severed by undercut — profile truncated at the centreline.</li>{/if}
            {#each result.ok.clamps as c}<li>{c}</li>{/each}
          </ul>
        {/if}

        <Viewport points={outline} />
      {/if}
    {/if}
  </main>
</div>

<style>
  .shell { display: grid; grid-template-columns: 15rem 1fr; min-height: 100vh; }
  aside {
    padding: 1rem; border-right: 1px solid var(--rule);
    background: var(--panel); overflow-y: auto;
  }
  h1 { font-size: 1.1rem; margin: 0 0 0.75rem; letter-spacing: 0.02em; }
  .scaffold, .version { font-size: 0.75rem; color: var(--muted); line-height: 1.5; }
  main { padding: 1rem 1.25rem; display: flex; flex-direction: column; gap: 1rem; min-width: 0; }

  .params { display: grid; grid-template-columns: repeat(auto-fit, minmax(15rem, 1fr)); gap: 0.4rem 1rem; }
  label { display: grid; grid-template-columns: 1fr 6.5rem 1.5rem; align-items: center; gap: 0.5rem; font-size: 0.85rem; }
  input {
    font: inherit; font-variant-numeric: tabular-nums; text-align: right;
    padding: 0.2rem 0.4rem; border: 1px solid var(--rule); border-radius: 3px;
    background: var(--bg); color: var(--fg);
  }
  em { font-style: normal; color: var(--muted); font-size: 0.75rem; }

  .outputs {
    display: grid; grid-template-columns: repeat(auto-fit, minmax(13rem, 1fr));
    gap: 0.25rem 1rem; padding: 0.75rem; border: 1px solid var(--rule);
    border-radius: 4px; background: var(--panel);
  }
  .outputs div { display: flex; justify-content: space-between; gap: 1rem; font-size: 0.85rem; }
  .outputs b { font-variant-numeric: tabular-nums; font-weight: 600; }
  .outputs span { color: var(--muted); }

  .notes { margin: 0; padding-left: 1.1rem; font-size: 0.8rem; color: var(--warn); }
  .error { color: var(--warn); font-size: 0.85rem; }
</style>
