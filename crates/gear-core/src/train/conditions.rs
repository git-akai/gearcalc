//! **What a train asks of its bodies, and how its stages are joined** — the
//! boundary layer, kept apart from the topology ([`super::wiring`]) and the
//! geometry ([`super::shape`]).
//!
//! Three layers with three edit frequencies, and this is the one that changes
//! most: which body is held and which is loaded is what a designer turns to
//! see a different machine, and nothing about the stages moves when they do.
//! It used to live on the stage — a planetary set carried an `Arrangement`
//! naming its input and its held body — which is why a set's ratio was a
//! single number, why two epicyclic stages could not share a body, and why a
//! train could not have a third port: the train had no vocabulary for a body
//! at all.
//!
//! # A body is the train's, and a stage has ends of it
//!
//! A **body** is one thing that turns — a pair's output and the next set's
//! sun on one body are one body — numbered across the train from 1, ground
//! being 0, and named the same way from anywhere: a load's `at`, a hold, a
//! member's `body`, an axis's `carried_by`. A stage lists the bodies on its
//! axes ([`super::shape::BodyOn`]) and numbers them in its own order for its
//! own kinematics (a *slot*, ground 0); a body two stages list is what a
//! coupling used to say, and there is no coupling now — the body is the
//! statement. "Input" and "output" are not names here: they are *readings* of
//! a load case — a loaded body is where power comes in, and which body
//! power leaves by is a result. That is the handoff's point about mobility
//! above one taken seriously: two drives and one load, and one drive with two
//! loads, are the same kinematic object, and asking the designer to declare
//! which is which as structure would be asking them to tell the model what it
//! is about to work out.
//!
//! # The chain is a constructor, not a rule
//!
//! [`Train::chained`] gives each stage's bodies train numbers and joins each
//! stage's conventional output body with the next stage's conventional
//! input, and writes what each stage holds by convention as the train's
//! holds. A train says everything it has: what a file lists is the graph,
//! and what it holds is what it says it holds.

use super::wiring::Wiring;
use crate::kinematics::{Body, Condition, GROUND};
use crate::ratio::Ratio;

/// **A stage's conventional ports and what it holds by default** — what a chain
/// is built from when a train says nothing of its own, and what a stage asked
/// about on its own is solved under.
///
/// Conventions, and named as such: a pair's first member is its input because
/// that is the way round it is written; a set holds its ring and drives its
/// sun because that is the arrangement most sets are built for. Nothing in the
/// solve reads them: a stage's conventional holds are written as the train's
/// when it is inserted, and its conventional ends are where a chain joins
/// and a fresh case starts.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ports {
    /// **Every body a train may couple to, in the order a chain prefers
    /// them.** A pair's two members; a set's three central bodies, sun first;
    /// a hula stage's crank, its output gear, its grounded gear.
    ///
    /// A chain takes the first that is not held as its input and the next as
    /// its output — so a set whose *carrier* a train holds is coupled onward by
    /// its ring, and a designer changing what is held does not also have to
    /// rewire the chain. A body in this list is a port; one not in it — a
    /// planet's, a wobble body's — is nobody's to couple.
    pub ports: Vec<Body>,
    /// The bodies held to ground by convention.
    pub held: Vec<Body>,
}

impl Ports {
    /// **Where a chain enters and leaves this stage**, given what is held —
    /// and, where the train drives one of its ports directly, that one.
    ///
    /// The conventional reading is the first port not held and the next; a
    /// driven port is the input whatever its place in the list, since a drive
    /// is a statement and the order is a preference.
    #[must_use]
    pub fn ends(&self, held: &[Body], driven: Option<Body>) -> (Body, Body) {
        let free: Vec<Body> = self
            .ports
            .iter()
            .copied()
            .filter(|p| !held.contains(p))
            .collect();
        let input = driven
            .filter(|d| self.ports.contains(d))
            .or_else(|| free.first().copied())
            .unwrap_or(self.ports[0]);
        let output = free.iter().copied().find(|&p| p != input).unwrap_or(input);
        (input, output)
    }

    /// The conventional input: the first port, with the conventional holds.
    #[must_use]
    pub fn input(&self) -> Body {
        self.ends(&self.held, None).0
    }

    /// The conventional output.
    #[must_use]
    pub fn output(&self) -> Body {
        self.ends(&self.held, None).1
    }
}

/// **What one stage is asked**, as its solver needs it: a condition per local
/// body, and which body power comes in and leaves by.
///
/// Assembled by the train from its holds, or from the stage's own [`Ports`]
/// where it is asked alone.
#[derive(Clone, Debug, PartialEq)]
pub struct StageBoundary {
    /// One per local body, ground first.
    pub conditions: Vec<Condition>,
    /// The local body a load is referred to.
    pub input: Body,
    /// The local body a load leaves by.
    pub output: Body,
}

impl StageBoundary {
    /// A stage on its own, under its own conventions: ground held, the
    /// conventional bodies held, the conventional input driven at one turn.
    #[must_use]
    pub fn conventional(wiring: &Wiring, ports: &Ports) -> Self {
        let (input, output) = ports.ends(&ports.held, None);
        Self::holding(wiring.slots.len(), &ports.held, input, output)
    }

    /// `slots` slots with `held` fixed to ground and `input` driven
    /// at one turn, the rest free.
    #[must_use]
    pub fn holding(slots: usize, held: &[Body], input: Body, output: Body) -> Self {
        let mut conditions = vec![Condition::Free; slots];
        conditions[GROUND] = Condition::Ground;
        for &h in held {
            conditions[h] = Condition::Ground;
        }
        conditions[input] = Condition::Drive(Ratio::ONE);
        Self {
            conditions,
            input,
            output,
        }
    }

    /// The local bodies held to ground, ascending — what an epicyclic kind
    /// reads its arrangement from.
    #[must_use]
    pub fn held(&self) -> Vec<Body> {
        self.conditions
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, c)| **c == Condition::Ground)
            .map(|(i, _)| i)
            .collect()
    }
}

// ------------------------------------------------- the train as one system ---

use super::graph::Part;
use super::wiring::BodyLabel;
use super::{Shape, Train};

/// **The parts a body is listed on**, with its slot in each, in part order.
pub(crate) fn ends_in(parts: &[Part], body: usize) -> Vec<(usize, Body)> {
    parts
        .iter()
        .enumerate()
        .filter_map(|(k, p)| p.shape.slot_if_any(body).map(|slot| (k, slot)))
        .collect()
}
use crate::kinematics::{Mobility, Refusal, Solution, System};

/// Why a train's motion could not be worked out.
///
/// **None of these is a geometric refusal**, and that is the point: a ratio
/// needs tooth counts and a topology, so a stage whose centre distances cannot
/// be made to agree still has one. `TrainError` is the other question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionError {
    /// A train with no stages has no motion.
    Empty,
    /// A stage's wiring does not describe meshes.
    Wiring(usize, super::WiringError),
    /// A constraint or a load names a body the train does not have.
    NoSuchBody(usize),
    /// **The constraints and the structure cannot both hold, at this body**
    /// — a condition asked of it contradicts what the meshes and the earlier
    /// conditions already decided. Named, because "over-determined" is not
    /// something a designer can act on and "the sun cannot turn while the
    /// carrier and the ring are both held" is.
    Conflicts(usize),
    /// An exact answer too large to represent: the product of tooth counts
    /// along the train has outgrown `i128`, and a ratio is refused rather
    /// than wrapped ([`crate::ratio`]).
    Overflow,
}

/// **What a whole train does, from tooth counts and topology alone.**
///
/// No module, no shift, no material and no load. That independence is the
/// finding this type exists for: `solve_train` refused a train outright when
/// any one stage would not close geometrically, and took every other stage's
/// ratio down with it — though Willis needs none of what failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainMotion {
    /// Every body's turns per turn of the driven end — exactly — by body
    /// number, ground first.
    pub speeds: Vec<Ratio>,
    /// One per stage: its input over its output, or `None` where the output
    /// does not turn — two meshes stepping by the same amount and cancelling,
    /// which is a refusal rather than a very large number.
    pub ratios: Vec<Option<Ratio>>,
    /// The first stage's input to the last stage's output.
    pub total: Option<Ratio>,
    /// How many conditions the train needs, and which bodies nothing touches.
    pub mobility: Mobility,
    /// The solution in full, for a caller that wants a body this does not
    /// name — a member's, through its stage's [`Wiring::mounts`].
    pub solution: Solution,
}

impl TrainMotion {
    /// **`x` at one body, read at another**: a speed, a sweep or a
    /// revolution count stated at `per`, as `of` sees it through the ratios —
    /// and zero where `per` does not turn, which is a body nothing drives.
    ///
    /// **Divided exactly, and the float multiplied in last.** Both speeds
    /// are quotients of tooth counts; the quotient is taken exactly and
    /// [`Ratio::scale`] multiplies before it divides, so the answer rounds
    /// once. `x * (a/b).to_f64()` rounds twice, and the second rounding put
    /// a recorded speed one ULP from the correctly rounded value — measured
    /// here, on `30000 · 17/43`, when this was first written that way.
    #[must_use]
    pub fn read(&self, x: f64, of: usize, per: usize) -> f64 {
        self.solution.values[of]
            .checked_div(self.solution.values[per])
            .map_or(0.0, |r| r.scale(x))
    }
}

impl Train {
    /// **A train of these stages joined as a chain**, each stage's
    /// conventional output body one with the next stage's conventional
    /// input, the cases written by `cases` once the bodies have their train
    /// numbers — so a fixture names a body by its stage and slot
    /// ([`Self::port`]) and the numbers are the train's.
    #[must_use]
    pub fn chained(stages: Vec<Shape>, cases: impl FnOnce(&Self) -> Vec<super::LoadCase>) -> Self {
        let mut train = Self {
            load_cases: Vec::new(),
            reversed_bending: false,
            shape: Shape::default(),
            held: Vec::new(),
        };
        for stage in stages {
            train.push_stage(stage);
        }
        train.load_cases = cases(&train);
        train
    }

    /// **The train's parts** ([`Shape::parts`]) — the pieces that close,
    /// search and rate apart, which is what a card is. Derived: the train
    /// is one graph, and a part is how it falls apart.
    #[must_use]
    pub fn parts(&self) -> Vec<Part> {
        self.shape.parts()
    }

    /// **Each part's shape**, in part order — what a stage was, read off
    /// the graph.
    #[must_use]
    pub fn stages(&self) -> Vec<Shape> {
        self.parts().into_iter().map(|p| p.shape).collect()
    }

    /// **The body at a part's slot** — how a fixture or the harness names a
    /// body by where it is, and the inverse of what the part's own
    /// numbering does. Ground for a slot the part does not have.
    #[must_use]
    pub fn port(&self, part: usize, slot: Body) -> usize {
        self.parts()
            .get(part)
            .map_or(GROUND, |p| p.shape.body_at(slot))
    }

    /// **The slot a body has on a part**, where the part has it — the
    /// inverse of [`Self::port`].
    #[must_use]
    pub fn slot(&self, part: usize, body: usize) -> Option<Body> {
        self.parts()
            .get(part)
            .and_then(|p| p.shape.slot_if_any(body))
    }

    /// **A part's member by the graph's index** — how a fixture changes a
    /// card's gear, the train being one list of them.
    #[must_use]
    pub fn member(&self, part: usize, member: usize) -> usize {
        self.parts()[part].members[member]
    }

    /// The largest body number anything in the train names — the graph, a
    /// case or a hold — ground where nothing does.
    #[must_use]
    pub fn max_body(&self) -> usize {
        let stages = std::iter::once(self.shape.max_body());
        let cases = self.load_cases.iter().flat_map(|c| {
            c.loads.iter().map(|l| l.at).chain(match c.duty {
                super::Duty::Intermittent { at, .. } => Some(at),
                super::Duty::Continuous { .. } => None,
            })
        });
        let holds = self.held.iter().copied();
        stages.chain(cases).chain(holds).max().unwrap_or(GROUND)
    }

    /// **The parts a body is listed on**, with its slot in each, in part
    /// order: one part for a body a part has to itself, more for a shaft
    /// two parts' gears are fixed to.
    #[must_use]
    pub fn ends_of(&self, body: usize) -> Vec<(usize, Body)> {
        ends_in(&self.parts(), body)
    }

    /// **Every body renumbered by `map`** — on every stage, in every case
    /// and every hold — `None` dropping what named it.
    fn renumber(&mut self, map: impl Fn(usize) -> Option<usize>) {
        let keep = |b: usize| b == GROUND || map(b).is_some();
        let to = |b: usize| {
            if b == GROUND {
                GROUND
            } else {
                map(b).unwrap_or(GROUND)
            }
        };
        let s = &mut self.shape;
        s.bodies.retain(|b| keep(b.body));
        s.couplings.retain(|c| c.iter().all(|&b| keep(b)));
        s.renumber_bodies(to);
        self.held.retain(|&b| keep(b));
        for b in &mut self.held {
            *b = to(*b);
        }
        for case in &mut self.load_cases {
            case.loads.retain(|l| keep(l.at));
            for l in &mut case.loads {
                l.at = to(l.at);
            }
            if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                *at = to(*at);
            }
        }
    }

    /// **The bodies numbered densely**, in order, every number nothing
    /// names given up — what every remove ends with, so a body's number is
    /// its place in the list as a gear's is.
    fn prune(&mut self) {
        let max = self.max_body();
        let named: Vec<bool> = (0..=max)
            .map(|b| {
                b == GROUND
                    || self.shape.bodies.iter().any(|x| x.body == b)
                    || self.held.contains(&b)
                    || self.load_cases.iter().any(|c| {
                        c.loads.iter().any(|l| l.at == b)
                            || matches!(c.duty, super::Duty::Intermittent { at, .. } if at == b)
                    })
            })
            .collect();
        let mut next = 0;
        let map: Vec<Option<usize>> = named
            .iter()
            .map(|&n| {
                if n {
                    let m = next;
                    next += 1;
                    Some(m)
                } else {
                    None
                }
            })
            .collect();
        if map.iter().enumerate().all(|(b, m)| *m == Some(b)) {
            return;
        }
        self.renumber(|b| map.get(b).copied().flatten());
    }

    /// **What a case or a hold names that no stage has**, dropped: a body
    /// that left the train with its stage, or with the member that was
    /// alone on it. Not on a train with no stages, where the cases wait
    /// ([`Self::push_stage`]).
    fn drop_orphans(&mut self) {
        if self.shape.members.is_empty() {
            return;
        }
        let on_a_stage = |b: usize| b == GROUND || self.shape.bodies.iter().any(|x| x.body == b);
        self.held.retain(|&b| on_a_stage(b));
        for case in &mut self.load_cases {
            case.loads.retain(|l| on_a_stage(l.at));
            if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                if !on_a_stage(*at) {
                    *at = GROUND;
                }
            }
        }
    }

    /// How many bodies the stages have between them, ground counted — the
    /// nodes of the train's system. A body only a case or a hold names is
    /// no node: it would turn freely, and a train with stages has none
    /// ([`Self::drop_orphans`]), so a hold at one is a hold at nothing.
    fn stage_bodies(&self) -> usize {
        self.shape.max_body() + 1
    }

    /// **The whole train as one system**: one node per body, ground shared,
    /// and each stage's meshes written in terms of the bodies on its axes —
    /// a body two stages list is one node, which is what a coupling row
    /// used to say.
    ///
    /// # Errors
    ///
    /// [`MotionError::Empty`], or a stage whose wiring does not describe
    /// meshes.
    pub fn system(&self) -> Result<System, MotionError> {
        self.system_counting(&self.parts(), |_, _, z| z)
    }

    /// **The system with one gear a tooth larger** — gear `member` of stage
    /// `stage` — which is what a path's *one more tooth* asks.
    ///
    /// # Errors
    ///
    /// As [`Self::system`].
    pub fn system_raising(&self, stage: usize, member: usize) -> Result<System, MotionError> {
        self.system_counting(&self.parts(), |k, i, z| {
            if (k, i) == (stage, member) {
                z + 1
            } else {
                z
            }
        })
    }

    /// The system part by part — each part's rows in its own order, which
    /// is the order its results and a path's play read them in — and then
    /// every coupling no part keeps: a join that could not be coaxial,
    /// between two parts' bodies.
    pub(crate) fn system_counting(
        &self,
        parts: &[Part],
        count: impl Fn(usize, usize, u32) -> u32,
    ) -> Result<System, MotionError> {
        if parts.is_empty() {
            return Err(MotionError::Empty);
        }
        let mut system = System::new(self.stage_bodies());
        for (k, part) in parts.iter().enumerate() {
            let teeth: Vec<u32> = super::teeth_of(part.shape.gears())
                .into_iter()
                .enumerate()
                .map(|(i, z)| count(k, i, z))
                .collect();
            part.shape
                .wiring()
                .add_to(&mut system, &teeth, |slot| part.shape.body_at(slot))
                .map_err(|e| MotionError::Wiring(k, e))?;
        }
        for (c, &[a, b]) in self.shape.couplings.iter().enumerate() {
            if !parts.iter().any(|p| p.couplings.contains(&c)) {
                system
                    .couple(a, b)
                    .ok_or(MotionError::NoSuchBody(a.max(b)))?;
            }
        }
        Ok(system)
    }

    /// **What the train asks of every body**, one condition per body: ground
    /// held, and each body the train holds.
    ///
    /// # Errors
    ///
    /// A hold at a body the train does not have.
    pub fn conditions(&self, bodies: usize) -> Result<Vec<Condition>, MotionError> {
        let mut out = vec![Condition::Free; bodies];
        out[GROUND] = Condition::Ground;
        for &body in &self.held {
            if body == GROUND {
                continue;
            }
            if body >= bodies {
                return Err(MotionError::NoSuchBody(body));
            }
            out[body] = Condition::Ground;
        }
        Ok(out)
    }

    /// Whether a body is listed on a stage before or after `k`.
    fn shared_with(parts: &[Part], body: usize, k: usize, earlier: bool) -> bool {
        ends_in(parts, body)
            .iter()
            .any(|&(s, _)| s != k && ((s < k) == earlier))
    }

    /// **What each stage is asked**, as its own solver needs it: its slots'
    /// conditions, and the slot power comes in and leaves by.
    ///
    /// A stage in a train is driven by what it shares as often as by a
    /// motor, so a body shared with an *earlier* stage is its input and is
    /// driven at one turn for the stage's own solve; one shared with a later
    /// stage is its output. Where neither says, the stage's conventions do.
    /// This is the chain read off the graph rather than assumed of it, and
    /// it is the one place "earlier" means anything — a general graph has
    /// no order, and the train-level family ([`Train::motion`]) needs none.
    ///
    /// # Errors
    ///
    /// As [`Self::system`].
    pub fn boundaries(&self) -> Result<Vec<StageBoundary>, MotionError> {
        self.boundaries_of(&self.parts())
    }

    /// As [`Self::boundaries`], of parts already in hand.
    pub(crate) fn boundaries_of(&self, parts: &[Part]) -> Result<Vec<StageBoundary>, MotionError> {
        let conditions = self.conditions(self.stage_bodies())?;
        let mut out = Vec::with_capacity(parts.len());
        for (k, part) in parts.iter().enumerate() {
            let stage = &part.shape;
            let w = stage.wiring();
            let ports = stage.ports();
            let mut local: Vec<Condition> = (0..w.slots.len())
                .map(|s| conditions[stage.body_at(s)])
                .collect();
            local[GROUND] = Condition::Ground;
            let side = |earlier: bool| -> Option<Body> {
                ports
                    .ports
                    .iter()
                    .copied()
                    .find(|&slot| Self::shared_with(parts, stage.body_at(slot), k, earlier))
            };
            let held: Vec<Body> = local
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, c)| **c == Condition::Ground)
                .map(|(i, _)| i)
                .collect();
            // A body shared with an earlier stage says where power enters;
            // failing that, the body the first case's first load is at,
            // where that is one of this stage's — a set alone loaded at its
            // carrier reads carrier in; failing that the stage's convention
            // — its first port neither held nor shared onward, so a set at
            // the head of a chain sharing its sun onward is entered at its
            // carrier. **A stage's input and output are its own reporting
            // convention** — which way its ratio, its efficiency both ways
            // and its play are read — and decide nothing about a load case,
            // whose loads say what turns; reading the first load here moves
            // no case, since a case names bodies and not ends.
            let onward = side(false);
            let open = |p: &Body| !held.contains(p) && Some(*p) != onward;
            let first_load = self
                .load_cases
                .iter()
                .filter(|c| c.enabled)
                .flat_map(|c| c.loads.iter())
                .find(|l| l.is_load())
                .and_then(|l| {
                    let slot = stage.slot_if_any(l.at)?;
                    ports.ports.contains(&slot).then_some(slot)
                })
                .filter(open);
            let input = side(true).or(first_load).unwrap_or_else(|| {
                ports
                    .ports
                    .iter()
                    .copied()
                    .find(open)
                    .unwrap_or_else(|| ports.ends(&held, None).0)
            });
            // **The output is chosen knowing the input.** A set behind a pair
            // and sharing its *ring* with it had its conventional output read
            // with no input in hand — and the convention, with the sun held,
            // is "carrier in, ring out", so the ring was named both ends.
            // **A load names nothing here.** Which of two free ports is "the
            // output" decides only the stage's own no-load figures — which a
            // stage with two free ports has none of, its motion being a
            // family. The convention stands.
            let output = onward.unwrap_or_else(|| ports.ends(&held, Some(input)).1);
            local[input] = Condition::Drive(Ratio::ONE);
            // **A stage held still is named at the hold that locked it**: a
            // set with its carrier and its ring both held cannot be entered
            // at its sun at all, and the designer's own holds go in last so
            // the one that closed the set is the one named — the ring, not
            // the sun the convention drives.
            let holds: Vec<Body> = self
                .held
                .iter()
                .filter_map(|&b| stage.slot_if_any(b))
                .collect();
            let first: Vec<Body> = (0..local.len()).filter(|s| !holds.contains(s)).collect();
            if let Err(Refusal::Conflicts(i)) = w
                .alone(&super::teeth_of(stage.gears()))
                .map_err(|e| MotionError::Wiring(k, e))?
                .motion_in(&local, &first)
            {
                return Err(MotionError::Conflicts(stage.body_at(i)));
            }
            out.push(StageBoundary {
                conditions: local,
                input,
                output,
            });
        }
        Ok(out)
    }

    /// **The train's motion**, at one turn of what is driven.
    ///
    /// # Errors
    ///
    /// [`MotionError`] — and never a geometric one, which is the whole reason
    /// this is separate from [`super::solve_train`].
    pub fn motion(&self) -> Result<TrainMotion, MotionError> {
        self.motion_of(&self.parts())
    }

    /// As [`Self::motion`], of the graph cut into `parts`.
    pub(crate) fn motion_of(&self, parts: &[Part]) -> Result<TrainMotion, MotionError> {
        let system = self.system_counting(parts, |_, _, z| z)?;
        let mut conditions = self.conditions(system.bodies())?;
        // **The motion is read along the headline case**, at one turn of its
        // load: a fresh train's reading is its first case. With no case to
        // say so nothing is driven, the motion is a family, and each case
        // decides its own.
        let boundaries = self.boundaries_of(parts)?;
        let ends = self.headline();
        if let Some(a) = self.headline_load() {
            conditions[a] = Condition::Drive(Ratio::ONE);
        }
        // **Conventions in first, the train's own last**, so that a conflict
        // is named at the statement the designer made rather than at the
        // convention it contradicts: holding a set's carrier beside its
        // held ring is reported at the carrier.
        let order: Vec<Body> = std::iter::once(GROUND)
            .chain(self.held.iter().copied())
            .collect();
        let solution = system.motion_in(&conditions, &order).map_err(|e| match e {
            Refusal::Conflicts(i) => MotionError::Conflicts(i),
            Refusal::NoMotion | Refusal::Overflow => MotionError::Overflow,
        })?;
        // **A family reads per turn of a body.** The solver parameterises
        // what is free at whichever body fell last in its elimination — a
        // planet, on a set with its ring released — and a designer wants the
        // ring. The open bodies nothing shares come first, then any port,
        // and a planet only where no port moves with the freedom.
        let mut preferred: Vec<Body> = Vec::new();
        for open in [true, false] {
            for part in parts {
                for slot in part.shape.ports().ports {
                    let b = part.shape.body_at(slot);
                    let shared = ends_in(parts, b).len() > 1;
                    if (conditions[b] == Condition::Free && !shared) == open
                        && !preferred.contains(&b)
                    {
                        preferred.push(b);
                    }
                }
            }
        }
        let solution = solution.rebased(&preferred).ok_or(MotionError::Overflow)?;
        let mobility = system.mobility().ok_or(MotionError::Overflow)?;
        let speeds = solution.values.clone();
        let ratios: Vec<Option<Ratio>> = boundaries
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let (i, o) = (
                    parts[k].shape.body_at(b.input),
                    parts[k].shape.body_at(b.output),
                );
                // `None` where the answer is a family: the quotient of two
                // families is not a number, and the particular values alone
                // would print one as if it were.
                solution.ratio(i, o)
            })
            .collect();
        let total = match ends {
            Some((a, b)) => solution.ratio(a, b),
            None => None,
        };
        Ok(TrainMotion {
            speeds,
            ratios,
            total,
            mobility,
            solution,
        })
    }
}

// ---------------------------------------------- where a load can enter ---

impl Train {
    /// **Every body a load can enter by**: every body the train does not
    /// hold that some stage has as a port, in the order the chain runs.
    /// What a picker offers, and what [`super::solve_train`] admits a load
    /// at.
    #[must_use]
    pub fn open_ports(&self, boundaries: &[StageBoundary]) -> Vec<PortBody> {
        self.bodies(boundaries)
            .into_iter()
            .filter(|b| !b.held)
            .collect()
    }

    /// **The headline case's path**: the first case switched on, from its
    /// first load to its first reaction, where both are open bodies of the
    /// train and not one body. What the train's motion is read along; a
    /// train with no such case has no reading of its own, and each case
    /// decides its own motion.
    #[must_use]
    pub fn headline(&self) -> Option<(usize, usize)> {
        let load = self.headline_load()?;
        let reaction = self
            .load_cases
            .iter()
            .find(|c| c.enabled)?
            .loads
            .iter()
            .find(|l| l.role == super::LoadRole::Reacted)?
            .at;
        (load != reaction && self.is_open(reaction)).then_some((load, reaction))
    }

    /// **Where the headline case's first load enters**, at an open body:
    /// what the train's motion is driven at, whether or not the case
    /// reacts it anywhere a path could end.
    #[must_use]
    pub fn headline_load(&self) -> Option<usize> {
        let case = self.load_cases.iter().find(|c| c.enabled)?;
        let load = case.loads.iter().find(|l| l.is_load())?.at;
        self.is_open(load).then_some(load)
    }

    /// A body some stage has that the train does not hold.
    fn is_open(&self, body: usize) -> bool {
        body != GROUND && !self.held.contains(&body) && !self.ends_of(body).is_empty()
    }

    /// **A chain's two ends by convention**: the first stage's conventional
    /// input and the last stage's conventional output, where each is open
    /// and no other stage's — where a case starts on a train that has none,
    /// and where a fixture writes its cases. A seed for a case and nothing
    /// the train reports: its figures are its cases'. `None` where either
    /// is held or shared, or the train is one stage with one open port.
    #[must_use]
    pub fn chain_ends(&self) -> Option<(usize, usize)> {
        let boundaries = self.boundaries().ok()?;
        let boundaries = boundaries.as_slice();
        let (first, last) = (boundaries.first()?, boundaries.last()?);
        let a = self.port(0, first.input);
        let b = self.port(boundaries.len() - 1, last.output);
        let open = self.open_ports(boundaries);
        let single =
            |body: usize| open.iter().any(|p| p.body == body) && self.ends_of(body).len() == 1;
        (a != b && single(a) && single(b)).then_some((a, b))
    }
}

// --------------------------------------------- what crosses the boundary ---

/// A number that is exactly a quotient of integers, and how it reads.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Exact {
    /// For arithmetic and for a readout that rounds.
    pub value: f64,
    /// The quotient itself — `3721/16`, or `49` — for a readout that does not.
    /// Digits and a slash, not a word.
    pub text: String,
}

impl From<Ratio> for Exact {
    fn from(r: Ratio) -> Self {
        Self {
            value: r.to_f64(),
            text: r.to_string(),
        }
    }
}

/// One of a stage's ports, as the front end names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PortSpec {
    /// The stage's own numbering of it.
    pub slot: Body,
    /// The train's body it is.
    pub body: usize,
}

/// **A stage's ports and its conventional holds**, so a panel can offer
/// exactly the bodies a train may constrain or share — read from the
/// stage's own wiring rather than written into the front end a second time.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct StagePorts {
    pub ports: Vec<PortSpec>,
    /// What each member is — sun, planet, ring, worm, wheel or a gear by
    /// its number — read off the shape by the one rule
    /// ([`super::shape::Shape::member_names`]), so a panel names a member
    /// as the harness does without deriving it a second time.
    pub members: Vec<super::shape::MemberName>,
    /// **The mesh groups** — the mesh graph's connected components
    /// ([`super::shape::Shape::mesh_groups`]), the members a run of meshes
    /// joins — so a panel offers one module and one pressure angle per
    /// group and writes them to every member in it, and deals the cards a
    /// group to a row.
    pub mesh_groups: Vec<Vec<usize>>,
    /// **The family the shape reads as** ([`super::shape::Shape::family`]),
    /// which decides the card's structural buttons and its chip — the
    /// core's reading, so the panel does not derive it a second time.
    pub family: super::StageFamily,
    /// **The part the card is** — its shape, in its own numbering, and
    /// where each of its pieces is in the train's one graph, which is what
    /// a panel binds a card's inputs through.
    pub part: Part,
}

/// One end of a body: the stage it is listed on, and what it is there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct BodyEnd {
    pub stage: usize,
}

/// One body of the train's motion, for the front end.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct BodyReport {
    pub body: usize,
    /// Every stage it is listed on, and what it is there — a slot that is
    /// no port too, a planet's.
    pub ends: Vec<BodyEnd>,
    /// **A body a train may be addressed at**: one some stage has as a
    /// port, which is every body but a replicated one — a planet orbits
    /// and nothing can be attached to it.
    pub port: bool,
    /// **Held by the train** — ground under another name — so no case can
    /// say anything of it. A port and held is a body the picker does not
    /// offer; a port and not held is what a load can enter by.
    pub held: bool,
    /// Turns per turn of what is driven — the whole answer where it is one
    /// answer, and the particular part of it where it is a family.
    pub speed: Exact,
    /// **The rest of a family**: one term per free body this one depends
    /// on, *coefficient turns per turn of that body*. Empty where the
    /// answer is one answer.
    pub terms: Vec<Term>,
}

/// One term of a body's speed in a family: so many turns per turn of a
/// body the conditions left free.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Term {
    pub per: usize,
    pub coefficient: Exact,
}

/// **The train's motion as the front end receives it** — present whenever the
/// tooth counts and topology give one, which is whether or not the geometry
/// solved.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MotionReport {
    /// How many independent conditions the mechanism needs beyond its frame
    /// — 1 for a chain, 2 for a set with nothing held.
    pub mobility: usize,
    /// How many it has been given, so a panel can say *one short* or *one
    /// too many* rather than leaving a designer to count.
    pub constrained: usize,
    /// Bodies no mesh touches — named, not counted. Ground is the frame and
    /// is not listed.
    pub untouched: Vec<usize>,
    /// **Every body, ground first** — its ends, whether a case may address
    /// it, whether the train holds it, and what it turns at. One list: the
    /// ports are the ones flagged, and what a load may enter by is a port
    /// the train does not hold.
    pub bodies: Vec<BodyReport>,
    /// One per stage, input over output. `None` where the output does not
    /// turn.
    pub ratios: Vec<Option<Exact>>,
    /// The first stage's input to the last stage's output. `None` where the
    /// answer is a family, since a quotient of two families is not a number.
    pub total: Option<Exact>,
    /// **The bodies whose turn parameterises a family** — one per condition
    /// the train is short — and empty where the answer is one answer. Every
    /// [`BodyReport::terms`] is per turn of one of these.
    pub free: Vec<usize>,
    /// Bodies whose condition said nothing the structure had not already
    /// said. Not a fault — a ring held and also fixed by its stage is a
    /// designer being explicit — but worth a reader's knowing.
    pub redundant: Vec<usize>,
}

/// **One body a train may be addressed at**: a port, on one stage or
/// several — a pair's output and the next set's sun are one body with two
/// ends, and a case says one thing of it.
///
/// The core's own answer, which [`BodyReport`] carries to the front end
/// beside the body's speed rather than in a list of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortBody {
    pub body: usize,
    /// Every stage it is a port of, in stage order, with its name there.
    pub ends: Vec<(usize, BodyLabel)>,
    /// Held by the train — ground under another name — so no case can say
    /// anything of it.
    pub held: bool,
}

impl Train {
    /// Every stage's ports, for a panel to offer.
    #[must_use]
    pub fn topology(&self) -> Vec<StagePorts> {
        self.parts()
            .into_iter()
            .map(|part| {
                let stage = &part.shape;
                StagePorts {
                    members: stage.member_names(),
                    mesh_groups: stage.mesh_groups(),
                    family: stage.family(),
                    ports: stage
                        .ports()
                        .ports
                        .iter()
                        .map(|&slot| PortSpec {
                            slot,
                            body: stage.body_at(slot),
                        })
                        .collect(),
                    part: part.clone(),
                }
            })
            .collect()
    }

    /// The train's motion in the shape the boundary sends, or `None` where
    /// there is none to send.
    ///
    /// **A family is sent as a family.** Where the conditions leave `m`
    /// bodies free, every body's speed is a particular value plus one term
    /// per free body — `ω_i = v_i + Σ_k c_ik · ω_k` — with the free bodies
    /// named in `free`, so a differential's *"the carrier turns at half the
    /// sum of its two sides"* is what a reader sees rather than a refusal.
    #[must_use]
    pub fn motion_report(&self) -> Option<MotionReport> {
        let m = self.motion().ok()?;
        // **The mechanism's mobility, not the matrix's.** Ground is a body
        // in the system and the frame in the world: it counts one degree
        // and one condition in the matrix, and neither to a designer, who
        // reads "mobility 2, one given" of a set with its ring released —
        // the one given being the drive at the train's end its ratio is read
        // from, which the holds do not count.
        let boundaries = self.boundaries().ok()?;
        let constrained = self
            .conditions(m.solution.values.len())
            .ok()?
            .iter()
            .skip(1)
            .filter(|c| **c != Condition::Free)
            .count()
            + usize::from(self.headline_load().is_some());
        let free: Vec<usize> = m.solution.residual.iter().map(|r| r.at).collect();
        let ports = self.bodies(&boundaries);
        let ends = |body: usize| -> Vec<BodyEnd> {
            if body == GROUND {
                return vec![];
            }
            self.ends_of(body)
                .into_iter()
                .map(|(stage, _)| BodyEnd { stage })
                .collect()
        };
        Some(MotionReport {
            mobility: m.mobility.degrees.saturating_sub(1),
            constrained,
            untouched: m
                .mobility
                .untouched
                .iter()
                .copied()
                .filter(|&i| i != GROUND)
                .collect(),
            bodies: m
                .speeds
                .iter()
                .enumerate()
                .map(|(i, s)| BodyReport {
                    body: i,
                    ends: ends(i),
                    port: ports.iter().any(|p| p.body == i),
                    held: ports.iter().any(|p| p.body == i && p.held),
                    speed: (*s).into(),
                    terms: m
                        .solution
                        .residual
                        .iter()
                        .zip(&free)
                        .filter(|(r, _)| !r.direction[i].is_zero())
                        .map(|(r, &per)| Term {
                            per,
                            coefficient: r.direction[i].into(),
                        })
                        .collect(),
                })
                .collect(),
            ratios: m.ratios.iter().map(|r| r.map(Exact::from)).collect(),
            total: m.total.map(Exact::from),
            free,
            redundant: m.solution.redundant.clone(),
        })
    }

    /// **Every body some stage has as a port** — see [`PortBody`] — in
    /// body order, each with its ends.
    #[must_use]
    pub fn bodies(&self, boundaries: &[StageBoundary]) -> Vec<PortBody> {
        self.bodies_of(&self.parts(), boundaries)
    }

    /// As [`Self::bodies`], of the graph cut into `parts`.
    pub(crate) fn bodies_of(&self, parts: &[Part], boundaries: &[StageBoundary]) -> Vec<PortBody> {
        let mut out: Vec<PortBody> = Vec::new();
        for (k, part) in parts.iter().enumerate() {
            let stage = &part.shape;
            let w = stage.wiring();
            let Some(b) = boundaries.get(k) else { break };
            for slot in stage.ports().ports {
                let body = stage.body_at(slot);
                let held = b.conditions[slot] == Condition::Ground;
                if let Some(x) = out.iter_mut().find(|x| x.body == body) {
                    x.ends.push((k, w.slots[slot]));
                    x.held |= held;
                } else {
                    out.push(PortBody {
                        body,
                        ends: vec![(k, w.slots[slot])],
                        held,
                    });
                }
            }
        }
        out.sort_by_key(|b| b.body);
        out
    }

    /// **Two bodies made one**: everything that named `b` names `a` now —
    /// every part's end of it, every case entry, every hold — and `b`'s
    /// number is given up; the axes the two turned about are one line,
    /// since a shaft is straight ([`Self::merge`]). A reaction declared at
    /// either becomes a load with its torque derived — an inline take-off,
    /// the same physics — since a body two parts share cannot be a
    /// reaction; a free entry at either is dropped, there being nothing
    /// free about it now; and a case with an entry at each keeps the
    /// first. Joining a body to itself changes nothing.
    ///
    /// **A join that cannot be coaxial is an offset coupling**: an end on
    /// an axis a carrier turns orbits, and no shaft on a fixed axis can be
    /// the same body — so the two keep their numbers and turn as one
    /// through a coupling, which is what the join meant. Two ends at an
    /// axis distance from each other are refused, as is a part with both.
    pub fn join(&mut self, a: usize, b: usize) {
        if a == b || a == GROUND || b == GROUND {
            return;
        }
        // A part with both would have one body at two of its slots, which
        // is a mesh or a carrier turning against itself: not a body.
        let both = |p: &Part| p.shape.slot_if_any(a).is_some() && p.shape.slot_if_any(b).is_some();
        if self.parts().iter().any(both) {
            return;
        }
        let axis = |body: usize| {
            self.shape
                .bodies
                .iter()
                .find(|x| x.body == body)
                .map(|x| x.axis)
        };
        if let (Some(x), Some(y)) = (axis(a), axis(b)) {
            let carried = |axis: usize| self.shape.axes[axis].carried_by != GROUND;
            if carried(x) || carried(y) {
                if !self
                    .shape
                    .couplings
                    .iter()
                    .any(|c| c.contains(&a) && c.contains(&b))
                {
                    self.shape.couplings.push([a, b]);
                }
                return;
            }
            let apart = |d: &super::shape::Distance| d.axes == [x, y] || d.axes == [y, x];
            if x != y && self.shape.distances.iter().any(apart) {
                return;
            }
        }
        // The lower number survives, which is the order a chain names in.
        let (a, b) = if b < a { (b, a) } else { (a, b) };
        for case in &mut self.load_cases {
            case.loads
                .retain(|l| !((l.at == a || l.at == b) && l.role == super::LoadRole::Free));
            for l in &mut case.loads {
                if (l.at == a || l.at == b) && l.role == super::LoadRole::Reacted {
                    l.role = super::LoadRole::Load;
                    l.torque.auto = true;
                }
            }
        }
        self.merge(a, b);
    }

    /// Everything that named `b` names `a`, once, and `b`'s number is given
    /// up: what [`Self::join`] does once the roles are settled, and what a
    /// parked case's body becomes when a stage takes it up.
    ///
    /// **Where both are on axes, the two axes are one line** — every body
    /// and distance on the later moved to the earlier, whose reading stands
    /// ([`Shape::merge_axes`]) — and the body is listed once, where it was
    /// first listed, so each part keeps the order it numbers its bodies in.
    fn merge(&mut self, a: usize, b: usize) {
        self.keep_orders(a, b);
        let axis =
            |s: &Shape, body: usize| s.bodies.iter().find(|x| x.body == body).map(|x| x.axis);
        if let (Some(x), Some(y)) = (axis(&self.shape, a), axis(&self.shape, b)) {
            self.shape.merge_axes(x.min(y), x.max(y));
        }
        self.renumber(|x| Some(if x == b { a } else { x }));
        let mut seen: Vec<usize> = Vec::new();
        self.shape.bodies.retain(|x| {
            let first = !seen.contains(&x.body);
            seen.push(x.body);
            first
        });
        // A coupling between the two couples nothing now.
        self.shape.couplings.retain(|c| c[0] != c[1]);
        for case in &mut self.load_cases {
            let mut seen = false;
            case.loads.retain(|l| {
                if l.at != a {
                    return true;
                }
                let keep = !seen;
                seen = true;
                keep
            });
        }
        self.held.sort_unstable();
        self.held.dedup();
        self.prune();
    }

    /// **Every part keeps the order it numbers its bodies in** through a
    /// merge of `a` and `b`: the body is listed once, where it was listed
    /// first, and the bodies a part at the later listing numbers before
    /// it — and no part at the earlier one has — move ahead of it with
    /// it. A part's slots are the order its conventions read (a set's sun
    /// first), so a join does not reorder a card.
    fn keep_orders(&mut self, a: usize, b: usize) {
        let at = |body: usize| self.shape.bodies.iter().position(|x| x.body == body);
        let (Some(ia), Some(ib)) = (at(a), at(b)) else {
            return;
        };
        let (first, last) = (ia.min(ib), ia.max(ib));
        let (earlier, later) = (self.shape.bodies[first].body, self.shape.bodies[last].body);
        let parts = self.parts();
        let with = |body: usize, x: usize| {
            parts
                .iter()
                .any(|p| p.shape.slot_if_any(body).is_some() && p.shape.slot_if_any(x).is_some())
        };
        let ahead: Vec<usize> = (first + 1..last)
            .filter(|&i| {
                let x = self.shape.bodies[i].body;
                with(later, x) && !with(earlier, x)
            })
            .collect();
        let moved: Vec<super::shape::BodyOn> =
            ahead.iter().map(|&i| self.shape.bodies[i]).collect();
        for &i in ahead.iter().rev() {
            self.shape.bodies.remove(i);
        }
        for (j, entry) in moved.into_iter().enumerate() {
            self.shape.bodies.insert(first + j, entry);
        }
    }

    /// **A part's end of a body made its own** — split off as a body of
    /// its own where another part has it too, the body it leaves keeping
    /// its number and whatever the train wrote on it, and the part's
    /// members, carriers and couplings on it moving to the new one, listed
    /// where the body is so the part numbers its bodies as it did; and
    /// uncoupled from another part's body, where a join that could not be
    /// coaxial coupled it ([`Self::join`]). The end's number is returned —
    /// the body's own where it was the part's alone.
    pub fn split(&mut self, stage: usize, body: usize) -> usize {
        let parts = self.parts();
        let Some(part) = parts.get(stage) else {
            return body;
        };
        if part.shape.slot_if_any(body).is_none() {
            return body;
        }
        let across: Vec<usize> = (0..self.shape.couplings.len())
            .filter(|c| {
                self.shape.couplings[*c].contains(&body)
                    && !parts.iter().any(|p| p.couplings.contains(c))
            })
            .collect();
        for &c in across.iter().rev() {
            self.shape.couplings.remove(c);
        }
        if ends_in(&parts, body).len() < 2 {
            return body;
        }
        let fresh = self.max_body() + 1;
        let s = &mut self.shape;
        for &i in &part.members {
            if s.members[i].body == body {
                s.members[i].body = fresh;
            }
        }
        for &a in &part.axes {
            if s.axes[a].carried_by == body {
                s.axes[a].carried_by = fresh;
            }
        }
        for &c in &part.couplings {
            for x in &mut s.couplings[c] {
                if *x == body {
                    *x = fresh;
                }
            }
        }
        if let Some(at) = s.bodies.iter().position(|x| x.body == body) {
            let axis = s.bodies[at].axis;
            s.bodies
                .insert(at + 1, super::shape::BodyOn { body: fresh, axis });
        }
        fresh
    }

    /// **A stage's end of a body moved to another** — what the select
    /// beside a port means, one rule: the end is split off where the body
    /// ran on to another stage ([`Self::split`]), and then is held to
    /// ground (`to` ground), made one with the body named (`to` another,
    /// [`Self::join`]), or left a body of its own with every hold on it
    /// withdrawn (`to` none, or the body itself, [`Self::release`]).
    pub fn move_end(&mut self, stage: usize, body: usize, to: Option<usize>) {
        let mine = self.split(stage, body);
        match to {
            Some(GROUND) => self.hold(mine),
            Some(other) if other != mine => self.join(mine, other),
            _ => self.release(mine),
        }
    }

    /// **A body held to ground**, in so many words. Every case entry at it
    /// goes with it: a held body is fixed, and no case can say anything of
    /// it.
    pub fn hold(&mut self, body: usize) {
        if body == GROUND {
            return;
        }
        if !self.held.contains(&body) {
            self.held.push(body);
        }
        for case in &mut self.load_cases {
            case.loads.retain(|l| l.at != body);
        }
    }

    /// **A body released**: the hold on it taken out, and nothing written
    /// in its place — a body the train does not hold is free.
    pub fn release(&mut self, body: usize) {
        self.held.retain(|&b| b != body);
    }

    /// **The bodies a case names that no stage has** — where the cases
    /// wait while the train has no stages, in number order: what was at
    /// the last stage's input first, at its output second.
    fn parked(&self) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .load_cases
            .iter()
            .flat_map(|c| c.loads.iter().map(|l| l.at))
            .filter(|&b| b != GROUND && self.ends_of(b).is_empty())
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// **A fresh case of this kind along the headline case's path** — or,
    /// on a train with no case, between the chain's two ends: a torque at
    /// the first, driven at a speed, reacted at the second, the duty's sweep
    /// measured at the second — the case a panel's button adds, **switched
    /// off**, so a case added at its default figures moves no rating until
    /// the designer has written it and switched it on. A train with neither
    /// gets it at two bodies of its own, parked for the first stage to take
    /// up or the designer to move.
    #[must_use]
    pub fn fresh_case(&self, kind: super::CaseKind, torque: f64, speed: f64) -> super::LoadCase {
        let ends = self.headline().or_else(|| self.chain_ends());
        let (input, output) = ends.unwrap_or_else(|| {
            let parked = self.parked();
            match parked.as_slice() {
                [a, b, ..] => (*a, *b),
                [a] => (*a, self.max_body() + 1),
                [] => (self.max_body() + 1, self.max_body() + 2),
            }
        });
        let mut case = match kind {
            super::CaseKind::Ultimate => super::LoadCase::ultimate(input, output, torque, speed),
            super::CaseKind::Fatigue => super::LoadCase::fatigue(input, output, torque, speed),
        };
        case.enabled = false;
        case
    }

    /// **A stage removed.** Its bodies leave with it where no other stage
    /// has them, and every case entry and hold at those goes too — except
    /// on the last stage, whose cases stay parked at their bodies, every
    /// figure kept, for the first stage pushed to take up again
    /// ([`Self::push_stage`]), so a designer who swaps their only stage for
    /// another keeps their loads. The bodies left are numbered densely
    /// again.
    pub fn remove_stage(&mut self, k: usize) {
        let Some(part) = self.parts().into_iter().nth(k) else {
            return;
        };
        self.shape.remove_part(&part);
        self.drop_orphans();
        self.prune();
    }

    /// **A part edited on its card** ([`super::StageEdit`]): the graph
    /// takes the edit, read in the part's own indices
    /// ([`Shape::edit_part`]), numbering any body it adds after every body
    /// the train has; a body the edit took off that nothing else has
    /// leaves the train, with every case entry and hold at it, and the rest
    /// are numbered densely again.
    ///
    /// **A body an edit leaves empty stays while anything still names it**
    /// — another stage, a hold, a case — and is given up otherwise
    /// ([`Self::drop_bare`]). A gear moved off a shaft does not take the
    /// shaft with it: that is how a layshaft's engaged ratio is changed,
    /// one gear off and the other on, with the output the train couples to
    /// standing still there between the two.
    ///
    /// # Errors
    ///
    /// [`super::EditRefused`] where the shape refuses, with nothing changed.
    pub fn edit_stage(
        &mut self,
        k: usize,
        edit: super::StageEdit,
    ) -> Result<(), super::EditRefused> {
        let next = self.max_body() + 1;
        let part = self
            .parts()
            .into_iter()
            .nth(k)
            .ok_or(super::EditRefused::NoSuchIndex)?;
        self.shape.edit_part(&part, edit, next)?;
        self.drop_bare();
        self.drop_orphans();
        self.prune();
        Ok(())
    }

    /// **Every body with nothing on it that nothing else names, given up.**
    ///
    /// A body with no member, no axis to carry and no coupling is a shaft
    /// in neutral, which is a state worth being able to reach — but only
    /// while something means it to be there. A hold or a case's entry is a
    /// reason. Neither, and it is a number nothing would ever read again.
    fn drop_bare(&mut self) {
        let named = |body: usize| {
            self.held.contains(&body)
                || self.load_cases.iter().any(|c| {
                    c.loads.iter().any(|l| l.at == body)
                        || matches!(c.duty, super::Duty::Intermittent { at, .. } if at == body)
                })
        };
        let s = &self.shape;
        let bare: Vec<usize> = s
            .bodies
            .iter()
            .map(|b| b.body)
            .filter(|&b| {
                s.members_on_body(b).is_empty()
                    && !s.carries_an_axis(b)
                    && !s.couplings.iter().any(|c| c.contains(&b))
                    && !named(b)
            })
            .collect();
        self.shape.bodies.retain(|b| !bare.contains(&b.body));
    }

    /// **A case's duty switched**, to intermittent or continuous, seeded from
    /// the same numbers a fresh case starts with: a thousand sweeps of 25°,
    /// measured at the case's reaction — its first reacted entry, else its
    /// first entry of any kind, else ground — or a thousand hours.
    pub fn set_duty(&mut self, case: usize, intermittent: bool) {
        let Some(c) = self.load_cases.get_mut(case) else {
            return;
        };
        let at = c
            .loads
            .iter()
            .find(|l| l.role == super::LoadRole::Reacted)
            .or_else(|| c.loads.first())
            .map_or(GROUND, |l| l.at);
        c.duty = if intermittent {
            super::Duty::intermittent(at)
        } else {
            super::Duty::Continuous {
                runtime_hours: 1000.0,
            }
        };
    }

    /// **A stage appended to the train and joined onward**: its bodies
    /// given train numbers after every body the train has, and its
    /// conventional input made one with the last stage's remaining open
    /// output, where the train has exactly one to give — a chain grows by
    /// one — and left its own otherwise, an isolated stage for the designer
    /// to tie in. Every case entry at the body just shared moves to the new
    /// stage's conventional output: a load or a reaction at what was the
    /// chain's end is at its new end, which is what the chain did without
    /// saying so. **The first stage takes up the parked cases** at its
    /// conventional input and output.
    pub fn push_stage(&mut self, mut stage: Shape) {
        let parts = self.parts();
        let k = parts.len();
        let next = self.max_body() + 1;
        // The stage's own numbering, moved above everything the train has:
        // its slot `i` becomes body `next + i - 1`.
        let slots: Vec<usize> = stage.bodies.iter().map(|b| b.body).collect();
        stage.renumber_bodies(|b| slots.iter().position(|&x| x == b).map_or(b, |i| next + i));
        let ports = stage.ports();
        let (input, output) = (ports.input(), ports.output());
        // **What the stage holds by convention is written**, in train
        // numbers: from here on the train holds it because it says so.
        let held: Vec<usize> = ports.held.iter().map(|&slot| stage.body_at(slot)).collect();
        let onward = self.boundaries_of(&parts).ok().and_then(|b| {
            let open = self.open_ports(&b);
            let last = k.checked_sub(1)?;
            let shape = &parts[last].shape;
            let conventional = shape.body_at(shape.ports().output());
            let mine: Vec<usize> = open
                .iter()
                .map(|p| p.body)
                .filter(|&body| ends_in(&parts, body).iter().any(|&(s, _)| s == last))
                .collect();
            if mine.contains(&conventional) {
                Some(conventional)
            } else if let [only] = mine.as_slice() {
                Some(*only)
            } else {
                None
            }
        });
        // Where the stage's slots land in the graph's list of bodies, which
        // no merge below reorders.
        let at = self.shape.bodies.len();
        self.shape.append(stage);
        let body = |train: &Self, slot: Body| train.shape.bodies[at + slot - 1].body;
        for body in held {
            if !self.held.contains(&body) {
                self.held.push(body);
            }
        }
        if k == 0 {
            // Taken up as they were: a reaction parked at the output is a
            // reaction at the stage's, not a body two stages share. Each
            // merge closes the numbers up, so the second is read afresh.
            if let Some(&a) = self.parked().first() {
                self.merge(body(self, input), a);
            }
            if let Some(&b) = self.parked().first() {
                self.merge(body(self, output), b);
            }
            return;
        }
        let (input, output) = (body(self, input), body(self, output));
        if let Some(from) = onward {
            for case in &mut self.load_cases {
                for l in &mut case.loads {
                    if l.at == from {
                        l.at = output;
                    }
                }
                if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                    if *at == from {
                        *at = output;
                    }
                }
            }
            self.join(from, input);
        }
    }
}
