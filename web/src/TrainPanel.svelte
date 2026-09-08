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
    type Stage,
    type PlanetaryStage,
    type Optimisation,
    type Value,
    type GearResult,
    type Note,
    type Cutter,
    type WormResult,
    type MeshReport,
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

  /** **An epicyclic set has two shifts to give, not three.** Its two centre
   *  distances have to agree, which is one relation among the three — so two
   *  are a design and the third is whatever they leave. Pinning all three
   *  over-specifies it, and the one furthest from what was just touched returns
   *  to automatic to absorb the relation again. */
  function relievePlanetary(stage: PlanetaryStage, just: Auto<number>) {
    relieve([stage.sun, stage.planet, stage.ring].map((g) => g.profile_shift), 2, just);
  }

  /** **A hula mesh has one shift to give, not two.** The crank offset fixes the
   *  difference of a pair's two shifts, so pinning both over-specifies the mesh
   *  — the same triangle the spur stage's distance and two shifts make, one
   *  freedom smaller. Pinning one returns the other to automatic, which is what
   *  the select this replaced used to say in words. */
  function relieveHula(stage: Extract<Stage, { kind: "hula" }>, mesh: number, just: Auto<number>) {
    const pair = [stage.gears[mesh * 2], stage.gears[mesh * 2 + 1]].map((g) => g.profile_shift);
    relieve(pair, 1, just);
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
  /** The answer, where there is one — `undefined` reads through every formatter
   *  below as a blank rather than as a row that is not there. */
  const solved = $derived(result.result ?? undefined);
  const failure = $derived(result.failure);

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

  /** **A figure a stage has only once it has solved.**
   *
   *  Blank while it has not, rather than absent: a readout that disappears
   *  takes its label with it, so the panel a designer is editing changes shape
   *  under them at the moment they most need it to hold still. Every formatter
   *  below takes the same `undefined` and answers the same way, so a readout is
   *  written once and reads either way. */
  const BLANK = "";
  const num = (v: number | null | undefined, digits: number) =>
    v == null ? BLANK : v.toFixed(digits);
  const count = (v: number | null | undefined) => (v == null ? BLANK : v.toLocaleString());
  const pct = (v: number | null | undefined) => (v == null ? BLANK : (100 * v).toFixed(3));
  const n = (v: number | null | undefined) => (v == null ? BLANK : v.toFixed(3));
  /** "lo to hi" — one shape, so the word between two numbers is written once. */
  /** "lo to hi" — one shape, so the word between two numbers is written once,
   *  and the brackets too: every reader of this wrapped it in a pair, which is
   *  a pair of brackets that would have been left stranded round a blank. */
  const range = (lo: string, hi: string) =>
    lo === BLANK ? BLANK : `(${t("ui.range", { lo, hi })})`;
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
  /** **A gear's own note, rendered under the input it is about.**
   *
   *  A hint that names a field and carries a number — a shift raised to clear
   *  undercut, an addendum held down to keep a tip — is one the reader wants
   *  beside that field, not in a list at the foot of the stage where it has to
   *  be matched back up by tooth count. The stage's list keeps what is about
   *  the stage. */
  const clampNote = (from: Note[], keys: readonly string[]) =>
    from.filter((n) => keys.includes(n.key)).map(note).join(" · ") || undefined;

  /** Which of a gear's notes belong to which of its fields. One table, so a
   *  stage that lists what is left over can subtract exactly what was drawn
   *  rather than repeating the keys. */
  const FIELD_NOTES = {
    profile_shift: ["stage.shift_raised_for_undercut"],
    addendum: ["stage.addendum_held_to_tip_width", "stage.addendum_above_tip_width"],
  } as const;
  const UNDER_A_FIELD: readonly string[] = Object.values(FIELD_NOTES).flat();

  /** A mesh efficiency, both ways round. Two keys rather than one sentence: the
   *  two halves are separated by a bullet the grammar of no language owns. */
  const bothWays = (e: { forward: number; backward: number } | undefined) =>
    e === undefined
      ? BLANK
      : `${t("ui.train_driven_forward", { percent: pct(e.forward) })} · ${t(
          "ui.train_driven_backward",
          { percent: pct(e.backward) },
        )}`;

  /** A rating in both load cases, written in the order they are read: what the
   *  part must survive once, then what it must survive for the duty.
   *
   *  Formatting, not arithmetic — every number here came from Rust. A rating
   *  that does not exist for a member renders as a dash rather than as a zero,
   *  because those are different facts. */
  const cases = (
    v: { peak: number | null; cyclic: number | null } | undefined,
    digits: number,
  ) =>
    v === undefined
      ? BLANK
      : `${v.peak === null ? "—" : v.peak.toFixed(digits)} / ${
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
<!-- **Not a `<label>`.** A label activates its first labelable descendant when
     any part of it is clicked, and `<button>` is labelable — so a switch row
     written as a label had a hit area running the width of the row, well
     outside the button a reader can see. There is nothing else in this row to
     focus, so there is nothing for a label to be for. -->
{#snippet switchField(key: string, on: boolean, set: (v: boolean) => void, note?: string | null)}
  <div class="switchrow">
    <span class="control"><Switch label={t(key)} {on} {set} /></span>
    {#if note !== undefined}
      <FieldNote notes={notes(note, null)} />
    {/if}
  </div>
{/snippet}


<!-- One gear card, used by every stage that has gears. A sun, a planet, a ring
     and a spur gear take the same inputs and produce the same readout, so they
     are one definition rather than several that drift apart — which is what had
     happened to the planetary section. What genuinely differs is passed in: a
     ring's root belongs to its cutter, a planet's shift is solved rather than
     chosen, and a member may have something of its own to report. -->
<!-- **What one parallel-axis mesh reports**, drawn once for every stage that has
     more than one of them.

     A `MeshReport` is the same six figures whether it is an epicyclic set's
     sun–planet pair or a hula drive's, and both were drawing them out
     separately — which is how the axial-overlap warning came to appear on a
     spur pair alone and the contact-ratio one on a hula alone, though either
     mesh can be the one that loses contact. Rows rather than a whole list, so a
     stage with something of its own to say puts it in the same `<dl>` before or
     after these; `members` names the two ends the one gap is seen from, in the
     order the mesh was built. -->
{#snippet meshRows(m: MeshReport | undefined, members: [string, string], helical: boolean)}
  <dt>{t("ui.train_coprime")}</dt>
  <dd>{m === undefined ? BLANK : m.coprime ? t("ui.train_yes") : t("ui.train_no")}</dd>
  <dt>{t("ui.train_operating_pressure_angle")}</dt>
  <dd>{num(m?.operating_pressure_angle, 3)}{m ? "°" : BLANK}</dd>
  <dt>{t("ui.train_contact_ratio")}</dt>
  <dd>
    {#if m}
      <span class:warn={m.contact_ratios.transverse < 1}>
        ε<sub>α</sub> {num(m.contact_ratios.transverse, 4)}
      </span>
      · ε<sub>β</sub>
      {num(m.contact_ratios.overlap, 4)} · ε<sub>γ</sub>
      {num(m.contact_ratios.total, 4)}
      {#if m.contact_ratios.transverse < 1}
        <small class="warn">{t("ui.train_note_contact_ratio_below_one")}</small>
      {:else if helical && m.contact_ratios.overlap < 1}
        <small class="warn">{t("ui.train_no_full_axial_overlap")}</small>
      {/if}
    {/if}
  </dd>
  <dt>{t("ui.train_mesh_efficiency")}</dt>
  <dd>{bothWays(m?.efficiency)}</dd>
  <!-- One gap, seen from each of its two ends, with the tolerance band on the
       first — the way every other mesh here writes its play. -->
  <dt>{t("ui.train_mesh_backlash")}</dt>
  <dd>
    {t("ui.train_backlash_at", {
      angle: num(m?.backlash[0].nominal, 5),
      member: members[0],
    })}
    <small>{range(num(m?.backlash[0].minimum, 5), num(m?.backlash[0].maximum, 5))}</small>
    · {t("ui.train_backlash_at", {
      angle: num(m?.backlash[1].nominal, 5),
      member: members[1],
    })}
  </dd>
  <!-- The one figure both members share: same patch, same normal force, same
       E*, one instant. Each member's own rating is on its card and is this or
       worse. -->
  <dt>{t("ui.train_contact_stress_at_pitch_point")}</dt>
  <dd>
    {cases(m?.contact_stress_at_pitch_point, 1)} {m && t("ui.train_mpa")}
    <small>{t("ui.train_peak_cyclic")}</small>
    <small>{m ? `ρ ${num(m.relative_radius, 3)} mm` : BLANK}</small>
  </dd>
{/snippet}

<!-- What a crossed-axis mesh reports, whether it was entered as a worm drive or
     as a gear pair with its shafts turned: the same mathematics answers both
     (docs/reference.md#crossed-axes), so it is one readout rather than two that drift. -->
{#snippet screwReadout(r: WormResult | undefined, members: [string, string])}
<!-- Ordered to match the spur stage's shared readout — centre distance,
     contact ratio, efficiency, backlash — with what only a screw pair has
     following on. The backlash lives here rather than on the two member cards
     for the same reason it does there: it is one gap seen from two ends, not a
     property either member owns. -->
<dl class="out">
  <dt>{t("ui.train_centre_distance")}</dt>
  <dd>
    {num(r?.centre_distance, 4)} mm
    <small>{t("ui.train_nominal_value", { value: num(r?.centre_distance_nominal, 4) })}</small>
  </dd>
  {#if r?.crossed}
    <dt>{t("ui.train_contact_ratio")}</dt>
    <dd>
      <span class:warn={r.crossed.contact_ratio < 1}>
        ε {num(r?.crossed.contact_ratio, 4)}
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
    {bothWays(r?.efficiency)}
    {#if r && r.efficiency.backward <= 0}
      <small class="warn">{t("ui.train_self_locking")}</small>
    {/if}
    {#if r?.crossed?.parallel_axis_efficiency != null}
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
      angle: num(r?.backlash.forward.nominal, 5),
      member: members[1],
    })}
    <small
      >{range(num(r?.backlash.forward.minimum, 5), num(r?.backlash.forward.maximum, 5))}</small
    >
    · {t("ui.train_backlash_at", {
      angle: num(r?.backlash.backward.nominal, 5),
      member: members[0],
    })}
  </dd>
  <dt>{t("ui.train_self_locks_at")}</dt>
  <dd>{num(r?.self_locking_friction, 4)}</dd>
  <dt>{t("ui.train_contact_stress")}</dt>
  <dd>
    {cases(r && { peak: r.contact.peak.max_pressure, cyclic: r.contact.cyclic.max_pressure }, 1)} {t("ui.train_mpa")}
    <small>{t("ui.train_peak_cyclic")}</small>
    <small>
      {#if r}
      {t("ui.train_patch", {
        length: num(r.contact.peak.patch_length, 4),
        width: num(r.contact.peak.patch_width, 4),
      })} ·
      {Math.abs(r.contact.peak.worst_position) < 1e-9
        ? t("ui.train_worst_at_pitch_point")
        : t("ui.train_worst_along_the_path", {
            position: num(r.contact.peak.worst_position, 3),
          })}
      · {t("ui.train_pitch_point_alone_gives", {
        stress: num(r.contact.peak.at_pitch_point, 1),
      })}
      {/if}
    </small>
  </dd>
  <dt>{t("ui.train_sliding_speed")}</dt>
  <dd>{num(r?.sliding_velocity, 1)} mm/s</dd>
  <dt>{t("ui.train_bending_stress")}</dt>
  <dd>
    <small>{t("ui.train_not_reported_for_crossed_axes_no")}</small>
  </dd>
  <dt>{t("ui.train_flank_type")}</dt>
  <dd>
    {t("ui.train_flank_type_zi")}
    <small>{t("ui.train_zn_worm_s_contact_stress_1")}</small>
  </dd>
  {#if r?.crossed}
    <dt>{t("ui.train_contact_travel")}</dt>
    <dd>
      {num(r?.crossed.axial_travel[0], 3)} · {num(r?.crossed.axial_travel[1], 3)} mm
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
    /** **What decides this gear's face width**, which is one question with
     *  three answers rather than a flag with two.
     *
     *  `"rating"` — the default: a stress inverted, so the sources that size it
     *  are offered as toggles. `"continuity"` — a crossed pair, whose contact is
     *  a point no stress depends on, so the width comes from ε = 1 instead and
     *  says which kind of minimum it is. */
    faceWidth?: "rating" | "continuity";
    /** Called when this gear's shift is switched between given and automatic,
     *  so a stage whose inputs constrain one another can relieve whichever of
     *  them that has over-specified. */
    onShiftAuto?: () => void;
    /** The width at which ε = 1, for a crossed pair. */
    faceFromContinuity?: number;
    /** **The tool this ring is shaped with.**
     *
     *  A ring has no meaningful geometry without one, so its cutter belongs in
     *  its card rather than in a block of its own beside the stage's shared
     *  inputs — where a reader had to know which of the section's gears it was
     *  about. Given only for `cut: "shaper"`, which is the only kind of member
     *  that has one. */
    cutter?: Cutter;
    /** **What this member's speed has to add to the figure.**
     *
     *  A member with something more to say about a row the shared readout
     *  already prints says it *on* that row. The planet said it on a second one
     *  instead: an `extra` block repeating the speed with the annotation
     *  attached, so the card carried the same figure twice and only the copy
     *  was explained. `extra` is for a readout the shared one does not have —
     *  which is a crossed pair's, where there is no `GearResult` and no shared
     *  readout at all. */
    speedNote?: string;
    /** A readout only this member has; given the member's position.
     *
     *  Rendered whether or not the shared readout above it did, so anything it
     *  prints that the shared one also prints appears twice. */
    extra?: Snippet<[number]>;
    extraIndex?: number;
  },
)}
{@const own = g?.notes ?? []}
{@const mat = library.materials.material.find((m) => m.name === gear.material)}
{@const spare = (g?.notes ?? []).filter((n) => !UNDER_A_FIELD.includes(n.key))}
<div class="gear">
  <!-- **A heading names the fields under it**, and nothing else — so a card
       whose ring is shaped by a tool opens with that tool's heading and the
       gear's own comes back above the gear's own fields. Every card reads the
       same way: heading, then the fields it named. -->
  {#if opts.cutter}
    <!-- **The tool comes before the part**, because the part's root and fillet
         are the tool's: a ring's dedendum and root radius are not inputs of its
         own, and reading the cutter first is reading them. -->
    <h4>{t("ui.train_ring_cutter")}</h4>
    <label>
      <span>{t("ui.train_cutter_teeth")}</span>
      <input type="number" step="1" min="1" bind:value={opts.cutter.teeth} />
      <em></em>
      <FieldNote notes={notes(t("ui.train_note_cutter_teeth"), null)} />
    </label>
    <label>
      <span>{t("ui.train_cutter_addendum")}</span>
      <input type="number" step="0.05" bind:value={opts.cutter.addendum} />
      <em>{t("ui.train_m")}</em>
    </label>
    <label>
      <span>{t("ui.train_cutter_tip_round")}</span>
      <input type="number" step="0.02" bind:value={opts.cutter.tip_round} />
      <em>{t("ui.train_m")}</em>
    </label>
  {/if}
  <h4 class:later={opts.cutter !== undefined}>{title}</h4>
  <label class:invalid={g && outside(gear.teeth, g.ranges.teeth)}>
    <span>{t("ui.train_tooth_count")}</span>
    <input type="number" step="1" bind:value={gear.teeth} />
  </label>
  <!-- **The other end of the tooth, and the same shape as the shift's.** The
       addendum had an `auto` toggle whose only answer was the tallest tooth the
       tip width allows — a bound wearing a source's clothes, and one that went
       unread on every addendum a designer typed. It is the bound now, and the
       number is always the designer's. -->
  {@render boundedNumber(
    "ui.train_addendum",
    () => gear.addendum,
    (v) => (gear.addendum = v),
    0.05,
    "ui.train_m",
    undefined,
    {
      label: "ui.train_no_sharp_tip",
      title: "ui.train_note_no_sharp_tip",
      on: gear.no_sharp_tip,
      set: (v) => (gear.no_sharp_tip = v),
    },
    // What the bound came to, where it had something to say. A hint that names
    // an input belongs under that input rather than in a list at the foot of
    // the stage — and in the warning colour, because the number in the box is
    // not the number the gear has.
    clampNote(own, FIELD_NOTES.addendum),
  )}
  {#if gear.no_sharp_tip}
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
  <!-- **A ring is not asked about undercut.** Its flank is its shaper's
       rather than a rack's, so the constraint has nothing to bound and the
       searches never ask it (`auto::member_is_buildable`). -->
  {@render autoNumber(
    "ui.train_profile_shift",
    gear.profile_shift,
    g?.profile_shift,
    0.05,
    opts.onShiftAuto,
    undefined,
    "ui.train_m",
    opts.cut === "shaper"
      ? undefined
      : {
          label: "ui.train_no_undercut",
          title: "ui.train_note_no_undercut",
          on: gear.no_undercut,
          set: (v) => (gear.no_undercut = v),
        },
    clampNote(own, FIELD_NOTES.profile_shift),
  )}
  <!-- The depth the undercut question is asked at, so it is offered exactly
       while that question is being asked — which is now the constraint's
       business rather than the `auto` toggle's. -->
  {#if gear.no_undercut && opts.cut !== "shaper"}
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
  {#if !gear.profile_shift.auto}
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
              ? t("ui.bound_profile_shift", {
                  min: n(r.bound.min ?? 0),
                  max: n(r.bound.max ?? 0),
                  undercut: n(r.undercut),
                  sharp: n(r.sharp_rack_undercut),
                })
              : null,
          r ? outside(gear.profile_shift.manual, r.bound) : null,
        )
      } />
    </p>
  {/if}
  {#if opts.faceWidth === "continuity"}
    <!-- A crossed pair's automatic width is a **geometric** minimum: the width
         at which one tooth pair hands over to the next (ε = 1). The spur
         stage's inverts a stress instead, and the two must not read alike. -->
    {@render autoNumber(
      "ui.train_face_width",
      gear.face_width,
      opts.faceFromContinuity,
      0.5,
      undefined,
      // Inside the field, not beside it. A note outside its label is not the
      // label's row and does not get the gap that pairs the two — it sits a
      // whole field-gap below, reading as a heading for whatever follows.
      opts.faceFromContinuity === undefined
        ? t("ui.train_note_no_continuous_width")
        : t("ui.train_note_face_width_continuity", { width: n(opts.faceFromContinuity) }),
      "ui.train_mm",
    )}
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
  {#if gear.face_width.auto && (opts.faceWidth ?? "rating") === "rating"}
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

  <!-- **The material's own figures are the library's, not the solve's.**
       Reading them off the result meant that a stage which failed to build hid
       every override box a designer would reach for to make it build — inputs
       withheld for want of an answer they do not depend on. -->
  <div class="props">
    {@render property(t("ui.train_density"), gear, "density", g?.material.density ?? mat?.density, 10, t("ui.train_kg_m3"))}
    {@render property(t("ui.train_elastic_modulus"), gear, "elastic_modulus", g?.material.elastic_modulus ?? mat?.elastic_modulus, 100, t("ui.train_mpa"))}
    {@render property(t("ui.train_poissons_ratio"), gear, "poissons_ratio", g?.material.poissons_ratio ?? mat?.poissons_ratio, 0.01, "")}
    {@render property(t("ui.train_ultimate_allowable"), gear, "ultimate_allowable", g?.material.ultimate_allowable ?? mat?.ultimate_allowable, 10, t("ui.train_mpa"))}
    {@render property(t("ui.train_fatigue_allowable"), gear, "fatigue_allowable", g?.material.fatigue_allowable ?? mat?.fatigue_allowable, 10, t("ui.train_mpa"))}
  </div>
  <dl class="out small">
    <dt>{t("ui.train_torque")}</dt>
    <dd>{num(g?.torque, 4)} {g && "Nm"}</dd>
    <!-- Only where there is one. A back-driving load that nothing reacts
         reaches no gear, and an empty row is the honest report of that. -->
    {#if g?.back_driving_torque != null}
      <dt>{t("ui.train_back_driving_torque")}</dt>
      <dd>{num(g.back_driving_torque, 4)} Nm</dd>
    {/if}
    <dt>{t("ui.train_speed")}</dt>
    <dd>
      {num(g?.speed, 1)} {g && t("ui.train_rpm")}
      <!-- Where a member's speed has more to say than the number — a planet's
           teeth see the carrier's frame, not the ground's — it is said here,
           against the figure it qualifies. -->
      {#if opts.speedNote}<small>{opts.speedNote}</small>{/if}
    </dd>
    <dt>{t("ui.train_tooth_cycles")}</dt>
    <dd>
      {g ? `${count(g.tooth_cycles.bending)} / ${count(g.tooth_cycles.contact)}` : BLANK}
      <small>{t("ui.train_bending_contact")}</small>
    </dd>
    <dt>{t("ui.train_bending_stress")}</dt>
    <dd>
      {cases(g?.bending_stress, 1)} {g && t("ui.train_mpa")}
      <small>{t("ui.train_peak_cyclic")}</small>
    </dd>
    <!-- Per gear, and genuinely so. The two flanks share one pressure at any
         instant — the individual curvatures reach Hertz only through their
         sum — but the two gears are not rated at the same instant: each one's
         dedendum carries the load alone at its own end of the path, and that
         is where its pitting is assessed. -->
    <dt>{t("ui.train_contact_stress")}</dt>
    <dd>
      {cases(g?.contact_stress, 1)} {g && t("ui.train_mpa")}
      <small>{t("ui.train_peak_cyclic")}</small>
    </dd>
    <dt>{t("ui.train_min_face_width")}</dt>
    <dd>
      <span class="line">
        {cases(g && { peak: g.min_face_width.peak.bending, cyclic: g.min_face_width.cyclic.bending }, 3)} {g && "mm"}
        <small>{t("ui.train_bending_peak_cyclic")}</small>
      </span>
      <span class="line">
        {cases(g && { peak: g.min_face_width.peak.contact, cyclic: g.min_face_width.cyclic.contact }, 3)} {g && "mm"}
        <small>{t("ui.train_contact_peak_cyclic")}</small>
      </span>
    </dd>
  </dl>
  <!-- What the fields did not take. A note naming an input is drawn under
       that input, so repeating it here would be the same sentence twice in
       one card. -->

  {#if (g?.clamps.length ?? 0) > 0 || spare.length}
    <ul class="notes">
      {#each g?.clamps ?? [] as c, i (i)}<li>{t("ui.gear_clamped")} {note(c)}</li>{/each}
      <!-- The rating's own remarks, unprefixed: nothing here was clamped.
           Keyed by position rather than by note key, because two notes of one
           kind on one list is a thing that happens and a keyed list must not
           be what discovers it. -->
      {#each spare as n, i (i)}<li>{note(n)}</li>{/each}
    </ul>
  {/if}
{#if !g}
  <!-- **Only where there is no shared readout**, which is the whole of what
       this is for: a crossed pair produces no per-gear rating, so `g` is
       absent exactly when this is the only readout there is. Rendered
       unconditionally it was free to restate a row the shared one had already
       printed — the planet did, repeating its speed to hang an annotation on
       the copy — and nothing could have caught that but reading both. A
       member that *has* a rating adds to the row it belongs to instead
       (`speedNote`). -->
  {@render (opts.extra ?? noExtra)(opts.extraIndex ?? 0)}
{/if}
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
<!-- **A number with a bound on it, and nothing deciding it.** The same row as
     `autoNumber` without the `auto` switch: the bound stands in that column,
     which is where a switch that qualifies the box belongs whether it says who
     chose the number or what the number has to satisfy. -->
{#snippet boundedNumber(
  key: string,
  get: () => number,
  set: (v: number) => void,
  step: number,
  unit: string | undefined,
  note: string | null | undefined,
  constraint: { label: string; title: string; on: boolean; set: (v: boolean) => void },
  /** **A bound that actually moved this number.** Rendered in the warning
   *  colour and in front of the remark, because it is the same kind of finding
   *  the stage and mesh lists draw attention to and the reader has not got what
   *  they asked for. */
  warn?: string | null,
)}
  <label class="auto">
    <span class="name">{t(key)}</span>
    <input
      type="number"
      {step}
      value={get()}
      oninput={(e) => set(e.currentTarget.valueAsNumber)}
    />
    <span class="sw auto">
      <Switch
        small
        label={t(constraint.label)}
        on={constraint.on}
        title={t(constraint.title)}
        set={constraint.set}
      />
    </span>
    <em>{unit ? t(unit) : ""}</em>
    {#if note !== undefined || warn !== undefined}
      <FieldNote notes={notes(note ?? null, warn ?? null)} />
    {/if}
  </label>
{/snippet}

{#snippet autoNumber(
  key: string,
  a: Auto<number>,
  computed: number | undefined,
  step: number,
  after?: () => void,
  note?: string | null,
  unit?: string,
  /** A second switch, between `auto` and the box. **A constraint, where `auto`
   *  is a source**: `auto` says who decides the number, this says what the
   *  answer has to satisfy however it is decided — so the two combine rather
   *  than competing, and the nearer one to the box is the one that is about
   *  the value rather than about who supplies it. */
  constraint?: { label: string; title: string; on: boolean; set: (v: boolean) => void },
  /** As `boundedNumber`'s: a bound that moved the number, in warning colour. */
  warn?: string | null,
)}
  <label class="auto" class:constrained={constraint !== undefined}>
    <span class="name">{t(key)}</span>
    <!-- **The box comes first so the label is the box's.** A label activates
         its first labelable descendant, and a `<button>` is one — with the
         switches written above the input, clicking the field's *name* pressed
         the `auto` toggle, and every switch in the panel had a hit area
         stretching left to the previous thing in its row. Reading order is
         restored by placing each child in its own column below, which is where
         the columns were going to have to be named anyway once a row could
         carry two switches. -->
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
    <!-- **Left of the number it qualifies**, because that is what it qualifies.
         On the right it took the cell every other row prints its unit in, so an
         automatic field was the one field that could not say what it was
         measured in. -->
    <span class="sw auto">
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
    </span>
    {#if constraint}
      <span class="sw bound">
        <Switch
          small
          label={t(constraint.label)}
          on={constraint.on}
          title={t(constraint.title)}
          set={constraint.set}
        />
      </span>
    {/if}
    <em>{unit ? t(unit) : ""}</em>
    <!-- Inside the label, because that is where a note is laid out: `.note`
         spans this row's own columns and is right-aligned against them. Placed
         beside the field instead it spans whatever grid it lands in, which is
         the outer one, and lines up with nothing. -->
    {#if note !== undefined || warn !== undefined}
      <FieldNote notes={notes(note ?? null, warn ?? null)} />
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
        solved?.operating_torque_percent != null
          ? t("ui.train_note_operating_torque_percent", {
              percent: solved.operating_torque_percent.toFixed(1),
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
           cycle count, and the note says how — whether or not it is on. -->
      {@const act = tab.train.actuation.intermittent}
      {@render switchField(
        "ui.train_reversing",
        act.reversing,
        (v) => (act.reversing = v),
        t("ui.train_note_reversing"),
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
         uncorrected.

         Its note says what it does whether or not it is on: pressing a button
         to find out what it does is not an answer, and the slot is reserved
         either way. -->
    {@render switchField(
      "ui.train_reversed_bending",
      tab.train.reversed_bending,
      (v) => (tab.train.reversed_bending = v),
      t("ui.train_note_reversed_bending", {
        coefficient: defaults().reverse_loading_coefficient.toFixed(2),
      }),
    )}
  </div>

  <div class="summary">
    <!-- **The rows stand whether or not there is an answer in them.** A
         readout that vanishes takes its label with it, so the page a designer
         is editing changes shape at the moment they most need it to hold
         still; every figure below is blank instead until the train solves. -->
    <dl class="out">
      <dt>{t("ui.train_total_ratio")}</dt>
      <dd>
        {solved === undefined
          ? BLANK
          : solved.total_ratio >= 1
            ? `${num(solved.total_ratio, 4)} : 1`
            : `1 : ${num(1 / solved.total_ratio, 4)}`}
      </dd>
      <dt>{t("ui.train_output_speed_peak")}</dt>
      <dd>{num(solved?.output_speed, 1)} {solved && t("ui.train_rpm")}</dd>
      <dt>{t("ui.train_output_torque_peak")}</dt>
      <dd>{num(solved?.output_torque, 4)} {solved && "Nm"}</dd>
      <dt>{t("ui.train_total_efficiency")}</dt>
      <dd>
        {bothWays(solved?.total_efficiency)}
        {#if solved && solved.total_efficiency.backward <= 0}
          <small class="warn">{t("ui.train_cannot_be_back_driven")}</small>
        {/if}
      </dd>
      <dt>{t("ui.train_backlash_at_output_shaft")}</dt>
      <dd>
        {num(solved?.backlash.forward.nominal, 5)}{solved ? "°" : BLANK}
        <small
          >{range(num(solved?.backlash.forward.minimum, 5), num(solved?.backlash.forward.maximum, 5))}</small
        >
      </dd>
      <dt>{t("ui.train_backlash_at_input_shaft")}</dt>
      <dd>
        {num(solved?.backlash.backward.nominal, 5)}{solved ? "°" : BLANK}
        <small
          >{range(num(solved?.backlash.backward.minimum, 5), num(solved?.backlash.backward.maximum, 5))}</small
        >
      </dd>
    </dl>
    <!-- What the shaft line wants read, which no single stage is in a
         position to say: an input clamped against its peak, and where — or
         whether — the back-driving load is reacted. And, at the head of it,
         why there is no answer at all — through the catalogue like every other
         message, naming the stage where one is to blame. -->
    {#if failure || (solved?.notes.length ?? 0) > 0}
      <ul class="notes">
        {#if failure}
          <li class="warn">
            {failure.stage === null
              ? note(failure.note)
              : `${stageName(failure.stage - 1)}: ${note(failure.note)}`}
          </li>
        {/if}
        {#each solved?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
      </ul>
    {/if}
  </div>
</section>

<div class="stages">
  {#each tab.train.stages as stage, i (i)}
    {@const res = solved?.stages[i] ?? null}
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
                  faceWidth: stage.shaft_angle === 0 ? "rating" : "continuity",
                  faceFromContinuity: xres?.crossed?.face_width_for_continuity?.[j],
                  extra: xres ? crossedMember : undefined,
                  extraIndex: j,
                })}
              {/each}
            </div>

            {#if stage.shaft_angle !== 0}
              {@render screwReadout(xres ?? undefined, [gearName(i, 0), gearName(i, 1)])}
              {#if (xres?.notes.length ?? 0) > 0}
                <ul class="notes">
                  {#each xres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}
            {/if}

            {#if stage.shaft_angle === 0}
              <dl class="out">
                <dt>{t("ui.train_centre_distance")}</dt>
                <dd>
                  {num(sres?.centre_distance, 4)} {sres && "mm"}
                  <small>{sres && t("ui.train_nominal_value", { value: num(sres.centre_distance_nominal, 4) })}</small>
                </dd>
                <dt>{t("ui.train_operating_pressure_angle")}</dt>
                <dd>{num(sres?.operating_pressure_angle, 3)}{sres ? "°" : BLANK}</dd>
                <dt>{t("ui.train_contact_ratio")}</dt>
                <dd>
                  {#if sres}
                    ε<sub>α</sub> {num(sres.contact_ratios.transverse, 4)} · ε<sub>β</sub>
                    {num(sres.contact_ratios.overlap, 4)} · ε<sub>γ</sub>
                    {num(sres.contact_ratios.total, 4)}
                    {#if stage.additional_helix !== 0 && sres.contact_ratios.overlap < 1}
                      <small class="warn">{t("ui.train_no_full_axial_overlap")}</small>
                    {/if}
                  {/if}
                </dd>
                <!-- The one figure both members share: same patch, same normal
                     force, same E*, one instant. Each gear's own rating is on
                     its card and is this or worse. -->
                <dt>{t("ui.train_contact_stress_at_pitch_point")}</dt>
                <dd>
                  {cases(sres?.contact_stress_at_pitch_point, 1)} {sres && t("ui.train_mpa")}
                  <small>{t("ui.train_peak_cyclic")}</small>
                  <small>{sres ? `ρ ${num(sres.relative_radius, 3)} mm` : BLANK}</small>
                </dd>
                <dt>{t("ui.train_mesh_efficiency")}</dt>
                <dd>
                  {bothWays(sres?.efficiency)}
                </dd>
                <dt>{t("ui.train_backlash")}</dt>
                <dd>
                  {sres
                    ? t("ui.train_backlash_at", {
                        angle: num(sres.backlash.forward.nominal, 5),
                        member: gearName(i, 1),
                      })
                    : BLANK}
                  <small
                    >{range(num(sres?.backlash.forward.minimum, 5), num(sres?.backlash.forward.maximum, 5))}</small
                  >
                  {sres
                    ? `· ${t("ui.train_backlash_at", {
                        angle: num(sres.backlash.backward.nominal, 5),
                        member: gearName(i, 0),
                      })}`
                    : BLANK}
                </dd>
                <dt>{t("ui.train_coprime")}</dt>
                <dd>{sres ? (sres.coprime ? t("ui.train_yes") : t("ui.train_no")) : BLANK}</dd>
              </dl>
              {#if (sres?.notes.length ?? 0) > 0}
                <ul class="notes">
                  {#each sres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
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
                <dl class="out">
                  <dt>{t("ui.train_lead_angle")}</dt>
                  <dd>{num(wres?.lead_angle, 4)}{wres ? "°" : BLANK}</dd>
                  <dt>{t("ui.train_lead")}</dt>
                  <dd>{num(wres?.lead, 4)} {wres && "mm"}</dd>
                  <dt>{t("ui.train_torque")}</dt>
                  <dd>{num(wres?.members[0].torque, 4)} {wres && "N·m"}</dd>
                  <dt>{t("ui.train_speed")}</dt>
                  <dd>{num(wres?.members[0].speed, 1)} {wres && t("ui.train_rpm")}</dd>
                </dl>
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
                <dl class="out">
                  <dt>{t("ui.train_pitch_diameter")}</dt>
                  <dd>{num(wres?.members[1].pitch_diameter, 4)} {wres && "mm"}</dd>
                  <dt>{t("ui.train_torque")}</dt>
                  <dd>{num(wres?.members[1].torque, 4)} {wres && "N·m"}</dd>
                  <dt>{t("ui.train_speed")}</dt>
                  <dd>{num(wres?.members[1].speed, 1)} {wres && t("ui.train_rpm")}</dd>
                </dl>
              </div>
            </div>

            {@render screwReadout(wres ?? undefined, [t("ui.train_the_worm"), t("ui.train_the_wheel")])}
            {#if (wres?.notes.length ?? 0) > 0}
              <ul class="notes">
                {#each wres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
              </ul>
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
              </label>
              {@render efficiencyToggle(stage.optimisation)}
            </div>

            <!-- **One of the three shifts closes the set**, and which one is
                 read off the toggles rather than named by a control of its own:
                 the member left automatic absorbs, and the planet is preferred
                 because it is the one in both meshes. So pinning the planet is
                 how a designer asks for the sun to close it instead. The
                 absorbing member is an automatic one like any other, box and
                 toggle and all: hiding either would hide the only way to hand
                 the job on. -->
            <div class="gears">
              {@render gearCard(t("ui.train_sun"), stage.sun, pres?.sun, {
                cut: "rack",
                onShiftAuto: () => relievePlanetary(stage, stage.sun.profile_shift),
              })}
              {@render gearCard(t("ui.train_planet"), stage.planet, pres?.planet.gear, {
                cut: "rack",
                onShiftAuto: () => relievePlanetary(stage, stage.planet.profile_shift),
                // The one thing only a planet's speed has: its teeth turn in
                // the carrier's frame, and that is the speed they wear at.
                speedNote: pres
                  ? t("ui.train_relative_to_the_carrier", {
                      speed: pres.planet.speed_relative.toFixed(1),
                    })
                  : undefined,
              })}
              <!-- A ring's root and fillet are its cutter's, so it has neither a
                   dedendum nor a root radius of its own (docs/reference.md#internal-gears); the tool
                   it is shaped by leads its card. -->
              {@render gearCard(t("ui.train_ring"), stage.ring, pres?.ring, {
                cut: "shaper",
                cutter: stage.cutter,
                onShiftAuto: () => relievePlanetary(stage, stage.ring.profile_shift),
              })}
            </div>

              <dl class="out">
                <dt>{t("ui.train_ratio")}</dt>
                <dd>
                  {num(pres?.ratio, 4)} : 1
                  <small>
                    {pres &&
                      t("ui.train_in_held_out", {
                        input: shaft(pres.arrangement.input),
                        held: shaft(pres.arrangement.fixed),
                        output: shaft(pres.output),
                      })}
                  </small>
                </dd>
                <dt>{t("ui.train_centre_distance")}</dt>
                <dd>
                  {num(pres?.centre_distance, 4)} {pres && "mm"}
                  <small>
                    {pres &&
                      t("ui.train_common_to_both_meshes", {
                        residual: pres.planet.shift_residual.toExponential(1),
                      })}
                  </small>
                </dd>
                <dt>{t("ui.train_efficiency")}</dt>
                <dd>
                  {bothWays(pres?.efficiency)}
                </dd>
                <!-- Both shafts the same two plays are seen from, as every other
                     stage kind names both ends of its own: driving forward the
                     play is read at the output, and driving backward at the
                     shaft that was the input. -->
                <dt>{t("ui.train_backlash")}</dt>
                <dd>
                  {pres &&
                    t("ui.train_at_the_shaft", {
                      angle: num(pres.backlash.forward.nominal, 5),
                      shaft: shaft(pres.output),
                    })}
                  <small
                    >{range(num(pres?.backlash.forward.minimum, 5), num(pres?.backlash.forward.maximum, 5))}</small
                  >
                  {pres
                    ? `· ${t("ui.train_at_the_shaft", {
                        angle: num(pres.backlash.backward.nominal, 5),
                        shaft: shaft(pres.arrangement.input),
                      })}`
                    : BLANK}
                </dd>
                <dt>{t("ui.train_planet_clearance")}</dt>
                <dd>
                  {#if pres}
                    {pres.planet_clearance === null
                      ? t("ui.train_one_planet_no_neighbour")
                      : `${num(pres.planet_clearance, 3)} mm`}
                    {#if pres.planet_clearance !== null}
                      <small class:warn={!pres.planet_clearance_ok}>
                        {t(pres.planet_clearance_ok ? "ui.train_meets_the_minimum" : "ui.train_below_the_minimum")}
                      </small>
                    {/if}
                  {/if}
                </dd>
                <!-- Two separate layout checks, so two rows. Even spacing is
                     `N | z_sun + z_ring`; simultaneous meshing is the stricter
                     `N | z_sun` *and* `N | z_ring`, and a false answer is not a
                     fault — it means the planets engage staggered, which is
                     usually preferable. -->
                <dt>{t("ui.train_even_spacing")}</dt>
                <dd>{pres ? (pres.equal_spacing ? t("ui.train_yes") : t("ui.train_no")) : BLANK}</dd>
                <dt>{t("ui.train_simultaneous_meshing")}</dt>
                <dd>{pres ? (pres.simultaneous_meshing ? t("ui.train_yes") : t("ui.train_no")) : BLANK}</dd>
              </dl>

              <!-- The two meshes, each stacked like a spur stage's readout
                   rather than a table that lines up with nothing else on the
                   panel. The coprime check belongs to a mesh — the sun against
                   the planets, the ring against the planets — so it leads each
                   list. -->
              <!-- Two meshes, always — the set has them whether or not it
                   solved, so the sections stand and their figures go blank. -->
              {#each [
                [t("ui.train_mesh_sun_planet"), pres?.sun_planet, pres?.sun_coprime_with_planets, "ui.train_the_sun", "ui.train_the_planet"],
                [t("ui.train_mesh_planet_ring"), pres?.planet_ring, pres?.ring_coprime_with_planets, "ui.train_the_planet", "ui.train_the_ring"],
              ] as const as [label, m, coprime, first, second] (label)}
                <h4 class="mesh">{label}</h4>
                <dl class="out indent">
                  <!-- **Two coprime checks, and they are different questions.**
                       This one is the central member against the *planet count*
                       — whether the set's meshes come round together — and it
                       shared a label with the hunting check every mesh reports,
                       which is the one `meshRows` draws below. -->
                  <dt>{t("ui.train_coprime_with_planets")}</dt>
                  <dd>{coprime === undefined ? BLANK : coprime ? t("ui.train_yes") : t("ui.train_no")}</dd>
                  {@render meshRows(m, [t(first), t(second)], stage.helix_angle !== 0)}
                </dl>
              {/each}

              {#if (pres?.notes.length ?? 0) > 0}
                <ul class="notes">
                  {#each pres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
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


            {#each [0, 1] as m (m)}
              <!-- **Which member is the ring is a tooth count, not a result.**
                   Two axes one crank offset apart can only be an internal pair,
                   so the ring is whichever has more teeth — `hula::Teeth::pair`
                   says the same thing in the core. Reading it off the solve
                   instead left the cards, and the tool that shapes the ring,
                   labelled from a stale answer before the first one arrived. -->
              {@const ring = stage.gears[m * 2].teeth >= stage.gears[m * 2 + 1].teeth ? m * 2 : m * 2 + 1}
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
                  <span>{t("ui.train_sliding_friction")}</span>
                  <input type="number" step="0.01" bind:value={stage.sliding_friction[m]} />
                  <em></em>
                </label>
                <label>
                  <span>{t("ui.train_static_friction")}</span>
                  <input type="number" step="0.01" bind:value={stage.static_friction[m]} />
                  <em></em>
                  <FieldNote notes={notes(t("ui.train_note_static_friction"), null)} />
                </label>
              </div>
              <div class="gears">
                {#each [ring, pinion] as j (j)}
                  {@render gearCard(
                    t(j === ring ? "ui.train_ring" : "ui.train_pinion") +
                      " — " +
                      t(
                        ["ui.train_hula_role_grounded", "ui.train_hula_role_wobble", "ui.train_hula_role_wobble", "ui.train_hula_role_output"][j],
                      ),
                    stage.gears[j],
                    hres?.gears[j].gear,
                    {
                      cut: j === ring ? "shaper" : "rack",
                      cutter: j === ring ? stage.cutter[m] : undefined,
                      onShiftAuto: () => relieveHula(stage, m, stage.gears[j].profile_shift),
                    },
                  )}
                {/each}
              </div>
              <!-- Not indented: an epicyclic set's mesh readouts sit under a
                   heading of their own and are inset from it, where these stand
                   in their mesh's own section beneath its gear cards — the same
                   place a pair's readout stands in its stage. -->
              <dl class="out">
                <!-- What every parallel-axis mesh reports, then what only this
                     arrangement has — the same order the spur and screw
                     readouts take. The mesh was built pinion first, so the
                     pinion leads the pair of backlash figures. -->
                {@render meshRows(
                  hres?.meshes[m].report,
                  [t("ui.train_the_pinion"), t("ui.train_the_ring")],
                  stage.helix_angle !== 0,
                )}
                <dt>{t("ui.train_hula_clearance_result")}</dt>
                <dd>
                  {num(hres?.meshes[m].clearance, 4)} {t("ui.train_mm")}
                  <small>
                    {t("ui.train_hula_clearance_as_cut", {
                      value: num(hres?.meshes[m].clearance_as_cut, 4),
                    })}
                  </small>
                </dd>
                <!-- **Which conditions bite, and nothing when none do.** The
                     tip margin was a number beside this saying the same thing
                     in degrees of pinion rotation — the row that reports the
                     finding is the one worth drawing attention to. -->
                <dt>{t("ui.train_hula_interference")}</dt>
                <dd>
                  {#each [[
                    hres?.meshes[m].trochoid_interference
                      ? t("ui.train_hula_interference_trochoid")
                      : null,
                    hres?.meshes[m].involute_interference
                      ? t("ui.train_hula_interference_involute")
                      : null,
                    hres?.meshes[m].tip_interference
                      ? t("ui.train_hula_interference_tip")
                      : null,
                  ].filter((x) => x !== null)] as fouling (0)}
                    <span class:warn={fouling.length > 0}>
                      {fouling.join(" · ") || t("ui.train_hula_interference_none")}
                    </span>
                  {/each}
                </dd>
              </dl>
              <!-- A clamp that is about the part rather than about a box is
                   drawn on the card that owns it, like every other stage's —
                   the pair-wide list that used to stand here was the only place
                   in the application where one gear's finding was filed under
                   two gears. -->
            {/each}

            <!-- The drive as a whole, under the meshes it is made of — where every
                 other stage puts its readout. -->
              <!-- Ordered as the spur and screw readouts are — the distance the
                   pair runs at, contact, efficiency, backlash — with what only
                   this arrangement has following on. -->
              <dl class="out">
                <dt>{t("ui.train_ratio")}</dt>
                <dd>
                  {num(hres?.ratio, 4)} : 1
                  <small>{`${t("ui.train_hula_ratio_products", {
                        numerator: String(hres?.ratio_products[0]),
                        denominator: String(hres?.ratio_products[1]),
                      })} · ${t("ui.train_hula_note_ratio", {
                        denominator: String(hres?.ratio_products[1]),
                      })}`}</small>
                </dd>
                <dt>{t("ui.train_efficiency")}</dt>
                <dd>
                  {bothWays(hres?.efficiency)}
                  {#if hres && hres.efficiency.backward <= 0}
                    <small class="warn">{t("ui.train_self_locking")}</small>
                  {/if}
                </dd>
                <!-- The two shafts the same two plays are seen from, which
                     differ by the whole reduction — so both are named, as the
                     spur stage names its two members. -->
                <dt>{t("ui.train_backlash")}</dt>
                <dd>
                  {t("ui.train_at_the_shaft", {
                    angle: num(hres?.backlash.forward.nominal, 5),
                    shaft: t("ui.train_hula_role_output"),
                  })}
                  <small
                    >{range(num(hres?.backlash.forward.minimum, 5), num(hres?.backlash.forward.maximum, 5))}</small
                  >
                  · {t("ui.train_at_the_shaft", {
                    angle: num(hres?.backlash.backward.nominal, 5),
                    shaft: t("ui.train_hula_role_crank"),
                  })}
                </dd>
                <!-- **The crank alone**, because it is the one shaft here that
                     is not a gear. The wobble body's speed and the output's are
                     printed on the cards of the gears that turn at them, and a
                     row repeating them was the same figure twice on one page. -->
                <dt>{t("ui.train_hula_speeds")}</dt>
                <dd>{num(hres?.crank_speed, 1)} {hres && t("ui.train_rpm")}</dd>
              </dl>

              <!-- What the drive had to say that no one gear owns, as every
                   other stage kind reports its own. -->
              {#if (hres?.notes.length ?? 0) > 0}
                <ul class="notes">
                  {#each hres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
                </ul>
              {/if}

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
    /* The gap *between* fields, which is the one a note is paired against: at
       0.4rem a note sat nearly as far from its own box as from the next field,
       which is the reading `--note-gap`/`--field-gap` exist to prevent. The
       gear cards below this column and the whole gear tab were already using
       it. */
    gap: var(--field-gap);
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
  /* A second switch takes a second column of its own, out of the label's share
     again, so the box keeps the edge every other box in the card shares. */
  label.auto.constrained {
    grid-template-columns: 1fr auto auto 6rem 3.5rem;
  }
  .grid.shared > label.auto.constrained {
    grid-template-columns: 1fr auto auto 9rem 3.5rem;
  }
  .gear label.auto.constrained {
    grid-template-columns: 1fr auto auto 6.5rem 3.5rem;
  }
  /* **Read in columns, not in source order.** The box is written first so the
     row's label is for the box; these put everything back where it reads. Each
     switch sits in a wrapper for the same reason — a child component's own
     element is out of this stylesheet's reach, and the wrapper is the grid item
     it needs to place. */
  /* **Every one of them says which row as well as which column.** Naming only
     the column leaves the row to auto-placement, and auto-placement will not go
     back: with the box written first and placed in column 3, the switch that
     belongs in column 2 no longer fits on the cursor's row and starts a new one
     — which put the toggle and the unit a line below the box they belong to. */
  label.auto > .name,
  label.auto > .sw,
  label.auto > input,
  label.auto > em {
    grid-row: 1;
  }
  label.auto > .name {
    grid-column: 1;
  }
  label.auto > .sw.auto {
    grid-column: 2;
  }
  label.auto > input {
    grid-column: 3;
  }
  label.auto > em {
    grid-column: 4;
  }
  label.auto.constrained > .sw.bound {
    grid-column: 3;
  }
  label.auto.constrained > input {
    grid-column: 4;
  }
  label.auto.constrained > em {
    grid-column: 5;
  }
  /* The wrapper is only a handle for placement; it must not add a hit area of
     its own beyond the button it holds. */
  label.auto > .sw {
    display: flex;
    justify-self: start;
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
    gap: var(--row-gap);
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
    padding-left: 0.9rem;
  }
  /* A readout **directly** under its heading hugs it; one further down the
     section keeps the standard gap. Written as the adjacency it is, rather than
     folded into `.indent`, which is about the inset and says nothing about what
     comes above. */
  h4.mesh + .out {
    margin-top: 0.2rem;
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
  /* A second heading in a card opens a second section, so it needs the gap
     between sections above it — the first one is against the card's own top
     padding and needs none. */
  .gear h4.later {
    margin-top: 0.8rem;
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
    /* Carried explicitly now that this is not a `<label>`: the row gap is what
       pairs a note to the control above it, and the size is every field row's. */
    row-gap: var(--note-gap);
    font-size: 0.85rem;
    /* **Stretch, explicitly.** `label` sets `align-items: center` for its grid
       rows, where it means "centre the box against its label vertically". On a
       flex column it means "centre every child horizontally", which is not a
       thing any row here wants and is what this quietly became when the rule
       that used to override it moved onto `.control`. */
    align-items: stretch;
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
