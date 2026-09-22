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

/** The gear number of one member, counting every member of the stages
 *  before it — one number per gear across the train, which is what the
 *  adopt list and the case rows name a gear by. */
export function gearNumber(train: Train, stage: number, member: number): number {
  let n = member + 1;
  for (let i = 0; i < stage; i++) n += memberCount(train.stages[i]);
  return n;
}

/** **The name a member goes by in a list** — the adopt list's, and a
 *  case's rows and delivered table use the same: "Gear 3" on a pair, and
 *  the role with its number elsewhere, "Sun (5)". */
export function memberListName(train: Train, topology: StagePorts[], stage: number, member: number): string {
  const role = roleName(topology, stage, member);
  const number = String(gearNumber(train, stage, member));
  return role === null
    ? t("ui.train_gear_name", { number })
    : t("ui.train_member_numbered", { name: role, number });
}

/** Whether a member's axis is carried — turns in a frame that is not the
 *  ground's — which is what makes it a planet. */
export function carried(shape: Shape, member: number): boolean {
  const axis = shape.axes[shape.shafts[shape.members[member].shaft - 1]?.axis];
  return axis !== undefined && axis.carried_by !== 0;
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

/** **A shaft's number**, counting every shaft of the stages before it —
 *  one number per shaft across the train, as a gear has, so a shaft is
 *  named the way a gear is and a reference to it reads the same way. */
export function shaftNumber(train: Train, stage: number, shaft: number): number {
  let n = shaft;
  for (let i = 0; i < stage; i++) n += train.stages[i].shafts.length;
  return n;
}

/** **What sits on a shaft**, for the line under its name: the members on
 *  it by their list names, or the carrier where it carries an axis and no
 *  gear. Read off the shape; the core's label names the first of them. */
export function onShaft(
  train: Train,
  topology: StagePorts[],
  stage: number,
  shaft: number,
  numbered = true,
): string {
  const shape = train.stages[stage];
  const gears = shape.members
    .map((m, j) =>
      m.shaft === shaft
        ? numbered
          ? memberListName(train, topology, stage, j)
          : memberName(train, topology, stage, j)
        : null,
    )
    .filter((x) => x !== null);
  if (gears.length > 0) return gears.join(" · ");
  return shape.axes.some((a) => a.carried_by === shaft) ? t("ui.train_carrier") : "";
}

/** **The name a shaft goes by**: "Shaft 4", numbered across the train as
 *  a gear is; the ground by its own word. What the shaft carries is
 *  {@link onShaft}, drawn under or beside it. */
export function shaftName(train: Train, stage: number, label: ShaftLabel, shaft: number): string {
  if (label.kind === "ground") return t("ui.train_ground");
  return t("ui.train_shaft_name", { number: String(shaftNumber(train, stage, shaft)) });
}

/** A shaft with what it carries, for a reference in a row: "Shaft 4 (Ring 2)". */
export function shaftRefName(train: Train, topology: StagePorts[], stage: number, shaft: number): string {
  const name = t("ui.train_shaft_name", { number: String(shaftNumber(train, stage, shaft)) });
  const on = onShaft(train, topology, stage, shaft, false);
  return on ? t("ui.train_shaft_with", { shaft: name, on }) : name;
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
      out.push({
        stage: i,
        member: j,
        number,
        label: memberListName(train, topology, i, j),
        // A worm is a thread with proportions of its own: the first member
        // of the first mesh on a distance marked as a worm drive.
        adoptable: !(isWorm(stage) && stage.meshes[0]?.a === j),
      });
    }
  });
  return out;
}
