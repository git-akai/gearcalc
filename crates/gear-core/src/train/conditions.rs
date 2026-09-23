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
//! a [`Constraint`] — a loaded body is where power comes in, and which body
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
//! input; a train with no constraints of its own holds what each stage holds
//! by convention. A train says everything it has: what a file lists is the
//! graph.

use super::wiring::Wiring;
use crate::kinematics::{Body, Condition, GROUND};
use crate::ratio::Ratio;

/// What is asked of one body. Two things, and no third.
///
/// The train-level reading of [`crate::kinematics::Condition`], without the
/// drive's speed: a train's motion is solved at one turn of whatever is driven,
/// and a load case's speed scales it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Constraint {
    /// Fixed to ground.
    Held,
    /// Not held — it does what the rest decides. Whether it carries a
    /// torque, or drives, is a load case's question ([`super::LoadRole`]):
    /// the output of a set is free in exactly this sense and carries the
    /// whole load. (There was a third word, *driven*, from when the train
    /// was a chain with a head; what drives is a load on an open body now,
    /// and nothing more.)
    Free,
}

impl Constraint {
    /// The condition the solver takes.
    #[must_use]
    pub const fn condition(self) -> Condition {
        match self {
            Self::Held => Condition::Ground,
            Self::Free => Condition::Free,
        }
    }
}

/// One body, and what is asked of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct BodyConstraint {
    pub body: usize,
    pub constraint: Constraint,
}

impl BodyConstraint {
    /// A body held to ground.
    #[must_use]
    pub const fn held(body: usize) -> Self {
        Self {
            body,
            constraint: Constraint::Held,
        }
    }

    /// A body released from a hold a stage's convention put on it.
    #[must_use]
    pub const fn free(body: usize) -> Self {
        Self {
            body,
            constraint: Constraint::Free,
        }
    }
}

/// **A stage's conventional ports and what it holds by default** — what a chain
/// is built from when a train says nothing of its own, and what a stage asked
/// about on its own is solved under.
///
/// Conventions, and named as such: a pair's first member is its input because
/// that is the way round it is written; a set holds its ring and drives its
/// sun because that is the arrangement most sets are built for. Nothing in the
/// solve depends on these being the *only* way to ask — a train's own
/// constraints override every one of them.
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

use super::wiring::BodyLabel;
use super::{Shape, Train};
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
            stages: Vec::new(),
            constraints: Vec::new(),
        };
        for stage in stages {
            train.push_stage(stage);
        }
        train.load_cases = cases(&train);
        train
    }

    /// **The body at a stage's slot** — how a fixture or the harness names a
    /// body by where it is, and the inverse of what the stage's own
    /// numbering does. Ground for a slot the stage does not have.
    #[must_use]
    pub fn port(&self, stage: usize, slot: Body) -> usize {
        self.stages.get(stage).map_or(GROUND, |s| s.body_at(slot))
    }

    /// **The slot a body has on a stage**, where the stage has it — the
    /// inverse of [`Self::port`].
    #[must_use]
    pub fn slot(&self, stage: usize, body: usize) -> Option<Body> {
        self.stages.get(stage).and_then(|s| s.slot_if_any(body))
    }

    /// The largest body number anything in the train names — a stage, a
    /// case or a hold — ground where nothing does.
    #[must_use]
    pub fn max_body(&self) -> usize {
        let stages = self.stages.iter().map(super::shape::Shape::max_body);
        let cases = self.load_cases.iter().flat_map(|c| {
            c.loads.iter().map(|l| l.at).chain(match c.duty {
                super::Duty::Intermittent { at, .. } => Some(at),
                super::Duty::Continuous { .. } => None,
            })
        });
        let holds = self.constraints.iter().map(|c| c.body);
        stages.chain(cases).chain(holds).max().unwrap_or(GROUND)
    }

    /// **The stages a body is listed on**, with its slot in each, in stage
    /// order: one stage for a body a stage has to itself, more for one that
    /// runs on.
    #[must_use]
    pub fn ends_of(&self, body: usize) -> Vec<(usize, Body)> {
        self.stages
            .iter()
            .enumerate()
            .filter_map(|(k, s)| {
                let slot = s.slot(body);
                (slot != GROUND).then_some((k, slot))
            })
            .collect()
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
        for s in &mut self.stages {
            s.bodies.retain(|b| keep(b.body));
            s.members.retain(|m| keep(m.body));
            s.couplings.retain(|c| c.iter().all(|&b| keep(b)));
            s.renumber_bodies(to);
        }
        self.constraints.retain(|c| keep(c.body));
        for c in &mut self.constraints {
            c.body = to(c.body);
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
                    || self
                        .stages
                        .iter()
                        .any(|s| s.bodies.iter().any(|x| x.body == b))
                    || self.constraints.iter().any(|c| c.body == b)
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
        if self.stages.is_empty() {
            return;
        }
        let on_a_stage = |b: usize| b == GROUND || self.stages.iter().any(|s| s.slot(b) != GROUND);
        self.constraints.retain(|c| on_a_stage(c.body));
        for case in &mut self.load_cases {
            case.loads.retain(|l| on_a_stage(l.at));
            if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                if !on_a_stage(*at) {
                    *at = GROUND;
                }
            }
        }
    }

    /// **The constraints in force**: each stage's conventions — its own holds
    /// — with the train's own laid over them.
    ///
    /// Laid over rather than replacing, so that a train stating one thing
    /// keeps the rest. Two things follow and each is what a designer means:
    ///
    /// - a constraint on a body **replaces** the convention on that body, so
    ///   `Free` on a conventionally held ring releases it;
    /// - a hold on any body of a stage **replaces the conventional holds on
    ///   that stage** — "hold the carrier" means instead of the ring, not as
    ///   well, and a set locked by holding two of its bodies is what a
    ///   designer asks for by writing both.
    ///
    /// A convention is the weakest statement there is, and gives way to any
    /// statement of the same kind about the same stage.
    #[must_use]
    pub fn constraints_in_force(&self) -> Vec<BodyConstraint> {
        let mut out: Vec<(usize, BodyConstraint)> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            for &slot in &stage.ports().held {
                out.push((k, BodyConstraint::held(self.port(k, slot))));
            }
        }
        for own in &self.constraints {
            if own.constraint == Constraint::Held {
                for (k, _) in self.ends_of(own.body) {
                    out.retain(|(s, c)| !(*s == k && c.constraint == Constraint::Held));
                }
            }
        }
        let mut out: Vec<BodyConstraint> = out.into_iter().map(|(_, c)| c).collect();
        for own in &self.constraints {
            out.retain(|c| c.body != own.body);
            out.push(*own);
        }
        out
    }

    /// How many bodies the stages have between them, ground counted — the
    /// nodes of the train's system. A body only a case or a hold names is
    /// no node: it would turn freely, and a train with stages has none
    /// ([`Self::drop_orphans`]), so a hold at one is a hold at nothing.
    fn stage_bodies(&self) -> usize {
        self.stages
            .iter()
            .map(super::shape::Shape::max_body)
            .max()
            .unwrap_or(GROUND)
            + 1
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
        self.system_counting(|_, _, z| z)
    }

    /// **The system with one gear a tooth larger** — gear `member` of stage
    /// `stage` — which is what a path's *one more tooth* asks.
    ///
    /// # Errors
    ///
    /// As [`Self::system`].
    pub fn system_raising(&self, stage: usize, member: usize) -> Result<System, MotionError> {
        self.system_counting(|k, i, z| if (k, i) == (stage, member) { z + 1 } else { z })
    }

    fn system_counting(
        &self,
        count: impl Fn(usize, usize, u32) -> u32,
    ) -> Result<System, MotionError> {
        if self.stages.is_empty() {
            return Err(MotionError::Empty);
        }
        let mut system = System::new(self.stage_bodies());
        for (k, stage) in self.stages.iter().enumerate() {
            let teeth: Vec<u32> = super::teeth_of(stage.gears())
                .into_iter()
                .enumerate()
                .map(|(i, z)| count(k, i, z))
                .collect();
            stage
                .wiring()
                .add_to(&mut system, &teeth, |slot| self.port(k, slot))
                .map_err(|e| MotionError::Wiring(k, e))?;
        }
        Ok(system)
    }

    /// **What the train asks of every body**, one condition per body: ground
    /// held, and each constraint in force at the body it names.
    ///
    /// # Errors
    ///
    /// A constraint naming a body the train does not have.
    pub fn conditions(&self, bodies: usize) -> Result<Vec<Condition>, MotionError> {
        let mut out = vec![Condition::Free; bodies];
        out[GROUND] = Condition::Ground;
        for c in self.constraints_in_force() {
            if c.body == GROUND {
                continue;
            }
            if c.body >= bodies {
                return Err(MotionError::NoSuchBody(c.body));
            }
            out[c.body] = c.constraint.condition();
        }
        Ok(out)
    }

    /// Whether a body is listed on a stage before or after `k`.
    fn shared_with(&self, body: usize, k: usize, earlier: bool) -> bool {
        self.ends_of(body)
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
        let conditions = self.conditions(self.stage_bodies())?;
        let mut out = Vec::with_capacity(self.stages.len());
        for (k, stage) in self.stages.iter().enumerate() {
            let w = stage.wiring();
            let ports = stage.ports();
            let mut local: Vec<Condition> = (0..w.slots.len())
                .map(|s| conditions[self.port(k, s)])
                .collect();
            local[GROUND] = Condition::Ground;
            let side = |earlier: bool| -> Option<Body> {
                ports
                    .ports
                    .iter()
                    .copied()
                    .find(|&slot| self.shared_with(self.port(k, slot), k, earlier))
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
                    let slot = self.slot(k, l.at)?;
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
                .constraints
                .iter()
                .filter_map(|c| self.slot(k, c.body))
                .collect();
            let first: Vec<Body> = (0..local.len()).filter(|s| !holds.contains(s)).collect();
            if let Err(Refusal::Conflicts(i)) = w
                .alone(&super::teeth_of(stage.gears()))
                .map_err(|e| MotionError::Wiring(k, e))?
                .motion_in(&local, &first)
            {
                return Err(MotionError::Conflicts(self.port(k, i)));
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
        let system = self.system()?;
        let mut conditions = self.conditions(system.bodies())?;
        // **The train has a ratio between exactly two open bodies**, driven
        // at the first: a chain's two ends, whatever stage each is on. With
        // any other number nothing is driven, the motion is a family, and
        // the train has no figure of its own — each stage still has, and
        // each case decides its own.
        let boundaries = self.boundaries()?;
        let ends = self.ends(&boundaries);
        if let Some((a, _)) = ends {
            conditions[a] = Condition::Drive(Ratio::ONE);
        }
        // **Conventions in first, the train's own last**, so that a conflict
        // is named at the statement the designer made rather than at the
        // convention it contradicts: holding a set's carrier beside its
        // held ring is reported at the carrier.
        let order: Vec<Body> = std::iter::once(GROUND)
            .chain(self.constraints_in_force().iter().map(|c| c.body))
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
            for (k, stage) in self.stages.iter().enumerate() {
                for slot in stage.ports().ports {
                    let b = self.port(k, slot);
                    let shared = self.ends_of(b).len() > 1;
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
                let (i, o) = (self.port(k, b.input), self.port(k, b.output));
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

    /// **The train's two ends**: the first stage's conventional input and
    /// the last stage's conventional output, where each is open and no
    /// other stage's. What the train's own ratio is read between, and what
    /// a preset puts its load and its reaction at; a convention for
    /// reporting and nothing more, since a case says what turns. `None`
    /// where either is held or shared, or the train is one stage with one
    /// open port.
    #[must_use]
    pub fn ends(&self, boundaries: &[StageBoundary]) -> Option<(usize, usize)> {
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
    /// **What this port is asked if the train says nothing about it** — the
    /// stage's convention *as the overlay leaves it*, with everything else
    /// the train states in force: a set's ring reads `free` here once its
    /// carrier is held, because holding the carrier releases it. What a
    /// panel's "convention" choice would come to, computed by the rule
    /// rather than guessed from the preset.
    pub by_convention: Constraint,
}

/// **A stage's ports and its conventional holds**, so a panel can offer
/// exactly the bodies a train may constrain or share — read from the
/// stage's own wiring rather than written into the front end a second time.
#[derive(Clone, Debug, PartialEq, Eq)]
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
        self.stages
            .iter()
            .enumerate()
            .map(|(k, stage)| {
                StagePorts {
                    members: stage.member_names(),
                    mesh_groups: stage.mesh_groups(),
                    family: stage.family(),
                    ports: stage
                        .ports()
                        .ports
                        .iter()
                        .map(|&slot| {
                            let body = self.port(k, slot);
                            // The train without its own word on this body,
                            // and what the overlay then asks of it.
                            let mut without = self.clone();
                            without.constraints.retain(|c| c.body != body);
                            PortSpec {
                                slot,
                                body,
                                by_convention: without
                                    .constraints_in_force()
                                    .iter()
                                    .find(|c| c.body == body)
                                    .map_or(Constraint::Free, |c| c.constraint),
                            }
                        })
                        .collect(),
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
            + usize::from(self.ends(&boundaries).is_some());
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
        let mut out: Vec<PortBody> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            let w = stage.wiring();
            let Some(b) = boundaries.get(k) else { break };
            for slot in stage.ports().ports {
                let body = self.port(k, slot);
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
    /// every stage's end of it, every case entry, every hold — and `b`'s
    /// number is given up. A reaction declared at either becomes a load
    /// with its torque derived — an inline take-off, the same physics —
    /// since a body two stages share cannot be a reaction; a free entry at
    /// either is dropped, there being nothing free about it now; and a case
    /// with an entry at each keeps the first. Joining a body to itself
    /// changes nothing.
    pub fn join(&mut self, a: usize, b: usize) {
        if a == b || a == GROUND || b == GROUND {
            return;
        }
        // A stage with both would have one body at two of its slots, which
        // is a mesh or a carrier turning against itself: not a body.
        let both = |s: &Shape| s.slot(a) != GROUND && s.slot(b) != GROUND;
        if self.stages.iter().any(both) {
            return;
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
    fn merge(&mut self, a: usize, b: usize) {
        self.renumber(|x| Some(if x == b { a } else { x }));
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
        self.constraints.dedup_by_key(|c| c.body);
        self.prune();
    }

    /// **A stage's end of a body split off** as a body of its own — the
    /// body it leaves keeps its number and whatever the train wrote on it,
    /// and the stage's members and carriers on it move to the new one,
    /// whose number is returned. Nothing happens where the body is the
    /// stage's alone, and the body's own number is returned.
    pub fn split(&mut self, stage: usize, body: usize) -> usize {
        if self.ends_of(body).len() < 2 {
            return body;
        }
        let fresh = self.max_body() + 1;
        if let Some(s) = self.stages.get_mut(stage) {
            s.renumber_bodies(|x| if x == body { fresh } else { x });
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

    /// **A body held to ground**, in so many words: held by the train, which
    /// replaces its stages' conventional holds ([`Self::constraints_in_force`]).
    /// Every case entry at it goes with it: a held body is fixed, and no
    /// case can say anything of it.
    pub fn hold(&mut self, body: usize) {
        if body == GROUND {
            return;
        }
        self.constraints.retain(|c| c.body != body);
        self.constraints.push(BodyConstraint::held(body));
        for case in &mut self.load_cases {
            case.loads.retain(|l| l.at != body);
        }
    }

    /// **A body released**: every statement the train made about it
    /// withdrawn, and a hold a stage's convention puts on it written off in
    /// so many words.
    pub fn release(&mut self, body: usize) {
        self.constraints.retain(|c| c.body != body);
        let by_convention = self
            .ends_of(body)
            .iter()
            .any(|&(k, slot)| self.stages[k].ports().held.contains(&slot));
        if by_convention {
            self.constraints.push(BodyConstraint::free(body));
        }
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

    /// **A fresh case of this kind between the train's two ends**: a
    /// torque at the first, driven at a speed, reacted at the second, the
    /// duty's sweep measured at the second — the case a panel's button adds,
    /// **switched off**, so a case added at its default figures moves no
    /// rating until the designer has written it and switched it on. A train
    /// with no two ends gets it at two bodies of its own, parked for the
    /// first stage to take up or the designer to move.
    #[must_use]
    pub fn fresh_case(&self, kind: super::CaseKind, torque: f64, speed: f64) -> super::LoadCase {
        let ends = self.boundaries().ok().and_then(|b| self.ends(&b));
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
        if k >= self.stages.len() {
            return;
        }
        self.stages.remove(k);
        self.drop_orphans();
        self.prune();
    }

    /// **A stage edited on its card** ([`super::StageEdit`]): the shape
    /// takes the edit, numbering any body it adds after every body the
    /// train has; a body the edit took off the stage that no other stage
    /// has leaves the train, with every case entry and hold at it, and the
    /// rest are numbered densely again.
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
        let shape = self
            .stages
            .get_mut(k)
            .ok_or(super::EditRefused::NoSuchIndex)?;
        shape.edit(edit, next)?;
        self.drop_bare();
        self.drop_orphans();
        self.prune();
        Ok(())
    }

    /// **Every body with nothing on it that nothing else names, given up.**
    ///
    /// A body a stage lists with no member and no axis to carry is a shaft
    /// in neutral, which is a state worth being able to reach — but only
    /// while something means it to be there. Another stage on it, a hold,
    /// a case's entry: any of those is a reason. None of them, and it is a
    /// number nothing would ever read again.
    fn drop_bare(&mut self) {
        let named = |train: &Self, body: usize, except: usize| {
            train
                .stages
                .iter()
                .enumerate()
                .any(|(k, s)| k != except && s.slot(body) != GROUND)
                || train.constraints.iter().any(|c| c.body == body)
                || train.load_cases.iter().any(|c| {
                    c.loads.iter().any(|l| l.at == body)
                        || matches!(c.duty, super::Duty::Intermittent { at, .. } if at == body)
                })
        };
        for k in 0..self.stages.len() {
            let bare: Vec<usize> = self.stages[k]
                .bodies
                .iter()
                .map(|b| b.body)
                .filter(|&b| {
                    self.stages[k].members_on_body(b).is_empty()
                        && !self.stages[k].carries_an_axis(b)
                        && !self.stages[k].couplings.iter().any(|c| c.contains(&b))
                        && !named(self, b, k)
                })
                .collect();
            self.stages[k].bodies.retain(|b| !bare.contains(&b.body));
        }
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
        let k = self.stages.len();
        let next = self.max_body() + 1;
        // The stage's own numbering, moved above everything the train has:
        // its slot `i` becomes body `next + i - 1`.
        let slots: Vec<usize> = stage.bodies.iter().map(|b| b.body).collect();
        stage.renumber_bodies(|b| slots.iter().position(|&x| x == b).map_or(b, |i| next + i));
        let input = stage.ports().input();
        let output = stage.ports().output();
        let onward = self.boundaries().ok().and_then(|b| {
            let open = self.open_ports(&b);
            let last = k.checked_sub(1)?;
            let conventional = self.port(last, self.stages[last].ports().output());
            let mine: Vec<usize> = open
                .iter()
                .map(|p| p.body)
                .filter(|&body| self.ends_of(body).iter().any(|&(s, _)| s == last))
                .collect();
            if mine.contains(&conventional) {
                Some(conventional)
            } else if let [only] = mine.as_slice() {
                Some(*only)
            } else {
                None
            }
        });
        self.stages.push(stage);
        if k == 0 {
            // Taken up as they were: a reaction parked at the output is a
            // reaction at the stage's, not a body two stages share. Each
            // merge closes the numbers up, so the second is read afresh.
            if let Some(&a) = self.parked().first() {
                self.merge(self.port(0, input), a);
            }
            if let Some(&b) = self.parked().first() {
                self.merge(self.port(0, output), b);
            }
            return;
        }
        let (input, output) = (self.port(k, input), self.port(k, output));
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
