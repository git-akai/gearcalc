// The members of a geartrain, numbered and named the one way both panels use.
//
// Gear numbering runs across the whole train — stage 1's members first, then
// stage 2's — counting every member a stage has: two on a pair, three on a
// set, four on a hula stage. It used to be `stage × 2 + which + 1`, which is
// right only while every stage before this one is a pair, and it lived in the
// geartrain panel alone; the gear tab's *adopt* list needs the same numbers,
// so they are written once here and both panels read them.

import {
  solveTrain,
  t,
  type MemberName,
  type Shape,
  type ShaftLabel,
  type Stage,
  type StagePorts,
  type Train,
} from "./core";

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
  return stage.members.length;
}

/** **The gear number of one member, within its stage.** A member is named
 *  with its stage wherever the two can be told apart — "Stage 2 · Gear 1"
 *  on a case's row, in the adopt list, on the shaft line — and within its
 *  stage's own card the stage is the card, so the count starts over at
 *  each stage; a body two stages share reads "Stage 1 · Gear 2; Stage 2 ·
 *  Gear 1", which is the two names one shaft has. (It counted across the
 *  stages once, which put a fresh stage's first gear at "Gear 3".) */
export function gearNumber(_train: Train, _stage: number, member: number): number {
  return member + 1;
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

/** **What each member of a train is**, as the core reads it off the shape —
 *  one rule, in Rust, that the harness and this side both read
 *  (`Shape::member_names`). It arrives with every solve's topology, which
 *  needs no geometry and is present whether or not the train solved. */
export function memberNames(train: Train): StagePorts[] {
  return solveTrain(train).topology;
}

/** The word for a role, numbered where the shape has more than one of it;
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

/** The card's own name for a member, without its number. `null` where the
 *  name *is* the number — a pair's gears. */
function roleName(topology: StagePorts[], stage: number, member: number): string | null {
  return roleLabel(topology[stage]?.members[member]);
}

/** **The name a shaft goes by**, from what the core says it is: ground, the
 *  shaft a member spins with — named after the member — or a carrier, which
 *  a hula stage calls its crank. Nothing here decides which shaft is which;
 *  that arrives with the label. */
export function shaftName(
  train: Train,
  topology: StagePorts[],
  stage: number,
  label: ShaftLabel,
): string {
  switch (label.kind) {
    case "ground":
      return t("ui.train_ground");
    case "member":
      return memberName(train, topology, stage, label.member);
    case "carrier":
      return t("ui.train_carrier");
  }
}

/** The name a member's card carries: "gear 3" on a pair, the role elsewhere. */
export function memberName(
  train: Train,
  topology: StagePorts[],
  stage: number,
  member: number,
): string {
  const role = roleName(topology, stage, member);
  const number = String(gearNumber(train, stage, member));
  return role === null ? t("ui.train_gear_name", { number }) : role;
}

/** Every member of a train, in order, with the label a list shows. */
export function memberRefs(train: Train, topology: StagePorts[] = memberNames(train)): MemberRef[] {
  const out: MemberRef[] = [];
  train.stages.forEach((stage, i) => {
    for (let j = 0; j < memberCount(stage); j++) {
      const number = gearNumber(train, i, j);
      const role = roleName(topology, i, j);
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
        adoptable: !(isWorm(stage) && stage.meshes[0]?.a === j),
      });
    }
  });
  return out;
}
