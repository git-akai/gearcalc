// What an offer is called.
//
// The core lists the edits a piece of the train offers and tries each one
// (`offersAt`); this names them, in the reader's language, from the names the
// panel already gives a gear, a body, an axis, a mesh and a preset. Nothing
// here decides what can be done, or whether: an offer arrives with its edit
// and, where it would be refused, the refusal's key.

import { t, type Edit, type Offer, type StagePreset, type Target } from "./core";

/** The panel's names for the pieces an edit points at, by the graph's
 *  index — a body by its number. */
export interface Names {
  gear: (i: number) => string;
  body: (b: number) => string;
  axis: (a: number) => string;
  mesh: (k: number) => string;
  distance: (d: number) => string;
  preset: (p: StagePreset) => string;
  /** The family a preset is listed under, by its label. */
  family: (p: StagePreset) => string;
}

/** **Whether an edit adds to the train** — what the one add menu lists —
 *  rather than moving, joining, holding or removing what is there, which a
 *  piece's strip of verbs does. */
export function adds(e: Edit): boolean {
  return "add_gear" in e || "add_ratio" in e || "add_step" in e || "couple" in e || "insert" in e;
}

/** **Where a move or a join goes**, for an entry listed under the verb
 *  that says what is moved or joined — `null` for any other edit. */
export function destination(e: Edit, n: Names): string | null {
  if ("move" in e) return e.move.to === null ? t("ui.train_offer_a_body_of_its_own") : n.body(e.move.to);
  if ("join" in e) return n.body(e.join.b);
  return null;
}

/** **An offer, said whole** — what it adds, moves, joins, holds or removes,
 *  and where. */
export function offerLabel(o: Offer, n: Names): string {
  const e = o.edit;
  if ("add_gear" in e) {
    const { mate, on, ring } = e.add_gear;
    const gear = n.gear(mate);
    if (on === "new_axis") {
      return ring
        ? t("ui.train_offer_ring_new_axis", { mate: gear })
        : t("ui.train_offer_gear_new_axis", { mate: gear });
    }
    if ("new_body" in on) {
      const axis = n.axis(on.new_body);
      return ring
        ? t("ui.train_offer_ring_new_body", { mate: gear, axis })
        : t("ui.train_offer_gear_new_body", { mate: gear, axis });
    }
    const body = n.body(on.body);
    return ring
      ? t("ui.train_offer_ring_on_body", { mate: gear, body })
      : t("ui.train_offer_gear_on_body", { mate: gear, body });
  }
  if ("add_ratio" in e) {
    const { distance, shared } = e.add_ratio;
    return t("ui.train_offer_ratio", { distance: n.distance(distance), body: n.body(shared) });
  }
  if ("add_step" in e) return t("ui.train_offer_step", { axis: n.axis(e.add_step.axis) });
  if ("couple" in e) return t("ui.train_couple_of", { body: n.body(e.couple.body) });
  if ("insert" in e) {
    const stage = o.preset === null ? "" : n.preset(o.preset);
    return e.insert.at === null
      ? t("ui.train_offer_stage_at_output", { stage })
      : t("ui.train_offer_stage_at_body", { stage, body: n.body(e.insert.at) });
  }
  if ("move" in e) {
    const gear = n.gear(e.move.member);
    return e.move.to === null
      ? t("ui.train_offer_move_own", { gear })
      : t("ui.train_offer_move_to", { gear, body: n.body(e.move.to) });
  }
  if ("join" in e) return t("ui.train_offer_join", { a: n.body(e.join.a), b: n.body(e.join.b) });
  if ("hold" in e) return t("ui.train_hold");
  if ("release" in e) return t("ui.train_release");
  const p = e.remove;
  if ("coupling" in p) return t("ui.train_offer_remove_coupling");
  const piece =
    "member" in p ? n.gear(p.member) : "mesh" in p ? n.mesh(p.mesh) : "axis" in p ? n.axis(p.axis) : n.body(p.body);
  return t("ui.train_offer_remove", { piece });
}

/** **A run of a piece's adds under one heading** — what they add — or, with
 *  no heading, an add named whole: a kind the piece offers once. */
export interface Section {
  heading: string | null;
  entries: { offer: Offer; label: string }[];
}

/** **What an add adds, and what tells it from its fellows** — the piece it
 *  is offered at being the menu's heading over it, so no entry repeats it:
 *  at a gear, a gear or a ring meshing it by where it goes; at a body, one
 *  on it by what it meshes; at an axis, one on a new body there by what it
 *  meshes; another ratio by the body it shares; a stage by its preset,
 *  under its family. A step and a coupling are one entry each, named whole. */
function kindOf(o: Offer, at: Target, n: Names): { kind: string; heading: string | null; label: string } {
  const e = o.edit;
  const is = (k: "body" | "axis") => typeof at === "object" && k in at;
  if ("add_gear" in e) {
    const { mate, on, ring } = e.add_gear;
    const kind = ring ? "ring" : "gear";
    if (is("body")) {
      const heading = ring ? t("ui.train_offer_kind_ring_on_it") : t("ui.train_offer_kind_gear_on_it");
      return { kind, heading, label: n.gear(mate) };
    }
    if (is("axis")) {
      const heading = ring ? t("ui.train_offer_kind_ring_new_body_on_it") : t("ui.train_offer_kind_gear_new_body_on_it");
      return { kind, heading, label: n.gear(mate) };
    }
    const heading = ring ? t("ui.train_offer_kind_ring_meshing_it") : t("ui.train_offer_kind_gear_meshing_it");
    const label =
      on === "new_axis"
        ? t("ui.train_offer_on_new_axis")
        : "new_body" in on
          ? t("ui.train_offer_on_new_body", { axis: n.axis(on.new_body) })
          : t("ui.train_offer_on_body", { body: n.body(on.body) });
    return { kind, heading, label };
  }
  if ("add_ratio" in e) return { kind: "ratio", heading: t("ui.train_offer_kind_ratio"), label: n.body(e.add_ratio.shared) };
  if ("add_step" in e) return { kind: "step", heading: null, label: t("ui.train_offer_step_here") };
  if ("couple" in e) return { kind: "couple", heading: null, label: t("ui.train_offer_couple_here") };
  if ("insert" in e && o.preset !== null) {
    const family = n.family(o.preset);
    return { kind: `stage ${family}`, heading: family, label: n.preset(o.preset) };
  }
  return { kind: "other", heading: null, label: offerLabel(o, n) };
}

/** **A piece's adds, grouped by what they add** — each kind where it first
 *  appears in the core's order, and its entries in that order under it: the
 *  core offers a gear and a ring at each place in turn, and the menu reads
 *  every gear, then every ring. */
export function sections(offers: Offer[], at: Target, n: Names): Section[] {
  const out: (Section & { kind: string })[] = [];
  for (const offer of offers) {
    const { kind, heading, label } = kindOf(offer, at, n);
    const run = heading === null ? undefined : out.find((x) => x.kind === kind);
    if (run !== undefined) run.entries.push({ offer, label });
    else out.push({ kind, heading, entries: [{ offer, label }] });
  }
  return out;
}
