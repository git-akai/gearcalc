<script lang="ts">
  import type { Snippet } from "svelte";
  import {
    defaults,
    solveTrain,
    STAGE_KINDS,
    type StageKindSpec,
    outside,
    type Auto,
    type Overrides,
    type StageGear,
    type SpurStage,
    type Optimisation,
    type Value,
    type GearResult,
    type WormResult,
    note,
    t,
  } from "./core";
  import { developer, trains, library, type TrainTab } from "./state.svelte";
  import { exportTrain, relieve } from "./core";
  import FieldNote from "./FieldNote.svelte";
  import Switch from "./Switch.svelte";
  import { notes, type Notes } from "./notes";

  /** A mesh chooses one shift per gear, and its centre distance is one relation
   *  among them — so at most `gears.length` of {distance, shift…} can be given,
   *  and the count comes from the stage rather than from a number written here.
   *
   *  Only while the stage is choosing them at all: with the optimiser off the
   *  shifts are not being solved for anything, so pinning every one of them is
   *  the fully specified design it always was and nothing is relieved. */
  function relieveSpur(stage: SpurStage, just: Auto<number>) {
    if (!stage.optimisation.enabled) return;
    relieve(
      [stage.centre_distance, ...stage.gears.map((g) => g.profile_shift)],
      stage.gears.length,
      just,
    );
  }

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

  // Which stages are expanded lives on the **tab**, so looking at another
  // train — or at a gear — and coming back finds the panel as it was left.
  // Read through `tab` at each use rather than aliased, so there is one object
  // and no question about which of the two a write lands on.

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
        ? { intermittent: { range_degrees: 25, actuations: 1000, reversing: false } }
        : // The operating speed starts at the peak, which is the one value that
          // is certainly admissible and asserts nothing about the drive.
          { continuous: { operating_speed: tab.train.input_speed, runtime_hours: 1000 } };
  }

  /** The kinds on offer. A kind the developer mode hides cannot already be in
   *  a train the reader is looking at — the picker is the only way one arrives
   *  — so nothing is stranded by the mode being off. */
  const stageKinds = $derived(STAGE_KINDS.filter((k) => !k.developer || developer.enabled));

  function addStageOfKind(kind: StageKindSpec) {
    tab.train.stages.push(kind.fresh());
    tab.open[tab.train.stages.length - 1] = true;
  }




  function removeStage(i: number) {
    tab.train.stages.splice(i, 1);
    // The expansions are keyed by index, so the ones after the hole move down
    // with the stages they belong to. Left alone, deleting a stage reopens
    // whichever stage inherited its number.
    const was = { ...tab.open };
    tab.open = {};
    for (const [k, v] of Object.entries(was)) {
      const at = Number(k);
      if (at < i) tab.open[at] = v;
      else if (at > i) tab.open[at - 1] = v;
    }
  }

  /** Gear numbering runs across the whole train: stage 1 is gears 1 and 2,
   *  stage 2 is gears 3 and 4, as the specification describes. */
  function gearNumber(stage: number, which: number): number {
    return stage * 2 + which + 1;
  }

  /** The candidates for one note slot: a blank to reserve the space, the note
   *  itself when the stage has solved, and the out-of-range message when the
   *  value is outside its bound. All are rendered; see the slot's comment. */

  const pct = (v: number) => (100 * v).toFixed(3);
  const n = (v: number) => v.toFixed(3);
  /** "lo to hi" — one shape, so the word between two numbers is written once. */
  const range = (lo: string, hi: string) => t("ui.range", { lo, hi });
  /** A planetary shaft's name, from the same key its own option in the
   *  arrangement selects uses — the readout used to print the wire value, which
   *  is an identifier and was never English to begin with. */
  const shaft = (s: string) => t(`ui.train_${s}`);
  /** A stage or a gear by number. The *word* is a label and belongs in the
   *  catalogue; the number is a name and does not. Both are written here rather
   *  than at each of the five places they are read, so a heading and a reference
   *  to it cannot drift apart. */
  const stageName = (i: number) => t("ui.train_stage_heading", { number: String(i + 1) });
  const gearName = (stage: number, which: number) =>
    t("ui.train_gear_name", { number: String(gearNumber(stage, which)) });
  /** A mesh efficiency, both ways round. Two keys rather than one sentence: the
   *  two halves are separated by a bullet the grammar of no language owns. */
  const bothWays = (e: { forward: number; backward: number }) =>
    `${t("ui.train_driven_forward", { percent: pct(e.forward) })} · ${t(
      "ui.train_driven_backward",
      { percent: pct(e.backward) },
    )}`;

  /** A rating in both load cases, written in the order they are read: what the
   *  part must survive once, then what it must survive for the duty.
   *
   *  Formatting, not arithmetic — every number here came from Rust. A rating
   *  that does not exist for a member renders as a dash rather than as a zero,
   *  because those are different facts. */
  const cases = (v: { peak: number | null; cyclic: number | null }, digits: number) =>
    `${v.peak === null ? "—" : v.peak.toFixed(digits)} / ${
      v.cyclic === null ? "—" : v.cyclic.toFixed(digits)
    }`;

</script>

<!-- A value + automatic toggle, locked while automatic (docs/rationale.md#inputs-are-the-only-state).
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
  used: Value | undefined,
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
        title={t("ui.train_restore_library_value")}
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
<!-- A boolean that belongs in a column of fields. The switch carries the
     field's own name rather than a bare "on" beside a label that would then say
     it twice, and sits at the right edge the inputs share — where the actuation
     control's buttons sit, which is the other switch of this size in the panel.
     The four width sources need no row at all: they are a group, and the group
     has the heading. -->
{#snippet switchField(key: string, on: boolean, set: (v: boolean) => void, note?: string | null)}
  <label class="switchrow">
    <span class="control"><Switch label={t(key)} {on} {set} /></span>
    {#if note !== undefined}
      <FieldNote notes={notes(note, null)} />
    {/if}
  </label>
{/snippet}


<!-- One gear card, used by every stage that has gears. A sun, a planet, a ring
     and a spur gear take the same inputs and produce the same readout, so they
     are one definition rather than several that drift apart — which is what had
     happened to the planetary section. What genuinely differs is passed in: a
     ring's root belongs to its cutter, a planet's shift is solved rather than
     chosen, and a member may have something of its own to report. -->
<!-- What a crossed-axis mesh reports, whether it was entered as a worm drive or
     as a gear pair with its shafts turned: the same mathematics answers both
     (docs/reference.md#crossed-axes), so it is one readout rather than two that drift. -->
{#snippet screwReadout(r: WormResult, members: [string, string])}
<!-- Ordered to match the spur stage's shared readout — centre distance,
     contact ratio, efficiency, backlash — with what only a screw pair has
     following on. The backlash lives here rather than on the two member cards
     for the same reason it does there: it is one gap seen from two ends, not a
     property either member owns. -->
<dl class="out">
  <dt>{t("ui.train_centre_distance")}</dt>
  <dd>
    {r.centre_distance.toFixed(4)} mm
    <small>{t("ui.train_nominal_value", { value: r.centre_distance_nominal.toFixed(4) })}</small>
  </dd>
  {#if r.crossed}
    <dt>{t("ui.train_contact_ratio")}</dt>
    <dd>
      <span class:warn={r.crossed.contact_ratio < 1}>
        ε {r.crossed.contact_ratio.toFixed(4)}
      </span>
      <small>
        {r.crossed.contact_ratio < 1
          ? t("ui.train_note_contact_ratio_below_one")
          : t("ui.train_crossed_pairs_in_contact", {
              limit: t(
                r.crossed.limited_by === "face"
                  ? "ui.train_limited_by_face_width"
                  : "ui.train_limited_by_teeth",
              ),
            })}
        {#if r.crossed.tooth_height_assumed}
          {t("ui.train_crossed_height_assumed")}
        {/if}
      </small>
    </dd>
  {/if}
  <dt>{t("ui.train_mesh_efficiency")}</dt>
  <dd>
    {bothWays(r.efficiency)}
    {#if r.efficiency.backward <= 0}
      <small class="warn">{t("ui.train_self_locking")}</small>
    {/if}
    {#if r.crossed?.parallel_axis_efficiency != null}
      <small>
        {t("ui.train_parallel_shafts_would_give", {
          percent: pct(r.crossed.parallel_axis_efficiency),
        })}
      </small>
    {/if}
  </dd>
  <dt>{t("ui.train_backlash")}</dt>
  <dd>
    {t("ui.train_backlash_at", {
      angle: r.backlash.forward.nominal.toFixed(5),
      member: members[1],
    })}
    <small
      >({range(r.backlash.forward.minimum.toFixed(5), r.backlash.forward.maximum.toFixed(5))})</small
    >
    · {t("ui.train_backlash_at", {
      angle: r.backlash.backward.nominal.toFixed(5),
      member: members[0],
    })}
  </dd>
  <dt>{t("ui.train_self_locks_at")}</dt>
  <dd>{r.self_locking_friction.toFixed(4)}</dd>
  <dt>{t("ui.train_contact_stress")}</dt>
  <dd>
    {cases({ peak: r.contact.peak.max_pressure, cyclic: r.contact.cyclic.max_pressure }, 1)} {t("ui.train_mpa")}
    <small>{t("ui.train_peak_cyclic")}</small>
    <small>
      {t("ui.train_patch", {
        length: r.contact.peak.patch_length.toFixed(4),
        width: r.contact.peak.patch_width.toFixed(4),
      })} ·
      {Math.abs(r.contact.peak.worst_position) < 1e-9
        ? t("ui.train_worst_at_pitch_point")
        : t("ui.train_worst_along_the_path", {
            position: r.contact.peak.worst_position.toFixed(3),
          })}
      · {t("ui.train_pitch_point_alone_gives", {
        stress: r.contact.peak.at_pitch_point.toFixed(1),
      })}
    </small>
  </dd>
  <dt>{t("ui.train_sliding_speed")}</dt>
  <dd>{r.sliding_velocity.toFixed(1)} mm/s</dd>
  <dt>{t("ui.train_bending_stress")}</dt>
  <dd>
    <small>{t("ui.train_not_reported_for_crossed_axes_no")}</small>
  </dd>
  <dt>{t("ui.train_flank_type")}</dt>
  <dd>
    {t("ui.train_flank_type_zi")}
    <small>{t("ui.train_zn_worm_s_contact_stress_1")}</small>
  </dd>
  {#if r.crossed}
    <dt>{t("ui.train_contact_travel")}</dt>
    <dd>
      {r.crossed.axial_travel[0].toFixed(3)} · {r.crossed.axial_travel[1].toFixed(3)} mm
      <small>{t("ui.train_along_each_member_s_own_axis")}</small>
    </dd>
  {/if}
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
    /** Called when this gear's shift is switched between given and automatic,
     *  so a stage whose inputs constrain one another can relieve whichever of
     *  them that has over-specified. */
    onShiftAuto?: () => void;
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
    <span>{t("ui.train_tooth_count")}</span>
    <input type="number" step="1" bind:value={gear.teeth} />
  </label>
  {@render autoNumber(
    "ui.train_addendum",
    gear.addendum,
    g?.addendum,
    0.05,
    undefined,
    undefined,
    "ui.train_m",
  )}
  {#if gear.addendum.auto}
    <label class="sub">
      <span>{t("ui.train_minimum_tip_width")}</span>
      <input type="number" step="0.02" bind:value={gear.min_tip_width} />
      <em>{t("ui.train_mm")}</em>
    </label>
  {/if}
  {#if opts.cut !== "shaper"}
    <label class:invalid={g && outside(gear.dedendum, g.ranges.dedendum)}>
      <span>{t("ui.train_dedendum")}</span>
      <input type="number" step="0.05" bind:value={gear.dedendum} />
      <em>{t("ui.train_m")}</em>
      <!-- The same sentences the gear tab shows. They used to be written out
           here as well, and drifted: this one lost its reason altogether and
           the fillet bound below was abbreviated past the point of saying
           anything. -->
      <FieldNote notes={
        notes(
          g
            ? t("ui.bound_dedendum", {
                min: n(g.ranges.dedendum.min ?? 0),
                max: n(g.ranges.dedendum.max ?? 0),
              })
            : null,
          g ? outside(gear.dedendum, g.ranges.dedendum) : null,
        )
      } />
    </label>
    <label class:invalid={g && outside(gear.root_radius, g.ranges.root_radius)}>
      <span>{t("ui.train_root_radius")}</span>
      <input type="number" step="0.01" bind:value={gear.root_radius} />
      <em>{t("ui.train_m")}</em>
      <FieldNote notes={
        notes(
          g ? t("ui.bound_root_radius", { max: n(g.ranges.root_radius.max ?? 0) }) : null,
          g ? outside(gear.root_radius, g.ranges.root_radius) : null,
        )
      } />
    </label>
  {/if}
  {#if opts.solvedShift === undefined}
    {@render autoNumber(
      "ui.train_profile_shift",
      gear.profile_shift,
      g?.profile_shift,
      0.05,
      opts.onShiftAuto,
      undefined,
      "ui.train_m",
    )}
    {#if gear.profile_shift.auto}
      <!-- Automatic is the gear's own dedendum, which asks the same question the
           profile generator answers: is the flank undercut *at all*? A fixed 1
           module — what this used to be — asks whether it is undercut within a
           module of depth, and the two part company at 18 teeth and 22. -->
      {@render autoNumber(
        "ui.train_working_tooth_depth",
        gear.working_depth,
        gear.dedendum,
        0.05,
        undefined,
        // It used to sit indented under the shift, which said "this belongs to
        // that" without words. The indent went when it became an `auto` field
        // like its neighbours, so the note says it instead.
        t("ui.train_note_working_depth"),
        "ui.train_m",
      )}
    {/if}
  {:else}
    <label>
      <span>{t("ui.train_profile_shift")}</span>
      <input type="number" value={Number(opts.solvedShift.toFixed(4))} disabled class="computed" />
      <em>{t("ui.train_module")}</em>
      <FieldNote notes={notes(t("ui.train_note_planet_shift_solved"), null)} />
    </label>
  {/if}
  {#if opts.solvedShift === undefined && !gear.profile_shift.auto}
    {@const r = opts.cut === "shaper" ? undefined : g?.ranges.profile_shift}
    <p class="hint">
      <!-- A shaper-cut ring's bounds are not the rack's shown here — its own
           base circle, its cutter's reach and the generation limit are what
           limit it (docs/reference.md#internal-gears) — and the core does not report those for a
           stage member yet. It shows no bound rather than the wrong one. -->
      <FieldNote notes={
        notes(
          opts.cut === "shaper"
            ? null
            : r
              ? t(r.pointed === null ? "ui.bound_profile_shift" : "ui.bound_profile_shift_pointed", {
                  min: n(r.bound.min ?? 0),
                  max: n(r.bound.max ?? 0),
                  undercut: n(r.undercut),
                  sharp: n(r.sharp_rack_undercut),
                  pointed: n(r.pointed ?? 0),
                })
              : null,
          r ? outside(gear.profile_shift.manual, r.bound) : null,
        )
      } />
    </p>
  {/if}
  {#if opts.faceAuto === false}
    <!-- A crossed pair's automatic width is a **geometric** minimum: the width
         at which one tooth pair hands over to the next (ε = 1). The spur
         stage's inverts a stress instead, and the two must not read alike. -->
    {@render autoNumber(
      "ui.train_face_width",
      gear.face_width,
      opts.faceFromContinuity,
      0.5,
      undefined,
      undefined,
      "ui.train_mm",
    )}
    <FieldNote notes={
      notes(
        opts.faceFromContinuity === undefined
          ? t("ui.train_note_no_continuous_width")
          : t("ui.train_note_face_width_continuity", { width: n(opts.faceFromContinuity) }),
        null,
      )
    } />
  {:else}
    {@render autoNumber(
      "ui.train_face_width",
      gear.face_width,
      g?.face_width,
      0.5,
      undefined,
      undefined,
      "ui.train_mm",
    )}
  {/if}
  {#if gear.face_width.auto && opts.faceAuto !== false}
    <!-- Four ratings, so four toggles: a rating exists for every combination
         of what fails (bending or contact) and what it is rated against (the
         peak load, against the ultimate, or the cyclic one, against fatigue).
         The width is the largest any enabled rating asks for. With none of them
         enabled there is nothing to invert and the width comes out zero, which
         the stage says in a note rather than hiding. -->
    <div class="subtoggles">
      <Switch
        label={t("ui.train_from_bending_peak")}
        on={gear.face_sources.bending.peak}
        set={(v) => (gear.face_sources.bending.peak = v)}
      />
      <Switch
        label={t("ui.train_from_bending_cyclic")}
        on={gear.face_sources.bending.cyclic}
        set={(v) => (gear.face_sources.bending.cyclic = v)}
      />
      <Switch
        label={t("ui.train_from_contact_peak")}
        on={gear.face_sources.contact.peak}
        set={(v) => (gear.face_sources.contact.peak = v)}
      />
      <Switch
        label={t("ui.train_from_contact_cyclic")}
        on={gear.face_sources.contact.cyclic}
        set={(v) => (gear.face_sources.contact.cyclic = v)}
      />
    </div>
  {/if}
  <label>
    <span>{t("ui.train_material")}</span>
    <select bind:value={gear.material}>
      {#each library.materials.material as m (m.name)}
        <option value={m.name}>{m.name}</option>
      {/each}
    </select>
  </label>

  {#if g}
    <div class="props">
      {@render property(t("ui.train_density"), gear, "density", g.material.density, 10, t("ui.train_kg_m3"))}
      {@render property(t("ui.train_elastic_modulus"), gear, "elastic_modulus", g.material.elastic_modulus, 100, t("ui.train_mpa"))}
      {@render property(t("ui.train_poissons_ratio"), gear, "poissons_ratio", g.material.poissons_ratio, 0.01, "")}
      {@render property(t("ui.train_ultimate_allowable"), gear, "ultimate_allowable", g.material.ultimate_allowable, 10, t("ui.train_mpa"))}
      {@render property(t("ui.train_fatigue_allowable"), gear, "fatigue_allowable", g.material.fatigue_allowable, 10, t("ui.train_mpa"))}
    </div>
    <dl class="out small">
      <dt>{t("ui.train_torque")}</dt>
      <dd>{g.torque.toFixed(4)} Nm</dd>
      <!-- Only where there is one. A back-driving load that nothing reacts
           reaches no gear, and an empty row is the honest report of that. -->
      {#if g.back_driving_torque !== null}
        <dt>{t("ui.train_back_driving_torque")}</dt>
        <dd>{g.back_driving_torque.toFixed(4)} Nm</dd>
      {/if}
      <dt>{t("ui.train_speed")}</dt>
      <dd>{g.speed.toFixed(1)} {t("ui.train_rpm")}</dd>
      <dt>{t("ui.train_tooth_cycles")}</dt>
      <dd>
        {g.tooth_cycles.bending.toLocaleString()} / {g.tooth_cycles.contact.toLocaleString()}
        <small>{t("ui.train_bending_contact")}</small>
      </dd>
      <dt>{t("ui.train_bending_stress")}</dt>
      <dd>
        {cases(g.bending_stress, 1)} {t("ui.train_mpa")}
        <small>{t("ui.train_peak_cyclic")}</small>
      </dd>
      <!-- Per gear, and genuinely so. The two flanks share one pressure at any
           instant — the individual curvatures reach Hertz only through their
           sum — but the two gears are not rated at the same instant: each one's
           dedendum carries the load alone at its own end of the path, and that
           is where its pitting is assessed. -->
      <dt>{t("ui.train_contact_stress")}</dt>
      <dd>
        {cases(g.contact_stress, 1)} {t("ui.train_mpa")}
        <small>{t("ui.train_peak_cyclic")}</small>
      </dd>
      <dt>{t("ui.train_min_face_width")}</dt>
      <dd>
        <span class="line">
          {cases({ peak: g.min_face_width.peak.bending, cyclic: g.min_face_width.cyclic.bending }, 3)} mm
          <small>{t("ui.train_bending_peak_cyclic")}</small>
        </span>
        <span class="line">
          {cases({ peak: g.min_face_width.peak.contact, cyclic: g.min_face_width.cyclic.contact }, 3)} mm
          <small>{t("ui.train_contact_peak_cyclic")}</small>
        </span>
      </dd>
    </dl>
    {#if g.clamps.length || g.notes.length}
      <ul class="notes">
        {#each g.clamps as c, i (i)}<li>{t("ui.gear_clamped")} {note(c)}</li>{/each}
        <!-- The rating's own remarks, unprefixed: nothing here was clamped.
             Keyed by position rather than by note key, because two notes of one
             kind on one list is a thing that happens and a keyed list must not
             be what discovers it. -->
        {#each g.notes as n, i (i)}<li>{note(n)}</li>{/each}
      </ul>
    {/if}
  {/if}
  <!-- Outside the guard above: a crossed pair produces no per-gear rating, so
       `g` is absent exactly when this readout is the only one there is. -->
  {@render (opts.extra ?? noExtra)(opts.extraIndex ?? 0)}
</div>
{/snippet}

<!-- `key` rather than a label, so these read from the catalogue like every
     other piece of chrome. Passing the English through as an argument is how
     five labels stayed hard-coded through the extraction that caught the other
     185: they are not markup, so nothing scanning markup could see them. -->
<!-- `after` is how a stage says that its inputs constrain one another: the
     toggle flips, then the stage relieves whatever that has over-specified
     (`relieve`). Stages that have no such rule pass nothing and behave as they
     always have. -->
{#snippet autoNumber(
  key: string,
  a: Auto<number>,
  computed: number | undefined,
  step: number,
  after?: () => void,
  note?: string | null,
  unit?: string,
)}
  <label class="auto">
    <span>{t(key)}</span>
    <!-- **Left of the number it qualifies**, because that is what it qualifies.
         On the right it took the cell every other row prints its unit in, so an
         automatic field was the one field that could not say what it was
         measured in. -->
    <Switch
      small
      label={t("ui.train_auto")}
      on={a.auto}
      title={t("ui.train_automatic")}
      set={(v) => {
        a.auto = v;
        after?.();
      }}
    />
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
    <em>{unit ? t(unit) : ""}</em>
    <!-- Inside the label, because that is where a note is laid out: `.note`
         spans this row's own columns and is right-aligned against them. Placed
         beside the field instead it spans whatever grid it lands in, which is
         the outer one, and lines up with nothing. -->
    {#if note !== undefined}
      <FieldNote notes={notes(note, null)} />
    {/if}
  </label>
{/snippet}

<!-- **What the automatic shifts are chosen for.** Off, they are the least that
     clears undercut, which is the smallest admissible pair rather than the best
     one; on, they are chosen to lose least with that undercut shift as a floor.
     The contact ratio comes with it because it is the constraint the answer sits
     against: sliding loss falls with the length of the path, so without a floor
     the least-loss pair is always the one whose teeth barely reach. -->
{#snippet efficiencyToggle(o: Optimisation, after?: () => void)}
  {@render switchField(
    "ui.train_optimise_efficiency",
    o.enabled,
    (v) => {
      o.enabled = v;
      after?.();
    },
    t("ui.train_note_optimise_efficiency"),
  )}
  {#if o.enabled}
    <label class="sub">
      <span>{t("ui.train_min_contact_ratio")}</span>
      <input type="number" step="0.05" bind:value={o.min_contact_ratio} />
      <em>{t("ui.train_epsilon")}</em>
      <FieldNote notes={notes(t("ui.train_note_min_contact_ratio"), null)} />
    </label>
  {/if}
{/snippet}

<header>
  <input class="title" bind:value={tab.name} aria-label={t("ui.train_name")} />
  <div class="actions">
    <button onclick={saveTrain}>{t("ui.train_export")}</button>
    <button onclick={() => picker.click()}>{t("ui.train_import")}</button>
    <button onclick={() => trains.create()}>{t("ui.train_new")}</button>
    <button onclick={() => trains.copy(tab.id)}>{t("ui.train_copy")}</button>
    <button class="danger" onclick={() => (confirmingDelete = true)}>{t("ui.train_delete")}</button>
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
  <p class="error">{t("ui.train_import_failed", { reason: trains.importError })}</p>
{/if}
{#if exportError}
  <p class="error">{t("ui.train_export_failed", { reason: exportError })}</p>
{/if}

{#if confirmingDelete}
  <div class="confirm" role="alertdialog">
    <span>{t("ui.train_delete_question", { name: tab.name || t("ui.train_unnamed") })}</span>
    <button
      class="danger"
      onclick={() => {
        trains.remove(tab.id);
        confirmingDelete = false;
      }}>{t("ui.train_delete_confirm")}</button
    >
    <button onclick={() => (confirmingDelete = false)}>{t("ui.train_cancel")}</button>
  </div>
{/if}

<section class="train">
  <div class="grid shared">
    <label>
      <span>{t("ui.train_input_speed_peak")}</span>
      <input type="number" step="100" bind:value={tab.train.input_speed} />
      <em>{t("ui.train_rpm")}</em>
    </label>
    <label>
      <span>{t("ui.train_input_torque_peak")}</span>
      <input type="number" step="0.01" bind:value={tab.train.input_torque} />
      <em>{t("ui.train_nm")}</em>
    </label>
    <!-- A load applied at the *output*, trying to turn the train the other way.
         It is not a sign on the input torque: it enters at the far end and is
         attenuated by each stage's backward efficiency on the way up, and on a
         train that can be back-driven it is reacted by nothing and reaches no
         gear at all. The train says which of those happened. -->
    <label>
      <span>{t("ui.train_back_driving_torque_peak")}</span>
      <input type="number" step="0.01" bind:value={tab.train.back_driving_torque} />
      <em>{t("ui.train_nm")}</em>
    </label>

    <div class="mode">
      <span>{t("ui.train_actuation")}</span>
      <div class="segmented">
        <button class:on={mode === "intermittent"} onclick={() => setMode("intermittent")}>
          {t("ui.train_intermittent")}
        </button>
        <button class:on={mode === "continuous"} onclick={() => setMode("continuous")}>
          {t("ui.train_continuous")}
        </button>
      </div>
    </div>

    <!-- First under the actuation, and in both modes: the load the train's
         fatigue life is spent against, as opposed to the peak it must merely
         survive. Absolute rather than a percentage of peak — this tool declines
         to assert a relation between torque and speed on the user's behalf — so
         the percentage is reported beside it instead of driving it. Zero is a
         legitimate entry: a train that only ever sees its peak has no cyclic
         case. -->
    <label>
      <span>{t("ui.train_operating_torque")}</span>
      <input
        type="number"
        step="0.01"
        max={tab.train.input_torque}
        bind:value={tab.train.operating_torque}
      />
      <em>{t("ui.train_nm")}</em>
      <FieldNote notes={notes(
        "ok" in result && result.ok.operating_torque_percent !== null
          ? t("ui.train_note_operating_torque_percent", {
              percent: result.ok.operating_torque_percent.toFixed(1),
            })
          : null,
        null,
      )} />
    </label>

    {#if "intermittent" in tab.train.actuation}
      <label>
        <span>{t("ui.train_actuation_range")}</span>
        <input
          type="number"
          step="1"
          bind:value={tab.train.actuation.intermittent.range_degrees}
        />
        <em>{t("ui.train_at_output")}</em>
      </label>
      <label>
        <span>{t("ui.train_actuation_count")}</span>
        <input type="number" step="100" bind:value={tab.train.actuation.intermittent.actuations} />
        <em></em>
      </label>
      <!-- Offered only here, because it only means something here: a continuous
           drive has no actuation to reverse between. It changes nothing but the
           cycle count, and the note says how. -->
      {@const act = tab.train.actuation.intermittent}
      {@render switchField(
        "ui.train_reversing",
        act.reversing,
        (v) => (act.reversing = v),
        act.reversing ? t("ui.train_note_reversing") : null,
      )}
    {:else if "continuous" in tab.train.actuation}
      <label>
        <span>{t("ui.train_operating_speed")}</span>
        <input
          type="number"
          step="100"
          max={tab.train.input_speed}
          bind:value={tab.train.actuation.continuous.operating_speed}
        />
        <em>{t("ui.train_rpm")}</em>
      </label>
      <label>
        <span>{t("ui.train_runtime")}</span>
        <input type="number" step="100" bind:value={tab.train.actuation.continuous.runtime_hours} />
        <em>{t("ui.train_hours")}</em>
      </label>
    {/if}

    <!-- Last, and train-wide, because it is one decision about how every gear
         is judged rather than a property of any stage. A planet's root is
         loaded on both flanks whatever the drive does, and a reversing drive
         loads every root both ways — but the allowance for it is a fraction on
         an allowable a part is sized against, which this tool asks for rather
         than applies. Off, the stages say where reversal is present and
         uncorrected. -->
    {@render switchField(
      "ui.train_reversed_bending",
      tab.train.reversed_bending,
      (v) => (tab.train.reversed_bending = v),
      tab.train.reversed_bending
        ? t("ui.train_note_reversed_bending_on", {
            coefficient: defaults().reverse_loading_coefficient.toFixed(2),
          })
        : null,
    )}
  </div>

  <div class="summary">
    {#if "error" in result}
      <p class="error">{result.error}</p>
    {:else}
      <dl class="out">
        <dt>{t("ui.train_total_ratio")}</dt>
        <dd>
        {result.ok.total_ratio >= 1
          ? `${result.ok.total_ratio.toFixed(4)} : 1`
          : `1 : ${(1 / result.ok.total_ratio).toFixed(4)}`}
        </dd>
        <dt>{t("ui.train_output_speed_peak")}</dt>
        <dd>{result.ok.output_speed.toFixed(1)} {t("ui.train_rpm")}</dd>
        <dt>{t("ui.train_output_torque_peak")}</dt>
        <dd>{result.ok.output_torque.toFixed(4)} Nm</dd>
        <dt>{t("ui.train_total_efficiency")}</dt>
        <dd>
        {bothWays(result.ok.total_efficiency)}
        {#if result.ok.total_efficiency.backward <= 0}
          <small class="warn">{t("ui.train_cannot_be_back_driven")}</small>
        {/if}
        </dd>
        <dt>{t("ui.train_backlash_at_output_shaft")}</dt>
        <dd>
        {result.ok.backlash.forward.nominal.toFixed(5)}°
        <small
          >({range(result.ok.backlash.forward.minimum.toFixed(5), result.ok.backlash.forward.maximum.toFixed(5))})</small
        >
        </dd>
        <dt>{t("ui.train_backlash_at_input_shaft")}</dt>
        <dd>
          {result.ok.backlash.backward.nominal.toFixed(5)}°
          <small
            >({range(result.ok.backlash.backward.minimum.toFixed(5), result.ok.backlash.backward.maximum.toFixed(5))})</small
          >
        </dd>
      </dl>
      <!-- What the shaft line wants read, which no single stage is in a
           position to say: an input clamped against its peak, and where — or
           whether — the back-driving load is reacted. -->
      {#if result.ok.notes.length}
        <ul class="notes">
          {#each result.ok.notes as n, i (i)}<li>{note(n)}</li>{/each}
        </ul>
      {/if}
      {/if}
  </div>
</section>

<div class="stages">
  {#each tab.train.stages as stage, i (i)}
    {@const res = "ok" in result ? result.ok.stages[i] : null}
    <section class="stage">
      {#if stage.kind === "spur"}
        <!-- One stage, two meshes. Crossing the shafts turns a line contact
             into a point one and changes what sliding costs, so the answer has
             a different shape: `sres` when the shafts are parallel, the screw
             result when they are not (docs/reference.md#crossed-axes). The *inputs* below are
             the same either way, which is what the specification asks for. -->
        {@const sres = res && res.kind === "spur" ? res : null}
        {@const xres = res && res.kind === "worm" ? res : null}
        <button class="head" onclick={() => (tab.open[i] = !tab.open[i])}>
          <span class="caret">{tab.open[i] ? "▾" : "▸"}</span>
          <strong>{stageName(i)}</strong>
          {#if stage.shaft_angle !== 0}
            <span class="kind">{t("ui.train_crossed")}</span>
          {/if}
          <span class="teeth">z {stage.gears[0].teeth} / {stage.gears[1].teeth}</span>
          {#if sres ?? xres}
            <span class="ratio">{(sres ?? xres)?.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct((sres ?? xres)?.efficiency.forward ?? 0)} %</span>
          {/if}
        </button>

        {#if tab.open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>{t("ui.train_normal_module")}</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_pressure_angle")}</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_axis_angle")}</span>
                <input type="number" step="5" bind:value={stage.shaft_angle} />
                <em>°</em>
                <FieldNote notes={
                  notes(
                    stage.shaft_angle === 0
                      ? t("ui.train_note_shafts_parallel")
                      : t("ui.train_note_shafts_crossed"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_additional_helix_angle")}</span>
                <input type="number" step="1" bind:value={stage.additional_helix} />
                <em>°</em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_helix_split", {
                      first: n(stage.shaft_angle / 2 + stage.additional_helix),
                      second: n(stage.shaft_angle / 2 - stage.additional_helix),
                    }),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_sliding_friction")}</span>
                <input type="number" step="0.01" bind:value={stage.sliding_friction} />
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_static_friction")}</span>
                <input type="number" step="0.01" bind:value={stage.static_friction} />
                <em></em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_static_friction"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_tooth_thickness_mod")}</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>{t("ui.train_k")}</em>
                <!-- One input where the specification had a pair, because the
                     two are not independent: `k₁ + k₂ = 2` is what keeps the
                     mesh at zero backlash (docs/rationale.md#inputs-are-the-only-state), so storing both would
                     be storing a constraint that can be broken. Which gear it
                     applies to therefore has to be said. -->
                <FieldNote notes={
                  notes(
                    t(
                      stage.shaft_angle === 0
                        ? "ui.train_note_thickness_mod_spur"
                        : "ui.train_note_thickness_mod_crossed",
                      { first: String(gearNumber(i, 0)), second: String(gearNumber(i, 1)) },
                    ),
                    null,
                  )
                } />
              </label>
              {@render autoNumber(
                "ui.train_c2c_distance",
                stage.centre_distance,
                (sres ?? xres)?.centre_distance,
                0.1,
                () => relieveSpur(stage, stage.centre_distance),
                undefined,
                "ui.train_mm",
              )}
              <!-- **Greyed by the answer, not by a rule kept here.** Whether
                   this is read depends on whether anything is free to absorb it
                   — the distance when it is automatic, the shifts when they are
                   being chosen — and that is decided once, in the solve, which
                   reports what it took (`SpurStage::clearance_taken`). Reading
                   it back is what keeps the panel from having to know the same
                   rule a second time and drift from it. -->
              <label>
                <span>{t("ui.train_c2c_clearance")}</span>
                {#if (sres ?? xres) && (sres ?? xres)?.clearance === 0}
                  <input type="number" value={0} disabled class="computed" />
                {:else}
                  <input type="number" step="0.01" bind:value={stage.clearance} />
                {/if}
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_plus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_minus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              {#if stage.shaft_angle === 0}
                <label>
                  <span>{t("ui.train_load_sharing")}</span>
                  <!-- Off by default and deliberately so: the ramp behind it is
                       an uncalibrated placeholder rather than a stiffness model.
                       Offered rather than hidden, because an estimate a designer
                       chooses is a feature and one applied on their behalf is
                       not — and offered only here, since it reaches bending
                       alone and a crossed stage reports none. -->
                  <select bind:value={stage.load_sharing}>
                    <option value="none">{t("ui.train_load_sharing_none")}</option>
                    <option value="linear_ramp">
                      {t("ui.train_load_sharing_linear_ramp")}
                    </option>
                  </select>
                  <em></em>
                  <FieldNote notes={notes(t("ui.train_note_load_sharing"), null)} />
                </label>
              {/if}
              {@render efficiencyToggle(stage.optimisation)}
            </div>
            <!-- Clearance is meaningless once the centre distance is set by hand:
                 the specification locks it to zero, and so does the solver. -->

            <!-- A crossed pair rates as one mesh, not two teeth: there is no
                 bending model and the point contact's pressure belongs to the
                 pair. What is left that is per-member is what each shaft sees. -->
            {#snippet crossedMember(j: number)}
              {#if xres}
                <dl class="out small">
                  <dt>{t("ui.train_pitch_diameter")}</dt>
                  <dd>{xres.members[j].pitch_diameter.toFixed(4)} mm</dd>
                  <dt>{t("ui.train_helix_angle")}</dt>
                  <dd>{n(j === 0 ? xres.helix_angle : xres.wheel_helix_angle)}°</dd>
                  <dt>{t("ui.train_torque")}</dt>
                  <dd>{xres.members[j].torque.toFixed(4)} N·m</dd>
                  {#if xres.members[j].back_driving_torque !== null}
                    <dt>{t("ui.train_back_driving_torque")}</dt>
                    <dd>{xres.members[j].back_driving_torque.toFixed(4)} N·m</dd>
                  {/if}
                  <dt>{t("ui.train_speed")}</dt>
                  <dd>{xres.members[j].speed.toFixed(1)} {t("ui.train_rpm")}</dd>
                  <dt>{t("ui.train_tooth_cycles")}</dt>
                  <dd>
                    {xres.members[j].tooth_cycles.bending.toLocaleString()} / {xres.members[
                      j
                    ].tooth_cycles.contact.toLocaleString()}
                    <small>{t("ui.train_bending_contact")}</small>
                  </dd>
                </dl>
              {/if}
            {/snippet}

            <!-- A crossed pair's members are ordinary helical gears, so their
                 tooth form is specified here as it is anywhere else — it is what
                 will be cut. What it does *not* do is move this stage's figures,
                 and saying which is which is the honesty required: the mesh is
                 solved at its pitch point, so a shift reaches the answer only
                 through the centre distance, which is an input of its own.
                 docs/reference.md#crossed-axes. -->
            {#if stage.shaft_angle !== 0}
              <p class="aside wide">{t("ui.train_tooth_form_below_shift_addendum_dedendum")}</p>
            {/if}

            <div class="gears">
              {#each stage.gears as gear, j (j)}
                {@const g = sres?.gears[j]}
                {@render gearCard(gearName(i, j), gear, g, {
                  cut: "rack",
                  onShiftAuto: () => relieveSpur(stage, gear.profile_shift),
                  faceAuto: stage.shaft_angle === 0,
                  faceFromContinuity: xres?.crossed?.face_width_for_continuity?.[j],
                  extra: xres ? crossedMember : undefined,
                  extraIndex: j,
                })}
              {/each}
            </div>

            {#if xres}
              {@render screwReadout(xres, [gearName(i, 0), gearName(i, 1)])}
              {#if xres.notes.length}
                <ul class="notes">
                  {#each xres.notes as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}
            {/if}

            {#if sres}
              <dl class="out">
                <dt>{t("ui.train_centre_distance")}</dt>
                <dd>
                  {sres.centre_distance.toFixed(4)} mm
                  <small>{t("ui.train_nominal_value", { value: sres.centre_distance_nominal.toFixed(4) })}</small>
                </dd>
                <dt>{t("ui.train_contact_ratio")}</dt>
                <dd>
                  ε<sub>α</sub> {sres.contact_ratios.transverse.toFixed(4)} · ε<sub>β</sub>
                  {sres.contact_ratios.overlap.toFixed(4)} · ε<sub>γ</sub>
                  {sres.contact_ratios.total.toFixed(4)}
                  {#if stage.additional_helix !== 0 && sres.contact_ratios.overlap < 1}
                    <small class="warn">{t("ui.train_no_full_axial_overlap")}</small>
                  {/if}
                </dd>
                <!-- The one figure both members share: same patch, same normal
                     force, same E*, one instant. Each gear's own rating is on
                     its card and is this or worse. -->
                <dt>{t("ui.train_contact_stress_at_pitch_point")}</dt>
                <dd>
                  {cases(sres.contact_stress_at_pitch_point, 1)} {t("ui.train_mpa")}
                  <small>{t("ui.train_peak_cyclic")}</small>
                  <small>ρ {sres.relative_radius.toFixed(3)} mm</small>
                </dd>
                <dt>{t("ui.train_mesh_efficiency")}</dt>
                <dd>
                  {bothWays(sres.efficiency)}
                </dd>
                <dt>{t("ui.train_backlash")}</dt>
                <dd>
                  {t("ui.train_backlash_at", {
                    angle: sres.backlash.forward.nominal.toFixed(5),
                    member: gearName(i, 1),
                  })}
                  <small
                    >({range(sres.backlash.forward.minimum.toFixed(5), sres.backlash.forward.maximum.toFixed(5))})</small
                  >
                  · {t("ui.train_backlash_at", {
                    angle: sres.backlash.backward.nominal.toFixed(5),
                    member: gearName(i, 0),
                  })}
                </dd>
                <dt>{t("ui.train_coprime")}</dt>
                <dd>{sres.coprime ? t("ui.train_yes") : t("ui.train_no")}</dd>
              </dl>
              {#if sres.notes.length}
                <ul class="notes">
                  {#each sres.notes as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>{t("ui.train_remove_stage")}</button
            >
          </div>
        {/if}
      {:else if stage.kind === "worm"}
        {@const wres = res && res.kind === "worm" ? res : null}
        <button class="head" onclick={() => (tab.open[i] = !tab.open[i])}>
          <span class="caret">{tab.open[i] ? "▾" : "▸"}</span>
          <strong>{stageName(i)}</strong>
          <span class="kind">{t("ui.train_worm")}</span>
          <span class="teeth">z {stage.starts} / {stage.wheel_teeth}</span>
          {#if wres}
            <span class="ratio">{wres.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct(wres.efficiency.forward)} %</span>
          {/if}
        </button>

        {#if tab.open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>{t("ui.train_normal_module")}</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_pressure_angle")}</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_axis_angle")}</span>
                <input type="number" step="1" bind:value={stage.shaft_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_sliding_friction")}</span>
                <input type="number" step="0.01" bind:value={stage.sliding_friction} />
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_static_friction")}</span>
                <input type="number" step="0.01" bind:value={stage.static_friction} />
                <em></em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_static_friction"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_tooth_thickness_mod")}</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>{t("ui.train_k")}</em>
                <!-- The same single input the spur stage has, and for the same
                     reason: `k₁ + k₂ = 2` is what keeps the mesh at zero
                     backlash, so storing both would be storing a breakable
                     constraint. What it reaches differs, and the note says so —
                     the pair's play is unchanged *because* of that invariant,
                     which is an answer rather than a gap. -->
                <FieldNote notes={
                  notes(
                    t("ui.train_note_thickness_mod_worm"),
                    null,
                  )
                } />
              </label>
              {@render autoNumber(
                "ui.train_c2c_distance",
                stage.centre_distance,
                wres?.centre_distance,
                0.1,
                undefined,
                undefined,
                "ui.train_mm",
              )}
              <label>
                <span>{t("ui.train_c2c_clearance")}</span>
                {#if wres && wres.clearance === 0}
                  <input type="number" value={0} disabled class="computed" />
                {:else}
                  <input type="number" step="0.01" bind:value={stage.clearance} />
                {/if}
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_plus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_minus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_worm_axial_clearance")}</span>
                <input type="number" step="0.01" bind:value={stage.axial_clearance} />
                <em>{t("ui.train_mm")}</em>
              </label>
            </div>

            <div class="gears">
              <div class="gear">
                <h4>{t("pitch_diameter" in stage.sizing ? "ui.train_worm_member" : "ui.train_first_gear")}</h4>
                <label>
                  <span>{t("pitch_diameter" in stage.sizing ? "ui.train_starts" : "ui.train_tooth_count")}</span>
                  <input type="number" step="1" bind:value={stage.starts} />
                </label>
                <label>
                  <span>{t("ui.train_sized_by")}</span>
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
                    <option value="diameter">{t("ui.train_pitch_diameter_worm")}</option>
                    <option value="helix">{t("ui.train_helix_angle_gear")}</option>
                  </select>
                </label>
                {#if "pitch_diameter" in stage.sizing}
                  <label>
                    <span>{t("ui.train_pitch_diameter")}</span>
                    <input type="number" step="0.5" bind:value={stage.sizing.pitch_diameter} />
                    <em>{t("ui.train_mm")}</em>
                  </label>
                {:else}
                  <label>
                    <span>{t("ui.train_helix_angle")}</span>
                    <input type="number" step="1" bind:value={stage.sizing.helix_angle} />
                    <em>°</em>
                    <small>{t("ui.train_mate_takes_rest_shaft_angle")}</small>
                  </label>
                {/if}
                {#if wres && wres.members[0].recommended_face_width == null}
                  <!-- No recommendation exists for a crossed pair, so there is
                       nothing for an automatic toggle to take: showing one
                       would lock the field to a value nothing computed. -->
                  <label>
                    <span>{t("ui.train_length")}</span>
                    <input type="number" step="1" bind:value={stage.worm.face_width.manual} />
                    <em>{t("ui.train_mm")}</em>
                  </label>
                {:else}
                  {@render autoNumber(
                    "ui.train_length",
                    stage.worm.face_width,
                    wres?.members[0].recommended_face_width ?? undefined,
                    1,
                  )}
                {/if}
                {#if wres?.members[0].recommended_face_width != null}
                  <p class="convention">
                    {t("ui.train_note_proportions", {
                      width: wres.members[0].recommended_face_width?.toFixed(2) ?? "",
                      formula: "(11 + 0.06 z₂) m_x",
                    })}
                  </p>
                {/if}
                <label>
                  <span>{t("ui.train_material")}</span>
                  <select bind:value={stage.worm.material}>
                    {#each library.materials.material as m (m.name)}
                      <option value={m.name}>{m.name}</option>
                    {/each}
                  </select>
                </label>
                {#if wres}
                  <dl class="out">
                    <dt>{t("ui.train_lead_angle")}</dt>
                    <dd>{wres.lead_angle.toFixed(4)}°</dd>
                    <dt>{t("ui.train_lead")}</dt>
                    <dd>{wres.lead.toFixed(4)} mm</dd>
                    <dt>{t("ui.train_torque")}</dt>
                    <dd>{wres.members[0].torque.toFixed(4)} N·m</dd>
                    <dt>{t("ui.train_speed")}</dt>
                    <dd>{wres.members[0].speed.toFixed(1)} {t("ui.train_rpm")}</dd>
                  </dl>
                {/if}
              </div>

              <div class="gear">
                <h4>{t("ui.train_wormwheel")}</h4>
                <label>
                  <span>{t("ui.train_tooth_count")}</span>
                  <input type="number" step="1" bind:value={stage.wheel_teeth} />
                </label>
                {#if wres && wres.members[1].recommended_face_width == null}
                  <label>
                    <span>{t("ui.train_face_width")}</span>
                    <input type="number" step="1" bind:value={stage.wheel.face_width.manual} />
                    <em>{t("ui.train_mm")}</em>
                  </label>
                {:else}
                  {@render autoNumber(
                    "ui.train_face_width",
                    stage.wheel.face_width,
                    wres?.members[1].recommended_face_width ?? undefined,
                    1,
                  )}
                {/if}
                <label>
                  <span>{t("ui.train_material")}</span>
                  <select bind:value={stage.wheel.material}>
                    {#each library.materials.material as m (m.name)}
                      <option value={m.name}>{m.name}</option>
                    {/each}
                  </select>
                </label>
                {#if wres}
                  <dl class="out">
                    <dt>{t("ui.train_pitch_diameter")}</dt>
                    <dd>{wres.members[1].pitch_diameter.toFixed(4)} mm</dd>
                    <dt>{t("ui.train_torque")}</dt>
                    <dd>{wres.members[1].torque.toFixed(4)} N·m</dd>
                    <dt>{t("ui.train_speed")}</dt>
                    <dd>{wres.members[1].speed.toFixed(1)} {t("ui.train_rpm")}</dd>
                  </dl>
                {/if}
              </div>
            </div>

            {#if wres}
              {@render screwReadout(wres, [t("ui.train_the_worm"), t("ui.train_the_wheel")])}
              {#if wres.notes.length}
                <ul class="notes">
                  {#each wres.notes as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>{t("ui.train_remove_stage")}</button
            >
          </div>
        {/if}
      {:else if stage.kind === "planetary"}
        {@const pres = res && res.kind === "planetary" ? res : null}
        <button class="head" onclick={() => (tab.open[i] = !tab.open[i])}>
          <span class="caret">{tab.open[i] ? "▾" : "▸"}</span>
          <strong>{stageName(i)}</strong>
          <span class="kind">{t("ui.train_planetary")}</span>
          <span class="teeth">z {stage.sun.teeth} / {stage.planet.teeth} / {stage.ring.teeth}</span>
          {#if pres}
            <span class="ratio">{pres.ratio.toFixed(4)} : 1</span>
            <span class="eff">{pct(pres.efficiency.forward)} %</span>
          {/if}
        </button>
        {#if tab.open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>{t("ui.train_normal_module")}</span>
                <input type="number" step="0.1" bind:value={stage.module} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_pressure_angle")}</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_helix_angle")}</span>
                <input type="number" step="1" bind:value={stage.helix_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_sliding_friction_sun_planet")}</span>
                <input type="number" step="0.01" bind:value={stage.sliding_friction_sun_planet} />
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_static_friction_sun_planet")}</span>
                <input type="number" step="0.01" bind:value={stage.static_friction_sun_planet} />
                <em></em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_static_friction"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_sliding_friction_planet_ring")}</span>
                <input type="number" step="0.01" bind:value={stage.sliding_friction_planet_ring} />
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_static_friction_planet_ring")}</span>
                <input type="number" step="0.01" bind:value={stage.static_friction_planet_ring} />
                <em></em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_static_friction"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_tooth_thickness_mod")}</span>
                <input type="number" step="0.05" bind:value={stage.thickness_mod} />
                <em>{t("ui.train_k")}</em>
                <FieldNote notes={
                  notes(
                    t("ui.train_note_thickness_mod_planetary"),
                    null,
                  )
                } />
              </label>
              <label>
                <span>{t("ui.train_c2c_clearance")}</span>
                <input type="number" step="0.01" bind:value={stage.clearance} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_plus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_minus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_minimum_planet_clearance")}</span>
                <input type="number" step="0.05" bind:value={stage.min_planet_clearance} />
                <em>{t("ui.train_mm")}</em>
                <FieldNote notes={notes(t("ui.train_note_planet_clearance"), null)} />
              </label>
              <label>
                <span>{t("ui.train_planets")}</span>
                <input type="number" step="1" min="1" bind:value={stage.planets} />
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_driven_by")}</span>
                <select bind:value={stage.arrangement.input}>
                  <option value="sun">{t("ui.train_sun")}</option>
                  <option value="carrier">{t("ui.train_carrier")}</option>
                  <option value="ring">{t("ui.train_ring")}</option>
                </select>
                <em></em>
              </label>
              <label>
                <span>{t("ui.train_held")}</span>
                <select bind:value={stage.arrangement.fixed}>
                  <option value="sun">{t("ui.train_sun")}</option>
                  <option value="carrier">{t("ui.train_carrier")}</option>
                  <option value="ring">{t("ui.train_ring")}</option>
                </select>
                <em></em>
                <FieldNote notes={notes(null, null)} />
              </label>
              {@render efficiencyToggle(stage.optimisation)}
            </div>

            <h4 class="mesh">{t("ui.train_ring_cutter")}</h4>
            <div class="grid shared">
              <label>
                <span>{t("ui.train_cutter_teeth")}</span>
                <input type="number" step="1" min="1" bind:value={stage.cutter.teeth} />
                <em></em>
                <FieldNote notes={notes(null, null)} />
              </label>
              <label>
                <span>{t("ui.train_cutter_addendum")}</span>
                <input type="number" step="0.05" bind:value={stage.cutter.addendum} />
                <em>{t("ui.train_m")}</em>
              </label>
              <label>
                <span>{t("ui.train_cutter_tip_round")}</span>
                <input type="number" step="0.02" bind:value={stage.cutter.tip_round} />
                <em>{t("ui.train_m")}</em>
              </label>
            </div>

            <!-- The planet's reversal is a *stage* note now, not a readout here:
                 it is the same sentence a reversing drive earns for every gear,
                 and one home for it means the two cannot say different things.
                 What stays is what is only true of a planet — the speed its
                 teeth actually see, relative to the carrier. -->
            {#snippet planetExtra(_j: number)}
              {#if pres}
                <dl class="out small">
                  <dt>{t("ui.train_speed")}</dt>
                  <dd>
                    {pres.planet.speed_absolute.toFixed(1)} {t("ui.train_rpm")}
                    <small>
                      {t("ui.train_relative_to_the_carrier", {
                        speed: pres.planet.speed_relative.toFixed(1),
                      })}
                    </small>
                  </dd>
                </dl>
              {/if}
            {/snippet}

            <div class="gears">
              {@render gearCard(t("ui.train_sun"), stage.sun, pres?.sun, { cut: "rack" })}
              {@render gearCard(t("ui.train_planet"), stage.planet, pres?.planet.gear, {
                cut: "rack",
                solvedShift: pres?.planet.profile_shift,
                extra: planetExtra,
              })}
              <!-- A ring's root and fillet are its cutter's, so it has neither a
                   dedendum nor a root radius of its own (docs/reference.md#internal-gears); the tool
                   is a stage input, above. -->
              {@render gearCard(t("ui.train_ring"), stage.ring, pres?.ring, { cut: "shaper" })}
            </div>

            {#if pres}
              <dl class="out">
                <dt>{t("ui.train_ratio")}</dt>
                <dd>
                  {pres.ratio.toFixed(4)} : 1
                  <small>
                    {t("ui.train_in_held_out", {
                      input: shaft(pres.arrangement.input),
                      held: shaft(pres.arrangement.fixed),
                      output: shaft(pres.output),
                    })}
                  </small>
                </dd>
                <dt>{t("ui.train_centre_distance")}</dt>
                <dd>
                  {pres.centre_distance.toFixed(4)} mm
                  <small>
                    {t("ui.train_common_to_both_meshes", {
                      residual: pres.planet.shift_residual.toExponential(1),
                    })}
                  </small>
                </dd>
                <dt>{t("ui.train_efficiency")}</dt>
                <dd>
                  {bothWays(pres.efficiency)}
                  <small>
                    {t("ui.train_fixed_carrier_efficiency", {
                      percent: pct(pres.fixed_carrier_efficiency.forward),
                    })}
                  </small>
                </dd>
                <dt>{t("ui.train_backlash")}</dt>
                <dd>
                  {t("ui.train_at_the_shaft", {
                    angle: pres.backlash.forward.nominal.toFixed(5),
                    shaft: shaft(pres.output),
                  })}
                  <small
                    >({range(pres.backlash.forward.minimum.toFixed(5), pres.backlash.forward.maximum.toFixed(5))})</small
                  >
                </dd>
                <dt>{t("ui.train_planet_clearance")}</dt>
                <dd>
                  {pres.planet_clearance === null
                    ? t("ui.train_one_planet_no_neighbour")
                    : `${pres.planet_clearance.toFixed(3)} mm`}
                  {#if pres.planet_clearance !== null}
                    <small class:warn={!pres.planet_clearance_ok}>
                      {t(pres.planet_clearance_ok ? "ui.train_meets_the_minimum" : "ui.train_below_the_minimum")}
                    </small>
                  {/if}
                </dd>
                <!-- Two separate layout checks, so two rows. Even spacing is
                     `N | z_sun + z_ring`; simultaneous meshing is the stricter
                     `N | z_sun` *and* `N | z_ring`, and a false answer is not a
                     fault — it means the planets engage staggered, which is
                     usually preferable. -->
                <dt>{t("ui.train_even_spacing")}</dt>
                <dd>{pres.equal_spacing ? t("ui.train_yes") : t("ui.train_no")}</dd>
                <dt>{t("ui.train_simultaneous_meshing")}</dt>
                <dd>{pres.simultaneous_meshing ? t("ui.train_yes") : t("ui.train_no")}</dd>
              </dl>

              <!-- The two meshes, each stacked like a spur stage's readout
                   rather than a table that lines up with nothing else on the
                   panel. The coprime check belongs to a mesh — the sun against
                   the planets, the ring against the planets — so it leads each
                   list. -->
              {#each [
                [t("ui.train_mesh_sun_planet"), pres.sun_planet, pres.sun_coprime_with_planets],
                [t("ui.train_mesh_planet_ring"), pres.planet_ring, pres.ring_coprime_with_planets],
              ] as const as [label, m, coprime] (label)}
                <h4 class="mesh">{label}</h4>
                <dl class="out indent">
                  <dt>{t("ui.train_coprime")}</dt>
                  <dd>{coprime ? t("ui.train_yes") : t("ui.train_no")}</dd>
                  <dt>{t("ui.train_contact_ratio")}</dt>
                  <dd>
                    ε<sub>α</sub> {m.contact_ratios.transverse.toFixed(4)} · ε<sub>β</sub>
                    {m.contact_ratios.overlap.toFixed(4)} · ε<sub>γ</sub>
                    {m.contact_ratios.total.toFixed(4)}
                  </dd>
                  <dt>{t("ui.train_mesh_efficiency")}</dt>
                  <dd>
                    {bothWays(m.efficiency)}
                  </dd>
                  <dt>{t("ui.train_contact_stress_at_pitch_point")}</dt>
                  <dd>
                    {cases(m.contact_stress_at_pitch_point, 1)} {t("ui.train_mpa")}
                    <small>{t("ui.train_peak_cyclic")}</small>
                    <small>ρ {m.relative_radius.toFixed(3)} mm</small>
                  </dd>
                </dl>
              {/each}

              {#if pres.notes.length}
                <ul class="notes">
                  {#each pres.notes as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}
            {/if}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>{t("ui.train_remove_stage")}</button
            >
          </div>
        {/if}

      {:else if stage.kind === "hula"}
        {@const hres = res && res.kind === "hula" ? res : null}
        <button class="head" onclick={() => (tab.open[i] = !tab.open[i])}>
          <span class="caret">{tab.open[i] ? "▾" : "▸"}</span>
          <strong>{stageName(i)}</strong>
          <span class="kind">{t("ui.train_hula")}</span>
          <span class="teeth">z {stage.gears.map((g) => g.teeth).join(" / ")}</span>
          {#if hres}
            <span class="ratio">{hres.ratio.toFixed(2)} : 1</span>
            <span class="eff">{pct(hres.efficiency.forward)} %</span>
          {/if}
        </button>
        {#if tab.open[i]}
          <div class="body">
            <div class="grid shared">
              <label>
                <span>{t("ui.train_pressure_angle")}</span>
                <input type="number" step="0.5" bind:value={stage.pressure_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_helix_angle")}</span>
                <input type="number" step="1" bind:value={stage.helix_angle} />
                <em>°</em>
              </label>
              <label>
                <span>{t("ui.train_hula_gap")}</span>
                {#if hres && hres.clearance === 0}
                  <input type="number" value={0} disabled class="computed" />
                {:else}
                  <input type="number" step="0.05" bind:value={stage.clearance} />
                {/if}
                <em>{t("ui.train_mm")}</em>
                <FieldNote notes={notes(t("ui.train_hula_note_gap"), null)} />
              </label>
              <!-- The crank offset is this arrangement's centre distance and is
                   entered as one: automatic derives it from the clearances the
                   parts must keep, a number given by hand is what it runs at.
                   It carried a mode select and a second box before, which said
                   the same thing in two controls neither of which looked like
                   the field it replaced. What it settled at was also reported
                   again below; the note it earned there belongs here, beside
                   the number a designer is reading. -->
              {@render autoNumber(
                "ui.train_hula_crank_offset",
                stage.offset,
                hres?.offset_nominal,
                0.01,
                undefined,
                hres
                  ? `${t("ui.train_hula_running", { value: hres.offset.toFixed(4) })}${
                      hres.binding_mesh !== null
                        ? ` · ${t("ui.train_hula_held_open_by", { mesh: String(hres.binding_mesh + 1) })}`
                        : ""
                    }`
                  : null,
                "ui.train_mm",
              )}
              <label>
                <span>{t("ui.train_c2c_clearance")}</span>
                <input type="number" step="0.01" bind:value={stage.running_clearance} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_plus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_plus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              <label>
                <span>{t("ui.train_c2c_tolerance_minus")}</span>
                <input type="number" step="0.01" bind:value={stage.tolerance_minus} />
                <em>{t("ui.train_mm")}</em>
              </label>
              {@render efficiencyToggle(stage.optimisation)}
            </div>

            {#if hres}
              <!-- Ordered as the spur and screw readouts are — the distance the
                   pair runs at, contact, efficiency, backlash — with what only
                   this arrangement has following on. -->
              <dl class="out">
                <dt>{t("ui.train_ratio")}</dt>
                <dd>
                  {hres.ratio.toFixed(4)} : 1
                  <small>{`${t("ui.train_hula_ratio_products", {
                        numerator: String(hres.ratio_products[0]),
                        denominator: String(hres.ratio_products[1]),
                      })} · ${t("ui.train_hula_note_ratio", {
                        denominator: String(hres.ratio_products[1]),
                      })}`}</small>
                </dd>
                <dt>{t("ui.train_efficiency")}</dt>
                <dd>
                  {pct(hres.efficiency.forward)} %
                  {#if hres.efficiency.backward === 0}
                    <small class="warn">{t("ui.train_self_locking")}</small>
                  {/if}
                  <small>{`${t("ui.train_hula_meshes_alone", {
                        percent: pct(hres.fixed_carrier_efficiency.forward),
                      })} · ${t("ui.train_hula_note_circulating")}`}</small>
                </dd>
                <dt>{t("ui.train_backlash_at_output_shaft")}</dt>
                <dd>
                  {t("ui.train_backlash_at", {
                    angle: hres.backlash.forward.nominal.toFixed(4),
                    member: t("ui.train_hula_role_output"),
                  })}
                </dd>
                <dt>{t("ui.train_hula_speeds")}</dt>
                <dd>
                  {t("ui.train_hula_speeds_at", {
                    crank: hres.crank_speed.toFixed(1),
                    wobble: hres.gears[1].speed.toFixed(3),
                    output: hres.gears[3].speed.toFixed(4),
                  })}
                </dd>
              </dl>
            {/if}

            {#each [0, 1] as m (m)}
              {@const ring = hres && hres.gears[m * 2].ring ? m * 2 : m * 2 + 1}
              {@const pinion = ring === m * 2 ? m * 2 + 1 : m * 2}
              <h4 class="mesh">{t("ui.train_hula_mesh", { mesh: String(m + 1) })}</h4>
              <div class="grid shared">
                <label>
                  <span>{t("ui.train_normal_module")}</span>
                  <input type="number" step="0.05" bind:value={stage.module[m]} />
                  <em>{t("ui.train_mm")}</em>
                </label>
                <label>
                  <span>{t("ui.train_tooth_thickness_mod")}</span>
                  <input type="number" step="0.05" bind:value={stage.thickness_mod[m]} />
                  <em>{t("ui.train_k")}</em>
                  <FieldNote notes={notes(t("ui.train_hula_note_thickness_mod"), null)} />
                </label>
                <label>
                  <span>{t("ui.train_cutter_teeth")}</span>
                  <input type="number" step="1" min="4" bind:value={stage.cutter[m].teeth} />
                  <em></em>
                  <FieldNote notes={notes(t("ui.train_hula_note_shaper"), null)} />
                </label>
                <label>
                  <span>{t("ui.train_sliding_friction")}</span>
                  <input type="number" step="0.01" bind:value={stage.sliding_friction[m]} />
                  <em></em>
                </label>
                <label>
                  <span>{t("ui.train_hula_given_shift")}</span>
                  <select bind:value={stage.given_shift[m]}>
                    <option value="pinion">{t("ui.train_hula_split_pinion")}</option>
                    <option value="ring">{t("ui.train_hula_split_ring")}</option>
                  </select>
                  <em></em>
                  <FieldNote notes={notes(t("ui.train_hula_note_split"), null)} />
                </label>
              </div>
              {#if hres}
                <dl class="out indent">
                  <dt>{t("ui.train_hula_operating_pressure_angle")}</dt>
                  <dd>
                    {hres.meshes[m].operating_pressure_angle.toFixed(3)}°
                    <small>{t("ui.train_hula_note_operating_pressure_angle")}</small>
                  </dd>
                  <dt>{t("ui.train_contact_ratio")}</dt>
                  <dd>
                    {hres.meshes[m].contact_ratio.toFixed(4)}
                    {#if hres.meshes[m].contact_ratio < 1}
                      <small class="warn">{t("ui.train_note_contact_ratio_below_one")}</small>
                    {/if}
                  </dd>
                  <dt>{t("ui.train_hula_gap_result")}</dt>
                  <dd>
                    {hres.meshes[m].clearance.toFixed(4)} {t("ui.train_mm")}
                    <small>
                      {t("ui.train_hula_gap_as_cut", {
                        value: hres.meshes[m].clearance_as_cut.toFixed(4),
                      })}
                    </small>
                  </dd>
                  <dt>{t("ui.train_hula_tip_margin")}</dt>
                  <dd>
                    {hres.meshes[m].tip_margin.toFixed(4)}°
                    <small>{t("ui.train_hula_note_tip_margin")}</small>
                  </dd>
                  <dt>{t("ui.train_hula_interference")}</dt>
                  <dd>
                    {[
                      hres.meshes[m].trochoid_interference
                        ? t("ui.train_hula_interference_trochoid")
                        : null,
                      hres.meshes[m].involute_interference
                        ? t("ui.train_hula_interference_involute")
                        : null,
                      hres.meshes[m].tip_interference
                        ? t("ui.train_hula_interference_tip")
                        : null,
                    ]
                      .filter((x) => x !== null)
                      .join(" · ") || t("ui.train_hula_interference_none")}
                  </dd>
                  <dt>{t("ui.train_backlash")}</dt>
                  <dd>
                    {hres.meshes[m].backlash.map((b) => `${b.nominal.toFixed(5)}°`).join(" / ")}
                  </dd>
                </dl>
              {/if}
              <div class="gears">
                {#each [ring, pinion] as j (j)}
                  {@render gearCard(
                    t(hres && hres.gears[j].ring ? "ui.train_ring" : "ui.train_pinion") +
                      " — " +
                      t(
                        ["ui.train_hula_role_grounded", "ui.train_hula_role_wobble", "ui.train_hula_role_wobble", "ui.train_hula_role_output"][j],
                      ),
                    stage.gears[j],
                    undefined,
                    {
                      cut: hres && hres.gears[j].ring ? "shaper" : "rack",
                      // A shift is an *input* only where it is the member this
                      // mesh names and is still being read: the other member's
                      // follows from the crank, and once the split is being
                      // chosen for efficiency the named one follows too — unless
                      // it was given, which pins it and leaves the mesh nothing
                      // to search.
                      solvedShift:
                        hres &&
                        ((stage.given_shift[m] === "ring") !== (hres.gears[j].ring === true) ||
                          (stage.optimisation.enabled && stage.gears[j].profile_shift.auto))
                          ? hres.gears[j].profile_shift
                          : undefined,
                      faceAuto: false,
                    },
                  )}
                {/each}
              </div>
              {#if hres}
                {@const clamped = [ring, pinion].flatMap((j) =>
                  hres.gears[j].clamps.map((c) => ({ teeth: hres.gears[j].teeth, note: c })),
                )}
                {#if clamped.length}
                  <!-- One list for the pair, as every other clamped gear in the
                       application reports: `.hint` is a note pulled *up* against
                       the field above it, so a run of them closed on each other
                       instead of reading as a list. -->
                  <ul class="notes">
                    {#each clamped as c, i (i)}
                      <li>z{c.teeth}: {note(c.note)}</li>
                    {/each}
                  </ul>
                {/if}
              {/if}
            {/each}

            <button
              class="danger small"
              onclick={() => removeStage(i)}
              disabled={tab.train.stages.length === 1}>{t("ui.train_remove_stage")}</button
            >
          </div>
        {/if}

      {/if}
    </section>
  {/each}

  <!-- One button a kind, from the table rather than by hand: a kind marked for
       the developer mode is not offered until the sidebar's title has been
       knocked on, which is the same gate the gear tab's eccentric kind is
       behind and the same table shape. -->
  {#each stageKinds as k (k.key)}
    <button class="add" onclick={() => addStageOfKind(k)}>{t(k.label)}</button>
  {/each}
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
  /* Inputs on the left, stacked; what they produce beside them, also stacked.
     The same shape as a gear card and its readout, without the border or the
     heading — this is the whole train, so there is nothing to tell it apart
     from. Falls back to one column when there is no room for two, which is the
     only place the two are allowed to sit above each other. */
  .train {
    border: 1px solid var(--rule);
    border-radius: 4px;
    padding: 0.75rem;
    margin-bottom: 1rem;
    display: grid;
    /* Two equal halves, so the results start at the middle of the box — the
       same split the gear cards below use, and for the same reason: a reader
       scanning down the page finds the same edge in both. Equal fractions
       rather than a fixed input width, so it reflows with the window like
       everything else. */
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: start;
    gap: 0.4rem 2rem;
  }
  @media (max-width: 52rem) {
    .train {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .train .summary {
    min-width: 0;
  }
  /* No top margin here: it is beside the inputs, not below them. */
  .train .summary .out {
    margin-top: 0;
  }
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
    gap: var(--field-gap) 1rem;
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
  /* The boxes sit further right than a gear card's, toward the middle of the
     stage. A stage's field names are the long ones — "Minimum planet
     clearance", "Static friction, planet–ring" — and with the narrow label
     column they wrapped while the box floated close enough to read as part of
     the name. Only the shared block: the gear cards below are half as wide and
     their own column is right for them. */
  .grid.shared > label {
    grid-template-columns: 1fr 9rem 3.5rem;
  }
  /* The `auto` toggle takes a column of its own, out of the label's share, so
     the number keeps the edge every other number in the panel shares. */
  label.auto {
    grid-template-columns: 1fr auto 6rem 3.5rem;
  }
  .grid.shared > label.auto {
    grid-template-columns: 1fr auto 9rem 3.5rem;
  }
  .gear label.auto {
    grid-template-columns: 1fr auto 6.5rem 3.5rem;
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
    /* See GearPanel: the column gap spaces a row, the row gap pairs a note to
       the box above it. */
    column-gap: var(--row-gap);
    row-gap: var(--note-gap);
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
  /* **A `select` is painted by the browser, not by the page, unless the page
     says otherwise.** With a transparent background the text is the token's and
     the box behind it is the platform's native control — which in a dark theme
     came out light, and put near-white text on it. The gear tab's selects were
     right all along because they name a background; these now do too. */
  select {
    background: var(--bg);
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
  /* The actuation control ends where the switches and the inputs do, for the
     same reason: its trailing cell is the unit cell every field keeps, left
     empty. It used to run to the row's edge, which put the one control in the
     column that lined up with nothing. */
  .mode {
    display: grid;
    grid-template-columns: 1fr auto 3.5rem;
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
  /* The source of a shipped convention, next to the number it produced —
     the project's rule is to say what a figure is where it is shown. */
  .convention {
    grid-column: 1 / -1;
    margin: -0.1rem 0 0.4rem;
    font-size: 0.72rem;
    line-height: 1.4;
    color: var(--muted);
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
  /* One mesh's readout, sitting under its heading. */
  h4.mesh {
    margin: 0.75rem 0 0;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }
  .out.indent {
    margin-top: 0.2rem;
    padding-left: 0.9rem;
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
  /* A readout with more than one figure to give — the four face widths a gear
     asks for — puts each on its own line rather than running them together. */
  .out dd .line {
    display: block;
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
  /* Four sources now, not two, so the row wraps rather than squeezing them. */
  /* A switch that carries its own name has nothing to put in a label column,
     so the row is the button alone, at the edge every **input box** in the
     column ends on — one unit cell in from the row's own edge, which is where
     the notes end too.
     That edge rather than the row's, because the row's is where the *unit* cell
     ends and a unit is short left-aligned text: a button flush to it lines up
     with the empty air past "mm" rather than with anything drawn.
     **Laid out as a column of its own rather than as a grid row**, which is not
     a style choice. As a grid it inherits whatever `grid-template-columns` the
     container sets, and those selectors carry two classes and an element to
     this one's single class — so it kept the three-column template underneath
     and `justify-items` put the button at the end of the *label* column,
     mid-row, lining up with nothing. Racing that needs a selector naming every
     container a switch might sit in, and the list goes stale the first time
     there is a fourth. A flex row cannot be reached by a column template at
     all. */
  .switchrow {
    display: flex;
    flex-direction: column;
  }
  /* The control stops where the input boxes stop; its note runs the full row,
     as every other note does. Taking the inset off the control rather than off
     the row is what lets the two differ. */
  .switchrow .control {
    display: flex;
    justify-content: flex-end;
    padding-right: var(--unit-inset);
  }
  .subtoggles {
    display: flex;
    flex-wrap: wrap;
    gap: 0.25rem 0.9rem;
    padding: 0 0 0.3rem 0.8rem;
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
    /* A note in its own element rather than inside the label, so it has to undo
       the field gap above it to sit as close as an in-label note does. It ends
       where an in-label note does, which is the row's own edge. */
    margin: calc(var(--note-gap) - var(--field-gap)) 0 var(--field-gap);
    font-size: 0.72rem;
    color: var(--muted);
    text-align: right;
  }
  /* See the note slot's own comment: candidates stack, the tallest sets the
     height, and nothing moves when the visible one changes. */
  .aside {
    grid-column: 1 / -1;
    margin: 0 0 var(--field-gap);
    font-size: 0.72rem;
    color: var(--muted);
  }
  .aside.wide {
    margin: 0.2rem 0 0.5rem;
    max-width: 60rem;
  }
  label.invalid input {
    border-color: var(--warn);
  }
  .gear label {
    grid-template-columns: 1fr 6.5rem 3.5rem;
    margin-bottom: var(--field-gap);
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
