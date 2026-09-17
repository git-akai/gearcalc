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
