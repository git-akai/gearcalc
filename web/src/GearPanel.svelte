<script lang="ts">
  import {
    FIELDS,
    dxf,
    isUnavailable,
    profile,
    solve,
    validate,
    boundFor,
    ringDxf,
    ringProfile,
    solveRing,
    type GearRequest,
    type RingRequest,
    type Maybe,
    type PinsOut,
  } from "./core";
  import { workspace, type GearTab as Tab } from "./state.svelte";
  import Viewport from "./Viewport.svelte";

  let { tab }: { tab: Tab } = $props();

  // Raw text per field, so a half-typed "-" or "1e" never reaches the solver.
  // The last valid value stays in `tab.params`, which is what Rust sees.
  let raw = $state<Record<string, string>>(
    Object.fromEntries(FIELDS.map((f) => [f.key, String(tab.params[f.key])])),
  );
  let errors = $state<Record<string, string | null>>({});

  /** The bound, phrased for the field it belongs to. */
  function boundNote(key: string): string | null {
    if (!("ok" in result)) return null;
    const r = result.ok.ranges;
    switch (key) {
      case "addendum":
        return r.addendum.min === null ? null : `above ${n(r.addendum.min)}: the tooth must have height`;
      case "dedendum":
        return r.dedendum.max === null
          ? null
          : `${n(r.dedendum.min ?? 0)} … ${n(r.dedendum.max)} · the root circle must clear the axis`;
      case "root_radius":
        return r.root_radius.max === null
          ? null
          : `up to ${n(r.root_radius.max)} · the fillet must fit the tooth space`;
      default:
        return null;
    }
  }

  function onInput(key: string, text: string) {
    raw[key] = text;
    const f = FIELDS.find((f) => f.key === key)!;
    const v = Number(text);
    const b = "ok" in result ? boundFor(f.key, result.ok.ranges) : null;
    const err = text.trim() === "" ? "required" : validate(f, v, b);
    errors[key] = err;
    if (!err) tab.params[f.key] = v;
  }

  const n = (v: number) => v.toFixed(3);

  const request = $derived<GearRequest>({
    params: tab.params,
    pin_diameter: tab.pinDiameter > 0 ? tab.pinDiameter : null,
    tolerance_class: tab.toleranceClass,
    chord_tolerance: tab.chordTolerance,
    reference_circles: tab.referenceCircles,
  });

  const result = $derived(solve(request));

  // A ring is a different part with different answers, so it gets its own
  // request and its own summary rather than optional fields on the gear's.
  const ringRequest = $derived<RingRequest>({
    params: tab.params,
    cutter: tab.cutter,
    chord_tolerance: tab.chordTolerance,
    reference_circles: tab.referenceCircles,
  });
  const ring = $derived(tab.internal ? solveRing(ringRequest) : null);

  // Kept as a typed array rather than an inline tuple list: destructuring a
  // mixed tuple inside {#each} widens both members to their union and loses the
  // field types.
  const pinRows = $derived<{ label: string; value: Maybe<PinsOut> }[]>(
    "ok" in result
      ? [
          { label: "2 pins", value: result.ok.over_two_pins },
          { label: "3 pins", value: result.ok.over_three_pins },
        ]
      : [],
  );
  const outline = $derived(
    tab.internal
      ? ring && "ok" in ring
        ? ringProfile(ringRequest, 600)
        : null
      : "ok" in result
        ? profile(request, 600)
        : null,
  );

  let confirmingDelete = $state(false);

  function saveDxf() {
    const r = tab.internal ? ringDxf(ringRequest) : dxf(request);
    if ("error" in r) return;
    const blob = new Blob([r.ok], { type: "application/dxf" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    const kind = tab.internal ? "ring" : "gear";
    a.download = `${(tab.name || kind).replace(/\s+/g, "_")}_m${tab.params.module}_z${tab.params.teeth}.dxf`;
    a.click();
    URL.revokeObjectURL(url);
  }

  const mm = (v: number) => `${v.toFixed(4)} mm`;
  const um = (v: number) => `${v.toFixed(1)} µm`;
</script>

<header>
  <input class="title" bind:value={tab.name} aria-label="Gear name" />
  <div class="actions">
    <button onclick={() => workspace.copy(tab.id)}>Copy</button>
    <button onclick={() => workspace.create()}>New</button>
    <button class="danger" onclick={() => (confirmingDelete = true)}>Delete</button>
  </div>
</header>

{#if confirmingDelete}
  <div class="confirm" role="alertdialog">
    <span>Delete “{tab.name || "Unnamed"}”?</span>
    <button
      class="danger"
      onclick={() => {
        workspace.remove(tab.id);
        confirmingDelete = false;
      }}>Delete</button
    >
    <button onclick={() => (confirmingDelete = false)}>Cancel</button>
  </div>
{/if}

<div class="columns">
  <section class="inputs">
    <h2>Parameters</h2>
    <div class="grid">
      <label class="check">
        <input type="checkbox" bind:checked={tab.internal} />
        <span>Internal (ring) gear</span>
        <small>the teeth point inward: the tip circle is inside the pitch circle</small>
      </label>
    </div>
    {#if tab.internal}
      <div class="grid">
        <label>
          <span>Cutter teeth</span>
          <input type="number" step="1" min="1" bind:value={tab.cutter.teeth} />
          <small>a ring is shaped by a pinion, and its fillet depends on which one</small>
        </label>
        <label>
          <span>Cutter addendum</span>
          <input type="number" step="0.05" bind:value={tab.cutter.addendum} />
          <em>m</em>
        </label>
        <label>
          <span>Cutter tip round</span>
          <input type="number" step="0.02" bind:value={tab.cutter.tip_round} />
          <em>m</em>
        </label>
      </div>
    {/if}
    <div class="grid">
      {#each FIELDS.filter((f) => !(tab.internal && f.externalOnly)) as f (f.key)}
        <label class:invalid={errors[f.key]}>
          <span>{f.label}</span>
          <input
            type="number"
            step={f.step}
            value={raw[f.key]}
            oninput={(e) => onInput(f.key, e.currentTarget.value)}
          />
          <em>{f.unit}</em>
          {#if errors[f.key]}<small class="err">{errors[f.key]}</small>
          {:else if f.key === "profile_shift" && "ok" in result}
            {@const r = result.ok.ranges.profile_shift}
            <small
              >buildable {n(r.bound.min ?? 0)} … {n(r.bound.max ?? 0)} · undercut below {n(
                r.undercut,
              )}
              <span class="ref">(sharp rack {n(r.sharp_rack_undercut)})</span>{#if r.pointed !== null}
                · pointed above {n(r.pointed)}{/if}</small
            >
          {:else if boundNote(f.key)}<small>{boundNote(f.key)}</small>
          {:else if tab.internal && f.ringNote}<small>{f.ringNote}</small>
          {:else if f.note}<small>{f.note}</small>{/if}
        </label>
      {/each}
    </div>

    <h2>Measurement</h2>
    <div class="grid">
      <label>
        <span>Pin / ball diameter</span>
        <input type="number" step="0.05" bind:value={tab.pinDiameter} />
        <em>mm</em>
      </label>
      {#if "ok" in result}
        <label>
          <span>Tolerance class</span>
          <select
            value={result.ok.tolerance && !isUnavailable(result.ok.tolerance)
              ? `${result.ok.tolerance.class.scale}:${result.ok.tolerance.class.grade}`
              : ""}
            onchange={(e) => {
              const [scale, grade] = e.currentTarget.value.split(":");
              tab.toleranceClass = scale
                ? { scale: scale as "fine" | "standard", grade: Number(grade) }
                : null;
            }}
          >
            {#each result.ok.available_classes as c}
              <option value="{c.scale}:{c.grade}">
                {c.scale === "fine" ? "Fine" : "Standard"}
                {c.grade}
              </option>
            {:else}
              <option value="">none available</option>
            {/each}
          </select>
          <em></em>
        </label>
      {/if}
    </div>

    <h2>Export</h2>
    <div class="grid">
      <label>
        <span>Chord tolerance</span>
        <input type="number" step="0.0005" min="0" bind:value={tab.chordTolerance} />
        <em>mm</em>
        <small>maximum deviation of the exported outline from the true curve</small>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={tab.referenceCircles} />
        <span>Include reference circles</span>
      </label>
    </div>
    <button class="primary" onclick={saveDxf} disabled={!("ok" in result)}>Export DXF</button>
  </section>

  <section class="results">
    {#if tab.internal}
      {#if ring && "error" in ring}
        <p class="error">{ring.error}</p>
      {:else if ring && "ok" in ring}
        {@const r = ring.ok}
        <Viewport
          points={outline}
          pitch={r.pitch_radius}
          base={r.base_radius}
          tip={r.tip_radius}
          root={r.root_radius}
        />
        <h2>Geometry</h2>
        <dl>
          <dt>Transverse module</dt>
          <dd>{mm(r.transverse_module)}</dd>
          <dt>Transverse pressure angle</dt>
          <dd>{r.transverse_pressure_angle.toFixed(4)}°</dd>
          <dt>Pitch radius</dt>
          <dd>{mm(r.pitch_radius)}</dd>
          <dt>Base radius</dt>
          <dd>{mm(r.base_radius)}</dd>
          <dt>Tip radius</dt>
          <dd>{mm(r.tip_radius)} <small>inside the pitch circle</small></dd>
          <dt>Root radius</dt>
          <dd>{mm(r.root_radius)} <small>outside it</small></dd>
          <dt>Flank / fillet junction</dt>
          <dd>{mm(r.junction_radius)}</dd>
          <dt>Root form</dt>
          <dd>
            {r.fully_filleted_root ? "fully filleted — no root arc" : "root arc between the fillets"}
          </dd>
          <dt>Generated down to</dt>
          <dd>
            {mm(r.generation_limit)}
            {#if !r.fully_generated}
              <small class="warn">
                below the tip: the cutter's own involute runs out there, so the flank near the tip
                is not generated. A cutter with more teeth reaches further
              </small>
            {:else}
              <small>past the tip, so the whole flank is generated</small>
            {/if}
          </dd>
          <dt>Smallest tooth count</dt>
          <dd>
            {r.smallest_tooth_count}
            <small>
              below this the tip would reach inside the base circle; it moves with the addendum,
              pressure angle and helix
            </small>
          </dd>
        </dl>
        {#if r.clamps.length}
          <ul class="notes">
            {#each r.clamps as c (c)}<li>{c}</li>{/each}
          </ul>
        {/if}
        <p class="muted">
          Measurement over teeth and pins is not shown for an internal gear: span and over-pins are
          external-gear constructions and this tool does not yet have their internal equivalents. A
          strength rating is not shown here for any gear — bending and contact need a mesh and a
          load, which belong to a stage; the geartrain tab rates rings, spur and helical alike.
        </p>
      {/if}
    {:else if "error" in result}
      <p class="error">{result.error}</p>
    {:else}
      {@const s = result.ok}
      <Viewport
        points={outline}
        pitch={s.pitch_radius}
        base={s.base_radius}
        tip={s.tip_radius}
        root={s.root_radius}
      />

      {#if s.undercut || s.severed || s.clamps.length}
        <ul class="notes">
          {#if s.undercut}<li>Undercut.</li>{/if}
          {#if s.severed}<li>Tooth severed by undercut — profile truncated at the centreline.</li>{/if}
          {#each s.clamps as c}<li>Clamped: {c}</li>{/each}
        </ul>
      {/if}

      <h2>Geometry</h2>
      <dl>
        <dt>Pitch radius</dt><dd>{mm(s.pitch_radius)}</dd>
        <dt>Base radius</dt><dd>{mm(s.base_radius)}</dd>
        <dt>Tip radius</dt><dd>{mm(s.tip_radius)}</dd>
        <dt>Root radius</dt><dd>{mm(s.root_radius)}</dd>
        <dt>Tooth thickness</dt><dd>{mm(s.tooth_thickness)}</dd>
        <dt>Fillet radius</dt><dd>{mm(s.fillet_radius)}</dd>
        <dt>Transverse pressure angle</dt><dd>{s.transverse_pressure_angle.toFixed(4)}°</dd>
        <dt>Cutter tip width</dt><dd>{mm(s.cutter_tip_width)}</dd>
      </dl>

      <h2>Measurement over teeth</h2>
      <dl>
        {#if isUnavailable(s.span)}
          <dt>Span</dt><dd class="na">{s.span.unavailable}</dd>
        {:else}
          <dt>Teeth spanned</dt><dd>{s.span.teeth_spanned}</dd>
          <dt>Nominal</dt><dd>{mm(s.span.nominal)}</dd>
          <dt>Contact radius</dt><dd>{mm(s.span.contact_radius)}</dd>
        {/if}
      </dl>

      <h2>Measurement over pins</h2>
      <dl>
        {#each pinRows as row (row.label)}
          {#if isUnavailable(row.value)}
            <dt>{row.label}</dt><dd class="na">{row.value.unavailable}</dd>
          {:else}
            <dt>{row.label}, nominal</dt><dd>{mm(row.value.nominal)}</dd>
          {/if}
        {/each}
      </dl>

      <h2>Composite error, JGMA 116-02</h2>
      <dl>
        {#if isUnavailable(s.tolerance)}
          <dt>Tolerance</dt><dd class="na">{s.tolerance.unavailable}</dd>
        {:else}
          <dt>Class</dt>
          <dd>
            {s.tolerance.class.scale === "fine" ? "Fine" : "Standard"}
            {s.tolerance.class.grade}
          </dd>
          <dt>Tooth-to-tooth, max</dt><dd>{um(s.tolerance.tooth_to_tooth)}</dd>
          <dt>Total, max</dt><dd>{um(s.tolerance.total)}</dd>
        {/if}
      </dl>
    {/if}
  </section>
</div>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 1rem;
    margin-bottom: 1rem;
  }
  .title {
    font: inherit;
    font-size: 1.15rem;
    font-weight: 600;
    flex: 1;
    min-width: 0;
    padding: 0.25rem 0.4rem;
    border: 1px solid transparent;
    border-radius: 3px;
    background: none;
    color: var(--fg);
  }
  .title:hover,
  .title:focus {
    border-color: var(--rule);
    background: var(--bg);
  }
  .actions {
    display: flex;
    gap: 0.4rem;
  }
  button {
    font: inherit;
    font-size: 0.8rem;
    padding: 0.25rem 0.7rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--panel);
    color: var(--fg);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
  }
  button:disabled {
    opacity: 0.5;
    cursor: default;
  }
  button.danger {
    color: var(--warn);
  }
  button.primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--on-accent);
    margin-top: 0.5rem;
  }

  .confirm {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.5rem 0.75rem;
    margin-bottom: 1rem;
    border: 1px solid var(--warn);
    border-radius: 4px;
    font-size: 0.85rem;
  }

  .columns {
    display: grid;
    grid-template-columns: minmax(20rem, 26rem) minmax(0, 1fr);
    gap: 1.5rem;
    align-items: start;
  }
  @media (max-width: 62rem) {
    .columns {
      grid-template-columns: 1fr;
    }
  }

  h2 {
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    color: var(--muted);
    margin: 1.25rem 0 0.5rem;
  }
  h2:first-child {
    margin-top: 0;
  }

  .grid {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  label {
    display: grid;
    grid-template-columns: 1fr 7rem 3.5rem;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.85rem;
  }
  label.check {
    grid-template-columns: auto 1fr;
  }
  label small {
    grid-column: 1 / -1;
    font-size: 0.72rem;
    color: var(--muted);
    margin-top: -0.15rem;
  }
  label small.err {
    color: var(--warn);
  }
  input[type="number"],
  select {
    font: inherit;
    font-size: 0.85rem;
    font-variant-numeric: tabular-nums;
    text-align: right;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--bg);
    color: var(--fg);
  }
  select {
    text-align: left;
  }
  label.invalid input {
    border-color: var(--warn);
  }
  em {
    font-style: normal;
    color: var(--muted);
    font-size: 0.75rem;
  }

  dl {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 0.15rem 1rem;
    margin: 0;
    font-size: 0.85rem;
  }
  dt {
    color: var(--muted);
  }
  dd {
    margin: 0;
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-weight: 600;
  }
  dd.na {
    font-weight: 400;
    color: var(--muted);
    font-style: italic;
    text-align: right;
  }

  .notes {
    margin: 0.75rem 0 0;
    padding-left: 1.1rem;
    font-size: 0.8rem;
    color: var(--warn);
  }
  .error {
    color: var(--warn);
  }
</style>
