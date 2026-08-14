// The only module that talks to WebAssembly.
//
// Project rule: no engineering calculation lives on this side of the boundary.
// Everything here either forwards inputs to Rust or formats what Rust returned.

import init, { solve_gear, gear_profile, version } from "./wasm/gear_wasm.js";

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
}

export const defaultParams: GearParams = {
  module: 1,
  pressure_angle: 20,
  teeth: 17,
  profile_shift: 0,
  helix_angle: 0,
  addendum: 1,
  dedendum: 1.25,
  root_radius: 0.38,
  thickness_mod: 1,
};

let ready: Promise<void> | null = null;

/** Load the core once. Safe to await repeatedly. */
export function loadCore(): Promise<void> {
  ready ??= init().then(() => undefined);
  return ready;
}

export function coreVersion(): string {
  return version();
}

/** Derived geometry, or the reason the input was rejected. */
export function solve(p: GearParams): { ok: GearSummary } | { error: string } {
  try {
    return { ok: JSON.parse(solve_gear(JSON.stringify(p))) as GearSummary };
  } catch (e) {
    return { error: e instanceof Error ? e.message : String(e) };
  }
}

/** Closed cross-section as a flat [x0, y0, x1, y1, ...] array. */
export function profile(p: GearParams, pointsPerTooth: number): Float64Array {
  return gear_profile(JSON.stringify(p), pointsPerTooth);
}
