// What crosses into the core, and what a request that cannot cross is called.
//
// `JSON.stringify` writes NaN and ±∞ as `null`, which serde refuses as the
// wrong type for the whole request. No box commits such a number
// (`NumberBox`); this is the guard behind that. It finds the number before it
// is sent and names it the way the panels do — the piece by the core's
// numbering and roles (`members.ts`), the field by its label — so the refusal
// says which box, not which JSON leaf.

import { note, t } from "./strings.svelte";
import type { Note } from "./wire";
import { bodyName, gearLabel } from "./members";
import { FIELDS, solveTrain, type Train } from "./core";

type Path = (string | number)[];

/** A request holding a number that is not finite, at `path` within `root`. */
export class NotFinite extends Error {
  constructor(
    readonly path: Path,
    readonly root: unknown,
  ) {
    super(path.join("."));
  }
}

/** Where in `v` the first non-finite number is, or null. */
function nonFinite(v: unknown, path: Path = []): Path | null {
  if (typeof v === "number") return Number.isFinite(v) ? null : path;
  if (typeof v !== "object" || v === null) return null;
  for (const [k, x] of Object.entries(v)) {
    const found = nonFinite(x, [...path, Array.isArray(v) ? Number(k) : k]);
    if (found) return found;
  }
  return null;
}

/** A request as it crosses: JSON, refusing a number that is not finite. */
export function wire(v: unknown): string {
  const path = nonFinite(v);
  if (path) throw new NotFinite(path, v);
  return JSON.stringify(v);
}

/** Every number field an input box writes, by its name on the wire, and the
 *  catalogue key of the label that box carries. The gear tab's own
 *  parameters are `FIELDS`'. */
const LABELS: Record<string, string> = {
  // A train member's gear, and the member.
  teeth: "ui.train_tooth_count",
  profile_shift: "ui.train_profile_shift",
  addendum: "ui.train_addendum",
  dedendum: "ui.train_dedendum",
  root_radius: "ui.train_root_radius",
  min_tip_width: "ui.train_minimum_tip_width",
  working_depth: "ui.train_working_tooth_depth",
  face_width: "ui.train_face_width",
  helix_angle: "ui.train_helix_angle",
  module: "ui.train_normal_module",
  pressure_angle: "ui.train_pressure_angle",
  thickness_mod: "ui.train_tooth_thickness_mod",
  pitch_diameter: "ui.train_pitch_diameter",
  density: "ui.train_density",
  elastic_modulus: "ui.train_elastic_modulus",
  poissons_ratio: "ui.train_poissons_ratio",
  ultimate_allowable: "ui.train_ultimate_allowable",
  fatigue_allowable: "ui.train_fatigue_allowable",
  contact_fatigue_allowable: "ui.train_contact_fatigue_allowable",
  // A mesh.
  sliding_friction: "ui.train_sliding_friction",
  static_friction: "ui.train_static_friction",
  min_contact_ratio: "ui.train_min_contact_ratio",
  overlap: "ui.train_overlap",
  // An axis distance.
  angle: "ui.train_axis_angle",
  distance: "ui.train_distance",
  clearance: "ui.train_distance_clearance",
  tip_clearance: "ui.train_tip_gap",
  tolerance_plus: "ui.train_distance_tolerance_plus",
  tolerance_minus: "ui.train_distance_tolerance_minus",
  axial_clearance: "ui.train_worm_axial_clearance",
  // An axis.
  count: "ui.train_planets",
  min_planet_clearance: "ui.train_minimum_planet_clearance",
  // A load case.
  application_factor: "ui.train_application_factor",
  range_degrees: "ui.train_actuation_range",
  actuations: "ui.train_actuation_count",
  runtime_hours: "ui.train_runtime",
  torque: "ui.train_torque",
  speed: "ui.train_speed",
  // The gear tab's own.
  pin_diameter: "ui.gear_pin_ball_diameter",
  chord_tolerance: "ui.gear_chord_tolerance",
  eccentric_throw: "ui.gear_axis_distance_throw",
};
/** A ring's cutter, whose fields share names with the gear's. */
const CUTTER: Record<string, string> = {
  teeth: "ui.train_cutter_teeth",
  addendum: "ui.train_cutter_addendum",
  tip_round: "ui.train_cutter_tip_round",
};
const MATE: Record<string, string> = {
  teeth: "ui.gear_mate_teeth",
  profile_shift: "ui.gear_mate_profile_shift",
};

/** The field at `path`, by its box's label: the last name on the path that
 *  is not an `Auto`'s own. */
function fieldLabel(path: Path): string | null {
  const names = path.filter((p): p is string => typeof p === "string" && p !== "manual");
  const leaf = names[names.length - 1];
  const within = names[names.length - 2];
  if (leaf === undefined) return null;
  if (within === "ring" || within === "cutter") return CUTTER[leaf] ? t(CUTTER[leaf]) : null;
  if (within === "mate") return MATE[leaf] ? t(MATE[leaf]) : null;
  if (within === "params") {
    const f = FIELDS.find((f) => f.key === leaf);
    if (f) return t(f.label);
  }
  return LABELS[leaf] ? t(LABELS[leaf]) : null;
}

/** The train `root` carries, if it is a train's request. */
function trainOf(root: unknown): Train | null {
  const r = root as { train?: Train; shape?: Train["shape"] };
  if (r?.train?.shape) return r.train;
  if (r?.shape) return { shape: r.shape, held: [], load_cases: [] } as unknown as Train;
  return null;
}

/** The copy of `v` with every non-finite number made 0, which crosses: a
 *  gear's name is read off the graph and does not depend on the number. */
function finite<T>(v: T): T {
  return JSON.parse(JSON.stringify(v, (_, x) => (typeof x === "number" && !Number.isFinite(x) ? 0 : x)));
}

/** The piece of `train` that `path` is in, named as the panel names it. */
function pieceName(train: Train, path: Path): string | null {
  const at = (key: string) => {
    const i = path.indexOf(key);
    return i >= 0 && typeof path[i + 1] === "number" ? (path[i + 1] as number) : null;
  };
  const shape = train.shape;
  const axis = (a: number) => t("ui.train_axis_name", { number: String(a + 1) });
  const names = () => solveTrain(finite(train)).names;
  const member = at("members");
  if (member !== null) return gearLabel(names(), member);
  const mesh = at("meshes");
  if (mesh !== null) {
    const m = shape.meshes[mesh];
    const n = names();
    return t("ui.train_mesh_heading", { a: gearLabel(n, m.a), b: gearLabel(n, m.b) });
  }
  const distance = at("distances");
  if (distance !== null) {
    const [a, b] = shape.distances[distance].axes;
    return t("ui.train_distance_between", { a: axis(a), b: axis(b) });
  }
  const ax = at("axes");
  if (ax !== null) return axis(ax);
  const c = at("load_cases");
  if (c !== null) {
    const load = at("loads");
    const name = t("ui.train_case_heading", { number: String(c + 1) });
    return load === null ? name : `${name} · ${bodyName(train.load_cases[c].loads[load].at)}`;
  }
  return null;
}

/** **A refusal that crossed as a note** — a file of another format, a graph
 *  that describes no train, a value the core's table refuses by the field it
 *  names — in the catalogue's words; a parser's complaint, which names the
 *  line, as it came. */
export function said(message: string): string {
  try {
    const n = JSON.parse(message) as Note;
    if (typeof n?.key === "string") return note(n);
  } catch {
    // Not a note: the parser's own words.
  }
  return message;
}

/** **Why a call into the core failed**, as the catalogue says it: a number
 *  that would not cross, named; a refusal that crossed as a note, in its
 *  words; any other failure, as it came. */
export function failureDetail(e: unknown): string {
  if (e instanceof NotFinite) {
    const train = trainOf(e.root);
    const inner = train && e.path[0] === "train" ? e.path.slice(1) : e.path;
    const piece = train ? pieceName(train, inner) : null;
    const field = fieldLabel(e.path);
    const named = [piece, field].filter((x) => x !== null).join(" · ");
    return t("ui.boundary_not_finite", { field: named || t("ui.boundary_an_input") });
  }
  return said(e instanceof Error ? e.message : String(e));
}
