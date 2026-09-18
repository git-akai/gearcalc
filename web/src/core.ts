// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

import {
  setCatalogue,
  setLanguages,
  setCurrentLanguage,
  t,
  type LanguageOption,
  type Note,
} from "./strings.svelte";
import type {
  Figure,
  Freedom,
  FreedomGroup,
  Arrangement,
  CaseKind,
  LoadCase,
  Port,
  Auto,
  Optimisation,
  Backlash,
  Basis,
  Bound,
  CentreProfile,
  ClassRef,
  ContactRatios,
  PairKind,
  LineContact,
  ContactPatch,
  PairResult,
  PairStage,
  PointContact,
  Cutter,
  CutterRef,
  Defaults,
  Directional,
  GearParams,
  GearRequest,
  GearResult,
  GearSummary,
  LoadSharing,
  MateRef,
  Value,
  Material,
  MaterialLibrary,
  Maybe,
  Measure,
  MeshReport,
  Overrides,
  PerToothClamps,
  PinsOut,
  PlanetResult,
  PlanetaryResult,
  PlanetaryStage,
  Ranges,
  RingRequest,
  RingSummary,
  ShiftRange,
  SpanOut,
  Stage,
  StageGear,
  StageResult,
  ToleranceOut,
  Train,
  TrainResult,
  TrainOutcome,
  AdoptOutcome,
  Adopted,
  TrainFailure,
  Variation,
  Constraint,
  ShaftConstraint,
  PortSpec,
  StagePorts,
  MotionReport,
  ShaftLabel,
  Exact,
} from "./wire";
export type { CaseKind, LoadCase, Port };
export type {
  Arrangement,
  Duty,
  GearCase,
  MeshCase,
  TrainCase,
  Figure,
  Freedom,
  FreedomGroup,
  Auto,
  Backlash,
  Basis,
  Bound,
  CentreProfile,
  ClassRef,
  ContactRatios,
  PairKind,
  LineContact,
  ContactPatch,
  PairResult,
  PairStage,
  PointContact,
  Cutter,
  CutterRef,
  Defaults,
  Directional,
  GearParams,
  GearRequest,
  GearResult,
  GearSummary,
  LoadSharing,
  MateRef,
  Optimisation,
  Value,
  Material,
  MaterialLibrary,
  Maybe,
  Measure,
  MeshReport,
  Overrides,
  PerToothClamps,
  PinsOut,
  PlanetResult,
  PlanetaryResult,
  PlanetaryStage,
  Ranges,
  RingRequest,
  RingSummary,
  ShiftRange,
  SpanOut,
  Stage,
  StageGear,
  StageResult,
  ToleranceOut,
  Train,
  TrainResult,
  TrainOutcome,
  AdoptOutcome,
  Adopted,
  TrainFailure,
  Variation,
  Constraint,
  ShaftConstraint,
  PortSpec,
  StagePorts,
  MotionReport,
  ShaftLabel,
  Exact,
} from "./wire";

import init, {
  solve_gear,
  solve_ring,
  ring_profile,
  export_ring_dxf,
  solve_train,
  gear_profile,
  export_dxf,
  version,
  defaults as wasm_defaults,
  strings as wasm_strings,
  languages as wasm_languages,
  resolve_language as wasm_resolve_language,
  default_materials,
  import_materials,
  export_materials,
  import_train,
  export_train,
  relieve_stage,
  arrange_stage,
  adopt_member,
} from "./wasm/gear_wasm.js";

/** Narrow a `Maybe` to its "there is no value" arm.
 *
 *  The predicate names the arm's own shape rather than restating it: the reason
 *  is a `Note` now — a key and its values, like every other message the core
 *  emits — and a predicate written as `{ unavailable: string }` would silently
 *  stop narrowing the moment that changed, which is exactly what it did. */
export function isUnavailable<T>(v: Maybe<T>): v is { unavailable: Note } {
  return v !== null && typeof v === "object" && "unavailable" in v;
}

/** Why `v` is outside `b`, or null if it is inside. Comparison only.
 *
 *  The four sentences are the catalogue's. `gear_core::auto::Bound` used to
 *  produce the same English independently — the same value written down in two
 *  languages that docs/corrections.md is largely about — and this is the copy that was
 *  ever shown, so it is the one that survived. */
export function outside(v: number, b: Bound): string | null {
  if (!Number.isFinite(v)) return t("ui.validation_not_a_number");
  if (b.min !== null && (b.exclusive_min ? v <= b.min : v < b.min)) {
    const key = b.exclusive_min
      ? "ui.validation_greater_than"
      : "ui.validation_at_least";
    return t(key, { bound: String(b.min) });
  }
  if (b.max !== null && (b.exclusive_max ? v >= b.max : v > b.max)) {
    const key = b.exclusive_max
      ? "ui.validation_less_than"
      : "ui.validation_at_most";
    return t(key, { bound: String(b.max) });
  }
  return null;
}

/** The bound for a field, from the gear's own ranges. */
export function boundFor(key: keyof GearParams, r: Ranges): Bound | null {
  switch (key) {
    case "module":
      return r.module;
    case "pressure_angle":
      return r.pressure_angle;
    case "teeth":
      return r.teeth;
    case "helix_angle":
      return r.helix_angle;
    case "thickness_mod":
      return r.thickness_mod;
    case "profile_shift":
      return r.profile_shift.bound;
    case "addendum":
      return r.addendum;
    case "dedendum":
      return r.dedendum;
    case "root_radius":
      return r.root_radius;
    case "angular_shift":
      return r.angular_shift;
    default:
      return null;
  }
}

// --------------------------------------------------------------------- //
//  Field definitions — valid ranges declared once, as data, next to the
//  field they describe. The UI renders from this and validates from this,
//  so a range cannot drift between the two.
// --------------------------------------------------------------------- //

/** What kind of gear a tab holds.
 *
 *  Three cases rather than a boolean, because an eccentric gear is a third
 *  thing to *enter* even though it is the concentric one's `Δx = 0` everywhere
 *  below the boundary. The core has no such enum and does not want one. */
export type GearKind = "external" | "internal" | "eccentric";

export interface KindSpec {
  key: GearKind;
  /** Catalogue key for the kind's name. Not the name, for the reason a field's
   *  label is not either: it is a word the application shows. */
  label: string;
  /** Catalogue key for the note under the picker, where the kind carries
   *  something its name does not. "External" says it and so does "Internal": a
   *  note repeating the word above it is one more thing to keep true for no
   *  reading gained. */
  note?: string;
  /** Offered only while the developer mode is on — see `developer` in
   *  `state.svelte.ts`, and docs/rationale.md#unfinished-work-is-knocked-for-not-switched-on. */
  developer?: boolean;
}

/** The kinds, as data, for the same reason `FIELDS` below is: the picker renders
 *  from this and `setKind` reads the fields against it, so a fourth kind is a
 *  row here rather than an option, a note and a filter kept in step by hand. */
export const KINDS: KindSpec[] = [
  { key: "external", label: "ui.gear_kind_external" },
  { key: "internal", label: "ui.gear_kind_internal" },
  {
    key: "eccentric",
    label: "ui.gear_kind_eccentric",
    note: "ui.gear_kind_eccentric_note",
    developer: true,
  },
];

/// What kind of stage a geartrain holds.
export type StageKind = "spur" | "worm" | "planetary" | "hula";

export interface StageKindSpec {
  key: StageKind;
  /** Catalogue key for the button that adds one. */
  label: string;
  /** A fresh stage of this kind, from the core. */
  fresh: () => Stage;
  /** Offered only while the developer mode is on — the same knock the gear
   *  tab's eccentric kind is behind, through the same table shape, so one
   *  mechanism gates both. */
  developer?: boolean;
}

/** The stage kinds, as data, for the same reason `KINDS` and `FIELDS` are: the
 *  "add stage" buttons render from this, so a fifth kind is a row rather than a
 *  hand-written button that has to be remembered.
 *
 *  A **crossed** pair is deliberately not here. It is a spur stage with its
 *  shafts at an angle, not a kind of its own, and the core says so. */
// A default stage arrives **tagged** — Rust's `Stage` is an internally tagged
// enum, so the object carries its own `kind`. They used to be four hand-written
// accessors and three hand-written buttons; a kind is one row here now, and the
// tag is what the panel branches on.
export const STAGE_KINDS: StageKindSpec[] = [
  {
    key: "spur",
    label: "ui.train_add_spur_stage",
    fresh: () => defaults().spur_stage,
  },
  {
    key: "worm",
    label: "ui.train_add_worm_stage",
    fresh: () => defaults().worm_stage,
  },
  {
    key: "planetary",
    label: "ui.train_add_planetary_stage",
    fresh: () => defaults().planetary_stage,
  },
  {
    key: "hula",
    label: "ui.train_add_hula_stage",
    fresh: () => defaults().hula_stage,
    developer: true,
  },
];

export interface CaseKindSpec {
  key: CaseKind;
  /** Catalogue key for the kind's name — the chip on a heading, the option in
   *  the select. */
  label: string;
  /** Catalogue key for the button that adds one. */
  add: string;
  /** A fresh case of this kind, from the core. */
  fresh: () => LoadCase;
}

/** The load case kinds, as data, for the reason the stage kinds are: the "add
 *  load case" buttons and the kind select render from this, and a kind decides
 *  which allowable the core judges against and which inputs are put in front
 *  of the designer — nothing else. */
export const CASE_KINDS: CaseKindSpec[] = [
  {
    key: "ultimate",
    label: "ui.train_case_ultimate",
    add: "ui.train_add_ultimate_case",
    fresh: () => defaults().ultimate_case,
  },
  {
    key: "fatigue",
    label: "ui.train_case_fatigue",
    add: "ui.train_add_fatigue_case",
    fresh: () => defaults().fatigue_case,
  },
];

/** **A port as a select's value.** A port is a name or a shaft reference,
 *  and a `<select>` binds to strings, so each is keyed by a string that
 *  round-trips through {@link portOptions} — the option list the core sent
 *  — rather than being parsed back. Nothing here decides what a port is. */
export function portKey(p: Port): string {
  if (p === "start" || p === "end") return p;
  return p.at.kind === "ground" ? "at:ground" : `at:${p.at.stage}.${p.at.shaft}`;
}

/** **Where a load can enter**, in the order the chain runs: the open ports
 *  the core reports with the motion — its two ends by name, and any other
 *  uncoupled, un-held shaft by reference — or, where the train has no motion
 *  to report, the two names alone so a case can still be written. */
export function portOptions(motion: MotionReport | null): { key: string; port: Port }[] {
  const ports: Port[] = motion?.ports.map((p) => p.port) ?? ["start", "end"];
  return ports.map((port) => ({ key: portKey(port), port }));
}

export interface FieldSpec {
  key: keyof GearParams;
  /** Catalogue key for the field's name. Not the name: an input label is a word
   *  the application shows, so it belongs with the other words. */
  label: string;
  /** Catalogue key for the unit, or "" where the field is a bare number. */
  unit: string;
  step: number;
  integer?: boolean;
  /** Catalogue key for the note shown under the field. */
  note?: string;
  /**
   * Not an input for an internal gear, and hidden there.
   *
   * A ring's root circle is wherever its cutter's tip reaches and its fillet
   * round is the cutter's own, so both are properties of the tool rather than
   * of the part. Showing a box that changes nothing is worse than showing none.
   */
  /** The kinds this field applies to. Absent means all of them. */
  kinds?: GearKind[];
  /** Catalogue key replacing `note` for an internal gear, where the rule
   *  differs. */
  ringNote?: string;
}

export const FIELDS: FieldSpec[] = [
  {
    key: "module",
    label: "ui.gear_field_module",
    unit: "ui.gear_mm",
    step: 0.1,
  },
  {
    key: "pressure_angle",
    label: "ui.gear_field_pressure_angle",
    unit: "ui.gear_deg",
    step: 0.5,
  },
  {
    key: "teeth",
    label: "ui.gear_field_teeth",
    unit: "",
    step: 1,
    integer: true,
  },
  {
    key: "helix_angle",
    label: "ui.gear_field_helix_angle",
    unit: "ui.gear_deg",
    step: 1,
  },
  {
    key: "profile_shift",
    label: "ui.gear_field_profile_shift",
    unit: "ui.gear_m",
    step: 0.05,
  },
  {
    key: "addendum",
    label: "ui.gear_field_addendum",
    unit: "ui.gear_m",
    step: 0.05,
  },
  {
    key: "dedendum",
    label: "ui.gear_field_dedendum",
    unit: "ui.gear_m",
    step: 0.05,
    kinds: ["external", "eccentric"],
  },
  {
    key: "root_radius",
    label: "ui.gear_field_root_radius",
    unit: "ui.gear_m",
    step: 0.01,
    kinds: ["external", "eccentric"],
  },
  {
    key: "thickness_mod",
    label: "ui.gear_field_thickness_mod",
    unit: "",
    step: 0.05,
    note: "ui.gear_note_thickness_mod",
    // On a ring it is the SPACE this describes, so a pinion and a ring that mesh
    // want the SAME k rather than complementary ones.
    ringNote: "ui.gear_note_thickness_mod_ring",
  },
  {
    key: "angular_shift",
    label: "ui.gear_field_angular_shift",
    kinds: ["eccentric"],
    unit: "ui.gear_m",
    step: 0.05,
    note: "ui.gear_note_angular_shift",
  },
  {
    key: "index_offset",
    label: "ui.gear_field_index_offset",
    kinds: ["eccentric"],
    unit: "",
    step: 0.1,
    note: "ui.gear_note_index_offset",
  },
];

/** Why a value is not acceptable, given the bound Rust returned. */
export function validate(
  f: FieldSpec,
  v: number,
  b: Bound | null,
): string | null {
  if (f.integer && !Number.isInteger(v))
    return t("ui.validation_not_a_whole_number");
  return b === null
    ? Number.isFinite(v)
      ? null
      : t("ui.validation_not_a_number")
    : outside(v, b);
}

// --------------------------------------------------------------------- //

let ready: Promise<void> | null = null;
let cachedDefaults: Defaults | null = null;

/** Where the chosen language is remembered.
 *
 *  A preference about *reading*, not an input to a calculation — so unlike
 *  everything in a tab it is allowed to outlive the session, and unlike
 *  everything in a tab losing it costs nothing. Both reads and writes are
 *  guarded: a browser with site data blocked throws on the accessor itself, and
 *  a language picker is not worth a blank page. */
export const LANGUAGE_KEY = "gearcalc.language";
const STORED = LANGUAGE_KEY;

function stored(): string | null {
  try {
    return localStorage.getItem(STORED);
  } catch {
    return null;
  }
}

/** Switch language, reloading the catalogue from the core.
 *
 *  Nothing else has to happen: `t()` is a reactive read, so every label on
 *  screen re-renders itself. That is the same property that lets the catalogue
 *  arrive late at start-up (`strings.svelte.ts`), used a second time. */
export function setLanguage(tag: string): void {
  // Rust decides which shipped language a tag names — see
  // `gear_io::strings::Language::resolve` — so a browser's `zh-TW` and a stored
  // `de-CH` both land somewhere real, and the picker shows what is in force.
  const code = wasm_resolve_language(tag);
  setCurrentLanguage(code);
  try {
    localStorage.setItem(STORED, code);
  } catch {
    // A viewer who cannot store a preference can still change it for now.
  }
  setCatalogue(JSON.parse(wasm_strings(code)) as Record<string, string>);
}

/** Load the core once. Safe to await repeatedly. */
export function loadCore(): Promise<void> {
  if (!ready) {
    ready = init().then(() => {
      cachedDefaults = JSON.parse(wasm_defaults()) as Defaults;
      setLanguages(JSON.parse(wasm_languages()) as LanguageOption[]);
      // A stored preference, else what the browser asks for. Neither needs
      // validating: an unrecognised tag resolves to English.
      setLanguage(stored() ?? navigator.language ?? "en");
    });
  }
  return ready;
}

/** The defaults, as a fresh copy: everything handed out here is about to
 *  become a tab's mutable state, so callers must not share one object. */
export function defaults(): Defaults {
  if (!cachedDefaults) {
    throw new Error(
      "the defaults were asked for before the core finished loading",
    );
  }
  return structuredClone(cachedDefaults);
}

export function coreVersion(): string {
  return version();
}

export function solve(
  req: GearRequest,
): { ok: GearSummary } | { error: string } {
  try {
    return { ok: JSON.parse(solve_gear(JSON.stringify(req))) as GearSummary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function profile(
  req: GearRequest,
  pointsPerTooth: number,
): Float64Array | null {
  try {
    return gear_profile(JSON.stringify(req), pointsPerTooth);
  } catch {
    return null;
  }
}

export function dxf(req: GearRequest): { ok: string } | { error: string } {
  try {
    return { ok: export_dxf(JSON.stringify(req)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function solveRing(
  req: RingRequest,
): { ok: RingSummary } | { error: string } {
  try {
    return { ok: JSON.parse(solve_ring(JSON.stringify(req))) as RingSummary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function ringProfile(
  req: RingRequest,
  pointsPerTooth: number,
): Float64Array | null {
  try {
    return ring_profile(JSON.stringify(req), pointsPerTooth);
  } catch {
    return null;
  }
}

export function ringDxf(req: RingRequest): { ok: string } | { error: string } {
  try {
    return { ok: export_ring_dxf(JSON.stringify(req)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

/** A geartrain as it is exchanged: the tab's name, and the train's inputs.
 *
 *  The name is in the document because a `Train` has none and a tab does, and
 *  recovering it from the filename would lose it to any rename. */
export interface TrainDocument {
  name: string;
  train: Train;
}

/** What reading a geartrain came to: the document, and whether Rust adjusted
 *  it on the way in — a toggle the file had given that no stage can honour,
 *  turned back automatic with its number kept. */
export interface Imported {
  document: TrainDocument;
  adjusted: boolean;
}

/** Parse an exported geartrain. The TOML never touches TypeScript: the file is
 *  handed to Rust as text, so exactly one parser exists. */
export function importTrain(
  tomlText: string,
): { ok: Imported } | { error: string } {
  try {
    return { ok: JSON.parse(import_train(tomlText)) as Imported };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function exportTrain(
  doc: TrainDocument,
): { ok: string } | { error: string } {
  try {
    return { ok: export_train(JSON.stringify(doc)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

// --------------------------------------------------------------------- //
//  Materials
// --------------------------------------------------------------------- //

export function defaultLibrary(): MaterialLibrary {
  return JSON.parse(default_materials()) as MaterialLibrary;
}

/** Parse a hand-edited library. The TOML never touches TypeScript: the file is
 *  read as text and handed straight to the one tested parser. */
export function importLibrary(
  tomlText: string,
): { ok: MaterialLibrary } | { error: string } {
  try {
    return { ok: JSON.parse(import_materials(tomlText)) as MaterialLibrary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function exportLibrary(
  lib: MaterialLibrary,
): { ok: string } | { error: string } {
  try {
    return { ok: export_materials(JSON.stringify(lib)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

// --------------------------------------------------------------------- //
//  Geartrains
// --------------------------------------------------------------------- //

/** **Resolve an over-determined stage**, whatever kind it is.
 *
 *  `just` is the input the designer has this moment pinned, and is never the one
 *  relieved; `null` where what changed was not a toggle — a shaft angle — and
 *  nothing is spared. Which inputs argue with each other, how many may stand
 *  and which gives way first are facts about the geometry, so Rust decides all
 *  of it — this used to be three functions here, one per stage kind, each
 *  restating a relation the core already enforces, and none of them tested.
 *
 *  Written **in place**, leaf by leaf, rather than by replacing the stage: the
 *  caller holds a reactive proxy and a wholesale swap would detach every input
 *  bound to it. Only leaves that differ are written, and this side does not
 *  know which they are — it used to list every field relief could touch by
 *  name, per kind, so a kind with a field named otherwise got no relief and
 *  nothing said so. Now the core hands back the stage as it should stand and
 *  the copy is shape-blind.
 *
 *  **A box relief turns given keeps the number it was showing.** The core seeds
 *  it from `figures` — what the stage's inputs last came to, by name, which
 *  `solveTrain` returns beside the result — so a helix pinned when its
 *  neighbour was freed holds the angle it had rather than a stale zero, exactly
 *  as the designer's own toggle does. This side forwards the list and never
 *  learns which figure is which.
 *
 *  A stage that will not cross the boundary is left alone. Relief runs on a
 *  click, and a click is not the place to discover a broken boundary.
 */
export function relieveStage(stage: Stage, just: Freedom | null, figures: Figure[] = []): void {
  let corrected: Stage;
  try {
    corrected = JSON.parse(relieve_stage(JSON.stringify({ stage, just, figures }))) as Stage;
  } catch {
    return;
  }
  assignLeaves(stage, corrected);
}

/** Write every leaf of `from` that differs into `into`, in place, shape-blind:
 *  the two are the same stage before and after relief, so they have the same
 *  shape, and only toggles and the numbers relief seeded can differ. */
function assignLeaves(into: Record<string, unknown>, from: Record<string, unknown>): void {
  for (const key of Object.keys(from)) {
    const a = into[key];
    const b = from[key];
    if (typeof a === "object" && a !== null && typeof b === "object" && b !== null) {
      assignLeaves(a as Record<string, unknown>, b as Record<string, unknown>);
    } else if (a !== b) {
      into[key] = b;
    }
  }
}

/** **One member of a geartrain, as a gear tab would hold it** — the tooth the
 *  stage cut, with whether it is a ring and the cutter that cut it. Rust
 *  solves the train to answer, because the tooth as built is an output: a
 *  shift the stage chose, an addendum a tip width held down, a helix shared
 *  out of a shaft angle. *Adopt*, not import: `importTrain` reads a document
 *  this tool wrote, and this reads a member of a train that is open. */
export function adoptMember(
  train: Train,
  stage: number,
  member: number,
  materials?: MaterialLibrary,
): AdoptOutcome | { error: string } {
  try {
    const body = JSON.stringify({ train, materials: materials ?? null, stage, member });
    return JSON.parse(adopt_member(body)) as AdoptOutcome;
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

/** A fresh geartrain, one spur stage in it. */
export function defaultTrain(): Train {
  return defaults().train;
}

/** Solve a whole train. The library is omitted unless the user changed it, in
 *  which case Rust uses the one it ships with. */
/** **A train that will not build is an answer, not an exception.**
 *
 *  A geartrain halfway through an edit is regularly one that cannot be built,
 *  and the panel has to go on showing every input that produced it — so the
 *  refusal comes back as a value, carrying a `Note` the catalogue renders and
 *  the stage it happened in. The `throw` path is for a boundary that broke:
 *  bad JSON, a panicking module, nothing a designer typed. */
export function solveTrain(train: Train, materials?: MaterialLibrary): TrainOutcome {
  try {
    const body = JSON.stringify({ train, materials: materials ?? null });
    return JSON.parse(solve_train(body)) as TrainOutcome;
  } catch (e) {
    return {
      result: null,
      failure: {
        // **This one is the front end's own**, so its words live in the `[ui]`
        // section rather than in `[error]`, which is the core's: nothing in
        // Rust can emit it, because it means the call into Rust did not return.
        note: {
          key: "ui.train_boundary_failed",
          values: { detail: e instanceof Error ? e.message : String(e) },
        },
        stage: null,
      },
      figures: [],
      topology: [],
      motion: null,
    };
  }
}

/** **What a train asks of one of a stage's shafts**, as it stands: the
 *  constraint the train states, or `null` where it states none and the
 *  kind's convention holds. Read here, never decided here — which shaft a
 *  set holds by convention is the core's, and arrives as
 *  `topology[stage].held_by_convention`. */
export function constraintOn(train: Train, stage: number, shaft: number): Constraint | null {
  const c = train.constraints.find(
    (c) => c.at.kind === "of" && c.at.stage === stage && c.at.shaft === shaft,
  );
  return c ? c.constraint : null;
}

/** **Tell one stage what drives it and what it holds**, by the core's rules
 *  rather than this file's. What is held is a constraint; where the load
 *  comes in is a constraint on the first stage and a *coupling* on every
 *  other, and the panel is not the place to know which — the core rewrites
 *  the train's constraints and, where it must, its couplings
 *  (`Train::arranged`). Takes a plain snapshot and gives the train back; the
 *  panel writes the two lists into its state, as it does a relieved stage. */
export function arrangeStage(train: Train, stage: number, driven: number, held: number): Train {
  return JSON.parse(arrange_stage(JSON.stringify({ train, stage, driven, held }))) as Train;
}

// The words live in `strings.svelte.ts` — it has to be a rune module, because
// the catalogue arrives after the first render. Re-exported here so a component
// still reaches everything through one door.
export { t, note, languages, language } from "./strings.svelte";
export type { Note, LanguageOption } from "./strings.svelte";
