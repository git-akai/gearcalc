//! **The train's graph grouped three ways**, one per question a designer
//! asks of it (docs/reference.md#the-graph, *the graph grouped*),
//! each derived and never stored, so none can disagree with the gears:
//!
//! - **centres** — each pair of axes that mesh, and every mesh at that
//!   spacing: what is geometrically coupled, the unit the shifts close over;
//! - **axes** — each axis, the bodies that turn about it, the gears fixed to
//!   each: what turns with what;
//! - **flow** — in one case, the bodies in the order power reaches them from
//!   the case's load, the meshes that carry it between them, an epicyclic
//!   part as one junction joining its bodies, and a mesh that carries none
//!   as an idle branch: how the train works, and what each case does to it.
//!
//! The first two need no solve and travel with the topology; the flow reads
//! each mesh's share of the power in its case off the result.

use super::{Train, TrainResult};
use crate::kinematics::GROUND;

/// **Each pair of axes that mesh, and every mesh at that spacing.**
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Centre {
    /// The distance, by the graph's index.
    pub distance: usize,
    pub axes: [usize; 2],
    /// Every mesh across it, in the graph's order.
    pub meshes: Vec<usize>,
}

/// **An axis, and what turns about it.**
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct AxisGroup {
    pub axis: usize,
    /// The body that carries it round, where one does — a planet's axis.
    pub carried_by: Option<usize>,
    /// How many copies stand round the carrier.
    pub count: u32,
    /// The bodies on it, in the graph's order.
    pub bodies: Vec<AxisBody>,
}

/// One body on an axis: the gears fixed to it, the axes it carries round.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct AxisBody {
    pub body: usize,
    pub members: Vec<usize>,
    pub carries: Vec<usize>,
}

/// **The two groupings that need no solve** — see [the module](self).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Groupings {
    pub centres: Vec<Centre>,
    pub axes: Vec<AxisGroup>,
}

/// **One row of a case's flow.**
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub enum FlowRow {
    /// A body the flow reaches.
    Body { body: usize },
    /// A mesh power crosses, from the body before it to `to`.
    Mesh { mesh: usize, to: usize },
    /// A mesh that carries none of the case's power, and the body beyond it.
    Idle { mesh: usize, to: usize },
    /// **An epicyclic part as one junction**: its meshes, and the bodies it
    /// joins that do not orbit — its central members' and its carrier's.
    Junction {
        part: usize,
        meshes: Vec<usize>,
        terminals: Vec<usize>,
    },
    /// An offset coupling between two parts, and the body it turns.
    Coupling { coupling: usize, to: usize },
}

impl Train {
    /// **The train's centres and its axes** — see [the module](self).
    #[must_use]
    pub fn groupings(&self) -> Groupings {
        let s = &self.shape;
        let centres = (0..s.distances.len())
            .map(|d| Centre {
                distance: d,
                axes: s.distances[d].axes,
                meshes: s.meshes_on(d),
            })
            .collect();
        let axes = (0..s.axes.len())
            .map(|a| AxisGroup {
                axis: a,
                carried_by: (s.axes[a].carried_by != GROUND).then_some(s.axes[a].carried_by),
                count: s.axes[a].count,
                bodies: s
                    .bodies
                    .iter()
                    .filter(|b| b.axis == a)
                    .map(|b| AxisBody {
                        body: b.body,
                        members: s.members_on_body(b.body),
                        carries: (0..s.axes.len())
                            .filter(|&x| s.axes[x].carried_by == b.body)
                            .collect(),
                    })
                    .collect(),
            })
            .collect();
        Groupings { centres, axes }
    }

    /// **Each case's flow** — see [the module](self) — in the train's case
    /// order: from the case's first load, down every mesh that carries
    /// power, most first, a mesh that carries none an idle branch; and then
    /// from every body no walk has reached, so every body appears once. A
    /// case the train did not rate has no shares to read, and its flow is
    /// the graph's walk with every mesh carrying.
    #[must_use]
    pub fn flows(&self, result: &TrainResult) -> Vec<Vec<FlowRow>> {
        (0..self.load_cases.len())
            .map(|c| self.flow(result, c))
            .collect()
    }

    fn flow(&self, result: &TrainResult, case: usize) -> Vec<FlowRow> {
        let s = &self.shape;
        let parts = self.parts();
        // An epicyclic part is one junction; any other part's meshes are
        // edges between the bodies their gears are on.
        let junction_of: Vec<Option<usize>> = (0..s.meshes.len())
            .map(|k| {
                parts.iter().position(|p| {
                    p.meshes.contains(&k) && p.axes.iter().any(|&a| s.axes[a].carried_by != GROUND)
                })
            })
            .collect();
        let body_of = |i: usize| s.members[i].body;
        let orbits = |body: usize| {
            s.bodies
                .iter()
                .find(|b| b.body == body)
                .is_some_and(|b| s.axes[b.axis].carried_by != GROUND)
        };
        let terminals = |p: usize| -> Vec<usize> {
            parts[p]
                .shape
                .bodies
                .iter()
                .map(|b| b.body)
                .filter(|&b| !orbits(b))
                .collect()
        };
        // **A case that did not solve has no shares**: its meshes carry
        // nothing the flow can read, so none is idle either, and the walk
        // is the graph's own order from the case's load.
        let solved = result.cases.iter().any(|c| c.case == case && c.solved);
        let share = |k: usize| -> Option<f64> {
            result
                .meshes
                .get(k)
                .filter(|_| solved)
                .and_then(|m| m.cases.iter().find(|x| x.case == case))
                .map(|x| x.power_through.abs())
        };
        // Every step out of a body: (the row, the body it leads to, the
        // power it carries — `None` where the case has no shares).
        enum Step {
            Mesh(usize),
            Junction(usize),
            Coupling(usize),
        }
        let steps_from = |b: usize| -> Vec<(Step, usize, Option<f64>)> {
            let mut out = Vec::new();
            for (k, m) in s.meshes.iter().enumerate() {
                if junction_of[k].is_some() {
                    continue;
                }
                let (x, y) = (body_of(m.a), body_of(m.b));
                if x == b && y != b {
                    out.push((Step::Mesh(k), y, share(k)));
                } else if y == b && x != b {
                    out.push((Step::Mesh(k), x, share(k)));
                }
            }
            for p in 0..parts.len() {
                let ends = terminals(p);
                let meshes: Vec<usize> = (0..s.meshes.len())
                    .filter(|&k| junction_of[k] == Some(p))
                    .collect();
                if meshes.is_empty() || !ends.contains(&b) {
                    continue;
                }
                let carried = meshes
                    .iter()
                    .filter_map(|&k| share(k))
                    .fold(None, |m: Option<f64>, x| Some(m.map_or(x, |m| m.max(x))));
                for &t in &ends {
                    if t != b {
                        out.push((Step::Junction(p), t, carried));
                    }
                }
            }
            for (c, pair) in s.couplings.iter().enumerate() {
                if pair[0] == b {
                    out.push((Step::Coupling(c), pair[1], None));
                } else if pair[1] == b {
                    out.push((Step::Coupling(c), pair[0], None));
                }
            }
            out
        };
        let idle = |p: Option<f64>| p.is_some_and(|p| p < 1e-9);

        let mut rows = Vec::new();
        let mut seen: Vec<usize> = Vec::new();
        let mut junctions: Vec<usize> = Vec::new();
        let start = self.load_cases.get(case).and_then(|c| {
            c.loads
                .iter()
                .find(|l| l.is_load())
                .map(|l| l.at)
                .filter(|&b| s.bodies.iter().any(|x| x.body == b))
        });
        let mut roots: Vec<usize> = start.into_iter().collect();
        roots.extend(s.bodies.iter().map(|b| b.body));
        for root in roots {
            // Depth first: a body's row, the steps out of it that carry —
            // most first — and its idle branches; then down each step that
            // carries, and last down the idle branches, so the path the
            // case's power takes reads first and top to bottom.
            let mut stack = vec![root];
            while let Some(b) = stack.pop() {
                if seen.contains(&b) {
                    continue;
                }
                seen.push(b);
                rows.push(FlowRow::Body { body: b });
                let mut steps: Vec<(Step, usize, Option<f64>)> = steps_from(b)
                    .into_iter()
                    .filter(|(step, to, _)| {
                        !seen.contains(to)
                            && !matches!(step, Step::Junction(p) if junctions.contains(p))
                    })
                    .collect();
                steps.sort_by(|x, y| {
                    let p = |v: Option<f64>| v.unwrap_or(f64::INFINITY);
                    p(y.2).total_cmp(&p(x.2))
                });
                let (mut onward, mut branches) = (Vec::new(), Vec::new());
                for (step, to, power) in &steps {
                    match *step {
                        Step::Mesh(k) if idle(*power) => {
                            rows.push(FlowRow::Idle { mesh: k, to: *to });
                            branches.push(*to);
                        }
                        Step::Mesh(k) => {
                            rows.push(FlowRow::Mesh { mesh: k, to: *to });
                            onward.push(*to);
                        }
                        Step::Junction(p) => {
                            if !junctions.contains(&p) {
                                junctions.push(p);
                                rows.push(FlowRow::Junction {
                                    part: p,
                                    meshes: (0..s.meshes.len())
                                        .filter(|&k| junction_of[k] == Some(p))
                                        .collect(),
                                    terminals: terminals(p),
                                });
                                // **Said inside the junction**: its planets'
                                // bodies, which orbit.
                                seen.extend(
                                    parts[p]
                                        .shape
                                        .bodies
                                        .iter()
                                        .map(|x| x.body)
                                        .filter(|&x| orbits(x)),
                                );
                            }
                            // On to every body it joins that the power goes
                            // on from — one another part, a coupling or the
                            // case names; one held, or free with nothing
                            // beyond it — a carrier turning idle — is said
                            // inside the junction.
                            let beyond = steps_from(*to)
                                .iter()
                                .any(|(step, _, _)| !matches!(step, Step::Junction(q) if *q == p));
                            let named = self
                                .load_cases
                                .get(case)
                                .is_some_and(|c| c.loads.iter().any(|l| l.at == *to));
                            if !self.held.contains(to) && (beyond || named) {
                                onward.push(*to);
                            } else {
                                seen.push(*to);
                            }
                        }
                        Step::Coupling(c) => {
                            rows.push(FlowRow::Coupling {
                                coupling: c,
                                to: *to,
                            });
                            onward.push(*to);
                        }
                    }
                }
                for to in branches.into_iter().rev().chain(onward.into_iter().rev()) {
                    stack.push(to);
                }
            }
        }
        rows
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The groupings are the graph's, every piece once**: every mesh at
    //! exactly one centre, every body on exactly one axis, and every body
    //! in a case's flow exactly once, entered at the case's load, each mesh
    //! of it said once — as a step, an idle branch, or inside a junction.

    use super::super::arrangements::Preset;
    use super::super::{solve_train, test_library, LoadCase, Shape};
    use super::*;

    fn trains() -> Vec<(String, Train)> {
        let mut out = Vec::new();
        for a in Preset::ALL {
            for b in [None, Some(Preset::Spur), Some(Preset::Layshaft)] {
                let presets: Vec<Shape> = std::iter::once(a.build())
                    .chain(b.map(Preset::build))
                    .collect();
                let t = Train::chained(presets, |t| {
                    t.chain_ends()
                        .map(|(x, y)| vec![LoadCase::ultimate(x, y, 1.0, 1000.0)])
                        .unwrap_or_default()
                });
                out.push((format!("{a:?} then {b:?}"), t));
            }
        }
        out
    }

    #[test]
    fn every_mesh_is_at_one_centre_and_every_body_on_one_axis() {
        for (name, t) in trains() {
            let g = t.groupings();
            let mut meshes: Vec<usize> = g.centres.iter().flat_map(|c| c.meshes.clone()).collect();
            meshes.sort_unstable();
            assert_eq!(
                meshes,
                (0..t.shape.meshes.len()).collect::<Vec<_>>(),
                "{name}"
            );
            let mut bodies: Vec<usize> = g
                .axes
                .iter()
                .flat_map(|a| a.bodies.iter().map(|b| b.body))
                .collect();
            bodies.sort_unstable();
            let mut all: Vec<usize> = t.shape.bodies.iter().map(|b| b.body).collect();
            all.sort_unstable();
            assert_eq!(bodies, all, "{name}");
        }
    }

    /// **A case that does not solve calls no mesh idle.** A layshaft's
    /// case idles one ratio; with its reaction taken out nothing reacts
    /// the load, the case does not solve, and its flow is the graph walked
    /// from the load in its own order — every mesh a step, none a branch
    /// said to carry nothing, since nothing is known of what any carries.
    #[test]
    fn a_case_that_does_not_solve_calls_no_mesh_idle() {
        let lib = test_library();
        let idle = |rows: &[FlowRow]| rows.iter().any(|r| matches!(r, FlowRow::Idle { .. }));
        let mut t = Train::chained(vec![Preset::Layshaft.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(0, 2), 1.0, 1000.0)]
        });
        let r = solve_train(&t, &lib).unwrap();
        assert!(
            r.cases[0].solved && idle(&t.flows(&r)[0]),
            "one ratio idles"
        );
        t.load_cases[0].loads.retain(super::super::Load::is_load);
        let r = solve_train(&t, &lib).unwrap();
        assert!(!r.cases[0].solved, "nothing reacts the load");
        let rows = &t.flows(&r)[0];
        assert!(!idle(rows), "{rows:?}");
        assert!(matches!(rows[0], FlowRow::Body { body } if body == t.port(0, 1)));
    }

    /// **A case's flow says every body once and every mesh once**, and
    /// starts where the case's load goes in: a body in a row of its own,
    /// or said inside the junction it orbits in or ends at.
    #[test]
    fn a_flow_says_every_body_and_every_mesh_once() {
        let lib = test_library();
        for (name, t) in trains() {
            let Ok(r) = solve_train(&t, &lib) else {
                continue;
            };
            for (c, rows) in t.flows(&r).iter().enumerate() {
                let mut bodies: Vec<usize> = rows
                    .iter()
                    .filter_map(|row| match row {
                        FlowRow::Body { body } => Some(*body),
                        _ => None,
                    })
                    .collect();
                let first = bodies.first().copied();
                let load = t.load_cases[c]
                    .loads
                    .iter()
                    .find(|l| l.is_load())
                    .unwrap()
                    .at;
                assert_eq!(first, Some(load), "{name}: case {c} starts at its load");
                bodies.sort_unstable();
                let before = bodies.len();
                bodies.dedup();
                assert_eq!(bodies.len(), before, "{name}: a body twice");
                // ...and no body left unsaid: each in a row of its own, at
                // the end of an idle branch, or inside a junction.
                let parts = t.parts();
                let mut said: Vec<usize> = rows
                    .iter()
                    .flat_map(|row| match row {
                        FlowRow::Body { body } => vec![*body],
                        FlowRow::Idle { to, .. } | FlowRow::Coupling { to, .. } => vec![*to],
                        FlowRow::Junction {
                            part, terminals, ..
                        } => terminals
                            .iter()
                            .copied()
                            .chain(parts[*part].shape.bodies.iter().map(|b| b.body))
                            .collect(),
                        FlowRow::Mesh { to, .. } => vec![*to],
                    })
                    .collect();
                said.sort_unstable();
                said.dedup();
                let mut all: Vec<usize> = t.shape.bodies.iter().map(|b| b.body).collect();
                all.sort_unstable();
                assert_eq!(said, all, "{name}: case {c}: {rows:?}");
                let mut meshes: Vec<usize> = rows
                    .iter()
                    .flat_map(|row| match row {
                        FlowRow::Mesh { mesh, .. } | FlowRow::Idle { mesh, .. } => vec![*mesh],
                        FlowRow::Junction { meshes, .. } => meshes.clone(),
                        _ => Vec::new(),
                    })
                    .collect();
                meshes.sort_unstable();
                assert_eq!(
                    meshes,
                    (0..t.shape.meshes.len()).collect::<Vec<_>>(),
                    "{name}: case {c}: {rows:?}"
                );
            }
        }
    }
}
