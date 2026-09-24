//! **The graph a train is, and every preset a tick pattern of** — axes,
//! bodies on them, members on the bodies, meshes between members, and one
//! distance per pair of axes that mesh.
//!
//! A spur pair is two axes fixed in ground with one mesh between them. A
//! planetary set is a central axis and a planet axis carried by a body on
//! the central one, replicated `N` times, with two meshes on the one
//! distance between the axes. A hula is the same with `N = 1`, both meshes
//! internal and a compound planet. A layshaft transmission is two ground
//! axes with several meshes on one distance. None of these is a type here:
//! the shape says which frames there are, and everything else — the wiring
//! the kinematics reads, the closure the shifts obey, where the power goes —
//! is derived from it once. A train is one of these, and so is each part it
//! falls into ([`Shape::parts`]), which is what the solve closes, searches
//! and rates.
//!
//! # One distance per pair of axes
//!
//! Every mesh between a member on axis `A` and a member on axis `B` runs at
//! the distance between those axes, in the frame both stand still in. That is
//! the law a planet's carrier radius obeys and a hula's crank offset, and it
//! is not an epicyclic law: a layshaft's pairs obey it in ground. Automatic, the distance is what
//! the shifts leave and every mesh past the first has one automatic shift
//! that *absorbs* the difference; given, every mesh on it has a shift sum to
//! reach ([`crate::mesh::shift_sum_for`]).
//!
//! # What a member owns, what a mesh owns, what an axis pair owns
//!
//! A member has one module and one tooth-thickness coefficient, so they are
//! its ([`Member`]); a mesh owns its friction, since two meshes on one member
//! can be lubricated differently ([`MeshInput`]); an axis pair owns its
//! distance, the clearance that opens it, the tolerances on it and the axial
//! float along it ([`Distance`]). A ring is a member cut by a pinion cutter
//! rather than a rack, and a mesh is internal exactly where one of its two
//! members is one — the sign of a mesh is derived from its members, never
//! stated.

use super::wiring::{BodyLabel, MeshSpec, Mount, Wiring};
use super::{
    ContactRatios, Freedom, FreedomGroup, GearResult, Loading, MemberFacts, MemberFreedom,
    MemberGear, MemberRating, MeshReport, Ports, Reading, TrainError, PROBE,
};
use crate::contact::{efficiency, ContactPath, Directional, Drive, LoadSharing};
use crate::kinematics::{Body, GROUND};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{operating_geometry, shift_sum_for, Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::plane::BasicRack;
use crate::ring::{Cutter, Ring};
use crate::screw::{CrossedPath, Screw, ScrewParams};
use crate::strength::{bending_stress, contact_stress, Load, RootStressModel, PARALLEL_AXES};
use crate::tooth::Tooth;

/// An axis gears turn about: fixed in ground, or carried round another axis
/// by a body — a planet's, riding the carrier.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Axis {
    /// The body whose frame this axis stands still in: a carrier, or
    /// **ground** (body 0) for an axis fixed in it — a spur pair's axes
    /// are carried by ground, which is what makes a pair the epicyclic
    /// family with its carrier held. A train body, not a slot. Absent in a
    /// file, ground — and `null`
    /// too, which is how a browser's stored train wrote it when this was an
    /// `Option`, so that state keeps loading.
    #[cfg_attr(feature = "serde", serde(default, deserialize_with = "ground_if_null"))]
    pub carried_by: Body,
    /// How many times this axis, its bodies and their gears are replicated
    /// about the axis it is carried round — `N` planets. One elsewhere.
    pub count: u32,
    /// **The least tip-to-tip gap between neighbouring instances**, mm —
    /// read only where the axis is replicated, and reported against the
    /// closest pair of this axis's planets (`LayoutReport`). An axis's own
    /// since two planet axes on one carrier can run at different radii and
    /// be allowed different gaps; it was the stage's until the stage went.
    /// Absent in a file, three tenths of a millimetre.
    #[cfg_attr(feature = "serde", serde(default = "default_planet_clearance"))]
    pub min_planet_clearance: f64,
}

/// The gap a replicated axis a file does not give one is held to.
pub(crate) fn default_planet_clearance() -> f64 {
    0.3
}

/// The crate's pressure angle, for a member a file does not give one:
/// [`GearParams`]'s, said once — and followed from its group rather than
/// stated, since a file that says nothing states nothing.
#[cfg(feature = "serde")]
fn default_pressure_angle() -> Auto<f64> {
    Auto::automatic(GearParams::default().pressure_angle)
}

impl Member {
    /// **The normal module this member is cut at** — its mesh group's,
    /// once the shape has shared it ([`Shape::share`]); the solve shares
    /// before it reads.
    #[must_use]
    pub fn normal_module(&self) -> f64 {
        self.module.manual
    }

    /// The normal pressure angle this member is cut at, degrees, as
    /// [`Self::normal_module`].
    #[must_use]
    pub fn normal_pressure_angle(&self) -> f64 {
        self.pressure_angle.manual
    }
}

/// `carried_by` as a stored train wrote it while it was an `Option`: `null`
/// reads as ground.
#[cfg(feature = "serde")]
fn ground_if_null<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Body, D::Error> {
    let s: Option<Body> = serde::Deserialize::deserialize(d)?;
    Ok(s.unwrap_or(GROUND))
}

/// A body of the train on one of the shape's axes. The `i`th listed is
/// slot `i + 1` of the wiring; ground is slot 0 and is not listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct BodyOn {
    /// The train's body — one number across the train, ground being 0 —
    /// that turns on this axis. Its position in the list is the shape's
    /// own numbering of it for the kinematics, ground 0 and the first
    /// listed 1.
    pub body: usize,
    pub axis: usize,
}

/// A gear, on a body.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Member {
    /// The train's body it spins with, one of the shape's [`BodyOn`]s.
    pub body: usize,
    pub gear: MemberGear,
    /// **Normal module, mm — given on one member of a mesh group, and
    /// followed by the rest.** Every mesh a member is in shares it, so the
    /// members a run of meshes joins ([`Shape::mesh_groups`]) are cut at one
    /// module and two groups may differ. It is the helix's rule: at most one
    /// member of the group states it, relief keeps it so, and where none
    /// does the group's first member's box stands ([`Shape::share`]). Read
    /// through [`Member::normal_module`].
    pub module: Auto<f64>,
    /// **Normal pressure angle, degrees**, by the module's rule — a tooth is
    /// cut at one angle, so a mesh group has one. Absent in a file,
    /// following its group at 20°. Read through
    /// [`Member::normal_pressure_angle`].
    #[cfg_attr(feature = "serde", serde(default = "default_pressure_angle"))]
    pub pressure_angle: Auto<f64>,
    /// Tooth-thickness coefficient, `k`: above 1 this gear's teeth thicken.
    /// **Given on one member of a mesh and automatic on the other**, which
    /// follows the mesh's rule — the two sum to 2 across an external mesh, a
    /// ring takes its pinion's — and relief keeps it so
    /// ([`Shape::thickness_mods`]). Every member automatic is `k = 1`.
    pub thickness_mod: Auto<f64>,
    /// **A ring is a gear cut by a pinion cutter.** `Some` makes this member
    /// internal, and every mesh it is in an internal one.
    pub ring: Option<Cutter>,
    /// The pitch diameter as a reading of the helix — `cos β = z m_n / d` —
    /// which is a worm's way of stating its size.
    pub pitch_diameter: Auto<f64>,
}

/// Two members in mesh, and what the mesh owns.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MeshInput {
    /// Indices into [`Shape::members`]. On an internal mesh `b` is the ring.
    pub a: usize,
    pub b: usize,
    pub sliding_friction: f64,
    pub static_friction: f64,
    /// **The axial contact ratio** `ε_β` this mesh is asked for — a floor
    /// under an automatic face width, or, with every width of its mesh
    /// group given, the thing that decides the group's helix (the group's
    /// first mesh's is the reading; the rest of the group carry the same
    /// number, written by the panel). Absent in a file, automatic at one.
    #[cfg_attr(feature = "serde", serde(default = "default_overlap"))]
    pub overlap: Auto<f64>,
    /// **The transverse contact ratio the efficiency search may not take
    /// this mesh below** ([`super::DEFAULT_MIN_CONTACT_RATIO`] where a file
    /// gives none) — each mesh's own, since a pair that must stay
    /// continuous by more than its neighbour should not have its neighbour
    /// held to the same. Bounds the optimiser only.
    #[cfg_attr(feature = "serde", serde(default = "default_min_contact_ratio"))]
    pub min_contact_ratio: f64,
    /// **How the load is shared between tooth pairs in contact** on this
    /// mesh — a model of one contact, so a mesh's own: two meshes on one
    /// member can be rated under different ones. Absent in a file, none,
    /// which is what every file written before it meant.
    #[cfg_attr(feature = "serde", serde(default))]
    pub load_sharing: LoadSharing,
    /// **Choose this mesh's automatic shifts for efficiency** rather than
    /// for undercut, the undercut shift then a floor rather than the answer
    /// (docs/reference.md#efficiency-parallel-axes).
    ///
    /// **The search's unit is not the mesh but its component** — the meshes
    /// a free member is shared between, and every mesh on an automatic
    /// distance an absorber ties together ([`Shape::search_components`]):
    /// a planet's shift moves both its meshes. So a component is searched
    /// where *any* of its meshes asks, and a component none of whose
    /// meshes asks keeps its undercut shifts. What is given constrains the
    /// search rather than being overruled by it. Absent in a file, not
    /// asked.
    #[cfg_attr(feature = "serde", serde(default))]
    pub search: bool,
}

/// The floor a mesh a file does not give one is held to by the search.
pub(crate) fn default_min_contact_ratio() -> f64 {
    super::DEFAULT_MIN_CONTACT_RATIO
}

/// The overlap a mesh a file does not give one runs at: asked for
/// automatically, at one.
pub(crate) fn default_overlap() -> Auto<f64> {
    Auto::automatic(1.0)
}

/// The distance between two axes, in the frame both stand still in, and what
/// goes with it.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Distance {
    pub axes: [usize; 2],
    /// The angle between the two axes, degrees: 0 for parallel, 90 for a
    /// worm and its wheel. A mesh on crossed axes is a point contact with a
    /// model of its own ([`super::crossed`]).
    pub angle: f64,
    /// **Size the two members as a worm and its wheel** — the worm's length
    /// and the wheel's face from the conventional proportions — rather than
    /// as two crossed helical gears. A preset's word, and an input because
    /// the recommendation is one a designer takes or leaves.
    pub worm: bool,
    /// The running distance, mm: automatic is whatever the shifts leave,
    /// opened by the clearance.
    pub distance: Auto<f64>,
    /// The assembly clearance every mesh on this pair runs with.
    pub clearance: Auto<f64>,
    /// **The least far-side tip gap an internal mesh on this distance may
    /// run at**, mm — the gap between the pinion's tip and the ring's on
    /// the side away from contact, which at a few teeth of difference is
    /// what sets the distance. Read while the distance is automatic: an
    /// automatic distance is what the shifts leave *or* what the tips need,
    /// whichever is larger, and the shifts then reach it. A given distance
    /// leaves whatever gap it leaves, reported and not asked for.
    #[cfg_attr(feature = "serde", serde(default))]
    pub tip_clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// Axial float of the first axis's members, mm — a rigid slide that
    /// opens one flank as far as it closes the other on a helical mesh.
    pub axial_clearance: f64,
}

/// **The graph**: a train's, a part's, or a preset's before it is laid in.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Shape {
    pub axes: Vec<Axis>,
    /// **The bodies on the shape's axes**, each once, in the shape's order —
    /// which is the order its slots count in. The train numbers bodies once,
    /// and the members and carriers name them; a part lists the bodies it
    /// has, and a body two parts share is listed in each.
    pub bodies: Vec<BodyOn>,
    pub members: Vec<Member>,
    pub meshes: Vec<MeshInput>,
    pub distances: Vec<Distance>,
    /// **Offset couplings**: two bodies on parallel axes that turn as one —
    /// the pins that take a cycloidal disc's rotation off to a shaft on the
    /// centre line, an Oldham coupling, a Schmidt coupling. No geometry and
    /// no play: a row in the motion, `ω_a = ω_b`, and a lossless way through
    /// the flow. What lets an orbiting body drive a shaft that does not
    /// orbit, and never what a shape has to have: one is added and taken
    /// away like a step ([`super::Edit::Couple`], and a removal).
    #[cfg_attr(feature = "serde", serde(default))]
    pub couplings: Vec<[usize; 2]>,
}

/// **The empty shape** — what the builder starts from. Every default a
/// piece takes is the piece's own now: a mesh's sharing and search, an
/// axis's planet gap.
impl Default for Shape {
    fn default() -> Self {
        Self {
            axes: Vec::new(),
            bodies: Vec::new(),
            members: Vec::new(),
            meshes: Vec::new(),
            distances: Vec::new(),
            couplings: Vec::new(),
        }
    }
}

// ------------------------------------------------------------ the shape ---

impl Shape {
    /// **A body's slot in this shape**, where the shape has it — its own
    /// numbering of the bodies on its axes, ground 0 and the first listed 1,
    /// which is what its kinematics and its conventions count in.
    ///
    /// The one lookup: [`Self::slot`] reads ground for a body the shape
    /// does not have, which is what a frame wants; this says which, which
    /// is what a case naming a body wants.
    pub(crate) fn slot_if_any(&self, body: usize) -> Option<Body> {
        self.bodies
            .iter()
            .position(|b| b.body == body)
            .map(|i| i + 1)
    }

    /// As [`Self::slot_if_any`], ground for a body the shape does not have.
    pub(crate) fn slot(&self, body: usize) -> Body {
        self.slot_if_any(body).unwrap_or(GROUND)
    }

    /// The train's body at one of the shape's slots; ground at 0.
    pub(crate) fn body_at(&self, slot: Body) -> usize {
        if slot == GROUND {
            GROUND
        } else {
            self.bodies[slot - 1].body
        }
    }

    /// **Whether a member is a worm's thread**: the first member of a mesh
    /// across a distance sized as a worm drive — which is a thread with
    /// proportions of its own, and no gear a gear tab can hold; its wheel
    /// is one.
    #[must_use]
    pub fn is_worm_thread(&self, member: usize) -> bool {
        self.meshes.iter().enumerate().any(|(k, m)| {
            m.a == member && self.distance_of(k).is_some_and(|d| self.distances[d].worm)
        })
    }

    /// The slot a member spins with.
    pub(crate) fn slot_of_member(&self, member: usize) -> Body {
        self.slot(self.members[member].body)
    }

    /// The axis a slot turns about — a slot, not a body: a body's is
    /// `axis_of_slot(slot(body))`.
    pub(crate) fn axis_of_slot(&self, shaft: Body) -> Option<usize> {
        (shaft != GROUND).then(|| self.bodies[shaft - 1].axis)
    }

    /// The slot that carries an axis, where a body does.
    fn carrier_slot(&self, axis: usize) -> Body {
        self.slot(self.axes[axis].carried_by)
    }

    /// The largest body number the shape names, ground where it names
    /// none — what a fresh body is numbered after.
    #[must_use]
    pub fn max_body(&self) -> usize {
        self.bodies.iter().map(|b| b.body).max().unwrap_or(GROUND)
    }

    /// **Every body renumbered** by `map` — the bodies on the axes, the
    /// members' and the carriers' — as the train renumbers when a body goes
    /// or a preset's bodies are given train numbers.
    pub fn renumber_bodies(&mut self, map: impl Fn(usize) -> usize) {
        for b in &mut self.bodies {
            b.body = map(b.body);
        }
        for m in &mut self.members {
            m.body = map(m.body);
        }
        for a in &mut self.axes {
            if a.carried_by != GROUND {
                a.carried_by = map(a.carried_by);
            }
        }
        for c in &mut self.couplings {
            *c = c.map(&map);
        }
    }

    /// **The frame a member's axis stands still in for meshing purposes**:
    /// its carrier where it rides one; where it is central, the carrier of
    /// the planets it meshes — a sun's or a ring's, whose axis that carrier
    /// turns about — and ground otherwise ([`super::wiring`]).
    ///
    /// **Asked of the member's meshes, not of its axis.** It was the first
    /// carrier on the member's axis, which inside one preset is the same
    /// answer — every central member of a set meshes its planets — and on
    /// a graph is not: a gear on a sun's shaft meshing a pinion on a fixed
    /// axis turns about the same line as the carrier and meshes in ground.
    pub(crate) fn frame_of_member(&self, member: usize) -> Body {
        let shaft = self.slot_of_member(member);
        let Some(axis) = self.axis_of_slot(shaft) else {
            return GROUND;
        };
        let c = self.carrier_slot(axis);
        if c != GROUND {
            return c;
        }
        self.meshes
            .iter()
            .filter_map(|m| {
                (m.a == member)
                    .then_some(m.b)
                    .or((m.b == member).then_some(m.a))
            })
            .find_map(|mate| {
                let carrier = self.carrier_slot(self.axis_of_slot(self.slot_of_member(mate))?);
                (carrier != GROUND && self.axis_of_slot(carrier) == Some(axis)).then_some(carrier)
            })
            .unwrap_or(GROUND)
    }

    /// Whether a body's axis is one of `N` alike.
    fn replicated(&self, shaft: Body) -> bool {
        self.axis_of_slot(shaft)
            .is_some_and(|a| self.axes[a].count > 1)
    }

    /// How many instances of a body's axis there are.
    fn count_of(&self, shaft: Body) -> u32 {
        self.axis_of_slot(shaft)
            .map_or(1, |a| self.axes[a].count.max(1))
    }

    /// A mesh's kind, from its members: internal where exactly one is a
    /// ring, on the side of the ring. A ring on crossed shafts is no mesh
    /// the screw model has: both its flanks are involute helicoids on
    /// cylinders, and a ring's is inside one.
    pub(crate) fn kind_of(&self, mesh: usize) -> Option<MeshKind> {
        let m = self.meshes[mesh];
        match (
            self.members[m.a].ring.is_some(),
            self.members[m.b].ring.is_some(),
        ) {
            (false, false) => Some(MeshKind::External),
            (false, true) if self.shaft_angle_of(mesh) == 0.0 => Some(MeshKind::Internal),
            _ => None,
        }
    }

    /// The angle between a mesh's two axes, degrees — nought on a
    /// parallel-axis mesh, which is the line contact; anything else is the
    /// point contact of crossed-axis screw gearing.
    fn shaft_angle_of(&self, mesh: usize) -> f64 {
        self.distance_of(mesh)
            .map_or(0.0, |d| self.distances[d].angle)
    }

    /// Whether a mesh is the point contact of crossed shafts.
    pub(crate) fn is_crossed(&self, mesh: usize) -> bool {
        self.shaft_angle_of(mesh) != 0.0
    }

    /// **The screw gearing a crossed mesh is**, at these shifts and helices:
    /// the first member's size read from its helix, the shifts entering as a
    /// rack's do with each member's thickness modification as an equivalent
    /// shift ([`ScrewParams::profile_shifts`]).
    fn screw_of(&self, mesh: usize, shifts: &[f64], helix: &[f64]) -> Result<Screw, TrainError> {
        let m = self.meshes[mesh];
        // Caught here rather than in `Screw::new`, because by then the helix
        // angle has become a diameter and the information is gone: `cos 90°`
        // is 6e-17, not zero, so the diameter comes out enormous rather than
        // infinite and passes every finiteness check downstream.
        if helix[m.a].abs() >= 90.0 {
            return Err(TrainError::Screw(
                crate::screw::ScrewError::FirstMemberIsADisc,
            ));
        }
        let module = self.members[m.a].normal_module();
        // Two gears in mesh share a normal module, on crossed shafts as on
        // parallel ones — where `Mesh::new` asks it of the racks.
        // ...and a pressure angle, which `Mesh::new` asks of a line contact's
        // racks and this asks here.
        let pressure_angle = self.members[m.a].normal_pressure_angle();
        if (self.members[m.b].normal_module() - module).abs() > crate::params::compat::SAME_RACK
            || (self.members[m.b].normal_pressure_angle() - pressure_angle).abs()
                > crate::params::compat::SAME_RACK
        {
            return Err(TrainError::Mesh(crate::mesh::MeshError::Incompatible));
        }
        let eff = |i: usize| shifts[i] + self.base_params(i, helix).thickness_shift();
        Screw::new(&ScrewParams {
            normal_module: module,
            normal_pressure_angle_rad: pressure_angle.to_radians(),
            shaft_angle_rad: self.shaft_angle_of(mesh).to_radians(),
            starts: self.members[m.a].gear.teeth,
            wheel_teeth: self.members[m.b].gear.teeth,
            worm_pitch_diameter: f64::from(self.members[m.a].gear.teeth.max(1)) * module
                / helix[m.a].to_radians().cos(),
            profile_shifts: [eff(m.a), eff(m.b)],
        })
        .map_err(TrainError::Screw)
    }

    /// **The effective shift sum that puts a mesh at `nominal`** — the
    /// parallel mesh's involute relation, or the crossed mesh's rack law.
    /// `None` where no sum reaches it.
    fn shift_sum_reaching(&self, mesh: usize, nominal: f64, helix: &[f64]) -> Option<f64> {
        if self.is_crossed(mesh) {
            // At zero shift, which is the only thing the reference depends on.
            let zero = vec![0.0; self.members.len()];
            let s = self.screw_of(mesh, &zero, helix).ok()?;
            let m = self.meshes[mesh];
            let sum = (nominal - s.reference_distance) / self.members[m.a].normal_module();
            return sum.is_finite().then_some(sum);
        }
        let rack = self.rack_of(mesh, helix);
        shift_sum_for(
            rack.mt,
            rack.alpha_t,
            rack.alpha_n,
            self.tooth_sum(mesh),
            nominal,
        )
    }

    /// The distance a mesh runs at — the entry for its two axes, either way
    /// round.
    pub(crate) fn distance_of(&self, mesh: usize) -> Option<usize> {
        let m = self.meshes[mesh];
        let (a, b) = (
            self.axis_of_slot(self.slot_of_member(m.a))?,
            self.axis_of_slot(self.slot_of_member(m.b))?,
        );
        self.distances
            .iter()
            .position(|d| d.axes == [a, b] || d.axes == [b, a])
    }

    /// The meshes on one distance, in order.
    pub(crate) fn meshes_on(&self, distance: usize) -> Vec<usize> {
        (0..self.meshes.len())
            .filter(|&m| self.distance_of(m) == Some(distance))
            .collect()
    }

    /// **Who can absorb for a later mesh on a distance**, in order of
    /// preference: the members of the first mesh and of this one with
    /// leverage on the difference between the two — a shift moves a
    /// distance one way on an external mesh and the other on an internal
    /// one, so a planet between a sun and a ring moves the two apart at
    /// twice the rate while a planet between two rings moves them together
    /// and closes nothing — the most leverage first and, among equals, the
    /// later mesh's own member; never one in a mesh already closed
    /// (`closed`, the first mesh among them counting only while it is
    /// alone), which is what solving the constraints one after another
    /// relies on. The plan takes the first of these that is free; relief
    /// declares them in this order so what it leaves automatic is one the
    /// plan can use.
    fn absorbers(&self, first: usize, m: usize, closed: &[usize]) -> Vec<usize> {
        let MeshInput { a, b, .. } = self.meshes[m];
        let MeshInput { a: fa, b: fb, .. } = self.meshes[first];
        let lever = |i: usize, mesh: usize| -> f64 {
            let mm = self.meshes[mesh];
            let sign = self.kind_of(mesh).map_or(1.0, MeshKind::sign);
            let c = f64::from(u8::from(mm.a == i)) + if mm.b == i { sign } else { 0.0 };
            c * self.tooth_sum(mesh).signum()
        };
        let undisturbed = |i: usize| {
            closed.iter().all(|&q| {
                q == m
                    || (q == first && closed.len() == 1)
                    || (self.meshes[q].a != i && self.meshes[q].b != i)
            })
        };
        // The later mesh's own members first, its first before its second,
        // then the first mesh's — the order the plan always preferred among
        // equals.
        let mut out: Vec<(usize, f64)> = Vec::new();
        for i in [a, b, fa, fb] {
            let leverage = (lever(i, first) - lever(i, m)).abs();
            if undisturbed(i) && leverage > 0.0 && !out.iter().any(|x| x.0 == i) {
                out.push((i, leverage));
            }
        }
        // Stable: the later mesh's own members stay first among equals.
        out.sort_by(|p, q| q.1.total_cmp(&p.1));
        out.into_iter().map(|(i, _)| i).collect()
    }

    /// **The running distance the shifts are asked to reach**: given where
    /// both the distance and the clearance are stated, so the shifts have a
    /// sum to close on; automatic otherwise.
    fn given_running(&self, distance: usize) -> Option<f64> {
        let d = &self.distances[distance];
        (!d.distance.auto && !d.clearance.auto).then_some(d.distance.manual)
    }

    /// **The running distance the meshes are built at**: the one in the box
    /// wherever it is given — with the clearance automatic the shifts stand
    /// where they were asked and the clearance is whatever the distance
    /// leaves, which is the pair's reading and now every shape's — and the
    /// first mesh's zero-backlash distance opened by the clearance otherwise.
    fn running_target(&self, distance: usize) -> Option<f64> {
        let d = &self.distances[distance];
        (!d.distance.auto).then_some(d.distance.manual)
    }

    /// **Whether `count` identical planets on an axis can be assembled
    /// equally spaced**, and whether they all mesh in the same phase.
    ///
    /// `None` where the rule below does not reach: an axis with a mesh to
    /// another replicated axis, whose phase is not a central member's.
    ///
    /// # The rule
    ///
    /// A mesh between a central member `c` and a gear `p` on the axis is
    /// the phase relation `z_c(θ_c − φ) + z_p(ψ − φ) ≡ K (mod 2π)`, the
    /// kinematic row integrated, with the counts signed as the rows sign
    /// them (a ring's negative), `φ` the planet's place round the carrier
    /// and `ψ` its own turn. With every central member held at `θ_c = 0`
    /// and the planets at `φ_j = 2πj/N`, two meshes `i, i'` on one axis
    /// each fix `ψ_j` up to a whole tooth of their own gear, and the two
    /// agree exactly when
    ///
    /// ```text
    /// j (z_ci z_pi' − z_ci' z_pi) / N  ∈  z_pi ℤ + z_pi' ℤ  =  gcd(z_pi, z_pi') ℤ
    /// ```
    ///
    /// for every `j` — that is, `N · gcd(z_pi, z_pi')` divides
    /// `z_ci z_pi' − z_ci' z_pi`. On a simple planet, `z_pi = z_pi'`, this
    /// is the textbook `N | z_s + z_r`; on a stepped planet it is
    /// `N · gcd(z_p1, z_p2) | z_s z_p2 + z_r z_p1`. Simultaneous meshing —
    /// every planet in the same phase — is `N | z_c` for every central
    /// member the axis meets. `assembly` in this module's tests holds the
    /// rule to a search over the phases that shares none of it.
    #[must_use]
    pub fn assembly(&self, axis: usize) -> Option<(bool, bool)> {
        let count = self.axes.get(axis)?.count;
        // Each mesh from this axis to a central member: (signed central
        // count, the axis gear's count).
        let mut pairs: Vec<(i64, i64)> = Vec::new();
        for (k, m) in self.meshes.iter().enumerate() {
            let on = |i: usize| self.axis_of_slot(self.slot_of_member(i)) == Some(axis);
            let (planet, central) = match (on(m.a), on(m.b)) {
                (true, false) => (m.a, m.b),
                (false, true) => (m.b, m.a),
                _ => continue,
            };
            // A central member that is itself carried is another planet's,
            // and the rule does not reach it.
            if self
                .axis_of_slot(self.slot_of_member(central))
                .is_some_and(|a| self.axes[a].carried_by != GROUND)
            {
                return None;
            }
            let sign = match self.kind_of(k)? {
                MeshKind::External => 1,
                MeshKind::Internal => -1,
            };
            pairs.push((
                sign * i64::from(self.members[central].gear.teeth),
                i64::from(self.members[planet].gear.teeth),
            ));
        }
        if pairs.len() < 2 {
            return None;
        }
        let n = i64::from(count);
        let gcd = |mut a: i64, mut b: i64| {
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a.abs()
        };
        let mut equal = true;
        for (i, &(c1, p1)) in pairs.iter().enumerate() {
            for &(c2, p2) in &pairs[i + 1..] {
                equal &= (c1 * p2 - c2 * p1) % (n * gcd(p1, p2)) == 0;
            }
        }
        let simultaneous = pairs.iter().all(|&(c, _)| c % n == 0);
        Some((equal, simultaneous))
    }

    /// **What each member is, read off the shape** — the one rule, for the
    /// harness's English and the panel's catalogue alike. A ring is a member
    /// with a cutter; a planet is one on a carried axis; a sun meets a planet
    /// from an axis that is not carried; a worm and its wheel are the two
    /// ends of the first mesh on a distance marked as a worm drive; anything
    /// else is a gear that goes by its number. Numbered where a role is
    /// shared — a Wolfrom's two rings, a Ravigneaux's two suns, a hula's
    /// two wobble gears — by the order the shape lists them.
    #[must_use]
    pub fn member_names(&self) -> Vec<MemberName> {
        let carried = |i: usize| self.is_planet_gear(i);
        let role = |i: usize| -> MemberRole {
            for (k, m) in self.meshes.iter().enumerate() {
                let worm = self
                    .distance_of(k)
                    .is_some_and(|d| self.distances[d].worm && self.meshes_on(d)[0] == k);
                if worm && m.a == i {
                    return MemberRole::Worm;
                }
                if worm && m.b == i {
                    return MemberRole::Wheel;
                }
            }
            if self.members[i].ring.is_some() {
                return MemberRole::Ring;
            }
            if carried(i) {
                return MemberRole::Planet;
            }
            let meets_a_planet = self
                .meshes
                .iter()
                .any(|m| (m.a == i && carried(m.b)) || (m.b == i && carried(m.a)));
            if meets_a_planet {
                MemberRole::Sun
            } else {
                MemberRole::Gear
            }
        };
        let roles: Vec<MemberRole> = (0..self.members.len()).map(role).collect();
        roles
            .iter()
            .enumerate()
            .map(|(i, &r)| {
                let alike: Vec<usize> = (0..roles.len()).filter(|&j| roles[j] == r).collect();
                MemberName {
                    role: r,
                    ordinal: (alike.len() > 1 && r != MemberRole::Gear)
                        .then(|| alike.iter().position(|&j| j == i).map_or(1, |p| p + 1)),
                }
            })
            .collect()
    }

    /// The label a wiring gives a body: a carrier where it carries an axis,
    /// the first member on it otherwise.
    fn label_of(&self, shaft: Body) -> BodyLabel {
        if shaft == GROUND {
            return BodyLabel::Ground;
        }
        let carriers: Vec<Body> = (1..=self.bodies.len())
            .filter(|&s| self.axes.iter().any(|a| self.slot(a.carried_by) == s))
            .collect();
        if let Some(index) = carriers.iter().position(|&c| c == shaft) {
            return BodyLabel::Carrier { index };
        }
        if let Some(member) = self.members.iter().position(|m| self.slot(m.body) == shaft) {
            return BodyLabel::Member { member };
        }
        self.couplings
            .iter()
            .find_map(|c| {
                let [a, b] = c.map(|x| self.slot(x));
                (a == shaft).then_some(b).or((b == shaft).then_some(a))
            })
            .map_or(BodyLabel::Bare, |to| BodyLabel::Coupled { to })
    }

    // ---------------------------------------------------------- helices ---

    /// **Every member's helix**, from what is stated: a member's own reading
    /// (its helix, or its pitch diameter), else its mate's through the mesh
    /// they share — the opposite hand across an external mesh, the same
    /// across an internal one — else straight teeth.
    pub(crate) fn helix_angles(&self) -> Vec<f64> {
        let readings = self.readings();
        // The last reading given is the one relief leaves standing, and the
        // one the solve honours.
        let stated = |i: usize| -> Option<f64> {
            readings.iter().rev().find_map(|r| match r.freedom {
                Freedom::Member(m, MemberFreedom::Helix | MemberFreedom::PitchDiameter)
                    if m == i =>
                {
                    r.helix
                }
                // A mesh's overlap reads the size of its first member.
                Freedom::Overlap(k) if self.meshes[k].a == i => r.helix,
                _ => None,
            })
        };
        let mut out: Vec<Option<f64>> = (0..self.members.len()).map(stated).collect();
        // **A given distance with both shifts pinned decides the size.** With
        // nothing else free to reach it, the helix of a mesh's first member
        // is solved so the zero-backlash distance opened by the clearance is
        // the one given — the pair's rule, on every parallel mesh.
        for (k, m) in self.meshes.iter().enumerate() {
            if out[m.a].is_some() || out[m.b].is_some() {
                continue;
            }
            let Some(d) = self.distance_of(k) else {
                continue;
            };
            let Some(kind) = self.kind_of(k) else {
                continue;
            };
            let sign = kind.sign();
            let angle = self.distances[d].angle;
            let both_pinned = [m.a, m.b]
                .iter()
                .all(|&i| !self.members[i].gear.profile_shift.auto);
            let target = self
                .given_running(d)
                .filter(|_| both_pinned)
                .map(|running| kind.nominal_of(running, self.distances[d].clearance.manual));
            // The zero-backlash distance at a helix of the first member, with
            // the shifts as the members will actually be cut at it — a typed
            // shift held to the undercut bound moves with the helix, and the
            // size must reach the distance with the shift it gets.
            let at = |beta: f64| -> f64 {
                let mut helix = vec![0.0; self.members.len()];
                helix[m.a] = beta;
                helix[m.b] = angle - sign * beta;
                let shifts: Vec<f64> = self.asked(&helix).iter().map(|a| a.settled).collect();
                self.nominal_of(k, &shifts, &helix).unwrap_or(f64::NAN)
            };
            let sized = match target {
                None => None,
                Some(target) if angle == 0.0 => {
                    // Straight teeth are the floor; the distance grows with
                    // the helix without bound below ninety degrees.
                    (at(0.0) <= target)
                        .then(|| {
                            crate::solve::brent(
                                |b| at(b) - target,
                                0.0,
                                89.0,
                                crate::solve::Tol::default(),
                            )
                        })
                        .flatten()
                }
                Some(target) => self.size_reaching(k, target, &at),
            };
            // A crossed mesh with nothing stating its size shares the body
            // angle evenly, which at a right angle is a 45°/45° crossed pair;
            // a parallel one has straight teeth.
            if let Some(beta) = sized.or_else(|| (angle != 0.0).then_some(angle / 2.0)) {
                out[m.a] = Some(beta);
                out[m.b] = Some(angle - sign * beta);
            }
        }
        // Propagate through the meshes until nothing moves.
        loop {
            let mut moved = false;
            for (k, m) in self.meshes.iter().enumerate() {
                let sign = self.kind_of(k).map_or(1.0, MeshKind::sign);
                let angle = self.shaft_angle_of(k);
                // `β_b = Σ − sign · β_a`: the opposite hand across an
                // external mesh, the same hand across an internal one, and
                // on crossed shafts the shaft angle shared between the two.
                match (out[m.a], out[m.b]) {
                    (Some(a), None) => {
                        out[m.b] = Some(angle - sign * a);
                        moved = true;
                    }
                    (None, Some(b)) => {
                        out[m.a] = Some(angle - sign * b);
                        moved = true;
                    }
                    _ => {}
                }
            }
            if !moved {
                break;
            }
        }
        out.into_iter().map(|h| h.unwrap_or(0.0)).collect()
    }

    /// **The first member's helix that puts a crossed mesh at `target`**,
    /// degrees — where one exists on the branch the member's own diameter
    /// is on. `at` is the mesh's zero-backlash distance at a helix.
    ///
    /// # Two answers, and the branch is chosen by continuity
    ///
    /// On crossed shafts the distance has a **minimum** in the first
    /// member's diameter ([`Screw::least_distance_lead_angle`]): steepening
    /// the thread shrinks the worm and grows the wheel, and past the turning
    /// point the second wins. So a target above the minimum is reached by
    /// two worms, and picking one is a decision rather than a calculation.
    /// It is taken **on the side the designer's own number is on** — the
    /// diameter in the box — which is the only choice under which nudging
    /// the target moves the answer smoothly instead of jumping between a
    /// thin fast worm and a fat slow one. A target *below* the minimum is
    /// reached by neither and there is no answer to give.
    fn size_reaching(&self, mesh: usize, target: f64, at: &dyn Fn(f64) -> f64) -> Option<f64> {
        let m = self.meshes[mesh];
        let a = &self.members[m.a];
        let z1 = f64::from(a.gear.teeth.max(1));
        let floor = z1 * a.normal_module();
        // The designer's own number, held to the tooth's own diameter below
        // which no pair exists.
        let from = a.pitch_diameter.manual.max(floor * 1.000_001);
        let sigma = self.shaft_angle_of(mesh).to_radians();
        let turning = Screw::least_distance_lead_angle(
            a.gear.teeth.max(1),
            self.members[m.b].gear.teeth,
            sigma,
        )
        .map(|least| floor / least.sin());
        // The diameter is the helix read the other way, and the search runs
        // over the diameter so the branch is the diameter's.
        let helix_of = |d1: f64| (floor / d1).clamp(-1.0, 1.0).acos().to_degrees();
        let distance = |d1: f64| at(helix_of(d1));
        // No upper bound in the geometry, so one is grown until it brackets
        // — the distance rises without bound on this branch, so it does.
        let grown = |from: f64| {
            let mut top = from * 2.0;
            for _ in 0..60 {
                if distance(top) >= target || !distance(top).is_finite() {
                    break;
                }
                top *= 2.0;
            }
            top
        };
        let (lo, hi) = match turning {
            Some(turning) if from <= turning => (floor * (1.0 + 1e-9), turning),
            Some(turning) => (turning, grown(turning.max(from))),
            None => (floor * (1.0 + 1e-9), grown(from.max(floor * 2.0))),
        };
        crate::solve::brent(
            |d1| distance(d1) - target,
            lo,
            hi,
            crate::solve::Tol::default(),
        )
        .map(helix_of)
    }

    /// The width a mesh group's overlap reads its size against: the least
    /// given width among its members.
    fn given_width(&self, group: &[usize]) -> f64 {
        group
            .iter()
            .map(|&i| self.members[i].gear.face_width.manual)
            .fold(f64::INFINITY, f64::min)
    }

    /// Whether a mesh group's overlap decides its helix: every width of
    /// the group given.
    fn overlap_reads_size(&self, group: &[usize]) -> bool {
        group.iter().all(|&i| !self.members[i].gear.face_width.auto)
    }

    /// **The meshes of each mesh group**, in the groups' order: the mesh's
    /// members are one group's, so the mesh is that group's.
    pub(crate) fn group_meshes(&self) -> Vec<Vec<usize>> {
        self.mesh_groups()
            .iter()
            .map(|g| {
                (0..self.meshes.len())
                    .filter(|&k| g.contains(&self.meshes[k].a))
                    .collect()
            })
            .collect()
    }

    /// The readings of mesh `k`'s group alone — the size entry its
    /// relation counts.
    fn readings_for(&self, k: usize) -> Vec<Reading> {
        let (group, meshes) = self.group_of_mesh(k);
        self.readings()
            .into_iter()
            .filter(|r| match r.freedom {
                Freedom::Member(i, _) => group.contains(&i),
                Freedom::Overlap(j) => meshes.contains(&j),
                _ => false,
            })
            .collect()
    }

    /// The mesh group a mesh belongs to, and that group's meshes.
    fn group_of_mesh(&self, k: usize) -> (Vec<usize>, Vec<usize>) {
        let groups = self.mesh_groups();
        let meshes = self.group_meshes();
        let g = groups
            .iter()
            .position(|g| g.contains(&self.meshes[k].a))
            .unwrap_or(0);
        (groups[g].clone(), meshes[g].clone())
    }

    // ----------------------------------------------------------- params ---

    /// A member's parameters at a shift, its helix as the readings decide.
    fn base_params(&self, i: usize, helix: &[f64]) -> GearParams {
        let m = &self.members[i];
        GearParams {
            angular_shift: 0.0,
            index_offset: 0.0,
            module: m.normal_module(),
            pressure_angle: m.normal_pressure_angle(),
            teeth: m.gear.teeth,
            helix_angle: helix[i],
            profile_shift: m.gear.profile_shift.manual,
            addendum: m.gear.addendum,
            dedendum: m.gear.dedendum,
            root_radius: m.gear.root_radius,
            thickness_mod: self.thickness_mods()[i],
        }
    }

    /// **Each member's thickness coefficient, the automatic ones following
    /// the given.** A mesh binds its two: across an external mesh they sum
    /// to 2, and a ring takes its pinion's — so a given `k` is propagated
    /// mesh by mesh, as the helix is, until nothing moves, and a member
    /// nothing reaches is the standard tooth, `k = 1`.
    #[must_use]
    pub fn thickness_mods(&self) -> Vec<f64> {
        let mut out: Vec<Option<f64>> = self
            .members
            .iter()
            .map(|m| (!m.thickness_mod.auto).then_some(m.thickness_mod.manual))
            .collect();
        loop {
            let mut moved = false;
            for (k, m) in self.meshes.iter().enumerate() {
                let mate = |x: f64| match self.kind_of(k) {
                    Some(MeshKind::Internal) => x,
                    _ => 2.0 - x,
                };
                match (out[m.a], out[m.b]) {
                    (Some(a), None) => {
                        out[m.b] = Some(mate(a));
                        moved = true;
                    }
                    (None, Some(b)) => {
                        out[m.a] = Some(mate(b));
                        moved = true;
                    }
                    _ => {}
                }
            }
            if !moved {
                break;
            }
        }
        out.into_iter().map(|k| k.unwrap_or(1.0)).collect()
    }

    /// **The mesh groups**: the connected components of the mesh graph, in
    /// member order — two gears in mesh share a normal module and a pressure
    /// angle, so everything a run of meshes joins does. One group for a pair
    /// or a set; two for a hula or a stepped planet, whose meshes do
    /// not join; three for a layshaft's three pairs. A **layer over the
    /// graph**, read off it and never stored: what a panel offers one box
    /// for and writes to every member of.
    #[must_use]
    pub fn mesh_groups(&self) -> Vec<Vec<usize>> {
        let n = self.members.len();
        let mut group: Vec<usize> = (0..n).collect();
        let find = |group: &Vec<usize>, mut i: usize| {
            while group[i] != i {
                i = group[i];
            }
            i
        };
        for m in &self.meshes {
            let (a, b) = (find(&group, m.a), find(&group, m.b));
            if a != b {
                group[a.max(b)] = a.min(b);
            }
        }
        let mut out: Vec<Vec<usize>> = Vec::new();
        for i in 0..n {
            let root = find(&group, i);
            match out.iter_mut().find(|g| find(&group, g[0]) == root) {
                Some(g) => g.push(i),
                None => out.push(vec![i]),
            }
        }
        out
    }

    /// The parameters a member builds at, at a shift: the addendum held to
    /// its tip width where that was asked.
    fn params_at(&self, i: usize, x: f64, helix: &[f64]) -> GearParams {
        let with_shift = GearParams {
            profile_shift: x,
            ..self.base_params(i, helix)
        };
        if self.members[i].ring.is_some() {
            return with_shift;
        }
        GearParams {
            addendum: self.members[i].gear.addendum_asked(&with_shift).used,
            ..with_shift
        }
    }

    /// What each member's shift asks, before anything constrains it: a ring
    /// is not asked about undercut and stands where it is put.
    fn asked(&self, helix: &[f64]) -> Vec<super::pair::ShiftAsked> {
        (0..self.members.len())
            .map(|i| {
                let m = &self.members[i];
                if m.ring.is_some() {
                    let given = (!m.gear.profile_shift.auto).then_some(m.gear.profile_shift.manual);
                    super::pair::ShiftAsked {
                        search_floor: None,
                        given,
                        settled: given.unwrap_or(0.0),
                        raised: false,
                    }
                } else {
                    m.gear.shift_asked(&self.base_params(i, helix))
                }
            })
            .collect()
    }

    /// The basic rack a mesh is cut with: both members' module and
    /// pressure angle (the first's, the second agreeing or the mesh
    /// refused), the mesh's helix.
    fn rack_of(&self, mesh: usize, helix: &[f64]) -> BasicRack {
        let m = self.meshes[mesh];
        BasicRack::new(
            self.members[m.a].normal_module(),
            self.members[m.a].normal_pressure_angle(),
            helix[m.a].abs(),
        )
    }

    /// The signed tooth sum of a mesh, a ring's negative.
    fn tooth_sum(&self, mesh: usize) -> f64 {
        let m = self.meshes[mesh];
        let sign = self.kind_of(mesh).map_or(1.0, MeshKind::sign);
        f64::from(self.members[m.a].gear.teeth) + sign * f64::from(self.members[m.b].gear.teeth)
    }

    /// The shift sum a mesh's operating distance is a function of: `x_a + x_b`
    /// external, `x_a − x_b` internal, each shift carrying its member's
    /// thickness modification as an equivalent shift.
    fn shift_sum(&self, mesh: usize, shifts: &[f64], helix: &[f64]) -> f64 {
        let m = self.meshes[mesh];
        let sign = self.kind_of(mesh).map_or(1.0, MeshKind::sign);
        let eff = |i: usize| shifts[i] + self.base_params(i, helix).thickness_shift();
        eff(m.a) + sign * eff(m.b)
    }

    /// The zero-backlash distance of a mesh at these shifts, closed form:
    /// the involute relation on parallel shafts, the rack law on crossed.
    fn nominal_of(&self, mesh: usize, shifts: &[f64], helix: &[f64]) -> Option<f64> {
        if self.is_crossed(mesh) {
            return self
                .screw_of(mesh, shifts, helix)
                .ok()
                .map(|s| s.centre_distance);
        }
        let rack = self.rack_of(mesh, helix);
        operating_geometry(
            rack.mt,
            rack.alpha_t,
            rack.alpha_n,
            self.tooth_sum(mesh),
            self.shift_sum(mesh, shifts, helix),
        )
        .map(|(_, _, a)| a)
    }

    /// The running distance of a mesh at these shifts: the zero-backlash one
    /// opened by its pair's clearance.
    fn running_of(&self, mesh: usize, shifts: &[f64], helix: &[f64]) -> Option<f64> {
        let d = self.distance_of(mesh)?;
        let kind = self.kind_of(mesh)?;
        Some(kind.run_at(
            self.nominal_of(mesh, shifts, helix)?,
            self.distances[d].clearance.manual,
        ))
    }

    // ---------------------------------------------------------- closure ---

    /// **Who decides each shift**, with every distance held that is stated.
    fn plan(&self, helix: &[f64]) -> Plan {
        let held: Vec<Option<f64>> = (0..self.distances.len())
            .map(|d| self.given_running(d))
            .collect();
        self.plan_held(helix, held)
    }

    /// [`Self::plan`] with the distances held as given — stated, or sized
    /// by the tips.
    fn plan_held(&self, helix: &[f64], held: Vec<Option<f64>>) -> Plan {
        let asked = self.asked(helix);
        let n = self.members.len();
        let in_meshes = |i: usize| self.meshes.iter().filter(|m| m.a == i || m.b == i).count();
        let mut role: Vec<Role> = (0..n)
            .map(|i| {
                if asked[i].given.is_some() {
                    Role::Given
                } else {
                    Role::Free
                }
            })
            .collect();
        let mut constraints: Vec<Constraint> = Vec::new();
        let mut reaches_in_order: Vec<(usize, usize)> = Vec::new();
        for (d, held_at) in held.iter().enumerate() {
            let meshes = self.meshes_on(d);
            let Some((&first, rest)) = meshes.split_first() else {
                continue;
            };
            if held_at.is_some() {
                // Every mesh has a sum to reach. A mesh with one free member
                // has it reach the sum — and a shared member reached that way
                // is then fixed for the next mesh, so those go first and
                // repeat until nothing moves. A mesh with both free has its
                // second member follow its first; a shared member left with
                // nothing to decide it stands where it was asked.
                let mut planned = vec![false; self.meshes.len()];
                loop {
                    let mut moved = false;
                    for &m in &meshes {
                        if planned[m] {
                            continue;
                        }
                        let MeshInput { a, b, .. } = self.meshes[m];
                        let free: Vec<usize> = [a, b]
                            .into_iter()
                            .filter(|&i| role[i] == Role::Free)
                            .collect();
                        if free.len() == 1 {
                            role[free[0]] = Role::Reaches(m);
                            reaches_in_order.push((free[0], m));
                            planned[m] = true;
                            moved = true;
                        } else if free.is_empty() {
                            planned[m] = true;
                        }
                    }
                    if !moved {
                        break;
                    }
                }
                for &m in &meshes {
                    if planned[m] {
                        continue;
                    }
                    let MeshInput { a, b, .. } = self.meshes[m];
                    // Both free: the one in more meshes stands, the other
                    // reaches; alike, the second follows the first.
                    let (stands, reaches) = if in_meshes(a) > in_meshes(b) {
                        (Some(a), b)
                    } else if in_meshes(b) > in_meshes(a) {
                        (Some(b), a)
                    } else {
                        (None, b)
                    };
                    if let Some(i) = stands {
                        role[i] = Role::Settled;
                    }
                    role[reaches] = Role::Reaches(m);
                    reaches_in_order.push((reaches, m));
                    planned[m] = true;
                }
            } else {
                // Each mesh past the first runs at the first's distance, and
                // one member absorbs the difference. **Which member can** is
                // a question of leverage, not of membership: a shift moves a
                // distance one way on an external mesh and the other on an
                // internal one, so a planet between a sun and a ring moves
                // the two apart at twice the rate while a planet between two
                // rings moves them together and closes nothing. The member
                // with the most leverage on the difference absorbs it, from
                // the later mesh by preference — and never one that would
                // disturb a mesh already closed, which is what solving the
                // constraints one after another relies on.
                let mut closed: Vec<usize> = vec![first];
                for &m in rest {
                    let absorber = self
                        .absorbers(first, m, &closed)
                        .into_iter()
                        .find(|&i| role[i] == Role::Free);
                    if let Some(i) = absorber {
                        role[i] = Role::Absorbs(constraints.len());
                    }
                    constraints.push(Constraint {
                        first,
                        mesh: m,
                        absorber,
                    });
                    closed.push(m);
                }
            }
        }
        Plan {
            asked,
            role,
            constraints,
            reaches: reaches_in_order,
            held,
        }
    }

    /// **The shifts every mesh on every distance agrees at**, given what
    /// the search set the free ones to — `free` being one value per
    /// member, read at the free ones, or empty for every free member at
    /// its floor. `None` where a sum cannot be reached or a distance cannot
    /// be closed.
    fn closed(&self, plan: &Plan, free: &[f64], helix: &[f64]) -> Option<Vec<f64>> {
        let n = self.members.len();
        let mut x: Vec<f64> = (0..n).map(|i| plan.asked[i].settled).collect();
        for (i, v) in x.iter_mut().enumerate() {
            match plan.role[i] {
                Role::Given => *v = plan.asked[i].given.unwrap_or(*v),
                Role::Free => *v = free.get(i).copied().unwrap_or(*v),
                Role::Settled | Role::Reaches(_) | Role::Absorbs(_) => {}
            }
        }
        // Sums on given distances, in the order they were planned.
        for &(i, m) in &plan.reaches {
            let d = self.distance_of(m)?;
            let running = plan.held[d]?;
            let kind = self.kind_of(m)?;
            let nominal = kind.nominal_of(running, self.distances[d].clearance.manual);
            let sum = self.shift_sum_reaching(m, nominal, helix)?;
            let MeshInput { a, b, .. } = self.meshes[m];
            let sign = kind.sign();
            let (ta, tb) = (
                self.base_params(a, helix).thickness_shift(),
                self.base_params(b, helix).thickness_shift(),
            );
            if i == b {
                if plan.role[a] == Role::Free && free.is_empty() {
                    // Nothing searched: divide the sum between the two, as
                    // a pair with both shifts automatic does. Where no
                    // admissible division reaches a distance the *designer*
                    // stated, that is the answer and the mesh says so
                    // (`distance_notes`); where the distance is one the
                    // tips sized, the first stands at its floor and the
                    // second takes the whole of the rest — what a hula's
                    // ring did for its crank: a shift past what its cutter
                    // reaches is a part that says so through its clamps,
                    // not a crank refused.
                    let floor = [plan.asked[a].search_floor, plan.asked[b].search_floor];
                    let at = |k: usize, v: f64| self.params_at([a, b][k], v, helix);
                    let sized = self.given_running(d).is_none();
                    match crate::auto::divide_shift_sum(&at, sign, sum - ta - sign * tb, floor) {
                        Some(both) => {
                            x[a] = both[0];
                            x[b] = both[1];
                        }
                        None if sized => x[b] = (sum - x[a] - ta) / sign - tb,
                        None => return None,
                    }
                } else {
                    x[b] = (sum - x[a] - ta) / sign - tb;
                }
            } else {
                x[a] = sum - sign * (x[b] + tb) - ta;
            }
        }
        // Absorbers on automatic distances, in order.
        for (k, c) in plan.constraints.iter().enumerate() {
            match c.absorber {
                Some(i) => {
                    x[i] = self.absorb(i, c.first, c.mesh, &x, helix)?;
                    debug_assert_eq!(plan.role[i], Role::Absorbs(k));
                }
                None => {
                    let (p, q) = (
                        self.running_of(c.first, &x, helix)?,
                        self.running_of(c.mesh, &x, helix)?,
                    );
                    if (p - q).abs() > 1e-9 * p.abs().max(1.0) {
                        return None;
                    }
                }
            }
        }
        Some(x)
    }

    /// The shift of member `i` at which mesh `m` runs at the distance mesh
    /// `first` runs at — a bracketed Newton, as `planetary::solve` absorbs
    /// into the planet.
    fn absorb(&self, i: usize, first: usize, m: usize, x: &[f64], helix: &[f64]) -> Option<f64> {
        let with = |v: f64| {
            let mut y = x.to_vec();
            y[i] = v;
            y
        };
        // The coefficient of `x_i` in each mesh's shift sum.
        let coefficient = |mesh: usize| -> f64 {
            let mm = self.meshes[mesh];
            let sign = self.kind_of(mesh).map_or(1.0, MeshKind::sign);
            f64::from(u8::from(mm.a == i)) + if mm.b == i { sign } else { 0.0 }
        };
        let slope = |mesh: usize, v: f64| -> Option<f64> {
            if self.is_crossed(mesh) {
                // The rack law: a module of distance per module of shift.
                return Some(coefficient(mesh) * self.members[self.meshes[mesh].a].normal_module());
            }
            let rack = self.rack_of(mesh, helix);
            let sum_z = self.tooth_sum(mesh);
            let (aw, _, a) = operating_geometry(
                rack.mt,
                rack.alpha_t,
                rack.alpha_n,
                sum_z,
                self.shift_sum(mesh, &with(v), helix),
            )?;
            Some(coefficient(mesh) * 2.0 * a * rack.alpha_n.tan() / (sum_z * aw.tan()))
        };
        let g = |v: f64| match (
            self.running_of(first, &with(v), helix),
            self.running_of(m, &with(v), helix),
        ) {
            (Some(p), Some(q)) => p - q,
            _ => f64::NAN,
        };
        let dg = |v: f64| match (slope(first, v), slope(m, v)) {
            (Some(p), Some(q)) => p - q,
            _ => f64::NAN,
        };
        // The bracket: where both meshes' operating geometry exists —
        // `Σx · sgn(Σz) ≥ −reach · |Σz|` for each, read in `x_i`.
        let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
        for mesh in [first, m] {
            let c = coefficient(mesh);
            if c == 0.0 || self.is_crossed(mesh) {
                continue;
            }
            let rack = self.rack_of(mesh, helix);
            let sum_z = self.tooth_sum(mesh);
            let reach = crate::inv(rack.alpha_t) / (2.0 * rack.alpha_n.tan());
            let rest = self.shift_sum(mesh, &with(0.0), helix);
            let bound = (-reach * sum_z.abs() * sum_z.signum() - rest) / c;
            if c * sum_z.signum() > 0.0 {
                lo = lo.max(bound);
            } else {
                hi = hi.min(bound);
            }
        }
        if !lo.is_finite() || !hi.is_finite() {
            let p = self.params_at(i, 0.0, helix);
            let range = crate::auto::admissible_ranges(&p, p.dedendum)
                .profile_shift
                .bound;
            lo = lo.max(range.min.unwrap_or(-5.0));
            hi = hi.min(range.max.unwrap_or(5.0));
        }
        if hi <= lo {
            return None;
        }
        let mid = 0.5 * (lo + hi);
        if !g(mid).is_finite() {
            return None;
        }
        let (lo, hi) = (pull_in(&g, lo, mid)?, pull_in(&g, hi, mid)?);
        crate::solve::newton_bracketed(
            g,
            dg,
            lo,
            hi,
            0.0_f64.clamp(lo, hi),
            crate::solve::Tol::default(),
        )
    }

    /// **How much room the tips on one distance have**, at these shifts, in
    /// the frame where zero is the bound: the least over its internal
    /// meshes of the far-side gap less what was asked, mm, and the room the
    /// tips have where their circles cross, degrees — two quantities, one
    /// sign, since only where the least of them turns positive is asked.
    /// `None` where the meshes cannot be built at all, and the mesh at the
    /// minimum beside the figure.
    fn tip_room(
        &self,
        distance: usize,
        x: &[f64],
        helix: &[f64],
        held: &[Option<f64>],
    ) -> Option<(f64, usize)> {
        // The parts alone, not the whole build: a trial distance on the way
        // to the one that clears may leave a mesh with no path of contact,
        // which is nothing to the question of where its tips stand.
        let meshes = self.meshes_on(distance);
        let running = match held.get(distance).copied().flatten() {
            Some(e) => e,
            None => self
                .running_target(distance)
                .or_else(|| self.running_of(*meshes.first()?, x, helix))?,
        };
        let asked = self.distances[distance].tip_clearance;
        meshes
            .into_iter()
            .filter(|&k| self.kind_of(k) == Some(MeshKind::Internal))
            .map(|k| {
                let m = self.meshes[k];
                let pinion = Tooth::new(self.params_at(m.a, x[m.a], helix));
                let ring = self.members[m.b]
                    .ring
                    .map(|cutter| Ring::cut_by(&self.params_at(m.b, x[m.b], helix), &cutter));
                // Both rooms rise with the distance: the pinion's tip on the
                // side away from contact stands `r_tip − e` from the ring's
                // centre, and the tips' room where their circles cross opens
                // as the pinion moves out.
                let room = ring.map_or(f64::INFINITY, |ring| {
                    super::TipRoom::at(&ring, &pinion, running).map_or(f64::NAN, |t| {
                        let crossing = if t.tip_interference {
                            -1.0
                        } else {
                            t.tip_margin
                        };
                        (t.far_gap - asked).min(crossing)
                    })
                });
                (room, k)
            })
            .min_by(|p, q| p.0.total_cmp(&q.0))
    }

    /// **The distances the tips size**, where an automatic one would run
    /// its internal meshes' tips into each other at what the shifts leave —
    /// a hula's two meshes at a tooth of difference, a planocentric's one.
    /// Each such distance opens out to the least at which every tip clears
    /// by what was asked, the room rising with the distance on every
    /// internal mesh, and the shifts then reach it as they reach a stated
    /// distance. Everything else stands as the plan had it.
    ///
    /// `free` is where the search has the free members, since which
    /// division a mesh's shift sum takes moves its tips a little.
    ///
    /// # Errors
    ///
    /// [`TrainError::TipsUnclearable`] where no distance in the involute
    /// domain clears the tips on some mesh.
    fn sized(&self, helix: &[f64], plan: &Plan, free: &[f64]) -> Result<TipSizing, TrainError> {
        let mut held = plan.held.clone();
        let mut bound_by = vec![None; self.distances.len()];
        for d in 0..self.distances.len() {
            if self.given_running(d).is_some()
                || !self
                    .meshes_on(d)
                    .iter()
                    .any(|&k| self.kind_of(k) == Some(MeshKind::Internal))
            {
                continue;
            }
            // **A gap asked for is a distance asked for**: the least at which
            // the tips clear by it, the shifts following — a hula's crank,
            // which the hula's own solver took from the clearance and nothing else.
            // With no gap asked the shifts' own distance stands, opened out
            // only where the tips would cross at it.
            let asked = self.distances[d].tip_clearance > 0.0;
            held[d] = None;
            // Held at `e`, the plan reaches it; the room at that plan's
            // closure is what is driven to zero.
            let room_at = |e: f64| -> f64 {
                let mut h = held.clone();
                h[d] = Some(e);
                let plan = self.plan_held(helix, h.clone());
                self.closed(&plan, free, helix)
                    .and_then(|x| self.tip_room(d, &x, helix, &h))
                    .map_or(f64::NAN, |(room, _)| room)
            };
            // At what the shifts leave, with nothing holding this distance
            // — or, where that is no mesh at all (a ring pinned low enough
            // that its pinion's floor puts the pair outside the involute
            // domain) and a gap was asked, from the domain's own floor.
            let loose = self.plan_held(helix, held.clone());
            let meshes = self.meshes_on(d);
            let mesh_0 = *meshes.first().unwrap_or(&0);
            let (from, room, mesh) = match self.closed(&loose, free, helix).and_then(|x| {
                let (room, mesh) = self.tip_room(d, &x, helix, &held)?;
                let from = self
                    .running_target(d)
                    .or_else(|| self.running_of(mesh_0, &x, helix))?;
                Some((from, room, mesh))
            }) {
                Some(found) => found,
                None if asked => {
                    let floor = meshes
                        .iter()
                        .map(|&k| {
                            let rack = self.rack_of(k, helix);
                            rack.mt * self.tooth_sum(k).abs() / 2.0 * rack.alpha_t.cos()
                        })
                        .fold(0.0_f64, f64::max)
                        * (1.0 + 1e-6);
                    let room = room_at(floor);
                    if room.is_nan() {
                        return Err(TrainError::TipsUnclearable { mesh: mesh_0 });
                    }
                    (floor, room, mesh_0)
                }
                None => continue,
            };
            if room >= 0.0 && !asked {
                continue;
            }
            // Bracket the root by growing steps — outward from a distance
            // the tips cross at, inward from one with room to spare — then
            // close in. Inward, the involute domain may end before the room
            // does; the shifts' own distance then stands, the tips asking
            // nothing of it.
            let (mut lo, mut hi) = (from, from);
            let mut step = from.abs().max(1.0) * 0.01;
            let mut found = false;
            if room < 0.0 {
                // No internal mesh runs more than a couple of reference
                // distances out — past that the pinion has left its ring —
                // so a walk that gets there has found that nothing clears.
                let reach = self.rack_of(mesh_0, helix).mt * self.tooth_sum(mesh_0).abs();
                for _ in 0..40 {
                    hi = lo + step;
                    if hi > from + reach {
                        break;
                    }
                    let r = room_at(hi);
                    if r.is_nan() {
                        // Past the involute domain on some mesh: come back in.
                        step *= 0.5;
                        continue;
                    }
                    if r >= 0.0 {
                        found = true;
                        break;
                    }
                    lo = hi;
                    step *= 1.5;
                }
                if !found {
                    return Err(TrainError::TipsUnclearable { mesh });
                }
            } else {
                for _ in 0..80 {
                    lo = hi - step;
                    if lo <= 0.0 {
                        break;
                    }
                    let r = room_at(lo);
                    if r.is_nan() {
                        // Below the involute domain on some mesh: come back
                        // out, by halves, until the step is nothing.
                        step *= 0.5;
                        if step < 1e-12 * from.abs().max(1.0) {
                            break;
                        }
                        continue;
                    }
                    if r < 0.0 {
                        found = true;
                        break;
                    }
                    hi = lo;
                    step *= 1.5;
                }
                if !found {
                    // The tips clear all the way to the domain's floor: they
                    // ask nothing of the distance, and the shifts' own stands.
                    continue;
                }
            }
            let e = crate::solve::brent(room_at, lo, hi, crate::solve::Tol::default())
                .ok_or(TrainError::TipsUnclearable { mesh })?;
            // Lean to the clear side of the root by the solver's own
            // tolerance, so the parts built at it have the room asked for.
            let e = if room_at(e) < 0.0 {
                hi.min(e + 1e-9)
            } else {
                e
            };
            held[d] = Some(e);
            bound_by[d] = self
                .closed(&self.plan_held(helix, held.clone()), free, helix)
                .and_then(|x| self.tip_room(d, &x, helix, &held))
                .map(|(_, k)| k);
        }
        Ok((held, bound_by))
    }

    /// **The shifts the shape settles on**: closed where nothing is searched,
    /// searched for efficiency over the free ones where that was asked —
    /// and, first, every automatic distance the tips size opened out to
    /// where they clear ([`Self::sized`]).
    fn chosen_at(&self, search: &crate::auto::Search, helix: &[f64]) -> Result<Chosen, TrainError> {
        let mut plan = self.plan(helix);
        let (held, mut bound_by) = self.sized(helix, &plan, &[])?;
        if held != plan.held {
            plan = self.plan_held(helix, held);
        }
        let settled: Vec<f64> = plan.asked.iter().map(|a| a.settled).collect();
        // **Where nothing was searched, the closure is the answer** — and
        // where it has none, what that means depends on what failed. A sum
        // on a given distance that no shift reaches leaves the shifts where
        // they were asked and the mesh saying it did not reach the distance
        // (`distance_notes`); an automatic distance that no absorber can
        // close is a set that cannot be assembled, and is refused for it.
        let fallback =
            |plan: &Plan, bound_by: &[Option<usize>], how| -> Result<Chosen, TrainError> {
                match self.closed(plan, &[], helix) {
                    Some(shifts) => Ok(Chosen {
                        shifts,
                        how,
                        held: plan.held.clone(),
                        bound_by: bound_by.to_vec(),
                    }),
                    None if plan.constraints.iter().any(|c| c.absorber.is_some()) => {
                        Err(TrainError::NoCommonDistance)
                    }
                    None => Ok(Chosen {
                        shifts: settled.clone(),
                        how,
                        held: plan.held.clone(),
                        bound_by: bound_by.to_vec(),
                    }),
                }
            };
        if !self.meshes.iter().any(|m| m.search) {
            return fallback(&plan, &bound_by, super::Searched::NotAsked);
        }
        // **Only the components that ask.** A free member in a component
        // none of whose meshes asks keeps its undercut shift; the rest are
        // searched as ever. Every mesh asking, or none, is what one switch on
        // the stage used to mean, and both come out as they did.
        let asking = self.asking_members(&plan);
        let free: Vec<usize> = (0..self.members.len())
            .filter(|&i| plan.role[i] == Role::Free && asking[i])
            .collect();
        if free.is_empty() {
            return fallback(&plan, &bound_by, super::Searched::NotAsked);
        }
        // Each free member's own interval.
        let Some(intervals) = free
            .iter()
            .map(|&i| {
                let p = self.params_at(i, 0.0, helix);
                if self.members[i].ring.is_some() {
                    let b = crate::auto::admissible_ranges(&p, p.dedendum)
                        .profile_shift
                        .bound;
                    let (lo, hi) = (b.min?, b.max?);
                    (lo < hi).then_some((lo, hi))
                } else {
                    crate::auto::searchable_shift(
                        &|x| self.params_at(i, x, helix),
                        plan.asked[i].search_floor,
                    )
                }
            })
            .collect::<Option<Vec<_>>>()
        else {
            return fallback(&plan, &bound_by, super::Searched::FoundNothing);
        };
        // **The sum and the division, not the two shifts**, wherever both
        // members of one mesh are free: the sum sets the operating pressure
        // angle and the length of the path, the division only moves its two
        // ends, and a search that moves one shift at a time can only
        // zig-zag up that diagonal (`auto::Pinned::place`). A member free on
        // its own keeps its own coordinate.
        let paired: Vec<(usize, usize, f64)> = self
            .meshes
            .iter()
            .enumerate()
            .filter_map(|(k, m)| {
                let (pa, pb) = (
                    free.iter().position(|&i| i == m.a)?,
                    free.iter().position(|&i| i == m.b)?,
                );
                Some((pa, pb, self.kind_of(k).map_or(1.0, MeshKind::sign)))
            })
            .collect();
        let mut used = vec![false; free.len()];
        let mut axes: Vec<Coordinate> = Vec::new();
        for (pa, pb, sign) in paired {
            if used[pa] || used[pb] {
                continue;
            }
            used[pa] = true;
            used[pb] = true;
            axes.push(Coordinate::Sum(pa, pb, sign));
            axes.push(Coordinate::Division(pa, pb, sign));
        }
        for (k, &u) in used.iter().enumerate() {
            if !u {
                axes.push(Coordinate::Own(k));
            }
        }
        let box_: Vec<(f64, f64)> = axes
            .iter()
            .map(|c| match *c {
                Coordinate::Own(k) => intervals[k],
                Coordinate::Sum(pa, pb, sign) => {
                    let (a, b) = (intervals[pa], intervals[pb]);
                    let (lo, hi) = (sign * b.0, sign * b.1);
                    (a.0 + lo.min(hi), a.1 + lo.max(hi))
                }
                Coordinate::Division(pa, pb, sign) => {
                    let (a, b) = (intervals[pa], intervals[pb]);
                    let (lo, hi) = (sign * b.0, sign * b.1);
                    (a.0 - lo.max(hi), a.1 - lo.min(hi))
                }
            })
            .collect();
        // The search's point as one value per member, the free ones set.
        let place = |v: &[f64]| -> Vec<f64> {
            let mut out = settled.clone();
            let mut k = 0;
            while k < axes.len() {
                match axes[k] {
                    Coordinate::Own(j) => {
                        out[free[j]] = v[k];
                        k += 1;
                    }
                    Coordinate::Sum(pa, pb, sign) => {
                        let (s, d) = (v[k], v[k + 1]);
                        out[free[pa]] = (s + d) / 2.0;
                        out[free[pb]] = (s - d) / (2.0 * sign);
                        k += 2;
                    }
                    Coordinate::Division(..) => unreachable!("a division follows its sum"),
                }
            }
            out
        };
        let cache: TeethCache = std::cell::RefCell::new(std::collections::HashMap::new());
        // **Meshes that share nothing are searched apart.** The objective is
        // a product over the meshes, and where a distance is held — stated
        // or sized — each mesh on it answers to its own members alone, so
        // the product is largest where every factor is and one search in
        // `n` variables is several in fewer: a hula's two meshes at a held
        // crank, which the hula's own solver searched one at a time for a thirteenth of
        // the work. Meshes on an automatic distance with more than one mesh
        // are one component, an absorber carrying any member's move across
        // it; and an axis that touches two meshes joins them.
        let components = self.search_components(&plan, &axes, &free);
        // **A sized distance and the divisions chosen at it settle
        // together.** The division a search chooses moves the tips a
        // little, so a distance the tips size is sized again at what was
        // chosen, and the search run again at that distance — the rounds the
        // hula's own solver ran. Three at most; the second usually moves nothing
        // and the third never has.

        let mut found: Option<Vec<f64>> = None;
        for _ in 0..3 {
            let objective = |v: &[f64], only: Option<&[usize]>| -> Option<f64> {
                let x = self.closed(&plan, &place(v), helix)?;
                self.trial_efficiency(&plan, &x, helix, Some(&cache), only)
            };
            let this_round = if components.len() > 1 {
                let mut at: Vec<f64> = box_.iter().map(|(lo, hi)| 0.5 * (lo + hi)).collect();
                let mut any = false;
                for (component, meshes) in &components {
                    let sub_box: Vec<(f64, f64)> = component.iter().map(|&k| box_[k]).collect();
                    let sub = |w: &[f64]| -> Option<f64> {
                        let mut v = at.clone();
                        for (j, &k) in component.iter().enumerate() {
                            v[k] = w[j];
                        }
                        objective(&v, Some(meshes))
                    };
                    if let Some(w) = search.maximise(&sub_box, &sub) {
                        for (j, &k) in component.iter().enumerate() {
                            at[k] = w[j];
                        }
                        any = true;
                    }
                }
                any.then_some(at)
            } else {
                search.maximise(&box_, &|v| objective(v, None))
            };
            let Some(v) = this_round else {
                break;
            };
            let chosen = place(&v);
            found = Some(v);
            if !bound_by.iter().any(Option::is_some) {
                break;
            }
            let (held, again) = self.sized(helix, &plan, &chosen)?;
            let moved = held.iter().zip(&plan.held).any(|(a, b)| match (a, b) {
                (Some(a), Some(b)) => (a - b).abs() > search.resolution * 1e-3,
                (a, b) => a.is_some() != b.is_some(),
            });
            if !moved {
                break;
            }
            plan = self.plan_held(helix, held);
            bound_by = again;
        }
        match found {
            None => fallback(&plan, &bound_by, super::Searched::FoundNothing),
            Some(v) => Ok(Chosen {
                shifts: self
                    .closed(&plan, &place(&v), helix)
                    .unwrap_or_else(|| settled.clone()),
                how: super::Searched::Chose,
                held: plan.held.clone(),
                bound_by,
            }),
        }
    }

    /// **Every member cut at its mesh group's module and pressure angle** —
    /// the one member that states each, or the group's first where none
    /// does — written into the members that follow. Two members that both
    /// state one and disagree are left as they are: the mesh they share
    /// refuses them by name, which is what a designer needs to see.
    pub fn share(&mut self) {
        for group in self.mesh_groups() {
            let pick = |get: &dyn Fn(&Member) -> Auto<f64>| -> f64 {
                group
                    .iter()
                    .map(|&i| get(&self.members[i]))
                    .find(|a| !a.auto)
                    .unwrap_or_else(|| get(&self.members[group[0]]))
                    .manual
            };
            let module = pick(&|m| m.module);
            let pressure_angle = pick(&|m| m.pressure_angle);
            for &i in &group {
                let m = &mut self.members[i];
                if m.module.auto {
                    m.module.manual = module;
                }
                if m.pressure_angle.auto {
                    m.pressure_angle.manual = pressure_angle;
                }
            }
        }
    }

    /// This shape, shared ([`Self::share`]).
    #[must_use]
    pub fn shared(&self) -> Self {
        let mut s = self.clone();
        s.share();
        s
    }

    /// **Every mesh searched, or none** — what one switch on a stage used to
    /// say, for a fixture that means the whole shape. The panel sets each
    /// mesh's own.
    pub fn set_search(&mut self, on: bool) {
        for m in &mut self.meshes {
            m.search = on;
        }
    }

    /// **Every mesh under one sharing model** — as [`Self::set_search`].
    pub fn set_load_sharing(&mut self, sharing: LoadSharing) {
        for m in &mut self.meshes {
            m.load_sharing = sharing;
        }
    }

    /// **Which members a search may move**: those in a component — meshes
    /// sharing a free member, or on one automatic distance an absorber ties
    /// together, as [`Self::search_components`] reads them — where at least
    /// one mesh asks to be searched.
    fn asking_members(&self, plan: &Plan) -> Vec<bool> {
        let n = self.meshes.len();
        let mut parent: Vec<usize> = (0..n).collect();
        fn find(parent: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while parent[r] != r {
                r = parent[r];
            }
            r
        }
        let mut union = |a: usize, b: usize| {
            let (ra, rb) = (find(&mut parent, a), find(&mut parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        };
        for d in 0..self.distances.len() {
            if plan.held[d].is_none() {
                let on = self.meshes_on(d);
                for w in on.windows(2) {
                    union(w[0], w[1]);
                }
            }
        }
        for i in 0..self.members.len() {
            if plan.role[i] != Role::Free {
                continue;
            }
            let mine: Vec<usize> = (0..n)
                .filter(|&k| self.meshes[k].a == i || self.meshes[k].b == i)
                .collect();
            for w in mine.windows(2) {
                union(w[0], w[1]);
            }
        }
        let asks: Vec<bool> = {
            let mut asks = vec![false; n];
            for k in 0..n {
                if self.meshes[k].search {
                    let r = find(&mut parent, k);
                    asks[r] = true;
                }
            }
            asks
        };
        (0..self.members.len())
            .map(|i| {
                (0..n).any(|k| {
                    (self.meshes[k].a == i || self.meshes[k].b == i) && asks[find(&mut parent, k)]
                })
            })
            .collect()
    }

    /// **The search's axes grouped by the meshes they can move**: two axes
    /// are one component where they touch one mesh, or two meshes on an
    /// automatic distance that an absorber ties together. Each component is
    /// its axis indices and the meshes they score.
    fn search_components(
        &self,
        plan: &Plan,
        axes: &[Coordinate],
        free: &[usize],
    ) -> Vec<(Vec<usize>, Vec<usize>)> {
        let n_meshes = self.meshes.len();
        // Union-find over meshes, then over axes through the meshes they touch.
        let mut parent: Vec<usize> = (0..n_meshes + axes.len()).collect();
        fn find(parent: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while parent[r] != r {
                r = parent[r];
            }
            let mut i = i;
            while parent[i] != r {
                let next = parent[i];
                parent[i] = r;
                i = next;
            }
            r
        }
        let union = |parent: &mut Vec<usize>, a: usize, b: usize| {
            let (ra, rb) = (find(parent, a), find(parent, b));
            if ra != rb {
                parent[ra] = rb;
            }
        };
        for d in 0..self.distances.len() {
            if plan.held[d].is_none() {
                let on = self.meshes_on(d);
                for w in on.windows(2) {
                    union(&mut parent, w[0], w[1]);
                }
            }
        }
        let members_of = |c: &Coordinate| -> Vec<usize> {
            match *c {
                Coordinate::Own(j) => vec![free[j]],
                Coordinate::Sum(pa, pb, _) | Coordinate::Division(pa, pb, _) => {
                    vec![free[pa], free[pb]]
                }
            }
        };
        for (k, axis) in axes.iter().enumerate() {
            for i in members_of(axis) {
                for (m, mesh) in self.meshes.iter().enumerate() {
                    if mesh.a == i || mesh.b == i {
                        union(&mut parent, n_meshes + k, m);
                    }
                }
            }
        }
        let mut out: Vec<(usize, Vec<usize>, Vec<usize>)> = Vec::new();
        for k in 0..axes.len() {
            let root = find(&mut parent, n_meshes + k);
            match out.iter_mut().find(|(r, _, _)| *r == root) {
                Some((_, list, _)) => list.push(k),
                None => out.push((root, vec![k], Vec::new())),
            }
        }
        for m in 0..n_meshes {
            let root = find(&mut parent, m);
            if let Some((_, _, meshes)) = out.iter_mut().find(|(r, _, _)| *r == root) {
                meshes.push(m);
            }
        }
        out.into_iter()
            .map(|(_, axes, meshes)| (axes, meshes))
            .collect()
    }

    /// The shifts the closure settles on, or why it could not — what the
    /// laws of the set's closure ask, with no rating in the way.
    #[cfg(test)]
    pub(crate) fn closure(&self) -> Result<Vec<f64>, TrainError> {
        let helix = self.helix_angles();
        self.chosen_at(&crate::auto::Search::SHIPPED, &helix)
            .map(|c| c.shifts)
    }

    /// **The screw gearing of a crossed mesh**, at the shifts the shape
    /// settles on and the helices the readings decide — what the harness
    /// prints a worm's lead angle and sizing from.
    ///
    /// # Errors
    ///
    /// [`TrainError::Screw`] where the pair cannot exist, and a wiring error
    /// where the mesh is not on crossed shafts.
    pub fn screw(&self, mesh: usize) -> Result<Screw, TrainError> {
        if !self.is_crossed(mesh) {
            return Err(TrainError::Wiring(super::WiringError::NotAMesh(mesh)));
        }
        let helix = self.helix_angles();
        let x = self
            .chosen_at(&crate::auto::Search::SHIPPED, &helix)?
            .shifts;
        self.screw_of(mesh, &x, &helix)
    }

    /// The gear a member would build at a shift, the helix as the readings
    /// decide.
    #[cfg(test)]
    pub(crate) fn params_of(&self, i: usize, x: f64) -> GearParams {
        self.params_at(i, x, &self.helix_angles())
    }

    /// A member's parameters before any automatic value is resolved.
    #[cfg(test)]
    pub(crate) fn base_params_of(&self, i: usize) -> GearParams {
        self.base_params(i, &self.helix_angles())
    }

    /// Whether the optimiser chose, agreed with the floor, or found nothing.
    #[cfg(test)]
    pub(crate) fn searched(&self, search: &crate::auto::Search) -> super::Searched {
        let helix = self.helix_angles();
        self.chosen_at(search, &helix)
            .map_or(super::Searched::FoundNothing, |c| c.how)
    }

    /// The shifts the shape settles on at the shipped effort.
    #[cfg(test)]
    pub(crate) fn shifts(&self) -> Vec<f64> {
        self.shifts_at(&crate::auto::Search::SHIPPED)
    }

    /// **The first member's pitch diameter**, mm, as the shape reads it
    /// from the helix the readings decide — a worm's size, stated or
    /// derived.
    #[cfg(test)]
    pub(crate) fn first_pitch_diameter(&self) -> f64 {
        let m = &self.members[0];
        f64::from(m.gear.teeth.max(1)) * m.normal_module()
            / self.helix_angles()[0].to_radians().cos()
    }

    /// The shifts the shape settles on under a search — what the tests
    /// written against the retired stage types' own choosers ask.
    #[cfg(test)]
    pub(crate) fn shifts_at(&self, search: &crate::auto::Search) -> Vec<f64> {
        let helix = self.helix_angles();
        self.chosen_at(search, &helix)
            .map(|c| c.shifts)
            .unwrap_or_else(|_| self.asked(&helix).iter().map(|a| a.settled).collect())
    }

    /// Every member cut and every mesh at its running distance, at these
    /// shifts, the helices as the readings decide.
    #[cfg(test)]
    pub(crate) fn build_at(&self, x: &[f64]) -> Result<Built, TrainError> {
        let helix = self.helix_angles();
        self.build(x, &helix, &self.plan(&helix).held)
    }

    /// The product of every mesh's efficiency at these shifts, or nothing
    /// where any mesh is inadmissible ([`crate::auto::MeshTrial`]).
    fn trial_efficiency(
        &self,
        plan: &Plan,
        x: &[f64],
        helix: &[f64],
        cache: Option<&TeethCache>,
        only: Option<&[usize]>,
    ) -> Option<f64> {
        let built = self.build_cached(x, helix, &plan.held, cache).ok()?;
        let cut = |i: usize| -> crate::auto::Cut<'_> {
            match &*built.members[i] {
                BuiltMember::Ring { ring, .. } => crate::auto::Cut::ByShaper { ring },
                BuiltMember::Rack { tooth } => {
                    if plan.role[i] == Role::Given {
                        return crate::auto::Cut::Pinned { tooth };
                    }
                    let how = match plan.role[i] {
                        Role::Absorbs(_) | Role::Reaches(_) => super::pair::Decided::Absorbed,
                        _ => super::pair::Decided::Chosen,
                    };
                    crate::auto::Cut::ByRack {
                        tooth,
                        floor: super::pair::undercut_bound(
                            self.members[i].gear.no_undercut,
                            &tooth.params,
                            self.members[i].gear.dedendum,
                            how,
                        ),
                    }
                }
            }
        };
        let mut product = 1.0;
        for (k, bm) in built.meshes.iter().enumerate() {
            // A component's search scores its own meshes; the rest are a
            // constant factor it cannot move.
            if only.is_some_and(|m| !m.contains(&k)) {
                continue;
            }
            let m = self.meshes[k];
            let members = [cut(m.a), cut(m.b)];
            let min_contact_ratio = m.min_contact_ratio;
            product *= match &bm.contact {
                BuiltContact::Line(l) => crate::auto::MeshTrial {
                    members,
                    mesh: &l.operating,
                    path: &l.path,
                    min_contact_ratio,
                    friction: m.sliding_friction,
                }
                .efficiency()?,
                BuiltContact::Point(p) => crate::auto::CrossedTrial {
                    members,
                    screw: &p.screw,
                    path: p.path.as_ref(),
                    centre: bm.running,
                    min_contact_ratio,
                    friction: m.sliding_friction,
                }
                .efficiency()?,
            };
        }
        Some(product)
    }
}

/// Who decides a member's shift.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Role {
    /// The designer.
    Given,
    /// The search, or — with nothing searched — the undercut floor.
    Free,
    /// Nothing: a member in more than one mesh on a given distance stands
    /// where it was asked, and the meshes' other members reach the sum.
    Settled,
    /// The sum a given distance asks of this mesh.
    Reaches(usize),
    /// Closing this constraint on an automatic distance.
    Absorbs(usize),
}

/// One mesh made to run at the distance another does, by one shift.
#[derive(Clone, Copy, Debug)]
struct Constraint {
    first: usize,
    mesh: usize,
    absorber: Option<usize>,
}

struct Plan {
    asked: Vec<super::pair::ShiftAsked>,
    role: Vec<Role>,
    constraints: Vec<Constraint>,
    /// **The running distance each distance is held to**, where it is: the
    /// one stated with its clearance, or the one the tips sized it to
    /// ([`Shape::sized`]). A held distance is one every mesh on it reaches.
    held: Vec<Option<f64>>,
    /// The members that reach a sum on a given distance, **in the order
    /// they were planned** — a shared member reached from one mesh is what
    /// the next mesh's member reaches from.
    reaches: Vec<(usize, usize)>,
}

/// What the tips sized: the running distance each distance is held to, and
/// per distance the mesh whose tips sized it.
type TipSizing = (Vec<Option<f64>>, Vec<Option<usize>>);

/// The teeth a search has cut, by member, shift and helix.
type TeethCache =
    std::cell::RefCell<std::collections::HashMap<(usize, u64, u64), std::rc::Rc<BuiltMember>>>;

/// One axis of the efficiency search, over the free members.
#[derive(Clone, Copy, Debug)]
enum Coordinate {
    Own(usize),
    Sum(usize, usize, f64),
    Division(usize, usize, f64),
}

/// The shifts a shape settled on, and how.
#[derive(Clone, Debug)]
pub struct Chosen {
    shifts: Vec<f64>,
    how: super::Searched,
    /// The running distance each distance was held to — see [`Plan::held`].
    held: Vec<Option<f64>>,
    /// Per distance, the mesh whose tips sized it, where the tips did.
    bound_by: Vec<Option<usize>>,
}

/// **The nearest point to `from` at which `g` is finite**, stepping toward
/// `toward` by a step that starts negligible and doubles. A bracket's end
/// sits on the edge of the involute domain, where the geometry is refused
/// exactly and exists a hair inside; halving toward the middle instead
/// stepped a third of the way in at once and, on a hula's mesh, past the
/// root — which read as *no common distance* on a stage that had one.
fn pull_in(g: &impl Fn(f64) -> f64, from: f64, toward: f64) -> Option<f64> {
    if g(from).is_finite() {
        return Some(from);
    }
    let span = toward - from;
    let mut step = span * 1e-9;
    while step.abs() <= span.abs() {
        let x = from + step;
        if g(x).is_finite() {
            return Some(x);
        }
        step *= 2.0;
    }
    None
}

// ------------------------------------------------------------ building ---

/// One member as cut: by a rack, or by a pinion cutter with the tooth the
/// mesh geometry reads it as.
pub(crate) enum BuiltMember {
    Rack { tooth: Tooth },
    Ring { ring: Box<Ring>, as_gear: Tooth },
}

impl BuiltMember {
    /// The tooth the mesh geometry is built on.
    pub(crate) fn as_gear(&self) -> &Tooth {
        match self {
            Self::Rack { tooth } | Self::Ring { as_gear: tooth, .. } => tooth,
        }
    }

    /// The tip radius, as the cutter left it.
    pub(crate) fn tip_radius(&self) -> f64 {
        match self {
            Self::Rack { tooth } => tooth.ra,
            Self::Ring { ring, .. } => ring.ra,
        }
    }

    /// The root radius, as the cutter left it.
    #[cfg(test)]
    pub(crate) fn root_radius(&self) -> f64 {
        match self {
            Self::Rack { tooth } => tooth.rf,
            Self::Ring { ring, .. } => ring.rf,
        }
    }

    fn flank_ends(&self) -> crate::mesh::FlankEnds {
        match self {
            Self::Rack { tooth } => tooth.flank_ends(),
            Self::Ring { ring, .. } => ring.flank_ends(),
        }
    }

    pub(crate) fn params(&self) -> &GearParams {
        &self.as_gear().params
    }

    fn clamps(&self) -> Vec<Note> {
        match self {
            Self::Rack { tooth } => tooth.clamps.notes.clone(),
            Self::Ring { ring, .. } => ring.clamps.clone(),
        }
    }

    /// The bending section, on whichever outline the member has.
    fn bending(
        &self,
        contact_ratio: f64,
        model: LoadSharing,
        rim: Option<f64>,
    ) -> Option<super::Bending> {
        match self {
            Self::Rack { tooth } => super::Bending::of(tooth, contact_ratio, model, rim),
            Self::Ring { ring, .. } => super::Bending::of(ring.as_ref(), contact_ratio, model, rim),
        }
    }
}

/// One mesh as it runs.
pub(crate) struct BuiltMesh {
    pub(crate) kind: MeshKind,
    pub(crate) running: f64,
    pub(crate) contact: BuiltContact,
}

/// **What the teeth of a mesh do to each other**: a line of contact on
/// parallel shafts, a point on crossed ones. The one seam between the two
/// models, as [`BuiltMember`] is between a rack-cut gear and a ring — every
/// question the solve asks of a mesh is answered here, once per model, and
/// nothing above needs to know which it is holding.
pub(crate) enum BuiltContact {
    Line(LineBuilt),
    Point(PointBuilt),
}

/// The involute mesh of two gears on parallel shafts.
pub(crate) struct LineBuilt {
    /// At zero backlash, where the shifts put it.
    pub(crate) design: Mesh,
    /// At the running distance, where the teeth touch.
    pub(crate) operating: Mesh,
    pub(crate) path: ContactPath,
}

/// Crossed-axis screw gearing between two involute helicoids.
pub(crate) struct PointBuilt {
    pub(crate) screw: Screw,
    /// The zone the teeth leave at the running distance, before any face
    /// limits it; `None` where the teeth never meet.
    pub(crate) path: Option<CrossedPath>,
}

impl BuiltMesh {
    /// The zero-backlash distance, where the shifts put the mesh.
    pub(crate) fn nominal(&self) -> f64 {
        match &self.contact {
            BuiltContact::Line(l) => l.design.a_w,
            BuiltContact::Point(p) => p.screw.centre_distance,
        }
    }

    /// The line contact, where the mesh is one.
    pub(crate) fn line(&self) -> Option<&LineBuilt> {
        match &self.contact {
            BuiltContact::Line(l) => Some(l),
            BuiltContact::Point(_) => None,
        }
    }

    /// The mesh's own efficiency at a friction coefficient, both ways: the
    /// loss integral along a line contact, the friction balance along a
    /// point's — which contains the pitch-point formula exactly and is the
    /// fallback where the teeth leave no zone.
    pub(crate) fn efficiency(&self, a: &Tooth, mu: f64, face: [f64; 2]) -> Directional<f64> {
        match &self.contact {
            BuiltContact::Line(l) => {
                Directional::of(|d| efficiency(&l.path, &l.operating, a, mu, d))
            }
            BuiltContact::Point(p) => {
                let zone = p.zone(face);
                Directional::of(|d| {
                    zone.and_then(|z| z.efficiency(&p.screw, mu, d, PATH_SAMPLES))
                        .unwrap_or_else(|| p.screw.efficiency(mu, d))
                })
            }
        }
    }
}

impl PointBuilt {
    /// The zone as the faces in use actually leave it.
    pub(crate) fn zone(&self, face: [f64; 2]) -> Option<CrossedPath> {
        self.path.as_ref().map(|path| {
            path.limited_by_face(&self.screw, face)
                .map_or(*path, |(z, _)| z)
        })
    }

    /// **The friction at which each direction locks**, quoted beside the
    /// efficiency along the same path — so it is the friction at which *it*
    /// reaches zero rather than the pitch point's, in **both** directions,
    /// since a pair that cannot be driven forward has a threshold too and
    /// the reader is owed it. The pitch-point form is the fallback per
    /// direction.
    fn locking_friction(&self, face: [f64; 2]) -> Directional<f64> {
        let pitch_point = self.screw.locking_friction();
        let along = self
            .zone(face)
            .map(|z| z.locking_friction(&self.screw, PATH_SAMPLES));
        Directional::of(|d| {
            along
                .and_then(|t| *t.get(d))
                .unwrap_or_else(|| *pitch_point.get(d))
        })
    }

    /// **One contact rating**, under a torque on the driving member in the
    /// direction that presses that flank. The closure knows nothing about
    /// which case asked.
    ///
    /// **The patch is the governing model's.** `hertz::peak_pressure` is the
    /// larger of the ellipse and the line the teeth actually have — each
    /// member's flank runs along its own ruling for `b / cos β_b`, and the
    /// shorter bounds the patch exactly as the narrower face carries a
    /// parallel pair — and the patch reported is the one that gave the
    /// number. Without the line the crossed rating was the elliptical
    /// solution alone, which assumes half-spaces of unlimited extent: at a
    /// worm's 90° the patch is a small fraction of the face and the
    /// assumption costs nothing; as the bodies come parallel the ellipse
    /// lengthens without bound and the pressure it reports falls toward
    /// zero, while the real pair is carrying its load on a line that has not
    /// grown at all (`docs/corrections.md`).
    ///
    /// **Rated along the path, not at the pitch point.** The relative radius
    /// peaks where the two roll lengths are equal and falls toward both ends
    /// of the zone; the pitch point sits near that peak, so rating there
    /// alone took the mesh at close to its gentlest. The points that matter
    /// are the two boundaries of single-pair contact, where one tooth carries
    /// everything; where `ε ≤ 1` those are the ends of the zone, so a face
    /// too narrow raises this figure as well as costing continuity. The
    /// force at *each* contact is that contact's own: a stress evaluated in
    /// one place with a load computed in another is two answers wearing one
    /// number.
    fn rate(
        &self,
        face: [f64; 2],
        e_star: f64,
        friction: f64,
        torque: f64,
        on: MeshSide,
        drive: Drive,
    ) -> Result<super::ContactPatch, TrainError> {
        let s = &self.screw;
        let (curvature_along, curvature_across) =
            s.contact_curvatures().ok_or(TrainError::NoContact)?;
        let line_length = [0, 1]
            .map(|i| face[i] / s.member_base(i).1.cos())
            .into_iter()
            .fold(f64::MAX, f64::min);
        let patch_at = |along: f64, across: f64, force: f64| -> Option<(f64, [f64; 2])> {
            let line = crate::hertz::line_pressure(across, force, line_length, e_star);
            let ellipse = crate::hertz::elliptical_contact(along, across, force, e_star);
            let pressure = crate::hertz::peak_pressure(along, across, force, line_length, e_star)?;
            Some(match ellipse {
                Some(e) if e.max_pressure > line => (
                    pressure,
                    [
                        (2.0 * e.semi_major()).min(line_length),
                        2.0 * e.semi_minor(),
                    ],
                ),
                _ => (
                    pressure,
                    [
                        line_length,
                        2.0 * crate::hertz::line_half_width(across, pressure, e_star),
                    ],
                ),
            })
        };
        let force = s.normal_force(torque, on, friction, drive);
        let (pitch_pressure, pitch_patch) =
            patch_at(curvature_along, curvature_across, force).ok_or(TrainError::NoContact)?;
        let mut max_pressure = pitch_pressure;
        let mut patch = pitch_patch;
        let mut worst_position = 0.0;
        let mut curvatures = (curvature_along, curvature_across);
        if let Some(path) = self.zone(face) {
            for position in path.single_pair_bounds(s) {
                let contact = path.contact_at(s, position);
                let Some((along, across)) = path.curvatures_at(s, position) else {
                    continue;
                };
                let Some(force) = contact.normal_force(torque, on, friction, drive) else {
                    continue;
                };
                let Some((pressure, here)) = patch_at(along, across, force) else {
                    continue;
                };
                if pressure > max_pressure {
                    max_pressure = pressure;
                    patch = here;
                    worst_position = position;
                    curvatures = (along, across);
                }
            }
        }
        Ok(super::ContactPatch {
            max_pressure,
            at_pitch_point: pitch_pressure,
            worst_position,
            patch_length: patch[0],
            patch_width: patch[1],
            curvature_along: curvatures.0,
            curvature_across: curvatures.1,
        })
    }

    /// Angular play at one member, radians, at a centre distance `a`: the
    /// separation above nominal and the axial slack projected onto the one
    /// contact normal — the module documentation of [`super::crossed`]
    /// derives it — through the law every mesh shares.
    fn angular_play(&self, a: f64, axial_clearance: f64, teeth: u32) -> f64 {
        let s = &self.screw;
        let Some(n) = s.contact_normal() else {
            return 0.0;
        };
        // A separation opens **both** flanks; a rigid-body slide along the
        // worm's axis opens one exactly as far as it closes the other.
        let separation = 2.0 * (a - s.centre_distance).max(0.0) * n[0].abs();
        let slide = axial_clearance * n[2].abs();
        crate::mesh::angular_play(separation + slide, teeth, s.normal_base_pitch())
    }
}

/// Quadrature points for the friction balance a point contact reports: the
/// trapezium rule's residual is below 1e-9 here (`train::crossed`).
const PATH_SAMPLES: usize = 2048;

/// **What a point contact's builder has in hand** — the crossed-axis
/// counterpart of [`super::LineMesh`], gathered for the same reason.
struct PointMesh {
    coprime: bool,
    efficiency: Directional<f64>,
    efficiency_at_rest: Directional<f64>,
    locking_friction: Directional<f64>,
    /// One contact per load case, in the loads' order.
    contact: Vec<super::ContactPatch>,
    /// The power through the mesh per load case, in the loads' order.
    case_power: Vec<f64>,
    backlash: [super::Backlash; 2],
    row_play: [f64; 3],
    flank_interference: [bool; 2],
    /// The first member's reference radius, mm — what its pitch line speed
    /// is read at.
    first_reference_radius: f64,
    /// The first member's speed against the mesh's frame, rpm, per case.
    first_speed: Vec<f64>,
}

/// A point contact's [`MeshReport`]: the zone as the faces leave it, the
/// sliding at each case's own speed, and what the mesh has to say — whether
/// it locks or nearly does, in **both** directions, against the static
/// coefficient because that is what decides breaking away.
fn point_mesh_report(
    cases: &[super::CaseLoad],
    p: &PointBuilt,
    face: [f64; 2],
    static_friction: f64,
    m: PointMesh,
) -> MeshReport {
    let s = &p.screw;
    let mut notes: Vec<Note> = Vec::new();
    // Asked in both directions: a worm that cannot be back-driven is the
    // familiar case and the one the word "self-locking" is for; a crossed
    // pair at a steep helix split cannot be driven *forward*. Same
    // construction, roles swapped — the keys differ because the sentence
    // differs, which is a catalogue matter and not a computation.
    let locked = m.efficiency.locked();
    for drive in Drive::BOTH {
        let threshold = *m.locking_friction.get(drive);
        let (is_locked, is_near) = match drive {
            Drive::Backward => (key::MESH_SELF_LOCKING, key::MESH_NEAR_SELF_LOCKING),
            Drive::Forward => (key::MESH_FORWARD_LOCKING, key::MESH_NEAR_FORWARD_LOCKING),
        };
        let said = |k: &'static str| {
            Note::new(k)
                .number("friction", static_friction, 3)
                .number("threshold", threshold, 4)
        };
        if *locked.get(drive) {
            notes.push(said(is_locked));
        } else if threshold > 0.0 && static_friction > 0.8 * threshold {
            // **`threshold > 0.0` is load-bearing, not defensive.** A
            // direction no friction can lock has a negative threshold, and
            // `µ > 0.8 × a negative number` is true of every µ — so without
            // this the "close to locking" note fires on precisely the pairs
            // that are furthest from it. Forwards that is the ordinary case.
            notes.push(said(is_near));
        }
    }
    if m.efficiency.forward < 0.5 {
        notes.push(Note::new(key::MESH_LOW_EFFICIENCY).number(
            "percent",
            m.efficiency.forward * 100.0,
            1,
        ));
    }
    // The zone as the widths in use actually leave it — one construction,
    // asked twice for different things. No zone at all is a contact ratio
    // of nought, which the note says as plainly as a short one.
    let (contact_ratio, zone) = p.path.as_ref().map_or((0.0, None), |path| {
        let (zone, limited_by) = path
            .limited_by_face(s, face)
            .unwrap_or((*path, crate::screw::ZoneLimit::Face));
        (
            zone.contact_ratio,
            Some((
                limited_by,
                path.face_widths_for(s, 1.0),
                zone.axial_travel(s),
            )),
        )
    });
    if contact_ratio < 1.0 {
        notes.push(Note::new(key::MESH_CONTACT_RATIO_BELOW_ONE).number("ratio", contact_ratio, 3));
    }
    MeshReport {
        notes,
        coprime: m.coprime,
        contact_ratio,
        locking_friction: m.locking_friction,
        efficiency: m.efficiency,
        efficiency_at_rest: m.efficiency_at_rest,
        sliding_ratio: s.sliding_ratio,
        cases: cases
            .iter()
            .zip(m.contact)
            .zip(&m.first_speed)
            .zip(m.case_power)
            .map(|(((c, contact), speed), power_through)| super::MeshCase {
                case: c.case,
                contact,
                power_through,
                // **How fast the surfaces slide past each other**, which is a
                // speed with no sign to it: a pair rubbing at 3 m/s rubs at
                // 3 m/s whichever way round it is turning.
                sliding_velocity: s.sliding_ratio
                    * (speed / 60.0 * std::f64::consts::TAU).abs()
                    * m.first_reference_radius,
            })
            .collect(),
        backlash: m.backlash,
        row_play: m.row_play,
        flank_interference: m.flank_interference,
        // Both members external: the tips meet on the line of action or not
        // at all.
        tips: None,
        line: None,
        point: Some(super::PointContact {
            limited_by: zone.map_or(crate::screw::ZoneLimit::Face, |z| z.0),
            face_width_for_continuity: zone.and_then(|z| z.1),
            axial_travel: zone.map_or([0.0; 2], |z| z.2),
        }),
    }
}

pub(crate) struct Built {
    /// Shared, so a search that moves one member's shift rebuilds that
    /// member and reuses the rest ([`TeethCache`]).
    pub(crate) members: Vec<std::rc::Rc<BuiltMember>>,
    pub(crate) meshes: Vec<BuiltMesh>,
    /// Per distance: the running distance every mesh on it agrees at.
    pub(crate) running: Vec<Option<f64>>,
}

impl Shape {
    /// **Every member cut and every mesh at its running distance**, at these
    /// shifts.
    fn build(&self, x: &[f64], helix: &[f64], held: &[Option<f64>]) -> Result<Built, TrainError> {
        self.build_cached(x, helix, held, None)
    }

    /// [`Self::build`], with the teeth a search has already cut kept for it:
    /// a search over one mesh's division moves two members of a shape and
    /// rebuilds two, where cutting a ring is the whole cost of a trial.
    fn build_cached(
        &self,
        x: &[f64],
        helix: &[f64],
        held: &[Option<f64>],
        cache: Option<&TeethCache>,
    ) -> Result<Built, TrainError> {
        let cut = |i: usize| -> std::rc::Rc<BuiltMember> {
            let p = self.params_at(i, x[i], helix);
            std::rc::Rc::new(match &self.members[i].ring {
                Some(cutter) => BuiltMember::Ring {
                    ring: Box::new(Ring::cut_by(&p, cutter)),
                    as_gear: Tooth::new(p),
                },
                None => BuiltMember::Rack {
                    tooth: Tooth::new(p),
                },
            })
        };
        let members: Vec<std::rc::Rc<BuiltMember>> = (0..self.members.len())
            .map(|i| match cache {
                None => cut(i),
                Some(cache) => {
                    let key = (i, x[i].to_bits(), helix[i].to_bits());
                    let mut kept = cache.borrow_mut();
                    if let Some(m) = kept.get(&key) {
                        return std::rc::Rc::clone(m);
                    }
                    let m = cut(i);
                    // A search walks a few hundred trials; the cache is
                    // never let past a few thousand teeth.
                    if kept.len() > 4096 {
                        kept.clear();
                    }
                    kept.insert(key, std::rc::Rc::clone(&m));
                    m
                }
            })
            .collect();
        let mut running: Vec<Option<f64>> = vec![None; self.distances.len()];
        let mut meshes = Vec::with_capacity(self.meshes.len());
        for (k, m) in self.meshes.iter().enumerate() {
            let kind = self
                .kind_of(k)
                .ok_or(TrainError::Wiring(super::WiringError::NotAMesh(k)))?;
            let d = self
                .distance_of(k)
                .ok_or(TrainError::Wiring(super::WiringError::NotAMesh(k)))?;
            // The mesh at zero backlash, where the shifts put it: the
            // involute mesh on parallel shafts, the screw gearing on crossed.
            enum Zero {
                Line(Mesh),
                Point(Screw),
            }
            let zero = if self.is_crossed(k) {
                Zero::Point(self.screw_of(k, x, helix)?)
            } else {
                Zero::Line(
                    Mesh::new(members[m.a].as_gear(), members[m.b].as_gear(), kind)
                        .map_err(TrainError::Mesh)?,
                )
            };
            let clearance = self.distances[d].clearance.manual;
            let opened = |nominal: f64| kind.run_at(nominal, clearance);
            let nominal = match &zero {
                Zero::Line(design) => design.a_w,
                Zero::Point(screw) => screw.centre_distance,
            };
            // The distance the pair of axes runs at: given, or the first
            // mesh's zero-backlash distance opened by the clearance — and
            // every later mesh on an automatic distance must agree with it,
            // which the closure arranged unless no shift was free to.
            let target = held
                .get(d)
                .copied()
                .flatten()
                .or_else(|| self.running_target(d));
            let at = match running[d] {
                Some(r) => {
                    if target.is_none() && (opened(nominal) - r).abs() > 1e-9 * r.abs().max(1.0) {
                        return Err(TrainError::NoCommonDistance);
                    }
                    r
                }
                None => {
                    let r = target.unwrap_or_else(|| opened(nominal));
                    running[d] = Some(r);
                    r
                }
            };
            let contact = match zero {
                Zero::Line(design) => {
                    let operating = design.at(at).map_err(TrainError::Mesh)?;
                    let path = ContactPath::new(
                        members[m.a].as_gear(),
                        members[m.b].tip_radius(),
                        &operating,
                    )
                    .ok_or(TrainError::NoContact)?;
                    BuiltContact::Line(LineBuilt {
                        design,
                        operating,
                        path,
                    })
                }
                Zero::Point(screw) => {
                    // The tips are the teeth's own: this is the one place a
                    // crossed pair's tooth form reaches an answer, which is
                    // why it is specified at all (docs/reference.md#crossed-axes).
                    // No zone at all — the teeth never meet — is not a
                    // refusal here: it is a contact ratio of nought, said
                    // by the mesh, and a flank inside a base cylinder is
                    // fouling rather than idle.
                    let path = screw.path_of_contact_at(
                        members[m.a].tip_radius(),
                        members[m.b].tip_radius(),
                        at,
                    );
                    BuiltContact::Point(PointBuilt { screw, path })
                }
            };
            meshes.push(BuiltMesh {
                kind,
                running: at,
                contact,
            });
        }
        Ok(Built {
            members,
            meshes,
            running,
        })
    }
}

/// What a member is, as a designer names it — read off the shape by
/// [`Shape::member_names`], so the harness and the panel name a member the
/// same way from one rule.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MemberRole {
    /// A gear with no role but its number.
    Gear,
    Worm,
    Wheel,
    Sun,
    Planet,
    Ring,
}

/// A member's role, numbered where the shape has more than one of it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MemberName {
    pub role: MemberRole,
    /// `1`, `2`, … where the role is shared, `None` where it is not.
    pub ordinal: Option<usize>,
}

// ------------------------------------------------------------ the result ---

/// What one distance came to.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct DistanceReport {
    /// The zero-backlash distance of each mesh on it, in mesh order.
    pub nominal: Vec<f64>,
    /// The distance every mesh on it runs at.
    pub running: f64,
    /// `running − nominal` of the first mesh, signed as the mesh reads it.
    pub clearance: f64,
    /// **The mesh whose tips sized this distance**, where an automatic one
    /// was opened out past what the shifts left so that its tips clear
    /// ([`Distance::tip_clearance`]); `None` where the shifts' own distance
    /// stood, or the distance was given.
    pub sized_by: Option<usize>,
}

/// What the layout of one replicated axis came to.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LayoutReport {
    /// The axis, as [`Shape::axes`] numbers them.
    pub axis: usize,
    pub count: u32,
    /// Whether `N` equally spaced instances assemble: `Some` where the
    /// arrangement is one the rule is known for — one gear on the axis
    /// meshing two central members.
    pub equal_spacing: Option<bool>,
    pub simultaneous_meshing: Option<bool>,
    /// Tip-to-tip gap between neighbouring instances, mm.
    pub clearance: f64,
    pub clearance_ok: bool,
}

/// Every body's speed and torque in one load case.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct SlotCase {
    pub case: usize,
    /// Per local body, ground first.
    pub speeds: Vec<f64>,
    pub torques: Vec<f64>,
}

/// What a shape produces: its geometry, and its members and meshes rated
/// for the train's cases.
///
/// **No figures of its own.** A ratio, an efficiency, a play and what one
/// more tooth does are a path's ([`super::PathReport`]), read between two
/// bodies a case names. A stage's were a second motion solved under a
/// convention of its own, and a law held them to the path across its ends
/// until nothing read them but the stage.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShapeResult {
    pub distances: Vec<DistanceReport>,
    /// One per replicated axis, in axis order.
    pub layouts: Vec<LayoutReport>,
    pub cases: Vec<SlotCase>,
    pub members: Vec<GearResult>,
    pub meshes: Vec<MeshReport>,
    pub notes: Vec<Note>,
}

/// **A shape cut**: everything about it no load case moves — the shifts its
/// search settled on, every member cut and every mesh at its running
/// distance, the materials, the bending sections, and each mesh's
/// efficiency, which is what the train's flow is written from. A train cuts
/// each of its parts once, solves one flow across them all, and rates each
/// under what the flow hands it ([`rate`]).
pub struct Cut {
    /// Every member at its group's module and pressure angle.
    shape: Shape,
    helix: Vec<f64>,
    wiring: Wiring,
    chosen: Chosen,
    built: Built,
    materials: Vec<Material>,
    /// The meshes each member is in.
    meshes_of: Vec<Vec<usize>>,
    /// The width a worm distance's proportions give a member, where one does.
    recommended: Vec<Option<f64>>,
    /// Each mesh's efficiency sliding, and at rest.
    sliding: Vec<Directional<f64>>,
    at_rest: Vec<Directional<f64>>,
    /// Per member, a bending section in each line mesh it is in.
    bendings: Vec<Vec<(usize, Option<super::Bending>)>>,
}

impl Cut {
    /// **Mesh `k`'s efficiency**, once moving and at rest — what the train's
    /// flow is written from.
    pub(crate) fn efficiency(&self, k: usize) -> [Directional<f64>; 2] {
        [
            self.sliding[k].once_moving(&self.at_rest[k]),
            self.at_rest[k],
        ]
    }

    /// Member `i`'s tooth count as cut.
    pub(crate) fn teeth(&self, i: usize) -> u32 {
        self.built.members[i].params().teeth
    }

    /// Whether member `i` is in a line mesh.
    fn on_a_line(&self, i: usize) -> bool {
        self.meshes_of[i]
            .iter()
            .any(|&k| self.built.meshes[k].line().is_some())
    }

    /// **Member `i`'s width before any rating sizes one**: a worm distance's
    /// conventional proportions, or the width in the box.
    fn early_width(&self, i: usize) -> f64 {
        let w = &self.shape.members[i].gear.face_width;
        self.recommended[i].map_or(w.manual, |r| w.resolve(r))
    }

    /// Mesh `k`'s two members' widths, as `width` gives a member's.
    fn face_of(&self, k: usize, width: &dyn Fn(usize) -> f64) -> [f64; 2] {
        let m = self.shape.meshes[k];
        [width(m.a), width(m.b)]
    }
}

/// **A shape cut** ([`Cut`]).
///
/// # Errors
///
/// A wiring that describes no mechanism, a mesh that cannot mesh, a distance
/// no shift reaches, a material not in the library, or a member with no root
/// section in any of its line meshes.
pub fn cut(shape: &Shape, lib: &MaterialLibrary) -> Result<Cut, TrainError> {
    // **Every member at its group's module and pressure angle** before
    // anything reads one — a member that follows reads what it follows.
    let shape = shape.shared();
    let n = shape.members.len();
    let helix = shape.helix_angles();

    // ---- the wiring describes a mechanism before anything is built at it:
    // a member with no teeth is refused by name here, and not as a tooth
    // too undercut to have a root ([`super::WiringError::MemberWithoutTeeth`]).
    let wiring = shape.wiring();
    wiring.alone(&super::teeth_of(shape.gears()))?;

    // ---- the shifts, and every member and mesh built at them.
    let chosen = shape.chosen_at(&crate::auto::Search::SHIPPED, &helix)?;
    let built = shape.build(&chosen.shifts, &helix, &chosen.held)?;

    // ---- materials.
    let materials: Vec<Material> = shape
        .members
        .iter()
        .map(|m| {
            lib.get(&m.gear.material)
                .ok_or_else(|| TrainError::UnknownMaterial(m.gear.material.clone()))
                .map(|mat| mat.overridden(&m.gear.material_overrides))
        })
        .collect::<Result<_, _>>()?;

    // ---- the meshes each member is in.
    let meshes_of: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            (0..shape.meshes.len())
                .filter(|&k| shape.meshes[k].a == i || shape.meshes[k].b == i)
                .collect()
        })
        .collect();
    // ---- the widths a point contact runs at, decided before anything that
    // reads them: a worm distance's conventional proportions, or the width
    // in the box. Nothing rates a crossed pair's face — its pressure does
    // not depend on the width, so no rating can be inverted for one, and an
    // overlap ratio is a line contact's — so the width at which contact
    // stays continuous is reported beside it as a figure, not acted on. A
    // member on a line mesh as well is sized by that mesh below, and its
    // point contact's zone is read at the width it ends with.
    let recommended: Vec<Option<f64>> = (0..n)
        .map(|i| {
            meshes_of[i].iter().find_map(|&k| {
                let d = shape.distance_of(k)?;
                let BuiltContact::Point(p) = &built.meshes[k].contact else {
                    return None;
                };
                if !shape.distances[d].worm {
                    return None;
                }
                let m = shape.meshes[k];
                Some(if i == m.a {
                    super::crossed::proportions::worm_length(
                        p.screw.axial_module,
                        shape.members[m.b].gear.teeth,
                        shape.members[m.a].gear.teeth,
                    )
                } else {
                    super::crossed::proportions::wheel_face_width(
                        p.screw.axial_module,
                        p.screw.worm_pitch_diameter,
                    )
                })
            })
        })
        .collect();
    let mut cut = Cut {
        shape,
        helix,
        wiring,
        chosen,
        built,
        materials,
        meshes_of,
        recommended,
        sliding: Vec::new(),
        at_rest: Vec::new(),
        bendings: Vec::new(),
    };

    // ---- each mesh's own efficiency, sliding and at rest.
    let mesh_efficiency = |k: usize, mu: f64| -> Directional<f64> {
        let bm = &cut.built.meshes[k];
        let a = cut.built.members[cut.shape.meshes[k].a].as_gear();
        bm.efficiency(a, mu, cut.face_of(k, &|i| cut.early_width(i)))
    };
    let meshes = &cut.shape.meshes;
    let sliding: Vec<Directional<f64>> = (0..meshes.len())
        .map(|k| mesh_efficiency(k, meshes[k].sliding_friction))
        .collect();
    let at_rest: Vec<Directional<f64>> = (0..meshes.len())
        .map(|k| mesh_efficiency(k, meshes[k].static_friction))
        .collect();
    cut.sliding = sliding;
    cut.at_rest = at_rest;

    // ---- bending sections: one per (member, line mesh) pair. **No bending
    // on a point contact, and that is a decision rather than a gap**: the
    // tooth a beam formula would measure is not the tooth a crossed mesh
    // loads — its contact is a point tracking diagonally across the flank,
    // and a cantilever loaded across its whole face has no honest reading
    // of it (docs/rationale.md#a-worm-stage-reports-no-bending-stress).
    let mut bendings: Vec<Vec<(usize, Option<super::Bending>)>> =
        (0..n).map(|_| Vec::new()).collect();
    for (k, m) in cut.shape.meshes.iter().enumerate() {
        let Some(line) = cut.built.meshes[k].line() else {
            continue;
        };
        let cr = line.path.contact_ratio;
        for i in [m.a, m.b] {
            bendings[i].push((
                k,
                cut.built.members[i].bending(
                    cr,
                    m.load_sharing,
                    cut.shape.members[i].gear.rim_thickness,
                ),
            ));
        }
    }
    // **A member with no root section in any of its line meshes refuses the
    // shape**; one rated in some mesh keeps that rating and says which flank
    // went unrated — a planet whose ring-side load point falls off its flank
    // is still rated on its sun side, which is what the set's own kind did
    // without saying so. A ring refuses only its own rating.
    for (i, (m, b)) in cut.shape.members.iter().zip(&bendings).enumerate() {
        if m.ring.is_none() && cut.on_a_line(i) && b.iter().all(|(_, b)| b.is_none()) {
            return Err(TrainError::NoRootSection);
        }
    }

    cut.bendings = bendings;
    Ok(cut)
}

/// **A cut shape rated for the train's cases** — each member and mesh under
/// what the train's flow puts on its bodies in each case ([`super::CaseLoad`]).
///
/// # Errors
///
/// No contact, or a point contact that cannot be rated.
pub fn rate(
    cut: &Cut,
    cases: &[super::CaseLoad],
    reversal: super::Reversal,
) -> Result<ShapeResult, TrainError> {
    let Cut {
        shape,
        helix,
        wiring,
        chosen,
        built,
        materials,
        meshes_of,
        recommended,
        sliding,
        at_rest,
        bendings,
    } = cut;
    let n = shape.members.len();
    let x = &chosen.shifts;
    let on_a_line = |i: usize| cut.on_a_line(i);
    let early_width = |i: usize| cut.early_width(i);
    let face_of = |k: usize, width: &dyn Fn(usize) -> f64| cut.face_of(k, width);

    // **A case's torques are the train's** ([`super::solve_train`]): one
    // flow across every part's meshes, read off for this shape's — which
    // member drives each mesh and the torque pressing it. A shape solves no
    // flow of its own for a case.
    //
    // **How many instances of a mesh act in parallel** — one per planet —
    // which is the mesh's own count, not what either member sees: a
    // planet sees one path of every mesh it is in, and a mesh between two
    // planets is still one of `N`. The flow's torques are totals over the
    // instances, since a central member's is; one instance carries a share.
    let paths = |k: usize| -> f64 { f64::from(wiring.meshes[k].paths) };
    // The tangential force a mesh instance carries in a case, quoted as a
    // torque at member `a`: the driving member's torque, read across —
    // which is what the flow's mesh torque is, whichever member drives.
    let pressing_torque_at_a =
        |k: usize, c: &super::CaseLoad| -> f64 { (c.mesh_torques[k] / paths(k)).abs() };
    // Each mesh's worst pressing torque over the cases, and every case as a
    // scale of it — one evaluation per mesh, every case a multiplication.
    let scaled: Vec<(f64, Vec<f64>)> = (0..shape.meshes.len())
        .map(|k| super::scaled(cases, |c| pressing_torque_at_a(k, c)))
        .collect();

    // ---- contact stress per mesh at the probe width.
    let e_star: Vec<f64> = (0..shape.meshes.len())
        .map(|k| contact_modulus(&materials[shape.meshes[k].a], &materials[shape.meshes[k].b]))
        .collect();
    // A line contact's stress at a width, from the worst torque the mesh
    // carries; a point contact has none here — it is rated per case below.
    let contact_at =
        |k: usize, width: f64| -> Result<Option<crate::strength::ContactStress>, TrainError> {
            let m = shape.meshes[k];
            let Some(line) = built.meshes[k].line() else {
                return Ok(None);
            };
            // Nothing to rate: a shape rated under no case — every case
            // switched off — takes no contact at all, rather than one at a
            // face of no width to refuse.
            if cases.is_empty() {
                return Ok(None);
            }
            contact_stress(
                &line.path,
                &line.operating,
                built.members[m.a].as_gear(),
                PARALLEL_AXES,
                &Load::new(scaled[k].0, width),
                e_star[k],
            )
            .ok_or(TrainError::NoContact)
            .map(Some)
        };
    let probes: Vec<Option<crate::strength::ContactStress>> = (0..shape.meshes.len())
        .map(|k| contact_at(k, PROBE))
        .collect::<Result<_, _>>()?;
    // **A point contact is rated on the torque of the member driving it, in
    // the direction that presses that flank**, case by case — the efficiency
    // *is* the friction balance, so the driver's torque and the driven
    // member's are the same reading wherever the pair transmits, and where
    // it does not — a locked pair — the driver's is the one the flanks are
    // pressed by. The flow says which member drives in each case.
    let point_contact =
        |k: usize, faces: [f64; 2]| -> Result<Option<Vec<super::ContactPatch>>, TrainError> {
            let m = shape.meshes[k];
            let BuiltContact::Point(p) = &built.meshes[k].contact else {
                return Ok(None);
            };
            cases
                .iter()
                .map(|c| {
                    let at_a = pressing_torque_at_a(k, c);
                    let (torque, on, drive) = match c.directions[k] {
                        Drive::Forward => (at_a, MeshSide::First, Drive::Forward),
                        Drive::Backward => (
                            at_a * f64::from(shape.members[m.b].gear.teeth)
                                / f64::from(shape.members[m.a].gear.teeth),
                            MeshSide::Second,
                            Drive::Backward,
                        ),
                    };
                    p.rate(faces, e_star[k], m.sliding_friction, torque, on, drive)
                })
                .collect::<Result<Vec<_>, _>>()
                .map(Some)
        };
    let point_probes: Vec<Option<Vec<super::ContactPatch>>> = (0..shape.meshes.len())
        .map(|k| point_contact(k, face_of(k, &early_width)))
        .collect::<Result<_, _>>()?;

    // ---- what each member's ratings come to, from the probe pass.
    let stress_at = |b: &super::Bending, ft: f64, load: &Load| {
        bending_stress(
            &b.section,
            ft,
            load.face_width,
            RootStressModel::DolanBroghamer,
            b.rim,
        )
        .map(|s| s * b.share)
    };
    let always_reverses = |i: usize| meshes_of[i].len() > 1;
    let rating = |i: usize, widths: &[f64]| -> MemberRating<'_> {
        // A line mesh's loading at the probe width, every case a scale of
        // it; a point mesh's at its own width, case by case.
        let cases = cases
            .iter()
            .enumerate()
            .map(|(c, load)| super::CaseLoadings {
                case: load.case,
                kind: load.kind,
                reverses: load.reverses(),
                meshes: meshes_of[i]
                    .iter()
                    .map(|&k| {
                        let m = shape.meshes[k];
                        let side = usize::from(m.b == i);
                        match (&probes[k], &point_probes[k]) {
                            (Some(probe_stress), _) => {
                                let probe = Load::new(scaled[k].0, PROBE);
                                let ft = probe.tangential(built.members[m.a].as_gear());
                                let b = bendings[i]
                                    .iter()
                                    .find(|(kk, _)| *kk == k)
                                    .and_then(|(_, b)| b.as_ref());
                                Loading {
                                    bending: b.and_then(|b| stress_at(b, ft, &probe)),
                                    contact: probe_stress.governing(side),
                                    measured_at: PROBE,
                                    carried_at: widths[k],
                                    sizes_face: true,
                                }
                                .under(scaled[k].1[c])
                            }
                            (None, Some(patches)) => Loading {
                                bending: None,
                                // The mesh's figure, which is the members'
                                // figure: two flanks share one patch, one
                                // normal force and one `E*`, and a point
                                // contact has only the one place.
                                contact: patches[c].max_pressure,
                                measured_at: widths[k],
                                carried_at: widths[k],
                                sizes_face: false,
                            },
                            (None, None) => Loading {
                                bending: None,
                                contact: 0.0,
                                measured_at: widths[k],
                                carried_at: widths[k],
                                sizes_face: false,
                            },
                        }
                    })
                    .collect(),
            })
            .collect();
        MemberRating {
            material: &materials[i],
            reversal,
            always_reverses: always_reverses(i),
            cases,
        }
    };
    let probe_widths = vec![PROBE; shape.meshes.len()];
    // ...and the width a given axial contact ratio needs, a floor under an
    // automatic width: each mesh's own, at its own helix, and a member's is
    // the largest of its meshes'.
    let mesh_floor: Vec<f64> = shape
        .meshes
        .iter()
        .enumerate()
        .map(|(k, m)| {
            if built.meshes[k].line().is_none() {
                return 0.0;
            }
            super::width_for_overlap(&m.overlap, helix[m.a], shape.members[m.a].normal_module())
                .unwrap_or(0.0)
        })
        .collect();
    let for_overlap = |i: usize| -> f64 {
        meshes_of[i]
            .iter()
            .map(|&k| mesh_floor[k])
            .fold(0.0_f64, f64::max)
    };
    // A member on no line mesh has nothing to ask of a rating, and asks
    // nothing of its mate: its width is its own — a worm distance's
    // proportions, or the box's, and the gear says which.
    let asks: Vec<f64> = (0..n)
        .map(|i| {
            if on_a_line(i) {
                shape.members[i]
                    .gear
                    .face_sources
                    .width_for(
                        &rating(i, &probe_widths).asks(),
                        shape.members[i].gear.face_width.manual,
                    )
                    .max(for_overlap(i))
            } else {
                0.0
            }
        })
        .collect();
    let as_entered = |i: usize| {
        shape.members[i].gear.face_width.auto && !on_a_line(i) && recommended[i].is_none()
    };
    // **A member's automatic width is the largest requirement of any mesh it
    // is in**, because the narrower face carries the pair.
    let mesh_ask: Vec<f64> = shape
        .meshes
        .iter()
        .map(|m| asks[m.a].max(asks[m.b]))
        .collect();
    let widths: Vec<f64> = (0..n)
        .map(|i| {
            let wanted = meshes_of[i]
                .iter()
                .map(|&k| mesh_ask[k])
                .fold(0.0_f64, f64::max);
            let own = shape.members[i].gear.face_width;
            // Nothing rated it: its own proportions, or the box as entered.
            let wanted = if on_a_line(i) {
                wanted.max(recommended[i].unwrap_or(0.0))
            } else {
                recommended[i].unwrap_or(own.manual)
            };
            own.resolve(wanted)
        })
        .collect();
    let mesh_widths: Vec<f64> = shape
        .meshes
        .iter()
        .map(|m| widths[m.a].min(widths[m.b]))
        .collect();
    let rated_contact: Vec<Option<crate::strength::ContactStress>> = (0..shape.meshes.len())
        .map(|k| contact_at(k, mesh_widths[k]))
        .collect::<Result<_, _>>()?;
    let final_width = |i: usize| widths[i];
    let rated_point: Vec<Option<Vec<super::ContactPatch>>> = (0..shape.meshes.len())
        .map(|k| point_contact(k, face_of(k, &final_width)))
        .collect::<Result<_, _>>()?;

    // ---- backlash: each mesh's play, as its row and at each member. Where
    // it shows on a body is a path's to say ([`super::PathReport`]).
    let play_of = |k: usize, a: f64| -> f64 {
        let bm = &built.meshes[k];
        let m = shape.meshes[k];
        let d = shape.distance_of(k).unwrap_or(0);
        let axial = shape.distances[d].axial_clearance;
        match &bm.contact {
            BuiltContact::Line(l) => {
                let rack = shape.rack_of(k, helix);
                let alpha_n = shape.members[m.a].normal_pressure_angle().to_radians();
                let bb = crate::plane::base_helix_angle(helix[m.a].to_radians(), alpha_n);
                let slide = axial * bb.sin().abs();
                let p_bn = std::f64::consts::PI * rack.mn * alpha_n.cos();
                // The row's play: `Δ = j |Σz| / a`, plus the axial float's.
                l.design.backlash(a).unwrap_or(0.0) * shape.tooth_sum(k).abs() / a
                    + 2.0 * std::f64::consts::PI * slide / p_bn
            }
            // The screw law takes the *separation* from the geometric
            // distance rather than the distance itself, and the row's play
            // is a member's angular play at its own count.
            BuiltContact::Point(p) => {
                let z = shape.members[m.a].gear.teeth;
                p.angular_play(a, axial, z) * f64::from(z)
            }
        }
    };
    // **Each mesh's play at its own distance**, and the band is every
    // distance at the same end of its own tolerance at once: `t` is −1, 0
    // or +1 and each mesh reads its distance's minus, running or plus. A
    // first draft evaluated every mesh at the *first* distance, which on a
    // shape with one distance is the same thing and on a Ravigneaux put a
    // planet–planet mesh 7 mm from where it runs.
    let at_band = |k: usize, t: f64| -> f64 {
        let d = &shape.distances[shape.distance_of(k).unwrap_or(0)];
        built.meshes[k].running
            + if t < 0.0 {
                -d.tolerance_minus
            } else if t > 0.0 {
                d.tolerance_plus
            } else {
                0.0
            }
    };
    let member_backlash = |k: usize, side: MeshSide| -> super::Backlash {
        let m = shape.meshes[k];
        let z = f64::from(match side {
            MeshSide::First => shape.members[m.a].gear.teeth,
            MeshSide::Second => shape.members[m.b].gear.teeth,
        });
        let d = shape.distance_of(k).unwrap_or(0);
        super::Backlash::banded(
            built.meshes[k].running,
            shape.distances[d].tolerance_minus,
            shape.distances[d].tolerance_plus,
            |a| (play_of(k, a) / z).to_degrees(),
        )
    };

    // ---- notes.
    let mut notes = Vec::new();
    let mut distances = Vec::new();
    for d in 0..shape.distances.len() {
        let meshes = shape.meshes_on(d);
        let nominal: Vec<f64> = meshes.iter().map(|&k| built.meshes[k].nominal()).collect();
        let Some(&first) = meshes.first() else {
            continue;
        };
        let running = built.running[d].unwrap_or(0.0);
        let clearance = built.meshes[first].kind.sign() * (running - built.meshes[first].nominal());
        // What the distance has to say, **asked of every mesh on it**: a
        // given distance one mesh's shifts could not reach — a set with two
        // of its three shifts given and its distance too — is said of that
        // mesh, and a mesh assembled inside its zero-backlash distance is
        // said of that one.
        for &k in &meshes {
            let bm = &built.meshes[k];
            notes.extend(super::distance_notes(
                shape
                    .given_running(d)
                    .map(|r| bm.kind.nominal_of(r, shape.distances[d].clearance.manual)),
                bm.nominal(),
                bm.kind.sign() * (running - bm.nominal()),
            ));
        }
        distances.push(DistanceReport {
            nominal,
            running,
            clearance,
            sized_by: chosen.bound_by.get(d).copied().flatten(),
        });
    }
    notes.extend(chosen.how.note());
    // What each mesh group's ratio owes its reader: the group's first mesh
    // reads the size, every mesh's is a floor.
    for (group, meshes) in shape.mesh_groups().iter().zip(shape.group_meshes()) {
        let reads_size = shape.overlap_reads_size(group);
        for (n, &k) in meshes.iter().enumerate() {
            let m = shape.meshes[k];
            notes.extend(super::overlap_notes(
                n == 0 && reads_size && !m.overlap.auto,
                &m.overlap,
                shape.members[m.a].normal_module(),
                shape.given_width(group),
                helix[m.a],
            ));
        }
    }
    // ---- the layout of every replicated axis.
    let layouts: Vec<LayoutReport> = shape
        .axes
        .iter()
        .enumerate()
        .filter(|(_, a)| a.count > 1)
        .filter_map(|(axis, a)| {
            let count = a.count;
            let on_axis: Vec<usize> = (0..n)
                .filter(|&i| shape.axis_of_slot(shape.slot_of_member(i)) == Some(axis))
                .collect();
            // The radius the instances stand at: the distance from the axis
            // the carrier turns about, not whatever mesh comes first — a
            // planet meshing another planet has a distance that is neither.
            let central = shape.axis_of_slot(shape.slot(a.carried_by))?;
            let d = shape
                .distances
                .iter()
                .position(|d| d.axes == [central, axis] || d.axes == [axis, central])?;
            let running = built.running[d]?;
            let tip = on_axis
                .iter()
                .map(|&i| 2.0 * built.members[i].tip_radius())
                .fold(0.0_f64, f64::max);
            let clearance = 2.0 * running * (std::f64::consts::PI / f64::from(count)).sin() - tip;
            let assembly = shape.assembly(axis);
            let equal_spacing = assembly.map(|a| a.0);
            let simultaneous_meshing = assembly.map(|a| a.1);
            Some(LayoutReport {
                axis,
                count,
                equal_spacing,
                simultaneous_meshing,
                clearance,
                clearance_ok: clearance >= a.min_planet_clearance,
            })
        })
        .collect();
    for l in &layouts {
        notes.push(Note::new(key::PART_PLANETS_SHARE_LOAD_EQUALLY).count("planets", l.count));
        if l.equal_spacing == Some(false) {
            notes.push(Note::new(key::PART_PLANETS_NOT_EVENLY_SPACED).count("planets", l.count));
        }
        if !l.clearance_ok {
            notes.push(
                Note::new(key::PART_PLANET_CLEARANCE_BELOW_MINIMUM)
                    .number("gap", l.clearance, 3)
                    .number("minimum", shape.axes[l.axis].min_planet_clearance, 3),
            );
        }
    }

    // ---- each member's result.
    let gear_notes = |i: usize| -> Vec<Note> {
        let mut out = Vec::new();
        let rim = bendings[i]
            .iter()
            .find_map(|(_, b)| b.as_ref().and_then(|b| b.rim));
        out.extend(super::rim_below_minimum(rim));
        if let BuiltMember::Rack { tooth } = &*built.members[i] {
            out.extend(super::undercut_note(tooth));
        }
        out.extend(reversal.note_for(reversal.reverses(
            always_reverses(i),
            cases.iter().any(super::CaseLoad::reverses),
        )));
        let g = &shape.members[i].gear;
        if shape.members[i].ring.is_none() {
            out.extend(g.shift_asked(&shape.base_params(i, helix)).note());
            out.extend(
                g.addendum_asked(&GearParams {
                    profile_shift: x[i],
                    ..shape.base_params(i, helix)
                })
                .note(),
            );
        }
        out.extend(g.face_width_note());
        out.extend(as_entered(i).then(|| Note::new(key::GEAR_FACE_WIDTH_AS_ENTERED)));
        // ...and each mesh whose load point leaves this member no section
        // to rate, where another mesh's did.
        if bendings[i].iter().any(|(_, b)| b.is_some()) {
            for (k, b) in &bendings[i] {
                if b.is_none() {
                    // Numbered as the panel numbers meshes, from one.
                    out.push(
                        Note::new(key::GEAR_BENDING_UNRATED_IN_MESH)
                            .text("mesh", (k + 1).to_string()),
                    );
                }
            }
        }
        if let BuiltMember::Ring { ring, .. } = &*built.members[i] {
            if ring.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_RAISED)) {
                out.push(Note::new(key::GEAR_RING_ADDENDUM_CLAMPED));
            }
        }
        out
    };
    // **A member's torque is the torque its teeth carry** — the mesh force at
    // its reference cylinder, which is the driver's torque read across the
    // mesh and the one number every stress on the member is proportional to.
    // What a member's *body* delivers, `η` less on the driven side, is the
    // body's figure (`ShapeResult::cases`), not the gear's. A member in two
    // meshes reports the larger.
    let member_torque = |i: usize, c: &super::CaseLoad| -> f64 {
        meshes_of[i]
            .iter()
            .map(|&k| {
                let m = shape.meshes[k];
                let at_a = pressing_torque_at_a(k, c);
                if m.a == i {
                    at_a
                } else {
                    at_a * f64::from(shape.members[m.b].gear.teeth)
                        / f64::from(shape.members[m.a].gear.teeth)
                }
            })
            .fold(0.0_f64, f64::max)
    };
    let members: Vec<GearResult> = (0..n)
        .map(|i| {
            let shaft = shape.slot_of_member(i);
            let frame = shape.frame_of_member(i);
            let rated_cases = rating(i, &mesh_widths)
                .rated()
                .into_iter()
                .zip(cases)
                .map(|(r, c)| {
                    // **How often this member's teeth are engaged** over
                    // the duty: once per turn against the frame its mesh
                    // stands still in, once for each parallel path it sees.
                    // In the carrier's frame the arm stands still and
                    // everything else turns past it, and it is one rule for
                    // a sun, a ring, a planet, a grounded gear and a wobble
                    // body. It had been three: a sun and a ring counted the
                    // input body's turns — a held ring, loaded while it does
                    // not turn at all, three and a half times over on the
                    // shipped counts — and a planet was referred to the
                    // sun's speed rather than the input's, right only where
                    // those are the same body.
                    let cycles = c.turns.as_ref().map(|t| {
                        super::loaded_cycles(super::Turns {
                            revolutions: (t[shaft] - t[frame]).abs()
                                * f64::from(wiring.paths_seen(i)),
                            reversing_actuations: c.reversing_actuations,
                        })
                    });
                    r.into_case(
                        member_torque(i, c),
                        (c.speeds[shaft], c.speeds[shaft] - c.speeds[frame]),
                        cycles,
                    )
                })
                .collect();
            GearResult::of(MemberFacts {
                profile_shift: x[i],
                params: built.members[i].params(),
                input: &shape.members[i].gear,
                cases: rated_cases,
                face_width: widths[i],
                recommended_face_width: recommended[i],
                material: materials[i].clone(),
                clamps: built.members[i].clamps(),
                notes: gear_notes(i),
            })
        })
        .collect();

    // ---- each mesh's report.
    let meshes: Vec<MeshReport> = (0..shape.meshes.len())
        .map(|k| {
            let m = shape.meshes[k];
            let bm = &built.meshes[k];
            let case_power: Vec<f64> = cases
                .iter()
                .map(|c| c.mesh_powers.get(k).copied().unwrap_or(0.0))
                .collect();
            let (a, b) = (&built.members[m.a], &built.members[m.b]);
            let coprime =
                super::gcd(shape.members[m.a].gear.teeth, shape.members[m.b].gear.teeth) == 1;
            let efficiency = sliding[k].once_moving(&at_rest[k]);
            let backlash = [
                member_backlash(k, MeshSide::First),
                member_backlash(k, MeshSide::Second),
            ];
            let row_play = [-1.0, 0.0, 1.0].map(|t| play_of(k, at_band(k, t)));
            match &bm.contact {
                BuiltContact::Line(l) => super::line_mesh_report(
                    cases,
                    super::LineMesh {
                        coprime,
                        contact_ratios: ContactRatios::of(
                            l.path.contact_ratio,
                            mesh_widths[k],
                            helix[m.a],
                            shape.members[m.a].normal_module(),
                        ),
                        operating_pressure_angle: l.operating.alpha_w.to_degrees(),
                        efficiency,
                        efficiency_at_rest: at_rest[k],
                        // Every line contact was rated above; the map is
                        // over the option so nothing here can panic.
                        contact: rated_contact[k].as_ref().map_or_else(Vec::new, |stress| {
                            scaled[k]
                                .1
                                .iter()
                                .map(|&s| {
                                    super::ContactPatch::line(stress, s, mesh_widths[k], e_star[k])
                                })
                                .collect()
                        }),
                        case_power,
                        backlash,
                        row_play,
                        flank_interference: l
                            .operating
                            .flank_interference([a.flank_ends(), b.flank_ends()]),
                        tips: match &**b {
                            BuiltMember::Ring { ring, .. } => {
                                super::TipRoom::at(ring, a.as_gear(), bm.running)
                            }
                            BuiltMember::Rack { .. } => None,
                        },
                        notes: bendings[m.a]
                            .iter()
                            .find(|(kk, _)| *kk == k)
                            .and_then(|(_, b)| b.as_ref().and_then(|b| b.note.clone()))
                            .into_iter()
                            .collect(),
                    },
                ),
                BuiltContact::Point(p) => point_mesh_report(
                    cases,
                    p,
                    face_of(k, &final_width),
                    m.static_friction,
                    PointMesh {
                        coprime,
                        efficiency,
                        efficiency_at_rest: at_rest[k],
                        locking_friction: p.locking_friction(face_of(k, &final_width)),
                        contact: rated_point[k].clone().unwrap_or_default(),
                        case_power,
                        backlash,
                        row_play,
                        flank_interference: p.path.as_ref().map_or([true, true], |path| {
                            path.flank_interference(&p.screw, [a.flank_ends(), b.flank_ends()])
                        }),
                        first_reference_radius: f64::from(shape.members[m.a].gear.teeth)
                            * shape.members[m.a].normal_module()
                            / helix[m.a].to_radians().cos()
                            / 2.0,
                        // The first member's speed against the frame the
                        // mesh stands still in, per case.
                        first_speed: cases
                            .iter()
                            .map(|c| {
                                c.speeds[shape.slot_of_member(m.a)]
                                    - c.speeds[shape.frame_of_member(m.a)]
                            })
                            .collect(),
                    },
                ),
            }
        })
        .collect();

    let result = ShapeResult {
        distances,
        layouts,
        cases: cases
            .iter()
            .map(|c| SlotCase {
                case: c.case,
                speeds: c.speeds.clone(),
                torques: c.torques.clone(),
            })
            .collect(),
        members,
        meshes,
        notes,
    };
    Ok(result)
}

// -------------------------------------------------- what a shape owes ---

/// **What the shape declares so the machinery above it can serve it** — the
/// whole of what a stage owed, once a trait with one implementor.
///
/// Six questions: which gears it has, which inputs relief may turn and by
/// what name, how its helix may be stated, which of its inputs argue with
/// each other, **where its bodies and meshes sit**, and **which bodies a
/// train may address** and what convention holds when it addresses none.
/// Everything that walks those — counting, relieving, seeding a box, reading
/// the helix the readings state, assembling the kinematic system, laying a
/// train's constraints over the convention — is written once above the
/// shape (`train/mod.rs`), so what answers the six has all of it without
/// writing any.
///
/// The fifth and sixth are the ones that tested the claim
/// `docs/rationale.md#one-stage-one-result` makes: that a new arrangement
/// should be new **kinematics** and no new rating machinery. The shape
/// states its topology here and the one solver in [`crate::kinematics`]
/// answers every question about motion, torque and play that used to be
/// answered per stage type; it states its ports here and the train's
/// conditions decide which is driven and which held, which used to be a
/// field on the type. The claim held so well that the types went, and the
/// trait that stated the six went after them: one shape answers them.
impl Shape {
    /// The gears in the order [`ShapeResult::members`] reports them.
    pub fn gears(&self) -> Vec<&MemberGear> {
        self.members.iter().map(|m| &m.gear).collect()
    }

    /// Every input relief may turn: each distance's, by index, the overlap,
    /// and each member's.
    pub(crate) fn inputs(&mut self) -> Vec<(Freedom, &mut Auto<f64>)> {
        let mut out = Vec::new();
        for (k, d) in self.distances.iter_mut().enumerate() {
            out.push((Freedom::Distance(k), &mut d.distance));
            out.push((Freedom::Clearance(k), &mut d.clearance));
        }
        for (k, m) in self.meshes.iter_mut().enumerate() {
            out.push((Freedom::Overlap(k), &mut m.overlap));
        }
        for (i, m) in self.members.iter_mut().enumerate() {
            out.push((
                Freedom::Member(i, MemberFreedom::PitchDiameter),
                &mut m.pitch_diameter,
            ));
            let MemberGear {
                profile_shift,
                helix_angle,
                face_width,
                ..
            } = &mut m.gear;
            out.push((Freedom::Member(i, MemberFreedom::Shift), profile_shift));
            out.push((Freedom::Member(i, MemberFreedom::Helix), helix_angle));
            out.push((Freedom::Member(i, MemberFreedom::FaceWidth), face_width));
            out.push((
                Freedom::Member(i, MemberFreedom::ThicknessMod),
                &mut m.thickness_mod,
            ));
            out.push((Freedom::Member(i, MemberFreedom::Module), &mut m.module));
            out.push((
                Freedom::Member(i, MemberFreedom::PressureAngle),
                &mut m.pressure_angle,
            ));
        }
        out
    }

    /// **Every reading of a size, mesh group by mesh group**: each member's
    /// helix in its own hand and its pitch diameter, and the group's first
    /// mesh's overlap where every width of the group is given and the
    /// ratio decides the helix. One size per group — the helix propagates
    /// through meshes and no further — so a layshaft's pairs each state
    /// their own, and [`Self::readings_for`] is the entry a mesh's relation
    /// counts.
    pub(crate) fn readings(&self) -> Vec<Reading> {
        let mut out: Vec<Reading> = Vec::new();
        let meshes = self.group_meshes();
        for (g, group) in self.mesh_groups().iter().enumerate() {
            for &i in group {
                let m = &self.members[i];
                out.push(Reading::helix(i, &m.gear, |b| b));
                let z1 = f64::from(m.gear.teeth.max(1)) * m.normal_module();
                out.push(Reading {
                    freedom: Freedom::Member(i, MemberFreedom::PitchDiameter),
                    helix: (!m.pitch_diameter.auto).then(|| {
                        (z1 / m.pitch_diameter.manual)
                            .clamp(-1.0, 1.0)
                            .acos()
                            .to_degrees()
                    }),
                });
            }
            if self.overlap_reads_size(group) {
                if let Some(&k) = meshes[g].first() {
                    let m = self.meshes[k];
                    out.push(Reading::overlap(
                        k,
                        &m.overlap,
                        self.members[m.a].normal_module(),
                        self.given_width(group),
                    ));
                }
            }
        }
        out
    }

    /// **One relation per mesh**: a mesh's two shifts, the clearance and the
    /// size are related to the distance it runs at by one equation, so of
    /// them all but one may be given. **The distance is a freedom of the
    /// first mesh on it alone**: an automatic distance is whatever the
    /// first mesh's shifts leave, and every later mesh on it *absorbs* the
    /// difference on one of its members — so a later mesh's relation is
    /// the shifts that can absorb for it, in the plan's own order of
    /// preference ([`Self::absorbers`]), and at least one of those gives:
    /// a planet between a sun and a ring before either, never a planet
    /// between two rings, which closes nothing, and neither the clearance
    /// nor the size, which move every mesh on the distance together and
    /// close no difference between two.
    /// The distance gives way first — it is the one a designer expects to
    /// give when they pin everything else — then the shifts, then the
    /// clearance, then the size, since a shift moves the teeth where a
    /// size changes them. And of the distance and the clearance at most
    /// one may be automatic.
    ///
    /// It was one relation per *distance*, counting entries less meshes —
    /// the right total and the wrong distribution: a layshaft's three pairs
    /// pinned and relieved gave the distance and the first pair's two
    /// shifts back and left the other two pairs both-given on a distance
    /// the first defined, which the solve refuses (`NoCommonDistance`).
    /// The law `every_input_relief_leaves_given_is_honoured_by_the_solve`
    /// found it the day the layshaft joined the presets it sweeps.
    ///
    /// The clearance sits *after* the shifts, where the pair had it before
    /// them, because a shape with three shifts on one distance can be over
    /// by two: relieving the clearance would hand it straight back to the
    /// group below, which pins it again, and the walk would never settle.
    /// A shift gives instead, which is what the set's own kind did.
    pub fn freedoms(&self) -> Vec<FreedomGroup> {
        let mut groups = Vec::new();
        for d in 0..self.distances.len() {
            let meshes = self.meshes_on(d);
            if meshes.is_empty() {
                continue;
            }
            for (n, &k) in meshes.iter().enumerate() {
                let m = self.meshes[k];
                let mut order = Vec::new();
                if n == 0 {
                    order.push(vec![Freedom::Distance(d)]);
                    order.push(vec![Freedom::Member(m.a, MemberFreedom::Shift)]);
                    order.push(vec![Freedom::Member(m.b, MemberFreedom::Shift)]);
                    order.push(vec![Freedom::Clearance(d)]);
                    order.push(super::entry(&self.readings_for(k)));
                } else {
                    // A later mesh gives on a member that can absorb for
                    // it — the plan's own list — and on nothing else: not
                    // a member with no leverage, and not the clearance or
                    // the size, which move every mesh on the distance
                    // together and close no difference between two.
                    order.extend(
                        self.absorbers(meshes[0], k, &meshes[..n])
                            .into_iter()
                            .map(|i| vec![Freedom::Member(i, MemberFreedom::Shift)]),
                    );
                    if order.is_empty() {
                        continue;
                    }
                }
                let entries = order.len();
                groups.push(FreedomGroup {
                    given_at_most: entries - 1,
                    automatic_at_most: entries,
                    order,
                });
            }
            groups.push(super::distance_and_clearance(d));
        }
        // **A mesh's two thickness coefficients are one number said twice**
        // — the second is the first's mate by the mesh's rule — so at most
        // one of them is given; the one just touched stays and the other
        // follows. Both automatic is the standard tooth on both.
        for m in &self.meshes {
            groups.push(FreedomGroup {
                given_at_most: 1,
                automatic_at_most: 2,
                order: vec![
                    vec![Freedom::Member(m.a, MemberFreedom::ThicknessMod)],
                    vec![Freedom::Member(m.b, MemberFreedom::ThicknessMod)],
                ],
            });
        }
        // **A mesh group's module and pressure angle are one number each,
        // said once.** Exactly one member of the group states it — the one
        // just touched stays, and the rest follow — which is the helix's
        // rule with the relation made equality. Given nowhere, the first
        // member is pinned: a module is a designer's number, and a box that
        // says *given* is the one they read it from.
        for group in self.mesh_groups() {
            for freedom in [MemberFreedom::Module, MemberFreedom::PressureAngle] {
                groups.push(FreedomGroup {
                    given_at_most: 1,
                    automatic_at_most: group.len() - 1,
                    order: group
                        .iter()
                        .map(|&i| vec![Freedom::Member(i, freedom)])
                        .collect(),
                });
            }
        }
        // On crossed shafts an axial contact ratio is nothing at all, and is
        // turned back automatic.
        for k in 0..self.meshes.len() {
            if self.is_crossed(k) {
                groups.push(super::always_automatic(Freedom::Overlap(k)));
            }
        }
        groups
    }

    /// The shape *is* the topology: each member spins with its body in the
    /// frame its axis stands still in, and a mesh's sign is its members'.
    pub fn wiring(&self) -> Wiring {
        Wiring {
            slots: (0..=self.bodies.len()).map(|s| self.label_of(s)).collect(),
            mounts: (0..self.members.len())
                .map(|i| Mount {
                    spins_with: self.slot_of_member(i),
                    axis_fixed_in: self.frame_of_member(i),
                    replicated: self.replicated(self.slot_of_member(i)),
                })
                .collect(),
            meshes: (0..self.meshes.len())
                .map(|k| {
                    let m = self.meshes[k];
                    MeshSpec {
                        a: m.a,
                        b: m.b,
                        // Two rings in mesh is no mesh; the wiring's own check
                        // refuses it as it refuses a member meshing itself.
                        kind: self.kind_of(k).unwrap_or(MeshKind::External),
                        paths: self
                            .count_of(self.slot_of_member(m.a))
                            .max(self.count_of(self.slot_of_member(m.b))),
                    }
                })
                .collect(),
            couplings: self
                .couplings
                .iter()
                .map(|c| c.map(|b| self.slot(b)))
                .collect(),
        }
    }

    /// **Every body that is not replicated is a port**, in body order — a
    /// pair's two members, a set's sun, carrier and ring, a layshaft, a
    /// shaft an offset coupling turns, and a single orbiting member: a
    /// hula's wobble body is the same body with four gears on it. (For a
    /// while a body
    /// on a carried axis was no port, because a case reacted every open
    /// port it did not load and so held the wobble body; a case declares
    /// what it reacts now, and an orbiting port is a port.) What is held by
    /// convention is the first ring's body, where there is a ring.
    pub fn ports(&self) -> Ports {
        let ports: Vec<Body> = (1..=self.bodies.len())
            .filter(|&s| !self.replicated(s))
            .collect();
        let held: Vec<Body> = self
            .members
            .iter()
            .filter(|m| m.ring.is_some())
            .map(|m| self.slot(m.body))
            .find(|s| ports.contains(s))
            .into_iter()
            .collect();
        Ports { ports, held }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The laws a stage obeys, asked of the shape.** Most of these were
    //! written against the planetary kind and moved here when it retired;
    //! they read the shape's members and meshes by position — sun, planet,
    //! ring; sun–planet, planet–ring — as the preset lays them out.
    //!
    //! The gate that held the shape against the retired stage types figure
    //! for figure — every member's every case and every mesh's every figure,
    //! on a pair, a worm, and every arrangement of a set — lived beside them
    //! while both existed and went with them (`git show ac0dccc`); what it found is
    //! in `docs/corrections.md`, and the corpus records what moved.

    use super::*;
    use crate::planetary::{Arrangement, PlanetaryShaft};
    use crate::train::arrangements as arr;
    use crate::train::test_library;

    /// How far a set's two meshes disagree about the one distance, from the
    /// zero-backlash distances it reports, each opened by the clearance its
    /// own way.
    fn residual(r: &ShapeResult) -> f64 {
        let d = &r.distances[0];
        ((d.nominal[0] + d.clearance) - (d.nominal[1] - d.clearance)).abs()
    }

    /// A set through the shape, under its convention or a boundary.
    fn solve_set(
        set: &Shape,
        torque: f64,
        speed: f64,
        lib: &MaterialLibrary,
    ) -> Result<crate::train::Alone, TrainError> {
        crate::train::solve_alone(&crate::train::Train::alone(set, torque, speed), lib)
    }

    /// **A crossed distance is built as the point-contact model**, and the
    /// shape reports what it reports: a point contact, no bending on the worm.
    #[test]
    fn a_crossed_distance_is_a_point_contact() {
        for shape in [arr::worm(1, 40), {
            let mut s = arr::worm(1, 40).with_first_helix(45.0);
            s.distances[0].worm = false;
            s
        }] {
            assert!(shape.is_crossed(0) && shape.screw(0).is_ok());
            let r = solve_set(&shape, 2.0, 3000.0, &test_library()).unwrap();
            assert!(r.meshes[0].point.is_some());
            assert_eq!(r.members.len(), 2);
            assert!(
                r.ratio.unwrap() < 0.0,
                "an external pair reverses: {}",
                r.ratio.unwrap()
            );
        }
    }

    /// **Every arrangement of a set solves through the shape** and reports a
    /// ratio the graph gives, an efficiency below one both ways, and a play
    /// at whichever body is the output.
    /// A set as the closure's laws ask it: the shifts as typed, no
    /// undercut floor, zero backlash, the planet closing it.
    fn closure_set(sun: u32, planet: u32, ring: u32) -> Shape {
        let mut set = arr::planetary(12, 30, 72, 3);
        set.distances[0].clearance = Auto::fixed(0.0);
        for (m, z) in set.members.iter_mut().zip([sun, planet, ring]) {
            m.gear.teeth = z;
        }
        set.members[0].gear.profile_shift = Auto::fixed(0.0);
        set.members[2].gear.profile_shift = Auto::fixed(0.0);
        for m in &mut set.members {
            m.gear.no_undercut = false;
        }
        set
    }

    /// **The ideal ring needs no planet shift** — `z_r = z_s + 2 z_p` puts
    /// the planet exactly halfway, so the shift is zero, not nearly zero, and
    /// the common distance is the reference one. The check the whole closure
    /// has to pass, kept from the set's own solver.
    #[test]
    fn the_ideal_ring_needs_no_planet_shift() {
        for (sun, planet) in [(17u32, 17u32), (20, 25), (13, 31), (40, 15)] {
            let ring = sun + 2 * planet;
            let shape = closure_set(sun, planet, ring);
            let x = shape.closure().unwrap();
            assert!(
                x[1].abs() < 1e-12,
                "z={sun}/{planet}/{ring}: shift {}",
                x[1]
            );
            let b = shape.build_at(&x).unwrap();
            let a_ref = f64::from(sun + planet) / 2.0;
            assert!((b.running[0].unwrap() - a_ref).abs() < 1e-12);
        }
    }

    /// **The required planet shift rises with the ring's count**, which is
    /// what made the set's own ring search provably complete, and **the
    /// counts that close form one run with no hole in it** — the ideal ring
    /// inside it. A hole would be a bracket's endpoint rounded outside the
    /// involute domain it was meant to sit on, which the set's solver once
    /// had on a 24/16 set with four planets.
    #[test]
    fn the_admissible_ring_counts_are_one_run_and_the_shift_rises_along_it() {
        for (sun, planet) in [
            (17u32, 17u32),
            (18, 18),
            (24, 16),
            (13, 31),
            (40, 15),
            (20, 25),
            (9, 21),
            (31, 13),
        ] {
            let ideal = sun + 2 * planet;
            let mut found: Vec<(u32, f64)> = Vec::new();
            for ring in (planet + 1)..=(2 * ideal) {
                if let Ok(x) = closure_set(sun, planet, ring).closure() {
                    found.push((ring, x[1]));
                }
            }
            assert!(!found.is_empty(), "z={sun}/{planet}: nothing admissible");
            let (first, last) = (found[0].0, found.last().unwrap().0);
            assert_eq!(
                found.iter().map(|f| f.0).collect::<Vec<_>>(),
                (first..=last).collect::<Vec<u32>>(),
                "z={sun}/{planet}: the admissible run has a hole in it"
            );
            assert!(
                found.iter().any(|f| f.0 == ideal),
                "z={sun}/{planet}: {ideal} missing"
            );
            for w in found.windows(2) {
                assert!(
                    w[1].1 > w[0].1,
                    "z={sun}/{planet}: the shift fell from {:?} to {:?}",
                    w[0],
                    w[1]
                );
            }
        }
        // The 17/17 run the set's solver named: 48 to 54.
        let run: Vec<u32> = (40..=70)
            .filter(|&z| closure_set(17, 17, z).closure().is_ok())
            .collect();
        assert_eq!(run, (48..=54).collect::<Vec<u32>>());
    }

    /// **The ratio with one more tooth is the graph's own**, member by
    /// member: on a pair the wheel's tooth moves it by exactly `1/z₁`, the
    /// pinion's by the classical quotient; on a set with its ring held the
    /// planet's count moves the ratio not at all.
    #[test]
    fn one_more_tooth_moves_the_ratio_as_the_graph_says() {
        let lib = test_library();
        let pair = arr::pair([17, 43]);
        let r = solve_set(&pair, 2.0, 3000.0, &lib).unwrap();
        let per = |i: usize| r.ratio_per_tooth.as_ref().unwrap()[i].unwrap();
        assert!((per(1) + 44.0 / 17.0).abs() < 1e-12);
        assert!((per(0) + 43.0 / 18.0).abs() < 1e-12);
        assert!(
            (r.circulation.unwrap().forward - 1.0).abs() < 1e-12,
            "a pair passes it all once"
        );
        let set = arr::planetary(12, 30, 72, 3);
        let r = solve_set(&set, 2.0, 3000.0, &lib).unwrap();
        assert!((r.ratio.unwrap() - 7.0).abs() < 1e-12);
        let per = |i: usize| r.ratio_per_tooth.as_ref().unwrap()[i].unwrap();
        assert!(
            (per(0) - (1.0 + 72.0 / 13.0)).abs() < 1e-12,
            "a sun's tooth"
        );
        assert!(
            (per(1) - 7.0).abs() < 1e-12,
            "a planet's tooth moves nothing"
        );
        assert!(
            (per(2) - (1.0 + 73.0 / 12.0)).abs() < 1e-12,
            "a ring's tooth"
        );
        // A tooth that locks the stage is no figure: a Wolfrom's held ring
        // brought level with its output ring stops the output.
        let wolfrom = solve_set(
            &super::super::arrangements::wolfrom(18, [60, 61], 3),
            2.0,
            3000.0,
            &lib,
        )
        .unwrap();
        // Members: the planet, ring 1, ring 2.
        let per = wolfrom.ratio_per_tooth.as_ref().unwrap();
        assert_eq!(per[1], None, "ring 1 at 61: locked");
        assert!(
            per[2].is_some_and(|x| (x - 31.0).abs() < 1e-9),
            "ring 2 at 62: 62/2"
        );
        // ...and the power through a set's sun mesh is under the power in,
        // the carrier carrying the rest bodily: with the ring held, the
        // fraction the sun turns against the carrier — and the same power,
        // less that mesh's loss, crosses the ring mesh after it.
        let (sun_mesh, ring_mesh) = (&r.meshes[0], &r.meshes[1]);
        assert!((sun_mesh.cases[0].power_through - 6.0 / 7.0).abs() < 1e-9);
        assert!(
            (ring_mesh.cases[0].power_through - 6.0 / 7.0 * sun_mesh.efficiency.forward).abs()
                < 1e-9
        );
        assert!(
            (r.circulation.unwrap().forward
                - (sun_mesh.cases[0].power_through + ring_mesh.cases[0].power_through))
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn every_arrangement_of_a_set_solves() {
        use crate::planetary::{Arrangement, PlanetaryShaft};
        let set = arr::planetary(12, 30, 72, 3);
        let mut checked = 0;
        for input in PlanetaryShaft::ALL {
            for fixed in PlanetaryShaft::ALL {
                if input == fixed {
                    continue;
                }
                let arrangement = Arrangement { input, fixed };
                let r = crate::train::solve_alone(
                    &crate::train::Train::alone(&set, 2.0, 3000.0).arranged_as(arrangement),
                    &test_library(),
                )
                .unwrap();
                let want = crate::planetary::power(
                    crate::planetary::basic_ratio(crate::planetary::Teeth {
                        sun: 12,
                        planet: 30,
                        ring: 72,
                    }),
                    arrangement,
                    1.0,
                    1.0,
                    1.0,
                )
                .unwrap();
                assert!(
                    (r.ratio.unwrap() - want.ratio).abs() < 1e-12,
                    "{arrangement:?}: {} vs {}",
                    r.ratio.unwrap(),
                    want.ratio
                );
                assert!(
                    r.efficiency.unwrap().forward > 0.9 && r.efficiency.unwrap().forward < 1.0,
                    "{arrangement:?}"
                );
                assert!(
                    r.efficiency.unwrap().backward > 0.9 && r.efficiency.unwrap().backward < 1.0,
                    "{arrangement:?}"
                );
                assert!(r.backlash.unwrap().forward.nominal > 0.0);
                checked += 1;
            }
        }
        assert_eq!(checked, 6);
    }

    // ---- the set's laws, ported from its own solver ----

    /// **A probe width leaves no trace.**
    ///
    /// An epicyclic set rates once at `PROBE` and scales to the width each mesh
    /// carries — bending inversely with the width, contact with its square root
    /// (`Loading::at_width`). So the stress it reports must be the stress a
    /// direct evaluation at that width gives, and this asks for one.
    ///
    /// **Nothing asked before, and the reason is a coincidence of two
    /// constants**: `PROBE` is 10.0 and `MemberGear`'s default face width is
    /// 10.0, so every shipped case scales by exactly one and the exponent could
    /// be anything. Perturbing it to 0.51 left all 558 tests and all 27 golden
    /// files unchanged. *Two unrelated numbers that happen to be equal will hide
    /// whatever lies between them.*
    ///
    /// Widths well away from the probe on both sides, so a scale that is wrong
    /// in either direction shows.
    #[test]
    fn a_probe_width_leaves_no_trace() {
        let lib = test_library();
        let mut checked = 0u32;
        for face in [2.5_f64, 10.0, 40.0] {
            let stage = {
                let mut s = arr::planetary(12, 30, 72, 3);
                for m in &mut s.members {
                    m.gear.face_width = Auto::fixed(face);
                }
                s
            };
            let r =
                solve_set(&stage, 2.0, 0.0, &lib).unwrap_or_else(|e| panic!("face {face}: {e}"));
            let b = stage
                .build_at(
                    &r.members
                        .iter()
                        .map(|m| m.profile_shift)
                        .collect::<Vec<_>>(),
                )
                .expect("the set has geometry");

            // The sun mesh, evaluated where it is carried rather than scaled
            // there. Same `contact_stress`; what it does not share is the
            // scaling under test. The sun's torque is its tooth load per
            // planet path already.
            let load = Load::new(r.members[0].cases[0].torque, face);
            let e_star = contact_modulus(
                &lib.get(&stage.members[0].gear.material)
                    .expect("a material")
                    .clone(),
                &lib.get(&stage.members[1].gear.material)
                    .expect("a material")
                    .clone(),
            );
            let direct = contact_stress(
                &b.meshes[0].line().unwrap().path,
                &b.meshes[0].line().unwrap().operating,
                b.members[0].as_gear(),
                PARALLEL_AXES,
                &load,
                e_star,
            )
            .expect("the sun mesh has contact");

            let got = r.members[0].cases[0].contact_stress;
            assert!(
                (got - direct.governing(0)).abs() < 1e-9 * direct.governing(0),
                "face {face}: the sun reports {got} MPa where a direct evaluation \
                 at that width gives {}",
                direct.governing(0)
            );
            checked += 1;
        }
        assert_eq!(checked, 3, "a width went unrun");
    }

    /// **A planet's root answers to both of its meshes.**
    ///
    /// The sun loads one flank and the ring the other, and it had only the
    /// sun's. The two tangential forces are equal — it is the same planet
    /// transmitting through — so what separates them is the section each mesh's
    /// contact ratio puts the load at, and the **width each mesh carries it
    /// over**: the narrower of that pair's two faces. A narrow ring is the
    /// ordinary way for the second mesh to be the worse one, and it is what the
    /// second fixture is.
    ///
    /// Both orderings are exercised deliberately. A test that only ever met the
    /// sun-governed case would pass against a stage that had gone back to
    /// looking at one mesh, which is exactly what the first draft of this did.
    #[test]
    fn a_planets_root_answers_to_both_of_its_meshes() {
        let lib = test_library();
        let mut sun_won = false;
        let mut ring_won = false;
        for ring_face in [10.0_f64, 3.0] {
            let stage = {
                let mut s = stage_of(24, 18, 60, 0.0);
                s.members[2].gear.face_width = Auto::fixed(ring_face);
                s
            };
            let r = solve_set(&stage, 2.0, 0.0, &lib)
                .unwrap_or_else(|e| panic!("ring face {ring_face}: {e}"));
            let got = r.members[1].cases[0]
                .bending_stress
                .expect("a planet has a root section");

            // The two contributions, rebuilt from what the result reports rather
            // than from the expression that produced them: each mesh's own
            // section under its own load, over the width that mesh carries.
            let built = stage
                .build_at(
                    &r.members
                        .iter()
                        .map(|m| m.profile_shift)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let planets = f64::from(stage.axes[1].count);
            let sp_width = r.members[1].face_width.min(r.members[0].face_width);
            let pr_width = r.members[1].face_width.min(r.members[2].face_width);
            let planet = built.members[1].as_gear();
            let each = |contact_ratio: f64, torque: f64, b: f64| {
                let section = crate::strength::bending_section(planet, contact_ratio).unwrap();
                bending_stress(
                    &section,
                    Load::new(torque, b).tangential(planet),
                    b,
                    RootStressModel::DolanBroghamer,
                    None,
                )
                .unwrap()
            };
            // The bodies: ground, sun, carrier, ring, planet. The sun's torque
            // per planet path read across the sun mesh presses the planet;
            // the ring's, per path, is `η` less than the planet pressed it
            // with, read back across the ring mesh.
            let from_sun = each(
                built.meshes[0].line().unwrap().path.contact_ratio,
                Load::new((r.cases[0].torques[1] / planets).abs(), sp_width)
                    .across_mesh(built.members[0].as_gear(), planet)
                    .torque,
                sp_width,
            );
            let from_ring = each(
                built.meshes[1].line().unwrap().path.contact_ratio,
                (r.cases[0].torques[3] / planets).abs()
                    / built.meshes[1].line().unwrap().operating.ratio()
                    / r.meshes[1].efficiency.forward,
                pr_width,
            );
            assert!(
                (got - from_sun.max(from_ring)).abs() < 1e-9 * got,
                "ring face {ring_face}: reported {got}, sun {from_sun}, ring {from_ring}"
            );
            sun_won |= from_sun > from_ring;
            ring_won |= from_ring > from_sun;
        }
        assert!(
            sun_won && ring_won,
            "one fixture each way, or the max is never asked a question"
        );
    }

    /// **A ring that cannot be rated for bending costs the rating, not the set.**
    ///
    /// A planetary set gives its ring `k = 2 − k_stage`, so an ordinary stage
    /// thickness modification of 1.4 puts the ring at 0.6 — thick enough that
    /// the cutter which would leave its space comes to a point before its own
    /// tip, and no fillet is generated. There is then no notch, so no `Y_S`, so
    /// no bending number for that member.
    ///
    /// None of which stops the set existing. The geometry draws, exports and
    /// meshes, and the ratio, both contact stresses, the efficiencies, the
    /// cycles and the other two members' bending are all still answerable. A
    /// stage that threw those away over one missing input would be deciding for
    /// the designer rather than informing them, which is the opposite of what
    /// this tool is for.
    ///
    /// Run against the code this replaced, every case below is
    /// `Err(NoRootSection)` — and the message blamed undercut, which nothing
    /// here is.
    #[test]
    fn a_ring_with_no_notch_costs_its_bending_rather_than_the_stage() {
        let lib = test_library();
        for k in [1.3_f64, 1.4, 1.5, 1.7] {
            let stage = {
                let mut s = arr::planetary(12, 30, 72, 3);
                s.members[0].thickness_mod = Auto::fixed(k);
                for m in &mut s.members[1..] {
                    m.thickness_mod = Auto::automatic(2.0 - k);
                }
                s
            };
            let r = solve_set(&stage, 2.0, 0.0, &lib)
                .unwrap_or_else(|e| panic!("k={k}: the set should still solve, got {e}"));

            assert!(
                r.members[2].cases[0].bending_stress.is_none(),
                "k={k}: a ring with no notch cannot have a bending stress"
            );
            // ...and everything that never needed the notch is still there.
            assert!(
                r.ratio.unwrap().is_finite() && r.ratio.unwrap() != 0.0,
                "k={k}: no ratio"
            );
            assert!(
                r.members[0].cases[0].bending_stress.is_some(),
                "k={k}: the sun's own bending went with it"
            );
            for (name, gear) in [("sun", &r.members[0]), ("ring", &r.members[2])] {
                assert!(
                    gear.cases[0].contact_stress > 0.0,
                    "k={k}: {name} lost its contact stress"
                );
            }
            // The member says why, so the blank is read rather than guessed at.
            assert!(
                !r.members[2].clamps.is_empty(),
                "k={k}: the ring reports no reason for having no fillet"
            );
        }
    }

    fn stage_of(sun: u32, planet: u32, ring: u32, helix: f64) -> Shape {
        let mut s = arr::planetary(sun, planet, ring, 3);
        s.members[0].gear.helix_angle = Auto::fixed(helix);
        s
    }

    fn solved(sun: u32, planet: u32, ring: u32) -> crate::train::Alone {
        solve_set(
            &stage_of(sun, planet, ring, 0.0),
            2.0,
            3000.0,
            &test_library(),
        )
        .unwrap()
    }

    /// **With a distance given, only one shift is free** — asserted against what
    /// the set actually does, not against the number the declaration carries.
    ///
    /// A set's two distances must agree, which is one relation among its three
    /// shifts. Give it a distance as well and there is a second — each mesh must
    /// reach *that* distance — so one shift is a design and two are absorbed.
    /// `Shape::freedoms` says so by reading the distance's toggle, and this is
    /// what makes that reading true rather than declared: give **two** shifts at
    /// a given distance and one of them cannot survive.
    ///
    /// Checking the declaration against the declaration is what
    /// `docs/corrections.md` calls a check built from the thing under test. The
    /// first version of this did exactly that and passed against a limit hard
    /// -wired to the wrong number.
    #[test]
    fn a_given_distance_leaves_a_set_one_free_shift() {
        let lib = super::super::test_library();
        let base = arr::planetary(12, 30, 72, 3);
        let free = solve_set(&base, 2.0, 0.0, &lib).expect("the shipped set solves");
        let asked = free.distances[0].running + 0.1;

        // One shift given — the ring's, as the shipped set has it — and every
        // given number stands.
        let mut one = base.clone();
        one.distances[0].distance = Auto::fixed(asked);
        let r = solve_set(&one, 2.0, 0.0, &lib).expect("one free shift is enough");
        assert!((r.distances[0].running - asked).abs() < 1e-9);
        assert!(
            (r.members[2].profile_shift - one.members[2].gear.profile_shift.manual).abs() < 1e-12
        );

        // A second shift given, and it cannot also stand: three relations'
        // worth of demands on two freedoms. Every input is honoured as
        // typed — a given shift is not the solve's to move, and the set's
        // kind used to move one silently — and the mesh whose sum nothing
        // was left to reach says it did not reach the distance: the planet
        // reaches the sun mesh's, and the ring mesh is left with the ring
        // and the planet both decided.
        let mut two = one.clone();
        two.members[0].gear.profile_shift = Auto::fixed(r.members[0].profile_shift + 0.25);
        let over = solve_set(&two, 2.0, 0.0, &lib)
            .expect("it still builds; it just cannot honour everything");
        assert!((over.distances[0].running - asked).abs() < 1e-9);
        assert!(
            (over.members[0].profile_shift - two.members[0].gear.profile_shift.manual).abs() < 1e-9
        );
        assert!(
            (over.members[2].profile_shift - two.members[2].gear.profile_shift.manual).abs() < 1e-9
        );
        assert!(
            over.notes
                .iter()
                .any(|n| n.is(key::PART_DISTANCE_NOT_REACHED)),
            "the mesh whose sum nothing reached says so: {:?}",
            over.notes
        );
        let ring_mesh_wants =
            MeshKind::Internal.nominal_of(asked, two.distances[0].clearance.manual);
        assert!(
            (over.distances[0].nominal[1] - ring_mesh_wants).abs() > 1e-3,
            "and the ring mesh does not run at the clearance asked: {} vs {ring_mesh_wants}",
            over.distances[0].nominal[1]
        );

        // ...and the declaration says the same thing: one relation per
        // mesh. The sun's mesh relates the distance, its two shifts, the
        // clearance and the size, four of five given at most; the ring's
        // mesh, which absorbs on the distance the first defines, relates
        // only the shifts that can absorb for it — the planet first, with
        // twice anyone's leverage, then the ring, then the sun — all but
        // one given; so a set pinned whole keeps the distance, the
        // clearance, the size and two shifts, the planet's absorbing.
        use super::super::{Freedom, MemberFreedom};
        let groups = one.freedoms();
        let shift = |i: usize| vec![Freedom::Member(i, MemberFreedom::Shift)];
        let first = groups
            .iter()
            .find(|g| g.order[0] == vec![Freedom::Distance(0)])
            .expect("the sun's mesh declares its relation");
        assert_eq!(first.given_at_most, 4);
        assert_eq!(first.order.len(), 5);
        assert_eq!(first.order[3], vec![Freedom::Clearance(0)]);
        let second = groups
            .iter()
            .find(|g| g.order[0] == shift(1))
            .expect("the ring's mesh declares its relation, the planet first");
        assert_eq!(second.order.len(), 3);
        assert_eq!(second.given_at_most, 2);
        assert_eq!(second.order[1], shift(2));
        assert_eq!(second.order[2], shift(0));
    }

    /// **A set runs at the centre distance it was given**, and both of its
    /// meshes do — which is the whole of F39's third item.
    ///
    /// A planetary set had no distance input at all: the common distance was
    /// whatever the shifts left. Given one, each mesh has a shift sum it must
    /// reach to run at it, and both of those are closed form — so a target makes
    /// the layout *easier* and the Newton iteration disappears.
    ///
    /// Three claims. The distance asked for is the distance run at; the two
    /// meshes agree there to the bit (`residual`, which is the layout's own
    /// measure of whether it closed); and a shift the designer **gave** is
    /// untouched, since with one freedom left it is the freedom.
    #[test]
    fn a_set_runs_at_the_centre_distance_it_was_given() {
        let lib = super::super::test_library();
        let base = arr::planetary(12, 30, 72, 3);
        let free = solve_set(&base, 2.0, 0.0, &lib).expect("the shipped set solves");

        let mut checked = 0u32;
        for step in -2..=4 {
            let asked = free.distances[0].running + 0.2 * f64::from(step);
            let mut stage = base.clone();
            stage.distances[0].distance = Auto::fixed(asked);
            let Ok(r) = solve_set(&stage, 2.0, 0.0, &lib) else {
                // A distance no set can reach is refused, not answered — which
                // is the honest end of the range rather than a gap in it.
                continue;
            };
            checked += 1;

            assert!(
                (r.distances[0].running - asked).abs() < 1e-9,
                "asked {asked}, ran at {}",
                r.distances[0].running
            );
            assert!(
                (r.distances[0].clearance - stage.distances[0].clearance.manual).abs() < 1e-9,
                "the clearance asked for should be the clearance left: {}",
                r.distances[0].clearance
            );

            // Both meshes at that distance, measured from their own
            // zero-backlash distances rather than from the expression that
            // placed the shifts: opened by the clearance each its own way,
            // they must land on one running distance.
            let residual = (r.distances[0].nominal[0] + stage.distances[0].clearance.manual
                - (r.distances[0].nominal[1] - stage.distances[0].clearance.manual))
                .abs();
            assert!(
                residual < 1e-9,
                "the two meshes disagree by {residual} at a given distance"
            );

            // The ring's shift is given on the shipped set, so it is the one
            // freedom a target leaves and must come back untouched.
            assert!(
                (r.members[2].profile_shift - stage.members[2].gear.profile_shift.manual).abs()
                    < 1e-12,
                "a given shift moved: {} for {}",
                r.members[2].profile_shift,
                stage.members[2].gear.profile_shift.manual
            );
        }
        assert!(checked >= 5, "only {checked} distances were reachable");
    }

    /// **The constraint that makes it a planetary set**: sun-to-planet and
    /// planet-to-ring are one distance measured twice, and the planet's shift is
    /// what makes them agree.
    #[test]
    fn the_two_centre_distances_are_one_number() {
        for (s, p, r) in [
            (24u32, 18u32, 60u32),
            (17, 17, 52),
            (20, 20, 62),
            (30, 15, 62),
        ] {
            let res = solved(s, p, r);
            assert!(
                residual(&res) < 1e-12,
                "z={s}/{p}/{r}: residual {} mm",
                residual(&res)
            );
            assert!(res.distances[0].running > 0.0);
        }
        // The ideal ring needs no shift at all to *agree* — and gets exactly
        // none at no clearance. The shipped 0.02 mm is then all that moves the
        // planet: thinned by that much it opens both meshes with the planets
        // where they always were, which is why the running distance stays at
        // the ideal 21 to well under a micron while the two zero-backlash
        // distances part by twice the clearance.
        let lib = test_library();
        let mut exact = stage_of(24, 18, 60, 0.0);
        exact.distances[0].clearance = Auto::fixed(0.0);
        let exact = solve_set(&exact, 2.0, 0.0, &lib).unwrap();
        assert!(exact.members[1].profile_shift.abs() < 1e-12);
        assert!(exact.distances[0]
            .nominal
            .iter()
            .all(|a| (a - 21.0).abs() < 1e-12));

        let ideal = solved(24, 18, 60);
        let c = ideal.distances[0].clearance;
        assert!(c > 0.0, "the shipped set has a running clearance");
        assert!(
            ideal.members[1].profile_shift < 0.0,
            "{}",
            ideal.members[1].profile_shift
        );
        let (ext, int) = (ideal.distances[0].nominal[0], ideal.distances[0].nominal[1]);
        assert!((ideal.distances[0].running - ext - c).abs() < 1e-12);
        assert!((int - ideal.distances[0].running - c).abs() < 1e-12);
        assert!((ideal.distances[0].running - 21.0).abs() < 1e-3);
    }

    /// The classical ratios, through the whole stage rather than the bare
    /// algebra — so a wiring error between them would show.
    /// **Any one of the three shifts can close the set**, and the other two are
    /// then exactly what was asked for.
    ///
    /// The relation is that the two centre distances agree, so whichever member
    /// absorbs it, the answer has to satisfy the same equality — which is what
    /// `residual` reports and what this checks rather than checking the wiring.
    /// The two that did not absorb must come back untouched, since a shift a
    /// designer gave is not the solve's to move.
    #[test]
    fn whichever_shift_is_left_automatic_is_the_one_that_closes_the_set() {
        let lib = test_library();
        let base = arr::planetary(12, 30, 72, 3);
        // The shifts the default set settles at, so each variant below asks for
        // values a set of these counts can actually be built at.
        let settled = base.shifts_at(&crate::auto::Search::SHIPPED);

        for absorber in 0..3 {
            let mut s = arr::planetary(12, 30, 72, 3);
            // Pin every member but the one meant to absorb.
            for (i, m) in s.members.iter_mut().enumerate() {
                m.gear.profile_shift = if i == absorber {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(settled[i])
                };
            }
            let plan = s.plan(&s.helix_angles());
            assert_eq!(
                plan.role[absorber],
                Role::Absorbs(0),
                "the member left automatic should be the one that absorbs"
            );
            let r = solve_set(&s, 2.0, 0.0, &lib)
                .unwrap_or_else(|e| panic!("{absorber:?} could not close the set: {e:?}"));

            // The equality actually closed...
            assert!(
                residual(&r) < 1e-9,
                "{absorber:?}: the two centre distances differ by {}",
                residual(&r)
            );
            // ...and the given members were left exactly as given.
            let got = [
                r.members[0].profile_shift,
                r.members[1].profile_shift,
                r.members[2].profile_shift,
            ];
            for i in 0..3 {
                if i == absorber {
                    continue;
                }
                assert!(
                    (got[i] - settled[i]).abs() < 1e-9,
                    "{absorber:?}: member {i} was given {} and came back {}",
                    settled[i],
                    got[i]
                );
            }
        }
    }

    /// **The default set is the planet's**, which is what it has always been:
    /// the member in both meshes absorbs by preference, then whichever is
    /// left automatic — and with nothing left automatic, nothing absorbs and
    /// the set is refused for it unless its distances happen to agree.
    #[test]
    fn the_planet_closes_the_set_unless_it_is_pinned() {
        let role_of = |shape: &Shape| shape.plan(&shape.helix_angles()).role;
        assert_eq!(role_of(&arr::planetary(12, 30, 72, 3))[1], Role::Absorbs(0));
        let mut s = arr::planetary(12, 30, 72, 3);
        s.members[1].gear.profile_shift = Auto::fixed(0.0);
        assert_eq!(
            role_of(&s)[0],
            Role::Absorbs(0),
            "pinning the planet hands it to the sun"
        );
        s.members[0].gear.profile_shift = Auto::fixed(0.0);
        // The shipped ring's shift is given, so with the other two pinned as
        // well nothing is left automatic and the set is over-specified: the
        // panel relieves it as it is created, and a document that reaches
        // this state is refused with the distances' own reason.
        assert!(role_of(&s).iter().all(|r| *r == Role::Given));
        assert_eq!(
            solve_set(&s, 2.0, 0.0, &test_library()).err(),
            Some(TrainError::NoCommonDistance)
        );
        s.members[2].gear.profile_shift = Auto::automatic(0.0);
        assert_eq!(
            role_of(&s)[2],
            Role::Absorbs(0),
            "...and a ring left free takes it instead"
        );
    }

    #[test]
    fn the_stage_reports_the_classical_ratios() {
        let want = [
            (
                PlanetaryShaft::Sun,
                PlanetaryShaft::Ring,
                PlanetaryShaft::Carrier,
                3.5,
            ),
            (
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
                PlanetaryShaft::Ring,
                -2.5,
            ),
            (
                PlanetaryShaft::Ring,
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
                1.4,
            ),
        ];
        for (input, fixed, output, ratio) in want {
            // The same stage, asked six things.
            let stage = stage_of(24, 18, 60, 0.0);
            let r = crate::train::solve_alone(
                &crate::train::Train::alone(&stage, 2.0, 0.0)
                    .arranged_as(Arrangement { input, fixed }),
                &test_library(),
            )
            .unwrap();
            let _ = output;
            assert!(
                (r.ratio.unwrap() - ratio).abs() < 1e-12,
                "{input:?}/{fixed:?}: {}",
                r.ratio.unwrap()
            );
        }
    }

    /// **A held carrier makes the set two meshes in series**, so its efficiency
    /// must be exactly the product of theirs — through the stage, not just the
    /// algebra.
    #[test]
    fn a_held_carrier_gives_exactly_the_product_of_the_mesh_efficiencies() {
        let stage = stage_of(24, 18, 60, 0.0);
        let carrier_held = Arrangement {
            input: PlanetaryShaft::Sun,
            fixed: PlanetaryShaft::Carrier,
        };
        let r = crate::train::solve_alone(
            &crate::train::Train::alone(&stage, 2.0, 0.0).arranged_as(carrier_held),
            &test_library(),
        )
        .unwrap();
        let product = r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward;
        assert!(
            (r.efficiency.unwrap().forward - product).abs() < 1e-12,
            "{}",
            r.efficiency.unwrap().forward
        );
    }

    /// **The internal mesh is the gentler one**, in both the ways it should be:
    /// more contact and less pressure. Both were proved as laws in `ring.rs`;
    /// asserting them here says the stage wired the two meshes the right way
    /// round, which no amount of core testing would catch.
    #[test]
    fn the_internal_mesh_carries_better_than_the_external_one() {
        for (s, p, r) in [(24u32, 18u32, 60u32), (17, 17, 52), (30, 15, 62)] {
            let res = solved(s, p, r);
            assert!(
                res.meshes[1].line.unwrap().contact_ratios.transverse
                    > res.meshes[0].line.unwrap().contact_ratios.transverse,
                "z={s}/{p}/{r}: internal contact ratio {} not above external {}",
                res.meshes[1].line.unwrap().contact_ratios.transverse,
                res.meshes[0].line.unwrap().contact_ratios.transverse
            );
            assert!(
                res.meshes[1].cases[0].contact.curvature_across
                    < res.meshes[0].cases[0].contact.curvature_across,
                "z={s}/{p}/{r}: internal relative radius should be the larger"
            );
            // ...and a ring's tooth is the stronger, so it carries the less
            // bending stress. Every member is rated: a ring's critical section
            // sits on its involute flank for most tooth counts, and that used to
            // withhold the figure entirely — see
            // `the_rating_is_continuous_across_the_flank_fillet_transition`.
            let (sun_s, ring_s) = (
                res.members[0].cases[0]
                    .bending_stress
                    .expect("the sun is always rated"),
                res.members[2].cases[0]
                    .bending_stress
                    .expect("and so is the ring"),
            );
            assert!(
                ring_s < sun_s,
                "z={s}/{p}/{r}: ring {ring_s} vs sun {sun_s}"
            );
        }
    }

    /// **The backlash referral, against the kinematics.**
    ///
    /// The same play measured at two different output bodies must differ by
    /// exactly the ratio between them — and those ratios come from
    /// `planetary::power`, which shares none of the referral's algebra. That is
    /// what makes this a check rather than a restatement.
    ///
    /// It is also the law the train-level test uses on a multi-stage train
    /// ("backlash at the two ends differs by exactly the total ratio"), asked of
    /// one stage with three bodies instead of a line of two-body ones.
    #[test]
    fn backlash_referred_to_two_shafts_differs_by_exactly_their_ratio() {
        let lib = test_library();
        for (s, p, r) in [(24u32, 18u32, 60u32), (17, 17, 52), (30, 15, 62)] {
            // Ring held: the sun and the carrier are the two possible outputs.
            let stage = stage_of(s, p, r, 0.0);
            let asked = |input| {
                let arrangement = Arrangement {
                    input,
                    fixed: PlanetaryShaft::Ring,
                };
                crate::train::solve_alone(
                    &crate::train::Train::alone(&stage, 2.0, 0.0).arranged_as(arrangement),
                    &lib,
                )
                .unwrap()
            };
            let a = asked(PlanetaryShaft::Sun);
            let b = asked(PlanetaryShaft::Carrier);

            // `a` outputs at the carrier, `b` at the sun.
            let at_carrier = a.backlash.unwrap().forward.nominal;
            let at_sun = b.backlash.unwrap().forward.nominal;
            assert!(at_carrier > 0.0 && at_sun > 0.0);
            assert!(
                (at_sun - at_carrier * a.ratio.unwrap()).abs() < 1e-9 * at_sun,
                "z={s}/{p}/{r}: {at_sun} vs {at_carrier} x {}",
                a.ratio.unwrap()
            );
            // ...and the body that turns faster carries the looser play.
            assert!(at_sun > at_carrier);
        }
    }

    /// Both meshes contribute, and more play in either loosens the output.
    ///
    /// A referral that dropped one mesh would still satisfy the ratio law above,
    /// since that law is about *where* the play is measured rather than where it
    /// came from — so it needs saying separately.
    #[test]
    fn both_meshes_contribute_to_the_output_backlash() {
        let lib = test_library();
        let base = stage_of(24, 18, 60, 0.0);
        let tight = solve_set(&base, 2.0, 0.0, &lib).unwrap();

        // More clearance opens both meshes, so the output must loosen.
        let loose = {
            let mut s = base.clone();
            s.distances[0].clearance = Auto::fixed(base.distances[0].clearance.manual + 0.05);
            s
        };
        let loose = solve_set(&loose, 2.0, 0.0, &lib).unwrap();
        assert!(
            loose.backlash.unwrap().forward.nominal > tight.backlash.unwrap().forward.nominal,
            "{} should exceed {}",
            loose.backlash.unwrap().forward.nominal,
            tight.backlash.unwrap().forward.nominal
        );

        // And the tolerance band holds the nominal. On the ideal ring it is a
        // point — the referred play is invariant in the running distance, the
        // sun mesh gaining exactly what the ring mesh loses (the law is in
        // `train::tests::a_tolerance_band_widens_with_the_centre_distance`) —
        // so a set one tooth off the ideal is what shows the band opening, and
        // it opens on both sides of the nominal since the two meshes' operating
        // angles no longer move together.
        let b = &tight.backlash.unwrap().forward;
        assert!(b.minimum <= b.nominal && b.nominal <= b.maximum);
        let off = stage_of(24, 18, 61, 0.0);
        let off = solve_set(&off, 2.0, 0.0, &lib).unwrap();
        let b = &off.backlash.unwrap().forward;
        assert!(
            b.minimum < b.nominal && b.nominal < b.maximum,
            "off the ideal ring the band opens: {} … {} … {}",
            b.minimum,
            b.nominal,
            b.maximum
        );

        // At the zero-backlash centre distance there is no play at all.
        let exact = {
            let mut s = base.clone();
            s.distances[0].clearance = Auto::fixed(0.0);
            s.distances[0].tolerance_plus = 0.0;
            s.distances[0].tolerance_minus = 0.0;
            s
        };
        let exact = solve_set(&exact, 2.0, 0.0, &lib).unwrap();
        assert!(
            exact.backlash.unwrap().forward.nominal < 1e-12,
            "zero clearance must give zero play, got {}",
            exact.backlash.unwrap().forward.nominal
        );
    }

    /// The planet turns at a speed measured **relative to the carrier**, which
    /// is what its teeth actually see.
    #[test]
    fn the_planet_is_reported_as_the_special_case_it_is() {
        let r = solved(24, 18, 60);
        let c = &r.members[1].cases[0];
        assert!(c.speed_against_carrier.abs() > 0.0);
        assert!((c.speed_against_carrier - c.speed).abs() > 1e-9);
    }

    /// **A planet's root is loaded both ways, and what to do about it is asked
    /// rather than assumed.**
    ///
    /// The derate is a convention — a fraction on an allowable a part is sized
    /// against — so it is a switch, off by default, and the stage says which of
    /// its members the reversal reaches either way. It used to be applied to the
    /// planet silently, and to the planet alone, so a reversing *drive* derated
    /// nothing while a set nobody had told anything about derated one member.
    #[test]
    fn a_reversed_root_is_corrected_only_when_the_train_asks() {
        let lib = test_library();
        let stage = arr::planetary(12, 30, 72, 3);
        // A plain case reverses nothing; the reversing duty is the fatigue
        // case's own, so the second solve hands the stage one — a whole turn
        // of its input, back and forth once.
        let solve = |reversal: crate::train::Reversal, reversing: bool| {
            let mut train = crate::train::Train::alone(&stage, 2.0, 0.0).with_reversal(reversal);
            let input = train.load_cases[1].loads[0].at;
            train.load_cases[1].duty = crate::train::Duty::Intermittent {
                range_degrees: 360.0,
                at: input,
                actuations: 1,
                reversing,
            };
            crate::train::solve_alone(&train, &lib).unwrap()
        };
        // **On the member, not the stage.** Three members raising one note is
        // exactly what a stage-level list could not carry: one key, three
        // entries, and a keyed list in the front end that cannot draw it.
        let fired = |r: &ShapeResult, k: &str| {
            [&r.members[0], &r.members[1], &r.members[2]]
                .iter()
                .filter(|g| g.notes.iter().any(|n| n.is(k)))
                .count()
        };

        // Off: nothing is derated, and the planet's reversal is disclosed.
        let plain = solve(crate::train::Reversal::default(), false);
        assert_eq!(fired(&plain, key::GEAR_REVERSED_BENDING_UNCORRECTED), 1);
        assert_eq!(fired(&plain, key::GEAR_REVERSED_BENDING_APPLIED), 0);

        // On: the same member is derated, and the note says so instead.
        let corrected = solve(crate::train::Reversal { correct: true }, false);
        assert_eq!(fired(&corrected, key::GEAR_REVERSED_BENDING_APPLIED), 1);
        assert_eq!(fired(&corrected, key::GEAR_REVERSED_BENDING_UNCORRECTED), 0);

        // A smaller allowable asks for more face, and only for the planet.
        let width = |r: &ShapeResult, g: &GearResult| {
            let _ = r;
            g.cases[1].min_face_width.bending.unwrap()
        };
        assert!(
            width(&corrected, &corrected.members[1]) > width(&plain, &plain.members[1]) * 1.2,
            "the correction must reach the planet's minimum width"
        );
        for (name, a, b) in [
            ("sun", &corrected.members[0], &plain.members[0]),
            ("ring", &corrected.members[2], &plain.members[2]),
        ] {
            assert_eq!(
                width(&corrected, a).to_bits(),
                width(&plain, b).to_bits(),
                "{name}: a one-way root must not be derated"
            );
        }

        // A reversing **drive** reverses all three, and does not stack with the
        // planet's own — which is the whole point of asking `reverses` once.
        let driven = solve(crate::train::Reversal { correct: true }, true);
        assert_eq!(fired(&driven, key::GEAR_REVERSED_BENDING_APPLIED), 3);
        // ...and no member's own list carries one note twice, which is the shape
        // that broke the panel: a keyed list cannot draw two of one key.
        for g in &driven.members {
            let mut keys: Vec<&str> = g.notes.iter().map(|n| n.key.as_str()).collect();
            let before = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(before, keys.len(), "a member repeated a note key");
        }
        assert_eq!(
            width(&driven, &driven.members[1]).to_bits(),
            width(&corrected, &corrected.members[1]).to_bits(),
            "a reversing duty cannot make a planet more reversed than it is"
        );
    }

    /// Layout is arithmetic on the tooth counts, and it reaches the result.
    #[test]
    fn the_layout_checks_reach_the_result() {
        let r = solved(24, 18, 60);
        let layout = r.layouts.first().expect("three planets have a layout");
        assert_eq!(layout.equal_spacing, Some(true), "(24+60)/3 = 28");
        assert!(layout.clearance > 0.0);
        assert!(layout.clearance_ok);

        // A single planet has no neighbour to clear, and says so rather than
        // reporting a gap of nothing.
        let one = {
            let mut s = stage_of(24, 18, 60, 0.0);
            s.axes[1].count = 1;
            s
        };
        let r = solve_set(&one, 2.0, 0.0, &test_library()).unwrap();
        assert!(r.layouts.is_empty(), "one planet has no layout to check");
    }

    /// **Helical works, to parity with spur.** Every figure a spur set reports,
    /// a helical one reports too — including the ring's bending, which goes
    /// through the virtual spur ring.
    #[test]
    fn a_helical_set_reports_everything_a_spur_one_does() {
        for helix in [10.0, 20.0, 30.0] {
            let stage = stage_of(24, 18, 60, helix);
            let r = solve_set(&stage, 2.0, 0.0, &test_library())
                .unwrap_or_else(|e| panic!("helix={helix}: {e}"));
            assert!(
                r.members[0].cases[0].bending_stress.is_some(),
                "helix={helix}: sun"
            );
            assert!(
                r.members[1].cases[0].bending_stress.is_some(),
                "helix={helix}: planet"
            );
            assert!(
                r.members[2].cases[0].bending_stress.is_some(),
                "helix={helix}: ring"
            );
            assert!(
                r.meshes[0].line.unwrap().contact_ratios.overlap > 0.0,
                "helix={helix}"
            );
            assert!(residual(&r) < 1e-12);
        }
    }

    /// Tooth counts that admit no planet shift are refused, not fudged into an
    /// answer. Most combinations are impossible (docs/reference.md#planetary-sets) and that is the common
    /// case rather than an exceptional one.
    #[test]
    fn an_impossible_set_is_refused() {
        assert!(solve_set(&stage_of(24, 18, 200, 0.0), 2.0, 0.0, &test_library()).is_err());
    }

    /// The thickness invariants differ between the two meshes and both hold from
    /// one given `k`, whichever member states it: the external pair sums to
    /// two, the internal pair matches — and the automatic members follow
    /// whichever one is given, so pinning the ring's instead reaches the sun
    /// through the planet.
    #[test]
    fn one_thickness_modification_satisfies_both_invariants() {
        for k in [0.9, 1.0, 1.15] {
            let mut shape = stage_of(24, 18, 60, 0.0);
            for given in 0..3 {
                for (i, m) in shape.members.iter_mut().enumerate() {
                    m.thickness_mod = if i == given {
                        Auto::fixed(k)
                    } else {
                        Auto::automatic(1.0)
                    };
                }
                let ks = shape.thickness_mods();
                let (sun, planet, ring) = (ks[0], ks[1], ks[2]);
                assert!(
                    (sun + planet - 2.0).abs() < 1e-15,
                    "external pair must sum to two"
                );
                assert!((planet - ring).abs() < 1e-15, "internal pair must match");
                assert!((ks[given] - k).abs() < 1e-15, "the given one is the given");
            }
            // ...and it still solves.
            assert!(solve_set(&shape, 2.0, 0.0, &test_library()).is_ok());
        }
    }
    /// **The set's shifts follow the same rule as a pair's**: off, the sun sits
    /// at its undercut minimum and the ring where it was put; on, the two are
    /// searched together and the set keeps more of its power.
    ///
    /// `η₀` is the thing maximised and the set efficiency is what has to rise,
    /// which is the claim that [`crate::planetary::power`] is monotone in `η₀`
    /// being checked rather than assumed.
    #[test]
    fn choosing_the_shifts_for_efficiency_leaves_the_set_more_of_its_power() {
        let lib = test_library();
        // Both free: a shift given by hand is a constraint, and a set with two
        // of them has nothing left to search.
        let free = || {
            let mut s = stage_of(24, 18, 60, 0.0);
            s.members[0].gear.profile_shift = Auto::automatic(0.0);
            s.members[2].gear.profile_shift = Auto::automatic(0.0);
            s
        };
        let solve = |on: bool| {
            solve_set(
                &{
                    let mut s = free();
                    s.set_search(on);
                    s
                },
                2.0,
                0.0,
                &lib,
            )
            .expect("the set solves")
        };
        let plain = solve(false);
        let tuned = solve(true);
        let eta0 =
            |r: &ShapeResult| r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward;

        assert!(
            (tuned.members[0].profile_shift - plain.members[0].profile_shift).abs() > 1e-6
                || (tuned.members[2].profile_shift - plain.members[2].profile_shift).abs() > 1e-6,
            "the search moved nothing: sun {} ring {}",
            tuned.members[0].profile_shift,
            tuned.members[2].profile_shift
        );
        assert!(
            eta0(&tuned) > eta0(&plain),
            "eta0 {:.6} should beat {:.6}",
            eta0(&tuned),
            eta0(&plain)
        );
        assert!(
            tuned.efficiency.unwrap().forward > plain.efficiency.unwrap().forward,
            "the set efficiency {:.6} should beat {:.6}, or power is not monotone in eta0",
            tuned.efficiency.unwrap().forward,
            plain.efficiency.unwrap().forward
        );
        // Both meshes stay continuous by at least the margin asked for.
        for eps in [
            tuned.meshes[0].line.unwrap().contact_ratios.transverse,
            tuned.meshes[1].line.unwrap().contact_ratios.transverse,
        ] {
            let asked = free().meshes[0].min_contact_ratio;
            assert!(eps >= asked - 1e-3, "contact ratio {eps} under {asked}");
        }
    }

    /// A shift given by hand is a constraint the search may not overrule.
    #[test]
    fn a_given_shift_survives_the_search() {
        let mut stage = stage_of(24, 18, 60, 0.0);
        stage.set_search(true);
        stage.members[0].gear.profile_shift = Auto::automatic(0.0);
        stage.members[2].gear.profile_shift = Auto::fixed(0.25);
        let r = solve_set(&stage, 2.0, 0.0, &test_library()).expect("solves");
        assert!((r.members[2].profile_shift - 0.25).abs() < 1e-9);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod pressure_angle {
    //! **A pressure angle is a member's, shared across a mesh** — as the
    //! module is — so two mesh groups of one stage may run at two angles,
    //! and two members in mesh at two are refused.

    use super::super::arrangements::{layshaft, Preset};
    use super::super::{test_library, TrainError};
    use super::*;

    fn solve(shape: &Shape) -> Result<crate::train::Alone, TrainError> {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, 2.0, 3000.0),
            &test_library(),
        )
    }

    /// A layshaft's second pair at 25° runs at 25° and its first at 20°,
    /// each mesh reporting its own operating angle above its own reference.
    #[test]
    fn two_mesh_groups_of_one_stage_run_at_two_pressure_angles() {
        let mut shape = layshaft((17, 43), &[(41, 19)], 0);
        for j in [2, 3] {
            shape.members[j].pressure_angle = Auto::fixed(25.0);
        }
        let r = solve(&shape).unwrap();
        let alpha = |k: usize| r.meshes[k].line.unwrap().operating_pressure_angle;
        assert!(alpha(0) >= 20.0 && alpha(0) < 22.0, "{}", alpha(0));
        assert!(alpha(1) >= 25.0 && alpha(1) < 27.0, "{}", alpha(1));
        assert_eq!(r.members[2].params.pressure_angle, 25.0);
        assert_eq!(r.members[0].params.pressure_angle, 20.0);
    }

    /// Two members in one mesh at two angles cannot mesh, and the stage
    /// says so — on a line contact through the mesh, on a point contact
    /// through the screw.
    #[test]
    fn two_members_in_mesh_at_two_pressure_angles_are_refused() {
        for preset in [Preset::Spur, Preset::Worm] {
            let mut shape = preset.build();
            shape.members[1].pressure_angle = Auto::fixed(25.0);
            assert!(
                matches!(
                    solve(&shape),
                    Err(TrainError::Mesh(crate::mesh::MeshError::Incompatible))
                ),
                "{preset:?}"
            );
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod overlap_per_group {
    //! **An axial contact ratio is a mesh's, and a size is a mesh group's**:
    //! a layshaft's pairs, each with every width given and its own ratio,
    //! each take the helix that ratio needs — one size per group, not one
    //! per stage — and a ratio given where a width is automatic is a floor
    //! under that mesh's own widths.

    use super::super::arrangements::layshaft;
    use super::super::{helix_for_overlap, test_library};
    use super::*;

    fn solve(shape: &Shape) -> crate::train::Alone {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, 2.0, 3000.0),
            &test_library(),
        )
        .unwrap()
    }

    #[test]
    fn each_mesh_group_takes_the_helix_its_own_ratio_needs() {
        // Two pairs on one distance: the second's counts chosen so its
        // steeper helix reaches the first's distance by a shift.
        let mut shape = layshaft((17, 43), &[(38, 19)], 0);
        for m in &mut shape.members {
            m.gear.face_width = Auto::fixed(10.0);
        }
        shape.meshes[0].overlap = Auto::fixed(0.8);
        shape.meshes[1].overlap = Auto::fixed(1.2);
        let r = solve(&shape);
        let want = |ratio: f64| helix_for_overlap(ratio, 1.0, 10.0).unwrap();
        assert!((r.members[0].helix_angle.abs() - want(0.8)).abs() < 1e-9);
        assert!((r.members[2].helix_angle.abs() - want(1.2)).abs() < 1e-9);
        assert!(
            (r.meshes[0].line.unwrap().contact_ratios.overlap - 0.8).abs() < 1e-6
                && (r.meshes[1].line.unwrap().contact_ratios.overlap - 1.2).abs() < 1e-6
        );
    }

    #[test]
    fn a_ratio_given_over_an_automatic_width_floors_that_meshs_widths_alone() {
        let mut shape = layshaft((17, 43), &[(41, 19)], 0);
        for m in &mut shape.members {
            m.gear.face_width = Auto::automatic(5.0);
        }
        // One helix stated per pair; its mate follows in the other hand.
        shape.members[0].gear.helix_angle = Auto::fixed(15.0);
        shape.members[2].gear.helix_angle = Auto::fixed(15.0);
        shape.meshes[1].overlap = Auto::fixed(2.0);
        let r = solve(&shape);
        let floor = super::super::width_for_overlap(&Auto::fixed(2.0), 15.0, 1.0).unwrap();
        assert!(r.members[2].face_width >= floor - 1e-9 && r.members[3].face_width >= floor - 1e-9);
        assert!(
            r.members[0].face_width < floor,
            "the first pair asked no floor"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod hula_recorded {
    //! **The hula stage through the shape is the hula stage**, held to the
    //! figures its own solver recorded before it retired. The gate that ran the
    //! two side by side lived at `2b71654` and found them the same to 1e-6
    //! — ratio, crank offset and the mesh that held it open, every shift
    //! and width, every mesh's figures, the stage's efficiency both ways and
    //! its backlash at both bodies — apart from the two differences the
    //! set's retirement had already recorded (`docs/corrections.md`): a
    //! driven member pressed with its driver's force, and a case from the
    //! output entered at the output rather than read as the crank's
    //! delivered torque. What is held here is what the corpus printed for
    //! `gear-cli hula 18 0.2` and what `docs/reference.md#the-hula-stage`
    //! quotes for the shipped stage, to the digits they print.

    use super::super::arrangements::hula;
    use super::super::test_library;
    use super::*;

    /// The harness's fixture: 19/18/17/18, shapers of 14 and 13 teeth for
    /// the two rings (the grounded gear and the output, members 2 and 3
    /// after the two wobble gears), the gap asked.
    fn hula_18(clearance: f64) -> Shape {
        let mut shape = hula([19, 18, 17, 18], [1.0, 1.0]);
        shape.distances[0].tip_clearance = clearance;
        for (m, teeth) in [14, 13].into_iter().enumerate() {
            shape.members[2 + m].ring.as_mut().unwrap().teeth = teeth;
        }
        shape
    }

    /// The shipped stage, 65/61/57/61.
    fn shipped() -> Shape {
        hula([65, 61, 57, 61], [1.0, 1.0])
    }

    /// The hula's own boundary: crank driven, grounded gear held, output
    /// out — bodies 1, 2 and 3 of the shape.
    fn solve(shape: &Shape, torque: f64, speed: f64) -> crate::train::Alone {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, torque, speed).arranged(&[2], 1, 3),
            &test_library(),
        )
        .unwrap()
    }

    fn close(a: f64, b: f64, tol: f64, what: &str) {
        assert!((a - b).abs() <= tol, "{what}: recorded {a}, shape {b}");
    }

    #[test]
    fn the_harness_hula_is_what_the_corpus_recorded() {
        let r = solve(&hula_18(0.2), 2.0, 1000.0);
        close(324.0, r.ratio.unwrap(), 1e-9, "ratio");
        let d = &r.distances[0];
        close(0.726_026, d.running, 1e-6, "crank offset, running");
        close(
            0.746_026,
            d.nominal[0],
            1e-6,
            "crank offset at zero backlash",
        );
        assert_eq!(d.sized_by, Some(0), "held open by mesh 1");
        // The rings at +0.4519, the pinions at their floor: the members are
        // the two wobble pinions, then the grounded ring and the output ring.
        for (i, want) in [(0, 0.0), (1, 0.0), (2, 0.4519), (3, 0.4519)] {
            close(
                want,
                r.members[i].profile_shift,
                5e-5,
                &format!("gear {i} shift"),
            );
        }
        close(
            31.310,
            r.efficiency.unwrap().forward * 100.0,
            5e-4,
            "forward efficiency, %",
        );
        close(0.0, r.efficiency.unwrap().backward, 1e-12, "self-locking");
        close(
            0.9932,
            r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward,
            5e-5,
            "the two meshes alone",
        );
        close(
            0.405_565,
            r.backlash.unwrap().forward.nominal,
            1e-6,
            "backlash at the output",
        );
        close(
            131.4032,
            r.backlash.unwrap().backward.nominal,
            5e-5,
            "backlash at the crank",
        );
        // The operating angle is the **running** mesh's, 0.02 mm inside the
        // zero-backlash offset the old solver quoted its 50.965° at — the stated
        // change every pair took with the shape (`docs/corrections.md`).
        for (k, (aw, eps, far, tip)) in [
            (49.673, 0.9735, 0.2779, 0.0),
            (49.673, 0.9712, 0.2779, 0.0106),
        ]
        .into_iter()
        .enumerate()
        {
            let m = &r.meshes[k];
            close(aw, m.line.unwrap().operating_pressure_angle, 5e-4, "α_w");
            close(eps, m.contact_ratio, 5e-5, "ε");
            let tips = m.tips.unwrap();
            close(far, tips.far_gap, 5e-5, "far-side gap");
            close(tip, tips.tip_margin, 5e-5, "tip margin");
            assert!(!tips.tip_interference);
        }
        // **Every mesh is pressed with its driver's force** — the stated
        // change every driven member took with the shape. The old solver
        // anchored each mesh at the torque the power flow put on its
        // central member: the output's, which drives its wobble gear in the
        // crank's frame and so is the flank force itself; and the grounded
        // ring's, which is *driven* by its wobble gear and stood `η₁` under
        // the force on its flank — so the whole of mesh 1 is `1/η₁` over
        // what it printed, bending with it, and mesh 2 is unchanged.
        let eta1 = r.meshes[0].efficiency.forward;
        close(
            200.8899 / eta1,
            r.members[2].cases[0].torque,
            5e-4,
            "z19 torque over η₁",
        );
        close(
            190.3167 / eta1,
            r.members[0].cases[0].torque,
            5e-4,
            "z18 torque over η₁",
        );
        close(191.6182, r.members[1].cases[0].torque, 5e-4, "z17 torque");
        close(
            202.8899,
            r.members[3].cases[0].torque,
            5e-4,
            "output z18 torque",
        );
        close(
            6663.3 / eta1,
            r.members[0].cases[0].bending_stress.unwrap(),
            0.05,
            "z18 σ_F",
        );
        close(
            7171.7,
            r.members[1].cases[0].bending_stress.unwrap(),
            0.05,
            "z17 σ_F",
        );
        for i in [2, 3] {
            assert!(
                r.members[i].cases[0].bending_stress.is_none(),
                "no fillet, no rating"
            );
        }
    }

    #[test]
    fn the_shipped_hula_stage_reports_the_figures_the_documents_quote() {
        let r = solve(&shipped(), 2.0, 1000.0);
        close(3721.0 / 16.0, r.ratio.unwrap(), 1e-9, "the reduction");
        close(
            81.92,
            r.efficiency.unwrap().forward * 100.0,
            0.005,
            "forward, %",
        );
        close(
            77.91,
            r.efficiency.unwrap().backward * 100.0,
            0.005,
            "backward, %",
        );
    }

    /// **A load at the output reaches the crank cut by the backward
    /// efficiency.** Stated where it enters — at the output, as a train's
    /// case states it — the output carries it and the crank delivers it
    /// over `η_backward`, once the ratio has referred it.
    #[test]
    fn a_load_at_the_output_reaches_the_crank_over_its_backward_efficiency() {
        let ratio = solve(&shipped(), 2.0, 3000.0).ratio.unwrap();
        let mut train = crate::train::Train::alone(&shipped(), 2.0, 3000.0).arranged(&[2], 1, 3);
        train
            .load_cases
            .push(crate::train::LoadCase::back_driving(1, 3, 0.5 * ratio));
        let r = crate::train::solve_alone(&train, &test_library()).unwrap();
        let c = &r.cases[2];
        close(
            0.5 * ratio,
            c.torques[3].abs(),
            1e-9,
            "the output carries the case",
        );
        close(
            0.5 * r.efficiency.unwrap().backward,
            c.torques[1].abs(),
            1e-9,
            "the crank delivers it over η_b",
        );
    }
}

#[cfg(test)]
mod member_names {
    //! **One rule names every member**, and the harness's corpus — unchanged
    //! when its own naming was replaced by this — is the second reader.

    use super::super::arrangements as arr;
    use super::{MemberRole, Shape};

    fn names(shape: &Shape) -> Vec<String> {
        shape
            .member_names()
            .iter()
            .map(|n| {
                let word = format!("{:?}", n.role).to_lowercase();
                n.ordinal.map_or(word.clone(), |o| format!("{word} {o}"))
            })
            .collect()
    }

    #[test]
    fn every_preset_and_arrangement_names_its_members_as_a_designer_does() {
        let s = |v: &[&str]| v.iter().map(|s| (*s).to_string()).collect::<Vec<_>>();
        assert_eq!(names(&arr::pair([17, 43])), s(&["gear", "gear"]));
        assert_eq!(names(&arr::worm(1, 40)), s(&["worm", "wheel"]));
        // The same teeth as a crossed gear pair are gears by number.
        assert_eq!(
            names(&{
                let mut s = arr::worm(1, 40);
                s.distances[0].worm = false;
                s
            }),
            s(&["gear", "gear"])
        );
        assert_eq!(
            names(&arr::planetary(12, 30, 72, 3)),
            s(&["sun", "planet", "ring"])
        );
        assert_eq!(
            names(&arr::hula([65, 61, 57, 61], [1.0, 1.0])),
            s(&["planet 1", "planet 2", "ring 1", "ring 2"])
        );
        assert_eq!(
            names(&arr::wolfrom(18, [60, 61], 3)),
            s(&["planet", "ring 1", "ring 2"])
        );
        assert_eq!(
            names(&arr::ravigneaux([18, 30], [22, 18], 62, 3)),
            s(&["sun 1", "planet 1", "planet 2", "sun 2", "ring"])
        );
        assert_eq!(
            names(&arr::worm_and_pair((1, 40), (17, 43))),
            s(&["worm", "wheel", "gear", "gear"])
        );
        // A gear is never numbered by its role: its number is the train's.
        assert!(arr::layshaft((17, 43), &[(41, 19)], 0)
            .member_names()
            .iter()
            .all(|n| n.role == MemberRole::Gear && n.ordinal.is_none()));
    }
}

#[cfg(test)]
mod assembly {
    //! **The assembly rule against a search over the phases** that shares
    //! none of its arithmetic: each mesh on the axis fixes the planet's
    //! turn up to a whole tooth of its own gear, and the planets assemble
    //! where some tooth of the first gear makes every other mesh's phase
    //! whole too, at every station round the carrier.

    use super::super::arrangements as arr;

    use crate::params::Auto;

    /// Whether `n` stations can each find a turn `ψ` that puts every mesh
    /// `(z_c, z_p)` in phase — `z_p ψ ≡ (z_c + z_p) φ (mod 2π)`, the
    /// kinematic row integrated — trying every tooth of the first gear.
    fn assembles_by_search(pairs: &[(i64, i64)], n: i64) -> bool {
        let tau = std::f64::consts::TAU;
        (0..n).all(|j| {
            let phi = tau * j as f64 / n as f64;
            let (c1, p1) = pairs[0];
            (0..p1).any(|a| {
                let psi = ((c1 + p1) as f64 * phi + tau * a as f64) / p1 as f64;
                pairs[1..].iter().all(|&(c, p)| {
                    let residual = (p as f64 * psi - (c + p) as f64 * phi) / tau;
                    (residual - residual.round()).abs() < 1e-9
                })
            })
        })
    }

    #[test]
    fn the_rule_agrees_with_the_search_on_simple_and_stepped_sets() {
        let mut checked = 0;
        for n in 2..=5_u32 {
            for sun in [12_u32, 17, 24, 30] {
                for p1 in [8_u32, 12, 18, 20] {
                    for p2 in [8_u32, 10, 12, 17, 18] {
                        for ring in [48_u32, 59, 60, 72, 81] {
                            if ring <= p2 + 1 {
                                continue;
                            }
                            let shape = arr::stepped(sun, [p1, p2], [ring, ring + 1], n);
                            // The first planet gear meshes the sun and the
                            // first ring, the second the second ring; the
                            // rule reads all three meshes.
                            let Some((equal, simultaneous)) = shape.assembly(1) else {
                                panic!("a stepped planet's assembly has an answer");
                            };
                            let pairs = [
                                (i64::from(sun), i64::from(p1)),
                                (-i64::from(ring), i64::from(p1)),
                                (-i64::from(ring + 1), i64::from(p2)),
                            ];
                            assert_eq!(
                                equal,
                                assembles_by_search(&pairs, i64::from(n)),
                                "stepped {sun}/{p1}/{p2}/{ring} × {n}"
                            );
                            assert_eq!(
                                simultaneous,
                                [sun, ring, ring + 1].iter().all(|z| z % n == 0)
                            );
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 1000);
        // ...and on a simple set the rule is the textbook `N | z_s + z_r`.
        for n in 2..=6_u32 {
            for (sun, planet) in [(24_u32, 18_u32), (17, 20), (30, 15)] {
                for ring in [sun + 2 * planet, sun + 2 * planet + 1] {
                    let mut set = arr::planetary(12, 30, 72, 3);
                    set.members[0].gear.teeth = sun;
                    set.members[1].gear.teeth = planet;
                    set.members[2].gear.teeth = ring;
                    set.axes[1].count = n;
                    set.members[1].gear.profile_shift = Auto::automatic(0.0);
                    assert_eq!(
                        set.assembly(1),
                        Some(((sun + ring) % n == 0, sun % n == 0 && ring % n == 0))
                    );
                }
            }
        }
    }

    /// An axis meshing another planet's axis is outside the rule, and says
    /// so rather than guessing.
    #[test]
    fn a_planet_meshing_a_planet_has_no_answer() {
        let shape = arr::ravigneaux([18, 30], [22, 18], 62, 3);
        assert_eq!(shape.assembly(1), None);
        assert_eq!(shape.assembly(2), None);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod a_mesh_stands_where_its_axes_do {
    //! **A mesh's frame is the body both its axes stand still in**, asked of
    //! the mesh rather than of the axis a member turns about — which is what
    //! lets a set and the pair that drives it be one shape, the shaft they
    //! share one line.

    use super::super::arrangements::Builder;
    use super::super::{solve_train, test_library, Train};
    use super::*;

    #[test]
    fn a_gear_on_a_suns_shaft_meshes_in_ground() {
        let mut b = Builder::new(1.0);
        let (fixed, central) = (b.axis(), b.axis());
        let pinion_body = b.body(fixed);
        let sun_body = b.body(central);
        let carrier = b.body(central);
        let ring_body = b.body(central);
        let planets = b.carried_axis(carrier, 3);
        let planet_body = b.body(planets);
        let pinion = b.gear(pinion_body, 17);
        let drive = b.gear(sun_body, 43);
        let sun = b.gear(sun_body, 12);
        let planet = b.gear(planet_body, 30);
        let ring = b.ring(ring_body, 72);
        b.mesh(pinion, drive).distance([fixed, central]);
        b.mesh(sun, planet)
            .mesh(planet, ring)
            .distance([central, planets]);
        let shape = b.build();
        let w = shape.wiring();
        assert_eq!(w.frame(0).unwrap(), GROUND, "the pair meshes in ground");
        let c = shape.slot(carrier);
        assert_eq!(w.frame(1).unwrap(), c, "the sun meshes in its carrier");
        assert_eq!(w.frame(2).unwrap(), c, "...and so does the ring");
        // One shape is the pair and the set in a row: the ratio a chain of
        // the two reports, the pair's reversal times the set's `1 + z_r/z_s`.
        let r = solve_train(
            &Train::alone(&shape, 2.0, 3000.0).arranged(&[ring_body], pinion_body, carrier),
            &test_library(),
        )
        .unwrap();
        let want = -(43.0 / 17.0) * (1.0 + 72.0 / 12.0);
        assert!(
            (r.paths[0].ratio - want).abs() < 1e-9 * want.abs(),
            "{:?} against {want}",
            r.paths[0].ratio
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod the_pieces_own {
    //! **What was the stage's is the piece's**: a mesh's search and its
    //! sharing, a replicated axis's planet gap. Each law turns one piece and
    //! holds every other to what it was — the claim a switch on the stage
    //! could not make.

    use super::super::arrangements::{self as arr, Builder};
    use super::super::test_library;
    use super::*;

    fn conventionally(shape: &Shape) -> crate::train::Alone {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, 2.0, 3000.0),
            &test_library(),
        )
        .unwrap()
    }

    fn shifts(members: &[GearResult]) -> Vec<f64> {
        members.iter().map(|g| g.profile_shift).collect()
    }

    /// Two pairs in series through a compound shaft — which is what two
    /// stages in a row are once the train is one graph: their meshes share
    /// no member and no automatic distance, so they are two components.
    fn compound() -> Shape {
        let mut b = Builder::new(1.0);
        let (x0, x1, x2) = (b.axis(), b.axis(), b.axis());
        let (s0, s1, s2) = (b.body(x0), b.body(x1), b.body(x2));
        let g1 = b.gear(s0, 17);
        let g2 = b.gear(s1, 43);
        let g3 = b.gear(s1, 13);
        let g4 = b.gear(s2, 31);
        b.mesh(g1, g2).distance([x0, x1]);
        b.mesh(g3, g4).distance([x1, x2]);
        b.build()
    }

    /// **A component is searched where one of its meshes asks, and only
    /// then.** The first pair asking and the second not: the first's shifts
    /// are the ones both asking give, and the second's the ones neither
    /// asking gives — and the search moves something, or the law is empty.
    #[test]
    fn a_component_that_does_not_ask_keeps_its_undercut_shifts() {
        let solve = |ask: [bool; 2]| {
            let mut s = compound();
            s.meshes[0].search = ask[0];
            s.meshes[1].search = ask[1];
            // Two parts — the pairs share a shaft and nothing else — read
            // off the train's one result.
            let train = crate::train::Train::alone(&s, 2.0, 3000.0).arranged(&[], 1, 3);
            shifts(
                &crate::train::solve_train(&train, &test_library())
                    .unwrap()
                    .members,
            )
        };
        let (none, both, first) = (solve([false; 2]), solve([true; 2]), solve([true, false]));
        assert!(
            (0..2).any(|i| (both[i] - none[i]).abs() > 1e-6),
            "the search should move the first pair: {both:?} vs {none:?}"
        );
        for i in 0..2 {
            assert!((first[i] - both[i]).abs() < 1e-12, "{first:?} vs {both:?}");
        }
        for i in 2..4 {
            assert!((first[i] - none[i]).abs() < 1e-12, "{first:?} vs {none:?}");
        }
    }

    /// **...and searched whole, where a member is shared.** A set's planet
    /// is in both its meshes, so the sun mesh alone asking searches the
    /// ring mesh too: every shift is the one both asking give.
    #[test]
    fn one_mesh_asking_searches_its_whole_component() {
        let solve = |ask: [bool; 2]| {
            let mut s = arr::planetary(12, 30, 72, 3);
            s.members[0].gear.profile_shift = Auto::automatic(0.0);
            s.members[2].gear.profile_shift = Auto::automatic(0.0);
            s.meshes[0].search = ask[0];
            s.meshes[1].search = ask[1];
            shifts(&conventionally(&s).members)
        };
        let (none, both, one) = (solve([false; 2]), solve([true; 2]), solve([true, false]));
        assert!(
            (0..3).any(|i| (both[i] - none[i]).abs() > 1e-6),
            "the search should move the set: {both:?} vs {none:?}"
        );
        for i in 0..3 {
            assert!((one[i] - both[i]).abs() < 1e-12, "{one:?} vs {both:?}");
        }
    }

    /// **Sharing is each mesh's own.** The ramp on one of a set's meshes:
    /// the member only in that mesh bends as with the ramp on both, the
    /// member only in the other as with it on neither — each way round,
    /// since at these counts the ramp moves the ring and leaves the sun,
    /// and the check that matters is that it does not leak.
    #[test]
    fn sharing_is_each_meshes_own() {
        let solve = |ramp: [bool; 2]| {
            let mut s = arr::planetary(12, 30, 72, 3);
            for m in &mut s.members {
                m.gear.addendum = 1.35;
            }
            for (k, on) in ramp.into_iter().enumerate() {
                s.meshes[k].load_sharing = if on {
                    LoadSharing::LinearRamp
                } else {
                    LoadSharing::None
                };
            }
            conventionally(&s)
                .members
                .iter()
                .map(|g| g.cases[0].bending_stress.unwrap())
                .collect::<Vec<f64>>()
        };
        let (none, both) = (solve([false; 2]), solve([true; 2]));
        let (sun, ring) = (0, 2);
        assert!(
            (both[ring] - none[ring]).abs() > 1e-6,
            "the ramp should move the ring: {both:?} vs {none:?}"
        );
        let on_sun = solve([true, false]);
        assert!(
            (on_sun[sun] - both[sun]).abs() < 1e-9,
            "{on_sun:?} vs {both:?}"
        );
        assert!(
            (on_sun[ring] - none[ring]).abs() < 1e-9,
            "{on_sun:?} vs {none:?}"
        );
        let on_ring = solve([false, true]);
        assert!(
            (on_ring[ring] - both[ring]).abs() < 1e-9,
            "{on_ring:?} vs {both:?}"
        );
        assert!(
            (on_ring[sun] - none[sun]).abs() < 1e-9,
            "{on_ring:?} vs {none:?}"
        );
    }

    /// **A planet axis keeps its own gap.** Meshed planets run two carried
    /// axes; a minimum no gap meets on one is reported against that axis
    /// alone.
    #[test]
    fn a_planet_axis_keeps_its_own_gap() {
        let mut s = arr::meshed_planets(18, [13, 13], 72, 3);
        let before = conventionally(&s);
        assert!(before.layouts.len() == 2 && before.layouts.iter().all(|l| l.clearance_ok));
        let first = before.layouts[0].axis;
        s.axes[first].min_planet_clearance = 1e3;
        let after = conventionally(&s);
        for l in &after.layouts {
            assert_eq!(l.clearance_ok, l.axis != first, "{:?}", after.layouts);
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod one_module_per_group {
    //! **A mesh group's module and pressure angle are stated once**: on one
    //! member, followed by the rest, and relief keeps exactly one standing —
    //! the helix's rule with the relation made equality.

    use super::super::arrangements::{self as arr, layshaft};
    use super::super::{test_library, Freedom, MemberFreedom};
    use super::*;

    fn solve(shape: &Shape) -> crate::train::Alone {
        crate::train::solve_alone(
            &crate::train::Train::alone(shape, 2.0, 3000.0),
            &test_library(),
        )
        .unwrap()
    }

    fn given(shape: &Shape, group: &[usize], f: MemberFreedom) -> usize {
        let mut s = shape.clone();
        group
            .iter()
            .filter(|&&i| s.input_mut(Freedom::Member(i, f)).is_some_and(|a| !a.auto))
            .count()
    }

    /// Every preset states each group's module and pressure angle on one
    /// member, and relief leaves exactly one — whatever it is handed.
    #[test]
    fn relief_keeps_one_statement_per_group() {
        for preset in arr::Preset::ALL {
            let shape = preset.build();
            for group in shape.mesh_groups() {
                for f in [MemberFreedom::Module, MemberFreedom::PressureAngle] {
                    assert_eq!(given(&shape, &group, f), 1, "{preset:?} as built");
                    // Everything stated at once: relief turns all but one back.
                    let mut all = shape.clone();
                    for &i in &group {
                        all.input_mut(Freedom::Member(i, f)).unwrap().auto = false;
                    }
                    assert_eq!(
                        given(&all.relieved(None), &group, f),
                        1,
                        "{preset:?} all given"
                    );
                    // Nothing stated: relief pins one.
                    let mut none = shape.clone();
                    for &i in &group {
                        none.input_mut(Freedom::Member(i, f)).unwrap().auto = true;
                    }
                    assert_eq!(
                        given(&none.relieved(None), &group, f),
                        1,
                        "{preset:?} none given"
                    );
                }
            }
        }
    }

    /// **The one touched stands, and the rest follow it.** A pair's second
    /// gear stated at module 2: relief turns the first's back, and both are
    /// cut at 2 — the solve reads what the follower follows.
    #[test]
    fn the_rest_of_a_group_follow_the_member_that_states_it() {
        let mut shape = arr::pair([17, 43]);
        let touched = Freedom::Member(1, MemberFreedom::Module);
        *shape.input_mut(touched).unwrap() = Auto::fixed(2.0);
        let settled = shape.relieved(Some(touched));
        assert!(settled.members[0].module.auto && !settled.members[1].module.auto);
        assert_eq!(
            settled.members[0].normal_module(),
            2.0,
            "the follower shows it"
        );
        let r = solve(&settled);
        assert!(r.members.iter().all(|g| g.params.module == 2.0));
        // ...and unrelieved, the solve shares before it reads.
        let mut raw = arr::pair([17, 43]);
        raw.members[0].module = Auto::automatic(1.0);
        raw.members[1].module = Auto::fixed(2.0);
        assert!(solve(&raw).members.iter().all(|g| g.params.module == 2.0));
    }

    /// **A group is the mesh graph's, not the stage's.** A layshaft's
    /// second pair stated at 1.5 follows no other pair: the constant mesh
    /// stays at 1.
    #[test]
    fn a_group_follows_its_own_statement_and_no_other() {
        let mut shape = layshaft((17, 43), &[(41, 19)], 0);
        shape.members[2].module = Auto::fixed(1.0 / 0.998);
        let shared = shape.shared();
        assert_eq!(shared.members[3].normal_module(), 1.0 / 0.998);
        assert_eq!(shared.members[0].normal_module(), 1.0);
        assert_eq!(shared.members[1].normal_module(), 1.0);
    }
}
