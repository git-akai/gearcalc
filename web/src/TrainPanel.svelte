<script lang="ts">
  import {
    solveTrain,
    defaultStage,
    defaultWormStage,
    outside,
    type Auto,
    type Overrides,
    type StageGear,
    type MaterialValue,
  } from "./core";
  import { trains, library, type TrainTab } from "./state.svelte";

  let { tab }: { tab: TrainTab } = $props();

  let confirmingDelete = $state(false);
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
    <button onclick={() => trains.create()}>New</button>
    <button onclick={() => trains.copy(tab.id)}>Copy</button>
    <button class="danger" onclick={() => (confirmingDelete = true)}>Delete</button>
  </div>
</header>

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
          >({result.ok.backlash.forward.minimum.toFixed(5)} – {result.ok.backlash.forward.maximum.toFixed(
            5,
          )})</small
        >
      </dd>
      <dt>Backlash at the input shaft</dt>
      <dd>
        {result.ok.backlash.backward.nominal.toFixed(5)}°
        <small
          >({result.ok.backlash.backward.minimum.toFixed(5)} – {result.ok.backlash.backward.maximum.toFixed(
            5,
          )})</small
        >
        <small>the same play, at a shaft turning the whole ratio faster</small>
      </dd>
    </dl>
  {/if}
</section>

<div class="stages">
  {#each tab.train.stages as stage, i (i)}
    {@const res = "ok" in result ? result.ok.stages[i] : null}
    <section class="stage">
      {#if stage.kind === "spur"}
        {@const sres = res && res.kind === "spur" ? res : null}
        <button class="head" onclick={() => (open[i] = !open[i])}>
          <span class="caret">{open[i] ? "▾" : "▸"}</span>
          <strong>Stage {i + 1}</strong>
          <span class="teeth">z {stage.gears[0].teeth} / {stage.gears[1].teeth}</span>
          {#if sres}
            <span class="ratio">{sres.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct(sres.efficiency.forward)} %</span>
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
                <span>Coefficient of friction</span>
                <input type="number" step="0.01" bind:value={stage.friction} />
                <em></em>
              </label>
              <label>
                <span>Tooth thickness mod.</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>k₁</em>
              </label>
              {@render autoNumber("C2C distance", stage.centre_distance, sres?.centre_distance, 0.1)}
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

            <div class="gears">
              {#each stage.gears as gear, j (j)}
                {@const g = sres?.gears[j]}
                <div class="gear">
                  <h4>Gear {gearNumber(i, j)}</h4>
                  <label class:invalid={g && outside(gear.teeth, g.ranges.teeth)}>
                    <span>Tooth count</span>
                    <input type="number" step="1" bind:value={gear.teeth} />
                  </label>
                  <label class:invalid={g && outside(gear.dedendum, g.ranges.dedendum)}>
                    <span>Dedendum</span>
                    <input type="number" step="0.05" bind:value={gear.dedendum} />
                    {#if g}
                      {@const bad = outside(gear.dedendum, g.ranges.dedendum)}
                      <small class:err={bad}>
                        {bad ?? `${n(g.ranges.dedendum.min ?? 0)} … ${n(g.ranges.dedendum.max ?? 0)}`}
                      </small>
                    {/if}
                  </label>
                  <label class:invalid={g && outside(gear.root_radius, g.ranges.root_radius)}>
                    <span>Root radius</span>
                    <input type="number" step="0.01" bind:value={gear.root_radius} />
                    {#if g}
                      {@const bad = outside(gear.root_radius, g.ranges.root_radius)}
                      <small class:err={bad}>
                        {bad ?? `up to ${n(g.ranges.root_radius.max ?? 0)} · fillet must fit`}
                      </small>
                    {/if}
                  </label>
                  {@render autoNumber("Profile shift", gear.profile_shift, g?.profile_shift, 0.05)}
                  {#if gear.profile_shift.auto}
                    <label class="sub">
                      <span>Working tooth depth</span>
                      <input type="number" step="0.05" bind:value={gear.working_depth} />
                      <em>module</em>
                    </label>
                  {/if}
                  {#if g && !gear.profile_shift.auto}
                    {@const r = g.ranges.profile_shift}
                    {@const bad = outside(gear.profile_shift.manual, r.bound)}
                    <p class="hint" class:err={bad}>
                      {bad ??
                        `buildable ${n(r.bound.min ?? 0)} … ${n(r.bound.max ?? 0)} · undercut below ${n(r.undercut)}`}
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
                  {@render autoNumber("Face width", gear.face_width, g?.face_width, 0.5)}
                  {#if gear.face_width.auto}
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
                </div>
              {/each}
            </div>

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
                  {#if stage.helix_angle !== 0 && sres.contact_ratios.overlap < 1}
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
                    >({sres.backlash.forward.minimum.toFixed(5)} – {sres.backlash.forward.maximum.toFixed(
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
      {:else}
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
                <h4>Worm</h4>
                <label>
                  <span>Starts</span>
                  <input type="number" step="1" bind:value={stage.starts} />
                </label>
                <label>
                  <span>Pitch diameter</span>
                  <input type="number" step="0.5" bind:value={stage.worm_pitch_diameter} />
                  <em>mm</em>
                </label>
                <label>
                  <span>Length</span>
                  <input type="number" step="1" bind:value={stage.worm.face_width} />
                  <em>mm</em>
                </label>
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
                <label>
                  <span>Face width</span>
                  <input type="number" step="1" bind:value={stage.wheel.face_width} />
                  <em>mm</em>
                </label>
                <label>
                  <span>Material</span>
                  <select bind:value={stage.wheel.material}>
                    {#each library.materials.material as m (m.name)}
                      <option value={m.name}>{m.name}</option>
                    {/each}
                  </select>
                </label>
                {#if wres}
                  <dl class="out">
                    <dt>Helix angle</dt>
                    <dd>{wres.wheel_helix_angle.toFixed(4)}°</dd>
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
              <dl class="out">
                <dt>Centre distance</dt>
                <dd>
                  {wres.centre_distance.toFixed(4)} mm
                  <small>nominal {wres.centre_distance_nominal.toFixed(4)}</small>
                </dd>
                <dt>Mesh efficiency</dt>
                <dd>
                  {pct(wres.efficiency.forward)} % driven forward
                  · {pct(wres.efficiency.backward)} % driven backward
                  {#if wres.efficiency.backward <= 0}
                    <small class="warn">self-locking</small>
                  {/if}
                </dd>
                <dt>Self-locks at μ</dt>
                <dd>{wres.self_locking_friction.toFixed(4)}</dd>
                <dt>Contact stress</dt>
                <dd>
                  {wres.contact.max_pressure.toFixed(1)} MPa
                  <small>
                    patch {wres.contact.patch_length.toFixed(4)} × {wres.contact.patch_width.toFixed(
                      4,
                    )} mm
                  </small>
                </dd>
                <dt>Sliding speed</dt>
                <dd>{wres.sliding_velocity.toFixed(1)} mm/s</dd>
                <dt>Bending stress</dt>
                <dd>
                  <small
                    >not reported — a worm wheel's tooth form and load case are not the ones this
                    bending model measures (DESIGN §4.5.1)</small
                  >
                </dd>
                <dt>Flank type</dt>
                <dd>
                  ZI (involute helicoid)
                  <small>a ZN worm's contact stress is 1–15 % lower, rising with lead angle</small>
                </dd>
              </dl>
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

      {/if}
    </section>
  {/each}

  <button class="add" onclick={addStage}>+ Add spur stage</button>
  <button class="add" onclick={addWormStage}>+ Add worm stage</button>
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
  label {
    display: grid;
    grid-template-columns: 1fr 6rem auto;
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
    grid-template-columns: 1fr 6.5rem 2.2rem auto;
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
  .hint {
    margin: -0.1rem 0 0.3rem;
    font-size: 0.72rem;
    color: var(--muted);
    text-align: right;
  }
  .err {
    color: var(--warn);
  }
  label.invalid input {
    border-color: var(--warn);
  }
  .gear label {
    grid-template-columns: 1fr 6.5rem auto;
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
