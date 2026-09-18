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
    /// Neither — it does what the rest decides. Whether it carries a torque
    /// is a load case's question, not this one's: the output of a set is
    /// free in exactly this sense and carries the whole load.
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
        // The conventional drive goes from every stage the train drives
        // itself — **before** any of the train's own are laid, so that two
        // drives the designer wrote on one set are both kept. Written the
        // other way round, the second drive removed the first, and a
        // differential's two inputs came out as one input and a family.
        for own in &self.constraints {
            if own.constraint == Constraint::Driven {
                if let ShaftRef::Of { stage, .. } = own.at {
                    out.retain(|c| {
                        !(c.constraint == Constraint::Driven
                            && matches!(c.at, ShaftRef::Of { stage: s, .. } if s == stage))
                    });
                }
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
            let held: Vec<Shaft> = local
                .iter()
                .enumerate()
                .skip(1)
                .filter(|(_, c)| **c == Condition::Ground)
                .map(|(i, _)| i)
                .collect();
            let input = driven
                .or_else(|| side(true, true))
                .unwrap_or_else(|| ports.ends(&held, None).0);
            // **The output is chosen knowing the input.** A set behind a pair
            // and coupled to it by its *ring* had its conventional output
            // read with no input in hand — and the convention, with the sun
            // held, is "carrier in, ring out", so the ring was named both
            // ends. The set solved, at a ratio of exactly one, and nothing
            // said so until a load was routed through it and found it had
            // nowhere to leave by.
            let output = side(false, false).unwrap_or_else(|| ports.ends(&held, Some(input)).1);
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
        let couplings = self.couplings_in_force();
        let joined = |r: ShaftRef| couplings.iter().any(|c| c.a == r || c.b == r);
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
                // `None` where the answer is a family: the quotient of two
                // families is not a number, and the particular values alone
                // would print one as if it were.
                solution.ratio(i, o)
            })
            .collect();
        let total = match (boundaries.first(), boundaries.last()) {
            (Some(f), Some(l)) => {
                let i = at[0].of(f.input);
                let o = at[boundaries.len() - 1].of(l.output);
                solution.ratio(i, o)
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

// ------------------------------------------------- where a load travels ---

use super::Port;
use crate::contact::Drive;

/// **The route one load takes** from the shaft it enters by to the far end of
/// the shaft line: each stage it crosses, in order, with the direction it
/// crosses it in — and the port it arrives at, whatever holds it there.
///
/// This is the chain read off the graph for one load rather than assumed of
/// it: a load entering at the conventional start walks forward through every
/// stage and one entering at the end walks back, as they always did, and one
/// entering by a set's carrier is *backward* through that set and forward
/// through everything after it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Route {
    pub steps: Vec<(usize, Drive)>,
    pub far: ShaftRef,
}

impl Route {
    /// The direction the load crosses stage `k` in, where it reaches it.
    #[must_use]
    pub fn drive_at(&self, k: usize) -> Option<Drive> {
        self.steps.iter().find(|(s, _)| *s == k).map(|(_, d)| *d)
    }
}

/// Why a load cannot be routed from a shaft.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// The shaft is not one a load can be put on: ground, a held shaft, a
    /// shaft the train does not have, or a stage's shaft that is not a port.
    NotAPort,
    /// The shaft sits between two stages, so the load could leave by either
    /// end and the route is not one route.
    Shared,
}

/// A shaft a load can enter the train by, with the name it answers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct OpenPort {
    /// `Start` or `End` where the chain names it, else the shaft itself.
    pub port: Port,
    pub at: ShaftRef,
    pub label: ShaftLabel,
}

impl Train {
    /// **The shaft a port names**, under the constraints in force.
    ///
    /// `Start` is the first stage's input and `End` the last stage's output —
    /// read off the boundaries, so a set at the head of a chain driven by its
    /// carrier has its carrier as the start — and `At` is itself.
    #[must_use]
    pub fn port_shaft(&self, boundaries: &[StageBoundary], port: Port) -> ShaftRef {
        let last = boundaries.len().saturating_sub(1);
        match port {
            Port::Start => ShaftRef::Of {
                stage: 0,
                shaft: boundaries.first().map_or(0, |b| b.input),
            },
            Port::End => ShaftRef::Of {
                stage: last,
                shaft: boundaries.last().map_or(0, |b| b.output),
            },
            Port::At(r) => r,
        }
    }

    /// The name a shaft answers to as a port: `Start` or `End` where it is
    /// one of the chain's two ends, else itself. The inverse of
    /// [`Self::port_shaft`], so a result names a shaft the way the input
    /// would have.
    #[must_use]
    pub fn port_named(&self, boundaries: &[StageBoundary], at: ShaftRef) -> Port {
        [Port::Start, Port::End]
            .into_iter()
            .find(|&p| self.port_shaft(boundaries, p) == at)
            .unwrap_or(Port::At(at))
    }

    /// **Every shaft a load can enter by**: each stage's ports that are
    /// neither held nor coupled to another stage, each with the name the
    /// chain gives it. What a picker offers.
    #[must_use]
    pub fn open_ports(&self, boundaries: &[StageBoundary]) -> Vec<OpenPort> {
        let couplings = self.couplings_in_force();
        let joined = |r: ShaftRef| couplings.iter().any(|c| c.a == r || c.b == r);
        let mut out = Vec::new();
        for (k, stage) in self.stages.iter().enumerate() {
            let w = stage.wiring();
            let Some(b) = boundaries.get(k) else { break };
            for shaft in stage.ports().ports {
                let at = ShaftRef::Of { stage: k, shaft };
                if b.conditions[shaft] != Condition::Ground && !joined(at) {
                    out.push(OpenPort {
                        port: self.port_named(boundaries, at),
                        at,
                        label: w.shafts[shaft],
                    });
                }
            }
        }
        out
    }

    /// **The route a load entering at `from` takes** to the far end of the
    /// shaft line.
    ///
    /// At each stage the load arrives by that stage's input or its output —
    /// forward or backward through it — and leaves by the other, which is
    /// coupled to the next stage or is where the route ends. A shaft that is
    /// neither end of its stage's boundary is held, or is no port at all, and
    /// no load can be put on it; a shaft coupled to a stage on *both* sides
    /// gives the load two ways out, which is a division this model does not
    /// make ([`super::TrainError::LoadShared`]).
    ///
    /// # Errors
    ///
    /// [`RouteError`].
    pub fn route(&self, boundaries: &[StageBoundary], from: ShaftRef) -> Result<Route, RouteError> {
        let couplings = self.couplings_in_force();
        // The stage a coupling joins `r` to, and the shaft it enters there.
        let across = |r: ShaftRef| -> Vec<ShaftRef> {
            couplings
                .iter()
                .filter_map(|c| {
                    if c.a == r {
                        Some(c.b)
                    } else if c.b == r {
                        Some(c.a)
                    } else {
                        None
                    }
                })
                .collect()
        };
        // Crossing one stage: the direction, and the shaft the load leaves by.
        let cross = |r: ShaftRef| -> Result<(usize, Drive, ShaftRef), RouteError> {
            let ShaftRef::Of { stage, shaft } = r else {
                return Err(RouteError::NotAPort);
            };
            let b = boundaries.get(stage).ok_or(RouteError::NotAPort)?;
            let (drive, leaves) = if shaft == b.input && shaft != b.output {
                (Drive::Forward, b.output)
            } else if shaft == b.output && shaft != b.input {
                (Drive::Backward, b.input)
            } else {
                return Err(RouteError::NotAPort);
            };
            Ok((
                stage,
                drive,
                ShaftRef::Of {
                    stage,
                    shaft: leaves,
                },
            ))
        };
        // Entering by a shaft another stage is coupled to is entering between
        // two stages.
        if !across(from).is_empty() {
            return Err(RouteError::Shared);
        }
        let mut steps = Vec::new();
        let mut here = from;
        loop {
            let (stage, drive, leaves) = cross(here)?;
            if steps.iter().any(|(s, _)| *s == stage) {
                return Err(RouteError::Shared);
            }
            steps.push((stage, drive));
            match across(leaves).as_slice() {
                [] => return Ok(Route { steps, far: leaves }),
                [next] => here = *next,
                _ => return Err(RouteError::Shared),
            }
        }
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
        // reads "mobility 2, one given" of a set with its ring released.
        let constrained = self
            .constraints_in_force()
            .iter()
            .filter(|c| c.constraint != Constraint::Free)
            .count();
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
