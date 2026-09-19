<script lang="ts">
  import {
    defaults,
    solveTrain,
    STAGE_PRESETS,
    type StagePresetSpec,
    type Figure,
    CASE_KINDS,
    type CaseKindSpec,
    portKey,
    portOptions,
    type CaseKind,
    type Port,
    type ShaftRef,
    type ShaftLabel,
    type LoadCase,
    type GearCase,
    type MeshCase,
    outside,
    type Auto,
    type Overrides,
    type StageGear,
    type Stage,
    type Shape,
    type Member,
    type Optimisation,
    type Value,
    type GearResult,
    type Note,
    type Cutter,
    type MeshReport,
    type LoadSharing,
    type Constraint,
    constraintOn,
    note,
    t,
  } from "./core";
  import { developer, trains, library, type TrainTab } from "./state.svelte";
  import { exportTrain, relieveStage } from "./core";
  import FieldNote from "./FieldNote.svelte";
  import Switch from "./Switch.svelte";
  import { notes, type Notes } from "./notes";
  import {
    shaftName,
    memberName,
    isWorm,
    carried,
  } from "./members";

  /** **Resolving an over-determined stage is the core's rule, not this file's.**
   *
   *  Which of a stage's inputs argue with each other, how many may stand and
   *  which gives way first are facts about the geometry, and they lived here as
   *  three functions — one per stage type — each restating a relation Rust
   *  already enforces. That is an engineering rule outside Rust and the same
   *  idea written once per type: a new arrangement would have arrived with no relief
   *  at all, and one of the three carried a justification that went stale the
   *  moment the core stopped needing it.
   *
   *  `Stage::freedoms` declares them now and `relieveStage` walks them, so this
   *  file passes on which toggle was just pinned and nothing else. */

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
  // ...against the library the reader has, where they imported one — the
  // shipped one otherwise, which Rust supplies for itself. It was never
  // sent, so an imported library reached the material readouts and not the
  // ratings.
  const result = $derived(
    solveTrain(tab.train, library.origin === null ? undefined : library.materials),
  );
  /** The answer, where there is one — `undefined` reads through every formatter
   *  below as a blank rather than as a row that is not there. */
  const solved = $derived(result.result ?? undefined);
  const failure = $derived(result.failure);
  /** What a stage's inputs last came to, by name — handed back to relief so a
   *  box it turns given holds the number it showed. An empty list where the
   *  train has not solved, and the box keeps what it had. */
  const figuresOf = (stage: Stage): Figure[] =>
    result.figures[tab.train.stages.indexOf(stage)] ?? [];

  /** **What the train asks of one shaft**, written as a constraint or, for
   *  the empty choice, withdrawn so the stage's convention stands again. A
   *  constraint is a choice and not a number, so it is written here; what
   *  the choice *does* — a hold on a set releasing the ring it held by
   *  convention, a drive behind a pair being where the chain enters — is
   *  the core's rule, read back through the shaft line. */
  function constrain(stage: number, shaft: number, c: Constraint | "") {
    const at: ShaftRef = { kind: "of", stage, shaft };
    tab.train.constraints = tab.train.constraints.filter(
      (x) => !(x.at.kind === "of" && x.at.stage === stage && x.at.shaft === shaft),
    );
    if (c !== "") tab.train.constraints.push({ at, constraint: c });
  }
  /** The word for what a shaft is asked, or for what its stage's convention
   *  asks where the train says nothing. */
  const constraintWord = (c: Constraint) =>
    t(
      { held: "ui.train_constraint_held", driven: "ui.train_constraint_driven", free: "ui.train_constraint_free" }[
        c
      ],
    );

  /** Which duty a fatigue case is counted over. Switching seeds the other
   *  shape from the core's own defaults — a fresh case's intermittent duty,
   *  and the continuous one the boundary carries for exactly this — so no
   *  number is written on this side. */
  const dutyMode = (c: LoadCase): "intermittent" | "continuous" =>
    "intermittent" in c.duty ? "intermittent" : "continuous";
  function setDuty(c: LoadCase, m: "intermittent" | "continuous") {
    if (m === dutyMode(c)) return;
    c.duty = m === "intermittent" ? defaults().fatigue_case.duty : defaults().continuous_duty;
  }

  /** **The load cases are a list, as the stages are**, added one of each kind
   *  from the core's own defaults. Unlike the stages, the last one may go: a
   *  train with no load case is a shaft line and nothing else, every rating
   *  row stands empty, and the two buttons under the list are how one comes
   *  back — where a train with no stage is one the core refuses. */
  function addCaseOfKind(kind: CaseKindSpec) {
    tab.train.load_cases.push(kind.fresh());
    tab.openCases[tab.train.load_cases.length - 1] = true;
  }
  function removeCase(i: number) {
    tab.train.load_cases.splice(i, 1);
    const was = { ...tab.openCases };
    tab.openCases = {};
    for (const [k, v] of Object.entries(was)) {
      const at = Number(k);
      if (at < i) tab.openCases[at] = v;
      else if (at > i) tab.openCases[at - 1] = v;
    }
  }
  /** A load case by number, as a stage is; and the words for its kind and its
   *  port, from the same tables the selects offer them from. */
  const caseName = (i: number) => t("ui.train_case_heading", { number: String(i + 1) });
  const kindLabel = (k: CaseKind) => t(CASE_KINDS.find((x) => x.key === k)?.label ?? k);
  /** The word for a port: the chain's two ends by name, and a named shaft
   *  as its stage and the shaft's own name — read off the topology the core
   *  sent, so a shaft's name is the wiring's and not a guess from its index. */
  const portLabel = (p: Port): string => {
    if (p === "start") return t("ui.train_port_start");
    if (p === "end") return t("ui.train_port_end");
    return refLabel(p.at);
  };
  /** A shaft by reference: its stage and its own name. The label is the
   *  wiring's where the caller has it — a shaft-line row carries its own —
   *  and looked up among the stage's ports otherwise, which is every shaft a
   *  port can name. */
  const refLabel = (at: ShaftRef, label?: ShaftLabel): string => {
    if (at.kind === "ground") return t("ui.train_ground");
    const { stage, shaft } = at;
    const known = label ?? result.topology[stage]?.ports.find((x) => x.shaft === shaft)?.label;
    return t("ui.train_port_at", {
      stage: stageName(stage),
      shaft: known ? shaftName(tab.train, result.topology, stage, known) : String(shaft),
    });
  };
  /** The ports a case's select offers, keyed for the select; a port is set
   *  by looking its key up here, never by parsing the key. */
  const portOptionsNow = $derived(portOptions(result.motion));
  const portByKey = (key: string): Port =>
    portOptionsNow.find((o) => o.key === key)?.port ?? "start";
  /** **Every result names its case**, so a readout that stands for a case looks
   *  its figures up by that index rather than by position: a case switched off
   *  has no result and its row draws blank, and the rows are the train's cases
   *  in the train's order either way. */
  const forCase = <T extends { case: number }>(list: T[] | undefined, i: number) =>
    list?.find((c) => c.case === i);

  /** The presets on offer. A preset the developer mode hides cannot already be in
   *  a train the reader is looking at — the picker is the only way one arrives
   *  — so nothing is stranded by the mode being off. */
  const stagePresets = $derived(STAGE_PRESETS.filter((k) => !k.developer || developer.enabled));

  function addStagePreset(preset: StagePresetSpec) {
    tab.train.stages.push(preset.fresh());
    tab.open[tab.train.stages.length - 1] = true;
  }




  /** Deleting the last stage leaves a fresh one, as deleting the last gear
   *  tab or the last geartrain does — a train with no stages is one the core
   *  refuses, and a button that greys out to prevent that is a rule the
   *  reader has to infer. */
  function removeStage(i: number) {
    tab.train.stages.splice(i, 1);
    // The constraints and couplings name stages by index: those on the
    // removed stage go with it, and those after it move down with the
    // stages they belong to. Left alone, holding stage 3's carrier would
    // hold whatever became stage 3.
    const moved = (r: ShaftRef): ShaftRef | null =>
      r.kind !== "of" ? r : r.stage === i ? null : r.stage > i ? { ...r, stage: r.stage - 1 } : r;
    tab.train.constraints = tab.train.constraints.flatMap((c) => {
      const at = moved(c.at);
      return at ? [{ ...c, at }] : [];
    });
    tab.train.couplings = tab.train.couplings.flatMap((c) => {
      const [a, b] = [moved(c.a), moved(c.b)];
      return a && b ? [{ a, b }] : [];
    });
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
    if (tab.train.stages.length === 0) addStagePreset(STAGE_PRESETS[0]);
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
  /** **A shaft of a stage by name**, off the topology the core sent — the
   *  label a port carries — and by its number where the wiring has no name
   *  for it. */
  const shaftLabel = (stage: number, s: number): string => {
    const label = result.topology[stage]?.ports.find((p) => p.shaft === s)?.label;
    return label ? shaftName(tab.train, result.topology, stage, label) : String(s);
  };
  /** Whether a mesh is internal and on distance `k` of a shape: one of
   *  its members has a cutter, and its two members' axes are the distance's. */
  const internalOn = (shape: Shape, m: { a: number; b: number }, k: number): boolean => {
    const axisOf = (member: number) => shape.shafts[shape.members[member].shaft - 1]?.axis;
    const [a, b] = [axisOf(m.a), axisOf(m.b)];
    const d = shape.distances[k];
    const on = (d.axes[0] === a && d.axes[1] === b) || (d.axes[0] === b && d.axes[1] === a);
    return on && (shape.members[m.a].ring !== null) !== (shape.members[m.b].ring !== null);
  };
  /** **An axis by the members on it**, which is how a designer knows one:
   *  "gear 3" is an axis, and so is "sun". A carrier's axis has no member
   *  and goes by the shaft that carries it. */
  const axisName = (shape: Shape, stage: number, axis: number): string => {
    const on = shape.members
      .map((m, j) => (shape.shafts[m.shaft - 1]?.axis === axis ? memberName(tab.train, result.topology, stage, j) : null))
      .filter((x) => x !== null);
    if (on.length > 0) return on.join(" / ");
    const s = shape.shafts.findIndex((x) => x.axis === axis) + 1;
    return s > 0 ? shaftLabel(stage, s) : String(axis + 1);
  };
  /** A stage by number. The *word* is a label and belongs in the catalogue;
   *  the number is a name and does not. Written here rather than at each place
   *  it is read, so a heading and a reference to it cannot drift apart. A
   *  member's name is `members.ts`'s, where the gear tab reads the same one. */
  const stageName = (i: number) => t("ui.train_stage_heading", { number: String(i + 1) });
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
    profile_shift: ["gear.shift_raised_for_undercut"],
    addendum: ["gear.addendum_held_to_tip_width"],
    face_width: ["gear.face_width_no_source", "gear.face_width_as_entered"],
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

  /** **A mesh's notes are drawn beside the figure each is about.** Those
   *  about its contact — a ratio below one, a helical pair short of full
   *  overlap, a sharing model extrapolating past the single-pair zone — go
   *  under the contact ratio; everything else a mesh can say is about its
   *  efficiency — locking, nearly locking, losing more than it keeps — and
   *  goes under that. Two groups and a remainder rather than a key per note,
   *  so a note the core adds is drawn somewhere rather than nowhere. The core
   *  raises them on the mesh, so a set says which of its two meshes. */
  const CONTACT_NOTES: readonly string[] = [
    "mesh.contact_ratio_below_one",
    "mesh.overlap_below_one",
    "mesh.load_sharing_out_of_band",
  ];
  const contactNotes = (m: MeshReport | undefined) =>
    (m?.notes ?? []).filter((n) => CONTACT_NOTES.includes(n.key));
  const efficiencyNotes = (m: MeshReport | undefined) =>
    (m?.notes ?? []).filter((n) => !CONTACT_NOTES.includes(n.key));

  /** **Which directions a mesh refuses to be driven in**, named.
   *
   *  The mirror of `Directional::locked`, and written once here because it was
   *  written three times: the crossed readout, the hula stage and the train
   *  total each tested `efficiency.backward <= 0` inline and each could say only
   *  *self-locking*. A pair that cannot be driven **forward** — a steep helix
   *  split, which `gear-cli crossed 17 23 90` reaches at 9°/81° — showed a bare
   *  `0.000 %` and no words at all.
   *
   *  A screw mesh says more than this, and says it from Rust: `mesh.self_locking`
   *  and `mesh.forward_locking` carry the coefficient and the threshold, and
   *  are drawn beside the mesh's efficiency. This is for the readouts that have
   *  no note behind them. */
  const lockedWays = (e: { forward: number; backward: number } | undefined) => {
    if (e === undefined) return undefined;
    const [f, b] = [e.forward <= 0, e.backward <= 0];
    if (f && b) return t("ui.train_turns_neither_way");
    if (f) return t("ui.train_cannot_be_driven_forward");
    if (b) return t("ui.train_cannot_be_back_driven");
    return undefined;
  };

  /** The friction each direction stops driving at. Negative means no friction
   *  locks it that way, which is a fact and not a missing value — the ordinary
   *  case forwards for anything you can turn. */
  const locksAt = (e: { forward: number; backward: number } | undefined) => {
    if (e === undefined) return BLANK;
    const at = (v: number) => (v > 0 ? v.toFixed(4) : t("ui.train_locking_never"));
    return `${t("ui.train_locking_forward", { mu: at(e.forward) })} · ${t(
      "ui.train_locking_backward",
      { mu: at(e.backward) },
    )}`;
  };

  /** A rating that may not exist for a member renders as a dash rather than as
   *  a zero, because those are different facts — and as a blank where nothing
   *  has solved, like every other figure. */
  const rated = (v: number | null | undefined, digits: number) =>
    v === undefined ? BLANK : v === null ? "—" : v.toFixed(digits);

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
<!-- **What one mesh reports**, drawn once for every mesh of every stage.

     A `MeshReport` is the same shape whether it is a spur pair's, a worm's, an
     epicyclic set's sun–planet pair or a hula stage's: the shared rows first,
     then the little only a line contact has (the transverse decomposition and
     the operating angle) or only a point contact has (the zone as the faces
     leave it, the parallel counterpart). A crossed pair used to have a readout
     of its own beside this one, and before that the spur stage did — each a
     second place the same row could be drawn differently. `members` names the
     two ends the one gap is seen from, in the order the mesh was built;
     `notes` is the stage's, for the lock notes drawn beside the efficiency. -->
{#snippet meshRows(m: MeshReport | undefined, members: [string, string])}
  <dt>{t("ui.train_coprime")}</dt>
  <dd>{m === undefined ? BLANK : m.coprime ? t("ui.train_yes") : t("ui.train_no")}</dd>
  {#if m?.line}
    <dt>{t("ui.train_operating_pressure_angle")}</dt>
    <dd>{num(m.line.operating_pressure_angle, 3)}°</dd>
  {/if}
  <dt>{t("ui.train_contact_ratio")}</dt>
  <dd>
    {#if m?.line}
      {@const r = m.line.contact_ratios}
      <span class:warn={r.transverse < 1}>
        ε<sub>α</sub> {num(r.transverse, 4)}
      </span>
      · ε<sub>β</sub>
      {num(r.overlap, 4)} · ε<sub>γ</sub>
      {num(r.total, 4)}
    {:else if m?.point}
      <span class:warn={m.contact_ratio < 1}>
        ε {num(m.contact_ratio, 4)}
      </span>
      {#if m.contact_ratio >= 1}
        <small>
          {t("ui.train_crossed_pairs_in_contact", {
            limit: t(
              m.point.limited_by === "face"
                ? "ui.train_limited_by_face_width"
                : "ui.train_limited_by_teeth",
            ),
          })}
        </small>
      {/if}
    {/if}
    {#each contactNotes(m) as n, i (i)}
      <small class="warn">{note(n)}</small>
    {/each}
  </dd>
  <dt>{t("ui.train_mesh_power_through")}</dt>
  <dd>{m ? t("ui.train_circulation_both", { forward: num(m.power_through.forward, 2), backward: num(m.power_through.backward, 2) }) : BLANK}</dd>
  <dt>{t("ui.train_mesh_efficiency")}</dt>
  <dd>
    {bothWays(m?.efficiency)}
    {#each efficiencyNotes(m) as n, i (i)}
      <small class="warn">{note(n)}</small>
    {/each}
  </dd>
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
  <!-- Every mesh has a locking threshold and a sliding at the pitch point;
       a line contact's are *never* and *zero*, and are shown where the
       shafts cross, which is where they are the numbers a designer reads. -->
  {#if m?.point}
    <dt>{t("ui.train_locks_at")}</dt>
    <dd>{locksAt(m.locking_friction)}</dd>
    <!-- At each case's own speed, since a case is a speed as well as a torque. -->
    <dt>{t("ui.train_sliding_speed")}</dt>
    <dd>
      {#each m.cases as c (c.case)}
        <span class="line">{caseName(c.case)}: {num(c.sliding_velocity, 1)} mm/s</span>
      {/each}
    </dd>
  {/if}
  <!-- The one patch both members share: same normal force, same E*, one
       instant — an ellipse on crossed shafts, a line on parallel ones, and
       the same rows either way, once per load case. Each member's own rating
       is on its card and is this or worse. -->
  <dt>{t("ui.train_contact_stress")}</dt>
  <dd>
    {#each m?.cases ?? [] as c (c.case)}
      <span class="line">
        {caseName(c.case)}: {num(c.contact.max_pressure, 1)} {t("ui.train_mpa")}
        <small>
          {t("ui.train_patch", {
            length: num(c.contact.patch_length, 4),
            width: num(c.contact.patch_width, 4),
          })} ·
          {Math.abs(c.contact.worst_position) < 1e-9
            ? t("ui.train_worst_at_pitch_point")
            : t("ui.train_worst_along_the_path", {
                position: num(c.contact.worst_position, 3),
              })}
          · {t("ui.train_pitch_point_alone_gives", {
            stress: num(c.contact.at_pitch_point, 1),
          })}
          · ρ {num(1 / c.contact.curvature_across, 3)} mm
        </small>
      </span>
    {/each}
  </dd>
  <!-- **Which conditions bite, and nothing when none do.** Drawn for **every**
       mesh. A tip reaching past the usable end of the flank it meshes with
       is the classical interference condition and belongs to any pair; it was
       asked of internal meshes under two names — trochoid and involute — and of
       external ones not at all, though a long addendum on a small pinion is
       exactly where it bites. Only the *tips crossing away from the line of
       action* is an internal pair's alone, and that row appears with the mesh
       that can have it.

       Named by member rather than by the classical pair, because the classical
       names describe which member is a ring and the condition does not. -->
  <!-- The room an internal mesh's tips have on the side away from contact
       — what sizes the distance at a few teeth of difference, and a large
       number nobody reads on an ordinary ring. -->
  {#if m?.tips}
    <dt>{t("ui.train_far_side_gap")}</dt>
    <dd>{num(m.tips.far_gap, 4)} {t("ui.train_mm")}</dd>
  {/if}
  {#if m}
    <dt>{t("ui.train_interference")}</dt>
    <dd>
      {#each [[
        m.flank_interference[0] ? t("ui.train_interference_flank", { member: members[0] }) : null,
        m.flank_interference[1] ? t("ui.train_interference_flank", { member: members[1] }) : null,
        m.tips?.tip_interference ? t("ui.train_interference_tip") : null,
      ].filter((x) => x !== null)] as fouling (0)}
        <span class:warn={fouling.length > 0}>
          {fouling.join(" · ") || t("ui.train_interference_none")}
        </span>
      {/each}
    </dd>
  {/if}
  {#if m?.point}
    <dt>{t("ui.train_contact_travel")}</dt>
    <dd>
      {num(m.point.axial_travel[0], 3)} · {num(m.point.axial_travel[1], 3)} mm
      <small>{t("ui.train_along_each_member_s_own_axis")}</small>
    </dd>
    <dt>{t("ui.train_bending_stress")}</dt>
    <dd>
      <small>{t("ui.train_not_reported_for_crossed_axes_no")}</small>
    </dd>
    <dt>{t("ui.train_flank_type")}</dt>
    <dd>
      {t("ui.train_flank_type_zi")}
      <small>{t("ui.train_zn_worm_s_contact_stress_1")}</small>
    </dd>
  {/if}
{/snippet}

{#snippet caseRows(cases: GearCase[] | undefined, carrier: string | undefined)}
  <div class="caselist">
    <table class="cases">
      <!-- Units in the headings, so a cell is a number and a column stays a
           column: "30000.0 rpm" folded in two where "30000.0" does not. -->
      <thead>
        <tr>
          <th></th>
          <th>{t("ui.train_torque")}<small>{t("ui.train_nm")}</small></th>
          <th>{t("ui.train_speed")}<small>{t("ui.train_rpm")}</small></th>
          <th>{t("ui.train_tooth_cycles")}<small>{t("ui.train_bending_contact")}</small></th>
          <th>{t("ui.train_bending_stress")}<small>{t("ui.train_mpa")}</small></th>
          <th>{t("ui.train_contact_stress")}<small>{t("ui.train_mpa")}</small></th>
          <th>{t("ui.train_min_face_width")}<small>{t("ui.train_bending_contact")} · mm</small></th>
        </tr>
      </thead>
      <tbody>
        {#each tab.train.load_cases as c, i (i)}
          {#if c.enabled}
            {@const r = forCase(cases, i)}
            <tr>
              <th>{caseName(i)}</th>
              <td>{num(r?.torque, 4)}</td>
              <td>
                {num(r?.speed, 1)}
                {#if r && carrier}
                  <small>{t("ui.train_relative_to", { speed: r.speed_against_carrier.toFixed(1), shaft: carrier })}</small>
                {/if}
              </td>
              <td>
                {r === undefined
                  ? BLANK
                  : r.cycles === null
                    ? "—"
                    : `${count(r.cycles.bending)} / ${count(r.cycles.contact)}`}
              </td>
              <td>{rated(r?.bending_stress, 1)}</td>
              <td>{rated(r?.contact_stress, 1)}</td>
              <td>{rated(r?.min_face_width.bending, 3)} / {rated(r?.min_face_width.contact, 3)}</td>
            </tr>
          {/if}
        {/each}
      </tbody>
    </table>
  </div>
{/snippet}

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
     *  are offered as toggles. `"continuity"` — a crossed gear pair, whose
     *  contact is a point no stress depends on, so nothing sizes it and the
     *  width at which contact stays continuous is reported beside the box.
     *  `"proportion"` — a worm's convention. */
    faceWidth?: "rating" | "continuity" | "proportion";
    /** The width a worm drive's convention recommends, for `"proportion"`. */
    faceRecommended?: number;
    /** Catalogue key for the face width's label — a worm's is a *length*. */
    faceLabel?: string;
    /** Catalogue key for the tooth count's label — a worm's teeth are *starts*. */
    teethLabel?: string;
    /** **Which of the stage's inputs this card's toggles argue with**, so a
     *  toggle switched here can relieve whichever of them has over-specified:
     *  the stage, and this member's index in the core's own order. Every
     *  toggle on the card — the shift, the helix, the face width, a worm's
     *  diameter — asks the same relief, since the core declares the relations
     *  and this side only says which input was just touched. */
    relief?: { stage: Stage; member: number; figures: Figure[] };
    /** **The member this gear is, on a shape**: its module and its tooth
     *  thickness coefficient are the member's rather than the stage's, since
     *  a shape's members need not all share one, and they are drawn on the
     *  card under the tooth count. A hula stage's are per mesh and it passes
     *  none. */
    member?: Member;
    /** **A worm's pitch diameter**, drawn on its card right under its starts:
     *  the same size freedom as the helix angles read as a size, which is a
     *  worm's reading and a gear's only by derivation. */
    pitchDiameter?: Auto<number>;
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
    /** **The carrier this member's teeth see**, where it has one.
     *
     *  A member of an epicyclic kind turns in its carrier's frame, and that is
     *  the speed its teeth wear at — so beside its fixed-frame speed each case
     *  says what it is against the carrier, which the core reports rather than
     *  leaving this side to subtract. The name says which shaft that is. A
     *  pair's members have none, and the two figures would be one. */
    carrier?: string;
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
    {@const cut = opts.cutter}
    <h4 class="section-heading">{t("ui.train_ring_cutter")}</h4>
    <label>
      <span>{t("ui.train_cutter_teeth")}</span>
      <input type="number" step="1" min="1" bind:value={cut.teeth} />
      <em></em>
      <FieldNote notes={notes(t("ui.gear_note_cutter_teeth"), null)} />
    </label>
    {@render numberField("ui.train_cutter_addendum", () => cut.addendum, (v) => (cut.addendum = v), 0.05, "ui.train_m")}
    {@render numberField("ui.train_cutter_tip_round", () => cut.tip_round, (v) => (cut.tip_round = v), 0.02, "ui.train_m")}
  {/if}
  <h4 class="section-heading" class:later={opts.cutter !== undefined}>{title}</h4>
  <label class:invalid={g && outside(gear.teeth, g.ranges.teeth)}>
    <span>{t(opts.teethLabel ?? "ui.train_tooth_count")}</span>
    <input type="number" step="1" bind:value={gear.teeth} />
  </label>
  {#if opts.member}
    {@const m = opts.member}
    {@render numberField("ui.train_normal_module", () => m.module, (v) => (m.module = v), 0.1, "ui.train_mm")}
    <!-- One coefficient per member rather than one per mesh with a pair
         hidden behind it: `k` thickens this gear's teeth above 1, and the two
         in a mesh ordinarily sum to 2, which keeps the mesh at zero backlash
         at its reference distance — a preset writes them so, and a designer
         who writes otherwise has asked for a thicker or thinner pair, which
         the shift sum carries as an equivalent shift. -->
    {@render numberField("ui.train_tooth_thickness_mod", () => m.thickness_mod, (v) => (m.thickness_mod = v), 0.05, "ui.train_k", t("ui.train_note_thickness_mod"))}
  {/if}
  {#if opts.pitchDiameter}
    {@render autoNumber(
      "ui.train_pitch_diameter",
      opts.pitchDiameter,
      g?.pitch_diameter,
      0.5,
      () => opts.relief && relieveStage(opts.relief.stage, "first_pitch_diameter", opts.relief.figures),
      undefined,
      "ui.train_mm",
    )}
  {/if}
  <!-- **The helix, with who decides it.** Every member's is bound to the
       others' — a pair's two by the shaft angle, a set's three by the hands
       its meshes require — so at most one member states it and the rest
       follow; none stated is the shaft angle shared evenly, or what a given
       distance or a given axial contact ratio decides. Automatic shows the
       angle the stage arrived at. -->
  {@render autoNumber(
    "ui.train_helix_angle",
    gear.helix_angle,
    g?.helix_angle,
    1,
    () => opts.relief && relieveStage(opts.relief.stage, { member: [opts.relief.member, "helix"] }, opts.relief.figures),
    undefined,
    "°",
  )}
  <!-- **The other end of the tooth, and the same shape as the shift's.** The
       addendum had an `auto` toggle whose only answer was the tallest tooth the
       tip width allows — a bound wearing a source's clothes, and one that went
       unread on every addendum a designer typed. It is the bound now, and the
       number is always the designer's.

       **A ring is not asked about its tip width**, for the reason it is not
       asked about undercut a few fields below: the bound reads the tooth a
       *rack* would leave, and a ring's form is its shaper's. Its own tip has a
       guard of its own, on the part rather than on this input. -->
  {@render boundedNumber(
    "ui.train_addendum",
    () => gear.addendum,
    (v) => (gear.addendum = v),
    0.05,
    "ui.train_m",
    undefined,
    opts.cut === "shaper"
      ? undefined
      : {
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
  {#if gear.no_sharp_tip && opts.cut !== "shaper"}
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
    () => opts.relief && relieveStage(opts.relief.stage, { member: [opts.relief.member, "shift"] }, opts.relief.figures),
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
  {#if opts.faceWidth === "proportion"}
    <!-- A worm drive's width is a **convention with a named source**, not a
         derivation, and it sizes no stress here (`crossed::proportions`); it
         is offered as the automatic value with its source beside it, and the
         box stays editable. -->
    {@render autoNumber(
      opts.faceLabel ?? "ui.train_face_width",
      gear.face_width,
      opts.faceRecommended,
      1,
      () => opts.relief && relieveStage(opts.relief.stage, { member: [opts.relief.member, "face_width"] }, opts.relief.figures),
      opts.faceRecommended === undefined
        ? null
        : t(opts.faceLabel ? "ui.train_note_worm_length" : "ui.train_note_wheel_width", {
            width: n(opts.faceRecommended),
          }),
      "ui.train_mm",
      undefined,
      clampNote(own, FIELD_NOTES.face_width),
    )}
  {:else}
    <!-- Sized by its ratings and by a given axial contact ratio on a line
         contact; by nothing on a crossed gear pair, whose contact is a point
         and whose card says so — with the width at which its contact stays
         continuous reported beside it as a figure. -->
    {@render autoNumber(
      opts.faceLabel ?? "ui.train_face_width",
      gear.face_width,
      g?.face_width,
      0.5,
      () => opts.relief && relieveStage(opts.relief.stage, { member: [opts.relief.member, "face_width"] }, opts.relief.figures),
      opts.faceWidth === "continuity"
        ? opts.faceFromContinuity === undefined
          ? t("ui.train_note_no_continuous_width")
          : t("ui.train_note_face_width_continuity", { width: n(opts.faceFromContinuity) })
        : undefined,
      "ui.train_mm",
      undefined,
      // A width nothing sizes, said under the box it stands at.
      clampNote(own, FIELD_NOTES.face_width),
    )}
  {/if}
  {#if gear.face_width.auto && (opts.faceWidth ?? "rating") === "rating"}
    <!-- Four toggles: a rating exists for every combination of what fails
         (bending or contact) and what it is rated against (the ultimate
         allowable, or fatigue). Per kind rather than per case: however many
         loads of a kind there are, the width is the largest any enabled one
         asks for. With none enabled there is nothing to invert and the width
         stands at its box, which the stage says in a note rather than hiding. -->
    <div class="subtoggles">
      <Switch
        label={t("ui.train_from_bending_ultimate")}
        on={gear.face_sources.bending.ultimate}
        set={(v) => (gear.face_sources.bending.ultimate = v)}
      />
      <Switch
        label={t("ui.train_from_bending_fatigue")}
        on={gear.face_sources.bending.fatigue}
        set={(v) => (gear.face_sources.bending.fatigue = v)}
      />
      <Switch
        label={t("ui.train_from_contact_ultimate")}
        on={gear.face_sources.contact.ultimate}
        set={(v) => (gear.face_sources.contact.ultimate = v)}
      />
      <Switch
        label={t("ui.train_from_contact_fatigue")}
        on={gear.face_sources.contact.fatigue}
        set={(v) => (gear.face_sources.contact.fatigue = v)}
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
  <!-- **What every load case does to this gear**, one row per enabled case:
       the torque it puts on it and the speed it turns at, how often it is
       loaded, the two stresses, and the width each would need. The rows are
       the train's enabled cases in the train's order and stand blank until the
       train solves, as every readout here does; a result names its case, so a
       row finds its own figures however the list was edited.

       Contact is per gear, and genuinely so. The two flanks share one pressure
       at any instant — the individual curvatures reach Hertz only through
       their sum — but the two gears are not rated at the same instant: each
       one's dedendum carries the load alone at its own end of the path, and
       that is where its pitting is assessed. -->
  {@render caseRows(g?.cases, opts.carrier)}
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
<!-- **A plain number with a label and a unit** — the third of the row family.
     `autoNumber` is one whose source can be the tool, `boundedNumber` one with a
     range on it, and this is one that is simply typed. It was the only member
     written out by hand, thirty-four times, and the shape had already drifted:
     some carried a `FieldNote` and some did not for no reason but which form
     they happened to be in.

     Bound through a getter and a setter rather than an object, because a plain
     `f64` has no object to hold an `auto` flag in — which is the same reason
     `autoNumber` can take one and this cannot. -->
<!-- **What the train asks of each of a stage's ports** — held, driven, free,
     or nothing, in which case the stage's convention stands and is named. The
     ports and their names come from the core (`topology`), so an arrangement
     with a fourth port is one more row here and no change to this file, and
     the same rows serve a pair, a set and a hula stage alike. A hold or a
     drive written here replaces the stage's convention *of that kind* on the
     stage — holding a set's carrier releases its ring — which is the core's
     rule and is read back, not repeated. -->
{#snippet shafts(i: number)}
  {@const ports = result.topology[i]?.ports ?? []}
  {#if ports.length > 0}
    <h4 class="shafts">{t("ui.train_shafts")}</h4>
    {#each ports as p (p.shaft)}
      {@const stated = constraintOn(tab.train, i, p.shaft)}
      <label>
        <span>{shaftName(tab.train, result.topology, i, p.label)}</span>
        <select
          value={stated ?? ""}
          onchange={(e) => constrain(i, p.shaft, e.currentTarget.value as Constraint | "")}
        >
          <option value=""
            >{t("ui.train_constraint_convention", { what: constraintWord(p.by_convention) })}</option
          >
          {#each ["held", "driven", "free"] as const as c (c)}
            <option value={c}>{constraintWord(c)}</option>
          {/each}
        </select>
        <em></em>
      </label>
    {/each}
  {/if}
{/snippet}

{#snippet numberField(
  key: string,
  get: () => number,
  set: (v: number) => void,
  step: number,
  /** Catalogue key for the unit, `"°"` for the degree sign, or omitted for a
   *  bare number — the same three cases every other row in this family has. */
  unit?: string,
  /** A note under the field, already rendered. Omitted draws none at all, which
   *  is different from drawing an empty one: the row keeps its height. */
  note?: string | null,
)}
  <label>
    <span>{t(key)}</span>
    <input type="number" {step} bind:value={get, set} />
    <em>{unit === "°" ? "°" : unit ? t(unit) : ""}</em>
    {#if note !== undefined}
      <FieldNote notes={notes(note ?? null, null)} />
    {/if}
  </label>
{/snippet}

{#snippet boundedNumber(
  key: string,
  get: () => number,
  set: (v: number) => void,
  step: number,
  unit: string | undefined,
  note: string | null | undefined,
  /** **Absent where the bound has nothing to bound.** A ring's form is its
   *  shaper's, so neither of the questions a rack asks — undercut at the root,
   *  a tip that keeps its width — is one a ring can be asked; `autoNumber`
   *  takes the same option for the same reason, on the other end of the tooth. */
  constraint: { label: string; title: string; on: boolean; set: (v: boolean) => void } | undefined,
  /** **A bound that actually moved this number.** Rendered in the warning
   *  colour and in front of the remark, because it is the same kind of finding
   *  the stage and mesh lists draw attention to and the reader has not got what
   *  they asked for. */
  warn?: string | null,
)}
  <!-- **No `constrained` class here**, whatever the switch does. That class
       means *two* switches and opens a fifth column for the second; this row has
       one, and the column it sits in is `auto`, so leaving it empty collapses it
       to nothing while the `1fr` name column absorbs the slack. The box and the
       unit are packed against the same right edge either way, which is what
       keeps a ring's addendum lined up with the boxes above and below it. -->
  <label class="auto">
    <span class="name">{t(key)}</span>
    <input
      type="number"
      {step}
      value={get()}
      oninput={(e) => set(e.currentTarget.valueAsNumber)}
    />
    {#if constraint}
      <span class="sw auto">
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
  {@const shown = computed === undefined ? a.manual : Number(computed.toFixed(4))}
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
      <input type="number" {step} value={shown} disabled class="computed" />
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
          // **Turning automatic off keeps the number the box was showing.**
          // `manual` is held while `auto` is on so the field has something
          // to fall back to, and `params::Auto` says seeding it from the
          // solved value is the front end's job — which it was not doing, so
          // a centre distance turned manual dropped to the stale zero it was
          // created with and the stage fell over. Seeded to the digits shown
          // rather than the full value, so what the reader saw is what they
          // now hold; the gear tab's throw and amplitude do the same.
          if (!v && a.auto) a.manual = shown;
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
<!-- **How the load is divided while two tooth pairs are engaged**, offered by
     every stage that reports a bending stress.

     Off by default and deliberately so: the ramp behind it is an uncalibrated
     placeholder rather than a stiffness model. Offered rather than hidden,
     because an estimate a designer chooses is a feature and one applied on
     their behalf is not — and one field rather than one per preset, because it
     selects a *model* and a stage running two meshes under two readings of the
     same thing would be reporting a comparison rather than a design.

     Withheld only where there is no bending stress to reach: a crossed pair
     contacts at a point, and this touches bending alone. -->
{#snippet loadSharing(stage: { load_sharing: LoadSharing })}
  <label>
    <span>{t("ui.train_load_sharing")}</span>
    <select bind:value={stage.load_sharing}>
      <option value="none">{t("ui.train_load_sharing_none")}</option>
      <option value="linear_ramp">
        {t("ui.train_load_sharing_linear_ramp")}
      </option>
    </select>
    <em></em>
    <FieldNote notes={notes(t("ui.train_note_load_sharing"), null)} />
  </label>
{/snippet}

<!-- **The axial contact ratio, as an input**, on every stage with a line
     contact. Automatic it shows what the helix and the width the mesh carries
     come to; given, it is a floor under an automatic face width, or — with
     every width given — the thing that decides the helix, and the core's
     relief keeps those readings from arguing. The mesh's finding that the
     ratio is below one is drawn here, beside the box it is about, rather than
     under a contact-ratio row that already prints the figure. -->
{#snippet overlapField(
  stage: { overlap: Auto<number> } & Stage,
  computed: number | undefined,
  meshes: (MeshReport | undefined)[],
)}
  {@render autoNumber(
    "ui.train_overlap",
    stage.overlap,
    computed,
    0.1,
    () => relieveStage(stage, "overlap", figuresOf(stage)),
    meshes
      .flatMap((m) => m?.notes ?? [])
      .filter((n) => n.key === "mesh.overlap_below_one")
      .slice(0, 1)
      .map(note)[0] ?? t("ui.train_note_overlap"),
    "",
    undefined,
    undefined,
  )}
{/snippet}

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

<header class="tab-bar">
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
{#if trains.importAdjusted}
  <p class="notice">{t("ui.train_import_adjusted")}</p>
{/if}
{#if exportError}
  <p class="error">{t("ui.train_export_failed", { reason: exportError })}</p>
{/if}

{#if confirmingDelete}
  <div class="tab-confirm" role="alertdialog">
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
    <!-- Train-wide, because it is one decision about how every gear is judged
         rather than a property of any stage or any load: a planet's root is
         loaded on both flanks whatever the load does, and a reversing duty
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
         still; every figure below is blank instead until the train solves.
         What is here is the shaft line — what no load moves. What each load
         comes to is on the load case that carries it. -->
    <dl class="out">
      <dt>{t("ui.train_total_ratio")}</dt>
      <dd>
        <!-- Signed, since a train's ratio is read off the graph and an
             external pair reverses; the magnitude decides which way round
             the two numbers are written, so a reduction reads as one
             whichever way it turns. -->
        {solved === undefined
          ? BLANK
          : Math.abs(solved.total_ratio) >= 1
            ? `${num(solved.total_ratio, 4)} : 1`
            : `1 : ${num(1 / solved.total_ratio, 4)}`}
      </dd>
      <dt>{t("ui.train_total_efficiency")}</dt>
      <dd>
        {bothWays(solved?.total_efficiency)}
        {#if lockedWays(solved?.total_efficiency)}
          <small class="warn">{lockedWays(solved?.total_efficiency)}</small>
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
    <!-- **The shaft line, from tooth counts and topology alone** — present
         whether or not the geometry solved, since a ratio needs neither a
         module nor a material. Every shaft's speed per turn of what is driven,
         exactly; and where the train is a condition short, the *family*: each
         speed as a value plus so many turns per turn of a shaft the conditions
         left free, which for a differential is the answer a designer wanted. -->
    {#if result.motion}
      {@const m = result.motion}
      <details class="shaftline" open>
        <summary class="section-heading">
          {t("ui.train_shaft_line")}
          <span class="aside">
            {t("ui.train_mobility", { mobility: String(m.mobility), given: String(m.constrained) })}
          </span>
        </summary>
        {#if m.free.length > 0}
          <p class="notice">{t("ui.train_conditions_short", { short: String(m.free.length) })}</p>
        {/if}
        <dl class="out">
          {#each m.shafts as s (portKey({ at: s.at }))}
            {#if s.at.kind !== "ground"}
              <dt>{refLabel(s.at, s.label)}</dt>
              <dd>
                {s.speed.text}{#each s.terms as term (portKey({ at: term.per }))}
                  <span class="term">+ {term.coefficient.text} × {t("ui.train_per_turn_of", { shaft: refLabel(term.per) })}</span>
                {/each}
              </dd>
            {/if}
          {/each}
        </dl>
        <small class="notice">{t("ui.train_turns_per_drive")}</small>
        {#each m.redundant as r (portKey({ at: r }))}
          <p class="notice">{t("ui.train_redundant_at", { shaft: refLabel(r) })}</p>
        {/each}
      </details>
    {/if}
    <!-- Why there is no answer at all — through the catalogue like every other
         message, naming the stage where one is to blame. -->
    {#if failure}
      <ul class="notes">
        <li class="warn">
          {failure.stage === null
            ? note(failure.note)
            : `${stageName(failure.stage - 1)}: ${note(failure.note)}`}
        </li>
      </ul>
    {/if}
  </div>
</section>

<!-- **The load cases, as the stages are: a list, each one an accordion.** A
     case is a torque at a port, held or not at the far end, judged against one
     of the two allowables — and any number of them, since a train is rated for
     every load it will see and not for two. The switch on the heading takes a
     case out of every rating without losing it, and works with the case
     collapsed; the heading itself says what the case is, so a closed one still
     reads. -->
<div class="stages">
  {#each tab.train.load_cases as c, i (i)}
    {@const cres = forCase(solved?.cases, i)}
    <section class="stage" class:off={!c.enabled}>
      <div class="casehead">
        <button class="head section-heading" onclick={() => (tab.openCases[i] = !tab.openCases[i])}>
          <span class="caret aside">{tab.openCases[i] ? "▾" : "▸"}</span>
          <strong>{caseName(i)}</strong>
          <span class="kind aside">{kindLabel(c.kind)}</span>
          <span class="teeth aside"
            >{num(c.torque, 3)} {t("ui.train_nm")} · {num(c.speed, 0)} {t("ui.train_rpm")} ·
            {t("ui.train_at_port", { port: portLabel(c.port) })}</span
          >
          {#if cres}
            <span class="eff aside"
              >{num(cres.delivered_torque, 3)} {t("ui.train_nm")}
              {t("ui.train_at_port", { port: portLabel(cres.delivered_at) })}</span
            >
          {/if}
        </button>
        <span class="control">
          <Switch label={t("ui.train_case_enabled")} on={c.enabled} set={(v) => (c.enabled = v)} />
        </span>
      </div>
      {#if tab.openCases[i]}
        <div class="body">
          <div class="grid shared">
            <!-- The kind decides which allowable the core judges against and
                 which inputs are put in front of the designer; nothing else
                 about a case knows which it is. It is chosen when the case is
                 added and shown on the heading, not switched here. -->
            <label>
              <span>{t("ui.train_case_port")}</span>
              <select value={portKey(c.port)} onchange={(e) => (c.port = portByKey(e.currentTarget.value))}>
                {#each portOptionsNow as p (p.key)}
                  <option value={p.key}>{portLabel(p.port)}</option>
                {/each}
              </select>
              <em></em>
            </label>
            <!-- Whether the far end holds the load. On, it is carried through
                 every stage to the far port; off, only a stage that cannot be
                 driven that way holds it, and a train with no such stage turns
                 under it and carries none of it. Either way a stage that locks
                 in the load's direction holds it where it stands. -->
            {@render switchField(
              "ui.train_case_reacted",
              c.reacted,
              (v) => (c.reacted = v),
              t(c.reacted ? "ui.train_note_reacted" : "ui.train_note_not_reacted"),
            )}
            {@render numberField("ui.train_torque", () => c.torque, (v) => (c.torque = v), 0.01, "ui.train_nm")}
            {@render numberField("ui.train_speed", () => c.speed, (v) => (c.speed = v), 100, "ui.train_rpm")}

            {#if c.kind === "fatigue"}
              <!-- A fatigue case alone has a duty: an ultimate load is
                   survived once and counts nothing. The sweep is measured at
                   a named port, since it is a fact about the mechanism's
                   motion and not about where its load enters. -->
              <div class="mode">
                <span>{t("ui.train_actuation")}</span>
                <div class="segmented">
                  <button class:on={dutyMode(c) === "intermittent"} onclick={() => setDuty(c, "intermittent")}>
                    {t("ui.train_intermittent")}
                  </button>
                  <button class:on={dutyMode(c) === "continuous"} onclick={() => setDuty(c, "continuous")}>
                    {t("ui.train_continuous")}
                  </button>
                </div>
              </div>
              {#if "intermittent" in c.duty}
                {@const act = c.duty.intermittent}
                <label>
                  <span>{t("ui.train_actuation_range")}</span>
                  <input type="number" step="1" bind:value={act.range_degrees} />
                  <em>°</em>
                </label>
                <label>
                  <span>{t("ui.train_actuation_range_at")}</span>
                  <select value={portKey(act.at)} onchange={(e) => (act.at = portByKey(e.currentTarget.value))}>
                    {#each portOptionsNow as p (p.key)}
                      <option value={p.key}>{portLabel(p.port)}</option>
                    {/each}
                  </select>
                  <em></em>
                </label>
                {@render numberField("ui.train_actuation_count", () => act.actuations, (v) => (act.actuations = v), 100, "")}
                <!-- It changes nothing but the cycle count and which roots are
                     loaded both ways, and the note says how — whether or not
                     it is on. -->
                {@render switchField(
                  "ui.train_reversing",
                  act.reversing,
                  (v) => (act.reversing = v),
                  t("ui.train_note_reversing"),
                )}
              {:else if "continuous" in c.duty}
                {@const cont = c.duty.continuous}
                {@render numberField("ui.train_runtime", () => cont.runtime_hours, (v) => (cont.runtime_hours = v), 100, "ui.train_hours")}
              {/if}
            {/if}
          </div>

          <!-- What this load comes to at the train level: what reaches the
               far port after every loss — and, in the notes under it, where a
               stage held it or that nothing did. The row stands while the
               train has no answer, as every readout here does. -->
          <dl class="out">
            <dt>{t("ui.train_case_delivered")}</dt>
            <dd>
              {num(cres?.delivered_torque, 4)} {cres && t("ui.train_nm")}
              {cres ? "·" : ""}
              {num(cres?.delivered_speed, 1)} {cres && t("ui.train_rpm")}
              {#if cres}
                <small>{t("ui.train_at_port", { port: portLabel(cres.delivered_at) })}</small>
              {/if}
            </dd>
          </dl>
          {#if (cres?.notes.length ?? 0) > 0}
            <ul class="notes">
              {#each cres?.notes ?? [] as n, j (j)}<li>{note(n)}</li>{/each}
            </ul>
          {/if}
          <button class="action danger" onclick={() => removeCase(i)}>{t("ui.train_remove_case")}</button>
        </div>
      {/if}
    </section>
  {/each}

  {#each CASE_KINDS as k (k.key)}
    <button class="action add" onclick={() => addCaseOfKind(k)}>{t(k.add)}</button>
  {/each}
</div>

<div class="stages">
  {#each tab.train.stages as stage, i (i)}
    {@const res = solved?.stages[i] ?? null}
    {@const figures = figuresOf(stage)}
    <section class="stage">
      {#if stage.kind === "shape"}
        <!-- **One stage, whatever it is.** A spur pair, a worm, a crossed pair
             and a planetary set are the same `Shape` — axes, the shafts on
             them, members, meshes and distances — and are drawn by the one
             block below: the shape's own inputs first, then each distance,
             then each mesh, then a card per member, then what it came to.
             What a kind used to decide is read off the shape instead: a
             worm drive is a distance marked as one, a crossed pair is an
             angle, a set is an axis carried by a shaft, a ring is a member
             with a cutter. There is no branch on a kind here because there
             is no kind in the core to branch on. -->
        {@const sres = res && res.kind === "shape" ? res : null}
        {@const worm = isWorm(stage)}
        {@const crossed = stage.distances.some((d) => d.angle !== 0)}
        {@const epicyclic = stage.axes.some((a) => a.carried_by !== null)}
        {@const replicated = stage.axes.map((a, k) => (a.count > 1 ? k : -1)).filter((k) => k >= 0)}
        {@const name = (j: number) => memberName(tab.train, result.topology, i, j)}
        {@const carriers = stage.shafts
          .map((_, s) => s + 1)
          .filter((s) => !stage.members.some((m) => m.shaft === s))}
        <button class="head section-heading" onclick={() => (tab.open[i] = !tab.open[i])}>
          <span class="caret aside">{tab.open[i] ? "▾" : "▸"}</span>
          <strong>{stageName(i)}</strong>
          {#if worm}
            <span class="kind aside">{t("ui.train_worm")}</span>
          {:else if crossed}
            <span class="kind aside">{t("ui.train_crossed")}</span>
          {:else if epicyclic}
            <span class="kind aside">{t("ui.train_planetary")}</span>
          {/if}
          <span class="teeth aside">z {stage.members.map((m) => m.gear.teeth).join(" / ")}</span>
          {#if sres}
            <span class="ratio aside">{sres.ratio.toFixed(4)} : 1</span>
            <span class="eff aside">{pct(sres.efficiency.forward)} %</span>
          {/if}
        </button>

        {#if tab.open[i]}
          <div class="body">
            <div class="grid shared">
              {@render numberField("ui.train_pressure_angle", () => stage.pressure_angle, (v) => (stage.pressure_angle = v), 0.5, "°")}
              {#if !crossed}
                {@render overlapField(stage, sres?.overlap, sres?.meshes ?? [])}
                {@render loadSharing(stage)}
              {/if}
              <!-- One search for either contact: the loss integral along a
                   line, the friction balance along a point's. -->
              {@render efficiencyToggle(stage.optimisation)}
              <!-- A replicated axis is a set of planets: how many, and how
                   close their tips may come. Asked only where there is one,
                   and once per such axis where there are more. -->
              {#each replicated as k (k)}
                <label>
                  <span>{replicated.length > 1 ? t("ui.train_planets_on", { axis: axisName(stage, i, k) }) : t("ui.train_planets")}</span>
                  <input type="number" step="1" min="1" bind:value={stage.axes[k].count} />
                  <em></em>
                </label>
              {/each}
              {#if replicated.length > 0}
                {@render numberField("ui.train_minimum_planet_clearance", () => stage.min_planet_clearance, (v) => (stage.min_planet_clearance = v), 0.05, "ui.train_mm", t("ui.train_note_planet_clearance"))}
              {/if}
              {@render shafts(i)}
            </div>

            <!-- **Each distance between two axes**, with what goes with it: the
                 angle the axes cross at, the distance and the clearance —
                 either may be the one given and the other the one derived,
                 which is what an `Auto` says and a plain number could not —
                 and the tolerance band. `Stage::relieved` is what stops both
                 being left automatic. -->
            {#each stage.distances as d, k (k)}
              {@const dres = sres?.distances[k]}
              <h4 class="mesh section-heading">
                {t("ui.train_distance_between", { a: axisName(stage, i, d.axes[0]), b: axisName(stage, i, d.axes[1]) })}
              </h4>
              <div class="grid shared">
                <label>
                  <span>{t("ui.train_axis_angle")}</span>
                  <!-- Crossing the shafts takes the axial contact ratio's box
                       away — a point contact has no overlap — so the stage is
                       relieved as after any other change, with nothing just
                       touched: the core turns a ratio that was given back to
                       automatic rather than leaving it acting unseen
                       (`docs/rationale.md#a-hidden-input-is-still-an-input`). -->
                  <input
                    type="number"
                    step="5"
                    bind:value={d.angle}
                    onchange={() => relieveStage(stage, null, figures)}
                  />
                  <em>°</em>
                  <FieldNote notes={
                    notes(
                      d.angle === 0 ? t("ui.train_note_shafts_parallel") : t("ui.train_note_shafts_crossed"),
                      null,
                    )
                  } />
                </label>
                {#if d.angle !== 0}
                  <!-- A preset's word, and an input because the recommendation
                       is one a designer takes or leaves: sized as a worm and
                       its wheel, the members get the conventional proportions
                       rather than a crossed pair's continuity width. -->
                  {@render switchField(
                    "ui.train_size_as_worm",
                    d.worm,
                    (v) => (d.worm = v),
                    t("ui.train_note_size_as_worm"),
                  )}
                {/if}
                {@render autoNumber(
                  "ui.train_c2c_distance",
                  d.distance,
                  dres?.running,
                  0.1,
                  () => relieveStage(stage, { centre_distance: k }, figures),
                  // An automatic distance the tips sized says which mesh
                  // held it open, under the number it opened to.
                  dres?.sized_by == null
                    ? undefined
                    : t("ui.train_distance_sized_by", { mesh: String(dres.sized_by + 1) }),
                  "ui.train_mm",
                )}
                <!-- The far-side tip gap an internal mesh on this distance
                     is held to — what sizes the distance at a few teeth of
                     difference. Offered where there is such a mesh, and
                     read while the distance is automatic. -->
                {#if stage.meshes.some((m) => internalOn(stage, m, k))}
                  {@render numberField("ui.train_tip_gap", () => d.tip_clearance, (v) => (d.tip_clearance = v), 0.05, "ui.train_mm", t("ui.train_note_tip_gap"))}
                {/if}
                {@render autoNumber(
                  "ui.train_c2c_clearance",
                  d.clearance,
                  dres?.clearance,
                  0.01,
                  () => relieveStage(stage, { clearance: k }, figures),
                  undefined,
                  "ui.train_mm",
                )}
                <label>
                  <span>{t("ui.train_c2c_tolerance_plus")}</span>
                  <input type="number" step="0.01" bind:value={d.tolerance_plus} />
                  <em>{t("ui.train_mm")}</em>
                </label>
                <label>
                  <span>{t("ui.train_c2c_tolerance_minus")}</span>
                  <input type="number" step="0.01" bind:value={d.tolerance_minus} />
                  <em>{t("ui.train_mm")}</em>
                </label>
                <!-- **Exposed where it is relevant, present everywhere.** Every
                     pair has an axial float on the model's side — a helical
                     gear sliding along its axis opens the flanks by `j sin β_b`
                     — and a worm's thrust bearing is where it is the dominant
                     source of backlash. Anything else leaves it at zero unseen. -->
                {#if d.worm}
                  {@render numberField("ui.train_worm_axial_clearance", () => d.axial_clearance, (v) => (d.axial_clearance = v), 0.01, "ui.train_mm")}
                {/if}
              </div>
            {/each}

            <!-- **Each mesh's own inputs**: what its flanks rub with. A set's
                 two meshes may differ, and a pair has one. -->
            {#each stage.meshes as m, k (k)}
              <h4 class="mesh section-heading">
                {t("ui.train_mesh_between", { a: name(m.a), b: name(m.b) })}
              </h4>
              <div class="grid shared">
                {@render numberField("ui.train_sliding_friction", () => m.sliding_friction, (v) => (m.sliding_friction = v), 0.01, "")}
                {@render numberField("ui.train_static_friction", () => m.static_friction, (v) => (m.static_friction = v), 0.01, "", t("ui.train_note_static_friction"))}
              </div>
            {/each}

            <!-- **One card per member**, in the shape's order. Which of the
                 shifts closes a distance is read off the toggles rather than
                 named by a control of its own: a member left automatic
                 absorbs, and the one in both meshes is preferred. So pinning
                 a planet is how a designer asks for the sun to close it
                 instead; the absorbing member is an automatic one like any
                 other, box and toggle and all. -->
            <div class="gears">
              {#each stage.members as m, j (j)}
                {@const g = sres?.members[j]}
                {@const isWormMember = worm && stage.meshes[0]?.a === j}
                {@render gearCard(name(j), m.gear, g, {
                  cut: m.ring ? "shaper" : "rack",
                  cutter: m.ring ?? undefined,
                  member: m,
                  teethLabel: isWormMember ? "ui.train_starts" : undefined,
                  relief: { stage, member: j, figures },
                  // The first member's diameter is the size freedom the helix
                  // reads as — a worm's way of stating its size.
                  pitchDiameter: isWormMember && j === 0 ? m.pitch_diameter : undefined,
                  faceWidth: worm ? "proportion" : crossed ? "continuity" : "rating",
                  faceFromContinuity: sres?.meshes[0]?.point?.face_width_for_continuity?.[j],
                  faceRecommended: g?.recommended_face_width ?? undefined,
                  faceLabel: isWormMember ? "ui.train_length" : undefined,
                  // A member on a carried axis turns in its carrier's frame,
                  // and that is the speed its teeth wear at.
                  carrier: carried(stage, j) ? t("ui.train_the_carrier") : undefined,
                })}
              {/each}
            </div>

            <!-- No centre-distance row: the distance each pair of axes runs at
                 and the clearance it runs with are the two inputs above, each
                 showing its solved value. -->
            <dl class="out">
              <dt>{t("ui.train_ratio")}</dt>
              <dd>
                {num(sres?.ratio, 4)} : 1
                <!-- What one more tooth on each member would make it: the
                     graph's exact answer, so a designer choosing counts sees
                     where a tooth tells and where it does not. -->
                {#if sres}
                  <small>{t("ui.train_ratio_per_tooth")}: {sres.ratio_per_tooth.map((r, j) => `${name(j)} ${num(r, 4)}`).join(" · ")}</small>
                {/if}
              </dd>
              {#if worm && sres}
                <dt>{t("ui.train_lead_angle")}</dt>
                <dd>
                  {num(sres.members[0].lead_angle, 4)}° · {num(sres.members[1].lead_angle, 4)}°
                  <small>{t("ui.train_lead")} {num(sres.members[0].lead, 4)} mm</small>
                </dd>
              {/if}
              <dt>{t("ui.train_efficiency")}</dt>
              <dd>
                {bothWays(sres?.efficiency)}
                {#if lockedWays(sres?.efficiency)}
                  <small class="warn">{lockedWays(sres?.efficiency)}</small>
                {/if}
              </dd>
              <!-- The power the teeth pass, as a multiple of the power in:
                   one on a pair, and where it is many the stage's loss is
                   the meshes' loss that many times over. -->
              <dt>{t("ui.train_circulation")}</dt>
              <dd>
                {sres ? t("ui.train_circulation_both", { forward: num(sres.circulation.forward, 2), backward: num(sres.circulation.backward, 2) }) : BLANK}
                <small>{t("ui.train_note_circulation")}</small>
              </dd>
              <!-- The two shafts the same two plays are seen from: driving
                   forward the play is read at the output, and driving backward
                   at the shaft that was the input. -->
              <dt>{t("ui.train_backlash")}</dt>
              <dd>
                {t("ui.train_backlash_at_output_shaft")}: {num(sres?.backlash.forward.nominal, 5)}{sres ? "°" : BLANK}
                <small>{range(num(sres?.backlash.forward.minimum, 5), num(sres?.backlash.forward.maximum, 5))}</small>
                · {t("ui.train_backlash_at_input_shaft")}: {num(sres?.backlash.backward.nominal, 5)}{sres ? "°" : BLANK}
              </dd>
              <!-- **The shafts that are not gears.** A member's card prints its
                   own speed and torque; a carrier is the one shaft a reader can
                   see nothing of, and it is regularly the input or the output. -->
              {#each carriers as s (s)}
                <dt>{shaftLabel(i, s)}</dt>
                <dd>
                  {#each sres?.cases ?? [] as sc (sc.case)}
                    <span class="line"
                      >{caseName(sc.case)}: {num(sc.speeds[s], 1)} {t("ui.train_rpm")} · {num(sc.torques[s], 4)} {t("ui.train_nm")}</span
                    >
                  {/each}
                </dd>
              {/each}
              {#each replicated as k (k)}
                {@const lay = sres?.layouts.find((l) => l.axis === k)}
                <dt>{replicated.length > 1 ? t("ui.train_planet_clearance_on", { axis: axisName(stage, i, k) }) : t("ui.train_planet_clearance")}</dt>
                <dd>
                  {#if lay}
                    {num(lay.clearance, 3)} mm
                    <small class:warn={!lay.clearance_ok}>
                      {t(lay.clearance_ok ? "ui.train_meets_the_minimum" : "ui.train_below_the_minimum")}
                    </small>
                  {/if}
                </dd>
                <!-- Two separate layout checks, so two rows. Even spacing is
                     `N | z_sun + z_ring`; simultaneous meshing is the stricter
                     `N | z_sun` *and* `N | z_ring`, and a false answer is not a
                     fault — it means the planets engage staggered, which is
                     usually preferable. -->
                <dt>{t("ui.train_even_spacing")}</dt>
                <dd>{lay?.equal_spacing == null ? BLANK : lay.equal_spacing ? t("ui.train_yes") : t("ui.train_no")}</dd>
                <dt>{t("ui.train_simultaneous_meshing")}</dt>
                <dd>{lay?.simultaneous_meshing == null ? BLANK : lay.simultaneous_meshing ? t("ui.train_yes") : t("ui.train_no")}</dd>
              {/each}
            </dl>

            <!-- Each mesh, stacked like the readout above rather than a table
                 that lines up with nothing else on the panel — and drawn by the
                 one snippet that draws every mesh, the same rows whether the
                 shafts are parallel or not. The sections stand whether or not
                 the stage solved, and their figures go blank. -->
            {#each stage.meshes as m, k (k)}
              <h4 class="mesh section-heading">{t("ui.train_mesh_between", { a: name(m.a), b: name(m.b) })}</h4>
              <dl class="out indent">
                {@render meshRows(sres?.meshes[k], [name(m.a), name(m.b)])}
              </dl>
            {/each}

            {#if (sres?.notes.length ?? 0) > 0}
              <ul class="notes">
                {#each sres?.notes ?? [] as n, i (i)}<li>{note(n)}</li>{/each}
              </ul>
            {/if}

            <button
              class="action danger"
              onclick={() => removeStage(i)}>{t("ui.train_remove_stage")}</button
            >
          </div>
        {/if}
      {/if}
    </section>
  {/each}

  <!-- One button a preset, from the table rather than by hand: a preset marked for
       the developer mode is not offered until the sidebar's title has been
       knocked on, which is the same gate the gear tab's eccentric kind is
       behind and the same table shape. -->
  {#each stagePresets as k (k.key)}
    <button class="action add" onclick={() => addStagePreset(k)}>{t(k.label)}</button>
  {/each}
</div>

<style>
  /* The bar, the delete strip and every `.action` — a stage or a case added
     or removed — are `app.css`'s, shared with the gear tab, and a `.head` is
     a heading that happens to be a button; this serves the buttons that show
     a state, and leaves those to their own rules rather than outranking them
     by being scoped (`:not()` counts toward specificity, so this would). */
  button:not(.action):not(.head) {
    font: inherit;
    font-size: 0.8rem;
    padding: 0.25rem 0.6rem;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: none;
    color: var(--fg);
    cursor: pointer;
  }
  button:not(.action):not(.head):hover:not(:disabled) {
    background: var(--hover);
  }
  button:not(.action):not(.head):disabled {
    color: var(--muted);
    cursor: default;
  }
  .danger:hover {
    border-color: var(--warn);
    color: var(--warn);
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
  .train .summary .shaftline {
    margin-top: 0.75rem;
  }
  .train .summary .shaftline summary {
    cursor: pointer;
  }
  .train .summary .shaftline .out {
    margin-top: 0.3rem;
  }
  /* A term of a family sits after the particular value, on its own line, so
     `1/7 + 6/7 × per turn of the ring` reads as one sum however long the
     shaft's name. */
  .train .summary .shaftline .term {
    display: block;
    color: var(--muted);
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
  /* The same box every other row has. These were 9 rem, on an argument
     about long field names wrapping that a narrower box does not bear on —
     the label column is what is left of the block after the box, so a
     narrower box gives a name *more* room — and a stage's row was the one
     place in the application with a box of its own width. */
  .grid.shared > label {
    grid-template-columns: 1fr var(--field-box) var(--unit-cell);
  }
  /* The `auto` toggle takes a column of its own, out of the label's share, so
     the number keeps the edge every other number in the panel shares. */
  label.auto {
    grid-template-columns: 1fr auto var(--field-box) var(--unit-cell);
  }
  .grid.shared > label.auto {
    grid-template-columns: 1fr auto var(--field-box) var(--unit-cell);
  }
  .gear label.auto {
    grid-template-columns: 1fr auto var(--field-box) var(--unit-cell);
  }
  /* A second switch takes a second column of its own, out of the label's share
     again, so the box keeps the edge every other box in the card shares. */
  label.auto.constrained {
    grid-template-columns: 1fr auto auto var(--field-box) var(--unit-cell);
  }
  .grid.shared > label.auto.constrained {
    grid-template-columns: 1fr auto auto var(--field-box) var(--unit-cell);
  }
  .gear label.auto.constrained {
    grid-template-columns: 1fr auto auto var(--field-box) var(--unit-cell);
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
    grid-template-columns: 1fr var(--field-box) var(--unit-cell);
    align-items: center;
    /* See GearPanel: the column gap spaces a row, the row gap pairs a note to
       the box above it. */
    column-gap: var(--row-gap);
    row-gap: var(--note-gap);
    font-size: 0.85rem;
  }
  /* An input's name is at full contrast, as on the gear tab; what is lower
     contrast is a note, a unit, a readout's name. The names here were muted,
     which put an input and its readout at the same weight. */
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
    grid-template-columns: 1fr auto var(--unit-cell);
    align-items: center;
    gap: var(--row-gap);
    font-size: 0.85rem;
  }
  /* The row's name is an input's name — at full contrast like every other;
     it was the one left muted when the labels changed. */
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
  .out {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.15rem 0.75rem;
    margin: 0.75rem 0 0;
    font-size: 0.85rem;
  }
  /* The shafts' rows sit under their own small heading, in the shared grid. */
  h4.shafts {
    margin: 0.75rem 0 0;
  }
  /* One mesh's readout, sitting under its heading. */
  h4.mesh {
    margin: 0.75rem 0 0;
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
  /* Size, weight and colour are `app.css`'s `.section-heading`, shared with
     the gear tab's `h2`; the figures beside the name keep their own. */
  .head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    width: 100%;
    text-align: left;
    border: none;
    border-radius: 4px;
    /* The bar keeps the height it had at 0.9 rem: the heading's face is
       0.8 rem now, and the difference goes into the padding rather than
       into a shorter bar and a smaller caret. */
    padding: 0.5rem 0.7rem;
    /* A button's own face, undone one property at a time rather than with
       the `font` shorthand, which would reset the size and weight the shared
       heading class gives it. */
    font-family: inherit;
    line-height: 1.2;
    background: none;
    color: var(--muted);
    cursor: pointer;
  }
  .head:hover {
    background: var(--hover);
  }
  .head strong {
    font-weight: inherit;
  }
  /* The band of load cases and the band of stages are two lists, and the
     second stands off from the first's add buttons. */
  .stages + .stages {
    margin-top: 1rem;
  }
  /* A load case's heading is a button and a switch side by side: the button
     opens it, the switch takes it out of every rating — and a switch cannot
     sit inside a button, so the two share a row rather than one wrapping the
     other. The button keeps the heading's own look and takes the width. */
  .casehead {
    display: flex;
    align-items: center;
    padding-right: 0.5rem;
    border-radius: 4px;
  }
  /* The whole bar lights, switch included, as a stage's heading does — the
     row is one heading with two controls on it, not a button beside a gap. */
  .casehead:hover {
    background: var(--hover);
  }
  .casehead .head {
    flex: 1;
    min-width: 0;
  }
  .casehead .head:hover {
    background: none;
  }
  /* A case switched off is still a case: its inputs stand, so it is dimmed
     rather than hidden, and its heading still says what it is. */
  .stage.off > .casehead {
    opacity: 0.55;
  }
  .caret {
    color: var(--muted);
    font-size: 0.9rem;
  }
  /* One member's ratings, a row per load case. A table rather than the
     label/figure list the rest of a card uses, because a case is one row of
     several figures and a reader compares down a column. */
  .caselist {
    overflow-x: auto;
  }
  table.cases {
    width: 100%;
    margin-top: 0.5rem;
    border-collapse: collapse;
    font-size: 0.8rem;
    font-variant-numeric: tabular-nums;
    /* A figure and its unit stay on one line; a card too narrow for every
       column scrolls the table rather than folding "30000.0 rpm" in two. */
    white-space: nowrap;
  }
  table.cases th {
    color: var(--muted);
    font-weight: normal;
    text-align: left;
    padding: 0.1rem 0.6rem 0.1rem 0;
    vertical-align: bottom;
  }
  table.cases thead th {
    border-bottom: 1px solid var(--rule);
    white-space: normal;
  }
  table.cases td {
    padding: 0.15rem 0.6rem 0.15rem 0;
    vertical-align: top;
  }
  table.cases small {
    display: block;
    color: var(--muted);
    margin-left: 0;
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
  /* Its face is `app.css`'s `.section-heading`; only the margin is its own. */
  .gear h4 {
    margin: 0 0 0.4rem;
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
    grid-template-columns: 1fr var(--field-box) 2.2rem 0.9rem;
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
  label.invalid input {
    border-color: var(--warn);
  }
  .gear label {
    grid-template-columns: 1fr var(--field-box) var(--unit-cell);
    margin-bottom: var(--field-gap);
  }
  /* A row holding words — a material, a sharing model — takes the wide box,
     as the gear tab's kind and class rows do. */
  label:has(> select),
  .gear label:has(> select),
  .grid.shared > label:has(> select) {
    grid-template-columns: 1fr var(--field-box-wide) var(--unit-cell);
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
  .notice {
    color: var(--muted);
  }
  /* An add is the same button as every other action in size and face, but
     drawn as a place where something is not yet: a dashed outline, no fill,
     the muted colour. */
  .add {
    align-self: flex-start;
    border-style: dashed;
    background: none;
    color: var(--muted);
  }
  /* A stage's or a case's remove sits below its inputs, apart from them. */
  .action.danger {
    margin-top: 0.6rem;
  }
</style>
