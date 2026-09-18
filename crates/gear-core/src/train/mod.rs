//! Geartrains: a stage at a time, and the accumulation along the shaft line.
//!
//! Each stage shape has its own module — `pair`, `planetary`, `hula` — and what
//! stays here is the vocabulary they share ([`Backlash`], [`TrainError`], the
//! duty cycle) and the train that strings them together.
//!
//! **A kind is a layer, not a model.** The spur, helical, crossed and worm
//! stages are one [`PairStage`] under two names — [`Stage::Spur`] and
//! [`Stage::Worm`] — and produce one [`PairResult`]. And a line contact and a
//! point contact are one [`MeshReport`]: the physics is one model with the
//! shaft angle as a parameter — one Hertz answer, one friction balance, one
//! backlash projection, one interference relation, each holding at the limit
//! — so the report is one shape with the numbers moving, and what only one of
//! the two has is the little in [`LineContact`] and [`PointContact`].
//!
//! # What is state and what is not
//!
//! Per `docs/rationale.md#inputs-are-the-only-state` the input structs here are the *only* state. Every
//! result is recomputed from them, so nothing can go stale. Two consequences are
//! visible in the shapes below:
//!
//! - Values shared across a stage — normal module, pressure angle, helix angle —
//!   are stored **once on the stage**, not per gear, so the two cannot disagree
//!   (docs/rationale.md#inputs-are-the-only-state).
//! - Tooth thickness modification is stored as `k₁` alone, with `k₂ = 2 − k₁`
//!   derived, because a meshing pair must sum to 2. The invariant is unwritable
//!   rather than merely tested.

use crate::auto::{addendum_for_tip_width, automatic_profile_shift, Ranges};
use crate::contact::{Directional, Drive};
use crate::material::{Material, MaterialLibrary, Overrides};
use crate::mesh::MeshError;
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::tooth::Tooth;

mod conditions;
pub mod crossed;
mod hula;
mod pair;
mod planetary;
mod wiring;

pub use conditions::{
    Constraint, Coupling, Exact, MotionError, MotionReport, OpenPort, PortSpec, Ports, Route,
    RouteError, ShaftConstraint, ShaftMotion, ShaftRef, ShaftReport, StageBoundary, StagePorts,
    Term, TrainMotion,
};
pub use crossed::solve_crossed_pair;
pub use hula::{
    solve_hula_stage, solve_hula_stage_with, stage_efficiency, HulaGear, HulaMesh, HulaResult,
    HulaStage,
};
pub use pair::{solve_pair_stage, solve_pair_stage_with, PairKind, PairStage};
pub(crate) use pair::{undercut_bound, Decided, ShiftAsked};
pub use planetary::{
    solve_planetary_stage, solve_planetary_stage_with, PlanetResult, PlanetaryResult,
    PlanetaryStage,
};
pub(crate) use wiring::teeth_of;
pub use wiring::{
    MemberMotion, MeshSpec, Mount, Offsets, ShaftLabel, UnitMotion, Wiring, WiringError,
};

/// The three contact ratios.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ContactRatios {
    /// Transverse, `ε_α` — profile overlap.
    pub transverse: f64,
    /// Overlap, `ε_β = b sin β / (π m_n)` — axial overlap. Exactly zero for a
    /// spur stage.
    pub overlap: f64,
    /// Total, `ε_γ = ε_α + ε_β`.
    pub total: f64,
}

impl ContactRatios {
    /// The three, from the transverse one and the mesh they belong to.
    ///
    /// `ε_β = b sin β / (π m_n)` and `ε_γ = ε_α + ε_β` — one line each, but
    /// written out once per stage kind they were three copies of the same two
    /// lines, and a fourth stage away from being four. The width is the mesh's
    /// **effective** one, the narrower of the two members, because that is the
    /// width that carries the pair.
    #[must_use]
    pub fn of(transverse: f64, width: f64, helix_angle: f64, normal_module: f64) -> Self {
        let overlap =
            width * helix_angle.to_radians().sin().abs() / (std::f64::consts::PI * normal_module);
        Self {
            transverse,
            overlap,
            total: transverse + overlap,
        }
    }

    /// Whether at least one contact line is engaged at all times.
    ///
    /// Below this a gear is helical in form but still transfers load like a spur
    /// gear — abrupt engagement, no smoothing — which is usually not what the
    /// helix angle was chosen for, and is invisible without the check.
    #[must_use]
    pub fn has_full_axial_overlap(&self) -> bool {
        self.overlap >= 1.0
    }
}

/// Angular backlash at one gear, in degrees.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Backlash {
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
}

impl Backlash {
    /// The band a centre-distance tolerance opens around a nominal distance.
    ///
    /// **One construction, and which end is which is read off the numbers.**
    /// A larger centre distance is more play on an external pair and less on an
    /// internal one, whose flanks part as the centres come together — so the
    /// tolerance's `minus` end is the minimum on one kind and the maximum on
    /// the other. This used to assign `minus` to the minimum outright, which
    /// reads perfectly well and is inside out on every internal mesh.
    ///
    /// A set that carries one of each, referred to one shaft, is neither: what
    /// the sun mesh gains from a planet moved outward the ring mesh loses, so
    /// on the ideal ring the referred play is stationary at the running
    /// distance and both ends of the band sit *below* the nominal. The nominal
    /// is therefore a candidate for either end, and the band is the extremes
    /// of the three rather than of the two.
    ///
    /// It was written out four times, once per stage kind, each closing over its
    /// own way of turning a distance into an angle. That is the part that
    /// genuinely differs — a parallel mesh, a screw pair and a crank each reach
    /// it differently — so it is the argument, and the three lines around it are
    /// not.
    pub fn banded(nominal: f64, minus: f64, plus: f64, angular: impl Fn(f64) -> f64) -> Self {
        let (lo, mid, hi) = (
            angular(nominal - minus),
            angular(nominal),
            angular(nominal + plus),
        );
        Self {
            nominal: mid,
            minimum: lo.min(mid).min(hi),
            maximum: lo.max(mid).max(hi),
        }
    }
}

/// **What one mesh reports**, whatever stage it is in and whichever way its
/// shafts run.
///
/// One type, because the physics is one model with the shaft angle as a
/// parameter and not two models: the Hertz answer is general contact of which a
/// line is the degenerate value ([`crate::hertz::peak_pressure`]), the
/// efficiency is one friction balance the parallel loss integral is the limit
/// of, the backlash is one gap projected onto one normal, and the interference
/// verdict is one relation asked in the transverse plane or along the line —
/// each of which a test holds at the limit. So a designer turning a shaft
/// angle from zero sees the same rows with the numbers moving, not a readout
/// changing shape. What a line contact has that a point does not, and the other
/// way, is the little in [`LineContact`] and [`PointContact`]: the transverse
/// decomposition of the contact ratio and the operating angle on one side; the
/// face-limited zone and the parallel counterpart on the other. Bending is
/// the members' and stays on [`GearResult`], `None` on a point contact for the
/// reason `docs/rationale.md#a-worm-stage-reports-no-bending-stress` gives.
///
/// A set's `sun_planet`/`planet_ring` and a hula stage's two pairs are the
/// same report, which is why it is one type; a pair's one mesh is on its own
/// result. This used to be the parallel-axis report only, with a crossed pair's
/// in a type of its own beside it and an enum choosing between them.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MeshReport {
    /// Whether the two members' tooth counts share no factor — a hunting pair,
    /// which spreads wear evenly instead of repeatedly bringing the same two
    /// teeth together.
    ///
    /// A property of a *mesh*, which is why it is here: a pair reports one
    /// (its one mesh being the stage), and a set with two meshes has two
    /// answers. An epicyclic set's separate question — each central member
    /// against the *planet count* — is a different check with a different
    /// reason, and it stays where it is.
    pub coprime: bool,
    /// **Tooth pairs sharing the load.** A line contact's total, `ε_α + ε_β`;
    /// a point contact's along its line of action, the zone over the normal
    /// base pitch, as the face widths in use leave it. Below 1 the mesh loses
    /// contact between one pair and the next, on either.
    ///
    /// The two are the same *count* and not the same measure, which is the
    /// one place the two contacts do not meet continuously: a line counts
    /// lines across the face, a point counts points along one line, and the
    /// point's limit as the shafts straighten is the normal-plane `ε_α / cos²β_b`
    /// rather than the total. The decomposition each has is in [`Self::line`]
    /// and [`Self::point`].
    pub contact_ratio: f64,
    /// Mesh efficiency, both drive senses. Equal for a parallel-axis mesh — the
    /// mirror flank is the same integral — and genuinely different on crossed
    /// shafts, where **either** can be zero or negative: backward is what
    /// self-locking is, forward a steep helix split that cannot drive at all.
    /// [`Directional::locked`] reads them rather than a separate flag that
    /// could disagree.
    pub efficiency: Directional<f64>,
    /// The coefficient of friction at which each direction stops driving,
    /// read along the path; a **negative** value means no friction locks the
    /// pair that way — a value rather than a missing one.
    ///
    /// Negative both ways on every line contact, and deliberately not the
    /// number its loss model extrapolates to. That model is first order in
    /// `μ` ([`crate::contact::efficiency`]), so it reaches zero at
    /// `μ / (1 − η)` — about 5 on the shipped pair — which is nowhere the
    /// model describes: the friction balance a hundredth of a degree off
    /// parallel puts the threshold at half that, and asymmetric. What is true
    /// within the model is that no friction near unity locks a parallel mesh,
    /// and *never* is the honest reading of that.
    pub locking_friction: Directional<f64>,
    /// Sliding speed at the pitch point as a multiple of the first member's
    /// pitch line speed — the lengthwise sliding crossed shafts have, and
    /// **exactly zero** on parallel ones, where the pitch point is the one
    /// place with no sliding at all and every loss is along the profile.
    pub sliding_ratio: f64,
    /// **What every load case does to this mesh**: the Hertzian contact — one
    /// patch the two members share, an ellipse on crossed shafts and a line on
    /// parallel ones — and the sliding speed at that case's speed.
    pub cases: Vec<MeshCase>,
    /// Angular backlash at each member, degrees, in the order the mesh was
    /// built: the pinion-side member first, then the other.
    pub backlash: [Backlash; 2],
    /// **Whether each member's flank is reached past its usable end** by the
    /// other member's tip, in the order the mesh was built.
    ///
    /// The classical interference condition, and it belongs to **every** mesh.
    /// An internal pair had it under two names — *involute* interference when
    /// the ring's tip reaches below where the pinion's flank ends, which is
    /// `[0]`, and *trochoid* when the pinion's tip reaches into the ring's
    /// fillet, which is `[1]` — and an external pair had it under none, though a
    /// long addendum on a small pinion is exactly where it bites. A crossed
    /// pair asks it along its line of action
    /// ([`crate::screw::CrossedPath::flank_interference`]).
    pub flank_interference: [bool; 2],
    /// **Where an internal mesh's tips are**, and `None` for an external one,
    /// which has no such question: an external pair's tip circles cross on the
    /// line of centres or not at all.
    pub tips: Option<TipRoom>,
    /// **What this mesh has to say**, on every kind alike: contact that does not
    /// stay continuous, a helical pair without full axial overlap, a sharing
    /// model that is extrapolating, a screw pair that locks or nearly does,
    /// or one that loses more than it keeps. A set has two meshes and says
    /// which, which a note on the stage could not.
    pub notes: Vec<Note>,
    /// What a line contact has and a point does not.
    pub line: Option<LineContact>,
    /// What a point contact has and a line does not.
    pub point: Option<PointContact>,
}

/// What one load case puts on the **shafts** of an epicyclic kind, by
/// [`PlanetaryShaft`](crate::planetary::PlanetaryShaft) index — the set's sun,
/// carrier and ring, or the hula stage's grounded gear, crank and output. The
/// members print their own; this is where the shaft that is not a gear reads.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShaftsCase {
    /// Index into the train's list of load cases.
    pub case: usize,
    /// rpm. The held shaft is exactly zero.
    pub speeds: [f64; 3],
    /// N·m. They sum to zero.
    pub torques: [f64; 3],
}

/// What one load case does to one mesh.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MeshCase {
    /// Index into the train's list of load cases.
    pub case: usize,
    pub contact: ContactPatch,
    /// Sliding speed at the pitch point, mm/s, at this case's speed — exactly
    /// zero on parallel shafts, where [`MeshReport::sliding_ratio`] is.
    pub sliding_velocity: f64,
}

/// What only a line contact reports: the transverse plane's own figures.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LineContact {
    /// Transverse operating pressure angle `α_w`, degrees — the zero-backlash
    /// pair's, which the profile shifts set. A point contact has no
    /// operating angle: its normal cannot turn, so its normal pressure angle
    /// is `α_n` at any shift.
    pub operating_pressure_angle: f64,
    /// The contact ratio decomposed: transverse, overlap and their total,
    /// which is [`MeshReport::contact_ratio`].
    pub contact_ratios: ContactRatios,
}

/// What only a point contact reports: the zone as the faces leave it, and the
/// pair it is compared against.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PointContact {
    /// What ended the zone: the teeth, or the face they are cut on.
    pub limited_by: crate::screw::ZoneLimit,
    /// The face width at which `ε = 1`, per member, mm.
    ///
    /// A **geometric** minimum: it keeps contact continuous and says nothing
    /// about stress. That is the opposite of a line contact's automatic width,
    /// which inverts a stress, and the difference has to travel with the
    /// number.
    pub face_width_for_continuity: Option<[f64; 2]>,
    /// How far the contact point runs along each member's own axis, mm — what a
    /// face has to cover, and what a line contact does not have at all.
    pub axial_travel: [f64; 2],
}

/// **The Hertzian contact a mesh presses** — one answer for an ellipse and a
/// line, since a line is the ellipse with one curvature at zero
/// ([`crate::hertz::peak_pressure`]).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ContactPatch {
    /// Peak Hertzian pressure, MPa — the worst anywhere on the path. On a line
    /// contact that is the envelope over both members' governing points, each
    /// member's own sitting on its [`GearResult`]; on a point contact it is
    /// the worst of the pitch point and the two boundaries of single-pair
    /// contact, which sit 10–27 % above the pitch point and are not what is
    /// rated with `ε > 1` because a second pair shares the load there.
    pub max_pressure: f64,
    /// What the pitch point alone says, MPa — the one figure both members
    /// share, and on a point contact close to the *least* severe the mesh
    /// offers, since the relative radius peaks near it.
    pub at_pitch_point: f64,
    /// Where the worst point sits on the path, mm from the pitch point.
    pub worst_position: f64,
    /// The patch, mm: its length along the contact and its width across.
    ///
    /// On a line contact the length is the face the mesh is rated on and the
    /// width the Hertz half-width doubled, `4 ρ p_max / E*`; on a point
    /// contact the ellipse's two axes, the length bounded by the line the teeth
    /// have. Both are what a stylus would measure.
    pub patch_length: f64,
    pub patch_width: f64,
    /// Relative curvature along the contact, 1/mm. **Zero is line contact** —
    /// the degenerate value, and every parallel mesh's — and that it is not
    /// zero is what crossing the shafts did.
    pub curvature_along: f64,
    /// ...and across it, `1/ρ` — the reciprocal of the relative radius of
    /// curvature at the worst point, in the normal plane.
    pub curvature_across: f64,
}

impl ContactPatch {
    /// A line contact's patch, from the stress along the path.
    ///
    /// `load_scale` is the load case's fraction of the load the stress was
    /// evaluated at, which an epicyclic set applies as its square root rather
    /// than re-running the path; a pair evaluates each case and passes one.
    /// The half-width is [`crate::hertz::line_half_width`], closed form from
    /// what the rating already has.
    pub(crate) fn line(
        cs: &crate::strength::ContactStress,
        load_scale: f64,
        face_width: f64,
        e_star: f64,
    ) -> Self {
        let k = load_scale.sqrt();
        let max_pressure = cs.worst * k;
        let curvature_across = 1.0 / cs.relative_radius;
        Self {
            max_pressure,
            at_pitch_point: cs.at_pitch_point * k,
            worst_position: cs.worst_position,
            patch_length: face_width,
            patch_width: 2.0
                * crate::hertz::line_half_width(curvature_across, max_pressure, e_star),
            curvature_along: crate::strength::PARALLEL_AXES,
            curvature_across,
        }
    }
}

/// **What a line contact's builder has in hand**, gathered so the three kinds
/// that report one fill the report through one function rather than each
/// restating which of its fields a line contact leaves at their degenerate
/// values.
pub(crate) struct LineMesh {
    pub coprime: bool,
    pub contact_ratios: ContactRatios,
    /// Transverse operating pressure angle, degrees — the report's unit.
    pub operating_pressure_angle: f64,
    pub efficiency: Directional<f64>,
    /// One contact per load case, in the loads' order.
    pub contact: Vec<ContactPatch>,
    pub backlash: [Backlash; 2],
    pub flank_interference: [bool; 2],
    pub tips: Option<TipRoom>,
    /// What the builder has to say that this function cannot read off the
    /// ratios — the sharing model's finding, raised where the section is.
    pub notes: Vec<Note>,
}

/// A line contact's [`MeshReport`]: the shared fields as given, the sliding at
/// the pitch point and the locking thresholds at their degenerate values, and
/// the transverse figures in [`LineContact`].
pub(crate) fn line_mesh_report(loads: &StageLoads, m: LineMesh) -> MeshReport {
    // The two findings every line contact's ratios can raise, asked here so
    // that no kind has to remember to — the pair asked both and neither
    // epicyclic kind asked either.
    let mut notes = m.notes;
    let r = &m.contact_ratios;
    if r.transverse < 1.0 {
        notes.push(Note::new(key::MESH_CONTACT_RATIO_BELOW_ONE).number("ratio", r.transverse, 3));
    }
    // Without the figure: it is drawn beside the ratio's own box.
    if r.overlap > 0.0 && !r.has_full_axial_overlap() {
        notes.push(Note::new(key::MESH_OVERLAP_BELOW_ONE));
    }
    MeshReport {
        notes,
        coprime: m.coprime,
        contact_ratio: m.contact_ratios.total,
        // No friction locks a line contact — see the field.
        locking_friction: Directional::of(|_| -1.0),
        efficiency: m.efficiency,
        sliding_ratio: 0.0,
        cases: loads
            .cases
            .iter()
            .zip(m.contact)
            .map(|(l, contact)| MeshCase {
                case: l.case,
                contact,
                sliding_velocity: 0.0,
            })
            .collect(),
        backlash: m.backlash,
        flank_interference: m.flank_interference,
        tips: m.tips,
        line: Some(LineContact {
            operating_pressure_angle: m.operating_pressure_angle,
            contact_ratios: m.contact_ratios,
        }),
        point: None,
    }
}

/// **The three ways an internal mesh's teeth can foul**, and the room the third
/// leaves.
///
/// An external pair has none of them: its two members curve opposite ways, so a
/// tip meets a flank where the teeth mesh and nowhere else, and the one question
/// left is whether the tips bottom out — which is a radial comparison every
/// mesh already makes ([`crate::mesh::Mesh::bottom_clearance`]). An internal
/// pair's members curve the *same* way, and that opens three more:
///
/// - the pinion's tip reaching past where the ring's flank ends, into the fillet
///   its shaper left (**trochoid**);
/// - the ring's tip reaching below where the pinion's flank ends, or not
///   reaching its involute at all (**involute**) — the one a **full-depth**
///   internal pair fails as a matter of course, which is why internal gears are
///   not built full-depth (`crate::ring::mesh_with`);
/// - and the tips fouling **away from the line of action** entirely (**tip**),
///   which the first two cannot see: they ask what happens where the teeth mesh,
///   and this asks whether two teeth try to occupy the same place somewhere
///   else. It is what decides a small tooth difference, where the tip circles
///   cross far from the line of centres and the mesh itself is perfectly
///   conjugate.
///
/// # Why it is on the mesh report
///
/// It was a hula stage's, in four fields of its own, and the epicyclic set with
/// the same internal mesh in it reported nothing — so a designer was told
/// whether the teeth foul or not according to which stage kind they had picked.
/// The set's shipped proportions fail the involute question and had never said
/// so. *A constraint belongs to the mesh*, and so does what it found: a ring's
/// tip is the mesh's business wherever the ring is.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TipRoom {
    /// The tips foul away from the line of action.
    ///
    /// **The only one of the three that is an internal pair's alone.** The other
    /// two — a tip reaching past a flank's usable end — are every mesh's, and
    /// are [`MeshReport::flank_interference`]; they used to be reported here as
    /// well, which was the same question answered in two places.
    pub tip_interference: bool,
    /// How much room the tips have where their circles cross, as an angle of
    /// **pinion** rotation, degrees. Negative is the overlap, and infinite where
    /// the tip circles do not cross at all — the ordinary case, where there is
    /// no place for the tips to meet.
    pub tip_margin: f64,
}

impl TipRoom {
    /// Read off the pair, at the centre distance it **runs** at — a clearance
    /// inside its zero-backlash one, for an internal pair. Read at the latter
    /// the margin is looser than the teeth have, and the shipped hula stage's
    /// binding mesh sat at exactly nought there and a hair under it as built.
    ///
    /// `None` where the two cannot be meshed at all, which the caller already
    /// knows by other means — every caller here has built the mesh.
    pub(crate) fn at(
        ring: &crate::ring::Ring,
        pinion: &crate::Tooth,
        running: f64,
    ) -> Option<Self> {
        crate::ring::mesh_at(ring, pinion, running).map(|m| Self {
            tip_interference: m.tip_interference,
            tip_margin: m.tip_margin.to_degrees(),
        })
    }

    /// Whether the tips clear **each other**, away from the line of action.
    ///
    /// **Not the whole question**, and it is named so it cannot be read as it:
    /// this was `clear()` and meant all three conditions, so when two of them
    /// moved onto [`MeshReport`] every caller that had been asking the whole
    /// question silently began asking a third of it. One of them was the
    /// harness's own admissibility filter, and the golden corpus caught it as a
    /// diff that read like an improvement.
    ///
    /// [`MeshReport::teeth_clear`] is the whole question.
    #[must_use]
    pub fn tips_clear(&self) -> bool {
        !self.tip_interference
    }
}

impl MeshReport {
    /// **Whether anything on this mesh fouls anything else.**
    ///
    /// Both questions at once: a tip reaching past the usable end of the flank
    /// it meshes with, which is every mesh's, and two tips meeting away from the
    /// line of action, which is an internal pair's alone and does not arise
    /// otherwise.
    ///
    /// It exists because asking them separately is asking a caller to remember
    /// both, and the first caller not to was in this repository within the hour:
    /// `TipRoom::clear` had meant all three and came to mean one, and a filter
    /// that had been rejecting fouling candidates quietly started accepting
    /// them.
    #[must_use]
    pub fn teeth_clear(&self) -> bool {
        self.flank_interference == [false, false] && self.tips.is_none_or(|t| t.tips_clear())
    }

    /// The one gap, read by **drive direction** rather than by member.
    ///
    /// `backlash` is per member — "the one gap seen from each of its ends" — and
    /// a *stage* reports it per direction, because the output of a forward drive
    /// is the second member and of a backward drive the first. Two indexings of
    /// two numbers, and this is the conversion, in one place: it was written out
    /// in the spur stage as a `match` inside a `Directional::of`, which is the
    /// same mapping stated a second time.
    #[must_use]
    pub fn backlash_by_drive(&self) -> Directional<Backlash> {
        Directional {
            forward: self.backlash[1],
            backward: self.backlash[0],
        }
    }
}

/// **A tooth the cutter has eaten into**, where that is a finding rather than a
/// clamp.
///
/// Severing truncates the profile, so `Tooth` records it as a clamp and every
/// stage's member list has carried it. Undercut short of severing alters
/// nothing — the tooth is exactly the one the inputs describe — which is why it
/// is not a clamp, and why nothing was reporting it: a gear tab has shown it
/// since undercut existed, and the same gear inside a geartrain said nothing at
/// all. So it goes where a remark about a member goes, beside the notch band
/// and the reversed root.
///
/// It matters most exactly where a stage cannot prevent it. `no undercut` bounds
/// a shift somebody chooses; a shift that a *relation* leaves over answers to no
/// bound at all — a hula pinion whose ring was pinned, an epicyclic absorber —
/// so the control can be on, the tooth undercut, and the two never meet.
///
/// Nothing for a ring: its flank is its shaper's, and undercut is not a question
/// that can be asked of it.
pub(crate) fn undercut_note(tooth: &crate::tooth::Tooth) -> Option<Note> {
    (tooth.undercut && !tooth.severed).then(|| Note::new(key::CLAMP_TOOTH_UNDERCUT))
}

/// The face width a pair of ratings asks for, at one load case.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Widths {
    /// From bending. `None` where the section has no rating.
    pub bending: Option<f64>,
    /// From contact. `None` where no contact *rating* sizes this member's face.
    ///
    /// Optional for the same reason `bending` is, and it took a crossed pair to
    /// make the case reachable: inverting a stress for a width assumes the
    /// stress depends on the width, and a **point** contact's peak pressure does
    /// not depend on it at all. A crossed pair's automatic width comes from
    /// continuity instead — `ε = 1`, a *geometric* minimum, reported as its own
    /// figure and labelled as the other kind of answer because the two differ by
    /// 2.4× (`docs/state.md`). Putting that number here would be the mixing this
    /// project refuses; putting a zero here would be a width nobody needs, which
    /// `docs/corrections.md` has already been caught by once.
    pub contact: Option<f64>,
}

/// The face width every rating is first evaluated at, mm.
///
/// Any width will do, and that is the point: `b_min` does not depend on the `b`
/// it was measured at (docs/reference.md#contact-stress), so one evaluation
/// gives every minimum and nothing has to be iterated to find the width a
/// member needs before it is given one.
pub(crate) const PROBE: f64 = 10.0;

/// **What one member bends at in one mesh**: the critical section, the share of
/// the mesh load acting on it, and what the sharing model has to say about
/// being asked outside the band it was described in.
///
/// All three are per *(member, mesh)*, which is why they travel together and
/// why this is one place rather than one per stage kind. It had been none: the
/// pair asked for the section and the share inline and raised the note itself,
/// and the two epicyclic kinds asked for neither — so the one estimate this
/// crate ships reached one stage of three, and a set that switched the model on
/// got it on the members it happened to share code with.
pub(crate) struct Bending {
    pub section: crate::strength::RootSection,
    /// Exactly 1 where no sharing model was asked for, so the ordinary rating
    /// is untouched to the bit.
    pub share: f64,
    /// **`Y_B`** — the one factor of `σ_F0` that is the member's *blank's*
    /// rather than its root section's. `None` where nobody described a rim,
    /// which is every gear this crate rated before it existed.
    pub rim: Option<crate::strength::RimSupport>,
    /// **The ramp outside the band it was described in.** It is a first-order
    /// stand-in for a mesh with a single-pair zone; at `ε_n ≥ 2` there is no
    /// such zone and the ramp never reaches a full share. **What that does to
    /// the figure has no fixed direction**: measured across high-contact-ratio
    /// spur designs it runs from a 24 % relief to a 15 % *increase*, because
    /// the swept maximum is a product of a form factor rising toward the tip
    /// and a share falling away there, and which wins is the tooth's business.
    /// A large number either way from an uncalibrated model — exactly what
    /// `docs/rationale.md` refuses to let pass silently — so it is said where
    /// the figure is shown. The model is still the one the designer asked for;
    /// what they are owed is knowing it is extrapolating.
    ///
    /// A *mesh's* finding, so a stage raises it once per mesh rather than once
    /// per member; a member's own findings — its notch band, its rim — go in
    /// its own list.
    pub note: Option<Note>,
}

impl Bending {
    /// **For any member that bends**, rack-cut or shaper-cut.
    ///
    /// `rim` is the thickness of the rim under its teeth, mm, or `None` where
    /// nobody said — see [`crate::strength::RimSupport`]. Which reference that
    /// thickness is measured against is the member's own business
    /// ([`ToothOutline::rim_support`]), as the direction its load point travels
    /// is; this was two functions and they differed in nothing else.
    pub(crate) fn of<T: crate::strength::ToothOutline>(
        member: &T,
        contact_ratio: f64,
        model: crate::contact::LoadSharing,
        rim: Option<f64>,
    ) -> Option<Self> {
        let (section, share) =
            crate::strength::bending_section_shared(member, contact_ratio, model)?;
        let cos_bb = member.base_helix_angle().cos();
        Some(Self::new(
            section,
            share,
            model,
            contact_ratio / (cos_bb * cos_bb),
            rim.map(|s| member.rim_support(s)),
        ))
    }

    fn new(
        section: crate::strength::RootSection,
        share: f64,
        model: crate::contact::LoadSharing,
        eps_n: f64,
        rim: Option<crate::strength::RimSupport>,
    ) -> Self {
        let out_of_band = !matches!(model, crate::contact::LoadSharing::None) && eps_n >= 2.0;
        Self {
            section,
            share,
            rim,
            note: out_of_band
                .then(|| Note::new(key::MESH_LOAD_SHARING_OUT_OF_BAND).number("ratio", eps_n, 3)),
        }
    }
}

/// The note a rating raises about one member's **rim**: it is thinner than the
/// clause will rate.
///
/// ISO 6336-3:2019, 9.3 says a backup ratio at or below 0,5 (external) or a rim
/// below 1,75 normal modules (internal) "shall be avoided" rather than giving a
/// value for it. `Y_B` still evaluates there and this crate still reports it —
/// the fit is a logarithm and does not fall over — but a design that far in is
/// past where the standard will go, and it says so instead of returning a
/// number that looks like the others.
///
/// A **member's** finding rather than a mesh's, which is why it is here and not
/// in [`Bending::note`]: two members of one mesh have two rims and one sharing
/// model between them.
pub(crate) fn rim_below_minimum(rim: Option<crate::strength::RimSupport>) -> Option<Note> {
    rim.filter(|r| !r.in_range())
        .map(|r| Note::new(key::GEAR_RIM_BELOW_MINIMUM).number("ratio", r.ratio(), 2))
}

/// **What one mesh does to one member**: the two stresses it produces there,
/// and the widths they belong to.
///
/// A member is not always in one mesh. A planet is in two, and a stage kind
/// nobody has written yet may put a member in more — so a rating is taken over
/// *however many there are* rather than over a named pair, and adding a mesh to
/// a member is adding an entry to a list rather than an arm to an expression.
///
/// Both widths are here because they are genuinely two questions. A mesh
/// carries a member at the narrower of its two members' faces, which differs
/// from mesh to mesh; and the stresses may have been evaluated somewhere else
/// entirely — at [`PROBE`], before any width was settled.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Loading {
    /// Root bending stress this mesh produces on the member, MPa at
    /// [`Self::measured_at`], under the torque the stage solved at. `None`
    /// where the section has no rating — a ring with no fillet is the ordinary
    /// way to get here.
    pub bending: Option<f64>,
    /// The member's **governing** contact stress in this mesh, MPa at
    /// [`Self::measured_at`]: where its own dedendum is loaded alone.
    pub contact: f64,
    /// The face width both figures were evaluated at, mm.
    pub measured_at: f64,
    /// The width this mesh actually carries the member at, mm.
    ///
    /// Equal to [`Self::measured_at`] in two quite different cases, and it is
    /// worth knowing which: during the probe pass, when no width has been
    /// settled and none is needed ([`MemberRating::asks`]); and for a stage
    /// that evaluated its stresses at the width it ended with, where there is
    /// nothing to scale and the figures are the ones its own arithmetic
    /// produced, bit for bit.
    pub carried_at: f64,
}

impl Loading {
    /// The two stresses as they stand at [`Self::carried_at`].
    ///
    /// Bending is inversely linear in width and contact goes as the inverse
    /// square root of it, so a change of width is a scale rather than a second
    /// solve — and where the two widths are equal this is the identity, which
    /// is what lets a stage that evaluated at its final width keep its own
    /// digits to the bit.
    fn at_width(self) -> (Option<f64>, f64) {
        let by = self.measured_at / self.carried_at;
        (self.bending.map(|s| s * by), self.contact * by.sqrt())
    }

    /// **The same loading under `k` times the torque.**
    ///
    /// Bending is linear in torque and contact goes as its square root
    /// (docs/reference.md#load-cases), so where a stage's power split does not
    /// depend on the *magnitude* of what passes through it — which is every
    /// kind here, and is a fact about each kind rather than about gearing — a
    /// second load case is this rather than a second solve. A stage whose flow
    /// does not have that property builds each case's loadings itself, which is
    /// why they are held per case rather than as one list and a factor.
    fn under(self, k: f64) -> Self {
        Self {
            bending: self.bending.map(|s| s * k),
            contact: self.contact * k.sqrt(),
            ..self
        }
    }

    /// Every load case, for a stage whose ratings scale with the torque.
    ///
    /// `meshes` is one loading per mesh the member is in, evaluated at the
    /// worst torque that mesh carries, with each case's torque as a fraction of
    /// it ([`StageLoads::scaled`]). **The scale is the mesh's, not the
    /// stage's**: which direction loads a mesh hardest is a fact about that
    /// mesh, and a set's two meshes need not agree about it.
    pub(crate) fn for_cases(loads: &StageLoads, meshes: &[(Self, Vec<f64>)]) -> Vec<CaseLoadings> {
        loads
            .cases
            .iter()
            .enumerate()
            .map(|(c, load)| CaseLoadings {
                load: *load,
                meshes: meshes.iter().map(|(l, by)| l.under(by[c])).collect(),
            })
            .collect()
    }
}

/// What every mesh a member is in does to it under one load case.
pub(crate) struct CaseLoadings {
    pub load: StageLoad,
    pub meshes: Vec<Loading>,
}

/// **What one member's ratings come to**, over every mesh it is in.
///
/// Every stage kind asks the same questions of every member it builds — two
/// stresses in every load case, and the width each of those would need — and
/// each of them had been writing the arithmetic out for itself. What
/// genuinely differs between kinds is what the meshes do to the member and
/// which allowable a reversed root answers to, and both arrive here as values.
///
/// **The worst mesh wins, figure by figure.** A planet's root is loaded by the
/// sun on one flank and the ring on the other, at different tangential forces
/// through different sections, and its flanks pit at whichever end of whichever
/// path is worst — so neither figure is the sun mesh's by right. Taking the
/// worse of the two was written out for contact and simply omitted for bending;
/// here it is one fold over a list, which is the same answer for two meshes and
/// an answer at all for three.
pub(crate) struct MemberRating<'a> {
    /// The material as used, after any overrides.
    pub material: &'a Material,
    /// How the train treats a root loaded on both flanks.
    pub reversal: Reversal,
    /// ...and whether this member's is, whatever the load does — a planet's.
    pub always_reverses: bool,
    /// **Every mesh this member is in, in each load case.** One for an ordinary
    /// gear, two for a planet, and a list rather than a pair so that neither is
    /// the special case.
    ///
    /// Per case rather than one list and a factor, because "the next case is
    /// this one times a number" is a claim about a *stage's power flow* rather
    /// than about gearing. It holds for every kind here and
    /// [`Loading::for_cases`] is how they say so; a kind whose flow does not
    /// scale with what passes through it builds each case for itself, and needs
    /// nothing added here to do it.
    pub cases: Vec<CaseLoadings>,
}

/// The rating fields of one [`GearCase`], over every mesh a member is in.
pub(crate) struct Rated {
    pub load: StageLoad,
    pub bending_stress: Option<f64>,
    pub contact_stress: f64,
    pub min_face_width: Widths,
}

impl Rated {
    /// The case's readout, once the stage has said what else it knows of the
    /// member in it: its torque at its own radius, its speed in the fixed
    /// frame and against its carrier, and how often it is engaged for each
    /// turn of the shaft the stage took the load on ([`engagements`]) — which
    /// for a held ring is not zero while its speed is.
    pub(crate) fn into_case(self, torque: f64, speeds: (f64, f64), engagements: f64) -> GearCase {
        GearCase {
            case: self.load.case,
            torque,
            speed: speeds.0,
            speed_against_carrier: speeds.1,
            cycles: self
                .load
                .turns
                .map(|t| loaded_cycles(t.scaled(engagements))),
            bending_stress: self.bending_stress,
            contact_stress: self.contact_stress,
            min_face_width: self.min_face_width,
        }
    }
}

impl MemberRating<'_> {
    /// The rating: each mesh at the width it carries the member at, and the
    /// worst of them, in every case.
    pub(crate) fn rated(&self) -> Vec<Rated> {
        self.cases
            .iter()
            .map(|c| {
                // A bending stress that no mesh could rate stays absent; one
                // that any mesh could rate is that mesh's worst, and a mesh
                // with no rating does not make an absence out of a figure
                // another mesh has.
                let mut bending: Option<f64> = None;
                let mut contact = 0.0_f64;
                let mut width = 0.0_f64;
                for l in &c.meshes {
                    let (b, s) = l.at_width();
                    if let Some(b) = b {
                        bending = Some(bending.map_or(b, |had: f64| had.max(b)));
                    }
                    if s >= contact {
                        contact = s;
                    }
                    width = width.max(l.carried_at);
                }
                let reverses = self
                    .reversal
                    .reverses(self.always_reverses, c.load.reverses());
                Rated {
                    load: c.load,
                    bending_stress: bending,
                    contact_stress: contact,
                    // **Each figure is inverted at the width it was taken at**,
                    // and the widest mesh is the one that answers: a minimum
                    // is what this member would need, and it needs enough for
                    // every mesh it is in.
                    min_face_width: Widths {
                        // Two allowables, because a reversed root endures less
                        // bending while its flank pits exactly as it did.
                        bending: bending.map(|s| {
                            crate::strength::min_face_width_bending(
                                s,
                                width,
                                self.reversal.bending_allowable(
                                    self.material,
                                    c.load.kind,
                                    reverses,
                                ),
                            )
                        }),
                        contact: Some(crate::strength::min_face_width_contact(
                            contact,
                            width,
                            allowable(self.material, c.load.kind),
                        )),
                    },
                }
            })
            .collect()
    }

    /// The widths this member's ratings ask for, each with the kind of case
    /// that asked.
    ///
    /// Takes no width, because the answer does not depend on one: a minimum
    /// width is a stress inverted, and the stress it inverts scales with the
    /// width it was measured at by exactly the amount that cancels
    /// (docs/reference.md#contact-stress). So a stage can size a member from a
    /// probe pass, before it has a width to size it at.
    pub(crate) fn asks(&self) -> Vec<(CaseKind, Widths)> {
        self.rated()
            .into_iter()
            .map(|r| (r.load.kind, r.min_face_width))
            .collect()
    }
}

/// **What a gear's addendum comes to**, once its bound has had its say.
///
/// The same shape as [`ShiftAsked`] and for the same reason: two stages were
/// working it out for themselves, in the same four lines, and the answer has
/// two halves — the number to build with, and whether it is the number that was
/// asked for.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AddendumAsked {
    /// The coefficient to build with, in modules.
    pub used: f64,
    /// Whether the bound cut it down, and so whether the tooth is the one that
    /// was asked for.
    pub clamped: bool,
}
impl AddendumAsked {
    /// The note a clamped addendum owes its reader, if it was clamped.
    pub(crate) fn note(&self) -> Option<crate::note::Note> {
        self.clamped.then(|| {
            crate::note::Note::new(crate::note::key::GEAR_ADDENDUM_HELD_TO_TIP_WIDTH)
                .number("addendum", self.used, 4)
        })
    }

    /// **The same finding where the stage cannot act on it**, which is a report
    /// rather than a clamp.
    ///
    /// A hula stage solves its crank offset from a gap written in the
    /// tips, in closed form with an analytic derivative. An addendum that moved
    /// with the shift — which moves with the offset — would put a tip-width
    /// solve inside that root-find and take the derivative away with it. Not
    /// every bound an input creates needs a solver behind it: this one says
    /// what the tooth would have to be and leaves the number alone.
    pub(crate) fn warning(&self) -> Option<crate::note::Note> {
        self.clamped.then(|| {
            crate::note::Note::new(crate::note::key::GEAR_ADDENDUM_ABOVE_TIP_WIDTH)
                .number("addendum", self.used, 4)
        })
    }
}

/// A shift clears undercut unless it is told not to — and a document written
/// before the question was asked separately meant exactly that.
#[cfg(feature = "serde")]
const fn yes() -> bool {
    true
}
/// A coefficient that used to be an `Auto` and is now a number.
///
/// Read either shape: a document written before the addendum's bound was split
/// from its value holds `{ auto, manual }`, and the number it meant is the
/// `manual` one — with `auto` on, it meant "and hold it to the tip width",
/// which [`StageGear::no_sharp_tip`] says now and defaults to.
#[cfg(feature = "serde")]
fn coefficient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Either {
        Number(f64),
        WasAuto { manual: f64 },
    }
    Ok(match <Either as serde::Deserialize>::deserialize(d)? {
        Either::Number(v) | Either::WasAuto { manual: v } => v,
    })
}

// -------------------------------------------------- the shared member ---
//
// `StageGear` is what *every* stage kind describes a member with, so it lives
// here with the rest of the shared vocabulary rather than in the kind that
// happened to need it first. It was declared in `spur.rs` and re-exported from
// this module, which read as though the parallel-axis stage owned it — and this
// module's own comment says it holds "what every stage kind shares".

/// One gear of a stage.
///
/// Note what is *absent*: module, pressure angle and helix angle live on the
/// stage, because they are shared.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct StageGear {
    pub teeth: u32,
    /// The shift, and who decides it: **automatic means the stage does**, not
    /// that undercut does. What it resolves to when nothing else constrains it
    /// is [`StageGear::no_undercut`]'s business.
    pub profile_shift: Auto<f64>,
    /// **The shift may not go below the least that clears undercut.**
    ///
    /// A constraint rather than a source, which is what lets it combine with
    /// everything else: it bounds a shift a designer typed, a shift the stage
    /// solved from a centre distance or a crank offset, and a shift the
    /// efficiency search chose, all in the same words.
    ///
    /// The bound is the **true** minimum from [`minimum_profile_shift`], which
    /// on a comfortable tooth count is negative — so a deliberate negative
    /// shift is left alone and only a genuinely undercut one is raised. That is
    /// deliberate: negative shift is a decision about centre distance or
    /// balance, and this is a question about undercut. Where the *stage* is
    /// choosing and nothing else decides, the answer is instead
    /// [`automatic_profile_shift`] — the same bound taken no lower than zero,
    /// because a shift chosen for no reason should not thin a tooth that needed
    /// no help.
    ///
    /// Off, the gear may undercut, and the searches stop asking
    /// ([`crate::auto::member_is_buildable`]).
    ///
    /// **Meaningless on a ring**, whose flank is its shaper's rather than a
    /// rack's, and which is never asked — see `member_is_buildable`.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub no_undercut: bool,
    /// Depth, in modules, at which the undercut question is asked.
    ///
    /// **Automatic is the gear's own dedendum**, which makes this ask the same
    /// question the profile generator answers: *is the flank undercut at all?*
    /// A fixed 1 module — the classical rule, and what this used to default to —
    /// asks a narrower one, *is it undercut within a module of depth?*, and the
    /// two have different answers: at α = 20° with a sharp rack they part at 18
    /// teeth and 22 (docs/reference.md#automatic-values). Following the dedendum rather than naming a number
    /// also means a gear cut shallower is asked about the depth it actually has.
    pub working_depth: Auto<f64>,
    /// Addendum coefficient, in modules, as asked for.
    ///
    /// Plain, because the only thing an automatic addendum ever computed was
    /// the tallest tooth that keeps a tip [`Self::min_tip_width`] wide — which
    /// is a **bound on the number**, not a source for it, and now says so.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "coefficient"))]
    pub addendum: f64,
    /// **The tooth may not be taller than its tip is wide.**
    ///
    /// The same shape as [`Self::no_undercut`], on the other end of the tooth:
    /// a constraint the number answers to however it arrived, rather than a
    /// mode the number is only read in. It was the latter, and so
    /// `min_tip_width` went unread on every addendum a designer typed — a
    /// tooth could come to a point and nothing said so.
    ///
    /// Off, the addendum stands as asked and the tip is whatever it is.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub no_sharp_tip: bool,
    /// Minimum transverse tooth tip width, mm.
    pub min_tip_width: f64,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Helix angle, degrees, signed by hand — and who decides it.
    ///
    /// **Automatic means the stage does**, through whatever relates this
    /// member's helix to the rest of it: a pair's two are bound by
    /// `β₁ + β₂ = Σ` and the first member's by its pitch diameter, a set's
    /// three by the hands its two meshes require, a hula stage's four by its
    /// two internal meshes. So at most one member of a stage states a helix
    /// and the others follow — or none does, and the stage's own relation
    /// decides: a given centre distance with both shifts pinned sizes a pair's
    /// first member, and a given axial contact ratio with every face width
    /// given sizes the helix any kind needs to reach it
    /// ([`Stage::freedoms`] says which may stand). Where nothing decides it,
    /// the first member's stands at its box.
    pub helix_angle: Auto<f64>,
    /// Automatic takes the larger of the enabled minimums below, and the
    /// width a given axial contact ratio needs where the stage has one.
    pub face_width: Auto<f64>,
    /// Which of the four ratings an automatic face width is sized from.
    pub face_sources: FaceSources,
    /// **Thickness of the rim under this member's teeth, mm** — `s_R`, and with
    /// it the rim thickness factor `Y_B` (ISO 6336-3:2019, Clause 9).
    ///
    /// `None` is the default and means *nobody said*, which is not the same
    /// claim as a thick rim even though both rate at `Y_B = 1`: a gear whose rim
    /// was never described cannot be told it is too thin, and one that was can.
    /// Given, it de-rates the root — a thin rim moves the failure out of the
    /// fillet and through the rim — and never relieves it.
    ///
    /// Measured against the whole tooth depth on a rack-cut member and against
    /// the normal module on a ring, which is the clause's own distinction and
    /// the only one: see [`RimSupport`](crate::strength::RimSupport).
    ///
    /// It reaches bending alone. A rim under the teeth has nothing to do with
    /// the pressure between two flanks, so no contact rating reads it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rim_thickness: Option<f64>,
    /// Name of a material in the library.
    pub material: String,
    /// Properties replaced for this gear only. Empty means "as the library
    /// says" — see [`Overrides`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub material_overrides: Overrides,
}

impl StageGear {
    /// **An automatic width with nothing to size it**, said on the gear.
    ///
    /// Every source switched off leaves nothing to invert, so the width stands
    /// at the number in its box ([`FaceSources::width_for`]) — said rather than
    /// divided by, which is what a zero width was. Every kind used to raise it
    /// on the stage, naming the gear; it is the gear's, drawn under its own
    /// face-width field.
    pub(crate) fn face_width_note(&self) -> Option<Note> {
        (self.face_width.auto && !self.face_sources.any())
            .then(|| Note::new(key::GEAR_FACE_WIDTH_NO_SOURCE))
    }

    /// **The addendum this gear builds at**, at a given shift.
    ///
    /// The bound is an upper one — a taller tooth is a sharper tooth — so the
    /// answer is the smaller of what was asked and what the tip width allows.
    /// It depends on the shift, which is why it is asked per built tooth rather
    /// than once per gear.
    ///
    /// A tooth already thinner than `min_tip_width` at its base circle has no
    /// admissible addendum at all ([`addendum_for_tip_width`] returns `None`);
    /// there is nothing to clamp to, so the number stands and the tooth's own
    /// `pointed` reporting is what says it is wrong.
    pub(crate) fn addendum_asked(&self, at_shift: &crate::params::GearParams) -> AddendumAsked {
        let asked = self.addendum;
        if !self.no_sharp_tip {
            return AddendumAsked {
                used: asked,
                clamped: false,
            };
        }
        let ceiling = addendum_for_tip_width(
            &Tooth::new(GearParams {
                addendum: asked,
                ..*at_shift
            }),
            self.min_tip_width,
        );
        let used = ceiling.map_or(asked, |c| asked.min(c));
        AddendumAsked {
            used,
            clamped: used < asked - 1e-12,
        }
    }

    /// [`ShiftAsked`], for this gear at its own working depth.
    pub(crate) fn shift_asked(&self, base: &crate::params::GearParams) -> ShiftAsked {
        let depth = self.working_depth.resolve(self.dedendum);
        if !self.no_undercut {
            // Nothing asked of the shift. Given, it is taken as typed; left to
            // the stage with no objective either, there is no reason to move
            // the tooth at all.
            let given = (!self.profile_shift.auto).then_some(self.profile_shift.manual);
            return ShiftAsked {
                search_floor: None,
                given,
                settled: given.unwrap_or(0.0),
                raised: false,
            };
        }
        if self.profile_shift.auto {
            let floor = automatic_profile_shift(base, depth);
            return ShiftAsked {
                search_floor: Some(floor),
                given: None,
                settled: floor,
                raised: false,
            };
        }
        // Given, and held to the true minimum rather than to the search's — a
        // negative shift somebody meant is not an undercut one. Nothing is
        // choosing it, so it carries no search bound.
        let typed = self.profile_shift.manual;
        let used = typed.max(crate::auto::minimum_profile_shift(base, depth).with_cutter_radius);
        ShiftAsked {
            search_floor: None,
            given: Some(used),
            settled: used,
            raised: used > typed,
        }
    }
}

impl Default for StageGear {
    fn default() -> Self {
        Self {
            teeth: 17,
            profile_shift: Auto::automatic(0.0),
            no_undercut: true,
            working_depth: Auto::automatic(1.0),
            addendum: 1.0,
            no_sharp_tip: true,
            min_tip_width: 0.1,
            dedendum: 1.25,
            root_radius: 0.38,
            helix_angle: Auto::automatic(0.0),
            face_width: Auto::fixed(10.0),
            face_sources: FaceSources::default(),
            // A rim nobody described: `Y_B` is 1, and the gear cannot be told
            // its rim is thin because it has not said what its rim is.
            rim_thickness: None,
            material: "4340 Hardened Steel".to_string(),
            material_overrides: Overrides::default(),
        }
    }
}

/// What one load case does to one gear.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct GearCase {
    /// Index into the train's list of load cases.
    pub case: usize,
    /// Torque on this gear, N·m, signed like its own rotation, in this case's
    /// direction of travel — a member at the far end of a mesh carries the
    /// load referred by the ratio and cut by the loss the mesh takes carrying
    /// it *that* way, which for a locked mesh is nought.
    pub torque: f64,
    /// Rotational speed, rpm, from the case's speed at its port.
    pub speed: f64,
    /// Speed **relative to the carrier of its mesh**, rpm — what its teeth
    /// actually see, and what its cycles are counted from. A pair has no
    /// carrier and this is [`Self::speed`]; an epicyclic member's is not, and
    /// a held ring's is not zero while its speed is.
    pub speed_against_carrier: f64,
    /// Tooth load cycles over the duty this case describes — a fatigue case's,
    /// and `None` on an ultimate one, which is survived once.
    ///
    /// One cycle per revolution for a simple gear: a given tooth meets the mate
    /// once per turn. An epicyclic set's members are engaged once per turn
    /// relative to the carrier, once per planet — docs/reference.md#trains.
    pub cycles: Option<Cycles>,
    /// Tooth root bending stress, MPa. `None` where the stress correction is
    /// undefined for this section — see [`crate::strength::bending_stress`].
    pub bending_stress: Option<f64>,
    /// Hertzian contact stress, MPa, at **this gear's** governing point.
    ///
    /// The two gears of a mesh share one patch and one pressure at any instant,
    /// so this is not a per-tooth curvature effect. They differ because they are
    /// rated at different *moments*: each gear's dedendum is loaded alone at one
    /// end of the path, and that is where its own pitting is assessed. See
    /// [`crate::strength::ContactStress::governing`].
    pub contact_stress: f64,
    /// The face width each rating would need.
    pub min_face_width: Widths,
}

/// What a stage does to one of its gears.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct GearResult {
    /// **The tooth as built** — the parameters the stage cut this member
    /// with, every automatic value resolved and every convention applied: the
    /// shift in force, the addendum the tip allows, the helix with the hand
    /// this member has, a planet's `2 − k`. What the gear tab receives when it
    /// adopts a member ([`Stage::member_cutter`] says whether it is a ring),
    /// and what a caller that wants to draw or measure this member starts
    /// from, rather than rebuilding it from the inputs and hoping to agree.
    pub params: GearParams,
    /// The shift in force, after any automatic calculation.
    pub profile_shift: f64,
    /// Likewise the addendum.
    pub addendum: f64,
    /// Likewise the face width.
    pub face_width: f64,
    /// What a convention recommends for the face width, mm, where one applies
    /// and whether or not it is in use — a worm's length and its wheel's width
    /// ([`crossed::proportions`]). `None` where a rating sizes the face
    /// instead, which is every other member.
    pub recommended_face_width: Option<f64>,
    /// Reference pitch diameter, mm — `z m_n / cos β`.
    ///
    /// An output rather than an echo: a worm's is solved where its size is
    /// automatic, and every other member's follows from a helix that may have
    /// been.
    pub pitch_diameter: f64,
    /// Helix angle, degrees, signed by hand — likewise from the size as
    /// solved, whichever reading of it was given.
    pub helix_angle: f64,
    /// Lead angle, degrees — `90° − |β|`, the same fact from the other datum. A
    /// worm is described by how far its thread advances, a gear by how far its
    /// tooth leans; both are reported because a pair is entered either way.
    pub lead_angle: f64,
    /// Lead, mm — how far a point on the flank advances per revolution,
    /// `π d tan γ`. `None` on a spur gear, whose flank does not advance.
    pub lead: Option<f64>,
    /// **What every load case does to this gear**: the torque it puts on it,
    /// the speed it turns at, how often it is loaded, and the two stresses with
    /// the width each would need. One entry per enabled case, in the train's
    /// order, each naming its case.
    ///
    /// Per case rather than a field per figure with a case inside it, because
    /// a case is the thing a reader looks up: what does *this* load do here.
    pub cases: Vec<GearCase>,
    /// Guards that altered this gear's geometry.
    pub clamps: Vec<crate::note::Note>,
    /// What the **rating** has to say about this gear, as against what was done
    /// to its geometry.
    ///
    /// Kept apart from [`Self::clamps`] because nothing here moved a dimension:
    /// a root loaded on both flanks and a notch outside the `Y_S` fit's band are
    /// statements about how the number was arrived at, not about the part.
    ///
    /// **Per gear rather than per stage**, and that is not filing. A stage note
    /// naming a member has to carry the member's name in its own text, and two
    /// members raising the same note give one list two entries with one key —
    /// which a keyed list in the front end cannot render (`docs/corrections.md`).
    /// A note that belongs to a gear belongs *on* the gear.
    #[cfg_attr(feature = "serde", serde(default))]
    pub notes: Vec<crate::note::Note>,
    /// The material as used, after any overrides — what the numbers were
    /// actually computed from, rather than what the library holds.
    pub material: Material,
    /// What this gear's geometry allows its own inputs to be.
    ///
    /// Computed from the **resolved** parameters, so an automatic profile shift
    /// or addendum is already folded in. The UI bounds its fields by these
    /// rather than by constants — see `docs/reference.md#input-ranges`.
    pub ranges: Ranges,
}

/// The facts a stage has about one of its members, gathered so that assembling
/// a [`GearResult`] is one expression rather than one per stage kind.
///
/// # Why this exists
///
/// Three kinds built a `GearResult` field by field, listing the same seventeen
/// names each time. Sixteen agreed. The seventeenth did not: a member's share
/// of a load from the far end was the mesh projection of the backward torque in
/// the spur stage and this gear's forward torque scaled by `|t| / |forward|` in
/// the other two — the same number wherever the projection is linear, which it
/// is, **except in sign**. A stage with a negative ratio gave a signed figure
/// from one kind and a magnitude from another, for the field beside the forward
/// torque, which is signed.
///
/// That is the fault `docs/corrections.md` opens with: a duplicated formula is a
/// place where two answers can differ, and nothing compared these two.
pub(crate) struct MemberFacts<'a> {
    /// The shift in force — which for a hula gear is the layout's, not the
    /// parameters', so it is given rather than read.
    pub profile_shift: f64,
    pub params: &'a GearParams,
    pub input: &'a StageGear,
    /// Every load case's readout, from [`Rated::into_case`].
    ///
    /// **The torque in each is given by the stage rather than derived here**,
    /// and that is the finding. Three kinds referred a back-driving load by
    /// scaling this member's *forward* torque, which is exact wherever the
    /// forward torque is a geometric projection or the two directional
    /// efficiencies agree — true of every parallel-axis kind. A worm stage is
    /// neither: its wheel's forward torque carries a forward efficiency of 62 %
    /// that a backward load does not share, and its backward efficiency is
    /// zero. So each kind projects each case's torque through its own
    /// construction in that case's direction, and this holds no formula a kind
    /// could need to disagree with.
    pub cases: Vec<GearCase>,
    /// The width this member is *rated at*, which is its mesh's rather than its
    /// own ([`Widths`]).
    pub face_width: f64,
    /// A convention's recommendation for the width, where one applies.
    pub recommended_face_width: Option<f64>,
    pub material: Material,
    pub clamps: Vec<Note>,
    pub notes: Vec<Note>,
}

impl GearResult {
    /// One member's result, from what the stage knows about it.
    ///
    /// Every stage kind comes through here, so a field cannot be filled two ways
    /// — see [`MemberFacts`] for the one that was.
    pub(crate) fn of(f: MemberFacts) -> Self {
        let pitch_diameter =
            f64::from(f.params.teeth) * f.params.module / f.params.helix_angle.to_radians().cos();
        Self {
            params: *f.params,
            profile_shift: f.profile_shift,
            addendum: f.params.addendum,
            face_width: f.face_width,
            recommended_face_width: f.recommended_face_width,
            pitch_diameter,
            helix_angle: f.params.helix_angle,
            lead_angle: 90.0 - f.params.helix_angle.abs(),
            lead: (f.params.helix_angle != 0.0).then(|| {
                std::f64::consts::PI * pitch_diameter
                    / f.params.helix_angle.abs().to_radians().tan()
            }),
            cases: f.cases,
            clamps: f.clamps,
            notes: f.notes,
            material: f.material,
            ranges: crate::auto::admissible_ranges(
                f.params,
                f.input.working_depth.resolve(f.input.dedendum),
            ),
        }
    }

    /// **Whether this is the gear that was asked for.**
    ///
    /// False where a guard moved a dimension ([`Self::clamps`]), and false where
    /// a bound overrode a number a designer typed or reported that it would have
    /// to be — the shift raised to clear undercut, and the addendum that a tip
    /// width cannot carry. Those two live in [`Self::notes`] because that is
    /// where a note naming an *input* belongs, but they are statements about the
    /// part rather than about the rating.
    ///
    /// The rest of `notes` is deliberately not consulted. A notch outside the
    /// `Y_S` fit's band or a root loaded on both flanks are remarks about how a
    /// number was *arrived at*, and a candidate design is not worse for carrying
    /// one.
    ///
    /// One home, because a search choosing between designs wants exactly this
    /// question and there is more than one such search. Reading the clamp list
    /// alone was the same question asked with half the evidence, and it changed
    /// its answer the day the two notes moved out of that list into the one the
    /// spur stage had always put them in.
    #[must_use]
    pub fn as_asked(&self) -> bool {
        self.clamps.is_empty()
            && !self.notes.iter().any(|n| {
                n.is(key::GEAR_SHIFT_RAISED_FOR_UNDERCUT)
                    || n.is(key::GEAR_ADDENDUM_ABOVE_TIP_WIDTH)
                    || n.is(key::GEAR_ADDENDUM_HELD_TO_TIP_WIDTH)
            })
    }
}

/// Everything a pair produces, whichever mesh it has.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PairResult {
    /// `z₂ / z₁`.
    pub ratio: f64,
    /// Zero-backlash centre distance, mm.
    pub centre_distance_nominal: f64,
    /// **The clearance the pair actually runs at**, which is
    /// `centre_distance − centre_distance_nominal` and is derived rather than
    /// echoed.
    ///
    /// A centre distance is the **true** distance and a clearance is what
    /// portion of it is clearance, so these three numbers are two facts and a
    /// subtraction — and reporting the input back is a fourth place the same
    /// quantity can be said, which was wrong whenever the distance was given.
    /// A pair told to run at 30.3 mm whose shifts put it at 30.0057 has 0.294 mm
    /// of clearance; it used to report 0.02, the number in the box, or zero.
    ///
    /// Derived, so the panel can still grey the input by reading the answer,
    /// and so the answer cannot disagree with the two numbers above it.
    pub clearance: f64,
    /// The centre distance actually used, including clearance.
    pub centre_distance: f64,
    /// **What this pair's one mesh reports**, in the type every mesh of every
    /// kind reports in — a line contact on parallel shafts, a point contact
    /// on crossed ones, and [`MeshReport`] says which by what it carries.
    ///
    /// Seven fields used to sit here loose — the operating pressure angle, the
    /// three contact ratios, whether the pair hunts, the shared contact stress
    /// and relative radius, the efficiency and the backlash — which made the
    /// parallel-axis stage the one kind not using the type named for what a
    /// parallel-axis mesh reports; then a crossed pair had a type of its own
    /// and an enum chose. The front end had the same split each time.
    ///
    /// A pair's efficiency and backlash **are** its mesh's — one mesh, no
    /// carrier — so they are read through here rather than stored a second
    /// time, and `StageResult::efficiency` is where every kind is made to agree
    /// about which level it is being asked for.
    pub mesh: MeshReport,
    pub gears: [GearResult; 2],
    /// Anything the stage had to say about the design.
    pub notes: Vec<crate::note::Note>,
}

/// Why a stage could not be solved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrainError {
    /// The screw pair cannot exist — see [`crate::screw::ScrewError`].
    Screw(crate::screw::ScrewError),
    /// The pair cannot mesh, or the shifts put it outside the involute domain.
    Mesh(MeshError),
    /// No usable path of contact — the teeth do not reach each other.
    ///
    /// **Three different things used to say this**, and two of them were not
    /// about teeth at all: a set whose centre distances no shift can bring
    /// together, and a power flow with no self-consistent answer. Both are
    /// their own variants below, and both used to render *"the teeth never
    /// contact"* at a reader who would then go and look at the teeth. See
    /// `docs/corrections.md`.
    NoContact,
    /// **No profile shift brings the two centre distances together.**
    ///
    /// An epicyclic set's meshes share one physical distance and nothing in
    /// the tooth counts makes them agree; a shift is solved to close the gap,
    /// and for most counts none can ([`crate::planetary::shift_bracket`]). It
    /// is a statement about the *assembly*, not about contact — the teeth are
    /// fine, and the kinematics is unaffected, which is why a train reports its
    /// ratios through this.
    NoCommonDistance,
    /// **The stage's members and meshes do not describe a mechanism.**
    ///
    /// The one refusal here a *design* can reach is a member with no teeth —
    /// `StageGear::teeth` is a `u32` and nothing stops a designer typing zero.
    /// It used to be reported as *"the tooth is too undercut to have a root
    /// section"*, which describes a tooth that exists, and the check now runs
    /// **before** any geometry so the answer is about what is wrong.
    Wiring(WiringError),
    /// **No self-consistent power flow.** The arrangement is self-locking, or
    /// the shaft named as the input is not the one driving (`T ω ≤ 0`) — see
    /// [`crate::planetary::power`], which tries both signs of the rolling power
    /// and keeps the physical one, or neither.
    NoPowerFlow,

    /// A material name that is not in the library.
    UnknownMaterial(String),
    /// A tooth so undercut there is no root section left to rate.
    NoRootSection,
    /// The train has no stages, so there is nothing to accumulate.
    Empty,
    /// **The train is short of conditions**: with what it holds and drives,
    /// its shafts have this many more degrees of freedom than one motion.
    /// The motion is still reported, as a family — a differential's carrier
    /// at half the sum of its sides is an answer, and a useful one — but a
    /// rating wants one torque at one speed, so the stages are not rated
    /// until a shaft is held or driven for each.
    Underdetermined { short: usize },
    /// **Two conditions cannot both hold**, at this shaft: what it is asked
    /// to do contradicts what the meshes and the other conditions already
    /// decided — a sun driven while its carrier and its ring are both held.
    Overdetermined { at: ShaftRef },
    /// A constraint, a coupling or a load case names a stage or a shaft the
    /// train does not have.
    NoSuchShaft { at: ShaftRef },
    /// The tooth counts along the shaft line multiply past what an exact
    /// ratio can hold, and a ratio is refused rather than wrapped.
    Overflow,
    /// **A load case enters by a shaft no load can be put on**: ground, a
    /// held shaft, a shaft the train does not have, or one of a stage's
    /// shafts that is not a port — a planet's. Zero-based, as the cases are
    /// indexed; the front end numbers from 1.
    LoadPort { case: usize },
    /// **A load case enters between two stages, and could leave by either
    /// end.** How such a load divides is a statement about what holds it at
    /// each end — the rowspace of the shaft line, with a loss model that
    /// follows power mesh by mesh — and this model refers a load along one
    /// route with one efficiency per stage. So it says so rather than
    /// choosing an end, and the refusal is the boundary of the model, not of
    /// the mechanism.
    LoadShared { case: usize },
    /// A hula stage that has no geometry — see [`crate::hula::Error`].
    Hula(crate::hula::Error),
    /// **Which stage could not be solved**, wrapped around why.
    ///
    /// A train is a chain: a stage that fails takes the shaft line with it, so
    /// every stage after it has no speed or torque to be solved at and the
    /// second pass — which rates *every* stage against the efficiencies
    /// downstream — cannot run at all. So the whole train has no answer, and
    /// the least a reader is owed is which of its stages is the one to look at.
    InStage {
        /// Zero-based, as the stages are indexed; the front end numbers from 1.
        stage: usize,
        cause: Box<TrainError>,
    },
}

/// So a hula stage can report its findings in the vocabulary every other stage
/// kind reports in. It had a refusal type of its own, from before the train had
/// an arm to put one in; now that it has, the arrangement's own error is a
/// [`TrainError`] like a mesh's, and the stage returns the same `Result` as
/// every other solver here — which is what lets it name a material the library
/// does not have, or a member too undercut to rate, without inventing a second
/// place to say so.
/// So a train's motion can refuse in the vocabulary a train refuses in.
///
/// A motion failure is never geometric — that is the whole point of the split —
/// so the only things it can carry are a stage whose wiring is not a mechanism
/// and a train with no stages.
impl From<MotionError> for TrainError {
    fn from(e: MotionError) -> Self {
        match e {
            MotionError::Empty => Self::Empty,
            MotionError::Wiring(stage, cause) => Self::InStage {
                stage,
                cause: Box::new(Self::Wiring(cause)),
            },
            MotionError::NoSuchShaft(at) => Self::NoSuchShaft { at },
            MotionError::Conflicts(at) => Self::Overdetermined { at },
            MotionError::Overflow => Self::Overflow,
        }
    }
}

/// So a kind's wiring can refuse in the vocabulary every stage refuses in.
impl From<WiringError> for TrainError {
    fn from(e: WiringError) -> Self {
        Self::Wiring(e)
    }
}

impl From<crate::hula::Error> for TrainError {
    fn from(e: crate::hula::Error) -> Self {
        Self::Hula(e)
    }
}

/// A note about one shaft, carrying where it is as the front end counts —
/// stage and shaft from one — or `0` and `0` for ground, which every stage
/// shares and no stage numbers.
fn located(key: &'static str, at: ShaftRef) -> Note {
    let (stage, shaft) = match at {
        ShaftRef::Ground => (0, 0),
        ShaftRef::Of { stage, shaft } => (stage + 1, shaft),
    };
    Note::new(key)
        .text("stage", stage.to_string())
        .text("shaft", shaft.to_string())
}

impl crate::note::Explain for TrainError {
    /// Why the stage could not be solved, as a key and its values.
    ///
    /// The nested cases delegate rather than restating: a mesh that will not
    /// mesh says so once, wherever it is asked.
    fn note(&self) -> crate::note::Note {
        use crate::note::{key, Note};
        match self {
            Self::Mesh(e) => e.note(),
            // The drive diagnoses itself; the train carries the note rather
            // than restating it.
            Self::Hula(e) => e.note(),
            Self::Screw(e) => e.note(),
            Self::NoContact => Note::new(key::ERROR_TRAIN_NO_CONTACT),
            Self::NoCommonDistance => Note::new(key::ERROR_TRAIN_NO_COMMON_DISTANCE),
            Self::NoPowerFlow => Note::new(key::ERROR_TRAIN_NO_POWER_FLOW),
            // A stage whose boundary leaves it more than one motion is not a
            // stage that describes no mechanism, and the two used to share a
            // sentence.
            Self::Wiring(WiringError::Unsolvable) => Note::new(key::ERROR_TRAIN_STAGE_UNDETERMINED),
            Self::Wiring(_) => Note::new(key::ERROR_TRAIN_WIRING),
            Self::UnknownMaterial(n) => {
                Note::new(key::ERROR_TRAIN_UNKNOWN_MATERIAL).text("name", n.clone())
            }
            Self::NoRootSection => Note::new(key::ERROR_TRAIN_NO_ROOT_SECTION),
            Self::Empty => Note::new(key::ERROR_TRAIN_EMPTY),
            Self::Underdetermined { short } => {
                Note::new(key::ERROR_TRAIN_UNDERDETERMINED).text("short", short.to_string())
            }
            Self::Overdetermined { at } => located(key::ERROR_TRAIN_OVERDETERMINED, *at),
            Self::NoSuchShaft { at } => located(key::ERROR_TRAIN_NO_SUCH_SHAFT, *at),
            Self::Overflow => Note::new(key::ERROR_TRAIN_OVERFLOW),
            Self::LoadPort { case } => {
                Note::new(key::ERROR_TRAIN_LOAD_PORT).text("case", (case + 1).to_string())
            }
            Self::LoadShared { case } => {
                Note::new(key::ERROR_TRAIN_LOAD_SHARED).text("case", (case + 1).to_string())
            }
            // The stage number belongs to the reader rather than to the reason,
            // so the note is the cause's and the number reaches the front end
            // through the error's own shape.
            Self::InStage { cause, .. } => cause.note(),
        }
    }
}

/// A shaft as the harness writes it: one-based stage and the shaft's index.
fn place(at: ShaftRef) -> String {
    match at {
        ShaftRef::Ground => "ground".to_string(),
        ShaftRef::Of { stage, shaft } => format!("stage {}, shaft {shaft}", stage + 1),
    }
}

/// English, for the CLI and for `Debug`. **Not** what the browser renders — see
/// [`TrainError::note`].
impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mesh(e) => write!(f, "{e}"),
            Self::Hula(e) => write!(f, "the arrangement has no geometry: {e:?}"),
            Self::Screw(e) => match e {
                crate::screw::ScrewError::NotPositive => {
                    write!(f, "a module, diameter or tooth count is not positive")
                }
                crate::screw::ScrewError::WormTooThin => write!(
                    f,
                    "the first member has no lead angle: given a diameter, the worm \
                     is too thin for that many starts at that module and its thread \
                     would have to wrap at ninety degrees or more; given a helix \
                     angle, a zero helix makes it a spur gear, which has no lead at \
                     all — put the helical member first and the spur one second"
                ),
                crate::screw::ScrewError::ShaftAngleImpossible => {
                    write!(f, "that shaft angle leaves the wheel with no lead angle")
                }
                crate::screw::ScrewError::FirstMemberIsADisc => write!(
                    f,
                    "the first member's helix angle reaches ninety degrees: its teeth \
                     would run circumferentially and its pitch diameter is unbounded, \
                     which is a disc rather than a gear"
                ),
                crate::screw::ScrewError::AxesAreParallel => write!(
                    f,
                    "parallel axes: a worm stage needs crossed shafts, and a \
                     parallel pair is a spur stage"
                ),
            },
            Self::NoContact => write!(f, "the teeth never come into contact"),
            Self::NoCommonDistance => write!(
                f,
                "no profile shift brings the two centre distances together; \
                 these tooth counts cannot be assembled"
            ),
            Self::Wiring(e) => match e {
                WiringError::MemberWithoutTeeth(i) => {
                    write!(f, "member {} has no teeth", i + 1)
                }
                WiringError::NoCommonFrame(k) => write!(
                    f,
                    "mesh {}'s two members' axes are not fixed in one frame, so \
                     they cannot stay a fixed distance apart",
                    k + 1
                ),
                WiringError::NotAMesh(k) => {
                    write!(f, "mesh {} is not a mesh this stage has", k + 1)
                }
                WiringError::Unsolvable => write!(
                    f,
                    "with what the train holds and drives, this stage has more than one \
                     free shaft, and a rating wants one motion to rate under"
                ),
            },
            Self::NoPowerFlow => write!(
                f,
                "no self-consistent power flow: the arrangement is self-locking, \
                 or the shaft named as the input is not the one driving"
            ),
            Self::UnknownMaterial(n) => write!(f, "no material named {n:?} in the library"),
            Self::NoRootSection => write!(f, "the tooth is too undercut to have a root section"),
            Self::Empty => write!(f, "the geartrain has no stages"),
            Self::Underdetermined { short } => write!(
                f,
                "the train is {short} condition(s) short of one motion: hold or drive \
                 another shaft, or read the family the graph reports"
            ),
            Self::Overdetermined { at } => {
                write!(f, "two conditions cannot both hold at {}", place(*at))
            }
            Self::NoSuchShaft { at } => write!(f, "no such shaft: {}", place(*at)),
            Self::Overflow => write!(
                f,
                "the tooth counts along the shaft line multiply past what an exact ratio holds"
            ),
            Self::LoadPort { case } => write!(
                f,
                "load case {}: enters by a shaft no load can be put on — ground, a held \
                 shaft, or one that is not a port",
                case + 1
            ),
            Self::LoadShared { case } => write!(
                f,
                "load case {}: enters between two stages and could leave by either end; \
                 this model refers a load along one route",
                case + 1
            ),
            Self::InStage { stage, cause } => write!(f, "stage {}: {cause}", stage + 1),
        }
    }
}

impl std::error::Error for TrainError {}

/// A self-contained material library for tests, so `gear-core` keeps no
/// dependency on `gear-io`. Shared by every stage kind's tests.
#[cfg(test)]
pub(super) fn test_library() -> MaterialLibrary {
    use crate::material::{Basis, Family, Measure, Value};
    let steel = Material {
        name: "4340 Hardened Steel".into(),
        class: Family::Steel,
        grade: "test".into(),
        condition: "test".into(),
        source: "test".into(),
        density: Value::datasheet(7850.0),
        elastic_modulus: Value::datasheet(190_000.0),
        poissons_ratio: Value::datasheet(0.29),
        ultimate_allowable: Value::datasheet(1365.0),
        ultimate_measure: Measure::Yield,
        fatigue_allowable: Value {
            value: 750.0,
            basis: Basis::Estimated,
            note: Some("test".into()),
        },
    };
    let bronze = Material {
        name: "Brass C360".into(),
        class: Family::Brass,
        elastic_modulus: Value::datasheet(97_000.0),
        poissons_ratio: Value::datasheet(0.321),
        ultimate_allowable: Value::datasheet(310.0),
        fatigue_allowable: Value {
            value: 140.0,
            basis: Basis::Estimated,
            note: Some("test".into()),
        },
        ..steel.clone()
    };
    MaterialLibrary {
        materials: vec![steel, bronze],
    }
}

/// **What a stage is asked to optimise, and what it may not do to get there.**
///
/// Every stage with shifts to choose carries the same two decisions, and they
/// were the same two fields written out three times — which is three places to
/// edit, three serde defaults to keep in step, and a fourth stage away from
/// being four. The searches differ in what is free and what it is worth; this
/// does not differ at all.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(feature = "serde", serde(default))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Optimisation {
    /// **Choose the automatic shifts for efficiency rather than for undercut.**
    ///
    /// Off, so a stage answers as it always has. On, the shifts a designer has
    /// left automatic are chosen to make the stage lose least, with the undercut
    /// shift as a *floor* rather than as the answer
    /// (docs/reference.md#efficiency-parallel-axes).
    ///
    /// What is already given constrains it rather than being overruled by it: a
    /// manual shift is that gear's, and a manual centre distance or crank offset
    /// fixes a shift sum. Enough of them leave nothing to choose, which is a
    /// design fully specified rather than an error.
    pub enabled: bool,
    /// **The transverse contact ratio the optimiser may not go below.**
    ///
    /// Sliding loss falls monotonically with the length of the path, so the
    /// least-loss pair is always the one whose teeth barely reach: this is the
    /// constraint that answers rather than the optimum, which is why it is an
    /// input and not a constant. 1.2 is the usual design minimum; a mesh of one
    /// tooth of difference sits just above continuous contact at every shift it
    /// can be built at, and asks for less.
    ///
    /// It bounds the *optimiser* only. A design specified by hand is reported as
    /// it is, with the existing note below 1.
    pub min_contact_ratio: f64,
}

impl Default for Optimisation {
    fn default() -> Self {
        Self {
            enabled: false,
            min_contact_ratio: 1.2,
        }
    }
}

/// A stage of a geartrain, of whichever kind.
///
/// Serialised with a `kind` tag alongside the stage's own fields, so a train
/// file says what each stage is rather than relying on position.
///
/// **Two of the kinds are one shape.** `Spur` and `Worm` both carry a
/// [`PairStage`]; the kind is the layer a designer sees — a preset, a
/// vocabulary, and which inputs are put in front of them — over one model
/// ([`PairKind`]). It is the tag and nothing else that says which, which is
/// what lets a worm stage's file say `kind = "worm"` and a front end say
/// *starts* and *wheel* without the core holding a second stage type.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum Stage {
    Spur(PairStage),
    Worm(PairStage),
    // Boxed for the same reason `StageResult`'s variants are: a planetary stage
    // carries three gears, a cutter and an arrangement where a spur stage carries
    // two gears, so a `Vec<Stage>` would otherwise pay the largest of them for
    // every stage whatever its kind. Invisible to readers and to serde.
    Planetary(Box<PlanetaryStage>),
    Hula(Box<HulaStage>),
}

impl Default for Stage {
    fn default() -> Self {
        Self::Spur(PairStage::default())
    }
}

impl Stage {
    /// The pair this stage is, with its kind, where it is one.
    #[must_use]
    pub fn as_pair(&self) -> Option<(&PairStage, PairKind)> {
        match self {
            Self::Spur(p) => Some((p, PairKind::Spur)),
            Self::Worm(p) => Some((p, PairKind::Worm)),
            _ => None,
        }
    }
}

/// **Whether a stage's shift chooser actually chose**, and what it means when
/// the shifts come back where they started.
///
/// A designer who turns *optimise for efficiency* on and sees no shift move is
/// owed the reason, and there are two of them that look identical:
///
/// - the search ran and **agreed** — the optimum is on the floor the stage
///   already sits at, which is the ordinary answer wherever loss falls
///   monotonically toward the shortest admissible path; or
/// - the search ran and found **nothing admissible at all**, so there was no
///   answer to choose and the stage kept what it had.
///
/// The first is the tool working. The second is a design with no room in it, and
/// it was silent on every kind — the audit's record (`docs/history/audit.md`,
/// F58) measured both ends of it on one
/// stage before this existed to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Searched {
    /// The optimiser was off, or every freedom was given by hand. Nothing was
    /// asked, so nothing failing to move says anything.
    NotAsked,
    /// It ran and chose. The answer may still equal the floor; that is the
    /// search agreeing rather than the search failing.
    Chose,
    /// It ran and **nothing in the admissible range could be built**. These
    /// shifts are the ones the stage takes unasked.
    FoundNothing,
}

/// A chooser's answer, and how it arrived.
pub(crate) struct Chosen<const N: usize> {
    pub shifts: [f64; N],
    pub how: Searched,
}

impl Searched {
    /// The note this deserves, if any — so no kind has to remember the wording
    /// or which of the three states is worth saying out loud.
    pub(crate) fn note(self) -> Option<Note> {
        (self == Self::FoundNothing).then(|| Note::new(key::STAGE_OPTIMISER_FOUND_NOTHING))
    }
}

/// **What a stage has to say about the distance it ended up running at**, if
/// anything.
///
/// Two findings, and both were silent. They are here rather than in each stage's
/// own file because every kind that has a centre distance can reach them, and a
/// rule one kind asks is a rule the others forget — which `docs/corrections.md`
/// records happening to the tip-room flags and to the axial-overlap warning.
///
/// # The distance was given and the shifts could not reach it
///
/// Mode 3 says a given distance and a given clearance decide the shifts. Where
/// no admissible shifts reach that distance the stage still answers — at the
/// shifts it would have built anyway — and used to say nothing at all, so a
/// designer read a stage that was **not** running where they put it and had only
/// the clearance readout to notice by.
///
/// # The running distance is inside the zero-backlash one
///
/// A negative clearance is a pair whose teeth overlap at rest: it cannot be
/// assembled, and every figure computed at that distance describes nothing. A
/// 9/37 pair told to run at 23.00 mm, whose shifts put it at 23.4433, reported
/// **−0.4433 mm** of clearance and not one word. "Inside" is the mesh's own
/// reading — an internal pair overlaps when its centres are too far *apart* —
/// so the finding is asked of the clearance rather than of which distance is
/// the larger.
///
/// Notes rather than refusals, on rule 5's reading: the gears exist and are
/// cuttable, so what is wrong is the *assembly*, and a designer is owed the
/// number rather than an error. `asked` is `None` where no distance was given,
/// which is the case neither finding can arise in.
pub(crate) fn distance_notes(target: Option<f64>, nominal: f64, clearance: f64) -> Vec<Note> {
    // A hundredth of a micron. Reaching the target is a solve, so agreement is
    // near machine precision and a failure is gross — the 9/37 pair below misses
    // by 0.44 mm. This separates the two, and is not a tolerance on an answer.
    const REACHED: f64 = 1e-6;

    let mut out = Vec::new();
    // `target` is the **nominal** distance mode 3 asked the shifts for — the
    // distance given with the clearance given taken out of it, in the mesh's
    // own direction — and `None` where the stage was not in mode 3 at all,
    // which is the case this cannot arise in. `clearance` is the gap the stage
    // actually left, signed the way the mesh reads it: positive is play on
    // either kind, so a negative one is teeth overlapping at rest whichever way
    // the centres moved to get there.
    if let Some(target) = target {
        if (nominal - target).abs() > REACHED {
            out.push(
                Note::new(key::STAGE_CENTRE_DISTANCE_NOT_REACHED)
                    .number("asked", target, 4)
                    .number("reached", nominal, 4),
            );
        }
    }
    if clearance < 0.0 {
        out.push(
            Note::new(key::STAGE_CLEARANCE_NEGATIVE)
                .number("overlap", -clearance, 4)
                .number("nominal", nominal, 4),
        );
    }
    out
}

/// One of a stage's constrainable inputs, named so a caller can find it.
///
/// Two levels, because a stage has inputs of two kinds: its own — a distance,
/// a clearance, a ratio, a worm's diameter — and one of each per member.
/// `Member(i, _)` indexes the members in the order [`StageResult::members`]
/// reports them — a pair's two gears, a set's sun, planet and ring, a hula
/// stage's four — and is resolved once for every kind ([`member_inputs`]), so
/// a member input that arrives costs one line there and none per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Freedom {
    /// The distance the stage's members run at.
    CentreDistance,
    /// What portion of that distance is running play.
    Clearance,
    /// A pair's first member's pitch diameter — the same freedom as its helix
    /// read as a size, which is a worm's reading.
    FirstPitchDiameter,
    /// The stage's axial contact ratio, which relates the helix to the face
    /// width the mesh carries: given with every width given, it decides the
    /// helix; given with a width automatic, it is a floor under that width.
    Overlap,
    /// One member's own input.
    Member(usize, MemberFreedom),
}

/// One of a member's constrainable inputs.
///
/// A helix here is one reading of **the size of the stage's teeth along the
/// axis**: every member's is bound to the others', so a pair's first member
/// is as big as its helix makes it, which is what absorbs a centre distance
/// once both shifts are pinned.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum MemberFreedom {
    /// The profile shift.
    Shift,
    /// The helix angle.
    Helix,
    /// The face width.
    FaceWidth,
}

/// **Every constrainable input a run of members has**, by name — the one
/// place a [`MemberFreedom`] meets the field it names, for every kind at once.
pub(crate) fn member_inputs<'a>(
    gears: impl IntoIterator<Item = &'a mut StageGear>,
) -> Vec<(Freedom, &'a mut Auto<f64>)> {
    gears
        .into_iter()
        .enumerate()
        .flat_map(|(i, g)| {
            let StageGear {
                profile_shift,
                helix_angle,
                face_width,
                ..
            } = g;
            [
                (Freedom::Member(i, MemberFreedom::Shift), profile_shift),
                (Freedom::Member(i, MemberFreedom::Helix), helix_angle),
                (Freedom::Member(i, MemberFreedom::FaceWidth), face_width),
            ]
        })
        .collect()
}

/// **What one input came to**, by the name relief knows it by — the number a
/// box turned given by relief is seeded with, so a designer holds what they
/// were shown rather than a stale zero.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Figure {
    pub freedom: Freedom,
    /// `None` where the result carries no such figure — a crossed pair's
    /// overlap — and the box keeps what it had.
    pub value: Option<f64>,
}

/// **One way of stating a stage's helix**, and what it states.
///
/// The helix is one number a stage's members share — `β₂ = Σ − β₁` on a pair,
/// the hand flipped across an external mesh in a set, one angle on a hula
/// stage's four — so a kind has several inputs that all say it: each member's
/// helix, a pair's first pitch diameter (`d = z m_n / cos β`), and the axial
/// contact ratio where every face is given and the ratio can be turned into a
/// helix at the width the mesh carries. Those are *readings* of one freedom,
/// and at most one of them stands.
///
/// A kind lists its readings in **relief order, least precious first**, and
/// the solve honours the **last** one given ([`stated_helix`]). So the reading
/// relief leaves standing is the reading the solve reads, by construction —
/// there is no second list stating the precedence again in an `if` chain, and
/// nothing for the two to disagree about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub freedom: Freedom,
    /// The first member's helix, degrees, as this reading states it. `None`
    /// where the input is automatic — or given and reaching nothing, which a
    /// ratio no helix reaches at the given width is, and the stage says so
    /// ([`overlap_notes`]).
    pub helix: Option<f64>,
}

impl Reading {
    /// A reading from a member's helix box, `to_first` turning what it states
    /// into the first member's angle.
    pub(crate) fn helix(i: usize, gear: &StageGear, to_first: impl FnOnce(f64) -> f64) -> Self {
        Self {
            freedom: Freedom::Member(i, MemberFreedom::Helix),
            helix: (!gear.helix_angle.auto).then(|| to_first(gear.helix_angle.manual)),
        }
    }

    /// The ratio's reading, where it is given the size to decide.
    pub(crate) fn overlap(overlap: &Auto<f64>, module: f64, width: f64) -> Self {
        Self {
            freedom: Freedom::Overlap,
            helix: (!overlap.auto)
                .then(|| helix_for_overlap(overlap.manual, module, width))
                .flatten(),
        }
    }
}

/// The helix the readings state, degrees of the first member — the last
/// reading given, which is the one relief leaves standing — or `None` where
/// every reading is automatic and something else decides.
pub(crate) fn stated_helix(readings: &[Reading]) -> Option<f64> {
    readings.iter().rev().find_map(|r| r.helix)
}

/// **The face width an axial contact ratio needs**, mm — `ε_β π m_n / sin |β|`
/// — where the ratio is given and the helix can carry one. `None` where the
/// ratio is automatic or the teeth are straight, which no width makes helical.
///
/// One home for the relation every kind with a line contact asks, read as a
/// width: the ask an automatic face width adds to its ratings'.
pub(crate) fn width_for_overlap(overlap: &Auto<f64>, helix_deg: f64, module: f64) -> Option<f64> {
    let sin = helix_deg.to_radians().sin().abs();
    (!overlap.auto && sin > 0.0).then(|| overlap.manual * std::f64::consts::PI * module / sin)
}

/// **What a given ratio owes its reader when it could not be read.** Given to
/// decide the helix (`taken`) and reaching none at the width the mesh carries,
/// the helix stood at its box; given as a floor and the teeth straight, no
/// width buys any overlap and the floor is nothing. Either is an input the
/// solve could not honour, said rather than swallowed. One home, for every
/// kind that reads a ratio.
pub(crate) fn overlap_notes(
    taken: bool,
    overlap: &Auto<f64>,
    module: f64,
    width: f64,
    helix_deg: f64,
) -> Vec<Note> {
    let mut out = Vec::new();
    if overlap.auto {
        return out;
    }
    if taken && helix_for_overlap(overlap.manual, module, width).is_none() {
        out.push(Note::new(key::STAGE_OVERLAP_UNREACHABLE).number("ratio", overlap.manual, 3));
    }
    if !taken && helix_deg == 0.0 {
        out.push(Note::new(key::STAGE_OVERLAP_NEEDS_HELIX).number("ratio", overlap.manual, 3));
    }
    out
}

/// **The helix an axial contact ratio needs**, degrees, at a face width the
/// mesh carries — the same relation read the other way, for a stage whose
/// widths are all given and whose ratio is. `None` where no helix reaches it:
/// `ε_β π m_n / b` above 1 asks for a tooth wrapped past a right angle.
pub(crate) fn helix_for_overlap(overlap: f64, module: f64, width: f64) -> Option<f64> {
    let sin = overlap * std::f64::consts::PI * module / width;
    (width > 0.0 && (0.0..=1.0).contains(&sin)).then(|| sin.asin().to_degrees())
}

/// **A set of inputs bound by one relation, and how many of them may be given.**
///
/// A geometric relation among `n` inputs leaves `n − 1` of them free, so giving
/// `n` is not a tighter specification — it is a contradiction, and one of the
/// numbers would have to be ignored. This says which inputs are in that
/// argument and how many may stand.
///
/// # Why the *stage* declares this rather than the front end
///
/// It was three functions in TypeScript, one per stage kind, each restating a
/// relation the core already enforces. That is two faults at once: an
/// engineering rule written outside Rust, and the same idea written once per
/// kind — so a fifth kind arrives with no relief at all, and a rule that changes
/// changes in one of four places. It is also untestable there, and was untested.
///
/// The stage kinds genuinely differ in *what* is related, which is why this is a
/// declaration and not a constant: a pair relates its distance to its two
/// shifts, an epicyclic set relates its three shifts to each other through the
/// two distances that must agree, and a hula stage relates each mesh's pair
/// separately because its crank fixes their difference one mesh at a time.
///
/// What does **not** differ is the resolution: too many given means the first
/// one in `order` that the designer is not this moment touching goes back to
/// automatic. Least precious first, and stated by the stage.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct FreedomGroup {
    /// How many of `order` may be **given** before the set is over-determined.
    pub given_at_most: usize,
    /// How many of `order` may be **automatic** before it is under-determined.
    ///
    /// The mirror of `given_at_most`, and it is needed for a real case rather
    /// than for symmetry: a centre distance is *nominal + clearance* and an
    /// automatic clearance is *distance − nominal*, so with both automatic
    /// neither has anything to derive from. One of the two has to be a number
    /// somebody gave.
    ///
    /// A kind with **no** distance input would say `0` here — its clearance
    /// could never be derived, because there is nothing to derive it from. That
    /// is the same statement, counted; every kind has the input now, so none
    /// says it.
    pub automatic_at_most: usize,
    /// The inputs in the argument, **relief order, least precious first**.
    ///
    /// **An entry is one input, stated one or more ways.** A pair's size is
    /// either helix or the first pitch diameter — one freedom, three boxes —
    /// so the entry that names it lists all three, and the relation counts it
    /// once: given while any reading is, automatic while all are. Within an
    /// entry at most one reading stands, relieved in the same order, so the
    /// exclusivity of the readings is not a second group with a count to keep
    /// in step with this one.
    ///
    /// One order serves both directions: too many given turns the first entry
    /// that is not being touched automatic, and too many automatic pins the
    /// first that is not being touched — by its last reading, the one the
    /// solve reads first. The same walk, read the other way.
    pub order: Vec<Vec<Freedom>>,
}

/// **A single relation** among its inputs, so exactly one of them is the one
/// the others decide. Written once rather than as a count per kind, because a
/// count per kind is a count to get wrong — and the first draft did, by one,
/// on the kind with the most tests.
///
/// Says nothing about how many may be *automatic*: every input in one of these
/// has a rule of its own to fall back on, so leaving them all automatic is a
/// design with nothing pinned rather than a contradiction.
pub(crate) fn one_relation(order: Vec<Vec<Freedom>>) -> FreedomGroup {
    FreedomGroup {
        given_at_most: order.len() - 1,
        automatic_at_most: order.len(),
        order,
    }
}

/// **Two ways of saying one number, so one of them must be said.** A distance
/// is nominal + clearance and an automatic clearance is distance − nominal;
/// with both automatic neither has anything to derive from. A pair's group;
/// the two epicyclic kinds cannot derive a clearance at all and say so with
/// [`always_given`].
pub(crate) fn distance_and_clearance() -> FreedomGroup {
    FreedomGroup {
        given_at_most: 2,
        automatic_at_most: 1,
        order: vec![vec![Freedom::Clearance], vec![Freedom::CentreDistance]],
    }
}

/// **An input that is always given**, alone in its group with none allowed
/// automatic, so relief pins it back whatever was touched — the tool saying
/// that nothing can hand it back, rather than offering a toggle and reading
/// the box regardless.
pub(crate) fn always_given(f: Freedom) -> FreedomGroup {
    FreedomGroup {
        given_at_most: 1,
        automatic_at_most: 0,
        order: vec![vec![f]],
    }
}

/// **An input that is never read**, the mirror of [`always_given`]: relief
/// turns it automatic whatever was touched, so a box the solve would
/// disregard is not left showing a number as if it were being read.
pub(crate) fn always_automatic(f: Freedom) -> FreedomGroup {
    FreedomGroup {
        given_at_most: 0,
        automatic_at_most: 1,
        order: vec![vec![f]],
    }
}

/// The readings as one entry, for a group to count as one input.
pub(crate) fn entry(readings: &[Reading]) -> Vec<Freedom> {
    readings.iter().map(|r| r.freedom).collect()
}

/// **The readings of the helix as a group of their own**, for a kind whose
/// size is in no other relation: nothing counts it against a distance, so all
/// the group holds is that at most one reading stands.
pub(crate) fn readings_group(readings: &[Reading]) -> FreedomGroup {
    FreedomGroup {
        given_at_most: 1,
        automatic_at_most: 1,
        order: vec![entry(readings)],
    }
}

/// **What a kind declares so the machinery shared by every kind can serve it**
/// — the whole of what a new kind owes.
///
/// Six questions: which members it has, which inputs relief may turn and by
/// what name, how its helix may be stated, which of its inputs argue with each
/// other, **where its shafts and meshes sit**, and **which shafts a train may
/// address** and what convention holds when it addresses none. Everything
/// that walks those — counting, relieving, seeding a box, reading the helix
/// the readings state, assembling the kinematic system, laying a train's
/// constraints over the convention — is written once above the kinds, so a
/// kind that answers the six has all of it without writing any.
///
/// The fifth and sixth are the newest, and they are the ones that test the
/// claim `docs/rationale.md#each-stage-kind-keeps-its-own-result-type` makes
/// and calls untested: that a new kind should be new **kinematics** and no new
/// rating machinery. A kind states its topology here and the one solver in
/// [`crate::kinematics`] answers every question about motion, torque and play
/// that used to be answered per kind; it states its ports here and the train's
/// [`conditions`] decide which is driven and which
/// held, which used to be a field on the kind.
pub(crate) trait Constrained {
    /// The members in the order [`StageResult::members`] reports them.
    fn members(&self) -> Vec<&StageGear>;
    /// Every input relief may turn, by name — the stage's own and, through
    /// [`member_inputs`], each member's.
    fn inputs(&mut self) -> Vec<(Freedom, &mut Auto<f64>)>;
    /// The readings of the helix, relief order, least precious first.
    fn readings(&self) -> Vec<Reading>;
    /// Every argument this kind's inputs can get into with each other.
    fn freedoms(&self) -> Vec<FreedomGroup>;
    /// **Where this kind's shafts and meshes sit** — topology alone, with no
    /// module, no shift and no distance in it. See [`Wiring`].
    fn wiring(&self) -> Wiring;
    /// **The kind's conventional ports and what it holds by convention** — what
    /// a chain is built from and a lone stage is solved under, and nothing a
    /// train's own constraints cannot override. See [`Ports`].
    fn ports(&self) -> Ports;
}

impl Stage {
    fn kind(&self) -> &dyn Constrained {
        match self {
            Self::Spur(s) | Self::Worm(s) => s,
            Self::Planetary(p) => &**p,
            Self::Hula(h) => &**h,
        }
    }

    fn kind_mut(&mut self) -> &mut dyn Constrained {
        match self {
            Self::Spur(s) | Self::Worm(s) => s,
            Self::Planetary(p) => &mut **p,
            Self::Hula(h) => &mut **h,
        }
    }

    /// **The input a [`Freedom`] names**, where this kind has it. `None` is the
    /// same thing [`Self::freedoms`] says by not mentioning it.
    pub(crate) fn input_mut(&mut self, f: Freedom) -> Option<&mut Auto<f64>> {
        self.kind_mut()
            .inputs()
            .into_iter()
            .find_map(|(g, a)| (g == f).then_some(a))
    }

    /// Whether the input a freedom names is given. An input this kind does
    /// not have is neither given nor automatic.
    fn is_given(&mut self, f: Freedom) -> bool {
        self.input_mut(f).is_some_and(|a| !a.auto)
    }

    /// **Every input relief may turn, and whether it is automatic** — in the
    /// one order the kind declares, so a caller lining two stages up against
    /// each other can do it by name rather than by field.
    #[must_use]
    pub fn toggles(&self) -> Vec<(Freedom, bool)> {
        // Read through a copy: the kinds hand their inputs out mutably, once,
        // and a second accessor for reading would be the same list twice.
        let mut probe = self.clone();
        probe
            .kind_mut()
            .inputs()
            .into_iter()
            .map(|(f, a)| (f, a.auto))
            .collect()
    }

    /// **The stage's members**, in the order [`StageResult::members`] reports
    /// them — the order [`Freedom::Member`] indexes.
    #[must_use]
    pub fn members(&self) -> Vec<&StageGear> {
        self.kind().members()
    }

    /// **Where this stage's shafts and meshes sit** ([`Wiring`]) — the topology
    /// the one kinematic solver is assembled from, with no geometry in it.
    #[must_use]
    pub fn wiring(&self) -> Wiring {
        self.kind().wiring()
    }

    /// **The kind's conventional ports and holds** ([`Ports`]) — what a chain
    /// is built from and a lone stage is solved under.
    #[must_use]
    pub fn ports(&self) -> Ports {
        self.kind().ports()
    }

    /// **What this stage is asked when it stands alone**: its kind's
    /// conventions, as a boundary its own solver can take.
    #[must_use]
    pub fn conventional_boundary(&self) -> StageBoundary {
        StageBoundary::conventional(&self.wiring(), &self.ports())
    }

    /// **This stage's kinematic system**, from its wiring and its tooth counts.
    ///
    /// It needs neither a module nor a shift nor a material, which is the whole
    /// point: a stage that will not close geometrically still has a ratio, and
    /// this is what can still be asked of it.
    ///
    /// # Errors
    ///
    /// [`WiringError`] for a wiring that does not describe meshes.
    pub fn system(&self) -> Result<crate::kinematics::System, WiringError> {
        self.wiring().alone(&teeth_of(self.members()))
    }

    /// **The pinion cutter a member is cut by, where it is a ring** — which is
    /// the one question *is this member internal?* has, asked of the stage
    /// that knows: a pair has no ring, a set's ring is its third member, and
    /// a hula stage's are the larger gear of each mesh, cut by that mesh's
    /// cutter. `None` for every rack-cut member, including a worm.
    #[must_use]
    pub fn member_cutter(&self, i: usize) -> Option<&crate::ring::Cutter> {
        match self {
            Self::Spur(_) | Self::Worm(_) => None,
            Self::Planetary(p) => (i == 2).then_some(&p.cutter),
            Self::Hula(h) => {
                let teeth = crate::hula::Teeth(h.gears.each_ref().map(|g| g.teeth));
                let pair = teeth.pair(i / 2).ok()?;
                (pair.ring == i).then_some(&h.cutter[i / 2])
            }
        }
    }

    /// **Every argument this stage's inputs can get into with each other.**
    ///
    /// A stage may have **more than one** group: a hula stage's crank fixes the
    /// difference of each mesh's two shifts, which is one relation per mesh
    /// rather than one for the stage. Each kind declares its own
    /// ([`Constrained::freedoms`]); what is shared is everything that reads
    /// them.
    #[must_use]
    pub fn freedoms(&self) -> Vec<FreedomGroup> {
        self.kind().freedoms()
    }

    /// **This stage with its over-determined inputs relieved.**
    ///
    /// A designer who pins a pair's distance *and* both its shifts has asked for
    /// a contradiction: the three are bound by one relation, so one of them
    /// would have to be ignored. Rather than accept an input and quietly
    /// disregard it, the first one in relief order that the designer is **not**
    /// this moment pinning goes back to automatic, visibly.
    ///
    /// `just` is the input they have this moment given, and is never the one
    /// relieved while sparing it leaves an answer; `None` where what changed
    /// was not a toggle — a shaft angle, say — and nothing is spared.
    ///
    /// # Why this is here and not in the panel
    ///
    /// It was three functions in TypeScript, one per stage kind. Rule 1 puts an
    /// engineering rule in Rust, rule 4 says one idea belongs in one place, and
    /// neither was being followed — nor was any of it tested. What decided it is
    /// that the rule is not the panel's to know: **it is the same relation the
    /// solve enforces**, read from the other end, and the two must agree or a
    /// designer is offered an input the solve will disregard.
    ///
    /// Nothing here decides a *value*. It only says which inputs are still being
    /// read, which is why it can be a pure function of the inputs.
    ///
    /// # Settled, whatever order the groups come in
    ///
    /// Groups share inputs — a pair's distance is in its relation and in the
    /// argument with its clearance — so satisfying one can hand another an
    /// input, and the walk repeats until a pass moves nothing. It is bounded
    /// by the number of toggles a pass could turn, and the declaration is
    /// checked for settling at all by the test that relief is idempotent:
    /// a declaration where satisfying one group breaks another for ever would
    /// fail there, rather than depend on the order the groups were written in.
    #[must_use]
    pub fn relieved(&self, just: Option<Freedom>) -> Self {
        let mut out = self.clone();
        let bound = out.toggles().len() + 1;
        for _ in 0..bound {
            let mut moved = false;
            for group in out.freedoms() {
                moved |= out.settle(&group, just);
            }
            if !moved {
                break;
            }
        }
        out
    }

    /// **This stage relieved, with every box relief turned given seeded from
    /// what it last showed** — the number the designer would have copied in,
    /// to the digits they saw, so a helix pinned when its neighbour was freed
    /// holds the angle it had rather than a stale zero.
    #[must_use]
    pub fn relieved_from(&self, just: Option<Freedom>, figures: &[Figure]) -> Self {
        let mut out = self.relieved(just);
        for (f, was_auto) in self.toggles() {
            let Some(a) = out.input_mut(f) else { continue };
            if was_auto && !a.auto {
                if let Some(v) = figures
                    .iter()
                    .find(|x| x.freedom == f)
                    .and_then(|x| x.value)
                {
                    a.manual = (v * 1e4).round() / 1e4;
                }
            }
        }
        out
    }

    /// One group brought within its limits, and whether anything moved.
    fn settle(&mut self, group: &FreedomGroup, just: Option<Freedom>) -> bool {
        let mut moved = false;
        // **Within an entry, one reading stands.** Least precious first, the
        // one just touched last of all — and only then, when sparing it would
        // leave two.
        for entry in &group.order {
            let mut given = entry.iter().filter(|f| self.is_given(**f)).count();
            for spare_just in [true, false] {
                for f in entry {
                    if given <= 1 {
                        break;
                    }
                    if spare_just && Some(*f) == just {
                        continue;
                    }
                    if let Some(a) = self.input_mut(*f) {
                        if !a.auto {
                            a.auto = true;
                            given -= 1;
                            moved = true;
                        }
                    }
                }
            }
        }
        // **One walk, both directions.** Too many given turns one entry
        // automatic; too many automatic pins one. Each time it is the first in
        // relief order that the designer is not this moment touching, so the
        // answer depends on the order the *stage* declares and never on the
        // order the toggles happened to be turned in.
        for wanted_auto in [false, true] {
            let limit = if wanted_auto {
                group.given_at_most
            } else {
                group.automatic_at_most
            };
            // An entry is given while any reading is, automatic while all
            // are; one the kind has no input for is neither.
            let state = |s: &mut Self, entry: &[Freedom]| -> Option<bool> {
                let present: Vec<bool> = entry
                    .iter()
                    .filter_map(|f| s.input_mut(*f).map(|a| a.auto))
                    .collect();
                (!present.is_empty()).then(|| present.iter().all(|auto| *auto))
            };
            let mut over = group
                .order
                .iter()
                .filter(|e| state(self, e) == Some(!wanted_auto))
                .count();
            // **`just` is a preference; the relation is a law.** Two passes:
            // the first spares the input the designer is this moment
            // touching, and the second — reached only when sparing it leaves
            // the group unsatisfiable — does not.
            //
            // The planetary set reaches the second pass: its clearance is
            // alone in its group with none allowed automatic, because it is
            // the amount its two zero-backlash distances differ by and
            // nothing else can hand it back, and the toggle snapping back is
            // the tool saying so.
            for spare_just in [true, false] {
                for entry in &group.order {
                    if over <= limit {
                        break;
                    }
                    if spare_just && entry.iter().any(|f| Some(*f) == just) {
                        continue;
                    }
                    if state(self, entry) != Some(!wanted_auto) {
                        continue;
                    }
                    if wanted_auto {
                        for f in entry {
                            if let Some(a) = self.input_mut(*f) {
                                a.auto = true;
                            }
                        }
                    } else {
                        for f in entry.iter().rev() {
                            if let Some(a) = self.input_mut(*f) {
                                a.auto = false;
                                break;
                            }
                        }
                    }
                    over -= 1;
                    moved = true;
                }
            }
        }
        moved
    }
}

/// What a stage produced, of whichever kind.
///
/// **Each shape keeps its own result** — a pair's, a set's, a hula stage's —
/// and the two pair kinds share one. What the train needs from any of them is
/// small enough to read through the accessors below — ratio, efficiency, the
/// backlash at the output member — so the accumulation never asks what kind it
/// was, without every result having to pretend to be the same shape.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum StageResult {
    // Both variants are boxed. A result carries material records, admissible
    // ranges and notes, so the kinds differ in size by an order of magnitude and
    // will keep doing so as more arrive; a `Vec<StageResult>` would otherwise
    // pay the largest of them for every stage whatever its kind. The boxes are
    // invisible to readers and to serde.
    Pair(Box<PairResult>),
    Planetary(Box<PlanetaryResult>),
    Hula(Box<HulaResult>),
}

impl StageResult {
    /// `z₂/z₁`.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Pair(r) => r.ratio,
            Self::Planetary(r) => r.ratio,
            Self::Hula(r) => r.ratio,
        }
    }

    /// Mesh efficiency, both directions.
    ///
    /// A parallel-axis stage puts the same number in both and a worm does not;
    /// the train does not have to know which is which. It takes `.forward` to
    /// propagate torque, and reports the pair.
    #[must_use]
    pub fn efficiency(&self) -> Directional<f64> {
        match self {
            Self::Pair(r) => r.mesh.efficiency,
            Self::Planetary(r) => r.efficiency,
            Self::Hula(r) => r.efficiency,
        }
    }

    /// Angular backlash at whichever member is the *output* in each direction.
    ///
    /// The same tooth gap seen from two lever arms: it subtends a larger angle
    /// at the smaller member, so a pair with different tooth counts genuinely
    /// reports two different numbers.
    #[must_use]
    pub fn backlash(&self) -> Directional<Backlash> {
        match self {
            Self::Pair(r) => r.mesh.backlash_by_drive(),
            Self::Planetary(r) => r.backlash,
            Self::Hula(r) => r.backlash,
        }
    }

    /// **Every member of this stage that is a gear**, whatever kind it is.
    ///
    /// The kind-independent accessors beside this one — [`Self::ratio`],
    /// [`Self::efficiency`], [`Self::backlash`] — say what every stage has. This
    /// says what every *member* has, and it is the one that was missing: a sweep
    /// over "every number every member reports" had to know the five kinds and
    /// name their fields, which is how a formula comes to be written five times
    /// and one of them to be wrong (`docs/corrections.md`, and F30 of the audit
    /// that added this).
    ///
    /// A worm and its wheel are here too: in this model both are involute
    /// helicoids on cylinders — a worm is a helical gear with a few starts at a
    /// steep helix — so both carry a tooth form, a shift, and every question a
    /// gear can be asked. A worm stage used to contribute nothing, on the
    /// reading that a thread is not a gear; that was a separate stage type
    /// speaking, not the model.
    #[must_use]
    pub fn members(&self) -> Vec<&GearResult> {
        match self {
            Self::Pair(r) => r.gears.iter().collect(),
            Self::Planetary(r) => vec![&r.sun, &r.planet.gear, &r.ring],
            Self::Hula(r) => r.gears.iter().map(|g| &g.gear).collect(),
        }
    }

    /// **What one of the stage's inputs came to**, by the name relief knows
    /// it by — the one place a [`Freedom`] meets the figure a result carries
    /// for it, so a box relief turns given can be seeded from what it showed
    /// without the panel knowing which field that is. `None` where the result
    /// has no such figure: a crossed pair has no overlap.
    #[must_use]
    pub fn figure(&self, f: Freedom) -> Option<f64> {
        match f {
            Freedom::CentreDistance => Some(match self {
                Self::Pair(r) => r.centre_distance,
                Self::Planetary(r) => r.centre_distance,
                Self::Hula(r) => r.offset,
            }),
            Freedom::Clearance => Some(match self {
                Self::Pair(r) => r.clearance,
                Self::Planetary(r) => r.clearance,
                Self::Hula(r) => r.running_clearance,
            }),
            Freedom::FirstPitchDiameter => self.members().first().map(|g| g.pitch_diameter),
            Freedom::Overlap => match self {
                Self::Pair(r) => r.mesh.line.as_ref().map(|l| l.contact_ratios.overlap),
                Self::Planetary(r) => Some(r.overlap),
                Self::Hula(r) => Some(r.overlap),
            },
            Freedom::Member(i, m) => self.members().get(i).map(|g| match m {
                MemberFreedom::Shift => g.profile_shift,
                MemberFreedom::Helix => g.helix_angle,
                MemberFreedom::FaceWidth => g.face_width,
            }),
        }
    }

    /// **Every mesh this stage has**, in the order it built them.
    ///
    /// The companion of [`Self::members`], and for the same reason: a question
    /// about *a mesh* — is contact continuous, do the tips foul, how much play
    /// is there — is asked of a stage by asking each of its meshes, and a walk
    /// that names the kinds is a walk that forgets one. A pair has one, either
    /// epicyclic kind has two — and a crossed pair has one like any other,
    /// which reports a point contact in the same type.
    #[must_use]
    pub fn meshes(&self) -> Vec<&MeshReport> {
        match self {
            Self::Pair(r) => vec![&r.mesh],
            Self::Planetary(r) => vec![&r.sun_planet, &r.planet_ring],
            Self::Hula(r) => r.meshes.iter().map(|m| &m.report).collect(),
        }
    }

    /// The hula result, if that is what this is.
    #[must_use]
    pub fn as_hula(&self) -> Option<&HulaResult> {
        match self {
            Self::Hula(r) => Some(r),
            _ => None,
        }
    }

    /// The pair's result, if that is what this is.
    #[must_use]
    pub fn as_pair(&self) -> Option<&PairResult> {
        match self {
            Self::Pair(r) => Some(r),
            _ => None,
        }
    }

    /// The planetary result, if that is what this is.
    #[must_use]
    pub fn as_planetary(&self) -> Option<&PlanetaryResult> {
        match self {
            Self::Planetary(r) => Some(r),
            _ => None,
        }
    }
}

/// Which ratings an automatic face width is sized from.
///
/// Four toggles, and shaped like the answer they select from so the UI can walk
/// them rather than naming each: a rating exists for every combination of what
/// fails (bending or contact) and what it is rated against (the ultimate or the
/// fatigue allowable). Per **kind** rather than per case: however many loads
/// of a kind there are, the width is the largest any of them asks for, and a
/// toggle says whether that kind of ask counts at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct FaceSources {
    pub bending: ByKind<bool>,
    pub contact: ByKind<bool>,
}

impl Default for FaceSources {
    fn default() -> Self {
        Self {
            bending: ByKind {
                ultimate: true,
                fatigue: true,
            },
            // **Neither contact rating sizes a width by default.** Both are
            // offered and both are computed; what they are not is *assumed*.
            //
            // The ultimate kind is the weaker of the two: a Hertzian pressure is not
            // a tensile stress, and the library's `ultimate_allowable` is a
            // tensile figure — a flank under a single overload fails by
            // subsurface shear, at a contact pressure well above it. Comparing
            // them is arithmetic with no mechanism behind it.
            //
            // The fatigue kind is sounder — the fatigue allowable is a flank
            // figure — but it is the one that *dominates*, by an order of
            // magnitude: on the reference train it asks 8.5 mm where bending
            // asks 0.9. A default that decides the answer is a default making
            // the design decision, so both are left to the designer and bending
            // is what a fresh stage is sized from.
            contact: ByKind {
                ultimate: false,
                fatigue: false,
            },
        }
    }
}

impl FaceSources {
    /// **The width to size a member to**: the largest an *enabled* rating asks
    /// for, or the width the member was given where none is enabled.
    ///
    /// Nothing enabled used to come out **zero**, and the stage then divided by
    /// it — every stress infinite, every minimum width a NaN. Those cross the
    /// boundary as `null` and draw as blanks, so a reader saw the note and no
    /// figures, which is the right outcome reached by accident. The note's
    /// promise is that the input is *said* rather than divided by, and this is
    /// what keeps it: an automatic width with nothing to choose between has
    /// nothing to choose, so it stands at the number already in its box, which
    /// is on screen beside the toggle that stopped deciding it.
    ///
    /// A width a designer **types** as zero is a different thing — it describes
    /// a gear with no face, and this cannot rescue it. See `docs/state.md`.
    ///
    /// `asks` is every load case's ask with the kind that made it: the largest
    /// over every case of a kind that is switched on — the highest case is the
    /// one that sizes the part, however many overlap. A train with no case
    /// asks nothing, and that is the same answer as no source: the width
    /// stands at its box.
    #[must_use]
    pub fn width_for(&self, asks: &[(CaseKind, Widths)], given: f64) -> f64 {
        if !self.any() || asks.is_empty() {
            return given;
        }
        let mut want = 0.0_f64;
        for (kind, w) in asks {
            if *self.bending.get(*kind) {
                if let Some(b) = w.bending {
                    want = want.max(b);
                }
            }
            if *self.contact.get(*kind) {
                if let Some(c) = w.contact {
                    want = want.max(c);
                }
            }
        }
        want
    }

    /// Whether anything at all is selected.
    #[must_use]
    pub fn any(&self) -> bool {
        self.bending.ultimate
            || self.bending.fatigue
            || self.contact.ultimate
            || self.contact.fatigue
    }
}

/// How a train treats a root loaded on **both** flanks.
///
/// Assembled by [`solve_train`] and handed down, because it is a fact about the
/// train rather than about any one stage: whether the designer asked for the
/// correction at all. Which members *are* reversed is a fact about each member
/// in each load case, and arrives beside the load ([`StageLoad::reverses`]).
///
/// # Why the correction is asked for rather than applied
///
/// A root loaded on both flanks endures less than one loaded on a single flank,
/// and the usual allowance is a fraction on the *allowable*
/// ([`REVERSED_BENDING_FRACTION`](crate::material::REVERSED_BENDING_FRACTION)).
/// That fraction is a convention: it multiplies a stress a part is sized
/// against, which is exactly what `docs/rationale.md` refuses to apply on a
/// designer's behalf. So it is a switch, off by default, and where it is off the
/// stage **says** that reversal is present and uncorrected rather than leaving
/// the reader to notice.
///
/// # Which members reverse
///
/// A planet always does — the sun drives one flank and the ring the other,
/// whatever the drive does. Every other gear does in a load case whose *duty*
/// reverses, which is the same flag that splits that case's contact cycles
/// between the two flanks. One rule, so a note cannot appear where a cycle count
/// does not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reversal {
    /// Judge a reversed root against the reduced allowable.
    pub correct: bool,
}

impl Reversal {
    /// Whether a member's root is loaded both ways in one load case. `always`
    /// is the member's own structural answer — true for a planet, false for
    /// everything else — and `in_case` is that case's duty's.
    #[must_use]
    pub fn reverses(self, always: bool, in_case: bool) -> bool {
        always || in_case
    }

    /// What a member's reversal is worth saying about it, if anything.
    ///
    /// One home, so a stage cannot report the correction on one member and stay
    /// silent about it on another — and so the sentence is the same whichever
    /// kind of stage raises it. Asked once per member, of whether *any* case
    /// reverses it: the note is about the root, and the figures beside it say
    /// which cases it is judged in.
    #[must_use]
    pub fn note_for(self, reverses: bool) -> Option<Note> {
        if !reverses {
            return None;
        }
        Some(if self.correct {
            Note::new(key::GEAR_REVERSED_BENDING_APPLIED).number(
                "fraction",
                crate::material::REVERSED_BENDING_FRACTION,
                2,
            )
        } else {
            Note::new(key::GEAR_REVERSED_BENDING_UNCORRECTED)
        })
    }

    /// The **bending** allowable a member is judged against, MPa.
    ///
    /// Only a fatigue case can be reduced: an ultimate load is survived once
    /// and has no reversal to endure. And only bending — pitting is compressive
    /// whichever flank carries it, so a contact rating keeps the material's own
    /// figure.
    #[must_use]
    pub fn bending_allowable(self, m: &Material, kind: CaseKind, reverses: bool) -> f64 {
        if self.correct && reverses && kind == CaseKind::Fatigue {
            crate::material::reversed_bending_allowable(m).value
        } else {
            allowable(m, kind)
        }
    }
}

/// Which allowable a load case is judged against.
///
/// **This is the whole of what a kind decides in the core.** An ultimate load
/// has to be survived once, so the ultimate figure is the right bar; a fatigue
/// load has to be survived for its duty, so the fatigue figure is — and only a
/// fatigue case has a duty to count cycles over, or a drive to reverse. Rating
/// a peak against a fatigue allowable asks the wrong question, and it is what
/// this crate did before the two kinds existed. Everything else about a load
/// case — where it enters, what holds it, how big it is — is the same question
/// for either kind, and a kind is otherwise a preset and a vocabulary, as
/// [`PairKind`] is over a pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum CaseKind {
    /// Survive it once: judged against the ultimate allowable.
    Ultimate,
    /// Survive it for the duty: judged against the fatigue allowable.
    Fatigue,
}

impl CaseKind {
    /// Both kinds, in the order they are reported.
    pub const BOTH: [Self; 2] = [Self::Ultimate, Self::Fatigue];
}

/// A quantity held for each **kind** of load case — the one two-valued shape
/// left once the cases themselves became a list, and it is about allowables
/// rather than loads: a material has two, so a rating has two things to be
/// judged against, however many loads there are.
///
/// Built through [`ByKind::of`] for the same reason [`Directional`] is: there is
/// no path by which one kind is answered and the other not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ByKind<T> {
    pub ultimate: T,
    pub fatigue: T,
}

impl<T> ByKind<T> {
    /// Ask the same question of both kinds.
    pub fn of(mut f: impl FnMut(CaseKind) -> T) -> Self {
        Self {
            ultimate: f(CaseKind::Ultimate),
            fatigue: f(CaseKind::Fatigue),
        }
    }

    /// The value for one kind.
    pub const fn get(&self, kind: CaseKind) -> &T {
        match kind {
            CaseKind::Ultimate => &self.ultimate,
            CaseKind::Fatigue => &self.fatigue,
        }
    }
}

/// The allowable a load case's kind is judged against, MPa.
#[must_use]
pub fn allowable(material: &Material, kind: CaseKind) -> f64 {
    match kind {
        CaseKind::Ultimate => material.ultimate_allowable.value,
        CaseKind::Fatigue => material.fatigue_allowable.value,
    }
}

/// Where a load enters the train, or where a sweep is measured.
///
/// A train has no forward (`docs/rationale.md#direction-is-the-readers-not-the-mechanisms`): a
/// load applied at a shaft works its way toward whatever holds it, and the
/// direction it travels in at each stage is found by walking the shaft line
/// from that shaft ([`Train::route`]) — never stored.
///
/// Two names and a reference. `Start` and `End` are the two ports a chain
/// names for itself — the first stage's input and the last stage's output,
/// *under the constraints in force* — and they survive a stage being added
/// or rearranged, which is why they are names rather than indices. `At` is
/// any shaft a stage lists as a port ([`Ports`]): a load that enters a set by
/// its carrier while its sun is turned from upstream, or a sweep measured at
/// a shaft the chain does not end at. [`Train::port_shaft`] resolves all
/// three to a shaft and nothing downstream of it knows which was written.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Port {
    /// The first stage's input.
    Start,
    /// The last stage's output.
    End,
    /// A named shaft.
    At(ShaftRef),
}

/// How a fatigue load is applied over the life of the train — what turns a
/// ratio into a tooth count.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Duty {
    /// A limited sweep, repeated. The range is measured at a **named** port —
    /// the sweep is a fact about the mechanism's motion, not about where its
    /// load enters, and a 25° sweep of the output is what a designer knows
    /// whichever shaft is driving it. Every other shaft's revolutions are
    /// worked from there through the ratios.
    Intermittent {
        /// Sweep per actuation, degrees, at [`Self::Intermittent::at`].
        range_degrees: f64,
        /// The shaft the sweep is measured at.
        at: Port,
        actuations: u32,
        /// Whether the duty reverses between actuations.
        ///
        /// It changes nothing but the **cycle count** and which roots are
        /// loaded both ways ([`loaded_cycles`], [`Reversal`]): each
        /// actuation's revolutions round up on their own rather than the total
        /// rounding once, because a partial sweep still loads the teeth it
        /// reaches and every tooth must meet the worst of them; and the contact
        /// count halves, because the two flanks share the engagements while
        /// both take the full bending.
        reversing: bool,
    },
    /// Continuous running, at the load case's own speed.
    Continuous { runtime_hours: f64 },
}

impl Default for Duty {
    fn default() -> Self {
        Self::Intermittent {
            range_degrees: 25.0,
            at: Port::End,
            actuations: 1000,
            reversing: false,
        }
    }
}

impl Duty {
    /// Whether this duty loads every root both ways.
    #[must_use]
    pub const fn reverses(&self) -> bool {
        matches!(
            self,
            Self::Intermittent {
                reversing: true,
                ..
            }
        )
    }
}

/// One load the train is rated for: a torque, where it enters, what holds it,
/// and which allowable it is judged against.
///
/// Named for the engineering term — a defined set of loads applied for analysis
/// — and deliberately *not* for a duty cycle, which is a fraction of time spent
/// running and a different idea with different implications. A fatigue case
/// carries one of those as [`Self::duty`]; the case is not it.
///
/// A train carries any number of these, as it carries any number of stages,
/// and for the same reason: the two loads it used to hold — a peak at the input
/// and a back-driving one at the output — were the first two entries of this
/// list with their ports and directions written into the field names. A case
/// applied at the far end is not a sign on one applied at the near end; it is
/// the same kind of thing entering elsewhere, and [`Port`] is what says where.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LoadCase {
    pub kind: CaseKind,
    /// A case switched off takes part in nothing — no rating, no sizing, no
    /// readout — and its inputs stand, so it can be switched back on as it was.
    pub enabled: bool,
    pub port: Port,
    /// **Whether the far end holds the load.** On, the load is carried through
    /// every stage to the far port, attenuated by each stage's efficiency in
    /// its direction of travel — a motor's torque delivered to the output, or a
    /// brake at the motor holding an output load through every mesh. Off, only
    /// a stage that cannot be driven that way holds it, and a train with no
    /// such stage simply turns under it and carries none of it
    /// (`docs/rationale.md#a-load-exists-only-where-it-is-reacted`). Either way
    /// a stage that locks in the direction of travel holds the load where it
    /// stands, and nothing beyond it sees any.
    pub reacted: bool,
    /// N·m at the port.
    pub torque: f64,
    /// rpm at the port. Zero is a load held still.
    pub speed: f64,
    /// The duty a fatigue case is spent over. Read by a fatigue case alone —
    /// an ultimate load is survived once and has no cycles to count — and kept
    /// while the case is ultimate for the reason [`Auto::manual`] is kept while
    /// automatic: switching the kind back finds it where it was.
    pub duty: Duty,
}

impl LoadCase {
    /// The train's own default load: an ultimate torque at the start, held at
    /// the far end.
    #[must_use]
    pub fn ultimate(torque: f64, speed: f64) -> Self {
        Self {
            kind: CaseKind::Ultimate,
            enabled: true,
            port: Port::Start,
            reacted: true,
            torque,
            speed,
            duty: Duty::default(),
        }
    }

    /// The same load, judged for fatigue over the default duty.
    #[must_use]
    pub fn fatigue(torque: f64, speed: f64) -> Self {
        Self {
            kind: CaseKind::Fatigue,
            ..Self::ultimate(torque, speed)
        }
    }

    /// The duty this case is spent over, where it has one.
    #[must_use]
    pub fn counted(&self) -> Option<&Duty> {
        (self.kind == CaseKind::Fatigue).then_some(&self.duty)
    }
}

/// How often the shaft a stage takes its load on comes round in one case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turns {
    /// Revolutions over the whole duty.
    pub revolutions: f64,
    /// The number of actuations, where the duty reverses between them — the
    /// count [`loaded_cycles`] rounds within, and the flag that reverses every
    /// root. `None` for a duty that does not reverse.
    pub reversing_actuations: Option<f64>,
}

/// **One load case as one stage sees it**: referred to the shaft every stage
/// solver takes its load on — its first member — and travelling in one
/// direction.
///
/// Assembled by [`solve_train`], which is the only level that knows where a
/// stage sits in the shaft line and what reaches it from each end. A stage
/// rates every case at its own torque *and* its own direction: which way a
/// stage is driven decides how a load distributes through it — where `η₀`
/// multiplies in an epicyclic set, which member's flank a screw pair pushes on
/// — so the direction is carried beside the number rather than folded into it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageLoad {
    /// Index into the train's list of load cases.
    pub case: usize,
    pub kind: CaseKind,
    pub drive: Drive,
    /// N·m at the stage's first member. Zero where the case is held before it
    /// reaches this stage, and zero is a load: a stage carrying nothing rates
    /// at nothing rather than refusing to answer.
    pub torque: f64,
    /// rpm at the stage's first member.
    pub speed: f64,
    /// How often that member comes round — a fatigue case's, and `None` on an
    /// ultimate one, which has no cycles to count.
    pub turns: Option<Turns>,
}

impl StageLoad {
    /// Whether this case's duty loads every root both ways.
    #[must_use]
    pub fn reverses(&self) -> bool {
        self.turns.is_some_and(|t| t.reversing_actuations.is_some())
    }
}

/// Every load case a stage is rated for, in the train's order — and what the
/// stage is asked, since both come from the train and a stage needs both to
/// rate anything.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StageLoads {
    pub cases: Vec<StageLoad>,
    /// **Which of the stage's shafts are held and which driven**, assembled by
    /// [`solve_train`] from the train's constraints and couplings
    /// ([`Train::boundaries`]). `None` is a stage asked about on its own — by
    /// a test, the harness, the sweep — which is solved under its kind's
    /// conventions ([`Stage::conventional_boundary`]).
    pub boundary: Option<StageBoundary>,
}

impl StageLoads {
    /// The two kinds, forward, at one torque and no speed — the shape a caller
    /// wants when it is asking about a single load: index 0 is the ultimate
    /// reading and 1 the fatigue one.
    #[must_use]
    pub fn just(torque: f64) -> Self {
        Self::at(torque, 0.0)
    }

    /// As [`Self::just`], at a speed.
    #[must_use]
    pub fn at(torque: f64, speed: f64) -> Self {
        Self {
            cases: CaseKind::BOTH
                .iter()
                .enumerate()
                .map(|(i, &kind)| StageLoad {
                    case: i,
                    kind,
                    drive: Drive::Forward,
                    torque,
                    speed,
                    turns: None,
                })
                .collect(),
            boundary: None,
        }
    }

    /// These loads, with the stage asked this rather than its convention.
    #[must_use]
    pub fn under(self, boundary: StageBoundary) -> Self {
        Self {
            boundary: Some(boundary),
            ..self
        }
    }

    /// Whether any case reverses the roots it reaches.
    #[must_use]
    pub fn any_reverse(&self) -> bool {
        self.cases.iter().any(StageLoad::reverses)
    }

    /// The largest magnitude among `torque(case)`, and each case's as a
    /// fraction of it — what a figure evaluated once at the worst load a mesh
    /// carries scales by to reach each case.
    ///
    /// A rating is evaluated once, at the worst torque the mesh carries, and
    /// every case is that scaled ([`Loading::for_cases`]). **A zero worst is
    /// zero everywhere** — a mesh carrying nothing is a load and not a division
    /// to guard against.
    pub fn scaled(&self, torque: impl Fn(&StageLoad) -> f64) -> (f64, Vec<f64>) {
        let torques: Vec<f64> = self.cases.iter().map(torque).collect();
        let worst = torques.iter().fold(0.0_f64, |m, t| m.max(t.abs()));
        let scales = torques
            .iter()
            .map(|t| if worst == 0.0 { 0.0 } else { t / worst })
            .collect();
        (worst, scales)
    }
}

/// How often a tooth is loaded, which is not one number once the duty reverses.
///
/// Both counts are always reported and are the **same number** when the duty
/// does not reverse — the ordinary case as a value rather than behind a flag, so
/// a reader can quote a range unconditionally.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Cycles {
    /// Engagements the tooth root sees. A reversing duty loads **both** flanks
    /// in bending, so every engagement counts.
    pub bending: f64,
    /// Engagements one flank sees. A reversing duty shares them between the two
    /// flanks, so each takes half; otherwise it is the same number as `bending`.
    pub contact: f64,
}

/// The greatest common divisor of two tooth counts.
///
/// One home, because it was two: a coprime check is what says whether a pair
/// hunts, and every stage kind that has a mesh asks it.
pub(crate) const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// **How often one member of an epicyclic set is engaged**, per revolution of
/// the shaft the train counted revolutions on.
///
/// One rule, and both epicyclic kinds here obey it: a member's teeth are
/// engaged once per revolution **relative to the carrier**, once for each
/// parallel mesh path the set has. In the carrier's frame the arm stands still
/// and everything else turns past it, which is what makes the relative speed the
/// one that counts — and it is the same sentence for a sun, a ring, a planet, a
/// grounded gear and a wobble body, so no member is the exception it has to be
/// remembered for.
///
/// It had been three sentences. A sun and a ring counted the *input shaft's*
/// revolutions, which on an ordinary set with the ring held over-counts the sun
/// by `(z_s + z_r)/z_r` and the ring — whose teeth are loaded while it does not
/// turn at all — by the whole of `(z_s + z_r)/z_s`, three and a half times on
/// the shipped counts. Only the planet was carrier-relative, and it was
/// referred to the *sun's* speed rather than to the input's, so it was right
/// only in the arrangements where those are the same shaft.
///
/// A ratio of speeds, so their magnitude cancels — which is why every kind asks
/// it of its **unit** kinematics, the speeds at one turn of its input, rather
/// than of a load case's: a case held still is still engaged by every sweep
/// its duty counts. A train whose input shaft does not turn has no ratio to
/// take, and answers zero.
pub(crate) fn engagements(member: f64, carrier: f64, input: f64, paths: f64) -> f64 {
    if input == 0.0 {
        return 0.0;
    }
    ((member - carrier) / input).abs() * paths
}

/// How many times a tooth is loaded, from how many times it comes round.
///
/// # Why this rounds, and why the rounding is here
///
/// A tooth is either loaded or it is not: two thirds of an engagement is one
/// engagement as far as the tooth is concerned, so a fractional count is
/// rounded **up**. That is a statement about how a gear is loaded, and it
/// belongs in the model.
///
/// It lived in `TrainPanel.svelte` as `Math.ceil` on the way to the screen —
/// harmless while it was only display, because rounding an integer up gives the
/// same integer wherever you do it, and against this project's standing rule the
/// whole time. Two things made it worth moving: the CLI printed the *unrounded*
/// figure while the browser printed the rounded one, so one model already had
/// two answers; and a reversing drive rounds at a different point in the
/// arithmetic, which makes *where* this happens a modelling decision rather than
/// a formatting one.
///
/// # What reversing changes, and why it is two changes
///
/// **Where the rounding happens.** Not reversing, the whole duty rounds once:
/// `ceil(revolutions_per_actuation × actuations)`. Reversing, each actuation
/// rounds on its own: `ceil(revolutions_per_actuation) × actuations`. A sweep of
/// less than a full turn loads only some of the teeth — but which ones is not
/// knowable, the teeth are radially symmetric, and every one of them has to meet
/// the worst case. So each actuation costs a whole engagement to whichever teeth
/// it reaches, and rounding the total instead would spread a fraction that is
/// not divisible.
///
/// **And the contact count halves.** Reversing puts the load on the other flank
/// on the way back, so a given flank sees half the engagements while the root
/// sees all of them.
///
/// `None` for a duty that does not reverse: there is no actuation to round
/// within, so the total rounds once and the two counts are equal.
#[must_use]
pub fn loaded_cycles(turns: Turns) -> Cycles {
    match turns.reversing_actuations {
        Some(actuations) => {
            let bending = (turns.revolutions / actuations).ceil() * actuations;
            Cycles {
                bending,
                contact: bending / 2.0,
            }
        }
        None => {
            let n = turns.revolutions.ceil();
            Cycles {
                bending: n,
                contact: n,
            }
        }
    }
}

impl Turns {
    /// The same duty seen from a shaft that turns `by` times for each turn of
    /// this one — a member's engagements from its stage's input revolutions.
    #[must_use]
    pub fn scaled(self, by: f64) -> Self {
        Self {
            revolutions: self.revolutions * by,
            ..self
        }
    }
}

/// Solve one stage of whichever kind, given the loads on its input member.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any(
    stage: &Stage,
    loads: &StageLoads,
    lib: &MaterialLibrary,
) -> Result<StageResult, TrainError> {
    solve_any_with(stage, loads, lib, Reversal::default())
}

/// The same, told how the train treats reversed bending.
///
/// A second entry point rather than a fourth argument on the first, because a
/// stage asked about in isolation — by a test, by the CLI, by the sweep — has no
/// train to inherit that from and should not have to invent one. The plain call
/// is this one at [`Reversal::default`]: no correction.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any_with(
    stage: &Stage,
    loads: &StageLoads,
    lib: &MaterialLibrary,
    reversal: Reversal,
) -> Result<StageResult, TrainError> {
    match stage {
        // One stage kind, two meshes. Crossing the shafts changes what the
        // teeth do to each other — a line contact becomes a point, and the
        // sliding changes direction — so a crossed pair answers with the screw
        // result. The *inputs* stay one set, as the specification has them.
        Stage::Spur(s) => solve_pair_stage_with(s, PairKind::Spur, loads, lib, reversal)
            .map(|r| StageResult::Pair(Box::new(r))),
        Stage::Worm(s) => solve_pair_stage_with(s, PairKind::Worm, loads, lib, reversal)
            .map(|r| StageResult::Pair(Box::new(r))),
        // An epicyclic kind's efficiency depends on which shaft is held, which
        // is a kinematic question its own arrangement answers; the loads carry
        // each case's speed, so the members' speeds follow from them.
        Stage::Planetary(s) => solve_planetary_stage_with(s, loads, lib, reversal)
            .map(|r| StageResult::Planetary(Box::new(r))),
        Stage::Hula(s) => {
            solve_hula_stage_with(s, loads, lib, reversal).map(|r| StageResult::Hula(Box::new(r)))
        }
    }
}

/// A whole geartrain.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Train {
    /// Every load the train is rated for. Any number, as the stages are any
    /// number — including none, which is a shaft line with nothing rated on
    /// it; a case switched off is kept and takes part in nothing.
    pub load_cases: Vec<LoadCase>,
    /// Judge a root that is loaded on **both** flanks against the reduced
    /// bending allowable.
    ///
    /// Off by default. A planet's root is always loaded both ways and a
    /// reversing duty loads every root both ways, but what to do about it is a
    /// convention that multiplies a stress — so the train asks rather than
    /// assumes, and says where reversal is present and uncorrected. See
    /// [`Reversal`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub reversed_bending: bool,
    pub stages: Vec<Stage>,
    /// **Which shafts turn as one.** Empty is the chain — each stage's
    /// conventional output to the next stage's input — which is what every
    /// train meant before this existed, so the absence is unambiguous and
    /// defaults rather than refuses. Written out, a train can join any two
    /// shafts: a coaxial output, a locked clutch, a second stage on a set's
    /// ring. See [`Coupling`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub couplings: Vec<Coupling>,
    /// **What is asked of each shaft** — held, driven or free. Empty is each
    /// kind's convention with the first stage's input driven, for the same
    /// reason the couplings default. Written out, this is where a planetary
    /// set's arrangement lives now, and where a second input or a third port
    /// is one more line. See [`ShaftConstraint`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub constraints: Vec<ShaftConstraint>,
}

impl Train {
    /// The cases that take part, with their index in the list — which is how
    /// every result names the case it belongs to.
    pub fn enabled_cases(&self) -> impl Iterator<Item = (usize, &LoadCase)> {
        self.load_cases
            .iter()
            .enumerate()
            .filter(|(_, c)| c.enabled)
    }
}

/// **What one load case comes to at the train level**: what reaches the far
/// end, and where — or whether — it was held.
///
/// Facts about the shaft line, so no stage is in a position to say them.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TrainCase {
    /// Index into the train's list of load cases.
    pub case: usize,
    /// The port the load is carried toward.
    pub delivered_at: Port,
    /// The torque arriving there, N·m, after every loss on the way — zero
    /// where a stage held the load first, or where nothing held it at all.
    pub delivered_torque: f64,
    /// The speed of that shaft, rpm, from the case's own speed through the
    /// ratios.
    pub delivered_speed: f64,
    /// The stage that holds the load because it cannot be driven in the
    /// load's direction, if one does. Stages beyond it carry none of it.
    pub reacted_at: Option<usize>,
    /// What the train wants read about this case: where it was held, or that
    /// nothing held it.
    pub notes: Vec<Note>,
}

/// What a train produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TrainResult {
    /// Product of the stage ratios.
    pub total_ratio: f64,
    /// Product of the stage efficiencies, in both drive directions.
    ///
    /// A train containing a self-locking stage cannot be back-driven at all, and
    /// [`Directional::locked`] on this pair says so.
    pub total_efficiency: Directional<f64>,
    /// Angular backlash referred to whichever shaft is the output, degrees: the
    /// last shaft driving forward, the first driving backward.
    pub backlash: Directional<Backlash>,
    /// Every enabled load case, in the train's order.
    pub cases: Vec<TrainCase>,
    pub stages: Vec<StageResult>,
}

/// Where one load case is carried, and what each stage feels of it.
///
/// # The model
///
/// A load exists only where something holds it. Applied at a shaft it works
/// its way along its [`Route`] to the far end, referred at each stage to that
/// stage's own first member — a division by the ratio when it arrives from the far side and
/// nothing else, since the mesh force is set by the torque at the wheel and the
/// losses sit between the mesh and the shaft beyond — and leaving attenuated
/// by the stage's efficiency **in the direction it is travelling**. A stage
/// that cannot be driven that way holds it: everything beyond sees nothing,
/// and the reaction is the load that stage carries. A self-locking worm holds
/// a load from the output, which is why a designer puts one in a lifting
/// drive; a crossed pair at a steep helix split holds one from the input, and
/// the same walk finds it.
///
/// If the walk reaches the far port with the load still turning something,
/// then what happens is the case's [`LoadCase::reacted`]: held there, the
/// whole train carries it; not held, nothing reacted it — the train simply
/// turns under the load and the case is zero at every gear. That last outcome
/// is an input reaching no number, which must be **said** rather than silently
/// ignored — so the case reports which stage held the load, or that none did.
///
/// # Why a two-pass solve
///
/// A stage's torque depends on the ratio and efficiency of every stage between
/// it and the port, which are not known until those stages are solved. Ratio
/// and efficiency do not depend on the load, so the train is solved once for
/// the shaft line and again for the ratings. The second pass is not a
/// refinement of the first — it is the same arithmetic with the loads it was
/// missing.
fn carry(stages: &[StageResult], route: &Route, case: &LoadCase) -> (Vec<f64>, Option<usize>, f64) {
    let mut torques = vec![0.0; stages.len()];
    let mut at_port = case.torque;
    let mut reacted_at = None;
    for &(k, drive) in &route.steps {
        let s = &stages[k];
        // **A referral is a magnitude; the direction of rotation is the
        // motion's.** A stage's ratio says two things at once — how much a
        // torque is multiplied by, and whether the output turns the other way
        // — and only the first belongs here. The second changes the *sign of
        // every shaft downstream*, which is a fact about the shaft line and
        // not about how hard a tooth is pressed.
        //
        // **Measured, and it was live on any train with a reversing stage.**
        // An epicyclic set reports a signed ratio where a pair reports a
        // magnitude (`docs/corrections.md`), so a set with its carrier held —
        // ratio −6 — handed the stage after it **−11.6 N·m**, and the pair
        // refused with *"the teeth never come into contact"*: a negative
        // tangential force has no Hertzian contact to press, at any face
        // width. So a planetary set with its carrier held could not be
        // followed by anything at all, and nothing said why.
        let referral = s.ratio().abs();
        let referred = match drive {
            Drive::Forward => at_port,
            Drive::Backward => at_port / referral,
        };
        torques[k] = referred;
        let efficiency = *s.efficiency().get(drive);
        if efficiency <= 0.0 {
            // Locked this way: this stage is where the load stops.
            reacted_at = Some(k);
            break;
        }
        at_port = match drive {
            Drive::Forward => referred * referral * efficiency,
            Drive::Backward => referred * efficiency,
        };
    }
    let delivered = if reacted_at.is_some() {
        0.0
    } else if case.reacted {
        at_port
    } else {
        // Nothing held it: the load turns the train rather than being carried.
        torques.iter_mut().for_each(|t| *t = 0.0);
        0.0
    };
    (torques, reacted_at, delivered)
}

/// # Errors
///
/// [`TrainError::Empty`] for a train with no stages, or whatever the first
/// failing stage reports.
pub fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<TrainResult, TrainError> {
    if train.stages.is_empty() {
        return Err(TrainError::Empty);
    }

    // Whether to correct for a root loaded on both flanks is one switch for the
    // whole train rather than a decision taken per stage.
    let reversal = Reversal {
        correct: train.reversed_bending,
    };

    // Two passes: the first learns the shaft line — each stage's ratio and
    // efficiency, which no load can move — and the second rates it. The first
    // runs at a unit load rather than at none: an automatic face width is
    // sized from a rating, and a stage asked to rate nothing on a face of no
    // width has a `0/0` to refuse where the shaft line was all that was wanted.
    let solve = |loads: &dyn Fn(usize) -> StageLoads| -> Result<Vec<StageResult>, TrainError> {
        train
            .stages
            .iter()
            .enumerate()
            .map(|(k, stage)| {
                solve_any_with(stage, &loads(k), lib, reversal).map_err(|e| TrainError::InStage {
                    stage: k,
                    cause: Box::new(e),
                })
            })
            .collect()
    };
    // **The shaft line comes from the graph, not from a product of ratios.**
    // It needs no geometry — a ratio is tooth counts and a topology — and it is
    // *signed*, where `StageResult::ratio` is a magnitude on a pair and a
    // signed reduction on an epicyclic set. That difference is what made one
    // physical shaft report two speeds, one at the end of a stage and the other
    // at the start of the next (`docs/corrections.md`).
    let motion = train.motion()?;
    // **A family is not rated.** The motion is still reported — the boundary
    // sends it beside the refusal — but every stage's rating wants one torque
    // at one speed, and which one is the designer's to say.
    if !motion.solution.is_unique() {
        return Err(TrainError::Underdetermined {
            short: motion.solution.residual.len(),
        });
    }
    // **What each stage is asked**, from the train's constraints and
    // couplings — which is where a set's arrangement lives now — handed to the
    // stage beside its loads.
    let boundaries = train.boundaries()?;
    // The reduction the graph already holds exactly, read as a float once
    // rather than reciprocated as one.
    let total_ratio = motion
        .total
        .map_or(f64::INFINITY, crate::ratio::Ratio::to_f64);

    // The first pass, too, is solved under what the train asks: a set's
    // arrangement decides its power flow, and the train is where it is asked.
    let first = solve(&|k| StageLoads {
        boundary: Some(boundaries[k].clone()),
        ..StageLoads::just(1.0)
    })?;

    // --- what each stage is loaded by, case by case.
    let mut cases = Vec::new();
    let mut per_stage: Vec<Vec<StageLoad>> = vec![Vec::new(); train.stages.len()];
    for (index, case) in train.enabled_cases() {
        // **Where the load enters is a shaft, and where it goes is read off
        // the graph** — forward through a stage it enters by the input of,
        // backward through one it enters by the output of — rather than a
        // direction stored beside the port.
        let entry = train.port_shaft(&boundaries, case.port);
        let route = train.route(&boundaries, entry).map_err(|e| match e {
            RouteError::NotAPort => TrainError::LoadPort { case: index },
            RouteError::Shared => TrainError::LoadShared { case: index },
        })?;
        let (torques, reacted_at, delivered_torque) = carry(&first, &route, case);
        let delivered_speed = motion.read(case.speed, route.far, entry);
        let mut notes = Vec::new();
        match reacted_at {
            Some(k) => {
                notes.push(Note::new(key::TRAIN_LOAD_REACTED_AT).text("stage", (k + 1).to_string()))
            }
            None if !case.reacted => notes.push(Note::new(key::TRAIN_LOAD_NOT_REACTED)),
            None => {}
        }
        cases.push(TrainCase {
            case: index,
            delivered_at: train.port_named(&boundaries, route.far),
            delivered_torque,
            delivered_speed,
            reacted_at,
            notes,
        });
        for (k, loads) in per_stage.iter_mut().enumerate() {
            // How many times this stage's input turns per turn of the shaft
            // the case is stated at: the whole shaft line's kinematics in one
            // exact quotient, so a speed, a sweep or a revolution count stated
            // anywhere reaches every stage by the same multiplication.
            let input = ShaftRef::Of {
                stage: k,
                shaft: boundaries[k].input,
            };
            let speed = motion.read(case.speed, input, entry);
            // How often this stage's first member comes round over a fatigue
            // case's duty: a sweep stated at a port, or a time at the case's
            // own speed, and either reaches here through the ratios.
            let turns = case.counted().map(|duty| match *duty {
                Duty::Intermittent {
                    range_degrees,
                    at: sweep_at,
                    actuations,
                    reversing,
                } => {
                    let n = f64::from(actuations);
                    Turns {
                        // **A sweep is a magnitude.** It is stated at a port as
                        // degrees of motion, and how many turns that is of some
                        // other shaft does not depend on which way either turns.
                        revolutions: motion
                            .read(
                                (range_degrees / 360.0) * n,
                                input,
                                train.port_shaft(&boundaries, sweep_at),
                            )
                            .abs(),
                        reversing_actuations: reversing.then_some(n),
                    }
                }
                Duty::Continuous { runtime_hours } => Turns {
                    revolutions: speed.abs() * 60.0 * runtime_hours,
                    reversing_actuations: None,
                },
            });
            loads.push(StageLoad {
                case: index,
                kind: case.kind,
                // A stage the route does not reach carries nothing of this
                // case, and nothing is the same load either way round.
                drive: route.drive_at(k).unwrap_or(Drive::Forward),
                torque: torques[k],
                speed,
                turns,
            });
        }
    }
    let stages = solve(&|k| StageLoads {
        cases: per_stage[k].clone(),
        boundary: Some(boundaries[k].clone()),
    })?;

    let total_efficiency = Directional::of(|d| {
        stages
            .iter()
            .map(|s| *s.efficiency().get(d))
            .product::<f64>()
    });

    // Each stage's backlash, referred to whichever shaft is the output.
    //
    // Driven forward that is the last shaft, so a stage's contribution is
    // divided by everything downstream of it. Driven backward the *input* shaft
    // is the output, so the contribution is multiplied by everything upstream
    // instead: those shafts turn faster, and the same play is a larger angle
    // there. The last stage dominates either way, and by more going backward.
    //
    // **The referral is a magnitude here for a second reason: play does not
    // cancel.** Two independent sources of lost motion add up whichever way
    // their shafts turn, so a reversing stage between a source and the output
    // must not subtract it. Signed, it did: a spur pair ahead of a set with its
    // carrier held — ratio −6 — reported **0.0422°** of forward backlash where
    // the two stages between them have 0.0552°, the pair's contribution coming
    // in negative and taking 23.5 % off the answer. It is the same conflation
    // `carry` above records, met on the other reading of a ratio.
    let refer = |drive: Drive, pick: fn(&Backlash) -> f64| -> f64 {
        let referral = |s: &StageResult| s.ratio().abs();
        stages
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let stage = pick(s.backlash().get(drive));
                match drive {
                    Drive::Forward => {
                        let downstream: f64 = stages[k + 1..].iter().map(referral).product();
                        stage / downstream
                    }
                    Drive::Backward => {
                        let upstream: f64 = stages[..k].iter().map(referral).product();
                        stage * upstream
                    }
                }
            })
            .sum()
    };

    Ok(TrainResult {
        total_ratio,
        total_efficiency,
        backlash: Directional::of(|d| Backlash {
            nominal: refer(d, |b| b.nominal),
            minimum: refer(d, |b| b.minimum),
            maximum: refer(d, |b| b.maximum),
        }),
        cases,
        stages,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn library() -> MaterialLibrary {
        super::test_library()
    }

    /// These tests build spur trains, so they know the kind and say so once.
    fn spur(r: &StageResult) -> &PairResult {
        r.as_pair().expect("this train's stages are all spur")
    }

    /// ...and a worm stage's result, with its point contact.
    fn worm(r: &StageResult) -> (&PairResult, &MeshReport) {
        let p = r.as_pair().expect("a worm stage is a pair");
        assert!(
            p.mesh.point.is_some(),
            "a worm stage's mesh is a point contact"
        );
        (p, &p.mesh)
    }

    /// The old entry points, as the tests were written against them.
    fn solve_spur_stage(
        stage: &PairStage,
        loads: &StageLoads,
        lib: &MaterialLibrary,
    ) -> Result<PairResult, TrainError> {
        solve_pair_stage(stage, PairKind::Spur, loads, lib)
    }
    fn solve_worm_stage(
        stage: &PairStage,
        loads: &StageLoads,
        lib: &MaterialLibrary,
    ) -> Result<PairResult, TrainError> {
        solve_pair_stage(stage, PairKind::Worm, loads, lib)
    }

    /// ...and the same for reaching into a stage's inputs.
    fn spur_input(s: &mut Stage) -> &mut PairStage {
        match s {
            Stage::Spur(st) => st,
            _ => panic!("this train's stages are all spur"),
        }
    }

    /// **A back-driving load is the load, and the reverse is the forward
    /// construction with the roles swapped.**
    ///
    /// It enters at the output. The member *on* that shaft carries the applied
    /// torque itself — nothing has happened to it yet — and the member at the
    /// other end carries it referred by the ratio and cut by the mesh's loss
    /// **the backward way**, exactly as driving forward the output member
    /// carries the input's referred by the ratio and cut by the forward loss.
    /// One construction, read from either end.
    ///
    /// Two things this has caught, in opposite directions. It once *divided* by
    /// the backward efficiency, and a worm's is **zero** whenever it self-locks
    /// — the case a worm is chosen for — so the wheel reported **2.2e307 N·m**:
    /// finite, so it crossed the boundary as a number rather than as the `null`
    /// an infinity becomes, and drew on screen as a figure. The correction
    /// dropped the factor altogether, which left the *worm* claiming a shaft
    /// torque a locked mesh does not deliver.
    ///
    /// **Both ends of the friction range**, because a self-locking worm alone
    /// cannot tell "times `η_backward`" from "times nought": the loose fixture
    /// is what makes the factor a factor.
    #[test]
    fn a_self_locking_worm_reports_the_load_it_reacts() {
        let lib = library();
        // A worm that locks, and one loose enough to be driven backward — with a
        // locking worm behind it either way, since a load exists only where
        // something reacts it and a train that can be back-driven end to end
        // carries none of it.
        for friction in [0.16_f64, 0.02] {
            for applied in [0.5_f64, 3.0, 12.5] {
                let mut train = two_stage();
                train.stages = vec![
                    Stage::Worm(PairStage::worm()),
                    Stage::Worm(PairStage {
                        sliding_friction: friction,
                        static_friction: friction,
                        ..PairStage::worm()
                    }),
                ];
                train.load_cases[BACK].torque = applied;

                let r = solve_train(&train, &lib).expect("a train that solves");
                let (w, m) = worm(&r.stages[1]);
                let locks = m.efficiency.locked().backward;
                assert_eq!(
                    locks,
                    friction > 0.1,
                    "the fixture must span both sides of the threshold"
                );

                // The stage reacts the load, so its output member carries it.
                let wheel = w.gears[1].cases[BACK].torque;
                assert!(
                    (wheel - applied).abs() < 1e-9,
                    "the wheel is on the output shaft and the load is {applied} N·m, \
                     but it reports {wheel}"
                );

                // ...and the worm carries it referred by the ratio and attenuated
                // by the loss the mesh takes carrying it that way.
                let worm = w.gears[0].cases[BACK].torque;
                // Referred by the *size* of the ratio, which is what a referral is.
                let want = wheel / w.ratio.abs() * m.efficiency.backward.max(0.0);
                assert!(
                    (worm - want).abs() < 1e-9 * applied,
                    "the worm reports {worm} where {wheel} at the wheel over a \
                     ratio of {} at {:.4} backward efficiency is {want}",
                    w.ratio,
                    m.efficiency.backward
                );
                assert_eq!(
                    locks,
                    worm == 0.0,
                    "a locked mesh delivers nothing to the worm's shaft, and an \
                     unlocked one delivers something: {worm} N·m at μ = {friction}"
                );
            }
        }
    }

    /// **Crossing the shafts does not stop a gear being a gear.**
    ///
    /// A crossed pair is a spur stage with an axis angle, and it is solved by
    /// translating it into the equivalent screw pair — which carries no tooth
    /// form, because a worm is a thread. So the translation dropped the shift,
    /// the dedendum and the root round on the way, and with them everything the
    /// members had to say about themselves: **the same pair reported
    /// `clamp.tooth_undercut` with its shafts parallel and nothing at all with
    /// them crossed.**
    ///
    /// `docs/corrections.md` records that gap being closed once already — "a gear
    /// in a geartrain never said it was undercut … reported now on every
    /// rack-cut member of every stage kind". It reached four kinds of five.
    ///
    /// The shaft angle is the helix angle doubled, so a crossed pair's teeth are
    /// genuinely *different* teeth from the parallel pair's — at 45° of helix a
    /// 9-tooth pinion is not undercut at all, because the transverse pressure
    /// angle has risen to 27°. So this is not an equality between the two
    /// answers. It is the claim that **the crossed member reports its own
    /// tooth's clamps**, checked at a shaft angle mild enough that the tooth is
    /// undercut on both sides of the comparison.
    #[test]
    fn a_crossed_pair_says_what_a_parallel_one_says_about_its_teeth() {
        let lib = library();
        let pair = |shaft_angle: f64| {
            let mut sp = PairStage {
                shaft_angle,
                ..PairStage::default()
            };
            // Small enough at zero shift to be eaten into by a standard rack.
            sp.gears[0].teeth = 9;
            sp.gears[0].profile_shift = Auto::fixed(0.0);
            sp.gears[0].no_undercut = false;
            sp.gears[1].teeth = 23;
            let mut t = two_stage();
            t.stages = vec![Stage::Spur(sp)];
            t
        };

        let flat = solve_train(&pair(0.0), &lib).expect("the parallel pair solves");
        let spur = flat.stages[0]
            .as_pair()
            .expect("parallel answers as a spur result");
        let said: Vec<&str> = spur.gears[0]
            .clamps
            .iter()
            .chain(&spur.gears[0].notes)
            .map(|n| n.key.as_str())
            .collect();
        assert!(
            said.contains(&"clamp.tooth_undercut"),
            "the fixture must undercut its pinion or it checks nothing: {said:?}"
        );

        // 20°: a 10° helix, so the transverse geometry is near enough the
        // parallel one that the same pinion is still undercut.
        let angled = solve_train(&pair(20.0), &lib).expect("the crossed pair solves");
        let screw = angled.stages[0]
            .as_pair()
            .expect("a crossed pair answers as a pair");
        let gear = &screw.gears[0];
        let crossed: Vec<&str> = gear
            .clamps
            .iter()
            .chain(&gear.notes)
            .map(|n| n.key.as_str())
            .collect();
        assert!(
            crossed.contains(&"clamp.tooth_undercut"),
            "the same pinion, shafts crossed, says {crossed:?} — an axis angle \
             is not what decides whether a cutter ate into a flank"
        );
    }

    /// **Every member of a stage that reacts a load reports its share of it.**
    ///
    /// A walk over `StageResult::members()` rather than over five named field
    /// paths, which is what that accessor is for: the fault it is looking for is
    /// a member quietly missing a figure, and a sweep that names the kinds can
    /// only miss it in the kind nobody named. F30 — a self-locking worm's wheel
    /// reporting 2.2e307 N·m — was in the fifth.
    ///
    /// The claim is deliberately the weak one: **present, finite, and signed
    /// like the stage's.** A *quantitative* law across kinds does not exist, and
    /// finding that out is what this test cost. A parallel-axis member's forward
    /// torque is a geometric projection with no efficiency in it, so the ratio
    /// of backward to forward is the same for both members. A screw pair's
    /// output torque carries a forward efficiency that the backward load does
    /// not share, so its two members differ by exactly `1/η_forward` — by
    /// construction, and correctly — which is why each kind projects each case's
    /// torque through its own construction in that case's direction.
    #[test]
    fn every_member_of_a_reacting_stage_reports_its_share() {
        let lib = library();
        let mut train = two_stage();
        train.load_cases[BACK].torque = 0.5;
        // **A self-locking stage at the input end**, or nothing reacts the load
        // and the case is correctly zero at every gear — which would leave this
        // walking an empty list and passing for the wrong reason. The load
        // enters at the output and walks up; a worm stops it, and every stage
        // between it and the output carries it.
        train.stages.insert(0, Stage::Worm(PairStage::worm()));
        train.stages.push(Stage::Planetary(Box::default()));
        train.stages.push(Stage::Spur(PairStage {
            shaft_angle: 90.0,
            ..PairStage::default()
        }));

        let r = solve_train(&train, &lib).expect("a train that solves");
        let (mut checked, mut kinds) = (0u32, 0u32);
        for (k, stage) in r.stages.iter().enumerate() {
            let members = stage.members();
            if members.is_empty() {
                continue;
            }
            kinds += 1;
            for g in members {
                let back = g.cases[BACK].torque;
                let forward = g.cases[PEAK].torque;
                checked += 1;
                assert!(
                    back.is_finite(),
                    "stage {k}: a member reports {back} N·m of the load from the end"
                );
                assert!(
                    forward == 0.0 || back == 0.0 || back.signum() == forward.signum(),
                    "stage {k}: {back} against a forward torque of {forward} — a reacted \
                     load that turns a gear the other way from the drive is a \
                     claim, not a rounding"
                );
            }
        }
        assert!(kinds >= 3, "only {kinds} stage kinds contributed members");
        assert!(checked >= 6, "only {checked} members carried the load");
    }

    /// **The search refines what its sweep found, and a narrow box does not stop
    /// it.**
    ///
    /// Checked against a scan of the same admissible interval at the search's own
    /// stopping distance — an instrument that shares no step, no direction and no
    /// budget with the walk, which is the kind of check this repository trusts.
    ///
    /// **The fault it is for.** The walk's first step is a fraction of the
    /// sweep's spacing, and the sweep's spacing is a fraction of the *box*. Pin a
    /// centre distance and the box is a fraction of a module wide, at which point
    /// that first step falls **below the distance the walk stops at** — so
    /// `step > resolution` was false before the body ran, the walk took no step
    /// at all, and the answer was the best of thirteen grid points. On 9/37 at a
    /// shift sum of 0.56 the optimum sits four ten-thousandths above the undercut
    /// floor, between two of them: **3.6e-5** of efficiency, on the path a given
    /// centre distance takes.
    ///
    /// The tolerance is the resolution's own worth. Near the floor the efficiency
    /// moves by a few parts in a million across one thousandth of a module, and
    /// no search that stops there can do better — which is a statement about the
    /// stopping distance rather than about the walk.
    #[test]
    fn the_search_beats_a_scan_of_the_same_interval() {
        use crate::auto::{Bounds, Pinned, Search};
        let lib = library();
        let mut worst = 0.0_f64;
        for teeth in [[9_u32, 37], [12, 29]] {
            let stage = PairStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..PairStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PairStage::default()
            };
            let asked = [0, 1].map(|i| stage.gears[i].shift_asked(&stage.base_params(i)));
            let bounds = Bounds {
                floor: asked.map(|a| a.search_floor),
                min_contact_ratio: stage.optimisation.min_contact_ratio,
                clearance: stage.clearance.manual,
            };
            let pair = |q: [f64; 2]| [0, 1].map(|i| stage.params_at(i, q[i]));
            // The stage's own answer at a shift pair, and `None` where the pair
            // is one no search may choose — asked of the search itself with both
            // shifts pinned, so the scan and the walk agree about what is
            // admissible and differ only in how they look.
            let at = |x: [f64; 2]| {
                crate::auto::shifts_for_efficiency(
                    &pair,
                    crate::mesh::MeshKind::External,
                    &bounds,
                    &Pinned {
                        shift: [Some(x[0]), Some(x[1])],
                        sum: None,
                    },
                    stage.sliding_friction,
                    &Search::SHIPPED,
                )?;
                let fixed = PairStage {
                    gears: [0, 1].map(|i| StageGear {
                        profile_shift: Auto::fixed(x[i]),
                        ..stage.gears[i].clone()
                    }),
                    optimisation: Optimisation::default(),
                    ..stage.clone()
                };
                solve_spur_stage(&fixed, &StageLoads::just(2.0), &lib)
                    .ok()
                    .map(|r| r.mesh.efficiency.forward)
            };

            for k in 0..=8 {
                let sum = 0.5 + f64::from(k) * 0.075;
                let found = crate::auto::shifts_for_efficiency(
                    &pair,
                    crate::mesh::MeshKind::External,
                    &bounds,
                    &Pinned {
                        shift: [None, None],
                        sum: Some(sum),
                    },
                    stage.sliding_friction,
                    &Search::SHIPPED,
                )
                .and_then(at);
                // The scan, over the whole interval either shift could take, at
                // the step the walk stops at.
                let mut best: Option<f64> = None;
                let mut x0 = -0.5;
                while x0 < 1.5 {
                    if let Some(e) = at([x0, sum - x0]) {
                        best = Some(best.map_or(e, |b: f64| b.max(e)));
                    }
                    x0 += Search::SHIPPED.resolution;
                }
                let (Some(found), Some(best)) = (found, best) else {
                    continue;
                };
                worst = worst.max(best - found);
                assert!(
                    best - found < 1e-5,
                    "{teeth:?} at a shift sum of {sum}: the search keeps {found} \
                     where a scan of the same interval finds {best}"
                );
            }
        }
        assert!(
            worst > 0.0,
            "the scan never beat the search anywhere, so it is not measuring what \
             it is meant to"
        );
    }

    /// **Every kind that searches asks the same of its meshes.**
    ///
    /// A constraint belongs to the mesh, not to the arrangement around it: a
    /// mesh whose teeth reach past the root circle they run into bottoms out
    /// whether a carrier is turning about it or not. Three kinds each wrote the
    /// question out for themselves and between them answered it three ways — the
    /// pair asked about bottoming, neither epicyclic kind did; the pair and the
    /// set asked whether each member could be cut, and a ring was asked by
    /// nobody.
    ///
    /// `auto::MeshTrial` is the one place now, and this is the claim that says
    /// so from the outside: **whatever each kind chose, the mesh it chose is one
    /// the shared contract admits.** A kind that stops asking chooses something
    /// that fails here; a kind added later that never asks fails here the first
    /// time its answer is pushed against a bound.
    #[test]
    fn every_kind_that_searches_asks_the_same_of_its_meshes() {
        use crate::auto::Search;
        let lib = library();
        let mut checked = 0u32;

        // --- a pair, rebuilt through the stage's own constructors.
        for teeth in [[9_u32, 37], [17, 43], [12, 29]] {
            let stage = PairStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..PairStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PairStage::default()
            };
            let x = stage.shifts_at(&Search::SHIPPED);
            let g = [0, 1].map(|i| Tooth::new(stage.params_at(i, x[i])));
            let zero = crate::mesh::Mesh::new(&g[0], &g[1], crate::mesh::MeshKind::External)
                .expect("the pair meshes");
            let mesh = zero
                .at(zero.a_w + stage.clearance.manual)
                .expect("...at its running distance");
            for (i, gap) in mesh
                .bottom_clearance([g[0].ra, g[1].ra], [g[0].rf, g[1].rf])
                .iter()
                .enumerate()
            {
                assert!(
                    *gap > 0.0,
                    "{teeth:?}: gear {i}'s tip bottoms out by {} mm at the shifts \
                     the search chose",
                    -gap
                );
                checked += 1;
            }
        }

        // --- an epicyclic set, both of its meshes.
        for (sun, planet) in [(17_u32, 17_u32), (24, 18), (13, 25)] {
            let mut set = PlanetaryStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PlanetaryStage::default()
            };
            set.sun.teeth = sun;
            set.planet.teeth = planet;
            set.ring.teeth = sun + 2 * planet;
            set.sun.profile_shift = Auto::automatic(0.0);
            set.ring.profile_shift = Auto::automatic(0.0);
            let b = set
                .built(set.shifts_at(&Search::SHIPPED))
                .expect("the set has geometry");
            let gaps = [
                b.sp_mesh
                    .bottom_clearance([b.sun.ra, b.planet.ra], [b.sun.rf, b.planet.rf]),
                b.pr_mesh
                    .bottom_clearance([b.planet.ra, b.ring.ra], [b.planet.rf, b.ring.rf]),
            ];
            for (m, mesh) in gaps.iter().enumerate() {
                for (i, gap) in mesh.iter().enumerate() {
                    assert!(
                        *gap > 0.0,
                        "{sun}/{planet}: mesh {m} member {i} bottoms out by {} mm",
                        -gap
                    );
                    checked += 1;
                }
            }
        }

        // --- a hula stage, from what it reports rather than from what built it.
        // Its two meshes share one crank, so the offset *is* their centre
        // distance, and the two radii are on the members' own cards.
        for n in [12_u32, 18, 30] {
            let mut stage = HulaStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..HulaStage::default()
            };
            for (gear, count) in stage.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                gear.teeth = count;
            }
            let Ok(r) = solve_hula_stage(&stage, &StageLoads::just(2.0), &lib) else {
                continue;
            };
            for mesh in 0..2 {
                // The ring is the member of the pair with more teeth, which the
                // stage's own inputs say and its cards echo as the larger tip.
                let (ring, pinion) = {
                    let (a, b) = (&r.gears[mesh * 2], &r.gears[mesh * 2 + 1]);
                    if stage.gears[mesh * 2].teeth > stage.gears[mesh * 2 + 1].teeth {
                        (a, b)
                    } else {
                        (b, a)
                    }
                };
                let gaps = [
                    (ring.root_radius - r.offset) - pinion.tip_radius,
                    (ring.tip_radius - r.offset) - pinion.root_radius,
                ];
                for (i, gap) in gaps.iter().enumerate() {
                    assert!(
                        *gap > 0.0,
                        "N {n} mesh {mesh} member {i} bottoms out by {} mm",
                        -gap
                    );
                    checked += 1;
                }
            }
        }

        assert!(checked >= 20, "only {checked} tooth spaces were measured");
    }

    /// **A search may not choose a part its tool has to alter.**
    ///
    /// `member_is_buildable` says this of a rack-cut member and declines to say
    /// it of a ring, on the reading that a rack's four questions mean nothing to
    /// one. True — and it left a ring asked *nothing at all*, so the search was
    /// free to walk past the shift where the shaper stops leaving the space that
    /// shift asked for. It did: **26 of these 30 sets** came back with a ring
    /// the cutter had capped, the shipped 13/25 choosing 2.35 modules where its
    /// space caps at 1.94. The efficiency reported for them is the efficiency of
    /// a part nobody makes.
    ///
    /// A ring is asked of its **cutter** instead (`auto::ring_is_cut_as_asked`),
    /// which is the one question it has an answer to, and it is free: every
    /// caller has already cut the ring to get the mesh it is scoring.
    ///
    /// Both epicyclic kinds, because the last time a rule reached one of them
    /// and not the other it cost a second entry in `docs/corrections.md`.
    #[test]
    fn a_search_chooses_only_parts_its_tool_leaves_alone() {
        let lib = library();
        let mut checked = 0u32;
        for sun in [11_u32, 13, 17, 19, 24, 31] {
            for planet in [14_u32, 17, 18, 21, 25] {
                let mut set = PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: true,
                        ..Optimisation::default()
                    },
                    ..PlanetaryStage::default()
                };
                set.sun.teeth = sun;
                set.planet.teeth = planet;
                set.ring.teeth = sun + 2 * planet;
                set.sun.profile_shift = Auto::automatic(0.0);
                set.ring.profile_shift = Auto::automatic(0.0);
                let Ok(r) = solve_planetary_stage(&set, &StageLoads::just(2.0), &lib) else {
                    continue;
                };
                checked += 1;
                for (name, g) in [
                    ("sun", &r.sun),
                    ("planet", &r.planet.gear),
                    ("ring", &r.ring),
                ] {
                    assert!(
                        g.clamps.is_empty(),
                        "{sun}/{planet}: the search chose a {name} at x = {} that its \
                         tool had to alter — {:?}",
                        g.profile_shift,
                        g.clamps.iter().map(|c| c.key.clone()).collect::<Vec<_>>()
                    );
                }
            }
        }
        assert!(checked >= 25, "only {checked} sets solved at all");
    }

    /// **Ask for the centre distance the tool chose and get the gears it chose.**
    ///
    /// A stage with the shift optimiser on and no centre distance picks both;
    /// with a distance given it picks the shifts that reach it. Those are the
    /// same question asked twice, so at the distance the free search *itself*
    /// settled on the two must agree — and the second must not be worse anywhere
    /// short of it, since a nearer distance is a shift sum the free search
    /// passed through on its way.
    ///
    /// **It did not.** `auto::maximise` opened on a grid at multiples of half a
    /// module across a fixed `±3`; pin the sum and the admissible interval in one
    /// shift can be a tenth of that and fall between two grid points, at which
    /// point the search reports *no admissible pair* and the stage drops to its
    /// undercut floor. A 9/37 pair asked for the 24.42 mm it had just
    /// recommended came back at **97.289 %** against the **97.706 %** it offered
    /// unasked, at shifts that do not reach the distance at all. The sweep's
    /// resolution had become a feasibility test (`docs/corrections.md`).
    ///
    /// The box is the members' own admissible interval now
    /// (`auto::searchable_shift`), so the sweep spans the answer by construction.
    #[test]
    fn a_given_distance_gets_the_gears_the_free_search_would_choose() {
        let lib = library();
        for teeth in [[9_u32, 37], [17, 43], [12, 29], [23, 61]] {
            let stage = PairStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..PairStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PairStage::default()
            };
            let free = solve_spur_stage(&stage, &StageLoads::just(2.0), &lib)
                .expect("the pair solves with the distance free");

            let at = |a: f64| {
                solve_spur_stage(
                    &PairStage {
                        centre_distance: Auto::fixed(a),
                        ..stage.clone()
                    },
                    &StageLoads::just(2.0),
                    &lib,
                )
                .expect("...and with it given")
            };

            // At the distance it chose: the same answer, to **twice** the
            // search's own stopping distance in the shifts it is reading.
            //
            // Twice rather than once because of where these answers now sit. The
            // interference refusal puts a small pinion's optimum **on a wall**,
            // and the two searches arrive at that wall along different
            // coordinates — one free in both shifts, the other with their sum
            // pinned — so each resolves it to its own last step and the two
            // steps need not be the same one. One step apart would be a claim
            // about the walk; two is a claim about the wall, which is what this
            // is measuring.
            let given = at(free.centre_distance);
            for i in 0..2 {
                let (a, b) = (free.gears[i].profile_shift, given.gears[i].profile_shift);
                assert!(
                    (a - b).abs() < 2.0 * crate::auto::Search::SHIPPED.resolution,
                    "{teeth:?} gear {i}: free chose {a} and {:.6} mm given chose {b}",
                    free.centre_distance
                );
            }

            // ...and on the way there, every distance is answered rather than
            // refused, with an efficiency that climbs toward the free answer
            // rather than collapsing to the floor.
            let mut last = 0.0;
            let steps = 12;
            for k in 1..=steps {
                let a = free.centre_distance
                    - (free.centre_distance - stage.module * 0.5 * f64::from(teeth[0] + teeth[1]))
                        * f64::from(steps - k)
                        / f64::from(steps);
                let here = at(a).mesh.efficiency.forward;
                assert!(
                    here > last - 1e-9,
                    "{teeth:?}: {a:.4} mm gives {here}, below the {last} a tighter \
                     distance gave — the search dropped out somewhere between"
                );
                last = here;
            }
            // The same ten parts in a million `the_search_is_converged_not_budgeted`
            // holds a pair to, and for the same reason: both answers sit on the
            // interference wall and each resolves it to its own last step.
            assert!(
                (last - free.mesh.efficiency.forward).abs() < 1e-5,
                "{teeth:?}: the last step reaches {last} where the free answer is {}",
                free.mesh.efficiency.forward
            );
        }
    }

    /// **The search is converged where its coordinates are the problem's, and
    /// not where they are not.**
    ///
    /// `auto::Search` carries six numbers that say how hard to look, and the
    /// claim beside them was that raising them together buys nothing — "the
    /// pair's answer not at all to eight decimals, the set's by 2e-7". That was
    /// a comment: the numbers were constants inside the loop, so nothing could
    /// raise them and nothing could check it. They are a value for this reason
    /// alone, and checking it found the claim half true.
    ///
    /// **The claim is about the objective, not about the point.** These surfaces
    /// have flat ridges and their optima sit against constraints, so two
    /// different shifts can be equally good and demanding that the *shift* not
    /// move would assert something the model does not say. What a converged
    /// search means is that more effort finds nothing better — and nothing
    /// worse either, since more effort searching the same set cannot lose an
    /// answer it already had, and a search that does depends on its own step
    /// size for more than speed.
    ///
    /// **A pair converges and a set does not**, and the difference is
    /// coordinates. A pair is searched in its *own* two directions — the shift
    /// sum, which sets the operating pressure angle, and the division, which
    /// moves the path's ends against each other — so the flat direction is an
    /// axis and a walk climbs rather than zig-zags. A set is searched in two of
    /// its three raw shifts, which is nobody's natural coordinate: its
    /// admissible region is bounded by a curve and the optimum lies against it,
    /// so the walk slides. The audit's record (F50) carries the size, the sweep and what
    /// is to be done; the bound below is a **canary on a known fault**, pinned
    /// so it can only get smaller.
    #[test]
    fn the_search_is_converged_not_budgeted() {
        use crate::auto::Search;
        let lib = library();
        // Fourteen times the work: a sweep of 18 a side, four starts, nine times
        // the walk and a third of the stopping distance.
        let hard = Search::refined(3);
        // **Ten parts in a million of efficiency** — two orders below the last
        // digit this tool prints, and a ceiling on the last refinement rather
        // than a tolerance chosen to be met.
        //
        // Measured across the pairs below, **thirteen of the fourteen are at
        // 1e-9 or better** — an order tighter than before the interference
        // refusal existed, because eliminating an infeasible region takes a
        // ridge out of several surfaces. The fourteenth is **9/20 at 3.3e-6**,
        // and it is one of the two pairs whose optimum the refusal *moved*.
        //
        // That is the constraint boundary being resolved rather than a search
        // failing to converge. A constrained optimum sits **on a wall** — past
        // it the objective has no value at all — and a pattern walk resolves a
        // wall to its own step, so the objective there inherits the *shift*
        // resolution this test already holds the shifts to. It is the same
        // standard, read on the other axis.
        let converged = 1e-5;

        // --- pairs. 9/37 is the fixture whose surface has a second summit
        // beyond a trough, so it is the one that punishes a short walk.
        let mut worst_pair = 0.0_f64;
        for teeth in [
            [9_u32, 37],
            [9, 20],
            [11, 41],
            [12, 29],
            [13, 31],
            [17, 43],
            [17, 17],
            [20, 20],
            [23, 61],
            [31, 37],
            [41, 43],
            [10, 51],
            [14, 22],
            [16, 33],
        ] {
            let stage = PairStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..PairStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PairStage::default()
            };
            // Scored by solving the stage at the shifts each search chose, so
            // the objective is the one the tool reports rather than a second
            // spelling of it.
            let at = |x: [f64; 2]| {
                let fixed = PairStage {
                    gears: [0, 1].map(|i| StageGear {
                        profile_shift: Auto::fixed(x[i]),
                        ..stage.gears[i].clone()
                    }),
                    optimisation: Optimisation::default(),
                    ..stage.clone()
                };
                solve_spur_stage(&fixed, &StageLoads::just(2.0), &lib)
                    .map(|r| r.mesh.efficiency.forward)
            };
            let (Ok(shipped), Ok(refined)) = (
                at(stage.shifts_at(&Search::SHIPPED)),
                at(stage.shifts_at(&hard)),
            ) else {
                panic!("{teeth:?}: both answers must be buildable pairs");
            };
            worst_pair = worst_pair.max((refined - shipped).abs());
            assert!(
                (refined - shipped).abs() < converged,
                "{teeth:?}: fourteen times the work moves the pair's efficiency \
                 by {}, which is more than the search claims to be converged to",
                refined - shipped
            );
        }
        assert!(
            worst_pair > 0.0,
            "every pair gave bit-identical answers at both efforts, so this \
             fixture never made the search work"
        );

        // --- sets. Two free shifts tied by one centre distance, and the harder
        // of the two searches: its admissible region is bounded by a curve its
        // planet's absorption draws, so a walk slides along that curve instead
        // of climbing an axis, and a walk that slides is a walk that costs.
        //
        // **Its budget is asked for separately**, and that is the claim worth
        // making: quadrupled, it moves nothing *at all*. A guard says that; a
        // truncation cannot, and this was a truncation — a pool shared across
        // the starts, spent entire by the first of six, leaving 3.2e-4 of `η₀`
        // unfound on the shipped set (F50, `docs/corrections.md`).
        //
        // What is left over **was** 2e-6 — the last step of a different grid, and
        // moving both ways as the sweep is refined.
        //
        // **The interference refusal widened it, on two sets of thirty**, and
        // this is a canary on a known fault rather than a tolerance: 28 of the
        // 30 are at 1e-6 or better and most far better, while **11/17 is 4.0e-5
        // and 11/18 is 1.9e-4**. Both have the smallest sun, which is exactly
        // where a mate's tip reaching past a flank is the condition that binds.
        //
        // It is F50's diagnosis met again on a new constraint. A refused region
        // is a **wall**, past which the objective has no value at all; a pattern
        // walk in the raw shifts steps along axes and a wall drawn diagonally
        // across them is a wall it zig-zags down, resolving it to its own step
        // rather than to the answer. The remedy is the one F50 names — search
        // the coordinate the constraint is flat in, not across it — and it is
        // recorded with its size in `docs/state.md` rather than left to be
        // rediscovered.
        //
        // **Pinned so it can only get smaller.** Widening this is not a fix.
        let settled = 2e-4;
        let mut worst_set = 0.0_f64;
        for sun in [11_u32, 13, 17, 19, 24, 31] {
            for planet in [14_u32, 17, 18, 21, 25] {
                let mut set = PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: true,
                        ..Optimisation::default()
                    },
                    ..PlanetaryStage::default()
                };
                set.sun.teeth = sun;
                set.planet.teeth = planet;
                set.ring.teeth = sun + 2 * planet;
                set.sun.profile_shift = Auto::automatic(0.0);
                set.ring.profile_shift = Auto::automatic(0.0);
                let eta0 = |x: [f64; 3]| {
                    let b = set.built(x).ok()?;
                    let one = |path, mesh, g, mu| {
                        crate::contact::efficiency(
                            path,
                            mesh,
                            g,
                            mu,
                            crate::contact::Drive::Forward,
                        )
                    };
                    Some(
                        one(
                            &b.sp_path,
                            &b.sp_mesh,
                            &b.sun,
                            set.sliding_friction_sun_planet,
                        ) * one(
                            &b.pr_path,
                            &b.pr_mesh,
                            &b.planet,
                            set.sliding_friction_planet_ring,
                        ),
                    )
                };
                let (Some(shipped), Some(refined)) = (
                    eta0(set.shifts_at(&Search::SHIPPED)),
                    eta0(set.shifts_at(&hard)),
                ) else {
                    continue;
                };
                worst_set = worst_set.max((refined - shipped).abs());
                assert!(
                    (refined - shipped).abs() < settled,
                    "{sun}/{planet}: fourteen times the work moves `η₀` by {}",
                    refined - shipped
                );
                // ...and the budget alone, which must not move it by a bit. It
                // is the one number here that is a guard rather than a
                // resolution, so it is the one that can be asked for exactly.
                let generous = eta0(set.shifts_at(&Search {
                    budget: Search::SHIPPED.budget * 4,
                    ..Search::SHIPPED
                }));
                assert_eq!(
                    generous,
                    Some(shipped),
                    "{sun}/{planet}: four times the budget moves the answer, so it \
                     is a ceiling the walk is running into rather than a guard"
                );
            }
        }
        assert!(
            worst_set > 0.0,
            "every set gave bit-identical answers at both efforts, so this \
             fixture never made the search work"
        );
    }

    /// **A member is rated at the load it carries, whichever way it carries it.**
    ///
    /// Every rating here is linear in the member's own torque, or the square root
    /// of it, so two load cases stand in exactly the ratio of the two torques the
    /// member reports in them — the case from the start, and the case from the
    /// end distributed the other way. Checkable from the outputs alone, without
    /// knowing what any of them should be; and a third case at the first's
    /// torque and direction reproduces it to the bit.
    ///
    /// **The fault it is for.** A load case was collapsed to one magnitude *at
    /// the stage's input shaft* and that magnitude pushed through the forward
    /// construction. That is the same answer only where the distribution does not
    /// depend on direction — a parallel-axis mesh carries one tangential force
    /// whichever way it turns — and an epicyclic set is not such a stage: which
    /// shaft drives decides on which side `η₀` multiplies. Measured on the shipped
    /// set, a back-driven ring was rated **6.0 % low** in bending and 3.0 % low in
    /// contact; on the hula stage, whose reduction is far larger, **41 % low** and
    /// 23 % low. The member torques themselves had already been put right
    /// (`docs/corrections.md`); the ratings had not, and nothing compared them.
    ///
    /// Every kind that reports per-member stresses is walked, through
    /// `StageResult::members()` rather than five named paths, for the reason that
    /// accessor exists.
    #[test]
    fn a_member_is_rated_at_the_load_it_carries() {
        let lib = library();
        // **Both sides of the ratio.** A small load from the end leaves every
        // member loaded harder from the start; a large one puts every member
        // harder on the backward distribution, which is the case the fault was
        // in — and the two must give the same law.
        let (mut checked, mut dominated, mut forward_won) = (0u32, 0u32, 0u32);
        for applied in [1.0e2_f64, 1.0e10] {
            let mut train = two_stage();
            train.load_cases[BACK].torque = applied;
            train.stages.insert(0, Stage::Worm(PairStage::worm()));
            train.stages.push(Stage::Planetary(Box::default()));
            train.stages.push(Stage::Hula(Box::default()));

            let r = solve_train(&train, &lib).expect("a train that solves");
            for (k, stage) in r.stages.iter().enumerate() {
                // A point contact is rated on the worse of two *constructions*
                // — the input torque on the worm forward, the applied load on
                // the wheel backward — so its members' stresses are not a
                // projection of their own torques and this law is not theirs.
                // The worm stage in this train is here for the walk, and its
                // own rating is gated in `crossed::tests`.
                if stage.as_pair().is_some_and(|p| p.mesh.point.is_some()) {
                    continue;
                }
                for (i, g) in stage.members().iter().enumerate() {
                    let (start, end, again) = (&g.cases[PEAK], &g.cases[BACK], &g.cases[CYCLIC]);
                    let forward = start.torque.abs();
                    let back = end.torque.abs();
                    assert!(
                        forward > 0.0 && back > 0.0,
                        "stage {k} member {i} carries {forward} from the start and \
                         {back} from the end, so this law has no ratio to check"
                    );
                    let want = back / forward;
                    if want > 1.0 {
                        dominated += 1;
                    } else {
                        forward_won += 1;
                    }
                    checked += 1;
                    if let (Some(a), Some(b)) = (start.bending_stress, end.bending_stress) {
                        assert!(
                            (b / a - want).abs() < 1e-9 * want,
                            "stage {k} member {i}: bending {b} against {a} is {}, \
                             where {back} N·m from the end and {forward} N·m from \
                             the start make {want}",
                            b / a
                        );
                    }
                    let (a, b) = (start.contact_stress, end.contact_stress);
                    assert!(
                        (b / a - want.sqrt()).abs() < 1e-9 * want.sqrt(),
                        "stage {k} member {i}: contact {b} against {a} is {}, \
                         where the torques make {}",
                        b / a,
                        want.sqrt()
                    );
                    // The same load again, the same way, is the same answer.
                    assert_eq!(start.bending_stress, again.bending_stress);
                    assert_eq!(start.contact_stress, again.contact_stress);
                }
            }
        }
        assert!(checked >= 18, "only {checked} members carried the load");
        assert!(
            dominated >= 9 && forward_won >= 9,
            "{dominated} members were loaded harder backward and {forward_won} \
             harder forward — both sides of the maximum have to be reached or \
             this only checks one of them"
        );
    }

    /// **A back-driving load reaches the wheel undiminished**, and the only
    /// thing the direction changes about the rating is which flank carries it.
    ///
    /// A screw pair is the kind whose distribution depends on direction: driving
    /// forward the wheel carries the worm's torque stepped up *and cut by the
    /// mesh's own loss*, while a back-driving load arrives at the wheel already.
    /// So a stage rated at `max(T_in, T_back)` and then stepped up and cut once
    /// is rating a back-driven pair at `η_forward` of its load — 62 % of it on
    /// the shipped worm, and 85 % of the pressure once a cube root has been
    /// through it.
    ///
    /// Asserted by putting the same torque on the wheel twice, from either end.
    /// The two answers are **not** identical and should not be: back-driving
    /// loads the other flank, which flips the normal term in the balance and
    /// leaves the friction term alone, and this pair's two flanks differ by half
    /// a percent. What makes this a test rather than a tolerance is the second
    /// bound — the gap between the two readings has to stay far smaller than the
    /// gap the old fault opened, or "the same load, the other flank" and "the
    /// wrong load" are not being told apart.
    #[test]
    fn a_back_driving_load_reaches_the_wheel_undiminished() {
        let lib = library();
        let load = 400.0;

        let driven = |input_torque: f64, back: f64| {
            let mut train = two_stage();
            train.load_cases[PEAK].torque = input_torque;
            train.load_cases[CYCLIC].torque = input_torque;
            train.load_cases[BACK].torque = back;
            train.stages = vec![Stage::Worm(PairStage::worm())];
            let r = solve_train(&train, &lib).expect("a train that solves");
            let (w, m) = worm(&r.stages[0]);
            // Whichever end the torque is put on, its case is the one read.
            let case = if back > 0.0 { BACK } else { PEAK };
            (
                m.cases[case].contact.max_pressure,
                w.ratio,
                m.efficiency.forward,
            )
        };

        // Driven backward at `load` on the wheel, with nothing at all coming the
        // other way — zero is a torque, and a stage carrying none of it forward
        // still carries this.
        let (from_output, ratio, eta) = driven(0.0, load);
        // ...and driven forward hard enough to put the same torque on the wheel.
        // `|ratio|`: the reduction is signed now and a worm's is negative, and
        // what refers a torque is its size — the same reading `carry` takes.
        let (from_input, _, _) = driven(load / (ratio.abs() * eta), 0.0);

        let apart = (from_output / from_input - 1.0).abs();
        assert!(
            apart < 0.02,
            "the same {load} N·m on the wheel rates at {from_output} MPa reached              from the output and {from_input} MPa reached from the input"
        );
        // The fault this is really about: `η` of the load, under a cube root.
        let old_fault = 1.0 - eta.cbrt();
        assert!(
            apart < old_fault / 5.0,
            "a flank swap moves the rating by {apart}, and rating the load at              η_forward would move it by {old_fault} — too close to tell apart"
        );

        // And it is the load itself that arrives, so the rating follows Hertz's
        // cube root of it exactly rather than approximately.
        let (doubled, _, _) = driven(0.0, 2.0 * load);
        assert!(
            (doubled / from_output - 2.0_f64.cbrt()).abs() < 1e-12,
            "twice the back-driving load should be 2^(1/3) of the pressure: {}",
            doubled / from_output
        );
    }

    /// **A pair that transmits nothing still has its flanks pressed.**
    ///
    /// A screw mesh's rating used to be taken from the torque on its **wheel**,
    /// `T_in · i · η_forward` — and `Directional::once_moving` clamps a locked
    /// pair's efficiency to zero, so that product is zero and the pair reported
    /// no flank load at all. It is not zero: something is holding the wheel, and
    /// whatever holds it is pressing the teeth.
    ///
    /// Reached at a helix split of 9° / 81° on a crossed 17/23 pair, which
    /// `gear-cli crossed 17 23 90` prints — the sliding is over six times the
    /// pitch-line speed there and the mesh cannot drive forward at µ = 0.06.
    /// Rated from the torque the stage was **given**, on the member it was given
    /// on, the answer is in line with its neighbours in that sweep rather than
    /// absent from it.
    #[test]
    fn a_pair_that_transmits_nothing_still_has_its_flanks_pressed() {
        let lib = library();
        let locked = PairStage {
            shaft_angle: 90.0,
            gears: [
                StageGear {
                    teeth: 17,
                    ..PairStage::worm().gears[0].clone()
                },
                StageGear {
                    teeth: 23,
                    ..PairStage::worm().gears[1].clone()
                },
            ],
            ..PairStage::worm()
        }
        .with_first_helix(9.0);
        let r = solve_worm_stage(&locked, &StageLoads::just(2.0), &lib)
            .expect("a locked pair is still a pair");
        let r_point = r.mesh;
        assert_eq!(
            r_point.efficiency.forward, 0.0,
            "this split is meant to be the forward-locked one"
        );
        assert!(
            r_point.cases[0].contact.max_pressure > 100.0,
            "a locked pair's flanks are pressed by whatever holds them: {} MPa",
            r_point.cases[0].contact.max_pressure
        );
        // Not merely non-zero: the same 2 N·m through a split that *does* drive
        // presses about as hard, because the flank load comes from the input
        // torque either way and the geometry has not changed much.
        let driving = PairStage { ..locked }.with_first_helix(18.0);
        let d = solve_worm_stage(&driving, &StageLoads::just(2.0), &lib).expect("and this one");
        let d_point = d.mesh;
        let ratio = r_point.cases[0].contact.max_pressure / d_point.cases[0].contact.max_pressure;
        assert!(
            (0.5..2.0).contains(&ratio),
            "the locked split rates at {} MPa against the driving split's {}",
            r_point.cases[0].contact.max_pressure,
            d_point.cases[0].contact.max_pressure
        );
    }

    /// **Zero is a torque.**
    ///
    /// A train's fatigue case is a load like any other, and nothing stops it
    /// being nought: a mechanism that is held rather than driven runs at no load
    /// and still has to be rated for the peak it sees. Every parallel-axis kind
    /// answered; a worm stage returned `NoContact` and took the whole train down
    /// with it, because the Hertz point solution refused a zero force where the
    /// limit is a closed form — the patch closes to a point and the pressure with
    /// it (`crate::hertz::elliptical_contact`).
    ///
    /// Asserted on every kind rather than on the one that failed, since what is
    /// being claimed is a property of the tool and not a patch to a stage.
    #[test]
    fn a_stage_carrying_nothing_is_a_stage() {
        let lib = library();
        for stage in [
            Stage::Spur(PairStage::default()),
            Stage::Worm(PairStage::worm()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::default()),
        ] {
            let mut train = two_stage();
            train.stages = vec![stage];
            train.load_cases[CYCLIC].torque = 0.0;
            let r = solve_train(&train, &lib)
                .unwrap_or_else(|e| panic!("a stage at no operating load: {e:?}"));
            // A worm's members are not gears, so the walk below is empty there
            // and the claim is the stage's own — which is the one that failed.
            if let Some(m) = r.stages[0]
                .as_pair()
                .filter(|p| p.mesh.point.is_some())
                .map(|p| &p.mesh)
            {
                assert_eq!(m.cases[1].contact.max_pressure, 0.0);
                assert!(m.cases[0].contact.max_pressure > 0.0);
            }
            for (i, g) in r.stages[0].members().iter().enumerate() {
                assert_eq!(
                    g.cases[1].contact_stress, 0.0,
                    "member {i} carries nothing and reports a stress"
                );
                assert!(
                    g.cases[1].bending_stress.is_none_or(|s| s == 0.0),
                    "member {i} carries nothing and reports {:?}",
                    g.cases[1].bending_stress
                );
                assert!(
                    g.cases[0].contact_stress > 0.0,
                    "member {i} still has a peak to survive"
                );
            }
        }
    }

    /// **An internal mesh is asked what an internal mesh is asked, whatever is
    /// turning around it — and an external one is not asked it at all.**
    ///
    /// The three ways an internal pair's teeth can foul were four fields on a
    /// *hula stage's* mesh row, and the epicyclic set with the same ring mesh in
    /// it reported nothing: a designer was told whether the teeth foul according
    /// to which stage kind they had picked. They are on [`MeshReport`] now, and
    /// this is the walk that says every kind gets them — through
    /// [`StageResult::meshes`], which is the same shape as `members()` and
    /// exists so a walk cannot forget a kind by naming them.
    ///
    /// `None` on an external mesh is the other half of the claim, and it is not
    /// three answers of `false`: an external pair's members curve opposite ways,
    /// its tip circles cross on the line of centres or not at all, and the
    /// question does not arise. *An absent thing is not a zero-length thing.*
    /// **One `ContactPatch` for both contacts is a claim about a limit, and
    /// this is the limit measured** — with the seams named, since "meet" has a
    /// size on each field and two of them are not zero.
    ///
    /// The same 17/43 pair, a hundredth of a degree off parallel against
    /// parallel, the contact centred (no clearance, or the near-parallel
    /// contact slides along the shafts by `Δa / sin Σ`) and the face wide
    /// enough that the line governs:
    ///
    /// - **the pitch point meets exactly** at no friction — a part in 10⁵ —
    ///   and by 1.5 % at `μ = 0.08`, which is the flank load: the crossed
    ///   balance presses the flank with `μ F_n` along a sliding direction that
    ///   stays finite as the sliding speed vanishes, and the line rating
    ///   presses it with the transverse projection alone, as ISO does. That
    ///   is a seam between two conventions and is recorded rather than closed;
    /// - **the worst point does not meet**, by 5 %, because *one pair carries
    ///   everything* means a different thing for a line and a point: the line
    ///   rating's single-pair points are a transverse base pitch in from the
    ///   path's ends, the point's a normal base pitch in along its line, and
    ///   the two differ by `cos² β_b`. The zones themselves agree to a micron;
    /// - the curvature across and the patch width travel with the worst point
    ///   and sit within a tenth; the sliding at the pitch point closes on the
    ///   line's zero, and the efficiency meets to first order, as its own gate
    ///   says.
    ///
    /// The count is a different measure on each and is not compared
    /// ([`MeshReport::contact_ratio`]).
    #[test]
    fn the_two_contacts_report_one_patch_at_the_limit() {
        let lib = library();
        let stage = |sigma: f64, mu: f64| {
            PairStage {
                shaft_angle: sigma,
                sliding_friction: mu,
                static_friction: mu,
                clearance: Auto::fixed(0.0),
                gears: [17u32, 43].map(|teeth| StageGear {
                    teeth,
                    face_width: Auto::fixed(30.0),
                    ..StageGear::default()
                }),
                ..PairStage::default()
            }
            .with_additional_helix(20.0)
        };
        let mesh = |sigma: f64, mu: f64| {
            solve_spur_stage(&stage(sigma, mu), &StageLoads::just(2.0), &lib)
                .expect("a pair either way")
                .mesh
        };
        let close = |name: &str, a: f64, b: f64, tol: f64| {
            assert!(
                ((a - b) / a).abs() < tol,
                "{name}: the line contact reports {a} and the point contact {b}"
            );
        };

        // Frictionless, the pitch point is one number.
        let (line, point) = (mesh(0.0, 0.0), mesh(0.01, 0.0));
        assert!(line.line.is_some() && line.point.is_none());
        assert!(point.point.is_some() && point.line.is_none());
        let (l, p) = (line.cases[0].contact, point.cases[0].contact);
        close(
            "pressure at the pitch point, μ = 0",
            l.at_pitch_point,
            p.at_pitch_point,
            1e-4,
        );
        // The worst point is a different point — see above — and is not one.
        close("peak pressure", l.max_pressure, p.max_pressure, 6e-2);
        assert!(
            (l.max_pressure - p.max_pressure) / l.max_pressure > 3e-2,
            "the single-pair seam is real, and this test would be asserting agreement it does \
             not have if it closed: {} against {}",
            l.max_pressure,
            p.max_pressure
        );
        // ...and the curvature and the width travel with it, 0.3 mm further
        // from the pinion's base on the line's reading.
        close(
            "curvature across",
            l.curvature_across,
            p.curvature_across,
            1e-1,
        );
        close("patch width", l.patch_width, p.patch_width, 1e-1);
        assert_eq!(
            l.curvature_along, 0.0,
            "a line contact is the ellipse at no curvature along"
        );
        assert!(
            p.curvature_along < 1e-6 * p.curvature_across,
            "and a hundredth of a degree off parallel it nearly is: {}",
            p.curvature_along
        );
        assert_eq!(line.sliding_ratio, 0.0);
        assert!(
            point.sliding_ratio < 1e-3,
            "the lengthwise sliding closes on the line's zero: {}",
            point.sliding_ratio
        );
        assert!(line.contact_ratio > 1.0 && point.contact_ratio > 1.0);

        // With friction, the flank load is the seam: 1.5 % at the pitch point.
        let (line, point) = (mesh(0.0, 0.08), mesh(0.01, 0.08));
        let gap = (line.cases[0].contact.at_pitch_point - point.cases[0].contact.at_pitch_point)
            / line.cases[0].contact.at_pitch_point;
        assert!(
            (0.005..0.03).contains(&gap),
            "the friction seam at the pitch point is {gap}, and it is the flank load \
             convention — not nothing, and not more than that"
        );
        close(
            "efficiency",
            line.efficiency.forward,
            point.efficiency.forward,
            2e-3,
        );
    }

    #[test]
    fn an_internal_mesh_is_asked_what_an_internal_mesh_is_asked() {
        let lib = library();
        // Which of each kind's meshes have a ring in them, in the order
        // `meshes()` returns them: a pair none, a set its second, a hula both.
        for (stage, internal) in [
            (Stage::Spur(PairStage::default()), vec![false]),
            (Stage::Planetary(Box::default()), vec![false, true]),
            (Stage::Hula(Box::default()), vec![true, true]),
            // A worm's one mesh is external too, and it is in the walk now:
            // a point contact answers in the same report as a line.
            (Stage::Worm(PairStage::worm()), vec![false]),
        ] {
            let mut train = two_stage();
            train.stages = vec![stage];
            let r = solve_train(&train, &lib).expect("every shipped kind solves");
            let meshes = r.stages[0].meshes();
            assert_eq!(
                meshes.len(),
                internal.len(),
                "the walk found a different number of meshes than the kind has"
            );
            for (mesh, is_internal) in meshes.iter().zip(&internal) {
                assert_eq!(
                    mesh.tips.is_some(),
                    *is_internal,
                    "tip room is an internal mesh's question and only an internal mesh's"
                );
            }
        }
    }

    /// **A full-depth internal pair interferes, and a shipped epicyclic set is
    /// one.**
    ///
    /// `ring::mesh_with` has said so since it existed — the ring's tip can only
    /// touch the pinion's involute while `√(r_a2² − r_b2²) ≥ a sin α_w`, and a
    /// standard ring misses it — but nothing ever put the question to a *set*,
    /// whose default ring is full-depth at zero shift. So the answer here is
    /// `true`, and it is asserted rather than fixed: it is a statement about
    /// what the shipped proportions are, and the remedy is the ring's addendum,
    /// which is an input.
    ///
    /// Both halves are pinned, because a canary that only says "it interferes"
    /// would pass if every set interfered for a new reason. Shortening the ring
    /// clears it, which is the same remedy `ring.rs` records and is where the
    /// rule of thumb about internal tooth differences comes from.
    #[test]
    fn a_shipped_sets_full_depth_ring_interferes_and_a_shorter_tooth_clears_it() {
        let lib = library();
        let solved = |addendum: f64| {
            let mut set = PlanetaryStage::default();
            set.ring.addendum = addendum;
            crate::train::solve_planetary_stage(&set, &StageLoads::just(2.0), &lib)
                .expect("the shipped set solves")
        };
        // **Read off the general flags now**, which is where the classical pair
        // live: `[0]` is the pinion's flank reached by the ring's tip — what the
        // literature calls involute interference — and `[1]` is the ring's flank
        // reached by the pinion's, which it calls trochoid.
        let full_mesh = solved(1.0).planet_ring;
        let full = full_mesh.tips.expect("an internal mesh");
        assert!(
            full_mesh.flank_interference[0],
            "a full-depth ring should interfere: {:?}",
            full_mesh.flank_interference
        );
        assert!(
            !full_mesh.flank_interference[1] && !full.tip_interference,
            "and it should be the involute one alone that bites: {:?} {full:?}",
            full_mesh.flank_interference
        );
        let short_mesh = solved(0.75).planet_ring;
        let short = short_mesh.tips.expect("an internal mesh");
        assert!(
            short.tips_clear() && short_mesh.flank_interference == [false, false],
            "shortening the ring's tooth should clear it: {:?} {short:?}",
            short_mesh.flank_interference
        );
    }

    /// **A parallel-axis pair's two members share one tangential force**, so the
    /// load they react is in the ratio of their tooth counts — and nothing about
    /// efficiency enters, because within a stage the load has not been
    /// attenuated yet.
    ///
    /// The quantitative half of the walk above, asserted where a quantitative
    /// law exists.
    #[test]
    fn a_parallel_pairs_reacted_load_is_in_the_tooth_count_ratio() {
        let lib = library();
        let mut train = two_stage();
        train.load_cases[BACK].torque = 0.5;
        train.stages.insert(0, Stage::Worm(PairStage::worm()));

        let r = solve_train(&train, &lib).expect("a train that solves");
        let mut checked = 0u32;
        for (k, stage) in r.stages.iter().enumerate() {
            // Parallel axes only: a worm's members do not share a tangential
            // force, and its own two-member relation is gated in `crossed`.
            let Some(spur) = stage.as_pair().filter(|p| p.mesh.line.is_some()) else {
                continue;
            };
            let (a, b) = (
                spur.gears[0].cases[BACK].torque,
                spur.gears[1].cases[BACK].torque,
            );
            if a == 0.0 {
                continue;
            }
            checked += 1;
            // Torques are in the *size* of the ratio; the sign is the shaft's.
            let want = spur.ratio.abs();
            assert!(
                (b / a - want).abs() < 1e-9 * want.abs(),
                "stage {k}: the two members react {a} and {b}, a ratio of {}, \
                 where the pair steps up by {want}",
                b / a
            );
        }
        assert!(
            checked >= 2,
            "only {checked} parallel stages carried the load"
        );
    }

    /// The four presets, one of each kind, for a law about every kind.
    fn every_kind() -> Vec<Stage> {
        let mut hula = HulaStage::default();
        hula.gears[0].profile_shift = Auto::automatic(0.0);
        vec![
            Stage::Spur(PairStage::default()),
            Stage::Worm(PairStage::worm()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::new(hula)),
        ]
    }

    /// **Every kind, at a spread of tooth counts and every arrangement** — the
    /// grid the wiring laws below are asserted over.
    ///
    /// `every_kind` is the presets; this turns the axes that change the
    /// *topology's* answer, which is what a law about topology has to be swept
    /// over. An axis nobody turns is an axis nobody tests
    /// (`docs/corrections.md`), and for the graph that axis is the arrangement:
    /// which shaft is held is the whole of what an epicyclic set's ratio
    /// depends on, and a sweep that leaves it at the preset checks one sixth of
    /// the model.
    fn every_wiring() -> Vec<(String, Stage, StageBoundary)> {
        use crate::planetary::{Arrangement, PlanetaryShaft};
        let conventional = |name: String, stage: Stage| {
            let b = stage.conventional_boundary();
            (name, stage, b)
        };
        let mut out = Vec::new();
        for (z1, z2) in [(17_u32, 43_u32), (9, 37), (13, 13)] {
            let mut s = PairStage::default();
            s.gears[0].teeth = z1;
            s.gears[1].teeth = z2;
            out.push(conventional(format!("spur {z1}/{z2}"), Stage::Spur(s)));
        }
        out.push(conventional("worm".into(), Stage::Worm(PairStage::worm())));
        // **The arrangement is what a set is asked, not what it is**: one
        // stage per tooth count, six boundaries each.
        for teeth in [[12_u32, 30, 72], [24, 18, 60], [17, 17, 51]] {
            let mut p = PlanetaryStage::default();
            for (g, z) in [&mut p.sun, &mut p.planet, &mut p.ring]
                .into_iter()
                .zip(teeth)
            {
                g.teeth = z;
            }
            for input in PlanetaryShaft::ALL {
                for fixed in PlanetaryShaft::ALL {
                    if input == fixed {
                        continue;
                    }
                    out.push((
                        format!("set {teeth:?} {input:?} in, {fixed:?} held"),
                        Stage::Planetary(Box::new(p.clone())),
                        PlanetaryStage::boundary_for(Arrangement { input, fixed }),
                    ));
                }
            }
        }
        for teeth in [[65_u32, 61, 57, 61], [19, 18, 17, 16]] {
            let mut h = HulaStage::default();
            h.gears[0].profile_shift = Auto::automatic(0.0);
            for (g, z) in h.gears.iter_mut().zip(teeth) {
                g.teeth = z;
            }
            out.push(conventional(
                format!("hula {teeth:?}"),
                Stage::Hula(Box::new(h)),
            ));
        }
        out
    }

    /// **The graph reproduces every kind's own kinematics**, which is the whole
    /// of what the wiring has to earn.
    ///
    /// Three kinds each solve their speeds a different way —
    /// `Mesh::ratio` for a pair, `planetary::power` for a set, the two
    /// products through that same solve for a hula stage — and one system of
    /// mesh rows has to give all three. Agreeing everywhere is what says the
    /// declaration is right; **and it is the only thing that can say so**,
    /// since the lock-up invariant is silent on a wrong sign and on a
    /// misattributed frame alike (`crate::kinematics`, measured).
    ///
    /// The ratio is compared **signed and to the float**, since every kind's
    /// reported ratio is now the graph's own reading. For a while this compared
    /// magnitudes and checked the sign on the epicyclic kinds only, because a
    /// pair's ratio was `Mesh::ratio` — *"ignoring sign"* by its own
    /// documentation — while a set's was signed; that disagreement is resolved
    /// and recorded (`docs/corrections.md`), and this is where it would show up
    /// again.
    #[test]
    fn the_graph_gives_every_kind_the_kinematics_it_gives_itself() {
        let lib = library();
        let mut checked = 0u32;
        for (name, stage, b) in every_wiring() {
            let w = stage.wiring();
            let system = stage.system().unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(system.lock_up_is_free(), "{name}");
            let m = system
                .motion(&b.conditions)
                .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(
                m.is_unique(),
                "{name}: the arrangement should determine every shaft, not {m:?}"
            );

            // --- the ratio, against the kind's own, asked the same thing.
            let r = solve_any(&stage, &StageLoads::at(1.0, 1.0).under(b.clone()), &lib)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let graph = m
                .ratio(b.input, b.output)
                .unwrap_or_else(|| panic!("{name}: the output does not turn"));
            // **Signed, and exact to the float**: every kind's reported ratio
            // is the graph's own reading now (`UnitMotion::ratio`), so this is
            // the same number read twice — once through the stage and once
            // here — and anything short of equality would be a kind that had
            // grown a second source. It used to compare magnitudes and check
            // the sign only on the epicyclic kinds, because a pair's ratio was
            // `z₂/z₁` and could not say its output reversed.
            let want = r.ratio();
            assert!(
                (graph.to_f64() - want).abs() < 1e-12 * want.abs().max(1.0),
                "{name}: graph {graph} vs stage {want}"
            );

            // --- every member's speed, and its speed against its own frame.
            //
            // Solved at unit input speed, so the graph's speeds *are* the
            // stage's — which is the strongest form of this check: not a
            // ratio each, but every shaft at once.
            for (i, g) in r.members().iter().enumerate() {
                let shaft = w.mounts[i].spins_with;
                let case = &g.cases[0];
                let speed = m.values[shaft].to_f64();
                assert!(
                    (speed.abs() - case.speed.abs()).abs() < 1e-9 * case.speed.abs().max(1.0),
                    "{name}: member {i} graph {speed} vs stage {}",
                    case.speed
                );
                let frame = m.values[w.mounts[i].axis_fixed_in].to_f64();
                let against = speed - frame;
                assert!(
                    (against.abs() - case.speed_against_carrier.abs()).abs()
                        < 1e-9 * case.speed_against_carrier.abs().max(1.0),
                    "{name}: member {i} against its frame, graph {against} vs stage {}",
                    case.speed_against_carrier
                );
                checked += 1;
            }
        }
        assert!(checked > 60, "only {checked} members checked");
    }

    /// **The transpose solve gives the equilibrium `planetary::power` does**,
    /// once the loss is taken out of it.
    ///
    /// `power` folds `η₀^w` into the torque split, so the two can only be
    /// compared at `η₀ = 1` — and that is the comparison worth having: it
    /// isolates the *equilibrium* from the loss model, so a disagreement is
    /// about the rowspace and not about friction. All six arrangements, on the
    /// shipped counts and on a set whose basic ratio is close to one, which is
    /// where `power`'s two branches straddle and where its own documentation
    /// says the sign of the rolling power stops being obvious.
    #[test]
    fn the_ideal_torque_split_is_the_equilibrium_the_set_already_solves() {
        use crate::kinematics::Condition;
        use crate::planetary::{self, Arrangement, PlanetaryShaft, Teeth};
        use crate::ratio::Ratio;
        let mut checked = 0u32;
        for teeth in [
            Teeth {
                sun: 24,
                planet: 18,
                ring: 60,
            },
            Teeth {
                sun: 12,
                planet: 30,
                ring: 72,
            },
            // i₀ = −60/57, a hair from −1.
            Teeth {
                sun: 57,
                planet: 3,
                ring: 60,
            },
        ] {
            let mut stage = PlanetaryStage::default();
            for (g, z) in [&mut stage.sun, &mut stage.planet, &mut stage.ring]
                .into_iter()
                .zip([teeth.sun, teeth.planet, teeth.ring])
            {
                g.teeth = z;
            }
            for input in PlanetaryShaft::ALL {
                for fixed in PlanetaryShaft::ALL {
                    if input == fixed {
                        continue;
                    }
                    let arrangement = Arrangement { input, fixed };
                    let b = PlanetaryStage::boundary_for(arrangement);
                    let w = stage.wiring();
                    let system = Stage::Planetary(Box::new(stage.clone())).system().unwrap();
                    let Some(p) =
                        planetary::power(planetary::basic_ratio(teeth), arrangement, 1.0, 1.0, 1.0)
                    else {
                        // A lossless flow that will not solve is `power`'s own
                        // refusal and not the graph's business.
                        continue;
                    };
                    // The planet carries no external torque and nothing in a
                    // pure epicyclic meshes against ground, so both are
                    // exact zeros rather than unknowns.
                    let mut applied = vec![None; w.shafts.len()];
                    applied[crate::kinematics::GROUND] = Some(Ratio::ZERO);
                    applied[w.mounts[1].spins_with] = Some(Ratio::ZERO);
                    applied[b.input] = Some(Ratio::ONE);
                    let t = system.torques(&applied).unwrap();
                    assert!(t.is_unique(), "{teeth:?} {input:?}/{fixed:?}");
                    for role in PlanetaryShaft::ALL {
                        let shaft = match role {
                            PlanetaryShaft::Sun => w.mounts[0].spins_with,
                            PlanetaryShaft::Ring => w.mounts[2].spins_with,
                            // The carrier is the frame both meshes are seen
                            // from, and the one shaft that is not a gear.
                            PlanetaryShaft::Carrier => w.mounts[1].axis_fixed_in,
                        };
                        let got = t.values[shaft].to_f64();
                        let want = p.torques[role.index_pub()];
                        assert!(
                            (got - want).abs() < 1e-9 * want.abs().max(1.0),
                            "{teeth:?} {input:?}/{fixed:?} {role:?}: {got} vs {want}"
                        );
                        checked += 1;
                    }
                    // ...and the speeds, which the same conditions give.
                    let motion = system.motion(&b.conditions).unwrap();
                    assert!(matches!(b.conditions[b.input], Condition::Drive(_)));
                    for role in PlanetaryShaft::ALL {
                        let shaft = match role {
                            PlanetaryShaft::Sun => w.mounts[0].spins_with,
                            PlanetaryShaft::Ring => w.mounts[2].spins_with,
                            PlanetaryShaft::Carrier => w.mounts[1].axis_fixed_in,
                        };
                        let got = motion.values[shaft].to_f64();
                        let want = p.speeds[role.index_pub()];
                        assert!(
                            (got - want).abs() < 1e-9 * want.abs().max(1.0),
                            "{teeth:?} {input:?}/{fixed:?} {role:?} speed: {got} vs {want}"
                        );
                    }
                }
            }
        }
        assert!(checked >= 50, "only {checked} torques checked");
    }

    /// **A stage that will not close geometrically still has a ratio.**
    ///
    /// The fault the whole refactor opens on: a set whose two centre distances
    /// no planet shift can bring together refuses, and takes the shaft line
    /// with it — though Willis needs the tooth counts and the topology and
    /// nothing else. The wiring is what makes the second answerable
    /// independently of the first, and this holds that it is.
    #[test]
    fn a_stage_with_no_geometry_still_answers_about_motion() {
        let lib = library();
        let mut stage = PlanetaryStage::default();
        // Only `z_ring ∈ [48, 54]` admits any planet shift at 17/17.
        stage.sun.teeth = 17;
        stage.planet.teeth = 17;
        stage.ring.teeth = 80;
        let stage = Stage::Planetary(Box::new(stage));
        assert!(
            solve_any(&stage, &StageLoads::just(1.0), &lib).is_err(),
            "this set is the one that cannot be built"
        );
        let b = stage.conventional_boundary();
        let m = stage.system().unwrap().motion(&b.conditions).unwrap();
        // Sun in, ring held: the carrier runs at z_s/(z_s + z_r) of the sun,
        // whatever the geometry can or cannot be made to do.
        assert_eq!(
            m.ratio(b.input, b.output).unwrap(),
            crate::ratio::Ratio::new(17 + 80, 17).unwrap()
        );
    }

    /// **A train's motion is its stages' motions, chained** — and the chain is
    /// a coupling row rather than a product of ratios taken by hand.
    ///
    /// `turns_per_port_turn` multiplies the stage ratios to refer a speed, a
    /// sweep or a revolution count from a port to any stage. That is the
    /// graph's answer for the special case of a path, hand-derived, and this
    /// holds that the two agree — on magnitudes, since `StageResult::ratio` is
    /// signed on some kinds and not on others (recorded in the plan; resolved
    /// where the graph starts answering).
    ///
    /// It also holds the thing a product cannot say: the **mobility** of the
    /// assembled train equals the number of conditions it is given, so a chain
    /// is neither over- nor under-determined by construction, and a stage kind
    /// that introduced a shaft nothing constrains would fail here rather than
    /// quietly widening the answer.
    #[test]
    fn the_chained_graph_agrees_with_the_product_of_the_stage_ratios() {
        let lib = library();
        let mut checked = 0u32;
        for train in [two_stage(), mixed_train()] {
            let r = solve_train(&train, &lib).expect("these trains solve");
            let m = train.motion().expect("...and have a motion");

            // Exactly as many conditions as degrees of freedom.
            let (system, at) = train.system().unwrap();
            let asked = train
                .conditions(&at, system.shafts())
                .unwrap()
                .iter()
                .filter(|c| **c != crate::kinematics::Condition::Free)
                .count();
            assert_eq!(
                asked, m.mobility.degrees,
                "a chain should be exactly determined"
            );
            assert!(m.solution.is_unique());
            assert!(
                m.solution.redundant.is_empty(),
                "{:?}",
                m.solution.redundant
            );

            for (k, stage) in r.stages.iter().enumerate() {
                let graph = m.ratios[k].expect("every stage here turns").to_f64();
                assert!(
                    (graph.abs() - stage.ratio().abs()).abs() < 1e-9 * stage.ratio().abs(),
                    "stage {k}: graph {graph} vs {}",
                    stage.ratio()
                );
                checked += 1;
            }
            let total = m.total.expect("the train turns").to_f64();
            assert!(
                (total.abs() - r.total_ratio.abs()).abs() < 1e-9 * r.total_ratio.abs(),
                "total: graph {total} vs {}",
                r.total_ratio
            );
            // **The product of the stage ratios, which used to be production
            // code and is now the fixture.**
            //
            // `turns_per_port_turn` referred a port's speed to a stage by
            // multiplying out `Π i`; it reads the graph now, so comparing the
            // graph against it would be comparing a thing with itself. The
            // expression it replaced is written out here instead, which is what
            // the plan means by the old model becoming a test fixture: it is
            // still the independent answer, it is simply no longer the one the
            // tool ships.
            let ratios: Vec<f64> = r.stages.iter().map(StageResult::ratio).collect();
            let boundaries = train.boundaries().unwrap();
            for (k, b) in boundaries.iter().enumerate() {
                let graph = m.solution.values[m.shaft_of(k, b.input)].to_f64();
                let hand = 1.0 / ratios[..k].iter().product::<f64>();
                assert!(
                    (graph.abs() - hand.abs()).abs() < 1e-9 * hand.abs(),
                    "stage {k} input speed: graph {graph} vs {hand}"
                );
                checked += 1;
            }
        }
        assert!(checked >= 10, "only {checked} readings checked");
    }

    /// **A train with a stage that cannot be built still has a shaft line.**
    ///
    /// `solve_train` refuses the whole train — every ratio, every efficiency,
    /// every backlash — when any one stage will not close, though the stages
    /// beside it are fine and a ratio needs no geometry at all. That is the
    /// fault the refactor opens on; this is the law that says the two questions
    /// are separable, asserted on a train whose *first* stage is perfectly
    /// good.
    #[test]
    fn a_train_that_will_not_close_still_reports_its_ratios() {
        let lib = library();
        let mut broken = PlanetaryStage::default();
        broken.sun.teeth = 17;
        broken.planet.teeth = 17;
        broken.ring.teeth = 80;
        let mut train = two_stage();
        train.stages.push(Stage::Planetary(Box::new(broken)));

        assert!(
            solve_train(&train, &lib).is_err(),
            "the third stage is the one that cannot be built"
        );
        let m = train.motion().expect("its motion needs none of that");
        assert_eq!(m.ratios.len(), 3);
        for (k, r) in m.ratios.iter().enumerate() {
            assert!(r.is_some(), "stage {k} turns");
        }
        // 17-tooth sun, 80-tooth ring, sun in and ring held: 97/17, exactly.
        assert_eq!(m.ratios[2], crate::ratio::Ratio::new(97, 17));
    }

    /// **A reversing stage is a stage**, and everything after it still solves.
    ///
    /// An epicyclic set reports a signed ratio where a pair reports a magnitude
    /// — one accessor, two meanings — and the sign leaked into two places that
    /// wanted a size:
    ///
    /// - **the torque referral.** A set with its carrier held has `i = −6`, so
    ///   the stage after it was handed **−11.6 N·m**, and a negative tangential
    ///   force has no Hertzian contact to press at any face width. The pair
    ///   refused with *"the teeth never come into contact"*, which is true of
    ///   nothing: those teeth mesh perfectly well. **A planetary set with its
    ///   carrier held could not be followed by any stage at all.**
    /// - **the backlash referral.** Play does not cancel — two independent
    ///   sources of lost motion add up whichever way their shafts turn — and a
    ///   reversing stage downstream made an upstream stage's contribution
    ///   *subtract*. A spur pair ahead of that same set reported 0.0422° where
    ///   the two stages have 0.0552°, 23.5 % light.
    ///
    /// Neither was reachable from a shipped fixture, because no shipped train
    /// puts a reversing stage in front of another. That is the standing trap of
    /// `docs/corrections.md`: *an axis nobody turns is an axis nobody tests* —
    /// and it is the corpus's `set-then-pair` and `pair-then-set` fixtures that
    /// turn it now.
    #[test]
    fn a_reversing_stage_does_not_poison_the_stages_around_it() {
        let lib = library();
        // A set with its **carrier** held reverses. Which shaft is held is the
        // train's to say now, so it is a constraint on the train and the set
        // itself is the default one. Shaft 2 is the carrier in the set's wiring.
        let reversing = || Stage::Planetary(Box::<PlanetaryStage>::default());
        // The set's three central shafts, stated in full: the train's
        // constraints lay over the kind's conventions shaft by shaft, so
        // holding the carrier *instead of* the ring says so about the ring.
        let carrier_held = |stage: usize| {
            vec![
                ShaftConstraint::held(stage, 2),
                ShaftConstraint {
                    at: ShaftRef::Of { stage, shaft: 3 },
                    constraint: Constraint::Free,
                },
            ]
        };

        // --- the set ahead of a pair. It used to refuse outright.
        let mut t = two_stage();
        t.stages = vec![reversing(), Stage::Spur(PairStage::default())];
        t.constraints = carrier_held(0);
        let r = solve_train(&t, &lib).expect("a reversing stage can be followed");
        assert!(r.stages[0].ratio() < 0.0, "this set reverses");
        for (k, s) in r.stages.iter().enumerate() {
            for g in s.members() {
                assert!(
                    g.cases[0].torque >= 0.0,
                    "stage {k}: a referral is a magnitude, not {}",
                    g.cases[0].torque
                );
            }
        }

        // --- and behind one, where the play is referred through it.
        let mut t = two_stage();
        t.stages = vec![Stage::Spur(PairStage::default()), reversing()];
        t.constraints = carrier_held(1);
        let r = solve_train(&t, &lib).expect("...and can follow one");
        let want = r.stages[0].backlash().forward.nominal / r.stages[1].ratio().abs()
            + r.stages[1].backlash().forward.nominal;
        assert!(
            (r.backlash.forward.nominal - want).abs() < 1e-12,
            "play accumulates: {} vs {want}",
            r.backlash.forward.nominal
        );
        // ...and it is strictly more than the last stage alone, which is what
        // "subtracting" would have taken it below.
        assert!(r.backlash.forward.nominal > r.stages[1].backlash().forward.nominal);
    }

    /// **One shaft, one speed** — a stage's output member and the next stage's
    /// input member are the same piece of metal and must say the same thing.
    ///
    /// They did not. Every kind worked its members' speeds out for itself, and
    /// they disagreed about *sign*: a pair's second member came back positive
    /// while turning backwards (`Mesh::ratio` is a magnitude), where an
    /// epicyclic set's came back signed. So a two-stage train reported
    /// `+1186 rpm` at the end of stage 1 and `+1186 rpm` at the start of stage
    /// 2 — agreeing by accident — and a set in front of a pair reported
    /// `−500` and `+500` for one shaft.
    ///
    /// Now every member's motion comes from the graph, through
    /// `Wiring::unit_motion`, and the coupling is a row in the same system.
    /// This is the law that says so, across three kinds and two junctions.
    #[test]
    fn a_shaft_shared_by_two_stages_reports_one_speed() {
        let lib = library();
        let mut checked = 0u32;
        for train in [two_stage(), mixed_train()] {
            let r = solve_train(&train, &lib).expect("these trains solve");
            let wirings: Vec<Wiring> = train.stages.iter().map(Stage::wiring).collect();
            let boundaries = train.boundaries().unwrap();
            // **Which member sits on a shaft is the wiring's answer, not the
            // member order's.** An epicyclic set's output is its *carrier*,
            // which carries no gear at all — the planet rides it and spins with
            // its own shaft — so "the stage's last member" is right for a pair
            // and wrong for a set, and a first draft of this test asserted it.
            let on = |k: usize, shaft: usize| -> Vec<usize> {
                wirings[k]
                    .mounts
                    .iter()
                    .enumerate()
                    .filter(|(_, m)| m.spins_with == shaft)
                    .map(|(i, _)| i)
                    .collect()
            };
            for k in 1..r.stages.len() {
                let before = r.stages[k - 1].members();
                let after = r.stages[k].members();
                for out in on(k - 1, boundaries[k - 1].output) {
                    for into in on(k, boundaries[k].input) {
                        for (a, b) in before[out].cases.iter().zip(&after[into].cases) {
                            assert!(
                                (a.speed - b.speed).abs() < 1e-9 * a.speed.abs().max(1.0),
                                "stage {k}: case {} leaves at {} and arrives at {}",
                                a.case + 1,
                                a.speed,
                                b.speed
                            );
                            checked += 1;
                        }
                    }
                }
            }
            // ...and the far port's delivered speed is what the member on that
            // shaft turns at, where one sits there.
            let k = r.stages.len() - 1;
            let last = r.stages[k].members();
            for into in on(k, boundaries[k].output) {
                for c in &r.cases {
                    let member = &last[into].cases[c.case];
                    assert!(
                        (member.speed - c.delivered_speed).abs()
                            < 1e-9 * c.delivered_speed.abs().max(1.0),
                        "case {}: delivered {} but the member turns {}",
                        c.case + 1,
                        c.delivered_speed,
                        member.speed
                    );
                    checked += 1;
                }
            }
        }
        // **A junction at an epicyclic set's carrier compares nothing**, and
        // that is the model rather than a gap: no gear sits on a carrier, so
        // there is no member whose speed to check. Pinned here so the count
        // below is a fact rather than a number that happened to pass.
        let set = Stage::Planetary(Box::<PlanetaryStage>::default());
        let w = set.wiring();
        assert!(
            !w.mounts
                .iter()
                .any(|m| m.spins_with == set.ports().output()),
            "a set driven sun-in with its ring held outputs on the carrier, \
             which carries no gear"
        );
        assert!(checked >= 12, "only {checked} couplings checked");
    }

    /// **A reversing train says so**, which the product of its stage ratios
    /// could not: a pair reports its own ratio as a magnitude, so a train of
    /// one external pair claimed its output turned with its input.
    #[test]
    fn a_trains_ratio_carries_the_direction_its_output_turns() {
        let lib = library();
        let one = |stage| {
            let mut t = two_stage();
            t.stages = vec![stage];
            solve_train(&t, &lib).expect("solves").total_ratio
        };
        // One external mesh reverses; two do not.
        assert!(one(Stage::Spur(PairStage::default())) < 0.0);
        let mut two = two_stage();
        two.load_cases.clear();
        assert!(solve_train(&two, &lib).expect("solves").total_ratio > 0.0);
        // A worm is an external mesh like any other.
        assert!(one(Stage::Worm(PairStage::worm())) < 0.0);
        // ...and an epicyclic set carries the sign its own kinematics gives:
        // sun in with the ring held turns the carrier the same way.
        assert!(one(Stage::Planetary(Box::default())) > 0.0);
    }

    /// **"Driven by" is a drive on the first stage and a coupling everywhere
    /// else** — and `Train::arranged` is what knows the difference.
    ///
    /// The panel's one gesture on a set, done by the core: a set at the head of
    /// a train told *sun in, carrier held* gets a drive on its sun; the same
    /// set behind a pair gets the chain moved to enter at its sun and no drive
    /// at all, because the pair is what turns it. Writing the drive there as
    /// well asks the sun to turn at one speed while the coupling turns it at
    /// another, which the solver refuses — and did, the first time a panel
    /// tried it.
    #[test]
    fn arranging_a_stage_writes_a_drive_at_the_head_and_moves_the_chain_elsewhere() {
        let lib = library();
        let set = || Stage::Planetary(Box::<PlanetaryStage>::default());
        let (sun, carrier, ring) = (1, 2, 3);

        // --- at the head: a drive on the sun, the carrier held, the ring free.
        let mut t = two_stage();
        t.stages = vec![set(), Stage::Spur(PairStage::default())];
        let t = t.arranged(0, sun, carrier);
        let own: Vec<_> = t
            .constraints
            .iter()
            .filter(|c| matches!(c.at, ShaftRef::Of { stage: 0, .. }))
            .map(|c| c.constraint)
            .collect();
        assert!(own.contains(&Constraint::Driven), "{own:?}");
        let r = solve_train(&t, &lib).expect("solves");
        assert!(
            r.stages[0].ratio() < 0.0,
            "carrier held reverses: {}",
            r.stages[0].ratio()
        );

        // --- behind a pair: no drive, the chain enters at the sun and leaves
        // by the ring, and it still solves.
        let mut t = two_stage();
        t.stages = vec![Stage::Spur(PairStage::default()), set()];
        let t = t.arranged(1, sun, carrier);
        let own: Vec<_> = t
            .constraints
            .iter()
            .filter(|c| matches!(c.at, ShaftRef::Of { stage: 1, .. }))
            .map(|c| c.constraint)
            .collect();
        assert!(!own.contains(&Constraint::Driven), "{own:?}");
        assert_eq!(
            t.couplings,
            vec![Coupling {
                a: ShaftRef::Of { stage: 0, shaft: 2 },
                b: ShaftRef::Of {
                    stage: 1,
                    shaft: sun
                },
            }]
        );
        let r = solve_train(&t, &lib).expect("solves behind a pair");
        assert!(r.stages[1].ratio() < 0.0);

        // ...and asked the other way — driven by the ring, sun held — the
        // chain enters at the ring.
        let t = t.arranged(1, ring, sun);
        assert_eq!(
            t.couplings[0].b,
            ShaftRef::Of {
                stage: 1,
                shaft: ring
            }
        );
        let r = solve_train(&t, &lib).expect("solves ring-in behind a pair");
        // ...and leaves by the carrier, at the ring-in ratio — not by the
        // ring it came in by, at a ratio of one, which is what it did.
        let b = t.boundaries().unwrap();
        assert_eq!((b[1].input, b[1].output), (ring, carrier));
        let set = r.stages[1].ratio();
        assert!(set > 1.0 && set < 2.0, "ring in, sun held: {set}");
    }

    /// **A port is a name for a shaft, and naming the shaft is the same load.**
    /// `Start` and `End` resolve through the boundaries to a stage's shaft,
    /// and a case written `At` that shaft — the wheel of the last pair, the
    /// carrier of a set — is one case, not a third kind of thing: every
    /// stage's torque, speed and turns, and what is delivered where, agree to
    /// the bit. Both ways round, on a chain of three kinds and on a lone set,
    /// with the sweep of a fatigue duty stated at the named shaft too.
    #[test]
    fn a_load_named_by_its_shaft_is_the_load_named_by_the_chain() {
        let lib = library();
        let lone_set = || {
            let mut t = two_stage();
            t.stages = vec![Stage::Planetary(Box::default())];
            t
        };
        let mut checked = 0;
        for train in [two_stage(), mixed_train(), lone_set()] {
            let boundaries = train.boundaries().unwrap();
            for name in [Port::Start, Port::End] {
                let shaft = train.port_shaft(&boundaries, name);
                assert_eq!(train.port_named(&boundaries, shaft), name);
                let mut by_name = train.clone();
                let mut by_shaft = train.clone();
                for (a, b) in by_name.load_cases.iter_mut().zip(&mut by_shaft.load_cases) {
                    a.port = name;
                    b.port = Port::At(shaft);
                    a.duty = Duty::Intermittent {
                        range_degrees: 25.0,
                        at: name,
                        actuations: 1000,
                        reversing: false,
                    };
                    b.duty = Duty::Intermittent {
                        range_degrees: 25.0,
                        at: Port::At(shaft),
                        actuations: 1000,
                        reversing: false,
                    };
                }
                let (n, s) = (
                    solve_train(&by_name, &lib).expect("by name"),
                    solve_train(&by_shaft, &lib).expect("by shaft"),
                );
                for (a, b) in n.cases.iter().zip(&s.cases) {
                    assert_eq!(a.delivered_at, b.delivered_at);
                    assert_eq!(a.delivered_torque, b.delivered_torque);
                    assert_eq!(a.delivered_speed, b.delivered_speed);
                    assert_eq!(a.reacted_at, b.reacted_at);
                }
                for (x, y) in n.stages.iter().zip(&s.stages) {
                    for (g, h) in x.members().iter().zip(y.members()) {
                        for (c, d) in g.cases.iter().zip(&h.cases) {
                            assert_eq!(c.torque, d.torque);
                            assert_eq!(c.speed, d.speed);
                            assert_eq!(c.cycles, d.cycles);
                            checked += 1;
                        }
                    }
                }
            }
        }
        assert!(checked > 40, "{checked} member-cases compared");
    }

    /// **The route is read off the graph**, and it says which way a load
    /// crosses each stage and where it ends up — forward through a stage it
    /// enters by the input of, backward through one it enters by the output
    /// of. A load entering a set by its *carrier* at the tail of a chain is
    /// backward through the set and backward through the pair before it; one
    /// on a shaft no load can be put on — ground, the held ring, the planet
    /// — is refused by name, and one on the shaft two stages share is refused
    /// as shared rather than sent one way.
    #[test]
    fn a_route_says_which_way_a_load_crosses_each_stage() {
        let lib = library();
        let (sun, carrier, ring, planet) = (1, 2, 3, 4);
        let mut t = two_stage();
        t.stages = vec![
            Stage::Spur(PairStage::default()),
            Stage::Planetary(Box::default()),
        ];
        let b = t.boundaries().unwrap();
        let at = |stage, shaft| ShaftRef::Of { stage, shaft };

        let forward = t.route(&b, at(0, 1)).unwrap();
        assert_eq!(
            forward.steps,
            vec![(0, Drive::Forward), (1, Drive::Forward)]
        );
        assert_eq!(forward.far, at(1, carrier));
        let back = t.route(&b, at(1, carrier)).unwrap();
        assert_eq!(back.steps, vec![(1, Drive::Backward), (0, Drive::Backward)]);
        assert_eq!(back.far, at(0, 1));

        for bad in [
            ShaftRef::Ground,
            at(1, ring),
            at(1, planet),
            at(0, 9),
            at(5, 1),
        ] {
            assert_eq!(t.route(&b, bad), Err(RouteError::NotAPort), "{bad:?}");
        }
        assert_eq!(t.route(&b, at(1, sun)), Err(RouteError::Shared));
        assert_eq!(t.route(&b, at(0, 2)), Err(RouteError::Shared));

        // ...and the refusals reach a load case by number.
        let mut shared = t.clone();
        shared.load_cases[1].port = Port::At(at(1, sun));
        assert_eq!(
            solve_train(&shared, &lib).err(),
            Some(TrainError::LoadShared { case: 1 })
        );
        let mut held = t.clone();
        held.load_cases[2].port = Port::At(at(1, ring));
        assert_eq!(
            solve_train(&held, &lib).err(),
            Some(TrainError::LoadPort { case: 2 })
        );

        // A set driven by its carrier at the head of a chain: `Start` *is*
        // the carrier now, and the load walks the set backward.
        let mut t = two_stage();
        t.stages = vec![
            Stage::Planetary(Box::default()),
            Stage::Spur(PairStage::default()),
        ];
        let t = t.arranged(0, carrier, ring);
        let b = t.boundaries().unwrap();
        assert_eq!(t.port_shaft(&b, Port::Start), at(0, carrier));
        let r = t.route(&b, at(0, carrier)).unwrap();
        assert_eq!(r.steps, vec![(0, Drive::Forward), (1, Drive::Forward)]);
        let r = solve_train(&t, &lib).expect("carrier-driven set at the head");
        assert!(
            r.total_ratio.abs() < 1.0,
            "a carrier-driven set multiplies speed"
        );
    }

    /// **The open ports are what a load can enter by**, each with the name
    /// the chain gives it: a chain's two ends are `Start` and `End`, and a
    /// released ring — un-held, uncoupled — is a third, by its own reference.
    #[test]
    fn the_open_ports_are_the_chains_ends_and_whatever_else_is_uncoupled() {
        let at = |stage, shaft| ShaftRef::Of { stage, shaft };
        let t = mixed_train();
        let ports = t.open_ports(&t.boundaries().unwrap());
        assert_eq!(
            ports.iter().map(|p| (p.port, p.at)).collect::<Vec<_>>(),
            vec![(Port::Start, at(0, 1)), (Port::End, at(3, 2))]
        );
        let mut t = two_stage();
        t.stages = vec![Stage::Planetary(Box::default())];
        t.constraints = vec![ShaftConstraint {
            at: at(0, 3),
            constraint: Constraint::Free,
        }];
        let ports = t.open_ports(&t.boundaries().unwrap());
        assert_eq!(
            ports.iter().map(|p| p.port).collect::<Vec<_>>(),
            vec![Port::Start, Port::End, Port::At(at(0, 3))]
        );
        assert_eq!(ports[2].label, ShaftLabel::Member { member: 2 });
    }

    /// **A train one condition short reports the family and refuses the
    /// rating**, by name. A set with its ring released and its sun driven has
    /// one free parameter; every shaft's speed is a particular value plus one
    /// term per turn of the free shaft, and at every value of that parameter
    /// the family satisfies Willis — `z_s ω_s + z_r ω_r = (z_s + z_r) ω_c` —
    /// exactly, with the sun at one turn throughout. The report's terms are
    /// the same family read as floats.
    #[test]
    fn a_train_one_condition_short_reports_the_family_and_refuses_the_rating() {
        let lib = library();
        let mut t = two_stage();
        t.stages = vec![Stage::Planetary(Box::default())];
        t.constraints = vec![ShaftConstraint {
            at: ShaftRef::Of { stage: 0, shaft: 3 },
            constraint: Constraint::Free,
        }];
        assert_eq!(
            solve_train(&t, &lib).err(),
            Some(TrainError::Underdetermined { short: 1 })
        );
        let m = t.motion().expect("the motion is a family, not a refusal");
        assert_eq!(m.solution.residual.len(), 1, "{:?}", m.solution);
        let r = t.motion_report().expect("...and it is reported");
        assert_eq!(r.free.len(), 1);
        assert!(
            r.total.is_none() && r.ratios[0].is_none(),
            "a family has no quotient"
        );
        let (sun, carrier, ring) = (1, 2, 3);
        let z = |k: usize| crate::ratio::Ratio::whole(i64::from(t.stages[0].members()[k].teeth));
        let (zs, zr) = (z(0), z(2));
        for p in [0i64, 1, -2, 7] {
            let p = crate::ratio::Ratio::whole(p);
            let at = |shaft: usize| {
                let i = m.shaft_of(0, shaft);
                m.solution.values[i]
                    .checked_add(m.solution.residual[0].direction[i].checked_mul(p).unwrap())
                    .unwrap()
            };
            assert_eq!(at(sun), crate::ratio::Ratio::ONE);
            let lhs = zs
                .checked_mul(at(sun))
                .unwrap()
                .checked_add(zr.checked_mul(at(ring)).unwrap())
                .unwrap();
            let rhs = zs
                .checked_add(zr)
                .unwrap()
                .checked_mul(at(carrier))
                .unwrap();
            assert_eq!(lhs, rhs, "Willis at p = {p}");
            // ...and the report is the same family, read as floats.
            for shaft in [sun, carrier, ring] {
                let s = &r.shafts[m.shaft_of(0, shaft)];
                let read = s.speed.value
                    + s.terms
                        .iter()
                        .map(|term| term.coefficient.value * p.to_f64())
                        .sum::<f64>();
                assert!(
                    (read - at(shaft).to_f64()).abs() < 1e-12,
                    "shaft {shaft} at p = {p}"
                );
            }
        }
    }

    /// **Two drives a designer writes on one set are both kept**, and are
    /// one motion: at one turn each of the sun and the carrier the whole set
    /// turns as one, so the ring turns at exactly one too. What is refused is
    /// the *rating*, by its own name — a set with two inputs has no held
    /// shaft for the power flow to be worked out against — and not the
    /// motion. (A first version of the overlay let the second drive remove
    /// the first, and a differential's two inputs came out as one input and
    /// a family.)
    #[test]
    fn two_drives_on_one_set_are_both_kept_and_are_one_motion() {
        let lib = library();
        let mut t = two_stage();
        t.stages = vec![Stage::Planetary(Box::default())];
        t.constraints = vec![
            ShaftConstraint::driven(0, 1),
            ShaftConstraint::driven(0, 2),
            ShaftConstraint {
                at: ShaftRef::Of { stage: 0, shaft: 3 },
                constraint: Constraint::Free,
            },
        ];
        let driven = t
            .constraints_in_force()
            .iter()
            .filter(|c| c.constraint == Constraint::Driven)
            .count();
        assert_eq!(driven, 2);
        let m = t.motion().expect("one motion");
        assert!(m.solution.is_unique());
        assert_eq!(
            m.solution.values[m.shaft_of(0, 3)],
            crate::ratio::Ratio::ONE
        );
        assert!(matches!(
            solve_train(&t, &lib),
            Err(TrainError::InStage {
                stage: 0,
                ref cause
            }) if **cause == TrainError::Wiring(WiringError::Unsolvable)
        ));
    }

    /// **Every other way the conditions can fail to give one motion is
    /// named**: a shaft asked two things that contradict, a shaft the train
    /// does not have, and tooth counts whose product outgrows an exact ratio
    /// — and none of them is the wiring sentence, which describes a stage
    /// that is no mechanism.
    #[test]
    fn a_conflict_a_missing_shaft_and_an_overflow_are_each_named() {
        let lib = library();
        let set = |constraints| {
            let mut t = two_stage();
            t.stages = vec![Stage::Planetary(Box::default())];
            t.constraints = constraints;
            t
        };
        assert!(matches!(
            solve_train(&set(vec![ShaftConstraint::held(0, 2)]), &lib),
            Err(TrainError::Overdetermined {
                at: ShaftRef::Of { stage: 0, .. }
            })
        ));
        assert_eq!(
            solve_train(&set(vec![ShaftConstraint::held(7, 1)]), &lib).err(),
            Some(TrainError::NoSuchShaft {
                at: ShaftRef::Of { stage: 7, shaft: 1 }
            })
        );
        let huge = |teeth| StageGear {
            teeth,
            ..StageGear::default()
        };
        let mut wide = two_stage();
        wide.stages = (0..6)
            .map(|k| {
                Stage::Spur(PairStage {
                    gears: [huge(4_000_000_000 + k), huge(4_000_000_001 + k)],
                    ..PairStage::default()
                })
            })
            .collect();
        assert_eq!(solve_train(&wide, &lib).err(), Some(TrainError::Overflow));
        assert!(wide.motion_report().is_none());
    }

    /// Every freedom a stage's groups mention, flat.
    fn mentioned(stage: &Stage) -> Vec<Freedom> {
        stage
            .freedoms()
            .into_iter()
            .flat_map(|g| g.order.into_iter().flatten())
            .collect()
    }

    /// **A member's result carries the tooth it was cut with**, agreeing with
    /// every figure the result quotes beside it — so a gear tab that adopts
    /// the member shows the tooth the stage rated, and not a rebuild from the
    /// inputs that could drift from it. On every kind, every member: the
    /// shift, the addendum and the helix in `params` are the ones in force,
    /// the count and module are the stage's, and a rack-cut member rebuilt
    /// from `params` alone is the reported pitch diameter; a ring is the
    /// member its stage cuts with a pinion cutter, and only that member.
    #[test]
    fn a_members_result_carries_the_tooth_it_was_cut_with() {
        let lib = library();
        let mut checked = 0u32;
        for stage in every_kind() {
            let mut t = two_stage();
            t.stages = vec![stage.clone()];
            let r = solve_train(&t, &lib).expect("every preset solves");
            let inputs = stage.members();
            for (i, g) in r.stages[0].members().iter().enumerate() {
                assert_eq!(
                    g.params.profile_shift, g.profile_shift,
                    "{stage:?} member {i}"
                );
                assert_eq!(g.params.addendum, g.addendum, "{stage:?} member {i}");
                assert_eq!(g.params.helix_angle, g.helix_angle, "{stage:?} member {i}");
                assert_eq!(g.params.teeth, inputs[i].teeth, "{stage:?} member {i}");
                assert_eq!(
                    g.params.dedendum, inputs[i].dedendum,
                    "{stage:?} member {i}"
                );
                assert_eq!(
                    g.params.root_radius, inputs[i].root_radius,
                    "{stage:?} member {i}"
                );
                let rebuilt = Tooth::new(g.params);
                assert!(
                    (rebuilt.params.teeth as f64 * rebuilt.params.module
                        / rebuilt.params.helix_angle.to_radians().cos()
                        - g.pitch_diameter)
                        .abs()
                        < 1e-12,
                    "{stage:?} member {i}: the tooth rebuilt from params is not the one reported"
                );
                checked += 1;
            }
            let rings: Vec<usize> = (0..inputs.len())
                .filter(|i| stage.member_cutter(*i).is_some())
                .collect();
            let expect: Vec<usize> = match &stage {
                Stage::Spur(_) | Stage::Worm(_) => vec![],
                Stage::Planetary(_) => vec![2],
                Stage::Hula(h) => (0..2)
                    .map(|m| {
                        let (a, b) = (2 * m, 2 * m + 1);
                        if h.gears[a].teeth > h.gears[b].teeth {
                            a
                        } else {
                            b
                        }
                    })
                    .collect(),
            };
            assert_eq!(
                rings, expect,
                "which members {stage:?} cuts with a pinion cutter"
            );
        }
        assert_eq!(checked, 2 + 2 + 3 + 4);
    }

    /// **The declared freedoms describe inputs the stage has, and each group
    /// has something to relieve.**
    ///
    /// The structural half. A declaration nothing checks is data, and this is
    /// the cheap part of checking it: a freedom that named an input the stage
    /// does not have would count as neither given nor automatic and relieve
    /// silently, and a group whose limits equal its size can never fire at all
    /// — unless an entry of it holds several readings, which is where such a
    /// group bites.
    #[test]
    fn every_declared_freedom_names_an_input_the_stage_has() {
        for stage in every_kind() {
            let has: Vec<Freedom> = stage.toggles().into_iter().map(|(f, _)| f).collect();
            for g in &stage.freedoms() {
                assert!(
                    !g.order.is_empty(),
                    "a group with no inputs relieves nothing: {g:?}"
                );
                assert!(
                    g.given_at_most < g.order.len()
                        || g.automatic_at_most < g.order.len()
                        || g.order.iter().any(|e| e.len() > 1),
                    "a group neither bound can bite on is not a group: {g:?}"
                );
                // **The two bounds have to be jointly satisfiable.** Every entry
                // is either given or automatic, so the counts always sum to
                // `order.len()` — and a group demanding fewer than that in total
                // is one no arrangement of toggles can satisfy, which would leave
                // relief flipping the same input back and forth forever.
                assert!(
                    g.given_at_most + g.automatic_at_most >= g.order.len(),
                    "no arrangement of toggles satisfies {g:?}"
                );
                for f in g.order.iter().flatten() {
                    assert!(
                        has.contains(f),
                        "{f:?} is declared but {stage:?} has no such input"
                    );
                }
            }
        }
    }

    /// **Relief settles, and settles once.** Relieving a relieved stage moves
    /// nothing, whatever was touched — so a declaration where satisfying one
    /// group breaks another for ever, which the walk's bound would cut short
    /// rather than resolve, cannot pass here. And it settles from any start:
    /// everything pinned, everything freed, and each single toggle turned.
    #[test]
    fn relief_is_idempotent_from_any_start() {
        for stage in every_kind() {
            let all = stage.toggles();
            let mut starts = vec![stage.clone()];
            for auto in [false, true] {
                let mut s = stage.clone();
                for (f, _) in &all {
                    if let Some(a) = s.input_mut(*f) {
                        a.auto = auto;
                    }
                }
                starts.push(s);
            }
            for (f, was) in &all {
                let mut s = stage.clone();
                s.input_mut(*f).expect("its own toggle").auto = !was;
                starts.push(s);
            }
            let justs: Vec<Option<Freedom>> = std::iter::once(None)
                .chain(all.iter().map(|(f, _)| Some(*f)))
                .collect();
            for start in &starts {
                for just in &justs {
                    let once = start.relieved(*just);
                    let twice = once.relieved(*just);
                    assert_eq!(
                        once.toggles(),
                        twice.toggles(),
                        "relief moved on a second pass after {just:?} from {start:?}"
                    );
                    // ...and once settled, every group is within its limits —
                    // counted the way relief counts, entries given while any
                    // reading is.
                    let mut probe = once.clone();
                    for g in once.freedoms() {
                        let states: Vec<bool> = g
                            .order
                            .iter()
                            .map(|e| e.iter().all(|f| probe.input_mut(*f).is_none_or(|a| a.auto)))
                            .collect();
                        let automatic = states.iter().filter(|a| **a).count();
                        let given = states.len() - automatic;
                        assert!(
                            given <= g.given_at_most && automatic <= g.automatic_at_most,
                            "{g:?} left {given} given and {automatic} automatic after {just:?}"
                        );
                        for e in &g.order {
                            assert!(
                                e.iter()
                                    .filter(|f| probe.input_mut(**f).is_some_and(|a| !a.auto))
                                    .count()
                                    <= 1,
                                "two readings of one input stand in {e:?} after {just:?}"
                            );
                        }
                    }
                }
            }
        }
    }

    /// **After relief, every given input in a relation is read** — the
    /// property the machinery exists for, asked of the solve rather than of
    /// the declaration. Everything is pinned, relief decides what may stand,
    /// and then each input left given is nudged and the solved stage has to
    /// move: an input that stands given and moves nothing is one the solve
    /// disregards, which is precisely what relief was written to prevent.
    ///
    /// This is the gate that sees a disagreement between the declaration and
    /// the solve — a freedom the groups let stand that the solve never
    /// reads — which no test of either side alone can.
    #[test]
    fn every_input_relief_leaves_given_is_honoured_by_the_solve() {
        let lib = library();
        let signature = |stage: &Stage| -> Vec<f64> {
            let mut t = two_stage();
            t.stages = vec![stage.clone()];
            let r = solve_train(&t, &lib).expect("a pinned preset solves");
            let solved = &r.stages[0];
            stage
                .toggles()
                .into_iter()
                .filter_map(|(f, _)| solved.figure(f))
                .chain(solved.members().iter().map(|g| g.pitch_diameter))
                .collect()
        };
        let nudge = |f: Freedom, a: &mut Auto<f64>| match f {
            Freedom::CentreDistance => a.manual += 0.2,
            Freedom::Clearance => a.manual += 0.01,
            Freedom::FirstPitchDiameter => a.manual *= 1.05,
            Freedom::Overlap => a.manual += 0.1,
            Freedom::Member(_, MemberFreedom::Shift) => a.manual += 0.05,
            Freedom::Member(_, MemberFreedom::Helix) => a.manual += 2.0,
            Freedom::Member(_, MemberFreedom::FaceWidth) => a.manual += 1.0,
        };
        let mut checked = 0u32;
        for stage in every_kind() {
            // Every box seeded with what the preset came to, as the panel
            // seeds a box a designer pins — so pinning everything pins the
            // design the preset already was, not a distance of nought.
            let mut t = two_stage();
            t.stages = vec![stage.clone()];
            let solved = solve_train(&t, &lib).expect("every preset solves");
            let mut pinned = stage.clone();
            for (f, _) in stage.toggles() {
                if let Some(v) = solved.stages[0].figure(f) {
                    pinned.input_mut(f).expect("its own input").manual = v;
                }
            }
            for f in mentioned(&stage) {
                if let Some(a) = pinned.input_mut(f) {
                    a.auto = false;
                }
            }
            let settled = pinned.relieved(None);
            let base = signature(&settled);
            for f in mentioned(&settled) {
                let mut moved = settled.clone();
                let a = moved.input_mut(f).expect("declared, so present");
                if a.auto {
                    continue;
                }
                nudge(f, a);
                let after = signature(&moved);
                assert!(
                    base.iter().zip(&after).any(|(x, y)| (x - y).abs() > 1e-9),
                    "{f:?} stands given on {settled:?} and the solve does not read it"
                );
                checked += 1;
            }
        }
        assert!(checked >= 12, "only {checked} given inputs were checked");
    }

    /// **The reading relief leaves standing is the reading the solve reads.**
    /// Two readings of a pair's size given with different stories — a helix
    /// that says one diameter and a diameter that says another — and after
    /// relief the pair is the size the surviving reading states, exactly.
    /// One list serves both, so this cannot fail by the two lists drifting;
    /// it is here so that stays true when the next reading arrives.
    #[test]
    fn the_surviving_reading_is_the_one_the_solve_reads() {
        let mut both = PairStage::default().with_first_helix(10.0);
        both.pitch_diameter = Auto::fixed(20.0);
        let relieved = Stage::Spur(both).relieved(None);
        let Stage::Spur(pair) = relieved else {
            unreachable!()
        };
        assert!(
            pair.gears[0].helix_angle.auto,
            "the helix is the less precious reading"
        );
        assert!(!pair.pitch_diameter.auto, "the diameter stands");
        assert_eq!(pair.first_pitch_diameter(), 20.0);
        let expect = (17.0_f64 / 20.0).acos().to_degrees();
        assert!((pair.helix_angles()[0] - expect).abs() < 1e-12);

        // ...and a ratio given the size to decide is the most precious of all.
        let mut with_ratio = PairStage::default().with_first_helix(10.0);
        with_ratio.overlap = Auto::fixed(1.2);
        let Stage::Spur(pair) = Stage::Spur(with_ratio).relieved(None) else {
            unreachable!()
        };
        assert!(pair.gears[0].helix_angle.auto && !pair.overlap.auto);
        let width = pair.gears[0]
            .face_width
            .manual
            .min(pair.gears[1].face_width.manual);
        let expect = helix_for_overlap(1.2, 1.0, width).expect("reachable at the preset's width");
        assert!((pair.helix_angles()[0] - expect).abs() < 1e-12);
    }

    /// **A crossed pair has no overlap, and its ratio is turned back
    /// automatic** — by relief, whatever was touched, rather than by a line in
    /// the panel that used to reset the toggle when the shaft angle moved.
    /// A point contact has nothing for the ratio to relate, and a box the
    /// solve disregards must not stand as if it were read.
    #[test]
    fn a_crossed_pairs_ratio_cannot_stand_given() {
        let mut crossed = PairStage::worm();
        crossed.overlap = Auto::fixed(1.5);
        for just in [None, Some(Freedom::Overlap), Some(Freedom::CentreDistance)] {
            let Stage::Worm(p) = Stage::Worm(crossed.clone()).relieved(just) else {
                unreachable!()
            };
            assert!(
                p.overlap.auto,
                "the ratio stood given on crossed shafts after {just:?}"
            );
            assert_eq!(p.overlap.manual, 1.5, "and the number is kept");
        }
        // ...and on parallel shafts the same input stands.
        let parallel = PairStage {
            overlap: Auto::fixed(1.5),
            ..PairStage::default()
        };
        let Stage::Spur(p) = Stage::Spur(parallel).relieved(None) else {
            unreachable!()
        };
        assert!(!p.overlap.auto);
    }

    /// **A ratio given on straight teeth is said to have asked nothing.** As
    /// a floor under an automatic width it needs a helix to buy overlap with,
    /// and at zero helix no width does — so the input is read as nothing, and
    /// the note says so rather than the box standing as if it had bitten.
    #[test]
    fn a_ratio_given_on_straight_teeth_says_it_asked_nothing() {
        let lib = library();
        let mut sp = PairStage {
            overlap: Auto::fixed(1.2),
            ..PairStage::default()
        };
        for g in &mut sp.gears {
            g.face_width = Auto::automatic(5.0);
        }
        let mut t = two_stage();
        t.stages = vec![Stage::Spur(sp.clone())];
        let r = solve_train(&t, &lib).expect("solves");
        let notes = &r.stages[0].as_pair().expect("a pair").notes;
        assert!(
            notes
                .iter()
                .any(|n| n.key == key::STAGE_OVERLAP_NEEDS_HELIX),
            "no note for a ratio with no helix to work on: {notes:?}"
        );
        // Give it a helix and the floor bites, and the note goes.
        let helical = sp.with_first_helix(20.0);
        t.stages = vec![Stage::Spur(helical)];
        let r = solve_train(&t, &lib).expect("solves");
        let pair = r.stages[0].as_pair().expect("a pair");
        assert!(!pair
            .notes
            .iter()
            .any(|n| n.key == key::STAGE_OVERLAP_NEEDS_HELIX));
        let ratio = pair
            .mesh
            .line
            .as_ref()
            .expect("a line contact")
            .contact_ratios
            .overlap;
        assert!(
            (ratio - 1.2).abs() < 1e-9,
            "the floor should buy exactly the ratio: {ratio}"
        );
    }

    /// **A box relief turns given holds what it was showing**, to the digits
    /// shown — seeded from the figure the result carried for it, by name, so
    /// the panel need not know which field a freedom is. A box relief leaves
    /// alone, and one it turns automatic, keep their numbers.
    #[test]
    fn a_box_relief_pins_is_seeded_from_its_figure() {
        let set = PlanetaryStage {
            clearance: Auto::automatic(0.02),
            ..PlanetaryStage::default()
        };
        let figures = [
            Figure {
                freedom: Freedom::Clearance,
                value: Some(0.123_456_789),
            },
            Figure {
                freedom: Freedom::CentreDistance,
                value: Some(99.0),
            },
        ];
        let Stage::Planetary(p) = Stage::Planetary(Box::new(set)).relieved_from(None, &figures)
        else {
            unreachable!()
        };
        assert!(!p.clearance.auto, "the set's clearance is pinned back");
        assert_eq!(p.clearance.manual, 0.1235, "and seeded to the digits shown");
        assert_eq!(
            p.centre_distance.manual,
            PlanetaryStage::default().centre_distance.manual,
            "a box relief did not turn keeps its number"
        );
    }

    /// **An epicyclic kind's clearance cannot be automatic, even when it is the
    /// input the designer just touched.** A set's is the amount its two
    /// zero-backlash distances differ by, which the shifts are solved from; a
    /// hula stage's shifts absorb the crank, so no given offset leaves a
    /// nominal one to subtract from. So the second pass of `relieved` — the
    /// one that does not spare `just` — pins it, and the toggle snapping back
    /// is the tool saying so.
    #[test]
    fn an_epicyclic_kinds_clearance_is_pinned_back_whatever_was_touched() {
        let set = PlanetaryStage {
            clearance: Auto::automatic(0.02),
            ..PlanetaryStage::default()
        };
        let hula = HulaStage {
            running_clearance: Auto::automatic(0.02),
            ..HulaStage::default()
        };
        for just in [
            Some(Freedom::Clearance),
            Some(Freedom::CentreDistance),
            Some(Freedom::Member(1, MemberFreedom::Shift)),
            None,
        ] {
            let (auto, manual) = match Stage::Planetary(Box::new(set.clone())).relieved(just) {
                Stage::Planetary(p) => (p.clearance.auto, p.clearance.manual),
                _ => unreachable!(),
            };
            assert!(
                !auto,
                "a set: relieving after {just:?} should pin the clearance"
            );
            assert_eq!(manual, 0.02, "and leave the number where it was");
            let (auto, manual) = match Stage::Hula(Box::new(hula.clone())).relieved(just) {
                Stage::Hula(h) => (h.running_clearance.auto, h.running_clearance.manual),
                _ => unreachable!(),
            };
            assert!(
                !auto,
                "a hula stage: relieving after {just:?} should pin the clearance"
            );
            assert_eq!(manual, 0.02, "and leave the number where it was");
        }
    }

    /// **Relieving an over-determined stage**, on every kind that has a relation
    /// — behaviour that had no test at all while it lived in the panel.
    ///
    /// Three claims, and the third is the one that makes it a rule rather than a
    /// habit: pinning everything relieves down to the limit; the input the
    /// designer has *this moment* pinned is never the one taken away; and a
    /// stage already within its limit is returned untouched, so relief cannot
    /// undo a design that was never over-determined.
    #[test]
    fn an_over_determined_stage_relieves_to_its_limit_and_keeps_what_was_just_pinned() {
        for stage in every_kind() {
            let mut over = stage.clone();
            for f in mentioned(&stage) {
                if let Some(a) = over.input_mut(f) {
                    a.auto = false;
                }
            }
            for just in mentioned(&stage) {
                let mut relieved = over.relieved(Some(just));
                // `just` is spared wherever sparing it leaves an answer, which
                // for every input in a relation it does — the always-given
                // and always-automatic groups are the exceptions, and they
                // are alone in theirs.
                let alone = stage
                    .freedoms()
                    .iter()
                    .any(|g| g.order == vec![vec![just]] && g.given_at_most == 0);
                assert!(
                    alone || relieved.input_mut(just).is_some_and(|a| !a.auto),
                    "{just:?} was just pinned and must not be the one relieved"
                );
            }
            // Already inside the limit — every kind's preset is, with a
            // clearance and a worm's diameter given — nothing moves. Asserted
            // against every freedom as `just`, so it cannot pass by picking a
            // lucky one.
            for just in mentioned(&stage) {
                assert_eq!(
                    stage.relieved(Some(just)).toggles(),
                    stage.toggles(),
                    "a toggle moved on a stage that was already within its limit, after {just:?}"
                );
            }
        }
    }

    /// **The shifts the optimiser recommends do not interfere** — which they did,
    /// on every pair whose pinion was small enough for the search to press.
    ///
    /// A mate's tip reaching past the usable end of a flank is the classical
    /// interference condition. It was asked of internal meshes under two names
    /// and of external ones **not at all**, so the shift search was free to walk
    /// into it — and did, because loss falls with the length of the path and the
    /// longest admissible path is the one that ends exactly where the flank
    /// does. Measured over the fourteen pairs below, **two interfered**: 9/37
    /// and 9/20, both nine-tooth pinions, and in both it was the pinion's flank
    /// the wheel's tip reached past.
    ///
    /// Asserted of what the stage actually builds — its own shifts, its own
    /// addenda, at the distance it runs — rather than of a trial, because the
    /// claim is about the recommendation and not about the search's internals.
    #[test]
    fn the_optimiser_does_not_recommend_a_pair_whose_teeth_interfere() {
        let mut checked = 0u32;
        for teeth in [
            [9_u32, 37],
            [9, 20],
            [11, 41],
            [12, 29],
            [13, 31],
            [17, 43],
            [17, 17],
            [20, 20],
            [23, 61],
            [10, 51],
            [14, 22],
            [16, 33],
        ] {
            let mut stage = PairStage::default();
            stage.optimisation.enabled = true;
            stage.gears = [0, 1].map(|i| StageGear {
                teeth: teeth[i],
                ..PairStage::default().gears[i].clone()
            });
            let x = stage.shifts();
            let g = [0, 1].map(|i| Tooth::new(stage.params_at(i, x[i])));
            let Ok(zero) = crate::mesh::Mesh::new(&g[0], &g[1], crate::mesh::MeshKind::External)
            else {
                continue;
            };
            // At the distance it runs, which is the mesh every figure is read
            // off and the less conservative of the two.
            let mesh = zero.at(zero.a_w + stage.clearance.manual).unwrap_or(zero);
            checked += 1;
            let foul = mesh.flank_interference([g[0].flank_ends(), g[1].flank_ends()]);
            assert_eq!(
                foul,
                [false, false],
                "{teeth:?}: the optimiser chose {x:?}, whose teeth interfere"
            );
        }
        assert!(checked >= 10, "only {checked} pairs were solvable");
    }

    /// **A distance the shifts cannot reach is said out loud**, and so is a pair
    /// that cannot be assembled — F55.
    ///
    /// Mode 3 says a given distance and a given clearance decide the shifts.
    /// Where no admissible shifts reach that distance the stage still answers,
    /// at the shifts it would have built anyway, and **said nothing at all**:
    /// the only trace was a clearance readout that no longer meant what it said.
    ///
    /// Worse at the tight end. A 9/37 pair told to run at 23.00 mm has its
    /// shifts pinned at the pinion's undercut floor, which puts the pair at
    /// 23.4433 — so it runs **0.44 mm inside its own zero-backlash distance**,
    /// which is teeth overlapping at rest. Every figure taken there describes
    /// nothing, and nothing said so.
    ///
    /// Notes rather than refusals, on rule 5's reading: the gears are cuttable
    /// and it is the *assembly* that is impossible, so the designer is owed the
    /// number. What the rule must not do is fire on a distance that **is**
    /// reached, which is the third case below.
    #[test]
    fn a_distance_the_shifts_cannot_reach_is_said_out_loud() {
        let lib = library();
        let at = |a: f64| {
            let mut sp = PairStage {
                centre_distance: Auto::fixed(a),
                ..PairStage::default()
            };
            sp.gears[0].teeth = 9;
            sp.gears[1].teeth = 37;
            let mut t = two_stage();
            t.stages = vec![Stage::Spur(sp)];
            let r = solve_train(&t, &lib).expect("all three of these solve");
            let s = r.stages[0].as_pair().expect("a spur stage");
            let keys: Vec<String> = s.notes.iter().map(|n| n.key.clone()).collect();
            (s.clearance, keys)
        };

        // Well inside what the teeth allow: both findings.
        let (clearance, keys) = at(23.0);
        assert!(clearance < 0.0, "this one should not be assemblable");
        assert!(
            keys.iter()
                .any(|k| k == key::STAGE_CENTRE_DISTANCE_NOT_REACHED),
            "a distance no shifts reach should say so: {keys:?}"
        );
        assert!(
            keys.iter().any(|k| k == key::STAGE_CLEARANCE_NEGATIVE),
            "a pair that cannot be assembled should say so: {keys:?}"
        );

        // Too far out: reachable by nothing, but assemblable.
        let (clearance, keys) = at(25.0);
        assert!(clearance > 0.0);
        assert!(
            keys.iter()
                .any(|k| k == key::STAGE_CENTRE_DISTANCE_NOT_REACHED)
                && !keys.iter().any(|k| k == key::STAGE_CLEARANCE_NEGATIVE),
            "the distance is unreachable but the pair goes together: {keys:?}"
        );

        // **And a distance that is reached says neither**, which is what stops
        // this being a note that fires on every stage with a distance.
        let (clearance, keys) = at(24.22);
        assert!(
            (clearance - PairStage::default().clearance.manual).abs() < 1e-6,
            "the shifts reach this one, so the clearance is the one asked for: {clearance}"
        );
        assert!(
            !keys.iter().any(|k| {
                k == key::STAGE_CENTRE_DISTANCE_NOT_REACHED || k == key::STAGE_CLEARANCE_NEGATIVE
            }),
            "a distance that is reached should say neither: {keys:?}"
        );
    }

    /// **A distance and a clearance cannot both be derived**, and relief pins one
    /// of them rather than leaving the stage undetermined.
    ///
    /// The under-constrained mirror of the over-constrained case, and the reason
    /// [`FreedomGroup`] bounds automatics as well as givens. A distance is
    /// *nominal + clearance*; an automatic clearance is *distance − nominal*.
    /// With both automatic neither has anything to derive from, and there is no
    /// answer to give — so one of them is pinned, in the same relief order and
    /// by the same walk.
    ///
    /// The planetary is the case worth reading twice: it has **no** distance
    /// input yet, so its clearance can never be derived at all, and the same
    /// count says so without a special case. When F39's third item gives it one,
    /// the declaration stops saying that on its own.
    #[test]
    fn a_distance_and_a_clearance_are_never_both_left_automatic() {
        let mut checked = 0u32;
        for stage in [
            Stage::Spur(PairStage::default()),
            Stage::Worm(PairStage::worm()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::default()),
        ] {
            let Some(group) = stage
                .freedoms()
                .into_iter()
                .find(|g| g.automatic_at_most < g.order.len())
            else {
                panic!("every kind has a clearance, so every kind bounds it");
            };

            // Both automatic — or, for a kind with no distance, the clearance
            // alone — which is the state that has no answer.
            let mut loose = stage.clone();
            let order: Vec<Freedom> = group.order.iter().flatten().copied().collect();
            for f in &order {
                if let Some(a) = loose.input_mut(*f) {
                    a.auto = true;
                }
            }

            // Relief pins one, and never the one being touched.
            for just in &order {
                let mut fixed = loose.relieved(Some(*just));
                let automatic = order
                    .iter()
                    .filter(|f| fixed.input_mut(**f).is_some_and(|a| a.auto))
                    .count();
                assert!(
                    automatic <= group.automatic_at_most,
                    "{automatic} left automatic, at most {} allowed: {group:?}",
                    group.automatic_at_most
                );
                // `just` is spared **wherever sparing it leaves an answer**.
                // A group whose only input is the one being touched has none,
                // and the planetary's clearance is exactly that: with no
                // distance to derive it from, asking for it to be derived has
                // no answer and the toggle snapping back is the tool saying so.
                let could_spare = group.order.len() > group.automatic_at_most + 1;
                if could_spare {
                    assert!(
                        fixed.input_mut(*just).is_some_and(|a| a.auto),
                        "{just:?} was just set automatic and something else could have given"
                    );
                }
                checked += 1;
            }
        }
        // Two inputs on each of the two pair kinds, and the one input the two
        // epicyclic kinds each bound on its own.
        assert!(checked >= 6, "only {checked} cases");
    }

    /// **The declared limit is the stage's actual freedom** — asserted by giving
    /// exactly that many and requiring every one of them to be honoured, then
    /// giving one more and requiring that it cannot be.
    ///
    /// This is what stops the declaration being a number somebody wrote down. A
    /// pair's group says four of `{a, clearance, x₁, x₂, size}` may be given:
    /// with the distance and the clearance given the shifts move to reach it;
    /// with **both shifts** given as well the *size* — here the helix — moves
    /// instead; and with the size given too the distance can no longer be what
    /// was asked at the clearance that was asked, because nothing is left to
    /// absorb the difference.
    ///
    /// Stated as *the clearance the pair actually runs at*, which is the
    /// quantity the relation is about, rather than as a shift value — so it says
    /// the same thing whatever the division rule decides.
    #[test]
    fn the_declared_limit_is_the_freedom_the_stage_actually_has() {
        let lib = library();
        let clearance = 0.02_f64;
        let asked = 24.4199_f64 + clearance;

        let solve = |pin_shifts: bool, size_free: bool| {
            let mut sp = PairStage {
                clearance: Auto::fixed(clearance),
                centre_distance: Auto::fixed(asked),
                ..PairStage::default()
            };
            sp.gears[0].teeth = 9;
            sp.gears[1].teeth = 37;
            if pin_shifts {
                // Two shifts a designer might well type, and nowhere near the
                // pair the given distance wants.
                sp.gears[0].profile_shift = Auto::fixed(0.20);
                sp.gears[1].profile_shift = Auto::fixed(0.20);
            }
            // A stated size, or every reading of it left to the distance.
            sp = if size_free {
                sp.size_free()
            } else {
                sp.with_first_helix(0.0)
            };
            let mut t = two_stage();
            t.stages = vec![Stage::Spur(sp)];
            let r = solve_train(&t, &lib).expect("the stage solves either way");
            let s = r.stages[0].as_pair().expect("a spur stage");
            (s.clearance, s.centre_distance, s.gears[0].helix_angle)
        };

        // Two given, the shifts free — every given number stands.
        let (got, distance, helix) = solve(false, false);
        assert!(
            (got - clearance).abs() < 1e-6 && (distance - asked).abs() < 1e-9,
            "two given should all be honoured: clearance {got} at {distance}"
        );
        assert_eq!(helix, 0.0, "nothing asked the helix to move");

        // Four given with the size free: the helix reaches it instead.
        let (got, distance, helix) = solve(true, true);
        assert!(
            (got - clearance).abs() < 1e-6 && (distance - asked).abs() < 1e-9,
            "with the shifts pinned the size should absorb: clearance {got} at {distance}"
        );
        assert!(
            helix > 1.0,
            "a helix angle is what reached the distance: {helix}°"
        );

        // One more, and the relation cannot hold. The distance is still what was
        // typed — it is the *clearance* that gives, which is the arm of the
        // contradiction the panel resolves by relieving the distance.
        let (over, distance, _) = solve(true, false);
        assert!(
            (distance - asked).abs() < 1e-9,
            "the given distance is still the distance"
        );
        assert!(
            (over - clearance).abs() > 1e-3,
            "with everything pinned, the clearance cannot also be {clearance}: got {over}"
        );

        // ...and the group says exactly that many: five inputs bound by one
        // relation, so four may stand and the distance is the first to give —
        // the size being one entry of three readings, which comes last.
        let groups = Stage::Spur(PairStage::default()).freedoms();
        let relation = groups
            .iter()
            .find(|g| g.order.len() == 5)
            .expect("the pair's relation");
        assert_eq!(relation.given_at_most, 4);
        assert_eq!(relation.order[0], vec![Freedom::CentreDistance]);
        assert_eq!(relation.order[1], vec![Freedom::Clearance]);
        assert_eq!(
            relation.order[4],
            vec![
                Freedom::Member(0, MemberFreedom::Helix),
                Freedom::Member(1, MemberFreedom::Helix),
                Freedom::FirstPitchDiameter,
                Freedom::Overlap,
            ],
            "the size is the last to give: a shift moves the teeth, a size changes them — and the preset's widths are given, so the ratio is a reading of it"
        );
        // ...and with a width automatic the ratio is a floor, in no argument.
        let mut width_free = PairStage::default();
        width_free.gears[0].face_width = Auto::automatic(8.0);
        assert!(!mentioned(&Stage::Spur(width_free)).contains(&Freedom::Overlap));

        // And the second group is the one that stops *both* ways of saying the
        // distance being left automatic at once.
        let both = groups
            .iter()
            .find(|g| g.automatic_at_most < g.order.len())
            .expect("a distance and a clearance cannot both be derived");
        assert_eq!(both.automatic_at_most, 1);
        assert_eq!(both.order[0], vec![Freedom::Clearance]);
    }

    /// **A given distance and a given clearance decide the shifts — with the
    /// optimiser off as much as on.**
    ///
    /// This is mode 3 of the clearance paradigm
    /// (`docs/reference.md#which-of-the-three-numbers-is-given-and-which-follows`): of the three related
    /// numbers, any two given leave the third derived, so a housing distance and
    /// a running clearance fix the shift sum and the stage must hit it.
    ///
    /// It used to hold **only with the optimiser on**. `shifts_at` returned the
    /// undercut floor before ever looking at the distance, so the plainest thing
    /// a designer does — type a distance with nothing asked to move — ran the
    /// pair at whatever distance the floor happened to make and reported the
    /// shortfall as clearance. On a 9/37 pair at 24.42 mm that was 0.47 mm of
    /// "clearance" nobody asked for.
    ///
    /// The claim asserted is the identity `clearance == what was asked`, which
    /// says the sum was reached without naming any shift — the division is a
    /// separate rule ([`crate::auto::divide_shift_sum`]) and this is true
    /// whatever it decides. **Both paths are checked against the same distances**,
    /// so a search and a stated rule cannot come to different sums.
    #[test]
    fn a_given_distance_and_clearance_decide_the_shifts_either_way() {
        let lib = library();
        let mut checked = 0u32;
        for (z1, z2, distances) in [
            (9u32, 37u32, [23.4866_f64, 23.9532, 24.4199]),
            (17, 43, [30.3922, 30.7645, 31.1367]),
        ] {
            for a in distances {
                for clearance in [0.0_f64, 0.02, 0.20] {
                    let mut sums = Vec::new();
                    for optimiser in [false, true] {
                        let mut sp = PairStage {
                            clearance: Auto::fixed(clearance),
                            centre_distance: Auto::fixed(a + clearance),
                            ..PairStage::default()
                        };
                        sp.gears[0].teeth = z1;
                        sp.gears[1].teeth = z2;
                        sp.optimisation.enabled = optimiser;
                        let mut t = two_stage();
                        t.stages = vec![Stage::Spur(sp)];
                        let Ok(r) = solve_train(&t, &lib) else {
                            continue;
                        };
                        let s = r.stages[0].as_pair().expect("a spur stage");

                        checked += 1;
                        assert!(
                            (s.clearance - clearance).abs() < 1e-6,
                            "{z1}/{z2} at a={a} clearance={clearance} optimiser={optimiser}:                              the shifts leave {} of clearance, so the pair does not run at the                              distance it was given",
                            s.clearance
                        );
                        sums.push(s.gears[0].profile_shift + s.gears[1].profile_shift);
                    }
                    // One relation, two ways of satisfying it: the searched
                    // division and the stated one must reach the *same* sum,
                    // because the sum is not either one's to choose.
                    if let [off, on] = sums[..] {
                        assert!(
                            (off - on).abs() < 1e-6,
                            "{z1}/{z2} at a={a}: the optimiser reaches a sum of {on}                              and the stated division {off}"
                        );
                    }
                }
            }
        }
        assert!(checked >= 30, "only {checked} cases reached the distance");
    }

    /// **The reported clearance is the gap the stage runs at.**
    ///
    /// A centre distance is the true distance and a clearance is what portion of
    /// it is clearance, so the three numbers are two facts and a subtraction.
    /// `clearance` was the *input* echoed back instead, gated by whether
    /// anything was free to absorb it — which is right only when the distance is
    /// automatic, because then the running distance was built by adding it.
    ///
    /// Given a distance, it was wrong every way it could be: a pair told to run
    /// at 30.3 mm whose shifts put it at 30.0057 has 0.294 mm of clearance, and
    /// it reported 0.02 with the optimiser on and 0.000 with it off.
    ///
    /// Asserted as the identity rather than against those figures, so it says
    /// the same thing on every kind and at every distance.
    #[test]
    fn the_reported_clearance_is_the_gap_the_stage_runs_at() {
        let lib = library();
        let mut checked = 0u32;
        for distance in [None, Some(30.3_f64), Some(30.5)] {
            for clearance in [0.0_f64, 0.02, 0.20] {
                for optimiser in [false, true] {
                    let mut sp = PairStage {
                        clearance: Auto::fixed(clearance),
                        ..PairStage::default()
                    };
                    sp.optimisation.enabled = optimiser;
                    if let Some(a) = distance {
                        sp.centre_distance = Auto::fixed(a);
                    }
                    let mut t = two_stage();
                    t.stages = vec![Stage::Spur(sp), Stage::Worm(PairStage::worm())];
                    let Ok(r) = solve_train(&t, &lib) else {
                        continue;
                    };

                    let s = r.stages[0].as_pair().expect("a spur stage");
                    checked += 1;
                    assert!(
                        (s.clearance - (s.centre_distance - s.centre_distance_nominal)).abs()
                            < 1e-12,
                        "spur at a={distance:?} clearance={clearance} optimiser={optimiser}: \
                         reports {} of clearance between {} and {}",
                        s.clearance,
                        s.centre_distance_nominal,
                        s.centre_distance
                    );
                    // ...and the same identity on the kind that has no shift to
                    // absorb anything, which is where an echoed input and a
                    // derived gap part company hardest.
                    let (w, _) = worm(&r.stages[1]);
                    assert!(
                        (w.clearance - (w.centre_distance - w.centre_distance_nominal)).abs()
                            < 1e-12,
                        "worm: reports {} between {} and {}",
                        w.clearance,
                        w.centre_distance_nominal,
                        w.centre_distance
                    );
                }
            }
        }
        assert!(checked >= 12, "only {checked} configurations solved");
    }

    /// **A tolerance band opens the way round it says it does**, on every kind
    /// that reports one.
    ///
    /// Less centre distance is less room and so less play. That is one claim,
    /// and it was written out four times — once per stage kind, each closing
    /// over its own way of turning a distance into an angle. Getting it
    /// backwards in one of them would have produced a band that reads perfectly
    /// well and is inside out, and nothing anywhere asserted the direction.
    ///
    /// `Backlash::banded` is the one construction now; this is the claim it
    /// makes, checked through all four kinds rather than at the constructor,
    /// because the argument each passes is the part that could still be wrong.
    #[test]
    fn a_tolerance_band_widens_with_the_centre_distance() {
        let lib = library();
        let mut train = two_stage();
        train.stages = vec![
            Stage::Spur(PairStage::default()),
            Stage::Worm(PairStage::worm()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::default()),
            Stage::Spur(PairStage {
                shaft_angle: 90.0,
                ..PairStage::default()
            }),
        ];
        let r = solve_train(&train, &lib).expect("a train of every kind");

        let mut checked = 0u32;
        let mut check = |what: &str, b: &Backlash, opens: bool| {
            checked += 1;
            assert!(
                b.minimum <= b.nominal && b.nominal <= b.maximum,
                "{what}: the band runs {} … {} … {}, which is not an order",
                b.minimum,
                b.nominal,
                b.maximum
            );
            if opens {
                assert!(
                    b.maximum > b.minimum,
                    "{what}: a tolerance that opens nothing — {} either way",
                    b.minimum
                );
            } else {
                // **The one band a tolerance cannot open.** On the ideal ring,
                // `z_r = z_s + 2 z_p`, both meshes have the same reference
                // distance, so their operating angles are the same function of
                // the running distance and the referred play
                // `2 (z_s + z_p)(inv α_w,ring − inv α_w,sun)` is invariant in
                // it — to all orders, not merely to first. What the sun mesh
                // gains from a planet moved out, the ring mesh loses. The
                // shipped set is that set.
                assert!(
                    b.maximum - b.minimum < 1e-12,
                    "{what}: the ideal set's play should not move with its centre tolerance, \
                     but the band is {} wide",
                    b.maximum - b.minimum
                );
            }
        };

        for (k, stage) in r.stages.iter().enumerate() {
            let d = stage.backlash();
            let opens = !matches!(stage, StageResult::Planetary(_));
            check(&format!("stage {k} forward"), &d.forward, opens);
            check(&format!("stage {k} backward"), &d.backward, opens);
        }
        assert!(checked >= 10, "only {checked} bands checked");
    }

    /// **A set driven backward distributes its torque the way it distributes it
    /// driven backward**, not the way it does driven forward.
    ///
    /// The reverse of a mechanism is the same construction with the roles
    /// swapped, so the answer is the reverse *solve* — which this stage already
    /// performs, for its efficiency, and whose torques it discarded. What it
    /// reported instead was the forward distribution scaled by the ratio of the
    /// two stage torques, which is exact wherever the forward torque is a
    /// geometric projection or the two directional efficiencies agree. An
    /// epicyclic set is neither: **which shaft drives decides where `η₀`
    /// multiplies.** The shipped set's ring came out 6 % low.
    ///
    /// The law is the degenerate case rather than the figure: **at zero friction
    /// the two distributions coincide**, because with `η₀ = 1` there is no loss
    /// for the direction to place. So the difference *is* the efficiency, and
    /// that is checkable without knowing either number.
    /// **And the hula stage, which is an epicyclic power flow and had the same
    /// fault.** Its two central gears stand in for the sun and the ring.
    #[test]
    fn a_back_driven_hula_distributes_torque_by_its_own_solve() {
        let lib = library();
        let ratios = |mu: f64| {
            let h = HulaStage {
                sliding_friction: [mu; 2],
                static_friction: [mu; 2],
                ..HulaStage::default()
            };
            let mut t = two_stage();
            t.load_cases[BACK].torque = 0.5;
            t.stages = vec![Stage::Worm(PairStage::worm()), Stage::Hula(Box::new(h))];
            let r = solve_train(&t, &lib).expect("a train that solves");
            let s = r.stages[1].as_hula().expect("a hula stage");
            let at = |g: &HulaGear, case: usize| g.gear.cases[case].torque;
            (
                at(&s.gears[3], PEAK) / at(&s.gears[0], PEAK),
                at(&s.gears[3], BACK) / at(&s.gears[0], BACK),
            )
        };
        let (forward, backward) = ratios(0.0);
        assert!(
            (forward - backward).abs() < 1e-9,
            "with no friction the stage distributes torque alike either way, \
             but forward gives {forward} and backward {backward}"
        );
        let (forward, backward) = ratios(0.08);
        assert!(
            (forward - backward).abs() / forward.abs() > 1e-3,
            "with friction the two directions must place the loss differently, \
             but both give {forward}"
        );
    }

    #[test]
    fn a_back_driven_set_distributes_torque_by_its_own_solve() {
        let lib = library();
        let ratios = |mu: f64| {
            let mut set = PlanetaryStage {
                sliding_friction_sun_planet: mu,
                sliding_friction_planet_ring: mu,
                ..PlanetaryStage::default()
            };
            set.static_friction_sun_planet = mu;
            set.static_friction_planet_ring = mu;
            let mut t = two_stage();
            t.load_cases[BACK].torque = 0.5;
            // A self-locking stage at the input end, so the set reacts the load.
            t.stages = vec![
                Stage::Worm(PairStage::worm()),
                Stage::Planetary(Box::new(set)),
            ];
            let r = solve_train(&t, &lib).expect("a train that solves");
            let p = r.stages[1].as_planetary().expect("a planetary stage");
            let at = |g: &GearResult, case: usize| g.cases[case].torque;
            (
                at(&p.ring, PEAK) / at(&p.sun, PEAK),
                at(&p.ring, BACK) / at(&p.sun, BACK),
            )
        };

        // Frictionless: nothing for the direction to place, so the two
        // distributions are the same one.
        let (forward, backward) = ratios(0.0);
        assert!(
            (forward - backward).abs() < 1e-9,
            "with no friction the set distributes torque alike either way, \
             but forward gives {forward} and backward {backward}"
        );

        // With friction they part, and that difference is the whole finding:
        // scaling the forward answer would have kept them equal at every `mu`.
        let (forward, backward) = ratios(0.06);
        assert!(
            (forward - backward).abs() / forward > 1e-3,
            "with friction the two directions must place the loss differently, \
             but both give {forward}"
        );
    }

    /// **The three cases a train used to hold as fields** — a peak at the
    /// start, a load from the end that nothing holds unless a stage does, and
    /// the load it runs at — so a test written against them reads as it did.
    /// Index a member's cases by these.
    const PEAK: usize = 0;
    const BACK: usize = 1;
    const CYCLIC: usize = 2;
    fn classic(
        input_speed: f64,
        input_torque: f64,
        back_driving_torque: f64,
        operating_torque: f64,
        operating_speed: f64,
        duty: Duty,
    ) -> Vec<LoadCase> {
        vec![
            LoadCase::ultimate(input_torque, input_speed),
            LoadCase {
                port: Port::End,
                reacted: false,
                ..LoadCase::ultimate(back_driving_torque, 0.0)
            },
            LoadCase {
                duty,
                ..LoadCase::fatigue(operating_torque, operating_speed)
            },
        ]
    }

    /// **A chain of three kinds**, so the graph's assembly meets a stage with
    /// three shafts sitting between two with two — which a train of one kind
    /// cannot exercise and which is where a coupling to the wrong shaft would
    /// show.
    fn mixed_train() -> Train {
        let mut t = two_stage();
        t.stages.insert(1, Stage::Planetary(Box::default()));
        t.stages.push(Stage::Worm(PairStage::worm()));
        t
    }

    fn two_stage() -> Train {
        Train {
            load_cases: classic(3000.0, 2.0, 0.0, 2.0, 3000.0, Duty::default()),
            reversed_bending: false,
            stages: vec![
                Stage::Spur(PairStage::default()),
                Stage::Spur(PairStage {
                    gears: [
                        StageGear {
                            teeth: 13,
                            ..StageGear::default()
                        },
                        StageGear {
                            teeth: 31,
                            ..StageGear::default()
                        },
                    ],
                    ..PairStage::default()
                }),
            ],
            couplings: Vec::new(),
            constraints: Vec::new(),
        }
    }

    #[test]
    fn a_two_stage_train_computes_end_to_end() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        assert_eq!(r.stages.len(), 2);

        // 43/17 * 31/13
        let want = (43.0 / 17.0) * (31.0 / 13.0);
        assert!((r.total_ratio - want).abs() < 1e-12);
        assert!((r.cases[PEAK].delivered_speed - 3000.0 / want).abs() < 1e-9);

        // Every stage produced real numbers.
        for s in r.stages.iter().map(spur) {
            assert!(s.centre_distance > 0.0);
            assert!(s.mesh.line.unwrap().contact_ratios.transverse > 1.0);
            assert!(s.mesh.efficiency.forward > 0.9 && s.mesh.efficiency.forward < 1.0);
            assert_eq!(
                s.mesh.efficiency.forward, s.mesh.efficiency.backward,
                "a parallel-axis stage is as efficient driven either way"
            );
            assert!(s.mesh.cases[0].contact.at_pitch_point > 0.0);
            for g in &s.gears {
                assert!(g.face_width > 0.0);
                assert!(g.cases[0].bending_stress.unwrap() > 0.0);
            }
        }
    }

    /// **The two backlash figures are one gap seen from the two ends.**
    ///
    /// Referred to the output shaft or to the input shaft, the same play must
    /// differ by exactly the total ratio — every stage's contribution scales the
    /// same way, because a stage's own two figures are its gap at two lever arms
    /// whose ratio *is* that stage's ratio. If a stage ever got that wrong the
    /// products would stop matching, which no per-stage check would catch.
    #[test]
    fn backlash_at_the_two_ends_differs_by_exactly_the_total_ratio() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        for (forward, backward) in [
            (r.backlash.forward.nominal, r.backlash.backward.nominal),
            (r.backlash.forward.maximum, r.backlash.backward.maximum),
        ] {
            assert!(forward > 0.0);
            assert!(
                (backward - forward * r.total_ratio).abs() < 1e-9 * backward,
                "{backward} vs {forward} x {}",
                r.total_ratio
            );
        }
        // ...and the input end is the looser one, because it turns faster.
        assert!(r.backlash.backward.nominal > r.backlash.forward.nominal);
    }

    /// A train of parallel-axis stages is as efficient driven either way, and
    /// cannot lock. Both are consequences of the meshes, not rules the train
    /// applies.
    #[test]
    fn a_parallel_axis_train_reports_equal_efficiencies_and_cannot_lock() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        assert_eq!(r.total_efficiency.forward, r.total_efficiency.backward);
        assert!(!r.total_efficiency.locked().backward);
    }

    /// Efficiency must always *reduce* delivered torque. Getting this sign wrong
    /// is the classic train-accumulation bug, and it hides because the ratio term
    /// is so much larger.
    #[test]
    fn efficiency_always_costs_torque() {
        let lib = library();
        let mut lossless = two_stage();
        for s in &mut lossless.stages {
            spur_input(s).sliding_friction = 0.0;
            spur_input(s).static_friction = 0.0;
        }
        let ideal = solve_train(&lossless, &lib).unwrap();
        let real = solve_train(&two_stage(), &lib).unwrap();

        assert!((ideal.total_efficiency.forward - 1.0).abs() < 1e-12);
        let (real_out, ideal_out) = (
            real.cases[PEAK].delivered_torque,
            ideal.cases[PEAK].delivered_torque,
        );
        assert!(real_out < ideal_out);
        // ...and the shortfall is exactly the product of the stage efficiencies.
        assert!((real_out - ideal_out * real.total_efficiency.forward).abs() < 1e-9);
    }

    /// **An automatic profile shift asks about the depth the tooth actually
    /// has.**
    ///
    /// `working_depth` is the depth the undercut question is asked at, and it
    /// now follows the **dedendum** by default instead of sitting at a fixed
    /// module. The two are different questions — "is the flank undercut within a
    /// module of depth?" against "is it undercut at all?" — and at α = 20° with
    /// a sharp rack they part company at 18 teeth and 22 (docs/reference.md#automatic-values). Following the
    /// dedendum also means a gear cut shallower is asked about its own depth
    /// rather than about a convention.
    ///
    /// Gated because nothing did: when the default moved, every figure in
    /// `gear-cli train` moved with it and the suite stayed green. Asserted as
    /// the law rather than the numbers — a deeper cut can only need more shift
    /// to stay clear of undercut — plus the one identity that pins it, which is
    /// that fixing `working_depth` at the dedendum's value reproduces automatic
    /// exactly.
    #[test]
    fn an_automatic_profile_shift_follows_the_dedendum() {
        let lib = library();
        let shift_of = |dedendum: f64, working: Auto<f64>| {
            let stage = PairStage {
                gears: [
                    StageGear {
                        teeth: 15,
                        dedendum,
                        working_depth: working,
                        profile_shift: Auto::automatic(0.0),
                        ..Default::default()
                    },
                    StageGear {
                        teeth: 43,
                        dedendum,
                        working_depth: working,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            };
            solve_spur_stage(&stage, &StageLoads::just(2.0), &lib)
                .expect("a solvable stage")
                .gears[0]
                .profile_shift
        };

        // Deeper teeth, more shift. A law: undercut is a question about how far
        // down the flank has to stay clean.
        let mut previous = f64::NEG_INFINITY;
        for dedendum in [1.0_f64, 1.25, 1.5, 1.9] {
            let x = shift_of(dedendum, Auto::automatic(0.0));
            assert!(
                x > previous,
                "dedendum {dedendum}: a deeper cut cannot need less shift — \
                 {x} against {previous}"
            );
            previous = x;
        }

        // ...and automatic *is* the dedendum, not something near it.
        for dedendum in [1.0_f64, 1.25, 1.9] {
            let automatic = shift_of(dedendum, Auto::automatic(0.0));
            let named = shift_of(dedendum, Auto::fixed(dedendum));
            assert!(
                (automatic - named).abs() < 1e-12,
                "dedendum {dedendum}: automatic gave {automatic}, the dedendum \
                 by hand gave {named}"
            );
        }

        // The old default is still reachable and still different, so this is a
        // change of default rather than a loss of the control.
        let classical = shift_of(1.25, Auto::fixed(1.0));
        let now = shift_of(1.25, Auto::automatic(0.0));
        assert!(
            now > classical + 1e-6,
            "asking a module deep should ask for less shift than asking 1.25 \
             deep: {classical} against {now}"
        );
    }

    /// **A parallel stage is rated where it runs, not where it was designed.**
    ///
    /// `Mesh::a_w` is the **zero-backlash** centre distance — where the profile
    /// shifts put the pair — and a real one runs at that plus its assembly
    /// clearance. Every contact quantity belongs to the second: the path
    /// shortens, the operating pressure angle opens, the relative curvature
    /// grows. Rating at `a_w` was rating a pair nobody assembles, and the
    /// clearance is not a detail to round away — it is the reason the stage has
    /// any backlash to report at all.
    ///
    /// The direction is the law and is asserted as one: separating the centres
    /// can only shorten the path of contact. Everything downstream follows —
    /// less load sharing, so more bending stress — which is why the numbers
    /// moved when this landed (docs/reference.md#centre-distance-and-backlash).
    ///
    /// Backlash is deliberately *not* in this test's scope: it measures play
    /// against the zero-backlash reference and keeps the design mesh. That
    /// division is gated in `mesh.rs`.
    #[test]
    fn a_parallel_stage_is_rated_at_the_centre_distance_it_runs_at() {
        let lib = library();
        let stage = |clearance: f64| PairStage {
            clearance: Auto::fixed(clearance),
            ..PairStage::default()
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solve_spur_stage(&stage(clearance), &StageLoads::just(2.0), &lib).unwrap();
            let eps = r.mesh.line.unwrap().contact_ratios.transverse;
            let bending = r.gears[0].cases[0]
                .bending_stress
                .expect("a rateable tooth");
            if let Some((was_eps, was_bending)) = previous {
                assert!(
                    eps < was_eps,
                    "clearance {clearance}: separating the centres can only \
                     shorten the path — ε {eps} against {was_eps}"
                );
                assert!(
                    bending > was_bending,
                    "clearance {clearance}: a shorter path is less load sharing, \
                     so the tooth carries more — {bending} against {was_bending}"
                );
            }
            previous = Some((eps, bending));
        }
    }

    #[test]
    fn a_spur_stage_has_exactly_zero_overlap_and_a_helical_one_does_not() {
        let lib = library();
        let spur = solve_spur_stage(&PairStage::default(), &StageLoads::just(2.0), &lib).unwrap();
        assert_eq!(
            spur.mesh.line.unwrap().contact_ratios.overlap,
            0.0,
            "must be exactly zero"
        );
        assert_eq!(
            spur.mesh.line.unwrap().contact_ratios.total,
            spur.mesh.line.unwrap().contact_ratios.transverse
        );
        assert!(!spur
            .mesh
            .line
            .unwrap()
            .contact_ratios
            .has_full_axial_overlap());

        let helical = solve_spur_stage(
            &PairStage {
                ..PairStage::default()
            }
            .with_additional_helix(20.0),
            &StageLoads::just(2.0),
            &lib,
        )
        .unwrap();
        assert!(helical.mesh.line.unwrap().contact_ratios.overlap > 0.0);
        assert!(
            helical.mesh.line.unwrap().contact_ratios.total
                > helical.mesh.line.unwrap().contact_ratios.transverse
        );
    }

    /// The last stage dominates output backlash, which is the design consequence
    /// worth surfacing: tolerance spent at the input end is nearly free.
    #[test]
    fn backlash_referred_to_the_output_is_dominated_by_the_last_stage() {
        let lib = library();
        let base = two_stage();

        let loosen = |k: usize| {
            let mut t = base.clone();
            spur_input(&mut t.stages[k]).clearance.manual *= 4.0;
            solve_train(&t, &lib).unwrap().backlash.forward.nominal
        };

        let reference = solve_train(&base, &lib).unwrap().backlash.forward.nominal;
        let first = loosen(0) - reference;
        let last = loosen(1) - reference;
        assert!(first > 0.0 && last > 0.0);
        assert!(
            last > first * 2.0,
            "the last stage should dominate: {last} vs {first}"
        );
    }

    #[test]
    fn thickness_modification_cannot_break_its_own_invariant() {
        let stage = PairStage {
            thickness_mod: 1.3,
            ..PairStage::default()
        };
        let k: Vec<f64> = (0..2)
            .map(|i| stage.params_at(i, stage.shifts()[i]).thickness_mod)
            .collect();
        assert!((k[0] + k[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_automatic_face_width_is_the_larger_of_the_enabled_checks() {
        let lib = library();
        let off = ByKind {
            ultimate: false,
            fatigue: false,
        };
        let width = |sources: FaceSources| {
            let mut s = PairStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
            }
            solve_spur_stage(&s, &StageLoads::just(2.0), &lib)
                .unwrap()
                .gears[0]
                .face_width
        };
        // One source at a time, then every combination of them: the width is the
        // largest of whatever is enabled, and that is the whole rule.
        let each: Vec<f64> = [
            FaceSources {
                bending: ByKind {
                    ultimate: true,
                    fatigue: false,
                },
                contact: off,
            },
            FaceSources {
                bending: ByKind {
                    ultimate: false,
                    fatigue: true,
                },
                contact: off,
            },
            FaceSources {
                bending: off,
                contact: ByKind {
                    ultimate: true,
                    fatigue: false,
                },
            },
            FaceSources {
                bending: off,
                contact: ByKind {
                    ultimate: false,
                    fatigue: true,
                },
            },
        ]
        .into_iter()
        .map(width)
        .collect();
        // The law is about what is *enabled*, so it is checked against every
        // source switched on rather than against the default — which is a
        // separate decision, and is asserted as one below.
        let on = ByKind {
            ultimate: true,
            fatigue: true,
        };
        let all = width(FaceSources {
            bending: on,
            contact: on,
        });
        let largest = each.iter().copied().fold(0.0_f64, f64::max);
        assert!((all - largest).abs() < 1e-9, "{all} vs max{each:?}");

        // Peak and cyclic are the same torque on this train, so what separates
        // them is only the allowable — and the ultimate is above the fatigue
        // figure, so the cyclic case is the one that asks for more face.
        assert!(each[1] > each[0] && each[3] > each[2]);
        // Contact governs a lightly loaded steel pair, as it usually does.
        assert!(each[3] > each[1]);

        // **A fresh stage is sized from bending alone.** Both contact ratings
        // are computed and both are offered; neither decides a width until a
        // designer says so, because the cyclic one dominates by an order of
        // magnitude and a default that picks the answer is a default making the
        // design decision.
        let d = FaceSources::default();
        assert_eq!(d.bending, on, "bending is what sizes a fresh stage");
        assert_eq!(d.contact, off, "neither contact rating is assumed");
        assert!(
            (width(d) - each[0].max(each[1])).abs() < 1e-9,
            "the default width is the bending pair and nothing else"
        );

        // Nothing enabled asks for nothing, which is a degenerate gear rather
        // than a divide by zero.
        assert_eq!(
            width(FaceSources {
                bending: off,
                contact: off
            }),
            0.0
        );
    }

    /// An intermittent duty measured at the end port makes upstream gears turn
    /// further, not less. Getting the direction backwards would silently
    /// under-count cycles on exactly the gears that see the most — and the
    /// same sweep stated at the start port is the whole ratio fewer turns
    /// everywhere, which is what a named port means.
    #[test]
    fn intermittent_cycles_are_worked_backwards_from_the_output() {
        let mut t = two_stage();
        t.load_cases[CYCLIC].duty = Duty::Intermittent {
            range_degrees: 360.0,
            at: Port::End,
            actuations: 100,
            reversing: false,
        };
        let r = solve_train(&t, &library()).unwrap();
        let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");

        // The output gear turns exactly once per actuation. A whole number of
        // revolutions, so the rounding has nothing to do and the count is the
        // revolutions exactly — which is the case worth putting first, because
        // it is where a count and a revolution coincide.
        let last = cycles(&spur(&r.stages[1]).gears[1]);
        assert_eq!(last.bending, 100.0);
        // Not reversing, so one flank takes every engagement.
        assert_eq!(last.contact, last.bending);
        // An ultimate case is survived once and counts nothing.
        assert!(spur(&r.stages[1]).gears[1].cases[PEAK].cycles.is_none());

        // Every gear upstream turns more than the one after it.
        let seq = [
            cycles(&spur(&r.stages[0]).gears[0]).bending,
            cycles(&spur(&r.stages[0]).gears[1]).bending,
            cycles(&spur(&r.stages[1]).gears[1]).bending,
        ];
        for w in seq.windows(2) {
            assert!(w[0] > w[1], "cycles must fall towards the output: {seq:?}");
        }
        // The input gear sees the whole train ratio's worth — **rounded up**,
        // because these are engagements rather than revolutions and a tooth
        // three quarters of the way through one has still been loaded by it.
        assert_eq!(seq[0], (100.0 * r.total_ratio).ceil());

        // The two gears meshing with each other turn at different speeds but
        // share a mesh, so their cycle counts differ by that stage ratio — and
        // only **up to the rounding**, now that each count is a whole number of
        // engagements rather than a revolution count.
        //
        // The bound is derived rather than chosen: rounding adds less than one
        // to each, so `a/b` moves from `A/B` by less than `(1 + ratio)/b`. On
        // this train that is 6e-8, which is why the old exact-to-1e-9 assertion
        // was the thing that had to change and not the arithmetic.
        let s0 = spur(&r.stages[0]);
        let (a, b) = (cycles(&s0.gears[0]).bending, cycles(&s0.gears[1]).bending);
        // Counts are magnitudes; the ratio carries a sign now.
        let ratio = s0.ratio.abs();
        assert!(
            (a / b - ratio).abs() < (1.0 + ratio) / b,
            "{a}/{b} = {} against a stage ratio of {}",
            a / b,
            ratio
        );

        // The same sweep at the start port: the input gear turns exactly once
        // per actuation, and every count is the total ratio smaller.
        if let Duty::Intermittent { at, .. } = &mut t.load_cases[CYCLIC].duty {
            *at = Port::Start;
        }
        let r = solve_train(&t, &library()).unwrap();
        assert_eq!(cycles(&spur(&r.stages[0]).gears[0]).bending, 100.0);
        assert_eq!(
            cycles(&spur(&r.stages[1]).gears[1]).bending,
            (100.0 / r.total_ratio).ceil()
        );
    }

    #[test]
    fn continuous_cycles_follow_each_gears_own_speed() {
        let mut t = two_stage();
        t.load_cases[CYCLIC].speed = 1500.0;
        t.load_cases[CYCLIC].duty = Duty::Continuous { runtime_hours: 2.0 };
        let r = solve_train(&t, &library()).unwrap();
        let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");

        // Input gear: 1500 rpm * 60 min * 2 h — its own speed, for as long as
        // the train runs. Rounded up, as every count is.
        let want = (1500.0_f64 * 60.0 * 2.0).ceil();
        let first = cycles(&spur(&r.stages[0]).gears[0]);
        assert!((first.bending - want).abs() < 1e-6);
        // A continuous duty has no actuation to reverse within, so the two
        // counts agree.
        assert_eq!(first.bending, first.contact);
        // Speeds fall through the train, and cycles follow them — each case
        // at its own.
        assert!((spur(&r.stages[0]).gears[0].cases[PEAK].speed - 3000.0).abs() < 1e-9);
        assert!((spur(&r.stages[0]).gears[0].cases[CYCLIC].speed - 1500.0).abs() < 1e-9);
        assert!(
            (spur(&r.stages[1]).gears[1].cases[PEAK].speed - r.cases[PEAK].delivered_speed).abs()
                < 1e-9
        );
        assert!(cycles(&spur(&r.stages[1]).gears[1]).bending < first.bending);
    }

    /// **An epicyclic member is engaged once per turn against the carrier**, per
    /// planet — for every member and both kinds, which is the whole of
    /// [`engagements`].
    ///
    /// Checked against arithmetic the stage shares nothing with: the counts a
    /// member's own speed and the carrier's give, taken from the result's speed
    /// array rather than from the rule that filled the cycles. Three things
    /// would have failed here before it existed, and each was a different way of
    /// counting the wrong revolutions:
    ///
    /// - the **ring** of a set with the ring held is loaded while it does not
    ///   turn at all, and it counted the *sun's* revolutions — three and a half
    ///   times too many on these counts;
    /// - the **sun** counted its own rather than its rotation against the
    ///   carrier, which is 1.4 times too many;
    /// - the **planet** was referred to the sun's speed rather than to the input
    ///   shaft's, right only where those are the same shaft.
    ///
    /// # And a fourth, which this test asserted for a while
    ///
    /// *Per planet* is true of a **central** member and false of a planet. A
    /// sun tooth passes all N planets in one turn against the carrier; a planet
    /// tooth meets the one sun, because it *is* one of the N and the others
    /// have their own teeth. Applying the count to all three members reported
    /// the shipped set's planet cycles **three times over**, and this test
    /// asserted it, having been written from the same expression.
    ///
    /// The rule in `docs/reference.md#tooth-cycles` was right all along —
    /// *once for each parallel mesh path* — and what was missing was anywhere
    /// for the two members of a mesh to differ. `Wiring::paths_seen` is that
    /// somewhere, and it is what this now reads: **the expectation comes from
    /// the wiring's own answer**, so a kind that gets its mounts wrong fails
    /// here rather than agreeing with itself.
    #[test]
    fn an_epicyclic_members_cycles_are_its_turns_against_the_carrier() {
        let lib = library();
        for (name, stage) in [
            (
                "epicyclic",
                Stage::Planetary(Box::new(PlanetaryStage {
                    planets: 3,
                    ..PlanetaryStage::default()
                })),
            ),
            ("hula", Stage::Hula(Box::default())),
        ] {
            let mut train = two_stage();
            train.load_cases[CYCLIC].duty = Duty::Continuous { runtime_hours: 1.0 };
            train.stages = vec![stage];
            let r = solve_train(&train, &lib).unwrap_or_else(|e| panic!("{name}: {e}"));
            let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");
            // The revolutions the input shaft turns over the duty, which is what
            // every member's count is a multiple of.
            let turns = 3000.0 * 60.0;

            let want = |member: f64, carrier: f64, paths: f64| {
                (turns * ((member - carrier) / 3000.0).abs() * paths).ceil()
            };
            match &r.stages[0] {
                StageResult::Planetary(p) => {
                    let shafts = &p.cases[CYCLIC];
                    let carrier = shafts.speeds[1];
                    let n = f64::from(p.planets);
                    let w = train.stages[0].wiring();
                    for (member, which, got, speed) in [
                        (0, "sun", cycles(&p.sun).bending, shafts.speeds[0]),
                        (2, "ring", cycles(&p.ring).bending, shafts.speeds[2]),
                        (
                            1,
                            "planet",
                            cycles(&p.planet.gear).bending,
                            p.planet.gear.cases[CYCLIC].speed,
                        ),
                    ] {
                        let paths = f64::from(w.paths_seen(member));
                        let expected = want(speed, carrier, paths);
                        assert!(
                            (got - expected).abs() <= 1.0,
                            "{which}: {got} engagements against {expected}"
                        );
                    }
                    // ...and the two answers are genuinely different, or the
                    // reading above would pass on a wiring that had never heard
                    // of a planet.
                    assert_eq!(w.paths_seen(0), p.planets, "a sun meets every planet");
                    assert_eq!(w.paths_seen(2), p.planets, "and so does a ring");
                    assert_eq!(w.paths_seen(1), 1, "a planet meets the one sun");
                    assert!(n > 1.0, "this fixture has to have more than one planet");
                    // **A shaft that does not turn is still loaded**, which is
                    // the case the old rule could not state: it counted the
                    // input's revolutions, so a held ring came out as though it
                    // turned with the sun. It meets a planet once per *carrier*
                    // turn instead, which is `z_s/(z_s + z_r)` of that.
                    assert_eq!(shafts.speeds[2], 0.0, "the ring is the held shaft here");
                    let zs = f64::from(PlanetaryStage::default().sun.teeth);
                    let zr = f64::from(PlanetaryStage::default().ring.teeth);
                    assert!(
                        (cycles(&p.ring).bending
                            - (turns * zs / (zs + zr) * f64::from(p.planets)).ceil())
                        .abs()
                            <= 1.0,
                        "a held ring counts carrier turns: {}",
                        cycles(&p.ring).bending
                    );
                }
                StageResult::Hula(h) => {
                    let crank = h.cases[CYCLIC].speeds[1];
                    for g in &h.gears {
                        let c = &g.gear.cases[CYCLIC];
                        let expected = want(c.speed, crank, 1.0);
                        assert!(
                            (cycles(&g.gear).bending - expected).abs() <= 1.0,
                            "z{}: {} engagements against {expected}",
                            g.teeth,
                            cycles(&g.gear).bending
                        );
                        // ...and the speed its teeth see is the one against the
                        // crank, reported rather than left to be subtracted.
                        assert!((c.speed_against_carrier - (c.speed - crank)).abs() < 1e-9);
                    }
                    // The grounded gear stands still and is engaged once a crank
                    // turn, which is the whole duty's worth of revolutions.
                    assert!(
                        (cycles(&h.gears[0].gear).bending - turns).abs() <= 1.0,
                        "the grounded gear meets the wobble body once a crank turn"
                    );
                }
                _ => panic!("{name}: wrong kind"),
            }
        }
    }

    /// **The tip width is a bound on the addendum, not a target for it.**
    ///
    /// Exercised through a whole stage rather than in isolation: ask for a
    /// minimum tip width, solve, and measure the tip off the gear the stage
    /// actually built. Two things have to hold and they are different
    /// statements — the tip is never narrower than was asked for, and where the
    /// bound bit it bit *exactly*, leaving the tallest tooth that keeps the tip
    /// rather than an arbitrary shorter one.
    ///
    /// It used to be a target, because the addendum's only automatic value was
    /// the tooth this bound allows — so an addendum a designer typed went
    /// unbounded and one they left automatic was overwritten. Neither is what
    /// either control was for.
    #[test]
    fn the_tip_width_bounds_the_addendum_and_bites_exactly() {
        for want in [0.05, 0.15, 0.3] {
            for asked in [0.8, 1.0, 1.6] {
                let mut stage = PairStage::default();
                for g in &mut stage.gears {
                    g.addendum = asked;
                    g.min_tip_width = want;
                }
                let r = solve_spur_stage(&stage, &StageLoads::just(2.0), &library()).unwrap();

                for i in 0..2 {
                    let built = Tooth::new(stage.params_at(i, stage.shifts()[i]));
                    let got = 2.0 * built.ra * built.theta_a;
                    assert!(
                        got > want - 1e-9,
                        "gear {i} at h={asked}: tip {got} is under the {want} asked for"
                    );
                    let clamped = built.params.addendum < asked - 1e-12;
                    assert!(
                        !clamped || (got - want).abs() < 1e-9,
                        "gear {i} at h={asked}: held down to {} but the tip is {got}, not {want}",
                        built.params.addendum
                    );
                    assert!(
                        clamped || (built.params.addendum - asked).abs() < 1e-12,
                        "gear {i} at h={asked}: unclamped, so the addendum should be as asked"
                    );
                    // ...and the addendum reported is the one that produced it.
                    assert!((r.gears[i].addendum - built.params.addendum).abs() < 1e-12);
                }
            }
        }
    }

    /// **An automatic width with nothing to size it is said, not divided by.**
    ///
    /// The note has promised exactly that since it was written, and the stage
    /// then resolved the width to zero and divided by it: every stress came out
    /// infinite and every minimum width a NaN. Both cross the boundary as JSON
    /// `null` and draw as blanks, so the browser was honest by accident — while
    /// the CLI printed `inf`, and the generated TypeScript said `number` of a
    /// field that could arrive `null`.
    ///
    /// Asked of all three stage kinds that have the control, because it is one
    /// rule and this is the shape of a bound reaching the search it was written
    /// in and no other.
    #[test]
    fn a_width_with_no_rating_to_size_it_stands_where_it_was() {
        let lib = library();
        let off = FaceSources {
            bending: ByKind {
                ultimate: false,
                fatigue: false,
            },
            contact: ByKind {
                ultimate: false,
                fatigue: false,
            },
        };
        const GIVEN: f64 = 7.5;
        let gear = || StageGear {
            face_width: Auto::automatic(GIVEN),
            face_sources: off,
            ..StageGear::default()
        };

        let mut spur = PairStage::default();
        for g in &mut spur.gears {
            *g = StageGear {
                teeth: g.teeth,
                ..gear()
            };
        }
        let mut set = PlanetaryStage::default();
        for g in [&mut set.sun, &mut set.planet, &mut set.ring] {
            *g = StageGear {
                teeth: g.teeth,
                profile_shift: g.profile_shift,
                ..gear()
            };
        }
        let mut hula = HulaStage::default();
        for g in &mut hula.gears {
            *g = StageGear {
                teeth: g.teeth,
                addendum: g.addendum,
                dedendum: g.dedendum,
                ..gear()
            };
        }

        let spur_r = solve_spur_stage(&spur, &StageLoads::just(2.0), &lib).unwrap();
        let set_r = solve_planetary_stage(&set, &StageLoads::just(2.0), &lib).unwrap();
        let hula_r = solve_hula_stage(&hula, &StageLoads::just(2.0), &lib).unwrap();

        let members: Vec<&GearResult> = spur_r
            .gears
            .iter()
            .chain([&set_r.sun, &set_r.planet.gear, &set_r.ring])
            .chain(hula_r.gears.iter().map(|g| &g.gear))
            .collect();
        assert_eq!(members.len(), 9);
        for g in members {
            assert!(
                (g.face_width - GIVEN).abs() < 1e-12,
                "a width nothing sizes stands at the number in its box, not {}",
                g.face_width
            );
            // ...and with a width, every figure taken at one is a number.
            assert!(
                g.cases[0].contact_stress.is_finite()
                    && g.cases[0]
                        .min_face_width
                        .contact
                        .is_some_and(f64::is_finite),
                "contact: {} and {:?}",
                g.cases[0].contact_stress,
                g.cases[0].min_face_width.contact
            );
            if let Some(s) = g.cases[0].bending_stress {
                assert!(s.is_finite(), "bending: {s}");
            }
        }
        // The note still fires — the point is that it is now the *only* thing
        // that happens, not that it stopped happening — and it fires on the
        // gear whose width it is about, on every kind, not in a stage's list.
        let members: Vec<&GearResult> = spur_r
            .gears
            .iter()
            .chain([&set_r.sun, &set_r.planet.gear, &set_r.ring])
            .chain(hula_r.gears.iter().map(|g| &g.gear))
            .collect();
        for (i, g) in members.iter().enumerate() {
            assert!(
                g.notes.iter().any(|n| n.is(key::GEAR_FACE_WIDTH_NO_SOURCE)),
                "member {i} should say no rating sizes its width: {:?}",
                g.notes
            );
        }
    }

    /// **The sharing model reaches every member of every kind that has one.**
    ///
    /// It was a spur input only, so the one estimate this crate ships reached
    /// one stage of three — and a ring had no shared section at all, so even
    /// that stage would have rated one member of an internal mesh under the
    /// model and the other without.
    ///
    /// Asked as *does switching it on move the number*, member by member, since
    /// a member the input never reaches reports the same stress either way and
    /// looks exactly like one the model happens not to relieve. Above a virtual
    /// contact ratio of 2 there is no single-pair zone, the ramp never reaches a
    /// full share, and every member it reaches is relieved — so the fixtures are
    /// chosen to put each mesh there, which for a spur pair and an epicyclic set
    /// is an ordinary high-contact-ratio tooth.
    #[test]
    fn the_sharing_model_reaches_every_member_that_bends() {
        use crate::contact::LoadSharing;
        let lib = library();
        // Tall enough that every mesh is above `ε_n = 2` **where it runs**:
        // the set's sun–planet mesh sits at 1.997 at 1.35 modules once rated a
        // clearance off its zero-backlash distance, which is just the wrong
        // side of the band and exactly what this test would misread as a
        // member the model does not reach.
        let tall = |g: &StageGear| StageGear {
            addendum: 1.4,
            ..g.clone()
        };
        let bending_of = |sharing: LoadSharing| {
            let mut spur = PairStage {
                load_sharing: sharing,
                ..PairStage::default()
            };
            for g in &mut spur.gears {
                *g = tall(g);
            }
            let mut set = PlanetaryStage {
                load_sharing: sharing,
                ..PlanetaryStage::default()
            };
            // Named rather than the shipped counts: a set with a small sun
            // cannot reach the band on its sun mesh at any addendum a tooth
            // can carry, and what this test asks is of a set that does.
            set.sun = tall(&set.sun);
            set.planet = tall(&set.planet);
            set.ring = tall(&set.ring);
            set.sun.teeth = 24;
            set.planet.teeth = 18;
            set.ring.teeth = 60;
            // **A hula stage needs a taller tooth than it can be built with**,
            // and that is the point of the row below rather than a defect in
            // the fixture: at 1.1 modules its meshes reach `ε_n ≈ 2.02` and its
            // teeth foul, which the stage reports. The rating path is the one a
            // buildable stage uses, so this is what says the input reaches it.
            let mut hula = HulaStage {
                load_sharing: sharing,
                ..HulaStage::default()
            };
            for g in &mut hula.gears {
                g.addendum = 1.1;
            }

            let s = solve_spur_stage(&spur, &StageLoads::just(2.0), &lib).unwrap();
            let p = solve_planetary_stage(&set, &StageLoads::just(2.0), &lib).unwrap();
            let h = solve_hula_stage(&hula, &StageLoads::just(2.0), &lib).unwrap();
            let mut out: Vec<(String, Option<f64>)> = Vec::new();
            for (i, g) in s.gears.iter().enumerate() {
                out.push((format!("spur {i}"), g.cases[0].bending_stress));
            }
            for (what, g) in [
                ("sun", &p.sun),
                ("planet", &p.planet.gear),
                ("ring", &p.ring),
            ] {
                out.push((what.to_string(), g.cases[0].bending_stress));
            }
            for g in &h.gears {
                out.push((format!("hula z{}", g.teeth), g.gear.cases[0].bending_stress));
            }
            out
        };

        let off = bending_of(LoadSharing::None);
        let on = bending_of(LoadSharing::LinearRamp);
        assert_eq!(off.len(), on.len());
        let mut rated = 0;
        for ((what, a), (_, b)) in off.iter().zip(&on) {
            let (Some(a), Some(b)) = (a, b) else {
                continue; // a member with no notch has no bending either way
            };
            rated += 1;
            // **That it moved is the claim; which way is not.** This test is
            // about *reach* — every member that bends must see the model — and
            // the direction was asserted for years only because it happened to
            // hold. It does not in general: the swept maximum is a product of a
            // form factor rising toward the tip and a share falling away there,
            // so a member whose factor rises steeply enough is governed near its
            // tip, at a partial share, above what the unshared convention (full
            // load at the single-pair boundary) assumes. A planetary ring is
            // that member — its `Y_F` runs 2.49 to 0.14 across its flank — and
            // it comes out 2.4 % *higher* with sharing on. That is the sweep
            // doing its job rather than a fault in it, and the unshared
            // convention being the approximation it is documented as.
            assert!(
                (b - a).abs() / a > 1e-9,
                "{what}: sharing must reach a tooth that bends — {a} to {b}"
            );
        }
        assert!(
            rated >= 8,
            "most members should have a bending rating: {off:?}"
        );
    }

    /// **...and below that band it changes nothing, which is not the same as
    /// not reaching them.**
    ///
    /// The ramp's worst point is the largest `Y_F · Y_S · share`, and below
    /// `ε_n = 2` the single-pair boundary is in the sweep with a share of
    /// exactly 1 — so the maximum is the point the unshared rating already
    /// took, and the answer is the one already reported. A hula stage is the
    /// case worth pinning: **its meshes cannot reach the band at any proportion
    /// it can be built at**, running just above continuous contact by
    /// construction, so the control is offered and provably cannot bite there.
    #[test]
    fn below_the_band_the_model_reports_the_tooth_it_was_given() {
        use crate::contact::LoadSharing;
        let lib = library();
        let solve = |sharing| {
            let stage = HulaStage {
                load_sharing: sharing,
                ..HulaStage::default()
            };
            solve_hula_stage(&stage, &StageLoads::just(2.0), &lib).unwrap()
        };
        let off = solve(LoadSharing::None);
        let on = solve(LoadSharing::LinearRamp);
        for (a, b) in off.gears.iter().zip(&on.gears) {
            assert_eq!(
                a.gear.cases[0].bending_stress, b.gear.cases[0].bending_stress,
                "z{}: below the band the model has nothing to find",
                a.teeth
            );
        }
        // ...and the reason, rather than the symptom: the shipped stage's
        // meshes are nowhere near the band. If a hula stage ever is, this fails
        // and the paragraph above needs rewriting.
        for m in &off.meshes {
            assert!(
                m.report.line.unwrap().contact_ratios.transverse < 2.0,
                "a hula mesh above the band would change the claim: {}",
                m.report.line.unwrap().contact_ratios.transverse
            );
        }
    }

    /// **Every search runs on every keystroke**, so each has to cost like an
    /// input and not like a build.
    ///
    /// All three, because the point is the slowest one: the pair's search was
    /// what this was written for, and by the time the epicyclic set had its own
    /// it was twice as dear and ungated. Each stage names its own bound, since
    /// what they do differs — an epicyclic candidate solves a planet and cuts a
    /// ring where a pair's builds two teeth.
    ///
    /// The bounds are loose, but only by about a decade. Wall-clock in a suite
    /// that runs its tests in parallel measures the machine as much as the
    /// code, so a bound near the measurement would fail on a loaded one and
    /// teach a reader to ignore it — while a bound far above it stops catching
    /// anything. They sit at roughly five times what each search costs (8 ms,
    /// 39 ms and 2.6 ms), which is loose enough for a busy machine and tight
    /// enough to catch the kind of regression that has actually happened here:
    /// 800 ms, 100 ms and 68 ms at various points, every time because something
    /// was built per candidate that nothing then read.
    ///
    /// # These went up, and why
    ///
    /// They were 10 / 60 / 20 ms against 0.7 / 10 / 1.8, and the first of those
    /// measurements was of a search doing **a sixth of its own work**:
    /// `auto::Search::budget` was a pool shared across the starts, so the first
    /// walk spent it and the other five never ran. Per walk they all run, which
    /// costs a pair eight times what it was paying and buys it nothing — its
    /// answer is the same at a fifth of the guard — while it is the whole of an
    /// epicyclic set's missing 3.2e-4.
    ///
    /// **A ceiling raised to fit a change needs its reason written down**, so:
    /// the number went up because the work went up, the work went up because it
    /// was being silently skipped, and the multiplier came *down* from ten to
    /// five so the gate did not go slack while the measurement grew. The
    /// optimiser is off by default, so nothing pays this unless it was asked
    /// for.
    #[test]
    fn every_search_is_quick_enough_to_type_over() {
        let lib = library();
        let tuned = Optimisation {
            enabled: true,
            ..Optimisation::default()
        };
        let each = |name: &str, ceiling: u64, f: &dyn Fn()| {
            let start = std::time::Instant::now();
            for _ in 0..5 {
                f();
            }
            let took = start.elapsed() / 5;
            assert!(
                took < std::time::Duration::from_millis(ceiling),
                "the {name} search took {took:?}, over its {ceiling} ms"
            );
        };

        let pair = PairStage {
            optimisation: tuned,
            ..PairStage::default()
        };
        each("pair's", 40, &|| {
            solve_spur_stage(&pair, &StageLoads::just(2.0), &lib).unwrap();
        });

        let mut set = PlanetaryStage {
            optimisation: tuned,
            ..PlanetaryStage::default()
        };
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        each("epicyclic set's", 200, &|| {
            solve_planetary_stage(&set, &StageLoads::just(2.0), &lib).unwrap();
        });

        let drive = HulaStage {
            optimisation: tuned,
            ..HulaStage::default()
        };
        each("hula stage's", 20, &|| {
            solve_hula_stage(&drive, &StageLoads::just(2.0), &lib).unwrap();
        });
    }

    /// **The root round a designer asked for bounds the shift.**
    ///
    /// The tip round a cutter can leave shrinks as the shift rises — it bites
    /// less deep and the space it cuts narrows — so a fillet specified at one
    /// shift becomes unbuildable at a larger one. Without that bound the search
    /// pushes the shift up until some other limit stops it and hands back a
    /// tooth nobody can cut.
    ///
    /// Two things are asked of it: the pair it returns carries the round it was
    /// given, and asking for more round never buys more shift. Where the round
    /// cannot be cut at *any* admissible shift the search finds nothing and the
    /// stage falls back to the shifts it would have had, which the panel's
    /// existing note against the input is what explains.
    #[test]
    fn a_larger_root_round_holds_the_shift_down() {
        let stage = |rho: f64| {
            let gear = |teeth: u32| StageGear {
                teeth,
                root_radius: rho,
                ..PairStage::default().gears[0].clone()
            };
            PairStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                gears: [gear(9), gear(37)],
                ..PairStage::default()
            }
        };
        let mut last = f64::INFINITY;
        let mut fell = false;
        for k in 0..=8 {
            let rho = f64::from(k) * 0.05;
            let s = stage(rho);
            let x = s.shifts();
            let sum = x[0] + x[1];
            // Where it optimised at all, the teeth it chose can be cut.
            let cuttable = (0..2).all(|i| {
                let p = s.params_at(i, x[i]);
                crate::auto::root_radius_fits(&p, p.dedendum)
            });
            if !cuttable {
                // The round is unreachable at every shift; nothing was chosen.
                assert_eq!(
                    x,
                    PairStage {
                        optimisation: Optimisation::default(),
                        ..s
                    }
                    .shifts()
                );
                continue;
            }
            // **To the search's own stopping distance**, not to the bit. The
            // claim is about the *bound* — a larger round admits less shift —
            // and what reads it is a search that stops when its answer has
            // settled to `auto::Search::resolution`. Asserting past that asserts
            // where the walk happened to halt, which is not a fact about
            // gearing.
            assert!(
                sum <= last + crate::auto::Search::SHIPPED.resolution,
                "round {rho} bought shift: sum {sum} against {last}"
            );
            fell = fell || sum < last - 1e-6;
            last = sum;
        }
        assert!(fell, "the round never bound the shift at all");
    }

    /// The efficiency toggle is **additive**: a stage that never asked for it
    /// answers exactly as it did before the toggle existed, and a stage that
    /// does is moved somewhere else.
    #[test]
    fn a_stage_that_did_not_ask_keeps_the_shifts_it_had() {
        let stage = |on: bool| PairStage {
            gears: [
                StageGear {
                    teeth: 17,
                    ..PairStage::default().gears[0].clone()
                },
                StageGear {
                    teeth: 43,
                    ..PairStage::default().gears[1].clone()
                },
            ],
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            ..PairStage::default()
        };
        let plain = stage(false).shifts();
        let tuned = stage(true).shifts();
        assert!(
            (tuned[0] + tuned[1]) - (plain[0] + plain[1]) > 0.05,
            "the toggle should move the pair off its undercut floor, {tuned:?} from {plain:?}"
        );
        // ...and where it moves it, the pair loses less than it did.
        let lib = library();
        let loss = |on: bool| {
            1.0 - solve_spur_stage(&stage(on), &StageLoads::just(2.0), &lib)
                .unwrap()
                .mesh
                .efficiency
                .forward
        };
        assert!(
            loss(true) < loss(false),
            "optimised loss {:.5} should beat the undercut minimum {:.5}",
            loss(true),
            loss(false)
        );
    }

    /// **Whatever a stage chooses, it can be cut.**
    ///
    /// The bound that matters is not any single one but that every stage asks
    /// the same questions. They did not: the root round bounded a pair and not
    /// an epicyclic set, and the hula stage checked its pinion for a
    /// pointed tip but never for the round it was given — so it was returning a
    /// pinion nobody could cut, and a worse answer for it.
    ///
    /// This asks the invariant rather than the wiring, so a stage added later
    /// that forgets `member_is_buildable` fails here rather than shipping.
    #[test]
    fn every_stage_chooses_a_member_that_can_be_cut() {
        use crate::GearParams;
        let cuttable = |p: &GearParams, what: &str| {
            let t = Tooth::new(*p);
            assert!(
                crate::auto::member_is_buildable(
                    &t,
                    Some(crate::auto::automatic_profile_shift(p, p.dedendum))
                ),
                "{what} at x {} cannot be cut: undercut {} severed {} round {}",
                p.profile_shift,
                t.undercut,
                t.severed,
                crate::auto::root_radius_fits(p, p.dedendum)
            );
        };

        let spur = PairStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..PairStage::default()
        };
        for (i, x) in spur.shifts().iter().enumerate() {
            cuttable(&spur.params_at(i, *x), "the pair's gear");
        }

        let mut set = PlanetaryStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..PlanetaryStage::default()
        };
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        let built = set.built(set.shifts()).expect("the set has geometry");
        cuttable(&built.sun.params, "the sun");
        cuttable(&built.planet.params, "the planet");

        // The hula stage's rack-generated members are its pinions; its
        // rings are the shaper's and are not asked.
        let drive = HulaStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..HulaStage::default()
        };
        let r = solve_hula_stage(&drive, &StageLoads::just(2.0), &test_library())
            .expect("the stage solves");
        for (i, g) in r.gears.iter().enumerate() {
            if g.ring {
                continue;
            }
            cuttable(
                &GearParams {
                    module: drive.module[i / 2],
                    pressure_angle: drive.pressure_angle,
                    helix_angle: drive.helix_angle(),
                    teeth: g.teeth,
                    profile_shift: g.gear.profile_shift,
                    addendum: drive.gears[i].addendum,
                    dedendum: drive.gears[i].dedendum,
                    root_radius: drive.gears[i].root_radius,
                    thickness_mod: drive.thickness_mod[i / 2],
                    ..GearParams::default()
                },
                "the stage's pinion",
            );
        }
    }

    /// **The clearance is taken by whatever is free to absorb it.**
    ///
    /// With the distance automatic, the distance absorbs it — it is the
    /// zero-backlash distance opened out, and that opening is the backlash.
    /// With the distance given and the shifts pinned at their undercut minimum
    /// nothing is left to move, so the input goes unread and the answer says so
    /// by reporting zero. With the distance given *and* the shifts being chosen,
    /// the shifts absorb it: the pair closes to zero backlash a clearance inside
    /// the housing, so the designer gets both the distance they specified and
    /// the play they asked for.
    #[test]
    fn the_clearance_is_taken_by_whatever_is_free_to_absorb_it() {
        let lib = library();
        let free = solve_spur_stage(&PairStage::default(), &StageLoads::just(2.0), &lib).unwrap();
        // A housing the pair can actually meet: a clearance inside it is the
        // distance the automatic solve already closes to.
        let asked = free.centre_distance_nominal + 0.05;
        let at = |on: bool| PairStage {
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            clearance: Auto::fixed(0.05),
            ..PairStage::default()
        };

        // **Nothing free to absorb it, so the shifts do not move — and the gap
        // is whatever the housing leaves.** Here that is the 0.05 the fixture
        // built the housing out of, so the number matches the input by
        // arithmetic rather than because the input was read.
        //
        // This used to assert `clearance == 0.0`, on the reading that an unread
        // input should report as nothing. It is a *gap*, and the gap is
        // 0.05: a centre distance is the true distance and a clearance is what
        // portion of it is clearance, so the pair cannot run 0.05 mm wide of its
        // own nominal and report none.
        let pinned = solve_spur_stage(&at(false), &StageLoads::just(2.0), &lib).unwrap();
        assert!(
            (pinned.clearance - (pinned.centre_distance - pinned.centre_distance_nominal)).abs()
                < 1e-12,
            "read {} against a gap of {}",
            pinned.clearance,
            pinned.centre_distance - pinned.centre_distance_nominal
        );
        assert!(
            (pinned.clearance - 0.05).abs() < 1e-9,
            "the housing is 0.05 outside the nominal, so the gap is 0.05, not {}",
            pinned.clearance
        );

        // The shifts free: they take it, and the backlash is the one asked for.
        let chosen = solve_spur_stage(&at(true), &StageLoads::just(2.0), &lib).unwrap();
        assert!((chosen.clearance - 0.05).abs() < 1e-12);
        assert!(
            (chosen.centre_distance - asked).abs() < 1e-9,
            "the housing still holds: {}",
            chosen.centre_distance
        );
        assert!(
            (chosen.centre_distance - chosen.centre_distance_nominal - 0.05).abs() < 1e-6,
            "the pair should close to zero backlash 0.05 inside the housing, not {}",
            chosen.centre_distance - chosen.centre_distance_nominal
        );

        // And with the distance automatic it is read either way, as it always was.
        for on in [false, true] {
            let r = solve_spur_stage(
                &PairStage {
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    clearance: Auto::fixed(0.05),
                    ..PairStage::default()
                },
                &StageLoads::just(2.0),
                &lib,
            )
            .unwrap();
            assert!((r.clearance - 0.05).abs() < 1e-12);
        }
    }

    /// A centre distance the designer typed is a **constraint on the pair**, not
    /// a suggestion the optimiser may overrule: the shifts it chooses still add
    /// up to the housing it was given, and only their split is free.
    #[test]
    fn a_given_centre_distance_still_sets_the_distance() {
        let lib = library();
        let free = solve_spur_stage(&PairStage::default(), &StageLoads::just(2.0), &lib).unwrap();
        let asked = free.centre_distance_nominal + 0.4;
        let stage = PairStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            ..PairStage::default()
        };
        let r = solve_spur_stage(&stage, &StageLoads::just(2.0), &lib).unwrap();
        assert!(
            (r.centre_distance - asked).abs() < 1e-9,
            "asked for {asked}, ran at {}",
            r.centre_distance
        );
    }

    /// **A train that cannot be solved says which stage stopped it.**
    ///
    /// The chain is why it has no answer at all: a stage that fails takes the
    /// shaft line with it, so the stages after it have no speed to be solved at
    /// and the pass that rates every stage against the efficiencies downstream
    /// cannot run. That is a fair reason to return nothing, and no reason at
    /// all to leave a reader hunting for which of five stages is the one to
    /// edit — the number was there at the point of failure and simply was not
    /// kept.
    #[test]
    fn a_train_that_fails_names_the_stage_that_failed() {
        let mut train = two_stage();
        train.stages.push(Stage::Spur(PairStage::default()));
        assert!(
            solve_train(&train, &library()).is_ok(),
            "three good stages solve"
        );

        // A centre distance of zero is not a mesh, and it is the middle stage's.
        let Stage::Spur(s) = &mut train.stages[1] else {
            panic!("the stage this test set up is a spur one")
        };
        s.centre_distance = Auto::fixed(0.0);

        let e = solve_train(&train, &library()).expect_err("a mesh at no distance is not a train");
        let TrainError::InStage { stage, cause } = &e else {
            panic!("the failure should name the stage it happened in, got {e:?}")
        };
        assert_eq!(*stage, 1, "the middle stage is the one that failed");
        // ...and the reason is the stage's own, unchanged by being carried.
        assert!(
            matches!(**cause, TrainError::Mesh(_)),
            "the cause should be the mesh's, got {cause:?}"
        );
        // The note a reader sees is the cause's, so nothing is invented to
        // carry the number.
        use crate::note::Explain;
        assert_eq!(e.note().key, cause.note().key);
        assert!(e.to_string().starts_with("stage 2: "), "{e}");
    }

    /// Both shifts given leaves the optimiser nothing to choose, and it says so
    /// by handing back what it was given rather than by failing.
    ///
    /// **A negative shift somebody meant is not an undercut one.** The bound a
    /// given value is held to is the true minimum, so −0.1 on a 43-tooth wheel
    /// — whose flank is clear down to −1.76 — is a decision about centre
    /// distance and is left exactly where it was put. Only a value that
    /// genuinely undercuts is raised, which the second half asks of a 17-tooth
    /// pinion, clear only above +0.006.
    #[test]
    fn a_fully_specified_pair_is_left_alone() {
        let given = |teeth: u32, x: f64| StageGear {
            teeth,
            profile_shift: Auto::fixed(x),
            ..PairStage::default().gears[0].clone()
        };
        let tuned = |gears: [StageGear; 2]| PairStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            gears,
            ..PairStage::default()
        };
        assert_eq!(
            tuned([given(17, 0.3), given(43, -0.1)]).shifts(),
            [0.3, -0.1]
        );

        // ...and the same pinion asked for a shift its own flank will not carry.
        let raised = tuned([given(17, -0.1), given(43, -0.1)]).shifts();
        assert!(
            raised[0] > -0.1 && raised[0] < 0.01,
            "a pinion below its undercut minimum should be raised to it, not past it: {raised:?}"
        );
        assert!(
            (raised[1] + 0.1).abs() < 1e-12,
            "and the wheel, which undercuts nowhere near here, left alone: {raised:?}"
        );

        // With the constraint off, the number stands however it undercuts.
        let loose = |teeth: u32, x: f64| StageGear {
            no_undercut: false,
            ..given(teeth, x)
        };
        assert_eq!(
            tuned([loose(17, -0.1), loose(43, -0.1)]).shifts(),
            [-0.1, -0.1]
        );
    }

    /// Setting the centre distance by hand takes clearance out of the picture,
    /// which is what the specification requires and what changes the backlash.
    #[test]
    fn a_manual_centre_distance_ignores_the_clearance() {
        let lib = library();
        let auto = solve_spur_stage(&PairStage::default(), &StageLoads::just(2.0), &lib).unwrap();

        // The same distance, set by hand, with a clearance that must be ignored.
        let manual = solve_spur_stage(
            &PairStage {
                centre_distance: Auto::fixed(auto.centre_distance_nominal),
                clearance: Auto::fixed(0.5),
                ..PairStage::default()
            },
            &StageLoads::just(2.0),
            &lib,
        )
        .unwrap();

        assert!((manual.centre_distance - auto.centre_distance_nominal).abs() < 1e-12);
        // At the zero-backlash distance there is, by construction, no backlash.
        assert!(manual.mesh.backlash_by_drive().forward.nominal.abs() < 1e-9);
        // Whereas the automatic one carries its clearance into real backlash.
        assert!(auto.mesh.backlash_by_drive().forward.nominal > 0.0);
    }

    /// An override has to reach the arithmetic, not just the display — and each
    /// load case has to read **its own** allowable.
    ///
    /// Doubling an allowable must quarter the face width contact asks for, since
    /// `b_min ∝ (σ_H/σ_allow)²`. Which allowable does it is the whole point of
    /// separating the cases: the peak case is judged against the ultimate and
    /// the cyclic one against the fatigue figure, so each override moves exactly
    /// one of the two widths and leaves the other alone.
    #[test]
    fn a_material_override_changes_the_answer() {
        let lib = library();
        let off = ByKind {
            ultimate: false,
            fatigue: false,
        };
        let auto_width = |sources: FaceSources, o: Overrides| {
            let mut s = PairStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
                g.material_overrides = o;
            }
            solve_spur_stage(&s, &StageLoads::just(2.0), &lib).unwrap()
        };
        let contact_only = |kind: CaseKind| FaceSources {
            bending: off,
            contact: ByKind::of(|k| k == kind),
        };

        for (case, other) in [
            (CaseKind::Fatigue, CaseKind::Ultimate),
            (CaseKind::Ultimate, CaseKind::Fatigue),
        ] {
            let base = auto_width(contact_only(case), Overrides::default());
            // Twice **this material's** figure, read from the answer rather than
            // written down again: a test that repeats the library's numbers
            // stops testing the arithmetic the moment the library moves.
            let doubled = 2.0 * allowable(&base.gears[0].material, case);
            let over = match case {
                CaseKind::Ultimate => Overrides {
                    ultimate_allowable: Some(doubled),
                    ..Default::default()
                },
                CaseKind::Fatigue => Overrides {
                    fatigue_allowable: Some(doubled),
                    ..Default::default()
                },
            };

            let width = |sources, o| auto_width(sources, o).gears[0].face_width;
            let ratio = base.gears[0].face_width / width(contact_only(case), over);
            assert!(
                (ratio - 4.0).abs() < 1e-9,
                "{case:?}: doubling the allowable should quarter the width: ratio {ratio}"
            );
            // ...and it moved only the case it belongs to.
            let untouched = width(contact_only(other), Overrides::default());
            assert_eq!(untouched, width(contact_only(other), over));
        }

        // ...and the reported material says the number came from the user.
        let doubled = auto_width(
            FaceSources::default(),
            Overrides {
                fatigue_allowable: Some(2.0 * 750.0),
                ..Default::default()
            },
        );
        assert_eq!(
            doubled.gears[0].material.fatigue_allowable.basis,
            crate::material::Basis::Overridden
        );
        assert_eq!(
            auto_width(FaceSources::default(), Overrides::default()).gears[0]
                .material
                .fatigue_allowable
                .basis,
            crate::material::Basis::Estimated
        );
    }

    /// Overriding the modulus moves contact stress, and by the right law.
    #[test]
    fn overriding_the_modulus_moves_contact_stress_as_the_square_root() {
        let lib = library();
        let at = |e: Option<f64>| {
            let mut s = PairStage::default();
            for g in &mut s.gears {
                g.material_overrides = Overrides {
                    elastic_modulus: e,
                    ..Default::default()
                };
            }
            solve_spur_stage(&s, &StageLoads::just(2.0), &lib)
                .unwrap()
                .mesh
                .cases[0]
                .contact
                .at_pitch_point
        };
        let base = at(None);
        let quarter = at(Some(190_000.0 / 4.0));
        assert!(
            (base / quarter - 2.0).abs() < 1e-9,
            "sigma_H goes as sqrt(E*): {base} vs {quarter}"
        );
    }

    #[test]
    fn an_unknown_material_is_named_rather_than_swallowed() {
        let mut s = PairStage::default();
        s.gears[0].material = "unobtainium".into();
        let e = solve_spur_stage(&s, &StageLoads::just(2.0), &library()).unwrap_err();
        assert!(matches!(e, TrainError::UnknownMaterial(ref n) if n == "unobtainium"));
        assert!(e.to_string().contains("unobtainium"));
    }

    #[test]
    fn an_empty_train_says_so() {
        let t = Train {
            load_cases: vec![LoadCase::ultimate(1.0, 1.0)],
            reversed_bending: false,
            stages: vec![],
            couplings: Vec::new(),
            constraints: Vec::new(),
        };
        assert_eq!(solve_train(&t, &library()).unwrap_err(), TrainError::Empty);
    }

    /// **A tooth cycle count is what the browser used to round, and now what the
    /// model returns.**
    ///
    /// The `Math.ceil` in `TrainPanel.svelte` was a modelling decision on the
    /// side of the boundary with no tests, and the CLI printed the *unrounded*
    /// figure beside it — one model with two answers, which is the fault this
    /// crate spends most of its gates on.
    ///
    /// The revolutions are written out here rather than referenced, so the check
    /// is against the shaft line rather than against the code that counts it.
    /// Every count must be whole, and must be the ceiling taken in the right
    /// place: over the whole duty for a one-way drive, and **within one
    /// actuation** for a reversing one, where a tooth part way through an
    /// actuation has still been loaded by that actuation and by every one after.
    #[test]
    fn every_cycle_count_is_the_ceiling_of_the_revolutions_it_replaced() {
        let lib = test_library();
        let with = |speed: f64, duty: Duty| {
            let mut t = two_stage();
            t.load_cases[CYCLIC].speed = speed;
            t.load_cases[CYCLIC].duty = duty;
            t
        };
        for (train, what) in [
            (two_stage(), "two spur stages"),
            (
                with(
                    2400.0,
                    Duty::Continuous {
                        runtime_hours: 1000.0,
                    },
                ),
                "continuous",
            ),
            (
                with(
                    3000.0,
                    Duty::Intermittent {
                        range_degrees: 25.0,
                        at: Port::End,
                        actuations: 1000,
                        reversing: false,
                    },
                ),
                "intermittent",
            ),
            (
                with(
                    3000.0,
                    Duty::Intermittent {
                        range_degrees: 25.0,
                        at: Port::End,
                        actuations: 1000,
                        reversing: true,
                    },
                ),
                "reversing",
            ),
        ] {
            let r = solve_train(&train, &lib).expect(what);
            let case = &train.load_cases[CYCLIC];

            // The revolutions each member turns, before anything rounds them.
            // Revolutions are counted, not signed: `|ratio|` throughout.
            let ratios: Vec<f64> = r.stages.iter().map(|s| s.ratio().abs()).collect();
            for (k, s) in r.stages.iter().enumerate() {
                let upstream: f64 = ratios[..k].iter().product();
                let speed_in = case.speed / upstream;
                let speeds = [speed_in, speed_in / ratios[k]];
                let expected = [0usize, 1].map(|i| {
                    let to_output: f64 = if i == 0 {
                        ratios[k..].iter().product()
                    } else {
                        ratios[k + 1..].iter().product()
                    };
                    match case.duty {
                        Duty::Intermittent {
                            range_degrees,
                            actuations,
                            reversing,
                            ..
                        } => {
                            let each = (range_degrees / 360.0) * to_output;
                            let n = f64::from(actuations);
                            if reversing {
                                let bending = each.ceil() * n;
                                Cycles {
                                    bending,
                                    contact: bending / 2.0,
                                }
                            } else {
                                let n = (each * n).ceil();
                                Cycles {
                                    bending: n,
                                    contact: n,
                                }
                            }
                        }
                        Duty::Continuous { runtime_hours } => {
                            let n = (speeds[i] * 60.0 * runtime_hours).ceil();
                            Cycles {
                                bending: n,
                                contact: n,
                            }
                        }
                    }
                });

                let Some(sp) = s.as_pair() else { continue };
                for (i, (g, want)) in sp.gears.iter().zip(expected).enumerate() {
                    let got = g.cases[CYCLIC].cycles.expect("a fatigue case counts");
                    assert_eq!(got, want, "{what}, stage {k} gear {i}");
                    assert_eq!(
                        got.bending,
                        got.bending.trunc(),
                        "{what}: {} is not a whole count",
                        got.bending
                    );
                }
            }
        }
    }

    /// **An automatic face width has to satisfy the mesh, not one gear.**
    ///
    /// The narrower face carries the pair, so a width sized to its own gear's
    /// requirement satisfies nothing: give one gear a weaker material and it
    /// asks for more face, while the other — sized to its own smaller figure —
    /// pulls the effective width, and the weak gear with it, under what the weak
    /// gear needed.
    ///
    /// Stated as the invariant rather than as the arithmetic: **at an automatic
    /// width, every enabled rating is met**. That is what the control claims to
    /// do, and it is false in both directions if either gear is sized alone.
    #[test]
    fn an_automatic_face_width_satisfies_every_enabled_rating_of_both_gears() {
        let lib = library();
        let weak = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            ..Default::default()
        };
        // A matched pair, then each gear in turn made much weaker than the
        // other, so whichever gear governs the mesh is the one that changes.
        for over in [
            [Overrides::default(), Overrides::default()],
            [weak(250.0), Overrides::default()],
            [Overrides::default(), weak(250.0)],
        ] {
            let mut stage = PairStage::default();
            for (g, o) in stage.gears.iter_mut().zip(over) {
                g.face_width = Auto::automatic(0.0);
                g.material_overrides = o;
            }
            let r = solve_spur_stage(&stage, &StageLoads::just(2.0), &lib).unwrap();
            let effective = r.gears[0].face_width.min(r.gears[1].face_width);
            assert!(effective > 0.0);

            for (i, g) in r.gears.iter().enumerate() {
                let sources = &stage.gears[i].face_sources;
                // Every case, whatever its kind: the width answers to all of them.
                for (case, kind) in g.cases.iter().zip(CaseKind::BOTH) {
                    let asks = case.min_face_width;
                    if *sources.contact.get(kind) {
                        if let Some(c) = asks.contact {
                            assert!(
                                effective >= c * (1.0 - 1e-9),
                                "gear {i} {kind:?} contact needs {c} mm, mesh carries {effective}"
                            );
                        }
                    }
                    if let (true, Some(b)) = (*sources.bending.get(kind), asks.bending) {
                        assert!(
                            effective >= b * (1.0 - 1e-9),
                            "gear {i} {kind:?} bending needs {b} mm, mesh carries {effective}"
                        );
                    }
                }
            }
        }
    }

    /// **The two gears of a mesh are rated at different *points*, not at
    /// different curvatures.**
    ///
    /// The mechanism matters, because the wrong one is very plausible: two teeth
    /// in mesh do have different flank curvatures, so it looks as though they
    /// should carry different stresses. They do not — Hertz reaches the contact
    /// through the *gap*, which depends on the individual radii only as their
    /// sum, and each body is then a half-space under a shared pressure. At one
    /// instant there is one pressure.
    ///
    /// What separates them is *when* each is rated: each gear's dedendum carries
    /// the load alone at one end of the path, and that is where its own pitting
    /// is assessed. So this pins three things — the shared figure is shared and
    /// reaches both materials, the two governing figures differ and each is its
    /// own end of the path, and an allowable moves a width and never a stress.
    #[test]
    fn the_two_gears_of_a_mesh_are_rated_at_different_points() {
        let lib = library();
        // At a **fixed** width: an automatic one is inverted from the stress, so
        // it lands the stress on the allowable and hides the material.
        let solved = |auto: bool, over: [Overrides; 2]| {
            let mut s = PairStage::default();
            for (g, o) in s.gears.iter_mut().zip(over) {
                g.face_width = if auto {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(10.0)
                };
                g.material_overrides = o;
            }
            solve_spur_stage(&s, &StageLoads::just(2.0), &lib).unwrap()
        };
        let modulus = |e: f64| Overrides {
            elastic_modulus: Some(e),
            ..Default::default()
        };

        // --- the shared figure. Softening **either** gear softens the pair, so
        // neither material is being ignored, and `1/E*` is symmetric in the two
        // so it does not matter which was softened.
        let base = solved(false, [Overrides::default(), Overrides::default()]);
        let soft_first = solved(false, [modulus(70_000.0), Overrides::default()]);
        let soft_second = solved(false, [Overrides::default(), modulus(70_000.0)]);
        let pitch = |r: &PairResult| r.mesh.cases[0].contact.at_pitch_point;
        for (r, which) in [(&soft_first, "gear 1"), (&soft_second, "gear 2")] {
            assert!(
                pitch(r) < pitch(&base),
                "softening {which} must soften the pair: {} against {}",
                pitch(r),
                pitch(&base)
            );
        }
        assert!(
            (pitch(&soft_first) - pitch(&soft_second)).abs() < 1e-9,
            "1/E* is symmetric in the two gears"
        );

        // --- the two ratings. On this pair they differ, and each is at least
        // the shared figure — a gear is rated at the worse of the pitch point
        // and its own end of the path, never below it.
        let (a, b) = (
            base.gears[0].cases[0].contact_stress,
            base.gears[1].cases[0].contact_stress,
        );
        assert!(
            a != b,
            "17/43 is not symmetric, so its two gears are not rated alike: {a} and {b}"
        );
        for g in &base.gears {
            assert!(g.cases[0].contact_stress >= pitch(&base));
        }
        // ...and the envelope is the worse of them, which is what the *mesh*
        // would be rated on with no member named.
        assert!((a.max(b) - base.gears[0].cases[0].contact_stress.max(b)).abs() < 1e-12);

        // --- the allowable. At a fixed width again, and for a reason worth
        // stating: with an *automatic* width the allowable does reach the stress,
        // because it moves the width the pair is rated at. Held still, it moves
        // only what it should.
        let allowable = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            ..Default::default()
        };
        let wide = base;
        let half = allowable(0.5 * super::allowable(&wide.gears[1].material, CaseKind::Fatigue));
        let derated = solved(false, [Overrides::default(), half]);
        assert_eq!(
            derated.mesh.cases[0].contact.at_pitch_point, wide.mesh.cases[0].contact.at_pitch_point,
            "an allowable is not a stress and must not move one"
        );
        let contact_width = |r: &PairResult| {
            r.gears[1].cases[1]
                .min_face_width
                .contact
                .expect("a spur member is contact-rated")
        };
        let (was, now) = (contact_width(&wide), contact_width(&derated));
        assert!(
            (now / was - 4.0).abs() < 1e-9,
            "halving the allowable should quadruple the width: {was} to {now}"
        );
        assert_eq!(
            derated.gears[0].cases[1].min_face_width.contact,
            wide.gears[0].cases[1].min_face_width.contact,
            "and it must not reach the other gear"
        );
    }

    /// **A load exists only where it is reacted.**
    ///
    /// A load from the end of a train of ordinary spur stages that nothing
    /// holds reaches no number: every stage can be driven backward, so the
    /// load simply turns the train. Put one self-locking stage in the way and
    /// the load stops there — that stage and everything downstream of it carry
    /// it, and everything upstream still sees nothing. And the same load with
    /// the far end holding it is carried through every stage, attenuated by
    /// each one's efficiency in the direction it travels.
    #[test]
    fn a_back_driving_load_is_carried_only_where_something_reacts_it() {
        let lib = test_library();
        let mut t = two_stage();
        t.load_cases[BACK].torque = 5.0;
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        for s in &r.stages {
            for g in &spur(s).gears {
                assert_eq!(
                    g.cases[BACK].torque, 0.0,
                    "a back-drivable train reacts nothing"
                );
            }
        }
        assert!(r.cases[BACK]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_LOAD_NOT_REACTED)));
        assert_eq!(r.cases[BACK].reacted_at, None);
        assert_eq!(r.cases[BACK].delivered_torque, 0.0);

        // **Held at the far end**, the same load reaches every gear. The
        // torque at each stage's input is the load referred by every ratio
        // and every backward efficiency between it and the end, which is the
        // one walk the forward case takes the other way.
        let mut held = t.clone();
        held.load_cases[BACK].reacted = true;
        let h = solve_train(&held, &lib).unwrap();
        assert!(h.cases[BACK].notes.is_empty());
        let mut at = 5.0;
        for s in h.stages.iter().rev() {
            let p = spur(s);
            // A referral is a magnitude; the ratio carries a sign now.
            let expect = at / p.ratio.abs();
            assert!(
                (p.gears[0].cases[BACK].torque.abs() - expect).abs() < 1e-9 * expect,
                "the load referred to this stage's input is {expect}, not {}",
                p.gears[0].cases[BACK].torque
            );
            at = expect * p.mesh.efficiency.backward;
        }
        assert!((h.cases[BACK].delivered_torque - at).abs() < 1e-12);
        assert_eq!(h.cases[BACK].delivered_at, Port::Start);

        // The same load against a stage that cannot be driven backward. A worm
        // with enough friction locks, and then the load stops there: the worm
        // stage carries it, and the spur stage ahead of it carries none.
        t.stages.push(Stage::Worm(PairStage {
            sliding_friction: 0.3,
            static_friction: 0.3,
            ..PairStage::worm()
        }));
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        let (worm, screw) = worm(&r.stages[2]);
        assert!(
            screw.efficiency.locked().backward,
            "this worm was meant to lock: backward efficiency {}",
            screw.efficiency.backward
        );
        // The wheel is on the shaft the load enters by and carries all of it;
        // the worm is at the far end of a mesh that cannot pass it, so its shaft
        // carries **none** — which is what a locked stage means and is not the
        // same as the case being absent (`a_self_locking_worm_reports_the_load_it_reacts`).
        assert_eq!(
            worm.gears
                .iter()
                .map(|m| m.cases[BACK].torque)
                .collect::<Vec<_>>(),
            vec![0.0, 5.0],
            "the stage that reacts the load carries it, on the member the load \
             is on"
        );
        for s in &r.stages[..2] {
            for g in &spur(s).gears {
                assert_eq!(
                    g.cases[BACK].torque, 0.0,
                    "nothing upstream of a self-locking stage sees the load"
                );
            }
        }

        // ...and the case says which stage held it, rather than leaving the
        // reader to infer it from a column of noughts.
        assert_eq!(r.cases[BACK].reacted_at, Some(2));
        assert!(r.cases[BACK]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_LOAD_REACTED_AT)));

        // **The two torques are two facts.** Put the locking stage first, so
        // the spur stages after it carry the load as well: a gear's torque in
        // the case from the start must stay what it was even when the case
        // from the end is the larger of the two.
        let mut t = two_stage();
        t.load_cases[BACK].torque = 500.0;
        t.stages.insert(
            0,
            Stage::Worm(PairStage {
                sliding_friction: 0.3,
                static_friction: 0.3,
                ..PairStage::worm()
            }),
        );
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        let mut forward_only = t.clone();
        forward_only.load_cases[BACK].torque = 0.0;
        let forward_only = solve_train(&forward_only, &lib).unwrap();
        let mut seen = 0;
        for (loaded, plain) in r.stages[1..].iter().zip(&forward_only.stages[1..]) {
            for (g, unloaded) in spur(loaded).gears.iter().zip(&spur(plain).gears) {
                let back = g.cases[BACK].torque;
                assert!(
                    back > g.cases[PEAK].torque,
                    "this fixture is meant to load it backward"
                );
                assert_eq!(
                    g.cases[PEAK].torque, unloaded.cases[PEAK].torque,
                    "a load from the end must not move the case from the start"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 4, "both spur stages, both gears");
    }

    /// **A case is rated on its own, whatever else the train carries.**
    ///
    /// The law the list rests on: a load case's figures depend on that case
    /// and the shaft line, and on nothing about the other cases — not their
    /// number, not their order, not whether one is switched off. A scale taken
    /// against "the worst torque a mesh carries" is exactly the kind of shared
    /// reference that could make a case move when its neighbour did, and the
    /// epicyclic kinds rate through one (`StageLoads::scaled`); the scale is
    /// exact in the mathematics, and this holds it to the digits the corpus
    /// prints. **Run against a scale read from the wrong case, it fails on
    /// every member**, which is the fault this is for.
    #[test]
    fn a_case_is_rated_on_its_own_whatever_else_the_train_carries() {
        let lib = library();
        let mut train = two_stage();
        train.stages.push(Stage::Planetary(Box::default()));
        train.stages.push(Stage::Hula(Box::default()));
        train.load_cases = vec![
            LoadCase::ultimate(2.0, 3000.0),
            LoadCase {
                port: Port::End,
                reacted: true,
                ..LoadCase::fatigue(0.7, 40.0)
            },
        ];
        let alone = solve_train(&train, &lib).expect("solves");

        // The same two cases, with a heavier one in front of them and a
        // switched-off one after: the first two cases' figures are unmoved.
        let mut crowded = train.clone();
        crowded
            .load_cases
            .insert(0, LoadCase::ultimate(50.0, 100.0));
        crowded.load_cases.push(LoadCase {
            enabled: false,
            ..LoadCase::ultimate(1.0e6, 1.0)
        });
        let with = solve_train(&crowded, &lib).expect("solves");
        assert_eq!(with.cases.len(), 3, "a case switched off reports nothing");
        let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1e-300);
        let mut checked = 0;
        for (a, b) in alone.stages.iter().zip(&with.stages) {
            for (ga, gb) in a.members().iter().zip(b.members()) {
                assert_eq!(gb.cases.len(), 3);
                for (want, got) in ga.cases.iter().zip(&gb.cases[1..]) {
                    checked += 1;
                    assert!(
                        near(want.torque, got.torque)
                            && near(want.contact_stress, got.contact_stress)
                            && want.cycles == got.cycles
                            && match (want.bending_stress, got.bending_stress) {
                                (Some(x), Some(y)) => near(x, y),
                                (a, b) => a == b,
                            },
                        "a case moved when a heavier one was put in front of it: \
                         {want:?} against {got:?}"
                    );
                }
            }
        }
        assert!(checked >= 20, "only {checked} readings compared");
        // ...and every case names its index in the train's list, which is how
        // the front end joins a result to its input.
        assert_eq!(
            with.cases.iter().map(|c| c.case).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
    }

    /// **A stage that cannot be driven forward holds a load from the start**,
    /// by the same walk that lets a self-locking worm hold one from the end.
    ///
    /// A crossed pair at a 9°/81° helix split cannot drive forward at the
    /// shipped static friction. Put first, it is where a load entering at the
    /// start stops: the pair carries it, the stages after it carry none, and
    /// the case says which stage held it. A forward load used to be pushed
    /// through such a stage at its efficiency — nought or less — and reported
    /// downstream as a torque of nothing with no word about why.
    #[test]
    fn a_forward_locked_stage_holds_a_load_from_the_start() {
        let lib = library();
        let mut train = two_stage();
        train.stages.insert(
            0,
            Stage::Worm(
                PairStage {
                    shaft_angle: 90.0,
                    gears: [
                        StageGear {
                            teeth: 17,
                            ..PairStage::worm().gears[0].clone()
                        },
                        StageGear {
                            teeth: 23,
                            ..PairStage::worm().gears[1].clone()
                        },
                    ],
                    ..PairStage::worm()
                }
                .with_first_helix(9.0),
            ),
        );
        let r = solve_train(&train, &lib).expect("solves");
        assert!(
            r.stages[0].efficiency().locked().forward,
            "this fixture is meant to be forward-locked"
        );
        let start = &r.cases[PEAK];
        assert_eq!(start.reacted_at, Some(0));
        assert_eq!(start.delivered_torque, 0.0);
        assert!(start.notes.iter().any(|n| n.is(key::TRAIN_LOAD_REACTED_AT)));
        // The locked pair carries it on the member the load is on...
        assert!(r.stages[0].members()[0].cases[PEAK].torque > 0.0);
        // ...and nothing beyond it sees any.
        for s in &r.stages[1..] {
            for g in s.members() {
                assert_eq!(g.cases[PEAK].torque, 0.0);
            }
        }
    }

    /// **A train with no load case is a shaft line**, and every kind solves it.
    ///
    /// Ratios, efficiencies and backlash stand; every member reports no case;
    /// and an automatic face width, with nothing to ask, stands at its box —
    /// the same answer as no source switched on, since both are a width with
    /// nothing to choose between. A case switched off is the same train.
    #[test]
    fn a_train_with_no_load_case_is_a_shaft_line() {
        let lib = library();
        for cases in [
            vec![],
            vec![LoadCase {
                enabled: false,
                ..LoadCase::ultimate(2.0, 1.0)
            }],
        ] {
            let mut train = two_stage();
            train.stages.push(Stage::Worm(PairStage::worm()));
            train.stages.push(Stage::Planetary(Box::default()));
            train.stages.push(Stage::Hula(Box::default()));
            if let Stage::Spur(s) = &mut train.stages[0] {
                for g in &mut s.gears {
                    g.face_width = Auto::automatic(7.0);
                }
            }
            train.load_cases = cases;
            let r = solve_train(&train, &lib).expect("a shaft line solves");
            assert!(r.cases.is_empty());
            // **The magnitude**, because a train's ratio is signed now: it
            // comes off the graph and says whether the output reverses, where
            // the product of the stage ratios could not — a pair reports its
            // own as a magnitude. This train has an odd number of external
            // meshes, so it turns backwards and says so.
            assert!(r.total_ratio.abs() > 1.0 && r.total_efficiency.forward > 0.0);
            for s in &r.stages {
                for g in s.members() {
                    assert!(g.cases.is_empty());
                }
                for m in s.meshes() {
                    assert!(m.cases.is_empty());
                }
            }
            for g in &spur(&r.stages[0]).gears {
                assert_eq!(g.face_width, 7.0);
            }
        }
    }
}
