// The members of a geartrain, numbered and named the one way both panels use.
//
// Gear numbering runs across the whole train — stage 1's members first, then
// stage 2's — counting every member a stage has: two on a pair, three on a
// set, four on a hula stage. It used to be `stage × 2 + which + 1`, which is
// right only while every stage before this one is a pair, and it lived in the
// geartrain panel alone; the gear tab's *adopt* list needs the same numbers,
// so they are written once here and both panels read them.

import {
  portKey,
  solveTrain,
  t,
  type MemberName,
  type Shape,
  type ShaftRef,
  type Stage,
  type StagePorts,
  type Train,
  type TrainBody,
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

/** **A shaft is a body of the train**, not a stage's: the ports the
 *  couplings join into one thing that turns — a pair's output and the next
 *  set's sun on one shaft — numbered across the train in the order the
 *  core lists its bodies, as a gear is numbered. A held body is the
 *  ground and has no number. What a stage calls a shaft is that body's
 *  port on the stage. */
export interface TrainShaft {
  /** The body's number among the unheld bodies, `null` for one held. */
  number: number | null;
  body: TrainBody;
}

/** The train's shafts, off the bodies the core sent. */
export function trainShafts(bodies: TrainBody[]): TrainShaft[] {
  let n = 0;
  return bodies.map((body) => ({ number: body.held ? null : ++n, body }));
}

/** The shaft a port is on, if the core listed one for it — a replicated
 *  shaft is no port and has none. */
export function shaftOfPort(shafts: TrainShaft[], at: ShaftRef): TrainShaft | undefined {
  return shafts.find((s) => s.body.shafts.some(([p]) => portKey(p) === portKey(at)));
}

/** A shaft's own name: "Shaft 4", or the ground's word for one held. */
export function trainShaftName(shaft: TrainShaft | undefined): string {
  return shaft === undefined || shaft.number === null
    ? t("ui.train_ground")
    : t("ui.train_shaft_name", { number: String(shaft.number) });
}

/** **What sits on a port** of a stage, for the line under its name: the
 *  members on it by their list names, or the carrier where it carries an
 *  axis and no gear. Read off the shape; the core's label names the first
 *  of them. */
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

/** **Everything on a shaft, across the train**: each port's stage and what
 *  it carries — "Stage 1 Gear 2 · Stage 2 Sun" — which is what says at a
 *  glance what is linked to what. */
export function acrossShaft(train: Train, topology: StagePorts[], shaft: TrainShaft): string {
  return shaft.body.shafts
    .map(([p]) =>
      p.kind === "of"
        ? t("ui.train_port_at", {
            stage: t("ui.train_stage_heading", { number: String(p.stage + 1) }),
            shaft: onShaft(train, topology, p.stage, p.shaft, false),
          })
        : t("ui.train_ground"),
    )
    .join(" · ");
}

/** **A port in a reference**: the shaft it is on and, in parentheses, the
 *  port's own stage and member — "Shaft 2 (Stage 2 Sun)" — since a shaft
 *  bridges stages and a reference has to say which end. */
export function shaftRefName(
  train: Train,
  topology: StagePorts[],
  shafts: TrainShaft[],
  at: ShaftRef,
): string {
  if (at.kind === "ground") return t("ui.train_ground");
  const name = trainShaftName(shaftOfPort(shafts, at));
  const on = t("ui.train_port_at", {
    stage: t("ui.train_stage_heading", { number: String(at.stage + 1) }),
    shaft: onShaft(train, topology, at.stage, at.shaft, false),
  });
  return t("ui.train_shaft_with", { shaft: name, on });
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
