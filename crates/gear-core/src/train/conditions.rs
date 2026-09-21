//! **What a train asks of its shafts, and how its stages are joined** — the
//! boundary layer, kept apart from the topology ([`super::wiring`]) and the
//! geometry ([`super::shape`]).
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
//! A train with no couplings of its own is a chain — each stage's conventional
//! *output port* coupled to the next stage's *input port* — and a train with no
//! constraints of its own holds what each stage holds by convention and drives
//! the first stage's input. That is what every file written before these lists
//! existed meant, so their absence is unambiguous and they default rather than
//! refuse. Written out, they can say anything a graph of shafts can say.

use super::wiring::Wiring;
use crate::kinematics::{Condition, Shaft, GROUND};
use crate::ratio::Ratio;

/// Where a shaft is, from the train's point of view.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    /// Not held — it does what the rest decides. What it is coupled to is
    /// the train's [`Coupling`]s; whether it carries a torque, or drives,
    /// is a load case's question ([`super::LoadRole`]): the output of a set
    /// is free in exactly this sense and carries the whole load. (There was
    /// a third word, *driven*, from when the train was a chain with a head;
    /// what drives is a load on an open port now, and nothing more.)
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

    /// A stage's shaft, released from a hold its stage's convention put on
    /// it.
    #[must_use]
    pub const fn free(stage: usize, shaft: Shaft) -> Self {
        Self {
            at: ShaftRef::Of { stage, shaft },
            constraint: Constraint::Free,
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
/// [`super::StageLoads`] is assembled from its load cases — or from the
/// stage's own [`Ports`] where it is solved alone.
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
    /// its loads, or its own convention where it is being asked alone. One
    /// helper because three stage types once wrote the same `unwrap_or`.
    #[must_use]
    pub fn of(loads: &super::StageLoads, wiring: &Wiring, ports: &Ports) -> Self {
        loads
            .boundary
            .clone()
            .unwrap_or_else(|| Self::conventional(wiring, ports))
    }

    /// A stage on its own, under its own conventions: ground held, the
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
use super::{Stage, Train};
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
    /// **The constraints and the structure cannot both hold, at this shaft**
    /// — a condition asked of it contradicts what the meshes and the earlier
    /// conditions already decided. Named, because "over-determined" is not
    /// something a designer can act on and "the sun cannot turn while the
    /// carrier and the ring are both held" is.
    Conflicts(ShaftRef),
    /// An exact answer too large to represent: the product of tooth counts
    /// along the shaft line has outgrown `i128`, and a ratio is refused
    /// rather than wrapped ([`crate::ratio`]).
    Overflow,
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
    /// **`x` at one shaft, read at another**: a speed, a sweep or a
    /// revolution count stated at `per`, as `of` sees it through the ratios —
    /// and zero where `per` does not turn, which is a shaft nothing drives.
    ///
    /// **Divided exactly, and the float multiplied in last.** Both speeds
    /// are quotients of tooth counts; the quotient is taken exactly and
    /// [`Ratio::scale`] multiplies before it divides, so the answer rounds
    /// once. `x * (a/b).to_f64()` rounds twice, and the second rounding put
    /// a recorded speed one ULP from the correctly rounded value — measured
    /// here, on `30000 · 17/43`, when this was first written that way.
    #[must_use]
    pub fn read(&self, x: f64, of: ShaftRef, per: ShaftRef) -> f64 {
        self.solution.values[self.global(of)]
            .checked_div(self.solution.values[self.global(per)])
            .map_or(0.0, |r| r.scale(x))
    }

    /// A shaft reference's index in the assembled system.
    #[must_use]
    pub fn global(&self, r: ShaftRef) -> Shaft {
        match r {
            ShaftRef::Ground => GROUND,
            ShaftRef::Of { stage, shaft } => self.shaft_of(stage, shaft),
        }
    }

    /// The global index of one of a stage's own shafts.
    #[must_use]
    pub fn shaft_of(&self, stage: usize, local: Shaft) -> Shaft {
        self.at[stage].of(local)
    }
}

/// **Where a case's entries wait while the train has no stages**: the input
/// side is ground — the one shaft every train has — and the output side is
/// stage 0's own ground shaft, a reference no stage's port can ever be,
/// which is what makes it a place rather than a shaft. Both are refused as
/// ports by name should a file write one on a train with stages.
pub const PARKED_IN: ShaftRef = ShaftRef::Ground;
/// See [`PARKED_IN`].
pub const PARKED_OUT: ShaftRef = ShaftRef::Of { stage: 0, shaft: 0 };

impl Train {
    /// **The chain's couplings, written once**: each stage's conventional
    /// output to the next stage's conventional input. What a train's
    /// couplings *were* by a rule at every solve — a chain assumed of the
    /// stages whenever a file listed none — is a constructor now, so a
    /// train carries every coupling it has and the graph is what it says.
    #[must_use]
    pub fn chain(stages: &[Stage]) -> Vec<Coupling> {
        (1..stages.len())
            .map(|k| Coupling {
                a: ShaftRef::Of {
                    stage: k - 1,
                    shaft: stages[k - 1].ports().output(),
                },
                b: ShaftRef::Of {
                    stage: k,
                    shaft: stages[k].ports().input(),
                },
            })
            .collect()
    }

    /// A train of these stages coupled as a chain, with these cases.
    #[must_use]
    pub fn chained(stages: Vec<Stage>, load_cases: Vec<super::LoadCase>) -> Self {
        Self {
            couplings: Self::chain(&stages),
            load_cases,
            reversed_bending: false,
            stages,
            constraints: Vec::new(),
        }
    }

    /// **The constraints in force**: each stage's conventions — its own holds
    /// — with the train's own laid over them.
    ///
    /// Laid over rather than replacing, so that a train stating one thing
    /// keeps the rest. Two things follow and each is what a designer means:
    ///
    /// - a constraint on a shaft **replaces** the convention on that shaft, so
    ///   `Free` on a conventionally held ring releases it;
    /// - a hold on any shaft of a stage **replaces the conventional holds on
    ///   that stage** — "hold the carrier" means instead of the ring, not as
    ///   well, and a set locked by holding two of its shafts is what a
    ///   designer asks for by writing both.
    ///
    /// A convention is the weakest statement there is, and gives way to any
    /// statement of the same kind about the same stage. (The second rule
    /// arrived after the first delivery: a hold used to replace the hold on
    /// its own shaft only, so holding the carrier needed the ring written
    /// free as well, and a panel offering one select per shaft had to write
    /// two.)
    #[must_use]
    pub fn constraints_in_force(&self) -> Vec<ShaftConstraint> {
        let mut out: Vec<ShaftConstraint> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            for &shaft in &stage.ports().held {
                out.push(ShaftConstraint::held(k, shaft));
            }
        }
        for own in &self.constraints {
            if let (ShaftRef::Of { stage, .. }, Constraint::Held) = (own.at, own.constraint) {
                out.retain(|c| {
                    !(c.constraint == Constraint::Held
                        && matches!(c.at, ShaftRef::Of { stage: s, .. } if s == stage))
                });
            }
        }
        for own in &self.constraints {
            out.retain(|c| c.at != own.at);
            out.push(*own);
        }
        out
    }

    /// Where each stage's shafts begin in the assembled system, and how many
    /// shafts there are.
    pub(crate) fn layout(&self) -> (Vec<Offsets>, usize) {
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
        for c in self.couplings.clone() {
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
            if i == GROUND {
                continue;
            }
            out[i] = c.constraint.condition();
        }
        Ok(out)
    }

    /// **What each stage is asked**, as its own solver needs it: its local
    /// shafts' conditions, and the shaft power comes in and leaves by.
    ///
    /// A stage in a train is driven by what it is coupled to as often as by a
    /// motor, so a shaft coupled to an *earlier* stage is its input and is
    /// driven at one turn for the stage's own solve; a shaft coupled to a
    /// later one is its output. Where neither says, the stage's conventions
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
        let couplings = &self.couplings;
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
            let held: Vec<Shaft> = local
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, c)| **c == Condition::Ground)
                .map(|(i, _)| i)
                .collect();
            // A coupling from an earlier stage says where power enters;
            // failing that, the port the first case's first load is at,
            // where that is one of this stage's — a set alone loaded at its
            // carrier reads carrier in; failing that the stage's convention
            // — its first port neither held nor coupled onward, so a set at
            // the head of a chain coupled onward by its sun is entered at
            // its carrier. **A stage's input and output are its own
            // reporting convention** — which way its ratio, its efficiency
            // both ways and its play are read — and decide nothing about a
            // load case, whose loads say what turns; reading the first load
            // here moves no case, since a case names shafts and not ends.
            let onward = side(false, false);
            let open = |p: &Shaft| !held.contains(p) && Some(*p) != onward;
            let first_load = self
                .load_cases
                .iter()
                .filter(|c| c.enabled)
                .flat_map(|c| c.loads.iter())
                .find(|l| l.is_load())
                .and_then(|l| match l.at {
                    ShaftRef::Of { stage, shaft } if stage == k && ports.ports.contains(&shaft) => {
                        Some(shaft)
                    }
                    _ => None,
                })
                .filter(open);
            let input = side(true, true).or(first_load).unwrap_or_else(|| {
                ports
                    .ports
                    .iter()
                    .copied()
                    .find(open)
                    .unwrap_or_else(|| ports.ends(&held, None).0)
            });
            // **The output is chosen knowing the input.** A set behind a pair
            // and coupled to it by its *ring* had its conventional output
            // read with no input in hand — and the convention, with the sun
            // held, is "carrier in, ring out", so the ring was named both
            // ends. The set solved, at a ratio of exactly one, and nothing
            // said so until a load was routed through it and found it had
            // nowhere to leave by.
            // **A load names nothing here.** A load written at a free port
            // used to name the stage's output, and every load a case added
            // could move it from one shaft to another. Which of two free
            // ports is "the output" decides only the stage's own no-load
            // figures — which a stage with two free ports has none of, its
            // motion being a family. The convention stands.
            let output = onward.unwrap_or_else(|| ports.ends(&held, Some(input)).1);
            local[input] = Condition::Drive(Ratio::ONE);
            // **A stage held still is named at the hold that locked it**: a
            // set with its carrier and its ring both held cannot be entered
            // at its sun at all, and the designer's own holds go in last so
            // the one that closed the set is the one named — the ring, not
            // the sun the convention drives.
            let holds: Vec<Shaft> = self
                .constraints
                .iter()
                .filter_map(|c| match c.at {
                    ShaftRef::Of { stage, shaft } if stage == k => Some(shaft),
                    _ => None,
                })
                .collect();
            let first: Vec<Shaft> = (0..local.len()).filter(|s| !holds.contains(s)).collect();
            if let Err(Refusal::Conflicts(i)) = w
                .alone(&super::teeth_of(stage.members()))
                .map_err(|e| MotionError::Wiring(k, e))?
                .motion_in(&local, &first)
            {
                return Err(MotionError::Conflicts(ShaftRef::Of { stage: k, shaft: i }));
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
        let (system, at) = self.system()?;
        let mut conditions = self.conditions(&at, system.shafts())?;
        // **The train has a ratio between exactly two open bodies**, driven
        // at the first: a chain's two ends, whatever stage each is on. With
        // any other number nothing is driven, the motion is a family, and
        // the train has no figure of its own — each stage still has, and
        // each case decides its own.
        let boundaries = self.boundaries()?;
        let ends = self.ends(&boundaries);
        if let Some((a, _)) = ends {
            conditions[self.resolve(&at, a)?] = Condition::Drive(Ratio::ONE);
        }
        // **Conventions in first, the train's own last**, so that a conflict
        // is named at the statement the designer made rather than at the
        // convention it contradicts: holding a set's carrier beside its
        // held ring is reported at the carrier.
        let order: Vec<Shaft> = std::iter::once(Ok(GROUND))
            .chain(
                self.constraints_in_force()
                    .iter()
                    .map(|c| self.resolve(&at, c.at)),
            )
            .collect::<Result<_, _>>()?;
        let solution = system.motion_in(&conditions, &order).map_err(|e| match e {
            Refusal::Conflicts(i) => MotionError::Conflicts(self.locate(&at, i)),
            Refusal::NoMotion | Refusal::Overflow => MotionError::Overflow,
        })?;
        // **A family reads per turn of a port.** The solver parameterises
        // what is free at whichever shaft fell last in its elimination —
        // a planet, on a set with its ring released — and a designer wants
        // the ring. The ports nothing holds or couples come first, then any
        // port, and a planet only where no port moves with the freedom.
        let joined = |r: ShaftRef| self.couplings.iter().any(|c| c.a == r || c.b == r);
        let mut preferred: Vec<Shaft> = Vec::new();
        for open in [true, false] {
            for (k, stage) in self.stages.iter().enumerate() {
                for shaft in stage.ports().ports {
                    let r = ShaftRef::Of { stage: k, shaft };
                    let i = at[k].of(shaft);
                    if (conditions[i] == Condition::Free && !joined(r)) == open
                        && !preferred.contains(&i)
                    {
                        preferred.push(i);
                    }
                }
            }
        }
        let solution = solution.rebased(&preferred).ok_or(MotionError::Overflow)?;
        let mobility = system.mobility().ok_or(MotionError::Overflow)?;

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
                // `None` where the answer is a family: the quotient of two
                // families is not a number, and the particular values alone
                // would print one as if it were.
                solution.ratio(i, o)
            })
            .collect();
        let total = match ends {
            Some((a, b)) => solution.ratio(self.resolve(&at, a)?, self.resolve(&at, b)?),
            None => None,
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

// ---------------------------------------------- where a load can enter ---

/// A shaft a load can enter the train by, with its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct OpenPort {
    pub at: ShaftRef,
    pub label: ShaftLabel,
}

impl Train {
    /// **Every shaft a load can enter by**: the first shaft of every body
    /// the train does not hold, in the order the chain runs — a shaft two
    /// stages share listed once, under the earlier stage's name, since a
    /// load there is one load on the body the two make. What a picker
    /// offers, and what [`super::solve_train`] admits a load at.
    #[must_use]
    pub fn open_ports(&self, boundaries: &[StageBoundary]) -> Vec<OpenPort> {
        self.bodies(boundaries)
            .into_iter()
            .filter(|b| !b.held)
            .map(|b| OpenPort {
                at: b.shafts[0].0,
                label: b.shafts[0].1,
            })
            .collect()
    }

    /// **The train's two ends**: the first stage's conventional input and
    /// the last stage's conventional output, where each is open — neither
    /// held nor coupled. What the train's own ratio is read between, and
    /// what a preset puts its load and its reaction at; a convention for
    /// reporting and nothing more, since a case says what turns. `None`
    /// where either is held or coupled, or the train is one stage with one
    /// open port.
    #[must_use]
    pub fn ends(&self, boundaries: &[StageBoundary]) -> Option<(ShaftRef, ShaftRef)> {
        let (first, last) = (boundaries.first()?, boundaries.last()?);
        let a = ShaftRef::Of {
            stage: 0,
            shaft: first.input,
        };
        let b = ShaftRef::Of {
            stage: boundaries.len() - 1,
            shaft: last.output,
        };
        let open = self.open_ports(boundaries);
        let single = |r: ShaftRef| {
            open.iter().any(|p| p.at == r) && !self.couplings.iter().any(|c| c.a == r || c.b == r)
        };
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
    /// The local shaft index a [`ShaftRef`] names it by.
    pub shaft: Shaft,
    pub label: ShaftLabel,
    /// **What this port is asked if the train says nothing about it** — the
    /// stage's convention *as the overlay leaves it*, with everything else
    /// the train states in force: a set's ring reads `free` here once its
    /// carrier is held, because holding the carrier releases it. What a
    /// panel's "convention" choice would come to, computed by the rule
    /// rather than guessed from the preset.
    pub by_convention: Constraint,
}

/// **A stage's ports and its conventional holds**, so a panel can offer
/// exactly the shafts a train may constrain or couple — read from the
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
    /// **The members that share a normal module** — the mesh graph's
    /// connected components ([`super::shape::Shape::module_groups`]) — so a
    /// panel offers one box per group and writes it to every member in it,
    /// rather than one per member with nothing tying them.
    pub module_groups: Vec<Vec<usize>>,
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
    /// Turns per turn of what is driven — the whole answer where it is one
    /// answer, and the particular part of it where it is a family.
    pub speed: Exact,
    /// **The rest of a family**: one term per free shaft this one depends
    /// on, *coefficient turns per turn of that shaft*. Empty where the
    /// answer is one answer.
    pub terms: Vec<Term>,
}

/// One term of a shaft's speed in a family: so many turns per turn of a
/// shaft the conditions left free.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Term {
    pub per: ShaftRef,
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
    /// Shafts no mesh touches — named, not counted. Ground is the frame and
    /// is not listed.
    pub untouched: Vec<ShaftRef>,
    pub shafts: Vec<ShaftReport>,
    /// One per stage, input over output. `None` where the output does not
    /// turn.
    pub ratios: Vec<Option<Exact>>,
    /// The first stage's input to the last stage's output. `None` where the
    /// answer is a family, since a quotient of two families is not a number.
    pub total: Option<Exact>,
    /// **The shafts whose turn parameterises a family** — one per condition
    /// the train is short — and empty where the answer is one answer. Every
    /// [`ShaftReport::terms`] is per turn of one of these.
    pub free: Vec<ShaftRef>,
    /// Shafts whose condition said nothing the structure had not already
    /// said. Not a fault — a ring held and also coupled to ground is a
    /// designer being explicit — but worth a reader's knowing.
    pub redundant: Vec<ShaftRef>,
    /// Every shaft a load can enter by, named — what a load case's picker
    /// offers, in the order the chain runs.
    pub ports: Vec<OpenPort>,
    /// **The train's bodies**: every port of every stage, with the shafts
    /// the couplings make one of it — what a load case has a row for.
    pub bodies: Vec<TrainBody>,
}

/// **One body of the train**: a port, or the shafts the couplings fix to
/// one another — a pair's output and the next pair's input are one shaft
/// with two names, and a case says one thing of it. What a load case is a
/// row of: fixed where the train holds it, and otherwise a load, a
/// reaction or free.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TrainBody {
    /// Every shaft of it, in the order the chain runs, each with its name.
    pub shafts: Vec<(ShaftRef, ShaftLabel)>,
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
                let w = stage.wiring();
                StagePorts {
                    members: match stage {
                        super::Stage::Shape(s) => s.member_names(),
                    },
                    module_groups: match stage {
                        super::Stage::Shape(s) => s.module_groups(),
                    },
                    ports: stage
                        .ports()
                        .ports
                        .iter()
                        .map(|&shaft| {
                            let at = ShaftRef::Of { stage: k, shaft };
                            // The train without its own word on this shaft,
                            // and what the overlay then asks of it.
                            let mut without = self.clone();
                            without.constraints.retain(|c| c.at != at);
                            PortSpec {
                                shaft,
                                label: w.shafts[shaft],
                                by_convention: without
                                    .constraints_in_force()
                                    .iter()
                                    .find(|c| c.at == at)
                                    .map_or(Constraint::Free, |c| c.constraint),
                            }
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// A shaft of the assembled system, as a reference: the inverse of
    /// [`Self::resolve`].
    fn locate(&self, at: &[Offsets], i: Shaft) -> ShaftRef {
        if i == GROUND {
            return ShaftRef::Ground;
        }
        let stage = at.iter().rposition(|o| o.first <= i).unwrap_or(0);
        ShaftRef::Of {
            stage,
            shaft: i - at[stage].first + 1,
        }
    }

    /// The train's motion in the shape the boundary sends, or `None` where
    /// there is none to send.
    ///
    /// **A family is sent as a family.** Where the conditions leave `m`
    /// shafts free, every shaft's speed is a particular value plus one term
    /// per free shaft — `ω_i = v_i + Σ_k c_ik · ω_k` — with the free shafts
    /// named in `free`, so a differential's *"the carrier turns at half the
    /// sum of its two sides"* is what a reader sees rather than a refusal.
    #[must_use]
    pub fn motion_report(&self) -> Option<MotionReport> {
        let m = self.motion().ok()?;
        let at = &m.at;
        // **The mechanism's mobility, not the matrix's.** Ground is a shaft
        // in the system and the frame in the world: it counts one degree
        // and one condition in the matrix, and neither to a designer, who
        // reads "mobility 2, one given" of a set with its ring released —
        // the one given being the drive at the train's end its ratio is read
        // from, which the holds do not count.
        let constrained = self
            .conditions(at, m.solution.values.len())
            .ok()?
            .iter()
            .skip(1)
            .filter(|c| **c != Condition::Free)
            .count()
            + usize::from(self.boundaries().ok().and_then(|b| self.ends(&b)).is_some());
        let free: Vec<ShaftRef> = m
            .solution
            .residual
            .iter()
            .map(|r| self.locate(at, r.at))
            .collect();
        Some(MotionReport {
            mobility: m.mobility.degrees.saturating_sub(1),
            constrained,
            untouched: m
                .mobility
                .untouched
                .iter()
                .filter(|&&i| i != GROUND)
                .map(|&i| self.locate(at, i))
                .collect(),
            shafts: m
                .shafts
                .iter()
                .enumerate()
                .map(|(i, s)| ShaftReport {
                    at: self.locate(at, i),
                    label: s.label,
                    speed: s.speed.into(),
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
            redundant: m
                .solution
                .redundant
                .iter()
                .map(|&i| self.locate(at, i))
                .collect(),
            ports: self.open_ports(&self.boundaries().ok()?),
            bodies: self.bodies(&self.boundaries().ok()?),
        })
    }

    /// **Every body of the train** — see [`TrainBody`]: each stage's ports,
    /// with the shafts the couplings join gathered into one, in the order
    /// the chain runs.
    #[must_use]
    pub fn bodies(&self, boundaries: &[StageBoundary]) -> Vec<TrainBody> {
        // Every port of every stage, in order, then joined by the couplings:
        // a shaft joins the body of any shaft a coupling ties it to that is
        // already listed, and opens a body of its own otherwise.
        let mut bodies: Vec<TrainBody> = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            let w = stage.wiring();
            let Some(b) = boundaries.get(k) else { break };
            for shaft in stage.ports().ports {
                let at = ShaftRef::Of { stage: k, shaft };
                let partners: Vec<ShaftRef> = self
                    .couplings
                    .iter()
                    .filter_map(|c| {
                        if c.a == at {
                            Some(c.b)
                        } else if c.b == at {
                            Some(c.a)
                        } else {
                            None
                        }
                    })
                    .collect();
                let held = b.conditions[shaft] == Condition::Ground;
                if let Some(body) = bodies
                    .iter_mut()
                    .find(|x| x.shafts.iter().any(|(s, _)| partners.contains(s)))
                {
                    body.shafts.push((at, w.shafts[shaft]));
                    body.held |= held;
                    continue;
                }
                bodies.push(TrainBody {
                    shafts: vec![(at, w.shafts[shaft])],
                    held,
                });
            }
        }
        bodies
    }

    /// **Two shafts coupled**, in so many words — the same coupling under
    /// either name, so a select on either shaft shows it. A reaction a case
    /// declared at either becomes a load with its torque derived — an
    /// inline take-off, the same physics — since a body two stages share
    /// cannot be a reaction; a free port declared at either is dropped,
    /// there being nothing free about it now. Coupling a shaft to one it
    /// is coupled to already changes nothing.
    pub fn couple(&mut self, a: ShaftRef, b: ShaftRef) {
        if a == b
            || self
                .couplings
                .iter()
                .any(|c| (c.a == a && c.b == b) || (c.a == b && c.b == a))
        {
            return;
        }
        // The earlier stage first, which is the order a body is named in.
        let (a, b) = if b < a { (b, a) } else { (a, b) };
        self.couplings.push(Coupling { a, b });
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
    }

    /// **A shaft released from every coupling** it is in.
    pub fn uncouple(&mut self, at: ShaftRef) {
        self.couplings.retain(|c| c.a != at && c.b != at);
    }

    /// **A shaft held to ground**, in so many words: released from every
    /// coupling — a held shaft turns nothing — and held by the train, which
    /// replaces its stage's conventional holds ([`Self::constraints_in_force`]).
    /// Every case entry at it goes with it: a held shaft is fixed, and no
    /// case can say anything of it.
    pub fn hold(&mut self, at: ShaftRef) {
        self.uncouple(at);
        self.constraints.retain(|c| c.at != at);
        self.constraints.push(ShaftConstraint {
            at,
            constraint: Constraint::Held,
        });
        for case in &mut self.load_cases {
            case.loads.retain(|l| l.at != at);
        }
    }

    /// **A shaft released**: neither held nor coupled — every statement the
    /// train made about it withdrawn, and a hold its stage's convention
    /// puts on it written off in so many words.
    pub fn release(&mut self, at: ShaftRef) {
        self.uncouple(at);
        self.constraints.retain(|c| c.at != at);
        let by_convention = match at {
            ShaftRef::Of { stage, shaft } => self
                .stages
                .get(stage)
                .is_some_and(|s| s.ports().held.contains(&shaft)),
            ShaftRef::Ground => false,
        };
        if by_convention {
            self.constraints.push(ShaftConstraint {
                at,
                constraint: Constraint::Free,
            });
        }
    }

    /// **A fresh case of this kind between the train's two ends**: a
    /// torque at the first, driven at a speed, reacted at the second, the
    /// duty's sweep measured at the second — the case a panel's button adds,
    /// **switched off**, so a case added at its default figures moves no
    /// rating until the designer has written it and switched it on. A train
    /// with no two ends gets it parked, for the designer to move.
    #[must_use]
    pub fn fresh_case(&self, kind: super::CaseKind, torque: f64, speed: f64) -> super::LoadCase {
        let ends = self.boundaries().ok().and_then(|b| self.ends(&b));
        // No two ends: parked, for the first stage to take up
        // ([`Self::push_stage`]) — or, on a train with stages but no two
        // ends, for the designer to move.
        let (input, output) = ends.unwrap_or((PARKED_IN, PARKED_OUT));
        let mut case = match kind {
            super::CaseKind::Ultimate => super::LoadCase::ultimate(input, output, torque, speed),
            super::CaseKind::Fatigue => super::LoadCase::fatigue(input, output, torque, speed),
        };
        case.enabled = false;
        case
    }

    /// **A stage removed**, and everything that named a stage by index
    /// moved with the stages it belongs to: the constraints, the couplings
    /// and every case entry on the removed stage go with it, and those on
    /// the stages after it move down. Left alone, holding stage 3's carrier
    /// held whatever became stage 3. **The last stage removed parks the
    /// cases** at the two places a train with no stages has ([`PARKED_IN`],
    /// [`PARKED_OUT`]) — what was at its conventional input at the one, what
    /// was at its output at the other, every figure kept — and the first
    /// stage pushed takes them up again ([`Self::push_stage`]), so a designer
    /// who swaps their only stage for another keeps their loads.
    pub fn remove_stage(&mut self, k: usize) {
        if k >= self.stages.len() {
            return;
        }
        let parking = (self.stages.len() == 1).then(|| {
            let ports = self.stages[k].ports();
            (ports.input(), ports.output())
        });
        self.stages.remove(k);
        let moved = |r: ShaftRef| -> Option<ShaftRef> {
            match r {
                ShaftRef::Of { stage, shaft } if stage == k => match parking {
                    Some((input, _)) if shaft == input => Some(PARKED_IN),
                    Some((_, output)) if shaft == output => Some(PARKED_OUT),
                    _ => None,
                },
                ShaftRef::Of { stage, shaft } if stage > k => Some(ShaftRef::Of {
                    stage: stage - 1,
                    shaft,
                }),
                other => Some(other),
            }
        };
        self.constraints = self
            .constraints
            .iter()
            .filter_map(|c| {
                moved(c.at).map(|at| ShaftConstraint {
                    at,
                    constraint: c.constraint,
                })
            })
            .collect();
        self.couplings = self
            .couplings
            .iter()
            .filter_map(|c| {
                Some(Coupling {
                    a: moved(c.a)?,
                    b: moved(c.b)?,
                })
            })
            .collect();
        for case in &mut self.load_cases {
            case.loads = case
                .loads
                .iter()
                .filter_map(|l| moved(l.at).map(|at| super::Load { at, ..*l }))
                .collect();
            if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                *at = moved(*at).unwrap_or(ShaftRef::Ground);
            }
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
            .map_or(ShaftRef::Ground, |l| l.at);
        c.duty = if intermittent {
            super::Duty::intermittent(at)
        } else {
            super::Duty::Continuous {
                runtime_hours: 1000.0,
            }
        };
    }

    /// **A stage appended to the train and coupled onward**: its
    /// conventional input to the last stage's remaining open output, where
    /// the train has exactly one open port to offer — a chain grows by one
    /// — and left uncoupled otherwise, an isolated stage for the designer to
    /// tie in. Every case entry at the shaft just coupled moves to the new
    /// stage's conventional output: a load or a reaction at what was the
    /// chain's end is at its new end, which is what the chain did without
    /// saying so.
    pub fn push_stage(&mut self, stage: Stage) {
        let k = self.stages.len();
        let input = ShaftRef::Of {
            stage: k,
            shaft: stage.ports().input(),
        };
        let output = ShaftRef::Of {
            stage: k,
            shaft: stage.ports().output(),
        };
        // **The first stage takes up the parked cases** at its conventional
        // input and output — the conventional use of a stage alone, which
        // is what a fresh case is written as.
        if k == 0 {
            let home = |r: &mut ShaftRef| {
                if *r == PARKED_IN {
                    *r = input;
                } else if *r == PARKED_OUT {
                    *r = output;
                }
            };
            for case in &mut self.load_cases {
                for l in &mut case.loads {
                    home(&mut l.at);
                }
                if let super::Duty::Intermittent { at, .. } = &mut case.duty {
                    home(at);
                }
            }
        }
        let onward = self.boundaries().ok().and_then(|b| {
            let open = self.open_ports(&b);
            // The last stage's open port, where the train has one to give:
            // its output by convention if that is open, else its only one.
            let last = self.stages.last()?;
            let conventional = ShaftRef::Of {
                stage: k - 1,
                shaft: last.ports().output(),
            };
            let mine: Vec<ShaftRef> = open
                .iter()
                .map(|p| p.at)
                .filter(|r| matches!(r, ShaftRef::Of { stage, .. } if *stage == k - 1))
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
            self.couple(from, input);
        }
    }
}
