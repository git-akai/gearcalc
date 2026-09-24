<script lang="ts">
  import {
    defaults,
    solveTrain,
    presetsOf,
    type StagePreset,
    type Figure,
    CASE_KINDS,
    type CaseKindSpec,
    portOptions,
    isHeld,
    type CaseKind,
    type LoadCase,
    type Load,
    type LoadFreedom,
    type CaseBody,
    type BodyRole,
    type BodyReport,
    type LoadRole,
    type GearCase,
    type MeshCase,
    outside,
    type Auto,
    type Overrides,
    type MemberGear,
    type Shape,
    type Member,
    type Value,
    type GearResult,
    type Note,
    type Cutter,
    type MeshReport,
    type LoadSharing,
    type Edit,
    type Target,
    note,
    t,
  } from "./core";
  import { trains, library, type TrainTab, type Selection, type Grouping } from "./state.svelte";
  import { exportTrain, relieveCase, relieveTrain, editTrain, type Freedom } from "./core";
  import FieldNote from "./FieldNote.svelte";
  import Switch from "./Switch.svelte";
  import { notes, type Notes } from "./notes";
  import { axisOfBody, bodyName, carried, gearLabel } from "./members";
  import Offers from "./Offers.svelte";
  import { adds, type Names } from "./offers";

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
   *  `Shape::freedoms` declares them now and `relieve` walks them, so this
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

  // ------------------------------------------- the list and the workspace

  /** **The train's graph** — what the workspace stands on: a selection is
   *  by the graph's index, so every field is bound to the graph's own
   *  pieces under the graph's own numbers, and a box typed into writes the
   *  train. */
  const graph = $derived(tab.train.shape);
  /** **An input just touched, relieved**: the core's rule over the whole
   *  graph (`relieve`), the input named by the graph's index and handed the
   *  figures the train last came to, so a box relief turns given holds the
   *  number it showed. An empty list where the train has not solved, and the
   *  box keeps what it had. */
  const relieve = (just: Freedom | null) => relieveTrain(tab.train, just, result.figures);
  /** The case the flow and the workspace are shown for — the view's, held
   *  to the cases the train has. */
  const shownCase = $derived(Math.max(0, Math.min(tab.view.case, tab.train.load_cases.length - 1)));
  /** The shown case's flow, as the core walks it. */
  const flow = $derived(result.flows[shownCase] ?? []);
  /** **A gear by the graph's index**: its role and its number — "Sun (9)" —
   *  or, where the number is its name, "Gear 3" (`members.ts`, which the
   *  gear tab's adopt list reads too). */
  const gearName = (i: number): string => gearLabel(result.names, i);
  const meshName = (k: number): string => {
    const m = tab.train.shape.meshes[k];
    return m === undefined ? "" : `${gearName(m.a)} ⇄ ${gearName(m.b)}`;
  };
  /** The gears fixed to a body, by the graph's index. */
  const gearsOn = (body: number): number[] =>
    tab.train.shape.members.flatMap((m, i) => (m.body === body ? [i] : []));
  /** **The offset couplings a body is in**, each with the body it turns
   *  with — what a shaft with no gear on it is there for. */
  const couplingsOf = (body: number): { coupling: number; other: number }[] =>
    tab.train.shape.couplings.flatMap(([a, b], coupling) =>
      a === body ? [{ coupling, other: b }] : b === body ? [{ coupling, other: a }] : [],
    );
  /** What is on a body, as a row says it: its gears, or what it turns with. */
  const onBody = (body: number): string =>
    [
      ...gearsOn(body).map(gearName),
      ...couplingsOf(body).map((c) => t("ui.train_turns_with", { on: bodyName(c.other) })),
    ].join(" · ");
  const axisLabel = (a: number) => t("ui.train_axis_name", { number: String(a + 1) });
  /** **What a body does in the case shown**: its speed, and the torque a load
   *  or a reaction puts on it — the core's figures, said. */
  const bodyInCase = (body: number): string => {
    const c = forCase(solved?.cases, shownCase);
    const b = c?.solved ? c.bodies.find((x) => x.at === body) : undefined;
    if (b === undefined) return "";
    const speed = b.speed === null ? roleWord("fixed") : `${num(b.speed, 1)} ${t("ui.train_rpm")}`;
    return b.role === "load" || b.role === "reacted"
      ? `${speed} · ${roleWord(b.role)} ${num(b.torque, 4)} ${t("ui.train_nm")}`
      : speed;
  };
  /** Whether a mesh carries none of the shown case's power — which a case
   *  that did not solve says of none. */
  const idleInCase = (k: number): boolean =>
    (forCase(solved?.cases, shownCase)?.solved ?? false) &&
    (solved?.meshes[k]?.cases.find((c) => c.case === shownCase)?.power_through ?? 1) === 0;
  /** **Why the shown case does not solve**, where it does not — its own
   *  notes, which the list says above the rows it can then give no figure. */
  const shownUnsolved = $derived.by(() => {
    const c = forCase(solved?.cases, shownCase);
    return c === undefined || c.solved ? null : c.notes;
  });
  const select = (to: Selection) => (tab.view.selection = to);
  const isSelected = (s: Selection): boolean =>
    JSON.stringify(tab.view.selection) === JSON.stringify(s);
  const groupings: { key: Grouping; label: string }[] = [
    { key: "flow", label: "ui.train_grouping_flow" },
    { key: "centres", label: "ui.train_grouping_centres" },
    { key: "axes", label: "ui.train_grouping_axes" },
  ];
  /** **The path the shown case walks** — from its first load to its first
   *  reaction — where the train reports one. */
  const casePath = $derived.by(() => {
    const c = tab.train.load_cases[shownCase];
    const load = c?.loads.find((l) => l.role === "load")?.at;
    const reaction = c?.loads.find((l) => l.role === "reacted")?.at;
    return solved?.paths.find((p) => p.from === load && p.to === reaction);
  });
  /** **A part named by its meshes** — what a reader can find it by in the
   *  list — and shown by selecting its first. */
  const partName = (p: number): string => (result.parts[p]?.meshes ?? []).map(meshName).join(" · ");
  function showPart(p: number) {
    const k = result.parts[p]?.meshes[0];
    if (k !== undefined) select({ mesh: k });
  }
  /** Every note a part's own solve raised, with the part it is about. */
  const partNotes = $derived(
    (solved?.parts ?? []).flatMap((x, part) => x.notes.map((n) => ({ part, note: n }))),
  );
  /** **The parts a body is in**, each with the body's place in the part's
   *  own list of bodies (ground 0) and a name: the part's gears on the
   *  body, or the carrier's word where it has none there. */
  const partsThrough = (body: number): { part: number; slot: number; name: string }[] =>
    result.parts.flatMap((s, part) => {
      const slot = s.shape.bodies.findIndex((x) => x.body === body) + 1;
      if (slot === 0) return [];
      const on = s.members.filter((i) => tab.train.shape.members[i]?.body === body);
      return [{ part, slot, name: on.length > 0 ? on.map(gearName).join(" · ") : t("ui.train_the_carrier") }];
    });
  /** An axis distance by its two axes — "Axis 1 ↔ Axis 2". */
  const distanceName = (d: number): string => {
    const x = tab.train.shape.distances[d];
    return x === undefined ? "" : `${axisLabel(x.axes[0])} ↔ ${axisLabel(x.axes[1])}`;
  };
  /** **The names an offer is said in** — the ones the list says them in. */
  const names: Names = {
    gear: gearName,
    body: bodyName,
    axis: axisLabel,
    mesh: meshName,
    distance: distanceName,
    preset: (p) => t(defaults().stages.find((e) => e.preset === p)?.label ?? ""),
    family: (p) => {
      const family = defaults().stages.find((e) => e.preset === p)?.family;
      return t(defaults().families.find((f) => f.family === family)?.label ?? "");
    },
  };
  /** **The pieces a selection offers edits at**, each under the heading its
   *  entries go under: a mesh and its two gears; a body; an axis; a centre's
   *  distance; a coupling; a junction's planet axes. */
  const selected = $derived.by((): { target: Target; heading: string }[] => {
    const sel = tab.view.selection;
    const s = tab.train.shape;
    const at = (target: Target, piece: string) => ({ target, heading: t("ui.train_offers_at", { piece }) });
    if (sel === null) return [];
    if ("mesh" in sel) {
      const m = s.meshes[sel.mesh];
      if (m === undefined) return [];
      return [at({ mesh: sel.mesh }, meshName(sel.mesh)), at({ member: m.a }, gearName(m.a)), at({ member: m.b }, gearName(m.b))];
    }
    if ("body" in sel) return s.bodies.some((b) => b.body === sel.body) ? [at({ body: sel.body }, bodyName(sel.body))] : [];
    if ("axis" in sel) return s.axes[sel.axis] === undefined ? [] : [at({ axis: sel.axis }, axisLabel(sel.axis))];
    if ("centre" in sel) return s.distances[sel.centre] === undefined ? [] : [at({ distance: sel.centre }, distanceName(sel.centre))];
    if ("coupling" in sel) {
      return s.couplings[sel.coupling] === undefined ? [] : [at({ coupling: sel.coupling }, t("ui.train_coupling_heading"))];
    }
    if ("junction" in sel) {
      const axes = result.parts[sel.junction]?.axes ?? [];
      return axes.filter((a) => (s.axes[a]?.carried_by ?? 0) !== 0).map((a) => at({ axis: a }, axisLabel(a)));
    }
    return [];
  });
  /** What the add menu offers with nothing selected, and under what is. */
  const atOutput = $derived({ target: "train" as Target, heading: t("ui.train_offers_at_output") });
  /** **Where the reader looks after an edit**: what an add made — its first
   *  new mesh — or the body a join kept; nothing, after a removal, since
   *  what was selected has gone and the numbers after it have moved. */
  function made(edit: Edit, meshes: number) {
    if ("remove" in edit) tab.view.selection = null;
    else if ("join" in edit) tab.view.selection = { body: Math.min(edit.join.a, edit.join.b) };
    else if (adds(edit) && tab.train.shape.meshes.length > meshes) tab.view.selection = { mesh: meshes };
  }
  /** The mesh after this one along the shown case's flow, where there is one. */
  const nextAlong = (k: number): number | undefined => {
    const at = flow.findIndex((row) => "mesh" in row && row.mesh.mesh === k);
    const next = flow.slice(at + 1).find((row) => "mesh" in row);
    return at >= 0 && next !== undefined && "mesh" in next ? next.mesh.mesh : undefined;
  };


  /** Which duty a fatigue case is counted over. Switching seeds the other
   *  shape from the core's own defaults — a fresh case's intermittent duty,
   *  and the continuous one the boundary carries for exactly this — so no
   *  number is written on this side. */
  const dutyMode = (c: LoadCase): "intermittent" | "continuous" =>
    "intermittent" in c.duty ? "intermittent" : "continuous";
  function setDuty(i: number, c: LoadCase, m: "intermittent" | "continuous") {
    if (m === dutyMode(c)) return;
    editTrain(tab.train, { duty: { case: i, intermittent: m === "intermittent" } });
  }

  /** **The load cases are a list**, added one of each kind by the core,
   *  between the train's two ends, and shown as it is added. The last one
   *  may go: a train with no load case is a body line and nothing else,
   *  every rating row stands empty, and the two buttons on the strip are
   *  how one comes back. */
  function addCaseOfKind(kind: CaseKindSpec) {
    editTrain(tab.train, { add_case: kind.key });
    tab.view.case = tab.train.load_cases.length - 1;
    select({ case: tab.view.case });
  }
  /** A case removed, and the strip shows the one that takes its place —
   *  the next, or the last where it was the last. */
  function removeCase(i: number) {
    tab.train.load_cases.splice(i, 1);
    const left = tab.train.load_cases.length;
    tab.view.case = Math.max(0, Math.min(i, left - 1));
    tab.view.selection = left > 0 ? { case: tab.view.case } : null;
  }
  /** A load case by number, as a stage is; and the words for its kind and its
   *  port, from the same tables the selects offer them from. */
  const caseName = (i: number) => t("ui.train_case_heading", { number: String(i + 1) });
  const kindLabel = (k: CaseKind) => t(CASE_KINDS.find((x) => x.key === k)?.label ?? k);
  /** The ports a duty's select offers — bodies, by number, which is the
   *  select's key. */
  const portOptionsNow = $derived(portOptions(result.motion));
  /** **The train's bodies, as the core lists them** — every body of the
   *  train, each saying whether a case may address it and whether the
   *  train holds it. A case is a row per *port*: fixed where the train
   *  holds it, and otherwise a load, a reaction or free. Where the train
   *  has no motion to list them from there are no rows, and the summary
   *  says why. */
  const bodies = $derived<BodyReport[]>(
    (result.motion?.bodies ?? []).filter((b) => b.port),
  );
  /** **What the case declares a body** — the core's own four states
   *  ({@link BodyRole}), which it reports for a case that solved and this
   *  says for one that has not: fixed where the train holds it, and free
   *  where the case says nothing. */
  const roleOf = (c: LoadCase, b: BodyReport): BodyRole => {
    if (b.held) return "fixed";
    return entryOf(c, b)?.role ?? "free";
  };
  /** The case's entry for a body. */
  const entryOf = (c: LoadCase, b: BodyReport): Load | undefined => c.loads.find((l) => l.at === b.body);
  /** **A body declared a load, reacted or free.** The entry is written with
   *  both figures derived where it is new: relief never invents a given, so
   *  a load's boxes show what the case comes to, or stand blank until the
   *  designer gives one. The figures are kept while the body is reacted or
   *  free, and the core relieves what remains. */
  function setRole(i: number, b: BodyReport, role: LoadRole) {
    const c = tab.train.load_cases[i];
    const entry = entryOf(c, b);
    if (entry) {
      if (entry.role === role) return;
      entry.role = role;
    } else {
      c.loads.push({ at: b.body, role, torque: { auto: true, manual: 0 }, speed: { auto: true, manual: 0 } });
    }
    relieveCase(tab.train, i, null, ratedUnder());
  }
  /** The library the train is rated under, which relief seeds from too. */
  const ratedUnder = () => (library.origin === null ? undefined : library.materials);
  /** A figure of a load toggled: relief keeps this one and turns another. */
  const touched = (i: number, load: Load, which: LoadFreedom) => () => {
    const j = tab.train.load_cases[i].loads.indexOf(load);
    relieveCase(tab.train, i, j < 0 ? null : { load: j, which }, ratedUnder());
  };
  /** What the case comes to at a port: its row of the train-level result,
   *  which a derived box shows and a blank stands for where the case did
   *  not solve. */
  const bodyOf = (cres: { bodies: CaseBody[]; solved: boolean } | undefined, at: number) =>
    cres?.solved ? cres.bodies.find((s) => s.at === at) : undefined;
  /** **What a case comes to, body by body**: the frame first, then every
   *  body a case can name in the chain's order, then every body that is no
   *  port, a planet's, on its own. Nothing is computed here: each row is
   *  one of the core's rows, chosen. */
  const delivered = (
    cres: { bodies: CaseBody[] } | undefined,
  ): { key: number; name: string; role: BodyRole; speed: number | null; torque: number }[] => {
    if (!cres) return [];
    const row = (s: CaseBody) => ({ key: s.at, name: bodyName(s.at), role: s.role, speed: s.speed, torque: s.torque });
    const first = [0, ...bodies.map((b) => b.body)];
    const lead = first.map((at) => cres.bodies.find((s) => s.at === at)).filter((s) => s !== undefined);
    const rest = cres.bodies.filter((s) => !first.includes(s.at));
    return [...lead, ...rest].map(row);
  };
  const roleWord = (r: BodyRole) =>
    t(
      {
        load: "ui.train_role_load",
        reacted: "ui.train_role_reacted",
        fixed: "ui.train_role_fixed",
        free: "ui.train_role_free",
      }[r],
    );
  /** The heading's summary of a case: each given figure at its port —
   *  none while the train has no stages and the entries are parked. */
  const caseSummary = (c: LoadCase): string =>
    tab.train.shape.members.length === 0 ? "" : c.loads
      .filter((l) => l.role === "load")
      .map((l) => {
        const parts: string[] = [];
        if (!l.torque.auto) parts.push(`${num(l.torque.manual, 3)} ${t("ui.train_nm")}`);
        if (!l.speed.auto) parts.push(`${num(l.speed.manual, 0)} ${t("ui.train_rpm")}`);
        return parts.length ? `${parts.join(" · ")} ${t("ui.train_at_port", { port: bodyName(l.at) })}` : "";
      })
      .filter((s) => s !== "")
      .join(" · ");
  /** **Every result names its case**, so a readout that stands for a case looks
   *  its figures up by that index rather than by position: a case switched off
   *  has no result and its row draws blank, and the rows are the train's cases
   *  in the train's order either way. */
  const forCase = <T extends { case: number }>(list: T[] | undefined, i: number) =>
    list?.find((c) => c.case === i);

  /** The axis a member turns about; whether a carrier carries it is
   *  `members.ts`'s `carried`, the one reading the gear tab shares. */
  const axisOf = (shape: Shape, j: number) => axisOfBody(shape, shape.members[j].body);
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
  /** **A box cleared keeps the number it held.** A number input with its
   *  text deleted binds `null`, which the core refuses as a train — an
   *  `f64` cannot be nothing — and a load on a named body then vanished
   *  from the card with the motion it was listed under, unfixable but by
   *  deleting the case. The box may stand blank while the designer types;
   *  the train keeps its last number until a new one is there. Not a
   *  default: no number is written here that was not already in the box. */
  const finite = (set: (v: number) => void) => (v: number | null) => {
    if (v !== null && Number.isFinite(v)) set(v);
  };
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
  /** Whether a mesh is internal and on distance `k` of a shape: one of
   *  its members has a cutter, and its two members' axes are the distance's. */
  const internalOn = (shape: Shape, m: { a: number; b: number }, k: number): boolean => {
    const [a, b] = [axisOf(shape, m.a), axisOf(shape, m.b)];
    const d = shape.distances[k];
    const on = (d.axes[0] === a && d.axes[1] === b) || (d.axes[0] === b && d.axes[1] === a);
    return on && (shape.members[m.a].ring !== null) !== (shape.members[m.b].ring !== null);
  };
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
{#snippet flowList()}
  {#each flow as row, r (r)}
    {#if "body" in row}
      {@const b = row.body.body}
      <button class="fb" class:sel={isSelected({ body: b })} onclick={() => select({ body: b })}>
        <span class="name">{bodyName(b)}</span>
        <span class="on">{onBody(b)}</span>
        <span class="fig">{bodyInCase(b)}</span>
      </button>
    {:else if "mesh" in row}
      {@const k = row.mesh.mesh}
      <button class="fm" class:sel={isSelected({ mesh: k })} onclick={() => select({ mesh: k })}>
        <span class="arrow">↓</span> {meshName(k)}
      </button>
    {:else if "idle" in row}
      {@const k = row.idle.mesh}
      <button class="fm idle" class:sel={isSelected({ mesh: k })} onclick={() => select({ mesh: k })}>
        <span class="arrow">↳</span> {meshName(k)}
        <small>{t("ui.train_idle_to", { body: bodyName(row.idle.to) })} · {bodyInCase(row.idle.to)}</small>
      </button>
    {:else if "junction" in row}
      {@const j = row.junction}
      {@const axes = (result.parts[j.part]?.axes ?? []).map((a) => result.groupings.axes[a]).filter((a) => a !== undefined)}
      <div class="junction" class:sel={isSelected({ junction: j.part })}>
        <button class="fm" onclick={() => select({ junction: j.part })}>
          <span class="arrow">↓</span> {j.meshes.map(meshName).join(" · ")}
        </button>
        {#each axes.filter((a) => a.carried_by !== null) as a (a.axis)}
          {#each a.bodies as b (b.body)}
            <small class="line">{t("ui.train_junction_planets", { body: bodyName(b.body), count: String(a.count) })} · {bodyInCase(b.body)}</small>
          {/each}
        {/each}
        {#each j.terminals.filter((tb) => !flow.some((x) => "body" in x && x.body.body === tb)) as tb (tb)}
          {@const on = onBody(tb)}
          <small class="line">
            {#if axes.some((a) => a.carried_by === tb)}
              {t("ui.train_junction_carrier", { body: bodyName(tb) })} · {bodyInCase(tb)}
            {:else if isHeld(tab.train, tb)}
              {t("ui.train_junction_fixed", { on, body: bodyName(tb) })}
            {:else}
              {t("ui.train_junction_end", { on, body: bodyName(tb) })} · {bodyInCase(tb)}
            {/if}
          </small>
        {/each}
      </div>
    {:else if "coupling" in row}
      {@const c = row.coupling.coupling}
      <button class="fm" class:sel={isSelected({ coupling: c })} onclick={() => select({ coupling: c })}>
        <span class="arrow">↔</span> {t("ui.train_turns_with", { on: bodyName(row.coupling.to) })}
      </button>
    {/if}
  {/each}
{/snippet}

{#snippet centresList()}
  {#each result.groupings.centres as c (c.distance)}
    <button class="cen" class:sel={isSelected({ centre: c.distance })} onclick={() => select({ centre: c.distance })}>
      <span class="name">{axisLabel(c.axes[0])} ↔ {axisLabel(c.axes[1])}</span>
      <span class="fig">{num(solved?.distances[c.distance]?.running, 4)} {t("ui.train_mm")}</span>
    </button>
    {#each c.meshes as k (k)}
      <button class="gearrow" class:sel={isSelected({ mesh: k })} onclick={() => select({ mesh: k })}>
        {meshName(k)}
        {#if idleInCase(k)}<span class="chip">{t("ui.train_idle")}</span>{/if}
      </button>
    {/each}
  {/each}
{/snippet}

{#snippet axesList()}
  {#each result.groupings.axes as a (a.axis)}
    <div class="axisrow">
      <button class="axisname" class:sel={isSelected({ axis: a.axis })} onclick={() => select({ axis: a.axis })}>{axisLabel(a.axis)}</button>
      {#if a.carried_by !== null}<span class="chip">{t("ui.train_carried_by_body", { body: bodyName(a.carried_by) })}</span>{/if}
      {#if a.count > 1}<span class="chip">{t("ui.train_axis_count", { count: String(a.count) })}</span>{/if}
      <span class="rule"></span>
    </div>
    {#each a.bodies as b (b.body)}
      <button class="bodyrow" class:sel={isSelected({ body: b.body })} onclick={() => select({ body: b.body })}>
        <span class="name">{bodyName(b.body)}</span>
        {#if isHeld(tab.train, b.body)}<span class="chip held">{t("ui.train_case_fixed")}</span>{/if}
      </button>
      {#each b.members as i (i)}
        {@const first = tab.train.shape.meshes.findIndex((m) => m.a === i || m.b === i)}
        <button class="gearrow" class:sel={first >= 0 && isSelected({ mesh: first })} onclick={() => first >= 0 && select({ mesh: first })}>
          {gearName(i)} <span class="z">z {tab.train.shape.members[i].gear.teeth}</span>
        </button>
      {/each}
      {#if b.members.length === 0 && b.carries.length > 0}
        <small class="gearrow dim">{t("ui.train_the_carrier")}</small>
      {/if}
      {#each couplingsOf(b.body) as c (c.coupling)}
        <button class="gearrow" class:sel={isSelected({ coupling: c.coupling })} onclick={() => select({ coupling: c.coupling })}>
          ↔ {t("ui.train_turns_with", { on: bodyName(c.other) })}
        </button>
      {/each}
    {/each}
  {/each}
{/snippet}

{#snippet workspaceOf(sel: Selection | null)}
  {#if sel !== null && "mesh" in sel && tab.train.shape.meshes[sel.mesh] !== undefined}
    {@render meshWorkspace(sel.mesh)}
  {:else if sel !== null && "centre" in sel && tab.train.shape.distances[sel.centre] !== undefined}
    {@const d = graph.distances[sel.centre]}
    <h4 class="section-heading">{t("ui.train_distance_between", { a: axisLabel(d.axes[0]), b: axisLabel(d.axes[1]) })}</h4>
    {@render distanceFields(sel.centre)}
    {#each result.groupings.centres.find((c) => c.distance === sel.centre)?.meshes ?? [] as k (k)}
      <button class="gearrow" onclick={() => select({ mesh: k })}>{meshName(k)}</button>
    {/each}
  {:else if sel !== null && "body" in sel && tab.train.shape.bodies.some((b) => b.body === sel.body)}
    {@render bodyWorkspace(sel.body)}
  {:else if sel !== null && "axis" in sel && tab.train.shape.axes[sel.axis] !== undefined}
    {@render axisWorkspace(sel.axis)}
  {:else if sel !== null && "case" in sel && tab.train.load_cases[sel.case] !== undefined}
    {@const c = tab.train.load_cases[sel.case]}
    {@const complete = forCase(solved?.cases, sel.case)?.solved ?? false}
    <div class="ws-head">
      <h4 class="section-heading">{caseName(sel.case)} · {kindLabel(c.kind)}</h4>
      <small>{caseSummary(c)}</small>
      <span class="control" class:locked={solved !== undefined && !complete}>
        <Switch
          label={t("ui.train_case_enabled")}
          on={c.enabled}
          set={(v) => {
            if (v && solved !== undefined && !complete) return;
            c.enabled = v;
          }}
        />
      </span>
    </div>
    <div class="casebody">{@render caseEditor(c, sel.case)}</div>
  {:else if sel !== null && "coupling" in sel && tab.train.shape.couplings[sel.coupling] !== undefined}
    {@const [from, to] = tab.train.shape.couplings[sel.coupling]}
    <h4 class="section-heading">{t("ui.train_coupling_heading")}</h4>
    <p class="hint">{t("ui.train_coupling_between", { a: bodyName(from), b: bodyName(to) })}</p>
  {:else if sel !== null && "junction" in sel && result.parts[sel.junction] !== undefined}
    {@const part = result.parts[sel.junction]}
    {#each part.meshes as k (k)}
      <button class="gearrow" onclick={() => select({ mesh: k })}>{meshName(k)}</button>
    {/each}
  {:else}
    <p class="hint">{t("ui.train_select_a_piece")}</p>
  {/if}
{/snippet}

{#snippet meshWorkspace(k: number)}
  {@const m = graph.meshes[k]}
  {@const d = result.groupings.centres.find((c) => c.meshes.includes(k))?.distance}
  {@const dist = d === undefined ? undefined : graph.distances[d]}
  {@const crossed = (dist?.angle ?? 0) !== 0}
  {@const worm = dist?.worm ?? false}
  {@const group = result.mesh_groups.find((g) => g.includes(m.a)) ?? [m.a, m.b]}
  {@const groupMeshes = graph.meshes.flatMap((x, kk) => (group.includes(x.a) && group.includes(x.b) ? [kk] : []))}
  {@const next = nextAlong(k)}
  <div class="ws-head">
    <h4 class="section-heading">{t("ui.train_mesh_heading", { a: gearName(m.a), b: gearName(m.b) })}</h4>
    <small>
      {t("ui.train_gear_on_body", { gear: gearName(m.a), body: bodyName(graph.members[m.a].body) })}
      · {t("ui.train_gear_on_body", { gear: gearName(m.b), body: bodyName(graph.members[m.b].body) })}
    </small>
    {#if next !== undefined}
      <button class="link" onclick={() => select({ mesh: next })}>{t("ui.train_next_along_flow", { mesh: meshName(next) })} →</button>
    {/if}
  </div>
  <div class="ws">
    <div class="col first">{@render gearColumn(m.a, k, crossed, worm)}</div>
    <div class="col meshcol">
      <h4 class="section-heading">{t("ui.train_mesh_column")}</h4>
      <div class="grid shared">
        {@render numberField("ui.train_sliding_friction", () => m.sliding_friction, (v) => (m.sliding_friction = v), 0.01, "")}
        {@render numberField("ui.train_static_friction", () => m.static_friction, (v) => (m.static_friction = v), 0.01, "", t("ui.train_note_static_friction"))}
        {#if !crossed}
          {@render loadSharing(m)}
        {/if}
        {@render searchToggle(m)}
        <!-- The floor the efficiency search holds this mesh to, offered
             while the search is on. -->
        {#if m.search}
          {@render numberField("ui.train_min_contact_ratio", () => m.min_contact_ratio, (v) => (m.min_contact_ratio = v), 0.05, "ui.train_epsilon", t("ui.train_note_min_contact_ratio"))}
        {/if}
        {#if !crossed}
          {@render overlapField(graph, groupMeshes, solved?.meshes ?? [])}
        {/if}
      </div>
      {#if dist !== undefined && d !== undefined}
        <h4 class="section-heading">{t("ui.train_distance_between", { a: axisLabel(dist.axes[0]), b: axisLabel(dist.axes[1]) })}</h4>
        {@render distanceFields(d)}
      {/if}
    </div>
    <div class="col second">{@render gearColumn(m.b, k, crossed, worm)}</div>
  </div>
  <!-- **What the mesh comes to, running down** under the three columns,
       at the workspace's width: a label and a figure per row. -->
  <h4 class="section-heading">{t("ui.train_what_mesh_comes_to")}</h4>
  <dl class="out comes">
    {@render meshRows(solved?.meshes[k], [gearName(m.a), gearName(m.b)])}
    {#if worm && solved}
      <dt>{t("ui.train_lead_angle")}</dt>
      <dd>
        {num(solved.members[m.a]?.lead_angle, 4)}° · {num(solved.members[m.b]?.lead_angle, 4)}°
        <small>{t("ui.train_lead")} {num(solved.members[m.a]?.lead, 4)} {t("ui.train_mm")}</small>
      </dd>
    {/if}
    <!-- **What one more tooth on either gear makes the shown case's path**
         — the graph's exact answer, so a designer choosing counts sees
         where a tooth tells and where it does not, and where it locks the
         path, which a gear of another part can. -->
    {#if casePath}
      <dt>{t("ui.train_ratio_per_tooth")}</dt>
      <dd>
        {#each [m.a, m.b] as i (i)}
          {@const r = casePath.per_tooth[i]}
          <span class="line">{gearName(i)}: {r == null ? t("ui.train_ratio_per_tooth_locked") : num(r, 4)}</span>
        {/each}
      </dd>
    {/if}
  </dl>
{/snippet}

{#snippet gearColumn(i: number, k: number, crossed: boolean, worm: boolean)}
  {@const mem = graph.members[i]}
  {@const g = solved?.members[i]}
  {@const isWormMember = result.names[i]?.role === "worm"}
  {@render gearCard(gearName(i), mem.gear, g, {
    cut: mem.ring ? "shaper" : "rack",
    cutter: mem.ring ?? undefined,
    member: mem,
    teethLabel: isWormMember ? "ui.train_starts" : undefined,
    relief: i,
    pitchDiameter: isWormMember ? mem.pitch_diameter : undefined,
    faceWidth: worm ? "proportion" : crossed ? "continuity" : "rating",
    faceFromContinuity: solved?.meshes[k]?.point?.face_width_for_continuity?.[graph.meshes[k].a === i ? 0 : 1],
    faceRecommended: g?.recommended_face_width ?? undefined,
    faceLabel: isWormMember ? "ui.train_length" : undefined,
    carrier: carried(graph, i) ? t("ui.train_the_carrier") : undefined,
  })}
{/snippet}

{#snippet distanceFields(d: number)}
  {@const dist = graph.distances[d]}
  {@const dres = solved?.distances[d] ?? undefined}
  <div class="grid shared">
    <label>
      <span>{t("ui.train_axis_angle")}</span>
      <input
        type="number"
        step="5"
        bind:value={() => dist.angle, finite((v) => (dist.angle = v))}
        onchange={() => relieve(null)}
      />
      <em>°</em>
      <FieldNote notes={notes(dist.angle === 0 ? t("ui.train_note_axes_parallel") : t("ui.train_note_axes_crossed"), null)} />
    </label>
    {#if dist.angle !== 0}
      {@render switchField("ui.train_size_as_worm", dist.worm, (v) => (dist.worm = v), t("ui.train_note_size_as_worm"))}
    {/if}
    {@render autoNumber(
      "ui.train_distance",
      dist.distance,
      dres?.running,
      0.1,
      () => relieve({ distance: d }),
      dres?.sized_by == null ? undefined : t("ui.train_distance_sized_by", { mesh: meshName(dres.sized_by) }),
      "ui.train_mm",
    )}
    {#if graph.meshes.some((m) => internalOn(graph, m, d))}
      {@render numberField("ui.train_tip_gap", () => dist.tip_clearance, (v) => (dist.tip_clearance = v), 0.05, "ui.train_mm", t("ui.train_note_tip_gap"))}
    {/if}
    {@render autoNumber(
      "ui.train_distance_clearance",
      dist.clearance,
      dres?.clearance,
      0.01,
      () => relieve({ clearance: d }),
      undefined,
      "ui.train_mm",
    )}
    {@render numberField("ui.train_distance_tolerance_plus", () => dist.tolerance_plus, (v) => (dist.tolerance_plus = v), 0.01, "ui.train_mm")}
    {@render numberField("ui.train_distance_tolerance_minus", () => dist.tolerance_minus, (v) => (dist.tolerance_minus = v), 0.01, "ui.train_mm")}
    {#if dist.worm}
      {@render numberField("ui.train_worm_axial_clearance", () => dist.axial_clearance, (v) => (dist.axial_clearance = v), 0.01, "ui.train_mm")}
    {/if}
  </div>
{/snippet}

{#snippet bodyWorkspace(b: number)}
  <h4 class="section-heading">
    {bodyName(b)}
    {#if isHeld(tab.train, b)}<span class="chip held">{t("ui.train_case_fixed")}</span>{/if}
  </h4>
  {#each gearsOn(b) as i (i)}
    {@const first = tab.train.shape.meshes.findIndex((m) => m.a === i || m.b === i)}
    <button class="gearrow" onclick={() => first >= 0 && select({ mesh: first })}>{gearName(i)}</button>
  {/each}
  {#each couplingsOf(b) as c (c.coupling)}
    <button class="gearrow" onclick={() => select({ coupling: c.coupling })}>↔ {t("ui.train_turns_with", { on: bodyName(c.other) })}</button>
  {/each}
  {@const through = partsThrough(b)}
  <dl class="out">
    {#each solved?.cases ?? [] as c (c.case)}
      {@const x = c.bodies.find((y) => y.at === b)}
      {#if x}
        <dt>{caseName(c.case)}</dt>
        <dd>
          {x.speed === null ? roleWord("fixed") : `${num(x.speed, 1)} ${t("ui.train_rpm")}`} · {roleWord(x.role)} {num(x.torque, 4)} {t("ui.train_nm")}
          <!-- **What each part's meshes put on it**, where the train's
               figure does not say it: a shaft two parts share hands the
               one's torque to the other and carries no load of the case's
               own, and a carrier has no gear to read it off. -->
          {#if through.length > 1 || gearsOn(b).length === 0}
            {#each through as x2 (x2.part)}
              {@const sc = solved?.parts[x2.part]?.cases.find((y) => y.case === c.case)}
              <span class="line">{x2.name}: {num(sc?.torques[x2.slot], 4)} {t("ui.train_nm")}</span>
            {/each}
          {/if}
        </dd>
      {/if}
    {/each}
  </dl>
{/snippet}

{#snippet axisWorkspace(a: number)}
  {@const axis = graph.axes[a]}
  {@const group = result.groupings.axes[a]}
  <h4 class="section-heading">{axisLabel(a)}</h4>
  {#if group?.carried_by != null}
    <div class="grid shared">
      <label>
        <span>{t("ui.train_planets")}</span>
        <input type="number" step="1" min="1" bind:value={() => axis.count, finite((v) => (axis.count = v))} />
        <em></em>
      </label>
      {#if axis.count > 1}
        {@render numberField("ui.train_minimum_planet_clearance", () => axis.min_planet_clearance, (v) => (axis.min_planet_clearance = v), 0.05, "ui.train_mm", t("ui.train_note_planet_clearance"))}
      {/if}
    </div>
  {/if}
  {#each group?.bodies ?? [] as b (b.body)}
    <button class="bodyrow" onclick={() => select({ body: b.body })}><span class="name">{bodyName(b.body)}</span></button>
    {#each b.members as i (i)}
      <span class="gearrow">{gearName(i)}</span>
    {/each}
  {/each}
  <!-- **How its planets lay out**, where it is replicated: the least tip
       gap between neighbours against the one asked for; and two checks —
       even spacing is `N | z_sun + z_ring`, simultaneous meshing the
       stricter `N | z_sun` *and* `N | z_ring`, and a false answer to the
       second is not a fault: the planets engage staggered, which is
       usually preferable. -->
  {@const lay = solved?.axes[a]?.layout ?? null}
  {#if lay !== null}
    <dl class="out">
      <dt>{t("ui.train_planet_clearance")}</dt>
      <dd>
        {num(lay.clearance, 3)} {t("ui.train_mm")}
        <small class:warn={!lay.clearance_ok}>{t(lay.clearance_ok ? "ui.train_meets_the_minimum" : "ui.train_below_the_minimum")}</small>
      </dd>
      <dt>{t("ui.train_even_spacing")}</dt>
      <dd>{lay.equal_spacing === null ? BLANK : lay.equal_spacing ? t("ui.train_yes") : t("ui.train_no")}</dd>
      <dt>{t("ui.train_simultaneous_meshing")}</dt>
      <dd>{lay.simultaneous_meshing === null ? BLANK : lay.simultaneous_meshing ? t("ui.train_yes") : t("ui.train_no")}</dd>
    </dl>
  {/if}
{/snippet}

{#snippet caseEditor(c: LoadCase, i: number)}
  {@const cres = forCase(solved?.cases, i)}
    <div class="grid shared">
      {#if c.kind === "fatigue"}
        <!-- **What is the case's, before what is each load's.** A
             fatigue case alone has a duty: an ultimate load is survived
             once and counts nothing. The sweep is measured at a named
             shaft, since it is a fact about the mechanism's motion and
             not about where its load enters. -->
        <div class="mode">
          <span>{t("ui.train_actuation")}</span>
          <div class="segmented">
            <button class:on={dutyMode(c) === "intermittent"} onclick={() => setDuty(i, c, "intermittent")}>
              {t("ui.train_intermittent")}
            </button>
            <button class:on={dutyMode(c) === "continuous"} onclick={() => setDuty(i, c, "continuous")}>
              {t("ui.train_continuous")}
            </button>
          </div>
        </div>
        {#if "intermittent" in c.duty}
          {@const act = c.duty.intermittent}
          <label>
            <span>{t("ui.train_actuation_range")}</span>
            <input type="number" step="1" bind:value={() => act.range_degrees, finite((v) => (act.range_degrees = v))} />
            <em>°</em>
          </label>
          <label>
            <span>{t("ui.train_actuation_range_at")}</span>
            <select value={String(act.at)} onchange={(e) => (act.at = Number(e.currentTarget.value))}>
              {#each portOptionsNow as p (p.body)}
                <option value={String(p.body)}>{bodyName(p.body)}</option>
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

      <!-- **One row per body of the train**, in the rows every other
           input sits in. A body the train holds is fixed, and no case
           can say otherwise. Every other body is what the case
           declares it: a load carries a torque and a speed, each
           given or derived — of the speeds exactly the train's
           mobility given, of the torques one statics equation fewer
           than the bodies that carry one, which the core keeps so
           through relief after every toggle; a reacted body turns as
           the motion says and carries whatever the flow puts on it;
           a free one turns and carries nothing. The chain's ends are
           reacted and everything else free until the case says so. A
           body two stages share has an end on each, and cannot be a
           reaction — a second reaction on one chain is a division by
           stiffness the core refuses — so it is a load, an inline
           take-off, or free. A derived box shows what the case comes
           to and stands blank until it can. -->
      {#each bodies as b (b.body)}
        {@const role = roleOf(c, b)}
        {@const load = role === "load" ? entryOf(c, b) : undefined}
        {@const at = bodyOf(cres, b.body)}
        {@const shared = b.ends.length > 1}
        <div class="mode" class:later={c.kind === "fatigue" || b !== bodies[0]}>
          <span>{bodyName(b.body)}</span>
          {#if role === "fixed"}
            <div class="segmented locked">
              <button class="on" disabled>{t("ui.train_case_fixed")}</button>
            </div>
          {:else}
            <div class="segmented">
              <button class:on={role === "load"} onclick={() => setRole(i, b, "load")}>
                {t("ui.train_case_load")}
              </button>
              <button
                class:on={role === "reacted"}
                disabled={shared}
                title={shared ? t("ui.train_note_shared_not_reacted") : undefined}
                onclick={() => setRole(i, b, "reacted")}
              >
                {t("ui.train_case_reacted")}
              </button>
              <button class:on={role === "free"} onclick={() => setRole(i, b, "free")}>
                {t("ui.train_case_free")}
              </button>
            </div>
          {/if}
        </div>
        {#if load}
          {@render autoNumber(
            "ui.train_torque",
            load.torque,
            load.torque.auto ? (at?.torque ?? null) : undefined,
            0.01,
            touched(i, load, "torque"),
            undefined,
            "ui.train_nm",
            undefined,
            undefined,
            true,
          )}
          {@render autoNumber(
            "ui.train_speed",
            load.speed,
            load.speed.auto ? (at?.speed ?? null) : undefined,
            100,
            touched(i, load, "speed"),
            undefined,
            "ui.train_rpm",
            undefined,
            undefined,
            true,
          )}
        {/if}
      {/each}
    </div>

    <!-- **What this case comes to, body by body**, in the table a
         gear's ratings use: every body of the train and the frame,
         what it is in this case — a load, a reaction, fixed, or free —
         and what it turns at and carries. A fixed body has no speed to
         report and shows none. The table stands while the train has no
         answer, as every readout does; the notes under it say why a
         case did not solve. -->
    <div class="delivered">
      <h4 class="section-heading">{t("ui.train_case_delivered")}</h4>
      <div class="caselist">
        <table class="cases">
          <thead>
            <tr>
              <th></th>
              <th></th>
              <th>{t("ui.train_speed")}<small>{t("ui.train_rpm")}</small></th>
              <th>{t("ui.train_torque")}<small>{t("ui.train_nm")}</small></th>
            </tr>
          </thead>
          <tbody>
            {#each delivered(cres) as row (row.key)}
              <tr class:muted={row.role === "free"}>
                <th>{row.name}</th>
                <td class="role">{roleWord(row.role)}</td>
                <td>{row.speed === null ? "—" : num(row.speed, 1)}</td>
                <td>{num(row.torque, 4)}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </div>
      {#if (cres?.notes.length ?? 0) > 0}
        <ul class="notes">
          {#each cres?.notes ?? [] as n, j (j)}<li class="warn">{note(n)}</li>{/each}
        </ul>
      {/if}
    </div>
    <button class="action danger" onclick={() => removeCase(i)}>{t("ui.train_remove_case")}</button>
{/snippet}

{#snippet property(
  label: string,
  gear: MemberGear,
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
{#snippet switchField(
  key: string,
  on: boolean,
  set: (v: boolean) => void,
  note?: string | null,
  /** What the label's key names, where it names something. */
  args?: Record<string, string>,
)}
  <div class="switchrow">
    <span class="control"><Switch label={t(key, args)} {on} {set} /></span>
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
  <!-- The power crossing the mesh over the power into the train, in each
       case, from the train's own flow: a mesh a case leaves unloaded
       passes nothing. -->
  <dt>{t("ui.train_mesh_power_through")}</dt>
  <dd>
    {#each m?.cases ?? [] as c (c.case)}
      <span class="line">{caseName(c.case)}: {num(c.power_through, 2)}×</span>
    {/each}
  </dd>
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
                  <small>{t("ui.train_relative_to", { speed: r.speed_against_carrier.toFixed(1), body: carrier })}</small>
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
  gear: MemberGear,
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
    /** **Which member this card's toggles argue as**, by the graph's index,
     *  so a toggle switched here can relieve whichever input it has
     *  over-specified. Every toggle on the card — the shift, the helix, the
     *  face width, a worm's diameter — asks the same relief, since the core
     *  declares the relations and this side only says which input was just
     *  touched. */
    relief?: number;
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
     *  leaving this side to subtract. The name says which body that is. A
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
      <input type="number" step="1" min="1" bind:value={() => cut.teeth, finite((v) => (cut.teeth = v))} />
      <em></em>
      <FieldNote notes={notes(t("ui.gear_note_cutter_teeth"), null)} />
    </label>
    {@render numberField("ui.train_cutter_addendum", () => cut.addendum, (v) => (cut.addendum = v), 0.05, "ui.train_m")}
    {@render numberField("ui.train_cutter_tip_round", () => cut.tip_round, (v) => (cut.tip_round = v), 0.02, "ui.train_m")}
  {/if}
  <h4 class="section-heading" class:later={opts.cutter !== undefined}>{title}</h4>
  <label class:invalid={g && outside(gear.teeth, g.ranges.teeth)}>
    <span>{t(opts.teethLabel ?? "ui.train_tooth_count")}</span>
    <input type="number" step="1" bind:value={() => gear.teeth, finite((v) => (gear.teeth = v))} />
  </label>
  {#if opts.member}
    {@const m = opts.member}
    <!-- **The module and the pressure angle, stated on one gear of a mesh
         group and followed by the rest** — the helix's rule with the
         relation made equality: a tooth is cut at one of each, so the gears
         a run of meshes joins share them. Touching one here makes it the
         group's, and relief hands the rest to it; automatic shows what the
         group is cut at. -->
    {@render autoNumber(
      "ui.train_normal_module",
      m.module,
      g?.params.module,
      0.1,
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "module"] }),
      undefined,
      "ui.train_mm",
    )}
    {@render autoNumber(
      "ui.train_pressure_angle",
      m.pressure_angle,
      g?.params.pressure_angle,
      0.5,
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "pressure_angle"] }),
      undefined,
      "°",
    )}
    <!-- One coefficient per member, given on one member of each mesh and
         automatic on the other, which follows the mesh's rule — the two sum
         to 2 across an external mesh, a ring takes its pinion's. Relief keeps
         at most one of a mesh's two given, so touching this one is what hands
         the mate over; the automatic box shows what it came to. -->
    {@render autoNumber(
      "ui.train_tooth_thickness_mod",
      m.thickness_mod,
      g?.params.thickness_mod,
      0.05,
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "thickness_mod"] }),
      undefined,
      "ui.train_k",
    )}
  {/if}
  {#if opts.pitchDiameter}
    {@render autoNumber(
      "ui.train_pitch_diameter",
      opts.pitchDiameter,
      g?.pitch_diameter,
      0.5,
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "pitch_diameter"] }),
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
    () => opts.relief !== undefined && relieve({ member: [opts.relief, "helix"] }),
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
      <input type="number" step="0.02" bind:value={() => gear.min_tip_width, finite((v) => (gear.min_tip_width = v))} />
      <em>{t("ui.train_mm")}</em>
    </label>
  {/if}
  {#if opts.cut !== "shaper"}
    <label class:invalid={g && outside(gear.dedendum, g.ranges.dedendum)}>
      <span>{t("ui.train_dedendum")}</span>
      <input type="number" step="0.05" bind:value={() => gear.dedendum, finite((v) => (gear.dedendum = v))} />
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
      <input type="number" step="0.01" bind:value={() => gear.root_radius, finite((v) => (gear.root_radius = v))} />
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
    () => opts.relief !== undefined && relieve({ member: [opts.relief, "shift"] }),
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
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "face_width"] }),
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
      () => opts.relief !== undefined && relieve({ member: [opts.relief, "face_width"] }),
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
  /** What the label's key names, where it names something. */
  args?: Record<string, string>,
)}
  <label>
    <span>{t(key, args)}</span>
    <input type="number" {step} bind:value={get, finite(set)} />
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
  /** What the box shows while automatic: the number the core came to,
   *  `undefined` to show the held number, or `null` for a blank — a figure
   *  the core could not come to and the designer has yet to give. */
  computed: number | undefined | null,
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
  /** Indented under the row above it, as a load's figures sit under its port. */
  sub?: boolean,
)}
  {@const shown = computed === undefined ? a.manual : computed === null ? null : Number(computed.toFixed(4))}
  <label class="auto" class:constrained={constraint !== undefined} class:sub>
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
      <input type="number" {step} value={shown ?? ""} disabled class="computed" />
    {:else}
      <input type="number" {step} bind:value={() => a.manual, finite((v) => (a.manual = v))} />
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
          if (!v && a.auto && shown !== null) a.manual = shown;
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
<!-- **How the load is divided while two tooth pairs are engaged**, a mesh's
     own: the ramp is a model of one contact, so two meshes on one gear can be
     rated under different ones.

     Off by default and deliberately so: the ramp behind it is an uncalibrated
     placeholder rather than a stiffness model. Offered rather than hidden,
     because an estimate a designer chooses is a feature and one applied on
     their behalf is not. It was one field on the stage, on the argument that a
     stage running two meshes under two readings would be reporting a
     comparison; the default answers that instead — every mesh starts at none,
     and a mesh an edit adds takes the first mesh's — and a designer who sets
     two differently has said so, mesh by mesh, where they can see it.

     Withheld only where there is no bending stress to reach: a crossed pair
     contacts at a point, and this touches bending alone. -->
{#snippet loadSharing(m: { load_sharing: LoadSharing }, pair?: { a: string; b: string })}
  <label>
    <span>{pair ? t("ui.train_load_sharing_of", pair) : t("ui.train_load_sharing")}</span>
    <select bind:value={m.load_sharing}>
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
{#snippet overlapField(stage: Shape, meshes: number[], reports: (MeshReport | undefined)[])}
  <!-- **One box for a mesh group's ratio.** The datum is each mesh's
       (`MeshInput.overlap`); the group's meshes carry one number, so the
       box reads the first and writes them all — an `Auto` with accessors,
       which is what the field binds to. The core reads the group's first
       mesh as the size reading and every mesh's as a floor. -->
  {@const first = stage.meshes[meshes[0]]}
  {@const shared: Auto<number> = {
    get auto() {
      return first.overlap.auto;
    },
    set auto(v: boolean) {
      for (const k of meshes) stage.meshes[k].overlap.auto = v;
    },
    get manual() {
      return first.overlap.manual;
    },
    set manual(v: number) {
      for (const k of meshes) stage.meshes[k].overlap.manual = v;
    },
  }}
  {@render autoNumber(
    "ui.train_overlap",
    shared,
    reports[meshes[0]]?.line?.contact_ratios.overlap,
    0.1,
    () => relieve({ overlap: meshes[0] }),
    meshes
      .flatMap((k) => reports[k]?.notes ?? [])
      .filter((n) => n.key === "mesh.overlap_below_one")
      .slice(0, 1)
      .map(note)[0] ?? t("ui.train_note_overlap"),
    "",
    undefined,
    undefined,
  )}
{/snippet}

{#snippet searchToggle(m: { search: boolean }, pair?: { a: string; b: string })}
  {@render switchField(
    pair ? "ui.train_optimise_efficiency_of" : "ui.train_optimise_efficiency",
    m.search,
    (v) => (m.search = v),
    t("ui.train_note_optimise_efficiency"),
    pair,
  )}
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

<!-- **The list and the workspace.** The list is the train grouped one of
     three ways, each the core's (`groupings`, `flows`): the flow, drawn
     down; the centres; the axes. Selecting a row shows that piece in the
     workspace beside it, and the selection is the tab's, so looking away
     and back finds it where it was. -->
<div class="panes">
  <div class="side">
    <!-- **The cases, one to a row, and the path the shown one walks** — in the
         list's column, so the workspace beside it starts at the top. The case
         chosen is the one the list's flow and the workspace are shown for, and
         the path under the cases is the one it reports: its load to its
         reaction, with what the train comes to along it. -->
    <section class="pane cases">
      <h4 class="section-heading">{t("ui.train_cases")}</h4>
      <div class="case-list">
        {#each tab.train.load_cases as c, i (i)}
          <button
            class="case"
            class:on={i === shownCase}
            class:off={!c.enabled}
            onclick={() => {
              tab.view.case = i;
              select({ case: i });
            }}
          >
            <span class="case-name">{caseName(i)} · {kindLabel(c.kind)}</span>
            {#if caseSummary(c)}
              <span class="case-sum">{caseSummary(c)}</span>
            {/if}
            {#if c.enabled && solved !== undefined && !(forCase(solved.cases, i)?.solved ?? false)}
              <span class="case-sum warn">{t("ui.train_case_incomplete")}</span>
            {/if}
          </button>
        {/each}
      </div>
      <div class="case-adds">
        {#each CASE_KINDS as k (k.key)}
          <button class="action add" onclick={() => addCaseOfKind(k)}>{t(k.add)}</button>
        {/each}
      </div>
    {#if casePath}
      <dl class="out pathbox">
        <dt>{t("ui.train_path_of", { case: caseName(shownCase) })}</dt>
        <dd>{t("ui.train_path_between", { from: bodyName(casePath.from), to: bodyName(casePath.to) })}</dd>
        <dt>{t("ui.train_ratio")}</dt>
        <dd>{Math.abs(casePath.ratio) >= 1 ? `${num(casePath.ratio, 4)} : 1` : `1 : ${num(1 / casePath.ratio, 4)}`}</dd>
        <dt>{t("ui.train_efficiency")}</dt>
        <dd>
          <span class="line">{t("ui.train_driven_forward", { percent: pct(casePath.efficiency.forward) })}</span>
          <span class="line">{t("ui.train_driven_backward", { percent: pct(casePath.efficiency.backward) })}</span>
          {#if lockedWays(casePath.efficiency)}<small class="warn">{lockedWays(casePath.efficiency)}</small>{/if}
        </dd>
        <!-- The play at each end, driven from the other: at the far end
             driving forward, and at the near end driving back. -->
        <dt>{t("ui.train_backlash")}</dt>
        <dd>
          <span class="line">
            {t("ui.train_backlash_at", { angle: num(casePath.backlash.forward.nominal, 5), member: bodyName(casePath.to) })}
            <small>{range(num(casePath.backlash.forward.minimum, 5), num(casePath.backlash.forward.maximum, 5))}</small>
          </span>
          <span class="line">
            {t("ui.train_backlash_at", { angle: num(casePath.backlash.backward.nominal, 5), member: bodyName(casePath.from) })}
            <small>{range(num(casePath.backlash.backward.minimum, 5), num(casePath.backlash.backward.maximum, 5))}</small>
          </span>
        </dd>
        <!-- The power the teeth pass, as a multiple of the power in: one
             across a pair, and where it is many the path's loss is the
             meshes' loss that many times over. -->
        <dt>{t("ui.train_circulation")}</dt>
        <dd>
          {t("ui.train_circulation_both", { forward: num(casePath.circulation.forward, 2), backward: num(casePath.circulation.backward, 2) })}
          <small>{t("ui.train_note_circulation")}</small>
        </dd>
      </dl>
    {:else if solved && tab.train.load_cases.length > 0}
      <p class="notice">{t("ui.train_no_path_for_case")}</p>
    {/if}
    </section>

    <!-- **What the train says of itself**: why there is no answer at all, and
         what each part's own solve raised — its search, its closures, its
         planets — each naming the part by its meshes and showing it on a
         click; and the one train-wide decision about how a gear is judged. -->
    <div class="said">
      {#if failure || partNotes.length > 0}
        <ul class="notes">
          {#if failure}
            <li class="warn">
              {#if failure.part !== null}
                <button class="link" onclick={() => showPart(failure.part! - 1)}>{partName(failure.part - 1)}</button>:
              {/if}
              {note(failure.note)}
            </li>
          {/if}
          {#each partNotes as x, i (i)}
            <li><button class="link" onclick={() => showPart(x.part)}>{partName(x.part)}</button>: {note(x.note)}</li>
          {/each}
        </ul>
      {/if}
      <!-- Train-wide, because it is one decision about how every gear is
           judged rather than a property of any part or any load: a planet's
           root is loaded on both flanks whatever the load does, and a
           reversing duty loads every root both ways — but the allowance for it
           is a fraction on an allowable a part is sized against, which this
           tool asks for rather than applies. Its note says what it does
           whether or not it is on. -->
      <div class="grid shared">
        {@render switchField(
          "ui.train_reversed_bending",
          tab.train.reversed_bending,
          (v) => (tab.train.reversed_bending = v),
          t("ui.train_note_reversed_bending", {
            coefficient: defaults().reverse_loading_coefficient.toFixed(2),
          }),
        )}
      </div>
    </div>
    <section class="pane">
      <div class="pane-head">
        <div class="seg">
          {#each groupings as g (g.key)}
            <button class:on={tab.view.grouping === g.key} onclick={() => (tab.view.grouping = g.key)}>{t(g.label)}</button>
          {/each}
        </div>
        {#if tab.view.grouping === "flow" && tab.train.load_cases.length > 0}
          <small>{t("ui.train_showing_case", { case: caseName(shownCase) })}</small>
        {/if}
      </div>
      <p class="hint">
        {t({ flow: "ui.train_note_flow", centres: "ui.train_note_centres", axes: "ui.train_note_axes" }[tab.view.grouping])}
      </p>
      {#if shownUnsolved !== null && tab.view.grouping === "flow"}
        <p class="notice warn">
          {t("ui.train_case_incomplete")}{shownUnsolved.length > 0 ? `: ${shownUnsolved.map(note).join(" · ")}` : ""}
        </p>
      {/if}
      {#if tab.view.grouping === "flow"}
        {@render flowList()}
      {:else if tab.view.grouping === "centres"}
        {@render centresList()}
      {:else}
        {@render axesList()}
      {/if}
      {#if tab.train.shape.members.length === 0}
        <p class="hint">{t("ui.train_no_stages")}</p>
      {/if}
      <Offers train={tab.train} at={[...selected, atOutput]} kind="adds" {names} materials={ratedUnder()} {made} />
    </section>
  </div>
  <section class="pane workspace">
    <Offers train={tab.train} at={selected} kind="verbs" {names} materials={ratedUnder()} {made} />
    {@render workspaceOf(tab.view.selection)}
  </section>
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

  /* No top margin here: it is beside the inputs, not below them. */

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
  /* A choice the train has taken from the case — a held body — is shown and
     cannot be pressed. */
  .segmented button:disabled {
    color: var(--muted);
    cursor: not-allowed;
  }
  .segmented button:disabled:hover {
    background: none;
  }
  .segmented button.on:disabled {
    color: var(--fg);
    background: var(--selected);
  }
  .out {
    display: grid;
    grid-template-columns: auto 1fr;
    gap: 0.15rem 0.75rem;
    margin: 0.75rem 0 0;
    font-size: 0.85rem;
  }

  .bodyrow {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.25rem 0.4rem;
    background: var(--panel);
    border: 1px solid var(--rule);
    border-radius: 3px;
    font-size: 0.8rem;
  }

  /* What a row states about itself and nothing edits: held, carried, how
     many copies stand round a carrier, idle. */
  .chip {
    font-size: 0.68rem;
    color: var(--muted);
    border: 1px solid var(--rule);
    border-radius: 999px;
    padding: 0 0.4rem;
    white-space: nowrap;
  }
  .chip.held {
    color: var(--warn);
    border-color: var(--warn);
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

  /* A switch the case cannot honour yet: the case is short of what the train
     needs to solve it, and switching it on would rate nothing. */
  .control.locked {
    opacity: 0.45;
    cursor: not-allowed;
  }
  /* **A case's inputs on the left, what it comes to on the right** — the
     inputs in one column, and the table beside them where the workspace
     is wide enough for both, under them where it is not: the workspace's
     width, not the window's, since the list takes a share of the window. */
  .casebody {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: 0.6rem 1.5rem;
    align-items: start;
  }
  @container (min-width: 60rem) {
    .casebody {
      grid-template-columns: minmax(0, 34rem) minmax(0, 1fr);
    }
  }
  .casebody > .action {
    grid-column: 1 / -1;
    justify-self: start;
  }
  .delivered {
    margin-top: 0.6rem;
  }
  .delivered h4 {
    margin: 0;
  }
  .delivered table.cases td.role {
    color: var(--muted);
  }
  .delivered table.cases tr.muted {
    opacity: 0.55;
  }
  /* A second port's row opens a second group, as a card's later heading does. */
  .mode.later {
    margin-top: 0.4rem;
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

  .shared {
    margin-top: 0.6rem;
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
  /* The name alone: an `auto` row's switches are spans too, and sit where
     their columns put them. */
  .sub > span:first-child {
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
  /* A case's remove sits below its inputs, apart from them. */
  .action.danger {
    margin-top: 0.6rem;
  }

  /* What the train says of itself, under its path: why it has no answer,
     each part's notes, and the switch every gear is judged by. */
  .said {
    margin: 0;
  }
  .said .notes {
    margin: 0 0 0.5rem;
    padding-left: 1.1rem;
    font-size: 0.82rem;
  }
  /* **The list's column and the workspace** — the canvas's page: the
     cases, what the train says of itself and the list one above the other,
     and the selected piece beside them from the top. */
  .side {
    display: flex;
    flex-direction: column;
    gap: 0.8rem;
    min-width: 0;
  }
  .case-list {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    margin-top: 0.4rem;
  }
  /* A case to a row: its name and kind, what it loads, and whether it
     solves, one under the other. */
  .case {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.05rem;
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.3rem 0.55rem;
    border: 1px solid var(--rule);
    border-radius: 4px;
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  .case .case-name {
    font-weight: 600;
  }
  .case .case-sum {
    font-size: 0.74rem;
    color: var(--muted);
  }
  .case .case-sum.warn {
    color: var(--warn);
  }
  .case-adds {
    display: flex;
    flex-wrap: wrap;
    gap: 0.35rem;
    margin-top: 0.5rem;
  }
  .case.on {
    background: var(--selected);
    border-color: var(--accent);
  }
  .case.off {
    color: var(--muted);
  }
  /* The path the shown case walks, under the cases: a rule above it, and
     its figures right-aligned against their labels. */
  .pathbox {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: 1rem;
    margin: 0.7rem 0 0;
    padding: 0.5rem 0 0;
    border-top: 1px solid var(--rule);
  }
  .pathbox dd {
    text-align: right;
  }
  /* A note under a figure wraps under it, and does not widen the box. */
  .pathbox dd small {
    display: block;
  }
  .panes {
    display: grid;
    grid-template-columns: minmax(16rem, 26rem) minmax(0, 1fr);
    gap: 0.8rem;
    align-items: start;
    margin-bottom: 1rem;
  }
  @media (max-width: 900px) {
    .panes {
      grid-template-columns: minmax(0, 1fr);
    }
  }
  .pane {
    border: 1px solid var(--rule);
    border-radius: 4px;
    padding: 0.6rem 0.7rem;
    min-width: 0;
  }
  .pane-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .pane-head small {
    color: var(--muted);
  }
  .seg {
    display: inline-flex;
    border: 1px solid var(--rule);
    border-radius: 3px;
    overflow: hidden;
  }
  .seg button {
    font: inherit;
    font-size: 0.78rem;
    padding: 0.2rem 0.7rem;
    border: 0;
    border-right: 1px solid var(--rule);
    background: var(--bg);
    color: var(--fg);
    cursor: pointer;
  }
  .seg button:last-child {
    border-right: 0;
  }
  .seg button.on {
    background: var(--selected);
    font-weight: 600;
  }
  .pane .hint {
    font-size: 0.72rem;
    color: var(--muted);
    margin: 0.3rem 0 0.5rem;
  }
  .fb,
  .cen,
  .bodyrow {
    display: grid;
    grid-template-columns: auto 1fr auto;
    align-items: baseline;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.3rem 0.45rem;
    margin: 0.15rem 0;
    border: 1px solid var(--rule);
    border-radius: 3px;
    background: var(--panel);
    color: var(--fg);
    cursor: pointer;
  }
  .fb .name,
  .cen .name,
  .bodyrow .name {
    font-weight: 600;
  }
  .fb .on {
    color: var(--muted);
  }
  .fb .fig,
  .cen .fig {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .fm,
  .gearrow {
    display: block;
    width: 100%;
    text-align: left;
    font: inherit;
    font-size: 0.82rem;
    padding: 0.25rem 0.45rem 0.25rem 1.2rem;
    border: 1px solid transparent;
    border-radius: 3px;
    background: none;
    color: var(--fg);
    cursor: pointer;
  }
  .fm .arrow {
    color: var(--accent);
    font-weight: 700;
  }
  .fm.idle {
    color: var(--muted);
  }
  .fm small {
    color: var(--muted);
    margin-left: 0.4rem;
  }
  .gearrow .z {
    font-size: 0.72rem;
    color: var(--muted);
  }
  .junction {
    margin: 0.1rem 0 0.1rem 1.2rem;
    padding: 0.2rem 0.4rem 0.35rem;
    border: 1px dashed var(--accent);
    border-radius: 4px;
    font-size: 0.8rem;
  }
  .junction .fm {
    padding-left: 0;
    font-weight: 600;
  }
  .junction .line {
    display: block;
    color: var(--muted);
  }
  .sel,
  .fb.sel,
  .cen.sel,
  .bodyrow.sel,
  .junction.sel {
    background: var(--selected);
    border-color: var(--accent);
  }
  .axisrow {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0.7rem 0 0.3rem;
  }
  .axisname {
    font: inherit;
    font-size: 0.72rem;
    font-weight: 700;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: var(--accent);
    background: none;
    border: 1px solid transparent;
    border-radius: 3px;
    cursor: pointer;
  }
  .axisrow .rule {
    flex: 1 1 auto;
    height: 1px;
    background: var(--rule);
  }
  .chip {
    font-size: 0.68rem;
    color: var(--muted);
    border: 1px solid var(--rule);
    border-radius: 999px;
    padding: 0 0.4rem;
    white-space: nowrap;
  }
  .chip.held {
    color: var(--warn);
    border-color: var(--warn);
  }
  .dim {
    color: var(--muted);
  }
  .ws-head {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    flex-wrap: wrap;
    margin-bottom: 0.5rem;
  }
  .ws-head small {
    color: var(--muted);
  }
  .ws-head .link {
    margin-left: auto;
    font: inherit;
    font-size: 0.8rem;
    color: var(--accent);
    background: none;
    border: 0;
    cursor: pointer;
  }
  /* **The workspace lays itself out by its own width**: the two gears
     either side of the mesh where there is room for three columns, the two
     gears side by side with the mesh under them where there is room for
     two, and one above the other where there is not — a gear's card read
     across to its mate's wherever it can be. */
  .workspace {
    container-type: inline-size;
  }
  .ws {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    grid-template-areas: "first" "mesh" "second";
    gap: 0.7rem;
    align-items: start;
  }
  @container (min-width: 44rem) {
    .ws {
      grid-template-columns: repeat(2, minmax(0, 1fr));
      grid-template-areas: "first second" "mesh mesh";
    }
  }
  @container (min-width: 72rem) {
    .ws {
      grid-template-columns: minmax(0, 1fr) minmax(0, 0.9fr) minmax(0, 1fr);
      grid-template-areas: "first mesh second";
    }
  }
  .ws .col {
    min-width: 0;
  }
  .ws .first {
    grid-area: first;
  }
  .ws .second {
    grid-area: second;
  }
  .ws .meshcol {
    grid-area: mesh;
  }
  .out.comes {
    max-width: 48rem;
  }
  .ws .meshcol {
    border: 1px solid var(--rule);
    border-radius: 4px;
    padding: 0.4rem 0.6rem;
    background: var(--panel);
  }
</style>
