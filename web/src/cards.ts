// A card's view of the train's one graph.
//
// The train is one graph (`Train.shape`) and a card is one of its parts —
// the pieces that close apart, which the core reads off the graph and sends
// with every solve (`StagePorts.part`). A card is drawn from its part's
// shape, in the part's own numbering, and its inputs are bound to the
// graph's own objects: a member, an axis, a mesh and a distance of the view
// *are* the graph's, so a box typed into writes the train, and relief copied
// back leaf by leaf (`relieveStage`) writes it too. Only the indices a mesh
// and a distance carry — which members it joins, which axes — are read in
// the part's numbering, and they are the part's to read and nobody's to
// write: a card changes them through an edit the core makes (`editTrain`).
//
// Nothing here decides what a part is. Which pieces close apart is the
// core's rule (`Shape::parts`); this is the bookkeeping that lets a card
// numbered its own way stand on the graph.

import type { Part, Shape, StagePorts, Train } from "./core";

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
  return {
    axes: part.axes.map((a) => graph.axes[a]),
    bodies: part.shape.bodies,
    members: part.members.map((i) => graph.members[i]),
    meshes: part.meshes.map((k, j) =>
      reindexed(graph.meshes[k], { a: part.shape.meshes[j].a, b: part.shape.meshes[j].b }),
    ),
    distances: part.distances.map((d, j) => reindexed(graph.distances[d], { axes: part.shape.distances[j].axes })),
    couplings: part.couplings.map((c) => graph.couplings[c]),
  };
}

/** Every card of a train, in the order the core deals its parts. */
export function cardsOf(train: Train, topology: StagePorts[]): Shape[] {
  return topology.map((s) => cardView(train, s.part));
}
