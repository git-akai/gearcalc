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

/** A bound on one input, mirroring Rust's `Bound`. */
export interface Bound {
  min: number | null;
  max: number | null;
  exclusive_min: boolean;
  exclusive_max: boolean;
}

/** The profile shifts this gear can be built at, plus the design thresholds
 *  inside them. Every number here is computed in Rust; this side only compares
 *  and formats. */
export interface ShiftRange {
  bound: Bound;
  undercut: number;
  sharp_rack_undercut: number;
  pointed: number | null;
}

/** Every bound on a gear's inputs — including the ones that do not vary.
 *
 *  There are deliberately no numeric limits anywhere in this file: a limit
 *  written here would be a second place it could be changed, and a second place
 *  it could be wrong. Rust decides them all. */
export interface Ranges {
  module: Bound;
  pressure_angle: Bound;
  teeth: Bound;
  helix_angle: Bound;
  thickness_mod: Bound;
  profile_shift: ShiftRange;
  addendum: Bound;
  dedendum: Bound;
  root_radius: Bound;
}

/** Why `v` is outside `b`, or null if it is inside. Comparison only. */
export function outside(v: number, b: Bound): string | null {
  if (!Number.isFinite(v)) return "must be a number";
  if (b.min !== null && (b.exclusive_min ? v <= b.min : v < b.min)) {
    return `must be ${b.exclusive_min ? "greater than" : "at least"} ${b.min}`;
  }
  if (b.max !== null && (b.exclusive_max ? v >= b.max : v > b.max)) {
    return `must be ${b.exclusive_max ? "less than" : "at most"} ${b.max}`;
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
    default:
      return null;
  }
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
  /** shown under the field */
  note?: string;
}

export const FIELDS: FieldSpec[] = [
  { key: "module", label: "Normal module", unit: "mm", step: 0.1 },
  { key: "pressure_angle", label: "Pressure angle", unit: "°", step: 0.5 },
  { key: "teeth", label: "Tooth count", unit: "", step: 1, integer: true },
  { key: "helix_angle", label: "Helix angle", unit: "°", step: 1 },
  { key: "profile_shift", label: "Profile shift", unit: "module", step: 0.05 },
  { key: "addendum", label: "Addendum", unit: "module", step: 0.05 },
  { key: "dedendum", label: "Dedendum", unit: "module", step: 0.05 },
  { key: "root_radius", label: "Root radius coefficient", unit: "module", step: 0.01 },
  {
    key: "thickness_mod",
    label: "Tooth thickness modification",
    unit: "",
    step: 0.05,
    note: "1 is the standard rack; a meshing pair must sum to 2",
  },
];

/** Why a value is not acceptable, given the bound Rust returned. */
export function validate(f: FieldSpec, v: number, b: Bound | null): string | null {
  if (f.integer && !Number.isInteger(v)) return "must be a whole number";
  return b === null ? (Number.isFinite(v) ? null : "must be a number") : outside(v, b);
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
export type Basis = "overridden" | "datasheet" | "derived" | "chart" | "estimated";

/** What the ultimate allowable measures. Glass-filled grades have no yield. */
export type Measure = "yield" | "break";

export type MaterialClass = "steel" | "brass" | "pom" | "polyamide";

/** One property: its value and its provenance. One number, because an entry
 *  describes a material in one state — the `condition` field names it. */
export interface MaterialValue {
  value: number;
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

/** Per-use replacements for a material's properties. `null` means "as the
 *  library says". These live in the input state, not in the library. */
export interface Overrides {
  density: number | null;
  elastic_modulus: number | null;
  poissons_ratio: number | null;
  ultimate_allowable: number | null;
  fatigue_allowable: number | null;
}

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
  material_overrides: Overrides;
}

export interface SpurStage {
  kind: "spur";
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

export interface WormMember {
  face_width: number;
  material: string;
  material_overrides: Overrides;
}
export interface WormStage {
  kind: "worm";
  module: number;
  pressure_angle: number;
  shaft_angle: number;
  friction: number;
  starts: number;
  worm_pitch_diameter: number;
  wheel_teeth: number;
  centre_distance: Auto<number>;
  clearance: number;
  tolerance_plus: number;
  tolerance_minus: number;
  axial_clearance: number;
  worm: WormMember;
  wheel: WormMember;
}

/** A stage of either kind. The `kind` tag is what Rust's enum serialises as. */
export type Stage = SpurStage | WormStage;

export interface Train {
  input_speed: number;
  input_torque: number;
  actuation: Actuation;
  stages: Stage[];
}

export interface ContactRatios {
  transverse: number;
  overlap: number;
  total: number;
}
/** A quantity reported for both drive directions — see Rust's `Directional`. */
export interface Directional<T> {
  forward: T;
  backward: T;
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
  material: Material;
  ranges: Ranges;
}
export interface SpurResult {
  kind: "spur";
  ratio: number;
  centre_distance_nominal: number;
  centre_distance: number;
  contact_ratios: ContactRatios;
  efficiency: Directional<number>;
  backlash: Directional<Backlash>;
  coprime: boolean;
  gears: [GearResult, GearResult];
  notes: string[];
}
export interface WormMemberResult {
  torque: number;
  speed: number;
  tooth_cycles: number;
  face_width: number;
  pitch_diameter: number;
  material: Material;
}
export interface WormContact {
  max_pressure: number;
  patch_length: number;
  patch_width: number;
  curvature_along: number;
  curvature_across: number;
}
export interface WormResult {
  kind: "worm";
  ratio: number;
  centre_distance_nominal: number;
  centre_distance: number;
  lead_angle: number;
  wheel_lead_angle: number;
  wheel_helix_angle: number;
  lead: number;
  axial_module: number;
  efficiency: Directional<number>;
  self_locking_friction: number;
  sliding_ratio: number;
  sliding_velocity: number;
  contact: WormContact;
  backlash: Directional<Backlash>;
  members: [WormMemberResult, WormMemberResult];
  notes: string[];
}

/**
 * What a stage produced. Each kind keeps its own shape — a worm stage has no
 * bending stress and two efficiencies — so this is a tagged union rather than
 * one interface with everything optional.
 */
export type StageResult = SpurResult | WormResult;

export interface TrainResult {
  total_ratio: number;
  output_speed: number;
  output_torque: number;
  total_efficiency: Directional<number>;
  backlash: Directional<Backlash>;
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
    material_overrides: {
      density: null,
      elastic_modulus: null,
      poissons_ratio: null,
      ultimate_allowable: null,
      fatigue_allowable: null,
    },
  };
}

export function defaultWormStage(): WormStage {
  const member = (material: string): WormMember => ({
    face_width: 10,
    material,
    material_overrides: {
      density: null,
      elastic_modulus: null,
      poissons_ratio: null,
      ultimate_allowable: null,
      fatigue_allowable: null,
    },
  });
  return {
    kind: "worm",
    module: 1,
    pressure_angle: 20,
    shaft_angle: 90,
    friction: 0.06,
    starts: 1,
    worm_pitch_diameter: 7,
    wheel_teeth: 40,
    centre_distance: { auto: true, manual: 0 },
    clearance: 0.02,
    tolerance_plus: 0.02,
    tolerance_minus: 0.02,
    axial_clearance: 0.04,
    worm: member("4340 Hardened Steel"),
    wheel: member("Brass C360"),
  };
}
export function defaultStage(): SpurStage {
  return {
    kind: "spur",
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
