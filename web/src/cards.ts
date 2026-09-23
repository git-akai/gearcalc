// A card's view of the train's one graph.
//
// The train is one graph (`Train.shape`) and a card is one of its parts —
// the pieces that close apart, which the core reads off the graph and sends
// with every solve (`StagePorts.part`). A card is drawn from its part's
// shape, in the part's own numbering, and its inputs are bound to the
// graph's own objects: a member, an axis, a mesh and a distance of the view
// *are* the graph's, so a box typed into writes the train, and relief —
// asked of the whole graph with the card's freedom read into the graph's
// (`relieveStage`) — writes it too. Only the indices a mesh and a distance
// carry — which members it joins, which axes — are read in the part's
// numbering, and they are the part's to read and nobody's to write: a card
// changes them through an edit the core makes (`editTrain`). What a card
// shows of the result is the core's view of it (`TrainOutcome.cards`).
//
// Nothing here decides what a part is. Which pieces close apart is the
// core's rule (`Shape::parts`); this is the bookkeeping that lets a card
// numbered its own way stand on the graph.

import { relieveTrain, type Figure, type Freedom, type Part, type Shape, type StagePorts, type Train } from "./core";

/** Which train and part a card's view stands on, for relief to find. */
const owners = new WeakMap<Shape, { train: Train; part: Part }>();

/** A graph piece seen through a part's numbering: `local` answers the keys
 *  it has, and every other key reads and writes the piece itself. */
function reindexed<T extends object>(piece: T, local: Partial<T>): T {
  const own = (key: string | symbol): key is string => typeof key === "string" && key in local;
  return new Proxy(piece, {
    get: (target, key) => (own(key) ? (local as Record<string, unknown>)[key] : Reflect.get(target, key)),
    set: (target, key, value) => own(key) || Reflect.set(target, key, value),
  });
}

/** **One card's view of the train** — its part's shape, standing on the
 *  graph's own pieces. */
export function cardView(train: Train, part: Part): Shape {
  const graph = train.shape;
  const view: Shape = {
    axes: part.axes.map((a) => graph.axes[a]),
    bodies: part.shape.bodies,
    members: part.members.map((i) => graph.members[i]),
    meshes: part.meshes.map((k, j) =>
      reindexed(graph.meshes[k], { a: part.shape.meshes[j].a, b: part.shape.meshes[j].b }),
    ),
    distances: part.distances.map((d, j) => reindexed(graph.distances[d], { axes: part.shape.distances[j].axes })),
    couplings: part.couplings.map((c) => graph.couplings[c]),
  };
  owners.set(view, { train, part });
  return view;
}

/** A card's freedom — its member, mesh or distance by the card's own
 *  number — named by the graph's. */
function toGraph(part: Part, f: Freedom): Freedom {
  if ("distance" in f) return { distance: part.distances[f.distance] };
  if ("clearance" in f) return { clearance: part.distances[f.clearance] };
  if ("overlap" in f) return { overlap: part.meshes[f.overlap] };
  return { member: [part.members[f.member[0]], f.member[1]] };
}

/** **A card's inputs relieved**: the train's graph relieved by the core —
 *  every group of inputs that argue is a part's, so relieving the graph is
 *  relieving the card — with the input just touched named by the graph's
 *  index, and `figures` the train's own (`TrainOutcome.figures`). */
export function relieveStage(view: Shape, just: Freedom | null, figures: Figure[]): void {
  const owner = owners.get(view);
  if (owner === undefined) return;
  relieveTrain(owner.train, just === null ? null : toGraph(owner.part, just), figures);
}

/** **The whole graph as one part** — every index its own — which is what
 *  the workspace stands on: a selection is by the graph's index, so the
 *  card view over this part hands every field snippet and relief hook the
 *  graph's pieces under the graph's numbers. */
export function wholePart(train: Train): Part {
  const s = train.shape;
  const upTo = (n: number) => Array.from({ length: n }, (_, i) => i);
  return {
    shape: s,
    members: upTo(s.members.length),
    meshes: upTo(s.meshes.length),
    distances: upTo(s.distances.length),
    axes: upTo(s.axes.length),
    couplings: upTo(s.couplings.length),
  };
}

/** Every card of a train, in the order the core deals its parts. */
export function cardsOf(train: Train, topology: StagePorts[]): Shape[] {
  return topology.map((s) => cardView(train, s.part));
}
