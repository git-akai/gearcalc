// The members of a geartrain, numbered and named the one way both panels use.
//
// Gear numbering runs across the whole train — stage 1's members first, then
// stage 2's — counting every member a stage has: two on a pair, three on a
// set, four on a hula stage. It used to be `stage × 2 + which + 1`, which is
// right only while every stage before this one is a pair, and it lived in the
// geartrain panel alone; the gear tab's *adopt* list needs the same numbers,
// so they are written once here and both panels read them.

import { t, type Shape, type ShaftLabel, type Stage, type Train } from "./core";

/** One member of a train, as a list can show it. */
export interface MemberRef {
  /** The stage's index in the train, and the member's in the core's order. */
  stage: number;
  member: number;
  /** The gear number across the train, from 1. */
  number: number;
  /** "gear 3", "Sun (4)", "Wormwheel (6)" — the card's name, numbered. */
  label: string;
  /** Whether a gear tab can hold it. A worm is a thread with proportions of
   *  its own, and the tab is offered it greyed rather than not at all. */
  adoptable: boolean;
}

/** How many members a stage has, in the core's member order. */
export function memberCount(stage: Stage): number {
  return stage.kind === "hula" ? stage.gears.length : stage.members.length;
}

/** The gear number of one member, counting every member of the stages
 *  before it. */
export function gearNumber(train: Train, stage: number, member: number): number {
  let n = member + 1;
  for (let i = 0; i < stage; i++) n += memberCount(train.stages[i]);
  return n;
}

/** Which member of a hula mesh is its ring: the larger gear, as the core
 *  decides it (`hula::Teeth::pair`). Read here for a label only. */
export function hulaRing(stage: Stage & { kind: "hula" }, mesh: number): number {
  const [a, b] = [mesh * 2, mesh * 2 + 1];
  return stage.gears[a].teeth >= stage.gears[b].teeth ? a : b;
}

/** Whether a member's axis is carried — turns in a frame that is not the
 *  ground's — which is what makes it a planet. */
export function carried(shape: Shape, member: number): boolean {
  const axis = shape.axes[shape.shafts[shape.members[member].shaft - 1]?.axis];
  return axis !== undefined && axis.carried_by !== null;
}

/** Whether a shape is a worm drive: its first distance says so, as a preset's
 *  word, and the worm is the first member of the first mesh on it. */
export const isWorm = (shape: Shape): boolean => shape.distances[0]?.worm === true;

/** **What a shape's member is, read off the shape.** A ring is a member with a
 *  cutter; a planet is one on a carried axis; a sun is what meshes with a
 *  planet from the ground; a worm and its wheel are the two ends of a distance
 *  marked as one. Anything else is a gear and goes by its number. The core
 *  keeps no such names — a set is a tick pattern of the shape, not a kind —
 *  so the words are found here, from the same facts the core solves on. */
export function shapeRole(shape: Shape, member: number): string | null {
  const m = shape.members[member];
  if (isWorm(shape)) {
    const first = shape.meshes[0];
    if (first?.a === member) return t("ui.train_worm_member");
    if (first?.b === member) return t("ui.train_wormwheel");
  }
  if (m.ring !== null) return t("ui.train_ring");
  if (carried(shape, member)) return t("ui.train_planet");
  const meetsAPlanet = shape.meshes.some(
    (x) => (x.a === member && carried(shape, x.b)) || (x.b === member && carried(shape, x.a)),
  );
  return meetsAPlanet ? t("ui.train_sun") : null;
}

/** The card's own name for a member, without its number. `null` where the
 *  name *is* the number — a pair's gears. */
function roleName(stage: Stage, member: number): string | null {
  if (stage.kind === "hula") {
    return t(hulaRing(stage, Math.floor(member / 2)) === member ? "ui.train_ring" : "ui.train_pinion");
  }
  return shapeRole(stage, member);
}

/** **The name a shaft goes by**, from what the core says it is: ground, the
 *  shaft a member spins with — named after the member — or a carrier, which
 *  a hula stage calls its crank. Nothing here decides which shaft is which;
 *  that arrives with the label. */
export function shaftName(train: Train, stage: number, label: ShaftLabel): string {
  switch (label.kind) {
    case "ground":
      return t("ui.train_ground");
    case "member":
      return memberName(train, stage, label.member);
    case "carrier":
      return t(train.stages[stage].kind === "hula" ? "ui.train_hula_crank" : "ui.train_carrier");
  }
}

/** The name a member's card carries: "gear 3" on a pair, the role elsewhere. */
export function memberName(train: Train, stage: number, member: number): string {
  const role = roleName(train.stages[stage], member);
  const number = String(gearNumber(train, stage, member));
  return role === null ? t("ui.train_gear_name", { number }) : role;
}

/** Every member of a train, in order, with the label a list shows. */
export function memberRefs(train: Train): MemberRef[] {
  const out: MemberRef[] = [];
  train.stages.forEach((stage, i) => {
    for (let j = 0; j < memberCount(stage); j++) {
      const number = gearNumber(train, i, j);
      const role = roleName(stage, j);
      out.push({
        stage: i,
        member: j,
        number,
        label:
          role === null
            ? t("ui.train_gear_name", { number: String(number) })
            : t("ui.train_member_numbered", { name: role, number: String(number) }),
        // A worm is a thread with proportions of its own: the first member
        // of the first mesh on a distance marked as a worm drive.
        adoptable: !(stage.kind === "shape" && isWorm(stage) && stage.meshes[0]?.a === j),
      });
    }
  });
  return out;
}
