//! The worm stage.
//!
//! A stage of a different shape from [`super::spur`], and deliberately not
//! forced into the same result type. A worm stage has no bending stress
//! (DESIGN.md §4.5.1), no meaningful minimum face width from contact — a point
//! contact does not care how wide the tooth is — and two efficiencies rather
//! than one. Bending those facts into [`super::StageResult`] would have meant
//! four `Option`s and a comment apologising for each; a separate result says the
//! same thing by being shaped like the answer.
//!
//! The mesh mathematics is all in [`crate::screw`]. What this module adds is the
//! *stage*: materials, face widths, torque, backlash and the notes a designer
//! needs to see.
//!
//! # Backlash comes out of one projection — and two kinds of displacement
//!
//! Two things open a gap between the flanks: axial slack in the worm, and a
//! centre distance larger than nominal. Both are displacements, and both matter
//! only through their component along the common flank normal `n̂` — the very
//! same `n̂` the path of contact is built on ([`crate::screw::Screw::contact_normal`]),
//! projected rather than re-derived:
//!
//! ```text
//! j_n = j_axial · (â₁·n̂)   +   2 Δa · (x̂·n̂)
//!     = j_axial · sin β_b1  +   2 Δa · sin α_n
//! ```
//!
//! **The two are not the same kind of displacement, and the factor of two is
//! where that shows.** A centre distance above nominal *separates the axes*, and
//! a separation opens **both** flanks by the same amount; lost motion is the sum
//! of the gaps a member can travel across, so it counts twice. The worm's axial
//! float is a *rigid-body slide* along its own axis: it opens one flank exactly
//! as far as it closes the other, so running from one end of the float to the
//! other is lost once. Writing both as "a gap along `n̂`" and giving them one
//! coefficient is what left a factor of two in this model for a milestone
//! (DESIGN §12).
//!
//! `sin α_n` there is an identity, not a small-angle reading: the contact
//! normal's component along the line of centres is `sin α_n` at **every** shaft
//! angle, which `screw.rs` gates and `tools/crossed_path.py` measures off the
//! surfaces.
//!
//! Closing the gap takes a rotation, and that conversion is
//! [`crate::mesh::angular_play`] — the law every mesh in this crate shares. It
//! is the same number as the `j_n / (r cos α_n sin γ)` this module used to
//! write for itself, since `r sin γ = z m_n / 2` exactly; the point of calling
//! the shared one is that a law with two homes is a law with two answers, and
//! the home nothing checks is the one that is wrong.
//!
//! At a right-angle drive the axial half reduces to the two relations the
//! handbooks give separately — the wheel turns by `j_axial/r₂`, and the worm by
//! `2π j_axial/lead` — which is the check the tests make.
//!
//! ## Where this law stops, and where the parallel one takes over
//!
//! A crossed pair's backlash is **exactly linear** in the centre-distance error,
//! because its line of action cannot rotate when the centres move. A parallel
//! pair's is not: at `Σ = 0` the constraint on `n̂` degenerates, the freed
//! rotation *is* the operating pressure angle, and `j_t = 2a′(inv α′ − inv α_w)`
//! carries the difference. The two agree to first order — gated both ways in the
//! tests below — and part company by 0.21 % at the default clearance. That step
//! at `Σ = 0` is the degeneracy, not a seam in the code.

use super::{solve_stage, Backlash, TrainError};

/// Quadrature points for the path-averaged friction balance.
///
/// Not a tuning parameter: the trapezium rule is second order and a test
/// asserts that the error quarters as the step halves, which fixes what any
/// count is worth. At this one the residual is below 1e-9 in efficiency —
/// four orders under the last digit anything reports.
const PATH_SAMPLES: usize = 2048;
use crate::contact::{Directional, Drive};
use crate::material::{contact_modulus, Material, MaterialLibrary, Overrides};
use crate::mesh::Member;
use crate::params::Auto;
use crate::screw::{CrossedPath, Screw, ScrewParams, ZoneLimit};

/// The proportions a worm drive is conventionally given.
///
/// **These are conventions, not derivations, and they are shipped deliberately.**
/// §4.7's standing policy refuses published *rating* factors, and the reason is
/// specific: a correction factor multiplies a stress, so shipping one outside
/// its validated band silently moves a number a designer will size a part
/// against. These are a different kind of thing. A worm's length and a wheel's
/// face width **enter no stress in this crate** — the contact is a point, and
/// the appendix records the measurement: the same mesh at 4 mm and at 40 mm of
/// face width gives a bit-identical peak pressure. So a recommendation here
/// informs a choice and cannot distort an answer, which is what makes it
/// admissible where a `K_v` is not.
///
/// They are reported *as* recommendations, with the source named on screen, and
/// the input stays editable. See DESIGN.md §4.5.1.
///
/// # What they are for
///
/// A real worm drive has an enveloping wheel that wraps the worm, and both
/// dimensions are about **covering the zone of action**: the worm must be long
/// enough for the wheel to run off neither end, and the wheel wide enough to
/// take the thread but not so wide that its outer corners hang past where the
/// worm can touch. This crate models the pair as crossed-axis screw gearing
/// with point contact and does not derive that zone (§4.5.1, open), so the
/// proportions come from published practice rather than from our own geometry —
/// which is exactly why each carries its source.
pub mod proportions {
    /// Recommended minimum worm length (thread length), mm.
    ///
    /// ```text
    /// b₁ = (11 + c z₂) m_x,     c = 0.06 for z₁ < 4, 0.09 for z₁ ≥ 4
    /// ```
    ///
    /// DIN/ČSN practice, as tabulated by MITcalc's worm-gear geometry
    /// documentation. It is a **function of the wheel's tooth count**, which is
    /// what the specification asks for: a bigger wheel wraps further round the
    /// worm, so the worm must be longer to carry the contact. More starts steepen
    /// the lead and stretch the same wrap over more axial length, which is the
    /// step in `c`.
    ///
    /// Takes the **axial** module, because that is the module these proportions
    /// are written in and the one the worm's own pitch is measured in.
    #[must_use]
    pub fn worm_length(axial_module: f64, wheel_teeth: u32, starts: u32) -> f64 {
        let c = if starts < 4 { 0.06 } else { 0.09 };
        (11.0 + c * f64::from(wheel_teeth)) * axial_module
    }

    /// Recommended wheel face width, mm — BS 721.
    ///
    /// ```text
    /// b₂ = 2 m_x √(q + 1),   capped at 0.67 d₁,   q = d₁ / m_x
    /// ```
    ///
    /// Two statements from the same source, and the cap is the operative one on
    /// a slender worm: past about two thirds of the worm's reference diameter
    /// the wheel's outer corners are beyond the thread they were widened to
    /// catch, so the extra face carries nothing. `q`, the diameter quotient, is
    /// how worm practice expresses the worm's slenderness.
    #[must_use]
    pub fn wheel_face_width(axial_module: f64, worm_pitch_diameter: f64) -> f64 {
        let q = worm_pitch_diameter / axial_module;
        let recommended = 2.0 * axial_module * (q + 1.0).sqrt();
        recommended.min(0.67 * worm_pitch_diameter)
    }
}

/// One member of a worm stage.
///
/// Note what is *absent* against [`super::StageGear`]: no profile shift and no
/// addendum. Both belong to a generated involute profile that a worm stage does
/// not yet build.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WormMember {
    /// Face width of the wheel, or length of the worm, mm.
    ///
    /// Automatic takes the recommended proportion for this member — see
    /// [`proportions`], and note what those are and are not. Unlike the spur
    /// stage's automatic face width, this one is **not** a minimum derived from
    /// a rating: nothing here is sized against a stress, because a point
    /// contact's peak pressure does not depend on the face width at all.
    pub face_width: Auto<f64>,
    /// Name of a material in the library.
    pub material: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub material_overrides: Overrides,
}

impl Default for WormMember {
    fn default() -> Self {
        Self {
            face_width: Auto::automatic(10.0),
            material: "4340 Hardened Steel".to_string(),
            material_overrides: Overrides::default(),
        }
    }
}

/// A crossed-axis worm stage.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WormStage {
    /// Normal module, mm. Shared.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// Shaft angle, degrees. 90 is the ordinary drive.
    pub shaft_angle: f64,
    /// Coefficient of friction for the mesh. **Not** optional in the way it is
    /// for a spur stage: a worm's whole character comes from it.
    pub friction: f64,
    /// Starts on the worm.
    pub starts: u32,
    /// How the first member's size is fixed — the *only* thing that
    /// distinguishes a worm drive from a crossed gear pair.
    pub sizing: FirstMemberSizing,
    /// Teeth on the wheel.
    pub wheel_teeth: u32,
    /// Automatic uses the geometry's own centre distance plus `clearance`.
    pub centre_distance: Auto<f64>,
    /// Added to the centre distance, mm. Forced to zero when the centre
    /// distance is set by hand, as in a spur stage.
    pub clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// Axial play of the worm, mm. The dominant source of backlash in a worm
    /// drive, and one a spur stage has no equivalent of.
    pub axial_clearance: f64,
    pub worm: WormMember,
    pub wheel: WormMember,
}

impl Default for WormStage {
    fn default() -> Self {
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            shaft_angle: 90.0,
            friction: 0.06,
            starts: 1,
            sizing: FirstMemberSizing::PitchDiameter(7.0),
            wheel_teeth: 40,
            centre_distance: Auto::automatic(0.0),
            clearance: 0.02,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            axial_clearance: 0.04,
            worm: WormMember::default(),
            wheel: WormMember {
                material: "Brass C360".to_string(),
                ..WormMember::default()
            },
        }
    }
}

/// What a worm stage does to one of its members.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WormMemberResult {
    /// Torque on this member, N·m.
    pub torque: f64,
    /// Rotational speed, rpm. Filled in by the train.
    pub speed: f64,
    /// Tooth load cycles over the duty. Filled in by the train.
    pub tooth_cycles: f64,
    /// The width in use, mm — the recommendation when automatic, the input
    /// otherwise. For the worm this is a *length* along its axis.
    pub face_width: f64,
    /// What the conventional proportion asks for, mm, whether or not it is what
    /// is in use. Reported always, so a hand-set width can be read against it.
    ///
    /// A **convention with a named source**, not a derivation, and it sizes no
    /// stress in this crate — see [`proportions`].
    ///
    /// `None` for a crossed gear pair: the proportions describe a worm carrying
    /// an enveloping wheel, and there is neither.
    pub recommended_face_width: Option<f64>,
    pub pitch_diameter: f64,
    /// The material as used, after any overrides.
    pub material: Material,
}

/// What the path of contact says about a crossed-axis mesh.
///
/// Reported for a worm drive as well as for a crossed gear pair, because both
/// are the same construction here: §4.5.1 takes **both flanks as involute
/// helicoids on cylinders**, and that is where the stage's contact stress,
/// efficiency and backlash already come from. See [`crossed_mesh`] for why a
/// real worm's throated wheel makes this a floor rather than a wrong number.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CrossedMesh {
    /// Tooth pairs in contact. **Below 1 the drive loses contact between one
    /// pair and the next**, which is a failure of kind rather than of margin.
    pub contact_ratio: f64,
    /// What ended the zone: the teeth, or the face they are cut on.
    pub limited_by: ZoneLimit,
    /// The face width at which `ε = 1`, per member, mm.
    ///
    /// A **geometric** minimum: it keeps contact continuous and says nothing
    /// about stress. That is the opposite of the spur stage's automatic width,
    /// which inverts a stress, and the difference has to travel with the number.
    pub face_width_for_continuity: Option<[f64; 2]>,
    /// How far the contact point runs along each member's own axis, mm — what a
    /// face has to cover, and what a parallel pair does not have at all.
    pub axial_travel: [f64; 2],
    /// True when the tooth height was **assumed** rather than given — a worm
    /// stage has no addendum input. The figures above are then a lower bound.
    pub tooth_height_assumed: bool,
    /// What the same teeth would lose with their shafts brought **parallel**, as
    /// an efficiency — `None` for a worm drive, whose single-start thread is not
    /// a parallel-axis gear.
    ///
    /// Reported for comparison: crossing shafts adds sliding, so this is the
    /// best the pair can be, and how far the crossed figure falls below it is
    /// what the shaft angle costs.
    ///
    /// It was once a *check* — the crossed model omitted the profile sliding, so
    /// beating this figure meant sliding had gone missing (§12). The friction
    /// balance counts that sliding now, and the check has become worse than
    /// useless: it is the **parallel** closed form that is approximate, first
    /// order in `μ` where the balance is exact, so at a very small shaft angle
    /// the crossed figure legitimately sits a hundredth of a point above and the
    /// old test would accuse the better number.
    pub parallel_axis_efficiency: Option<f64>,
}

/// The contact patch a worm mesh presses.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WormContact {
    /// Peak Hertzian pressure, MPa — **the** strength figure for this stage.
    ///
    /// The worst of the points a rating is taken at: the pitch point, and the
    /// two boundaries of single-pair contact where one tooth carries the whole
    /// load. Rated at the pitch point *alone* this came out 0.7–2.2 % low.
    ///
    /// The relative radius peaks where the two roll lengths are equal and falls
    /// away toward both ends of the zone, and the pitch point sits near that
    /// peak — so it is close to the *least* severe place the mesh offers, not
    /// the worst.
    ///
    /// The **ends** of the zone are 10–27 % worse than the pitch point, and the
    /// rating deliberately does not use them: with `ε > 1` a second pair is in
    /// contact there and the load is shared. Losing that sharing does push the
    /// rating outward — but a face narrow enough to lose it has also cut the
    /// zone's ends off, which pulls it back in, so the two effects nearly
    /// cancel and the net is about a further 0.5 %.
    pub max_pressure: f64,
    /// What the pitch point alone would have said, MPa — the figure this
    /// replaced, kept so the difference is visible rather than asserted.
    pub at_pitch_point: f64,
    /// Where the worst point sits on the path, mm from the pitch point.
    pub worst_position: f64,
    /// The patch, mm. An ellipse, not a line.
    pub patch_length: f64,
    pub patch_width: f64,
    /// Relative curvature along the contact, 1/mm. Zero would mean line
    /// contact; that it is not zero is what crossing the shafts did.
    pub curvature_along: f64,
    /// ...and across it.
    pub curvature_across: f64,
}

/// Everything a worm stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WormResult {
    /// `z₂ / z₁` — for a worm, usually large.
    pub ratio: f64,
    pub centre_distance_nominal: f64,
    pub centre_distance: f64,
    /// Lead angle of the worm, degrees.
    pub lead_angle: f64,
    /// Lead angle of the wheel, degrees.
    pub wheel_lead_angle: f64,
    /// Helix angle of the first member, degrees — `90° − γ₁`.
    ///
    /// The same fact as the lead angle, from the other datum: a worm is
    /// described by how far its thread advances, a gear by how far its tooth
    /// leans. Both are reported because a crossed *gear* pair is entered by
    /// helix angle and read that way, and converting between them on the far
    /// side of the boundary is arithmetic.
    pub helix_angle: f64,
    /// Helix angle of the wheel, degrees — an output, per the specification.
    pub wheel_helix_angle: f64,
    /// Lead, mm, and axial module, mm.
    pub lead: f64,
    pub axial_module: f64,
    /// The path of contact, for a crossed **gear** pair. `None` for a worm
    /// drive: see [`CrossedMesh`].
    pub crossed: Option<CrossedMesh>,
    /// Mesh efficiency in both drive directions.
    ///
    /// Unlike a parallel-axis stage these genuinely differ, and the backward one
    /// can be zero or negative — that is what self-locking is, and
    /// [`Directional::self_locking`] reads it rather than a separate flag that
    /// could disagree.
    pub efficiency: Directional<f64>,
    /// The coefficient of friction at which self-locking begins.
    pub self_locking_friction: f64,
    /// Sliding speed at the pitch point as a multiple of the worm's pitch line
    /// speed. The absolute figure needs a shaft speed, so the train fills
    /// [`Self::sliding_velocity`] instead.
    pub sliding_ratio: f64,
    /// Sliding speed at the pitch point, mm/s. Filled in by the train.
    pub sliding_velocity: f64,
    pub contact: WormContact,
    /// Angular backlash at whichever member is the output in each direction,
    /// degrees: the wheel driving forward, the worm driving backward. A worm
    /// stage shows the gap the two ways round more starkly than any other,
    /// because the ratio between the lever arms *is* the gear ratio.
    pub backlash: Directional<Backlash>,
    pub members: [WormMemberResult; 2],
    pub notes: Vec<String>,
}

/// How the first member's size is fixed.
///
/// **This is the whole of the difference between a worm drive and a crossed gear
/// pair**, and it is worth being explicit about because the mathematics is
/// otherwise identical — §4.5.1 argued they are one thing, and this is where that
/// argument is cashed.
///
/// A worm's pitch diameter is a *free choice*: nothing in its thread count fixes
/// it, and it is what sets the lead angle, the efficiency, and whether the drive
/// can be back-driven at all. An ordinary gear's is not free — it follows from
/// its tooth count and helix angle, `d = z m_n / cos β`. So the two differ in
/// which of `d` and `β` is the input, and in nothing else:
///
/// ```text
/// sin γ = z m_n / d        and      γ = 90° − β        so      sin γ = cos β
/// ```
///
/// [verified: a `Screw` built with `d₁ = z₁ m_n / cos β₁` reports
/// `γ₁ = 90° − β₁` exactly, and `β₂ = Σ − β₁`, over three tooth pairs × four
/// shaft angles × three helix angles.]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum FirstMemberSizing {
    /// A **worm**: the pitch diameter is given, and the lead angle follows.
    PitchDiameter(f64),
    /// A **gear**: the helix angle is given in degrees, and the pitch diameter
    /// follows. `β = 0` makes the first member an ordinary spur gear crossed with
    /// a helical one, which is what "crossed-axis spur" means.
    HelixAngle(f64),
}

impl WormStage {
    /// The first member's pitch diameter, mm — given, or derived from its helix
    /// angle.
    #[must_use]
    pub fn first_pitch_diameter(&self) -> f64 {
        match self.sizing {
            FirstMemberSizing::PitchDiameter(d) => d,
            FirstMemberSizing::HelixAngle(beta_deg) => {
                f64::from(self.starts.max(1)) * self.module / beta_deg.to_radians().cos()
            }
        }
    }

    /// The screw geometry this stage describes.
    ///
    /// # Errors
    ///
    /// [`TrainError::Screw`] if the pair cannot exist.
    pub fn geometry(&self) -> Result<Screw, TrainError> {
        // Caught here rather than in `Screw::new`, because by then the helix
        // angle has become a diameter and the information is gone: `cos 90°` is
        // 6e-17, not zero, so the diameter comes out enormous rather than
        // infinite and passes every finiteness check downstream.
        if let FirstMemberSizing::HelixAngle(beta) = self.sizing {
            if beta.abs() >= 90.0 {
                return Err(TrainError::Screw(
                    crate::screw::ScrewError::FirstMemberIsADisc,
                ));
            }
        }
        Screw::new(&ScrewParams {
            normal_module: self.module,
            normal_pressure_angle: self.pressure_angle.to_radians(),
            shaft_angle: self.shaft_angle.to_radians(),
            starts: self.starts,
            wheel_teeth: self.wheel_teeth,
            worm_pitch_diameter: self.first_pitch_diameter(),
        })
        .map_err(TrainError::Screw)
    }
}

/// Solve a **crossed gear pair** — a [`super::SpurStage`] whose shafts are not
/// parallel.
///
/// It is the same mesh as a worm drive and is solved by the same code, because
/// it *is* the same thing: crossed-axis screw gearing. The only difference is
/// which of the first member's diameter and helix angle is the input (§4.5.1),
/// and a gear's diameter follows from its teeth, so the helix angle is what is
/// given. That is the whole translation below.
///
/// Two things a crossed pair does not inherit from its parallel form, both
/// because the contact is a point:
///
/// - **No automatic face width.** A point contact's peak pressure does not
///   depend on the face width, and there is no bending model for a crossed
///   pair, so nothing can size it. A width left automatic is used as entered
///   and the result says so.
/// - **No axial clearance.** That is a worm's float along its own axis; a gear
///   pair has none, so the backlash comes from the centre distance alone.
///
/// # Errors
///
/// [`TrainError`] if the pair cannot exist or names a material the library does
/// not have.
pub fn solve_crossed_stage(
    stage: &super::SpurStage,
    input_torque: f64,
    lib: &MaterialLibrary,
) -> Result<WormResult, TrainError> {
    let member = |g: &super::StageGear| WormMember {
        // Fixed, not automatic: see above. `manual` is what the field holds
        // whichever way its toggle is set, so this is the number on screen.
        face_width: Auto::fixed(g.face_width.manual),
        material: g.material.clone(),
        material_overrides: g.material_overrides,
    };

    let mut equivalent = WormStage {
        module: stage.module,
        pressure_angle: stage.pressure_angle,
        shaft_angle: stage.shaft_angle,
        friction: stage.friction,
        starts: stage.gears[0].teeth,
        sizing: FirstMemberSizing::HelixAngle(stage.helix_angles()[0]),
        wheel_teeth: stage.gears[1].teeth,
        centre_distance: stage.centre_distance,
        clearance: stage.clearance,
        tolerance_plus: stage.tolerance_plus,
        tolerance_minus: stage.tolerance_minus,
        axial_clearance: 0.0,
        worm: member(&stage.gears[0]),
        wheel: member(&stage.gears[1]),
    };

    // --- the path of contact, which is what a crossed pair can now say.
    //
    // Its tips come from the tooth form the stage carries: this is the one place
    // that form reaches an answer, which is why it is specified at all (§4.5.1).
    let screw = equivalent.geometry()?;
    let tips = {
        let r = [
            screw.worm_pitch_diameter / 2.0,
            screw.wheel_pitch_diameter / 2.0,
        ];
        [
            r[0] + stage.gears[0].addendum.manual * stage.module,
            r[1] + stage.gears[1].addendum.manual * stage.module,
        ]
    };
    // At the centre distance the pair will run at, not the zero-backlash one:
    // an automatic face is sized for the machine that gets built. On a crossed
    // pair that is not a refinement — the contact slides bodily along the shafts
    // with a centre-distance error, so a face centred on the nominal contact can
    // miss it entirely (§4.4).
    let path = screw.path_of_contact_at(tips[0], tips[1], operating_centre(&screw, &equivalent));

    // **Automatic face width means continuity here, not strength.** The spur
    // stage inverts a stress to size a face; a crossed pair has no stress that
    // depends on its width at all, and what it does have is a contact point that
    // runs off the end of a face too narrow. So automatic takes the width at
    // which one tooth pair hands over to the next exactly — `ε = 1` — and the
    // result says which kind of minimum it is wherever it shows the number.
    let continuity = path.as_ref().and_then(|p| p.face_widths_for(&screw, 1.0));
    let mut notes = Vec::new();
    for (i, gear) in stage.gears.iter().enumerate() {
        if !gear.face_width.auto {
            continue;
        }
        match continuity {
            Some(widths) => {
                let m = if i == 0 {
                    &mut equivalent.worm
                } else {
                    &mut equivalent.wheel
                };
                m.face_width = Auto::fixed(widths[i]);
            }
            None => notes.push(format!(
                "gear {}'s face width used as entered: its teeth do not reach a full \
                 contact ratio at any width, so there is no width that would keep \
                 contact continuous",
                i + 1
            )),
        }
    }

    let mut result = solve_worm_stage(&equivalent, input_torque, lib)?;
    result.notes.extend(notes);

    // ...and the zone as the widths in use actually leave it.
    // The same teeth with their shafts brought parallel — which the stage can
    // describe exactly, since a crossed pair *is* this stage at `Σ = 0`. It is
    // here to be compared against, not to correct with: see
    // `CrossedMesh::omits_profile_sliding`.
    let parallel = solve_stage(
        &super::SpurStage {
            shaft_angle: 0.0,
            ..stage.clone()
        },
        input_torque,
        lib,
    )
    .ok()
    .map(|r| r.efficiency.forward);

    // ...and the zone as the widths in use actually leave it. The tips are known
    // here — a crossed pair carries its tooth form — so nothing is assumed.
    result.crossed = crossed_mesh(
        &screw,
        &result.members,
        Some(tips),
        parallel,
        result.centre_distance,
    );

    if let Some(zone) = result.crossed {
        if zone.contact_ratio < 1.0 {
            result.notes.push(format!(
                "contact ratio {:.3}: below 1 the pair loses contact between one tooth \
                 and the next, whatever the stresses say. A crossed pair's face width \
                 bounds this where a parallel pair's does not — see the width for \
                 continuity beside it",
                zone.contact_ratio
            ));
        }
    }
    Ok(result)
}

/// Solve one worm stage, given the torque on the worm.
///
/// # Errors
///
/// [`TrainError`] if the pair cannot exist or names a material the library does
/// not have.
pub fn solve_worm_stage(
    stage: &WormStage,
    input_torque: f64,
    lib: &MaterialLibrary,
) -> Result<WormResult, TrainError> {
    let s = stage.geometry()?;

    let materials: Vec<Material> = [&stage.worm, &stage.wheel]
        .iter()
        .map(|m| {
            lib.get(&m.material)
                .ok_or_else(|| TrainError::UnknownMaterial(m.material.clone()))
                .map(|found| found.overridden(&m.material_overrides))
        })
        .collect::<Result<_, _>>()?;

    let centre = operating_centre(&s, stage);

    // The face widths, decided before anything that reads them — the path is
    // one of those things now, since a face narrow enough to cut the zone
    // changes what the mesh does as well as how long it does it for.
    let recommended = match stage.sizing {
        FirstMemberSizing::PitchDiameter(_) => [
            Some(proportions::worm_length(
                s.axial_module,
                stage.wheel_teeth,
                stage.starts,
            )),
            Some(proportions::wheel_face_width(
                s.axial_module,
                s.worm_pitch_diameter,
            )),
        ],
        FirstMemberSizing::HelixAngle(_) => [None, None],
    };
    let widths = [
        recommended[0].map_or(stage.worm.face_width.manual, |r| {
            stage.worm.face_width.resolve(r)
        }),
        recommended[1].map_or(stage.wheel.face_width.manual, |r| {
            stage.wheel.face_width.resolve(r)
        }),
    ];

    // **Efficiency from the friction balance along the path, not at the pitch
    // point.** The pitch point is the one place with no sliding up the profile,
    // so a figure taken only there counts the sliding along the trace and
    // nothing else — which is why it tended to 100 % as the shafts came parallel
    // while the real pair keeps its profile loss (§4.5.1). The balance contains
    // the classical formula exactly: at the pitch point the two agree to 1e-12,
    // including the friction at which back-driving stops.
    //
    // Every figure this moves, it moves **down**: more sliding is counted, and
    // sliding cannot repay. See the canary note in DESIGN §4.5.1.
    let path = rating_path(&s, widths, None, centre);
    let efficiency = Directional::of(|d| {
        path.and_then(|p| p.efficiency(&s, stage.friction, d, PATH_SAMPLES))
            .unwrap_or_else(|| s.efficiency(stage.friction, d))
    });
    // Quoted beside that efficiency, so it has to be the friction at which *it*
    // reaches zero rather than the pitch point's.
    let threshold = path
        .and_then(|p| p.self_locking_friction(&s, PATH_SAMPLES))
        .unwrap_or_else(|| s.self_locking_friction());
    let output_torque = input_torque * s.ratio * efficiency.forward;

    // Contact is rated on the wheel's torque: which torque is held fixed decides
    // which way friction moves the flank load, and only this direction is the
    // conservative one. See `Screw::normal_force`.
    let e_star = contact_modulus(&materials[0], &materials[1]);
    let (curvature_along, curvature_across) =
        s.contact_curvatures().ok_or(TrainError::NoContact)?;
    let at_pitch = s
        .contact(output_torque, Member::Second, stage.friction, e_star)
        .ok_or(TrainError::NoContact)?;
    // **Rated along the path, not at the pitch point.** The relative radius
    // peaks where the two roll lengths are equal and falls toward both ends of
    // the zone; the pitch point sits near that peak, so rating there alone took
    // the mesh at close to its gentlest and flattered it.
    // The points that matter are the two boundaries of single-pair contact,
    // where one tooth carries everything; beyond them the next pair has taken
    // up. Where `ε ≤ 1` those boundaries are the ends of the zone, so a face
    // too narrow raises this figure as well as costing continuity.
    let mut patch = at_pitch;
    let mut worst_position = 0.0;
    if let Some(path) = path {
        for position in path.single_pair_bounds(&s) {
            let contact = path.contact_at(&s, position);
            let Some((along, across)) = path.curvatures_at(&s, position) else {
                continue;
            };
            // The force at *this* contact, not at the pitch point: a stress
            // evaluated in one place with a load computed in another is two
            // answers wearing one number.
            let Some(force) = contact.normal_force(output_torque, stage.friction, Drive::Forward)
            else {
                continue;
            };
            let Some(here) = crate::hertz::elliptical_contact(along, across, force, e_star) else {
                continue;
            };
            if here.max_pressure > patch.max_pressure {
                patch = here;
                worst_position = position;
            }
        }
    }

    let backlash = Directional::of(|d| {
        let at = match d {
            Drive::Forward => Member::Second,
            Drive::Backward => Member::First,
        };
        Backlash {
            nominal: angular_backlash(&s, stage, centre - s.centre_distance, at).to_degrees(),
            minimum: angular_backlash(
                &s,
                stage,
                centre - stage.tolerance_minus - s.centre_distance,
                at,
            )
            .to_degrees(),
            maximum: angular_backlash(
                &s,
                stage,
                centre + stage.tolerance_plus - s.centre_distance,
                at,
            )
            .to_degrees(),
        }
    });

    let mut notes = Vec::new();
    if efficiency.self_locking() {
        notes.push(format!(
            "self-locking at mu = {:.3}: the wheel cannot back-drive the worm \
             (the threshold is {threshold:.4})",
            stage.friction
        ));
    } else if stage.friction > 0.8 * threshold {
        notes.push(format!(
            "close to self-locking: mu = {:.3} against a threshold of {threshold:.4}, \
             so back-driving depends on a friction coefficient nobody measured",
            stage.friction
        ));
    }
    if efficiency.forward < 0.5 {
        notes.push(format!(
            "mesh efficiency {:.1} % — most of the input becomes heat, and a \
             worm drive is usually limited by that rather than by stress",
            efficiency.forward * 100.0
        ));
    }

    // The two conventional proportions, in the axial module they are written
    // in. They size the *part*, not the answer: nothing below reads a face
    // width, which is what makes shipping a convention here honest (see
    // `proportions`).
    //
    // **Only for a worm drive.** These describe a worm carrying an enveloping
    // wheel, and a crossed gear pair has neither — its members are two helical
    // gears touching at a point, with nothing wrapped round anything. §4.5.1
    // makes the first member's sizing the definition of which machine this is,
    // so that is what decides here. Offering the numbers anyway would be
    // shipping a convention outside the case it was written for, which is the
    // thing §4.7's policy exists to refuse.
    if recommended[0].is_none() && (stage.worm.face_width.auto || stage.wheel.face_width.auto) {
        notes.push(
            "face widths left as entered: the published proportions are for a worm \
             carrying an enveloping wheel, and a crossed gear pair has neither"
                .into(),
        );
    }

    let members = [
        WormMemberResult {
            torque: input_torque,
            speed: 0.0,
            tooth_cycles: 0.0,
            face_width: widths[0],
            recommended_face_width: recommended[0],
            pitch_diameter: s.worm_pitch_diameter,
            material: materials[0].clone(),
        },
        WormMemberResult {
            torque: output_torque,
            speed: 0.0,
            tooth_cycles: 0.0,
            face_width: widths[1],
            recommended_face_width: recommended[1],
            pitch_diameter: s.wheel_pitch_diameter,
            material: materials[1].clone(),
        },
    ];

    Ok(WormResult {
        ratio: s.ratio,
        centre_distance_nominal: s.centre_distance,
        centre_distance: centre,
        lead_angle: s.lead_angle.to_degrees(),
        wheel_lead_angle: s.wheel_lead_angle.to_degrees(),
        helix_angle: s.worm_helix_angle.to_degrees(),
        wheel_helix_angle: s.wheel_helix_angle.to_degrees(),
        lead: s.lead,
        axial_module: s.axial_module,
        crossed: crossed_mesh(&s, &members, None, None, centre),
        efficiency,
        self_locking_friction: threshold,
        sliding_ratio: s.sliding_ratio,
        sliding_velocity: 0.0,
        contact: WormContact {
            max_pressure: patch.max_pressure,
            at_pitch_point: at_pitch.max_pressure,
            worst_position,
            patch_length: 2.0 * patch.semi_major(),
            patch_width: 2.0 * patch.semi_minor(),
            curvature_along,
            curvature_across,
        },
        backlash,
        members,
        notes,
    })
}

/// Angular backlash at one member, radians, for a centre distance `delta` above
/// nominal.
///
/// The module documentation derives this; the short of it is that the axial
/// slack and the centre-distance change are two displacements, and only their
/// components along the common flank normal open a gap.
/// The path of contact, as far as the geometry in hand allows it to be known.
///
/// `tips` are the two tip radii when the stage carries a tooth height — a
/// crossed gear pair does. A **worm stage does not**: it has no addendum input,
/// so a standard one-normal-module tooth is assumed, and that assumption is what
/// makes the result a *floor* rather than a figure.
///
/// # Why a worm gets this at all
///
/// Every other number a worm stage reports — its contact stress, its efficiency,
/// its backlash — comes from a model in which **both flanks are involute
/// helicoids on cylinders** (§4.5.1). Withholding the one number that comes from
/// the same construction, on the grounds that a real worm's wheel is throated,
/// would be applying a standard to it that nothing else here is held to.
///
/// What throating does is *wrap* the wheel around the worm, engaging more of the
/// thread. That can only lengthen the zone of action, so the cylindrical figure
/// is a lower bound on an enveloping one: a worm that clears `ε = 1` here clears
/// it as built. That is an argument about the direction, not a computation, and
/// it is said next to the number.
/// The path of contact as the face widths in use leave it — what a rating walks.
///
/// Shares its assumptions with [`crossed_mesh`], which reports the same zone:
/// one construction, asked twice for different things, rather than two that can
/// disagree about where the teeth touch.
/// The centre distance the pair actually runs at.
///
/// Nominal plus the assembly clearance when the centre distance is automatic,
/// and the number as entered when it is not — the specification's rule, and the
/// one both the rating and the backlash have to agree on. One home for it
/// because a stage asks twice: once inside the solve, and once before it, to
/// size an automatic face for the pair **as built** rather than for the
/// zero-backlash one nobody assembles.
fn operating_centre(s: &Screw, stage: &WormStage) -> f64 {
    if stage.centre_distance.auto {
        s.centre_distance + stage.clearance
    } else {
        stage.centre_distance.manual
    }
}

fn rating_path(
    s: &Screw,
    face: [f64; 2],
    tips: Option<[f64; 2]>,
    centre: f64,
) -> Option<CrossedPath> {
    let assumed = s.normal_module();
    let tips = tips.unwrap_or([
        s.worm_pitch_diameter / 2.0 + assumed,
        s.wheel_pitch_diameter / 2.0 + assumed,
    ]);
    let path = s.path_of_contact_at(tips[0], tips[1], centre)?;
    Some(path.limited_by_face(s, face).map_or(path, |(z, _)| z))
}

fn crossed_mesh(
    s: &Screw,
    members: &[WormMemberResult; 2],
    tips: Option<[f64; 2]>,
    parallel_axis_efficiency: Option<f64>,
    centre: f64,
) -> Option<CrossedMesh> {
    let radii = [
        members[0].pitch_diameter / 2.0,
        members[1].pitch_diameter / 2.0,
    ];
    // A standard tooth where the stage does not say: one **normal** module, the
    // shorter of the two conventions a worm is cut to, so the assumption errs
    // the same way the throating argument does.
    let assumed = s.normal_module();
    // Carried, not deduced: a crossed pair whose addendum happens to be one
    // module produces the same tips as the assumption does, and asking the
    // numbers which they were would answer wrongly exactly there.
    let tooth_height_assumed = tips.is_none();
    let tips = tips.unwrap_or([radii[0] + assumed, radii[1] + assumed]);
    let path = s.path_of_contact_at(tips[0], tips[1], centre)?;
    let face = [members[0].face_width, members[1].face_width];
    let (zone, limited_by) = path
        .limited_by_face(s, face)
        .unwrap_or((path, ZoneLimit::Face));
    Some(CrossedMesh {
        contact_ratio: zone.contact_ratio,
        limited_by,
        face_width_for_continuity: path.face_widths_for(s, 1.0),
        axial_travel: zone.axial_travel(s),
        tooth_height_assumed,
        parallel_axis_efficiency,
    })
}

fn angular_backlash(s: &Screw, stage: &WormStage, delta: f64, at: Member) -> f64 {
    // One normal, and both displacements projected onto it — the same vector the
    // path of contact is built on, rather than its components written out again.
    let Some(n) = s.contact_normal() else {
        return 0.0;
    };

    // **Two displacements, and they are not the same kind of thing.** Reading
    // them as one gap is what kept a factor of two wrong here (§12).
    //
    // A centre distance above nominal *separates the axes*, and a separation
    // opens **both** flanks by the same amount. Lost motion is the sum of the
    // gaps a member can travel across, so the factor is two — the same "a tooth
    // has two flanks" the path construction turns on.
    //
    // A worm's axial float is a **rigid-body slide** along its own axis, not a
    // separation: it opens one flank exactly as far as it closes the other, so
    // running from one end of the float to the other is lost once, not twice.
    //
    // `n[0]` is the centre line's projection and comes out `sin α_n` at every
    // shaft angle; `n[2]` is the worm axis's and is `sin β_b1`. Neither is a
    // small-angle reading — see `Screw::contact_normal`.
    let separation = 2.0 * delta.max(0.0) * n[0].abs();
    let slide = stage.axial_clearance * n[2].abs();

    let teeth = match at {
        Member::First => stage.starts,
        Member::Second => stage.wheel_teeth,
    };
    // The gap becomes an angle by the law every mesh here shares. `r cos α_n
    // sin γ` is the same number — `r sin γ = z m_n / 2` exactly — but writing it
    // that way made this the second home of a law the parallel mesh already had,
    // and the second home is the one that got the wrong numerator.
    crate::mesh::angular_play(separation + slide, teeth, s.normal_base_pitch())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn library() -> MaterialLibrary {
        super::super::test_library()
    }

    fn solved(stage: &WormStage) -> WormResult {
        solve_worm_stage(stage, 2.0, &library()).unwrap()
    }

    /// **Backlash, derived once and checked against the two rules the handbooks
    /// give separately.** Axial slack in the worm turns the wheel by
    /// `j/r₂`, and turns the worm itself by `2π j/lead`. Both come out of one
    /// projection onto the flank normal, which is the claim worth testing.
    #[test]
    fn axial_slack_reproduces_the_two_handbook_relations() {
        for (starts, teeth, d1) in [(1u32, 40u32, 7.0), (2, 31, 9.0), (4, 60, 14.0)] {
            let stage = WormStage {
                starts,
                wheel_teeth: teeth,
                sizing: FirstMemberSizing::PitchDiameter(d1),
                axial_clearance: 0.04,
                // isolate the axial term: no clearance on the centre distance
                clearance: 0.0,
                tolerance_plus: 0.0,
                tolerance_minus: 0.0,
                ..Default::default()
            };
            let r = solved(&stage);
            let s = stage.geometry().unwrap();

            let wheel = 0.04 / (s.wheel_pitch_diameter / 2.0);
            assert!(
                (r.backlash.forward.nominal.to_radians() - wheel).abs() < 1e-12 * wheel,
                "z₁={starts}: wheel backlash {} vs j/r₂ {}",
                r.backlash.forward.nominal.to_radians(),
                wheel
            );

            let worm = std::f64::consts::TAU * 0.04 / s.lead;
            assert!(
                (r.backlash.backward.nominal.to_radians() - worm).abs() < 1e-12 * worm,
                "z₁={starts}: worm backlash {} vs 2π j/lead {}",
                r.backlash.backward.nominal.to_radians(),
                worm
            );
        }
    }

    /// Opening the centre distance opens the flanks too, by its component along
    /// the normal — and closing it below nominal cannot make the backlash
    /// negative, because the flanks would simply touch.
    #[test]
    fn the_centre_distance_tolerance_moves_the_backlash_the_right_way() {
        let stage = WormStage {
            clearance: 0.0,
            axial_clearance: 0.0,
            tolerance_plus: 0.05,
            tolerance_minus: 0.05,
            ..Default::default()
        };
        let r = solved(&stage);
        assert_eq!(
            r.backlash.forward.nominal, 0.0,
            "nominal, with no slack at all"
        );
        assert!(
            r.backlash.forward.maximum > 0.0,
            "opening the centres opens the mesh"
        );
        assert_eq!(
            r.backlash.forward.minimum, 0.0,
            "tighter than nominal is contact"
        );
    }

    /// The stage's headline numbers, and the one it deliberately does not have.
    #[test]
    fn a_worm_stage_reports_contact_and_two_efficiencies_and_no_bending() {
        let r = solved(&WormStage::default());
        assert!((r.ratio - 40.0).abs() < 1e-12);
        assert!(r.efficiency.forward > 0.0 && r.efficiency.forward < 1.0);
        assert!(
            r.efficiency.backward < r.efficiency.forward,
            "back-driving is the worse direction"
        );
        assert!(r.contact.max_pressure > 0.0 && r.contact.max_pressure.is_finite());
        assert!(
            r.contact.curvature_along > 0.0,
            "crossed shafts make a point contact, not a line"
        );
        assert!(
            r.contact.patch_length > r.contact.patch_width,
            "an ellipse: {} by {}",
            r.contact.patch_length,
            r.contact.patch_width
        );
        // Torque follows the ratio and the operative efficiency.
        let expected = 2.0 * r.ratio * r.efficiency.forward;
        assert!((r.members[1].torque - expected).abs() < 1e-12 * expected);
    }

    /// **Why the automatic face width here is a proportion and not a rating.**
    /// A point contact's peak pressure does not depend on the face width, so
    /// `σ_H ∝ 1/√b` — the relation a spur stage inverts to size a gear — has
    /// nothing to invert.
    ///
    /// That holds *while the face is not what bounds the zone of action*, which
    /// is the case here and on any sanely proportioned pair. Squeeze the face
    /// until it cuts the zone and the pressure does move — **upward**, because
    /// the pair loses the load sharing that kept the rating off the ends of the
    /// path. Which is the other reason nothing sizes a face from contact here:
    /// the relation runs the wrong way, and inverting it would ask for a
    /// *narrower* gear. The next test is that one.
    #[test]
    fn contact_pressure_does_not_depend_on_the_face_width() {
        let narrow = solved(&WormStage {
            wheel: WormMember {
                face_width: Auto::fixed(4.0),
                material: "Brass C360".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let wide = solved(&WormStage {
            wheel: WormMember {
                face_width: Auto::fixed(40.0),
                material: "Brass C360".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(
            narrow.contact.max_pressure, wide.contact.max_pressure,
            "a ten-fold face width must change nothing about a point contact"
        );
    }

    /// **The recommended proportions behave like the things they describe.**
    ///
    /// They are conventions, so there is no independent derivation to check
    /// them against — which makes it the more important to assert what they
    /// must *do* rather than the arithmetic they are. Each of these would catch
    /// a transcription error without repeating the transcription.
    #[test]
    fn the_recommended_proportions_behave_as_lengths_of_a_worm_drive() {
        use proportions::{wheel_face_width, worm_length};

        // A bigger wheel wraps further round the worm, so it needs more worm to
        // wrap onto. Strictly monotone in the wheel's tooth count.
        let lengths: Vec<f64> = (10..80).map(|z| worm_length(2.0, z, 1)).collect();
        assert!(
            lengths.windows(2).all(|w| w[1] > w[0]),
            "worm length must grow with the wheel it carries"
        );

        // More starts steepen the lead, so the same wrap is spread over more
        // axial length.
        assert!(worm_length(2.0, 40, 4) > worm_length(2.0, 40, 1));

        // Both scale with the module: they are proportions, so doubling the
        // whole drive doubles them exactly.
        for (a, b) in [
            (worm_length(1.0, 40, 2) * 2.0, worm_length(2.0, 40, 2)),
            (
                wheel_face_width(1.0, 7.0) * 2.0,
                wheel_face_width(2.0, 14.0),
            ),
        ] {
            assert!((a - b).abs() < 1e-12, "not homogeneous in the module");
        }

        // The BS 721 cap is the operative statement on a slender worm and not on
        // a stout one, which is the whole reason it is written as a cap.
        let slender = wheel_face_width(1.0, 5.0);
        assert!(
            (slender - 0.67 * 5.0).abs() < 1e-12,
            "the cap must bind at q = 5: {slender}"
        );
        let stout = wheel_face_width(1.0, 30.0);
        assert!(
            stout < 0.67 * 30.0,
            "the cap must not bind at q = 30: {stout}"
        );

        // A wheel wider than the worm's own diameter would hang past the thread
        // at every proportion in the range worm practice uses.
        for q in [6.0_f64, 8.0, 10.0, 14.0, 18.0, 25.0] {
            let b2 = wheel_face_width(1.0, q);
            assert!(b2 > 0.0 && b2 < q, "q={q}: face width {b2} against d1 {q}");
        }
    }

    /// **The automatic width is the recommendation, and a hand-set one survives
    /// untouched** — with the recommendation still reported beside it, so the
    /// two can be read against each other.
    #[test]
    fn automatic_takes_the_recommendation_and_manual_is_left_alone() {
        let auto = solved(&WormStage::default());
        assert_eq!(
            Some(auto.members[0].face_width),
            auto.members[0].recommended_face_width,
            "an automatic worm length is the recommendation"
        );
        assert_eq!(
            Some(auto.members[1].face_width),
            auto.members[1].recommended_face_width,
            "an automatic wheel face width is the recommendation"
        );

        let manual = solved(&WormStage {
            worm: WormMember {
                face_width: Auto::fixed(3.5),
                ..Default::default()
            },
            ..Default::default()
        });
        assert!((manual.members[0].face_width - 3.5).abs() < 1e-12);
        assert_eq!(
            manual.members[0].recommended_face_width, auto.members[0].recommended_face_width,
            "the recommendation is reported whether or not it is in use"
        );
    }

    /// **A crossed gear pair gets no recommendation, and is told so.**
    ///
    /// The proportions describe a worm carrying an enveloping wheel. A crossed
    /// pair is two helical gears touching at a point, so quoting them there
    /// would be shipping a convention outside the case it was written for —
    /// and quietly, since the number would look like any other.
    #[test]
    fn a_crossed_pair_is_not_given_a_worms_proportions() {
        let crossed = solved(&WormStage {
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(45.0),
            worm: WormMember {
                face_width: Auto::automatic(6.0),
                ..Default::default()
            },
            ..Default::default()
        });
        assert!(crossed.members[0].recommended_face_width.is_none());
        assert!(crossed.members[1].recommended_face_width.is_none());
        // Automatic has nothing to take, so the entered width stands...
        assert!((crossed.members[0].face_width - 6.0).abs() < 1e-12);
        // ...and the result says why rather than leaving it to be noticed.
        assert!(
            crossed.notes.iter().any(|n| n.contains("enveloping wheel")),
            "no note explaining the absent recommendation: {:?}",
            crossed.notes
        );
    }

    /// **A crossed stage reports its contact ratio, and an automatic face width
    /// is the width that keeps it at 1.**
    ///
    /// The two have to agree: size the face automatically, and the ratio that
    /// comes back must be exactly 1. Everything else here is a comparison —
    /// a hand-set face narrower than that gives less, a wider one gives what the
    /// teeth allow and no more, and the number moves the way the geometry says.
    #[test]
    fn an_automatic_face_width_on_a_crossed_pair_buys_exactly_continuous_contact() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let gear = |teeth: u32, face: Auto<f64>| StageGear {
            teeth,
            face_width: face,
            ..StageGear::default()
        };
        let stage = |face: Auto<f64>| SpurStage {
            shaft_angle: 90.0,
            gears: [gear(17, face), gear(23, face)],
            ..SpurStage::default()
        };

        // Automatic: the width for ε = 1, and the ratio comes back as 1.
        let auto = solve_crossed_stage(&stage(Auto::automatic(0.0)), 2.0, &lib).unwrap();
        let m = auto.crossed.expect("a crossed pair has a path of contact");
        assert!(
            (m.contact_ratio - 1.0).abs() < 1e-9,
            "automatic should buy exactly continuous contact, got {}",
            m.contact_ratio
        );
        assert_eq!(m.limited_by, ZoneLimit::Face);
        let sized = m.face_width_for_continuity.expect("a width for continuity");
        for (i, (member, want)) in auto.members.iter().zip(sized).enumerate() {
            assert!(
                (member.face_width - want).abs() < 1e-9,
                "member {i} should be sized to {want}"
            );
        }

        // Half that face, about half the contact — and slightly less than half,
        // which is the point. The face is centred on its own gear and the
        // contact is not quite centred on the face, because the pair runs at its
        // nominal centre distance *plus the clearance* and that slides the
        // contact along the shafts (§4.4). Trimming a face symmetrically about
        // an asymmetric contact loses a little more than half.
        let narrow = solve_crossed_stage(&stage(Auto::fixed(sized[0] / 2.0)), 2.0, &lib).unwrap();
        let n = narrow.crossed.unwrap();
        assert!(
            n.contact_ratio < 0.5 && n.contact_ratio > 0.45,
            "half the face should leave a little under half the contact: {}",
            n.contact_ratio
        );

        // ...and it is *exactly* half once the clearance is taken away, which
        // pins the shortfall on the slide rather than on the arithmetic.
        let tight = |face: Auto<f64>| SpurStage {
            clearance: 0.0,
            ..stage(face)
        };
        let centred = solve_crossed_stage(&tight(Auto::automatic(0.0)), 2.0, &lib).unwrap();
        let width = centred
            .crossed
            .expect("a path")
            .face_width_for_continuity
            .expect("a width for continuity");
        let halved = solve_crossed_stage(&tight(Auto::fixed(width[0] / 2.0)), 2.0, &lib).unwrap();
        assert!(
            (halved.crossed.unwrap().contact_ratio - 0.5).abs() < 1e-9,
            "with the contact centred, half the face is exactly half the contact"
        );
        assert!(
            narrow.notes.iter().any(|s| s.contains("loses contact")),
            "a contact ratio below 1 must be said: {:?}",
            narrow.notes
        );

        // Generous, and the teeth are what end it — a wider face buys nothing.
        let wide = solve_crossed_stage(&stage(Auto::fixed(60.0)), 2.0, &lib).unwrap();
        let w = wide.crossed.unwrap();
        assert_eq!(w.limited_by, ZoneLimit::Tips);
        assert!(w.contact_ratio > m.contact_ratio);
        let wider = solve_crossed_stage(&stage(Auto::fixed(120.0)), 2.0, &lib).unwrap();
        assert!((wider.crossed.unwrap().contact_ratio - w.contact_ratio).abs() < 1e-12);
    }

    /// **A worm drive reports the same path, and says what it assumed to.**
    ///
    /// Every other figure a worm stage gives comes from a model in which both
    /// flanks are involute helicoids on cylinders, so withholding the one that
    /// comes from the same construction would be holding it to a standard
    /// nothing else here meets. What it *does* lack is a tooth height — there is
    /// no addendum input — so the tips are assumed and the result says so.
    ///
    /// Its face width stays proportion-sized: BS 721 and DIN are worm-drive
    /// conventions and answer a different question from continuity of contact.
    /// Both numbers are reported, each labelled with what it is.
    #[test]
    fn a_worm_drive_reports_the_path_as_a_floor_and_keeps_its_proportions() {
        let r = solved(&WormStage::default());
        let m = r.crossed.expect("the same construction serves a worm");
        assert!(
            m.tooth_height_assumed,
            "a worm stage has no addendum, so the tips must be flagged as assumed"
        );
        assert!(m.contact_ratio > 0.0);
        // The proportions still size the face; continuity is reported beside
        // them rather than instead of them.
        assert!(r.members[0].recommended_face_width.is_some());
        assert!(m.face_width_for_continuity.is_some());

        // A crossed gear pair carries its own tooth height, so nothing is
        // assumed there — the flag is what separates the two cases.
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};
        let crossed = solve_crossed_stage(
            &SpurStage {
                shaft_angle: 90.0,
                gears: [
                    StageGear {
                        teeth: 17,
                        face_width: Auto::fixed(6.0),
                        ..StageGear::default()
                    },
                    StageGear {
                        teeth: 23,
                        face_width: Auto::fixed(6.0),
                        ..StageGear::default()
                    },
                ],
                ..SpurStage::default()
            },
            2.0,
            &library(),
        )
        .unwrap();
        assert!(!crossed.crossed.unwrap().tooth_height_assumed);
    }

    /// **The rating walks the path, and the pitch point is near its gentlest
    /// place.**
    ///
    /// The relative radius peaks where the two roll lengths are equal, which the
    /// pitch point is near, so a figure taken there alone is close to the least
    /// severe the mesh offers.
    /// Checked as an ordering rather than as a number — the rated pressure can
    /// only be at or above what the pitch point says — and then as the
    /// consequence that matters: squeeze the face until `ε` falls below 1, and
    /// the same mesh rates **higher**, because with nothing else in contact the
    /// one pair carries the load out to the ends of the zone.
    #[test]
    fn rating_along_the_path_is_never_kinder_than_the_pitch_point() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let crossed = |face: f64| SpurStage {
            shaft_angle: 90.0,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(face),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 23,
                    face_width: Auto::fixed(face),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };

        // Generous face: the teeth end the zone, the pair shares load, and the
        // rating sits just above the pitch-point figure.
        let wide = solve_crossed_stage(&crossed(20.0), 2.0, &lib).unwrap();
        assert!(
            wide.contact.max_pressure >= wide.contact.at_pitch_point,
            "the path cannot be kinder than its gentlest point"
        );
        assert!(wide.crossed.unwrap().contact_ratio > 1.0);

        // Squeeze it: ε falls below 1, load sharing goes, and the same mesh at
        // the same torque rates higher.
        let narrow = solve_crossed_stage(&crossed(0.8), 2.0, &lib).unwrap();
        assert!(narrow.crossed.unwrap().contact_ratio < 1.0);
        assert!(
            narrow.contact.max_pressure > wide.contact.max_pressure,
            "losing contact continuity should cost stress, not save it: {} against {}",
            narrow.contact.max_pressure,
            wide.contact.max_pressure
        );
        // ...and it is the *place* being rated that did it, not the load. The
        // load moved too — a narrower face engages only the middle of the zone,
        // which slides less, which delivers more torque — so the two figures
        // are compared as a ratio, which divides the load out: the same mesh
        // rated at its worst against rated at its pitch point.
        let severity = |r: &WormResult| r.contact.max_pressure / r.contact.at_pitch_point;
        // The margin is small, and the reason is worth knowing: a narrow face
        // cuts the zone's *ends* off, and those ends are what made the path
        // severe. So losing the sharing pushes the rating outward while the
        // shortened zone pulls it back in, and the net is slight — 1.012
        // against 1.007. Asserted as the ordering, since the two effects fight
        // and a margin would be a guess about which wins by how much.
        assert!(
            severity(&narrow) > severity(&wide),
            "losing load sharing should move the rating further from the pitch \
             point: {} against {}",
            severity(&narrow),
            severity(&wide)
        );
    }

    /// **Counting more sliding can only cost efficiency**, and the stage's
    /// figure is now the path average rather than the pitch point.
    ///
    /// The pitch point is the one place on the path with no sliding up the
    /// profile, so averaging over the path can only find more of it. That the
    /// stage's number is at or below the classical one — in **both** directions,
    /// across worms and crossed pairs — is the law this integration has to obey,
    /// and it is asserted rather than the numbers it produced.
    ///
    /// The self-locking threshold moves with it, and must: quoted beside an
    /// efficiency, it has to be the friction at which *that* efficiency reaches
    /// zero. More sliding means locking at less friction, so the threshold falls
    /// too — and the pair reported as self-locking is the one whose reported
    /// backward efficiency is not positive.
    #[test]
    fn the_stage_counts_all_the_sliding_and_can_only_lose_by_it() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();

        for stage in [
            WormStage::default(),
            WormStage {
                starts: 2,
                wheel_teeth: 40,
                sizing: FirstMemberSizing::PitchDiameter(12.0),
                ..WormStage::default()
            },
        ] {
            let r = solve_worm_stage(&stage, 2.0, &lib).unwrap();
            let classical = r.crossed.map(|_| ()).map(|()| {
                let s = stage.geometry().unwrap();
                Directional::of(|d| s.efficiency(stage.friction, d))
            });
            let classical = classical.expect("a screw geometry");
            for drive in Drive::BOTH {
                assert!(
                    r.efficiency.get(drive) <= classical.get(drive),
                    "{drive:?}: the path average {} should not beat the pitch point {}",
                    r.efficiency.get(drive),
                    classical.get(drive)
                );
            }
            // The threshold is the friction at which the *reported* backward
            // efficiency reaches zero, so it moves down with it.
            assert!(r.self_locking_friction <= stage.geometry().unwrap().self_locking_friction());
            assert_eq!(
                r.efficiency.self_locking(),
                r.efficiency.backward <= 0.0,
                "self-locking is what the reported figure says, not a second opinion"
            );
        }

        // ...and the same for a crossed gear pair, where the term being added is
        // the whole of a parallel mesh's loss.
        let crossed = SpurStage {
            shaft_angle: 60.0,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };
        let r = solve_crossed_stage(&crossed, 2.0, &lib).unwrap();
        let m = r.crossed.expect("a path");
        assert!(
            r.efficiency.forward < m.parallel_axis_efficiency.unwrap(),
            "a crossed pair must still lose more than the same teeth parallel"
        );
    }

    /// **Crossing shafts adds sliding, and the figures show it** — with one
    /// exception that is the *other* formula's approximation, not this one's.
    ///
    /// A crossed pair loses more than the same teeth running parallel, and by
    /// more the further the shafts are turned. At a shaft angle small enough
    /// that the two are the same mesh, the balance comes out a hundredth of a
    /// point *above* the parallel closed form — because that closed form is
    /// first order in `μ` and the balance is exact in it. `screw.rs` measures
    /// that difference as O(μ²); here it is allowed for and bounded.
    #[test]
    fn a_crossed_pair_loses_more_the_further_its_shafts_are_turned() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64| SpurStage {
            shaft_angle: sigma,
            additional_helix: 20.0,
            gears: [
                StageGear {
                    teeth: 17,
                    // Wide enough that the **tips** end the zone at every shaft
                    // angle below, which is what makes this a like-for-like
                    // comparison. A 12 mm face is not: near the parallel limit a
                    // centre-distance error slides the contact several
                    // millimetres along the shafts, the face cuts the zone
                    // short, and comparing a partly engaged pair against a fully
                    // engaged one measures the truncation rather than the
                    // sliding. That effect has its own test below.
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(60.0),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };

        let mut previous = 1.0;
        for sigma in [0.5f64, 2.0, 10.0, 45.0, 90.0] {
            let r = solve_crossed_stage(&stage(sigma), 2.0, &lib).unwrap();
            let mesh = r.crossed.expect("a path");
            assert_eq!(
                mesh.limited_by,
                ZoneLimit::Tips,
                "Σ={sigma}°: the face is cutting the zone, so this is not a \
                 like-for-like comparison"
            );
            let parallel = mesh
                .parallel_axis_efficiency
                .expect("a parallel counterpart");
            assert!(
                r.efficiency.forward < previous,
                "Σ={sigma}°: turning the shafts further must cost more"
            );
            previous = r.efficiency.forward;

            // Below the parallel figure everywhere the difference is physical,
            // and above it by no more than that formula's own linearisation
            // where the two are the same mesh.
            assert!(
                r.efficiency.forward < parallel + 2e-4,
                "Σ={sigma}°: {} against the parallel {parallel}",
                r.efficiency.forward
            );
        }

        // A worm has no parallel-axis counterpart — a one-start thread is not a
        // gear — so the comparison is absent rather than invented.
        assert!(solved(&WormStage::default())
            .crossed
            .unwrap()
            .parallel_axis_efficiency
            .is_none());
    }

    /// A self-locking pair says so, rather than reporting a negative efficiency
    /// and leaving the reader to notice.
    #[test]
    fn self_locking_is_said_out_loud() {
        let r = solved(&WormStage {
            starts: 1,
            sizing: FirstMemberSizing::PitchDiameter(25.0),
            friction: 0.06,
            ..Default::default()
        });
        assert!(r.efficiency.self_locking());
        assert!(
            r.notes.iter().any(|n| n.contains("self-locking")),
            "notes: {:?}",
            r.notes
        );
    }

    #[test]
    fn a_pair_that_cannot_exist_says_which_way_it_failed() {
        let err = solve_worm_stage(
            &WormStage {
                starts: 9,
                sizing: FirstMemberSizing::PitchDiameter(8.0),
                ..Default::default()
            },
            2.0,
            &library(),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("too thin"), "{err}");

        let err = solve_worm_stage(
            &WormStage {
                wheel: WormMember {
                    material: "Unobtainium".into(),
                    ..Default::default()
                },
                ..Default::default()
            },
            2.0,
            &library(),
        )
        .unwrap_err();
        assert!(matches!(err, TrainError::UnknownMaterial(_)), "{err}");
    }
    /// **A crossed gear pair is this same stage, sized the other way.**
    ///
    /// Giving the first member a helix angle instead of a diameter must produce a
    /// lead angle of exactly `90° − β₁`, and a second member at `β₂ = Σ − β₁`.
    /// That is the whole of §4.5.1's claim that worm and crossed-helical gearing
    /// are one thing, and it is checkable without knowing any answers.
    #[test]
    fn a_helix_angle_and_a_pitch_diameter_describe_the_same_pair() {
        for (z1, z2) in [(17u32, 23u32), (20, 20), (1, 40)] {
            for sigma in [45.0f64, 60.0, 90.0] {
                for beta1 in [15.0f64, 30.0, 45.0] {
                    if sigma - beta1 < 0.0 || sigma - beta1 > 89.0 {
                        continue;
                    }
                    let by_angle = WormStage {
                        shaft_angle: sigma,
                        starts: z1,
                        wheel_teeth: z2,
                        sizing: FirstMemberSizing::HelixAngle(beta1),
                        ..WormStage::default()
                    };
                    let Ok(a) = by_angle.geometry() else { continue };
                    assert!(
                        (a.lead_angle.to_degrees() - (90.0 - beta1)).abs() < 1e-9,
                        "z={z1}/{z2} S={sigma} b={beta1}: lead angle {}",
                        a.lead_angle.to_degrees()
                    );
                    assert!(
                        (a.wheel_helix_angle.to_degrees() - (sigma - beta1)).abs() < 1e-9,
                        "z={z1}/{z2} S={sigma} b={beta1}: wheel helix {}",
                        a.wheel_helix_angle.to_degrees()
                    );

                    // ...and handing the derived diameter back as a diameter is
                    // the same pair, which is what "sized the other way" means.
                    let by_diameter = WormStage {
                        sizing: FirstMemberSizing::PitchDiameter(by_angle.first_pitch_diameter()),
                        ..by_angle.clone()
                    };
                    let b = by_diameter.geometry().unwrap();
                    assert_eq!(a.lead_angle, b.lead_angle);
                    assert_eq!(a.wheel_pitch_diameter, b.wheel_pitch_diameter);
                    assert_eq!(a.centre_distance, b.centre_distance);
                }
            }
        }
    }

    /// **A crossed-axis *spur* pair works, with the spur member second.**
    ///
    /// Put the helical member first at `β₁ = Σ` and the mate comes out at
    /// `β₂ = 0` — an ordinary spur gear, crossed. Which member is "first" is a
    /// labelling choice, and this is the labelling that has a lead angle to
    /// speak of.
    ///
    /// The other order is genuinely not representable, and for a reason worth
    /// stating rather than working around: at `β₁ = 0` the first member's lead
    /// angle is exactly 90°, so its *lead* and *axial module* are infinite. Not a
    /// numerical artefact — a spur gear is not a screw, and it has no lead. The
    /// stage refuses that ordering and says so.
    #[test]
    fn a_crossed_axis_spur_pair_puts_the_spur_member_second() {
        for sigma in [20.0f64, 30.0, 45.0, 60.0] {
            let stage = WormStage {
                shaft_angle: sigma,
                starts: 17,
                wheel_teeth: 23,
                // The helical member takes the whole shaft angle...
                sizing: FirstMemberSizing::HelixAngle(sigma),
                ..WormStage::default()
            };
            let s = stage
                .geometry()
                .unwrap_or_else(|e| panic!("Sigma={sigma}: {e}"));
            // ...leaving the mate a spur gear.
            assert!(
                s.wheel_helix_angle.abs() < 1e-9,
                "Sigma={sigma}: wheel helix {}",
                s.wheel_helix_angle.to_degrees()
            );
            // Its pitch diameter is then the plain `z m_n`, no helix correction.
            assert!((s.wheel_pitch_diameter - 23.0).abs() < 1e-12);
            // And everything reported stays finite — the singular quantities are
            // the *first* member's, and the first member is the helical one.
            assert!(s.lead.is_finite() && s.axial_module.is_finite());
            assert!(s.sliding_ratio.is_finite() && s.centre_distance.is_finite());
        }

        // The other way round is refused, not fudged.
        let backwards = WormStage {
            shaft_angle: 30.0,
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(0.0),
            ..WormStage::default()
        };
        assert!(
            backwards.geometry().is_err(),
            "a spur first member has no lead angle to report"
        );
    }

    /// **A ninety-degree helix is a disc, not a gear**, and it has to be refused
    /// where the helix angle is still known.
    ///
    /// `cos 90°` is 6e-17 rather than zero, so a derived pitch diameter comes out
    /// *enormous* rather than infinite — 2.8e17 mm on a 17-tooth member — and
    /// then `sin γ₁ = z m_n / d` is 6e-17, so the `WormTooThin` guard at the
    /// other end of the range sees nothing wrong. Found by sweeping the helix
    /// split in `gear-cli crossed` and reading the last row.
    #[test]
    fn a_ninety_degree_helix_is_refused_where_it_is_still_visible() {
        for beta in [90.0f64, 91.0, 120.0, -90.0] {
            let stage = WormStage {
                shaft_angle: 90.0,
                starts: 17,
                wheel_teeth: 23,
                sizing: FirstMemberSizing::HelixAngle(beta),
                ..WormStage::default()
            };
            assert!(
                stage.geometry().is_err(),
                "beta={beta}: a disc is not a gear"
            );
        }
        // Just inside is silly but representable — the project's standing rule is
        // that a limit answers "could this exist", not "would anyone want it".
        let stage = WormStage {
            shaft_angle: 90.0,
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(89.0),
            ..WormStage::default()
        };
        let g = stage.geometry().unwrap();
        assert!(g.worm_pitch_diameter.is_finite() && g.worm_pitch_diameter > 0.0);
    }

    /// **The merged stage is the same machine, to the last bit.**
    ///
    /// A crossed pair used to be entered as a worm stage sized by helix angle;
    /// it is now a spur stage with a shaft angle, which is what the
    /// specification asks for. That is a change of *input*, so the answer must
    /// not move — and this compares the two routes rather than asserting
    /// remembered numbers, so it keeps meaning something as the model changes.
    #[test]
    fn a_crossed_spur_stage_is_the_screw_stage_it_used_to_be_entered_as() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let gear = |teeth: u32| StageGear {
            teeth,
            face_width: Auto::fixed(8.0),
            ..StageGear::default()
        };
        let spur = SpurStage {
            shaft_angle: 90.0,
            additional_helix: 0.0,
            gears: [gear(17), gear(23)],
            ..SpurStage::default()
        };
        let as_screw = WormStage {
            shaft_angle: 90.0,
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(45.0),
            axial_clearance: 0.0,
            worm: WormMember {
                face_width: Auto::fixed(8.0),
                ..WormMember::default()
            },
            wheel: WormMember {
                face_width: Auto::fixed(8.0),
                material: "4340 Hardened Steel".into(),
                ..WormMember::default()
            },
            ..WormStage::default()
        };

        let a = solve_crossed_stage(&spur, 2.0, &lib).unwrap();
        let b = solve_worm_stage(&as_screw, 2.0, &lib).unwrap();
        for (name, x, y) in [
            ("ratio", a.ratio, b.ratio),
            ("centre distance", a.centre_distance, b.centre_distance),
            ("lead angle", a.lead_angle, b.lead_angle),
            ("efficiency", a.efficiency.forward, b.efficiency.forward),
            ("contact", a.contact.max_pressure, b.contact.max_pressure),
            (
                "backlash",
                a.backlash.forward.nominal,
                b.backlash.forward.nominal,
            ),
        ] {
            assert_eq!(x.to_bits(), y.to_bits(), "{name}: {x} against {y}");
        }

        // The helix angles are what the shaft angle says they are, and they sum
        // to it — the relation the screw model runs on.
        let [b1, b2] = spur.helix_angles();
        assert!((b1 - 45.0).abs() < 1e-12 && (b2 - 45.0).abs() < 1e-12);
        assert!((b1 + b2 - spur.shaft_angle).abs() < 1e-12);
    }

    /// **A parallel stage is the shaft angle's zero, not a separate thing.**
    ///
    /// At `Σ = 0` the additional helix is the whole of each gear's helix and the
    /// two hands are opposed, which is exactly what the stage did before it had
    /// a shaft angle at all. Asserted on the geometry the mesh is built from,
    /// so it holds whatever the solve does with it.
    #[test]
    fn a_parallel_stage_is_the_shaft_angles_zero() {
        use crate::train::SpurStage;

        for additional in [0.0_f64, 12.5, -30.0] {
            let stage = SpurStage {
                additional_helix: additional,
                ..SpurStage::default()
            };
            let [b1, b2] = stage.helix_angles();
            assert!((b1 - additional).abs() < 1e-12);
            assert!((b2 + additional).abs() < 1e-12, "the hands must oppose");
            assert!(!stage.is_crossed());
        }
    }

    /// **The crossed backlash law and the parallel one meet where the shafts
    /// straighten — and the gate is the meeting, not a number.**
    ///
    /// This is the check that was missing while the crossed centre-distance term
    /// counted one flank where two open, and a factor of two sat in the model
    /// for a milestone (§12): every crossed backlash test held the crossed model
    /// against itself, and the parallel model — which has the exact involute law
    /// and its own tests — was never asked.
    ///
    /// They are **not the same law** and must not be forced to be. A parallel
    /// pair's line of action rotates as the centres separate, so its backlash is
    /// exactly `2a′(inv α′ − inv α_w)` and nonlinear in the error. A crossed
    /// pair's line cannot rotate (`Screw::contact_normal`), so its backlash is
    /// exactly linear. What must hold is that they agree to **first order**, and
    /// the way to test that without picking a tolerance out of the air is to
    /// watch the shortfall vanish *with the error*: halve the clearance and the
    /// relative gap must halve too.
    #[test]
    fn the_crossed_backlash_meets_the_parallel_law_at_its_limit() {
        use crate::params::Auto;
        use crate::train::spur::solve_stage;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| SpurStage {
            shaft_angle: sigma,
            additional_helix: 20.0,
            clearance,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };

        let mut last = f64::INFINITY;
        for clearance in [0.08_f64, 0.04, 0.02, 0.01, 0.005] {
            let parallel = solve_stage(&stage(0.0, clearance), 2.0, &lib)
                .expect("a parallel pair")
                .backlash
                .forward
                .nominal;
            // As close to parallel as the screw model will go. The pair is still
            // crossed, so it is still the crossed law answering.
            let crossed = solve_crossed_stage(&stage(0.001, clearance), 2.0, &lib)
                .expect("a crossed pair")
                .backlash
                .forward
                .nominal;

            assert!(
                parallel > 0.0 && crossed > 0.0,
                "clearance {clearance}: {parallel} and {crossed}"
            );
            let gap = (parallel - crossed) / parallel;
            assert!(
                gap > 0.0,
                "clearance {clearance}: the crossed figure must sit just below the \
                 parallel one, since the parallel law's second-order term only adds. \
                 Got {crossed} against {parallel}"
            );
            assert!(
                gap < last,
                "clearance {clearance}: the shortfall is {gap} and did not fall \
                 from {last} — the two laws are not meeting"
            );
            last = gap;
        }
        // ...and the agreement is real rather than merely improving: at 5 µm of
        // clearance the two laws sit 0.052 % apart, which is the parallel law's
        // second-order term and nothing else. The bound is loose because the
        // *trend* above is the gate; this only catches a model that converges
        // on the wrong number.
        assert!(
            last < 1e-3,
            "the closest the two laws came was {last} apart, relative"
        );
    }

    /// The other half of the same story: the shortfall above is **second order**,
    /// so it must fall by half when the clearance does. A first-order error —
    /// the factor of two that was there — would not move at all.
    #[test]
    fn the_shortfall_at_the_limit_is_second_order_in_the_error() {
        use crate::params::Auto;
        use crate::train::spur::solve_stage;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| SpurStage {
            shaft_angle: sigma,
            additional_helix: 20.0,
            clearance,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(8.0),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };
        let shortfall = |clearance: f64| {
            let parallel = solve_stage(&stage(0.0, clearance), 2.0, &lib)
                .expect("a parallel pair")
                .backlash
                .forward
                .nominal;
            let crossed = solve_crossed_stage(&stage(0.001, clearance), 2.0, &lib)
                .expect("a crossed pair")
                .backlash
                .forward
                .nominal;
            (parallel - crossed) / parallel
        };
        let (coarse, fine) = (shortfall(0.04), shortfall(0.02));
        let halving = coarse / fine;
        assert!(
            (halving - 2.0).abs() < 0.05,
            "halving the clearance moved the shortfall by {halving}×, not 2× — \
             the residual is not the second-order term it is documented as"
        );
    }

    /// **A worm stage is rated where it runs, too.**
    ///
    /// The crossed path above reaches the operating centre distance through
    /// `solve_crossed_stage`, which overrides it; a plain worm stage reaches it
    /// through `solve_worm_stage` itself, and that is a second call site with a
    /// second chance to be left on the nominal. Gated separately for exactly
    /// that reason — reverting either one alone must fail something.
    ///
    /// Asserted as a direction, not a figure: opening the centre distance
    /// separates the base cylinders along the line of action, which can only
    /// **shorten** the zone, so more clearance is less contact.
    ///
    /// The efficiency is asserted only to *move*, because its direction is not a
    /// law. A shorter zone is less sliding to pay for, but which end it loses
    /// decides whether that helps: a near-parallel pair loses the ends, where
    /// profile sliding is worst, and gets better (0.98786 → 0.98801 at
    /// `Σ = 0.5°`); a worm loses path where the balance was doing comparatively
    /// well and gets worse (68.430 → 67.525 % from zero to 0.3 mm). Asserting a
    /// direction here would be asserting one of those two cases.
    #[test]
    fn a_worm_stage_is_rated_at_the_centre_distance_it_runs_at() {
        let stage = |clearance: f64| WormStage {
            clearance,
            ..WormStage::default()
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solved(&stage(clearance));
            let eps = r.crossed.expect("a path of contact").contact_ratio;
            let eta = r.efficiency.forward;
            if let Some((was_eps, was_eta)) = previous {
                assert!(
                    eps < was_eps,
                    "clearance {clearance}: opening the centres can only shorten \
                     the zone — ε {eps} against {was_eps}"
                );
                assert!(
                    (eta - was_eta).abs() > 1e-9,
                    "clearance {clearance}: the rating did not move, so the centre \
                     distance is not reaching it"
                );
            }
            previous = Some((eps, eta));
        }
    }

    /// **A centre-distance error slides a crossed pair's contact along the
    /// shafts, and near the parallel limit it slides clean off the face.**
    ///
    /// The effect the model could not see while every path was built at the
    /// zero-backlash centre distance. A crossed pair's line of action cannot
    /// turn when the centres move (§4.4), so it *translates* instead — and the
    /// translation grows roughly as `1/sin Σ`, without bound. A 20 µm clearance
    /// is nothing at 90° and several millimetres at half a degree.
    ///
    /// Asserted as the trend and its consequence rather than as millimetres:
    /// with the clearance the near-parallel pair is **face-limited and below
    /// ε = 1**, and taking the clearance away restores it. That is a designer's
    /// question — will these teeth touch as built — answered where it used to
    /// silently report the contact ratio of a pair nobody assembled.
    #[test]
    fn a_centre_distance_error_slides_a_crossed_pairs_contact_off_its_face() {
        use crate::params::Auto;
        use crate::train::{SpurStage, StageGear};

        let lib = super::super::test_library();
        let stage = |sigma: f64, clearance: f64| SpurStage {
            shaft_angle: sigma,
            additional_helix: 20.0,
            clearance,
            gears: [
                StageGear {
                    teeth: 17,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 43,
                    face_width: Auto::fixed(12.0),
                    ..StageGear::default()
                },
            ],
            ..SpurStage::default()
        };
        let mesh = |sigma: f64, clearance: f64| {
            solve_crossed_stage(&stage(sigma, clearance), 2.0, &lib)
                .expect("a crossed pair")
                .crossed
                .expect("a path of contact")
        };

        // At a right angle the same clearance costs almost nothing...
        let square = mesh(90.0, 0.02);
        assert_eq!(square.limited_by, ZoneLimit::Tips);
        assert!(square.contact_ratio > 1.5);

        // ...and near the parallel limit it takes the mesh apart.
        let near = mesh(0.5, 0.02);
        assert_eq!(
            near.limited_by,
            ZoneLimit::Face,
            "the contact should have slid past the face"
        );
        assert!(
            near.contact_ratio < 1.0,
            "a pair whose contact has left the face cannot be continuous: {}",
            near.contact_ratio
        );

        // The clearance is the whole of it: take it away and the same teeth on
        // the same face are tip-limited again, with contact to spare.
        let centred = mesh(0.5, 0.0);
        assert_eq!(centred.limited_by, ZoneLimit::Tips);
        assert!(
            centred.contact_ratio > 1.5 * near.contact_ratio,
            "{} against {}",
            centred.contact_ratio,
            near.contact_ratio
        );

        // And it is monotone in the shaft angle, which is the `1/sin Σ` showing
        // through without a number being written down.
        let mut previous = 0.0;
        for sigma in [0.5f64, 1.0, 5.0, 30.0, 90.0] {
            let eps = mesh(sigma, 0.02).contact_ratio;
            assert!(
                eps > previous,
                "Σ={sigma}°: straightening the shafts must cost more contact, \
                 not less — {eps} against {previous}"
            );
            previous = eps;
        }
    }

    /// A crossed gear pair solves end to end, and reports what a worm stage
    /// reports — the result shape is shared because the mathematics is.
    #[test]
    fn a_crossed_gear_pair_solves_end_to_end() {
        let stage = WormStage {
            shaft_angle: 90.0,
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(45.0),
            ..WormStage::default()
        };
        let r = solve_worm_stage(&stage, 2.0, &super::super::test_library()).unwrap();
        assert!((r.ratio - 23.0 / 17.0).abs() < 1e-12);
        assert!(r.efficiency.forward > 0.0 && r.efficiency.forward < 1.0);
        assert!(r.contact.max_pressure > 0.0);
        assert!(!r.efficiency.self_locking());

        // **Where a crossed pair sits, stated as comparisons rather than a
        // threshold.** It slides hard at the pitch point — `1/cos γ₁`, which is
        // 1.41 × the pitch line speed at 45° — so it is far worse than a
        // parallel-axis mesh and far better than a worm of the same shaft angle.
        // Both bounds are computed here rather than remembered.
        let worm = solve_worm_stage(
            &WormStage {
                shaft_angle: 90.0,
                starts: 1,
                wheel_teeth: 40,
                sizing: FirstMemberSizing::PitchDiameter(7.0),
                ..WormStage::default()
            },
            2.0,
            &super::super::test_library(),
        )
        .unwrap();
        assert!(
            r.efficiency.forward > worm.efficiency.forward,
            "a crossed gear pair should beat a worm: {} vs {}",
            r.efficiency.forward,
            worm.efficiency.forward
        );
        assert!(
            r.efficiency.forward < 0.95,
            "...but it slides too much to approach a parallel-axis mesh: {}",
            r.efficiency.forward
        );

        // ...and more shaft angle means more sliding means less efficiency.
        let mut previous = f64::INFINITY;
        for sigma in [20.0f64, 40.0, 60.0, 90.0] {
            let e = solve_worm_stage(
                &WormStage {
                    shaft_angle: sigma,
                    starts: 17,
                    wheel_teeth: 23,
                    sizing: FirstMemberSizing::HelixAngle(sigma / 2.0),
                    ..WormStage::default()
                },
                2.0,
                &super::super::test_library(),
            )
            .unwrap()
            .efficiency
            .forward;
            assert!(
                e < previous,
                "Sigma={sigma}: {e} should be below {previous}"
            );
            previous = e;
        }
    }
}
