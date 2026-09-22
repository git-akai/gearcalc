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
export function memberCount(stage: Shape): number {
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

/** The axis a body turns about in a shape, or `undefined` where the shape
 *  does not list the body. */
export function axisOfBody(shape: Shape, body: number): number | undefined {
  return shape.bodies.find((b) => b.body === body)?.axis;
}

/** A body's slot in a stage — the stage's own numbering of it, ground 0
 *  and the first listed 1 — or 0 where the stage does not list it. */
export function slotOf(shape: Shape, body: number): number {
  return shape.bodies.findIndex((b) => b.body === body) + 1;
}

/** Whether a member's axis is carried — turns in a frame that is not the
 *  ground's — which is what makes it a planet. */
export function carried(shape: Shape, member: number): boolean {
  const axis = shape.axes[axisOfBody(shape, shape.members[member].body) ?? -1];
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

/** **One axis of a stage, the bodies on it and the gears on those** — the
 *  shape's own three levels, read off it in one pass. A card is drawn in
 *  this order because it is the order the shape is in: a gear is fixed to
 *  a body, a body turns about an axis, and what a gear may be moved to is
 *  what else is on its axis. */
export interface AxisGroup {
  /** The shape's index for it, and the number a name goes by, from 1. */
  axis: number;
  /** The body whose frame it stands still in, `null` where that is ground
   *  — a carried axis is a planet's, and what carries it is a body of
   *  this stage like any other. */
  carriedBy: number | null;
  /** How many times it is replicated about the axis it is carried round:
   *  the planet count, which nothing else on the card says. */
  count: number;
  bodies: {
    body: number;
    /** The members fixed to it, in the shape's order. */
    members: number[];
    /** Whether it carries an axis of its own — a carrier, which is what a
     *  body with no gear on it generally is. */
    carries: boolean;
  }[];
}

/** Every axis of a stage with what sits on it, in the shape's order. */
export function axisGroups(shape: Shape): AxisGroup[] {
  return shape.axes.map((a, axis) => ({
    axis,
    carriedBy: a.carried_by === 0 ? null : a.carried_by,
    count: a.count,
    bodies: shape.bodies
      .filter((b) => b.axis === axis)
      .map((b) => ({
        body: b.body,
        members: shape.members.flatMap((m, j) => (m.body === b.body ? [j] : [])),
        carries: shape.axes.some((x) => x.carried_by === b.body),
      })),
  }));
}

/** **Where a gear of this stage could be moved to**, which is the question
 *  a body answers that the train knows nothing about. The gear's own body
 *  is not among them: it is the row the gear is already listed under, and
 *  offering it back would be an option that does nothing.
 *
 *  A gear may go to a body on its own axis that carries no axis itself (a
 *  gear fixed to the carrier of the planets it meshes locks the stage, and
 *  the core refuses it), and, where it shares its body with another gear,
 *  to a body of its own.
 *
 *  **Two gears meshing the same member may not share a body**, and that is
 *  a filter rather than a refusal: turning as one, they hold their common
 *  mate to two ratios at once, so the stage is locked by construction
 *  rather than by its numbers. It is what a set's sun and ring would do,
 *  and offering it would put a control on every epicyclic card whose only
 *  answer is a refusal. The gears of a layshaft's ratios mesh *different*
 *  members of the layshaft, so its ratios stay on offer: that is how one
 *  is engaged. */
export function movableGears(
  shape: Shape,
): { member: number; bodies: number[]; own: boolean }[] {
  const partners = (j: number) =>
    shape.meshes.filter((m) => m.a === j || m.b === j).map((m) => (m.a === j ? m.b : m.a));
  return shape.members
    .map((m, member) => {
      const axis = axisOfBody(shape, m.body);
      const mine = partners(member);
      const bodies = shape.bodies
        .filter(
          (b) =>
            b.axis === axis &&
            b.body !== m.body &&
            !shape.axes.some((a) => a.carried_by === b.body) &&
            shape.members.every(
              (x, k) => x.body !== b.body || !partners(k).some((p) => mine.includes(p)),
            ),
        )
        .map((b) => b.body);
      const own = shape.members.some((x, k) => k !== member && x.body === m.body);
      return { member, bodies, own };
    })
    .filter((g) => g.bodies.length > 0 || g.own);
}

/** **A body's own name**: "Body 4", numbered across the train as the core
 *  numbers it — the same number a file writes and a case names — or the
 *  ground's word for body 0. */
export function bodyName(body: number): string {
  return body === 0 ? t("ui.train_ground") : t("ui.train_body_name", { number: String(body) });
}

/** **The stages a body is listed on**, with its slot in each, in stage
 *  order — read off the stages themselves, so a name needs no motion. */
export function endsOf(train: Train, body: number): { stage: number; slot: number }[] {
  return train.stages
    .map((shape, stage) => ({ stage, slot: slotOf(shape, body) }))
    .filter((e) => e.slot > 0);
}

/** **What sits on a slot** of a stage, for the line under its name: the
 *  members on it by their list names, or the carrier where it carries an
 *  axis and no gear. Read off the shape; the core's label names the first
 *  of them. */
export function onSlot(
  train: Train,
  topology: StagePorts[],
  stage: number,
  slot: number,
  numbered = true,
): string {
  const shape = train.stages[stage];
  const body = shape.bodies[slot - 1]?.body;
  const gears = shape.members
    .map((m, j) =>
      m.body === body
        ? numbered
          ? memberListName(train, topology, stage, j)
          : memberName(train, topology, stage, j)
        : null,
    )
    .filter((x) => x !== null);
  if (gears.length > 0) return gears.join(" · ");
  return shape.axes.some((a) => a.carried_by === body) ? t("ui.train_carrier") : "";
}

/** **Everything on a body, across the train**: each end's stage and what
 *  it carries — "Stage 1 Gear 2 · Stage 2 Sun" — which is what says at a
 *  glance what is linked to what. */
export function acrossBody(train: Train, topology: StagePorts[], body: number): string {
  if (body === 0) return t("ui.train_ground");
  return endsOf(train, body)
    .map((e) =>
      t("ui.train_port_at", {
        stage: t("ui.train_stage_heading", { number: String(e.stage + 1) }),
        on: onSlot(train, topology, e.stage, e.slot, false),
      }),
    )
    .join(" · ");
}

/** **A body in a reference**: its name and, in parentheses, every end of
 *  it — "Body 2 (Shape 1 Gear 2 · Stage 2 Sun)" — since a body bridges
 *  stages and a reference has to say what it is on each. */
export function bodyRefName(train: Train, topology: StagePorts[], body: number): string {
  if (body === 0) return t("ui.train_ground");
  return t("ui.train_body_with", { body: bodyName(body), on: acrossBody(train, topology, body) });
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
