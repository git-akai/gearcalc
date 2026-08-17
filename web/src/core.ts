// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

import init, {
  solve_gear,
  solve_train,
  gear_profile,
  export_dxf,
  version,
  default_materials,
  import_materials,
  export_materials,
} from "./wasm/gear_wasm.js";

export interface GearParams {
  module: number;
  pressure_angle: number;
  teeth: number;
  profile_shift: number;
  helix_angle: number;
  addendum: number;
  dedendum: number;
  root_radius: number;
  thickness_mod: number;
}

export interface ClassRef {
  scale: "fine" | "standard";
  grade: number;
}

export interface GearRequest {
  params: GearParams;
  /** Depth, in modules, at which the undercut question is asked. */
  working_depth?: number;
  pin_diameter?: number | null;
  tolerance_class?: ClassRef | null;
  chord_tolerance?: number;
  reference_circles?: boolean;
}

/** A value, or the reason there isn't one. Mirrors the Rust `Maybe`. */
export type Maybe<T> = T | { unavailable: string };

export function isUnavailable<T>(v: Maybe<T>): v is { unavailable: string } {
  return v !== null && typeof v === "object" && "unavailable" in v;
}

export interface SpanOut {
  teeth_spanned: number;
  nominal: number;
  contact_radius: number;
}
export interface PinsOut {
  nominal: number;
  pin_centre_radius: number;
  contact_radius: number;
}
export interface ToleranceOut {
  class: ClassRef;
  tooth_to_tooth: number;
  total: number;
}

/** The profile shifts this gear can be built at, plus the design thresholds
 *  inside them. Every number here is computed in Rust; this side only compares
 *  and formats. */
export interface ShiftRange {
  min: number;
  max: number;
  undercut: number;
  sharp_rack_undercut: number;
  pointed: number | null;
}

/** A bound the geometry imposes; either side absent means unbounded. */
export interface Bound {
  min: number | null;
  max: number | null;
}

/** Every input range this gear's own geometry decides. Fields not listed here
 *  have bounds that do not vary, and stay as constants in FIELDS. */
export interface Ranges {
  profile_shift: ShiftRange;
  addendum: Bound;
  dedendum: Bound;
  root_radius: Bound;
}

export interface GearSummary {
  ranges: Ranges;
  pitch_radius: number;
  base_radius: number;
  tip_radius: number;
  root_radius: number;
  tooth_thickness: number;
  fillet_radius: number;
  transverse_pressure_angle: number;
  cutter_tip_width: number;
  undercut: boolean;
  severed: boolean;
  clamps: string[];
  span: Maybe<SpanOut>;
  over_two_pins: Maybe<PinsOut>;
  over_three_pins: Maybe<PinsOut>;
  available_classes: ClassRef[];
  tolerance: Maybe<ToleranceOut>;
}

/** Defaults from the specification, which differ from gear-core's library
 *  default (a deliberately well-behaved z=17 reference gear). */
export const defaultParams: GearParams = {
  module: 1,
  pressure_angle: 20,
  teeth: 9,
  profile_shift: 0,
  helix_angle: 0,
  addendum: 1,
  dedendum: 1.25,
  root_radius: 0.38,
  thickness_mod: 1,
};

// --------------------------------------------------------------------- //
//  Field definitions — valid ranges declared once, as data, next to the
//  field they describe. The UI renders from this and validates from this,
//  so a range cannot drift between the two.
// --------------------------------------------------------------------- //

export interface FieldSpec {
  key: keyof GearParams;
  label: string;
  unit: string;
  step: number;
  integer?: boolean;
  min?: number;
  max?: number;
  /** true when the bound itself is not allowed */
  exclusiveMin?: boolean;
  exclusiveMax?: boolean;
  /** shown under the field */
  note?: string;
}

export const FIELDS: FieldSpec[] = [
  { key: "module", label: "Normal module", unit: "mm", step: 0.1, min: 0, exclusiveMin: true },
  // The bounds below are the mathematical ones, not the specification's
  // conventional ones. alpha -> 0 sends the thickness-equivalent shift to
  // infinity and alpha -> 90 collapses the base circle; |beta| -> 90 sends the
  // transverse module to infinity. Everything strictly inside generates a real
  // gear, however peculiar — see crates/gear-core/tests/extremes.rs.
  {
    key: "pressure_angle",
    label: "Pressure angle",
    unit: "°",
    step: 0.5,
    min: 0,
    max: 90,
    exclusiveMin: true,
    exclusiveMax: true,
  },
  { key: "teeth", label: "Tooth count", unit: "", step: 1, integer: true, min: 1 },
  {
    key: "helix_angle",
    label: "Helix angle",
    unit: "°",
    step: 1,
    min: -90,
    max: 90,
    exclusiveMin: true,
    exclusiveMax: true,
  },
  // No fixed range: the real bound depends on dedendum, pressure angle and
  // thickness modification, so it comes back from Rust per gear and is applied
  // in GearPanel. See DESIGN.md §4.3.
  { key: "profile_shift", label: "Profile shift", unit: "module", step: 0.05 },
  // Bounded by the geometry, not by a constant: the tooth must have positive
  // height, the root circle must stay off the axis, and the fillet must fit the
  // space. Rust returns all three per gear; GearPanel applies them.
  { key: "addendum", label: "Addendum", unit: "module", step: 0.05 },
  { key: "dedendum", label: "Dedendum", unit: "module", step: 0.05 },
  { key: "root_radius", label: "Root radius coefficient", unit: "module", step: 0.01 },
  {
    key: "thickness_mod",
    label: "Tooth thickness modification",
    unit: "",
    step: 0.05,
    min: 0,
    max: 2,
    exclusiveMin: true,
    exclusiveMax: true,
    note: "1 is the standard rack; a meshing pair must sum to 2",
  },
];

/** Why a value is not acceptable, or null if it is. */
export function validate(f: FieldSpec, v: number): string | null {
  if (!Number.isFinite(v)) return "must be a number";
  if (f.integer && !Number.isInteger(v)) return "must be a whole number";
  if (f.min !== undefined) {
    if (f.exclusiveMin ? v <= f.min : v < f.min) {
      return `must be ${f.exclusiveMin ? "greater than" : "at least"} ${f.min}`;
    }
  }
  if (f.max !== undefined) {
    if (f.exclusiveMax ? v >= f.max : v > f.max) {
      return `must be ${f.exclusiveMax ? "less than" : "at most"} ${f.max}`;
    }
  }
  return null;
}

// --------------------------------------------------------------------- //

let ready: Promise<void> | null = null;

/** Load the core once. Safe to await repeatedly. */
export function loadCore(): Promise<void> {
  if (!ready) ready = init().then(() => undefined);
  return ready;
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

/** How far a published value can be trusted. Mirrors the Rust `Basis`. */
export type Basis = "datasheet" | "derived" | "chart" | "estimated";

/** What the ultimate allowable measures. Glass-filled grades have no yield. */
export type Measure = "yield" | "break";

export type MaterialClass = "steel" | "brass" | "pom" | "polyamide";

/** One property: its value, its moisture states, and its provenance.
 *
 *  There is deliberately no `effective(v)` helper on this side. Choosing
 *  between `dry` and `conditioned` is an engineering decision, not formatting,
 *  and the project rule keeps those in Rust — see DESIGN.md §6.3. When a
 *  material property needs displaying as a single number, Rust will send that
 *  number. */
export interface MaterialValue {
  dry: number;
  conditioned?: number;
  basis: Basis;
  note?: string;
}

export interface Material {
  name: string;
  class: MaterialClass;
  grade: string;
  condition: string;
  source: string;
  density: MaterialValue;
  elastic_modulus: MaterialValue;
  poissons_ratio: MaterialValue;
  ultimate_allowable: MaterialValue;
  ultimate_measure: Measure;
  fatigue_allowable: MaterialValue;
}

/** The array is named `material` because that is what the TOML calls it, and
 *  the two formats are kept in step deliberately. */
export interface MaterialLibrary {
  material: Material[];
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

/** A value the solver can work out, or that you set. Mirrors Rust's `Auto<T>`. */
export interface Auto<T> {
  auto: boolean;
  manual: T;
}

export type Actuation =
  | { intermittent: { range_degrees: number; actuations: number } }
  | { continuous: { operating_percent: number; runtime_hours: number } };

export interface StageGear {
  teeth: number;
  profile_shift: Auto<number>;
  working_depth: number;
  addendum: Auto<number>;
  min_tip_width: number;
  dedendum: number;
  root_radius: number;
  face_width: Auto<number>;
  auto_face_from_bending: boolean;
  auto_face_from_contact: boolean;
  material: string;
}

export interface SpurStage {
  module: number;
  pressure_angle: number;
  helix_angle: number;
  friction: number;
  thickness_mod: number;
  centre_distance: Auto<number>;
  clearance: number;
  tolerance_plus: number;
  tolerance_minus: number;
  gears: [StageGear, StageGear];
}

export interface Train {
  input_speed: number;
  input_torque: number;
  actuation: Actuation;
  stages: SpurStage[];
}

export interface ContactRatios {
  transverse: number;
  overlap: number;
  total: number;
}
export interface Backlash {
  nominal: number;
  minimum: number;
  maximum: number;
}
export interface GearResult {
  profile_shift: number;
  addendum: number;
  face_width: number;
  torque: number;
  speed: number;
  tooth_cycles: number;
  bending_stress: number | null;
  contact_stress: number;
  min_face_width_bending: number | null;
  min_face_width_contact: number;
  clamps: string[];
}
export interface StageResult {
  ratio: number;
  centre_distance_nominal: number;
  centre_distance: number;
  contact_ratios: ContactRatios;
  efficiency: number;
  backlash: [Backlash, Backlash];
  coprime: boolean;
  gears: [GearResult, GearResult];
  notes: string[];
}
export interface TrainResult {
  total_ratio: number;
  output_speed: number;
  output_torque: number;
  total_efficiency: number;
  output_backlash: Backlash;
  stages: StageResult[];
}

/** Defaults from the specification. */
export function defaultStageGear(teeth: number): StageGear {
  return {
    teeth,
    profile_shift: { auto: true, manual: 0 },
    working_depth: 1,
    addendum: { auto: false, manual: 1 },
    min_tip_width: 0.1,
    dedendum: 1.25,
    root_radius: 0.38,
    face_width: { auto: true, manual: 5 },
    auto_face_from_bending: true,
    auto_face_from_contact: true,
    material: "4340 Hardened Steel",
  };
}

export function defaultStage(): SpurStage {
  return {
    module: 1,
    pressure_angle: 20,
    helix_angle: 0,
    friction: 0.06,
    thickness_mod: 1,
    centre_distance: { auto: true, manual: 0 },
    clearance: 0.02,
    tolerance_plus: 0.02,
    tolerance_minus: 0.02,
    gears: [defaultStageGear(17), defaultStageGear(43)],
  };
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
