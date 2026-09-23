// What an offer is called.
//
// The core lists the edits a piece of the train offers and tries each one
// (`offersAt`); this names them, in the reader's language, from the names the
// panel already gives a gear, a body, an axis, a mesh and a preset. Nothing
// here decides what can be done, or whether: an offer arrives with its edit
// and, where it would be refused, the refusal's key.

import { t, type Edit, type Offer, type StagePreset } from "./core";

/** The panel's names for the pieces an edit points at, by the graph's
 *  index — a body by its number. */
export interface Names {
  gear: (i: number) => string;
  body: (b: number) => string;
  axis: (a: number) => string;
  mesh: (k: number) => string;
  distance: (d: number) => string;
  preset: (p: StagePreset) => string;
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
