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
  CaseKind,
  LoadCase,
  Load,
  CaseFreedom,
  LoadFreedom,
  CaseBody,
  BodyRole,
  BodyReport,
  LoadRole,
  Auto,
  Backlash,
  Basis,
  Bound,
  CentreProfile,
  ClassRef,
  ContactRatios,
  LineContact,
  ContactPatch,
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
  Ranges,
  RingRequest,
  RingSummary,
  ShiftRange,
  SpanOut,
  Shape,
  ShapeResult,
  Member,
  MeshInput,
  Distance,
  DistanceReport,
  LayoutReport,
  SlotCase,
  Axis,
  BodyOn,
  StageGear,
  ToleranceOut,
  Train,
  TrainResult,
  TrainOutcome,
  AdoptOutcome,
  Adopted,
  TrainFailure,
  Variation,
  PortSpec,
  StagePorts,
  MemberName,
  MemberRole,
  MotionReport,
  Exact,
  StageFamily,
  StagePreset,
  StagePresetEntry,
  StageEdit,
} from "./wire";
export type { CaseKind, LoadCase };
export type {
  Duty,
  GearCase,
  MeshCase,
  TrainCase,
  Load,
  CaseFreedom,
  LoadFreedom,
  CaseBody,
  BodyRole,
  BodyReport,
  LoadRole,
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
  LineContact,
  ContactPatch,
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
  Ranges,
  RingRequest,
  RingSummary,
  ShiftRange,
  SpanOut,
  Shape,
  ShapeResult,
  Member,
  MeshInput,
  Distance,
  DistanceReport,
  LayoutReport,
  SlotCase,
  Axis,
  BodyOn,
  StageGear,
  ToleranceOut,
  Train,
  TrainResult,
  TrainOutcome,
  AdoptOutcome,
  Adopted,
  TrainFailure,
  Variation,
  PortSpec,
  StagePorts,
  MemberName,
  MemberRole,
  MotionReport,
  Exact,
  StageFamily,
  StagePreset,
  StagePresetEntry,
  StageEdit,
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
  relieve_case,
  edit_train,
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

/** **A preset over the one stage shape.** A stage is a `Shape` — axes,
 *  bodies, members, meshes and distances — and a preset is a shape the core
 *  pre-assembled at sensible teeth, listed under its family: the core's
 *  `StagePreset::ALL`, crossing in `defaults().stages` with the family and
 *  the catalogue key of its name, so the menu renders from that list and a
 *  new preset is a variant there, never a row here. A crossed pair is one of
 *  them for the menu's sake: it is a spur stage with its shafts at an angle,
 *  and a worm a distance marked as one, and neither is obvious to build from
 *  a pair — which is the whole reason a preset exists. */
/** The presets of one family, in the core's order; the families themselves
 *  are `defaults().families`, the core's list with the key of each name. */
export function presetsOf(family: StageFamily): StagePresetEntry[] {
  return defaults().stages.filter((e) => e.family === family);
}

export interface CaseKindSpec {
  key: CaseKind;
  /** Catalogue key for the kind's name — the chip on a heading, the option in
   *  the select. */
  label: string;
  /** Catalogue key for the button that adds one. */
  add: string;
}

/** The load case kinds, as data, for the reason the stage presets are: the "add
 *  load case" buttons and the kind select render from this, and a kind decides
 *  which allowable the core judges against and which inputs are put in front
 *  of the designer — nothing else. A fresh case of a kind is the core's
 *  ({@link editTrain} with `add_case`), since which bodies it is written
 *  between is the train's to say. */
export const CASE_KINDS: CaseKindSpec[] = [
  { key: "ultimate", label: "ui.train_case_ultimate", add: "ui.train_add_ultimate_case" },
  { key: "fatigue", label: "ui.train_case_fatigue", add: "ui.train_add_fatigue_case" },
];

/** **Where a load can enter**, in the order the chain runs: every body the
 *  core reports that a case may address and the train does not hold — one
 *  list of bodies, read rather than a second one sent. A `<select>` binds
 *  to strings, so a body's number is its key. */
export function portOptions(motion: MotionReport | null): BodyReport[] {
  return (motion?.bodies ?? []).filter((b) => b.port && !b.held);
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

/** **Resolve an over-determined stage**, whatever preset it came from.
 *
 *  `just` is the input the designer has this moment pinned, and is never the one
 *  relieved; `null` where what changed was not a toggle — a shaft angle — and
 *  nothing is spared. Which inputs argue with each other, how many may stand
 *  and which gives way first are facts about the geometry, so Rust decides all
 *  of it — this used to be three functions here, one per stage type, each
 *  restating a relation the core already enforces, and none of them tested.
 *
 *  Written **in place**, leaf by leaf, rather than by replacing the stage: the
 *  caller holds a reactive proxy and a wholesale swap would detach every input
 *  bound to it. Only leaves that differ are written, and this side does not
 *  know which they are — it used to list every field relief could touch by
 *  name, per type, so a type with a field named otherwise got no relief and
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
export function relieveStage(stage: Shape, just: Freedom | null, figures: Figure[] = []): void {
  let corrected: Shape;
  try {
    corrected = JSON.parse(relieve_stage(JSON.stringify({ stage, just, figures }))) as Shape;
  } catch {
    return;
  }
  assignLeaves(stage, corrected);
}

/** **A load case with its over-determined figures relieved**, the same
 *  relation `relieveStage` keeps on a stage's geometry kept on a case's
 *  loads: the train has some mobility, exactly that many of the case's
 *  speeds stand given and the torques one statics equation short of the
 *  bodies that carry one, and the figure just touched is the one that
 *  survives. Every figure relief turns derived is seeded by the core from
 *  what the case comes to, so a box shows the number rather than a stale
 *  one — this side copies the case back and never learns which is which.
 *
 *  Called after every change to a case's loads — a port loaded or released,
 *  a toggle flipped — with the library the train is rated under, so the
 *  seeding is the same solve the panel shows. A train that will not cross
 *  the boundary leaves the case as it stands. */
export function relieveCase(
  train: Train,
  index: number,
  just: CaseFreedom | null,
  materials?: MaterialLibrary,
): void {
  const c = train.load_cases[index];
  if (!c) return;
  let corrected: LoadCase;
  try {
    const library = materials ?? defaultLibrary();
    corrected = JSON.parse(relieve_case(JSON.stringify({ train, library, case: index, just }))) as LoadCase;
  } catch {
    return;
  }
  // Loads are a list and relief neither adds nor removes one, so each is
  // the same load before and after; only its toggles and seeded numbers move.
  corrected.loads.forEach((l, j) => assignLeaves(c.loads[j], l));
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

/** **Whether a train holds a body** — which is exactly what its list of
 *  holds says: every hold is stated, a preset's conventional one included. */
export function isHeld(train: Train, body: number): boolean {
  return train.held.includes(body);
}

/** **One edit to a train's graph, by the core's rules** — what a port's
 *  select and the panel's buttons mean: two bodies joined, a stage's end of
 *  a body split off, a body held or released, an end moved to another body
 *  (split, then held, joined or its own — the select's one rule), a stage
 *  pushed and joined onward with the cases carried to its far port, a fresh
 *  case added between the train's ends. Each is a rule about what else has
 *  to change — a join turns a reaction into a take-off, a body taken off
 *  its last stage leaves the train — and the rules are the core's, so this
 *  side hands the train over and copies the answer back. A train that will
 *  not cross the boundary is left as it stands. */
export type TrainEdit =
  | { join: { a: number; b: number } }
  | { split: { stage: number; body: number } }
  | { hold: number }
  | { release: number }
  | { move_end: { stage: number; body: number; to: number | null } }
  | { push_stage: Shape }
  | { remove_stage: number }
  | { add_case: CaseKind }
  | { duty: { case: number; intermittent: boolean } }
  | { stage: { stage: number; edit: StageEdit } };
/** The train edited by the core's rules, in place. A stage edit the core
 *  refuses leaves the train as it was and comes back as the catalogue key of
 *  the reason, for the panel to say; any other failure is a defect on this
 *  side of the boundary and is swallowed as before. */
export function editTrain(train: Train, edit: TrainEdit): string | null {
  let edited: Train;
  try {
    edited = JSON.parse(edit_train(JSON.stringify({ train, edit }))) as Train;
  } catch (e) {
    const message = e instanceof Error ? e.message : String(e);
    return message.startsWith("ui.") ? message : null;
  }
  train.stages = edited.stages;
  train.held = edited.held;
  train.load_cases = edited.load_cases;
  return null;
}

// The words live in `strings.svelte.ts` — it has to be a rune module, because
// the catalogue arrives after the first render. Re-exported here so a component
// still reaches everything through one door.
export { t, note, languages, language } from "./strings.svelte";
export type { Note, LanguageOption } from "./strings.svelte";
