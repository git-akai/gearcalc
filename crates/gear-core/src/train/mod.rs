//! Geartrains: one graph, its load cases, and what they come to.
//!
//! **The train is one graph.** A [`Train`] is a [`shape::Shape`] — axes, the
//! bodies on them, members, meshes, distances and couplings — with what it
//! holds and the cases it is loaded by. The graph falls into parts
//! ([`Train::parts`]), which close, search and rate apart and are never the
//! designer's to state; one motion and one flow run across all of them
//! ([`solve_train`]), and a train's figures are a path's ([`PathReport`]).
//! What stays here is the vocabulary every part shares ([`Backlash`],
//! [`TrainError`], the duty cycle, [`GearResult`], [`MeshReport`]), relief
//! over the graph's inputs, and the solve.
//!
//! **A kind is a preset, not a model.** A spur pair, a crossed pair, a worm
//! and a planetary set are lists of what sits where
//! ([`arrangements::pair`], [`arrangements::crossed`], [`arrangements::worm`],
//! [`arrangements::planetary`]), laid into the graph like anything else, and
//! the solve reads what a piece *is* off the shape. A line contact and a point contact are one
//! [`MeshReport`]: the physics is one model with the shaft angle as a
//! parameter — one Hertz answer, one friction balance, one backlash
//! projection, one interference relation, each holding at the limit — so the
//! report is one shape with the numbers moving, and what only one of the two
//! has is the little in [`LineContact`] and [`PointContact`].
//!
//! # What is state and what is not
//!
//! Per `docs/rationale.md#inputs-are-the-only-state` the input structs here are the *only* state. Every
//! result is recomputed from them, so nothing can go stale. Two consequences are
//! visible in the shapes below:
//!
//! - What a mesh group shares — its module and pressure angle — is stated
//!   **once**, on one member, and the rest follow it ([`Shape::share`]); what
//!   a mesh owns — its overlap, its sharing model, its friction — is the
//!   mesh's; what a member owns — its thickness coefficient — is the
//!   member's. Two members stated at different modules are refused
//!   ([`crate::mesh::MeshError::Incompatible`]) rather than averaged
//!   (docs/rationale.md#inputs-are-the-only-state).
//! - A mesh's two thickness coefficients ordinarily sum to 2, which a preset
//!   writes; a pair that does not is a thicker or thinner mesh, carried into
//!   the shift sum as an equivalent shift and not refused.

use crate::auto::{addendum_for_tip_width, Bound, Ranges};
use crate::contact::{Directional, Drive};
use crate::material::{Material, MaterialLibrary, Overrides};
use crate::mesh::MeshError;
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::tooth::Tooth;

pub mod arrangements;
#[cfg(test)]
mod case_laws;
mod conditions;
pub mod crossed;
mod edits;
#[cfg(test)]
mod error_laws;
pub mod flow;
pub mod graph;
mod groupings;
#[cfg(test)]
mod homogeneity;
mod incidence;
mod offers;
mod pair;
mod planetary;
mod preview;
#[cfg(test)]
mod rating_laws;
pub mod shape;
mod structure;
pub mod sweep;
#[cfg(test)]
mod testing;
mod wiring;

pub use conditions::{
    BodyReport, Exact, MotionError, MotionReport, PortBody, Ports, Term, TrainMotion,
};

use crate::kinematics::{Body, Condition, GROUND};
pub use arrangements::{Preset, PresetFamily};
pub use edits::{Edit, EditRefused, Invariant, Piece, Place};
pub use groupings::{AxisBody, AxisGroup, Centre, FlowRow, Groupings};
pub use offers::{Offer, Target};
pub(crate) use pair::ShiftAsked;
pub use preview::{preview, Preview};
pub use shape::{Shape, ShapeResult};
pub(crate) use wiring::teeth_of;
pub use wiring::{BodyLabel, MeshSpec, Mount, Wiring, WiringError};

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
    /// written out once per stage type they were three copies of the same two
    /// lines, and a fourth type away from being four. The width is the mesh's
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
    /// The band's least and greatest play; `None` where an end of the
    /// tolerance has no play to read — inside the base circles' limit, where
    /// the teeth have no operating angle — and that end might have set it.
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
}

/// **One source's play at its tolerance's ends**: at the minus end, the
/// running point and the plus end. An end inside the base circles' limit
/// has none (`None`); the running point always has one.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ends {
    pub minus: Option<f64>,
    pub running: f64,
    pub plus: Option<f64>,
}

impl Ends {
    /// Each end mapped.
    #[must_use]
    pub fn map(self, f: impl Fn(f64) -> f64) -> Self {
        Self {
            minus: self.minus.map(&f),
            running: f(self.running),
            plus: self.plus.map(&f),
        }
    }
}

impl Backlash {
    /// **One source's band**, from its play at the minus end, the running
    /// point and the plus end: the extremes of the three.
    ///
    /// **Which end is which is read off the numbers.** A larger centre
    /// distance is more play on an external pair and less on an internal
    /// one, so the minus end is the minimum on one kind and the maximum on
    /// the other; a set carrying one of each, referred to one body, is
    /// neither, and the running point is a candidate for either extreme.
    #[must_use]
    pub fn of_ends(ends: Ends) -> Self {
        Self::of_sources([ends])
    }

    /// **Independent sources stacked**, each given at its minus end, its
    /// running point and its plus end: the nominal is the sum of the
    /// running points, and each extreme the sum of every source's own —
    /// sources that do not move together reach their worst case at once. A
    /// source with an end that has no play leaves both extremes absent: that
    /// end might have set either.
    #[must_use]
    pub fn of_sources(sources: impl IntoIterator<Item = Ends>) -> Self {
        sources.into_iter().fold(
            Self {
                nominal: 0.0,
                minimum: Some(0.0),
                maximum: Some(0.0),
            },
            |b, e| {
                let ends = e.minus.zip(e.plus);
                Self {
                    nominal: b.nominal + e.running,
                    minimum: b
                        .minimum
                        .zip(ends)
                        .map(|(m, (lo, hi))| m + lo.min(e.running).min(hi)),
                    maximum: b
                        .maximum
                        .zip(ends)
                        .map(|(m, (lo, hi))| m + lo.max(e.running).max(hi)),
                }
            },
        )
    }
}

/// **What one mesh reports**, whatever part it is in and whichever way its
/// bodies run.
///
/// One type, because the physics is one model with the shaft angle as a
/// parameter and not two models: the Hertz answer is general contact of which a
/// line is the degenerate value ([`crate::hertz::peak_pressure`]), the
/// efficiency is one friction balance the parallel loss integral is the limit
/// of, the backlash is one gap projected onto one normal, and the interference
/// verdict is one relation asked in the transverse plane or along the line —
/// each of which a test holds at the limit. So a designer turning a body
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
    /// (its one mesh being the pair), and a set with two meshes has two
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
    /// point's limit as the bodies straighten is the normal-plane `ε_α / cos²β_b`
    /// rather than the total. The decomposition each has is in [`Self::line`]
    /// and [`Self::point`].
    pub contact_ratio: f64,
    /// Mesh efficiency, both drive senses. Equal for a parallel-axis mesh — the
    /// mirror flank is the same integral — and genuinely different on crossed
    /// bodies, where **either** can be zero or negative: backward is what
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
    /// **Not sent**: the harness's and the core's; the panel reads a crossed
    /// pair's sliding as a speed per case.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub sliding_ratio: f64,
    /// **What every load case does to this mesh**: the Hertzian contact — one
    /// patch the two members share, an ellipse on crossed shafts and a line on
    /// parallel ones — and the sliding speed at that case's speed.
    pub cases: Vec<MeshCase>,
    /// Angular backlash at each member, degrees, in the order the mesh was
    /// built: the pinion-side member first, then the other.
    pub backlash: [Backlash; 2],
    /// **The mesh's play as its row in the kinematics sees it**, radians of
    /// the row — `j |Σz| / a` on a line contact, plus the axial float's — at
    /// its distance's minus, running and plus tolerance in turn. What a
    /// play referred to any body of the train is read from
    /// ([`crate::kinematics::System::play`]), so a path's backlash sums every mesh
    /// it crosses and none it does not ([`PathReport`]). **Not sent**: the
    /// path's, read in the core.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub row_play: Ends,
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
    /// **What this mesh has to say**, on every mesh alike: contact that does not
    /// stay continuous, a helical pair without full axial overlap, a sharing
    /// model that is extrapolating, a screw pair that locks or nearly does,
    /// or one that loses more than it keeps. A set has two meshes and says
    /// which, which a note on the set could not.
    pub notes: Vec<Note>,
    /// What a line contact has and a point does not.
    pub line: Option<LineContact>,
    /// What a point contact has and a line does not.
    pub point: Option<PointContact>,
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
    /// The one patch the two members share in this case. **Not sent**: each
    /// member's own contact stress is on its card, which is this or worse,
    /// and the patch is the harness's and the core's to read.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub contact: ContactPatch,
    /// **The power crossing this mesh, over the power the case puts in** —
    /// the train's flow's own figure ([`flow::Flow::mesh_powers`]): one on a
    /// pair's mesh, under one where a carrier carries part of it bodily,
    /// many times one where power circulates. A case's, since which power
    /// crosses a mesh is a question about where the load goes in, and a
    /// case is what says.
    pub power_through: f64,
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
    /// `None` where the ideal contact runs off the faces
    /// (`mesh.contact_off_face`): no zone on them has a travel.
    pub axial_travel: Option<[f64; 2]>,
}

/// **The Hertzian contact a mesh presses** — one answer for an ellipse and a
/// line, since a line is the ellipse with one curvature at zero
/// ([`crate::hertz::peak_pressure`]).
#[derive(Clone, Copy, Debug)]
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
    /// zero is what crossing the bodies did.
    pub curvature_along: f64,
    /// ...and across it, `1/ρ` — the reciprocal of the relative radius of
    /// curvature at the worst point, in the normal plane.
    pub curvature_across: f64,
    /// The patch's aspect ratio `κ = b/a` there: 0 for a line.
    pub aspect: f64,
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
            // A line contact is the ellipse at no curvature along.
            aspect: crate::hertz::LINE_ASPECT,
        }
    }
}

/// **What a line contact's builder has in hand**, gathered so every caller
/// that reports one fills the report through one function rather than
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
    /// The power through the mesh per load case, in the loads' order.
    pub case_power: Vec<f64>,
    pub backlash: [Backlash; 2],
    pub row_play: Ends,
    pub flank_interference: [bool; 2],
    pub tips: Option<TipRoom>,
    /// What the builder has to say that this function cannot read off the
    /// ratios — the sharing model's finding, raised where the section is.
    pub notes: Vec<Note>,
}

/// A line contact's [`MeshReport`]: the shared fields as given, the sliding at
/// the pitch point and the locking thresholds at their degenerate values, and
/// the transverse figures in [`LineContact`].
pub(crate) fn line_mesh_report(cases: &[CaseLoad], m: LineMesh) -> MeshReport {
    // The two findings every line contact's ratios can raise, asked here so
    // that no caller has to remember to — the pair stage asked both and
    // neither epicyclic stage asked either, while there were such stages.
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
        cases: cases
            .iter()
            .zip(m.contact)
            .zip(m.case_power)
            .map(|((l, contact), power_through)| MeshCase {
                case: l.case,
                contact,
                power_through,
                sliding_velocity: 0.0,
            })
            .collect(),
        backlash: m.backlash,
        row_play: m.row_play,
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
/// whether the teeth foul or not according to which stage type they had picked.
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
    /// **pinion** rotation, degrees. Negative is the overlap, and `None` where
    /// the tip circles do not cross at all — the ordinary case, where there is
    /// no place for the tips to meet. **Not sent**: the panel reads whether
    /// the tips clear ([`Self::tip_interference`]), and the harness the
    /// margin.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub tip_margin: Option<f64>,
    /// **The far-side gap**, mm: the room between the pinion's tip and the
    /// ring's on the side away from contact, `r_tip,ring − r_tip,pinion +
    /// a`. At a few teeth of difference it is what the distance is sized to
    /// ([`shape::Distance::tip_clearance`]); on an ordinary internal mesh it
    /// is a large number nobody reads.
    pub far_gap: f64,
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
            tip_margin: m.tip_margin.map(f64::to_degrees),
            far_gap: m.far_gap,
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
    /// a *path* reads it per direction, because the output of a forward drive
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
/// member's result has carried it. Undercut short of severing alters
/// nothing — the tooth is exactly the one the inputs describe — which is why it
/// is not a clamp, and why nothing was reporting it: a gear tab has shown it
/// since undercut existed, and the same gear inside a geartrain said nothing at
/// all. So it goes where a remark about a member goes, beside the notch band
/// and the reversed root.
///
/// It matters most exactly where the solve cannot prevent it. `no undercut` bounds
/// a shift somebody chooses; a shift that a *relation* leaves over answers to no
/// bound at all — a hula pinion whose ring was pinned, an epicyclic absorber —
/// so the control can be on, the tooth undercut, and the two never meet.
///
/// Nothing for a ring: its reading of undercut, a flank short of its tip, is
/// the ring's own `clamp.ring_flank_ungenerated`.
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

/// **What one member bends at in one mesh**: the critical section the model
/// rates, the share of the mesh load acting on it, and the rim under it.
///
/// All are per *(member, mesh)*, which is why they travel together and why
/// this is one place rather than one per stage type. Absent where no load
/// point leaves a section the model rates ([`crate::strength::RootSection::bending_factor`]).
pub(crate) struct Bending {
    pub section: crate::strength::RootSection,
    /// Exactly 1 where no sharing model was asked for, so the ordinary rating
    /// is untouched to the bit.
    pub share: f64,
    /// **`Y_B`** — the one factor of `σ_F0` that is the member's *blank's*
    /// rather than its root section's. `None` where nobody described a rim,
    /// which is every gear this crate rated before it existed.
    pub rim: Option<crate::strength::RimSupport>,
}

impl Bending {
    /// **For any member that bends**, rack-cut or shaper-cut.
    ///
    /// `rim` is the thickness of the rim under its teeth, mm, or `None` where
    /// nobody said — see [`crate::strength::RimSupport`]. Which reference that
    /// thickness is measured against is the member's own business
    /// ([`crate::strength::ToothOutline::rim_support`]), as the direction its load point travels
    /// is; this was two functions and they differed in nothing else.
    /// `short_of_tip` is how far this member's last contact falls below its
    /// tip, in base pitches ([`crate::contact::ContactPath::short_of_tip`]).
    pub(crate) fn of<T: crate::strength::ToothOutline>(
        member: &T,
        contact_ratio: f64,
        short_of_tip: f64,
        model: crate::contact::LoadSharing,
        rim: Option<f64>,
    ) -> Result<Self, crate::strength::Unrated> {
        let (section, share) =
            crate::strength::bending_section_on_path(member, contact_ratio, short_of_tip, model)?;
        Ok(Self {
            section,
            share,
            rim: rim.map(|s| member.rim_support(s)),
        })
    }
}

/// **A mesh past the single-pair band**, `ε_αn ≥ 2`: two pairs are always
/// engaged, so there is no single-pair zone.
///
/// Both ways of rating bending lean on that zone. Unshared, the tooth is
/// loaded alone at the highest point of single-pair contact, `d = ε_n − 1`
/// base pitches from the tip, which here is a point where it never carries
/// the whole load, low on the flank where Dolan–Broghamer's factor shrinks
/// toward zero and then has no reading. Shared, the ramp never reaches a full
/// share; measured across high-contact-ratio spur designs it runs from a
/// 25–32 % relief on high-contact-ratio spur pairs, a direction the model does
/// not fix (T06.7 re-measures it). ISO corrects the
/// band with `Y_DT`, which this crate declines. **A mesh's finding**, raised
/// once per mesh whatever the sharing model.
pub(crate) fn single_pair_band(contact_ratio: f64, base_helix: f64) -> Option<Note> {
    let cos_bb = base_helix.cos();
    let eps_n = contact_ratio / (cos_bb * cos_bb);
    (eps_n >= 2.0).then(|| Note::new(key::MESH_LOAD_SHARING_OUT_OF_BAND).number("ratio", eps_n, 3))
}

/// **What a member's contact ratings have to say**: that its contact stress
/// stands above its allowable in some case — the worst such, numbered from
/// one — and which kinds of case it has no contact allowable for.
///
/// Said whether or not contact sizes the face: with contact sizing off, which
/// is the default, a width sized by bending can leave the flank far past its
/// endurance, and the designer is owed knowing.
pub(crate) fn contact_notes(rated: &[Rated]) -> Vec<Note> {
    let mut out = Vec::new();
    let worst = rated
        .iter()
        .filter_map(|r| r.contact_worst.map(|(s, a)| (r.case, s, a)))
        .filter(|(_, s, a)| s > a)
        .max_by(|x, y| (x.1 / x.2).total_cmp(&(y.1 / y.2)));
    if let Some((case, stress, allowable)) = worst {
        out.push(
            Note::new(key::GEAR_CONTACT_ABOVE_ALLOWABLE)
                .number("stress", stress, 1)
                .number("allowable", allowable, 1)
                .text("case", (case + 1).to_string()),
        );
    }
    for (kind, k) in [
        (CaseKind::Fatigue, key::GEAR_CONTACT_FATIGUE_UNJUDGED),
        (CaseKind::Ultimate, key::GEAR_CONTACT_ULTIMATE_UNJUDGED),
    ] {
        if rated.iter().any(|r| r.kind == kind && r.contact_unjudged) {
            out.push(Note::new(k));
        }
    }
    out
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
/// A **member's** finding rather than a mesh's, which is why it is raised per
/// member and not with the mesh's bending: two members of one mesh have two rims and one sharing
/// model between them.
pub(crate) fn rim_below_minimum(rim: Option<crate::strength::RimSupport>) -> Option<Note> {
    rim.filter(|r| !r.in_range())
        .map(|r| Note::new(key::GEAR_RIM_BELOW_MINIMUM).number("ratio", r.ratio(), 2))
}

/// **What one mesh does to one member**: the two stresses it produces there,
/// and the widths they belong to.
///
/// A member is not always in one mesh. A planet is in two, and an arrangement
/// nobody has laid out yet may put a member in more — so a rating is taken over
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
    /// [`Self::measured_at`], under the torque the part is rated at. `None`
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
    /// settled and none is needed ([`MemberRating::asks`]); and for a rating
    /// that evaluated its stresses at the width it ended with, where there is
    /// nothing to scale and the figures are the ones its own arithmetic
    /// produced, bit for bit.
    pub carried_at: f64,
    /// **Whether the contact figure can be inverted for a width.** A line
    /// contact's pressure falls with the face it is spread over, so a stress
    /// says what width would bring it to the allowable; a point contact's
    /// does not depend on the width at all, so it says nothing about one
    /// — what sizes a crossed pair's face is continuity, or a worm's
    /// proportions, reported as their own figure rather than smuggled in
    /// here. Such a loading is taken at its own width, case by case, and
    /// neither scales nor asks.
    pub sizes_face: bool,
    /// The contact patch's aspect ratio `κ = b/a`: 0 for a line, which every
    /// parallel mesh is. What first yield below the flank depends on
    /// ([`Rating::Contact`]).
    pub aspect: f64,
}

impl Loading {
    /// The two stresses as they stand at [`Self::carried_at`].
    ///
    /// Bending is inversely linear in width and contact goes as the inverse
    /// square root of it, so a change of width is a scale rather than a second
    /// solve — and where the two widths are equal this is the identity, which
    /// is what lets a rating that evaluated at its final width keep its own
    /// digits to the bit.
    fn at_width(self) -> (Option<f64>, f64) {
        let by = self.measured_at / self.carried_at;
        (self.bending.map(|s| s * by), self.contact * by.sqrt())
    }

    /// **The same loading under `k` times the torque.**
    ///
    /// Bending is linear in torque and contact goes as its square root
    /// (docs/reference.md#load-cases), so where a part's power split does not
    /// depend on the *magnitude* of what passes through it — which the shape's
    /// flow guarantees, being linear in the torque through it, and is a fact
    /// about the flow rather than about gearing — a second load case is this
    /// rather than a second solve. A flow that
    /// does not have that property builds each case's loadings itself, which is
    /// why they are held per case rather than as one list and a factor.
    pub(crate) fn under(self, k: f64) -> Self {
        Self {
            bending: self.bending.map(|s| s * k),
            contact: self.contact * k.sqrt(),
            ..self
        }
    }
}

/// What every mesh a member is in does to it under one load case.
pub(crate) struct CaseLoadings {
    /// The case's index, kind, and whether its duty reverses — what the
    /// rating reads of a case.
    pub case: usize,
    pub kind: CaseKind,
    pub reverses: bool,
    pub meshes: Vec<Loading>,
}

/// **What one member's ratings come to**, over every mesh it is in.
///
/// The rating asks the same questions of every member it builds — two
/// stresses in every load case, and the width each of those would need — and
/// each of the stage types that preceded it had been writing the arithmetic
/// out for itself. What genuinely differs between members is what the meshes
/// do to the member and
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
    /// this one times a number" is a claim about a *power flow* rather
    /// than about gearing, and a flow that did not scale with what passes
    /// through it would build each case for itself.
    pub cases: Vec<CaseLoadings>,
}

/// The rating fields of one [`GearCase`], over every mesh a member is in.
pub(crate) struct Rated {
    pub case: usize,
    pub kind: CaseKind,
    pub bending_stress: Option<f64>,
    pub contact_stress: f64,
    pub min_face_width: Widths,
    /// The mesh whose contact stress stands highest over its allowable at
    /// the width it is carried at: `(stress, allowable)`, MPa. `None` where
    /// no mesh's contact can be judged.
    pub contact_worst: Option<(f64, f64)>,
    /// Whether this case's kind has no contact allowable for the material.
    pub contact_unjudged: bool,
}

impl Rated {
    /// The case's readout, once the part has said what else it knows of the
    /// member in it: its torque at its own radius, its speed in the fixed
    /// frame and against its carrier, and how often its teeth are engaged
    /// over the case's duty — which for a held ring is not nought while its
    /// speed is.
    pub(crate) fn into_case(
        self,
        torque: f64,
        on_body: f64,
        speeds: (f64, f64),
        cycles: Option<Cycles>,
    ) -> GearCase {
        GearCase {
            case: self.case,
            torque,
            on_body,
            speed: speeds.0,
            speed_against_carrier: speeds.1,
            cycles,
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
                // The worst contact among the meshes whose figure a width
                // can be read off, which is what the width is inverted from.
                let mut sizable: Option<f64> = None;
                let mut width = 0.0_f64;
                // The contact figure standing highest over its own
                // allowable: first yield depends on each patch's shape.
                let mut worst: Option<(f64, f64)> = None;
                for l in &c.meshes {
                    let (b, s) = l.at_width();
                    let judged =
                        allowable(self.material, Rating::Contact { aspect: l.aspect }, c.kind);
                    if let Some(a) = judged {
                        if worst.is_none_or(|(ws, wa)| s / a > ws / wa) {
                            worst = Some((s, a));
                        }
                    }
                    if let Some(b) = b {
                        bending = Some(bending.map_or(b, |had: f64| had.max(b)));
                    }
                    if s >= contact {
                        contact = s;
                    }
                    if l.sizes_face {
                        sizable = Some(sizable.map_or(s, |had: f64| had.max(s)));
                        width = width.max(l.carried_at);
                    }
                }
                let reverses = self.reversal.reverses(self.always_reverses, c.reverses);
                // A line contact is what a width is inverted from.
                let flank = allowable(self.material, Rating::Contact { aspect: 0.0 }, c.kind);
                Rated {
                    case: c.case,
                    kind: c.kind,
                    bending_stress: bending,
                    contact_stress: contact,
                    contact_worst: worst,
                    contact_unjudged: flank.is_none() && !c.meshes.is_empty(),
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
                                self.reversal
                                    .bending_allowable(self.material, c.kind, reverses),
                            )
                        }),
                        contact: sizable.zip(flank).map(|(contact, flank)| {
                            crate::strength::min_face_width_contact(contact, width, flank)
                        }),
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
    /// (docs/reference.md#contact-stress). So a rating can size a member from a
    /// probe pass, before it has a width to size it at.
    pub(crate) fn asks(&self) -> Vec<(CaseKind, Widths)> {
        self.rated()
            .into_iter()
            .map(|r| (r.kind, r.min_face_width))
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
    /// Which bound cut it down, if one did — and so whether the tooth is the
    /// one that was asked for.
    pub held_by: Option<AddendumBound>,
}

/// **What an addendum can be held to**: the two ends of a tip's room.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AddendumBound {
    /// The tip no narrower than [`MemberGear::min_tip_width`].
    TipWidth,
    /// The tip reaching no further than a mate's usable flank
    /// ([`MemberGear::no_tip_past_mate_flank`]).
    MateFlank,
}

impl AddendumAsked {
    /// The note a held addendum owes its reader, naming what held it.
    pub(crate) fn note(&self) -> Option<crate::note::Note> {
        self.held_by.map(|bound| {
            crate::note::Note::new(match bound {
                AddendumBound::TipWidth => crate::note::key::GEAR_ADDENDUM_HELD_TO_TIP_WIDTH,
                AddendumBound::MateFlank => crate::note::key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK,
            })
            .number("addendum", self.used, 4)
        })
    }
}

// -------------------------------------------------- the shared member ---
//
// `MemberGear` is what every member of the train is described with, so it lives
// here with the rest of the shared vocabulary rather than in the preset that
// happened to need it first. It was declared in `spur.rs` and re-exported from
// this module, which read as though the parallel-axis stage owned it — and this
// module's own comment says it holds "what every stage shares".

/// **One gear of the train**: what is its own to state.
///
/// Note what is *absent*: its module, pressure angle and tooth thickness
/// coefficient are its [`shape::Member`]'s, which the gears a run of meshes
/// joins share ([`Shape::share`]), and which body it is fixed to is the
/// member's too.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MemberGear {
    pub teeth: u32,
    /// The shift, and who decides it: **automatic means the solve does**, not
    /// that undercut does. What it resolves to when nothing else constrains it
    /// is [`MemberGear::no_undercut`]'s business.
    pub profile_shift: Auto<f64>,
    /// **The shift may not go below the least that clears undercut.**
    ///
    /// A constraint rather than a source, which is what lets it combine with
    /// everything else: it bounds a shift a designer typed, a shift the solve
    /// reached from a centre distance or a crank offset, and a shift the
    /// efficiency search chose, all in the same words.
    ///
    /// The bound is the **true** minimum from [`crate::auto::minimum_profile_shift`], which
    /// on a comfortable tooth count is negative — so a deliberate negative
    /// shift is left alone and only a genuinely undercut one is raised. That is
    /// deliberate: negative shift is a decision about centre distance or
    /// balance, and this is a question about undercut. Where the *solve* is
    /// choosing and nothing else decides, the answer is instead
    /// [`crate::auto::automatic_profile_shift`] — the same bound taken no lower than zero,
    /// because a shift chosen for no reason should not thin a tooth that needed
    /// no help.
    ///
    /// Off, the gear may undercut, and the searches stop asking
    /// ([`crate::auto::member_is_buildable`]).
    ///
    /// **On a ring it asks the ring's reading**: its flank generated all the
    /// way to its tip ([`crate::ring::minimum_profile_shift`]), the edge its
    /// shaper leaves where a rack leaves undercut.
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
    pub no_sharp_tip: bool,
    /// Minimum transverse tooth tip width, mm.
    pub min_tip_width: f64,
    /// **The tip may not reach past a mate's usable flank.**
    ///
    /// The same shape as [`Self::no_sharp_tip`]: an upper bound on the
    /// addendum, which a typed number answers to. The bound is the mesh's to
    /// compute — where this tip meets the start of its mate's usable flank
    /// along the line of action, at the distance the mesh runs at — and the
    /// mesh hands it to the gear as a number of modules, so the gear never
    /// reads its mate. A member in several meshes is held to the lowest.
    ///
    /// It is what keeps a full-depth ring off its planet's form circle
    /// (`crate::ring::mesh_with`): the bound meets it exactly, rather than a
    /// fixed shorter addendum that is right for one pair of counts.
    ///
    /// **On for every gear**, a ring and an external gear alike. On an
    /// internal pair at full depth it is
    /// what keeps the ring off its planet's form circle; on an external pair
    /// it tops a tip a search or a designer has pushed past its mate's
    /// flank, where without it the search held the shifts off that wall — at
    /// 9/37 the least loss moves from Σx 1.41 to 1.64, `ε` to 1.20.
    ///
    /// Off, the addendum stands as asked and a tip that reaches too far is
    /// said on the mesh (`mesh.flank_interference`).
    pub no_tip_past_mate_flank: bool,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Helix angle, degrees, signed by hand — and who decides it.
    ///
    /// **Automatic means the solve does**, through whatever relates this
    /// member's helix to the rest of it: a pair's two are bound by
    /// `β₁ + β₂ = Σ` and the first member's by its pitch diameter, a set's
    /// three by the hands its two meshes require, a hula's four by its
    /// two internal meshes. So at most one member of a part states a helix
    /// and the others follow — or none does, and the part's own relation
    /// decides: a given centre distance with both shifts pinned sizes a pair's
    /// first member, and a given axial contact ratio with every face width
    /// given sizes the helix any kind needs to reach it
    /// ([`Shape::freedoms`] says which may stand). Where nothing decides it,
    /// the first member's stands at its box.
    pub helix_angle: Auto<f64>,
    /// Automatic takes the larger of the enabled minimums below, and the
    /// width a given axial contact ratio needs where its mesh has one.
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
    /// the pressure between two flanks, so no contact rating reads it. The
    /// panel has no field for it yet and carries it back unread, on purpose
    /// (docs/state.md#worth-doing-next). Left out of a file where `None`.
    pub rim_thickness: Option<f64>,
    /// Name of a material in the library.
    pub material: String,
    /// Properties replaced for this gear only. Empty means "as the library
    /// says" — see [`Overrides`].
    pub material_overrides: Overrides,
}

impl MemberGear {
    /// **An automatic width with nothing to size it**, said on the gear.
    ///
    /// Every source switched off leaves nothing to invert, so the width stands
    /// at the number in its box ([`FaceSources::width_for`]) — said rather than
    /// divided by, which is what a zero width was. Every kind used to raise it
    /// on the part, naming the gear; it is the gear's, drawn under its own
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
    ///
    /// `cutter` is the tool of a ring, whose tooth is thinnest at its tip
    /// because it narrows inward ([`crate::ring::addendum_for_tip_width`]):
    /// the same bound, read on the tooth that tool cuts.
    ///
    /// `mate` is the addendum at which this tip meets the nearest mate's
    /// usable flank, as the meshes resolved it
    /// ([`Self::no_tip_past_mate_flank`]); `None` where no mesh bounds it.
    pub(crate) fn addendum_asked(
        &self,
        at_shift: &crate::params::GearParams,
        cutter: Option<&crate::ring::Cutter>,
        mate: Option<f64>,
    ) -> AddendumAsked {
        let asked = self.addendum;
        let tip_width = self
            .no_sharp_tip
            .then(|| {
                let at = GearParams {
                    addendum: asked,
                    ..*at_shift
                };
                match cutter {
                    None => addendum_for_tip_width(&Tooth::new(at), self.min_tip_width),
                    Some(c) => crate::ring::addendum_for_tip_width(
                        &crate::ring::Ring::cut_by(&at, c),
                        self.min_tip_width,
                    ),
                }
            })
            .flatten();
        let used = tip_width.map_or(asked, |c| asked.min(c));
        let mut out = AddendumAsked {
            used,
            held_by: (used < asked - 1e-12).then_some(AddendumBound::TipWidth),
        };
        if let Some(c) = mate.filter(|&c| self.no_tip_past_mate_flank && c < out.used) {
            out = AddendumAsked {
                used: c,
                held_by: Some(AddendumBound::MateFlank),
            };
        }
        out
    }

    /// [`ShiftAsked`], for this gear at its own working depth — or, for a
    /// ring, cut by `cutter`, whose edge of undercut is where its flank is
    /// generated to its tip ([`crate::ring::minimum_profile_shift`]).
    pub(crate) fn shift_asked(
        &self,
        base: &crate::params::GearParams,
        cutter: Option<&crate::ring::Cutter>,
    ) -> ShiftAsked {
        let depth = self.working_depth.resolve(self.dedendum);
        if !self.no_undercut {
            // Nothing asked of the shift. Given, it is taken as typed; left to
            // the solve with no objective either, there is no reason to move
            // the tooth at all.
            let given = (!self.profile_shift.auto).then_some(self.profile_shift.manual);
            return ShiftAsked {
                search_floor: None,
                given,
                settled: given.unwrap_or(0.0), // absence: a shift nothing asks for leaves the tooth unshifted, at zero
                raised: false,
            };
        }
        let x_min = match cutter {
            None => crate::auto::minimum_profile_shift(base, depth).with_cutter_radius,
            Some(c) => crate::ring::minimum_profile_shift(base, c),
        };
        if self.profile_shift.auto {
            // `automatic_profile_shift`'s rule, for either tool: at the edge,
            // and not moved where the edge asks nothing.
            let settled = x_min.map_or(0.0, |x| x.max(0.0)); // absence: with no edge of undercut no shift changes the flank, and the tooth is cut unshifted
                                                             // A search is held at the edge — and, on an external tooth, no
                                                             // lower than zero, since a negative shift thins it. A negative
                                                             // shift thickens a ring's tooth, so a ring's search is held at
                                                             // its edge alone.
            let search_floor = if cutter.is_some() {
                x_min
            } else {
                Some(settled)
            };
            return ShiftAsked {
                search_floor,
                given: None,
                settled,
                raised: false,
            };
        }
        // Given, and held to the true minimum rather than to the search's — a
        // negative shift somebody meant is not an undercut one. Nothing is
        // choosing it, so it carries no search bound.
        let typed = self.profile_shift.manual;
        let used = x_min.map_or(typed, |x_min| typed.max(x_min));
        ShiftAsked {
            search_floor: None,
            given: Some(used),
            settled: used,
            raised: used > typed,
        }
    }
}

impl Default for MemberGear {
    fn default() -> Self {
        Self {
            teeth: 17,
            profile_shift: Auto::automatic(0.0),
            no_undercut: true,
            working_depth: Auto::automatic(1.0),
            addendum: 1.0,
            no_sharp_tip: true,
            min_tip_width: 0.1,
            no_tip_past_mate_flank: true,
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
    /// **What this gear's meshes put on its body**, N·m, signed as the
    /// train's body torques are: the gears on one shaft sum to the shaft's
    /// own load or reaction, so where a shaft carries two gears this is what
    /// it hands from the one to the other — which the case's figure for the
    /// body, being the external torque, does not say. Whichever parts the
    /// body lies between.
    pub on_body: f64,
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

/// What the solve does to one gear.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct GearResult {
    /// **The tooth as built** — the parameters the solve cut this member
    /// with, every automatic value resolved and every convention applied: the
    /// shift in force, the addendum the tip allows, the helix with the hand
    /// this member has, a planet's `2 − k`. What the gear tab receives when it
    /// adopts a member ([`Shape::member_cutter`] says whether it is a ring),
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
    /// **Per gear rather than per part**, and that is not filing. A part's note
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

/// The facts the rating has about one member, gathered so that assembling
/// a [`GearResult`] is one expression rather than one per stage type.
///
/// # Why this exists
///
/// Three stage types built a `GearResult` field by field, listing the same seventeen
/// names each time. Sixteen agreed. The seventeenth did not: a member's share
/// of a load from the far end was the mesh projection of the backward torque in
/// the spur stage and this gear's forward torque scaled by `|t| / |forward|` in
/// the other two — the same number wherever the projection is linear, which it
/// is, **except in sign**. A stage with a negative ratio gave a signed figure
/// from one type and a magnitude from another, for the field beside the forward
/// torque, which is signed.
///
/// That is the fault `docs/corrections.md` opens with: a duplicated formula is a
/// place where two answers can differ, and nothing compared these two.
pub(crate) struct MemberFacts<'a> {
    /// The shift in force — which for a hula gear is the layout's, not the
    /// parameters', so it is given rather than read.
    pub profile_shift: f64,
    pub params: &'a GearParams,
    pub input: &'a MemberGear,
    /// Every load case's readout, from [`Rated::into_case`].
    ///
    /// **The torque in each is given by the flow rather than derived here**,
    /// and that is the finding. Three stage types referred a back-driving load by
    /// scaling this member's *forward* torque, which is exact wherever the
    /// forward torque is a geometric projection or the two directional
    /// efficiencies agree — true of every parallel-axis mesh. A worm stage is
    /// neither: its wheel's forward torque carries a forward efficiency of 62 %
    /// that a backward load does not share, and its backward efficiency is
    /// zero. So the flow projects each case's torque through the meshes in that
    /// case's direction, and this holds no formula a caller could need to
    /// disagree with.
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
    /// One member's result, from what the rating knows about it.
    ///
    /// Every member comes through here, so a field cannot be filled two ways
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
                    || n.is(key::GEAR_ADDENDUM_HELD_TO_TIP_WIDTH)
            })
    }
}

/// Why a train, or a part of it, could not be solved.
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
    /// and for most counts none can. It
    /// is a statement about the *assembly*, not about contact — the teeth are
    /// fine, and the kinematics is unaffected, which is why a train reports its
    /// ratios through this.
    NoCommonDistance,
    /// **The part's members and meshes do not describe a mechanism.**
    ///
    /// The one refusal here a *design* can reach is a member with no teeth —
    /// `MemberGear::teeth` is a `u32` and nothing stops a designer typing zero.
    /// It used to be reported as *"the tooth is too undercut to have a root
    /// section"*, which describes a tooth that exists, and the check now runs
    /// **before** any geometry so the answer is about what is wrong.
    Wiring(WiringError),

    /// A material name that is not in the library.
    UnknownMaterial(String),
    /// **A tip reaches so far past its mate's usable flank that nothing can
    /// be rated.** The path of contact is cut to the usable flanks
    /// ([`crate::contact::ContactPath::new`]); here the cut leaves no path, or
    /// ends it at a base circle where, below a contact ratio of one, the
    /// whole load sits on a radius of curvature of zero. The teeth do touch —
    /// they collide — so this is not [`Self::NoContact`].
    FlankInterference,
    /// **Two conditions cannot both hold**, at this body: what it is asked
    /// to do contradicts what the meshes and the other conditions already
    /// decided — a sun driven while its carrier and its ring are both held.
    Overdetermined { at: usize },
    /// A constraint or a load case names a body the train does not have.
    NoSuchBody { at: usize },
    /// A train with nothing in it, asked for what it does.
    Empty,
    /// The tooth counts along the shaft line multiply past what an exact
    /// ratio can hold, and a ratio is refused rather than wrapped.
    Overflow,
    /// **A load case enters by a body no load can be put on**: ground, a
    /// held body, a body the train does not have, or one of a part's
    /// bodies that is not a port — a planet's. Zero-based, as the cases are
    /// indexed; the front end numbers from 1.
    LoadPort { case: usize },
    /// **A fatigue case's sweep measured at a body that is not an open
    /// port**: ground, a held body, a planet's, or one the train does not
    /// have. Zero-based, as the cases are indexed; the front end numbers
    /// from 1.
    DutyPort { case: usize, body: usize },
    /// **Two entries of one case at one body**: two speeds given there
    /// would be summed and of two torques one would be lost, so the case
    /// says two things of one body and is refused rather than read.
    DuplicateEntry { case: usize, body: usize },
    /// **No distance clears the tips of this mesh**: an automatic distance
    /// with an internal mesh on it opened out through the involute domain
    /// and the tips never came clear by what was asked
    /// ([`shape::Distance::tip_clearance`]). Zero-based, as the meshes are
    /// indexed; the front end numbers from 1.
    TipsUnclearable { mesh: usize },
    /// **Two axes on one carrier cannot stand where their distances put
    /// them**: the distance between them is longer than their two
    /// distances from the carrier's axis together, or shorter than their
    /// difference ([`staggers`](incidence::Indexed::staggers)) — `too_close` says which.
    /// Zero-based, as the distances are indexed; the front end numbers
    /// from 1.
    AxesCannotBePlaced { distance: usize, too_close: bool },
    /// **A loop of distances on one carrier does not close**: no placement
    /// of the axes about the carrier's axis stands at every distance in it
    /// (`Indexed::frames_close`). The loop's last-stated distance,
    /// zero-based; the front end numbers from 1.
    AxesLoopOpen { distance: usize },
    /// **Two given distances ask one mesh group two sizes**: each with both
    /// its mesh's shifts pinned decides the group's helix, and they decide
    /// it differently ([`resolved_helices`](incidence::Indexed::resolved_helices)). The mesh that
    /// sized the group first and the one that asked another, zero-based;
    /// the front end numbers from 1.
    SizeOverConstrained { meshes: [usize; 2] },
    /// **The graph is not well formed**, at the invariant named: input
    /// that describes no train, refused where it enters
    /// ([`Train::validate`]) before anything reads it.
    Malformed(Invariant),
    /// **A number that describes nothing**, refused where it enters
    /// ([`crate::input`]): not finite, outside its field's bound, or an
    /// index naming nothing in its list — named by its path.
    Input(crate::input::Refused),
    /// **Which part could not be solved**, wrapped around why.
    ///
    /// A part that fails takes the flow with it: every part the train's one
    /// flow crosses is rated against the efficiencies the others give, and
    /// with one missing the flow cannot run at all. So the whole train has
    /// no answer, and the least a reader is owed is which of its parts is
    /// the one to look at.
    InPart {
        /// Zero-based, as the parts are dealt ([`Train::parts`]); the front
        /// end names a part by its meshes.
        part: usize,
        cause: Box<TrainError>,
    },
}

/// So a train's motion can refuse in the vocabulary a train refuses in.
///
/// A motion failure is never geometric — that is the whole point of the split —
/// so the only things it can carry are a part whose wiring is not a mechanism
/// and a train with nothing in it.
impl From<MotionError> for TrainError {
    fn from(e: MotionError) -> Self {
        match e {
            // A train with nothing in it says so, under its own key.
            MotionError::Empty => Self::Empty,
            MotionError::Wiring(part, cause) => Self::InPart {
                part,
                cause: Box::new(Self::Wiring(cause)),
            },
            MotionError::NoSuchBody(at) => Self::NoSuchBody { at },
            MotionError::Conflicts(at) => Self::Overdetermined { at },
            MotionError::Overflow => Self::Overflow,
        }
    }
}

/// So the wiring can refuse in the vocabulary the solve refuses in.
impl From<WiringError> for TrainError {
    fn from(e: WiringError) -> Self {
        Self::Wiring(e)
    }
}

/// **A zero-based index as the front end numbers it**, from 1 — an axis,
/// a mesh or a distance a malformed graph names.
/// A note about one body, carrying its number as the train counts it —
/// ground being 0.
fn located(key: &'static str, at: usize) -> Note {
    Note::new(key).text("body", at.to_string())
}

impl crate::note::Explain for TrainError {
    /// Why the train could not be solved, as a key and its values.
    ///
    /// The nested cases delegate rather than restating: a mesh that will not
    /// mesh says so once, wherever it is asked.
    fn note(&self) -> crate::note::Note {
        use crate::note::{key, Note};
        match self {
            Self::Mesh(e) => e.note(),
            // The drive diagnoses itself; the train carries the note rather
            // than restating it.
            Self::TipsUnclearable { mesh } => {
                Note::new(key::ERROR_TRAIN_TIPS_UNCLEARABLE).ordinal("mesh", *mesh)
            }
            Self::Screw(e) => e.note(),
            Self::NoContact => Note::new(key::ERROR_TRAIN_NO_CONTACT),
            Self::NoCommonDistance => Note::new(key::ERROR_TRAIN_NO_COMMON_DISTANCE),
            Self::Wiring(WiringError::MemberWithoutTeeth(i)) => {
                Note::new(key::ERROR_TRAIN_WIRING_NO_TEETH).ordinal("member", *i)
            }
            Self::Wiring(WiringError::NoCommonFrame(k)) => {
                Note::new(key::ERROR_TRAIN_WIRING_NO_COMMON_FRAME).ordinal("mesh", *k)
            }
            Self::Wiring(WiringError::NotAMesh(k)) => {
                Note::new(key::ERROR_TRAIN_WIRING_NOT_A_MESH).ordinal("mesh", *k)
            }
            Self::Wiring(WiringError::NotACoupling(c)) => {
                Note::new(key::ERROR_TRAIN_WIRING_NOT_A_COUPLING).ordinal("coupling", *c)
            }
            Self::Empty => Note::new(key::ERROR_TRAIN_EMPTY),
            Self::UnknownMaterial(n) => {
                Note::new(key::ERROR_TRAIN_UNKNOWN_MATERIAL).text("name", n.clone())
            }
            Self::FlankInterference => Note::new(key::ERROR_TRAIN_FLANK_INTERFERENCE),
            Self::Overdetermined { at } => located(key::ERROR_TRAIN_OVERDETERMINED, *at),
            Self::NoSuchBody { at } => located(key::ERROR_TRAIN_NO_SUCH_BODY, *at),
            Self::Overflow => Note::new(key::ERROR_TRAIN_OVERFLOW),
            Self::LoadPort { case } => {
                Note::new(key::ERROR_TRAIN_LOAD_PORT).text("case", (case + 1).to_string())
            }
            Self::DutyPort { case, body } => {
                located(key::ERROR_TRAIN_DUTY_PORT, *body).text("case", (case + 1).to_string())
            }
            Self::DuplicateEntry { case, body } => located(key::ERROR_TRAIN_DUPLICATE_ENTRY, *body)
                .text("case", (case + 1).to_string()),
            Self::SizeOverConstrained { meshes: [a, b] } => {
                Note::new(key::ERROR_TRAIN_SIZE_OVER_CONSTRAINED)
                    .ordinal("first", *a)
                    .ordinal("second", *b)
            }
            Self::AxesCannotBePlaced {
                distance,
                too_close,
            } => Note::new(if *too_close {
                key::ERROR_TRAIN_AXES_TOO_CLOSE
            } else {
                key::ERROR_TRAIN_AXES_TOO_FAR
            })
            .ordinal("distance", *distance),
            Self::AxesLoopOpen { distance } => {
                Note::new(key::ERROR_TRAIN_AXES_LOOP_OPEN).ordinal("distance", *distance)
            }
            // Each field a refusal can name has its own key; the axis is
            // numbered from one, as the panel numbers axes.
            Self::Malformed(Invariant::CarriedByNothing(axis)) => {
                Note::new(key::ERROR_TRAIN_MALFORMED_CARRIED_BY).ordinal("axis", *axis)
            }
            Self::Malformed(Invariant::CarriedInACycle(axis)) => {
                Note::new(key::ERROR_TRAIN_MALFORMED_CARRIED_BY_CYCLE).ordinal("axis", *axis)
            }
            Self::Malformed(Invariant::DistanceTwice(d)) => {
                Note::new(key::ERROR_TRAIN_MALFORMED_DISTANCE_TWICE).ordinal("distance", *d)
            }
            Self::Malformed(Invariant::RingFirst(k)) => {
                Note::new(key::ERROR_TRAIN_MALFORMED_RING_FIRST).ordinal("mesh", *k)
            }
            Self::Malformed(Invariant::DistanceOffFrame(d)) => {
                Note::new(key::ERROR_TRAIN_MALFORMED_DISTANCE_OFF_FRAME).ordinal("distance", *d)
            }
            // A body is numbered as the train numbers it, ground 0.
            Self::Malformed(Invariant::NumberGap(body)) => {
                located(key::ERROR_TRAIN_MALFORMED_NUMBER_GAP, *body)
            }
            Self::Malformed(_) => Note::new(key::ERROR_TRAIN_MALFORMED),
            Self::Input(e) => e.note(),
            // Which part it is belongs to the reader rather than to the
            // reason, so the note is the cause's and the part reaches the
            // front end through the error's own shape.
            Self::InPart { part, cause } => cause.note().ordinal("part", *part),
        }
    }
}

impl std::fmt::Display for TrainError {
    /// The note — its key and values, no words ([`crate::note::Explain`]).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", crate::note::Explain::note(self))
    }
}

impl std::error::Error for TrainError {}

/// A self-contained material library for tests, so `gear-core` keeps no
/// dependency on `gear-io`. Shared by every preset's tests.
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
        fatigue_load_ratio: Some(crate::material::LoadRatio::Reversed),
        fatigue_specimen: Some(crate::material::Specimen::Coupon),
        contact_fatigue_allowable: None,
        contact_estimate: Some(crate::material::HardnessEstimate {
            line: crate::material::HardnessLine::ThroughHardenedAlloySteel,
            hardness: Value {
                value: 458.0,
                basis: Basis::Derived,
                note: Some("test".into()),
            },
            grade: crate::material::QualityGrade::Mq,
        }),
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
        contact_fatigue_allowable: None,
        contact_estimate: None,
        ..steel.clone()
    };
    MaterialLibrary {
        materials: vec![steel, bronze],
    }
}

/// **The transverse contact ratio the optimiser may not take a mesh
/// below**, where the mesh's own field (`MeshInput::min_contact_ratio`) is
/// not given: 1.2, the usual design minimum.
///
/// Sliding loss falls monotonically with the length of the path, so the
/// least-loss pair is always the one whose teeth barely reach: this is the
/// constraint that answers rather than the optimum, which is why it is an
/// input and not a constant. A mesh of one tooth of difference sits just
/// above continuous contact at every shift it can be built at, and asks
/// for less. It bounds the *optimiser* only: a design specified by hand is
/// reported as it is, with the existing note below 1.
pub const DEFAULT_MIN_CONTACT_RATIO: f64 = 1.2;

/// **Whether a shape's shift chooser actually chose**, and what it means when
/// the shifts come back where they started.
///
/// A designer who turns *optimise for efficiency* on and sees no shift move is
/// owed the reason, and there are two of them that look identical:
///
/// - the search ran and **agreed** — the optimum is on the floor the gears
///   already sit at, which is the ordinary answer wherever loss falls
///   monotonically toward the shortest admissible path; or
/// - the search ran and found **nothing admissible at all**, so there was no
///   answer to choose and the shape kept what it had.
///
/// The first is the tool working. The second is a design with no room in it, and
/// it was silent everywhere — the audit's record (`docs/history/audit.md`,
/// F58) measured both ends of it on one
/// set before this existed to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Searched {
    /// The optimiser was off, or every freedom was given by hand. Nothing was
    /// asked, so nothing failing to move says anything.
    NotAsked,
    /// It ran and chose. The answer may still equal the floor; that is the
    /// search agreeing rather than the search failing.
    Chose,
    /// It ran and **nothing in the admissible range could be built**. These
    /// shifts are the ones the shape takes unasked.
    FoundNothing,
}

impl Searched {
    /// The note this deserves, if any — so no caller has to remember the wording
    /// or which of the three states is worth saying out loud.
    pub(crate) fn note(self) -> Option<Note> {
        (self == Self::FoundNothing).then(|| Note::new(key::PART_OPTIMISER_FOUND_NOTHING))
    }
}

/// **What a part has to say about the distance it ended up running at**, if
/// anything.
///
/// Two findings, and both were silent. They are here rather than in each stage
/// type's own file, as they once would have been, because every distance can
/// reach them, and a rule one stage type asked was a rule the others forgot — which `docs/corrections.md`
/// records happening to the tip-room flags and to the axial-overlap warning.
///
/// # The distance was given and the shifts could not reach it
///
/// Mode 3 says a given distance and a given clearance decide the shifts. Where
/// no admissible shifts reach that distance the part still answers — at the
/// shifts it would have built anyway — and used to say nothing at all, so a
/// designer read a pair that was **not** running where they put it and had only
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
    // own direction — and `None` where the distance was not in mode 3 at all,
    // which is the case this cannot arise in. `clearance` is the gap the mesh
    // actually left, signed the way the mesh reads it: positive is play on
    // either kind, so a negative one is teeth overlapping at rest whichever way
    // the centres moved to get there.
    if let Some(target) = target {
        if (nominal - target).abs() > REACHED {
            out.push(
                Note::new(key::PART_DISTANCE_NOT_REACHED)
                    .number("asked", target, 4)
                    .number("reached", nominal, 4),
            );
        }
    }
    if clearance < 0.0 {
        out.push(
            Note::new(key::PART_CLEARANCE_NEGATIVE)
                .number("overlap", -clearance, 4)
                .number("nominal", nominal, 4),
        );
    }
    out
}

/// One of a shape's constrainable inputs, named so a caller can find it.
///
/// Two levels, because a shape has inputs of two kinds: its own — a distance,
/// a clearance, a ratio, a worm's diameter — and one of each per member.
/// `Member(i, _)` indexes the members in the order [`ShapeResult::members`]
/// reports them — a pair's two gears, a set's sun, planet and ring, a
/// hula's four — and is resolved once for every member (`Shape::inputs`), so
/// a member input that arrives costs one line there and none per preset.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Freedom {
    /// The distance a pair of the shape's axes run at — its axis distance —
    /// by the distance's index in the shape: a pair has one, a Ravigneaux
    /// three.
    Distance(usize),
    /// What portion of that distance is running play.
    Clearance(usize),
    /// **A mesh's axial contact ratio**, by the mesh's index in the shape,
    /// which relates the helix to the face width the mesh carries: given
    /// with every width of its mesh group given, it decides the group's
    /// helix; given with a width automatic, it is a floor under that
    /// width. The panel offers it once per mesh group and writes every
    /// mesh of the group.
    Overlap(usize),
    /// One member's own input.
    Member(usize, MemberFreedom),
}

/// One of a member's constrainable inputs.
///
/// A helix here is one reading of **the size of the teeth along the
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
    /// The pitch diameter — the same freedom as the helix read as a size,
    /// which is a worm's reading of it; any member may state it, and the
    /// panel offers the box on a worm.
    PitchDiameter,
    /// The face width.
    FaceWidth,
    /// The tooth-thickness coefficient `k` — one per member, with the two
    /// members of a mesh bound by the mesh's rule (they sum to 2 across an
    /// external mesh, a ring takes its pinion's), so at most one of a
    /// mesh's two is given and the other follows.
    ThicknessMod,
    /// The normal module — one per mesh group, stated on one member and
    /// followed by the rest ([`shape::Member::module`]).
    Module,
    /// The normal pressure angle, by the module's rule.
    PressureAngle,
}

impl MemberFreedom {
    /// What this input of a gear came to, off its result.
    #[must_use]
    pub fn of(self, g: &GearResult) -> f64 {
        match self {
            Self::Shift => g.profile_shift,
            Self::Helix => g.helix_angle,
            Self::PitchDiameter => g.pitch_diameter,
            Self::FaceWidth => g.face_width,
            Self::ThicknessMod => g.params.thickness_mod,
            Self::Module => g.params.module,
            Self::PressureAngle => g.params.pressure_angle,
        }
    }
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

/// **One way of stating a mesh group's helix**, and what it states.
///
/// The helix is one number a mesh group's members share — `β₂ = Σ − β₁` on a
/// pair, the hand flipped across an external mesh in a set, one angle on a
/// hula's four — so a shape has several inputs that all say it: each member's
/// helix, a pair's first pitch diameter (`d = z m_n / cos β`), and the axial
/// contact ratio where every face is given and the ratio can be turned into a
/// helix at the width the mesh carries. Those are *readings* of one freedom,
/// and at most one of them stands.
///
/// A kind lists its readings in **relief order, least precious first**, and
/// the solve honours the **last** one given ([`helix_angles`](incidence::Indexed::helix_angles)). So the reading
/// relief leaves standing is the reading the solve reads, by construction —
/// there is no second list stating the precedence again in an `if` chain, and
/// nothing for the two to disagree about.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Reading {
    pub freedom: Freedom,
    /// The first member's helix, degrees, as this reading states it. `None`
    /// where the input is automatic — or given and reaching nothing, which a
    /// ratio no helix reaches at the given width is, and the part says so
    /// ([`overlap_notes`]).
    pub helix: Option<f64>,
}

impl Reading {
    /// A reading from a member's helix box, `to_first` turning what it states
    /// into the first member's angle.
    pub(crate) fn helix(i: usize, gear: &MemberGear, to_first: impl FnOnce(f64) -> f64) -> Self {
        Self {
            freedom: Freedom::Member(i, MemberFreedom::Helix),
            helix: (!gear.helix_angle.auto).then(|| to_first(gear.helix_angle.manual)),
        }
    }

    /// The ratio's reading on mesh `k`, where it is given the size to
    /// decide.
    pub(crate) fn overlap(k: usize, overlap: &Auto<f64>, module: f64, width: f64) -> Self {
        Self {
            freedom: Freedom::Overlap(k),
            helix: (!overlap.auto)
                .then(|| helix_for_overlap(overlap.manual, module, width))
                .flatten(),
        }
    }
}

/// **The face width an axial contact ratio needs**, mm — `ε_β π m_n / sin |β|`
/// — where the ratio is given and the helix can carry one. `None` where the
/// ratio is automatic or the teeth are straight, which no width makes helical.
///
/// One home for the relation every line contact asks, read as a
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
        out.push(Note::new(key::PART_OVERLAP_UNREACHABLE).number("ratio", overlap.manual, 3));
    }
    if !taken && helix_deg == 0.0 {
        out.push(Note::new(key::PART_OVERLAP_NEEDS_HELIX).number("ratio", overlap.manual, 3));
    }
    out
}

/// **The helix an axial contact ratio needs**, degrees, at a face width the
/// mesh carries — the same relation read the other way, for a mesh whose
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
/// # Why the *shape* declares this rather than the front end
///
/// It was three functions in TypeScript, one per stage type, each restating a
/// relation the core already enforces. That is two faults at once: an
/// engineering rule written outside Rust, and the same idea written once per
/// type — so a new arrangement arrived with no relief at all, and a rule that
/// changed changed in one of four places. It is also untestable there, and was untested.
///
/// The arrangements genuinely differ in *what* is related, which is why this
/// is a declaration and not a constant: a pair relates its distance to its two
/// shifts, an epicyclic set relates its three shifts to each other through the
/// two distances that must agree, and a hula relates each mesh's pair
/// separately because its crank fixes their difference one mesh at a time.
///
/// What does **not** differ is the resolution: too many given means the first
/// one in `order` that the designer is not this moment touching goes back to
/// automatic. Least precious first, and stated by the shape.
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// A shape with **no** distance input would say `0` here — its clearance
    /// could never be derived, because there is nothing to derive it from. That
    /// is the same statement, counted; every distance has the input, so none
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

/// **Two ways of saying one number, so one of them must be said.** A distance
/// is nominal + clearance and an automatic clearance is distance − nominal;
/// with both automatic neither has anything to derive from. The group of
/// distance `d`, one per distance the shape has.
pub(crate) fn distance_and_clearance(d: usize) -> FreedomGroup {
    FreedomGroup {
        given_at_most: 2,
        automatic_at_most: 1,
        order: vec![vec![Freedom::Clearance(d)], vec![Freedom::Distance(d)]],
    }
}

/// **An input that is never read**: relief turns it automatic whatever was touched, so a box the solve would
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

impl shape::Shape {
    /// **The input a [`Freedom`] names**, where this shape has it. `None` is the
    /// same thing [`Self::freedoms`] says by not mentioning it.
    pub(crate) fn input_mut(&mut self, f: Freedom) -> Option<&mut Auto<f64>> {
        self.inputs()
            .into_iter()
            .find_map(|(g, a)| (g == f).then_some(a))
    }

    /// Whether the input a freedom names is given. An input this shape does
    /// not have is neither given nor automatic.
    fn is_given(&mut self, f: Freedom) -> bool {
        self.input_mut(f).is_some_and(|a| !a.auto)
    }

    /// **Every input relief may turn, and whether it is automatic** — in the
    /// one order the shape declares, so a caller lining two shapes up against
    /// each other can do it by name rather than by field.
    #[must_use]
    pub fn toggles(&self) -> Vec<(Freedom, bool)> {
        // Read through a copy: the shape hands its inputs out mutably, once,
        // and a second accessor for reading would be the same list twice.
        let mut probe = self.clone();
        probe
            .inputs()
            .into_iter()
            .map(|(f, a)| (f, a.auto))
            .collect()
    }

    /// **This shape's kinematic system**, from its wiring and its tooth counts.
    ///
    /// It needs neither a module nor a shift nor a material, which is the whole
    /// point: a shape that will not close geometrically still has a ratio, and
    /// this is what can still be asked of it.
    ///
    /// # Errors
    ///
    /// [`WiringError`] for a wiring that does not describe meshes.
    pub fn system(&self) -> Result<crate::kinematics::System, WiringError> {
        self.wiring().alone(&teeth_of(self.gears()))
    }

    /// **The pinion cutter a member is cut by, where it is a ring** — which is
    /// the one question *is this member internal?* has, and the member says
    /// it of itself. `None` for every rack-cut member, including a worm.
    #[must_use]
    pub fn member_cutter(&self, i: usize) -> Option<&crate::ring::Cutter> {
        self.members.get(i).and_then(|m| m.ring.as_ref())
    }

    /// **This shape with its over-determined inputs relieved.**
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
    /// It was three functions in TypeScript, one per stage type. Rule 1 puts an
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
        let mut out = self.shared();
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
        // What moved may be which member of a group states its module:
        // the rest follow the one that stands now.
        out.share();
        out
    }

    /// **This shape relieved, with every box relief turned given seeded from
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
        // answer depends on the order the *shape* declares and never on the
        // order the toggles happened to be turned in.
        for wanted_auto in [false, true] {
            let limit = if wanted_auto {
                group.given_at_most
            } else {
                group.automatic_at_most
            };
            // An entry is given while any reading is, automatic while all
            // are; one the shape has no input for is neither.
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
            // offered, computed and judged against a flank's own figure —
            // first yield below the surface, and a pitting endurance
            // ([`allowable`]) — and a member above either says so. What they
            // are not is *assumed*: contact genuinely governs, asking 6.7 mm
            // on the strength canary where bending asks 0.9, and a default
            // that decides the answer is a default making the design
            // decision. So both are left to the designer and bending is what
            // a fresh gear is sized from (docs/state.md).
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
    /// Nothing enabled used to come out **zero**, and the rating then divided by
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
    /// stands at its box — and so does one whose every case carries nothing,
    /// a case the train could not solve being a load of nought on every
    /// mesh, which asks a width of nought and means no width was asked.
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
        if want == 0.0 {
            given
        } else {
            want
        }
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
/// train rather than about any one part: whether the designer asked for the
/// correction at all. Which members *are* reversed is a fact about each member
/// in each load case, and arrives beside the load ([`CaseLoad::reverses`]).
///
/// # Why the correction is asked for rather than applied
///
/// A root loaded on both flanks endures less than one loaded on a single flank.
/// **Where the fatigue figure was measured fully reversed** — every figure the
/// shipped library carries a ratio for — it is already the reversed case, and a
/// reversed root is judged at it as it stands. Where it was measured one way,
/// or does not say, the allowance is ISO's fraction on the *allowable*
/// ([`REVERSED_BENDING_FRACTION`](crate::material::REVERSED_BENDING_FRACTION),
/// [`Material::reversed_bending_fraction`]). That fraction is a convention: it
/// multiplies a stress a part is sized against, which is exactly what
/// `docs/rationale.md` refuses to apply on a designer's behalf. So it is a
/// switch, off by default, and where it is off the member **says** that
/// reversal is present and uncorrected rather than leaving the reader to
/// notice.
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
    /// One home, so a part cannot report the correction on one member and stay
    /// silent about it on another — and so the sentence is the same whichever
    /// member raises it. Asked once per member, of whether *any* case
    /// reverses it: the note is about the root, and the figures beside it say
    /// which cases it is judged in.
    ///
    /// Nothing, where the figure was measured fully reversed: nothing is
    /// uncorrected and nothing is applied. Where the figure does not say how it
    /// was measured, the note says it was read one way
    /// ([`Material::reversed_bending_fraction`]).
    #[must_use]
    pub fn note_for(self, reverses: bool, m: &Material) -> Vec<Note> {
        let (fraction, stated) = m.reversed_bending_fraction();
        if !reverses || fraction == 1.0 {
            return Vec::new();
        }
        let mut out = vec![if self.correct {
            Note::new(key::GEAR_REVERSED_BENDING_APPLIED).number("fraction", fraction, 2)
        } else {
            Note::new(key::GEAR_REVERSED_BENDING_UNCORRECTED)
        }];
        if !stated {
            out.push(Note::new(key::GEAR_FATIGUE_RATIO_UNSTATED));
        }
        out
    }

    /// The **bending** allowable a member is judged against, MPa.
    ///
    /// Only a fatigue case can be reduced: an ultimate load is survived once
    /// and has no reversal to endure. And only bending — pitting is compressive
    /// whichever flank carries it, so a contact rating keeps the material's own
    /// figure. A figure measured fully reversed is judged as it stands, bit for
    /// bit.
    #[must_use]
    pub fn bending_allowable(self, m: &Material, kind: CaseKind, reverses: bool) -> f64 {
        let (fraction, _) = m.reversed_bending_fraction();
        let figure = root_allowable(m, kind);
        if self.correct && reverses && kind == CaseKind::Fatigue && fraction != 1.0 {
            figure * fraction
        } else {
            figure
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
/// for either kind, and a kind is otherwise a vocabulary.
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

    /// **The torque, N·m, and the speed, rpm, a fresh case of this kind
    /// starts at** — a new train's cases and every case the panel adds,
    /// stated once ([`LoadCase::fresh`]). The panel's starting example, for
    /// the designer to overwrite, and nothing any gear is sized by until a
    /// case is written and switched on.
    #[must_use]
    pub const fn fresh_figures(self) -> (f64, f64) {
        match self {
            Self::Ultimate => (FRESH_STALL_NM, FRESH_SPEED_RPM),
            Self::Fatigue => (FRESH_RUNNING_NM, FRESH_SPEED_RPM),
        }
    }
}

/// A fresh ultimate case's torque, N·m: a small motor's stall, the peak an
/// ultimate case is survived at.
const FRESH_STALL_NM: f64 = 0.1;
/// A fresh fatigue case's torque, N·m: a fifth of the stall — a running
/// load rather than the peak, so a fresh train's two ratings answer
/// different questions.
const FRESH_RUNNING_NM: f64 = FRESH_STALL_NM / 5.0;
/// The speed a fresh case turns at, rpm: the same small motor's.
const FRESH_SPEED_RPM: f64 = 30_000.0;

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

/// **What is being judged**: a root in bending, or a flank in contact, and a
/// flank by the shape of its patch.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Rating {
    /// A root's tensile bending stress.
    Bending,
    /// A flank's Hertzian pressure, on a patch of aspect ratio `κ = b/a`:
    /// 0 for a line contact, 1 for a circle.
    Contact { aspect: f64 },
}

/// **The allowable a rating is judged against in a load case's kind**, MPa,
/// or `None` where the material publishes nothing it could be.
///
/// | | ultimate | fatigue |
/// |---|---|---|
/// | bending | `ultimate_allowable` | `fatigue_allowable` |
/// | contact | first yield below the flank, `C(κ, ν)·σ_y` — only where `ultimate_allowable` is a yield stress | `contact_fatigue_allowable` |
///
/// A root figure is never handed to a flank: the two fail differently, and
/// the only figure a flank shares with a root is the yield stress, through
/// the subsurface field ([`crate::hertz::first_yield_factor`]).
#[must_use]
pub fn allowable(material: &Material, rating: Rating, kind: CaseKind) -> Option<f64> {
    match (rating, kind) {
        (Rating::Bending, kind) => Some(root_allowable(material, kind)),
        (Rating::Contact { .. }, CaseKind::Fatigue) => material.flank_endurance().map(|v| v.value),
        (Rating::Contact { aspect }, CaseKind::Ultimate) => (material.ultimate_measure
            == crate::material::Measure::Yield)
            .then(|| {
                // Every patch and every ν the table admits has a first yield:
                // a figure at break is the one reason there is none.
                let c = crate::hertz::first_yield_factor(aspect, material.poissons_ratio.value);
                debug_assert!(c.is_some(), "no first yield at κ {aspect}: {material:?}");
                c
            })
            .flatten()
            .map(|c| c * material.ultimate_allowable.value),
    }
}

/// A root's allowable: always published, one per kind.
fn root_allowable(material: &Material, kind: CaseKind) -> f64 {
    match kind {
        CaseKind::Ultimate => material.ultimate_allowable.value,
        CaseKind::Fatigue => material.fatigue_allowable.value,
    }
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
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub enum Duty {
    /// A limited sweep, repeated. The range is measured at a **named** port —
    /// the sweep is a fact about the mechanism's motion, not about where its
    /// load enters, and a 25° sweep of the output is what a designer knows
    /// whichever body is driving it. Every other body's revolutions are
    /// worked from there through the ratios.
    Intermittent {
        /// Sweep per actuation, degrees, at [`Self::Intermittent::at`].
        range_degrees: f64,
        /// The body the sweep is measured at — `None` where the case has
        /// none to follow ([`Train::set_duty`]), which the solve says and
        /// counts nothing over. Left out of a file where `None`.
        at: Option<usize>,
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

impl Duty {
    /// The default duty: a thousand sweeps of 25° measured at this body —
    /// the output, on a train's presets — or unset, `None`.
    #[must_use]
    pub const fn intermittent(at: Option<usize>) -> Self {
        Self::Intermittent {
            range_degrees: 25.0,
            at,
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
/// A train carries any number of these, as it carries any number of gears,
/// and for the same reason: the two loads it used to hold — a peak at the input
/// and a back-driving one at the output — were the first two entries of this
/// list with their ports and directions written into the field names. A case
/// applied at the far end is not a sign on one applied at the near end; it is
/// the same kind of thing entering elsewhere, and [`PortBody`] is what says where.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LoadCase {
    pub kind: CaseKind,
    /// A case switched off takes part in no rating and no sizing, and its
    /// inputs stand, so it can be switched back on as it was; what it comes
    /// to at the train level is still reported ([`TrainCase`]).
    pub enabled: bool,
    /// **The loads on the train's open ports.** The chain's two ends, where
    /// the case does not load them, are *reacted*: each turns as the motion
    /// says and carries the torque the flow puts on it, and both are
    /// reported — a reaction the designer cares about, where a body the
    /// train fixes is ground and reports no speed ([`BodyRole`]). Every
    /// other open port the case does not load is *free* — it turns and
    /// carries nothing, since a reaction there is a thing a designer
    /// attaches and says so by loading it. A load carries a torque and a
    /// speed, each given or derived: the train has some mobility `m` under
    /// what it fixes, exactly `m` of the loads' speeds decide the motion,
    /// the torques on the loads and the reacted ends are `m` short of all
    /// given, and the rest follow ([`Train::relieve_case`] keeps it so).
    pub loads: Vec<Load>,
    /// The duty a fatigue case is spent over. Read by a fatigue case alone —
    /// an ultimate load is survived once and has no cycles to count — and kept
    /// while the case is ultimate for the reason [`Auto::manual`] is kept while
    /// automatic: switching the kind back finds it where it was.
    pub duty: Duty,
    /// **`K_A`, the application factor**: what the driving and driven
    /// machines add to the torque entered, by the designer's judgement of
    /// them. Every stress is rated under the entered torque times it, so
    /// bending scales by `K_A` and a line contact by `√K_A`; the torques and
    /// the flow are reported as entered. Below 1, or not a number, it is
    /// held at 1 and the case says so: `K_A ≥ 1` by definition.
    pub application_factor: f64,
}

/// `K_A = 1`: the torque entered is the design torque.
#[must_use]
pub const fn unit_factor() -> f64 {
    1.0
}

/// **What a case says a port is.** A port the case does not mention has a
/// default — the chain's two ends are reacted, every other open port is
/// free — and an entry says otherwise, or says the default in so many
/// words.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub enum LoadRole {
    /// A load: a torque and a speed, each given or derived.
    #[default]
    Load,
    /// Held by whatever is attached: it turns as the motion says and carries
    /// the torque the flow puts on it, both found rather than given.
    Reacted,
    /// Turning and carrying nothing — a port with nothing attached. A free
    /// port beside a given torque nothing else holds is the question
    /// whether a mesh locks, which the case answers by name.
    Free,
}

/// One port of the train as a case declares it: a load, or a reaction or a
/// free port said in so many words.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Load {
    pub at: usize,
    /// What the port is. The two figures below are read only for a load,
    /// and kept while the port is reacted or free for the reason
    /// [`Auto::manual`] is kept while automatic: switching back finds them
    /// where they were.
    pub role: LoadRole,
    /// N·m, given or derived from the other loads through the flow.
    pub torque: Auto<f64>,
    /// rpm, given or derived from the other loads through the motion. A
    /// given nought is a load held still; the flow then takes its direction
    /// from its impending motion, signed so the given torques do positive
    /// work on it.
    pub speed: Auto<f64>,
}

impl Load {
    /// A load with both figures given.
    #[must_use]
    pub fn given(at: usize, torque: f64, speed: f64) -> Self {
        Self {
            at,
            role: LoadRole::Load,
            torque: Auto::fixed(torque),
            speed: Auto::fixed(speed),
        }
    }

    /// A load whose torque and speed the other loads decide — what a load
    /// case's output is.
    #[must_use]
    pub fn derived(at: usize) -> Self {
        Self {
            at,
            role: LoadRole::Load,
            torque: Auto::automatic(0.0),
            speed: Auto::automatic(0.0),
        }
    }

    /// A port declared reacted or free in so many words, its figures at
    /// nought until it is made a load.
    #[must_use]
    pub fn declared(at: usize, role: LoadRole) -> Self {
        Self {
            role,
            ..Self::derived(at)
        }
    }

    /// Whether this entry is a load — the only kind with figures to read.
    #[must_use]
    pub fn is_load(&self) -> bool {
        self.role == LoadRole::Load
    }
}

impl LoadCase {
    /// **A load between two ports**: an ultimate torque at `input`, driven
    /// at a speed, reacted at `output`, the duty's sweep measured at the
    /// output. The train's own default case, between its two ends.
    #[must_use]
    pub fn ultimate(input: usize, output: usize, torque: f64, speed: f64) -> Self {
        Self {
            kind: CaseKind::Ultimate,
            enabled: true,
            loads: vec![
                Load::given(input, torque, speed),
                Load::declared(output, LoadRole::Reacted),
            ],
            duty: Duty::intermittent(Some(output)),
            application_factor: unit_factor(),
        }
    }

    /// The same load, judged for fatigue over the default duty.
    #[must_use]
    pub fn fatigue(input: usize, output: usize, torque: f64, speed: f64) -> Self {
        Self {
            kind: CaseKind::Fatigue,
            ..Self::ultimate(input, output, torque, speed)
        }
    }

    /// **A fresh case of this kind between two ports**, at the figures a
    /// fresh case starts at ([`CaseKind::fresh_figures`]).
    #[must_use]
    pub fn fresh(kind: CaseKind, input: usize, output: usize) -> Self {
        let (torque, speed) = kind.fresh_figures();
        Self {
            kind,
            ..Self::ultimate(input, output, torque, speed)
        }
    }

    /// **A load from the far end, held at the near one**: a torque at
    /// `output`, held still, with `input` reacted — carrying whatever holds
    /// it, which through a mesh that locks is nothing — so every mesh
    /// carries it back, cut by each one's loss the backward way. The input
    /// *free* instead — turning, carrying nothing, so a train that can be
    /// back-driven turns under the load and reacts none of it — is the
    /// same case with the reaction declared free.
    #[must_use]
    pub fn back_driving(input: usize, output: usize, torque: f64) -> Self {
        Self {
            kind: CaseKind::Ultimate,
            enabled: true,
            loads: vec![
                Load::given(output, torque, 0.0),
                Load::declared(input, LoadRole::Reacted),
            ],
            duty: Duty::intermittent(Some(output)),
            application_factor: unit_factor(),
        }
    }

    /// **`K_A` as the rating applies it**: the factor given, or 1 where
    /// the factor given is below 1 or not a number.
    #[must_use]
    pub fn applied_factor(&self) -> f64 {
        if crate::input::APPLICATION_FACTOR_HELD.admits(self.application_factor) {
            self.application_factor
        } else {
            unit_factor()
        }
    }

    /// The entry at a body, where the case has one.
    #[must_use]
    pub fn load_at(&self, at: usize) -> Option<&Load> {
        self.loads.iter().find(|l| l.at == at)
    }

    /// The first load's given torque, N·m — the case's own, on a case with
    /// one load; `None` on a case with none.
    #[must_use]
    pub fn torque(&self) -> Option<f64> {
        self.loads.first().map(|l| l.torque.manual)
    }

    /// The first load's given speed, rpm; `None` on a case with no load.
    #[must_use]
    pub fn speed(&self) -> Option<f64> {
        self.loads.first().map(|l| l.speed.manual)
    }

    /// Give the first load this torque.
    pub fn set_torque(&mut self, torque: f64) {
        if let Some(l) = self.loads.first_mut() {
            l.torque = Auto::fixed(torque);
        }
    }

    /// Give the first load this speed.
    pub fn set_speed(&mut self, speed: f64) {
        if let Some(l) = self.loads.first_mut() {
            l.speed = Auto::fixed(speed);
        }
    }

    /// Move the first load to another body.
    pub fn set_port(&mut self, at: usize) {
        if let Some(l) = self.loads.first_mut() {
            l.at = at;
        }
    }

    /// The duty this case is spent over, where it has one.
    #[must_use]
    pub fn counted(&self) -> Option<&Duty> {
        (self.kind == CaseKind::Fatigue).then_some(&self.duty)
    }
}

/// How often the body a member takes its load on comes round in one case.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turns {
    /// Revolutions over the whole duty.
    pub revolutions: Revolutions,
    /// The number of actuations, where the duty reverses between them — the
    /// count [`loaded_cycles`] rounds within, and the flag that reverses every
    /// root. `None` for a duty that does not reverse.
    pub reversing_actuations: Option<u32>,
}

/// **One load case as one part is rated for it**: the train's flow, read
/// off for this part's meshes and bodies.
///
/// Assembled by [`solve_train`], which is the only level that solves a
/// case: the motion from the loads' given speeds, then one flow across every
/// part's meshes with the loads' given torques known and the reactions
/// unknown. A part rates what it is handed — the torque pressing each of
/// its meshes and which member drives it, every body's speed and the torque
/// its meshes put on it — and solves no flow of its own for a case.
#[derive(Clone, Debug, PartialEq)]
pub struct CaseLoad {
    /// Index into the train's list of load cases.
    pub case: usize,
    pub kind: CaseKind,
    /// Per mesh: the driver's torque read across to member `a`, summed over
    /// the mesh's instances, signed as the flow signs it
    /// ([`flow::Flow::mesh_torques`]); and which member drives.
    pub mesh_torques: Vec<f64>,
    pub directions: Vec<Drive>,
    /// Per mesh: the power crossing it over the power the case puts in —
    /// the flow's [`flow::Flow::mesh_powers`].
    pub mesh_powers: Vec<f64>,
    /// Per local body, ground first: rpm, and the torque this part's
    /// meshes put on it — the external load at a port, what the part
    /// delivers onward where another takes it up.
    pub speeds: Vec<f64>,
    pub torques: Vec<f64>,
    /// Per member of the part: the torque its meshes put on its body, signed
    /// as a body's torques are — [`GearCase::on_body`].
    pub on_members: Vec<f64>,
    /// Per local body: revolutions over a fatigue case's duty; `None` on an
    /// ultimate case, which has no cycles to count.
    pub turns: Option<Vec<Revolutions>>,
    /// The actuations a reversing duty counts, where it reverses.
    pub reversing_actuations: Option<u32>,
    /// `K_A` as applied ([`LoadCase::applied_factor`]): what every mesh's
    /// torque is multiplied by for rating, and for nothing else.
    pub application_factor: f64,
}

impl CaseLoad {
    /// Whether this case's duty loads every root both ways.
    #[must_use]
    pub fn reverses(&self) -> bool {
        self.turns.is_some() && self.reversing_actuations.is_some()
    }

    /// A case carrying nothing: every torque nought, every body still.
    #[must_use]
    pub fn nothing(case: usize, kind: CaseKind, wiring: &Wiring) -> Self {
        let (meshes, shafts) = (wiring.meshes.len(), wiring.slots.len());
        Self {
            case,
            kind,
            mesh_torques: vec![0.0; meshes],
            directions: vec![Drive::Forward; meshes],
            mesh_powers: vec![0.0; meshes],
            speeds: vec![0.0; shafts],
            torques: vec![0.0; shafts],
            on_members: vec![0.0; wiring.mounts.len()],
            turns: None,
            reversing_actuations: None,
            application_factor: unit_factor(),
        }
    }
}

/// **A preset asked about alone** — a train of one, laid in and loaded by
/// its own conventions.
///
/// Loaded at its conventional input with `torque` N·m at `speed` rpm and
/// reacted at its output, an ultimate case and then a fatigue one, both on:
/// what a test, the harness or the sweep asks of a preset with no train
/// round it. **Its bodies are numbered by its slots** — the identity on a
/// shape built alone, and the numbers closed up on one edited in place or
/// lifted off a longer train, where a number no body carries would be a body
/// free to turn and no motion unique. The held bodies of its convention are
/// written as the train's holds, so what it is asked is in the train and
/// nowhere else. **Its first path is what a stage's figures once were** — ratio,
/// efficiency, backlash, the power through its teeth, one more tooth — and
/// the law `a_path_is_what_it_crosses` holds a part of a chain to it.
impl Train {
    #[must_use]
    pub fn alone(preset: &Shape, torque: f64, speed: f64) -> Self {
        let mut shape = preset.clone();
        shape.renumber_bodies(|b| preset.slot(b));
        let ports = shape.ports();
        let (input, output) = (ports.input(), ports.output());
        Self {
            load_cases: vec![
                LoadCase::ultimate(input, output, torque, speed),
                LoadCase::fatigue(input, output, torque, speed),
            ],
            reversed_bending: false,
            held: ports.held,
            shape,
        }
    }

    /// **This lone preset asked in another arrangement**: `held` held and
    /// nothing else, and every case loaded at `input` and reacted at
    /// `output` — a set asked with its carrier held rather than its ring,
    /// say. Bodies are the preset's slots, which is how [`Self::alone`]
    /// numbers them.
    #[must_use]
    pub fn arranged(mut self, held: &[Body], input: Body, output: Body) -> Self {
        self.held = held.to_vec();
        for case in &mut self.load_cases {
            for load in &mut case.loads {
                if load.is_load() {
                    load.at = input;
                } else {
                    load.at = output;
                }
            }
            case.duty = match case.duty {
                Duty::Intermittent {
                    range_degrees,
                    actuations,
                    reversing,
                    ..
                } => Duty::Intermittent {
                    range_degrees,
                    at: Some(output),
                    actuations,
                    reversing,
                },
                continuous => continuous,
            };
        }
        self
    }

    /// This train, told how it treats a reversed root.
    #[must_use]
    pub fn with_reversal(mut self, reversal: Reversal) -> Self {
        self.reversed_bending = reversal.correct;
        self
    }
}

/// **A preset asked about alone, answered** ([`Train::alone`]): the train of
/// one solved as every train is, part by part ([`solve_train`]) — a preset
/// is one part, which a law holds every preset to — read as that part in
/// its own numbering, and the path its first case walks: ratio,
/// efficiency, backlash, the power through its teeth, one more tooth. Each
/// figure is `None` where the train reports no path — holds that leave the
/// motion a family — or where the path's figure was not computed either way.
///
/// It dereferences to the part's result, so what a caller reads of the
/// members, the meshes and the distances reads as the shape numbers them.
#[derive(Clone, Debug)]
pub struct Alone {
    pub part: ShapeResult,
    pub ratio: Option<f64>,
    pub efficiency: Option<Directional<f64>>,
    pub backlash: Option<Directional<Backlash>>,
    pub circulation: Option<Directional<f64>>,
    /// One more tooth on each member, in the preset's member order.
    pub ratio_per_tooth: Option<Vec<Option<f64>>>,
}

impl std::ops::Deref for Alone {
    type Target = ShapeResult;
    fn deref(&self) -> &ShapeResult {
        &self.part
    }
}

/// **A train of one, solved** ([`Train::alone`]) — see [`Alone`]. A lone
/// preset's fault is its own, so it is handed back as its part raised it
/// rather than wrapped in which part of one it was.
///
/// # Errors
///
/// Whatever the part reports.
pub fn solve_alone(train: &Train, lib: &MaterialLibrary) -> Result<Alone, TrainError> {
    if train.shape.members.is_empty() {
        return Err(MotionError::Empty.into());
    }
    entering(train, lib)?;
    let parts = train.parts();
    debug_assert_eq!(parts.len(), 1, "a preset asked alone is one part");
    let (r, mut own) = solve_parts(train, &parts, lib).map_err(|e| match e {
        TrainError::InPart { cause, .. } => *cause,
        other => other,
    })?;
    // The path its first case walks, from its load to its reaction — the
    // conventional ends, on a train fresh from [`Train::alone`] — or, for a
    // case that names neither, the first the train reports.
    let ends = train.load_cases.first().and_then(|c| {
        let load = c.loads.iter().find(|l| l.is_load())?;
        let reaction = c.loads.iter().find(|l| !l.is_load())?;
        Some((load.at, reaction.at))
    });
    let path = match ends {
        Some(ends) => r.paths.iter().find(|p| (p.from, p.to) == ends),
        None => r.paths.first(),
    }
    .cloned();
    Ok(Alone {
        part: own.remove(0),
        ratio: path.as_ref().map(|p| p.ratio),
        efficiency: path.as_ref().and_then(|p| p.efficiency.both()),
        backlash: path.as_ref().and_then(|p| p.backlash.both()),
        circulation: path.as_ref().and_then(|p| p.circulation.both()),
        ratio_per_tooth: path.map(|p| p.per_tooth),
    })
}

/// The largest magnitude among `torque(case)`, and each case's as a
/// fraction of it — what a figure evaluated once at the worst load a mesh
/// carries scales by to reach each case.
///
/// A rating is evaluated once, at the worst torque the mesh carries, and
/// every case is that scaled ([`Loading::under`]). **A zero worst is zero
/// everywhere** — a mesh carrying nothing is a load and not a division to
/// guard against.
pub(crate) fn scaled(cases: &[CaseLoad], torque: impl Fn(&CaseLoad) -> f64) -> (f64, Vec<f64>) {
    let torques: Vec<f64> = cases.iter().map(torque).collect();
    let worst = torques.iter().fold(0.0_f64, |m, t| m.max(t.abs()));
    let scales = torques
        .iter()
        .map(|t| if worst == 0.0 { 0.0 } else { t / worst })
        .collect();
    (worst, scales)
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
/// hunts, and every mesh asks it.
pub(crate) const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
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
///
/// **Rounded exactly**: the revolutions are a rational and the ceiling is
/// taken of it, so a count that is whole is that whole number — not one
/// more for a rounding above it. `None` where the count is past what an
/// `i128` holds.
#[must_use]
pub fn loaded_cycles(turns: Turns) -> Cycles {
    match turns.reversing_actuations {
        Some(actuations) => {
            let bending = turns.revolutions.per(actuations).ceil() * f64::from(actuations);
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

/// **A count of revolutions**, exact where an `i128` holds it — a quotient
/// of tooth counts times the duty's own figures, read exactly — and its
/// double beside, which is all there is past that.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Revolutions {
    pub exact: Option<crate::ratio::Ratio>,
    pub read: f64,
}

/// The roundings a count read as doubles carries: a speed's quotient, the
/// sixty or the sweep's 360th, the duration, a difference against the
/// frame and the paths — five, each at most `ε/2` of the value, with
/// margin to eight.
const COUNT_ROUNDINGS: f64 = 8.0;

impl Revolutions {
    fn exactly(r: crate::ratio::Ratio) -> Self {
        Self {
            exact: Some(r),
            read: r.to_f64(),
        }
    }

    /// Turns against another body's, as a magnitude.
    #[must_use]
    pub fn against(self, frame: Self) -> Self {
        Self {
            exact: self
                .exact
                .zip(frame.exact)
                .and_then(|(a, b)| a.checked_sub(b)?.checked_abs()),
            read: (self.read - frame.read).abs(),
        }
    }

    /// `n` times as many.
    #[must_use]
    pub fn times(self, n: u32) -> Self {
        Self {
            exact: self
                .exact
                .and_then(|a| a.checked_mul(crate::ratio::Ratio::whole(i64::from(n)))),
            read: self.read * f64::from(n),
        }
    }

    /// One `n`th as many.
    #[must_use]
    pub fn per(self, n: u32) -> Self {
        Self {
            exact: self
                .exact
                .and_then(|a| a.checked_div(crate::ratio::Ratio::whole(i64::from(n)))),
            read: self.read / f64::from(n),
        }
    }

    /// **The whole number of revolutions that covers this many**: the
    /// ceiling of the exact count, so a whole count reads whole. Past an
    /// `i128`, the double's, a count within its own roundings
    /// ([`COUNT_ROUNDINGS`]) of a whole number read as that number.
    #[must_use]
    pub fn ceil(self) -> f64 {
        match self.exact {
            Some(r) => r.ceil() as f64,
            None => {
                let whole = self.read.round();
                if (self.read - whole).abs() <= COUNT_ROUNDINGS * f64::EPSILON * self.read.abs() {
                    whole
                } else {
                    self.read.ceil()
                }
            }
        }
    }
}

/// **Each body's turns over a continuous duty**: its speed in rpm times
/// sixty times the hours, exactly where it fits.
fn continuous_turns(speeds: &[crate::ratio::Ratio], runtime_hours: f64) -> Vec<Revolutions> {
    use crate::ratio::Ratio;
    let minutes = Ratio::of_double(runtime_hours).and_then(|h| h.checked_mul(Ratio::whole(60)));
    speeds
        .iter()
        .map(|v| match minutes.and_then(|m| v.checked_mul(m)) {
            Some(r) => Revolutions::exactly(r),
            None => Revolutions {
                exact: None,
                read: v.to_f64() * 60.0 * runtime_hours,
            },
        })
        .collect()
}

/// **Each body's turns over an intermittent duty**: its share of the sweep
/// stated at `port`, `per[s] / per[port] · range / 360`, times the
/// actuations, exactly where it fits.
fn swept_turns(
    per: &[crate::ratio::Ratio],
    port: Body,
    range_degrees: f64,
    actuations: u32,
) -> Vec<Revolutions> {
    use crate::ratio::Ratio;
    let sweep = Ratio::of_double(range_degrees)
        .and_then(|r| r.checked_div(Ratio::whole(360)))
        .and_then(|r| r.checked_mul(Ratio::whole(i64::from(actuations))));
    per.iter()
        .map(
            |v| match sweep.and_then(|s| v.checked_div(per[port])?.checked_mul(s)) {
                Some(r) => Revolutions::exactly(r),
                None => Revolutions {
                    exact: None,
                    read: v.to_f64() / per[port].to_f64()
                        * (range_degrees / 360.0)
                        * f64::from(actuations),
                },
            },
        )
        .collect()
}

/// **One unit in the last place of a double**: the gap to the next one out,
/// the precision a figure given as that double carries.
fn ulp(x: f64) -> f64 {
    let a = x.abs();
    a.next_up() - a
}

/// **The motion a case's given speeds make**, every body's speed exactly.
///
/// The holds first, then each given port in the case's order. A port the
/// ports before it leave free is driven at its speed, read exactly
/// ([`crate::ratio::Ratio::of_double`]). A port they decide is one condition
/// said twice: it passes where its speed and the decided one differ by no
/// more than the speeds carry — its own ulp, and each earlier speed's ulp
/// through the exact ratio of the two — which is a rounding of one
/// condition and not a second; past that it is refused at its body
/// (`train.case_overdetermined`). A motion still free is a count short
/// (`train.case_underdetermined`, the count of freedoms left).
fn given_motion(
    system: &crate::kinematics::System,
    conditions: &[Condition],
    given: &[(Body, f64)],
) -> Result<Vec<crate::ratio::Ratio>, Note> {
    use crate::kinematics::Refusal;
    use crate::ratio::Ratio;
    let overflow = || Note::new(key::ERROR_TRAIN_OVERFLOW);
    let held: Vec<Body> = (0..conditions.len())
        .filter(|&b| conditions[b] == Condition::Ground)
        .collect();
    let solve = |drivers: &[(Body, Ratio)]| {
        let mut c = conditions.to_vec();
        for &(g, v) in drivers {
            c[g] = Condition::Drive(v);
        }
        let order: Vec<Body> = held
            .iter()
            .copied()
            .chain(drivers.iter().map(|d| d.0))
            .collect();
        system.motion_in(&c, &order)
    };
    let mut drivers: Vec<(Body, Ratio)> = Vec::new();
    let mut given_at: Vec<f64> = Vec::new();
    for &(g, s) in given {
        let v = Ratio::of_double(s).ok_or_else(overflow)?;
        let sol = solve(&drivers).map_err(|_| overflow())?;
        if sol.residual.iter().any(|r| !r.direction[g].is_zero()) {
            drivers.push((g, v));
            given_at.push(s);
            continue;
        }
        // Decided by the speeds before it: how far it may differ is what
        // they and it carry. Each earlier speed's reach is the motion at `g`
        // per unit of it, the others still.
        let mut reach = ulp(s);
        for (i, &si) in given_at.iter().enumerate() {
            let unit: Vec<(Body, Ratio)> = drivers
                .iter()
                .enumerate()
                .map(|(j, &(b, _))| (b, if i == j { Ratio::ONE } else { Ratio::ZERO }))
                .collect();
            let per = solve(&unit).map_err(|_| overflow())?.values[g];
            reach += per.to_f64().abs() * ulp(si);
        }
        let gap = v
            .checked_sub(sol.values[g])
            .ok_or_else(overflow)?
            .to_f64()
            .abs();
        if gap > reach {
            return Err(located(key::TRAIN_CASE_OVERDETERMINED, g));
        }
    }
    match solve(&drivers) {
        Ok(sol) if sol.is_unique() => Ok(sol.values),
        Ok(sol) => {
            Err(Note::new(key::TRAIN_CASE_UNDERDETERMINED).tally("short", sol.residual.len()))
        }
        Err(Refusal::Conflicts(at)) => Err(located(key::TRAIN_CASE_OVERDETERMINED, at)),
        Err(_) => Err(overflow()),
    }
}

/// **Why a case's flow left the load's division open**: a loop of meshes
/// carrying torque round that reaches no body (`train.mesh_loop`, the
/// meshes named by the graph's numbers) where the known torques leave no
/// port's torque free, and otherwise a load held at both ends
/// (`train.load_shared`). Read on the lossless rows, which are exact.
fn undetermined_note(
    parts: &[graph::Part],
    system: &crate::kinematics::System,
    asked: &flow::Asked,
    mesh_of_part: &[Vec<usize>],
) -> Note {
    let applied: Vec<Option<crate::ratio::Ratio>> = asked
        .known
        .iter()
        .map(|k| k.map(|_| crate::ratio::Ratio::ZERO))
        .collect();
    let ports_free = system
        .torques(&applied)
        .map_or(true, |s| !s.residual.is_empty());
    let looped = system.circulating().unwrap_or_default();
    if ports_free || looped.is_empty() {
        return Note::new(key::TRAIN_LOAD_SHARED);
    }
    // The system's meshes are the flow's, part by part: back to the graph's.
    let mut meshes: Vec<usize> = parts
        .iter()
        .zip(mesh_of_part)
        .flat_map(|(part, mine)| {
            mine.iter()
                .enumerate()
                .filter(|(_, at)| looped.contains(at))
                .map(|(j, _)| part.meshes[j])
        })
        .collect();
    meshes.sort_unstable();
    let named: Vec<String> = meshes.iter().map(|m| (m + 1).to_string()).collect();
    Note::new(key::TRAIN_MESH_LOOP).text("meshes", named.join(", "))
}

/// **The train's figures, one row per path** — see [`PathReport`]: every
/// path an enabled case uses, from each of its loads to each of its
/// reactions, once each and a direction each, in case order — so the
/// first is the headline case's. Empty where the train's holds leave its motion a family, since
/// a ratio between two ports of a mechanism with two freedoms needs a
/// third held, and which is the designer's to say.
#[allow(clippy::too_many_arguments)]
fn paths_of(
    train: &Train,
    parts: &[graph::Part],
    system: &crate::kinematics::System,
    [moving, resting]: [&[flow::MeshFlow]; 2],
    with_teeth: usize,
    base: &[Condition],
    rated: &[ShapeResult],
) -> Vec<PathReport> {
    let shafts = system.bodies();
    // **Which paths are worth a row**: in case order, every path an
    // enabled case actually uses, from each of its loads to each of its
    // reactions. Every pair of open bodies is a path the graph could
    // answer, and on a train of many parts nearly all of them are ones
    // nobody asked about; the way to ask is a case. (The chain's two
    // conventional ends used to be a row of their own, present whether or
    // not any case loaded them — a reading nobody had stated.)
    let mut wanted: Vec<(usize, usize)> = Vec::new();
    // **A path has a direction**: its ratio is the one end's turns per the
    // other's, its efficiency and its play are read driving from the one and
    // then from the other, so a case that loads the far end and reacts the
    // near one asks for the path the other way round, and gets it — it was
    // once taken for the same pair, and the case said it walked no path.
    let mut want = |a: usize, b: usize| {
        if a != b && !wanted.contains(&(a, b)) {
            wanted.push((a, b));
        }
    };
    // An entry at a held body, or at one no part has, is no path's end.
    let bodies = train.bodies_of(parts);
    let open = |b: usize| -> Option<usize> {
        bodies
            .iter()
            .find(|x| !x.held && x.body == b)
            .map(|x| x.body)
    };
    for case in train.load_cases.iter().filter(|c| c.enabled) {
        for load in case.loads.iter().filter(|l| l.is_load()) {
            for reaction in case.loads.iter().filter(|l| l.role == LoadRole::Reacted) {
                if let (Some(a), Some(b)) = (open(load.at), open(reaction.at)) {
                    want(a, b);
                }
            }
        }
    }
    // One motion, or none: driven at one open body with the rest free, the
    // holds must leave nothing else to decide.
    let motion_from = |from: Body| -> Option<Vec<f64>> {
        let mut c: Vec<Condition> = base.to_vec();
        c[from] = Condition::Drive(crate::ratio::Ratio::ONE);
        let s = system.motion_in(&c, &[from]).ok()?;
        s.is_unique()
            .then(|| s.values.iter().map(|r| r.to_f64()).collect())
    };
    let held: Vec<Body> = (1..shafts)
        .filter(|&s| base[s] == Condition::Ground)
        .collect();
    // Every mesh's play in the row's own units at the three band points,
    // in the flow's mesh order, which is each part's in turn — and the
    // graph's distance each is on, the source its play moves with.
    let row_play: Vec<Ends> = rated
        .iter()
        .flat_map(|r| r.meshes.iter().map(|m| m.row_play))
        .collect();
    let distance_of: Vec<Option<usize>> = parts
        .iter()
        .flat_map(|p| {
            let at = p.shape.indexed();
            (0..p.shape.meshes.len())
                .map(|j| at.distance_of(j).map(|d| p.distances[d]))
                .collect::<Vec<_>>()
        })
        .collect();
    // The flow driving `from` against `to` through `meshes`: its
    // efficiency, and the power its meshes pass over the power in.
    // `None` where the flow is refused: not a figure of nought.
    let flowing =
        |meshes: &[flow::MeshFlow], from: Body, to: Body, speed: &[f64]| -> Option<(f64, f64)> {
            flow::solve(
                shafts,
                meshes,
                speed,
                &flow::Asked::through(shafts, from, speed[from].signum(), to, &held),
            )
            .ok()
            .map(|f| {
                (
                    f.efficiency,
                    f.mesh_powers[..with_teeth.min(f.mesh_powers.len())]
                        .iter()
                        .sum(),
                )
            })
        };
    // **One more tooth on each gear**, by the graph's index for it: the
    // same motion asked of the system with that one count raised.
    let per_tooth = |from: Body, to: Body| -> Vec<Option<f64>> {
        let mut out = vec![None; train.shape.members.len()];
        for (k, part) in parts.iter().enumerate() {
            for i in 0..part.shape.members.len() {
                // No count past the wire's `u32` to ask: that gear has no
                // tooth more.
                if part.shape.members[i].gear.teeth == u32::MAX {
                    continue;
                }
                let raising = |j: usize, m: usize, z: u32| {
                    if (j, m) == (k, i) {
                        z.saturating_add(1)
                    } else {
                        z
                    }
                };
                let raised = train.system_counting(parts, raising).ok().and_then(|sys| {
                    let mut c: Vec<Condition> = base.to_vec();
                    c[from] = Condition::Drive(crate::ratio::Ratio::ONE);
                    let s = sys.motion_in(&c, &[from]).ok()?;
                    if !s.is_unique() {
                        return None;
                    }
                    let (a, b) = (s.values[from].to_f64(), s.values[to].to_f64());
                    (b != 0.0).then(|| a / b).filter(|r| r.is_finite())
                });
                out[part.members[i]] = raised;
            }
        }
        out
    };
    // Play at `read` per unit of play in mesh `k`, with `from` and the held
    // bodies standing still — one construction, on the whole graph. `None`
    // where a unit play in the mesh admits no motion: a mesh in a loop.
    let coefficient = |k: usize, read: Body, from: Body| -> Option<f64> {
        let mut c: Vec<Condition> = base.to_vec();
        c[from] = Condition::Ground;
        system
            .play(k, &c)
            .and_then(Result::ok)
            .map(|s| s.values[read].to_f64().abs())
    };
    // **The band, one source per distance**: the meshes on one distance
    // move together as it runs at its minus end, its running point or its
    // plus end, and distances apart are independent, so each reaches its
    // own extreme at once. A mesh with no distance is a source of its own.
    let backlash_at = |read: Body, from: Body| -> Option<Backlash> {
        let mut sources: Vec<(Result<usize, usize>, Ends)> = Vec::new();
        for (k, play) in row_play.iter().enumerate() {
            let c = coefficient(k, read, from)?;
            let source = distance_of.get(k).copied().flatten().ok_or(k);
            let at = match sources.iter().position(|(s, _)| *s == source) {
                Some(i) => i,
                None => {
                    sources.push((
                        source,
                        Ends {
                            minus: Some(0.0),
                            running: 0.0,
                            plus: Some(0.0),
                        },
                    ));
                    sources.len() - 1
                }
            };
            let sum = &mut sources[at].1;
            *sum = Ends {
                minus: sum.minus.zip(play.minus).map(|(s, p)| s + c * p),
                running: sum.running + c * play.running,
                plus: sum.plus.zip(play.plus).map(|(s, p)| s + c * p),
            };
        }
        Some(Backlash::of_sources(
            sources
                .into_iter()
                .map(|(_, ends)| ends.map(f64::to_degrees)),
        ))
    };
    let mut out = Vec::new();
    for (from, to) in wanted {
        let (a, b) = (from, to);
        let Some(forward) = motion_from(a) else {
            continue;
        };
        let Some(backward) = motion_from(b) else {
            continue;
        };
        if forward[b] == 0.0 || backward[a] == 0.0 {
            continue;
        }
        let (ahead, astern) = (
            flowing(moving, a, b, &forward),
            flowing(moving, b, a, &backward),
        );
        // **Whether it breaks away** is its own flow against static
        // friction, the whole path at once: a drive that cannot start
        // delivers nothing, whatever it would do once turning.
        let at_rest = Directional {
            forward: flowing(resting, a, b, &forward).map(|f| f.0),
            backward: flowing(resting, b, a, &backward).map(|f| f.0),
        };
        out.push(PathReport {
            from,
            to,
            ratio: forward[a] / forward[b],
            reads: RatioReading::of(forward[a] / forward[b]),
            efficiency: Directional {
                forward: ahead.map(|f| f.0),
                backward: astern.map(|f| f.0),
            }
            .once_moving(&at_rest),
            backlash: Directional {
                forward: backlash_at(b, a),
                backward: backlash_at(a, b),
            },
            circulation: Directional {
                forward: ahead.map(|f| f.1),
                backward: astern.map(|f| f.1),
            },
            per_tooth: per_tooth(a, b),
        });
    }
    out
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
    /// Every load the train is rated for. Any number, as the gears are any
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
    pub reversed_bending: bool,
    /// **The train as one graph** — its axes, the bodies on them, the gears
    /// on those, the meshes and the distances between axes that mesh, and
    /// the couplings — every preset added laid into it, a body two presets
    /// share listed once on the one axis it turns about. Its parts are what
    /// the stages were ([`Train::parts`]); nothing stores them.
    pub shape: Shape,
    /// **The bodies held to ground**, every one stated. A preset's
    /// conventional hold — a set's ring — is written here when it is
    /// inserted ([`Train::chain_on`]), so a body is held exactly where
    /// this says, and released by taking it out. It was a list of
    /// constraints laid over each stage's conventions, which held what
    /// nobody had written and needed a *free* to say otherwise.
    pub held: Vec<usize>,
}

/// One of a load's two figures, named so a caller can say which was
/// just touched.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum LoadFreedom {
    Torque,
    Speed,
}

/// One figure of one load of a case: what a panel names when it toggles it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
pub struct CaseFreedom {
    pub load: usize,
    pub which: LoadFreedom,
}

impl Train {
    /// **The mobility a case's loads have to decide** — the degrees of
    /// freedom the train has under what it fixes, with every port free.
    /// Exactly this many of a case's speeds, and this many of its torques,
    /// must be given; [`Self::relieve_case`] keeps it so.
    ///
    /// # Errors
    ///
    /// [`MotionError`] where the train has no graph to ask.
    pub fn case_mobility(&self) -> Result<usize, MotionError> {
        let system = self.system()?;
        let shafts = system.bodies();
        let conditions: Vec<Condition> = self
            .conditions(shafts)?
            .into_iter()
            .map(|c| match c {
                Condition::Drive(_) => Condition::Free,
                other => other,
            })
            .collect();
        system
            .motion_in(&conditions, &[])
            .map(|s| s.residual.len())
            .map_err(|e| match e {
                crate::kinematics::Refusal::Conflicts(i) => MotionError::Conflicts(i),
                crate::kinematics::Refusal::NoMotion | crate::kinematics::Refusal::Overflow => {
                    MotionError::Overflow
                }
            })
    }

    /// **One load case relieved**: of its loads' speeds exactly the
    /// train's mobility `m` given and the rest derived; of the torques on
    /// its loads and its reacted ends together, `m` derived and the rest
    /// given — one statics equation per degree of freedom, so a pair with
    /// one load and one reacted end has one torque given, a take-off
    /// between two presets two, a differential's three ports one. The
    /// figure just touched is kept and the others turned in load order from
    /// the last — so a designer who gives a second speed on a pair sees the
    /// first become derived. A case with fewer given than that is left
    /// short, and the solve says so; relief never invents a given, and a
    /// case short of a speed keeps every torque it was given, since there
    /// is no motion to hold them to.
    ///
    /// Every derived figure is then seeded from what the case last came to,
    /// where it solves, so a box turned derived shows the number rather
    /// than a stale one. [`Self::relieve_case_toggles`] is the first half
    /// alone, for a reader with no library to solve under.
    ///
    /// # Errors
    ///
    /// [`MotionError`] where the train has no graph to ask.
    pub fn relieve_case(
        &mut self,
        case: usize,
        just: Option<CaseFreedom>,
        lib: &MaterialLibrary,
    ) -> Result<(), MotionError> {
        self.relieve_case_toggles(case, just)?;
        // Seed what is derived from what it comes to.
        if let Ok(r) = solve_train(self, lib) {
            if let Some(solved) = r.cases.iter().find(|x| x.case == case && x.solved) {
                let c = &mut self.load_cases[case];
                for l in &mut c.loads {
                    let Some(s) = solved.shaft(l.at) else {
                        continue;
                    };
                    if l.torque.auto {
                        l.torque.manual = (s.torque * 1e4).round() / 1e4;
                    }
                    // A body that does not turn has no speed to seed from.
                    if let (true, Some(speed)) = (l.speed.auto, s.speed) {
                        l.speed.manual = (speed * 1e4).round() / 1e4;
                    }
                }
            }
        }
        Ok(())
    }

    /// [`Self::relieve_case`]'s toggles alone — which figures stand given —
    /// with every number kept; whether any toggle moved.
    ///
    /// # Errors
    ///
    /// [`MotionError`] where the train has no graph to ask.
    pub fn relieve_case_toggles(
        &mut self,
        case: usize,
        just: Option<CaseFreedom>,
    ) -> Result<bool, MotionError> {
        let m = self.case_mobility()?;
        let Some(c) = self.load_cases.get_mut(case) else {
            return Ok(false);
        };
        let mut moved = false;
        // **Relief turns the loads' figures and nothing else.** A port
        // declared reacted or free is a declaration, not a figure: a
        // reacted port is one unknown torque wherever it is, and a free
        // port is a port that could have held the load and was told not to
        // — it counts here as the reaction it declines to be, so that a
        // given torque beside it stands and the solve can say that nothing
        // holds it, which is the question whether a mesh locks, asked on
        // purpose and answered by name.
        let n_loads = c.loads.iter().filter(|l| l.is_load()).count();
        let reacted = c.loads.iter().filter(|l| !l.is_load()).count();
        // A case short of a speed has no motion, and no statics to hold its
        // torques to: they are left as given, and the solve says what is
        // short. Relief takes a figure back only where the case can use
        // what remains.
        let speeds_given = c
            .loads
            .iter()
            .filter(|l| l.is_load() && !l.speed.auto)
            .count();
        for which in [LoadFreedom::Speed, LoadFreedom::Torque] {
            // The speeds decide the motion: `m` of them. The torques are
            // one short of the bodies that carry one, per degree of freedom.
            let limit = match which {
                LoadFreedom::Speed => m,
                LoadFreedom::Torque if speeds_given < m => usize::MAX,
                LoadFreedom::Torque => (n_loads + reacted).saturating_sub(m),
            };
            fn field(l: &mut Load, which: LoadFreedom) -> &mut Auto<f64> {
                match which {
                    LoadFreedom::Torque => &mut l.torque,
                    LoadFreedom::Speed => &mut l.speed,
                }
            }
            let mut given = c
                .loads
                .iter()
                .filter(|l| l.is_load())
                .filter(|l| match which {
                    LoadFreedom::Torque => !l.torque.auto,
                    LoadFreedom::Speed => !l.speed.auto,
                })
                .count();
            // Spare the one just touched, then — only if it must — that too.
            for spare_just in [true, false] {
                for (i, l) in c.loads.iter_mut().enumerate().rev() {
                    if given <= limit {
                        break;
                    }
                    if !l.is_load() {
                        continue;
                    }
                    let is_just = just == Some(CaseFreedom { load: i, which });
                    if spare_just && is_just {
                        continue;
                    }
                    let f = field(l, which);
                    if !f.auto {
                        f.auto = true;
                        given -= 1;
                        moved = true;
                    }
                }
            }
        }
        Ok(moved)
    }
}

/// What a body is in one load case.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum BodyRole {
    /// An open port the case loads: it turns, and carries the torque given
    /// or derived for it.
    Load,
    /// An open port the case does not load: it turns as the motion says
    /// and carries the torque the flow puts on it, both reported — a
    /// reaction the designer cares about, where [`Self::Fixed`] is ground.
    Reacted,
    /// Held by the train: ground under another name.
    Fixed,
    /// Neither a port nor held — an idler's body, a planet's, a coupling
    /// inside the train: it turns as the graph says and carries no external
    /// torque.
    Free,
}

/// One body of the train in one load case.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct CaseBody {
    pub at: usize,
    pub role: BodyRole,
    /// rpm; `None` on a body that is held, which has none to report.
    pub speed: Option<f64>,
    /// The external torque on it, N·m: the load at a loaded port, the
    /// reaction at a held one, nought on a free body.
    pub torque: f64,
}

/// **What one load case comes to at the train level**: every body, with
/// what it is in this case and what it carries — the loads as given or
/// derived, the reactions found.
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
    /// Every body of the train, ground first, in number order.
    pub bodies: Vec<CaseBody>,
    /// Whether the case could be solved at all: its loads decide the motion
    /// and the flow. Where they do not — too few given, or nothing driving —
    /// the bodies carry nothing and the notes say why.
    pub solved: bool,
    /// What the train wants read about this case: that it is short of a
    /// speed or a torque, that nothing drives it, or that nothing holds it.
    pub notes: Vec<Note>,
}

impl TrainCase {
    /// One body of the case, by reference.
    #[must_use]
    pub fn shaft(&self, at: usize) -> Option<&CaseBody> {
        self.bodies.iter().find(|s| s.at == at)
    }

    /// The ports this case reacts, in the train's order.
    pub fn reacted(&self) -> impl Iterator<Item = &CaseBody> {
        self.bodies.iter().filter(|s| s.role == BodyRole::Reacted)
    }
}

/// **A ratio as it is read**: so many turns to one, whichever way round
/// keeps the figure at one or more — `2.5294 : 1` for a reduction, and
/// `1 : 2.5294` for a step-up rather than `0.3953 : 1`. The sign, where the
/// two ends turn opposite ways, is the figure's. Decided here once, so the
/// path's row and an edit's dry run read a path the same way.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct RatioReading {
    /// The side that is not one: `|turns| ≥ 1`.
    pub turns: f64,
    /// Whether the one is the first side's — a step-up, read `1 : turns`.
    pub step_up: bool,
}

impl RatioReading {
    /// A signed ratio, first per second, read. Never zero on a path: one
    /// whose either end stands still is no path (`paths_of`).
    pub fn of(ratio: f64) -> Self {
        if ratio.abs() >= 1.0 {
            Self {
                turns: ratio,
                step_up: false,
            }
        } else {
            Self {
                turns: 1.0 / ratio,
                step_up: true,
            }
        }
    }
}

/// **One path of the train**: what it comes to between two of its open
/// bodies, with nothing loaded anywhere else — the three questions a stage
/// once answered of itself, asked of the whole graph.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PathReport {
    pub from: usize,
    pub to: usize,
    /// Turns of `from` per turn of `to`, signed, off the one motion. **Not
    /// sent**: the harness's and the preview's; the panel shows [`Self::reads`].
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub ratio: f64,
    /// The same ratio as it is read, never under one on either side.
    pub reads: RatioReading,
    /// Driving `from` with `to` holding the load and every other open body
    /// free, and the reverse — the train's flow at unit load, so a path
    /// that crosses one preset of three is rated on that preset alone. A path
    /// through a self-locking mesh cannot be back-driven at all, and
    /// [`Directional::locked`] on this pair says so. **Whether it breaks
    /// away** is the same flow against every mesh's static friction — each
    /// mesh's efficiency at rest, off its cut ([`Directional::once_moving`]):
    /// a path that cannot start delivers nothing, and one that can runs on
    /// sliding friction. **`None` where the flow was not solved** — a load
    /// held at both ends, or a loop of meshes, which the case's note names —
    /// so nought keeps its one meaning: the path locks.
    pub efficiency: Directional<Option<f64>>,
    /// Angular play at `to` driving from `from`, degrees, and at `from`
    /// driving from `to`: every mesh's play through the kinematics' own
    /// coefficients, so a mesh the path does not cross adds nothing. `None`
    /// where a mesh's play has no single reading at the path's end — a mesh
    /// in a loop, whose play the loop's other branch takes up.
    pub backlash: Directional<Option<Backlash>>,
    /// **The power crossing the teeth, over the power in**, each way — the
    /// sum over the meshes the path loads of what each passes: one across a
    /// pair, under one where a carrier takes part of it bodily, many times
    /// one where power circulates, which is where such a path's efficiency
    /// goes ([`flow::Flow::mesh_powers`]). An offset coupling passes power
    /// and has no teeth, so it counts for nothing here. `None` where the flow
    /// was not solved, as for [`Self::efficiency`].
    pub circulation: Directional<Option<f64>>,
    /// **The ratio one more tooth on each gear would give**, each gear by
    /// the graph's index for it — the graph's exact answer at `z_i + 1`, which is
    /// what a designer choosing counts wants beside the ratio: where a tooth
    /// moves it a lot, and where not at all. `None` where that one tooth
    /// leaves the path no motion or locks it. **Not sent**: the harness
    /// records it; the panel shows none of it.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub per_tooth: Vec<Option<f64>>,
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
    /// **The train's own figures, one row per path**, where its holds leave
    /// it one motion: every path an enabled case uses, from each of its
    /// loads to each of its reactions, once each and a direction each, in
    /// case order — the headline case's first. Empty where the motion is
    /// a family (a differential, a part joined to nothing), which is still
    /// rated: each case's loads decide its motion, and every part rates under
    /// that; what a family has none of is a figure read under one motion.
    pub paths: Vec<PathReport>,
    /// Every load case, in the train's order — the ones switched off too,
    /// solved at the train level alone so a panel can say whether one
    /// could be switched on; no part rates a case that is off.
    pub cases: Vec<TrainCase>,
    /// **Every gear's result**, in the graph's order ([`Shape::members`]).
    pub members: Vec<GearResult>,
    /// Every mesh's report, in the graph's order.
    pub meshes: Vec<MeshReport>,
    /// **Every axis distance's report**, in the graph's order, the mesh
    /// that held one open named by the graph's number for it — `None` for a
    /// distance nothing meshes across, which has nothing to report.
    pub distances: Vec<Option<shape::DistanceReport>>,
    /// Every axis's report, in the graph's order.
    pub axes: Vec<AxisReport>,
    /// **What is a part's own**, one per part in the order the parts are
    /// dealt ([`Train::parts`]).
    pub parts: Vec<PartReport>,
}

/// **One axis's report**: how its planets lay out, where it is replicated.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct AxisReport {
    /// The layout, its `axis` the graph's number for it.
    pub layout: Option<shape::LayoutReport>,
    /// **The counts the axis may be replicated to** ([`shape::Axis::count`]),
    /// [`crate::auto::COUNT`]. Sent so the panel holds the box to the core's
    /// bound rather than one written beside it.
    pub count: Bound,
}

/// **What is a part's own** — what its search and its closures came to, and
/// the speed and the torque its meshes put on each of its bodies in each
/// case.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PartReport {
    /// Per case, per body of the part, ground first — **the harness's**:
    /// what the corpus prints part by part ([`TrainResult::by_part`]). The
    /// panel reads what a shaft's gears carry off each gear
    /// ([`GearCase::on_body`]), whichever parts the shaft lies between, so
    /// this stays on this side.
    #[cfg_attr(feature = "serde", serde(skip))]
    #[cfg_attr(feature = "typescript", ts(skip))]
    pub cases: Vec<shape::SlotCase>,
    pub notes: Vec<Note>,
}

impl TrainResult {
    /// **The train's headline figures** — the first path, which is the
    /// headline case's where it has one motion.
    #[must_use]
    pub fn total(&self) -> Option<&PathReport> {
        self.paths.first()
    }

    /// **The train's result, laid from its parts'**: each part's members,
    /// meshes, distances and replicated axes at their places in the graph,
    /// the indices they carry — the mesh that held a distance open, a
    /// layout's axis — renumbered to the graph's, beside its paths and cases.
    fn laid(
        shape: &Shape,
        parts: &[graph::Part],
        results: &[ShapeResult],
        paths: Vec<PathReport>,
        cases: Vec<TrainCase>,
    ) -> Self {
        let mut members: Vec<Option<GearResult>> = vec![None; shape.members.len()];
        let mut meshes: Vec<Option<MeshReport>> = vec![None; shape.meshes.len()];
        let mut distances: Vec<Option<shape::DistanceReport>> = vec![None; shape.distances.len()];
        let mut axes: Vec<AxisReport> = (0..shape.axes.len())
            .map(|_| AxisReport {
                layout: None,
                count: crate::auto::COUNT,
            })
            .collect();
        let mut own = Vec::with_capacity(parts.len());
        for (part, r) in parts.iter().zip(results) {
            for (&i, g) in part.members.iter().zip(&r.members) {
                members[i] = Some(g.clone());
            }
            for (&k, m) in part.meshes.iter().zip(&r.meshes) {
                meshes[k] = Some(m.clone());
            }
            for (&d, report) in part.distances.iter().zip(&r.distances) {
                distances[d] = Some(shape::DistanceReport {
                    sized_by: report.sized_by.and_then(|k| part.meshes.get(k).copied()),
                    ..report.clone()
                });
            }
            for l in &r.layouts {
                if let Some(&a) = part.axes.get(l.axis) {
                    axes[a].layout = Some(shape::LayoutReport {
                        axis: a,
                        ..l.clone()
                    });
                }
            }
            own.push(PartReport {
                cases: r.cases.clone(),
                notes: r.notes.clone(),
            });
        }
        Self {
            paths,
            cases,
            members: members.into_iter().flatten().collect(),
            meshes: meshes.into_iter().flatten().collect(),
            distances,
            axes,
            parts: own,
        }
    }

    /// **A part's view of the train's result** — the `k`th part's members,
    /// meshes, distances and layouts in its own numbering, and what is its
    /// own: the result it solved to, which is what the harness prints
    /// part by part. A law holds it to the part's own solve, field for
    /// field.
    #[must_use]
    pub fn part(&self, part: &graph::Part, k: usize) -> ShapeResult {
        let own = self.parts.get(k);
        ShapeResult {
            members: part
                .members
                .iter()
                .filter_map(|&i| self.members.get(i).cloned())
                .collect(),
            meshes: part
                .meshes
                .iter()
                .filter_map(|&m| self.meshes.get(m).cloned())
                .collect(),
            distances: part
                .distances
                .iter()
                .filter_map(|&d| self.distances.get(d).cloned().flatten())
                .map(|d| shape::DistanceReport {
                    sized_by: d
                        .sized_by
                        .and_then(|k| part.meshes.iter().position(|&m| m == k)),
                    ..d
                })
                .collect(),
            layouts: part
                .axes
                .iter()
                .enumerate()
                .filter_map(|(j, &a)| {
                    self.axes
                        .get(a)
                        .and_then(|x| x.layout.clone())
                        .map(|l| shape::LayoutReport { axis: j, ..l })
                })
                .collect(),
            cases: own.map(|o| o.cases.clone()).unwrap_or_default(),
            notes: own.map(|o| o.notes.clone()).unwrap_or_default(),
        }
    }

    /// **What an input of the train's came to** — the figure a box relief
    /// turns given is seeded from, a freedom named by the graph's indices
    /// ([`Shape::toggles`] on the train's shape).
    #[must_use]
    pub fn figure(&self, f: Freedom) -> Option<f64> {
        match f {
            Freedom::Distance(d) => self.distances.get(d).cloned().flatten().map(|d| d.running),
            Freedom::Clearance(d) => self
                .distances
                .get(d)
                .cloned()
                .flatten()
                .map(|d| d.clearance),
            Freedom::Overlap(k) => self
                .meshes
                .get(k)
                .and_then(|m| m.line.as_ref().map(|l| l.contact_ratios.overlap)),
            Freedom::Member(i, m) => self.members.get(i).map(|g| m.of(g)),
        }
    }

    /// **The result part by part** ([`Self::part`]), each in its own
    /// numbering, in the order the train's parts are dealt — what the
    /// harness prints part by part.
    #[must_use]
    pub fn by_part(&self, train: &Train) -> Vec<ShapeResult> {
        train
            .parts()
            .iter()
            .enumerate()
            .map(|(k, p)| self.part(p, k))
            .collect()
    }
}

/// **The train solved, case by case** — the one place a load case is
/// solved.
///
/// # The model
///
/// A case is a set of entries on the train's open ports — a load, a
/// reaction, or free — and a port it does not mention is free. That leaves
/// the train some mobility, and the loads'
/// given speeds decide the motion: each given speed drives its port at one
/// turn with every other given port still, the family of speeds is that
/// solution scaled and summed, and a case with fewer given speeds than
/// the mobility has no motion to rate under and says so. The loads' given
/// torques then decide the flow — one flow across every mesh at once
/// ([`flow::solve`]), with the given torques known and the derived loads,
/// the reacted ports, the held bodies and ground unknown — and each part is
/// rated under what that flow puts on its meshes and bodies ([`CaseLoad`]). A given
/// torque is a load whichever way it works: one that works with its port's
/// speed drives, one that works against it is driven — a brake, a load the
/// designer stated at the output — and what drives it is among the
/// unknowns, a derived load or a reacted end. A case in which no given
/// torque does any work has nothing to follow and says so.
///
/// A load nothing holds — a torque at the end with the start free and
/// carrying nothing — has no flow unless a mesh locks in that direction:
/// the train turns under it, every torque is nought, and the case says so.
/// One with a locked mesh is held there, the locked mesh's driver pressing
/// its flanks and nothing beyond it seeing any.
///
/// Speeds decide only the *direction* of the flow; a load held still takes
/// its direction from the sign of its torque, so a stall case rates as one
/// turning the way it pushes.
///
/// # Errors
///
/// A hold that locks a part, a case entry at a body no load can be put on,
/// a ratio too large to hold exactly, and whatever a part refuses as it is
/// cut or rated. An empty train is no error: it solves to nothing, its
/// cases waiting.
pub fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<TrainResult, TrainError> {
    entering(train, lib)?;
    solve_parts(train, &train.parts(), lib).map(|(r, _)| r)
}

/// **Why a case cannot be read**, where it cannot: an entry at a body
/// that is not one of the train's open `ports` ([`TrainError::LoadPort`]),
/// two entries at one body ([`TrainError::DuplicateEntry`]), or a fatigue
/// sweep with entries to rate measured at a body that is not an open port
/// ([`TrainError::DutyPort`]). An ultimate case's duty is kept for switching
/// back and read by nothing, so nothing asks where it is measured.
fn case_refusal(index: usize, case: &LoadCase, ports: &[Body]) -> Option<TrainError> {
    if case.loads.iter().any(|l| !ports.contains(&l.at)) {
        return Some(TrainError::LoadPort { case: index });
    }
    let repeat =
        (1..case.loads.len()).find(|&i| case.loads[..i].iter().any(|x| x.at == case.loads[i].at));
    if let Some(i) = repeat {
        return Some(TrainError::DuplicateEntry {
            case: index,
            body: case.loads[i].at,
        });
    }
    match case.counted() {
        Some(&Duty::Intermittent { at: Some(body), .. })
            if !case.loads.is_empty() && !ports.contains(&body) =>
        {
            Some(TrainError::DutyPort { case: index, body })
        }
        _ => None,
    }
}

/// **A train and its library as a solve takes them** — the train's graph
/// and every number in it ([`Train::validate`]), the library's every figure
/// ([`MaterialLibrary::check`]), and each member's material as it uses it,
/// its replacements laid over the library's ([`crate::input::resolved`]).
/// A material the library does not have is the part's to name.
fn entering(train: &Train, lib: &MaterialLibrary) -> Result<(), TrainError> {
    train.validate()?;
    lib.check().map_err(TrainError::Input)?;
    for (i, m) in train.shape.members.iter().enumerate() {
        if let Some(material) = lib.get(&m.gear.material) {
            crate::input::resolved(
                material,
                &m.gear.material_overrides,
                &format!("shape.members.{i}.gear.material_overrides"),
            )
            .map_err(TrainError::Input)?;
        }
    }
    Ok(())
}

/// **The train solved part by part** — each of `parts` closed, sized,
/// searched and rated on its own, one flow and one motion across them all.
/// The train's own parts ([`Train::parts`]) — a lone preset being one
/// ([`solve_alone`]). The train's result, and each part's own that it was
/// laid from.
fn solve_parts(
    train: &Train,
    parts: &[graph::Part],
    lib: &MaterialLibrary,
) -> Result<(TrainResult, Vec<ShapeResult>), TrainError> {
    // **An empty train is a train**: nothing to rate, no figure of its
    // own, and every case reported unsolved with nothing to say of it — the
    // train's own state says why — so a designer who removes the last gear
    // keeps the cases and sees them wait.
    if parts.is_empty() {
        return Ok(TrainResult {
            paths: Vec::new(),
            cases: train
                .load_cases
                .iter()
                .enumerate()
                .map(|(index, _)| TrainCase {
                    case: index,
                    bodies: Vec::new(),
                    solved: false,
                    notes: Vec::new(),
                })
                .collect(),
            members: Vec::new(),
            meshes: Vec::new(),
            distances: Vec::new(),
            axes: Vec::new(),
            parts: Vec::new(),
        })
        .map(|r| (r, Vec::new()));
    }

    // Whether to correct for a root loaded on both flanks is one switch for the
    // whole train rather than a decision taken per part.
    let reversal = Reversal {
        correct: train.reversed_bending,
    };

    // **The train's motion is asked first**, for what refuses before any
    // geometry: a hold that locks a part, a body no part has, a ratio no
    // exact number holds. What it comes to per path is read below, and a
    // family is rated under its cases: each case's loads decide its own
    // motion, and what a family has none of is a figure read under one.
    train.motion_of(parts)?;
    let port = |k: usize, slot: Body| parts[k].shape.body_at(slot);

    // **Each part cut once**: its shifts searched, its members cut and its
    // meshes at their running distances — the geometry, which no load case
    // moves, and each mesh's efficiency with it, which the flow is written
    // from. What the flow hands each part is rated below.
    let in_part = |k: usize| {
        move |e: TrainError| TrainError::InPart {
            part: k,
            cause: Box::new(e),
        }
    };
    let cuts: Vec<shape::Cut> = parts
        .iter()
        .enumerate()
        .map(|(k, part)| shape::cut(&part.shape, lib).map_err(in_part(k)))
        .collect::<Result<_, _>>()?;

    // ---- the train's graph, with every part's meshes on it.
    let system = train.system_counting(parts, |_, _, z| z)?;
    let shafts = system.bodies();
    let wirings: Vec<Wiring> = parts.iter().map(|p| p.shape.wiring()).collect();
    // Each part's meshes as the flow sees them, in one list — sliding, and
    // locked where it locks at rest — and the same list against static
    // friction, which is what a path's breaking away is asked of.
    let mut meshes: Vec<flow::MeshFlow> = Vec::new();
    let mut resting: Vec<flow::MeshFlow> = Vec::new();
    let mut mesh_of_part: Vec<Vec<usize>> = Vec::new();
    for (k, w) in wirings.iter().enumerate() {
        let mut mine = Vec::new();
        for (j, m) in w.meshes.iter().enumerate() {
            mine.push(meshes.len());
            let [efficiency, at_rest] = cuts[k].efficiency(j);
            let mesh = flow::MeshFlow {
                a: port(k, w.mounts[m.a].spins_with),
                b: port(k, w.mounts[m.b].spins_with),
                frame: w.frame(j).map_or(GROUND, |f| port(k, f)),
                za: f64::from(cuts[k].teeth(m.a)),
                zb: m.kind.sign() * f64::from(cuts[k].teeth(m.b)),
                efficiency,
                paths: f64::from(m.paths),
            };
            resting.push(flow::MeshFlow {
                efficiency: at_rest,
                ..mesh
            });
            meshes.push(mesh);
        }
        mesh_of_part.push(mine);
    }
    // **Every offset coupling after every mesh**: a way through the flow
    // that loses nothing either way — a mesh of one tooth and minus one,
    // so the frame it would stand in carries nothing — and no teeth, so
    // what a path says passes its teeth does not count it. A part's own
    // first, part by part, and then each between two parts' bodies — a
    // join that could not be coaxial — which no part rates.
    let with_teeth = meshes.len();
    let coupling = |a: Body, b: Body| flow::MeshFlow {
        a,
        b,
        frame: GROUND,
        za: 1.0,
        zb: -1.0,
        efficiency: Directional::of(|_| 1.0),
        paths: 1.0,
    };
    let mut coupling_of_part: Vec<Vec<usize>> = Vec::new();
    for (k, w) in wirings.iter().enumerate() {
        let mut mine = Vec::new();
        for &[a, b] in &w.couplings {
            mine.push(meshes.len());
            meshes.push(coupling(port(k, a), port(k, b)));
            resting.push(coupling(port(k, a), port(k, b)));
        }
        coupling_of_part.push(mine);
    }
    for (c, &[a, b]) in train.shape.couplings.iter().enumerate() {
        if !parts.iter().any(|p| p.couplings.contains(&c)) {
            meshes.push(coupling(a, b));
            resting.push(coupling(a, b));
        }
    }
    // What the train itself holds, before any case: ground, and every body
    // it holds. **A body is one thing to the flow** as to the kinematics:
    // the torque balance is on it, and what a mesh puts on either of a
    // body's ends is put on the body.
    let conditions = train.conditions(shafts)?;

    // --- what each part is loaded by, case by case. **Every case is
    // solved at the train level**, switched off or not, so a panel can say
    // of a case that is off whether it could be switched on; only a case
    // that is on reaches a part's rating.
    let mut cases = Vec::new();
    let mut per_part: Vec<Vec<CaseLoad>> = vec![Vec::new(); parts.len()];
    for (index, case) in train.load_cases.iter().enumerate() {
        let mut notes = Vec::new();
        // Every entry by the body it names, whatever it declares; the
        // loads among them are what carry figures.
        let entries: Vec<(Body, &Load)> = case.loads.iter().map(|l| (l.at, l)).collect();
        let loads: Vec<(Body, &Load)> = entries
            .iter()
            .filter(|(_, l)| l.is_load())
            .copied()
            .collect();
        // An entry at a body that is not a port — a planet's, a held
        // ring's — is refused by name. A body two parts share is a port
        // of both, and a load there is a load on the body they make.
        let mut ports: Vec<Body> = Vec::new();
        for (k, part) in parts.iter().enumerate() {
            for s in part.shape.ports().ports {
                if !train.held.contains(&port(k, s)) {
                    ports.push(port(k, s));
                }
            }
        }
        let loaded = |s: Body| loads.iter().any(|(g, _)| *g == s);
        let declared =
            |s: Body, role: LoadRole| entries.iter().any(|(g, l)| *g == s && l.role == role);
        // **A port is what the case declares it, and free where it says
        // nothing.** A reacted port turns as the motion says and carries
        // whatever torque the flow puts on it — the same thing as a body
        // the train holds by constraint, except that a fixed body is ground
        // and a reacted one may turn; a free port turns and carries nothing,
        // since a reaction is a thing a designer attaches, and says so by
        // declaring it. Nothing is driven but by a load: the train's own
        // motion, driven at one end for its ratio, is nobody's here.
        let reacted: Vec<Body> = ports
            .iter()
            .copied()
            .filter(|&s| declared(s, LoadRole::Reacted))
            .collect();
        // Roles, before anything is solved: what each body is in this case.
        let role = |s: Body| -> BodyRole {
            if loaded(s) {
                BodyRole::Load
            } else if reacted.contains(&s) {
                BodyRole::Reacted
            } else if conditions[s] == Condition::Ground {
                BodyRole::Fixed
            } else {
                BodyRole::Free
            }
        };
        let nothing =
            |notes: Vec<Note>, cases: &mut Vec<TrainCase>, per_part: &mut [Vec<CaseLoad>]| {
                cases.push(TrainCase {
                    case: index,
                    bodies: (0..shafts)
                        .map(|s| CaseBody {
                            at: s,
                            role: role(s),
                            speed: (conditions[s] != Condition::Ground).then_some(0.0),
                            torque: 0.0,
                        })
                        .collect(),
                    solved: false,
                    notes,
                });
                if !case.enabled {
                    return;
                }
                for (k, w) in wirings.iter().enumerate() {
                    per_part[k].push(CaseLoad::nothing(index, case.kind, w));
                }
            };
        // **What a case names, read before anything it says**: an entry at
        // a body no load can enter by, two entries at one body, and a sweep
        // at no open port. A case that is on refuses the train by name; one
        // that is off is unsolved, the refusal its note, and the train
        // solves without it.
        if let Some(e) = case_refusal(index, case, &ports) {
            if case.enabled {
                return Err(e);
            }
            notes.push(crate::note::Explain::note(&e));
            nothing(notes, &mut cases, &mut per_part);
            continue;
        }
        if loads.is_empty() {
            notes.push(Note::new(key::TRAIN_CASE_NO_LOAD));
            nothing(notes, &mut cases, &mut per_part);
            continue;
        }
        // ---- the motion: each given speed drives its port at one turn with
        // every other given port still, and the case's speeds are those
        // solutions scaled and summed. **A port held still has an impending
        // motion** — its own turn, the other given ports still — signed so
        // the given torques do positive work on it, and nought where they
        // do none; a derived torque, whose box holds only a seed, has no
        // say. The flow reads it only to break a tie the speeds leave.
        let given_speeds: Vec<(Body, f64)> = loads
            .iter()
            .filter(|(_, l)| !l.speed.auto)
            .map(|(g, l)| (*g, l.speed.manual))
            .collect();
        let given_torque = |b: Body| -> f64 {
            loads
                .iter()
                .filter(|(g, l)| *g == b && !l.torque.auto)
                .map(|(_, l)| l.torque.manual)
                .sum()
        };
        // **Every given port driven at its own speed, exactly, judged by
        // rank**: the holds first, then the given ports in the case's order.
        // A speed the ones before it decide is one condition said twice, and
        // passes where it agrees with them to the precision it was given
        // in — its own ulp and, through the exact ratio, theirs — and is
        // refused by name where it does not. One the ones before leave free
        // drives the motion.
        let motion = match given_motion(&system, &conditions, &given_speeds) {
            Ok(m) => m,
            Err(note) => {
                notes.push(note);
                nothing(notes, &mut cases, &mut per_part);
                continue;
            }
        };
        let speeds: Vec<f64> = motion.iter().map(|r| r.to_f64()).collect();
        // **A port held still has an impending motion** — its own turn, the
        // other given ports still — signed so the given torques do positive
        // work on it, and nought where they do none or where the other given
        // ports leave it none. Exact, as the speeds are, so a sweep read off
        // it counts exactly.
        let mut still_exact = vec![crate::ratio::Ratio::ZERO; shafts];
        for &(driver, _) in given_speeds.iter().filter(|(_, s)| *s == 0.0) {
            let mut c = conditions.clone();
            for &(other, _) in &given_speeds {
                c[other] = Condition::Ground;
            }
            c[driver] = Condition::Drive(crate::ratio::Ratio::ONE);
            let Ok(sol) = system.motion_in(&c, &[driver]) else {
                continue;
            };
            if !sol.is_unique() {
                continue;
            }
            let work: f64 = (0..shafts)
                .map(|b| given_torque(b) * sol.values[b].to_f64())
                .sum();
            if work == 0.0 {
                continue;
            }
            for (i, v) in sol.values.iter().enumerate() {
                let signed = if work > 0.0 {
                    Some(*v)
                } else {
                    v.checked_neg()
                };
                if let Some(sum) = signed.and_then(|v| still_exact[i].checked_add(v)) {
                    still_exact[i] = sum;
                }
            }
        }
        let still: Vec<f64> = still_exact.iter().map(|r| r.to_f64()).collect();
        // ---- the flow: the given torques known; the derived loads, the
        // reacted ports, the fixed bodies and ground to be found.
        let mut known: Vec<Option<f64>> = vec![Some(0.0); shafts];
        let mut unknown: Vec<Body> = vec![GROUND];
        for s in 0..shafts {
            if conditions[s] == Condition::Ground || reacted.contains(&s) {
                known[s] = None;
                if s != GROUND {
                    unknown.push(s);
                }
            }
        }
        for (g, l) in &loads {
            if l.torque.auto {
                known[*g] = None;
                if !unknown.contains(g) {
                    unknown.push(*g);
                }
            } else {
                known[*g] = Some(l.torque.manual);
            }
        }
        let asked = flow::Asked { known, unknown };
        let flow = match flow::solve_tied(shafts, &meshes, &speeds, &still, &asked) {
            Ok(flow) => flow,
            Err(flow::Refused::NothingDrives) => {
                notes.push(Note::new(key::TRAIN_CASE_NOTHING_DRIVES));
                nothing(notes, &mut cases, &mut per_part);
                continue;
            }
            // **Two ends that could both hold the same load** are a division
            // by stiffness this model does not make: a load with two ways
            // out goes the way that holds it — a mesh that locks in that
            // direction, with nothing holding the other end — and one held
            // at both is a question the case says it cannot answer, rather
            // than an end chosen for the designer. The boundary of the
            // model, not of the mechanism, and the case's to say.
            //
            // **A loop of meshes is the other way to it**: twin countershafts,
            // or a pair doubled, carry a torque round that reaches no body,
            // and how they share is the same division by stiffness, with no
            // hold to blame. Told apart on the lossless rows, exactly: where
            // the known torques leave no port's torque free, what is free is
            // the loop's.
            Err(flow::Refused::Undetermined) => {
                notes.push(undetermined_note(parts, &system, &asked, &mesh_of_part));
                nothing(notes, &mut cases, &mut per_part);
                continue;
            }
            // Nothing holds what drives: the train turns under it.
            Err(flow::Refused::Inconsistent) => {
                notes.push(Note::new(key::TRAIN_LOAD_NOT_REACTED));
                nothing(notes, &mut cases, &mut per_part);
                continue;
            }
            // More meshes than the enumeration can count: said as that,
            // never as a load the train cannot react.
            Err(flow::Refused::TooManyMeshes { meshes }) => {
                notes.push(
                    Note::new(key::TRAIN_FLOW_TOO_MANY_MESHES)
                        .tally("meshes", meshes)
                        .tally("most", flow::MOST_MESHES),
                );
                nothing(notes, &mut cases, &mut per_part);
                continue;
            }
        };
        // ---- how many times each body comes round over a fatigue duty —
        // counted over nothing where the sweep is unset, or measured at a
        // body that neither turns in this case nor would, which the case
        // says rather than counting nought.
        let turns: Option<Vec<Revolutions>> = match case.counted() {
            None => None,
            Some(&Duty::Continuous { runtime_hours }) => {
                Some(continuous_turns(&motion, runtime_hours))
            }
            Some(&Duty::Intermittent { at: None, .. }) => {
                notes.push(Note::new(key::TRAIN_DUTY_UNSET));
                None
            }
            Some(&Duty::Intermittent {
                range_degrees,
                at: Some(port),
                actuations,
                ..
            }) => {
                // **A sweep is a magnitude**, stated at a port; every body's
                // share of it is the ratio of the two speeds, which the
                // impending motion has where a held load's speed does not.
                // The turns are **signed** here, so a member's turns against
                // its carrier are a difference of two, taken as a magnitude
                // where they are counted.
                let per = if motion[port].is_zero() {
                    &still_exact
                } else {
                    &motion
                };
                if per[port].is_zero() {
                    notes.push(located(key::TRAIN_DUTY_AT_STILL, port));
                    None
                } else {
                    Some(swept_turns(per, port, range_degrees, actuations))
                }
            }
        };
        let reversing_actuations = case.counted().and_then(|d| match *d {
            Duty::Intermittent {
                actuations,
                reversing: true,
                ..
            } => Some(actuations),
            _ => None,
        });
        // ---- what the rating of this case leaves out, said where it is
        // rated: the stresses are nominal, under `K_A` and no other factor.
        if case.enabled {
            if case.applied_factor() != case.application_factor {
                notes.push(Note::new(key::TRAIN_APPLICATION_FACTOR_HELD).number(
                    "given",
                    case.application_factor,
                    2,
                ));
            }
            notes.push(Note::new(key::TRAIN_STRESSES_NOMINAL).number(
                "factor",
                case.applied_factor(),
                2,
            ));
        }
        // ---- what each part's meshes put on each of its bodies.
        for (k, w) in wirings.iter().enumerate().filter(|_| case.enabled) {
            let mine = &mesh_of_part[k];
            let mut torques = vec![0.0; w.slots.len()];
            let mut on_members = vec![0.0; w.mounts.len()];
            for (j, &g) in mine.iter().enumerate() {
                let [on_a, on_b, on_frame] = flow.on_shafts(g, &meshes[g]);
                let m = &w.meshes[j];
                torques[w.mounts[m.a].spins_with] += on_a;
                torques[w.mounts[m.b].spins_with] += on_b;
                torques[w.frame(j).unwrap_or(GROUND)] += on_frame;
                on_members[m.a] += on_a;
                on_members[m.b] += on_b;
            }
            for (&[a, b], &g) in w.couplings.iter().zip(&coupling_of_part[k]) {
                let [on_a, on_b, _] = flow.on_shafts(g, &meshes[g]);
                torques[a] += on_a;
                torques[b] += on_b;
            }
            per_part[k].push(CaseLoad {
                case: index,
                kind: case.kind,
                mesh_torques: mine.iter().map(|&g| flow.mesh_torques[g]).collect(),
                directions: mine.iter().map(|&g| flow.directions[g]).collect(),
                mesh_powers: mine.iter().map(|&g| flow.mesh_powers[g]).collect(),
                speeds: (0..w.slots.len()).map(|l| speeds[port(k, l)]).collect(),
                torques,
                on_members,
                turns: turns
                    .as_ref()
                    .map(|t| (0..w.slots.len()).map(|l| t[port(k, l)]).collect()),
                reversing_actuations,
                application_factor: case.applied_factor(),
            });
        }
        cases.push(TrainCase {
            case: index,
            bodies: (0..shafts)
                .map(|s| CaseBody {
                    at: s,
                    role: role(s),
                    speed: (conditions[s] != Condition::Ground).then_some(speeds[s]),
                    // A body's external torque is reported on the body it
                    // is applied at — the loaded or reacted one — and a
                    // body coupled to it carries the coupling, not a load.
                    // A free body carries nothing by declaration, and says
                    // exactly nought rather than the `−1e-17` the solve
                    // leaves on a body it was told carries none.
                    torque: if role(s) == BodyRole::Free {
                        0.0
                    } else {
                        flow.shaft_torques[s]
                    },
                })
                .collect(),
            solved: true,
            notes,
        });
    }
    // ---- each part rated under what the flow handed it.
    let rated: Vec<ShapeResult> = cuts
        .iter()
        .zip(&per_part)
        .enumerate()
        .map(|(k, (cut, loads))| shape::rate(cut, loads, reversal).map_err(in_part(k)))
        .collect::<Result<_, _>>()?;

    // **The train's own figures are per path**: between every two open
    // bodies, where the train's holds leave it one motion. Each is the same
    // three questions a stage once answered of itself, asked of the whole graph
    // at once — the ratio off the one motion, the efficiency off the flow
    // at unit load with everything else free, the play off the kinematics'
    // own coefficients so a mesh a path does not cross adds nothing — and
    // none of them assumes a chain, a list order, or a head.
    let paths = paths_of(
        train,
        parts,
        &system,
        [&meshes, &resting],
        with_teeth,
        &conditions,
        &rated,
    );

    Ok((
        TrainResult::laid(&train.shape, parts, &rated, paths, cases),
        rated,
    ))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::arrangements as arr;
    use super::*;
    use crate::train::testing::{shapes, try_alone, try_alone_at};

    fn library() -> MaterialLibrary {
        super::test_library()
    }

    /// **A train's result, and each part's view of it** — what these tests
    /// read part by part, read through [`TrainResult::part`] so every one
    /// of them reads the view the harness does; the law
    /// `a_part_is_its_own_solve_laid_out` holds the view to the part's own
    /// solve.
    #[derive(Debug)]
    pub(super) struct Solved {
        pub result: TrainResult,
        pub by_part: Vec<ShapeResult>,
    }

    impl std::ops::Deref for Solved {
        type Target = TrainResult;
        fn deref(&self) -> &TrainResult {
            &self.result
        }
    }

    pub(super) fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<Solved, TrainError> {
        let result = super::solve_train(train, lib)?;
        let by_part = result.by_part(train);
        Ok(Solved { result, by_part })
    }

    /// **A shaft's gears carry its own load between them.** What each gear's
    /// meshes put on its body ([`GearCase::on_body`]) sums to the body's own
    /// torque in the case ([`CaseBody::torque`]) on every body that carries
    /// no axis and turns with no coupling — the shaft's statics, whichever
    /// parts it lies between: the shaft between two presets hands
    /// one gear's torque to the next, and a layshaft's countershaft does the
    /// same inside one part. On every preset alone and after a pair and a
    /// layshaft, in every case that solved.
    #[test]
    fn a_shafts_gears_carry_its_own_load_between_them() {
        let lib = library();
        let mut bodies = 0;
        for a in arr::Preset::ALL {
            for b in [None, Some(arr::Preset::Spur), Some(arr::Preset::Layshaft)] {
                let presets: Vec<Shape> = b
                    .into_iter()
                    .map(arr::Preset::build)
                    .chain(std::iter::once(a.build()))
                    .collect();
                let t = Train::chained(presets, |t| {
                    t.chain_ends()
                        .map(|(x, y)| {
                            vec![
                                LoadCase::ultimate(x, y, 1.0, 1000.0),
                                LoadCase::back_driving(x, y, 0.6),
                            ]
                        })
                        .unwrap_or_default()
                });
                let Ok(r) = solve_train(&t, &lib) else {
                    continue;
                };
                for c in r.cases.iter().filter(|c| c.solved) {
                    for body in &c.bodies {
                        let at = body.at;
                        if at == GROUND
                            || t.shape.axes.iter().any(|x| x.carried_by == at)
                            || t.shape.couplings.iter().any(|p| p.contains(&at))
                        {
                            continue;
                        }
                        let on: Vec<f64> = t
                            .shape
                            .members
                            .iter()
                            .enumerate()
                            .filter(|(_, m)| m.body == at)
                            .map(|(i, _)| {
                                r.members[i]
                                    .cases
                                    .iter()
                                    .find(|g| g.case == c.case)
                                    .map_or(0.0, |g| g.on_body)
                            })
                            .collect();
                        let scale = on.iter().fold(body.torque.abs(), |m, x| m.max(x.abs()));
                        let sum: f64 = on.iter().sum::<f64>() - body.torque;
                        assert!(
                            sum.abs() <= 1e-9 * (1.0 + scale),
                            "{a:?} after {b:?}, case {}, body {at}: gears {on:?} and the body's {} do not balance",
                            c.case,
                            body.torque
                        );
                        bodies += 1;
                    }
                }
            }
        }
        assert!(bodies > 200, "only {bodies} bodies balanced");
    }

    /// **A path read either way is one figure, the one on the other side.**
    /// Every path's reading is its ratio, or the ratio's reciprocal with the
    /// one first, and never a figure under one; and a case loaded at the
    /// output walks the forward path the other way round, so its row reads
    /// the forward row's figure as a step-up where the forward row reads a
    /// reduction. On every preset alone.
    #[test]
    fn a_path_backwards_reads_its_figure_the_other_way_round() {
        let lib = library();
        for p in arr::Preset::ALL {
            let t = Train::chained(vec![p.build()], |t| {
                t.chain_ends()
                    .map(|(x, y)| {
                        vec![
                            LoadCase::ultimate(x, y, 1.0, 1000.0),
                            LoadCase::back_driving(x, y, 0.6),
                        ]
                    })
                    .unwrap_or_default()
            });
            let r = solve_train(&t, &lib).unwrap();
            for path in &r.paths {
                let read = path.reads;
                assert!(read.turns.abs() >= 1.0, "{p:?}: {read:?}");
                let back = if read.step_up {
                    1.0 / read.turns
                } else {
                    read.turns
                };
                assert!(
                    (back - path.ratio).abs() <= 1e-12 * path.ratio.abs(),
                    "{p:?}"
                );
            }
            let (x, y) = t.chain_ends().unwrap();
            let row = |a, b| r.paths.iter().find(|q| (q.from, q.to) == (a, b));
            let (there, back) = (row(x, y).unwrap(), row(y, x).unwrap());
            let (there, back) = (there.reads, back.reads);
            assert!(
                (there.turns - back.turns).abs() <= 1e-12 * there.turns.abs(),
                "{p:?}: {there:?} and {back:?}"
            );
            assert!(
                there.turns.abs() == 1.0 || there.step_up != back.step_up,
                "{p:?}: {there:?} and {back:?}"
            );
        }
    }

    /// **A part's view is its own solve**: the view the train's result gives
    /// of each part is the result the part solved to, field for field — on
    /// every preset alone and after every other, where a part's pieces are
    /// not the graph's by the same index.
    #[test]
    fn a_part_is_its_own_solve_laid_out() {
        let lib = library();
        for a in arr::Preset::ALL {
            for b in [None, Some(arr::Preset::Spur), Some(arr::Preset::Planetary)] {
                let presets: Vec<Shape> = b
                    .into_iter()
                    .map(arr::Preset::build)
                    .chain(std::iter::once(a.build()))
                    .collect();
                let t = Train::chained(presets, |t| {
                    t.chain_ends()
                        .map(|(x, y)| vec![LoadCase::ultimate(x, y, 1.0, 1000.0)])
                        .unwrap_or_default()
                });
                let parts = t.parts();
                let Ok((r, own)) = solve_parts(&t, &parts, &lib) else {
                    continue;
                };
                assert_eq!(own.len(), parts.len());
                for (k, p) in parts.iter().enumerate() {
                    assert_eq!(
                        format!("{:?}", r.part(p, k)),
                        format!("{:?}", own[k]),
                        "{a:?} after {b:?}: part {k}"
                    );
                }
                assert_eq!(r.members.len(), t.shape.members.len());
                assert_eq!(r.meshes.len(), t.shape.meshes.len());
                assert_eq!(r.distances.len(), t.shape.distances.len());
                assert!(r.distances.iter().all(Option::is_some));
            }
        }
    }

    /// These tests build pairs, and read a pair's result as the shape's:
    /// two members, one mesh, one distance.
    fn spur(r: &ShapeResult) -> &shape::ShapeResult {
        r
    }

    /// ...and a worm stage's result, with its point contact.
    fn worm(r: &ShapeResult) -> (&shape::ShapeResult, &MeshReport) {
        let p = spur(r);
        assert!(
            p.meshes[0].point.is_some(),
            "a worm stage's mesh is a point contact"
        );
        (p, &p.meshes[0])
    }

    /// **A one-mesh stage's reduction, in size** — its two tooth counts,
    /// which is what refers a torque or a count of turns across it.
    fn reduction(r: &ShapeResult) -> f64 {
        f64::from(r.members[1].params.teeth) / f64::from(r.members[0].params.teeth)
    }

    /// **Where a chain enters part `k` and where it leaves it**: a body an
    /// earlier part runs on to is its input and one a later part runs on
    /// from its output; its preset's convention says the rest, under the
    /// train's holds — what these tests measure a part's path between.
    pub(super) fn part_ends(train: &Train, k: usize) -> (usize, usize) {
        let parts = train.parts();
        let part = &parts[k];
        let ports = part.shape.ports();
        let body = |slot: Body| part.shape.body_at(slot);
        let shared = |slot: Body, earlier: bool| {
            train
                .ends_of(body(slot))
                .iter()
                .any(|&(x, _)| x != k && ((x < k) == earlier))
        };
        let held: Vec<Body> = ports
            .ports
            .iter()
            .copied()
            .filter(|&slot| train.held.contains(&body(slot)))
            .collect();
        let onward = ports
            .ports
            .iter()
            .copied()
            .find(|&slot| shared(slot, false));
        let entered = ports.ports.iter().copied().find(|&slot| shared(slot, true));
        let input = entered.unwrap_or_else(|| {
            let mut closed = held.clone();
            closed.extend(onward);
            ports.ends(&closed, None).0
        });
        let output = onward.unwrap_or_else(|| ports.ends(&held, Some(input)).1);
        (body(input), body(output))
    }

    /// **A preset asked one way**: what is held, where its case enters and
    /// where it is reacted — its slots, as a train of one numbers them.
    #[derive(Clone, Debug)]
    pub(super) struct Asked {
        pub held: Vec<Body>,
        pub input: Body,
        pub output: Body,
    }

    impl Asked {
        /// As its preset's conventions read it.
        pub(super) fn conventional(stage: &Shape) -> Self {
            let ports = stage.ports();
            Self {
                input: ports.input(),
                output: ports.output(),
                held: ports.held,
            }
        }

        /// A set, in its own words ([`Train::arranged_as`]).
        pub(super) fn set(stage: &Shape, arrangement: crate::planetary::Arrangement) -> Self {
            let t = Train::alone(stage, 1.0, 1.0).arranged_as(arrangement);
            let at = |role: LoadRole| {
                t.load_cases[0]
                    .loads
                    .iter()
                    .find(|l| l.role == role)
                    .unwrap()
                    .at
            };
            Self {
                input: at(LoadRole::Load),
                output: at(LoadRole::Reacted),
                held: t.held,
            }
        }

        /// What the wiring's system is asked: ground and the holds held, the
        /// input driven at one turn.
        pub(super) fn conditions(&self, slots: usize) -> Vec<crate::kinematics::Condition> {
            use crate::kinematics::Condition;
            let mut c = vec![Condition::Free; slots];
            c[GROUND] = Condition::Ground;
            for &h in &self.held {
                c[h] = Condition::Ground;
            }
            c[self.input] = Condition::Drive(crate::ratio::Ratio::ONE);
            c
        }

        /// A train of one, asked this way.
        pub(super) fn train(&self, stage: &Shape, torque: f64, speed: f64) -> Train {
            Train::alone(stage, torque, speed).arranged(&self.held, self.input, self.output)
        }
    }

    /// **Part `k`'s figures in this train**: the path across its ends,
    /// asked by a case on them under the train's holds — which is all a
    /// stage's own figures ever were.
    pub(super) fn across(train: &Train, k: usize, lib: &MaterialLibrary) -> PathReport {
        let (from, to) = part_ends(train, k);
        let mut t = train.clone();
        t.load_cases = vec![LoadCase::ultimate(from, to, 2.0, 3000.0)];
        super::solve_train(&t, lib)
            .unwrap()
            .paths
            .into_iter()
            .find(|p| (p.from, p.to) == (from, to))
            .unwrap()
    }

    /// The hula arrangement (`arrangements::hula`) at the shipped counts,
    /// as a stage: members the two wobble gears, the grounded gear, the
    /// output; bodies crank 1, grounded 2, output 3, wobble 4.
    fn hula() -> Shape {
        hula_shape([65, 61, 57, 61])
    }

    fn hula_shape(teeth: [u32; 4]) -> shape::Shape {
        arrangements::hula(teeth, [1.0, 1.0])
    }

    /// A hula shape under its own arrangement — crank driven, grounded gear
    /// held, output out — whatever its counts make the rings.
    fn solve_hula(
        shape: &shape::Shape,
        torque: f64,
        speed: f64,
        lib: &MaterialLibrary,
    ) -> Result<Alone, TrainError> {
        solve_alone(
            &Train::alone(shape, torque, speed).arranged(&[2], 1, 3),
            lib,
        )
    }

    /// **A back-driving load is the load, and the reverse is the forward
    /// construction with the roles swapped.**
    ///
    /// It enters at the output. The member *on* that body carries the applied
    /// torque itself — nothing has happened to it yet — and the member at the
    /// other end carries it **referred by the ratio**: a member's torque is
    /// the torque its teeth carry, the driver's read across the mesh, whether
    /// or not the mesh passes any of it on. What the far *body* delivers is
    /// that cut by the mesh's loss the backward way — nought where the mesh
    /// locks — and it is the body's figure, not the gear's. One
    /// construction, read from either end.
    ///
    /// Two things this has caught, in opposite directions. It once *divided* by
    /// the backward efficiency, and a worm's is **zero** whenever it self-locks
    /// — the case a worm is chosen for — so the wheel reported **2.2e307 N·m**:
    /// finite, so it crossed the boundary as a number rather than as the `null`
    /// an infinity becomes, and drew on screen as a figure. The correction
    /// dropped the factor altogether, which left the *worm* claiming a body
    /// torque a locked mesh does not deliver — which is why the body's
    /// delivered torque is asserted here beside the tooth load.
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
                let mut train = train_of(vec![arr::worm(1, 40), {
                    let mut s = arr::worm(1, 40);
                    s.meshes[0].sliding_friction = friction;
                    s.meshes[0].static_friction = friction;
                    s
                }]);
                train.load_cases[BACK].set_torque(applied);

                let r = solve_train(&train, &lib).expect("a train that solves");
                let (w, m) = worm(&r.by_part[1]);
                let locks = m.efficiency.locked().backward;
                assert_eq!(
                    locks,
                    friction > 0.1,
                    "the fixture must span both sides of the threshold"
                );

                // The stage reacts the load, so its output member carries it.
                let wheel = w.members[1].cases[BACK].torque;
                assert!(
                    (wheel - applied).abs() < 1e-9,
                    "the wheel is on the output shaft and the load is {applied} N·m, \
                     but it reports {wheel}"
                );

                // ...and the worm's teeth carry it referred by the ratio —
                // the *size* of the ratio, which is what a referral is —
                // whether or not the mesh passes it on.
                let worm = w.members[0].cases[BACK].torque;
                let want = wheel / reduction(w);
                assert!(
                    (worm - want).abs() < 1e-9 * applied,
                    "the worm's teeth carry {worm} where {wheel} at the wheel over a \
                     ratio of {} is {want}",
                    reduction(w),
                );
                // What the worm's *body* delivers is that attenuated by the
                // loss the mesh takes carrying it backward: nought where it
                // locks, something where it does not.
                let delivered = w.cases[BACK].torques[1].abs();
                let want = want * m.efficiency.backward.max(0.0);
                assert!(
                    (delivered - want).abs() < 1e-9 * applied,
                    "the worm's shaft delivers {delivered} where {want} is {:.4} of \
                     the tooth load",
                    m.efficiency.backward
                );
                assert_eq!(
                    locks,
                    delivered == 0.0,
                    "a locked mesh delivers nothing to the worm's shaft, and an \
                     unlocked one delivers something: {delivered} N·m at μ = {friction}"
                );
            }
        }
    }

    /// **Crossing the bodies does not stop a gear being a gear.**
    ///
    /// A crossed pair is a spur stage with an axis angle, and it is solved by
    /// translating it into the equivalent screw pair — which carries no tooth
    /// form, because a worm is a thread. So the translation dropped the shift,
    /// the dedendum and the root round on the way, and with them everything the
    /// members had to say about themselves: **the same pair reported
    /// `clamp.tooth_undercut` with its bodies parallel and nothing at all with
    /// them crossed.**
    ///
    /// `docs/corrections.md` records that gap being closed once already — "a gear
    /// in a geartrain never said it was undercut … reported now on every
    /// rack-cut member of every stage kind". It reached four stage types of five.
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
            let mut sp = arr::crossed([17, 43], shaft_angle);
            // Small enough at zero shift to be eaten into by a standard rack.
            sp.members[0].gear.teeth = 9;
            sp.members[0].gear.profile_shift = Auto::fixed(0.0);
            sp.members[0].gear.no_undercut = false;
            sp.members[1].gear.teeth = 23;
            train_of(vec![sp])
        };

        let flat = solve_train(&pair(0.0), &lib).expect("the parallel pair solves");
        let spur = &flat.by_part[0];
        let said: Vec<&str> = spur.members[0]
            .clamps
            .iter()
            .chain(&spur.members[0].notes)
            .map(|n| n.key.as_str())
            .collect();
        assert!(
            said.contains(&"clamp.tooth_undercut"),
            "the fixture must undercut its pinion or it checks nothing: {said:?}"
        );

        // 20°: a 10° helix, so the transverse geometry is near enough the
        // parallel one that the same pinion is still undercut.
        let angled = solve_train(&pair(20.0), &lib).expect("the crossed pair solves");
        let screw = &angled.by_part[0];
        let gear = &screw.members[0];
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
    /// A walk over `ShapeResult::members()` rather than over five named field
    /// paths, which is what that accessor is for: the fault it is looking for is
    /// a member quietly missing a figure, and a sweep that names the presets
    /// can only miss it in the one nobody named. F30 — a self-locking worm's wheel
    /// reporting 2.2e307 N·m — was in the fifth.
    ///
    /// The claim is deliberately the weak one: **present, finite, and signed
    /// like the stage's.** A *quantitative* law across arrangements does not exist, and
    /// finding that out is what this test cost. A parallel-axis member's forward
    /// torque is a geometric projection with no efficiency in it, so the ratio
    /// of backward to forward is the same for both members. A screw pair's
    /// output torque carries a forward efficiency that the backward load does
    /// not share, so its two members differ by exactly `1/η_forward` — by
    /// construction, and correctly — which is why the flow projects each case's
    /// torque through the meshes in that case's direction.
    #[test]
    fn every_member_of_a_reacting_pair_reports_its_share() {
        let lib = library();
        let mut train = two_pairs();
        train.load_cases[BACK].set_torque(0.5);
        // **A self-locking stage at the input end**, or nothing reacts the load
        // and the case is correctly zero at every gear — which would leave this
        // walking an empty list and passing for the wrong reason. The load
        // enters at the output and walks up; a worm stops it, and every stage
        // between it and the output carries it.
        {
            let mut s = train.part_shapes();
            s.insert(0, arr::worm(1, 40));
            rechain(&mut train, s);
        }
        train.chain_on(arr::planetary(12, 30, 72, 3));
        train.chain_on({
            let mut s = arr::pair([17, 43]);
            s.distances[0].angle = 90.0;
            s
        });

        let r = solve_train(&train, &lib).expect("a train that solves");
        let (mut checked, mut stages) = (0u32, 0u32);
        for (k, stage) in r.by_part.iter().enumerate() {
            let members = &stage.members;
            if members.is_empty() {
                continue;
            }
            stages += 1;
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
        assert!(stages >= 3, "only {stages} stages contributed members");
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
        let mut worst = 0.0_f64;
        for teeth in [[9_u32, 37], [12, 29]] {
            let stage = {
                let mut s = arr::pair(teeth);
                s.set_search(true);
                s
            };
            let asked = [0, 1].map(|i| {
                stage.members[i]
                    .gear
                    .shift_asked(&stage.base_params_of(i), stage.members[i].ring.as_ref())
            });
            let bounds = Bounds {
                floor: asked.map(|a| a.search_floor),
                min_contact_ratio: stage.meshes[0].min_contact_ratio,
                clearance: stage.distances[0].clearance.manual,
            };
            let pair = |q: [f64; 2]| [0, 1].map(|i| stage.params_of(i, q[i]));
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
                    stage.meshes[0].sliding_friction,
                    &Search::SHIPPED,
                )?;
                let fixed = {
                    let mut s = stage.clone();
                    for (i, m) in s.members.iter_mut().enumerate() {
                        m.gear.profile_shift = Auto::fixed(x[i]);
                    }
                    s.set_search(false);
                    s
                };
                try_alone(&fixed)
                    .ok()
                    .map(|r| r.meshes[0].efficiency.forward)
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
                    stage.meshes[0].sliding_friction,
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

    /// **Every search asks the same of its meshes.**
    ///
    /// A constraint belongs to the mesh, not to the arrangement around it: a
    /// mesh whose teeth reach past the root circle they run into bottoms out
    /// whether a carrier is turning about it or not. Three stage types each
    /// wrote the question out for themselves and between them answered it three
    /// ways — the pair asked about bottoming, neither epicyclic type did; the pair and the
    /// set asked whether each member could be cut, and a ring was asked by
    /// nobody.
    ///
    /// `auto::MeshTrial` is the one place now, and this is the claim that says
    /// so from the outside: **whatever the search chose, the mesh it chose is
    /// one the shared contract admits.** A search that stops asking chooses
    /// something that fails here; a search added later that never asks fails
    /// here the first time its answer is pushed against a bound.
    #[test]
    fn every_search_asks_the_same_of_its_meshes() {
        use crate::auto::Search;
        let mut checked = 0u32;

        // --- a pair, rebuilt through the stage's own constructors.
        for teeth in [[9_u32, 37], [17, 43], [12, 29]] {
            let stage = {
                let mut s = arr::pair(teeth);
                s.set_search(true);
                s
            };
            let x = stage.shifts_at(&Search::SHIPPED);
            // The teeth as built, tips held where their mates ask it.
            let built = stage.build_at(&x).expect("the pair builds");
            let g = [0, 1].map(|i| Tooth::new(*built.members[i].params()));
            let zero = crate::mesh::Mesh::new(&g[0], &g[1], crate::mesh::MeshKind::External)
                .expect("the pair meshes");
            let mesh = zero
                .at(zero.a_w + stage.distances[0].clearance.manual)
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
            let mut set = arr::planetary(sun, planet, sun + 2 * planet, 3);
            set.set_search(true);
            set.members[0].gear.profile_shift = Auto::automatic(0.0);
            set.members[2].gear.profile_shift = Auto::automatic(0.0);
            let shape = set.clone();
            let b = shape
                .build_at(&shape.shifts_at(&Search::SHIPPED))
                .expect("the set has geometry");
            let (gs, gp, gr) = (&b.members[0], &b.members[1], &b.members[2]);
            let gaps = [
                b.meshes[0].line().unwrap().operating.bottom_clearance(
                    [gs.tip_radius(), gp.tip_radius()],
                    [gs.root_radius(), gp.root_radius()],
                ),
                b.meshes[1].line().unwrap().operating.bottom_clearance(
                    [gp.tip_radius(), gr.tip_radius()],
                    [gp.root_radius(), gr.root_radius()],
                ),
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

        // --- a hula stage, both of its meshes, at the crank the shape sizes.
        for n in [12_u32, 18, 30] {
            let mut shape = hula_shape([n + 1, n, n - 1, n]);
            shape.set_search(true);
            let Ok(b) = shape.build_at(&shape.shifts_at(&Search::SHIPPED)) else {
                continue;
            };
            for (m, mesh) in b.meshes.iter().enumerate() {
                let (a, z) = (shape.meshes[m].a, shape.meshes[m].b);
                let (pinion, ring) = (&b.members[a], &b.members[z]);
                let gaps = mesh.line().unwrap().operating.bottom_clearance(
                    [pinion.tip_radius(), ring.tip_radius()],
                    [pinion.root_radius(), ring.root_radius()],
                );
                for (i, gap) in gaps.iter().enumerate() {
                    assert!(
                        *gap > 0.0,
                        "N {n} mesh {m} member {i} bottoms out by {} mm",
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
    /// Both epicyclic presets, because the last time a rule reached one of
    /// them and not the other it cost a second entry in `docs/corrections.md`.
    #[test]
    fn a_search_chooses_only_parts_its_tool_leaves_alone() {
        let mut checked = 0u32;
        for sun in [11_u32, 13, 17, 19, 24, 31] {
            for planet in [14_u32, 17, 18, 21, 25] {
                let mut set = arr::planetary(sun, planet, sun + 2 * planet, 3);
                set.set_search(true);
                set.members[0].gear.profile_shift = Auto::automatic(0.0);
                set.members[2].gear.profile_shift = Auto::automatic(0.0);
                let Ok(r) = try_alone(&set) else {
                    continue;
                };
                checked += 1;
                for (name, g) in [
                    ("sun", &r.members[0]),
                    ("planet", &r.members[1]),
                    ("ring", &r.members[2]),
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

    /// **What a pair's search is converged to**, in efficiency: ten parts in
    /// a million, two orders below the last digit the tool prints
    /// (`the_search_is_converged_not_budgeted` argues it).
    const CONVERGED_EFFICIENCY: f64 = 1e-5;

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
        for teeth in [[9_u32, 37], [17, 43], [12, 29], [23, 61]] {
            let stage = {
                let mut s = arr::pair(teeth);
                s.set_search(true);
                s
            };
            let free = try_alone(&stage).expect("the pair solves with the distance free");

            let at = |a: f64| {
                try_alone(&{
                    let mut s = stage.clone();
                    s.distances[0].distance = Auto::fixed(a);
                    s
                })
                .expect("...and with it given")
            };

            // At the distance it chose: the same answer — to **twice** the
            // search's own stopping distance in the shifts, or, where the
            // division is flat, to what `the_search_is_converged_not_budgeted`
            // holds a pair's efficiency to.
            //
            // Twice rather than once because of where these answers sit: on a
            // wall, which the two searches reach along different coordinates —
            // one free in both shifts, the other with their sum pinned — each
            // resolving it to its own last step. **And the efficiency beside
            // it** since tips are held off their mates' flanks: at 9/37 both
            // tips are held at the optimum, the division is flat to 6.6e-6 of
            // efficiency across 0.027 of shift (free 0.795/0.849, given
            // 0.822/0.822), and the claim is about the answer, not the point.
            let given = at(free.distances[0].running);
            let flat = (free.meshes[0].efficiency.forward - given.meshes[0].efficiency.forward)
                .abs()
                < CONVERGED_EFFICIENCY;
            for i in 0..2 {
                let (a, b) = (
                    free.members[i].profile_shift,
                    given.members[i].profile_shift,
                );
                assert!(
                    flat || (a - b).abs() < 2.0 * crate::auto::Search::SHIPPED.resolution,
                    "{teeth:?} gear {i}: free chose {a} and {:.6} mm given chose {b}",
                    free.distances[0].running
                );
            }

            // ...and on the way there, every distance is answered rather than
            // refused, with the efficiency a **scan** of the division finds
            // there: the pinion's shift at 401 points across two and a half
            // modules, the wheel's closing the distance, each kept where the
            // search would keep it (teeth clear, contact ratio at its floor or
            // over, nothing clamped). The search is no worse than the scan's
            // best by more than it is converged to, and no scan point beats it
            // by more.
            //
            // **Not climbing**, which is what this asked until tips were held
            // off their mates' flanks. At 12/29 the answer at a given distance
            // peaks at 21.447 mm (0.979670), falls to 0.979617 at 21.684 and
            // rises again to 0.979847 at 21.802, where the wheel's tip starts
            // to be held: two regions, and between them a dip the scan finds
            // too — the model's, not a walk that stopped.
            let scan = |a: f64| -> f64 {
                (0..=400)
                    .filter_map(|k| {
                        let mut s = stage.clone();
                        s.distances[0].distance = Auto::fixed(a);
                        s.members[0].gear.profile_shift =
                            Auto::fixed(-1.0 + 2.5 * f64::from(k) / 400.0);
                        s.set_search(false);
                        let r = try_alone(&s).ok()?;
                        let m = &r.meshes[0];
                        let kept = m.teeth_clear()
                            && m.line?.contact_ratios.transverse >= s.meshes[0].min_contact_ratio
                            && r.members
                                .iter()
                                .all(|g| g.notes.iter().all(|n| !n.key.starts_with("clamp.")));
                        kept.then_some(m.efficiency.forward)
                    })
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            let mut last = 0.0;
            let steps = 12;
            for k in 1..=steps {
                let a = free.distances[0].running
                    - (free.distances[0].running
                        - stage.members[0].normal_module() * 0.5 * f64::from(teeth[0] + teeth[1]))
                        * f64::from(steps - k)
                        / f64::from(steps);
                let here = at(a).meshes[0].efficiency.forward;
                let best = scan(a);
                assert!(
                    (here - best).abs() < CONVERGED_EFFICIENCY,
                    "{teeth:?}: {a:.4} mm gives {here} where a scan of the division \
                     finds {best}"
                );
                last = here;
            }
            // The same ten parts in a million `the_search_is_converged_not_budgeted`
            // holds a pair to, and for the same reason: both answers sit on the
            // interference wall and each resolves it to its own last step.
            assert!(
                (last - free.meshes[0].efficiency.forward).abs() < CONVERGED_EFFICIENCY,
                "{teeth:?}: the last step reaches {last} where the free answer is {}",
                free.meshes[0].efficiency.forward
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
        let converged = CONVERGED_EFFICIENCY;

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
            let stage = {
                let mut s = arr::pair(teeth);
                s.set_search(true);
                s
            };
            // Scored by solving the stage at the shifts each search chose, so
            // the objective is the one the tool reports rather than a second
            // spelling of it.
            let at = |x: &[f64]| {
                let fixed = {
                    let mut s = stage.clone();
                    for (i, m) in s.members.iter_mut().enumerate() {
                        m.gear.profile_shift = Auto::fixed(x[i]);
                    }
                    s.set_search(false);
                    s
                };
                try_alone(&fixed).map(|r| r.meshes[0].efficiency.forward)
            };
            let (Ok(shipped), Ok(refined)) = (
                at(&stage.shifts_at(&Search::SHIPPED)),
                at(&stage.shifts_at(&hard)),
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
                let mut set = arr::planetary(sun, planet, sun + 2 * planet, 3);
                set.set_search(true);
                set.members[0].gear.profile_shift = Auto::automatic(0.0);
                set.members[2].gear.profile_shift = Auto::automatic(0.0);
                let shape = set.clone();
                let eta0 = |x: Vec<f64>| {
                    let b = shape.build_at(&x).ok()?;
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
                            &b.meshes[0].line().unwrap().path,
                            &b.meshes[0].line().unwrap().operating,
                            b.members[0].as_gear(),
                            set.meshes[0].sliding_friction,
                        ) * one(
                            &b.meshes[1].line().unwrap().path,
                            &b.meshes[1].line().unwrap().operating,
                            b.members[1].as_gear(),
                            set.meshes[1].sliding_friction,
                        ),
                    )
                };
                let (Some(shipped), Some(refined)) = (
                    eta0(shape.shifts_at(&Search::SHIPPED)),
                    eta0(shape.shifts_at(&hard)),
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
                let generous = eta0(shape.shifts_at(&Search {
                    budget: Search::SHIPPED.budget * 4,
                    ..Search::SHIPPED
                }));
                // **One named exception, 11/17**: its ring's undercut floor
                // (U1) is a second wall, and the walk runs its budget out along
                // it — 4× the budget moves `η₀` by 1.75e-7. Pinned so it can
                // only shrink; docs/state.md#known-approximate-documented-at-the-call-site
                // has it, and the structural search (redesign C) replaces it.
                //
                // **And 13/18**, since its ring's tip is held off the planet's
                // usable flank rather than refused there: the region the
                // refusal closed is open, with a kink along the planet's form
                // circle where the hold begins, and the walk runs its budget
                // along it — 4× the budget moves `η₀` by 5.3e-9, and 16× and
                // 64× agree with 4×. The same wall F50 names, met again.
                if (sun, planet) == (11, 17) || (sun, planet) == (13, 18) {
                    let moved = generous.map_or(f64::INFINITY, |g| (g - shipped).abs());
                    assert!(
                        moved < 2e-7,
                        "{sun}/{planet}: 4× the budget moves `η₀` by {moved}"
                    );
                    continue;
                }
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
    /// the stage's input body* and that magnitude pushed through the forward
    /// construction. That is the same answer only where the distribution does not
    /// depend on direction — a parallel-axis mesh carries one tangential force
    /// whichever way it turns — and an epicyclic set is not such a stage: which
    /// body drives decides on which side `η₀` multiplies. Measured on the shipped
    /// set, a back-driven ring was rated **6.0 % low** in bending and 3.0 % low in
    /// contact; on the hula stage, whose reduction is far larger, **41 % low** and
    /// 23 % low. The member torques themselves had already been put right
    /// (`docs/corrections.md`); the ratings had not, and nothing compared them.
    ///
    /// Every preset that reports per-member stresses is walked, through
    /// `ShapeResult::members()` rather than five named paths, for the reason that
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
            let mut train = two_pairs();
            train.load_cases[BACK].set_torque(applied);
            {
                let mut s = train.part_shapes();
                s.insert(0, arr::worm(1, 40));
                rechain(&mut train, s);
            }
            train.chain_on(arr::planetary(12, 30, 72, 3));
            train.chain_on(hula());

            let r = solve_train(&train, &lib).expect("a train that solves");
            for (k, stage) in r.by_part.iter().enumerate() {
                // A point contact is rated on the worse of two *constructions*
                // — the input torque on the worm forward, the applied load on
                // the wheel backward — so its members' stresses are not a
                // projection of their own torques and this law is not theirs.
                // The worm stage in this train is here for the walk, and its
                // own rating is gated in `crossed::tests`.
                if stage.meshes[0].point.is_some() {
                    continue;
                }
                for (i, g) in stage.members.iter().enumerate() {
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
                    // **A member in two meshes carries two tooth loads** and
                    // reports the larger; its stress is the larger of two
                    // too, and which mesh gives it can differ by direction.
                    // The law is exact for a member in one mesh, and for a
                    // planet within the meshes' efficiencies of each other.
                    let meshes_in = train.part_shapes()[k]
                        .meshes
                        .iter()
                        .filter(|m| m.a == i || m.b == i)
                        .count();
                    let tol = if meshes_in > 1 { 2e-2 } else { 1e-9 };
                    if let (Some(a), Some(b)) = (start.bending_stress, end.bending_stress) {
                        assert!(
                            (b / a - want).abs() < tol * want,
                            "stage {k} member {i}: bending {b} against {a} is {}, \
                             where {back} N·m from the end and {forward} N·m from \
                             the start make {want}",
                            b / a
                        );
                    }
                    let (a, b) = (start.contact_stress, end.contact_stress);
                    assert!(
                        (b / a - want.sqrt()).abs() < tol * want.sqrt(),
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
    /// A screw pair is the mesh whose distribution depends on direction: driving
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
            let mut train = two_pairs();
            train.load_cases[PEAK].set_torque(input_torque);
            train.load_cases[CYCLIC].set_torque(input_torque);
            train.load_cases[BACK].set_torque(back);
            rechain(&mut train, vec![arr::worm(1, 40)]);
            let r = solve_train(&train, &lib).expect("a train that solves");
            let (w, m) = worm(&r.by_part[0]);
            // Whichever end the torque is put on, its case is the one read.
            let case = if back > 0.0 { BACK } else { PEAK };
            (
                m.cases[case].contact.max_pressure,
                reduction(w),
                m.efficiency.forward,
            )
        };

        // Driven backward at `load` on the wheel, with nothing at all coming the
        // other way — zero is a torque, and a stage carrying none of it forward
        // still carries this.
        let (from_output, ratio, eta) = driven(0.0, load);
        // ...and driven forward hard enough to put the same torque on the wheel.
        let (from_input, _, _) = driven(load / (ratio * eta), 0.0);

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
        let locked = arr::worm(17, 23).with_first_helix(9.0);
        let r = try_alone(&locked).expect("a locked pair is still a pair");
        let r_point = &r.meshes[0];
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
        let driving = locked.clone().with_first_helix(18.0);
        let d = try_alone(&driving).expect("and this one");
        let d_point = &d.meshes[0];
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
    /// Asserted on every preset rather than on the one that failed, since what is
    /// being claimed is a property of the tool and not a patch to a stage.
    #[test]
    fn a_preset_carrying_nothing_is_rated() {
        let lib = library();
        for stage in [
            arr::pair([17, 43]),
            arr::worm(1, 40),
            arr::planetary(12, 30, 72, 3),
            hula(),
        ] {
            let mut train = train_of(vec![stage]);
            train.load_cases[CYCLIC].set_torque(0.0);
            let r = solve_train(&train, &lib)
                .unwrap_or_else(|e| panic!("a stage at no operating load: {e:?}"));
            // A worm's members are not gears, so the walk below is empty there
            // and the claim is the stage's own — which is the one that failed.
            if let Some(m) = r.by_part[0].meshes.first().filter(|m| m.point.is_some()) {
                assert_eq!(m.cases[1].contact.max_pressure, 0.0);
                assert!(m.cases[0].contact.max_pressure > 0.0);
            }
            for (i, g) in r.by_part[0].members.iter().enumerate() {
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
    /// to which stage type they had picked. They are on [`MeshReport`] now, and
    /// this is the walk that says every mesh gets them — through
    /// [`ShapeResult::meshes`], which is the same shape as `members()` and
    /// exists so a walk cannot forget an arrangement by naming them.
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
    /// contact slides along the bodies by `Δa / sin Σ`) and the face wide
    /// enough that the line governs:
    ///
    /// - **the pitch point meets exactly** at no friction — to 1e-5 —
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
        // The table's rows: (μ, how near) at the pitch point, relative at
        // μ = 0 and in percent at μ = 0.08; and the peak, in percent.
        let ((_, pitch_free), (mu, pitch_friction_percent), peak_percent) =
            ((0.0, 1e-5), (0.08, 1.5), 5.0);
        let stage = |sigma: f64, mu: f64| {
            let mut s = arr::crossed([17, 43], sigma);
            s.meshes[0].sliding_friction = mu;
            s.meshes[0].static_friction = mu;
            s.distances[0].clearance = Auto::fixed(0.0);
            for m in &mut s.members {
                m.gear.face_width = Auto::fixed(30.0);
            }
            s.with_additional_helix(20.0)
        };
        let mesh = |sigma: f64, mu: f64| {
            try_alone(&stage(sigma, mu))
                .expect("a pair either way")
                .meshes[0]
                .clone()
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
            pitch_free,
        );
        // The worst point is a different point — see above — and is not one:
        // 5 %, to the whole percent the table quotes. The seam is real, and this
        // test would be asserting agreement it does not have if it closed.
        let seam = 100.0 * (l.max_pressure - p.max_pressure) / l.max_pressure;
        assert!(
            (seam - peak_percent).abs() <= 0.5,
            "the single-pair seam is {seam:.2} %: {} against {}",
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
        let (line, point) = (mesh(0.0, mu), mesh(0.01, mu));
        let gap = (line.cases[0].contact.at_pitch_point - point.cases[0].contact.at_pitch_point)
            / line.cases[0].contact.at_pitch_point;
        assert!(
            (100.0 * gap - pitch_friction_percent).abs() <= 0.05,
            "the friction seam at the pitch point is {:.3} %, and it is the flank load \
             convention — the table's 1.5 %",
            100.0 * gap
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
        // Which of each preset's meshes have a ring in them, in the order
        // `meshes()` returns them: a pair none, a set its second, a hula both.
        for (stage, internal) in [
            (arr::pair([17, 43]), vec![false]),
            (arr::planetary(12, 30, 72, 3), vec![false, true]),
            (hula(), vec![true, true]),
            // A worm's one mesh is external too, and it is in the walk now:
            // a point contact answers in the same report as a line.
            (arr::worm(1, 40), vec![false]),
        ] {
            let train = train_of(vec![stage]);
            let r = solve_train(&train, &lib).expect("every shipped kind solves");
            let meshes = &r.by_part[0].meshes;
            assert_eq!(
                meshes.len(),
                internal.len(),
                "the walk found a different number of meshes than the preset has"
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

    /// A 9/37 spur pair at a pinion shift as typed: no undercut floor, no
    /// search, so the shift can be walked into undercut.
    fn fixed_9_37(x1: f64, sharing: crate::contact::LoadSharing) -> Shape {
        let mut s = arr::pair([9, 37]);
        s.members[0].gear.profile_shift = Auto::fixed(x1);
        s.members[1].gear.profile_shift = Auto::fixed(0.0);
        for m in &mut s.members {
            m.gear.no_undercut = false;
        }
        s.meshes[0].load_sharing = sharing;
        s
    }

    /// **A pair rates continuously through the onset of flank interference**
    /// (T06.1).
    ///
    /// The 9-tooth pinion's shift is walked from −0.6 to 0.6 in steps of
    /// 0.002, through the shift near 0.45 where the wheel's tip stops reaching
    /// past the pinion's usable flank. Every step solves — the teeth touch and
    /// the root has a section, so `NoContact` is not true
    /// — and no figure jumps: a step is never more than twice the larger
    /// of its neighbours, which a piecewise-smooth figure keeps at a kink and
    /// a jump breaks. The tip-to-tip path broke it at −0.337, where the shared
    /// bending factor fell 34 % in one step, and refused from −0.343.
    #[test]
    fn a_pair_rates_continuously_through_the_onset_of_interference() {
        use crate::contact::LoadSharing;
        for sharing in [LoadSharing::None, LoadSharing::LinearRamp] {
            let figures: Vec<(f64, [f64; 5])> = (0..=600)
                .map(|i| {
                    let x1 = -0.6 + 0.002 * f64::from(i);
                    let r = try_alone_at(&fixed_9_37(x1, sharing), 2.0, 1000.0)
                        .unwrap_or_else(|e| panic!("x1={x1:.3} {sharing:?}: refused: {e}"));
                    let mesh = &r.meshes[0];
                    let bending = |i: usize| {
                        r.members[i].cases[0]
                            .bending_stress
                            .unwrap_or_else(|| panic!("x1={x1:.3}: member {i} unrated"))
                    };
                    (
                        x1,
                        [
                            mesh.contact_ratio,
                            mesh.efficiency.forward,
                            bending(0),
                            bending(1),
                            mesh.cases[0].contact.max_pressure,
                        ],
                    )
                })
                .collect();
            for k in 0..5 {
                for i in 1..figures.len() - 1 {
                    let d = |i: usize| (figures[i + 1].1[k] - figures[i].1[k]).abs();
                    let (before, here) = (d(i - 1), d(i));
                    let after = if i + 1 < figures.len() - 1 {
                        d(i + 1)
                    } else {
                        before
                    };
                    assert!(
                        here <= 2.0 * before.max(after) + 1e-12 * figures[i].1[k].abs(),
                        "{sharing:?}: figure {k} jumps by {here} between x1 {:.3} and {:.3} \
                         against neighbouring steps {before} and {after}",
                        figures[i].0,
                        figures[i + 1].0
                    );
                }
            }
        }
    }

    /// **Flank interference is said by the solve**, not only drawn by the
    /// panel: a mesh whose mate's tip reaches past a usable flank carries a
    /// note naming the member and how far, and a mesh that does not, none.
    #[test]
    fn flank_interference_is_said_by_the_mesh() {
        let mut seen = 0;
        for i in 0..=60 {
            let x1 = -0.6 + 0.02 * f64::from(i);
            let r = try_alone_at(
                &fixed_9_37(x1, crate::contact::LoadSharing::None),
                2.0,
                1000.0,
            )
            .unwrap_or_else(|e| panic!("x1={x1:.2}: refused: {e}"));
            let mesh = &r.meshes[0];
            let said = mesh
                .notes
                .iter()
                .filter(|n| n.is("mesh.flank_interference"))
                .count();
            let flagged = mesh.flank_interference.iter().filter(|f| **f).count();
            assert_eq!(
                said, flagged,
                "x1={x1:.2}: {:?} {:?}",
                mesh.flank_interference, mesh.notes
            );
            seen += flagged;
        }
        assert!(seen > 0, "the walk never reached interference");
    }

    /// **Which presets' rings are held off a usable flank**, stated rather
    /// than assumed: the four with a full-depth ring at no shift, whose tip
    /// would reach past its planet's usable flank. Each is held (the note on
    /// the ring), and each asked as typed — the bound off — interferes, as
    /// `a_full_depth_ring_interferes_as_typed_and_is_held_to_the_form_circle`
    /// records. Every other preset is the tips' path to the bit
    /// (`contact::tests::the_path_starts_and_ends_on_usable_flank`).
    #[test]
    fn the_presets_whose_rings_are_held_are_the_ones_named() {
        let as_typed = |p: arr::Preset| {
            let mut s = p.build();
            for m in &mut s.members {
                m.gear.no_tip_past_mate_flank = false;
            }
            s
        };
        let held: Vec<arr::Preset> = arr::Preset::ALL
            .into_iter()
            .filter(|p| {
                let r = try_alone_at(&p.build(), 2.0, 1000.0).expect("a preset solves");
                r.members.iter().any(|m| {
                    m.notes
                        .iter()
                        .any(|n| n.is(key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK))
                })
            })
            .collect();
        let interfering: Vec<arr::Preset> = arr::Preset::ALL
            .into_iter()
            .filter(|&p| {
                let r = try_alone_at(&as_typed(p), 2.0, 1000.0).expect("a preset solves");
                r.meshes
                    .iter()
                    .any(|m| m.flank_interference.contains(&true))
            })
            .collect();
        let named = [
            arr::Preset::Planetary,
            arr::Preset::Wolfrom,
            arr::Preset::Compound,
            arr::Preset::MeshedPlanets,
        ];
        assert_eq!(held, named);
        assert_eq!(interfering, named);
    }

    /// **A full-depth internal pair interferes as typed, and held its ring's
    /// tip meets the planet's form circle exactly.**
    ///
    /// `ring::mesh_with` has said so since it existed — the ring's tip can only
    /// touch the pinion's involute while `√(r_a2² − r_b2²) ≥ a sin α_w`, and a
    /// standard ring misses it. Typed at full depth with the bound off, the
    /// shipped set's ring reaches past the planet's usable flank (the
    /// literature's involute interference, `[0]`), and the other flank and
    /// the tips are clear. Held, nothing is reached past, and the tip sits on
    /// the conjugate of the planet's junction: the path the cut left as
    /// typed is the path the held tip reaches, so every figure the path
    /// gives is the same, and only the ring's own tooth is shorter.
    #[test]
    fn a_full_depth_ring_interferes_as_typed_and_is_held_to_the_form_circle() {
        let solved = |held: bool| {
            let mut set = arr::planetary(12, 30, 72, 3);
            set.members[2].gear.addendum = 1.0;
            set.members[2].gear.no_tip_past_mate_flank = held;
            try_alone(&set).expect("the shipped set solves")
        };
        let typed = solved(false);
        let full_mesh = typed.meshes[1].clone();
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
        let held = solved(true);
        let short_mesh = held.meshes[1].clone();
        let short = short_mesh.tips.expect("an internal mesh");
        assert!(
            short.tips_clear() && short_mesh.flank_interference == [false, false],
            "holding the ring's tip should clear it: {:?} {short:?}",
            short_mesh.flank_interference
        );
        let ha = held.members[2].addendum;
        assert!(ha < 1.0, "the ring is held shorter: {ha}");
        let eps = |r: &Alone| r.meshes[1].line.unwrap().contact_ratios.transverse;
        assert!(
            (eps(&typed) - eps(&held)).abs() < 1e-9,
            "the held tip ends the path where the cut did: {} and {}",
            eps(&typed),
            eps(&held)
        );
    }

    /// **The held tip is the typed one wherever that one clears**, and the
    /// bound where it does not: at a given shift the addendum a ring builds
    /// at is `min(typed, bound)`, continuous in the typed number and never
    /// above it; the note fires exactly where the two part, and no flank is
    /// reached past on either side of the bound.
    ///
    /// The bound moves with the ring's shift, and past about 1.02 modules
    /// typed the shift does move: the ring's undercut floor
    /// ([`MemberGear::no_undercut`]) is asked at the typed tip, not the held
    /// one, so a taller typed tooth raises it and the held tip follows,
    /// still increasing. Where the shift is the one the full-depth ring
    /// has, the held addendum is that ring's to the bit.
    #[test]
    fn a_held_ring_tip_is_the_lesser_of_typed_and_bound() {
        let at = |typed: f64| {
            let mut set = arr::planetary(12, 30, 72, 3);
            set.members[2].gear.addendum = typed;
            let r = try_alone(&set).expect("the set solves");
            let held = r.members[2]
                .notes
                .iter()
                .any(|n| n.is(key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK));
            let clear = r
                .meshes
                .iter()
                .all(|m| m.flank_interference == [false, false]);
            (
                r.members[2].addendum,
                r.members[2].profile_shift,
                held,
                clear,
            )
        };
        let (bound, shift, _, _) = at(1.0);
        assert!(bound < 1.0, "the full-depth ring is held: {bound}");
        let mut last = f64::NEG_INFINITY;
        let (mut below, mut above, mut moved) = (0, 0, 0);
        for k in 0..=40 {
            let typed = 0.8 + 0.3 * f64::from(k) / 40.0;
            let (used, x, held, clear) = at(typed);
            assert!(clear, "typed {typed}: a flank is reached past");
            assert!(used <= typed, "typed {typed}: built taller, at {used}");
            assert_eq!(held, used < typed, "typed {typed}: the note");
            assert!(used >= last, "typed {typed}: the tip went back");
            last = used;
            if x.to_bits() == shift.to_bits() {
                assert_eq!(used, typed.min(bound), "typed {typed}");
                if typed < bound {
                    below += 1;
                } else {
                    above += 1;
                }
            } else {
                moved += 1;
            }
        }
        assert!(
            below > 0 && above > 0 && moved > 0,
            "the sweep reaches all three: {below} {above} {moved}"
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
        let mut train = two_pairs();
        train.load_cases[BACK].set_torque(0.5);
        {
            let mut s = train.part_shapes();
            s.insert(0, arr::worm(1, 40));
            rechain(&mut train, s);
        }

        let r = solve_train(&train, &lib).expect("a train that solves");
        let mut checked = 0u32;
        for (k, stage) in r.by_part.iter().enumerate() {
            // Parallel axes only: a worm's members do not share a tangential
            // force, and its own two-member relation is gated in `crossed`.
            if stage.meshes[0].line.is_none() {
                continue;
            }
            let spur = stage;
            let (a, b) = (
                spur.members[0].cases[BACK].torque,
                spur.members[1].cases[BACK].torque,
            );
            if a == 0.0 {
                continue;
            }
            checked += 1;
            // Torques are in the *size* of the ratio; the sign is the body's.
            let want = reduction(spur);
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

    /// **Every kind, at a spread of tooth counts and every arrangement** — the
    /// grid the wiring laws below are asserted over.
    ///
    /// `every_kind` is the presets; this turns the axes that change the
    /// *topology's* answer, which is what a law about topology has to be swept
    /// over. An axis nobody turns is an axis nobody tests
    /// (`docs/corrections.md`), and for the graph that axis is the arrangement:
    /// which body is held is the whole of what an epicyclic set's ratio
    /// depends on, and a sweep that leaves it at the preset checks one sixth of
    /// the model.
    fn every_wiring() -> Vec<(String, Shape, Asked)> {
        use crate::planetary::{Arrangement, PlanetaryShaft};
        let conventional = |name: String, stage: Shape| {
            let b = Asked::conventional(&stage);
            (name, stage, b)
        };
        let mut out = Vec::new();
        for (z1, z2) in [(17_u32, 43_u32), (9, 37), (13, 13)] {
            let s = arr::pair([z1, z2]);
            out.push(conventional(format!("spur {z1}/{z2}"), s));
        }
        out.push(conventional("worm".into(), arr::worm(1, 40)));
        // **The arrangement is what a set is asked, not what it is**: one
        // stage per tooth count, six boundaries each.
        for teeth in [[12_u32, 30, 72], [24, 18, 60], [17, 17, 51]] {
            let mut p = arr::planetary(12, 30, 72, 3);
            for (m, z) in p.members.iter_mut().zip(teeth) {
                m.gear.teeth = z;
            }
            for input in PlanetaryShaft::ALL {
                for fixed in PlanetaryShaft::ALL {
                    if input == fixed {
                        continue;
                    }
                    out.push((
                        format!("set {teeth:?} {input:?} in, {fixed:?} held"),
                        p.clone(),
                        Asked::set(&p, Arrangement { input, fixed }),
                    ));
                }
            }
        }
        for teeth in [[65_u32, 61, 57, 61], [19, 18, 17, 16]] {
            let mut h = hula_shape(teeth);
            h.members[2].gear.profile_shift = Auto::automatic(0.0);
            out.push(conventional(format!("hula {teeth:?}"), h));
        }
        out
    }

    /// **The graph reproduces every preset's kinematics**, which is the whole
    /// of what the wiring has to earn.
    ///
    /// Three stage types each solved their speeds a different way —
    /// `Mesh::ratio` for a pair, `planetary::power` for a set, the two
    /// products through that same solve for a hula stage — and one system of
    /// mesh rows had to give all three. It did, and the types went; what this
    /// holds now is that a stage solved under its boundary reports the ratio
    /// the wiring reads for that boundary, so the plumbing between the two
    /// cannot pick a different port. **It is the only thing that can say so**,
    /// since the lock-up invariant is silent on a wrong sign and on a
    /// misattributed frame alike (`crate::kinematics`, measured).
    ///
    /// The ratio is compared **signed and to the float**, since every stage's
    /// reported ratio is the graph's own reading. For a while this compared
    /// magnitudes and checked the sign on the epicyclic types only, because a
    /// pair's ratio was `Mesh::ratio` — *"ignoring sign"* by its own
    /// documentation — while a set's was signed; that disagreement is resolved
    /// and recorded (`docs/corrections.md`), and this is where it would show up
    /// again.
    #[test]
    fn the_graph_gives_every_preset_the_kinematics_it_reports() {
        let lib = library();
        let mut checked = 0u32;
        for (name, stage, b) in every_wiring() {
            let w = stage.wiring();
            let system = stage.system().unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(system.lock_up_is_free(), "{name}");
            let m = system
                .motion(&b.conditions(w.slots.len()))
                .unwrap_or_else(|e| panic!("{name}: {e:?}"));
            assert!(
                m.is_unique(),
                "{name}: the arrangement should determine every shaft, not {m:?}"
            );

            // --- the ratio, against the stage's own, asked the same thing.
            let r = solve_alone(&b.train(&stage, 1.0, 1.0), &lib)
                .unwrap_or_else(|e| panic!("{name}: {e}"));
            let graph = m
                .ratio(b.input, b.output)
                .unwrap_or_else(|| panic!("{name}: the output does not turn"));
            // **Signed, and exact to the float**: a stage's ratio is its
            // path's, read off the train's own graph, so this is the same
            // number read twice — once through the train and once here — and
            // anything short of equality would be a second source grown
            // somewhere. It used to compare magnitudes and check the sign
            // only on the epicyclic types, because a pair's ratio was
            // `z₂/z₁` and could not say its output reversed.
            let want = r.ratio.unwrap();
            assert!(
                (graph.to_f64() - want).abs() < 1e-12 * want.abs().max(1.0),
                "{name}: graph {graph} vs stage {want}"
            );

            // --- every member's speed, and its speed against its own frame,
            // signed.
            //
            // Solved at unit input speed, so the graph's speeds *are* the
            // stage's — which is the strongest form of this check: not a
            // ratio each, but every body at once.
            for (i, g) in r.members.iter().enumerate() {
                let shaft = w.mounts[i].spins_with;
                let case = &g.cases[0];
                let speed = m.values[shaft].to_f64();
                assert!(
                    (speed - case.speed).abs() < 1e-9 * case.speed.abs().max(1.0),
                    "{name}: member {i} graph {speed} vs stage {}",
                    case.speed
                );
                let frame = m.values[w.mounts[i].axis_fixed_in].to_f64();
                let against = speed - frame;
                assert!(
                    (against - case.speed_against_carrier).abs()
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
            let mut stage = arr::planetary(12, 30, 72, 3);
            for (m, z) in stage
                .members
                .iter_mut()
                .zip([teeth.sun, teeth.planet, teeth.ring])
            {
                m.gear.teeth = z;
            }
            for input in PlanetaryShaft::ALL {
                for fixed in PlanetaryShaft::ALL {
                    if input == fixed {
                        continue;
                    }
                    let arrangement = Arrangement { input, fixed };
                    let b = Asked::set(&stage, arrangement);
                    let as_stage = stage.clone();
                    let w = as_stage.wiring();
                    let system = as_stage.system().unwrap();
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
                    let mut applied = vec![None; w.slots.len()];
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
                            // from, and the one body that is not a gear.
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
                    let motion = system.motion(&b.conditions(w.slots.len())).unwrap();
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
    fn a_part_with_no_geometry_still_answers_about_motion() {
        let lib = library();
        let mut stage = arr::planetary(12, 30, 72, 3);
        // Only `z_ring ∈ [48, 54]` admits any planet shift at 17/17.
        stage.members[0].gear.teeth = 17;
        stage.members[1].gear.teeth = 17;
        stage.members[2].gear.teeth = 80;
        let stage = stage.clone();
        assert!(
            solve_alone(&Train::alone(&stage, 1.0, 0.0), &lib).is_err(),
            "this set is the one that cannot be built"
        );
        let b = Asked::conventional(&stage);
        let m = stage
            .system()
            .unwrap()
            .motion(&b.conditions(stage.wiring().slots.len()))
            .unwrap();
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
    /// holds that the two agree — each stage's ratio the path across its ends
    /// ([`across`]) — on magnitudes, since a referral takes the size.
    ///
    /// It also holds the thing a product cannot say: the **mobility** of the
    /// assembled train equals the number of conditions it is given, so a chain
    /// is neither over- nor under-determined by construction, and an
    /// arrangement that introduced a body nothing constrains would fail here rather than
    /// quietly widening the answer.
    #[test]
    fn the_chained_graph_agrees_with_the_product_of_the_presets_ratios() {
        let lib = library();
        let mut checked = 0u32;
        for train in [two_pairs(), mixed_train()] {
            let r = solve_train(&train, &lib).expect("these trains solve");
            let m = train.motion().expect("...and have a motion");

            // Exactly as many conditions as degrees of freedom: what the
            // train holds, and the one drive at its end its motion is read
            // from.
            let system = train.system().unwrap();
            let asked = train
                .conditions(system.bodies())
                .unwrap()
                .iter()
                .filter(|c| **c != crate::kinematics::Condition::Free)
                .count()
                + 1;
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

            let ratios: Vec<f64> = (0..train.parts().len())
                .map(|k| across(&train, k, &lib).ratio)
                .collect();
            for (k, stage) in ratios.iter().enumerate() {
                let (i, o) = part_ends(&train, k);
                let graph = m
                    .solution
                    .ratio(i, o)
                    .expect("every part here turns")
                    .to_f64();
                assert!(
                    (graph.abs() - stage.abs()).abs() < 1e-9 * stage.abs(),
                    "stage {k}: graph {graph} vs {stage}"
                );
                checked += 1;
            }
            let total = m.total.expect("the train turns").to_f64();
            assert!(
                (total.abs() - r.total().unwrap().ratio.abs()).abs()
                    < 1e-9 * r.total().unwrap().ratio.abs(),
                "total: graph {total} vs {}",
                r.total().unwrap().ratio
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
            for k in 0..train.parts().len() {
                let graph = m.solution.values[part_ends(&train, k).0].to_f64();
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
        let mut broken = arr::planetary(12, 30, 72, 3);
        broken.members[0].gear.teeth = 17;
        broken.members[1].gear.teeth = 17;
        broken.members[2].gear.teeth = 80;
        let mut train = two_pairs();
        train.chain_on(broken.clone());

        assert!(
            solve_train(&train, &lib).is_err(),
            "the third stage is the one that cannot be built"
        );
        let m = train.motion().expect("its motion needs none of that");
        let ratio = |k: usize| {
            let (i, o) = part_ends(&train, k);
            m.solution.ratio(i, o)
        };
        assert_eq!(train.parts().len(), 3);
        for k in 0..3 {
            assert!(ratio(k).is_some(), "part {k} turns");
        }
        // 17-tooth sun, 80-tooth ring, sun in and ring held: 97/17, exactly.
        assert_eq!(ratio(2), crate::ratio::Ratio::new(97, 17));
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
    ///   sources of lost motion add up whichever way their bodies turn — and a
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
    fn a_reversing_part_does_not_poison_the_parts_around_it() {
        let lib = library();
        // A set with its **carrier** held reverses. Which body is held is the
        // train's to say now, so it is a constraint on the train and the set
        // itself is the default one. Slot 2 is the carrier in the set's wiring.
        let reversing = || arr::planetary(12, 30, 72, 3);
        // The carrier held and nothing else: every hold is stated, so the
        // ring the set was inserted holding is released by not being
        // listed.
        let carrier_held = |t: &Train, stage: usize| vec![t.port(stage, 2)];

        // --- the set ahead of a pair, chained by its ring once its carrier
        // is held: the pair's end of the carrier split off, and the ring
        // joined to it. It used to refuse outright.
        let mut t = train_of(vec![reversing(), arr::pair([17, 43])]);
        t.held = carrier_held(&t, 0);
        t.split(1, t.port(0, 2));
        t.join(t.port(0, 3), t.port(1, 1)).unwrap();
        let r = solve_train(&t, &lib).expect("a reversing stage can be followed");
        assert!(across(&t, 0, &lib).ratio < 0.0, "this set reverses");
        for (k, s) in r.by_part.iter().enumerate() {
            for g in &s.members {
                assert!(
                    g.cases[0].torque >= 0.0,
                    "stage {k}: a referral is a magnitude, not {}",
                    g.cases[0].torque
                );
            }
        }

        // --- and behind one, where the play is referred through it: the
        // pair's output coupled to the sun still, and the ring the end.
        let mut t = train_of(vec![arr::pair([17, 43]), reversing()]);
        t.held = carrier_held(&t, 1);
        let (was, end) = (t.port(1, 2), t.port(1, 3));
        for c in &mut t.load_cases {
            for l in &mut c.loads {
                if l.at == was {
                    l.at = end;
                }
            }
            if let Duty::Intermittent { at, .. } = &mut c.duty {
                if *at == Some(was) {
                    *at = Some(end);
                }
            }
        }
        let r = solve_train(&t, &lib).expect("...and can follow one");
        let (first, second) = (across(&t, 0, &lib), across(&t, 1, &lib));
        let want = first.backlash.forward.unwrap().nominal / second.ratio.abs()
            + second.backlash.forward.unwrap().nominal;
        assert!(
            (r.total().unwrap().backlash.forward.unwrap().nominal - want).abs() < 1e-12,
            "play accumulates: {} vs {want}",
            r.total().unwrap().backlash.forward.unwrap().nominal
        );
        // ...and it is strictly more than the last stage alone, which is what
        // "subtracting" would have taken it below.
        assert!(
            r.total().unwrap().backlash.forward.unwrap().nominal
                > second.backlash.forward.unwrap().nominal
        );
    }

    /// **One body, one speed** — a stage's output member and the next stage's
    /// input member are the same piece of metal and must say the same thing.
    ///
    /// They did not. Every kind worked its members' speeds out for itself, and
    /// they disagreed about *sign*: a pair's second member came back positive
    /// while turning backwards (`Mesh::ratio` is a magnitude), where an
    /// epicyclic set's came back signed. So a two-stage train reported
    /// `+1186 rpm` at the end of stage 1 and `+1186 rpm` at the start of stage
    /// 2 — agreeing by accident — and a set in front of a pair reported
    /// `−500` and `+500` for one body.
    ///
    /// Now every member's motion comes from the train's one graph, and the
    /// coupling is a row in the same system.
    /// This is the law that says so, across three presets and two junctions.
    #[test]
    fn a_shaft_shared_by_two_parts_reports_one_speed() {
        let lib = library();
        let mut checked = 0u32;
        for train in [two_pairs(), mixed_train()] {
            let r = solve_train(&train, &lib).expect("these trains solve");
            let wirings: Vec<Wiring> = train.part_shapes().iter().map(Shape::wiring).collect();
            // Each part's two ends, by its own slots.
            let ends: Vec<(Body, Body)> = (0..train.parts().len())
                .map(|k| {
                    let (i, o) = part_ends(&train, k);
                    (train.slot(k, i).unwrap(), train.slot(k, o).unwrap())
                })
                .collect();
            // **Which member sits on a body is the wiring's answer, not the
            // member order's.** An epicyclic set's output is its *carrier*,
            // which carries no gear at all — the planet rides it and spins with
            // its own body — so "the stage's last member" is right for a pair
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
            for k in 1..r.by_part.len() {
                let before = &r.by_part[k - 1].members;
                let after = &r.by_part[k].members;
                for out in on(k - 1, ends[k - 1].1) {
                    for into in on(k, ends[k].0) {
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
            // body turns at, where one sits there.
            let k = r.by_part.len() - 1;
            let last = &r.by_part[k].members;
            for into in on(k, ends[k].1) {
                for c in &r.cases {
                    let member = &last[into].cases[c.case];
                    let delivered = c
                        .shaft(train.port(k, ends[k].1))
                        .and_then(|s| s.speed)
                        .expect("the far port turns");
                    assert!(
                        (member.speed - delivered).abs() < 1e-9 * delivered.abs().max(1.0),
                        "case {}: delivered {} but the member turns {}",
                        c.case + 1,
                        delivered,
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
        let set = arr::planetary(12, 30, 72, 3);
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
        // Read along a case between the chain's ends, switched on.
        let read = |mut t: Train| {
            let mut case = t.fresh_case(CaseKind::Ultimate, 2.0, 3000.0);
            case.enabled = true;
            t.load_cases = vec![case];
            solve_train(&t, &lib)
                .expect("solves")
                .total()
                .unwrap()
                .ratio
        };
        let one = |stage| read(train_of(vec![stage]));
        // One external mesh reverses; two do not.
        assert!(one(arr::pair([17, 43])) < 0.0);
        let mut two = two_pairs();
        two.load_cases.clear();
        assert!(read(two) > 0.0);
        // A worm is an external mesh like any other.
        assert!(one(arr::worm(1, 40)) < 0.0);
        // ...and an epicyclic set carries the sign its own kinematics gives:
        // sun in with the ring held turns the carrier the same way.
        assert!(one(arr::planetary(12, 30, 72, 3)) > 0.0);
    }

    /// **Holds are stated, and a shared body says where a stage is
    /// entered.** A set is inserted holding its ring; holding its carrier
    /// instead is two statements — the ring released, the carrier held —
    /// each on the page. A set behind a pair, coupled to it by its carrier, is
    /// entered at the carrier and leaves by the sun; coupled by its ring
    /// with its sun held, it is entered at the ring and leaves by the
    /// carrier at the ring-in ratio, not by the ring at a ratio of one.
    /// Nothing is driven but by a load, and a statement about a body is
    /// about that body: holding one that runs on to a second stage holds
    /// that stage's end of it too, and it is a split that says otherwise.
    #[test]
    fn holds_are_stated_and_a_shared_body_says_where_a_part_is_entered() {
        let lib = library();
        let set = || arr::planetary(12, 30, 72, 3);
        let (sun, carrier, ring) = (1, 2, 3);

        // --- two lines: the ring released, the carrier held — and the set,
        // chained onward by its carrier, holds the pair's gear with it, so
        // the pair's end is split off and the ring is what it runs on by.
        let mut t = train_of(vec![set(), arr::pair([17, 43])]);
        assert_eq!(t.held, vec![t.port(0, ring)], "inserted holding its ring");
        t.release(t.port(0, ring));
        t.hold(t.port(0, carrier));
        assert_eq!(t.held, vec![t.port(0, carrier)]);
        assert_eq!(
            t.ends_of(t.port(0, carrier)).len(),
            2,
            "the pair is held with it"
        );
        t.split(1, t.port(0, carrier));
        t.join(t.port(0, ring), t.port(1, 1)).unwrap();
        solve_train(&t, &lib).expect("solves");
        assert!(across(&t, 0, &lib).ratio < 0.0, "carrier held reverses");

        // --- behind a pair, joined by its carrier: entered there, leaving
        // by the sun, and the pair before it is what it was.
        let mut t = Train::chained(vec![arr::pair([17, 43]), set()], |_| Vec::new());
        t.split(1, t.port(1, sun));
        t.join(t.port(0, 2), t.port(1, carrier)).unwrap();
        let (start, end) = ends_of(&t);
        assert_eq!((start, end), (t.port(0, 1), t.port(1, sun)));
        t.load_cases = vec![LoadCase::ultimate(start, end, 2.0, 3000.0)];
        let conditions = t.conditions(t.max_body() + 1).unwrap();
        assert_eq!(
            conditions[t.port(1, carrier)],
            crate::kinematics::Condition::Free,
            "turned by the pair, not driven"
        );
        let r = solve_train(&t, &lib).expect("solves behind a pair");
        assert!(
            across(&t, 1, &lib).ratio.abs() < 1.0,
            "a carrier-driven set speeds up"
        );
        assert!(r.cases[0].solved);

        // ...and joined by its ring with its sun held: entered at the ring,
        // leaving by the carrier at the ring-in ratio.
        t.split(1, t.port(1, carrier));
        t.join(t.port(0, 2), t.port(1, ring)).unwrap();
        t.held = vec![t.port(1, sun)];
        let (start, end) = ends_of(&t);
        assert_eq!(end, t.port(1, carrier));
        t.load_cases = vec![LoadCase::ultimate(start, end, 2.0, 3000.0)];
        let r = solve_train(&t, &lib).expect("solves ring-in behind a pair");
        let set_ratio = across(&t, 1, &lib).ratio;
        assert!(
            set_ratio > 1.0 && set_ratio < 2.0,
            "ring in, sun held: {set_ratio}"
        );
        assert!(r.cases[0].solved);
    }

    /// **A case's loads decide exactly the train's mobility, and relief
    /// keeps it so.** On a pair — one degree of freedom — a second given
    /// speed turns the first derived, the one just touched kept; a load
    /// added with everything derived is seeded from what the case came to;
    /// and a case short of a speed is left short and says so, rather than
    /// given one it did not state. On a differential — a set with its ring
    /// released, two degrees — two given speeds stand and one torque: three
    /// loads, nothing reacted, is one statics equation short of three, and
    /// the case solves.
    #[test]
    fn a_case_is_relieved_to_the_trains_mobility() {
        let lib = library();
        let mut t = two_pairs();
        t.load_cases.truncate(1);
        assert_eq!(t.case_mobility().unwrap(), 1);
        // The end a load with both figures given, in place of its reaction:
        // two given speeds on one degree of freedom, and the one just
        // touched is kept.
        let end = end_of(&t);
        t.load_cases[0].loads[1] = Load::given(end, 1.0, 100.0);
        t.relieve_case(
            0,
            Some(CaseFreedom {
                load: 1,
                which: LoadFreedom::Speed,
            }),
            &lib,
        )
        .unwrap();
        let c = &t.load_cases[0];
        // The speed just touched stands and the start's is derived; of the
        // torques, nothing was touched, so the last load's gives way.
        assert!(c.loads[0].speed.auto && !c.loads[1].speed.auto);
        assert!(!c.loads[0].torque.auto && c.loads[1].torque.auto);
        // ...and the derived figures are what the case comes to: the start
        // turns at the end's speed times the ratio, and the end carries the
        // start's torque stepped up through the losses.
        let r = solve_train(&t, &lib).unwrap();
        let start = at_port(&r, &t, 0, start_of(&t));
        let end = at_port(&r, &t, 0, end_of(&t));
        assert!(start.speed.unwrap().abs() > 100.0);
        assert!((c.loads[0].speed.manual - start.speed.unwrap()).abs() < 1e-3);
        assert!((c.loads[1].torque.manual - end.torque).abs() < 1e-3);
        // Everything derived: nothing drives, and relief invents nothing.
        let mut short = two_pairs();
        short.load_cases = vec![LoadCase {
            loads: vec![Load::derived(start_of(&t))],
            ..template(&t)
        }];
        short.relieve_case(0, None, &lib).unwrap();
        assert!(short.load_cases[0].loads[0].speed.auto);
        let r = solve_train(&short, &lib).unwrap();
        assert!(!r.cases[0].solved);
        assert!(r.cases[0]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_CASE_UNDERDETERMINED)));
        // A differential: a lone set with its ring released has two.
        let mut diff = train_of(vec![arr::planetary(12, 30, 72, 3)]);
        diff.release(diff.port(0, 3));
        assert_eq!(diff.case_mobility().unwrap(), 2);
        // The released ring is `End` by convention; the carrier by reference.
        diff.load_cases = vec![LoadCase {
            loads: vec![
                Load::given(start_of(&t), 2.0, 3000.0),
                Load::given(diff.port(0, 3), 1.0, -500.0),
                Load::derived(diff.port(0, 2)),
            ],
            ..template(&t)
        }];
        diff.relieve_case(0, None, &lib).unwrap();
        let c = &diff.load_cases[0];
        assert!(!c.loads[0].speed.auto && !c.loads[1].speed.auto && c.loads[2].speed.auto);
        assert!(!c.loads[0].torque.auto && c.loads[1].torque.auto && c.loads[2].torque.auto);
        let r = solve_train(&diff, &lib).unwrap();
        assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);
    }

    /// **A given torque is a load whichever way it works.** A pair with the
    /// start's torque given, the end's speed given and the other two figures
    /// derived, the start turning *against* its torque: the start absorbs
    /// what it is given, the end is what drives it, and the end's derived
    /// torque is the start's stepped through the ratio and *up* by the loss
    /// — the driver pays it. Reversed, the same case with the start driving
    /// puts the loss on the end. A case whose only given torque is nought
    /// does no work and says so rather than dividing by it.
    #[test]
    fn a_given_torque_that_works_against_its_port_is_driven_by_what_is_derived() {
        let lib = library();
        let mut t = two_pairs();
        let one = t.part_shapes()[..1].to_vec();
        rechain(&mut t, one);
        let ratio = 43.0 / 17.0;
        let case = |torque: f64, speed: f64| -> (Train, Solved) {
            let mut t = t.clone();
            t.load_cases = vec![LoadCase {
                loads: vec![
                    Load {
                        at: start_of(&t),
                        role: LoadRole::Load,
                        torque: Auto::fixed(torque),
                        speed: Auto::automatic(0.0),
                    },
                    Load {
                        at: end_of(&t),
                        role: LoadRole::Load,
                        torque: Auto::automatic(0.0),
                        speed: Auto::fixed(speed),
                    },
                ],
                ..template(&t)
            }];
            let r = solve_train(&t, &lib).unwrap();
            (t, r)
        };
        let eta = case(0.1, 100.0).1.by_part[0].meshes[0].efficiency.forward;
        assert!(eta > 0.9 && eta < 1.0);
        // The pair's ratio is negative: at +100 rpm at the end, the start
        // turns backward, and a positive torque there works against it.
        let (t, absorbing) = case(0.1, 100.0);
        let c = &absorbing.cases[0];
        assert!(c.solved, "{:?}", c.notes);
        let start = at_port(&absorbing, &t, 0, start_of(&t));
        let end = at_port(&absorbing, &t, 0, end_of(&t));
        assert!(start.speed.unwrap() < 0.0);
        assert!(
            (end.torque - 0.1 * ratio / eta).abs() < 1e-9,
            "{}",
            end.torque
        );
        // The same torque driving: the start turns with it, and the end's
        // torque is the start's through the ratio *less* the loss.
        let (t, driving) = case(-0.1, 100.0);
        let end = at_port(&driving, &t, 0, end_of(&t));
        assert!(driving.cases[0].solved);
        assert!(
            (end.torque.abs() - 0.1 * ratio * eta).abs() < 1e-9,
            "{}",
            end.torque
        );
        // Nought given: nothing drives, by name.
        let (_, nought) = case(0.0, 100.0);
        assert!(!nought.cases[0].solved);
        assert!(nought.cases[0]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_CASE_NOTHING_DRIVES)));
    }

    /// **A train whose every case carries nothing still solves, its
    /// automatic widths at their boxes.** A differential with one given
    /// speed per case is one speed short in every case; each says so and
    /// carries nought on every mesh, and a width asked of a load of nought
    /// is no width asked — the same answer as a train with no case, rather
    /// than a face of no width for the contact to refuse.
    #[test]
    fn a_train_no_case_of_which_solves_keeps_its_automatic_widths_at_their_boxes() {
        let lib = library();
        let mut t = train_of(vec![arr::pair([17, 43]), arr::planetary(12, 30, 72, 3)]);
        t.release(t.port(1, 3));
        let (start, end) = ends_of(&t);
        t.load_cases = vec![LoadCase::ultimate(start, end, 0.1, 30000.0)];
        {
            for m in &mut t.shape.members {
                m.gear.face_width = Auto::automatic(7.0);
            }
        }
        let r = solve_train(&t, &lib).expect("a case carrying nothing refuses nothing");
        assert!(r.paths.is_empty());
        assert!(!r.cases[0].solved);
        for s in &r.by_part {
            for g in &s.members {
                assert_eq!(g.face_width, 7.0);
            }
        }
    }

    /// **A port is what the case declares it, and relief turns only the
    /// loads.** A pair's end declared *free* beside a given start torque is
    /// the question whether the train turns under the load, which relief
    /// leaves standing — the start's torque is not one too many, since a
    /// free port is no unknown — and the solve answers by name. The same
    /// end declared *reacted* in so many words is what an unlisted end is.
    /// On a differential, a ring declared reacted is one unknown torque
    /// among the three ports, so of the sun's and the carrier's loads one
    /// torque stands given; and the declared entries' own figures are
    /// never turned.
    #[test]
    fn a_declared_port_is_what_it_says_and_relief_leaves_it_alone() {
        let lib = library();
        let mut t = two_pairs();
        let one = t.part_shapes()[..1].to_vec();
        rechain(&mut t, one);
        t.load_cases = vec![LoadCase {
            loads: vec![
                Load::given(start_of(&t), 0.1, 3000.0),
                Load::declared(end_of(&t), LoadRole::Free),
            ],
            ..template(&t)
        }];
        t.relieve_case(0, None, &lib).unwrap();
        let c = &t.load_cases[0];
        assert!(!c.loads[0].torque.auto && !c.loads[0].speed.auto);
        assert_eq!(c.loads[1].role, LoadRole::Free);
        let r = solve_train(&t, &lib).unwrap();
        assert!(!r.cases[0].solved);
        assert!(r.cases[0]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_LOAD_NOT_REACTED)));
        assert_eq!(at_port(&r, &t, 0, end_of(&t)).role, BodyRole::Free);
        // Reacted in so many words: the case solves, and the end holds the
        // load — where a port the case says nothing of is free, and a load
        // at the start alone is one nothing holds.
        t.load_cases[0].loads[1] = Load::declared(end_of(&t), LoadRole::Reacted);
        let said = solve_train(&t, &lib).unwrap();
        assert!(said.cases[0].solved);
        assert_eq!(at_port(&said, &t, 0, end_of(&t)).role, BodyRole::Reacted);
        assert!(at_port(&said, &t, 0, end_of(&t)).torque.abs() > 0.0);
        t.load_cases[0].loads.truncate(1);
        let unsaid = solve_train(&t, &lib).unwrap();
        assert!(!unsaid.cases[0].solved);
        assert_eq!(at_port(&unsaid, &t, 0, end_of(&t)).role, BodyRole::Free);
        // A differential's ring declared reacted: one unknown among three
        // ports, so one of the two loads' torques gives way — the last.
        let mut diff = train_of(vec![arr::planetary(12, 30, 72, 3)]);
        let at = |slot| diff.port(0, slot);
        let (b1, b2, b3) = (at(1), at(2), at(3));
        let at = |slot: usize| [b1, b2, b3][slot - 1];
        diff.release(at(3));
        diff.load_cases = vec![LoadCase {
            loads: vec![
                Load::given(at(1), 2.0, 3000.0),
                Load::given(at(2), 1.0, -500.0),
                Load::declared(at(3), LoadRole::Reacted),
            ],
            ..template(&t)
        }];
        diff.relieve_case(0, None, &lib).unwrap();
        let c = &diff.load_cases[0];
        assert!(!c.loads[0].torque.auto && c.loads[1].torque.auto);
        assert!(!c.loads[0].speed.auto && !c.loads[1].speed.auto);
        assert_eq!(c.loads[2].role, LoadRole::Reacted);
        let r = solve_train(&diff, &lib).unwrap();
        assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);
        let ring = r.cases[0].shaft(at(3)).unwrap();
        assert_eq!(ring.role, BodyRole::Reacted);
        assert!(ring.torque.abs() > 0.0);
    }

    /// **The last stage removed leaves the cases parked at their bodies,
    /// and the first stage pushed takes them up conventionally.** A pair's
    /// three cases, the pair removed, keep every figure; a set pushed in its
    /// place has them sun in, carrier out, the sweep at the carrier — the
    /// set's conventional use — and solves them.
    #[test]
    fn the_cases_survive_an_emptied_train_and_take_the_next_preset_conventionally() {
        let lib = library();
        let mut t = train_of(vec![arr::pair([17, 43])]);
        t.load_cases.truncate(1);
        t.load_cases[0].set_torque(0.7);
        // ...and a back-driving case beside it, its load at the output.
        let (start, end) = ends_of(&t);
        t.load_cases.push(LoadCase::back_driving(start, end, 3.0));
        // The pair's mesh taken out takes both its gears, and the train is
        // empty.
        t.edit(Edit::Remove(Piece::Mesh(0))).unwrap();
        assert!(t.parts().is_empty());
        let r = solve_train(&t, &lib).expect("a train with nothing in it solves");
        assert!(!r.cases[0].solved);
        assert_eq!(t.load_cases[0].loads[0].at, 1, "parked where it was");
        assert_eq!(t.load_cases[0].loads[1].at, 2);
        assert!(
            (t.load_cases[0].torque().unwrap() - 0.7).abs() < 1e-12,
            "the figures are kept"
        );
        t.chain_on(arr::planetary(12, 30, 72, 3));
        let at = |slot| t.port(0, slot);
        let c = &t.load_cases[0];
        assert_eq!((c.loads[0].at, c.loads[0].role), (at(1), LoadRole::Load));
        assert_eq!((c.loads[1].at, c.loads[1].role), (at(2), LoadRole::Reacted));
        assert!(matches!(c.duty, Duty::Intermittent { at: d, .. } if d == Some(at(2))));
        // The back-driving case is still from the output, held at the input.
        let b = &t.load_cases[1];
        assert_eq!((b.loads[0].at, b.loads[0].role), (at(2), LoadRole::Load));
        assert_eq!((b.loads[1].at, b.loads[1].role), (at(1), LoadRole::Reacted));
        let r = solve_train(&t, &lib).expect("solves on the set");
        assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);
        assert!(r.cases[1].solved, "{:?}", r.cases[1].notes);
        // ...and a fresh case on an emptied train is parked the same: the
        // set's planet axis taken out takes every gear on and round it.
        t.edit(Edit::Remove(Piece::Axis(1))).unwrap();
        assert!(t.parts().is_empty());
        let fresh = t.fresh_case(CaseKind::Fatigue, 1.0, 100.0);
        assert_eq!(fresh.loads[0].at, 1);
        assert_eq!(fresh.loads[1].at, 2);
        assert!(
            !fresh.enabled,
            "a fresh case moves no rating until it is switched on"
        );
    }

    /// **A train's figures are per path, and a path crossing one stage is
    /// that stage's.** Three pairs in a chain, loaded end to end, have one
    /// path, the ends, whatever else their four open bodies could pair; a
    /// case loading the first take-off from the first gear and one loading
    /// the second from the first add one each — a case switched off adds
    /// none, and the same pair asked twice is one row. The path from the
    /// first gear to the first take-off is stage 1 alone — its ratio, its
    /// efficiency both ways and its play at the take-off — and the one
    /// between the take-offs is stage 2 alone, which a product over the
    /// list could not say. The two-end path is the three stages' product. A
    /// differential has no path: a ratio between two of its ports needs a
    /// third held.
    #[test]
    fn a_trains_figures_are_per_path_and_a_path_across_one_preset_is_that_preset() {
        let lib = library();
        let mut t = train_of(vec![
            arr::pair([17, 43]),
            arr::pair([17, 43]),
            arr::pair([17, 43]),
        ]);
        // Three pairs chained: bodies 1 to 4, stage k's slot s at k + s.
        let at = |stage: usize, slot: usize| stage + slot;
        for (k, s) in [(0, 1), (0, 2), (1, 1), (1, 2), (2, 1), (2, 2)] {
            assert_eq!(t.port(k, s), at(k, s));
        }
        let ends_only = solve_train(&t, &lib).unwrap();
        assert_eq!(ends_only.paths.len(), 1, "the ends, and nothing unasked");
        t.load_cases
            .push(LoadCase::ultimate(at(0, 1), at(0, 2), 1.0, 1000.0));
        t.load_cases
            .push(LoadCase::ultimate(at(0, 2), at(1, 2), 1.0, 1000.0));
        t.load_cases
            .push(LoadCase::ultimate(at(1, 1), at(2, 1), 1.0, 1000.0));
        let mut off = LoadCase::ultimate(at(1, 2), at(2, 2), 1.0, 1000.0);
        off.enabled = false;
        t.load_cases.push(off);
        let r = solve_train(&t, &lib).unwrap();
        assert_eq!(r.paths.len(), 3, "the ends, two asked, one twice, one off");
        assert_eq!((r.paths[0].from, r.paths[0].to), (at(0, 1), at(2, 2)));
        let near = |x: f64, y: f64| (x - y).abs() < 1e-9 * x.abs().max(1e-12);
        // Each pair asked alone: a path across one stage of three is rated on
        // that stage and nothing else.
        let s: Vec<Alone> = t
            .part_shapes()
            .iter()
            .map(|x| solve_alone(&Train::alone(x, 1.0, 1000.0), &lib).unwrap())
            .collect();
        assert!(near(
            r.paths[0].ratio,
            s.iter().map(|x| x.ratio.unwrap()).product::<f64>()
        ));
        assert!(near(
            r.paths[0].efficiency.forward.unwrap(),
            s.iter()
                .map(|x| x.efficiency.unwrap().forward)
                .product::<f64>()
        ));
        let one = r
            .paths
            .iter()
            .find(|p| (p.from, p.to) == (at(0, 1), at(0, 2)))
            .expect("first gear to the first take-off");
        assert!(near(one.ratio, s[0].ratio.unwrap()));
        assert!(near(
            one.efficiency.forward.unwrap(),
            s[0].efficiency.unwrap().forward
        ));
        assert!(near(
            one.efficiency.backward.unwrap(),
            s[0].efficiency.unwrap().backward
        ));
        assert!(near(
            one.backlash.forward.unwrap().nominal,
            s[0].backlash.unwrap().forward.nominal
        ));
        assert!(near(
            one.backlash.backward.unwrap().nominal,
            s[0].backlash.unwrap().backward.nominal
        ));
        let mid = r
            .paths
            .iter()
            .find(|p| (p.from, p.to) == (at(0, 2), at(1, 2)))
            .expect("between the take-offs");
        assert!(near(mid.ratio, s[1].ratio.unwrap()));
        assert!(near(
            mid.efficiency.forward.unwrap(),
            s[1].efficiency.unwrap().forward
        ));
        assert!(near(
            mid.backlash.forward.unwrap().nominal,
            s[1].backlash.unwrap().forward.nominal
        ));
        // A differential has no path.
        let mut diff = train_of(vec![arr::planetary(12, 30, 72, 3)]);
        diff.release(diff.port(0, 3));
        let r = solve_train(&diff, &lib).unwrap();
        assert!(r.paths.is_empty());
    }

    /// **The train's bodies are numbered across it, each with its ends**:
    /// three pairs in a chain are four bodies, the two inner ones with an
    /// end on each of two parts, and only the outer two are ends of the
    /// train; a set's held ring is a body no case can name.
    #[test]
    fn the_bodies_are_numbered_across_the_train_each_with_its_ends() {
        let mut t = train_of(vec![
            arr::pair([17, 43]),
            arr::pair([17, 43]),
            arr::pair([17, 43]),
        ]);
        let bodies = t.bodies();
        let ends: Vec<(usize, Vec<usize>)> = bodies
            .iter()
            .map(|b| (b.body, t.ends_of(b.body).iter().map(|(k, _)| *k).collect()))
            .collect();
        assert_eq!(
            ends,
            vec![(1, vec![0]), (2, vec![0, 1]), (3, vec![1, 2]), (4, vec![2])]
        );
        for (k, slot, body) in [
            (0, 1, 1),
            (0, 2, 2),
            (1, 1, 2),
            (1, 2, 3),
            (2, 1, 3),
            (2, 2, 4),
        ] {
            assert_eq!(t.port(k, slot), body);
        }
        assert_eq!(ends_of(&t), (1, 4));
        assert!(bodies.iter().all(|b| !b.held));
        rechain(&mut t, vec![arr::planetary(12, 30, 72, 3)]);
        let bodies = t.bodies();
        assert_eq!(bodies.len(), 3, "the planet is no body a case can name");
        assert!(bodies[2].held);
        assert_eq!(t.chain_ends(), Some((1, 2)));
    }

    /// **A load between two stages is one flow across both**, and where it
    /// can be held decides what it does: a load on the sun a pair drives,
    /// with the pair's input reacted and the set's carrier reacted, is a
    /// load two bodies could hold — the division by stiffness this model
    /// does not make, which the train refuses by name; with the pair's input
    /// a load of nought — free, carrying nothing — the carrier holds it and
    /// every stage between carries it; and a load at a body that is not a
    /// port is refused by name.
    #[test]
    fn a_load_between_two_parts_is_held_by_what_can_hold_it() {
        let lib = library();
        let (sun, carrier, ring) = (1, 2, 3);
        let t = train_of(vec![arr::pair([17, 43]), arr::planetary(12, 30, 72, 3)]);
        let at = |stage, slot| t.port(stage, slot);
        // Both ends reacted: the sun's load could be held by either.
        let mut shared = t.clone();
        shared.load_cases[1] = LoadCase {
            loads: vec![
                Load::given(at(1, sun), 0.5, 0.0),
                Load::declared(start_of(&t), LoadRole::Reacted),
                Load::declared(end_of(&t), LoadRole::Reacted),
            ],
            ..template(&t)
        };
        let r = solve_train(&shared, &lib).expect("a shared load is a case's question");
        assert!(!r.cases[1].solved);
        assert!(r.cases[1]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_LOAD_SHARED)));
        // The start declared free: the carrier alone holds it, through the
        // set — and the pair, with nothing at either end, carries none.
        shared.load_cases[1].loads[1] = Load::declared(start_of(&t), LoadRole::Free);
        let r = solve_train(&shared, &lib).expect("the carrier holds it");
        assert!(r.cases[1].solved);
        let end = r.cases[1].shaft(at(1, carrier)).unwrap();
        assert_eq!(end.role, BodyRole::Reacted);
        assert!(
            end.torque.abs() > 0.5,
            "the carrier holds the sun's load stepped up"
        );
        for g in &r.by_part[0].members {
            assert_eq!(g.cases[1].torque, 0.0, "the pair carries none of it");
        }
        // A body that is not a port.
        let mut held = t.clone();
        held.load_cases[2].set_port(at(1, ring));
        assert_eq!(
            solve_train(&held, &lib).err(),
            Some(TrainError::LoadPort { case: 2 })
        );

        // A set at the head of a chain, coupled onward by its sun and loaded
        // at its carrier: the load walks the set backward, and the train's
        // ratio is read between its two ends, the carrier and the pair's
        // output.
        let mut t = Train::chained(
            vec![arr::planetary(12, 30, 72, 3), arr::pair([17, 43])],
            |_| Vec::new(),
        );
        t.split(1, t.port(0, carrier));
        t.join(t.port(0, sun), t.port(1, 1)).unwrap();
        let (start, end) = ends_of(&t);
        assert_eq!(start, t.port(0, carrier));
        t.load_cases = vec![LoadCase::ultimate(start, end, 2.0, 3000.0)];
        let r = solve_train(&t, &lib).expect("carrier-driven set at the head");
        assert!(
            r.total().unwrap().ratio.abs() < 1.0,
            "a carrier-driven set multiplies speed"
        );
        assert!(r.cases[0].solved);
    }

    /// **The open ports are what a load can enter by**: every body the
    /// train does not hold, once — a body two parts share is a take-off
    /// between them — and a released ring,
    /// un-held and shared with nothing, is another. A chain has two ends; a
    /// set with its ring released has three open ports and no ends.
    #[test]
    fn the_open_ports_are_the_bodies_the_train_does_not_hold() {
        let t = mixed_train();
        let at = |stage, slot| t.port(stage, slot);
        let ports = t.open_ports();
        assert_eq!(
            ports
                .iter()
                .map(|p| (p.body, t.ends_of(p.body)[0].0))
                .collect::<Vec<_>>(),
            vec![
                (at(0, 1), 0),
                (at(0, 2), 0),
                (at(1, 2), 1),
                (at(2, 2), 2),
                (at(3, 2), 3)
            ]
        );
        assert_eq!(
            t.chain_ends(),
            Some((at(0, 1), at(3, 2))),
            "the chain's ends, with three take-offs between them"
        );
        let mut t = train_of(vec![arr::planetary(12, 30, 72, 3)]);
        assert_eq!(ends_of(&t), (1, 2));
        t.release(3);
        let ports = t.open_ports();
        assert_eq!(
            ports.iter().map(|p| p.body).collect::<Vec<_>>(),
            vec![1, 2, 3]
        );
        let (k, slot) = t.ends_of(3)[0];
        assert_eq!(
            t.parts()[k].shape.wiring().slots[slot],
            BodyLabel::Member { member: 2 }
        );
        assert_eq!(
            t.chain_ends(),
            Some((1, 2)),
            "a set's ends by convention, whatever its ring does"
        );
    }

    /// **A train one condition short reports the family, has no figure of
    /// its own, and rates its cases.** A set with its ring released and its
    /// sun driven — a differential — has one free parameter; every body's
    /// speed is a particular value plus one term per turn of the free body,
    /// and at every value of that parameter the family satisfies Willis —
    /// `z_s ω_s + z_r ω_r = (z_s + z_r) ω_c` — exactly, with the sun at one
    /// turn throughout. The report's terms are the same family read as
    /// floats. The train's ratio, efficiency and play, and the stage's, are
    /// `None`: each is read under one motion. A case that gives two speeds
    /// and one torque decides its own motion and is rated — the carrier at
    /// the tooth-weighted mean of the sun's and the ring's, the ring's derived
    /// torque what holds the sun's through the set — and a case one speed
    /// short says so and rates nothing.
    #[test]
    fn a_train_one_condition_short_reports_the_family_and_rates_its_cases() {
        let lib = library();
        let mut t = train_of(vec![arr::planetary(12, 30, 72, 3)]);
        t.release(t.port(0, 3));
        let (sun, carrier, ring) = (1, 2, 3);
        // A lone stage's bodies are numbered as its slots are.
        let at = |slot| slot;
        // The ring, released, is the chain's `End` by convention — the
        // next free port in order — so the carrier is loaded by reference.
        // (A load used to name the output; it does not, since that moved
        // `End` from one body to another as loads were added.)
        t.load_cases = vec![
            LoadCase {
                loads: vec![
                    Load::given(start_of(&t), 2.0, 3000.0),
                    Load::given(at(ring), 0.0, -600.0),
                    Load::derived(at(carrier)),
                ],
                ..template(&t)
            },
            LoadCase {
                loads: vec![Load::given(start_of(&t), 2.0, 3000.0)],
                ..template(&t)
            },
        ];
        // The ring's torque is given as nought here and is one too many:
        // relief turns it derived, and the case is two speeds and one torque.
        t.relieve_case(0, None, &lib).unwrap();
        assert!(t.load_cases[0].loads[1].torque.auto);
        let r = solve_train(&t, &lib).expect("a family is rated under its cases");
        assert!(r.paths.is_empty(), "a family has no path");
        let zs = f64::from(t.part_shapes()[0].gears()[0].teeth);
        let zr = f64::from(t.part_shapes()[0].gears()[2].teeth);
        let c = &r.cases[0];
        assert!(c.solved, "{:?}", c.notes);
        let w = |shaft| c.shaft(at(shaft)).unwrap().speed.unwrap();
        assert!(((zs * w(sun) + zr * w(ring)) / (zs + zr) - w(carrier)).abs() < 1e-9);
        // Statics: the three external torques sum to nought against ground,
        // and every member is rated at a torque of its own.
        let tq = |shaft| c.shaft(at(shaft)).unwrap().torque;
        assert!((tq(sun) - 2.0).abs() < 1e-12);
        assert!(tq(ring).abs() > 0.0 && tq(carrier).abs() > 0.0);
        for g in &r.by_part[0].members {
            assert!(g.cases[0].torque.abs() > 0.0, "{:?}", g.cases);
        }
        // One speed short: the case says so.
        assert!(!r.cases[1].solved);
        assert!(r.cases[1]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_CASE_UNDERDETERMINED)));
        let m = t.motion().expect("the motion is a family, not a refusal");
        assert_eq!(m.solution.residual.len(), 1, "{:?}", m.solution);
        let r = t.motion_report().expect("...and it is reported");
        assert_eq!(r.free.len(), 1);
        assert!(r.total.is_none(), "a family has no quotient");
        let (sun, carrier, ring) = (1, 2, 3);
        let z =
            |k: usize| crate::ratio::Ratio::whole(i64::from(t.part_shapes()[0].gears()[k].teeth));
        let (zs, zr) = (z(0), z(2));
        for p in [0i64, 1, -2, 7] {
            let p = crate::ratio::Ratio::whole(p);
            let at = |shaft: usize| {
                let i = t.port(0, shaft);
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
                let s = &r.bodies[t.port(0, shaft)];
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

    /// **Every other way the conditions can fail to give one motion is
    /// named**: a body asked two things that contradict, a body the train
    /// does not have, and tooth counts whose product outgrows an exact ratio
    /// — and none of them is the wiring sentence, which describes a stage
    /// that is no mechanism.
    #[test]
    fn a_conflict_a_missing_shaft_and_an_overflow_are_each_named() {
        let lib = library();
        let set = |held: Vec<usize>| {
            let mut t = train_of(vec![arr::planetary(12, 30, 72, 3)]);
            t.held = held;
            t
        };
        // Holding the carrier and the ring is a locked set, and the sun
        // driven against it is the conflict — named at the last statement
        // the designer made, the ring.
        assert_eq!(
            solve_train(&set(vec![2, 3]), &lib).err(),
            Some(TrainError::Overdetermined { at: 3 })
        );
        // A hold at a body the train does not have is refused where it
        // enters, by the hold's own field (`crate::input`).
        let missing = solve_train(&set(vec![7]), &lib).err();
        assert!(
            matches!(&missing, Some(TrainError::Input(r)) if r.field == "held.0"),
            "{missing:?}"
        );
        let huge = |teeth| MemberGear {
            teeth,
            ..MemberGear::default()
        };
        let mut wide = two_pairs();
        rechain(
            &mut wide,
            (0..6)
                .map(|k| {
                    let mut s = arr::pair([17, 43]);
                    s.members[0].gear = huge(4_000_000_000 + k);
                    s.members[1].gear = huge(4_000_000_001 + k);
                    s
                })
                .collect(),
        );
        assert_eq!(solve_train(&wide, &lib).err(), Some(TrainError::Overflow));
        assert!(wide.motion_report().is_none());
    }

    /// Every freedom a stage's groups mention, flat.
    fn mentioned(stage: &Shape) -> Vec<Freedom> {
        stage
            .freedoms()
            .into_iter()
            .flat_map(|g| g.order.into_iter().flatten())
            .collect()
    }

    /// **A member's result carries the tooth it was cut with**, agreeing with
    /// every figure the result quotes beside it — so a gear tab that adopts
    /// the member shows the tooth the stage rated, and not a rebuild from the
    /// inputs that could drift from it. On every arrangement, every member: the
    /// shift, the addendum and the helix in `params` are the ones in force,
    /// the count and module are the stage's, and a rack-cut member rebuilt
    /// from `params` alone is the reported pitch diameter; a ring is the
    /// member its stage cuts with a pinion cutter, and only that member.
    #[test]
    fn a_members_result_carries_the_tooth_it_was_cut_with() {
        let lib = library();
        let mut checked = 0u32;
        for stage in shapes() {
            let t = train_of(vec![stage.clone()]);
            let r = solve_train(&t, &lib).expect("every preset solves");
            let inputs = stage.gears();
            for (i, g) in r.members.iter().enumerate() {
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
            let expect: Vec<usize> = (0..stage.members.len())
                .filter(|&i| stage.members[i].ring.is_some())
                .collect();
            assert_eq!(
                rings, expect,
                "which members {stage:?} cuts with a pinion cutter"
            );
        }
        let members: usize = shapes().iter().map(|s| s.gears().len()).sum();
        assert_eq!(checked, u32::try_from(members).unwrap());
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
    fn every_declared_freedom_names_an_input_the_shape_has() {
        for stage in shapes() {
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
        for stage in shapes() {
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

    /// **A given input moved by a step the solve can follow**, each kind by
    /// its own: small beside the preset's value, large beside a solve's
    /// resolution.
    fn nudge(f: Freedom, a: &mut Auto<f64>) {
        match f {
            Freedom::Distance(_) => a.manual += 0.2,
            Freedom::Clearance(_) => a.manual += 0.01,
            Freedom::Overlap(_) => a.manual += 0.1,
            Freedom::Member(_, MemberFreedom::PitchDiameter) => a.manual *= 1.05,
            Freedom::Member(_, MemberFreedom::Shift) => a.manual += 0.05,
            Freedom::Member(_, MemberFreedom::Helix) => a.manual += 2.0,
            Freedom::Member(_, MemberFreedom::FaceWidth) => a.manual += 1.0,
            Freedom::Member(_, MemberFreedom::ThicknessMod) => a.manual += 0.1,
            // Small: a layshaft's pairs share one distance, and a module
            // moved a tenth is further than the others' shifts can follow.
            Freedom::Member(_, MemberFreedom::Module) => a.manual *= 1.002,
            Freedom::Member(_, MemberFreedom::PressureAngle) => a.manual += 0.2,
        }
    }

    /// Every kind of freedom, by name: what a sweep over freedoms reports
    /// it reached.
    const FREEDOM_KINDS: [&str; 10] = [
        "Distance",
        "Clearance",
        "Overlap",
        "Shift",
        "Helix",
        "PitchDiameter",
        "FaceWidth",
        "ThicknessMod",
        "Module",
        "PressureAngle",
    ];

    fn kind_of(f: Freedom) -> &'static str {
        match f {
            Freedom::Distance(_) => "Distance",
            Freedom::Clearance(_) => "Clearance",
            Freedom::Overlap(_) => "Overlap",
            Freedom::Member(_, m) => match m {
                MemberFreedom::Shift => "Shift",
                MemberFreedom::Helix => "Helix",
                MemberFreedom::PitchDiameter => "PitchDiameter",
                MemberFreedom::FaceWidth => "FaceWidth",
                MemberFreedom::ThicknessMod => "ThicknessMod",
                MemberFreedom::Module => "Module",
                MemberFreedom::PressureAngle => "PressureAngle",
            },
        }
    }

    /// `stage` with every input pinned at what the stage solved to, as the
    /// panel seeds a box a designer pins — so pinning everything pins the
    /// design the stage already was, not a distance of nought.
    fn pinned_as_solved(stage: &Shape, lib: &MaterialLibrary) -> Shape {
        let solved =
            solve_train(&train_of(vec![stage.clone()]), lib).expect("every arrangement solves");
        let mut pinned = stage.clone();
        for (f, _) in stage.toggles() {
            let a = pinned.input_mut(f).expect("its own input");
            if let Some(v) = solved.figure(f) {
                a.manual = v;
            }
            a.auto = false;
        }
        pinned
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
        let signature = |stage: &Shape| -> Vec<f64> {
            let t = train_of(vec![stage.clone()]);
            let r = solve_train(&t, &lib)
                .unwrap_or_else(|e| panic!("a pinned preset solves: {e:?}\n{stage:?}"));
            stage
                .toggles()
                .into_iter()
                .filter_map(|(f, _)| r.figure(f))
                .chain(r.members.iter().map(|g| g.pitch_diameter))
                .collect()
        };
        let mut checked = 0u32;
        let mut unread: Vec<String> = Vec::new();
        for (name, stage) in crate::train::testing::arrangements() {
            // Every box seeded with what the preset came to, as the panel
            // seeds a box a designer pins — so pinning everything pins the
            // design the preset already was, not a distance of nought.
            let t = train_of(vec![stage.clone()]);
            let solved = solve_train(&t, &lib).expect("every preset solves");
            let mut pinned = stage.clone();
            for (f, _) in stage.toggles() {
                if let Some(v) = solved.figure(f) {
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
            // A distance the tips hold open runs wider than its clearance
            // asks, and says so (`sized_by`): the clearance is a floor the
            // mesh clears, not a figure the solve can move. A planocentric
            // at three teeth of difference sits *at* that limit at the
            // shipped clearance, so the nudge is what lands in the tips'
            // regime, and it is the nudged stage that is asked.
            let tips_hold = |stage: &Shape, d: usize| {
                let t = train_of(vec![stage.clone()]);
                let r = solve_train(&t, &lib).expect("the nudged preset solves");
                r.distances[d]
                    .as_ref()
                    .is_some_and(|x| x.sized_by.is_some())
            };
            for f in mentioned(&settled) {
                let mut moved = settled.clone();
                let a = moved.input_mut(f).expect("declared, so present");
                if a.auto {
                    continue;
                }
                nudge(f, a);
                if let Freedom::Clearance(d) = f {
                    if tips_hold(&moved, d) {
                        continue;
                    }
                }
                let after = signature(&moved);
                let named = format!("{name} {f:?}");
                if !base.iter().zip(&after).any(|(x, y)| (x - y).abs() > 1e-9)
                    && !unread.contains(&named)
                {
                    unread.push(named);
                }
                checked += 1;
            }
        }
        assert!(checked >= 12, "only {checked} given inputs were checked");
        // Given inputs relief leaves standing beside a closure that already
        // spends what would read them — a planet–planet clearance beside the
        // shift the sun's distance takes, a worm's shift beside its pinned
        // distance: T10.9's, and [`UNHONOURED`] lists them too.
        assert_eq!(
            unread,
            ["Ravigneaux Clearance(2)", "WormAndPair Member(0, Shift)"],
            "stand given and are not read"
        );
    }

    /// Where a given input the relief law nudges is not the figure the solve
    /// reached, or the solve refuses it: arrangement and freedom, each with
    /// the task that clears it. The law fails on an entry that passes as
    /// well as on one missing, so the list only shrinks.
    const UNHONOURED: &[(&str, &str, &str)] = &[
        // A second mesh's overlap ratio, given where a first sets the helix:
        // the note names the ratio asked and not the one reached.
        ("Idler", "Overlap(1)", "T10.11"),
        ("Planetary", "Overlap(1)", "T10.11"),
        ("Wolfrom", "Overlap(1)", "T10.11"),
        ("Compound", "Overlap(1)", "T10.11"),
        ("MeshedPlanets", "Overlap(1)", "T10.11"),
        ("MeshedPlanets", "Overlap(2)", "T10.11"),
        ("Ravigneaux", "Overlap(1)", "T10.11"),
        ("Ravigneaux", "Overlap(2)", "T10.11"),
        ("Ravigneaux", "Overlap(3)", "T10.11"),
        // A given shift or clearance relief leaves standing where the solve
        // spends it on a closure.
        ("Worm", "Member(0, Shift)", "T10.9"),
        ("WormAndPair", "Member(0, Shift)", "T10.9"),
        ("Wolfrom", "Member(2, Shift)", "T10.9"),
        ("Ravigneaux", "Clearance(2)", "T10.9"),
        // A worm's given distance, refused `NoContact`. The name is wrong:
        // the cause is relief leaving the worm's size and both shifts
        // pinned beside the distance, so nothing can absorb it, and the
        // refusal should say that (T10.9, with T02's naming).
        ("Worm", "Distance(0)", "T10.9"),
        ("WormAndPair", "Distance(0)", "T10.9"),
        // Not relief's: the shipped set sits 0.06 mm from where its planets
        // can be placed (21.02 + 18.02 against 38.98), so its planet
        // distance moved 0.1 mm is refused `AxesCannotBePlaced` with every
        // other input free as well. A preset with margin (T10.2's note).
        ("MeshedPlanets", "Distance(1)", "preset margin, T10.2"),
        // Refused `AxesCannotBePlaced` only with the rest pinned; with them
        // free, the sun at k = 1.1 solves (T10.4, one thickness freedom per
        // mesh group).
        ("MeshedPlanets", "Member(0, ThicknessMod)", "T10.4"),
        // A clearance the tips hold open, said by `sized_by` and no note.
        ("Planocentric", "Clearance(0)", "T10.10"),
        ("Hula", "Clearance(0)", "T10.10"),
    ];

    /// **A given input is honoured** — the stronger half of the relief law
    /// ([`every_input_relief_leaves_given_is_honoured_by_the_solve`] is the
    /// structural one). Every input pinned at what the arrangement solved
    /// to, one freedom `f` nudged, and relief asked with `just = f`, which
    /// keeps `f` given: the solve then reaches `f`'s typed value. On every
    /// arrangement of the grid, every input it has.
    ///
    /// Where the model cannot reach what was asked, a note that named both
    /// the value asked and the value reached would honour the input as
    /// well — it is physics, said. No note does today: the ones that fire
    /// here name one of the two, or a figure beside `f` (a nominal distance
    /// beside a running one). So that escape is not written; each input it
    /// would cover is in [`UNHONOURED`] until a note names both.
    ///
    /// Reached means within [`HONOURED`], the distance a solve for a given
    /// figure lands on it (`distance_notes`' `REACHED`), relative to the
    /// figure where it exceeds one.
    #[test]
    fn a_given_input_is_honoured() {
        const HONOURED: f64 = 1e-6;
        let lib = library();
        let mut kinds: Vec<&str> = Vec::new();
        let mut unhonoured: Vec<(String, String, String)> = Vec::new();
        let (mut checked, mut freed) = (0u32, 0u32);
        for (name, stage) in crate::train::testing::arrangements() {
            let pinned = pinned_as_solved(&stage, &lib);
            for (f, _) in stage.toggles() {
                let mut moved = pinned.clone();
                nudge(f, moved.input_mut(f).expect("its own input"));
                let asked = moved.input_mut(f).expect("its own input").manual;
                let mut relieved = moved.relieved(Some(f));
                // An input the shape cannot read at all is freed whatever
                // relief is asked to keep — a crossed pair's ratio — and
                // stands given nowhere to be honoured.
                if relieved.input_mut(f).expect("its own input").auto {
                    freed += 1;
                    continue;
                }
                let t = train_of(vec![relieved]);
                let solved = solve_train(&t, &lib);
                let detail = match &solved {
                    Ok(r) => format!(
                        "asked {asked} reached {:?} notes {:?}",
                        r.figure(f),
                        r.parts
                            .iter()
                            .flat_map(|p| &p.notes)
                            .map(|n| (&n.key, &n.values))
                            .collect::<Vec<_>>()
                    ),
                    Err(e) => format!("asked {asked}: {e:?}"),
                };
                let honoured = match solved {
                    Ok(r) => {
                        let Some(reached) = r.figure(f) else {
                            panic!("{name}: {f:?} has no figure");
                        };
                        (reached - asked).abs() <= HONOURED * asked.abs().max(1.0)
                    }
                    // A refusal names no figure reached.
                    Err(_) => false,
                };
                if !honoured {
                    unhonoured.push((name.clone(), format!("{f:?}"), detail));
                }
                if !kinds.contains(&kind_of(f)) {
                    kinds.push(kind_of(f));
                }
                checked += 1;
            }
        }
        // Every kind of freedom is reached.
        for k in FREEDOM_KINDS {
            assert!(kinds.contains(&k), "no arrangement nudged a {k}: {kinds:?}");
        }
        let listed = |a: &str, f: &str| UNHONOURED.iter().any(|(x, g, _)| (*x, *g) == (a, f));
        let new: Vec<_> = unhonoured
            .iter()
            .filter(|(a, f, _)| !listed(a, f))
            .collect();
        let cleared: Vec<_> = UNHONOURED
            .iter()
            .filter(|(x, g, _)| {
                !unhonoured
                    .iter()
                    .any(|(a, f, _)| (a.as_str(), f.as_str()) == (*x, *g))
            })
            .collect();
        assert!(
            new.is_empty() && cleared.is_empty(),
            "of {checked} given inputs ({freed} freed), not honoured and not listed: {new:?}; listed and now honoured: {cleared:?}"
        );
    }

    /// **The reading relief leaves standing is the reading the solve reads.**
    /// Two readings of a pair's size given with different stories — a helix
    /// that says one diameter and a diameter that says another — and after
    /// relief the pair is the size the surviving reading states, exactly.
    /// One list serves both, so this cannot fail by the two lists drifting;
    /// it is here so that stays true when the next reading arrives.
    #[test]
    fn the_surviving_reading_is_the_one_the_solve_reads() {
        let mut both = arr::pair([17, 43]).with_first_helix(10.0);
        both.members[0].pitch_diameter = Auto::fixed(20.0);
        let relieved = both.clone().relieved(None);
        let pair = relieved;
        assert!(
            pair.members[0].gear.helix_angle.auto,
            "the helix is the less precious reading"
        );
        assert!(!pair.members[0].pitch_diameter.auto, "the diameter stands");
        assert_eq!(pair.members[0].pitch_diameter.manual, 20.0);
        let expect = (17.0_f64 / 20.0).acos().to_degrees();
        assert!((pair.indexed().helix_angles()[0] - expect).abs() < 1e-12);

        // ...and a ratio given the size to decide is the most precious of all.
        let mut with_ratio = arr::pair([17, 43]).with_first_helix(10.0);
        with_ratio.meshes[0].overlap = Auto::fixed(1.2);
        let relieved = with_ratio.clone().relieved(None);
        let pair = relieved;
        assert!(pair.members[0].gear.helix_angle.auto && !pair.meshes[0].overlap.auto);
        let width = pair.members[0]
            .gear
            .face_width
            .manual
            .min(pair.members[1].gear.face_width.manual);
        let expect = helix_for_overlap(1.2, 1.0, width).expect("reachable at the preset's width");
        assert!((pair.indexed().helix_angles()[0] - expect).abs() < 1e-12);
    }

    /// **A crossed pair has no overlap, and its ratio is turned back
    /// automatic** — by relief, whatever was touched, rather than by a line in
    /// the panel that used to reset the toggle when the shaft angle moved.
    /// A point contact has nothing for the ratio to relate, and a box the
    /// solve disregards must not stand as if it were read.
    #[test]
    fn a_crossed_pairs_ratio_cannot_stand_given() {
        let mut crossed = arr::worm(1, 40);
        crossed.meshes[0].overlap = Auto::fixed(1.5);
        for just in [None, Some(Freedom::Overlap(0)), Some(Freedom::Distance(0))] {
            let relieved = crossed.clone().relieved(just);
            let p = relieved;
            assert!(
                p.meshes[0].overlap.auto,
                "the ratio stood given on crossed shafts after {just:?}"
            );
            assert_eq!(p.meshes[0].overlap.manual, 1.5, "and the number is kept");
        }
        // ...and on parallel shafts the same input stands.
        let parallel = {
            let mut s = arr::pair([17, 43]);
            s.meshes[0].overlap = Auto::fixed(1.5);
            s
        };
        let relieved = parallel.clone().relieved(None);
        assert!(!relieved.meshes[0].overlap.auto);
    }

    /// **A ratio given on straight teeth is said to have asked nothing.** As
    /// a floor under an automatic width it needs a helix to buy overlap with,
    /// and at zero helix no width does — so the input is read as nothing, and
    /// the note says so rather than the box standing as if it had bitten.
    #[test]
    fn a_ratio_given_on_straight_teeth_says_it_asked_nothing() {
        let lib = library();
        let mut sp = arr::pair([17, 43]);
        sp.meshes[0].overlap = Auto::fixed(1.2);
        for m in &mut sp.members {
            m.gear.face_width = Auto::automatic(5.0);
        }
        let mut t = train_of(vec![sp.clone()]);
        let r = solve_train(&t, &lib).expect("solves");
        let notes = &r.by_part[0].notes;
        assert!(
            notes.iter().any(|n| n.key == key::PART_OVERLAP_NEEDS_HELIX),
            "no note for a ratio with no helix to work on: {notes:?}"
        );
        // Give it a helix and the floor bites, and the note goes.
        let helical = sp.with_first_helix(20.0);
        rechain(&mut t, vec![helical.clone()]);
        let r = solve_train(&t, &lib).expect("solves");
        let pair = &r.by_part[0];
        assert!(!pair
            .notes
            .iter()
            .any(|n| n.key == key::PART_OVERLAP_NEEDS_HELIX));
        let ratio = pair.meshes[0]
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
        let set = {
            let mut s = arr::planetary(12, 30, 72, 3);
            s.distances[0].clearance = Auto::automatic(0.02);
            s
        };
        let figures = [
            Figure {
                freedom: Freedom::Clearance(0),
                value: Some(0.123_456_789),
            },
            Figure {
                freedom: Freedom::Distance(0),
                value: Some(99.0),
            },
        ];
        let relieved = set.clone().relieved_from(None, &figures);
        let p = relieved;
        let clearance = p.distances[0].clearance;
        assert!(!clearance.auto, "the set's clearance is pinned back");
        assert_eq!(clearance.manual, 0.1235, "and seeded to the digits shown");
        assert_eq!(
            p.distances[0].distance.manual,
            arr::planetary(12, 30, 72, 3).distances[0].distance.manual,
            "a box relief did not turn keeps its number"
        );
    }

    /// **Of a distance and its clearance at most one is automatic**, on every
    /// shape — and which one is pinned back is decided by what was just
    /// touched. A set used to pin its clearance whatever was touched, since
    /// its own solver could not run at a given distance with the clearance left to
    /// fall where it may; the shape can, as a pair always could, so the set
    /// has the pair's rule now: touching the clearance pins the distance and
    /// touching the distance pins the clearance, and the number in a box
    /// relief did not turn is kept. A hula stage's shifts absorb its crank,
    /// so no given offset leaves a nominal one to subtract from, and its
    /// clearance is pinned back whatever was touched.
    #[test]
    fn an_epicyclic_kinds_clearance_is_pinned_back_whatever_was_touched() {
        let set = {
            let mut s = arr::planetary(12, 30, 72, 3);
            s.distances[0].clearance = Auto::automatic(0.02);
            s
        };
        let mut hula = hula_shape([65, 61, 57, 61]);
        hula.distances[0].clearance = Auto::automatic(0.02);
        for just in [
            Some(Freedom::Clearance(0)),
            Some(Freedom::Distance(0)),
            Some(Freedom::Member(1, MemberFreedom::Shift)),
            None,
        ] {
            let relieved = set.clone().relieved(just);
            let d = relieved.distances[0];
            assert!(
                !(d.clearance.auto && d.distance.auto),
                "a set: relieving after {just:?} should pin one of the two"
            );
            if just == Some(Freedom::Clearance(0)) {
                assert!(
                    d.clearance.auto && !d.distance.auto,
                    "the clearance just touched is spared"
                );
            } else {
                assert!(!d.clearance.auto, "the clearance is the first to be pinned");
            }
            assert_eq!(
                d.clearance.manual, 0.02,
                "and the number is left where it was"
            );
            let relieved = hula.clone().relieved(just);
            let d = relieved.distances[0];
            assert!(
                !(d.clearance.auto && d.distance.auto),
                "a hula stage: relieving after {just:?} should pin one of the two"
            );
            assert_eq!(
                d.clearance.manual, 0.02,
                "and leave the number where it was"
            );
        }
    }

    /// **Relieving an over-determined stage**, on every preset that has a relation
    /// — behaviour that had no test at all while it lived in the panel.
    ///
    /// Three claims, and the third is the one that makes it a rule rather than a
    /// habit: pinning everything relieves down to the limit; the input the
    /// designer has *this moment* pinned is never the one taken away; and a
    /// stage already within its limit is returned untouched, so relief cannot
    /// undo a design that was never over-determined.
    #[test]
    fn an_over_determined_shape_relieves_to_its_limit_and_keeps_what_was_just_pinned() {
        for stage in shapes() {
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
            // Already inside the limit — every preset is, with a
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
            let mut stage = arr::pair([17, 43]);
            stage.set_search(true);
            for (m, z) in stage.members.iter_mut().zip(teeth) {
                m.gear.teeth = z;
            }
            let x = stage.shifts();
            // What the solve reports of the teeth it built at those shifts —
            // a tip held off its mate's flank reaches past nothing.
            let Ok(r) = try_alone(&stage) else {
                continue;
            };
            checked += 1;
            let foul = r.meshes[0].flank_interference;
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
            let mut sp = arr::pair([17, 43]);
            sp.distances[0].distance = Auto::fixed(a);
            sp.members[0].gear.teeth = 9;
            sp.members[1].gear.teeth = 37;
            let t = train_of(vec![sp.clone()]);
            let r = solve_train(&t, &lib).expect("all three of these solve");
            let s = &r.by_part[0];
            let keys: Vec<String> = s.notes.iter().map(|n| n.key.clone()).collect();
            (s.distances[0].clearance, keys)
        };

        // Well inside what the teeth allow: both findings.
        let (clearance, keys) = at(23.0);
        assert!(clearance < 0.0, "this one should not be assemblable");
        assert!(
            keys.iter().any(|k| k == key::PART_DISTANCE_NOT_REACHED),
            "a distance no shifts reach should say so: {keys:?}"
        );
        assert!(
            keys.iter().any(|k| k == key::PART_CLEARANCE_NEGATIVE),
            "a pair that cannot be assembled should say so: {keys:?}"
        );

        // Too far out: reachable by nothing, but assemblable.
        let (clearance, keys) = at(25.0);
        assert!(clearance > 0.0);
        assert!(
            keys.iter().any(|k| k == key::PART_DISTANCE_NOT_REACHED)
                && !keys.iter().any(|k| k == key::PART_CLEARANCE_NEGATIVE),
            "the distance is unreachable but the pair goes together: {keys:?}"
        );

        // **And a distance that is reached says neither**, which is what stops
        // this being a note that fires on every stage with a distance.
        let (clearance, keys) = at(24.22);
        assert!(
            (clearance - arr::pair([17, 43]).distances[0].clearance.manual).abs() < 1e-6,
            "the shifts reach this one, so the clearance is the one asked for: {clearance}"
        );
        assert!(
            !keys.iter().any(|k| {
                k == key::PART_DISTANCE_NOT_REACHED || k == key::PART_CLEARANCE_NEGATIVE
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
            arr::pair([17, 43]),
            arr::worm(1, 40),
            arr::planetary(12, 30, 72, 3),
            hula(),
        ] {
            let Some(group) = stage
                .freedoms()
                .into_iter()
                .find(|g| g.automatic_at_most < g.order.len())
            else {
                panic!("every preset has a clearance, so every preset bounds it");
            };

            // Both automatic — or, for a preset that holds its distance, the clearance
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
                checked += 1;
            }
        }
        // Two inputs on each of the two pair presets, and the one input the two
        // epicyclic presets each bound on their own.
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
    fn the_declared_limit_is_the_freedom_the_shape_actually_has() {
        let lib = library();
        let clearance = 0.02_f64;
        let asked = 24.4199_f64 + clearance;

        let solve = |pin_shifts: bool, size_free: bool| {
            let mut sp = arr::pair([17, 43]);
            sp.distances[0].clearance = Auto::fixed(clearance);
            sp.distances[0].distance = Auto::fixed(asked);
            sp.members[0].gear.teeth = 9;
            sp.members[1].gear.teeth = 37;
            if pin_shifts {
                // Two shifts a designer might well type, and nowhere near the
                // pair the given distance wants.
                sp.members[0].gear.profile_shift = Auto::fixed(0.20);
                sp.members[1].gear.profile_shift = Auto::fixed(0.20);
            }
            // A stated size, or every reading of it left to the distance.
            sp = if size_free {
                sp.size_free()
            } else {
                sp.with_first_helix(0.0)
            };
            let t = train_of(vec![sp.clone()]);
            let r = solve_train(&t, &lib).expect("the stage solves either way");
            let s = &r.by_part[0];
            (
                s.distances[0].clearance,
                s.distances[0].running,
                s.members[0].helix_angle,
            )
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
        // the size being one entry of three readings, which comes last. The
        // clearance gives after the shifts: a set's three shifts can leave
        // the relation over by two, and a clearance relieved is one the group
        // below pins straight back.
        let groups = arr::pair([17, 43]).freedoms();
        let relation = groups
            .iter()
            .find(|g| g.order.len() == 5)
            .expect("the pair's relation");
        assert_eq!(relation.given_at_most, 4);
        assert_eq!(relation.order[0], vec![Freedom::Distance(0)]);
        assert_eq!(
            relation.order[1],
            vec![Freedom::Member(0, MemberFreedom::Shift)]
        );
        assert_eq!(relation.order[3], vec![Freedom::Clearance(0)]);
        assert_eq!(
            relation.order[4],
            vec![
                Freedom::Member(0, MemberFreedom::Helix),
                Freedom::Member(0, MemberFreedom::PitchDiameter),
                Freedom::Member(1, MemberFreedom::Helix),
                Freedom::Member(1, MemberFreedom::PitchDiameter),
                Freedom::Overlap(0),
            ],
            "the size is the last to give: a shift moves the teeth, a size changes them — and the preset's widths are given, so the ratio is a reading of it"
        );
        // ...and with a width automatic the ratio is a floor, in no argument.
        let mut width_free = arr::pair([17, 43]);
        width_free.members[0].gear.face_width = Auto::automatic(8.0);
        assert!(!mentioned(&width_free.clone()).contains(&Freedom::Overlap(0)));

        // And the second group is the one that stops *both* ways of saying the
        // distance being left automatic at once.
        let both = groups
            .iter()
            .find(|g| g.automatic_at_most < g.order.len())
            .expect("a distance and a clearance cannot both be derived");
        assert_eq!(both.automatic_at_most, 1);
        assert_eq!(both.order[0], vec![Freedom::Clearance(0)]);
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
                        let mut sp = arr::pair([17, 43]);
                        sp.distances[0].clearance = Auto::fixed(clearance);
                        sp.distances[0].distance = Auto::fixed(a + clearance);
                        sp.members[0].gear.teeth = z1;
                        sp.members[1].gear.teeth = z2;
                        sp.set_search(optimiser);
                        let t = train_of(vec![sp.clone()]);
                        let Ok(r) = solve_train(&t, &lib) else {
                            continue;
                        };
                        let s = &r.by_part[0];

                        checked += 1;
                        assert!(
                            (s.distances[0].clearance - clearance).abs() < 1e-6,
                            "{z1}/{z2} at a={a} clearance={clearance} optimiser={optimiser}:                              the shifts leave {} of clearance, so the pair does not run at the                              distance it was given",
                            s.distances[0].clearance
                        );
                        sums.push(s.members[0].profile_shift + s.members[1].profile_shift);
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
    /// the same thing on every preset and at every distance.
    #[test]
    fn the_reported_clearance_is_the_gap_the_pair_runs_at() {
        let lib = library();
        let mut checked = 0u32;
        for distance in [None, Some(30.3_f64), Some(30.5)] {
            for clearance in [0.0_f64, 0.02, 0.20] {
                for optimiser in [false, true] {
                    let mut sp = arr::pair([17, 43]);
                    sp.distances[0].clearance = Auto::fixed(clearance);
                    sp.set_search(optimiser);
                    if let Some(a) = distance {
                        sp.distances[0].distance = Auto::fixed(a);
                    }
                    let t = train_of(vec![sp.clone(), arr::worm(1, 40)]);
                    let Ok(r) = solve_train(&t, &lib) else {
                        continue;
                    };

                    let s = &r.by_part[0];
                    checked += 1;
                    assert!(
                        (s.distances[0].clearance
                            - (s.distances[0].running - s.distances[0].nominal[0]))
                            .abs()
                            < 1e-12,
                        "spur at a={distance:?} clearance={clearance} optimiser={optimiser}: \
                         reports {} of clearance between {} and {}",
                        s.distances[0].clearance,
                        s.distances[0].nominal[0],
                        s.distances[0].running
                    );
                    // ...and the same identity on the preset that has no shift to
                    // absorb anything, which is where an echoed input and a
                    // derived gap part company hardest.
                    let (w, _) = worm(&r.by_part[1]);
                    assert!(
                        (w.distances[0].clearance
                            - (w.distances[0].running - w.distances[0].nominal[0]))
                            .abs()
                            < 1e-12,
                        "worm: reports {} between {} and {}",
                        w.distances[0].clearance,
                        w.distances[0].nominal[0],
                        w.distances[0].running
                    );
                }
            }
        }
        assert!(checked >= 12, "only {checked} configurations solved");
    }

    /// **A tolerance band opens the way round it says it does**, on every preset
    /// that reports one.
    ///
    /// Less centre distance is less room and so less play. That is one claim,
    /// and it was written out four times — once per stage type, each closing
    /// over its own way of turning a distance into an angle. Getting it
    /// backwards in one of them would have produced a band that reads perfectly
    /// well and is inside out, and nothing anywhere asserted the direction.
    ///
    /// `Backlash::of_ends` is the one construction now; this is the claim it
    /// makes, checked through all four presets rather than at the constructor,
    /// because the argument each passes is the part that could still be wrong.
    #[test]
    fn a_tolerance_band_widens_with_the_centre_distance() {
        let lib = library();
        let train = train_of(vec![
            arr::pair([17, 43]),
            arr::worm(1, 40),
            arr::planetary(12, 30, 72, 3),
            hula(),
            {
                let mut s = arr::pair([17, 43]);
                s.distances[0].angle = 90.0;
                s
            },
        ]);
        let r = solve_train(&train, &lib).expect("a train of every preset");

        let mut checked = 0u32;
        let mut check = |what: &str, b: &Backlash, opens: bool| {
            checked += 1;
            assert!(
                b.minimum.unwrap() <= b.nominal && b.nominal <= b.maximum.unwrap(),
                "{what}: the band runs {} … {} … {}, which is not an order",
                b.minimum.unwrap(),
                b.nominal,
                b.maximum.unwrap()
            );
            if opens {
                assert!(
                    b.maximum > b.minimum,
                    "{what}: a tolerance that opens nothing — {} either way",
                    b.minimum.unwrap()
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
                    b.maximum.unwrap() - b.minimum.unwrap() < 1e-12,
                    "{what}: the ideal set's play should not move with its centre tolerance, \
                     but the band is {} wide",
                    b.maximum.unwrap() - b.minimum.unwrap()
                );
            }
        };

        for (k, stage) in r.by_part.iter().enumerate() {
            let d = across(&train, k, &lib).backlash.both().unwrap();
            let opens = stage.layouts.is_empty();
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
    /// epicyclic set is neither: **which body drives decides where `η₀`
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
            let mut h = hula_shape([65, 61, 57, 61]);
            for m in &mut h.meshes {
                m.sliding_friction = mu;
                m.static_friction = mu;
            }
            let mut t = two_pairs();
            t.load_cases[BACK].set_torque(0.5);
            rechain(&mut t, vec![arr::worm(1, 40), h]);
            let r = solve_train(&t, &lib).expect("a train that solves");
            let s = &r.by_part[1];
            let at = |g: &GearResult, case: usize| g.cases[case].torque;
            // The output over the grounded gear: the two central members,
            // after the two wobble gears.
            (
                at(&s.members[3], PEAK) / at(&s.members[2], PEAK),
                at(&s.members[3], BACK) / at(&s.members[2], BACK),
            )
        };
        let (forward, backward) = ratios(0.0);
        assert!(
            (forward - backward).abs() < 1e-9,
            "with no friction the stage distributes torque alike either way, \
             but forward gives {forward} and backward {backward}"
        );
        // **By `η₁η₂`**, which is what the two pressing torques differ by
        // when the driving side of each mesh swaps — the hula's own solver's
        // row torques differed by its square, since a driven member's row stands `η`
        // under the force on its flank.
        let (forward, backward) = ratios(0.08);
        assert!(
            (forward - backward).abs() / forward.abs() > 5e-4,
            "with friction the two directions must place the loss differently, \
             but both give {forward}"
        );
    }

    #[test]
    fn a_back_driven_set_distributes_torque_by_its_own_solve() {
        let lib = library();
        let ratios = |mu: f64| {
            let mut set = arr::planetary(12, 30, 72, 3);
            for m in &mut set.meshes {
                m.sliding_friction = mu;
                m.static_friction = mu;
            }
            let mut t = two_pairs();
            t.load_cases[BACK].set_torque(0.5);
            // A self-locking stage at the input end, so the set reacts the load.
            rechain(&mut t, vec![arr::worm(1, 40), set.clone()]);
            let r = solve_train(&t, &lib).expect("a train that solves");
            let p = &r.by_part[1];
            let at = |g: &GearResult, case: usize| g.cases[case].torque;
            (
                at(&p.members[2], PEAK) / at(&p.members[0], PEAK),
                at(&p.members[2], BACK) / at(&p.members[0], BACK),
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

    /// The body a case reports at a port of the train.
    fn at_port<'a>(r: &'a TrainResult, _train: &Train, case: usize, at: usize) -> &'a CaseBody {
        r.cases[case]
            .shaft(at)
            .expect("every port is a shaft of the case")
    }

    /// A port declared free: turning, carrying nothing — what "not
    /// reacted" at that end comes to.
    fn free(at: usize) -> Load {
        Load::declared(at, LoadRole::Free)
    }
    /// **The chain's two ends**, where it has exactly two — every chain
    /// fixture here — which its preset cases are written between.
    fn ends_of(t: &Train) -> (usize, usize) {
        t.chain_ends().expect("a chain fixture has two ends")
    }
    fn start_of(t: &Train) -> usize {
        ends_of(t).0
    }
    /// An ultimate case between the train's ends with nothing on it, for a
    /// test to write its own loads into.
    fn template(t: &Train) -> LoadCase {
        let (start, end) = ends_of(t);
        LoadCase::ultimate(start, end, 0.0, 0.0)
    }
    fn end_of(t: &Train) -> usize {
        ends_of(t).1
    }
    /// The three cases a train used to hold as fields, written between the
    /// train's two ends; the back-driving case with its start *free* — so a
    /// train that can be back-driven reacts none of it; the tests here are
    /// about where a load is held, and the preset's reacted start is the
    /// case that always is.
    fn classic(
        t: &Train,
        input_speed: f64,
        input_torque: f64,
        back_driving_torque: f64,
        operating_torque: f64,
        operating_speed: f64,
        duty: Duty,
    ) -> Vec<LoadCase> {
        let (start, end) = ends_of(t);
        vec![
            LoadCase::ultimate(start, end, input_torque, input_speed),
            LoadCase {
                loads: vec![Load::given(end, back_driving_torque, 0.0), free(start)],
                ..LoadCase::back_driving(start, end, back_driving_torque)
            },
            LoadCase {
                duty,
                ..LoadCase::fatigue(start, end, operating_torque, operating_speed)
            },
        ]
    }

    /// **A fixture's stages replaced, chained afresh, and its cases
    /// carried to the new ends** — what the chain convention did without
    /// saying so when a test swapped a train's stages under its cases.
    fn rechain(t: &mut Train, stages: Vec<Shape>) {
        let (old_start, old_end) = ends_of(t);
        let fresh = Train::chained(stages, |_| Vec::new());
        t.shape = fresh.shape;
        t.held = fresh.held;
        let (start, end) = ends_of(t);
        let carry = |at: &mut usize| {
            if *at == old_start {
                *at = start;
            } else if *at == old_end {
                *at = end;
            }
        };
        for c in &mut t.load_cases {
            for l in &mut c.loads {
                carry(&mut l.at);
            }
            if let Duty::Intermittent { at: Some(at), .. } = &mut c.duty {
                carry(at);
            }
        }
    }

    /// **A chain of these stages with the classic cases between its ends.**
    /// What every fixture here is built by: the chain's bodies numbered
    /// once, and the cases at whatever bodies the ends come to.
    fn train_of(stages: Vec<Shape>) -> Train {
        let mut t = Train::chained(stages, |_| Vec::new());
        let duty = Duty::intermittent(Some(end_of(&t)));
        t.load_cases = classic(&t, 3000.0, 2.0, 0.0, 2.0, 3000.0, duty);
        t
    }

    /// **A chain of three presets**, so the graph's assembly meets a stage with
    /// three bodies sitting between two with two — which a train of one preset
    /// cannot exercise and which is where a coupling to the wrong body would
    /// show.
    fn mixed_train() -> Train {
        let second = two_pairs().part_shapes().remove(1);
        train_of(vec![
            arr::pair([17, 43]),
            arr::planetary(12, 30, 72, 3),
            second,
            arr::worm(1, 40),
        ])
    }

    fn two_pairs() -> Train {
        train_of(vec![arr::pair([17, 43]), {
            let mut s = arr::pair([17, 43]);
            s.members[0].gear = MemberGear {
                teeth: 13,
                ..MemberGear::default()
            };
            s.members[1].gear = MemberGear {
                teeth: 31,
                ..MemberGear::default()
            };
            s
        }])
    }

    #[test]
    fn a_train_of_two_pairs_computes_end_to_end() {
        let r = solve_train(&two_pairs(), &library()).unwrap();
        assert_eq!(r.by_part.len(), 2);

        // 43/17 * 31/13
        let want = (43.0 / 17.0) * (31.0 / 13.0);
        assert!((r.total().unwrap().ratio - want).abs() < 1e-12);
        let t = two_pairs();
        let end = at_port(&r, &t, PEAK, end_of(&t));
        assert_eq!(end.role, BodyRole::Reacted);
        assert!((end.speed.unwrap() - 3000.0 / want).abs() < 1e-9);

        // Every stage produced real numbers.
        for s in r.by_part.iter().map(spur) {
            assert!(s.distances[0].running > 0.0);
            assert!(s.meshes[0].line.unwrap().contact_ratios.transverse > 1.0);
            assert!(s.meshes[0].efficiency.forward > 0.9 && s.meshes[0].efficiency.forward < 1.0);
            assert_eq!(
                s.meshes[0].efficiency.forward, s.meshes[0].efficiency.backward,
                "a parallel-axis stage is as efficient driven either way"
            );
            assert!(s.meshes[0].cases[0].contact.at_pitch_point > 0.0);
            for g in &s.members {
                assert!(g.face_width > 0.0);
                assert!(g.cases[0].bending_stress.unwrap() > 0.0);
            }
        }
    }

    /// **The two backlash figures are one gap seen from the two ends.**
    ///
    /// Referred to the output body or to the input body, the same play must
    /// differ by exactly the total ratio — every stage's contribution scales the
    /// same way, because a stage's own two figures are its gap at two lever arms
    /// whose ratio *is* that stage's ratio. If a stage ever got that wrong the
    /// products would stop matching, which no per-stage check would catch.
    #[test]
    fn backlash_at_the_two_ends_differs_by_exactly_the_total_ratio() {
        let r = solve_train(&two_pairs(), &library()).unwrap();
        for (forward, backward) in [
            (
                r.total().unwrap().backlash.forward.unwrap().nominal,
                r.total().unwrap().backlash.backward.unwrap().nominal,
            ),
            (
                r.total()
                    .unwrap()
                    .backlash
                    .forward
                    .unwrap()
                    .maximum
                    .unwrap(),
                r.total()
                    .unwrap()
                    .backlash
                    .backward
                    .unwrap()
                    .maximum
                    .unwrap(),
            ),
        ] {
            assert!(forward > 0.0);
            assert!(
                (backward - forward * r.total().unwrap().ratio).abs() < 1e-9 * backward,
                "{backward} vs {forward} x {}",
                r.total().unwrap().ratio
            );
        }
        // ...and the input end is the looser one, because it turns faster.
        assert!(
            r.total().unwrap().backlash.backward.unwrap().nominal
                > r.total().unwrap().backlash.forward.unwrap().nominal
        );
    }

    /// A train of parallel-axis stages is as efficient driven either way, and
    /// cannot lock. Both are consequences of the meshes, not rules the train
    /// applies.
    #[test]
    fn a_parallel_axis_train_reports_equal_efficiencies_and_cannot_lock() {
        let r = solve_train(&two_pairs(), &library()).unwrap();
        // To the last bits: the two directions are two solves of the flow
        // with the driver on the other row, and differ by rounding alone.
        assert!(
            (r.total().unwrap().efficiency.forward.unwrap()
                - r.total().unwrap().efficiency.backward.unwrap())
            .abs()
                < 1e-14,
            "{} vs {}",
            r.total().unwrap().efficiency.forward.unwrap(),
            r.total().unwrap().efficiency.backward.unwrap()
        );
        assert!(!r.total().unwrap().efficiency.locked().backward);
    }

    /// Efficiency must always *reduce* delivered torque. Getting this sign wrong
    /// is the classic train-accumulation bug, and it hides because the ratio term
    /// is so much larger.
    #[test]
    fn efficiency_always_costs_torque() {
        let lib = library();
        let mut lossless = two_pairs();
        for m in &mut lossless.shape.meshes {
            m.sliding_friction = 0.0;
            m.static_friction = 0.0;
        }
        let ideal = solve_train(&lossless, &lib).unwrap();
        let real = solve_train(&two_pairs(), &lib).unwrap();

        assert!((ideal.total().unwrap().efficiency.forward.unwrap() - 1.0).abs() < 1e-12);
        let t = two_pairs();
        let (real_out, ideal_out) = (
            at_port(&real, &t, PEAK, end_of(&t)).torque.abs(),
            at_port(&ideal, &lossless, PEAK, end_of(&t)).torque.abs(),
        );
        assert!(real_out < ideal_out);
        // ...and the shortfall is exactly the product of the stage efficiencies.
        assert!(
            (real_out - ideal_out * real.total().unwrap().efficiency.forward.unwrap()).abs() < 1e-9
        );
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
        let shift_of = |dedendum: f64, working: Auto<f64>| {
            let stage = {
                let mut s = arr::pair([15, 43]);
                for m in &mut s.members {
                    m.gear.dedendum = dedendum;
                    m.gear.working_depth = working;
                }
                s.members[0].gear.profile_shift = Auto::automatic(0.0);
                s
            };
            try_alone(&stage).expect("a solvable stage").members[0].profile_shift
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
    /// moved when this landed (docs/reference.md#axis-distance-and-backlash).
    ///
    /// Backlash is deliberately *not* in this test's scope: it measures play
    /// against the zero-backlash reference and keeps the design mesh. That
    /// division is gated in `mesh.rs`.
    #[test]
    fn a_parallel_pair_is_rated_at_the_centre_distance_it_runs_at() {
        let stage = |clearance: f64| {
            let mut s = arr::pair([17, 43]);
            s.distances[0].clearance = Auto::fixed(clearance);
            s
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = try_alone(&stage(clearance)).unwrap();
            let eps = r.meshes[0].line.unwrap().contact_ratios.transverse;
            let bending = r.members[0].cases[0]
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
    fn a_spur_pair_has_exactly_zero_overlap_and_a_helical_one_does_not() {
        let spur = try_alone(&arr::pair([17, 43])).unwrap();
        assert_eq!(
            spur.meshes[0].line.unwrap().contact_ratios.overlap,
            0.0,
            "must be exactly zero"
        );
        assert_eq!(
            spur.meshes[0].line.unwrap().contact_ratios.total,
            spur.meshes[0].line.unwrap().contact_ratios.transverse
        );
        assert!(!spur.meshes[0]
            .line
            .unwrap()
            .contact_ratios
            .has_full_axial_overlap());

        let helical = try_alone(&arr::pair([17, 43]).with_additional_helix(20.0)).unwrap();
        assert!(helical.meshes[0].line.unwrap().contact_ratios.overlap > 0.0);
        assert!(
            helical.meshes[0].line.unwrap().contact_ratios.total
                > helical.meshes[0].line.unwrap().contact_ratios.transverse
        );
    }

    /// The last stage dominates output backlash, which is the design consequence
    /// worth surfacing: tolerance spent at the input end is nearly free.
    #[test]
    fn backlash_referred_to_the_output_is_dominated_by_the_last_mesh() {
        let lib = library();
        let base = two_pairs();

        let loosen = |k: usize| {
            let mut t = base.clone();
            let d = t.parts()[k].distances[0];
            t.shape.distances[d].clearance.manual *= 4.0;
            solve_train(&t, &lib)
                .unwrap()
                .total()
                .unwrap()
                .backlash
                .forward
                .unwrap()
                .nominal
        };

        let reference = solve_train(&base, &lib)
            .unwrap()
            .total()
            .unwrap()
            .backlash
            .forward
            .unwrap()
            .nominal;
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
        let stage = {
            let mut s = arr::pair([17, 43]);
            s.members[0].thickness_mod = Auto::fixed(1.3);
            s.members[1].thickness_mod = Auto::automatic(2.0 - 1.3);
            s
        };
        let k: Vec<f64> = (0..2)
            .map(|i| stage.params_of(i, stage.shifts()[i]).thickness_mod)
            .collect();
        assert!((k[0] + k[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_automatic_face_width_is_the_larger_of_the_enabled_checks() {
        let off = ByKind {
            ultimate: false,
            fatigue: false,
        };
        let width = |sources: FaceSources| {
            let mut s = arr::pair([17, 43]);
            for g in s.members.iter_mut().map(|m| &mut m.gear) {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
            }
            try_alone(&s).unwrap().members[0].face_width
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

        // Nothing enabled asks for nothing: the width stands at its box, and
        // a box of nought is a gear with no face, refused where it enters
        // by its field rather than rated at a width of nothing.
        let none = FaceSources {
            bending: off,
            contact: off,
        };
        let mut s = arr::pair([17, 43]);
        for g in s.members.iter_mut().map(|m| &mut m.gear) {
            g.face_width = Auto::automatic(0.0);
            g.face_sources = none;
        }
        match try_alone(&s) {
            Err(TrainError::Input(r)) => {
                assert_eq!(r.field, "shape.members.0.gear.face_width.manual");
            }
            other => panic!("a face of nought is refused by its field, not {other:?}"),
        }
        for g in s.members.iter_mut().map(|m| &mut m.gear) {
            g.face_width = Auto::automatic(7.0);
        }
        assert_eq!(try_alone(&s).unwrap().members[0].face_width, 7.0);
    }

    /// An intermittent duty measured at the end port makes upstream gears turn
    /// further, not less. Getting the direction backwards would silently
    /// under-count cycles on exactly the gears that see the most — and the
    /// same sweep stated at the start port is the whole ratio fewer turns
    /// everywhere, which is what a named port means.
    #[test]
    fn intermittent_cycles_are_worked_backwards_from_the_output() {
        let mut t = two_pairs();
        t.load_cases[CYCLIC].duty = Duty::Intermittent {
            range_degrees: 360.0,
            at: Some(end_of(&t)),
            actuations: 100,
            reversing: false,
        };
        let r = solve_train(&t, &library()).unwrap();
        let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");

        // The output gear turns exactly once per actuation. A whole number of
        // revolutions, so the rounding has nothing to do and the count is the
        // revolutions exactly — which is the case worth putting first, because
        // it is where a count and a revolution coincide.
        let last = cycles(&spur(&r.by_part[1]).members[1]);
        assert_eq!(last.bending, 100.0);
        // Not reversing, so one flank takes every engagement.
        assert_eq!(last.contact, last.bending);
        // An ultimate case is survived once and counts nothing.
        assert!(spur(&r.by_part[1]).members[1].cases[PEAK].cycles.is_none());

        // Every gear upstream turns more than the one after it.
        let seq = [
            cycles(&spur(&r.by_part[0]).members[0]).bending,
            cycles(&spur(&r.by_part[0]).members[1]).bending,
            cycles(&spur(&r.by_part[1]).members[1]).bending,
        ];
        for w in seq.windows(2) {
            assert!(w[0] > w[1], "cycles must fall towards the output: {seq:?}");
        }
        // The input gear sees the whole train ratio's worth — **rounded up**,
        // because these are engagements rather than revolutions and a tooth
        // three quarters of the way through one has still been loaded by it.
        assert_eq!(seq[0], (100.0 * r.total().unwrap().ratio).ceil());

        // The two gears meshing with each other turn at different speeds but
        // share a mesh, so their cycle counts differ by that stage ratio — and
        // only **up to the rounding**, now that each count is a whole number of
        // engagements rather than a revolution count.
        //
        // The bound is derived rather than chosen: rounding adds less than one
        // to each, so `a/b` moves from `A/B` by less than `(1 + ratio)/b`. On
        // this train that is 6e-8, which is why the old exact-to-1e-9 assertion
        // was the thing that had to change and not the arithmetic.
        let s0 = spur(&r.by_part[0]);
        let (a, b) = (
            cycles(&s0.members[0]).bending,
            cycles(&s0.members[1]).bending,
        );
        let ratio = reduction(s0);
        assert!(
            (a / b - ratio).abs() < (1.0 + ratio) / b,
            "{a}/{b} = {} against a stage ratio of {}",
            a / b,
            ratio
        );

        // The same sweep at the start port: the input gear turns exactly once
        // per actuation, and every count is the total ratio smaller.
        let start = start_of(&t);
        if let Duty::Intermittent { at, .. } = &mut t.load_cases[CYCLIC].duty {
            *at = Some(start);
        }
        let r = solve_train(&t, &library()).unwrap();
        assert_eq!(cycles(&spur(&r.by_part[0]).members[0]).bending, 100.0);
        assert_eq!(
            cycles(&spur(&r.by_part[1]).members[1]).bending,
            (100.0 / r.total().unwrap().ratio).ceil()
        );
    }

    #[test]
    fn continuous_cycles_follow_each_gears_own_speed() {
        let mut t = two_pairs();
        t.load_cases[CYCLIC].set_speed(1500.0);
        t.load_cases[CYCLIC].duty = Duty::Continuous { runtime_hours: 2.0 };
        let r = solve_train(&t, &library()).unwrap();
        let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");

        // Input gear: 1500 rpm * 60 min * 2 h — its own speed, for as long as
        // the train runs. Rounded up, as every count is.
        let want = (1500.0_f64 * 60.0 * 2.0).ceil();
        let first = cycles(&spur(&r.by_part[0]).members[0]);
        assert!((first.bending - want).abs() < 1e-6);
        // A continuous duty has no actuation to reverse within, so the two
        // counts agree.
        assert_eq!(first.bending, first.contact);
        // Speeds fall through the train, and cycles follow them — each case
        // at its own.
        assert!((spur(&r.by_part[0]).members[0].cases[PEAK].speed - 3000.0).abs() < 1e-9);
        assert!((spur(&r.by_part[0]).members[0].cases[CYCLIC].speed - 1500.0).abs() < 1e-9);
        assert!(
            (spur(&r.by_part[1]).members[1].cases[PEAK].speed
                - at_port(&r, &t, PEAK, end_of(&t)).speed.unwrap())
            .abs()
                < 1e-9
        );
        assert!(cycles(&spur(&r.by_part[1]).members[1]).bending < first.bending);
    }

    /// **An epicyclic member is engaged once per turn against the carrier**, per
    /// planet — for every member and both epicyclic presets, which is the whole of
    /// the rule a member's cycles are counted by ([`shape::rate`]).
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
    ///   body's, right only where those are the same body.
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
    /// the wiring's own answer**, so a preset that gets its mounts wrong fails
    /// here rather than agreeing with itself.
    #[test]
    fn an_epicyclic_members_cycles_are_its_turns_against_the_carrier() {
        let lib = library();
        for (name, stage) in [
            ("epicyclic", arr::planetary(12, 30, 72, 3)),
            ("hula", hula()),
        ] {
            let mut train = two_pairs();
            train.load_cases[CYCLIC].duty = Duty::Continuous { runtime_hours: 1.0 };
            rechain(&mut train, vec![stage]);
            let r = solve_train(&train, &lib).unwrap_or_else(|e| panic!("{name}: {e}"));
            let cycles = |g: &GearResult| g.cases[CYCLIC].cycles.expect("a fatigue case counts");
            // The revolutions the input body turns over the duty, which is what
            // every member's count is a multiple of.
            let turns = 3000.0 * 60.0;

            let want = |member: f64, carrier: f64, paths: f64| {
                (turns * ((member - carrier) / 3000.0).abs() * paths).ceil()
            };
            let p = &r.by_part[0];
            if name == "hula" {
                // The hula's bodies: ground, crank, grounded, output, wobble.
                let crank = p.cases[CYCLIC].speeds[1];
                for g in &p.members {
                    let c = &g.cases[CYCLIC];
                    let expected = want(c.speed, crank, 1.0);
                    assert!(
                        (cycles(g).bending - expected).abs() <= 1.0,
                        "z{}: {} engagements against {expected}",
                        g.params.teeth,
                        cycles(g).bending
                    );
                    // ...and the speed its teeth see is the one against the
                    // crank, reported rather than left to be subtracted.
                    assert!((c.speed_against_carrier - (c.speed - crank)).abs() < 1e-9);
                }
                // The grounded gear — member 2, after the two wobble gears —
                // stands still and is engaged once a crank turn, which is the
                // whole duty's worth of revolutions.
                assert!(
                    (cycles(&p.members[2]).bending - turns).abs() <= 1.0,
                    "the grounded gear meets the wobble body once a crank turn"
                );
            } else {
                {
                    // The shape's bodies: ground, sun, carrier, ring, planet.
                    let shafts = &p.cases[CYCLIC];
                    let carrier = shafts.speeds[2];
                    let planets = p.layouts.first().map_or(1, |l| l.count);
                    let n = f64::from(planets);
                    let w = train.part_shapes()[0].wiring();
                    let (sun, planet, ring) = (&p.members[0], &p.members[1], &p.members[2]);
                    for (member, which, got, speed) in [
                        (0, "sun", cycles(sun).bending, shafts.speeds[1]),
                        (2, "ring", cycles(ring).bending, shafts.speeds[3]),
                        (
                            1,
                            "planet",
                            cycles(planet).bending,
                            planet.cases[CYCLIC].speed,
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
                    assert_eq!(w.paths_seen(0), planets, "a sun meets every planet");
                    assert_eq!(w.paths_seen(2), planets, "and so does a ring");
                    assert_eq!(w.paths_seen(1), 1, "a planet meets the one sun");
                    assert!(n > 1.0, "this fixture has to have more than one planet");
                    // **A body that does not turn is still loaded**, which is
                    // the case the old rule could not state: it counted the
                    // input's revolutions, so a held ring came out as though it
                    // turned with the sun. It meets a planet once per *carrier*
                    // turn instead, which is `z_s/(z_s + z_r)` of that.
                    assert_eq!(shafts.speeds[3], 0.0, "the ring is the held shaft here");
                    let set = arr::planetary(12, 30, 72, 3);
                    let zs = f64::from(set.members[0].gear.teeth);
                    let zr = f64::from(set.members[2].gear.teeth);
                    assert!(
                        (cycles(ring).bending - (turns * zs / (zs + zr) * n).ceil()).abs() <= 1.0,
                        "a held ring counts carrier turns: {}",
                        cycles(ring).bending
                    );
                }
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
                let mut stage = arr::pair([17, 43]);
                for g in stage.members.iter_mut().map(|m| &mut m.gear) {
                    g.addendum = asked;
                    g.min_tip_width = want;
                    // The tip width alone is the bound under study.
                    g.no_tip_past_mate_flank = false;
                }
                let r = try_alone(&stage).unwrap();

                for i in 0..2 {
                    let built = Tooth::new(stage.params_of(i, stage.shifts()[i]));
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
                    assert!((r.members[i].addendum - built.params.addendum).abs() < 1e-12);
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
    /// Asked of all three presets that have the control, because it is one
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
        let gear = || MemberGear {
            face_width: Auto::automatic(GIVEN),
            face_sources: off,
            ..MemberGear::default()
        };

        let mut spur = arr::pair([17, 43]);
        for m in &mut spur.members {
            m.gear = MemberGear {
                teeth: m.gear.teeth,
                ..gear()
            };
        }
        let mut set = arr::planetary(12, 30, 72, 3);
        for m in &mut set.members {
            m.gear = MemberGear {
                teeth: m.gear.teeth,
                profile_shift: m.gear.profile_shift,
                ..gear()
            };
        }
        let mut hula = hula_shape([65, 61, 57, 61]);
        for m in &mut hula.members {
            m.gear = MemberGear {
                teeth: m.gear.teeth,
                addendum: m.gear.addendum,
                dedendum: m.gear.dedendum,
                ..gear()
            };
        }

        let spur_r = try_alone(&spur).unwrap();
        let set_r = try_alone(&set).unwrap();
        let hula_r = solve_hula(&hula, 2.0, 0.0, &lib).unwrap();

        let members: Vec<&GearResult> = spur_r
            .members
            .iter()
            .chain(set_r.members.iter())
            .chain(hula_r.members.iter())
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
        // gear whose width it is about, on every preset, not in a stage's list.
        let members: Vec<&GearResult> = spur_r
            .members
            .iter()
            .chain(set_r.members.iter())
            .chain(hula_r.members.iter())
            .collect();
        for (i, g) in members.iter().enumerate() {
            assert!(
                g.notes.iter().any(|n| n.is(key::GEAR_FACE_WIDTH_NO_SOURCE)),
                "member {i} should say no rating sizes its width: {:?}",
                g.notes
            );
        }
    }

    /// **The sharing model reaches every member of every preset that has one.**
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
        // Tall enough that every mesh is above `ε_n = 2` **where it runs**,
        // on counts large enough that the tall tips stay on usable flank: a
        // path cut by interference falls back below the band.
        let tall = |g: &MemberGear| MemberGear {
            addendum: 1.4,
            dedendum: 1.65,
            ..g.clone()
        };
        let bending_of = |sharing: LoadSharing| {
            let mut spur = arr::pair([25, 80]);
            spur.set_load_sharing(sharing);
            for m in &mut spur.members {
                m.gear = tall(&m.gear);
            }
            let mut set = arr::planetary(12, 30, 72, 3);
            set.set_load_sharing(sharing);
            // Named rather than the shipped counts: a set with a small sun
            // cannot reach the band on its sun mesh at any addendum a tooth
            // can carry, and what this test asks is of a set that does.
            for m in &mut set.members {
                m.gear = tall(&m.gear);
            }
            // Tall as asked on the ring too, whose tip a sharp-tip bound
            // would otherwise hold lower, and at its shift as typed.
            set.members[2].gear.no_sharp_tip = false;
            set.members[2].gear.no_undercut = false;
            set.members[0].gear.teeth = 40;
            set.members[1].gear.teeth = 30;
            set.members[2].gear.teeth = 100;
            // **No hula stage**: its internal meshes, four teeth apart, foul
            // before they reach `ε_n = 2`, and the path cut at the usable
            // flank keeps them below the band, where the ramp is the unshared
            // rating by construction. Its members bend through the same
            // `Bending::of` as the set's planet and ring.
            let s = try_alone(&spur).unwrap();
            let p = try_alone(&set).unwrap();
            let mut out: Vec<(String, Option<f64>)> = Vec::new();
            for (i, g) in s.members.iter().enumerate() {
                out.push((format!("spur {i}"), g.cases[0].bending_stress));
            }
            for (what, g) in [
                ("sun", &p.members[0]),
                ("planet", &p.members[1]),
                ("ring", &p.members[2]),
            ] {
                out.push((what.to_string(), g.cases[0].bending_stress));
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
            rated >= 5,
            "every member should have a bending rating: {off:?}"
        );
    }

    /// **...and below that band it changes nothing, which is not the same as
    /// not reaching them.**
    ///
    /// The ramp's worst point is the largest `Y_F · Y_S · share`, and below
    /// `ε_n = 2` the single-pair zone, with a share of exactly 1, holds the
    /// maximum — almost always at the point the unshared rating already took
    /// (`strength::bending_section_shared` says where it is not), and on a
    /// hula's meshes there. A hula stage is the
    /// case worth pinning: **its meshes cannot reach the band at any proportion
    /// it can be built at**, running just above continuous contact by
    /// construction, so the control is offered and provably cannot bite there.
    #[test]
    fn below_the_band_the_model_reports_the_tooth_it_was_given() {
        use crate::contact::LoadSharing;
        let lib = library();
        let solve = |sharing| {
            let mut stage = hula_shape([65, 61, 57, 61]);
            stage.set_load_sharing(sharing);
            solve_hula(&stage, 2.0, 0.0, &lib).unwrap()
        };
        let off = solve(LoadSharing::None);
        let on = solve(LoadSharing::LinearRamp);
        for (a, b) in off.members.iter().zip(&on.members) {
            assert_eq!(
                a.cases[0].bending_stress, b.cases[0].bending_stress,
                "z{}: below the band the model has nothing to find",
                a.params.teeth
            );
        }
        // ...and the reason, rather than the symptom: the shipped stage's
        // meshes are nowhere near the band. If a hula stage ever is, this fails
        // and the paragraph above needs rewriting.
        for m in &off.meshes {
            assert!(
                m.line.unwrap().contact_ratios.transverse < 2.0,
                "a hula mesh above the band would change the claim: {}",
                m.line.unwrap().contact_ratios.transverse
            );
        }
    }

    /// **Every search costs like an input**, counted rather than timed.
    ///
    /// Two claims per search, in work no machine changes:
    ///
    /// - **It evaluates exactly what its structure says.** Each call of
    ///   `auto::Search::maximise` evaluates every point of its sweep,
    ///   `(scan + 1)^dof`, each walk's start, and every one of the
    ///   `3^dof − 1` directions in each round of its walks
    ///   ([`crate::testing::work::Walked::evaluations`]). The walks and
    ///   rounds are the search's own control flow, recorded where it runs;
    ///   the count of evaluations must equal the sum, so a candidate evaluated
    ///   twice, or a point evaluated that the structure does not name, fails.
    /// - **A candidate is scored at most once per evaluation**: the trains the
    ///   searched solve scores, less those the same shape scores unsearched,
    ///   are no more than the evaluations and the answer's own checks (a
    ///   one-part search scores its answer at most twice after it: at the
    ///   latest plan, then at the round's), so an objective that solves a
    ///   candidate twice fails.
    /// - **A snapshot of the totals**, a change detector and not a law. How
    ///   many rounds a walk takes, and how many teeth and rings a candidate
    ///   cuts (the shape keeps the teeth it has cut; the hula sizes its crank by
    ///   a root over built teeth, a data-dependent number of trials), follow
    ///   from the loss surface, not from any bound the search's definition
    ///   gives. They are deterministic, so they are recorded exactly, as the
    ///   corpus records a figure: a move is a question to answer and re-record,
    ///   not evidence of a fault.
    ///   Doubling `Search::SHIPPED.starts` takes the evaluations to 2 249,
    ///   4 120 and 712 (measured before tips were held off their mates'
    ///   flanks, which moved the set's own count from 3 846 to 2 710).
    ///   Holding them re-cuts every held member once per candidate, which is
    ///   what moved the teeth and rings: the pair 1 822 → 4 912 teeth, the
    ///   set 4 930 → 7 525 teeth and 961 → 3 983 rings, the hula 1 361 →
    ///   2 135 teeth and 8 388 → 9 130 rings.
    ///
    /// A Layshaft is not here, and is the case this count exists to see: with
    /// its search on it evaluates about 46 000 candidates and cuts about
    /// 99 000 teeth per solve, about 1.4 s native (the audit's lens-performance#3).
    #[test]
    fn every_search_costs_like_an_input() {
        use crate::testing::work;
        let lib = library();
        // name, (evaluations, trials, teeth, rings) recorded, the solve.
        let each =
            |name: &str, recorded: (u64, u64, u64, u64), searched: &dyn Fn(), plain: &dyn Fn()| {
                let (unsearched, ()) = work::of(plain);
                let (w, (calls, ())) = work::of(|| work::searches(searched));
                let structural: u64 = calls.iter().map(|c| c.evaluations()).sum();
                assert_eq!(
                w.evaluations, structural,
                "the {name} search evaluated {} candidates where its structure names {structural} \
                 ({calls:?})",
                w.evaluations
            );
                // The answer's checks after a one-part search: at most two.
                const ANSWER_CHECKS: u64 = 2;
                assert!(
                    w.trials - unsearched.trials <= w.evaluations + ANSWER_CHECKS,
                    "the {name} search scored {} candidates in {} evaluations",
                    w.trials - unsearched.trials,
                    w.evaluations
                );
                assert_eq!(
                    (w.evaluations, w.trials, w.teeth, w.rings),
                    recorded,
                    "the {name} search's (evaluations, trials, teeth, rings) moved"
                );
            };

        let pair = |search: bool| {
            let mut s = arr::pair([17, 43]);
            s.set_search(search);
            s
        };
        let (on, off) = (pair(true), pair(false));
        each(
            "pair's",
            (1_967, 1_968, 4_912, 0),
            &|| drop(try_alone(&on).unwrap()),
            &|| drop(try_alone(&off).unwrap()),
        );

        let set = |search: bool| {
            let mut s = arr::planetary(12, 30, 72, 3);
            s.set_search(search);
            s.members[0].gear.profile_shift = Auto::automatic(0.0);
            s.members[2].gear.profile_shift = Auto::automatic(0.0);
            s
        };
        let (on, off) = (set(true), set(false));
        each(
            "epicyclic set's",
            (2_710, 2_711, 7_525, 3_983),
            &|| drop(try_alone(&on).unwrap()),
            &|| drop(try_alone(&off).unwrap()),
        );

        let drive = |search: bool| {
            let mut s = hula_shape([65, 61, 57, 61]);
            s.set_search(search);
            s
        };
        let (on, off) = (drive(true), drive(false));
        each(
            "hula stage's",
            (524, 525, 2_135, 9_130),
            &|| drop(solve_hula(&on, 2.0, 0.0, &lib).unwrap()),
            &|| drop(solve_hula(&off, 2.0, 0.0, &lib).unwrap()),
        );
    }

    /// **Every search runs on every keystroke**: the order of magnitude, by
    /// the clock.
    ///
    /// A timing canary, ignored in the suite and run alone and serially in CI:
    /// wall-clock measures the machine as much as the code (it failed 21 of 40
    /// runs under load when it was the gate). `every_search_costs_like_an_input`
    /// is the gate, in counts. Each ceiling is a decade over what its search
    /// costs alone (about 8, 39 and 25 ms), which is what an order of
    /// magnitude means; the regressions recorded against this were 800, 100
    /// and 68 ms, each something built per candidate that nothing then read.
    #[test]
    #[ignore = "timing canary: run serially (cargo nextest run --run-ignored only)"]
    fn every_search_is_quick_enough_to_type_over() {
        let lib = library();
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

        let pair = {
            let mut s = arr::pair([17, 43]);
            s.set_search(true);
            s
        };
        each("pair's", 80, &|| {
            try_alone(&pair).unwrap();
        });

        let mut set = arr::planetary(12, 30, 72, 3);
        set.set_search(true);
        set.members[0].gear.profile_shift = Auto::automatic(0.0);
        set.members[2].gear.profile_shift = Auto::automatic(0.0);
        each("epicyclic set's", 400, &|| {
            try_alone(&set).unwrap();
        });

        let mut drive = hula_shape([65, 61, 57, 61]);
        drive.set_search(true);
        each("hula stage's", 250, &|| {
            solve_hula(&drive, 2.0, 0.0, &lib).unwrap();
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
            let mut s = arr::pair([9, 37]);
            for m in &mut s.members {
                m.gear.root_radius = rho;
                // The root round's bound alone. Held, a mate's tip follows
                // the higher form circle a larger round leaves, and the
                // shorter path it cuts is efficiency the search then buys
                // with shift — a different claim.
                m.gear.no_tip_past_mate_flank = false;
            }
            s.set_search(true);
            s
        };
        let mut last = f64::INFINITY;
        let mut fell = false;
        let mut uncuttable = 0;
        // Up to ρ = 0.5: on 9/37 a round from 0.45 cannot be cut at any
        // shift, so both branches below are taken.
        for k in 0..=10 {
            let rho = f64::from(k) * 0.05;
            let s = stage(rho);
            let x = s.shifts();
            let sum = x[0] + x[1];
            // Where it optimised at all, the teeth it chose can be cut.
            let cuttable = (0..2).all(|i| {
                let p = s.params_of(i, x[i]);
                crate::auto::root_radius_fits(&p, p.dedendum)
            });
            if !cuttable {
                // The round is unreachable at every shift; nothing was chosen.
                assert_eq!(x, {
                    let mut plain = s.clone();
                    plain.set_search(false);
                    plain.shifts()
                });
                uncuttable += 1;
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
        assert!(
            uncuttable > 0,
            "no round was out of reach: the fallback went unasked"
        );
    }

    /// The efficiency toggle is **additive**: a stage that never asked for it
    /// answers exactly as it did before the toggle existed, and a stage that
    /// does is moved somewhere else.
    #[test]
    fn a_part_that_did_not_ask_keeps_the_shifts_it_had() {
        let stage = |on: bool| {
            let mut s = arr::pair([17, 43]);
            s.set_search(on);
            s
        };
        let plain = stage(false).shifts();
        let tuned = stage(true).shifts();
        assert!(
            (tuned[0] + tuned[1]) - (plain[0] + plain[1]) > 0.05,
            "the toggle should move the pair off its undercut floor, {tuned:?} from {plain:?}"
        );
        // ...and where it moves it, the pair loses less than it did.
        let loss = |on: bool| 1.0 - try_alone(&stage(on)).unwrap().meshes[0].efficiency.forward;
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
    fn every_preset_chooses_a_member_that_can_be_cut() {
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

        let spur = {
            let mut s = arr::pair([17, 43]);
            s.set_search(true);
            s
        };
        for (i, x) in spur.shifts().iter().enumerate() {
            cuttable(&spur.params_of(i, *x), "the pair's gear");
        }

        let mut set = arr::planetary(12, 30, 72, 3);
        set.set_search(true);
        set.members[0].gear.profile_shift = Auto::automatic(0.0);
        set.members[2].gear.profile_shift = Auto::automatic(0.0);
        let shape = set;
        let built = shape
            .build_at(&shape.shifts_at(&crate::auto::Search::SHIPPED))
            .expect("the set has geometry");
        cuttable(built.members[0].params(), "the sun");
        cuttable(built.members[1].params(), "the planet");

        // The hula stage's rack-generated members are its pinions; its
        // rings are the shaper's and are not asked.
        let mut shape = hula_shape([65, 61, 57, 61]);
        shape.set_search(true);
        let built = shape
            .build_at(&shape.shifts_at(&crate::auto::Search::SHIPPED))
            .expect("the stage solves");
        for (i, m) in shape.members.iter().enumerate() {
            if m.ring.is_some() {
                continue;
            }
            cuttable(built.members[i].params(), "the stage's pinion");
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
        let free = try_alone(&arr::pair([17, 43])).unwrap();
        // A housing the pair can actually meet: a clearance inside it is the
        // distance the automatic solve already closes to.
        let asked = free.distances[0].nominal[0] + 0.05;
        let at = |on: bool| {
            let mut s = arr::pair([17, 43]);
            s.set_search(on);
            s.distances[0].distance = Auto::fixed(asked);
            s.distances[0].clearance = Auto::fixed(0.05);
            s
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
        let pinned = try_alone(&at(false)).unwrap();
        assert!(
            (pinned.distances[0].clearance
                - (pinned.distances[0].running - pinned.distances[0].nominal[0]))
                .abs()
                < 1e-12,
            "read {} against a gap of {}",
            pinned.distances[0].clearance,
            pinned.distances[0].running - pinned.distances[0].nominal[0]
        );
        assert!(
            (pinned.distances[0].clearance - 0.05).abs() < 1e-9,
            "the housing is 0.05 outside the nominal, so the gap is 0.05, not {}",
            pinned.distances[0].clearance
        );

        // The shifts free: they take it, and the backlash is the one asked for.
        let chosen = try_alone(&at(true)).unwrap();
        assert!((chosen.distances[0].clearance - 0.05).abs() < 1e-12);
        assert!(
            (chosen.distances[0].running - asked).abs() < 1e-9,
            "the housing still holds: {}",
            chosen.distances[0].running
        );
        assert!(
            (chosen.distances[0].running - chosen.distances[0].nominal[0] - 0.05).abs() < 1e-6,
            "the pair should close to zero backlash 0.05 inside the housing, not {}",
            chosen.distances[0].running - chosen.distances[0].nominal[0]
        );

        // And with the distance automatic it is read either way, as it always was.
        for on in [false, true] {
            let r = try_alone(&{
                let mut s = arr::pair([17, 43]);
                s.set_search(on);
                s.distances[0].clearance = Auto::fixed(0.05);
                s
            })
            .unwrap();
            assert!((r.distances[0].clearance - 0.05).abs() < 1e-12);
        }
    }

    /// A centre distance the designer typed is a **constraint on the pair**, not
    /// a suggestion the optimiser may overrule: the shifts it chooses still add
    /// up to the housing it was given, and only their split is free.
    #[test]
    fn a_given_centre_distance_still_sets_the_distance() {
        let free = try_alone(&arr::pair([17, 43])).unwrap();
        let asked = free.distances[0].nominal[0] + 0.4;
        let stage = {
            let mut s = arr::pair([17, 43]);
            s.set_search(true);
            s.distances[0].distance = Auto::fixed(asked);
            s
        };
        let r = try_alone(&stage).unwrap();
        assert!(
            (r.distances[0].running - asked).abs() < 1e-9,
            "asked for {asked}, ran at {}",
            r.distances[0].running
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
    fn a_train_that_fails_names_the_part_that_failed() {
        let mut train = two_pairs();
        train.chain_on(arr::pair([17, 43]));
        assert!(
            solve_train(&train, &library()).is_ok(),
            "three good stages solve"
        );

        // A centre distance of a millimetre is no mesh any shift reaches, and
        // it is the middle stage's. (Nought is no distance at all, refused
        // where it enters by its field: `crate::input`.)
        let d = train.parts()[1].distances[0];
        train.shape.distances[d].distance = Auto::fixed(1.0);

        let e = solve_train(&train, &library()).expect_err("a mesh at no distance is not a train");
        let TrainError::InPart { part, cause } = &e else {
            panic!("the failure should name the part it happened in, got {e:?}")
        };
        assert_eq!(*part, 1, "the middle part is the one that failed");
        // ...and the reason is the stage's own, unchanged by being carried.
        assert!(
            matches!(**cause, TrainError::Mesh(_)),
            "the cause should be the mesh's, got {cause:?}"
        );
        // The note a reader sees is the cause's, so nothing is invented to
        // carry the number.
        use crate::note::Explain;
        assert_eq!(e.note().key, cause.note().key);
        // ...and carries the part, numbered as the panel numbers it.
        assert_eq!(e.note().values["part"], "2", "{e}");
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
        let given = |teeth: u32, x: f64| MemberGear {
            teeth,
            profile_shift: Auto::fixed(x),
            ..MemberGear::default()
        };
        let tuned = |gears: [MemberGear; 2]| {
            let mut s = arr::pair([gears[0].teeth, gears[1].teeth]);
            for (m, g) in s.members.iter_mut().zip(gears) {
                m.gear = g;
            }
            s.set_search(true);
            s
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
        let loose = |teeth: u32, x: f64| MemberGear {
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
        let auto = try_alone(&arr::pair([17, 43])).unwrap();

        // The same distance, set by hand, with a clearance that must be ignored.
        let manual = try_alone(&{
            let mut s = arr::pair([17, 43]);
            s.distances[0].distance = Auto::fixed(auto.distances[0].nominal[0]);
            s.distances[0].clearance = Auto::fixed(0.5);
            s
        })
        .unwrap();

        assert!((manual.distances[0].running - auto.distances[0].nominal[0]).abs() < 1e-12);
        // At the zero-backlash distance there is, by construction, no backlash.
        assert!(manual.meshes[0].backlash_by_drive().forward.nominal.abs() < 1e-9);
        // Whereas the automatic one carries its clearance into real backlash.
        assert!(auto.meshes[0].backlash_by_drive().forward.nominal > 0.0);
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
        let off = ByKind {
            ultimate: false,
            fatigue: false,
        };
        let auto_width = |sources: FaceSources, o: Overrides| {
            let mut s = arr::pair([17, 43]);
            for g in s.members.iter_mut().map(|m| &mut m.gear) {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
                g.material_overrides = o;
            }
            try_alone(&s).unwrap()
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
            // Contact ultimate is first yield, `C·σ_y`, so doubling the
            // yield stress doubles it; contact fatigue is its own column.
            let over = match case {
                CaseKind::Ultimate => Overrides {
                    ultimate_allowable: Some(
                        2.0 * base.members[0].material.ultimate_allowable.value,
                    ),
                    ..Default::default()
                },
                CaseKind::Fatigue => Overrides {
                    contact_fatigue_allowable: Some(
                        2.0 * allowable(
                            &base.members[0].material,
                            Rating::Contact { aspect: 0.0 },
                            case,
                        )
                        .unwrap(),
                    ),
                    ..Default::default()
                },
            };

            let width = |sources, o| auto_width(sources, o).members[0].face_width;
            let ratio = base.members[0].face_width / width(contact_only(case), over);
            assert!(
                (ratio - 4.0).abs() < 1e-9,
                "{case:?}: doubling the allowable should quarter the width: ratio {ratio}"
            );
            // ...and it moved only the case it belongs to.
            let untouched = width(contact_only(other), Overrides::default());
            assert_eq!(untouched, width(contact_only(other), over));
        }

        // ...and the reported material says the number came from the user.
        // Both allowables doubled: a cyclic figure above the static one is no
        // material, refused where it enters (`crate::input::resolved`).
        let doubled = auto_width(
            FaceSources::default(),
            Overrides {
                ultimate_allowable: Some(2.0 * 1365.0),
                fatigue_allowable: Some(2.0 * 750.0),
                ..Default::default()
            },
        );
        assert_eq!(
            doubled.members[0].material.fatigue_allowable.basis,
            crate::material::Basis::Overridden
        );
        assert_eq!(
            auto_width(FaceSources::default(), Overrides::default()).members[0]
                .material
                .fatigue_allowable
                .basis,
            crate::material::Basis::Estimated
        );
    }

    /// Overriding the modulus moves contact stress, and by the right law.
    #[test]
    fn overriding_the_modulus_moves_contact_stress_as_the_square_root() {
        let at = |e: Option<f64>| {
            let mut s = arr::pair([17, 43]);
            for g in s.members.iter_mut().map(|m| &mut m.gear) {
                g.material_overrides = Overrides {
                    elastic_modulus: e,
                    ..Default::default()
                };
            }
            try_alone(&s).unwrap().meshes[0].cases[0]
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
        let mut s = arr::pair([17, 43]);
        s.members[0].gear.material = "unobtainium".into();
        let e = try_alone(&s).unwrap_err();
        assert!(matches!(e, TrainError::UnknownMaterial(ref n) if n == "unobtainium"));
        assert!(e.to_string().contains("unobtainium"));
    }

    /// **A train with no stages is a train**: it solves to nothing rated,
    /// no figure of its own, and every case waiting — so a designer who
    /// removes the last stage keeps the cases.
    #[test]
    fn an_empty_train_solves_to_nothing() {
        let t = Train {
            load_cases: vec![LoadCase::ultimate(1, 2, 1.0, 1.0)],
            reversed_bending: false,
            shape: Shape::default(),
            held: Vec::new(),
        };
        let r = solve_train(&t, &library()).expect("a train with no stages is a train");
        assert!(r.by_part.is_empty() && r.paths.is_empty());
        assert_eq!(r.cases.len(), 1);
        assert!(!r.cases[0].solved);
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
        let t = two_pairs();
        let with = |speed: f64, duty: Duty| {
            let mut t = two_pairs();
            t.load_cases[CYCLIC].set_speed(speed);
            t.load_cases[CYCLIC].duty = duty;
            t
        };
        for (train, what) in [
            (two_pairs(), "two spur stages"),
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
                        at: Some(end_of(&t)),
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
                        at: Some(end_of(&t)),
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
            let ratios: Vec<f64> = r.by_part.iter().map(reduction).collect();
            for (k, s) in r.by_part.iter().enumerate() {
                let upstream: f64 = ratios[..k].iter().product();
                let speed_in = case.speed().unwrap() / upstream;
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

                for (i, (g, want)) in s.members.iter().zip(expected).enumerate() {
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
        let weak = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            ..Default::default()
        };
        let mut asked = 0;
        // A matched pair, then each gear in turn made much weaker than the
        // other, so whichever gear governs the mesh is the one that changes.
        for over in [
            [Overrides::default(), Overrides::default()],
            [weak(250.0), Overrides::default()],
            [Overrides::default(), weak(250.0)],
        ] {
            let mut stage = arr::pair([17, 43]);
            for (g, o) in stage.members.iter_mut().map(|m| &mut m.gear).zip(over) {
                g.face_width = Auto::automatic(0.0);
                g.material_overrides = o;
                // Every rating sizes the width, so every one is asked below:
                // contact sizes none by default.
                g.face_sources = FaceSources {
                    bending: ByKind {
                        ultimate: true,
                        fatigue: true,
                    },
                    contact: ByKind {
                        ultimate: true,
                        fatigue: true,
                    },
                };
            }
            let r = try_alone(&stage).unwrap();
            let effective = r.members[0].face_width.min(r.members[1].face_width);
            assert!(effective > 0.0);

            for (i, g) in r.members.iter().enumerate() {
                // Every case, whatever its kind: the width answers to all of them.
                for (case, kind) in g.cases.iter().zip(CaseKind::BOTH) {
                    let asks = case.min_face_width;
                    for (source, needs) in [("contact", asks.contact), ("bending", asks.bending)] {
                        let needs = needs.unwrap_or_else(|| {
                            panic!("gear {i} {kind:?}: {source} asks for no width")
                        });
                        assert!(
                            effective >= needs * (1.0 - 1e-9),
                            "gear {i} {kind:?} {source} needs {needs} mm, mesh carries {effective}"
                        );
                        asked += 1;
                    }
                }
            }
        }
        // Three pairs, two gears, two cases, two ratings each.
        assert_eq!(asked, 3 * 2 * 2 * 2);
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
        // At a **fixed** width: an automatic one is inverted from the stress, so
        // it lands the stress on the allowable and hides the material.
        let solved = |auto: bool, over: [Overrides; 2]| {
            let mut s = arr::pair([17, 43]);
            for (g, o) in s.members.iter_mut().map(|m| &mut m.gear).zip(over) {
                g.face_width = if auto {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(10.0)
                };
                g.material_overrides = o;
            }
            try_alone(&s).unwrap()
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
        let pitch = |r: &shape::ShapeResult| r.meshes[0].cases[0].contact.at_pitch_point;
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
            base.members[0].cases[0].contact_stress,
            base.members[1].cases[0].contact_stress,
        );
        assert!(
            a != b,
            "17/43 is not symmetric, so its two gears are not rated alike: {a} and {b}"
        );
        for g in &base.members {
            assert!(g.cases[0].contact_stress >= pitch(&base));
        }
        // ...and the envelope is the worse of them, which is what the *mesh*
        // would be rated on with no member named.
        assert!((a.max(b) - base.members[0].cases[0].contact_stress.max(b)).abs() < 1e-12);

        // --- the allowable. At a fixed width again, and for a reason worth
        // stating: with an *automatic* width the allowable does reach the stress,
        // because it moves the width the pair is rated at. Held still, it moves
        // only what it should.
        let allowable = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            contact_fatigue_allowable: Some(x),
            ..Default::default()
        };
        let wide = base;
        let half = allowable(
            0.5 * super::allowable(
                &wide.members[1].material,
                super::Rating::Contact { aspect: 0.0 },
                CaseKind::Fatigue,
            )
            .unwrap(),
        );
        let derated = solved(false, [Overrides::default(), half]);
        assert_eq!(
            derated.meshes[0].cases[0].contact.at_pitch_point,
            wide.meshes[0].cases[0].contact.at_pitch_point,
            "an allowable is not a stress and must not move one"
        );
        let contact_width = |r: &shape::ShapeResult| {
            r.members[1].cases[1]
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
            derated.members[0].cases[1].min_face_width.contact,
            wide.members[0].cases[1].min_face_width.contact,
            "and it must not reach the other gear"
        );
    }

    /// **A load between two stages goes the way that holds it.** A worm
    /// ahead of a pair locks against a load from its wheel; a load put on
    /// the body the two share, with both far ends free — loads of nought —
    /// can only be held by the worm, so the worm is loaded and the pair is
    /// not. The same load with the far ends reacted as well could be held
    /// at either, which is a division this model does not make, and is
    /// refused as such.
    #[test]
    fn a_load_between_two_parts_goes_the_way_that_holds_it() {
        let lib = library();
        let mut t = train_of(vec![arr::worm(1, 40), arr::pair([17, 43])]);
        // The shared body: the worm's wheel, one with the pair's first.
        t.load_cases[BACK].loads = vec![
            Load::given(t.port(0, 2), 0.5, 0.0),
            free(start_of(&t)),
            free(end_of(&t)),
        ];
        let r = solve_train(&t, &lib).expect("the worm holds it");
        assert!(r.cases[BACK].solved, "held by the worm");
        let worm = &r.by_part[0].members;
        let pair = &r.by_part[1].members;
        assert!(
            worm[1].cases[BACK].torque > 0.0,
            "the wheel carries the load"
        );
        assert_eq!(
            pair[0].cases[BACK].torque, 0.0,
            "the pair carries none of it"
        );
        assert_eq!(pair[1].cases[BACK].torque, 0.0);
        // ...and the same body named from the pair's side is the same body.
        let mut same = t.clone();
        assert_eq!(t.port(1, 1), t.port(0, 2));
        same.load_cases[BACK].set_port(t.port(1, 1));
        let again = solve_train(&same, &lib).expect("the same body");
        assert!(again.cases[BACK].solved);
        assert!(
            (again.by_part[0].members[1].cases[BACK].torque - worm[1].cases[BACK].torque).abs()
                < 1e-12
        );
        // The far ends reacted as well: two bodies could hold it, and a
        // refusal.
        t.load_cases[BACK].loads = vec![
            Load::given(t.port(0, 2), 0.5, 0.0),
            Load::declared(start_of(&t), LoadRole::Reacted),
            Load::declared(end_of(&t), LoadRole::Reacted),
        ];
        let r = solve_train(&t, &lib).expect("a shared load is a case's question");
        assert!(!r.cases[BACK].solved);
        assert!(r.cases[BACK]
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_LOAD_SHARED)));
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
        let mut t = two_pairs();
        t.load_cases[BACK].set_torque(5.0);
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        for s in &r.by_part {
            for g in &spur(s).members {
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
        assert!(!r.cases[BACK].solved);
        assert_eq!(at_port(&r, &t, BACK, start_of(&t)).torque, 0.0);

        // **Held at the far end**, the same load reaches every gear. The
        // torque at each stage's input is the load referred by every ratio
        // and every backward efficiency between it and the end, which is the
        // one walk the forward case takes the other way.
        let mut held = t.clone();
        held.load_cases[BACK] = LoadCase::back_driving(start_of(&t), end_of(&t), 5.0);
        let h = solve_train(&held, &lib).unwrap();
        // Nothing to say of it but what every rated case says.
        assert!(h.cases[BACK]
            .notes
            .iter()
            .all(|n| n.is(key::TRAIN_STRESSES_NOMINAL)));
        let mut at = 5.0;
        for s in h.by_part.iter().rev() {
            let p = spur(s);
            let expect = at / reduction(p);
            assert!(
                (p.members[0].cases[BACK].torque.abs() - expect).abs() < 1e-9 * expect,
                "the load referred to this stage's input is {expect}, not {}",
                p.members[0].cases[BACK].torque
            );
            at = expect * p.meshes[0].efficiency.backward;
        }
        let start = at_port(&h, &held, BACK, start_of(&t));
        assert_eq!(start.role, BodyRole::Reacted);
        assert!((start.torque.abs() - at).abs() < 1e-12);

        // The same load against a stage that cannot be driven backward. A worm
        // with enough friction locks, and then the load stops there: the worm
        // stage carries it, and the spur stage ahead of it carries none.
        t.chain_on({
            let mut s = arr::worm(1, 40);
            s.meshes[0].sliding_friction = 0.3;
            s.meshes[0].static_friction = 0.3;
            s
        });
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        let (worm, screw) = worm(&r.by_part[2]);
        assert!(
            screw.efficiency.locked().backward,
            "this worm was meant to lock: backward efficiency {}",
            screw.efficiency.backward
        );
        // The wheel is on the body the load enters by and carries all of it,
        // and the worm's teeth carry it read across the mesh; the worm is at
        // the far end of a mesh that cannot pass it, so its *body* carries
        // **none** — which is what a locked stage means and is not the same
        // as the case being absent (`a_self_locking_worm_reports_the_load_it_reacts`).
        let teeth: Vec<f64> = worm.members.iter().map(|m| m.cases[BACK].torque).collect();
        assert!(
            (teeth[1] - 5.0).abs() < 1e-12 && (teeth[0] - 5.0 / reduction(worm)).abs() < 1e-12,
            "the stage that reacts the load carries it on both members' teeth: {teeth:?}"
        );
        assert_eq!(
            worm.cases[BACK].torques[1], 0.0,
            "a locked mesh delivers nothing to the worm's shaft"
        );
        for s in &r.by_part[..2] {
            for g in &spur(s).members {
                assert_eq!(
                    g.cases[BACK].torque, 0.0,
                    "nothing upstream of a self-locking stage sees the load"
                );
            }
        }

        // ...and the case solved, nothing reaching the free start.
        assert!(
            r.cases[BACK].solved
                && r.cases[BACK]
                    .notes
                    .iter()
                    .all(|n| n.is(key::TRAIN_STRESSES_NOMINAL))
        );
        assert_eq!(at_port(&r, &t, BACK, start_of(&t)).torque, 0.0);

        // **The two torques are two facts.** Put the locking stage first, so
        // the spur stages after it carry the load as well: a gear's torque in
        // the case from the start must stay what it was even when the case
        // from the end is the larger of the two.
        let mut t = two_pairs();
        t.load_cases[BACK].set_torque(500.0);
        {
            let mut s = t.part_shapes();
            s.insert(0, {
                let mut s = arr::worm(1, 40);
                s.meshes[0].sliding_friction = 0.3;
                s.meshes[0].static_friction = 0.3;
                s
            });
            rechain(&mut t, s);
        }
        let r = match solve_train(&t, &lib) {
            Ok(r) => r,
            Err(e) => {
                println!("PROBE refused: {e}");
                panic!("probe")
            }
        };
        let mut forward_only = t.clone();
        forward_only.load_cases[BACK].set_torque(0.0);
        let forward_only = solve_train(&forward_only, &lib).unwrap();
        let mut seen = 0;
        for (loaded, plain) in r.by_part[1..].iter().zip(&forward_only.by_part[1..]) {
            for (g, unloaded) in spur(loaded).members.iter().zip(&spur(plain).members) {
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
    /// shape rates through one ([`scaled`]); the scale is
    /// exact in the mathematics, and this holds it to the digits the corpus
    /// prints. **Run against a scale read from the wrong case, it fails on
    /// every member**, which is the fault this is for.
    #[test]
    fn a_case_is_rated_on_its_own_whatever_else_the_train_carries() {
        let lib = library();
        let mut train = two_pairs();
        train.chain_on(arr::planetary(12, 30, 72, 3));
        train.chain_on(hula());
        let (start, end) = ends_of(&train);
        train.load_cases = vec![
            LoadCase::ultimate(start, end, 2.0, 3000.0),
            LoadCase {
                loads: vec![
                    Load::given(end, 0.7, 40.0),
                    Load::declared(start, LoadRole::Reacted),
                ],
                ..LoadCase::fatigue(start, end, 0.7, 40.0)
            },
        ];
        let alone = solve_train(&train, &lib).expect("solves");

        // The same two cases, with a heavier one in front of them and a
        // switched-off one after: the first two cases' figures are unmoved.
        let mut crowded = train.clone();
        crowded
            .load_cases
            .insert(0, LoadCase::ultimate(start, end, 50.0, 100.0));
        crowded.load_cases.push(LoadCase {
            enabled: false,
            ..LoadCase::ultimate(start, end, 1.0e6, 1.0)
        });
        let with = solve_train(&crowded, &lib).expect("solves");
        assert_eq!(
            with.cases.len(),
            4,
            "a case switched off is still solved at the train level"
        );
        assert!(with.cases[3].solved && with.cases[3].shaft(GROUND).is_some());
        let near = |x: f64, y: f64| (x - y).abs() <= 1e-9 * x.abs().max(1e-300);
        let mut checked = 0;
        for (a, b) in alone.by_part.iter().zip(&with.by_part) {
            for (ga, gb) in a.members.iter().zip(&b.members) {
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
            vec![0, 1, 2, 3]
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
    fn a_forward_locked_mesh_holds_a_load_from_the_start() {
        let lib = library();
        let mut train = two_pairs();
        {
            let mut s = train.part_shapes();
            s.insert(0, arr::worm(17, 23).with_first_helix(9.0));
            rechain(&mut train, s);
        }
        let r = solve_train(&train, &lib).expect("solves");
        assert!(
            r.by_part[0].meshes[0].efficiency.locked().forward,
            "this fixture is meant to be forward-locked"
        );
        let start = &r.cases[PEAK];
        assert!(start.solved);
        assert_eq!(at_port(&r, &train, PEAK, end_of(&train)).torque, 0.0);
        // The locked pair carries it on the member the load is on...
        assert!(r.by_part[0].members[0].cases[PEAK].torque > 0.0);
        // ...and nothing beyond it sees any.
        for s in &r.by_part[1..] {
            for g in &s.members {
                assert_eq!(g.cases[PEAK].torque, 0.0);
            }
        }
    }

    /// **A train with no load case is a shaft line**, and every preset solves it.
    ///
    /// Ratios, efficiencies and backlash stand; every member reports no case;
    /// and an automatic face width, with nothing to ask, stands at its box —
    /// the same answer as no source switched on, since both are a width with
    /// nothing to choose between. A case switched off is the same train.
    #[test]
    fn a_train_with_no_load_case_is_a_shaft_line() {
        let lib = library();
        for off in [false, true] {
            let mut train = two_pairs();
            train.chain_on(arr::worm(1, 40));
            train.chain_on(arr::planetary(12, 30, 72, 3));
            train.chain_on(hula());
            for i in train.parts()[0].members.clone() {
                train.shape.members[i].gear.face_width = Auto::automatic(7.0);
            }
            train.load_cases = if off {
                vec![LoadCase {
                    enabled: false,
                    ..template(&train)
                }]
            } else {
                Vec::new()
            };
            let r = solve_train(&train, &lib).expect("a shaft line solves");
            assert_eq!(r.cases.len(), train.load_cases.len());
            // **A path is a case's**, and a shaft line with none switched on
            // reports none: a reading nobody asked for is no reading.
            assert!(r.paths.is_empty(), "{:?}", r.paths);
            for s in &r.by_part {
                for g in &s.members {
                    assert!(g.cases.is_empty());
                }
                for m in &s.meshes {
                    assert!(m.cases.is_empty());
                }
            }
            for g in &spur(&r.by_part[0]).members {
                assert_eq!(g.face_width, 7.0);
            }
        }
    }

    /// Every path's band, each way, as `[nominal, minimum, maximum]` in
    /// degrees, in the order the solve lists its paths.
    fn bands(t: &Train) -> Vec<[f64; 3]> {
        solve_train(t, &test_library())
            .unwrap_or_else(|e| panic!("{e}"))
            .paths
            .iter()
            .flat_map(|p| [p.backlash.forward, p.backlash.backward])
            .map(|b| {
                [
                    b.unwrap().nominal,
                    b.unwrap().minimum.unwrap(),
                    b.unwrap().maximum.unwrap(),
                ]
            })
            .collect()
    }

    /// The trains the band laws sweep: every preset alone, every
    /// arrangement of a set, the hula, and every ordered pair of presets in
    /// a chain, each with a case along its conventional ends.
    fn band_trains() -> Vec<(String, Train)> {
        use crate::planetary::{Arrangement, PlanetaryShaft};
        let mut out: Vec<(String, Train)> = Vec::new();
        for p in Preset::ALL {
            out.push((format!("{p:?}"), Train::alone(&p.build(), 2.0, 3000.0)));
        }
        let set = arr::planetary(12, 30, 72, 3);
        for input in PlanetaryShaft::ALL {
            for fixed in PlanetaryShaft::ALL {
                if input != fixed {
                    let arrangement = Arrangement { input, fixed };
                    out.push((
                        format!("{arrangement:?}"),
                        Train::alone(&set, 2.0, 3000.0).arranged_as(arrangement),
                    ));
                }
            }
        }
        out.push(("hula".into(), Train::alone(&hula(), 2.0, 3000.0)));
        for a in Preset::ALL {
            for b in Preset::ALL {
                let mut t = Train::chained(vec![a.build(), b.build()], |_| Vec::new());
                let mut case = t.fresh_case(CaseKind::Ultimate, 2.0, 3000.0);
                case.enabled = true;
                t.load_cases.push(case);
                out.push((format!("{a:?}+{b:?}"), t));
            }
        }
        out
    }

    /// **A path's backlash band is the sum of each distance's own**: the
    /// meshes on one distance move together, and independent distances
    /// stack to their worst case. So the band with every tolerance is the
    /// bands with each distance's tolerance alone, summed end by end, less
    /// the nominal counted once too often per extra distance — which
    /// contains every corner of the tolerance box and reaches each end at
    /// one. A distance alone is read at its own three points, the route no
    /// fold can get wrong, so the law holds the fold to it. Every preset,
    /// every arrangement of a set, and every ordered pair of presets.
    #[test]
    fn a_path_band_is_each_distance_band_summed() {
        let mut stacked = 0;
        for (name, t) in band_trains() {
            let all = bands(&t);
            assert!(!all.is_empty(), "{name}: no path to read a band on");
            let d = t.shape.distances.len();
            let alone: Vec<Vec<[f64; 3]>> = (0..d)
                .map(|keep| {
                    let mut u = t.clone();
                    for (j, x) in u.shape.distances.iter_mut().enumerate() {
                        if j != keep {
                            x.tolerance_plus = 0.0;
                            x.tolerance_minus = 0.0;
                        }
                    }
                    bands(&u)
                })
                .collect();
            for (p, b) in all.iter().enumerate() {
                let extra = (d.max(1) - 1) as f64 * b[0];
                let sum = |e: usize| alone.iter().map(|a| a[p][e]).sum::<f64>() - extra;
                let scale = b[2].abs().max(1e-3);
                for (e, what) in [(1, "minimum"), (2, "maximum")] {
                    assert!(
                        (b[e] - sum(e)).abs() <= 1e-9 * scale,
                        "{name}: path row {p}: {what} {:.6} against {:.6} summed",
                        b[e],
                        sum(e)
                    );
                }
                if d > 1 && (b[2] - b[1]) > 1e-9 {
                    stacked += 1;
                }
            }
        }
        assert!(
            stacked > 0,
            "no train stacks two distances: the law is vacuous"
        );
    }

    /// **Widening a tolerance never narrows the band**: each distance's
    /// tolerances doubled in turn, every path's band each way contains the
    /// one it had.
    #[test]
    fn widening_a_tolerance_never_narrows_a_band() {
        for (name, t) in band_trains() {
            let before = bands(&t);
            for d in 0..t.shape.distances.len() {
                let mut u = t.clone();
                u.shape.distances[d].tolerance_plus *= 2.0;
                u.shape.distances[d].tolerance_minus *= 2.0;
                for (p, (a, b)) in before.iter().zip(bands(&u)).enumerate() {
                    assert!(
                        b[1] <= a[1] + 1e-12 && b[2] >= a[2] - 1e-12,
                        "{name}: distance {d} widened, path row {p}: {a:?} became {b:?}"
                    );
                }
            }
        }
    }

    /// **The bands the audit measured**, at the shipped proportions: the
    /// meshed-planet set alone, whose three distances stack to a band from
    /// nought, a pair ahead of a planocentric at ±0.02 mm, and a planetary
    /// set, whose one distance holds its path's play stationary at every
    /// tolerance.
    #[test]
    fn the_stacked_bands_the_audit_measured() {
        let deg = |b: &Backlash| [b.minimum.unwrap(), b.maximum.unwrap()];
        let meshed = solve_train(
            &Train::alone(&Preset::MeshedPlanets.build(), 2.0, 3000.0),
            &test_library(),
        )
        .unwrap();
        let got = deg(&meshed.paths[0].backlash.forward.unwrap());
        assert!(
            (got[0] - 0.0).abs() < 5e-7 && (got[1] - 0.139_563).abs() < 5e-7,
            "{got:?}"
        );
        let mut t = Train::chained(
            vec![Preset::Spur.build(), Preset::Planocentric.build()],
            |_| Vec::new(),
        );
        for x in &mut t.shape.distances {
            x.tolerance_plus = 0.02;
            x.tolerance_minus = 0.02;
        }
        let mut case = t.fresh_case(CaseKind::Ultimate, 2.0, 3000.0);
        case.enabled = true;
        t.load_cases.push(case);
        let r = solve_train(&t, &test_library()).unwrap();
        let got = deg(&r.paths[0].backlash.forward.unwrap());
        assert!(
            (got[0] - 0.0).abs() < 5e-7 && (got[1] - 0.197_206).abs() < 5e-7,
            "{got:?}"
        );
        // A set's two meshes share one distance and move together: its
        // path's play is stationary there, whatever the tolerance.
        for tol in [0.0, 0.02, 0.05, 0.2] {
            let mut set = Preset::Planetary.build();
            set.distances[0].tolerance_plus = tol;
            set.distances[0].tolerance_minus = tol;
            let r = solve_train(&Train::alone(&set, 2.0, 3000.0), &test_library()).unwrap();
            let b = r.paths[0].backlash.forward;
            for v in [
                b.unwrap().nominal,
                b.unwrap().minimum.unwrap(),
                b.unwrap().maximum.unwrap(),
            ] {
                assert!((v - 0.041_724).abs() < 5e-7, "planetary at ±{tol}: {b:?}");
            }
        }
    }

    /// Every preset alone with three cases along its conventional ends:
    /// its own, one stalled — the input's torque given and its speed
    /// derived, the output held still with its torque derived — and one
    /// back-driven from the output held still.
    fn still_cases() -> Vec<(String, Train)> {
        Preset::ALL
            .iter()
            .map(|p| {
                let mut t = Train::alone(&p.build(), 2.0, 3000.0);
                let (input, output) = (t.load_cases[0].loads[0].at, t.load_cases[0].loads[1].at);
                t.load_cases.push(LoadCase {
                    loads: vec![
                        Load {
                            speed: Auto::automatic(0.0),
                            ..Load::given(input, 2.0, 0.0)
                        },
                        Load {
                            torque: Auto::automatic(0.0),
                            ..Load::given(output, 0.0, 0.0)
                        },
                    ],
                    ..LoadCase::ultimate(input, output, 2.0, 0.0)
                });
                t.load_cases
                    .push(LoadCase::back_driving(input, output, 2.0));
                (format!("{p:?}"), t)
            })
            .collect()
    }

    /// What a train's cases come to, every figure to the bit.
    fn cases_of(t: &Train) -> String {
        match solve_train(t, &test_library()) {
            Ok(r) => format!("{:?}", r.cases),
            Err(e) => format!("{e:?}"),
        }
    }

    /// **A case is a function of what it gives**: every automatic figure's
    /// seed — the number a derived box shows until it is solved — set to
    /// ±1e-9, ±1 and ±1e3 changes nothing a case comes to, on every preset
    /// alone, stalled and back-driven.
    #[test]
    fn a_case_does_not_read_its_seeds() {
        for (name, t) in still_cases() {
            let want = cases_of(&t);
            for seed in [1e-9, -1e-9, 1.0, -1.0, 1e3, -1e3] {
                let mut u = t.clone();
                for c in &mut u.load_cases {
                    for l in &mut c.loads {
                        if l.torque.auto {
                            l.torque.manual = seed;
                        }
                        if l.speed.auto {
                            l.speed.manual = seed;
                        }
                    }
                }
                assert_eq!(cases_of(&u), want, "{name}: seeds at {seed}");
            }
        }
    }

    /// **Relief's seeding is idempotent**: a case relieved — its derived
    /// figures seeded from what it came to — comes to what it came to.
    #[test]
    fn relief_seeds_nothing_a_case_reads() {
        for (name, t) in still_cases() {
            let want = cases_of(&t);
            for c in 0..t.load_cases.len() {
                let mut u = t.clone();
                u.relieve_case(c, None, &test_library()).unwrap();
                assert_eq!(cases_of(&u), want, "{name}: case {c} relieved");
            }
        }
    }

    /// **A case is the same at any speed**: every given speed scaled by
    /// 1e-6 up to 1e3 moves no torque — on every preset's
    /// cases, and on a planetary set with its ring released, driven at the
    /// sun with the ring held still by a derived torque.
    #[test]
    fn a_case_is_the_same_at_any_speed() {
        let mut trains = still_cases();
        let mut set = Train::alone(&Preset::Planetary.build(), 2.0, 3000.0);
        let (sun, carrier) = (set.load_cases[0].loads[0].at, set.load_cases[0].loads[1].at);
        let ring = set.held[0];
        set.release(ring);
        set.load_cases = vec![LoadCase {
            loads: vec![
                Load::given(sun, 2.0, 3000.0),
                Load {
                    torque: Auto::automatic(0.0),
                    ..Load::given(ring, 0.0, 0.0)
                },
                Load::declared(carrier, LoadRole::Reacted),
            ],
            ..LoadCase::ultimate(sun, carrier, 2.0, 3000.0)
        }];
        trains.push(("planetary, ring still".into(), set));
        let torques = |t: &Train| -> Vec<Vec<(usize, u64)>> {
            solve_train(t, &test_library())
                .unwrap()
                .cases
                .iter()
                .map(|c| {
                    c.bodies
                        .iter()
                        .map(|b| (b.at, b.torque.to_bits()))
                        .collect()
                })
                .collect()
        };
        for (name, t) in trains {
            let want = torques(&t);
            for scale in [1e-6, 1e-4, 1e-3, 1e3] {
                let mut u = t.clone();
                for c in &mut u.load_cases {
                    for l in &mut c.loads {
                        if !l.speed.auto {
                            l.speed.manual *= scale;
                        }
                    }
                }
                assert_eq!(torques(&u), want, "{name}: speeds scaled by {scale}");
            }
        }
    }

    /// **A stall comes to its closed form**: the input's torque given, the
    /// output held still, the output carries `T_in · |i| · η_forward` — the
    /// mesh driven forward, as the given torque drives it — on a pair and
    /// on a worm, whatever the held output's derived torque was seeded at.
    /// A regression guard, not a proof: it passes at the base too, where
    /// the seed's sign happened to agree with the push on both fixtures.
    #[test]
    fn a_stall_comes_to_its_closed_form() {
        for ((name, t), seed) in still_cases()
            .into_iter()
            .filter(|(n, _)| n == "Spur" || n == "Worm")
            .flat_map(|x| [(x.clone(), 1.0), (x, -1.0)])
        {
            let mut t = t;
            t.load_cases[1].loads[1].torque.manual = seed;
            let r = solve_train(&t, &test_library()).unwrap();
            let stall = &r.cases[1];
            assert!(stall.solved, "{name}: {:?}", stall.notes);
            let output = t.load_cases[1].loads[1].at;
            let eta = r.meshes[0].efficiency.forward;
            let ratio = r.paths[0].ratio.abs();
            let want = 2.0 * ratio * eta;
            let got = stall.shaft(output).unwrap().torque.abs();
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{name} seeded {seed}: {got} against {want}"
            );
        }
    }

    /// **A train whose flow is wider than the mask says so on its case**:
    /// 32 pairs in a chain, each case refused by the flow's own key, never
    /// as a load nothing holds or a load held at both ends.
    #[test]
    fn a_flow_past_its_mask_is_said_on_the_case() {
        let stages: Vec<Shape> = (0..32)
            .map(|k| arr::pair(if k % 2 == 0 { [17, 19] } else { [19, 17] }))
            .collect();
        let t = Train::chained(stages, |t| {
            vec![
                LoadCase::ultimate(t.port(0, 1), t.port(31, 2), 2.0, 3000.0),
                LoadCase::back_driving(t.port(0, 1), t.port(31, 2), 2.0),
            ]
        });
        let r = solve_train(&t, &test_library()).unwrap();
        for c in &r.cases {
            assert!(!c.solved, "case {}", c.case);
            let keys: Vec<&str> = c.notes.iter().map(|n| n.key.as_str()).collect();
            assert_eq!(
                keys,
                vec![key::TRAIN_FLOW_TOO_MANY_MESHES],
                "case {}",
                c.case
            );
        }
    }
}

/// **A part's member by the graph's index, where the train has it**: every
/// member of every part of a chain, and nothing past the end of a part or
/// of the parts — an index the train lacks is `None`, not a panic.
#[cfg(test)]
#[test]
fn a_parts_member_is_the_graphs_where_it_has_one() {
    use arrangements::Preset;
    let t = Train::chained(
        vec![Preset::Spur.build(), Preset::Planetary.build()],
        |_| Vec::new(),
    );
    let parts = t.parts();
    let mut found = 0;
    for (k, part) in parts.iter().enumerate() {
        for (j, &member) in part.members.iter().enumerate() {
            assert_eq!(t.member(k, j), Some(member));
            found += 1;
        }
        assert_eq!(t.member(k, part.members.len()), None);
    }
    assert_eq!(
        found,
        2 + 3,
        "a pair's two gears and a set's sun, planet and ring"
    );
    assert_eq!(t.member(parts.len(), 0), None);
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod a_path_is_what_it_crosses {
    //! **A path across a stage is that stage's, wherever the stage is.**
    //!
    //! A stage reported its own ratio, efficiency, backlash, power through
    //! its teeth and one more tooth, from a second motion solved under its
    //! own convention. They went once a law had held them to the path across
    //! the stage's ends, alone and in every chain of two. What that law
    //! leaves standing is the path's own half: across a stage in a chain it
    //! is the path across the same stage alone, and a gear off it moves it
    //! only by locking it.

    use super::arrangements::Preset;
    use super::tests::across;
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    /// Stage `k` of a chain, against the same stage asked alone.
    fn check(name: &str, stages: Vec<Shape>, k: usize, lib: &MaterialLibrary) {
        let alone = solve_alone(&Train::alone(&stages[k], 2.0, 3000.0), lib).unwrap();
        let Some(ratio) = alone.ratio else {
            return; // a family: no path on either side to hold
        };
        let train = Train::chained(stages, |_| Vec::new());
        let path = across(&train, k, lib);
        assert!(
            close(path.ratio, ratio),
            "{name}: ratio {} vs {ratio}",
            path.ratio
        );
        let (e, pe) = (alone.efficiency.unwrap(), path.efficiency.both().unwrap());
        assert!(
            close(pe.forward, e.forward) && close(pe.backward, e.backward),
            "{name}: efficiency {pe:?} vs {e:?}"
        );
        assert_eq!(pe.locked(), e.locked(), "{name}: locked");
        let (b, pb) = (alone.backlash.unwrap(), path.backlash);
        assert!(
            close(pb.forward.unwrap().nominal, b.forward.nominal)
                && close(pb.backward.unwrap().nominal, b.backward.nominal),
            "{name}: backlash {pb:?} vs {b:?}"
        );
        let (c, pc) = (alone.circulation.unwrap(), path.circulation.both().unwrap());
        assert!(
            close(pc.forward, c.forward) && close(pc.backward, c.backward),
            "{name}: circulation {pc:?} vs {c:?}"
        );
        let part = &train.parts()[k];
        let mine = alone.ratio_per_tooth.as_ref().unwrap();
        for (i, want) in mine.iter().enumerate() {
            let got = path.per_tooth[part.members[i]];
            assert!(
                match (got, *want) {
                    (Some(a), Some(b)) => close(a, b),
                    (None, None) => true,
                    _ => false,
                },
                "{name}: one more tooth on member {i}: {got:?} vs {want:?}"
            );
        }
        // ...and a gear of another stage leaves this path's ratio where it
        // was — or takes its motion away altogether, which the stage alone
        // cannot see: one more tooth on a Wolfrom's ring can bring its two
        // rings' counts together and lock it, and a stage after it shares
        // the body it locks.
        for (j, got) in path.per_tooth.iter().enumerate() {
            if !part.members.contains(&j) {
                assert!(
                    got.is_none_or(|g| close(g, ratio)),
                    "{name}: gear {j} moved {got:?} from {ratio}"
                );
            }
        }
    }

    #[test]
    fn every_preset_in_every_pair_is_the_path_across_it_alone() {
        let lib = test_library();
        for a in Preset::ALL {
            for b in Preset::ALL {
                let name = format!("{a:?} then {b:?}");
                check(&name, vec![a.build(), b.build()], 0, &lib);
                check(&name, vec![a.build(), b.build()], 1, &lib);
            }
        }
    }

    /// **An offset coupling passes what reaches it and adds nothing**: a
    /// planocentric's path to the shaft its planet is coupled to is the
    /// path to the planet — the ratio, the efficiency both ways, the play at
    /// each end and the power through the teeth, which a coupling has none
    /// of — and the shaft turns with the planet in every case.
    #[test]
    fn a_coupled_shaft_is_the_body_it_turns_with() {
        let lib = test_library();
        let stage = Preset::Planocentric.build();
        assert_eq!(
            stage.couplings,
            vec![[3, 4]],
            "carrier, ring, shaft, planet"
        );
        let to = |output: Body| {
            solve_alone(
                &Train::alone(&stage, 2.0, 3000.0).arranged(&[2], 1, output),
                &lib,
            )
            .unwrap()
        };
        let (shaft, planet) = (to(3), to(4));
        assert!(close(shaft.ratio.unwrap(), planet.ratio.unwrap()));
        let (e, pe) = (shaft.efficiency.unwrap(), planet.efficiency.unwrap());
        assert!(close(e.forward, pe.forward) && close(e.backward, pe.backward));
        let (b, pb) = (shaft.backlash.unwrap(), planet.backlash.unwrap());
        assert!(
            close(b.forward.nominal, pb.forward.nominal)
                && close(b.backward.nominal, pb.backward.nominal)
        );
        let (c, pc) = (shaft.circulation.unwrap(), planet.circulation.unwrap());
        assert!(close(c.forward, pc.forward) && close(c.backward, pc.backward));
        for case in &shaft.cases {
            assert!(close(case.speeds[3], case.speeds[4]), "{case:?}");
        }
    }

    /// **A path holds at rest where no mesh of it does.** The Wolfrom
    /// preset, every mesh sliding at μ = 0.05: at a static μ of 0.05 it
    /// back-drives, well clear of nought; at a static μ of 0.10 its whole
    /// flow cannot break away backwards — nought exactly — while each mesh
    /// alone still back-drives, so the hold is the whole flow's.
    /// `tools/breakaway.py` derives the same with the sign kept.
    #[test]
    fn a_path_holds_at_rest_where_no_mesh_does() {
        const SLIDING: f64 = 0.05;
        let at_rest = |statik: f64| {
            let mut s = super::arrangements::Preset::Wolfrom.build();
            for m in &mut s.meshes {
                m.sliding_friction = SLIDING;
                m.static_friction = statik;
            }
            crate::train::testing::alone(&s)
        };
        let free = at_rest(SLIDING);
        let e = free.efficiency.unwrap();
        assert!(e.backward > 0.1, "at μ_s = μ it back-drives: {e:?}");
        let held = at_rest(2.0 * SLIDING);
        let e = held.efficiency.unwrap();
        assert_eq!(e.backward, 0.0, "the whole flow holds, nought exactly");
        assert!(
            held.meshes.iter().all(|m| m.efficiency.backward > 0.0),
            "no mesh of it locks alone"
        );
    }
}
