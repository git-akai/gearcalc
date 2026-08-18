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
//! # Backlash comes out of one projection
//!
//! Two things open a gap between the flanks: axial slack in the worm, and a
//! centre distance larger than nominal. Both are displacements, and what matters
//! is their component along the common flank normal `n̂`:
//!
//! ```text
//! j_n = j_axial · sin β_b1   +   Δa · sin α_n
//! ```
//!
//! — the worm's axis direction and the centre line, each projected onto `n̂`.
//! Closing that gap takes a rotation of either member, and the rotation is the
//! gap divided by how fast that member's surface moves along `n̂`:
//!
//! ```text
//! θ₁ = j_n / (r₁ cos α_n sin γ₁),    θ₂ = j_n / (r₂ cos α_n sin γ₂)
//! ```
//!
//! At a right-angle drive that reduces to the two relations the handbooks give
//! separately — the wheel turns by `j_axial/r₂`, and the worm by
//! `2π j_axial/lead` — which is the check the tests make. It is one projection
//! rather than two rules, and it holds at any shaft angle.

use super::{Backlash, TrainError};
use crate::contact::{Directional, Drive};
use crate::material::{contact_modulus, Material, MaterialLibrary, Overrides};
use crate::mesh::Member;
use crate::params::Auto;
use crate::screw::{Screw, ScrewParams};

/// One member of a worm stage.
///
/// Note what is *absent* against [`super::StageGear`]: no profile shift, no
/// addendum, no automatic face width. The first two belong to a generated
/// involute profile that a worm stage does not yet build; the third would need a
/// strength model that the face width enters, and for a point contact it does
/// not enter at all.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct WormMember {
    /// Face width of the wheel, or length of the worm, mm.
    ///
    /// A plain input. The specification offers an automatic calculation for
    /// each, and both published rules are proportions — `b₂ ≈ 2 m_x √(z₁+1)`
    /// and its relatives — which is to say conventions rather than derivations.
    /// §4.7's standing policy is that this project does not ship those, so the
    /// field stays manual until there is something to derive it from.
    pub face_width: f64,
    /// Name of a material in the library.
    pub material: String,
    #[cfg_attr(feature = "serde", serde(default))]
    pub material_overrides: Overrides,
}

impl Default for WormMember {
    fn default() -> Self {
        Self {
            face_width: 10.0,
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
    /// The worm's pitch diameter, mm — the input that sets the lead angle.
    pub worm_pitch_diameter: f64,
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
            worm_pitch_diameter: 7.0,
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
    pub face_width: f64,
    pub pitch_diameter: f64,
    /// The material as used, after any overrides.
    pub material: Material,
}

/// The contact patch a worm mesh presses.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WormContact {
    /// Peak Hertzian pressure, MPa — **the** strength figure for this stage.
    pub max_pressure: f64,
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
    /// Helix angle of the wheel, degrees — an output, per the specification.
    pub wheel_helix_angle: f64,
    /// Lead, mm, and axial module, mm.
    pub lead: f64,
    pub axial_module: f64,
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

impl WormStage {
    /// The screw geometry this stage describes.
    ///
    /// # Errors
    ///
    /// [`TrainError::Screw`] if the pair cannot exist.
    pub fn geometry(&self) -> Result<Screw, TrainError> {
        Screw::new(&ScrewParams {
            normal_module: self.module,
            normal_pressure_angle: self.pressure_angle.to_radians(),
            shaft_angle: self.shaft_angle.to_radians(),
            starts: self.starts,
            wheel_teeth: self.wheel_teeth,
            worm_pitch_diameter: self.worm_pitch_diameter,
        })
        .map_err(TrainError::Screw)
    }
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

    let (centre, _clearance) = if stage.centre_distance.auto {
        (s.centre_distance + stage.clearance, stage.clearance)
    } else {
        (stage.centre_distance.manual, 0.0)
    };

    let efficiency = Directional::of(|d| s.efficiency(stage.friction, d));
    let threshold = s.self_locking_friction();
    let output_torque = input_torque * s.ratio * efficiency.forward;

    // Contact is rated on the wheel's torque: which torque is held fixed decides
    // which way friction moves the flank load, and only this direction is the
    // conservative one. See `Screw::normal_force`.
    let e_star = contact_modulus(&materials[0], &materials[1]);
    let (curvature_along, curvature_across) =
        s.contact_curvatures().ok_or(TrainError::NoContact)?;
    let patch = s
        .contact(output_torque, Member::Second, stage.friction, e_star)
        .ok_or(TrainError::NoContact)?;

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

    let members = [
        WormMemberResult {
            torque: input_torque,
            speed: 0.0,
            tooth_cycles: 0.0,
            face_width: stage.worm.face_width,
            pitch_diameter: s.worm_pitch_diameter,
            material: materials[0].clone(),
        },
        WormMemberResult {
            torque: output_torque,
            speed: 0.0,
            tooth_cycles: 0.0,
            face_width: stage.wheel.face_width,
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
        wheel_helix_angle: s.wheel_helix_angle.to_degrees(),
        lead: s.lead,
        axial_module: s.axial_module,
        efficiency,
        self_locking_friction: threshold,
        sliding_ratio: s.sliding_ratio,
        sliding_velocity: 0.0,
        contact: WormContact {
            max_pressure: patch.max_pressure,
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
fn angular_backlash(s: &Screw, stage: &WormStage, delta: f64, at: Member) -> f64 {
    let alpha_n = stage.pressure_angle.to_radians();
    let beta_b1 = (s.worm_helix_angle.sin() * alpha_n.cos()).asin();

    let gap = stage.axial_clearance * beta_b1.sin() + delta.max(0.0) * alpha_n.sin();
    let (radius, lead) = match at {
        Member::First => (s.worm_pitch_diameter / 2.0, s.lead_angle),
        Member::Second => (s.wheel_pitch_diameter / 2.0, s.wheel_lead_angle),
    };
    gap / (radius * alpha_n.cos() * lead.sin())
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
                worm_pitch_diameter: d1,
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

    /// **Why there is no automatic face width here.** A point contact's peak
    /// pressure does not depend on the face width at all, so `σ_H ∝ 1/√b` — the
    /// relation a spur stage inverts to size a gear — has nothing to invert.
    #[test]
    fn contact_pressure_does_not_depend_on_the_face_width() {
        let narrow = solved(&WormStage {
            wheel: WormMember {
                face_width: 4.0,
                material: "Brass C360".into(),
                ..Default::default()
            },
            ..Default::default()
        });
        let wide = solved(&WormStage {
            wheel: WormMember {
                face_width: 40.0,
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

    /// A self-locking pair says so, rather than reporting a negative efficiency
    /// and leaving the reader to notice.
    #[test]
    fn self_locking_is_said_out_loud() {
        let r = solved(&WormStage {
            starts: 1,
            worm_pitch_diameter: 25.0,
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
                worm_pitch_diameter: 8.0,
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
}
