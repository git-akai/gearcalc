// The members and bodies of a geartrain, named the one way both panels use.
//
// A gear goes by the graph's number for it, from 1 — the index the core names
// it by, plus one — and by its role where the core reads it one off the graph
// (`Shape::member_names`, laid out train-wide): "Gear 3", "Sun (4)",
// "Planet 1 (12)". A body goes by the train's number for it, which is the one
// a file writes and a case names. The names arrive with every solve
// (`TrainOutcome.names`), and the gear tab's *adopt* list reads the same ones,
// so a gear is one name wherever a list has it.

import { solveTrain, t, type MemberName, type Shape, type Train } from "./core";

/** One member of a train, as the gear tab's adopt list shows it. */
export interface MemberRef {
  /** The member's index in the train's graph — what the core names it by. */
  index: number;
  /** "Gear 3", "Sun (4)", "Wormwheel (6)". */
  label: string;
  /** Whether a gear tab can hold it. A worm is a thread with proportions of
   *  its own, and the tab is offered it greyed rather than not at all. */
  adoptable: boolean;
}

/** The axis a body turns about in a shape, or `undefined` where the shape
 *  does not list the body. */
export function axisOfBody(shape: Shape, body: number): number | undefined {
  return shape.bodies.find((b) => b.body === body)?.axis;
}

/** Whether a member's axis is carried — turns in a frame that is not the
 *  ground's — which is what makes it a planet. */
export function carried(shape: Shape, member: number): boolean {
  const axis = shape.axes[axisOfBody(shape, shape.members[member].body) ?? -1];
  return axis !== undefined && axis.carried_by !== 0;
}

/** The word for a role, numbered where the train has more than one of it;
 *  `null` where the name *is* the number — a gear with no role. */
export function roleLabel(name: MemberName | undefined): string | null {
  if (name === undefined) return null;
  const word = (() => {
    switch (name.role) {
      case "gear":
        return null;
      case "worm":
        return t("ui.train_worm_member");
      case "wheel":
        return t("ui.train_wormwheel");
      case "sun":
        return t("ui.train_sun");
      case "planet":
        return t("ui.train_planet");
      case "ring":
        return t("ui.train_ring");
    }
  })();
  if (word === null) return null;
  return name.ordinal === null ? word : `${word} ${name.ordinal}`;
}

/** **A gear by the graph's index**: its role and its number — "Sun (9)" —
 *  or, where the number is its name, "Gear 3". */
export function gearLabel(names: MemberName[], i: number): string {
  const role = roleLabel(names[i]);
  const number = String(i + 1);
  return role === null
    ? t("ui.train_gear_name", { number })
    : t("ui.train_member_numbered", { name: role, number });
}

/** **A body's own name**: "Body 4", numbered across the train as the core
 *  numbers it — the same number a file writes and a case names — or the
 *  ground's word for body 0. */
export function bodyName(body: number): string {
  return body === 0 ? t("ui.train_ground") : t("ui.train_body_name", { number: String(body) });
}

/** **Every gear of a train**, in the graph's order, as the adopt list shows
 *  them — named by the core's reading of the graph, which needs no geometry
 *  and arrives whether or not the train solved. */
export function memberRefs(train: Train): MemberRef[] {
  const names = solveTrain(train).names;
  return train.shape.members.map((_, index) => ({
    index,
    label: gearLabel(names, index),
    adoptable: names[index]?.role !== "worm",
  }));
}
