//! **What a train asks of its shafts, and how its stages are joined** — the
//! boundary layer, kept apart from the topology ([`super::wiring`]) and the
//! geometry (each kind's own file).
//!
//! Three layers with three edit frequencies, and this is the one that changes
//! most: which shaft is held and which is driven is what a designer turns to
//! see a different machine, and nothing about the stages moves when they do.
//! It used to live on the stage — a planetary set carried an `Arrangement`
//! naming its input and its held shaft — which is why a set's ratio was a
//! single number, why two epicyclic stages could not share a shaft, and why a
//! train could not have a third port: the train had no vocabulary for a shaft
//! at all.
//!
//! # A shaft is named by where it is, not what it is for
//!
//! [`ShaftRef`] is a stage and a local index into that stage's wiring, or
//! ground. "Input" and "output" are not names here: they are *readings* of a
//! [`Constraint`] — a driven shaft is where power comes in, and which shaft
//! power leaves by is a result. That is the handoff's point about mobility
//! above one taken seriously: two drives and one load, and one drive with two
//! loads, are the same kinematic object, and asking the designer to declare
//! which is which as structure would be asking them to tell the model what it
//! is about to work out.
//!
//! # The chain is the default, not the only shape
//!
//! A train with no couplings of its own is a chain — each kind's conventional
//! *output port* coupled to the next kind's *input port* — and a train with no
//! constraints of its own holds what each kind holds by convention and drives
//! the first stage's input. That is what every file written before these lists
//! existed meant, so their absence is unambiguous and they default rather than
//! refuse. Written out, they can say anything a graph of shafts can say.

use super::wiring::Wiring;
use crate::kinematics::{Condition, Shaft, GROUND};
use crate::ratio::Ratio;

/// Where a shaft is, from the train's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum ShaftRef {
    /// The one held frame every stage shares.
    Ground,
    /// A stage's own shaft, by the index its [`Wiring::shafts`] gives it.
    /// Zero-based, as the stages are; the front end numbers from 1.
    Of { stage: usize, shaft: usize },
}

/// What is asked of one shaft. Three things, and no fourth.
///
/// The train-level reading of [`crate::kinematics::Condition`], without the
/// drive's speed: a train's motion is solved at one turn of whatever is driven,
/// and a load case's speed scales it. Where more than one shaft is driven they
/// are driven *together* at one turn each — which is a statement about a
/// differential's two inputs turning alike, and the family the solve returns
/// says what the other members do per turn of each.
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
    /// Turned by something outside the train.
    Driven,
    /// Neither — it does what the rest decides, and carries no torque.
    Free,
}

impl Constraint {
    /// The condition the solver takes.
    #[must_use]
    pub const fn condition(self) -> Condition {
        match self {
            Self::Held => Condition::Ground,
            Self::Driven => Condition::Drive(Ratio::ONE),
            Self::Free => Condition::Free,
        }
    }
}

/// One shaft, and what is asked of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct ShaftConstraint {
    pub at: ShaftRef,
    pub constraint: Constraint,
}

impl ShaftConstraint {
    /// A stage's shaft, held to ground.
    #[must_use]
    pub const fn held(stage: usize, shaft: Shaft) -> Self {
        Self {
            at: ShaftRef::Of { stage, shaft },
            constraint: Constraint::Held,
        }
    }

    /// A stage's shaft, driven.
    #[must_use]
    pub const fn driven(stage: usize, shaft: Shaft) -> Self {
        Self {
            at: ShaftRef::Of { stage, shaft },
            constraint: Constraint::Driven,
        }
    }
}

/// Two shafts that turn as one: a coaxial output coupling, a locked clutch,
/// the join between two stages of a chain. One mechanism, so one type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct Coupling {
    pub a: ShaftRef,
    pub b: ShaftRef,
}

/// **A kind's conventional ports and what it holds by default** — what a chain
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
    /// **Every shaft a train may couple to, in the order a chain prefers
    /// them.** A pair's two members; a set's three central shafts, sun first;
    /// a hula stage's crank, its output gear, its grounded gear.
    ///
    /// A chain takes the first that is not held as its input and the next as
    /// its output — so a set whose *carrier* a train holds is coupled onward by
    /// its ring, and a designer changing what is held does not also have to
    /// rewire the chain. A shaft in this list is a port; one not in it — a
    /// planet's, a wobble body's — is nobody's to couple.
    pub ports: Vec<Shaft>,
    /// The shafts held to ground by convention.
    pub held: Vec<Shaft>,
}

impl Ports {
    /// **Where a chain enters and leaves this stage**, given what is held —
    /// and, where the train drives one of its ports directly, that one.
    ///
    /// The conventional reading is the first port not held and the next; a
    /// driven port is the input whatever its place in the list, since a drive
    /// is a statement and the order is a preference.
    #[must_use]
    pub fn ends(&self, held: &[Shaft], driven: Option<Shaft>) -> (Shaft, Shaft) {
        let free: Vec<Shaft> = self
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
    pub fn input(&self) -> Shaft {
        self.ends(&self.held, None).0
    }

    /// The conventional output.
    #[must_use]
    pub fn output(&self) -> Shaft {
        self.ends(&self.held, None).1
    }
}

/// **What one stage is asked**, as its solver needs it: a condition per local
/// shaft, and which shaft power comes in and leaves by.
///
/// Assembled by the train from its constraints and couplings — the same way a
/// [`super::StageLoads`] is assembled from its load cases — or from a kind's
/// own [`Ports`] where a stage is solved alone.
#[derive(Clone, Debug, PartialEq)]
pub struct StageBoundary {
    /// One per local shaft, ground first.
    pub conditions: Vec<Condition>,
    /// The local shaft a load is referred to.
    pub input: Shaft,
    /// The local shaft a load leaves by.
    pub output: Shaft,
}

impl StageBoundary {
    /// **The boundary a stage solves under**: the one the train handed it with
    /// its loads, or its kind's convention where it is being asked alone. One
    /// helper because three kinds would otherwise write the same `unwrap_or`.
    #[must_use]
    pub fn of(loads: &super::StageLoads, wiring: &Wiring, ports: &Ports) -> Self {
        loads
            .boundary
            .clone()
            .unwrap_or_else(|| Self::conventional(wiring, ports))
    }

    /// A stage on its own, under its kind's conventions: ground held, the
    /// conventional shafts held, the conventional input driven at one turn.
    #[must_use]
    pub fn conventional(wiring: &Wiring, ports: &Ports) -> Self {
        let (input, output) = ports.ends(&ports.held, None);
        Self::holding(wiring.shafts.len(), &ports.held, input, output)
    }

    /// `shafts` local shafts with `held` fixed to ground and `input` driven
    /// at one turn, the rest free.
    #[must_use]
    pub fn holding(shafts: usize, held: &[Shaft], input: Shaft, output: Shaft) -> Self {
        let mut conditions = vec![Condition::Free; shafts];
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

    /// The local shafts held to ground, ascending — what an epicyclic kind
    /// reads its arrangement from.
    #[must_use]
    pub fn held(&self) -> Vec<Shaft> {
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

use super::wiring::{Offsets, ShaftLabel};
use super::Train;
use crate::kinematics::{Mobility, Refusal, Solution, System};

/// Why a train's motion could not be worked out.
///
/// **None of these is a geometric refusal**, and that is the point: a ratio
/// needs tooth counts and a topology, so a stage whose centre distances cannot
/// be made to agree still has one. `TrainError` is the other question.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MotionError {
    /// A train with no stages has no shaft line.
    Empty,
    /// A stage's wiring does not describe meshes.
    Wiring(usize, super::WiringError),
    /// A constraint or coupling names a stage or a shaft the train does not
    /// have.
    NoSuchShaft(ShaftRef),
    /// The constraints and the structure cannot both hold, at this shaft.
    Refused(Refusal),
}

/// One shaft of an assembled train.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaftMotion {
    /// Which stage introduced it, and `None` for ground, which every stage
    /// shares.
    pub stage: Option<usize>,
    pub label: ShaftLabel,
    /// Turns per turn of the driven port — exactly.
    pub speed: Ratio,
}

/// **What a whole train does, from tooth counts and topology alone.**
///
/// No module, no shift, no material and no load. That independence is the
/// finding this type exists for: `solve_train` refused a train outright when
/// any one stage would not close geometrically, and took every other stage's
/// ratio down with it — though Willis needs none of what failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainMotion {
    pub shafts: Vec<ShaftMotion>,
    /// One per stage: its input over its output, or `None` where the output
    /// does not turn — two meshes stepping by the same amount and cancelling,
    /// which is a refusal rather than a very large number.
    pub ratios: Vec<Option<Ratio>>,
    /// The first stage's input to the last stage's output.
    pub total: Option<Ratio>,
    /// How many conditions the train needs, and which shafts nothing touches.
    pub mobility: Mobility,
    /// Where each stage's shafts begin.
    pub at: Vec<Offsets>,
    /// The solution in full, for a caller that wants a shaft this does not
    /// name — a member's, through its stage's [`Wiring::mounts`].
    pub solution: Solution,
}

impl TrainMotion {
    /// The global index of one of a stage's own shafts.
    #[must_use]
    pub fn shaft_of(&self, stage: usize, local: Shaft) -> Shaft {
        self.at[stage].of(local)
    }
}

impl Train {
    /// **The couplings in force**: the train's own, or — where it has none —
    /// the chain, each kind's conventional output to the next kind's input.
    #[must_use]
    pub fn couplings_in_force(&self) -> Vec<Coupling> {
        if !self.couplings.is_empty() {
            return self.couplings.clone();
        }
        // The chain, read against what the train holds and drives: a stage
        // whose conventional output is held is coupled onward by the port its
        // constraints leave free.
        let ends: Vec<(Shaft, Shaft)> =
            (0..self.stages.len()).map(|k| self.chain_ends(k)).collect();
        (1..ends.len())
            .map(|k| Coupling {
                a: ShaftRef::Of {
                    stage: k - 1,
                    shaft: ends[k - 1].1,
                },
                b: ShaftRef::Of {
                    stage: k,
                    shaft: ends[k].0,
                },
            })
            .collect()
    }

    /// Where the chain enters and leaves stage `k`, under the constraints in
    /// force on that stage's own shafts.
    fn chain_ends(&self, k: usize) -> (Shaft, Shaft) {
        let ports = self.stages[k].ports();
        let mine = |c: &ShaftConstraint| match c.at {
            ShaftRef::Of { stage, shaft } if stage == k => Some(shaft),
            _ => None,
        };
        let constraints = self.constraints_in_force();
        let held: Vec<Shaft> = constraints
            .iter()
            .filter(|c| c.constraint == Constraint::Held)
            .filter_map(mine)
            .collect();
        let driven = constraints
            .iter()
            .filter(|c| c.constraint == Constraint::Driven)
            .find_map(mine);
        ports.ends(&held, driven)
    }

    /// **The constraints in force**: each kind's conventions — its own holds,
    /// and the first stage's input driven — with the train's own laid over
    /// them, shaft by shaft.
    ///
    /// Laid over rather than replacing, so that a train stating one thing
    /// keeps the rest: holding a set's carrier does not silently release the
    /// drive on the stage before it. Two things follow and both are what a
    /// designer means:
    ///
    /// - a constraint on a shaft **replaces** the convention on that shaft, so
    ///   `Free` on a conventionally held ring releases it;
    /// - a drive on any shaft of a stage **replaces the conventional drive on
    ///   that stage** — "driven by the carrier" means instead of the sun, not
    ///   as well, and two drives on one set is a family a designer asks for by
    ///   writing both.
    #[must_use]
    pub fn constraints_in_force(&self) -> Vec<ShaftConstraint> {
        let mut out: Vec<ShaftConstraint> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            let ports = stage.ports();
            for &shaft in &ports.held {
                out.push(ShaftConstraint::held(k, shaft));
            }
            if k == 0 {
                out.push(ShaftConstraint::driven(0, ports.input()));
            }
        }
        for own in &self.constraints {
            if own.constraint == Constraint::Driven {
                if let ShaftRef::Of { stage, .. } = own.at {
                    out.retain(|c| {
                        !(c.constraint == Constraint::Driven
                            && matches!(c.at, ShaftRef::Of { stage: s, .. } if s == stage))
                    });
                }
            }
            out.retain(|c| c.at != own.at);
            out.push(*own);
        }
        out
    }

    /// Where each stage's shafts begin in the assembled system, and how many
    /// shafts there are.
    fn layout(&self) -> (Vec<Offsets>, usize) {
        let mut at = Vec::with_capacity(self.stages.len());
        let mut next = 1;
        for stage in &self.stages {
            at.push(Offsets { first: next });
            next += stage.wiring().shafts.len() - 1;
        }
        (at, next)
    }

    /// A shaft reference resolved to its index in the assembled system.
    fn resolve(&self, at: &[Offsets], r: ShaftRef) -> Result<Shaft, MotionError> {
        match r {
            ShaftRef::Ground => Ok(GROUND),
            ShaftRef::Of { stage, shaft } => {
                let w = self
                    .stages
                    .get(stage)
                    .ok_or(MotionError::NoSuchShaft(r))?
                    .wiring();
                if shaft == 0 || shaft >= w.shafts.len() {
                    return Err(MotionError::NoSuchShaft(r));
                }
                Ok(at[stage].of(shaft))
            }
        }
    }

    /// **The whole train as one system**, and where each stage's shafts sit
    /// in it.
    ///
    /// One ground, shared; each stage's other shafts appended in order; and a
    /// rigid coupling wherever the train says two shafts turn as one — which,
    /// for a train that says nothing, is the chain.
    ///
    /// # Errors
    ///
    /// [`MotionError::Empty`], a stage whose wiring does not describe meshes,
    /// or a coupling naming a shaft the train does not have.
    pub fn system(&self) -> Result<(System, Vec<Offsets>), MotionError> {
        if self.stages.is_empty() {
            return Err(MotionError::Empty);
        }
        let (at, shafts) = self.layout();
        let mut system = System::new(shafts);
        for (k, stage) in self.stages.iter().enumerate() {
            let teeth = super::teeth_of(stage.members());
            stage
                .wiring()
                .add_to(&mut system, &teeth, &at[k])
                .map_err(|e| MotionError::Wiring(k, e))?;
        }
        for c in self.couplings_in_force() {
            let (a, b) = (self.resolve(&at, c.a)?, self.resolve(&at, c.b)?);
            system.couple(a, b).ok_or(MotionError::NoSuchShaft(c.a))?;
        }
        Ok((system, at))
    }

    /// **What the train asks of every shaft**, one condition per shaft of the
    /// assembled system: ground held, and each constraint in force at the
    /// shaft it names.
    ///
    /// # Errors
    ///
    /// A constraint naming a shaft the train does not have.
    pub fn conditions(&self, at: &[Offsets], shafts: usize) -> Result<Vec<Condition>, MotionError> {
        let mut out = vec![Condition::Free; shafts];
        out[GROUND] = Condition::Ground;
        for c in self.constraints_in_force() {
            let i = self.resolve(at, c.at)?;
            if i != GROUND {
                out[i] = c.constraint.condition();
            }
        }
        Ok(out)
    }

    /// **What each stage is asked**, as its own solver needs it: its local
    /// shafts' conditions, and the shaft power comes in and leaves by.
    ///
    /// A stage in a train is driven by what it is coupled to as often as by a
    /// motor, so a shaft coupled to an *earlier* stage is its input and is
    /// driven at one turn for the stage's own solve; a shaft coupled to a
    /// later one is its output. Where neither says, the kind's conventions
    /// do. This is the chain read off the graph rather than assumed of it,
    /// and it is the one place "earlier" means anything — a general graph has
    /// no order, and the train-level family ([`Train::motion`]) needs none.
    ///
    /// # Errors
    ///
    /// As [`Self::system`].
    pub fn boundaries(&self) -> Result<Vec<StageBoundary>, MotionError> {
        let (at, shafts) = self.layout();
        let conditions = self.conditions(&at, shafts)?;
        let couplings = self.couplings_in_force();
        let mut out = Vec::with_capacity(self.stages.len());
        for (k, stage) in self.stages.iter().enumerate() {
            let w = stage.wiring();
            let ports = stage.ports();
            let mut local: Vec<Condition> = (0..w.shafts.len())
                .map(|s| conditions[at[k].of(s)])
                .collect();
            local[GROUND] = Condition::Ground;
            // The shaft coupled to an earlier stage, if any, is where the
            // load comes in; the one coupled to a later stage is where it
            // leaves.
            let side = |mine: bool, other_earlier: bool| -> Option<Shaft> {
                couplings.iter().find_map(|c| {
                    let (here, there) = if mine { (c.b, c.a) } else { (c.a, c.b) };
                    match (here, there) {
                        (ShaftRef::Of { stage: s, shaft }, ShaftRef::Of { stage: o, .. })
                            if s == k && ((o < k) == other_earlier) =>
                        {
                            Some(shaft)
                        }
                        _ => None,
                    }
                })
            };
            let driven = local
                .iter()
                .enumerate()
                .skip(1)
                .find(|(_, c)| matches!(c, Condition::Drive(_)))
                .map(|(i, _)| i);
            let (conventional_in, conventional_out) = ports.ends(
                &local
                    .iter()
                    .enumerate()
                    .skip(1)
                    .filter(|(_, c)| **c == Condition::Ground)
                    .map(|(i, _)| i)
                    .collect::<Vec<_>>(),
                driven,
            );
            let input = driven
                .or_else(|| side(true, true))
                .unwrap_or(conventional_in);
            let output = side(false, false).unwrap_or(conventional_out);
            local[input] = Condition::Drive(Ratio::ONE);
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
        let (system, at) = self.system()?;
        let conditions = self.conditions(&at, system.shafts())?;
        let solution = system.motion(&conditions).map_err(MotionError::Refused)?;
        let mobility = system
            .mobility()
            .ok_or(MotionError::Refused(Refusal::Overflow))?;
        let boundaries = self.boundaries()?;

        let mut shafts = vec![ShaftMotion {
            stage: None,
            label: ShaftLabel::Ground,
            speed: solution.values[GROUND],
        }];
        for (k, stage) in self.stages.iter().enumerate() {
            for (local, label) in stage.wiring().shafts.iter().enumerate().skip(1) {
                shafts.push(ShaftMotion {
                    stage: Some(k),
                    label: *label,
                    speed: solution.values[at[k].of(local)],
                });
            }
        }
        let ratios: Vec<Option<Ratio>> = boundaries
            .iter()
            .enumerate()
            .map(|(k, b)| {
                let (i, o) = (at[k].of(b.input), at[k].of(b.output));
                solution.values[i].checked_div(solution.values[o])
            })
            .collect();
        let total = match (boundaries.first(), boundaries.last()) {
            (Some(f), Some(l)) => {
                let i = at[0].of(f.input);
                let o = at[boundaries.len() - 1].of(l.output);
                solution.values[i].checked_div(solution.values[o])
            }
            _ => None,
        };
        Ok(TrainMotion {
            shafts,
            ratios,
            total,
            mobility,
            at,
            solution,
        })
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
    /// The local shaft index a [`ShaftRef`] names it by.
    pub shaft: Shaft,
    pub label: ShaftLabel,
}

/// **A stage's ports and its conventional holds**, so a panel can offer
/// exactly the shafts a train may constrain or couple — read from the kind's
/// own wiring rather than written into the front end a second time.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct StagePorts {
    pub ports: Vec<PortSpec>,
    /// What the kind holds when the train says nothing.
    pub held_by_convention: Vec<Shaft>,
}

/// One shaft of the train's motion, for the front end.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShaftReport {
    pub at: ShaftRef,
    pub label: ShaftLabel,
    /// Turns per turn of what is driven.
    pub speed: Exact,
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
    /// How many independent constraints the train needs.
    pub mobility: usize,
    /// How many it has been given, so a panel can say *one short* or *one
    /// too many* rather than leaving a designer to count.
    pub constrained: usize,
    /// Shafts nothing constrains — named, not counted.
    pub untouched: Vec<ShaftRef>,
    pub shafts: Vec<ShaftReport>,
    /// One per stage, input over output. `None` where the output does not
    /// turn.
    pub ratios: Vec<Option<Exact>>,
    /// The first stage's input to the last stage's output.
    pub total: Option<Exact>,
}

impl Train {
    /// Every stage's ports, for a panel to offer.
    #[must_use]
    pub fn topology(&self) -> Vec<StagePorts> {
        self.stages
            .iter()
            .map(|stage| {
                let w = stage.wiring();
                let p = stage.ports();
                StagePorts {
                    ports: p
                        .ports
                        .iter()
                        .map(|&shaft| PortSpec {
                            shaft,
                            label: w.shafts[shaft],
                        })
                        .collect(),
                    held_by_convention: p.held,
                }
            })
            .collect()
    }

    /// The train's motion in the shape the boundary sends, or `None` where
    /// there is none to send.
    #[must_use]
    pub fn motion_report(&self) -> Option<MotionReport> {
        let m = self.motion().ok()?;
        let (at, _) = self.layout();
        let locate = |i: Shaft| -> ShaftRef {
            if i == GROUND {
                return ShaftRef::Ground;
            }
            let stage = at.iter().rposition(|o| o.first <= i).unwrap_or(0);
            ShaftRef::Of {
                stage,
                shaft: i - at[stage].first + 1,
            }
        };
        let constrained = self
            .constraints_in_force()
            .iter()
            .filter(|c| c.constraint != Constraint::Free)
            .count()
            + 1; // ground
        Some(MotionReport {
            mobility: m.mobility.degrees,
            constrained,
            untouched: m.mobility.untouched.iter().map(|&i| locate(i)).collect(),
            shafts: m
                .shafts
                .iter()
                .enumerate()
                .map(|(i, s)| ShaftReport {
                    at: locate(i),
                    label: s.label,
                    speed: s.speed.into(),
                })
                .collect(),
            ratios: m.ratios.iter().map(|r| r.map(Exact::from)).collect(),
            total: m.total.map(Exact::from),
        })
    }
}

// -------------------------------------------------- arranging one stage ---

impl Train {
    /// **This train with one stage told what drives it and what it holds** —
    /// the one gesture a panel offers on a set's card, done by the rules the
    /// core already keeps rather than by the panel.
    ///
    /// Two things that used to be one field, kept apart because they are
    /// different layers: **what is held** is a constraint; **where the load
    /// comes in** is a constraint only on a stage nothing drives from
    /// upstream — the first of a chain — and a *coupling* everywhere else,
    /// since a stage in the middle of a chain is turned by the stage before
    /// it, and writing a drive on its sun as well would ask the sun to turn at
    /// one speed while the coupling turns it at another. That contradiction
    /// is exactly what the solver refuses, and it is how this came to exist.
    ///
    /// Every port of the stage is stated — held, driven, or free — because the
    /// train's constraints lay over the kind's conventions shaft by shaft. The
    /// chain is materialised where it was implicit, so the coupling into this
    /// stage can be moved to `driven` and the one out of it to the port left
    /// free.
    #[must_use]
    pub fn arranged(&self, stage: usize, driven: Shaft, held: Shaft) -> Self {
        let mut out = self.clone();
        let Some(kind) = self.stages.get(stage) else {
            return out;
        };
        let ports = kind.ports();
        let couplings = self.couplings_in_force();
        // Whether one of this stage's shafts is coupled to an earlier stage —
        // in which case that is what drives it, and "driven by" names the port
        // the coupling enters rather than a motor.
        let at_stage = |r: ShaftRef| matches!(r, ShaftRef::Of { stage: s, .. } if s == stage);
        let earlier = |r: ShaftRef| match r {
            ShaftRef::Ground => false,
            ShaftRef::Of { stage: o, .. } => o < stage,
        };
        let entered_from_upstream = couplings
            .iter()
            .any(|c| (at_stage(c.b) && earlier(c.a)) || (at_stage(c.a) && earlier(c.b)));

        // --- what is held, and what is driven where a drive is what is meant.
        out.constraints
            .retain(|c| !matches!(c.at, ShaftRef::Of { stage: s, .. } if s == stage));
        for &p in &ports.ports {
            let constraint = if p == held {
                Constraint::Held
            } else if p == driven && !entered_from_upstream {
                Constraint::Driven
            } else {
                Constraint::Free
            };
            out.constraints.push(ShaftConstraint {
                at: ShaftRef::Of { stage, shaft: p },
                constraint,
            });
        }

        // --- where the chain enters and leaves, where either is a coupling.
        // The chain is materialised so there is a coupling to move; an end at
        // this stage that faces an earlier stage moves to `driven`, and one
        // that faces a later stage moves to the port left over.
        if couplings.iter().any(|c| at_stage(c.a) || at_stage(c.b)) {
            let leaves_by = ports
                .ports
                .iter()
                .copied()
                .find(|&p| p != driven && p != held)
                .unwrap_or(driven);
            let moved = |here: ShaftRef, there: ShaftRef| -> ShaftRef {
                if !at_stage(here) {
                    return here;
                }
                ShaftRef::Of {
                    stage,
                    shaft: if earlier(there) { driven } else { leaves_by },
                }
            };
            out.couplings = couplings
                .into_iter()
                .map(|c| Coupling {
                    a: moved(c.a, c.b),
                    b: moved(c.b, c.a),
                })
                .collect();
        }
        out
    }
}
