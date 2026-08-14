// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

import init, { solve_gear, gear_profile, export_dxf, version } from "./wasm/gear_wasm.js";

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

export interface GearSummary {
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
  { key: "pressure_angle", label: "Pressure angle", unit: "°", step: 0.5, min: 10, max: 60 },
  { key: "teeth", label: "Tooth count", unit: "", step: 1, integer: true, min: 3 },
  { key: "helix_angle", label: "Helix angle", unit: "°", step: 1, min: -45, max: 45 },
  { key: "profile_shift", label: "Profile shift", unit: "module", step: 0.05, min: -2, max: 2 },
  { key: "addendum", label: "Addendum", unit: "module", step: 0.05, min: 0 },
  { key: "dedendum", label: "Dedendum", unit: "module", step: 0.05, min: 0 },
  { key: "root_radius", label: "Root radius coefficient", unit: "module", step: 0.01, min: 0 },
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
