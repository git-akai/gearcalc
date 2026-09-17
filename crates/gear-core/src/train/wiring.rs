//! **Where a stage's shafts and meshes sit**, so that one solver serves every
//! kind.
//!
//! [`crate::kinematics`] holds the mathematics and knows nothing about gears;
//! this is the declaration that connects a stage to it. A kind answers what
//! shafts it introduces, which shaft each of its members spins with and in
//! whose frame that member's axis stands still, and which members mesh. Nothing
//! else: no module, no shift, no centre distance. **A ratio needs tooth counts
//! and topology, and this is the topology.**
//!
//! # The frame is derived, not stated
//!
//! The handoff this work answers proposes storing each mesh's frame, defaulted
//! to "the deeper `carriedBy` of the two". Stored, it is a second place the
//! same fact lives, and the two can disagree — the first fault
//! `docs/corrections.md` records.
//!
//! So it is **derived**: a mesh's frame is the shaft both its members' axes
//! stand still in, and if they do not name the same one it is not a mesh —
//! two gears whose axes move in different frames have a centre distance that
//! changes, and cannot stay engaged. The refusal is loud and says which mesh.
//!
//! What makes that single-valued is stating [`Mount::axis_fixed_in`] as *the
//! frame the member is stationary in for meshing purposes*, which for a central
//! member is the carrier it is coaxial with rather than the housing its
//! bearings are in. A sun's axis genuinely stands still in the carrier's frame
//! — they share an axis — and naming the carrier is what leaves the intersection
//! with a planet's frame a single shaft.
//!
//! **The one thing this cannot express** is a member central to *two different
//! carriers*, which would need a set of frames rather than one. No arrangement
//! in scope has it: a Ravigneaux has one carrier with two planet sets, and a
//! Simpson is two sets coupled shaft to shaft. If one arrives, the field
//! becomes a list and the frame becomes the intersection — the same derivation,
//! one type wider.
//!
//! # The sign is the mesh kind's
//!
//! A mesh declares [`MeshKind`] and the signed tooth count follows from it
//! ([`MeshKind::sign`]), so a wiring cannot carry a sign that disagrees with the
//! geometry the same mesh is built from. That is deliberate: the lock-up
//! invariant is **silent** on a wrong sign (measured — see
//! [`crate::kinematics`]), so the fault has to be made unrepresentable rather
//! than merely detectable.
//!
//! # What checks a declaration, and what checks the relation
//!
//! Two different things, and neither covers the other:
//!
//! - **the declaration** — that *this* kind's shafts, frames and mesh kinds are
//!   the ones it actually has — is checked against the kind's own kinematics,
//!   in `train::tests`. Six wiring faults were injected against that gate and
//!   all six fired;
//! - **the relation itself** — that a row written this way is what rigid-body
//!   motion gives — is checked by `tools/train_kinematics.py`, which derives
//!   every topology from velocities at the pitch point and shares no expression
//!   with any of this.

use super::StageGear;
use crate::kinematics::{Condition, MeshRow, Shaft, System, HOUSING};
use crate::mesh::MeshKind;

/// A shaft a stage introduces.
///
/// The label is for the harness and for `Debug`; when ports reach the front end
/// it becomes a catalogue key, because a shaft's name is a word the application
/// shows and those live in `strings_<code>.toml`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaftSpec {
    pub label: &'static str,
}

/// Where one member sits: what it turns with, and what its axis stands still
/// in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mount {
    /// The shaft this member rotates with. Two members sharing one — a
    /// compound planet's two gears, a wobble body's — need no coupling row;
    /// they are the same shaft.
    pub spins_with: Shaft,
    /// **The frame its axis is stationary in.** For a member riding a carrier
    /// that is the carrier; for a member coaxial with a carrier it is *that
    /// carrier*, not the housing (see the module note). For a fixed-axis pair
    /// it is the housing.
    pub axis_fixed_in: Shaft,
}

impl Mount {
    /// A member riding a carrier at some radius.
    #[must_use]
    pub const fn riding(spins_with: Shaft, carrier: Shaft) -> Self {
        Self {
            spins_with,
            axis_fixed_in: carrier,
        }
    }

    /// A member whose axis **is** the frame's axis — a sun, a ring, a hula
    /// stage's two fixed-axis gears — so it stands still in that frame.
    #[must_use]
    pub const fn coaxial_with(spins_with: Shaft, frame: Shaft) -> Self {
        Self {
            spins_with,
            axis_fixed_in: frame,
        }
    }
}

/// One mesh, by member index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshSpec {
    /// Indices into the stage's members, in [`super::StageResult::members`]
    /// order. On an internal mesh `b` is the **ring**, which is
    /// [`MeshKind::Internal`]'s own convention and the order every kind here
    /// already builds its meshes in.
    ///
    /// **The solve is indifferent to the order**, and that is measured rather
    /// than assumed: swapping the two on an internal mesh negates the whole
    /// row, and a row and its negation have the same nullspace, the same
    /// rowspace and the same rank. So the convention is for the reader and for
    /// the geometry that reads it, and a wiring cannot get the kinematics wrong
    /// by writing the pair the other way round. Every *other* transposition
    /// tried against these tests — the kind, the frame, the shaft a member
    /// spins with — is caught.
    pub a: usize,
    pub b: usize,
    pub kind: MeshKind,
    /// How many parallel instances of this mesh the stage has — the planet
    /// count. It changes no speed and no ratio, every instance being identical;
    /// it is what a member's engagements are counted over
    /// ([`super::engagements`]).
    pub paths: u32,
}

/// **What a stage owes the one solver**: its shafts, where its members sit, and
/// what meshes what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wiring {
    /// Shaft 0 is always the housing.
    pub shafts: Vec<ShaftSpec>,
    /// One per member, in [`super::StageResult::members`] order.
    pub mounts: Vec<Mount>,
    /// One per mesh, in [`super::StageResult::meshes`] order.
    pub meshes: Vec<MeshSpec>,
    /// **What the stage is presently asked**, one condition per shaft.
    ///
    /// Transitional. Which shaft is held is a fact about how the *train* is
    /// wired rather than about the stage's geometry, and the plan moves it
    /// there; until it does, a kind states the arrangement it already carries
    /// so that the graph can be asked the same question the kind answers.
    pub conditions: Vec<Condition>,
    /// The shaft a load arrives at, and the one it leaves by — the stage's two
    /// ports, in the order the train chains them. A kind with a third shaft a
    /// load could enter by is what [`Self::conditions`] is waiting on.
    pub input: Shaft,
    pub output: Shaft,
}

/// Why a wiring could not be turned into a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WiringError {
    /// A mesh whose two members' axes stand still in different frames. They
    /// cannot stay engaged: the distance between their axes changes as the two
    /// frames turn against each other.
    NoCommonFrame(usize),
    /// A member index, shaft index or tooth count a wiring cannot mean.
    NotAMesh(usize),
}

impl Wiring {
    /// The frame of one mesh: the shaft both its members' axes stand still in.
    ///
    /// # Errors
    ///
    /// [`WiringError::NoCommonFrame`] where they name different shafts.
    pub fn frame(&self, mesh: usize) -> Result<Shaft, WiringError> {
        let m = self.meshes.get(mesh).ok_or(WiringError::NotAMesh(mesh))?;
        let (a, b) = (
            self.mounts.get(m.a).ok_or(WiringError::NotAMesh(mesh))?,
            self.mounts.get(m.b).ok_or(WiringError::NotAMesh(mesh))?,
        );
        if a.axis_fixed_in != b.axis_fixed_in {
            return Err(WiringError::NoCommonFrame(mesh));
        }
        Ok(a.axis_fixed_in)
    }

    /// **The system this wiring and these tooth counts make**, with `teeth` one
    /// entry per member in the same order as [`Self::mounts`].
    ///
    /// `at` is where this stage's shafts begin in a larger system, so a train
    /// can lay several stages over one housing; a stage asked about on its own
    /// passes [`Offsets::alone`].
    ///
    /// # Errors
    ///
    /// [`WiringError`] for a wiring that does not describe meshes, and for a
    /// tooth count or shaft index the system will not take.
    pub fn add_to(
        &self,
        system: &mut System,
        teeth: &[u32],
        at: &Offsets,
    ) -> Result<(), WiringError> {
        for (k, m) in self.meshes.iter().enumerate() {
            let frame = self.frame(k)?;
            let (za, zb) = (
                i64::from(*teeth.get(m.a).ok_or(WiringError::NotAMesh(k))?),
                i64::from(*teeth.get(m.b).ok_or(WiringError::NotAMesh(k))?),
            );
            system
                .mesh(MeshRow {
                    a: at.of(self.mounts[m.a].spins_with),
                    b: at.of(self.mounts[m.b].spins_with),
                    za,
                    // **The sign is the mesh kind's**, never the wiring's.
                    zb: m.kind.signed(zb),
                    frame: at.of(frame),
                })
                .ok_or(WiringError::NotAMesh(k))?;
        }
        Ok(())
    }

    /// The system for this stage on its own, with its own housing.
    ///
    /// # Errors
    ///
    /// As [`Self::add_to`].
    pub fn alone(&self, teeth: &[u32]) -> Result<System, WiringError> {
        let mut system = System::new(self.shafts.len());
        self.add_to(&mut system, teeth, &Offsets::alone())?;
        Ok(system)
    }
}

/// Where a stage's shafts sit in a larger system.
///
/// The housing is shared — there is one of it — so shaft 0 maps to shaft 0 and
/// everything else is displaced. A stage asked about on its own is the identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Offsets {
    /// The global index this stage's shaft 1 takes.
    pub first: Shaft,
}

impl Offsets {
    /// A stage with the whole system to itself.
    #[must_use]
    pub const fn alone() -> Self {
        Self { first: 1 }
    }

    /// The global index of one of this stage's shafts.
    #[must_use]
    pub const fn of(&self, local: Shaft) -> Shaft {
        if local == HOUSING {
            HOUSING
        } else {
            local + self.first - 1
        }
    }
}

/// **Every constrainable member a stage has, as tooth counts** — the companion
/// of [`super::member_inputs`], and the argument [`Wiring::alone`] wants.
pub(crate) fn teeth_of<'a>(members: impl IntoIterator<Item = &'a StageGear>) -> Vec<u32> {
    members.into_iter().map(|g| g.teeth).collect()
}

/// Conditions for a stage on its own: the housing held, one shaft driven at
/// unit speed, another held where the arrangement holds one, and the rest free.
///
/// One helper because all three kinds want the same shape and a kind writing it
/// out is a kind that can write it out differently.
pub(crate) fn arranged(shafts: usize, input: Shaft, held: &[Shaft]) -> Vec<Condition> {
    let mut out = vec![Condition::Free; shafts];
    out[HOUSING] = Condition::Ground;
    for &h in held {
        out[h] = Condition::Ground;
    }
    out[input] = Condition::Drive(crate::ratio::Ratio::ONE);
    out
}

// ------------------------------------------------- the train as one system ---

use super::Train;
use crate::kinematics::{Mobility, Refusal, Solution};
use crate::ratio::Ratio;

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
    Wiring(usize, WiringError),
    /// The conditions and the structure cannot both hold.
    Refused(Refusal),
}

/// One shaft of an assembled train.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShaftMotion {
    /// Which stage introduced it, and `None` for the housing, which every
    /// stage shares.
    pub stage: Option<usize>,
    pub label: &'static str,
    /// Turns per turn of the driven port — exactly.
    pub speed: Ratio,
}

/// **What a whole train does, from tooth counts and topology alone.**
///
/// No module, no shift, no material and no load. That independence is the
/// finding this type exists for: `solve_train` refuses a train outright when
/// any one stage will not close geometrically, and takes every other stage's
/// ratio down with it — though Willis needs none of what failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrainMotion {
    pub shafts: Vec<ShaftMotion>,
    /// One per stage: input over output, or `None` where the output does not
    /// turn — two meshes stepping by the same amount and cancelling, which is a
    /// refusal rather than a very large number.
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

    /// **One member's speed, and its speed against the frame of its mesh** —
    /// turns per turn of the driven port.
    ///
    /// The second is what its teeth see and what its cycles are counted from,
    /// and it is the same subtraction for every kind: a pair's frame is the
    /// housing and the difference is the member's own speed; an epicyclic
    /// member's frame is its carrier, and a held ring's difference is *not*
    /// zero while its speed is.
    #[must_use]
    pub fn member(&self, wiring: &Wiring, stage: usize, member: usize) -> Option<(Ratio, Ratio)> {
        let mount = wiring.mounts.get(member)?;
        let own = self.solution.values[self.shaft_of(stage, mount.spins_with)];
        let frame = self.solution.values[self.shaft_of(stage, mount.axis_fixed_in)];
        Some((own, own.checked_sub(frame)?))
    }
}

impl Train {
    /// **The whole train as one system**, and where each stage's shafts sit in
    /// it.
    ///
    /// One housing, shared; each stage's other shafts appended in order; and a
    /// rigid coupling from each stage's output to the next stage's input, which
    /// is what a shaft line *is*. The chain is the default and the only
    /// topology a train can presently describe — a coupling list of its own is
    /// what the plan adds when ports become named shafts.
    ///
    /// # Errors
    ///
    /// [`MotionError::Empty`], or the stage whose wiring does not describe
    /// meshes.
    pub fn system(&self) -> Result<(System, Vec<Offsets>), MotionError> {
        if self.stages.is_empty() {
            return Err(MotionError::Empty);
        }
        let wirings: Vec<Wiring> = self.stages.iter().map(super::Stage::wiring).collect();
        let mut at = Vec::with_capacity(wirings.len());
        let mut next = 1;
        for w in &wirings {
            at.push(Offsets { first: next });
            next += w.shafts.len() - 1;
        }
        let mut system = System::new(next);
        for (k, w) in wirings.iter().enumerate() {
            let teeth = teeth_of(self.stages[k].members());
            w.add_to(&mut system, &teeth, &at[k])
                .map_err(|e| MotionError::Wiring(k, e))?;
        }
        for k in 1..wirings.len() {
            let from = at[k - 1].of(wirings[k - 1].output);
            let to = at[k].of(wirings[k].input);
            system
                .couple(from, to)
                .ok_or(MotionError::Wiring(k, WiringError::NotAMesh(k)))?;
        }
        Ok((system, at))
    }

    /// **What the train is asked**, one condition per shaft of the assembled
    /// system.
    ///
    /// The housing is held; every shaft a stage's own arrangement holds is
    /// held; and the **first** stage's input is driven at unit speed. The
    /// intermediate stages' drives are dropped, since a stage in a chain is
    /// turned by the one before it rather than by a motor of its own — which is
    /// the one thing a chain says that a stage cannot say for itself.
    #[must_use]
    pub fn conditions(&self, at: &[Offsets], shafts: usize) -> Vec<Condition> {
        let mut out = vec![Condition::Free; shafts];
        out[HOUSING] = Condition::Ground;
        let wirings: Vec<Wiring> = self.stages.iter().map(super::Stage::wiring).collect();
        for (k, w) in wirings.iter().enumerate() {
            for (local, c) in w.conditions.iter().enumerate() {
                if local != HOUSING && *c == Condition::Ground {
                    out[at[k].of(local)] = Condition::Ground;
                }
            }
        }
        if let (Some(w), Some(a)) = (wirings.first(), at.first()) {
            out[a.of(w.input)] = Condition::Drive(Ratio::ONE);
        }
        out
    }

    /// **The train's motion**, at one turn of the driven port.
    ///
    /// # Errors
    ///
    /// [`MotionError`] — and never a geometric one, which is the whole reason
    /// this is separate from [`super::solve_train`].
    pub fn motion(&self) -> Result<TrainMotion, MotionError> {
        let (system, at) = self.system()?;
        let conditions = self.conditions(&at, system.shafts());
        let solution = system.motion(&conditions).map_err(MotionError::Refused)?;
        let mobility = system
            .mobility()
            .ok_or(MotionError::Refused(Refusal::Overflow))?;

        let wirings: Vec<Wiring> = self.stages.iter().map(super::Stage::wiring).collect();
        let mut shafts = vec![ShaftMotion {
            stage: None,
            label: "housing",
            speed: solution.values[HOUSING],
        }];
        for (k, w) in wirings.iter().enumerate() {
            for (local, s) in w.shafts.iter().enumerate().skip(1) {
                shafts.push(ShaftMotion {
                    stage: Some(k),
                    label: s.label,
                    speed: solution.values[at[k].of(local)],
                });
            }
        }
        let ratios: Vec<Option<Ratio>> = wirings
            .iter()
            .enumerate()
            .map(|(k, w)| {
                let (i, o) = (at[k].of(w.input), at[k].of(w.output));
                solution.values[i].checked_div(solution.values[o])
            })
            .collect();
        let total = match (wirings.first(), wirings.last()) {
            (Some(f), Some(l)) => {
                let i = at[0].of(f.input);
                let o = at[wirings.len() - 1].of(l.output);
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
