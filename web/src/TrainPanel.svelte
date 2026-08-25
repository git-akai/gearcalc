<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    solveTrain,
    defaultStage,
    defaultPlanetaryStage,
    defaultWormStage,
    outside,
    type Auto,
    type Overrides,
    type StageGear,
    type MaterialValue,
    type GearResult,
    type WormResult,
  } from "./core";
  import { trains, library, type TrainTab } from "./state.svelte";
  import { exportTrain } from "./core";

  let { tab }: { tab: TrainTab } = $props();

  let confirmingDelete = $state(false);
  let exportError = $state<string | null>(null);
  let picker: HTMLInputElement;

  /** Export writes the inputs only — everything on screen below is recomputed
   *  from them, so a file cannot disagree with the tab it came from. */
  function saveTrain() {
    const r = exportTrain({ name: tab.name, train: $state.snapshot(tab.train) });
    if ("error" in r) {
      exportError = r.error;
      return;
    }
    exportError = null;
    const url = URL.createObjectURL(new Blob([r.ok], { type: "text/plain" }));
    const a = document.createElement("a");
    a.href = url;
    a.download = `${(tab.name || "geartrain").replace(/\s+/g, "_")}.toml`;
    a.click();
    URL.revokeObjectURL(url);
  }

  async function onPicked(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    const file = input.files?.[0];
    if (!file) return;
    trains.import(await file.text());
    // Clear it, or re-picking the same file after fixing it fires nothing.
    input.value = "";
  }

  let open = $state<Record<number, boolean>>({ 0: true });

  // Every number on screen comes back from Rust. Nothing here computes a
  // result — the project rule — so this is the only place a value is produced.
  const result = $derived(solveTrain(tab.train));

  const mode = $derived<"intermittent" | "continuous">(
    "intermittent" in tab.train.actuation ? "intermittent" : "continuous",
  );

  function setMode(m: "intermittent" | "continuous") {
    if (m === mode) return;
    tab.train.actuation =
      m === "intermittent"
        ? { intermittent: { range_degrees: 25, actuations: 1000 } }
        : { continuous: { operating_percent: 80, runtime_hours: 1000 } };
  }

  function addStage() {
    tab.train.stages.push(defaultStage());
    open[tab.train.stages.length - 1] = true;
  }

  function addPlanetaryStage() {
    tab.train.stages.push(defaultPlanetaryStage());
  }

  function addWormStage() {
    tab.train.stages.push(defaultWormStage());
    open[tab.train.stages.length - 1] = true;
  }

  function removeStage(i: number) {
    tab.train.stages.splice(i, 1);
  }

  /** Gear numbering runs across the whole train: stage 1 is gears 1 and 2,
   *  stage 2 is gears 3 and 4, as the specification describes. */
  function gearNumber(stage: number, which: number): number {
    return stage * 2 + which + 1;
  }

  /** The candidates for one note slot: a blank to reserve the space, the note
   *  itself when the stage has solved, and the out-of-range message when the
   *  value is outside its bound. All are rendered; see the slot's comment. */
  type Notes = { all: { text: string; err?: boolean }[]; shown: number };

  function notes(range: string | null, bad: string | null): Notes {
    const all: Notes["all"] = [{ text: "\u00a0" }];
    if (range) all.push({ text: range });
    if (bad) all.push({ text: bad, err: true });
    return { all, shown: bad ? all.length - 1 : range ? 1 : 0 };
  }

  const pct = (v: number) => (100 * v).toFixed(3);
  const n = (v: number) => v.toFixed(3);

</script>

<!-- A value + automatic toggle, locked while automatic (DESIGN.md 3.3).
     When automatic the field shows the SOLVED value, greyed, so a computed
     number is never mistaken for one that was chosen. Turning the toggle off
     leaves `manual` where it was, so the field does not jump. -->
<!-- A material property: the value the calculation used, greyed while it is the
     library's and un-greyed once replaced, so a default is never mistaken for a
     considered choice. Editing makes an override; the cross clears it.

     The number shown is Rust's — it has already chosen between the dry and
     conditioned states, which is an engineering decision and not this side's to
     make. -->
{#snippet property(
  label: string,
  gear: StageGear,
  key: keyof Overrides,
  used: MaterialValue | undefined,
  step: number,
  unit: string,
)}
  <label class="prop">
    <span>{label}</span>
    <input
      type="number"
      {step}
      value={gear.material_overrides[key] ?? used?.value ?? 0}
      class:computed={gear.material_overrides[key] === null}
      oninput={(e) => {
        const v = Number(e.currentTarget.value);
        gear.material_overrides[key] = Number.isFinite(v) ? v : null;
      }}
    />
    <em>{unit}</em>
    {#if gear.material_overrides[key] !== null}
      <button
        class="clear"
        title="Restore the library value"
        onclick={() => (gear.material_overrides[key] = null)}>×</button
      >
    {:else if used}
      <em class="basis" class:weak={used.basis === "estimated" || used.basis === "chart"}
        >{used.basis[0]}</em
      >
    {/if}
  </label>
{/snippet}

<!-- Every note a control can show, stacked in one grid cell with the ones that
     do not apply hidden. The slot is then as tall as the tallest of them at
     this width, so a note arriving or leaving — a value going out of range, a
     stage failing to solve — moves nothing below it. The blank candidate is
     what reserves the space when there is no note at all. -->
{#snippet noteSlot(notes: Notes)}
  <span class="note">
    {#each notes.all as note, i (i)}
      <small class:err={note.err} class:hidden={i !== notes.shown}>{note.text}</small>
    {/each}
  </span>
{/snippet}

<!-- One gear card, used by every stage that has gears. A sun, a planet, a ring
     and a spur gear take the same inputs and produce the same readout, so they
     are one definition rather than several that drift apart — which is what had
     happened to the planetary section. What genuinely differs is passed in: a
     ring's root belongs to its cutter, a planet's shift is solved rather than
     chosen, and a member may have something of its own to report. -->
<!-- What a crossed-axis mesh reports, whether it was entered as a worm drive or
     as a gear pair with its shafts turned: the same mathematics answers both
     (DESIGN §4.5.1), so it is one readout rather than two that drift. -->
{#snippet screwReadout(r: WormResult)}
<dl class="out">
  {#if r.crossed}
    <dt>Contact ratio</dt>
    <dd>
      <span class:warn={r.crossed.contact_ratio < 1}>
        ε {r.crossed.contact_ratio.toFixed(4)}
      </span>
      <small>
        {r.crossed.contact_ratio < 1
          ? "below 1: the pair loses contact between one tooth and the next"
          : `pairs in contact · ended by the ${r.crossed.limited_by === "face" ? "face width" : "teeth"}`}
        {#if r.crossed.tooth_height_assumed}
          · a floor: the tooth height is assumed at one module, and an enveloping
          wheel wraps further than the cylinder this is figured on
        {/if}
      </small>
    </dd>
    <dt>Contact travel</dt>
    <dd>
      {r.crossed.axial_travel[0].toFixed(3)} · {r.crossed.axial_travel[1].toFixed(3)} mm
      <small>
        along each member's own axis — what a face has to cover, and what a
        parallel pair does not have at all
      </small>
    </dd>
  {/if}
  <dt>Centre distance</dt>
  <dd>
    {r.centre_distance.toFixed(4)} mm
    <small>nominal {r.centre_distance_nominal.toFixed(4)}</small>
  </dd>
  <dt>Mesh efficiency</dt>
  <dd>
    {pct(r.efficiency.forward)} % driven forward
    · {pct(r.efficiency.backward)} % driven backward
    {#if r.efficiency.backward <= 0}
      <small class="warn">self-locking</small>
    {/if}
    {#if r.crossed?.parallel_axis_efficiency != null}
      <small>
        the same teeth with parallel shafts would give {pct(r.crossed.parallel_axis_efficiency)} %
        — crossing them is what the difference costs
      </small>
    {/if}
  </dd>
  <dt>Self-locks at μ</dt>
  <dd>{r.self_locking_friction.toFixed(4)}</dd>
  <dt>Contact stress</dt>
  <dd>
    {r.contact.max_pressure.toFixed(1)} MPa
    <small>
      patch {r.contact.patch_length.toFixed(4)} × {r.contact.patch_width.toFixed(4)} mm ·
      {Math.abs(r.contact.worst_position) < 1e-9
        ? "worst at the pitch point"
        : `worst ${r.contact.worst_position.toFixed(3)} mm along the path, where one tooth carries it alone`}
      · the pitch point alone gives {r.contact.at_pitch_point.toFixed(1)}
    </small>
  </dd>
  <dt>Sliding speed</dt>
  <dd>{r.sliding_velocity.toFixed(1)} mm/s</dd>
  <dt>Bending stress</dt>
  <dd>
    <small>
      not reported for crossed axes — no accepted analytical model exists, and
      deriving one from a parallel-axis calculation would be a convention rather
      than a derivation (DESIGN §4.5.1)
    </small>
  </dd>
  <dt>Flank type</dt>
  <dd>
    ZI (involute helicoid)
    <small>a ZN worm's contact stress is 1–15 % lower, rising with lead angle</small>
  </dd>
</dl>
{/snippet}

{#snippet noExtra(_j: number)}{/snippet}

{#snippet gearCard(
  title: string,
  gear: StageGear,
  g: GearResult | undefined,
  opts: {
    /** "shaper" for a ring, whose root and fillet are the tool's rather than
     *  inputs of its own; anything else is rack-generated. */
    cut?: "rack" | "shaper";
    /** Present when the shift is solved rather than offered — the planet's. */
    solvedShift?: number;
    /** False where nothing rates the face width — a crossed pair's contact is a
     *  point, so no stress depends on it and none can size it. */
    faceAuto?: boolean;
    /** The width at which ε = 1, for a crossed pair. */
    faceFromContinuity?: number;
    /** A readout only this member has; given the member's position. */
    extra?: Snippet<[number]>;
    extraIndex?: number;
  },
)}
<div class="gear">
  <h4>{title}</h4>
  <label class:invalid={g && outside(gear.teeth, g.ranges.teeth)}>
    <span>Tooth count</span>
    <input type="number" step="1" bind:value={gear.teeth} />
  </label>
  {#if opts.cut !== "shaper"}
    <label class:invalid={g && outside(gear.dedendum, g.ranges.dedendum)}>
      <span>Dedendum</span>
      <input type="number" step="0.05" bind:value={gear.dedendum} />
      {@render noteSlot(
        notes(
          g ? `${n(g.ranges.dedendum.min ?? 0)} to ${n(g.ranges.dedendum.max ?? 0)}` : null,
          g ? outside(gear.dedendum, g.ranges.dedendum) : null,
        ),
      )}
    </label>
    <label class:invalid={g && outside(gear.root_radius, g.ranges.root_radius)}>
      <span>Root radius</span>
      <input type="number" step="0.01" bind:value={gear.root_radius} />
      {@render noteSlot(
        notes(
          g ? `up to ${n(g.ranges.root_radius.max ?? 0)} · fillet must fit` : null,
          g ? outside(gear.root_radius, g.ranges.root_radius) : null,
        ),
      )}
    </label>
  {:else}
    <p class="aside">Root and fillet are the cutter's, below — a ring has no dedendum of its own.</p>
  {/if}
  {#if opts.solvedShift === undefined}
    {@render autoNumber("Profile shift", gear.profile_shift, g?.profile_shift, 0.05)}
    {#if gear.profile_shift.auto}
      <label class="sub">
        <span>Working tooth depth</span>
        <input type="number" step="0.05" bind:value={gear.working_depth} />
        <em>module</em>
      </label>
    {/if}
  {:else}
    <label>
      <span>Profile shift</span>
      <input type="number" value={Number(opts.solvedShift.toFixed(4))} disabled class="computed" />
      <em>module</em>
      {@render noteSlot(notes("solved: it is what makes the two centre distances agree", null))}
    </label>
  {/if}
  {#if opts.solvedShift === undefined && !gear.profile_shift.auto}
    {@const r = opts.cut === "shaper" ? undefined : g?.ranges.profile_shift}
    <p class="hint">
      <!-- The bounds shown here are the rack's — buildable range and undercut —
           and a shaper-cut ring's are not those: what limits it is its own base
           circle, its cutter's reach and the generation limit (DESIGN §4.11).
           The core does not yet report those for a stage member, so the ring is
           told what is missing rather than shown the wrong bound. -->
      {@render noteSlot(
        notes(
          opts.cut === "shaper"
            ? "a ring's bounds are its cutter's reach and its own base circle, not a rack's"
            : r
              ? `buildable ${n(r.bound.min ?? 0)} to ${n(r.bound.max ?? 0)} · undercut below ${n(r.undercut)}`
              : null,
          r ? outside(gear.profile_shift.manual, r.bound) : null,
        ),
      )}
    </p>
  {/if}
  {@render autoNumber("Addendum", gear.addendum, g?.addendum, 0.05)}
  {#if gear.addendum.auto}
    <label class="sub">
      <span>Minimum tip width</span>
      <input type="number" step="0.02" bind:value={gear.min_tip_width} />
      <em>mm</em>
    </label>
  {/if}
  {#if opts.faceAuto === false}
    <!-- A crossed pair's automatic width is a **geometric** minimum: the width
         at which one tooth pair hands over to the next (ε = 1). The spur
         stage's inverts a stress instead, and the two must not read alike. -->
    {@render autoNumber("Face width", gear.face_width, opts.faceFromContinuity, 0.5)}
    {@render noteSlot(
      notes(
        opts.faceFromContinuity === undefined
          ? "no width keeps contact continuous here — the teeth do not reach it at any width"
          : `automatic is ${n(opts.faceFromContinuity)} mm, where contact stays continuous (ε = 1) — a geometric minimum, not a strength one`,
        null,
      ),
    )}
  {:else}
    {@render autoNumber("Face width", gear.face_width, g?.face_width, 0.5)}
  {/if}
  {#if gear.face_width.auto && opts.faceAuto !== false}
    <div class="subtoggles">
      <label class="check">
        <input type="checkbox" bind:checked={gear.auto_face_from_bending} />
        <span>from bending</span>
      </label>
      <label class="check">
        <input type="checkbox" bind:checked={gear.auto_face_from_contact} />
        <span>from contact</span>
      </label>
    </div>
  {/if}
  <label>
    <span>Material</span>
    <select bind:value={gear.material}>
      {#each library.materials.material as m (m.name)}
        <option value={m.name}>{m.name}</option>
      {/each}
    </select>
  </label>

  {#if g}
    <div class="props">
      {@render property("Density", gear, "density", g.material.density, 10, "kg/m³")}
      {@render property("Elastic modulus", gear, "elastic_modulus", g.material.elastic_modulus, 100, "MPa")}
      {@render property("Poisson's ratio", gear, "poissons_ratio", g.material.poissons_ratio, 0.01, "")}
      {@render property("Ultimate allowable", gear, "ultimate_allowable", g.material.ultimate_allowable, 10, "MPa")}
      {@render property("Fatigue allowable", gear, "fatigue_allowable", g.material.fatigue_allowable, 10, "MPa")}
    </div>
    <dl class="out small">
      <dt>Torque</dt>
      <dd>{g.torque.toFixed(4)} Nm</dd>
      <dt>Speed</dt>
      <dd>{g.speed.toFixed(1)} rpm</dd>
      <dt>Tooth cycles</dt>
      <dd>{Math.ceil(g.tooth_cycles).toLocaleString()}</dd>
      <dt>Bending stress</dt>
      <dd>
        {g.bending_stress === null
          ? "—"
          : `${g.bending_stress.toFixed(1)} MPa`}
      </dd>
      <dt>Contact stress</dt>
      <dd>{g.contact_stress.toFixed(1)} MPa</dd>
      <dt>Min face width</dt>
      <dd>
        {g.min_face_width_bending === null
          ? "—"
          : `${g.min_face_width_bending.toFixed(3)}`} /
        {g.min_face_width_contact.toFixed(3)} mm
        <small>bending / contact</small>
      </dd>
    </dl>
    {#if g.clamps.length}
      <ul class="notes">
        {#each g.clamps as c (c)}<li>{c}</li>{/each}
      </ul>
    {/if}
  {/if}
  <!-- Outside the guard above: a crossed pair produces no per-gear rating, so
       `g` is absent exactly when this readout is the only one there is. -->
  {@render (opts.extra ?? noExtra)(opts.extraIndex ?? 0)}
</div>
{/snippet}

{#snippet autoNumber(label: string, a: Auto<number>, computed: number | undefined, step: number)}
  <label class="auto">
    <span>{label}</span>
    {#if a.auto}
      <input
        type="number"
        {step}
        value={computed === undefined ? a.manual : Number(computed.toFixed(4))}
        disabled
        class="computed"
      />
    {:else}
      <input type="number" {step} bind:value={a.manual} />
    {/if}
    <button class="toggle" class:on={a.auto} onclick={() => (a.auto = !a.auto)} title="Automatic">
      auto
    </button>
  </label>
{/snippet}

<header>
  <input class="title" bind:value={tab.name} aria-label="Geartrain name" />
  <div class="actions">
    <button onclick={saveTrain}>Export</button>
    <button onclick={() => picker.click()}>Import</button>
    <button onclick={() => trains.create()}>New</button>
    <button onclick={() => trains.copy(tab.id)}>Copy</button>
    <button class="danger" onclick={() => (confirmingDelete = true)}>Delete</button>
  </div>
</header>

<input
  bind:this={picker}
  type="file"
  accept=".toml,text/plain"
  onchange={onPicked}
  hidden
/>

{#if trains.importError}
  <p class="error">Import failed: {trains.importError}</p>
{/if}
{#if exportError}
  <p class="error">Export failed: {exportError}</p>
{/if}

{#if confirmingDelete}
  <div class="confirm" role="alertdialog">
    <span>Delete “{tab.name || "Unnamed"}”?</span>
    <button
      class="danger"
      onclick={() => {
        trains.remove(tab.id);
        confirmingDelete = false;
      }}>Delete</button
    >
    <button onclick={() => (confirmingDelete = false)}>Cancel</button>
  </div>
{/if}

<section class="train">
  <div class="grid">
    <label>
      <span>Input speed, peak</span>
      <input type="number" step="100" bind:value={tab.train.input_speed} />
      <em>rpm</em>
    </label>
    <label>
      <span>Input torque, peak</span>
      <input type="number" step="0.01" bind:value={tab.train.input_torque} />
      <em>Nm</em>
    </label>

    <div class="mode">
      <span>Actuation</span>
      <div class="segmented">
        <button class:on={mode === "intermittent"} onclick={() => setMode("intermittent")}>
          Intermittent
        </button>
        <button class:on={mode === "continuous"} onclick={() => setMode("continuous")}>
          Continuous
        </button>
      </div>
    </div>

    {#if "intermittent" in tab.train.actuation}
      <label>
        <span>Actuation range</span>
        <input
          type="number"
          step="1"
          bind:value={tab.train.actuation.intermittent.range_degrees}
        />
        <em>° at output</em>
      </label>
      <label>
        <span>Actuation count</span>
        <input type="number" step="100" bind:value={tab.train.actuation.intermittent.actuations} />
        <em></em>
      </label>
    {:else if "continuous" in tab.train.actuation}
      <label>
        <span>Operating speed</span>
        <input
          type="number"
          step="5"
          bind:value={tab.train.actuation.continuous.operating_percent}
        />
        <em>% of peak</em>
      </label>
      <label>
        <span>Runtime</span>
        <input type="number" step="100" bind:value={tab.train.actuation.continuous.runtime_hours} />
        <em>hours</em>
      </label>
    {/if}
  </div>

  {#if "error" in result}
    <p class="error">{result.error}</p>
  {:else}
    <dl class="out">
      <dt>Total ratio</dt>
      <dd>
        {result.ok.total_ratio >= 1
          ? `${result.ok.total_ratio.toFixed(4)} : 1`
          : `1 : ${(1 / result.ok.total_ratio).toFixed(4)}`}
      </dd>
      <dt>Output speed</dt>
      <dd>{result.ok.output_speed.toFixed(1)} rpm</dd>
      <dt>Output torque</dt>
      <dd>{result.ok.output_torque.toFixed(4)} Nm</dd>
      <dt>Total efficiency</dt>
      <dd>
        {pct(result.ok.total_efficiency.forward)} % driven forward
        · {pct(result.ok.total_efficiency.backward)} % driven backward
        {#if result.ok.total_efficiency.backward <= 0}
          <small class="warn">cannot be back-driven</small>
        {/if}
      </dd>
      <dt>Backlash at the output shaft</dt>
      <dd>
        {result.ok.backlash.forward.nominal.toFixed(5)}°
        <small
          >({result.ok.backlash.forward.minimum.toFixed(5)} to {result.ok.backlash.forward.maximum.toFixed(
            5,
          )})</small
        >
      </dd>
      <dt>Backlash at the input shaft</dt>
      <dd>
        {result.ok.backlash.backward.nominal.toFixed(5)}°
        <small
          >({result.ok.backlash.backward.minimum.toFixed(5)} to {result.ok.backlash.backward.maximum.toFixed(
            5,
          )})</small
        >
      </dd>
    </dl>
  {/if}
</section>

<div class="stages">
  {#each tab.train.stages as stage, i (i)}
    {@const res = "ok" in result ? result.ok.stages[i] : null}
    <section class="stage">
      {#if stage.kind === "spur"}
        <!-- One stage, two meshes. Crossing the shafts turns a line contact
             into a point one and changes what sliding costs, so the answer has
             a different shape: `sres` when the shafts are parallel, the screw
             result when they are not (DESIGN §4.5.1). The *inputs* below are
             the same either way, which is what the specification asks for. -->
        {@const sres = res && res.kind === "spur" ? res : null}
        {@const xres = res && res.kind === "worm" ? res : null}
        <button class="head" onclick={() => (open[i] = !open[i])}>
          <span class="caret">{open[i] ? "▾" : "▸"}</span>
          <strong>Stage {i + 1}</strong>
          {#if stage.shaft_angle !== 0}
            <span class="kind">crossed</span>
          {/if}
          <span class="teeth">z {stage.gears[0].teeth} / {stage.gears[1].teeth}</span>
          {#if sres ?? xres}
            <span class="ratio">{(sres ?? xres)?.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct((sres ?? xres)?.efficiency.forward ?? 0)} %</span>
          {/if}
        </button>

        {#if open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>Normal module</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>mm</em>
              </label>
              <label>
                <span>Pressure angle</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>Axis angle</span>
                <input type="number" step="5" bind:value={stage.shaft_angle} />
                <em>°</em>
                {@render noteSlot(
                  notes(
                    stage.shaft_angle === 0
                      ? "zero: the shafts are parallel"
                      : "the shafts cross, so the teeth touch at a point — see the results below",
                    null,
                  ),
                )}
              </label>
              <label>
                <span>Additional helix angle</span>
                <input type="number" step="1" bind:value={stage.additional_helix} />
                <em>°</em>
                {@render noteSlot(
                  notes(
                    `each gear carries ${n(stage.shaft_angle / 2 + stage.additional_helix)}° and ${n(
                      stage.shaft_angle / 2 - stage.additional_helix,
                    )}°, summing to the axis angle`,
                    null,
                  ),
                )}
              </label>
              <label>
                <span>Coefficient of friction</span>
                <input type="number" step="0.01" bind:value={stage.friction} />
                <em></em>
              </label>
              <label>
                <span>Tooth thickness mod.</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>k₁</em>
                <!-- One input where the specification had a pair, because the
                     two are not independent: `k₁ + k₂ = 2` is what keeps the
                     mesh at zero backlash (DESIGN §3.2), so storing both would
                     be storing a constraint that can be broken. Which gear it
                     applies to therefore has to be said. -->
                {@render noteSlot(
                  notes(
                    `gear ${gearNumber(i, 0)}'s: above 1 its teeth thicken and gear ${gearNumber(
                      i,
                      1,
                    )}'s thin by as much, since the pair must sum to 2` +
                      (stage.shaft_angle === 0
                        ? ""
                        : " — it shapes the teeth here, but a crossed mesh is solved at its pitch point"),
                    null,
                  ),
                )}
              </label>
              {@render autoNumber(
                "C2C distance",
                stage.centre_distance,
                (sres ?? xres)?.centre_distance,
                0.1,
              )}
              <label>
                <span>C2C clearance</span>
                <input
                  type="number"
                  step="0.01"
                  bind:value={stage.clearance}
                  disabled={!stage.centre_distance.auto}
                  class:computed={!stage.centre_distance.auto}
                />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance +</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance −</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>mm</em>
              </label>
            </div>
            <!-- Clearance is meaningless once the centre distance is set by hand:
                 the specification locks it to zero, and so does the solver. -->

            <!-- A crossed pair rates as one mesh, not two teeth: there is no
                 bending model and the point contact's pressure belongs to the
                 pair. What is left that is per-member is what each shaft sees. -->
            {#snippet crossedMember(j: number)}
              {#if xres}
                <dl class="out small">
                  <dt>Pitch diameter</dt>
                  <dd>{xres.members[j].pitch_diameter.toFixed(4)} mm</dd>
                  <dt>Helix angle</dt>
                  <dd>{n(j === 0 ? xres.helix_angle : xres.wheel_helix_angle)}°</dd>
                  <dt>Torque</dt>
                  <dd>{xres.members[j].torque.toFixed(4)} N·m</dd>
                  <dt>Speed</dt>
                  <dd>{xres.members[j].speed.toFixed(1)} rpm</dd>
                  <dt>Tooth cycles</dt>
                  <dd>{Math.ceil(xres.members[j].tooth_cycles).toLocaleString()}</dd>
                </dl>
              {/if}
            {/snippet}

            <!-- A crossed pair's members are ordinary helical gears, so their
                 tooth form is specified here as it is anywhere else — it is what
                 will be cut. What it does *not* do is move this stage's figures,
                 and saying which is which is the honesty required: the mesh is
                 solved at its pitch point, so a shift reaches the answer only
                 through the centre distance, which is an input of its own.
                 DESIGN §4.5.1. -->
            {#if stage.shaft_angle !== 0}
              <p class="aside wide">
                The tooth form below — shift, addendum, dedendum, root radius — describes the gears
                that will be cut. A crossed mesh is solved at its pitch point, so it does not move
                the figures in this stage: a profile shift reaches them only through the centre
                distance, which is an input of its own.
              </p>
            {/if}

            <div class="gears">
              {#each stage.gears as gear, j (j)}
                {@const g = sres?.gears[j]}
                {@render gearCard(`Gear ${gearNumber(i, j)}`, gear, g, {
                  cut: "rack",
                  faceAuto: stage.shaft_angle === 0,
                  faceFromContinuity: xres?.crossed?.face_width_for_continuity?.[j],
                  extra: xres ? crossedMember : undefined,
                  extraIndex: j,
                })}
              {/each}
            </div>

            {#if xres}
              {@render screwReadout(xres)}
              {#if xres.notes.length}
                <ul class="notes">
                  {#each xres.notes as note (note)}<li>{note}</li>{/each}
                </ul>
              {/if}
            {/if}

            {#if sres}
              <dl class="out">
                <dt>Centre distance</dt>
                <dd>
                  {sres.centre_distance.toFixed(4)} mm
                  <small>nominal {sres.centre_distance_nominal.toFixed(4)}</small>
                </dd>
                <dt>Contact ratio</dt>
                <dd>
                  ε<sub>α</sub> {sres.contact_ratios.transverse.toFixed(4)} · ε<sub>β</sub>
                  {sres.contact_ratios.overlap.toFixed(4)} · ε<sub>γ</sub>
                  {sres.contact_ratios.total.toFixed(4)}
                  {#if stage.additional_helix !== 0 && sres.contact_ratios.overlap < 1}
                    <small class="warn">no full axial overlap</small>
                  {/if}
                </dd>
                <dt>Mesh efficiency</dt>
                <dd>
                  {pct(sres.efficiency.forward)} % driven forward
                  · {pct(sres.efficiency.backward)} % driven backward
                  <small>equal, as a parallel-axis mesh must be</small>
                </dd>
                <dt>Backlash</dt>
                <dd>
                  {sres.backlash.forward.nominal.toFixed(5)}° at gear {gearNumber(i, 1)}
                  <small
                    >({sres.backlash.forward.minimum.toFixed(5)} to {sres.backlash.forward.maximum.toFixed(
                      5,
                    )})</small
                  >
                  · {sres.backlash.backward.nominal.toFixed(5)}° at gear {gearNumber(i, 0)}
                </dd>
                <dt>Coprime</dt>
                <dd>{sres.coprime ? "yes" : "no"}</dd>
              </dl>
              {#if sres.notes.length}
                <ul class="notes">
                  {#each sres.notes as n (n)}<li>{n}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>Remove stage</button
            >
          </div>
        {/if}
      {:else if stage.kind === "worm"}
        {@const wres = res && res.kind === "worm" ? res : null}
        <button class="head" onclick={() => (open[i] = !open[i])}>
          <span class="caret">{open[i] ? "▾" : "▸"}</span>
          <strong>Stage {i + 1}</strong>
          <span class="kind">worm</span>
          <span class="teeth">z {stage.starts} / {stage.wheel_teeth}</span>
          {#if wres}
            <span class="ratio">{wres.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct(wres.efficiency.forward)} %</span>
          {/if}
        </button>

        {#if open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>Normal module</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>mm</em>
              </label>
              <label>
                <span>Pressure angle</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>Axis angle</span>
                <input type="number" step="1" bind:value={stage.shaft_angle} />
                <em>°</em>
              </label>
              <label>
                <span>Coefficient of friction</span>
                <input type="number" step="0.01" bind:value={stage.friction} />
                <em></em>
              </label>
              {@render autoNumber("C2C distance", stage.centre_distance, wres?.centre_distance, 0.1)}
              <label>
                <span>C2C clearance</span>
                <input
                  type="number"
                  step="0.01"
                  bind:value={stage.clearance}
                  disabled={!stage.centre_distance.auto}
                  class:computed={!stage.centre_distance.auto}
                />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance +</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance −</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>mm</em>
              </label>
              <label>
                <span>Worm axial clearance</span>
                <input type="number" step="0.01" bind:value={stage.axial_clearance} />
                <em>mm</em>
              </label>
            </div>

            <div class="gears">
              <div class="gear">
                <h4>{"pitch_diameter" in stage.sizing ? "Worm" : "First gear"}</h4>
                <label>
                  <span>{"pitch_diameter" in stage.sizing ? "Starts" : "Tooth count"}</span>
                  <input type="number" step="1" bind:value={stage.starts} />
                </label>
                <label>
                  <span>Sized by</span>
                  <select
                    value={"pitch_diameter" in stage.sizing ? "diameter" : "helix"}
                    onchange={(e) => {
                      // Swap which of the two is the input, seeding the new one
                      // from the geometry so the pair does not jump.
                      const g = wres;
                      stage.sizing =
                        e.currentTarget.value === "diameter"
                          ? { pitch_diameter: g ? g.members[0].pitch_diameter : 7 }
                          : { helix_angle: g ? 90 - g.lead_angle : 45 };
                    }}
                  >
                    <option value="diameter">Pitch diameter — a worm</option>
                    <option value="helix">Helix angle — a gear</option>
                  </select>
                  <small>
                    a worm's diameter is a free choice and its lead angle follows; a gear's
                    diameter follows from its teeth and helix. Same geometry, opposite input
                  </small>
                </label>
                {#if "pitch_diameter" in stage.sizing}
                  <label>
                    <span>Pitch diameter</span>
                    <input type="number" step="0.5" bind:value={stage.sizing.pitch_diameter} />
                    <em>mm</em>
                  </label>
                {:else}
                  <label>
                    <span>Helix angle</span>
                    <input type="number" step="1" bind:value={stage.sizing.helix_angle} />
                    <em>°</em>
                    <small>the mate takes the rest of the shaft angle</small>
                  </label>
                {/if}
                {#if wres && wres.members[0].recommended_face_width == null}
                  <!-- No recommendation exists for a crossed pair, so there is
                       nothing for an automatic toggle to take: showing one
                       would lock the field to a value nothing computed. -->
                  <label>
                    <span>Length</span>
                    <input type="number" step="1" bind:value={stage.worm.face_width.manual} />
                    <em>mm</em>
                  </label>
                {:else}
                  {@render autoNumber(
                    "Length",
                    stage.worm.face_width,
                    wres?.members[0].recommended_face_width ?? undefined,
                    1,
                  )}
                {/if}
                {#if wres?.members[0].recommended_face_width != null}
                  <p class="convention">
                    automatic uses {wres.members[0].recommended_face_width?.toFixed(2)} mm —
                    <code>(11 + 0.06 z₂) m_x</code> for one to three starts, DIN/ČSN practice. A
                    proportion, not a derivation: it sizes the part and enters no stress here.
                  </p>
                {/if}
                <label>
                  <span>Material</span>
                  <select bind:value={stage.worm.material}>
                    {#each library.materials.material as m (m.name)}
                      <option value={m.name}>{m.name}</option>
                    {/each}
                  </select>
                </label>
                {#if wres}
                  <dl class="out">
                    <dt>Lead angle</dt>
                    <dd>{wres.lead_angle.toFixed(4)}°</dd>
                    <dt>Lead</dt>
                    <dd>{wres.lead.toFixed(4)} mm</dd>
                    <dt>Torque</dt>
                    <dd>{wres.members[0].torque.toFixed(4)} N·m</dd>
                    <dt>Speed</dt>
                    <dd>{wres.members[0].speed.toFixed(1)} rpm</dd>
                    <dt>Backlash</dt>
                    <dd>{wres.backlash.backward.nominal.toFixed(5)}°</dd>
                  </dl>
                {/if}
              </div>

              <div class="gear">
                <h4>Wormwheel</h4>
                <label>
                  <span>Tooth count</span>
                  <input type="number" step="1" bind:value={stage.wheel_teeth} />
                </label>
                {#if wres && wres.members[1].recommended_face_width == null}
                  <label>
                    <span>Face width</span>
                    <input type="number" step="1" bind:value={stage.wheel.face_width.manual} />
                    <em>mm</em>
                  </label>
                {:else}
                  {@render autoNumber(
                    "Face width",
                    stage.wheel.face_width,
                    wres?.members[1].recommended_face_width ?? undefined,
                    1,
                  )}
                {/if}
                {#if wres}
                  <dl class="out">
                    <dt>Pitch diameter</dt>
                    <dd>{wres.members[1].pitch_diameter.toFixed(4)} mm</dd>
                    <dt>Torque</dt>
                    <dd>{wres.members[1].torque.toFixed(4)} N·m</dd>
                    <dt>Speed</dt>
                    <dd>{wres.members[1].speed.toFixed(1)} rpm</dd>
                    <dt>Backlash</dt>
                    <dd>{wres.backlash.forward.nominal.toFixed(5)}°</dd>
                  </dl>
                {/if}
              </div>
            </div>

            {#if wres}
              {@render screwReadout(wres)}
              {#if wres.notes.length}
                <ul class="notes">
                  {#each wres.notes as n (n)}<li>{n}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>Remove stage</button
            >
          </div>
        {/if}
      {:else}
        {@const pres = res && res.kind === "planetary" ? res : null}
        <button class="head" onclick={() => (open[i] = !open[i])}>
          <span class="caret">{open[i] ? "▾" : "▸"}</span>
          <strong>Stage {i + 1}</strong>
          <span class="kind">planetary</span>
          <span class="teeth">z {stage.sun.teeth} / {stage.planet.teeth} / {stage.ring.teeth}</span>
          {#if pres}
            <span class="ratio">{pres.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct(pres.efficiency.forward)} %</span>
          {/if}
        </button>
        {#if open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>Normal module</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>mm</em>
              </label>
              <label>
                <span>Pressure angle</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>Helix angle</span>
                <input type="number" step="1" bind:value={stage.helix_angle} />
                <em>°</em>
              </label>
              <label>
                <span>Friction, sun–planet</span>
                <input type="number" step="0.01" bind:value={stage.friction_sun_planet} />
                <em></em>
              </label>
              <label>
                <span>Friction, planet–ring</span>
                <input type="number" step="0.01" bind:value={stage.friction_planet_ring} />
                <em></em>
              </label>
              <label>
                <span>Tooth thickness mod.</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>k₁</em>
                {@render noteSlot(
                  notes(
                    "the sun's: above 1 its teeth thicken and the planet's thin by as much " +
                      "(2 − k), and the ring matches the planet — an internal mesh needs equal " +
                      "k, not complementary",
                    null,
                  ),
                )}
              </label>
              <label>
                <span>Planets</span>
                <input type="number" step="1" min="1" bind:value={stage.planets} />
                <em></em>
              </label>
              <label>
                <span>Driven by</span>
                <select bind:value={stage.arrangement.input}>
                  <option value="sun">Sun</option>
                  <option value="carrier">Carrier</option>
                  <option value="ring">Ring</option>
                </select>
                <em></em>
              </label>
              <label>
                <span>Held</span>
                <select bind:value={stage.arrangement.fixed}>
                  <option value="sun">Sun</option>
                  <option value="carrier">Carrier</option>
                  <option value="ring">Ring</option>
                </select>
                <em></em>
                {@render noteSlot(
                  notes("the third shaft is the output; a set needs both named", null),
                )}
              </label>
              <label>
                <span>C2C clearance</span>
                <input type="number" step="0.01" bind:value={stage.clearance} />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance +</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>mm</em>
              </label>
              <label>
                <span>C2C tolerance −</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>mm</em>
              </label>
              <label>
                <span>Minimum planet clearance</span>
                <input type="number" step="0.05" bind:value={stage.min_planet_clearance} />
                <em>mm</em>
                {@render noteSlot(notes("tip to tip, between neighbouring planets", null))}
              </label>
            </div>

            <h4>Ring cutter</h4>
            <div class="grid shared">
              <label>
                <span>Cutter teeth</span>
                <input type="number" step="1" min="1" bind:value={stage.cutter.teeth} />
                <em></em>
                {@render noteSlot(
                  notes("a ring is shaped by a pinion; its root and fillet are the tool's", null),
                )}
              </label>
              <label>
                <span>Cutter addendum</span>
                <input type="number" step="0.05" bind:value={stage.cutter.addendum} />
                <em>m</em>
              </label>
              <label>
                <span>Cutter tip round</span>
                <input type="number" step="0.02" bind:value={stage.cutter.tip_round} />
                <em>m</em>
              </label>
            </div>

            <!-- The planet alone is loaded on both flanks, once per revolution relative to
                 its carrier, so it is rated against a **reversed** bending allowable
                 derived from the material rather than the one-way figure the sun and ring
                 use (DESIGN §4.9). Shown where the number it changes is shown. -->
            {#snippet planetExtra(_j: number)}
              {#if pres}
                <dl class="out small">
                  <dt>Bending</dt>
                  <dd>
                    {pres.planet.fully_reversed ? "fully reversed" : "one-way"}
                    <small>allowable {pres.planet.reversed_allowable.value.toFixed(0)} MPa</small>
                  </dd>
                  <dt>Min face width</dt>
                  <dd>
                    {pres.planet.min_face_width_reversed === null
                      ? "—"
                      : `${pres.planet.min_face_width_reversed.toFixed(3)} mm`}
                    <small>against that allowable</small>
                  </dd>
                  <dt>Speed</dt>
                  <dd>
                    {pres.planet.speed_absolute.toFixed(1)} rpm
                    <small>{pres.planet.speed_relative.toFixed(1)} relative to the carrier</small>
                  </dd>
                </dl>
              {/if}
            {/snippet}

            <div class="gears">
              {@render gearCard("Sun", stage.sun, pres?.sun, { cut: "rack" })}
              {@render gearCard("Planet", stage.planet, pres?.planet.gear, {
                cut: "rack",
                solvedShift: pres?.planet.profile_shift,
                extra: planetExtra,
              })}
              <!-- A ring's root and fillet are its cutter's, so it has neither a
                   dedendum nor a root radius of its own (DESIGN §4.11); the tool
                   is a stage input, above. -->
              {@render gearCard("Ring", stage.ring, pres?.ring, { cut: "shaper" })}
            </div>

            {#if pres}
              <dl class="out">
                <dt>Ratio</dt>
                <dd>
                  {pres.ratio.toFixed(4)} : 1
                  <small>{pres.arrangement.input} in · {pres.arrangement.fixed} held · {pres.output} out</small>
                </dd>
                <dt>Centre distance</dt>
                <dd>
                  {pres.centre_distance.toFixed(4)} mm
                  <small>
                    common to both meshes — the planet's shift is what makes them agree, to
                    {pres.planet.shift_residual.toExponential(1)} mm
                  </small>
                </dd>
                <dt>Efficiency</dt>
                <dd>
                  {pct(pres.efficiency.forward)} % driven forward · {pct(pres.efficiency.backward)} %
                  driven backward
                  <small>fixed-carrier η₀ {pct(pres.fixed_carrier_efficiency.forward)} %</small>
                </dd>
                <dt>Backlash</dt>
                <dd>
                  {pres.backlash.forward.nominal.toFixed(5)}° at the {pres.output} shaft
                  <small
                    >({pres.backlash.forward.minimum.toFixed(5)} to {pres.backlash.forward.maximum.toFixed(
                      5,
                    )})</small
                  >
                </dd>
                <dt>Planet clearance</dt>
                <dd>
                  {pres.planet_clearance === null
                    ? "one planet has no neighbour"
                    : `${pres.planet_clearance.toFixed(3)} mm`}
                  {#if pres.planet_clearance !== null}
                    <small class:warn={!pres.planet_clearance_ok}>
                      {pres.planet_clearance_ok ? "meets the minimum" : "below the minimum asked for"}
                    </small>
                  {/if}
                </dd>
                <dt>Even spacing</dt>
                <dd>
                  {pres.equal_spacing ? "yes" : "no"}
                  <small>simultaneous meshing {pres.simultaneous_meshing ? "yes" : "no"}</small>
                </dd>
                <dt>Coprime</dt>
                <dd>
                  sun {pres.sun_coprime_with_planets ? "yes" : "no"} · ring
                  {pres.ring_coprime_with_planets ? "yes" : "no"}
                  <small>with the planet count</small>
                </dd>
              </dl>

              <!-- Two meshes, so the per-mesh figures a spur stage lists once are
                   a short table here rather than a second and third readout. -->
              <table>
                <thead>
                  <tr>
                    <th>mesh</th><th>ε<sub>α</sub></th><th>ε<sub>β</sub></th><th>η forward</th>
                    <th>σ<sub>H</sub></th><th>ρ</th>
                  </tr>
                </thead>
                <tbody>
                  {#each [["sun–planet", pres.sun_planet], ["planet–ring", pres.planet_ring]] as const as [label, m] (label)}
                    <tr>
                      <td>{label}</td>
                      <td>{m.contact_ratios.transverse.toFixed(4)}</td>
                      <td>{m.contact_ratios.overlap.toFixed(4)}</td>
                      <td>{pct(m.efficiency.forward)} %</td>
                      <td>{m.contact_stress.toFixed(1)} MPa</td>
                      <td>{m.relative_radius.toFixed(3)} mm</td>
                    </tr>
                  {/each}
                </tbody>
              </table>

              {#if pres.notes.length}
                <ul class="notes">
                  {#each pres.notes as n (n)}<li>{n}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>Remove stage</button
            >
          </div>
        {/if}

      {/if}
    </section>
  {/each}

  <button class="add" onclick={addStage}>+ Add spur stage</button>
  <button class="add" onclick={addWormStage}>+ Add worm stage</button>
  <button class="add" onclick={addPlanetaryStage}>+ Add planetary stage</button>
</div>

<style>
  header {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    margin-bottom: 0.75rem;
  }
  .title {
    font: inherit;
    font-size: 1.1rem;
    flex: 1;
    background: none;
    border: none;
    border-bottom: 1px solid var(--rule);
    color: var(--fg);
    padding: 0.2rem 0;
  }
  .actions {
    display: flex;
    gap: 0.35rem;
  }
  button {
    font: inherit;
    font-size: 0.8rem;
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--fg);
    cursor: pointer;
  }
  button:hover:not(:disabled) {
    background: var(--hover);
  }
  button:disabled {
    color: var(--muted);
    cursor: default;
  }
  .danger:hover {
    border-color: var(--warn);
    color: var(--warn);
  }
  .confirm {
    display: flex;
    gap: 0.5rem;
    align-items: center;
    padding: 0.5rem 0.75rem;
    border: 1px solid var(--warn);
    border-radius: 3px;
    margin-bottom: 0.75rem;
  }
  .train {
    border: 1px solid var(--rule);
    border-radius: 4px;
    padding: 0.75rem;
    margin-bottom: 1rem;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
    gap: 0.4rem 1rem;
  }
  /* A stage's shared inputs stack, not flow into columns. Wrapped into two or
     three columns a field's note sat beside the *next* field's box, and the
     column count changed with the window, so the same stage read differently at
     two widths. One column reads the way the gear cards below it do. */
  .grid.shared {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
    max-width: 34rem;
  }
  /* The **input box** is the anchor, not the text after it. With an `auto`
     trailing column the boxes shifted left or right by however wide a unit
     happened to be — "module" against "°" — so nothing lined up down a column.
     Every trailing column below is a fixed width, and where a row has two of
     them (a material property carries a unit *and* a provenance marker) they
     sum with their gap to the same width, so every box in a card shares an
     edge. */
  label {
    display: grid;
    grid-template-columns: 1fr 6rem 3.5rem;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }
  label span {
    color: var(--muted);
  }
  input[type="number"],
  select {
    font: inherit;
    font-size: 0.85rem;
    width: 100%;
    padding: 0.15rem 0.3rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--fg);
    font-variant-numeric: tabular-nums;
  }
  /* A computed value is shown greyed, so a default is never mistaken for a
     considered choice. */
  input.computed {
    color: var(--muted);
    font-style: italic;
  }
  em {
    color: var(--muted);
    font-size: 0.75rem;
    font-style: normal;
  }
  .mode {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.85rem;
  }
  .mode > span {
    color: var(--muted);
  }
  .segmented {
    display: flex;
  }
  .segmented button {
    border-radius: 0;
    font-size: 0.75rem;
  }
  .segmented button:first-child {
    border-radius: 3px 0 0 3px;
  }
  .segmented button:last-child {
    border-radius: 0 3px 3px 0;
    border-left: none;
  }
  .segmented button.on {
    background: var(--selected);
  }
  .toggle {
    font-size: 0.7rem;
    padding: 0.1rem 0.35rem;
    color: var(--muted);
  }
  .toggle.on {
    background: var(--selected);
    color: var(--fg);
  }
  /* The source of a shipped convention, next to the number it produced —
     the project's rule is to say what a figure is where it is shown. */
  .convention {
    grid-column: 1 / -1;
    margin: -0.1rem 0 0.4rem;
    font-size: 0.72rem;
    line-height: 1.4;
    color: var(--muted);
  }
  .convention code {
    font-size: 0.72rem;
  }
  .out {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.15rem 0.75rem;
    margin: 0.75rem 0 0;
    font-size: 0.85rem;
  }
  .out.small {
    font-size: 0.8rem;
    margin-top: 0.5rem;
  }
  .out dt {
    color: var(--muted);
  }
  .out dd {
    margin: 0;
    font-variant-numeric: tabular-nums;
  }
  .out small {
    color: var(--muted);
    margin-left: 0.35rem;
  }
  .warn {
    color: var(--warn) !important;
  }
  .stages {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .stage {
    border: 1px solid var(--rule);
    border-radius: 4px;
  }
  .head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 4px;
    padding: 0.45rem 0.7rem;
    font-size: 0.9rem;
  }
  .caret {
    color: var(--muted);
  }
  .teeth,
  .ratio,
  .eff {
    color: var(--muted);
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
  }
  .eff {
    margin-left: auto;
  }
  .body {
    padding: 0 0.7rem 0.7rem;
    border-top: 1px solid var(--rule);
  }
  .shared {
    margin-top: 0.6rem;
  }
  .gears {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(20rem, 1fr));
    gap: 1rem;
    margin-top: 0.8rem;
  }
  .gear {
    border: 1px solid var(--rule);
    border-radius: 3px;
    padding: 0.5rem 0.7rem;
  }
  .gear h4 {
    margin: 0 0 0.4rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .sub {
    font-size: 0.78rem;
  }
  .sub span {
    padding-left: 0.8rem;
  }
  .subtoggles {
    display: flex;
    gap: 0.9rem;
    padding: 0 0 0.3rem 0.8rem;
  }
  .check {
    display: flex;
    grid-template-columns: none;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.75rem;
    margin: 0;
  }
  .check input {
    width: auto;
  }
  .props {
    margin: 0.2rem 0 0.4rem;
  }
  /* More specific than `.gear label`, which would otherwise win and collapse
     the basis marker onto its own line. */
  .gear .prop {
    /* 2.2 + 0.4 gap + 0.9 = 3.5rem, the same trailing width as a plain row, so
       these boxes share the edge with the ones above them. */
    grid-template-columns: 1fr 6.5rem 2.2rem 0.9rem;
    font-size: 0.78rem;
    margin-bottom: 0.15rem;
  }
  .basis {
    width: 1ch;
    text-align: center;
    opacity: 0.55;
  }
  .basis.weak {
    color: var(--warn);
    opacity: 0.9;
  }
  .clear {
    font-size: 0.75rem;
    line-height: 1;
    padding: 0.05rem 0.3rem;
    color: var(--muted);
  }
  /* Outside a label's grid, so it is padded by the trailing column's width plus
     its gap to finish on the same edge. */
  .hint {
    margin: -0.1rem 0 0.3rem;
    padding-right: 3.9rem;
    font-size: 0.72rem;
    color: var(--muted);
    text-align: right;
  }
  /* See the note slot's own comment: candidates stack, the tallest sets the
     height, and nothing moves when the visible one changes. */
  .aside {
    grid-column: 1 / -1;
    margin: 0 0 0.35rem;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .aside.wide {
    margin: 0.2rem 0 0.5rem;
    max-width: 60rem;
  }
  /* A note belongs to the box above it, so it ends where that box ends: it
     spans the label and input columns only, and is right-aligned within them.
     Running it to the row's full width ended it past the unit, against nothing. */
  .note {
    grid-column: 1 / 3;
    display: grid;
    text-align: right;
  }
  .note small {
    grid-area: 1 / 1;
    font-size: 0.72rem;
    color: var(--muted);
  }
  .note small.hidden {
    visibility: hidden;
  }
  .err {
    color: var(--warn);
  }
  label.invalid input {
    border-color: var(--warn);
  }
  .gear label {
    grid-template-columns: 1fr 6.5rem 3.5rem;
    margin-bottom: 0.25rem;
  }
  .notes {
    margin: 0.5rem 0 0;
    padding-left: 1rem;
    font-size: 0.78rem;
    color: var(--warn);
  }
  .error {
    color: var(--warn);
  }
  .add {
    align-self: flex-start;
    border-style: dashed;
    color: var(--muted);
  }
  .small {
    margin-top: 0.6rem;
    font-size: 0.75rem;
  }
</style>
