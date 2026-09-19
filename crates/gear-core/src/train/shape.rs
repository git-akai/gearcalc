//! **The stage every kind is a tick pattern of** — axes, shafts on them,
//! members on the shafts, meshes between members, and one distance per pair
//! of axes that mesh.
//!
//! A spur pair is two axes fixed in ground with one mesh between them. A
//! planetary set is a central axis and a planet axis carried by a shaft on
//! the central one, replicated `N` times, with two meshes on the one
//! distance between the axes. A hula stage is the same with `N = 1`, both
//! meshes internal and a compound planet. A layshaft transmission is two
//! ground axes with several meshes on one distance. None of these is a kind
//! here: the shape says which frames there are, and everything else — the
//! wiring the kinematics reads, the closure the shifts obey, where the power
//! goes — is derived from it once (`geartrain-refactor-plan.md`, *One stage
//! shape*).
//!
//! # One distance per pair of axes
//!
//! Every mesh between a member on axis `A` and a member on axis `B` runs at
//! the distance between those axes, in the frame both stand still in. That is
//! the law `train/planetary.rs` writes for a planet's carrier radius and
//! `train/hula.rs` writes for its crank offset, and it is not an epicyclic
//! law: a layshaft's pairs obey it in ground. Automatic, the distance is what
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

use super::wiring::{MeshSpec, Mount, ShaftLabel, Wiring};
use super::{
    Constrained, ContactRatios, Freedom, FreedomGroup, GearResult, Loading, MemberFacts,
    MemberFreedom, MemberRating, MeshReport, Optimisation, Ports, Reading, StageGear, StageLoads,
    TrainError, PROBE,
};
use crate::contact::{efficiency, ContactPath, Directional, Drive, LoadSharing};
use crate::kinematics::{Shaft, GROUND};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{operating_geometry, shift_sum_for, Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::plane::BasicRack;
use crate::ring::{Cutter, Ring};
use crate::strength::{bending_stress, contact_stress, Load, RootStressModel, PARALLEL_AXES};
use crate::tooth::Tooth;

/// An axis gears turn about: fixed in ground, or carried round another axis
/// by a shaft — a planet's, riding the carrier.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Axis {
    /// The shaft whose frame this axis stands still in, where it is not
    /// ground's: a carrier. `None` is an axis fixed in ground.
    pub carried_by: Option<Shaft>,
    /// How many times this axis, its shafts and their gears are replicated
    /// about the axis it is carried round — `N` planets. One elsewhere.
    pub count: u32,
}

/// A shaft, by the axis it turns about. Shaft `i` here is shaft `i + 1` of
/// the wiring; ground is shaft 0 and is not listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShaftOn {
    pub axis: usize,
}

/// A gear, on a shaft.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Member {
    /// The shaft it spins with, numbered as [`ShaftOn`] is.
    pub shaft: Shaft,
    pub gear: StageGear,
    /// Normal module, mm. Every mesh a member is in shares it.
    pub module: f64,
    /// Tooth-thickness coefficient, `k`: above 1 this gear's teeth thicken.
    /// Two gears in mesh sum to 2.
    pub thickness_mod: f64,
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
    /// model of its own ([`super::crossed`]), and the rest of the stage
    /// cannot yet share a member with it.
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

/// The stage.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Shape {
    /// Normal pressure angle, degrees. Shared by every member.
    pub pressure_angle: f64,
    pub overlap: Auto<f64>,
    pub optimisation: Optimisation,
    pub load_sharing: LoadSharing,
    /// Tip-to-tip clearance between neighbouring instances of a replicated
    /// axis's gears, mm — asked only where an axis is replicated.
    pub min_planet_clearance: f64,
    pub axes: Vec<Axis>,
    pub shafts: Vec<ShaftOn>,
    pub members: Vec<Member>,
    pub meshes: Vec<MeshInput>,
    pub distances: Vec<Distance>,
}

// ------------------------------------------------------------ the shape ---

impl Shape {
    /// The wiring shaft a member spins with.
    fn shaft_of(&self, member: usize) -> Shaft {
        self.members[member].shaft
    }

    /// The axis a wiring shaft turns about.
    fn axis_of_shaft(&self, shaft: Shaft) -> Option<usize> {
        (shaft != GROUND).then(|| self.shafts[shaft - 1].axis)
    }

    /// **The frame a member's axis stands still in for meshing purposes**:
    /// its carrier where it rides one, and the carrier on its own axis where
    /// it is central to one, and ground otherwise ([`super::wiring`]).
    fn frame_of_member(&self, member: usize) -> Shaft {
        let shaft = self.shaft_of(member);
        let Some(axis) = self.axis_of_shaft(shaft) else {
            return GROUND;
        };
        if let Some(c) = self.axes[axis].carried_by {
            return c;
        }
        // A carrier on this axis: the first shaft here that carries an axis.
        self.shafts
            .iter()
            .enumerate()
            .filter(|(_, s)| s.axis == axis)
            .map(|(i, _)| i + 1)
            .find(|&s| self.axes.iter().any(|a| a.carried_by == Some(s)))
            .unwrap_or(GROUND)
    }

    /// Whether a shaft's axis is one of `N` alike.
    fn replicated(&self, shaft: Shaft) -> bool {
        self.axis_of_shaft(shaft)
            .is_some_and(|a| self.axes[a].count > 1)
    }

    /// How many instances of a shaft's axis there are.
    fn count_of(&self, shaft: Shaft) -> u32 {
        self.axis_of_shaft(shaft)
            .map_or(1, |a| self.axes[a].count.max(1))
    }

    /// A mesh's kind, from its members: internal where exactly one is a
    /// ring, on the side of the ring.
    fn kind_of(&self, mesh: usize) -> Option<MeshKind> {
        let m = self.meshes[mesh];
        match (
            self.members[m.a].ring.is_some(),
            self.members[m.b].ring.is_some(),
        ) {
            (false, false) => Some(MeshKind::External),
            (false, true) => Some(MeshKind::Internal),
            _ => None,
        }
    }

    /// The distance a mesh runs at — the entry for its two axes, either way
    /// round.
    fn distance_of(&self, mesh: usize) -> Option<usize> {
        let m = self.meshes[mesh];
        let (a, b) = (
            self.axis_of_shaft(self.shaft_of(m.a))?,
            self.axis_of_shaft(self.shaft_of(m.b))?,
        );
        self.distances
            .iter()
            .position(|d| d.axes == [a, b] || d.axes == [b, a])
    }

    /// The meshes on one distance, in order.
    fn meshes_on(&self, distance: usize) -> Vec<usize> {
        (0..self.meshes.len())
            .filter(|&m| self.distance_of(m) == Some(distance))
            .collect()
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

    /// The label a wiring gives a shaft: a carrier where it carries an axis,
    /// the first member on it otherwise.
    fn label_of(&self, shaft: Shaft) -> ShaftLabel {
        if shaft == GROUND {
            return ShaftLabel::Ground;
        }
        let carriers: Vec<Shaft> = (1..=self.shafts.len())
            .filter(|&s| self.axes.iter().any(|a| a.carried_by == Some(s)))
            .collect();
        if let Some(index) = carriers.iter().position(|&c| c == shaft) {
            return ShaftLabel::Carrier { index };
        }
        let member = self
            .members
            .iter()
            .position(|m| m.shaft == shaft)
            .unwrap_or(0);
        ShaftLabel::Member { member }
    }

    // ---------------------------------------------------------- helices ---

    /// **Every member's helix**, from what is stated: a member's own reading
    /// (its helix, or its pitch diameter), else its mate's through the mesh
    /// they share — the opposite hand across an external mesh, the same
    /// across an internal one — else straight teeth.
    pub(crate) fn helix_angles(&self) -> Vec<f64> {
        let readings = self.readings();
        let first_a = self.meshes.first().map(|m| m.a);
        let stated = |i: usize| -> Option<f64> {
            readings.iter().find_map(|r| match r.freedom {
                Freedom::Member(m, MemberFreedom::Helix) if m == i => r.helix,
                Freedom::FirstPitchDiameter if i == 0 => r.helix,
                // The overlap reads the size of the first mesh's first
                // member, as the pair's did.
                Freedom::Overlap if Some(i) == first_a => r.helix,
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
            if self.distances[d].angle != 0.0 {
                continue;
            }
            let both_pinned = [m.a, m.b]
                .iter()
                .all(|&i| !self.members[i].gear.profile_shift.auto);
            let Some(running) = self.given_running(d) else {
                continue;
            };
            if !both_pinned {
                continue;
            }
            let Some(kind) = self.kind_of(k) else {
                continue;
            };
            let target = kind.nominal_of(running, self.distances[d].clearance.manual);
            let sign = kind.sign();
            // The shifts as the members will actually be cut at this helix —
            // a typed shift held to the undercut bound moves with the helix,
            // and the size must reach the distance with the shift it gets.
            let at = |beta: f64| -> f64 {
                let mut helix = vec![0.0; self.members.len()];
                helix[m.a] = beta;
                helix[m.b] = -sign * beta;
                let shifts: Vec<f64> = self.asked(&helix).iter().map(|a| a.settled).collect();
                self.nominal_of(k, &shifts, &helix).unwrap_or(f64::NAN) - target
            };
            // Straight teeth are the floor; the distance grows with the helix
            // without bound below ninety degrees.
            if at(0.0) > 0.0 {
                continue;
            }
            if let Some(beta) = crate::solve::brent(at, 0.0, 89.0, crate::solve::Tol::default()) {
                out[m.a] = Some(beta);
                out[m.b] = Some(-sign * beta);
            }
        }
        // Propagate through the meshes until nothing moves.
        loop {
            let mut moved = false;
            for (k, m) in self.meshes.iter().enumerate() {
                let sign = self.kind_of(k).map_or(1.0, MeshKind::sign);
                // The opposite hand across an external mesh, the same hand
                // across an internal one: `−sign · β`.
                match (out[m.a], out[m.b]) {
                    (Some(a), None) => {
                        out[m.b] = Some(-sign * a);
                        moved = true;
                    }
                    (None, Some(b)) => {
                        out[m.a] = Some(-sign * b);
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

    /// The width the overlap reads its size against: the least given width.
    fn given_width(&self) -> f64 {
        self.members
            .iter()
            .map(|m| m.gear.face_width.manual)
            .fold(f64::INFINITY, f64::min)
    }

    fn overlap_reads_size(&self) -> bool {
        self.members.iter().all(|m| !m.gear.face_width.auto)
    }

    fn size_taken_by_overlap(&self) -> bool {
        !self.overlap.auto && self.overlap_reads_size()
    }

    // ----------------------------------------------------------- params ---

    /// A member's parameters at a shift, its helix as the readings decide.
    fn base_params(&self, i: usize, helix: &[f64]) -> GearParams {
        let m = &self.members[i];
        GearParams {
            angular_shift: 0.0,
            index_offset: 0.0,
            module: m.module,
            pressure_angle: self.pressure_angle,
            teeth: m.gear.teeth,
            helix_angle: helix[i],
            profile_shift: m.gear.profile_shift.manual,
            addendum: m.gear.addendum,
            dedendum: m.gear.dedendum,
            root_radius: m.gear.root_radius,
            thickness_mod: m.thickness_mod,
        }
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

    /// The basic rack a mesh is cut with: both members' module, the stage's
    /// pressure angle, the mesh's helix.
    fn rack_of(&self, mesh: usize, helix: &[f64]) -> BasicRack {
        let m = self.meshes[mesh];
        BasicRack::new(
            self.members[m.a].module,
            self.pressure_angle,
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

    /// The zero-backlash distance of a mesh at these shifts, closed form.
    fn nominal_of(&self, mesh: usize, shifts: &[f64], helix: &[f64]) -> Option<f64> {
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
                    // Reversed, so that among equals the later mesh's own
                    // member — the last — is the one kept.
                    let absorber = [fb, fa, b, a]
                        .into_iter()
                        .filter(|&i| role[i] == Role::Free && undisturbed(i))
                        .map(|i| (i, (lever(i, first) - lever(i, m)).abs()))
                        .filter(|&(_, leverage)| leverage > 0.0)
                        .max_by(|p, q| p.1.total_cmp(&q.1))
                        .map(|(i, _)| i);
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
            let rack = self.rack_of(m, helix);
            let nominal = kind.nominal_of(running, self.distances[d].clearance.manual);
            let sum = shift_sum_for(
                rack.mt,
                rack.alpha_t,
                rack.alpha_n,
                self.tooth_sum(m),
                nominal,
            )?;
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
            if c == 0.0 {
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
            // which its kind solved from the clearance and nothing else.
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

    /// **The shifts the stage settles on**: closed where nothing is searched,
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
        if !self.optimisation.enabled {
            return fallback(&plan, &bound_by, super::Searched::NotAsked);
        }
        let free: Vec<usize> = (0..self.members.len())
            .filter(|&i| plan.role[i] == Role::Free)
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
        // crank, which its kind searched one at a time for a thirteenth of
        // the work. Meshes on an automatic distance with more than one mesh
        // are one component, an absorber carrying any member's move across
        // it; and an axis that touches two meshes joins them.
        let components = self.search_components(&plan, &axes, &free);
        // **A sized distance and the divisions chosen at it settle
        // together.** The division a search chooses moves the tips a
        // little, so a distance the tips size is sized again at what was
        // chosen, and the search run again at that distance — the hula
        // kind's own rounds. Three at most; the second usually moves nothing
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

    /// **The ratio with one more tooth on each member**, off the graph
    /// alone; the ratio itself where a count cannot change it, or where the
    /// changed counts are no mechanism.
    fn ratio_per_tooth(
        &self,
        wiring: &Wiring,
        teeth: &[u32],
        boundary: &super::StageBoundary,
    ) -> Vec<f64> {
        let base = wiring
            .unit_motion(teeth, boundary)
            .map_or(f64::NAN, |m| m.ratio());
        (0..teeth.len())
            .map(|i| {
                let mut more = teeth.to_vec();
                more[i] += 1;
                wiring
                    .unit_motion(&more, boundary)
                    .map_or(base, |m| m.ratio())
            })
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

    /// The shifts the shape settles on under a search — what the tests
    /// written against the kinds' own choosers ask.
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
            product *= crate::auto::MeshTrial {
                members: [cut(m.a), cut(m.b)],
                mesh: &bm.operating,
                path: &bm.path,
                min_contact_ratio: self.optimisation.min_contact_ratio,
                friction: m.sliding_friction,
            }
            .efficiency()?;
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
struct Chosen {
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
    /// At zero backlash, where the shifts put it.
    pub(crate) design: Mesh,
    /// At the running distance, where the teeth touch.
    pub(crate) operating: Mesh,
    pub(crate) path: ContactPath,
    pub(crate) running: f64,
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
            let design = Mesh::new(members[m.a].as_gear(), members[m.b].as_gear(), kind)
                .map_err(TrainError::Mesh)?;
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
                    if target.is_none()
                        && (design.running_distance(self.distances[d].clearance.manual) - r).abs()
                            > 1e-9 * r.abs().max(1.0)
                    {
                        return Err(TrainError::NoCommonDistance);
                    }
                    r
                }
                None => {
                    let r = target.unwrap_or_else(|| {
                        design.running_distance(self.distances[d].clearance.manual)
                    });
                    running[d] = Some(r);
                    r
                }
            };
            let operating = design.at(at).map_err(TrainError::Mesh)?;
            let path = ContactPath::new(
                members[m.a].as_gear(),
                members[m.b].tip_radius(),
                &operating,
            )
            .ok_or(TrainError::NoContact)?;
            meshes.push(BuiltMesh {
                kind,
                design,
                operating,
                path,
                running: at,
            });
        }
        Ok(Built {
            members,
            meshes,
            running,
        })
    }
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

/// Every shaft's speed and torque in one load case.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShaftCase {
    pub case: usize,
    /// Per local shaft, ground first.
    pub speeds: Vec<f64>,
    pub torques: Vec<f64>,
}

/// What a shape produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShapeResult {
    /// Input turns per output turn, signed.
    pub ratio: f64,
    /// **The ratio one more tooth on each member would give**, in member
    /// order — the graph's exact answer at `z_i + 1`, which is what a
    /// designer choosing counts wants beside the ratio: where a tooth
    /// moves it a lot, and where it moves it not at all.
    pub ratio_per_tooth: Vec<f64>,
    pub efficiency: Directional<f64>,
    /// **The power crossing the teeth, over the power in**, in each
    /// direction: one on a pair, under one on a set, and many times one
    /// where power circulates ([`super::flow::Flow::circulation`]). Zero
    /// where the stage does not turn that way.
    pub circulation: Directional<f64>,
    /// Play at the output shaft driven forward, at the input driven back.
    pub backlash: Directional<super::Backlash>,
    pub distances: Vec<DistanceReport>,
    pub overlap: f64,
    /// One per replicated axis, in axis order.
    pub layouts: Vec<LayoutReport>,
    pub cases: Vec<ShaftCase>,
    pub members: Vec<GearResult>,
    pub meshes: Vec<MeshReport>,
    pub notes: Vec<Note>,
}

/// # Errors
///
/// As every stage kind's own solve: a mesh that cannot mesh, no contact, a
/// distance no shift reaches, a material not in the library, a member whose
/// root cannot be rated, or a boundary that leaves the motion undetermined.
pub fn solve_shape(
    shape: &Shape,
    loads: &StageLoads,
    lib: &MaterialLibrary,
    reversal: super::Reversal,
) -> Result<ShapeResult, TrainError> {
    let n = shape.members.len();
    let helix = shape.helix_angles();

    // ---- motion first: tooth counts and topology, before any geometry.
    let wiring = Constrained::wiring(shape);
    let boundary = super::StageBoundary::of(loads, &wiring, &Constrained::ports(shape));
    let teeth = super::teeth_of(shape.members());
    let motion = wiring.unit_motion(&teeth, &boundary)?;

    // ---- crossed axes: the point-contact model, over the pair it is
    // written for.
    if let Some((pair, kind)) = shape.as_crossed_pair() {
        let sized = pair.sized();
        let r = super::crossed::solve_crossed_pair(&sized, kind, loads, lib, &motion)?;
        return Ok(ShapeResult {
            ratio: r.ratio,
            ratio_per_tooth: shape.ratio_per_tooth(&wiring, &teeth, &boundary),
            efficiency: r.mesh.efficiency,
            // The one mesh carries the whole of it, either way it turns.
            circulation: Directional {
                forward: f64::from(u8::from(r.mesh.efficiency.forward > 0.0)),
                backward: f64::from(u8::from(r.mesh.efficiency.backward > 0.0)),
            },
            backlash: r.mesh.backlash_by_drive(),
            distances: vec![DistanceReport {
                nominal: vec![r.centre_distance_nominal],
                running: r.centre_distance,
                clearance: r.clearance,
                sized_by: None,
            }],
            overlap: 0.0,
            layouts: Vec::new(),
            cases: Vec::new(),
            members: r.gears.to_vec(),
            meshes: vec![r.mesh],
            notes: r.notes,
        });
    }
    if shape.distances.iter().any(|d| d.angle != 0.0) {
        return Err(TrainError::Wiring(super::WiringError::Unsolvable));
    }
    let system = wiring.alone(&teeth)?;
    let speed: Vec<f64> = system
        .motion(&boundary.conditions)
        .map_err(|_| super::WiringError::Unsolvable)?
        .values
        .iter()
        .map(|r| r.to_f64())
        .collect();
    let held: Vec<Shaft> = boundary.held();
    // A stage driven at two of its ports is one motion and no arrangement
    // to rate under: the second input's torque is nobody's to know.
    if boundary
        .conditions
        .iter()
        .filter(|c| matches!(c, crate::kinematics::Condition::Drive(_)))
        .count()
        > 1
    {
        return Err(TrainError::Wiring(super::WiringError::Unsolvable));
    }

    // ---- the shifts, and every member and mesh built at them.
    let chosen = shape.chosen_at(&crate::auto::Search::SHIPPED, &helix)?;
    let x = chosen.shifts;
    let built = shape.build(&x, &helix, &chosen.held)?;

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

    // ---- each mesh's own efficiency, sliding and at rest.
    let mesh_efficiency = |k: usize, mu: f64| -> Directional<f64> {
        let bm = &built.meshes[k];
        let a = built.members[shape.meshes[k].a].as_gear();
        Directional::of(|d| efficiency(&bm.path, &bm.operating, a, mu, d))
    };
    let sliding: Vec<Directional<f64>> = (0..shape.meshes.len())
        .map(|k| mesh_efficiency(k, shape.meshes[k].sliding_friction))
        .collect();
    let at_rest: Vec<Directional<f64>> = (0..shape.meshes.len())
        .map(|k| mesh_efficiency(k, shape.meshes[k].static_friction))
        .collect();

    // ---- where the power goes, in each direction, at unit torque.
    let flows = |etas: &[Directional<f64>]| -> Directional<Option<super::flow::Flow>> {
        let meshes: Vec<super::flow::MeshFlow> = shape
            .meshes
            .iter()
            .enumerate()
            .map(|(k, m)| super::flow::MeshFlow {
                a: shape.shaft_of(m.a),
                b: shape.shaft_of(m.b),
                frame: wiring.frame(k).unwrap_or(GROUND),
                za: f64::from(shape.members[m.a].gear.teeth),
                zb: built.meshes[k].kind.sign() * f64::from(shape.members[m.b].gear.teeth),
                efficiency: etas[k],
                paths: f64::from(wiring.paths_seen(m.a).max(wiring.paths_seen(m.b))),
            })
            .collect();
        Directional::of(|d| {
            let (input, output) = match d {
                Drive::Forward => (boundary.input, boundary.output),
                Drive::Backward => (boundary.output, boundary.input),
            };
            super::flow::solve(
                speed.len(),
                &meshes,
                &speed,
                &super::flow::Asked {
                    input,
                    torque: speed[input].signum(),
                    output,
                    reactions: held.clone(),
                },
            )
        })
    };
    let moving = flows(&sliding);
    let resting = flows(&at_rest);
    let Some(forward) = moving.forward.as_ref() else {
        return Err(TrainError::NoPowerFlow);
    };
    let stage_efficiency = Directional {
        forward: forward.efficiency,
        backward: moving.backward.as_ref().map_or(0.0, |b| b.efficiency),
    }
    .once_moving(&Directional {
        forward: resting.forward.as_ref().map_or(0.0, |f| f.efficiency),
        backward: resting.backward.as_ref().map_or(0.0, |b| b.efficiency),
    });

    // **A case's torque is the mesh force referred to the stage's input
    // shaft** — what the train hands every stage (`solve_train`'s walk): a
    // load from the far end is divided by the ratio and nothing else, the
    // losses sitting between the mesh and the shaft beyond. So a forward case
    // enters at the input shaft at that torque, and a backward case enters
    // at the *output* shaft at that torque times the ratio, and the flow in
    // its direction is scaled to the shaft it enters by. Reading a backward
    // case's torque as the input shaft's *delivered* torque instead — which
    // an epicyclic set did — overstated everything inside it by `1/η`.
    let ratio = motion.ratio().abs();
    let flow_for = |d: Drive| -> Option<&super::flow::Flow> {
        match d {
            Drive::Forward => moving.forward.as_ref(),
            Drive::Backward => moving.backward.as_ref(),
        }
    };
    let scale_for = |c: &super::StageLoad| -> Option<(f64, &super::flow::Flow)> {
        let f = flow_for(c.drive)?;
        let (entry, torque) = match c.drive {
            Drive::Forward => (boundary.input, c.torque),
            Drive::Backward => (boundary.output, c.torque * ratio),
        };
        let at_entry = f.shaft_torques[entry];
        (at_entry != 0.0).then(|| (torque / at_entry.abs(), f))
    };
    let paths = |k: usize| -> f64 {
        let m = shape.meshes[k];
        f64::from(wiring.paths_seen(m.a).max(wiring.paths_seen(m.b)))
    };
    // The tangential force a mesh instance carries in a case, quoted as a
    // torque at member `a`: the driving member's torque, read across.
    let pressing_torque_at_a = |k: usize, c: &super::StageLoad| -> f64 {
        let Some((scale, f)) = scale_for(c) else {
            return 0.0;
        };
        let on_a = (f.mesh_torques[k] * scale / paths(k)).abs();
        match f.directions[k] {
            Drive::Forward => on_a,
            // `b` drives: its torque is `η` more than the row says of `a`'s,
            // and the force on the flanks is its.
            Drive::Backward => on_a / f.efficiency_of_mesh(k, &sliding),
        }
    };
    // Each mesh's worst pressing torque over the cases, and every case as a
    // scale of it — one evaluation per mesh, every case a multiplication.
    let scaled: Vec<(f64, Vec<f64>)> = (0..shape.meshes.len())
        .map(|k| loads.scaled(|c| pressing_torque_at_a(k, c)))
        .collect();

    // ---- bending sections: one per (member, mesh) pair.
    let sharing = shape.load_sharing;
    let mut bendings: Vec<Vec<(usize, Option<super::Bending>)>> =
        (0..n).map(|_| Vec::new()).collect();
    for (k, m) in shape.meshes.iter().enumerate() {
        let cr = built.meshes[k].path.contact_ratio;
        for i in [m.a, m.b] {
            bendings[i].push((
                k,
                built.members[i].bending(cr, sharing, shape.members[i].gear.rim_thickness),
            ));
        }
    }
    // **A member with no root section in any of its meshes refuses the
    // stage**; one rated in some mesh keeps that rating and says which flank
    // went unrated — a planet whose ring-side load point falls off its flank
    // is still rated on its sun side, which is what the set's own kind did
    // without saying so. A ring refuses only its own rating.
    for (m, b) in shape.members.iter().zip(&bendings) {
        if m.ring.is_none() && b.iter().all(|(_, b)| b.is_none()) {
            return Err(TrainError::NoRootSection);
        }
    }

    // ---- contact stress per mesh at the probe width.
    let e_star: Vec<f64> = (0..shape.meshes.len())
        .map(|k| contact_modulus(&materials[shape.meshes[k].a], &materials[shape.meshes[k].b]))
        .collect();
    let contact_at = |k: usize, width: f64| -> Result<crate::strength::ContactStress, TrainError> {
        let m = shape.meshes[k];
        let bm = &built.meshes[k];
        contact_stress(
            &bm.path,
            &bm.operating,
            built.members[m.a].as_gear(),
            PARALLEL_AXES,
            &Load::new(scaled[k].0, width),
            e_star[k],
        )
        .ok_or(TrainError::NoContact)
    };
    let probes: Vec<crate::strength::ContactStress> = (0..shape.meshes.len())
        .map(|k| contact_at(k, PROBE))
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
    let always_reverses = |i: usize| bendings[i].len() > 1;
    let rating = |i: usize, widths: &[f64]| -> MemberRating<'_> {
        let meshes: Vec<(Loading, Vec<f64>)> = bendings[i]
            .iter()
            .map(|(k, b)| {
                let m = shape.meshes[*k];
                let side = usize::from(m.b == i);
                let probe = Load::new(scaled[*k].0, PROBE);
                let ft = probe.tangential(built.members[m.a].as_gear());
                (
                    Loading {
                        bending: b.as_ref().and_then(|b| stress_at(b, ft, &probe)),
                        contact: probes[*k].governing(side),
                        measured_at: PROBE,
                        carried_at: widths[*k],
                    },
                    scaled[*k].1.clone(),
                )
            })
            .collect();
        MemberRating {
            material: &materials[i],
            reversal,
            always_reverses: always_reverses(i),
            cases: Loading::for_cases(loads, &meshes),
        }
    };
    let probe_widths = vec![PROBE; shape.meshes.len()];
    // ...and the width a given axial contact ratio needs, a floor under every
    // automatic width: one helix per mesh, the least floor the largest.
    let for_overlap = shape
        .meshes
        .iter()
        .map(|m| {
            super::width_for_overlap(&shape.overlap, helix[m.a], shape.members[m.a].module)
                .unwrap_or(0.0)
        })
        .fold(0.0_f64, f64::max);
    let asks: Vec<f64> = (0..n)
        .map(|i| {
            shape.members[i]
                .gear
                .face_sources
                .width_for(
                    &rating(i, &probe_widths).asks(),
                    shape.members[i].gear.face_width.manual,
                )
                .max(for_overlap)
        })
        .collect();
    // **A member's automatic width is the largest requirement of any mesh it
    // is in**, because the narrower face carries the pair.
    let mesh_ask: Vec<f64> = shape
        .meshes
        .iter()
        .map(|m| asks[m.a].max(asks[m.b]))
        .collect();
    let widths: Vec<f64> = (0..n)
        .map(|i| {
            let wanted = bendings[i]
                .iter()
                .map(|(k, _)| mesh_ask[*k])
                .fold(0.0_f64, f64::max);
            shape.members[i].gear.face_width.resolve(wanted)
        })
        .collect();
    let mesh_widths: Vec<f64> = shape
        .meshes
        .iter()
        .map(|m| widths[m.a].min(widths[m.b]))
        .collect();
    let rated_contact: Vec<crate::strength::ContactStress> = (0..shape.meshes.len())
        .map(|k| contact_at(k, mesh_widths[k]))
        .collect::<Result<_, _>>()?;

    // ---- backlash: each mesh's play, and where it shows on each shaft.
    let play_of = |k: usize, a: f64| -> f64 {
        let bm = &built.meshes[k];
        let m = shape.meshes[k];
        let d = shape.distance_of(k).unwrap_or(0);
        let rack = shape.rack_of(k, &helix);
        let bb = crate::plane::base_helix_angle(
            helix[m.a].to_radians(),
            shape.pressure_angle.to_radians(),
        );
        let slide = shape.distances[d].axial_clearance * bb.sin().abs();
        let p_bn = std::f64::consts::PI * rack.mn * shape.pressure_angle.to_radians().cos();
        // The row's play: `Δ = j |Σz| / a`, plus the axial float's.
        bm.design.backlash(a).unwrap_or(0.0) * shape.tooth_sum(k).abs() / a
            + 2.0 * std::f64::consts::PI * slide / p_bn
    };
    // Play at a shaft per unit of play in mesh `k`, with the input and the
    // held shafts standing still.
    let coefficient = |k: usize, at: Shaft, input: Shaft| -> f64 {
        let mut conditions = boundary.conditions.clone();
        for c in conditions.iter_mut() {
            if matches!(c, crate::kinematics::Condition::Drive(_)) {
                *c = crate::kinematics::Condition::Free;
            }
        }
        conditions[input] = crate::kinematics::Condition::Ground;
        system
            .play(k, &conditions)
            .and_then(Result::ok)
            .map_or(0.0, |s| s.values[at].to_f64().abs())
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
    let backlash_at = |at: Shaft, input: Shaft| -> super::Backlash {
        super::Backlash::banded(0.0, 1.0, 1.0, |t| {
            (0..shape.meshes.len())
                .map(|k| coefficient(k, at, input) * play_of(k, at_band(k, t)))
                .sum::<f64>()
                .to_degrees()
        })
    };
    let backlash = Directional {
        forward: backlash_at(boundary.output, boundary.input),
        backward: backlash_at(boundary.input, boundary.output),
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
        let nominal: Vec<f64> = meshes.iter().map(|&k| built.meshes[k].design.a_w).collect();
        let Some(&first) = meshes.first() else {
            continue;
        };
        let running = built.running[d].unwrap_or(0.0);
        let clearance =
            built.meshes[first].kind.sign() * (running - built.meshes[first].design.a_w);
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
                bm.design.a_w,
                bm.kind.sign() * (running - bm.design.a_w),
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
    if let Some(m) = shape.meshes.first() {
        notes.extend(super::overlap_notes(
            shape.size_taken_by_overlap(),
            &shape.overlap,
            shape.members[m.a].module,
            shape.given_width(),
            helix[m.a],
        ));
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
                .filter(|&i| shape.axis_of_shaft(shape.shaft_of(i)) == Some(axis))
                .collect();
            // The radius the instances stand at: the distance from the axis
            // the carrier turns about, not whatever mesh comes first — a
            // planet meshing another planet has a distance that is neither.
            let central = a.carried_by.and_then(|c| shape.axis_of_shaft(c))?;
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
            // The assembly rule is known for one gear on the axis meshing two
            // central members, and is not asserted elsewhere.
            let simple = (on_axis.len() == 1)
                .then(|| {
                    let centrals: Vec<u32> = shape
                        .meshes
                        .iter()
                        .filter(|m| m.a == on_axis[0] || m.b == on_axis[0])
                        .map(|m| {
                            shape.members[if m.a == on_axis[0] { m.b } else { m.a }]
                                .gear
                                .teeth
                        })
                        .collect();
                    (centrals.len() == 2).then_some(centrals)
                })
                .flatten();
            let equal_spacing = simple.as_ref().map(|c| (c[0] + c[1]) % count == 0);
            let simultaneous_meshing = simple.as_ref().map(|c| c.iter().all(|z| z % count == 0));
            Some(LayoutReport {
                axis,
                count,
                equal_spacing,
                simultaneous_meshing,
                clearance,
                clearance_ok: clearance >= shape.min_planet_clearance,
            })
        })
        .collect();
    for l in &layouts {
        notes.push(Note::new(key::STAGE_PLANETS_SHARE_LOAD_EQUALLY).count("planets", l.count));
        if l.equal_spacing == Some(false) {
            notes.push(Note::new(key::STAGE_PLANETS_NOT_EVENLY_SPACED).count("planets", l.count));
        }
        if !l.clearance_ok {
            notes.push(
                Note::new(key::STAGE_PLANET_CLEARANCE_BELOW_MINIMUM)
                    .number("gap", l.clearance, 3)
                    .number("minimum", shape.min_planet_clearance, 3),
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
        out.extend(reversal.note_for(reversal.reverses(always_reverses(i), loads.any_reverse())));
        let g = &shape.members[i].gear;
        if shape.members[i].ring.is_none() {
            out.extend(g.shift_asked(&shape.base_params(i, &helix)).note());
            out.extend(
                g.addendum_asked(&GearParams {
                    profile_shift: x[i],
                    ..shape.base_params(i, &helix)
                })
                .note(),
            );
        }
        out.extend(g.face_width_note());
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
    // What a member's *shaft* delivers, `η` less on the driven side, is the
    // shaft's figure (`ShapeResult::cases`), not the gear's. A member in two
    // meshes reports the larger.
    let member_torque = |i: usize, c: &super::StageLoad| -> f64 {
        bendings[i]
            .iter()
            .map(|(k, _)| {
                let m = shape.meshes[*k];
                let at_a = pressing_torque_at_a(*k, c);
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
            let cases = rating(i, &mesh_widths)
                .rated()
                .into_iter()
                .map(|r| {
                    let c = r.load;
                    let m = motion.members[i];
                    r.into_case(
                        member_torque(i, &c),
                        (m.speed.scale(c.speed), m.against_frame.scale(c.speed)),
                        m.engagements,
                    )
                })
                .collect();
            GearResult::of(MemberFacts {
                profile_shift: x[i],
                params: built.members[i].params(),
                input: &shape.members[i].gear,
                cases,
                face_width: widths[i],
                recommended_face_width: None,
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
            let (a, b) = (&built.members[m.a], &built.members[m.b]);
            super::line_mesh_report(
                loads,
                super::LineMesh {
                    power_through: Directional {
                        forward: moving.forward.as_ref().map_or(0.0, |f| f.mesh_powers[k]),
                        backward: moving.backward.as_ref().map_or(0.0, |b| b.mesh_powers[k]),
                    },
                    coprime: super::gcd(
                        shape.members[m.a].gear.teeth,
                        shape.members[m.b].gear.teeth,
                    ) == 1,
                    contact_ratios: ContactRatios::of(
                        bm.path.contact_ratio,
                        mesh_widths[k],
                        helix[m.a],
                        shape.members[m.a].module,
                    ),
                    operating_pressure_angle: bm.operating.alpha_w.to_degrees(),
                    efficiency: sliding[k].once_moving(&at_rest[k]),
                    contact: scaled[k]
                        .1
                        .iter()
                        .map(|&s| {
                            super::ContactPatch::line(
                                &rated_contact[k],
                                s,
                                mesh_widths[k],
                                e_star[k],
                            )
                        })
                        .collect(),
                    backlash: [
                        member_backlash(k, MeshSide::First),
                        member_backlash(k, MeshSide::Second),
                    ],
                    flank_interference: bm
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
            )
        })
        .collect();

    let overlap = meshes
        .first()
        .and_then(|m| m.line.as_ref().map(|l| l.contact_ratios.overlap))
        .unwrap_or(0.0);

    Ok(ShapeResult {
        ratio: motion.ratio(),
        ratio_per_tooth: shape.ratio_per_tooth(&wiring, &teeth, &boundary),
        efficiency: stage_efficiency,
        circulation: Directional {
            forward: moving
                .forward
                .as_ref()
                .map_or(0.0, super::flow::Flow::circulation),
            backward: moving
                .backward
                .as_ref()
                .map_or(0.0, super::flow::Flow::circulation),
        },
        backlash,
        distances,
        overlap,
        layouts,
        cases: loads
            .cases
            .iter()
            .map(|c| ShaftCase {
                case: c.case,
                speeds: speed.iter().map(|w| w * c.speed).collect(),
                torques: scale_for(c).map_or_else(
                    || vec![0.0; speed.len()],
                    |(scale, f)| f.shaft_torques.iter().map(|t| t * scale).collect(),
                ),
            })
            .collect(),
        members,
        meshes,
        notes,
    })
}

impl ShapeResult {
    /// **A shape of two members and one mesh read as the pair it is** — the
    /// crossed model's own result shape, which the tests written against the
    /// pair kind still read. Test-only: nothing in production wants a
    /// stage's result in a kind's shape.
    #[cfg(test)]
    pub(crate) fn pair_view(&self) -> super::CrossedResult {
        let d = &self.distances[0];
        super::CrossedResult {
            ratio: self.ratio,
            centre_distance_nominal: d.nominal[0],
            clearance: d.clearance,
            centre_distance: d.running,
            mesh: self.meshes[0].clone(),
            gears: [self.members[0].clone(), self.members[1].clone()],
            notes: self.notes.clone(),
        }
    }
}

impl super::flow::Flow {
    /// The efficiency the flow charged mesh `k` in the direction it chose.
    fn efficiency_of_mesh(&self, k: usize, etas: &[Directional<f64>]) -> f64 {
        *etas[k].get(self.directions[k])
    }
}

// -------------------------------------------------- what every kind owes ---

impl Constrained for Shape {
    fn members(&self) -> Vec<&StageGear> {
        self.members.iter().map(|m| &m.gear).collect()
    }

    /// Every input relief may turn: each distance's, the overlap, and each
    /// member's. **One distance is addressed for now** — `Freedom` names a
    /// stage's centre distance and clearance without saying which pair of
    /// axes, and every shape converted from a kind has one.
    fn inputs(&mut self) -> Vec<(Freedom, &mut Auto<f64>)> {
        let mut out = Vec::new();
        if let Some(d) = self.distances.first_mut() {
            out.push((Freedom::CentreDistance, &mut d.distance));
            out.push((Freedom::Clearance, &mut d.clearance));
        }
        out.push((Freedom::Overlap, &mut self.overlap));
        let mut members = self.members.iter_mut();
        if let Some(m) = members.next() {
            out.push((Freedom::FirstPitchDiameter, &mut m.pitch_diameter));
            out.extend(super::member_inputs(std::iter::once(&mut m.gear)));
        }
        // ...and every other member's, numbered from one on.
        for (i, m) in members.enumerate() {
            let StageGear {
                profile_shift,
                helix_angle,
                face_width,
                ..
            } = &mut m.gear;
            out.push((Freedom::Member(i + 1, MemberFreedom::Shift), profile_shift));
            out.push((Freedom::Member(i + 1, MemberFreedom::Helix), helix_angle));
            out.push((Freedom::Member(i + 1, MemberFreedom::FaceWidth), face_width));
        }
        out
    }

    /// Each member's helix in its own hand, the first member's diameter, and
    /// the overlap where it decides the size.
    fn readings(&self) -> Vec<Reading> {
        let mut out: Vec<Reading> = self
            .members
            .iter()
            .enumerate()
            .map(|(i, m)| Reading::helix(i, &m.gear, |b| b))
            .collect();
        if let Some(first) = self.members.first() {
            let z1 = f64::from(first.gear.teeth.max(1)) * first.module;
            out.push(Reading {
                freedom: Freedom::FirstPitchDiameter,
                helix: (!first.pitch_diameter.auto).then(|| {
                    (z1 / first.pitch_diameter.manual)
                        .clamp(-1.0, 1.0)
                        .acos()
                        .to_degrees()
                }),
            });
        }
        if self.overlap_reads_size() {
            if let Some(m) = self.meshes.first() {
                out.push(Reading::overlap(
                    &self.overlap,
                    self.members[m.a].module,
                    self.given_width(),
                ));
            }
        }
        out
    }

    /// **One relation per distance**: the distance, the shifts of every
    /// member on its meshes, the clearance and the size are related by one
    /// equation per mesh on it, so that many may be given less the meshes.
    /// The distance gives way first — it is the one a designer expects to
    /// give when they pin everything else — then the shifts in member order,
    /// then the clearance, then the size, since a shift moves the teeth
    /// where a size changes them. And of the distance and the clearance at
    /// most one may be automatic.
    ///
    /// The clearance sits *after* the shifts, where the pair had it before
    /// them, because a shape with three shifts on one distance can be over
    /// by two: relieving the clearance would hand it straight back to the
    /// group below, which pins it again, and the walk would never settle.
    /// A shift gives instead, which is what the set's own kind did.
    fn freedoms(&self) -> Vec<FreedomGroup> {
        let readings = self.readings();
        let mut groups = Vec::new();
        for d in 0..self.distances.len() {
            let meshes = self.meshes_on(d);
            if meshes.is_empty() {
                continue;
            }
            let mut members: Vec<usize> = Vec::new();
            for &k in &meshes {
                for i in [self.meshes[k].a, self.meshes[k].b] {
                    if !members.contains(&i) {
                        members.push(i);
                    }
                }
            }
            let mut order = vec![vec![Freedom::CentreDistance]];
            order.extend(
                members
                    .iter()
                    .map(|&i| vec![Freedom::Member(i, MemberFreedom::Shift)]),
            );
            order.push(vec![Freedom::Clearance]);
            order.push(super::entry(&readings));
            let entries = order.len();
            groups.push(FreedomGroup {
                given_at_most: entries - meshes.len(),
                automatic_at_most: entries,
                order,
            });
            groups.push(super::distance_and_clearance());
            // One distance is addressed by name; see `inputs`.
            break;
        }
        // On crossed shafts an axial contact ratio is nothing at all, and is
        // turned back automatic.
        if self.distances.iter().any(|d| d.angle != 0.0) {
            groups.push(super::always_automatic(Freedom::Overlap));
        }
        groups
    }

    /// The shape *is* the topology: each member spins with its shaft in the
    /// frame its axis stands still in, and a mesh's sign is its members'.
    fn wiring(&self) -> Wiring {
        Wiring {
            shafts: (0..=self.shafts.len()).map(|s| self.label_of(s)).collect(),
            mounts: (0..self.members.len())
                .map(|i| Mount {
                    spins_with: self.shaft_of(i),
                    axis_fixed_in: self.frame_of_member(i),
                    replicated: self.replicated(self.shaft_of(i)),
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
                            .count_of(self.shaft_of(m.a))
                            .max(self.count_of(self.shaft_of(m.b))),
                    }
                })
                .collect(),
        }
    }

    /// **Every shaft that is not replicated is a port**, in shaft order — a
    /// pair's two members, a set's sun, carrier and ring — and what is held
    /// by convention is the first ring's shaft, where there is a ring.
    fn ports(&self) -> Ports {
        let ports: Vec<Shaft> = (1..=self.shafts.len())
            .filter(|&s| !self.replicated(s))
            .collect();
        let held: Vec<Shaft> = self
            .members
            .iter()
            .filter(|m| m.ring.is_some())
            .map(|m| m.shaft)
            .find(|s| ports.contains(s))
            .into_iter()
            .collect();
        Ports { ports, held }
    }
}

// ----------------------------------------------------- from each kind ---

impl From<&super::PairStage> for Shape {
    /// A spur or crossed gear pair: [`Shape::from_pair`] without the worm's
    /// proportions.
    fn from(p: &super::PairStage) -> Self {
        Self::from_pair(p, super::PairKind::Spur)
    }
}

impl Shape {
    /// Two axes in ground at the pair's shaft angle, one mesh, one distance.
    #[must_use]
    pub fn from_pair(p: &super::PairStage, kind: super::PairKind) -> Self {
        let member = |i: usize, thickness_mod: f64| Member {
            shaft: i + 1,
            gear: p.gears[i].clone(),
            module: p.module,
            thickness_mod,
            ring: None,
            pitch_diameter: if i == 0 {
                p.pitch_diameter
            } else {
                Auto::automatic(0.0)
            },
        };
        Self {
            pressure_angle: p.pressure_angle,
            overlap: p.overlap,
            optimisation: p.optimisation,
            load_sharing: p.load_sharing,
            min_planet_clearance: 0.0,
            axes: vec![
                Axis {
                    carried_by: None,
                    count: 1,
                },
                Axis {
                    carried_by: None,
                    count: 1,
                },
            ],
            shafts: vec![ShaftOn { axis: 0 }, ShaftOn { axis: 1 }],
            members: vec![member(0, p.thickness_mod), member(1, 2.0 - p.thickness_mod)],
            meshes: vec![MeshInput {
                a: 0,
                b: 1,
                sliding_friction: p.sliding_friction,
                static_friction: p.static_friction,
            }],
            distances: vec![Distance {
                axes: [0, 1],
                angle: p.shaft_angle,
                worm: kind == super::PairKind::Worm,
                distance: p.centre_distance,
                clearance: p.clearance,
                tip_clearance: 0.0,
                tolerance_plus: p.tolerance_plus,
                tolerance_minus: p.tolerance_minus,
                axial_clearance: p.axial_clearance,
            }],
        }
    }

    /// **A crossed pair, read back as the pair the screw model takes.** The
    /// point-contact model in [`super::crossed`] is written over a
    /// [`super::PairStage`], and a shape whose one distance is at an angle
    /// is exactly one of those; the view is built here so that model is
    /// called and not copied. `None` where the shape is more than a pair.
    fn as_crossed_pair(&self) -> Option<(super::PairStage, super::PairKind)> {
        let d = self.distances.first()?;
        if d.angle == 0.0
            || self.distances.len() != 1
            || self.members.len() != 2
            || self.meshes.len() != 1
        {
            return None;
        }
        let m = self.meshes[0];
        let (a, b) = (&self.members[m.a], &self.members[m.b]);
        Some((
            super::PairStage {
                module: a.module,
                pressure_angle: self.pressure_angle,
                shaft_angle: d.angle,
                pitch_diameter: a.pitch_diameter,
                overlap: self.overlap,
                sliding_friction: m.sliding_friction,
                static_friction: m.static_friction,
                thickness_mod: a.thickness_mod,
                centre_distance: d.distance,
                clearance: d.clearance,
                tolerance_plus: d.tolerance_plus,
                tolerance_minus: d.tolerance_minus,
                optimisation: self.optimisation,
                load_sharing: self.load_sharing,
                axial_clearance: d.axial_clearance,
                gears: [a.gear.clone(), b.gear.clone()],
            },
            if d.worm {
                super::PairKind::Worm
            } else {
                super::PairKind::Spur
            },
        ))
    }
}

impl From<&super::PlanetaryStage> for Shape {
    /// A central axis with the sun, the carrier and the ring on it, a planet
    /// axis carried by the carrier and replicated `N` times, two meshes on
    /// the one distance between them. Shafts numbered as the kind numbered
    /// them: sun 1, carrier 2, ring 3, planet 4.
    fn from(s: &super::PlanetaryStage) -> Self {
        let member =
            |shaft: Shaft, gear: &StageGear, thickness_mod: f64, ring: Option<Cutter>| Member {
                shaft,
                gear: gear.clone(),
                module: s.module,
                thickness_mod,
                ring,
                pitch_diameter: Auto::automatic(0.0),
            };
        Self {
            pressure_angle: s.pressure_angle,
            overlap: s.overlap,
            optimisation: s.optimisation,
            load_sharing: s.load_sharing,
            min_planet_clearance: s.min_planet_clearance,
            axes: vec![
                Axis {
                    carried_by: None,
                    count: 1,
                },
                Axis {
                    carried_by: Some(2),
                    count: s.planets.max(1),
                },
            ],
            shafts: vec![
                ShaftOn { axis: 0 },
                ShaftOn { axis: 0 },
                ShaftOn { axis: 0 },
                ShaftOn { axis: 1 },
            ],
            members: vec![
                member(1, &s.sun, s.thickness_mod, None),
                member(4, &s.planet, 2.0 - s.thickness_mod, None),
                member(3, &s.ring, 2.0 - s.thickness_mod, Some(s.cutter)),
            ],
            meshes: vec![
                MeshInput {
                    a: 0,
                    b: 1,
                    sliding_friction: s.sliding_friction_sun_planet,
                    static_friction: s.static_friction_sun_planet,
                },
                MeshInput {
                    a: 1,
                    b: 2,
                    sliding_friction: s.sliding_friction_planet_ring,
                    static_friction: s.static_friction_planet_ring,
                },
            ],
            distances: vec![Distance {
                axes: [0, 1],
                angle: 0.0,
                worm: false,
                distance: s.centre_distance,
                clearance: s.clearance,
                tip_clearance: 0.0,
                tolerance_plus: s.tolerance_plus,
                tolerance_minus: s.tolerance_minus,
                axial_clearance: 0.0,
            }],
        }
    }
}

impl From<&super::HulaStage> for Shape {
    /// **The hula stage as a shape**: a central axis with the grounded gear,
    /// the crank and the output on it, a wobble axis carried by the crank
    /// with the two wobble gears on one shaft, two internal meshes on the
    /// one distance — the crank offset — which the tips size where it is
    /// automatic. Which member of each pair is the ring is a tooth count,
    /// the larger; the pinion's `k` is the ring's too, as the kind had it.
    /// Shafts: crank 1, output 2, grounded 3, wobble 4 — the driven one
    /// first, the output next, the grounded gear's ring the first ring
    /// listed and so the one held by convention.
    fn from(h: &super::HulaStage) -> Self {
        let mesh_of = |i: usize| i / 2;
        let is_ring = |i: usize| {
            let (a, b) = (mesh_of(i) * 2, mesh_of(i) * 2 + 1);
            h.gears[i].teeth > h.gears[if i == a { b } else { a }].teeth
        };
        let shaft_of = |i: usize| match i {
            0 => 3,
            3 => 2,
            _ => 4,
        };
        let members: Vec<Member> = (0..4)
            .map(|i| Member {
                shaft: shaft_of(i),
                gear: h.gears[i].clone(),
                module: h.module[mesh_of(i)],
                thickness_mod: h.thickness_mod[mesh_of(i)],
                ring: is_ring(i).then_some(h.cutter[mesh_of(i)]),
                pitch_diameter: Auto::automatic(0.0),
            })
            .collect();
        let mesh = |m: usize| {
            let (a, b) = (m * 2, m * 2 + 1);
            let (pinion, ring) = if is_ring(a) { (b, a) } else { (a, b) };
            MeshInput {
                a: pinion,
                b: ring,
                sliding_friction: h.sliding_friction[m],
                static_friction: h.static_friction[m],
            }
        };
        Self {
            pressure_angle: h.pressure_angle,
            overlap: h.overlap,
            optimisation: h.optimisation,
            load_sharing: h.load_sharing,
            min_planet_clearance: 0.0,
            axes: vec![
                Axis {
                    carried_by: None,
                    count: 1,
                },
                Axis {
                    carried_by: Some(1),
                    count: 1,
                },
            ],
            shafts: vec![
                ShaftOn { axis: 0 },
                ShaftOn { axis: 0 },
                ShaftOn { axis: 0 },
                ShaftOn { axis: 1 },
            ],
            members,
            meshes: vec![mesh(0), mesh(1)],
            distances: vec![Distance {
                axes: [0, 1],
                angle: 0.0,
                worm: false,
                distance: h.offset,
                clearance: h.running_clearance,
                tip_clearance: h.clearance,
                tolerance_plus: h.tolerance_plus,
                tolerance_minus: h.tolerance_minus,
                axial_clearance: 0.0,
            }],
        }
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
    //! The gate that held the shape against the kinds figure for figure —
    //! every member's every case and every mesh's every figure, on a pair, a
    //! worm, and every arrangement of a set — lived beside the kinds while
    //! both existed and went with them (`git show ac0dccc`); what it found is
    //! in `docs/corrections.md`, and the corpus records what moved.

    use super::*;
    use crate::planetary::{Arrangement, PlanetaryShaft};
    use crate::train::{test_library, PairKind, PairStage, PlanetaryStage, StageLoad};

    /// Both directions and both kinds, at a torque and a speed.
    fn loads() -> StageLoads {
        let mut l = StageLoads::at(2.0, 3000.0);
        l.cases.push(StageLoad {
            case: 2,
            kind: super::super::CaseKind::Ultimate,
            drive: Drive::Backward,
            torque: 0.5,
            speed: 0.0,
            turns: None,
        });
        l
    }

    /// How far a set's two meshes disagree about the one distance, from the
    /// zero-backlash distances it reports, each opened by the clearance its
    /// own way.
    fn residual(r: &ShapeResult) -> f64 {
        let d = &r.distances[0];
        ((d.nominal[0] + d.clearance) - (d.nominal[1] - d.clearance)).abs()
    }

    /// A set through the shape, under its convention or a boundary.
    fn solve_set(
        set: &PlanetaryStage,
        loads: &StageLoads,
        lib: &MaterialLibrary,
    ) -> Result<ShapeResult, TrainError> {
        solve_shape(
            &Shape::from(set),
            loads,
            lib,
            super::super::Reversal::default(),
        )
    }

    /// **A crossed distance reaches the point-contact model**, and the shape
    /// reports what it reports: a point contact, no bending on the worm.
    #[test]
    fn a_crossed_distance_is_a_point_contact() {
        for (pair, kind) in [
            (PairStage::worm(), PairKind::Worm),
            (PairStage::worm().with_first_helix(45.0), PairKind::Spur),
        ] {
            let shape = Shape::from_pair(&pair, kind);
            assert!(shape.as_crossed_pair().is_some());
            let r = solve_shape(
                &shape,
                &loads(),
                &test_library(),
                super::super::Reversal::default(),
            )
            .unwrap();
            assert!(r.meshes[0].point.is_some());
            assert_eq!(r.members.len(), 2);
            assert!(r.ratio < 0.0, "an external pair reverses: {}", r.ratio);
        }
    }

    /// **Every arrangement of a set solves through the shape** and reports a
    /// ratio the graph gives, an efficiency below one both ways, and a play
    /// at whichever shaft is the output.
    /// A set as the closure's laws ask it: the shifts as typed, no
    /// undercut floor, zero backlash, the planet closing it.
    fn closure_set(sun: u32, planet: u32, ring: u32) -> Shape {
        let mut set = PlanetaryStage {
            clearance: Auto::fixed(0.0),
            ..PlanetaryStage::default()
        };
        set.sun.teeth = sun;
        set.planet.teeth = planet;
        set.ring.teeth = ring;
        set.sun.profile_shift = Auto::fixed(0.0);
        set.ring.profile_shift = Auto::fixed(0.0);
        for g in [&mut set.sun, &mut set.planet, &mut set.ring] {
            g.no_undercut = false;
        }
        Shape::from(&set)
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
        let pair = Shape::from_pair(&PairStage::default(), PairKind::Spur);
        let r = solve_shape(&pair, &loads(), &lib, super::super::Reversal::default()).unwrap();
        assert!((r.ratio_per_tooth[1] + 44.0 / 17.0).abs() < 1e-12);
        assert!((r.ratio_per_tooth[0] + 43.0 / 18.0).abs() < 1e-12);
        assert!(
            (r.circulation.forward - 1.0).abs() < 1e-12,
            "a pair passes it all once"
        );
        let set = PlanetaryStage::default();
        let r = solve_set(&set, &loads(), &lib).unwrap();
        assert!((r.ratio - 7.0).abs() < 1e-12);
        assert!(
            (r.ratio_per_tooth[0] - (1.0 + 72.0 / 13.0)).abs() < 1e-12,
            "a sun's tooth"
        );
        assert!(
            (r.ratio_per_tooth[1] - 7.0).abs() < 1e-12,
            "a planet's tooth moves nothing"
        );
        assert!(
            (r.ratio_per_tooth[2] - (1.0 + 73.0 / 12.0)).abs() < 1e-12,
            "a ring's tooth"
        );
        // ...and the power through a set's sun mesh is under the power in,
        // the carrier carrying the rest bodily: with the ring held, the
        // fraction the sun turns against the carrier — and the same power,
        // less that mesh's loss, crosses the ring mesh after it.
        let (sun_mesh, ring_mesh) = (&r.meshes[0], &r.meshes[1]);
        assert!((sun_mesh.power_through.forward - 6.0 / 7.0).abs() < 1e-9);
        assert!(
            (ring_mesh.power_through.forward - 6.0 / 7.0 * sun_mesh.efficiency.forward).abs()
                < 1e-9
        );
        assert!(
            (r.circulation.forward
                - (sun_mesh.power_through.forward + ring_mesh.power_through.forward))
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn every_arrangement_of_a_set_solves() {
        use crate::planetary::{Arrangement, PlanetaryShaft};
        let set = PlanetaryStage::default();
        let mut checked = 0;
        for input in PlanetaryShaft::ALL {
            for fixed in PlanetaryShaft::ALL {
                if input == fixed {
                    continue;
                }
                let arrangement = Arrangement { input, fixed };
                let boundary = PlanetaryStage::boundary_for(arrangement);
                let r = solve_set(&set, &loads().under(boundary), &test_library()).unwrap();
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
                    (r.ratio - want.ratio).abs() < 1e-12,
                    "{arrangement:?}: {} vs {}",
                    r.ratio,
                    want.ratio
                );
                assert!(
                    r.efficiency.forward > 0.9 && r.efficiency.forward < 1.0,
                    "{arrangement:?}"
                );
                assert!(
                    r.efficiency.backward > 0.9 && r.efficiency.backward < 1.0,
                    "{arrangement:?}"
                );
                assert!(r.backlash.forward.nominal > 0.0);
                checked += 1;
            }
        }
        assert_eq!(checked, 6);
    }

    // ---- the set's laws, ported from its kind ----

    /// **A probe width leaves no trace.**
    ///
    /// An epicyclic set rates once at `PROBE` and scales to the width each mesh
    /// carries — bending inversely with the width, contact with its square root
    /// (`Loading::at_width`). So the stress it reports must be the stress a
    /// direct evaluation at that width gives, and this asks for one.
    ///
    /// **Nothing asked before, and the reason is a coincidence of two
    /// constants**: `PROBE` is 10.0 and `StageGear`'s default face width is
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
            let stage = PlanetaryStage {
                sun: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().sun
                },
                planet: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().planet
                },
                ring: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().ring
                },
                ..PlanetaryStage::default()
            };
            let r = solve_set(&stage, &StageLoads::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("face {face}: {e}"));
            let shape = Shape::from(&stage);
            let b = shape
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
                &lib.get(&stage.sun.material).expect("a material").clone(),
                &lib.get(&stage.planet.material).expect("a material").clone(),
            );
            let direct = contact_stress(
                &b.meshes[0].path,
                &b.meshes[0].operating,
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
            let stage = PlanetaryStage {
                ring: StageGear {
                    teeth: 60,
                    profile_shift: Auto::fixed(0.0),
                    face_width: Auto::fixed(ring_face),
                    ..StageGear::default()
                },
                ..stage_of(24, 18, 60, 0.0)
            };
            let r = solve_set(&stage, &StageLoads::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("ring face {ring_face}: {e}"));
            let got = r.members[1].cases[0]
                .bending_stress
                .expect("a planet has a root section");

            // The two contributions, rebuilt from what the result reports rather
            // than from the expression that produced them: each mesh's own
            // section under its own load, over the width that mesh carries.
            let shape = Shape::from(&stage);
            let built = shape
                .build_at(
                    &r.members
                        .iter()
                        .map(|m| m.profile_shift)
                        .collect::<Vec<_>>(),
                )
                .unwrap();
            let planets = f64::from(stage.planets);
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
            // The shafts: ground, sun, carrier, ring, planet. The sun's torque
            // per planet path read across the sun mesh presses the planet;
            // the ring's, per path, is `η` less than the planet pressed it
            // with, read back across the ring mesh.
            let from_sun = each(
                built.meshes[0].path.contact_ratio,
                Load::new((r.cases[0].torques[1] / planets).abs(), sp_width)
                    .across_mesh(built.members[0].as_gear(), planet)
                    .torque,
                sp_width,
            );
            let from_ring = each(
                built.meshes[1].path.contact_ratio,
                (r.cases[0].torques[3] / planets).abs()
                    / built.meshes[1].operating.ratio()
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
            let stage = PlanetaryStage {
                thickness_mod: k,
                ..Default::default()
            };
            let r = solve_set(&stage, &StageLoads::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("k={k}: the set should still solve, got {e}"));

            assert!(
                r.members[2].cases[0].bending_stress.is_none(),
                "k={k}: a ring with no notch cannot have a bending stress"
            );
            // ...and everything that never needed the notch is still there.
            assert!(r.ratio.is_finite() && r.ratio != 0.0, "k={k}: no ratio");
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

    fn stage_of(sun: u32, planet: u32, ring: u32, helix: f64) -> PlanetaryStage {
        PlanetaryStage {
            sun: StageGear {
                teeth: sun,
                helix_angle: Auto::fixed(helix),
                ..StageGear::default()
            },
            planet: StageGear {
                teeth: planet,
                ..StageGear::default()
            },
            ring: StageGear {
                teeth: ring,
                profile_shift: Auto::fixed(0.0),
                ..StageGear::default()
            },
            ..PlanetaryStage::default()
        }
    }

    fn solved(sun: u32, planet: u32, ring: u32) -> ShapeResult {
        solve_set(
            &stage_of(sun, planet, ring, 0.0),
            &StageLoads::at(2.0, 3000.0),
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
    /// `Stage::freedoms` says so by reading the distance's toggle, and this is
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
        let base = PlanetaryStage::default();
        let free = solve_set(&base, &StageLoads::just(2.0), &lib).expect("the shipped set solves");
        let asked = free.distances[0].running + 0.1;

        // One shift given — the ring's, as the shipped set has it — and every
        // given number stands.
        let mut one = base.clone();
        one.centre_distance = Auto::fixed(asked);
        let r = solve_set(&one, &StageLoads::just(2.0), &lib).expect("one free shift is enough");
        assert!((r.distances[0].running - asked).abs() < 1e-9);
        assert!((r.members[2].profile_shift - one.ring.profile_shift.manual).abs() < 1e-12);

        // A second shift given, and it cannot also stand: three relations'
        // worth of demands on two freedoms. Every input is honoured as
        // typed — a given shift is not the solve's to move, and the set's
        // kind used to move one silently — and the mesh whose sum nothing
        // was left to reach says it did not reach the distance: the planet
        // reaches the sun mesh's, and the ring mesh is left with the ring
        // and the planet both decided.
        let mut two = one.clone();
        two.sun.profile_shift = Auto::fixed(r.members[0].profile_shift + 0.25);
        let over = solve_set(&two, &StageLoads::just(2.0), &lib)
            .expect("it still builds; it just cannot honour everything");
        assert!((over.distances[0].running - asked).abs() < 1e-9);
        assert!((over.members[0].profile_shift - two.sun.profile_shift.manual).abs() < 1e-9);
        assert!((over.members[2].profile_shift - two.ring.profile_shift.manual).abs() < 1e-9);
        assert!(
            over.notes
                .iter()
                .any(|n| n.is(key::STAGE_CENTRE_DISTANCE_NOT_REACHED)),
            "the mesh whose sum nothing reached says so: {:?}",
            over.notes
        );
        let ring_mesh_wants = MeshKind::Internal.nominal_of(asked, two.clearance.manual);
        assert!(
            (over.distances[0].nominal[1] - ring_mesh_wants).abs() > 1e-3,
            "and the ring mesh does not run at the clearance asked: {} vs {ring_mesh_wants}",
            over.distances[0].nominal[1]
        );

        // ...and the declaration says the same thing: one relation over the
        // distance, the clearance, the three shifts and the size, with two
        // meshes on the distance, so four of six may be given — the distance,
        // the clearance, the size and **one** shift.
        use super::super::{Freedom, Stage};
        let relation = Stage::planetary(one.clone())
            .freedoms()
            .into_iter()
            .find(|g| g.order.len() == 6)
            .expect("a set declares one relation over its distance");
        assert_eq!(relation.given_at_most, 4);
        assert_eq!(relation.order[0], vec![Freedom::CentreDistance]);
        assert_eq!(relation.order[4], vec![Freedom::Clearance]);
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
        let base = PlanetaryStage::default();
        let free = solve_set(&base, &StageLoads::just(2.0), &lib).expect("the shipped set solves");

        let mut checked = 0u32;
        for step in -2..=4 {
            let asked = free.distances[0].running + 0.2 * f64::from(step);
            let mut stage = base.clone();
            stage.centre_distance = Auto::fixed(asked);
            let Ok(r) = solve_set(&stage, &StageLoads::just(2.0), &lib) else {
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
                (r.distances[0].clearance - stage.clearance.manual).abs() < 1e-9,
                "the clearance asked for should be the clearance left: {}",
                r.distances[0].clearance
            );

            // Both meshes at that distance, measured from their own
            // zero-backlash distances rather than from the expression that
            // placed the shifts: opened by the clearance each its own way,
            // they must land on one running distance.
            let residual = (r.distances[0].nominal[0] + stage.clearance.manual
                - (r.distances[0].nominal[1] - stage.clearance.manual))
                .abs();
            assert!(
                residual < 1e-9,
                "the two meshes disagree by {residual} at a given distance"
            );

            // The ring's shift is given on the shipped set, so it is the one
            // freedom a target leaves and must come back untouched.
            assert!(
                (r.members[2].profile_shift - stage.ring.profile_shift.manual).abs() < 1e-12,
                "a given shift moved: {} for {}",
                r.members[2].profile_shift,
                stage.ring.profile_shift.manual
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
        exact.clearance = Auto::fixed(0.0);
        let exact = solve_set(&exact, &StageLoads::just(2.0), &lib).unwrap();
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
        let base = Shape::from(&PlanetaryStage::default());
        // The shifts the default set settles at, so each variant below asks for
        // values a set of these counts can actually be built at.
        let settled = base.shifts_at(&crate::auto::Search::SHIPPED);

        for absorber in 0..3 {
            let mut s = PlanetaryStage::default();
            // Pin every member but the one meant to absorb.
            for (i, gear) in [&mut s.sun, &mut s.planet, &mut s.ring]
                .into_iter()
                .enumerate()
            {
                gear.profile_shift = if i == absorber {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(settled[i])
                };
            }
            let shape = Shape::from(&s);
            let plan = shape.plan(&shape.helix_angles());
            assert_eq!(
                plan.role[absorber],
                Role::Absorbs(0),
                "the member left automatic should be the one that absorbs"
            );
            let r = solve_set(&s, &StageLoads::just(2.0), &lib)
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
        let role_of = |s: &PlanetaryStage| {
            let shape = Shape::from(s);
            shape.plan(&shape.helix_angles()).role
        };
        assert_eq!(role_of(&PlanetaryStage::default())[1], Role::Absorbs(0));
        let mut s = PlanetaryStage::default();
        s.planet.profile_shift = Auto::fixed(0.0);
        assert_eq!(
            role_of(&s)[0],
            Role::Absorbs(0),
            "pinning the planet hands it to the sun"
        );
        s.sun.profile_shift = Auto::fixed(0.0);
        // The shipped ring's shift is given, so with the other two pinned as
        // well nothing is left automatic and the set is over-specified: the
        // panel relieves it as it is created, and a document that reaches
        // this state is refused with the distances' own reason.
        assert!(role_of(&s).iter().all(|r| *r == Role::Given));
        assert_eq!(
            solve_set(&s, &StageLoads::just(2.0), &test_library()).err(),
            Some(TrainError::NoCommonDistance)
        );
        s.ring.profile_shift = Auto::automatic(0.0);
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
            let asked = PlanetaryStage::boundary_for(Arrangement { input, fixed });
            let r =
                solve_set(&stage, &StageLoads::just(2.0).under(asked), &test_library()).unwrap();
            let _ = output;
            assert!(
                (r.ratio - ratio).abs() < 1e-12,
                "{input:?}/{fixed:?}: {}",
                r.ratio
            );
        }
    }

    /// **A held carrier makes the set two meshes in series**, so its efficiency
    /// must be exactly the product of theirs — through the stage, not just the
    /// algebra.
    #[test]
    fn a_held_carrier_gives_exactly_the_product_of_the_mesh_efficiencies() {
        let stage = stage_of(24, 18, 60, 0.0);
        let carrier_held = PlanetaryStage::boundary_for(Arrangement {
            input: PlanetaryShaft::Sun,
            fixed: PlanetaryShaft::Carrier,
        });
        let r = solve_set(
            &stage,
            &StageLoads::just(2.0).under(carrier_held),
            &test_library(),
        )
        .unwrap();
        let product = r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward;
        assert!(
            (r.efficiency.forward - product).abs() < 1e-12,
            "{}",
            r.efficiency.forward
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
    /// The same play measured at two different output shafts must differ by
    /// exactly the ratio between them — and those ratios come from
    /// `planetary::power`, which shares none of the referral's algebra. That is
    /// what makes this a check rather than a restatement.
    ///
    /// It is also the law the train-level test uses on a multi-stage train
    /// ("backlash at the two ends differs by exactly the total ratio"), asked of
    /// one stage with three shafts instead of a line of two-shaft ones.
    #[test]
    fn backlash_referred_to_two_shafts_differs_by_exactly_their_ratio() {
        let lib = test_library();
        for (s, p, r) in [(24u32, 18u32, 60u32), (17, 17, 52), (30, 15, 62)] {
            // Ring held: the sun and the carrier are the two possible outputs.
            let stage = stage_of(s, p, r, 0.0);
            let asked = |input| {
                StageLoads::just(2.0).under(PlanetaryStage::boundary_for(Arrangement {
                    input,
                    fixed: PlanetaryShaft::Ring,
                }))
            };
            let a = solve_set(&stage, &asked(PlanetaryShaft::Sun), &lib).unwrap();
            let b = solve_set(&stage, &asked(PlanetaryShaft::Carrier), &lib).unwrap();

            // `a` outputs at the carrier, `b` at the sun.
            let at_carrier = a.backlash.forward.nominal;
            let at_sun = b.backlash.forward.nominal;
            assert!(at_carrier > 0.0 && at_sun > 0.0);
            assert!(
                (at_sun - at_carrier * a.ratio).abs() < 1e-9 * at_sun,
                "z={s}/{p}/{r}: {at_sun} vs {at_carrier} x {}",
                a.ratio
            );
            // ...and the shaft that turns faster carries the looser play.
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
        let tight = solve_set(&base, &StageLoads::just(2.0), &lib).unwrap();

        // More clearance opens both meshes, so the output must loosen.
        let loose = PlanetaryStage {
            clearance: Auto::fixed(base.clearance.manual + 0.05),
            ..base.clone()
        };
        let loose = solve_set(&loose, &StageLoads::just(2.0), &lib).unwrap();
        assert!(
            loose.backlash.forward.nominal > tight.backlash.forward.nominal,
            "{} should exceed {}",
            loose.backlash.forward.nominal,
            tight.backlash.forward.nominal
        );

        // And the tolerance band holds the nominal. On the ideal ring it is a
        // point — the referred play is invariant in the running distance, the
        // sun mesh gaining exactly what the ring mesh loses (the law is in
        // `train::tests::a_tolerance_band_widens_with_the_centre_distance`) —
        // so a set one tooth off the ideal is what shows the band opening, and
        // it opens on both sides of the nominal since the two meshes' operating
        // angles no longer move together.
        let b = &tight.backlash.forward;
        assert!(b.minimum <= b.nominal && b.nominal <= b.maximum);
        let off = stage_of(24, 18, 61, 0.0);
        let off = solve_set(&off, &StageLoads::just(2.0), &lib).unwrap();
        let b = &off.backlash.forward;
        assert!(
            b.minimum < b.nominal && b.nominal < b.maximum,
            "off the ideal ring the band opens: {} … {} … {}",
            b.minimum,
            b.nominal,
            b.maximum
        );

        // At the zero-backlash centre distance there is no play at all.
        let exact = PlanetaryStage {
            clearance: Auto::fixed(0.0),
            tolerance_plus: 0.0,
            tolerance_minus: 0.0,
            ..base
        };
        let exact = solve_set(&exact, &StageLoads::just(2.0), &lib).unwrap();
        assert!(
            exact.backlash.forward.nominal < 1e-12,
            "zero clearance must give zero play, got {}",
            exact.backlash.forward.nominal
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
        let stage = PlanetaryStage::default();
        // `StageLoads::just` reverses nothing; the reversing duty is the
        // fatigue case's own, so the second solve hands the stage one.
        let solve = |reversal: crate::train::Reversal, reversing: bool| {
            let mut loads = StageLoads::just(2.0);
            loads.cases[1].turns = Some(crate::train::Turns {
                revolutions: 1.0,
                reversing_actuations: reversing.then_some(1.0),
            });
            solve_shape(&Shape::from(&stage), &loads, &lib, reversal).unwrap()
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
        let one = PlanetaryStage {
            planets: 1,
            ..stage_of(24, 18, 60, 0.0)
        };
        let r = solve_set(&one, &StageLoads::just(2.0), &test_library()).unwrap();
        assert!(r.layouts.is_empty(), "one planet has no layout to check");
    }

    /// **Helical works, to parity with spur.** Every figure a spur set reports,
    /// a helical one reports too — including the ring's bending, which goes
    /// through the virtual spur ring.
    #[test]
    fn a_helical_set_reports_everything_a_spur_one_does() {
        for helix in [10.0, 20.0, 30.0] {
            let stage = stage_of(24, 18, 60, helix);
            let r = solve_set(&stage, &StageLoads::just(2.0), &test_library())
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
        assert!(solve_set(
            &stage_of(24, 18, 200, 0.0),
            &StageLoads::just(2.0),
            &test_library()
        )
        .is_err());
    }

    /// The thickness invariants differ between the two meshes and both hold from
    /// one stored `k`: the external pair sums to two, the internal pair matches.
    #[test]
    fn one_thickness_modification_satisfies_both_invariants() {
        for k in [0.9, 1.0, 1.15] {
            let stage = PlanetaryStage {
                thickness_mod: k,
                ..stage_of(24, 18, 60, 0.0)
            };
            let shape = Shape::from(&stage);
            let (sun, planet, ring) = (
                shape.members[0].thickness_mod,
                shape.members[1].thickness_mod,
                shape.members[2].thickness_mod,
            );
            assert!(
                (sun + planet - 2.0).abs() < 1e-15,
                "external pair must sum to two"
            );
            assert!((planet - ring).abs() < 1e-15, "internal pair must match");
            // ...and it still solves.
            assert!(solve_set(&stage, &StageLoads::just(2.0), &test_library()).is_ok());
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
            s.sun.profile_shift = Auto::automatic(0.0);
            s.ring.profile_shift = Auto::automatic(0.0);
            s
        };
        let solve = |on: bool| {
            solve_set(
                &PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    ..free()
                },
                &StageLoads::just(2.0),
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
            tuned.efficiency.forward > plain.efficiency.forward,
            "the set efficiency {:.6} should beat {:.6}, or power is not monotone in eta0",
            tuned.efficiency.forward,
            plain.efficiency.forward
        );
        // Both meshes stay continuous by at least the margin asked for.
        for eps in [
            tuned.meshes[0].line.unwrap().contact_ratios.transverse,
            tuned.meshes[1].line.unwrap().contact_ratios.transverse,
        ] {
            let asked = free().optimisation.min_contact_ratio;
            assert!(eps >= asked - 1e-3, "contact ratio {eps} under {asked}");
        }
    }

    /// A shift given by hand is a constraint the search may not overrule.
    #[test]
    fn a_given_shift_survives_the_search() {
        let mut stage = PlanetaryStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..stage_of(24, 18, 60, 0.0)
        };
        stage.sun.profile_shift = Auto::automatic(0.0);
        stage.ring.profile_shift = Auto::fixed(0.25);
        let r = solve_set(&stage, &StageLoads::just(2.0), &test_library()).expect("solves");
        assert!((r.members[2].profile_shift - 0.25).abs() < 1e-9);
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod hula_recorded {
    //! **The hula stage through the shape is the hula stage**, held to the
    //! figures the kind recorded before it retired. The gate that ran the
    //! two side by side lived at `2b71654` and found them the same to 1e-6
    //! — ratio, crank offset and the mesh that held it open, every shift
    //! and width, every mesh's figures, the stage's efficiency both ways and
    //! its backlash at both shafts — apart from the two differences the
    //! set's retirement had already recorded (`docs/corrections.md`): a
    //! driven member pressed with its driver's force, and a case from the
    //! output entered at the output rather than read as the crank's
    //! delivered torque. What is held here is what the corpus printed for
    //! `gear-cli hula 18 0.2` and what `docs/reference.md#the-hula-stage`
    //! quotes for the shipped stage, to the digits they print.

    use super::super::{test_library, HulaStage, Reversal, StageLoads};
    use super::*;

    fn hula_18(clearance: f64) -> HulaStage {
        let mut stage = HulaStage {
            clearance,
            ..HulaStage::default()
        };
        for (g, z) in stage.gears.iter_mut().zip([19, 18, 17, 18]) {
            g.teeth = z;
        }
        for (m, c) in stage.cutter.iter_mut().enumerate() {
            c.teeth = [14, 13][m];
        }
        stage
    }

    /// The hula's own boundary: crank driven, grounded gear held, output
    /// out — shafts 1, 3 and 2 of the shape.
    fn solve(stage: &HulaStage, loads: StageLoads) -> ShapeResult {
        let boundary = super::super::StageBoundary::holding(5, &[3], 1, 2);
        solve_shape(
            &Shape::from(stage),
            &loads.under(boundary),
            &test_library(),
            Reversal::default(),
        )
        .unwrap()
    }

    fn close(a: f64, b: f64, tol: f64, what: &str) {
        assert!((a - b).abs() <= tol, "{what}: recorded {a}, shape {b}");
    }

    #[test]
    fn the_harness_hula_is_what_the_corpus_recorded() {
        let r = solve(&hula_18(0.2), StageLoads::at(2.0, 1000.0));
        close(324.0, r.ratio, 1e-9, "ratio");
        let d = &r.distances[0];
        close(0.726_026, d.running, 1e-6, "crank offset, running");
        close(
            0.746_026,
            d.nominal[0],
            1e-6,
            "crank offset at zero backlash",
        );
        assert_eq!(d.sized_by, Some(0), "held open by mesh 1");
        // The rings at +0.4519, the pinions at their floor.
        for (i, want) in [(0, 0.4519), (1, 0.0), (2, 0.0), (3, 0.4519)] {
            close(
                want,
                r.members[i].profile_shift,
                5e-5,
                &format!("gear {i} shift"),
            );
        }
        close(
            31.310,
            r.efficiency.forward * 100.0,
            5e-4,
            "forward efficiency, %",
        );
        close(0.0, r.efficiency.backward, 1e-12, "self-locking");
        close(
            0.9932,
            r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward,
            5e-5,
            "the two meshes alone",
        );
        close(
            0.405_565,
            r.backlash.forward.nominal,
            1e-6,
            "backlash at the output",
        );
        close(
            131.4032,
            r.backlash.backward.nominal,
            5e-5,
            "backlash at the crank",
        );
        // The operating angle is the **running** mesh's, 0.02 mm inside the
        // zero-backlash offset the kind quoted its 50.965° at — the stated
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
        // change every driven member took with the shape. The kind
        // anchored each mesh at the torque the power flow put on its
        // central member: the output's, which drives its wobble gear in the
        // crank's frame and so is the flank force itself; and the grounded
        // ring's, which is *driven* by its wobble gear and stood `η₁` under
        // the force on its flank — so the whole of mesh 1 is `1/η₁` over
        // what the kind printed, bending with it, and mesh 2 is the kind's.
        let eta1 = r.meshes[0].efficiency.forward;
        close(
            200.8899 / eta1,
            r.members[0].cases[0].torque,
            5e-4,
            "z19 torque over η₁",
        );
        close(
            190.3167 / eta1,
            r.members[1].cases[0].torque,
            5e-4,
            "z18 torque over η₁",
        );
        close(191.6182, r.members[2].cases[0].torque, 5e-4, "z17 torque");
        close(
            202.8899,
            r.members[3].cases[0].torque,
            5e-4,
            "output z18 torque",
        );
        close(
            6663.3 / eta1,
            r.members[1].cases[0].bending_stress.unwrap(),
            0.05,
            "z18 σ_F",
        );
        close(
            7171.7,
            r.members[2].cases[0].bending_stress.unwrap(),
            0.05,
            "z17 σ_F",
        );
        for i in [0, 3] {
            assert!(
                r.members[i].cases[0].bending_stress.is_none(),
                "no fillet, no rating"
            );
        }
    }

    #[test]
    fn the_shipped_hula_stage_reports_the_figures_the_documents_quote() {
        let r = solve(&HulaStage::default(), StageLoads::at(2.0, 1000.0));
        close(3721.0 / 16.0, r.ratio, 1e-9, "the reduction");
        close(81.92, r.efficiency.forward * 100.0, 0.005, "forward, %");
        close(77.91, r.efficiency.backward * 100.0, 0.005, "backward, %");
    }

    /// A case from the output enters at the output at the torque times the
    /// ratio, and the crank delivers that over `η_backward`.
    #[test]
    fn a_case_from_the_output_is_entered_at_the_output() {
        let mut loads = StageLoads::at(2.0, 3000.0);
        loads.cases.push(super::super::StageLoad {
            case: 2,
            kind: super::super::CaseKind::Ultimate,
            drive: crate::contact::Drive::Backward,
            torque: 0.5,
            speed: 0.0,
            turns: None,
        });
        let r = solve(&HulaStage::default(), loads);
        let c = &r.cases[2];
        close(
            0.5 * r.ratio,
            c.torques[2].abs(),
            1e-9,
            "the output carries the case",
        );
        close(
            0.5 * r.efficiency.backward,
            c.torques[1].abs(),
            1e-9,
            "the crank delivers it over η_b",
        );
    }
}
