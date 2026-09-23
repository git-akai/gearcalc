//! **The train as one graph** — every stage's axes, bodies, members, meshes,
//! distances and couplings in one shape, which is what the train is becoming
//! (`train-graph-plan.md`): a body two stages share listed once, and the
//! axes it turns about in each merged into one, since a shaft is straight.
//!
//! Derived rather than stored until the train holds the graph itself. What
//! it answers meanwhile is whether the one shape *is* the stages it came
//! from — the same motion, and parts that are the stages' pieces — which is
//! what lets the storage flip without a number moving.
//!
//! **A join that cannot be coaxial is a coupling.** A body two stages list
//! on fixed axes is one body on one axis. A body listed on an axis a carrier
//! turns in one stage and another axis in the next — a planocentric's planet
//! joined to the stage after it, as a file written before the offset
//! coupling has it — is two bodies, the second end's its own, turned by the
//! first through an offset coupling: what the join meant, since a stage's
//! axes were its own.

use super::shape::{Axis, BodyOn, Shape};
use super::{Train, GROUND};

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

/// The train as one shape, and where each stage went in it.
#[derive(Clone, Debug)]
pub struct Graph {
    pub shape: Shape,
    pub stages: Vec<Pieces>,
}

impl Train {
    /// **The train as one shape** — see [the module](self).
    #[must_use]
    pub fn graph(&self) -> Graph {
        // One id per stage axis, merged where a body joins two fixed ones.
        let offset: Vec<usize> = self
            .stages
            .iter()
            .scan(0, |at, s| {
                let here = *at;
                *at += s.axes.len();
                Some(here)
            })
            .collect();
        let total: usize = self.stages.iter().map(|s| s.axes.len()).sum();
        let mut parent: Vec<usize> = (0..total).collect();
        fn find(parent: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while parent[r] != r {
                r = parent[r];
            }
            parent[i] = r;
            r
        }
        let fixed = |k: usize, a: usize| self.stages[k].axes[a].carried_by == GROUND;

        // Each stage's name for each of its bodies in the graph, and the
        // couplings a join that cannot be coaxial becomes.
        let mut next = self.max_body() + 1;
        let mut first: Vec<Option<(usize, usize)>> = vec![None; next];
        let mut rename: Vec<Vec<(usize, usize)>> = vec![Vec::new(); self.stages.len()];
        let mut joined: Vec<[usize; 2]> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
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
        let mut pieces: Vec<Pieces> = vec![Pieces::default(); self.stages.len()];
        // Axes, in the order they are first met; a merged one takes the
        // first stage's reading of it, which on fixed axes is every stage's.
        let mut graph_axis: Vec<Option<usize>> = vec![None; total];
        for (k, stage) in self.stages.iter().enumerate() {
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
        for (k, stage) in self.stages.iter().enumerate() {
            for b in &stage.bodies {
                let body = named(k, b.body);
                if !shape.bodies.iter().any(|x| x.body == body) {
                    shape.bodies.push(BodyOn {
                        body,
                        axis: pieces[k].axes[b.axis],
                    });
                }
                pieces[k].bodies.push(body);
            }
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
}

/// **A piece of a shape that closes and is searched on its own**, and where
/// it came from: its members, meshes, distances and axes by the shape's
/// index, each in the shape's order.
#[derive(Clone, Debug)]
pub struct Part {
    pub shape: Shape,
    pub members: Vec<usize>,
    pub meshes: Vec<usize>,
    pub distances: Vec<usize>,
    pub axes: Vec<usize>,
}

impl Shape {
    /// **The pieces that close apart** — the connected components of its
    /// members, a member joined to every member it meshes and a mesh to
    /// every mesh on its distance. Nothing a part closes, sizes, searches or
    /// rates reads another part: a distance is closed by the meshes on it,
    /// a search moves the members a mesh joins, and a rating reads a mesh.
    /// Each part keeps the axes its members and distances turn about, the
    /// carriers of its carried axes and the bodies on them, and the
    /// couplings its bodies are in; a body with nothing on it stays with
    /// the first part its axis is in.
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
                    couplings: self
                        .couplings
                        .iter()
                        .filter(|c| c.iter().all(|&x| bodies.iter().any(|b| b.body == x)))
                        .copied()
                        .collect(),
                    bodies,
                };
                Part {
                    shape,
                    members,
                    meshes,
                    distances,
                    axes,
                }
            })
            .collect()
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The one shape is the stages it came from**: the same motion, and
    //! parts that are the stages' pieces, each solving as its stage does —
    //! every preset alone, every ordered pair and the three-stage train the
    //! graph corpus prints.

    use super::super::arrangements::StagePreset;
    use super::super::{shape::solve_shape, test_library, Reversal};
    use super::*;
    use crate::kinematics::Condition;

    fn fixtures() -> Vec<(String, Train)> {
        let train = |presets: &[StagePreset]| {
            Train::chained(presets.iter().map(|p| p.build()).collect(), |_| Vec::new())
        };
        let mut out = Vec::new();
        for a in StagePreset::ALL {
            out.push((format!("{a:?}"), train(&[a])));
            for b in StagePreset::ALL {
                out.push((format!("{a:?} then {b:?}"), train(&[a, b])));
            }
        }
        out.push((
            "spur then layshaft then compound".into(),
            train(&[
                StagePreset::Spur,
                StagePreset::Layshaft,
                StagePreset::Compound,
            ]),
        ));
        // A planocentric as a file written before the offset coupling has
        // it — its output the planet's own body — and a pair after it: the
        // join the graph cannot make coaxial, which it makes a coupling.
        use super::super::arrangements::{epicyclic, external, Central};
        let uncoupled = epicyclic(
            1,
            &[&[external(30)]],
            &[Central::Carrier, Central::Ring { on: 0, teeth: 33 }],
            &[],
        );
        out.push((
            "an uncoupled planocentric then spur".into(),
            Train::chained(vec![uncoupled, StagePreset::Spur.build()], |_| Vec::new()),
        ));
        out
    }

    /// **A join that cannot be coaxial is a coupling**: the pair after an
    /// uncoupled planocentric turns on a body of its own, coupled to the
    /// planet it was joined to.
    #[test]
    fn a_join_off_an_orbiting_body_is_a_coupling() {
        let (_, train) = fixtures().pop().unwrap();
        let graph = train.graph();
        // Bodies: carrier 1, ring 2, planet 3; the pair's second gear 4.
        assert_eq!(
            graph.stages[1].bodies[0], 5,
            "the pair's first end is its own"
        );
        assert_eq!(graph.shape.couplings, vec![[3, 5]]);
    }

    /// **The graph turns as the train does**: driven where the train's
    /// motion is, with what the train holds held, every body of the train
    /// at the same exact speed — and a body a join could not make coaxial
    /// at its first end's.
    #[test]
    fn the_graph_has_the_trains_motion() {
        let mut checked = 0;
        for (name, train) in fixtures() {
            let system = train.system().unwrap();
            let base = train.conditions(system.bodies()).unwrap();
            let (from, _) = train.ends(&train.boundaries().unwrap()).unwrap();
            let mut c = base.clone();
            c[from] = Condition::Drive(crate::ratio::Ratio::ONE);
            let want = system.motion(&c).unwrap();
            if !want.is_unique() {
                continue;
            }
            let graph = train.graph();
            let g = graph.shape.system().unwrap();
            let slot = |body: usize| graph.shape.slot(body);
            let mut gc = vec![Condition::Free; g.bodies()];
            gc[0] = Condition::Ground;
            for (body, condition) in base.iter().enumerate().skip(1) {
                if graph.shape.slot_if_any(body).is_some() {
                    gc[slot(body)] = *condition;
                }
            }
            gc[slot(from)] = Condition::Drive(crate::ratio::Ratio::ONE);
            let got = g.motion(&gc).unwrap();
            assert!(got.is_unique(), "{name}: the graph's motion is a family");
            for body in 1..system.bodies() {
                if graph.shape.slot_if_any(body).is_some() {
                    assert_eq!(
                        got.values[slot(body)],
                        want.values[body],
                        "{name}: body {body}"
                    );
                }
            }
            for c in &graph.shape.couplings {
                assert_eq!(
                    got.values[slot(c[0])],
                    got.values[slot(c[1])],
                    "{name}: {c:?}"
                );
            }
            checked += 1;
        }
        assert!(checked > 100, "only {checked} trains turned");
    }

    /// **A train of stages falls apart into its stages**: one part each,
    /// holding exactly that stage's members, meshes and distances, and each
    /// part solving to what its stage solves to — which is what lets the
    /// storage flip and the cards be the graph's parts.
    #[test]
    fn a_train_of_stages_falls_apart_into_its_stages() {
        let lib = test_library();
        for (name, train) in fixtures() {
            let graph = train.graph();
            let parts = graph.shape.parts();
            assert_eq!(parts.len(), train.stages.len(), "{name}");
            for (k, (part, pieces)) in parts.iter().zip(&graph.stages).enumerate() {
                assert_eq!(part.members, pieces.members, "{name}: stage {k}'s members");
                assert_eq!(part.meshes, pieces.meshes, "{name}: stage {k}'s meshes");
                assert_eq!(
                    part.distances, pieces.distances,
                    "{name}: stage {k}'s distances"
                );
                let solve = |s: &Shape| solve_shape(s, &[], &lib, Reversal::default()).unwrap();
                let (mine, theirs) = (solve(&part.shape), solve(&train.stages[k]));
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
