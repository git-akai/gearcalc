// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

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
  default_materials,
  import_materials,
  export_materials,
  import_train,
  export_train,
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
  /** Radii, mm — what the viewport draws with. */
  pitch_radius: number;
  base_radius: number;
  tip_radius: number;
  root_radius: number;
  /** The same circles as diameters, mm — how a gear is specified and gauged, so
   *  this is what the panel shows. Doubled in Rust, not here: the rule is that
   *  every number on screen is one Rust computed. */
  pitch_diameter: number;
  base_diameter: number;
  tip_diameter: number;
  root_diameter: number;
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
/** Everything a fresh tab starts at, as Rust holds it.
 *
 *  Not written down here. These are engineering numbers, and the one time they
 *  were kept in both languages the two drifted: this file's cutter carried the
 *  rack's 0.38 tip round, which no 20-tooth shaper can hold, so every ring the
 *  UI drew had no fillet at all. See DESIGN.md §12. */
export interface Defaults {
  gear: {
    params: GearParams;
    cutter: CutterRef;
    pin_diameter: number;
    chord_tolerance: number;
    reference_circles: boolean;
  };
  train: Train;
  spur_stage: SpurStage;
  worm_stage: WormStage;
  planetary_stage: PlanetaryStage;
}

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
  /**
   * Not an input for an internal gear, and hidden there.
   *
   * A ring's root circle is wherever its cutter's tip reaches and its fillet
   * round is the cutter's own, so both are properties of the tool rather than
   * of the part. Showing a box that changes nothing is worse than showing none.
   */
  externalOnly?: boolean;
  /** Replaces `note` for an internal gear, where the rule differs. */
  ringNote?: string;
}

export const FIELDS: FieldSpec[] = [
  { key: "module", label: "Normal module", unit: "mm", step: 0.1 },
  { key: "pressure_angle", label: "Pressure angle", unit: "°", step: 0.5 },
  { key: "teeth", label: "Tooth count", unit: "", step: 1, integer: true },
  { key: "helix_angle", label: "Helix angle", unit: "°", step: 1 },
  { key: "profile_shift", label: "Profile shift", unit: "module", step: 0.05 },
  { key: "addendum", label: "Addendum", unit: "module", step: 0.05 },
  { key: "dedendum", label: "Dedendum", unit: "module", step: 0.05, externalOnly: true },
  {
    key: "root_radius",
    label: "Root radius coefficient",
    unit: "module",
    step: 0.01,
    externalOnly: true,
  },
  {
    key: "thickness_mod",
    label: "Tooth thickness modification",
    unit: "",
    step: 0.05,
    note: "1 is the standard rack; a meshing pair must sum to 2",
    // On a ring it is the SPACE this describes, so a pinion and a ring that mesh
    // want the SAME k rather than complementary ones.
    ringNote: "1 is the standard rack; on a ring it widens the space, and a meshing pair matches",
  },
];

/** Why a value is not acceptable, given the bound Rust returned. */
export function validate(f: FieldSpec, v: number, b: Bound | null): string | null {
  if (f.integer && !Number.isInteger(v)) return "must be a whole number";
  return b === null ? (Number.isFinite(v) ? null : "must be a number") : outside(v, b);
}

// --------------------------------------------------------------------- //

let ready: Promise<void> | null = null;
let cachedDefaults: Defaults | null = null;

/** Load the core once. Safe to await repeatedly. */
export function loadCore(): Promise<void> {
  if (!ready) {
    ready = init().then(() => {
      cachedDefaults = JSON.parse(wasm_defaults()) as Defaults;
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

/** The pinion cutter that shapes a ring. Without one its fillet is undefined. */
export interface CutterRef {
  teeth: number;
  addendum: number;
  tip_round: number;
}
export interface RingRequest {
  params: GearParams;
  /** Pin or ball diameter for the between-pins measurement, mm. */
  pin_diameter?: number | null;
  cutter: CutterRef;
  chord_tolerance: number;
  reference_circles: boolean;
}
export interface RingSummary {
  /** Measurement between two pins — the internal counterpart of over-pins. The
   *  pin diameter subtracts here, because it is measured between inner
   *  surfaces rather than across outer ones. */
  between_pins: Maybe<PinsOut>;
  teeth: number;
  transverse_module: number;
  transverse_pressure_angle: number;
  pitch_radius: number;
  base_radius: number;
  tip_radius: number;
  root_radius: number;
  /** The same four as diameters, mm — what the panel shows. */
  pitch_diameter: number;
  base_diameter: number;
  tip_diameter: number;
  root_diameter: number;
  /** null when the cut generated no fillet: there is then no handover. */
  junction_radius: number | null;
  root_form: "fully_filleted" | "root_arc" | "no_fillet";
  /** Where the drawing shades the rim out to, mm. A convention, not a design
   *  output — the real outside diameter is the designer's. */
  rim_radius: number;
  generation_limit: number;
  fully_generated: boolean;
  smallest_tooth_count: number;
  clamps: string[];
}

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

/** Two gears on shafts at any angle: spur, helical, or — once the shafts are
 *  crossed — a crossed gear pair. One stage, as the specification has it, with
 *  the axis angle as the input that tells them apart. */
export interface SpurStage {
  kind: "spur";
  module: number;
  pressure_angle: number;
  /** Σ, degrees. Zero is a parallel-axis pair. */
  shaft_angle: number;
  /** What each gear carries beyond half the shaft angle; gear 2 takes it with
   *  the opposite sign, so at Σ = 0 this is the shared helix angle. */
  additional_helix: number;
  friction: number;
  thickness_mod: number;
  centre_distance: Auto<number>;
  clearance: number;
  tolerance_plus: number;
  tolerance_minus: number;
  gears: [StageGear, StageGear];
}

export interface WormMember {
  /** Automatic takes the conventional proportion for this member; see
   *  `recommended_face_width` on the result, and DESIGN §4.5.1 for what those
   *  proportions are and are not. */
  face_width: Auto<number>;
  material: string;
  material_overrides: Overrides;
}
/** How the first member's size is fixed — the only thing separating a worm drive
 *  from a crossed gear pair. A worm's diameter is a free choice and its lead
 *  angle follows; a gear's diameter follows from its tooth count and helix
 *  angle. Same geometry, opposite input. Mirrors Rust's `FirstMemberSizing`. */
export type FirstMemberSizing = { pitch_diameter: number } | { helix_angle: number };

export interface WormStage {
  kind: "worm";
  module: number;
  pressure_angle: number;
  shaft_angle: number;
  friction: number;
  starts: number;
  sizing: FirstMemberSizing;
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
/** Which shaft of a planetary set. Mirrors Rust's `planetary::Member`. */
export type PlanetaryMember = "sun" | "carrier" | "ring";

/** Which shaft drives and which is held.
 *
 *  Both are needed: naming only the driven shaft leaves the set undetermined,
 *  since a sun-driven set behaves quite differently with the ring held than with
 *  the carrier held. See DESIGN.md §8.1. */
export interface Arrangement {
  input: PlanetaryMember;
  fixed: PlanetaryMember;
}

export interface Cutter {
  teeth: number;
  addendum: number;
  tip_round: number;
}

export interface PlanetaryStage {
  kind: "planetary";
  module: number;
  pressure_angle: number;
  helix_angle: number;
  friction_sun_planet: number;
  friction_planet_ring: number;
  /** `k` for the sun. The planet takes `2 - k` and the ring takes the planet's,
   *  because an external pair must sum to two and an internal pair must match.
   *  One input, three consistent values. */
  thickness_mod: number;
  planets: number;
  arrangement: Arrangement;
  clearance: number;
  tolerance_plus: number;
  tolerance_minus: number;
  min_planet_clearance: number;
  cutter: Cutter;
  sun: StageGear;
  /** The planet's `profile_shift` is ignored: it is solved, not chosen. */
  planet: StageGear;
  /** The ring's `dedendum` and `root_radius` are ignored — a ring's root circle
   *  is where its cutter reaches. */
  ring: StageGear;
}

export type Stage = SpurStage | WormStage | PlanetaryStage;

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
  /** What the conventional proportion asks for, mm — reported whether or not it
   *  is what is in use, so a hand-set width can be read against it. Null for a
   *  crossed gear pair, which has no enveloping wheel for the proportions to
   *  describe. */
  recommended_face_width: number | null;
  pitch_diameter: number;
  material: Material;
}
export interface WormContact {
  /** The worst of the pitch point and the two single-pair boundaries. */
  max_pressure: number;
  /** What the pitch point alone would have said — kept so the difference is
   *  visible rather than asserted. */
  at_pitch_point: number;
  worst_position: number;
  patch_length: number;
  patch_width: number;
  curvature_along: number;
  curvature_across: number;
}
/** What the path of contact says — for a worm drive as well as a crossed gear
 *  pair, since both come from the same construction (§4.5.1 takes both flanks as
 *  involute helicoids on cylinders, which is where the stage's other numbers
 *  come from too). */
export interface CrossedMesh {
  contact_ratio: number;
  limited_by: "tips" | "face";
  /** The face width at which ε = 1 — a **geometric** minimum, not a strength
   *  one, and the label has to travel with the number. */
  face_width_for_continuity: [number, number] | null;
  axial_travel: [number, number];
  /** True when the tooth height was assumed rather than given: a worm stage has
   *  no addendum input, so its figures are a floor. */
  tooth_height_assumed: boolean;
  /** What the same teeth would give with their shafts parallel — the best the
   *  pair can be, since crossing shafts adds sliding. Null for a worm, which has
   *  no parallel-axis counterpart. */
  parallel_axis_efficiency: number | null;
}

export interface WormResult {
  kind: "worm";
  ratio: number;
  centre_distance_nominal: number;
  centre_distance: number;
  /** Null only when the path could not be built at all. */
  crossed: CrossedMesh | null;
  lead_angle: number;
  wheel_lead_angle: number;
  /** Helix angle of the first member, degrees — `90° − γ₁`, from Rust. */
  helix_angle: number;
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
export interface MeshReport {
  contact_ratios: ContactRatios;
  efficiency: Directional<number>;
  contact_stress: number;
  relative_radius: number;
  backlash: [Backlash, Backlash];
}

/** A material figure with its provenance, as Rust's `Value` serialises. */
export interface ProvenancedValue {
  value: number;
  basis: string;
  note: string | null;
}

export interface PlanetResult {
  gear: GearResult;
  profile_shift: number;
  shift_residual: number;
  speed_absolute: number;
  speed_relative: number;
  fully_reversed: boolean;
  reversed_allowable: ProvenancedValue;
  min_face_width_reversed: number | null;
}

export interface PlanetaryResult {
  kind: "planetary";
  arrangement: Arrangement;
  output: PlanetaryMember;
  ratio: number;
  centre_distance_nominal: number;
  centre_distance: number;
  fixed_carrier_efficiency: Directional<number>;
  efficiency: Directional<number>;
  /** Angular backlash at whichever shaft is the output, degrees. */
  backlash: Directional<Backlash>;
  /** `[sun, carrier, ring]`. The held shaft is exactly zero. */
  speeds: [number, number, number];
  /** `[sun, carrier, ring]`. They sum to zero. */
  torques: [number, number, number];
  sun_planet: MeshReport;
  planet_ring: MeshReport;
  equal_spacing: boolean;
  simultaneous_meshing: boolean;
  planet_clearance: number | null;
  planet_clearance_ok: boolean;
  sun_coprime_with_planets: boolean;
  ring_coprime_with_planets: boolean;
  sun: GearResult;
  planet: PlanetResult;
  ring: GearResult;
  planets: number;
  notes: string[];
}

export type StageResult = SpurResult | WormResult | PlanetaryResult;

export interface TrainResult {
  total_ratio: number;
  output_speed: number;
  output_torque: number;
  total_efficiency: Directional<number>;
  backlash: Directional<Backlash>;
  stages: StageResult[];
}

export function defaultWormStage(): WormStage {
  return defaults().worm_stage;
}

export function defaultPlanetaryStage(): PlanetaryStage {
  return defaults().planetary_stage;
}

export function defaultStage(): SpurStage {
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
