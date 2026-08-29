// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

import { setCatalogue, t, type Note } from "./strings.svelte";
import type {
  Actuation,
  Arrangement,
  Auto,
  Backlash,
  Basis,
  Bound,
  CentreProfile,
  ClassRef,
  ContactRatios,
  CrossedMesh,
  Cutter,
  CutterRef,
  Defaults,
  Directional,
  FirstMemberSizing,
  GearParams,
  GearRequest,
  GearResult,
  GearSummary,
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
  SpurResult,
  SpurStage,
  Stage,
  StageGear,
  StageResult,
  ToleranceOut,
  Train,
  TrainResult,
  Variation,
  WormContact,
  WormMember,
  WormMemberResult,
  WormResult,
  WormStage,
} from "./wire";
export type {
  Actuation,
  Arrangement,
  Auto,
  Backlash,
  Basis,
  Bound,
  CentreProfile,
  ClassRef,
  ContactRatios,
  CrossedMesh,
  Cutter,
  CutterRef,
  Defaults,
  Directional,
  FirstMemberSizing,
  GearParams,
  GearRequest,
  GearResult,
  GearSummary,
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
  SpurResult,
  SpurStage,
  Stage,
  StageGear,
  StageResult,
  ToleranceOut,
  Train,
  TrainResult,
  Variation,
  WormContact,
  WormMember,
  WormMemberResult,
  WormResult,
  WormStage,
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
  default_materials,
  import_materials,
  export_materials,
  import_train,
  export_train,
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
    const key = b.exclusive_min ? "ui.validation_greater_than" : "ui.validation_at_least";
    return t(key, { bound: String(b.min) });
  }
  if (b.max !== null && (b.exclusive_max ? v >= b.max : v > b.max)) {
    const key = b.exclusive_max ? "ui.validation_less_than" : "ui.validation_at_most";
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
  { key: "module", label: "ui.gear_field_module", unit: "ui.gear_mm", step: 0.1 },
  { key: "pressure_angle", label: "ui.gear_field_pressure_angle", unit: "ui.gear_deg", step: 0.5 },
  { key: "teeth", label: "ui.gear_field_teeth", unit: "", step: 1, integer: true },
  { key: "helix_angle", label: "ui.gear_field_helix_angle", unit: "ui.gear_deg", step: 1 },
  { key: "profile_shift", label: "ui.gear_field_profile_shift", unit: "ui.gear_m", step: 0.05 },
  { key: "addendum", label: "ui.gear_field_addendum", unit: "ui.gear_m", step: 0.05 },
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
export function validate(f: FieldSpec, v: number, b: Bound | null): string | null {
  if (f.integer && !Number.isInteger(v)) return t("ui.validation_not_a_whole_number");
  return b === null
    ? Number.isFinite(v)
      ? null
      : t("ui.validation_not_a_number")
    : outside(v, b);
}

// --------------------------------------------------------------------- //

let ready: Promise<void> | null = null;
let cachedDefaults: Defaults | null = null;

/** Load the core once. Safe to await repeatedly. */
export function loadCore(): Promise<void> {
  if (!ready) {
    ready = init().then(() => {
      cachedDefaults = JSON.parse(wasm_defaults()) as Defaults;
      setCatalogue(JSON.parse(wasm_strings()) as Record<string, string>);
    });
  }
  return ready;
}


/** The defaults, as a fresh copy: everything handed out here is about to
 *  become a tab's mutable state, so callers must not share one object. */
export function defaults(): Defaults {
  if (!cachedDefaults) {
    throw new Error("the defaults were asked for before the core finished loading");
  }
  return structuredClone(cachedDefaults);
}

export function coreVersion(): string {
  return version();
}

export function solve(req: GearRequest): { ok: GearSummary } | { error: string } {
  try {
    return { ok: JSON.parse(solve_gear(JSON.stringify(req))) as GearSummary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function profile(req: GearRequest, pointsPerTooth: number): Float64Array | null {
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

// --------------------------------------------------------------------- //
//  Materials
// --------------------------------------------------------------------- //

export type MaterialClass = "steel" | "brass" | "pom" | "polyamide";

/** One property: its value and its provenance. One number, because an entry
 *  describes a material in one state — the `condition` field names it. */
export function defaultParams(): GearParams {
  return defaults().gear.params;
}

export function defaultCutter(): CutterRef {
  return defaults().gear.cutter;
}

export function solveRing(req: RingRequest): { ok: RingSummary } | { error: string } {
  try {
    return { ok: JSON.parse(solve_ring(JSON.stringify(req))) as RingSummary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function ringProfile(req: RingRequest, pointsPerTooth: number): Float64Array | null {
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

/** Parse an exported geartrain. The TOML never touches TypeScript: the file is
 *  handed to Rust as text, so exactly one parser exists. */
export function importTrain(tomlText: string): { ok: TrainDocument } | { error: string } {
  try {
    return { ok: JSON.parse(import_train(tomlText)) as TrainDocument };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function exportTrain(doc: TrainDocument): { ok: string } | { error: string } {
  try {
    return { ok: export_train(JSON.stringify(doc)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function defaultLibrary(): MaterialLibrary {
  return JSON.parse(default_materials()) as MaterialLibrary;
}

/** Parse a hand-edited library. The TOML never touches TypeScript: the file is
 *  read as text and handed straight to the one tested parser. */
export function importLibrary(tomlText: string): { ok: MaterialLibrary } | { error: string } {
  try {
    return { ok: JSON.parse(import_materials(tomlText)) as MaterialLibrary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

export function exportLibrary(lib: MaterialLibrary): { ok: string } | { error: string } {
  try {
    return { ok: export_materials(JSON.stringify(lib)) };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

// --------------------------------------------------------------------- //
//  Geartrains
// --------------------------------------------------------------------- //

/** Which shaft of a planetary set. Mirrors Rust's `planetary::PlanetaryShaft`. */
export type PlanetaryMember = "sun" | "carrier" | "ring";

/** A material figure with its provenance, as Rust's `Value` serialises. */
export interface ProvenancedValue {
  value: number;
  basis: string;
  note: string | null;
}

// A default stage arrives **tagged** — Rust's `Stage` is an internally tagged
// enum, so the object carries its own `kind`. These used to be declared as the
// bare stage bodies, which typechecked only because TypeScript lets a wider
// value through; the tag was really there and the type said it was not.
export function defaultWormStage(): Stage {
  return defaults().worm_stage;
}

export function defaultPlanetaryStage(): Stage {
  return defaults().planetary_stage;
}

export function defaultSpurStage(): Stage {
  return defaults().spur_stage;
}

/** A fresh geartrain, one spur stage in it. */
export function defaultTrain(): Train {
  return defaults().train;
}

/** Solve a whole train. The library is omitted unless the user changed it, in
 *  which case Rust uses the one it ships with. */
export function solveTrain(
  train: Train,
  materials?: MaterialLibrary,
): { ok: TrainResult } | { error: string } {
  try {
    const body = JSON.stringify({ train, materials: materials ?? null });
    return { ok: JSON.parse(solve_train(body)) as TrainResult };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

// The words live in `strings.svelte.ts` — it has to be a rune module, because
// the catalogue arrives after the first render. Re-exported here so a component
// still reaches everything through one door.
export { t, note } from "./strings.svelte";
export type { Note } from "./strings.svelte";
