//! **Where a stage's slots and meshes sit**, so that one solver serves every
//! kind.
//!
//! [`crate::kinematics`] holds the mathematics and knows nothing about gears;
//! this is the declaration that connects a stage to it. A stage answers, in
//! its own numbering — its **slots**, ground 0 and then the bodies it lists
//! in order — which slot each of its members spins with and in whose frame
//! that member's axis stands still, and which members mesh. Nothing else: no
//! module, no shift, no centre distance, and nothing about which train body
//! a slot is (the stage's own list of bodies says, and the train hands
//! `add_to` the lookup). **A
//! ratio needs tooth counts and topology, and this is the topology.**
//!
//! # The frame is derived, not stated
//!
//! The handoff this work answers proposes storing each mesh's frame, defaulted
//! to "the deeper `carriedBy` of the two". Stored, it is a second place the
//! same fact lives, and the two can disagree — the first fault
//! `docs/corrections.md` records.
//!
//! So it is **derived**: a mesh's frame is the body both its members' axes
//! stand still in, and if they do not name the same one it is not a mesh —
//! two gears whose axes move in different frames have a centre distance that
//! changes, and cannot stay engaged. The refusal is loud and says which mesh.
//!
//! What makes that single-valued is stating [`Mount::axis_fixed_in`] as *the
//! frame the member is stationary in for meshing purposes*, which for a central
//! member is the carrier it is coaxial with rather than ground. A sun's axis
//! genuinely stands still in the carrier's frame
//! — they share an axis — and naming the carrier is what leaves the intersection
//! with a planet's frame a single body.
//!
//! **The one thing this cannot express** is a member central to *two different
//! carriers*, which would need a set of frames rather than one. No arrangement
//! in scope has it: a Ravigneaux has one carrier with two planet sets, and a
//! Simpson is two sets coupled body to body. If one arrives, the field
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
//! - **the declaration** — that *this* stage's bodies, frames and mesh kinds
//!   are the ones it actually has — was checked against each retired stage
//!   type's own kinematics, in `train::tests`, and is checked now against
//!   Pennestrì's closed form and the reference tables. Six wiring faults
//!   were injected against that gate and all six fired;
//! - **the relation itself** — that a row written this way is what rigid-body
//!   motion gives — is checked by `tools/train_kinematics.py`, which derives
//!   every topology from velocities along the base circles' common tangent
//!   and shares no expression with any of this: the mesh sense there is
//!   which tangent the two circles admit, not [`MeshKind::sign`].

use super::StageGear;
use crate::kinematics::{Body, MeshRow, System};
use crate::mesh::MeshKind;
use crate::ratio::Ratio;

/// **What a body is**, structurally — which is the only thing `gear-core` may
/// say about it, a name being a word the application shows.
///
/// It was a `&'static str` — `"sun"`, `"crank"`, `"wobble"` — which is English
/// in the core (rule 2) and a *role* rather than a fact. A general epicyclic
/// stage has no sun; it has central members and carriers, and this is the list
/// that survives that: ground, the body a member spins with, or a carrier —
/// a frame that is nobody's body. The front end names the second after the
/// member (`gearNumber`) and the third with its own word, as it already does.
///
/// **It does not cross the boundary.** The panel names a body off the train
/// it already holds — which member sits on it, by the one rule
/// `members.ts` reads — so a label sent beside every port, every case row
/// and every end was a second naming nothing read. The harness uses it in
/// process (`gear-cli kinematics`), which is what it is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum BodyLabel {
    /// The one held frame — see [`GROUND`].
    Ground,
    /// The body a member spins with, by member index in
    /// [`super::ShapeResult::members`] order. Where several members share one
    /// — a compound planet, a wobble body — it is the first of them.
    Member { member: usize },
    /// A frame that carries meshes and is no member's body: a set's carrier,
    /// a hula stage's crank. Numbered within the stage.
    Carrier { index: usize },
}

/// Where one member sits: what it turns with, and what its axis stands still
/// in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Mount {
    /// The body this member rotates with. Two members sharing one — a
    /// compound planet's two gears, a wobble body's — need no coupling row;
    /// they are the same body.
    pub spins_with: Body,
    /// **The frame its axis is stationary in.** For a member riding a carrier
    /// that is the carrier; for a member coaxial with a carrier it is *that
    /// carrier*, not ground (see the module note). For a fixed-axis pair it is
    /// ground.
    pub axis_fixed_in: Body,
    /// **Whether this member is one of a mesh's parallel instances** — a
    /// planet, of which there are N — as against a member the N of them all
    /// meet, which is a central one.
    ///
    /// It is the same distinction as riding a carrier versus sitting on its
    /// axis, and it exists because [`MeshSpec::paths`] is **not symmetric**: a
    /// sun tooth passes all N planets in one revolution against the carrier,
    /// and a planet tooth meets the one sun. Applying N to both counts a
    /// planet's engagements N times over — which the crate did, three times
    /// over on the shipped set, until the wiring gave the two members of a
    /// mesh somewhere to differ ([`Wiring::paths_seen`]).
    pub replicated: bool,
}

/// One mesh, by member index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MeshSpec {
    /// Indices into the stage's members, in [`super::ShapeResult::members`]
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
    /// tried against these tests — the mesh kind, the frame, the body a member
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

/// **What a stage owes the one solver**: its slots, where its members sit, and
/// what meshes what.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wiring {
    /// What each slot is; slot 0 is always ground.
    pub slots: Vec<BodyLabel>,
    /// One per member, in [`super::ShapeResult::members`] order.
    pub mounts: Vec<Mount>,
    /// One per mesh, in [`super::ShapeResult::meshes`] order.
    pub meshes: Vec<MeshSpec>,
}

/// Why a wiring could not be turned into a system.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WiringError {
    /// A mesh whose two members' axes stand still in different frames. They
    /// cannot stay engaged: the distance between their axes changes as the two
    /// frames turn against each other.
    NoCommonFrame(usize),
    /// **A member with no teeth**, by index. The one refusal here a *design*
    /// can reach — `StageGear::teeth` is a `u32` and nothing stops a designer
    /// typing zero — and it is checked before any geometry, because a gear with
    /// no teeth used to be reported as *"the tooth is too undercut to have a
    /// root section"*, which describes a tooth that exists.
    MemberWithoutTeeth(usize),
    /// A member or body index a wiring names and does not have, or a gear
    /// meshing itself. A preset's defect rather than a design's.
    NotAMesh(usize),
    /// **The stage's boundary does not determine its motion**, or contradicts
    /// it: too few of its bodies held or driven for one answer, or two
    /// conditions that cannot both hold. A train with mobility above one is
    /// answered with a family at the train level; a *stage* solve wants one
    /// motion to rate under, and this says the boundary did not give it one.
    Unsolvable,
}

impl Wiring {
    /// The frame of one mesh: the body both its members' axes stand still in.
    ///
    /// # Errors
    ///
    /// [`WiringError::NoCommonFrame`] where they name different bodies.
    pub fn frame(&self, mesh: usize) -> Result<Body, WiringError> {
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

    /// **How many parallel mesh paths this member's own teeth meet**, per
    /// revolution against the frame of its mesh.
    ///
    /// A central member meets every instance: a sun tooth passes all N planets
    /// in one turn against the carrier. A **planet meets one of each**, because
    /// it *is* one of the instances — the other planets have their own teeth
    /// and are not in its mesh at all.
    ///
    /// # It was N for every member, and that is N times too many for a planet
    ///
    /// `docs/reference.md#tooth-cycles` states the rule correctly — *once for
    /// each parallel mesh path* — and the planetary stage applied the planet
    /// count to all three members alike, so the shipped set reported its
    /// planet's cycles **three times over**. Nothing caught it because
    /// `MeshSpec::paths` looks symmetric and the two members of a mesh had
    /// nowhere to differ; [`Mount::replicated`] is that somewhere.
    ///
    /// A member in more than one mesh takes the largest, since the count is one
    /// number per member rather than one per mesh — a planet's two meshes are
    /// one path each, so the two agree and nothing is hidden by the choice.
    #[must_use]
    pub fn paths_seen(&self, member: usize) -> u32 {
        if self.mounts.get(member).is_some_and(|m| m.replicated) {
            return 1;
        }
        self.meshes
            .iter()
            .filter(|m| m.a == member || m.b == member)
            .map(|m| m.paths)
            .max()
            .unwrap_or(1)
    }

    /// **The system this wiring and these tooth counts make**, with `teeth` one
    /// entry per member in the same order as [`Self::mounts`].
    ///
    /// `at` says what this stage's slots are in a larger system, so a train
    /// can lay several stages over one set of bodies — a stage lists its
    /// bodies and answers this off that list ([`super::shape::Shape::body_at`]);
    /// a stage asked about on its own passes the identity.
    ///
    /// # Errors
    ///
    /// [`WiringError`] for a wiring that does not describe meshes, and for a
    /// tooth count or body index the system will not take.
    pub fn add_to(
        &self,
        system: &mut System,
        teeth: &[u32],
        at: impl Fn(Body) -> Body,
    ) -> Result<(), WiringError> {
        for (k, m) in self.meshes.iter().enumerate() {
            let frame = self.frame(k)?;
            let count = |member: usize| -> Result<i64, WiringError> {
                match teeth.get(member) {
                    None => Err(WiringError::NotAMesh(k)),
                    Some(0) => Err(WiringError::MemberWithoutTeeth(member)),
                    Some(z) => Ok(i64::from(*z)),
                }
            };
            let (za, zb) = (count(m.a)?, count(m.b)?);
            system
                .mesh(MeshRow {
                    a: at(self.mounts[m.a].spins_with),
                    b: at(self.mounts[m.b].spins_with),
                    za,
                    // **The sign is the mesh kind's**, never the wiring's.
                    zb: m.kind.signed(zb),
                    frame: at(frame),
                })
                .ok_or(WiringError::NotAMesh(k))?;
        }
        Ok(())
    }

    /// **Every member's motion at one turn of this stage's input**, from the
    /// topology, the tooth counts and what the stage is asked
    /// ([`super::StageBoundary`]) — and nothing else.
    ///
    /// No module, no shift, no material: a stage that will not close still
    /// turns, and this is what can be asked of it either way.
    ///
    /// # Errors
    ///
    /// As [`Self::alone`], or [`WiringError::Unsolvable`] where the boundary
    /// leaves the stage's motion undetermined or contradicts its structure.
    pub fn unit_motion(
        &self,
        teeth: &[u32],
        boundary: &super::StageBoundary,
    ) -> Result<UnitMotion, WiringError> {
        let system = self.alone(teeth)?;
        let solution = system
            .motion(&boundary.conditions)
            .map_err(|_| WiringError::Unsolvable)?;
        if !solution.is_unique() {
            return Err(WiringError::Unsolvable);
        }
        let input = solution.values[boundary.input].to_f64();
        let members = self
            .mounts
            .iter()
            .enumerate()
            .map(|(i, mount)| {
                let speed = solution.values[mount.spins_with];
                let frame = solution.values[mount.axis_fixed_in];
                MemberMotion {
                    speed,
                    against_frame: speed.checked_sub(frame).unwrap_or(Ratio::ZERO),
                    engagements: crate::train::engagements(
                        speed.to_f64(),
                        frame.to_f64(),
                        input,
                        f64::from(self.paths_seen(i)),
                    ),
                }
            })
            .collect();
        Ok(UnitMotion {
            members,
            output: solution.values[boundary.output],
        })
    }

    /// The system for this stage on its own, against its own ground.
    ///
    /// # Errors
    ///
    /// As [`Self::add_to`].
    pub fn alone(&self, teeth: &[u32]) -> Result<System, WiringError> {
        let mut system = System::new(self.slots.len());
        self.add_to(&mut system, teeth, |slot| slot)?;
        Ok(system)
    }
}

/// **What one member does at one turn of its stage's input body.**
///
/// The one place a stage's speeds come from. Three stage types each worked these out
/// for themselves — a pair from its tooth-count ratio, a set from
/// `planetary::power`, a hula stage from the two products — and between them
/// they disagreed about *sign*: a pair's second member came back positive while
/// turning backwards, so the output member of one stage and the input member of
/// the next reported opposite signs for one physical body.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemberMotion {
    /// Turns per turn of the stage's input — **signed**, so a member that runs
    /// backwards says so, and **exact**, so a case's speed is rounded once
    /// when it is scaled by it ([`Ratio::scale`]) rather than twice.
    pub speed: Ratio,
    /// ...against the frame of its mesh, which is what its teeth see. A pair's
    /// frame is ground and this is its own speed; a held ring's is **not zero**
    /// while its speed is.
    pub against_frame: Ratio,
    /// How often it is engaged per turn of the stage's input — its motion
    /// against its frame, over the paths its own teeth meet
    /// ([`Wiring::paths_seen`]).
    pub engagements: f64,
}

/// **Everything a stage's motion comes to at one turn of its input**: each
/// member's, and the stage's own reduction.
///
/// The reduction is here rather than derived from the members because a
/// stage's output is a **body**, and a body need not carry a gear — an
/// epicyclic set's output is its carrier, which no member spins with. Reading
/// "the last member's speed" for it is right on a pair and wrong on a set, and
/// this type exists because that was written twice before it was noticed.
#[derive(Clone, Debug, PartialEq)]
pub struct UnitMotion {
    /// One per member, in [`super::ShapeResult::members`] order.
    pub members: Vec<MemberMotion>,
    /// The output body's speed, turns per turn of the input — the wiring's
    /// `output`, whatever sits on it.
    pub output: Ratio,
}

impl UnitMotion {
    /// **The stage's reduction**: input turns per output turn, signed. An
    /// external pair's output turns the other way and its ratio says so;
    /// infinity where the output does not turn, which is what two meshes
    /// cancelling is.
    ///
    /// One reading, where `ShapeResult::ratio()` used to mean three: `z₂/z₁`
    /// on a pair, a magnitude; the Willis answer on a set, signed; the two
    /// products on a hula stage.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        self.output.recip().map_or(f64::INFINITY, Ratio::to_f64)
    }
}

/// **Every constrainable member a stage has, as tooth counts** — the companion
/// of [`super::member_inputs`], and the argument [`Wiring::alone`] wants.
pub(crate) fn teeth_of<'a>(members: impl IntoIterator<Item = &'a StageGear>) -> Vec<u32> {
    members.into_iter().map(|g| g.teeth).collect()
}
