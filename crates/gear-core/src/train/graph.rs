//! **The train is one graph** — every axis, body, member, mesh, distance
//! and coupling in one shape ([`super::Train::shape`]) — and **its parts
//! are what a stage was**: the pieces that close, search and rate apart
//! ([`Shape::parts`]), derived and never stored.
//!
//! A train was a list of stages, each with axes of its own, and a body two
//! stages listed was what joined them. [`graph_of`] reads such a list as
//! the one graph — a body two stages share listed once, and the axes it
//! turns about in each merged into one, since a shaft is straight — which
//! is how a file written as stages is converted (`gear-cli convert`), and
//! its laws below say the graph *is* the stages: the same motion, and
//! parts that are the stages' pieces, each solving as its stage did.
//!
//! **A join that cannot be coaxial is a coupling.** A body two stages list
//! on fixed axes is one body on one axis. A body listed on an axis a carrier
//! turns in one stage and another axis in the next — a planocentric's planet
//! joined to the stage after it, as a file written before the offset
//! coupling has it — is two bodies, the second end's its own, turned by the
//! first through an offset coupling: what the join meant, since a stage's
//! axes were its own.

use super::shape::{Axis, BodyOn, Shape};
use super::GROUND;

/// **Where one stage's pieces landed in the graph**, each by the graph's
/// index, in the stage's own order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Pieces {
    pub axes: Vec<usize>,
    /// The graph's body for each of the stage's bodies — the train's own
    /// number, or a new one where this end could not be coaxial with the
    /// body's first.
    pub bodies: Vec<usize>,
    pub members: Vec<usize>,
    pub meshes: Vec<usize>,
    pub distances: Vec<usize>,
    pub couplings: Vec<usize>,
}

/// A list of stages as one shape, and where each stage went in it.
#[derive(Clone, Debug)]
pub struct Graph {
    pub shape: Shape,
    pub stages: Vec<Pieces>,
}

/// **A list of stages as one shape** — see [the module](self) — each
/// stage's bodies numbered as the train numbers them, and a body a join
/// could not make coaxial numbered from `next`, the first number the train
/// has free.
#[must_use]
pub fn graph_of(stages: &[Shape], next: usize) -> Graph {
    // One id per stage axis, merged where a body joins two fixed ones.
    let offset: Vec<usize> = stages
        .iter()
        .scan(0, |at, s| {
            let here = *at;
            *at += s.axes.len();
            Some(here)
        })
        .collect();
    let total: usize = stages.iter().map(|s| s.axes.len()).sum();
    let mut parent: Vec<usize> = (0..total).collect();
    fn find(parent: &mut [usize], i: usize) -> usize {
        let mut r = i;
        while parent[r] != r {
            r = parent[r];
        }
        parent[i] = r;
        r
    }
    let fixed = |k: usize, a: usize| stages[k].axes[a].carried_by == GROUND;

    // Each stage's name for each of its bodies in the graph, and the
    // couplings a join that cannot be coaxial becomes.
    let mut next = next.max(stages.iter().map(Shape::max_body).max().unwrap_or(GROUND) + 1);
    let mut first: Vec<Option<(usize, usize)>> = vec![None; next];
    let mut rename: Vec<Vec<(usize, usize)>> = vec![Vec::new(); stages.len()];
    let mut joined: Vec<[usize; 2]> = Vec::new();
    for (k, stage) in stages.iter().enumerate() {
        for b in &stage.bodies {
            let name = match first[b.body] {
                None => {
                    first[b.body] = Some((k, b.axis));
                    b.body
                }
                Some((k0, a0)) if fixed(k0, a0) && fixed(k, b.axis) => {
                    let (x, y) = (
                        find(&mut parent, offset[k0] + a0),
                        find(&mut parent, offset[k] + b.axis),
                    );
                    parent[y] = x;
                    b.body
                }
                Some(_) => {
                    let own = next;
                    next += 1;
                    joined.push([b.body, own]);
                    own
                }
            };
            rename[k].push((b.body, name));
        }
    }
    let named = |k: usize, body: usize| -> usize {
        if body == GROUND {
            return GROUND;
        }
        rename[k]
            .iter()
            .find(|&&(b, _)| b == body)
            .map_or(body, |&(_, n)| n)
    };

    let mut shape = Shape::default();
    let mut pieces: Vec<Pieces> = vec![Pieces::default(); stages.len()];
    // Axes, in the order they are first met; a merged one takes the
    // first stage's reading of it, which on fixed axes is every stage's.
    let mut graph_axis: Vec<Option<usize>> = vec![None; total];
    for (k, stage) in stages.iter().enumerate() {
        for (a, axis) in stage.axes.iter().enumerate() {
            let root = find(&mut parent, offset[k] + a);
            let g = match graph_axis[root] {
                Some(g) => g,
                None => {
                    let g = shape.axes.len();
                    shape.axes.push(Axis {
                        carried_by: named(k, axis.carried_by),
                        ..*axis
                    });
                    graph_axis[root] = Some(g);
                    g
                }
            };
            pieces[k].axes.push(g);
        }
    }
    for (k, stage) in stages.iter().enumerate() {
        // **Each stage keeps the order it numbered its bodies in**: a body
        // an earlier stage listed stays where it was, and the bodies this
        // stage numbered before it are listed ahead of it — which is what
        // a chain's join does ([`super::Train::join`]).
        let mut pending: Vec<BodyOn> = Vec::new();
        for b in &stage.bodies {
            let body = named(k, b.body);
            match shape.bodies.iter().position(|x| x.body == body) {
                Some(at) => {
                    shape.bodies.splice(at..at, pending.drain(..));
                }
                None => pending.push(BodyOn {
                    body,
                    axis: pieces[k].axes[b.axis],
                }),
            }
            pieces[k].bodies.push(body);
        }
        shape.bodies.extend(pending);
        let members = shape.members.len();
        for m in &stage.members {
            pieces[k].members.push(shape.members.len());
            shape.members.push(super::shape::Member {
                body: named(k, m.body),
                ..m.clone()
            });
        }
        for m in &stage.meshes {
            pieces[k].meshes.push(shape.meshes.len());
            shape.meshes.push(super::shape::MeshInput {
                a: members + m.a,
                b: members + m.b,
                ..*m
            });
        }
        // **One distance per pair of axes**: two stages whose distances
        // fall between one pair of shafts share the first's.
        for d in &stage.distances {
            let axes = d.axes.map(|a| pieces[k].axes[a]);
            let at = shape
                .distances
                .iter()
                .position(|x| x.axes == axes || x.axes == [axes[1], axes[0]]);
            pieces[k].distances.push(at.unwrap_or_else(|| {
                shape.distances.push(super::shape::Distance { axes, ..*d });
                shape.distances.len() - 1
            }));
        }
        for c in &stage.couplings {
            pieces[k].couplings.push(shape.couplings.len());
            shape.couplings.push(c.map(|b| named(k, b)));
        }
    }
    shape.couplings.extend(joined);
    Graph {
        shape,
        stages: pieces,
    }
}

/// **A piece of a shape that closes and is searched on its own**, and where
/// it came from: its members, meshes, distances and axes by the shape's
/// index, each in the shape's order.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Part {
    pub shape: Shape,
    pub members: Vec<usize>,
    pub meshes: Vec<usize>,
    pub distances: Vec<usize>,
    pub axes: Vec<usize>,
    /// The shape's couplings the part keeps, by the shape's index.
    pub couplings: Vec<usize>,
}

impl Shape {
    /// **The pieces that close apart** — the connected components of its
    /// members, a member joined to every member it meshes and a mesh to
    /// every mesh on its distance. Nothing a part closes, sizes, searches or
    /// rates reads another part: a distance is closed by the meshes on it,
    /// a search moves the members a mesh joins, and a rating reads a mesh.
    /// Each part keeps the axes its members and distances turn about, the
    /// carriers of its carried axes and the bodies on them, and the
    /// couplings its bodies are in; a body with nothing on it — no gear,
    /// no carried axis, no coupling — stays with the first part its axis
    /// is in.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        let n = self.members.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(parent: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while parent[r] != r {
                r = parent[r];
            }
            parent[i] = r;
            r
        }
        let mut union = |a: usize, b: usize| {
            let (x, y) = (find(&mut parent, a), find(&mut parent, b));
            parent[y] = x;
        };
        for m in &self.meshes {
            union(m.a, m.b);
        }
        for d in 0..self.distances.len() {
            let on = self.meshes_on(d);
            for w in on.windows(2) {
                union(self.meshes[w[0]].a, self.meshes[w[1]].a);
            }
        }
        let mut roots: Vec<usize> = Vec::new();
        let mut of: Vec<usize> = Vec::with_capacity(n);
        for i in 0..n {
            let r = find(&mut parent, i);
            of.push(match roots.iter().position(|&x| x == r) {
                Some(p) => p,
                None => {
                    roots.push(r);
                    roots.len() - 1
                }
            });
        }
        let axis_of_body =
            |body: usize| self.bodies.iter().find(|b| b.body == body).map(|b| b.axis);
        let mut taken_axes: Vec<Option<usize>> = vec![None; self.axes.len()];
        let mut parts = Vec::new();
        for p in 0..roots.len() {
            let members: Vec<usize> = (0..n).filter(|&i| of[i] == p).collect();
            let meshes: Vec<usize> = (0..self.meshes.len())
                .filter(|&k| of[self.meshes[k].a] == p)
                .collect();
            let distances: Vec<usize> = (0..self.distances.len())
                .filter(|&d| {
                    self.meshes_on(d)
                        .first()
                        .is_some_and(|&k| of[self.meshes[k].a] == p)
                })
                .collect();
            let mut axes: Vec<usize> = members
                .iter()
                .filter_map(|&i| axis_of_body(self.members[i].body))
                .chain(distances.iter().flat_map(|&d| self.distances[d].axes))
                .collect();
            // A carried axis brings its carrier's.
            let mut k = 0;
            while k < axes.len() {
                let carrier = self.axes[axes[k]].carried_by;
                if carrier != GROUND {
                    if let Some(a) = axis_of_body(carrier) {
                        axes.push(a);
                    }
                }
                k += 1;
            }
            axes.sort_unstable();
            axes.dedup();
            for &a in &axes {
                taken_axes[a].get_or_insert(p);
            }
            parts.push((members, meshes, distances, axes));
        }
        parts
            .into_iter()
            .enumerate()
            .map(|(p, (members, meshes, distances, axes))| {
                let local_axis = |a: usize| axes.iter().position(|&x| x == a);
                // Bodies: what its members turn with, the carriers of its
                // carried axes, what its couplings join, and a body with
                // nothing on it where this is the first part on its axis.
                let wanted = |b: &BodyOn| -> bool {
                    let Some(_) = local_axis(b.axis) else {
                        return false;
                    };
                    members.iter().any(|&i| self.members[i].body == b.body)
                        || axes.iter().any(|&a| self.axes[a].carried_by == b.body)
                        || self.couplings.iter().any(|c| {
                            c.contains(&b.body)
                                && c.iter()
                                    .any(|&x| members.iter().any(|&i| self.members[i].body == x))
                        })
                        || (self.members.iter().all(|m| m.body != b.body)
                            && !self.axes.iter().any(|a| a.carried_by == b.body)
                            && !self.couplings.iter().any(|c| c.contains(&b.body))
                            && taken_axes[b.axis] == Some(p))
                };
                let bodies: Vec<BodyOn> = self
                    .bodies
                    .iter()
                    .filter(|b| wanted(b))
                    .map(|b| BodyOn {
                        body: b.body,
                        axis: local_axis(b.axis).unwrap_or_default(),
                    })
                    .collect();
                let local_member =
                    |i: usize| members.iter().position(|&x| x == i).unwrap_or_default();
                let kept: Vec<usize> = (0..self.couplings.len())
                    .filter(|&c| {
                        self.couplings[c]
                            .iter()
                            .all(|&x| bodies.iter().any(|b| b.body == x))
                    })
                    .collect();
                let shape = Shape {
                    axes: axes.iter().map(|&a| self.axes[a]).collect(),
                    members: members.iter().map(|&i| self.members[i].clone()).collect(),
                    meshes: meshes
                        .iter()
                        .map(|&k| super::shape::MeshInput {
                            a: local_member(self.meshes[k].a),
                            b: local_member(self.meshes[k].b),
                            ..self.meshes[k]
                        })
                        .collect(),
                    distances: distances
                        .iter()
                        .map(|&d| super::shape::Distance {
                            axes: self.distances[d]
                                .axes
                                .map(|a| local_axis(a).unwrap_or_default()),
                            ..self.distances[d]
                        })
                        .collect(),
                    couplings: kept.iter().map(|&c| self.couplings[c]).collect(),
                    bodies,
                };
                Part {
                    shape,
                    members,
                    meshes,
                    distances,
                    axes,
                    couplings: kept,
                }
            })
            .collect()
    }
}

impl Shape {
    /// **Another shape laid in beside this one** — its axes, bodies,
    /// members, meshes, distances and couplings after this one's, their
    /// indices moved past it. Its bodies keep their numbers, which the
    /// caller has made the train's.
    pub(crate) fn append(&mut self, other: Shape) {
        let (axes, members) = (self.axes.len(), self.members.len());
        self.axes.extend(other.axes);
        self.bodies.extend(other.bodies.into_iter().map(|b| BodyOn {
            axis: b.axis + axes,
            ..b
        }));
        self.members.extend(other.members);
        self.meshes
            .extend(other.meshes.into_iter().map(|m| super::shape::MeshInput {
                a: m.a + members,
                b: m.b + members,
                ..m
            }));
        self.distances
            .extend(other.distances.into_iter().map(|d| super::shape::Distance {
                axes: d.axes.map(|a| a + axes),
                ..d
            }));
        self.couplings.extend(other.couplings);
    }

    /// **Two axes made one line**: every body and distance on `gone` moved
    /// to `keep`, `gone` taken out and the axes after it numbered down,
    /// and a distance that now repeats an earlier one's pair of axes given
    /// up for the earlier — one distance per pair of axes, the first
    /// stated standing.
    pub(crate) fn merge_axes(&mut self, keep: usize, gone: usize) {
        if keep == gone {
            return;
        }
        let to = |a: usize| {
            let a = if a == gone { keep } else { a };
            if a > gone {
                a - 1
            } else {
                a
            }
        };
        for b in &mut self.bodies {
            b.axis = to(b.axis);
        }
        for d in &mut self.distances {
            d.axes = d.axes.map(to);
        }
        self.axes.remove(gone);
        let mut seen: Vec<[usize; 2]> = Vec::new();
        self.distances.retain(|d| {
            let pair = if d.axes[0] <= d.axes[1] {
                d.axes
            } else {
                [d.axes[1], d.axes[0]]
            };
            let fresh = !seen.contains(&pair);
            seen.push(pair);
            fresh
        });
    }

    /// **A part taken out**: its members with their meshes, its distances
    /// and couplings, every body no other part lists, and every axis left
    /// with no body and no distance on it.
    pub(crate) fn remove_part(&mut self, part: &Part) {
        let others: Vec<usize> = self
            .parts()
            .into_iter()
            .filter(|p| p.members != part.members)
            .flat_map(|p| p.shape.bodies.into_iter().map(|b| b.body))
            .collect();
        let mut members = part.members.clone();
        members.sort_unstable();
        for &i in members.iter().rev() {
            self.meshes.retain(|m| m.a != i && m.b != i);
            for m in &mut self.meshes {
                if m.a > i {
                    m.a -= 1;
                }
                if m.b > i {
                    m.b -= 1;
                }
            }
            self.members.remove(i);
        }
        let mut distances = part.distances.clone();
        distances.sort_unstable();
        for &d in distances.iter().rev() {
            self.distances.remove(d);
        }
        let mut couplings = part.couplings.clone();
        couplings.sort_unstable();
        for &c in couplings.iter().rev() {
            self.couplings.remove(c);
        }
        let gone: Vec<usize> = part
            .shape
            .bodies
            .iter()
            .map(|b| b.body)
            .filter(|b| !others.contains(b))
            .collect();
        self.bodies.retain(|b| !gone.contains(&b.body));
        self.couplings
            .retain(|c| !c.iter().any(|b| gone.contains(b)));
        self.drop_empty_axes();
    }

    /// Every axis with no body and no distance on it taken out, the axes
    /// after each numbered down.
    pub(crate) fn drop_empty_axes(&mut self) {
        let mut axis = self.axes.len();
        while axis > 0 {
            axis -= 1;
            let used = self.bodies.iter().any(|b| b.axis == axis)
                || self.distances.iter().any(|d| d.axes.contains(&axis));
            if !used {
                self.axes.remove(axis);
                for b in &mut self.bodies {
                    if b.axis > axis {
                        b.axis -= 1;
                    }
                }
                for d in &mut self.distances {
                    d.axes = d.axes.map(|a| if a > axis { a - 1 } else { a });
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **A file of stages is the graph a chain builds**: [`graph_of`] reads
    //! a list of stages — each numbered as the train numbered them, a body
    //! two stages share listed on both — as the one shape, and these hold
    //! it to what the stages were: the same motion, parts that are the
    //! stages' pieces each solving as its stage did, and the very graph a
    //! chain of the same presets builds now. Every preset alone, every
    //! ordered pair, the three-stage train the corpus prints, and a
    //! planocentric written before the offset coupling.

    use super::super::arrangements::{epicyclic, external, Central, StagePreset};
    use super::super::wiring::teeth_of;
    use super::super::{shape::solve_shape, test_library, Reversal, Train};
    use super::*;
    use crate::kinematics::{Condition, System};
    use crate::ratio::Ratio;

    /// **A chain as a file of stages had it** — the storage
    /// `Train::chained` wrote before the train was one graph: each stage's
    /// bodies numbered after the last's in its slot order, its
    /// conventional input the last stage's conventional output, and what
    /// each holds by convention the train's holds.
    fn stages_of(presets: &[Shape]) -> (Vec<Shape>, Vec<usize>) {
        let (mut stages, mut held) = (Vec::new(), Vec::new());
        let (mut next, mut onward) = (1, None);
        for preset in presets {
            let ports = preset.ports();
            let (input, output) = (ports.input(), ports.output());
            let number: Vec<usize> = (0..=preset.bodies.len())
                .map(|slot| match (slot, onward) {
                    (GROUND, _) => GROUND,
                    (s, Some(o)) if s == input => o,
                    _ => {
                        next += 1;
                        next - 1
                    }
                })
                .collect();
            let mut stage = preset.clone();
            stage.renumber_bodies(|b| number[preset.slot(b)]);
            held.extend(ports.held.iter().map(|&slot| number[slot]));
            onward = Some(number[output]);
            stages.push(stage);
        }
        (stages, held)
    }

    /// Every preset alone and after every preset, the three-stage train,
    /// and a planocentric as a file written before the offset coupling
    /// has it — its output the planet's own body — with a pair after it:
    /// the join the graph cannot make coaxial.
    fn fixtures() -> Vec<(String, Vec<Shape>)> {
        let built = |presets: &[StagePreset]| presets.iter().map(|p| p.build()).collect();
        let mut out = Vec::new();
        for a in StagePreset::ALL {
            out.push((format!("{a:?}"), built(&[a])));
            for b in StagePreset::ALL {
                out.push((format!("{a:?} then {b:?}"), built(&[a, b])));
            }
        }
        out.push((
            "spur then layshaft then compound".into(),
            built(&[
                StagePreset::Spur,
                StagePreset::Layshaft,
                StagePreset::Compound,
            ]),
        ));
        out.push((
            UNCOUPLED.into(),
            vec![uncoupled(), StagePreset::Spur.build()],
        ));
        // A set that lists its held ring before the sun it is entered at:
        // the ring stays ahead of the shaft the pair shares with it.
        let mut ring_first = StagePreset::Planetary.build();
        let ring = ring_first
            .members
            .iter()
            .find(|m| m.ring.is_some())
            .unwrap()
            .body;
        let at = ring_first
            .bodies
            .iter()
            .position(|b| b.body == ring)
            .unwrap();
        let entry = ring_first.bodies.remove(at);
        ring_first.bodies.insert(0, entry);
        out.push((
            "spur then a set listing its ring first".into(),
            vec![StagePreset::Spur.build(), ring_first],
        ));
        out
    }

    const UNCOUPLED: &str = "an uncoupled planocentric then spur";

    fn uncoupled() -> Shape {
        epicyclic(
            1,
            &[&[external(30)]],
            &[Central::Carrier, Central::Ring { on: 0, teeth: 33 }],
            &[],
        )
    }

    /// The file converted: its graph, with the train's holds.
    fn converted(stages: &[Shape], held: &[usize]) -> (Graph, Train) {
        let graph = graph_of(stages, 1);
        let train = Train {
            load_cases: Vec::new(),
            reversed_bending: false,
            shape: graph.shape.clone(),
            held: held.to_vec(),
        };
        (graph, train)
    }

    /// **A file of stages converts to the graph a chain builds now** —
    /// the same axes, bodies, members, meshes, distances and couplings in
    /// the same order under the same numbers, and the same holds — so a
    /// converted file and a fresh one are one train. The join that could
    /// not be coaxial numbers its second end afresh, where a chain keeps
    /// the stage's own number, and is held to the motion instead.
    #[test]
    fn a_file_of_stages_converts_to_what_a_chain_builds() {
        for (name, presets) in fixtures() {
            if name == UNCOUPLED {
                continue;
            }
            let (stages, mut held) = stages_of(&presets);
            let (graph, _) = converted(&stages, &held);
            let chain = Train::chained(presets, |_| Vec::new());
            assert_eq!(
                format!("{:?}", graph.shape),
                format!("{:?}", chain.shape),
                "{name}"
            );
            let mut chained = chain.held.clone();
            held.sort_unstable();
            chained.sort_unstable();
            assert_eq!(held, chained, "{name}: holds");
        }
    }

    /// **A join off an orbiting body is a coupling**, from a file and from
    /// a chain: the pair after an uncoupled planocentric turns on a body
    /// of its own, coupled to the planet it was joined to.
    #[test]
    fn a_join_off_an_orbiting_body_is_a_coupling() {
        let presets = vec![uncoupled(), StagePreset::Spur.build()];
        let (stages, _) = stages_of(&presets);
        let graph = graph_of(&stages, 1);
        // Bodies: carrier 1, ring 2, planet 3; the pair's second gear 4.
        assert_eq!(
            graph.stages[1].bodies[0], 5,
            "the pair's first end is its own"
        );
        assert_eq!(graph.shape.couplings, vec![[3, 5]]);
        // A chain joins the pair's first end, body 4, to the planet.
        let chain = Train::chained(presets, |_| Vec::new());
        assert_eq!(chain.shape.couplings, vec![[3, 4]]);
        assert_eq!(chain.parts().len(), 2, "a coupling joins no parts");
    }

    /// **A file of stages turns as its graph does**: driven at the first
    /// stage's input with the file's holds, every body the stages listed
    /// at the exact speed the stages' own system gives it — each stage's
    /// meshes written in the bodies it listed, a body two list one node,
    /// which is how the train was solved — and a coupling's two ends at
    /// one speed.
    #[test]
    fn a_file_of_stages_turns_as_its_graph_does() {
        let mut checked = 0;
        for (name, presets) in fixtures() {
            let (stages, held) = stages_of(&presets);
            let listed = stages.iter().map(Shape::max_body).max().unwrap();
            let mut old = System::new(listed + 1);
            for s in &stages {
                s.wiring()
                    .add_to(&mut old, &teeth_of(s.gears()), |slot| s.body_at(slot))
                    .unwrap();
            }
            let from = stages[0].body_at(stages[0].ports().input());
            let conditions = |bodies: usize| {
                let mut c = vec![Condition::Free; bodies];
                c[GROUND] = Condition::Ground;
                for &h in &held {
                    c[h] = Condition::Ground;
                }
                c[from] = Condition::Drive(Ratio::ONE);
                c
            };
            let want = old.motion(&conditions(old.bodies())).unwrap();
            if !want.is_unique() {
                continue;
            }
            let (_, train) = converted(&stages, &held);
            let system = train.system().unwrap();
            let got = system.motion(&conditions(system.bodies())).unwrap();
            assert!(got.is_unique(), "{name}: the graph's motion is a family");
            for body in 1..=listed {
                assert_eq!(got.values[body], want.values[body], "{name}: body {body}");
            }
            for c in &train.shape.couplings {
                assert_eq!(got.values[c[0]], got.values[c[1]], "{name}: {c:?}");
            }
            checked += 1;
        }
        assert!(checked > 100, "only {checked} trains turned");
    }

    /// **A file of stages falls apart into its stages**: one part each,
    /// holding exactly that stage's members, meshes and distances, and each
    /// part solving to what its stage solves to — so a converted file's
    /// parts are its stages, and each rates as it did.
    #[test]
    fn a_file_of_stages_falls_apart_into_its_stages() {
        let lib = test_library();
        for (name, presets) in fixtures() {
            let (stages, held) = stages_of(&presets);
            let (graph, train) = converted(&stages, &held);
            let parts = train.parts();
            assert_eq!(parts.len(), stages.len(), "{name}");
            for (k, (part, pieces)) in parts.iter().zip(&graph.stages).enumerate() {
                assert_eq!(part.members, pieces.members, "{name}: stage {k}'s members");
                assert_eq!(part.meshes, pieces.meshes, "{name}: stage {k}'s meshes");
                assert_eq!(
                    part.distances, pieces.distances,
                    "{name}: stage {k}'s distances"
                );
                let solve = |s: &Shape| solve_shape(s, &[], &lib, Reversal::default()).unwrap();
                let (mine, theirs) = (solve(&part.shape), solve(&stages[k]));
                assert_eq!(
                    format!(
                        "{:?}",
                        (&mine.members, &mine.meshes, &mine.distances, &mine.notes)
                    ),
                    format!(
                        "{:?}",
                        (
                            &theirs.members,
                            &theirs.meshes,
                            &theirs.distances,
                            &theirs.notes
                        )
                    ),
                    "{name}: stage {k} solved as a part"
                );
                let spacing = |r: &super::super::ShapeResult| {
                    r.layouts
                        .iter()
                        .map(|l| (l.count, l.equal_spacing, l.clearance.to_bits()))
                        .collect::<Vec<_>>()
                };
                assert_eq!(
                    spacing(&mine),
                    spacing(&theirs),
                    "{name}: stage {k}'s layouts"
                );
            }
        }
    }
}
