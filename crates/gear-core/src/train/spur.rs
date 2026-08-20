//! The parallel-axis stage: spur when the helix angle is zero, helical
//! otherwise.
//!
//! Split out of `train.rs` when the worm stage arrived and gave the division
//! something to divide. What stays in the parent module is everything a stage
//! of *any* kind produces — [`StageResult`], [`GearResult`], [`Backlash`] — and
//! the train that strings them together.

use super::{Backlash, ContactRatios, GearResult, SpurResult, TrainError};
use crate::auto::{addendum_for_tip_width, admissible_ranges, automatic_profile_shift};
use crate::contact::{efficiency, ContactPath, Directional, Drive};
use crate::material::{contact_modulus, Material, MaterialLibrary, Overrides};
use crate::mesh::{Member, Mesh, MeshKind};
use crate::params::{Auto, GearParams};
use crate::profile::Gear;
use crate::strength::{
    bending_section, bending_stress, contact_stress, min_face_width_bending,
    min_face_width_contact, Load, StressConcentration, PARALLEL_AXES,
};

/// One gear of a stage.
///
/// Note what is *absent*: module, pressure angle and helix angle live on the
/// stage, because they are shared.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct StageGear {
    pub teeth: u32,
    /// Automatic uses [`minimum_profile_shift`] at `working_depth`.
    pub profile_shift: Auto<f64>,
    /// Depth, in modules, at which the undercut question is asked.
    pub working_depth: f64,
    /// Automatic uses [`addendum_for_tip_width`] at `min_tip_width`.
    pub addendum: Auto<f64>,
    /// Minimum transverse tooth tip width, mm.
    pub min_tip_width: f64,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Automatic takes the larger of the enabled minimums below.
    pub face_width: Auto<f64>,
    pub auto_face_from_bending: bool,
    pub auto_face_from_contact: bool,
    /// Name of a material in the library.
    pub material: String,
    /// Properties replaced for this gear only. Empty means "as the library
    /// says" — see [`Overrides`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub material_overrides: Overrides,
}

impl Default for StageGear {
    fn default() -> Self {
        Self {
            teeth: 17,
            profile_shift: Auto::automatic(0.0),
            working_depth: 1.0,
            addendum: Auto::fixed(1.0),
            min_tip_width: 0.1,
            dedendum: 1.25,
            root_radius: 0.38,
            face_width: Auto::fixed(10.0),
            auto_face_from_bending: true,
            auto_face_from_contact: true,
            material: "4340 Hardened Steel".to_string(),
            material_overrides: Overrides::default(),
        }
    }
}

/// A parallel-axis stage: spur when the helix angle is zero, helical otherwise.
///
/// Crossed axes are milestone 10 and are not accepted here; the specification's
/// "Axis Angle" input belongs with that work.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SpurStage {
    /// Normal module, mm. Shared by both gears.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// Total helix angle, degrees. Shared; gear 2 takes the opposite hand.
    pub helix_angle: f64,
    /// Coefficient of friction for the mesh.
    pub friction: f64,
    /// `k₁`. Gear 2 takes `2 − k₁` by construction.
    pub thickness_mod: f64,
    /// Automatic uses the zero-backlash centre distance plus `clearance`.
    pub centre_distance: Auto<f64>,
    /// Added to the centre distance, mm. Forced to zero when the centre distance
    /// is set manually, per the specification.
    pub clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    pub gears: [StageGear; 2],
}

impl Default for SpurStage {
    fn default() -> Self {
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            helix_angle: 0.0,
            friction: 0.06,
            thickness_mod: 1.0,
            centre_distance: Auto::automatic(0.0),
            clearance: 0.02,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            gears: [
                StageGear::default(),
                StageGear {
                    teeth: 43,
                    ..StageGear::default()
                },
            ],
        }
    }
}

impl SpurStage {
    /// `GearParams` for one gear, with the automatic values resolved.
    ///
    /// The two automatic calculations are ordered, and the order matters: the
    /// shift is chosen first because the addendum solve needs `ψ_b`, which
    /// depends on it. The reverse dependency does not exist — `minimum_profile_shift`
    /// touches only `r`, `α_t` and the cutter, none of which the addendum moves —
    /// so there is no loop to iterate.
    pub(super) fn params(&self, i: usize) -> GearParams {
        let g = &self.gears[i];
        let base = GearParams {
            module: self.module,
            pressure_angle: self.pressure_angle,
            teeth: g.teeth,
            // Opposite hands mesh; the stage stores the magnitude once.
            helix_angle: if i == 0 {
                self.helix_angle
            } else {
                -self.helix_angle
            },
            profile_shift: g.profile_shift.manual,
            addendum: g.addendum.manual,
            dedendum: g.dedendum,
            root_radius: g.root_radius,
            // k1 + k2 = 2 by construction, not by assertion.
            thickness_mod: if i == 0 {
                self.thickness_mod
            } else {
                2.0 - self.thickness_mod
            },
        };

        let x = g
            .profile_shift
            .resolve(automatic_profile_shift(&base, g.working_depth));
        let with_shift = GearParams {
            profile_shift: x,
            ..base
        };

        let addendum = if g.addendum.auto {
            addendum_for_tip_width(&Gear::new(with_shift), g.min_tip_width)
                .unwrap_or(with_shift.addendum)
        } else {
            g.addendum.manual
        };

        GearParams {
            addendum,
            ..with_shift
        }
    }
}

/// Solve one stage, given the torque on its first gear.
///
/// # Errors
///
/// [`TrainError`] when the pair cannot mesh, never reaches contact, names a
/// material the library does not have, or is too undercut to rate.
pub fn solve_stage(
    stage: &SpurStage,
    input_torque: f64,
    lib: &MaterialLibrary,
) -> Result<SpurResult, TrainError> {
    let p = [stage.params(0), stage.params(1)];
    let g = [Gear::new(p[0]), Gear::new(p[1])];
    let mesh = Mesh::new(&g[0], &g[1], MeshKind::External).map_err(TrainError::Mesh)?;
    let path = ContactPath::new(&g[0], g[1].ra, &mesh).ok_or(TrainError::NoContact)?;

    // Owned rather than borrowed, because a gear's own overrides may replace
    // properties of the library entry and the result is a different material.
    let materials: Vec<Material> = stage
        .gears
        .iter()
        .map(|s| {
            lib.get(&s.material)
                .ok_or_else(|| TrainError::UnknownMaterial(s.material.clone()))
                .map(|m| m.overridden(&s.material_overrides))
        })
        .collect::<Result<_, _>>()?;

    // --- centre distance. Clearance applies only in automatic mode, per spec.
    let (centre, clearance) = if stage.centre_distance.auto {
        (mesh.a_w + stage.clearance, stage.clearance)
    } else {
        (stage.centre_distance.manual, 0.0)
    };
    let _ = clearance;

    // --- face width. `b_min` does not depend on the `b` it was measured at
    // (DESIGN §4.7), so one evaluation at any width gives every minimum, and
    // nothing has to be iterated.
    const PROBE: f64 = 10.0;
    let e_star = contact_modulus(&materials[0], &materials[1]);
    let probe_load = Load::new(input_torque, PROBE);

    let sections = [
        bending_section(&g[0], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
        bending_section(&g[1], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
    ];
    let probe_contact = contact_stress(&path, &mesh, &g[0], PARALLEL_AXES, &probe_load, e_star)
        .ok_or(TrainError::NoContact)?;

    let mut widths = [0.0_f64; 2];
    for i in 0..2 {
        let load_i = probe_load.across_mesh(&g[0], &g[i]);
        let sf = bending_stress(&sections[i], &g[i], &load_i, StressConcentration::Iso6336);
        let allow = materials[i].fatigue_allowable.value;

        let mut want: f64 = 0.0;
        if stage.gears[i].auto_face_from_bending {
            if let Some(sf) = sf {
                want = want.max(min_face_width_bending(sf, PROBE, allow));
            }
        }
        if stage.gears[i].auto_face_from_contact {
            want = want.max(min_face_width_contact(probe_contact.worst, PROBE, allow));
        }
        widths[i] = stage.gears[i].face_width.resolve(want);
    }

    // The spec is explicit: the *narrower* gear carries the mesh, so both gears
    // are rated at the smaller width regardless of which one owns it.
    let effective = widths[0].min(widths[1]);

    let load = Load::new(input_torque, effective);
    let cs = contact_stress(&path, &mesh, &g[0], PARALLEL_AXES, &load, e_star)
        .ok_or(TrainError::NoContact)?;

    let mut gears = Vec::with_capacity(2);
    for i in 0..2 {
        let load_i = load.across_mesh(&g[0], &g[i]);
        let sf = bending_stress(&sections[i], &g[i], &load_i, StressConcentration::Iso6336);
        let allow = materials[i].fatigue_allowable.value;
        gears.push(GearResult {
            profile_shift: p[i].profile_shift,
            addendum: p[i].addendum,
            face_width: widths[i],
            torque: load_i.torque,
            // Filled in by `solve_train`, which is the only level that knows the
            // duty cycle and where this gear sits in the shaft line.
            speed: 0.0,
            tooth_cycles: 0.0,
            bending_stress: sf,
            contact_stress: cs.worst,
            min_face_width_bending: sf.map(|s| min_face_width_bending(s, effective, allow)),
            min_face_width_contact: min_face_width_contact(cs.worst, effective, allow),
            clamps: g[i].clamps.notes.clone(),
            material: materials[i].clone(),
            ranges: admissible_ranges(&p[i], stage.gears[i].working_depth),
        });
    }

    // --- contact ratios. eps_beta needs the face width, which is why it could
    // not exist before this milestone.
    let beta = stage.helix_angle.to_radians();
    let overlap = effective * beta.sin().abs() / (std::f64::consts::PI * stage.module);
    let contact_ratios = ContactRatios {
        transverse: path.contact_ratio,
        overlap,
        total: path.contact_ratio + overlap,
    };

    // --- backlash at the three centre distances.
    let angular =
        |a: f64, at: Member| -> f64 { mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees() };
    // Reported by direction rather than by member: the output of a forward
    // drive is gear 2, of a backward drive gear 1, and the same gap subtends a
    // different angle at each.
    let backlash = Directional::of(|d| {
        let at = match d {
            Drive::Forward => Member::Second,
            Drive::Backward => Member::First,
        };
        Backlash {
            nominal: angular(centre, at),
            minimum: angular(centre - stage.tolerance_minus, at),
            maximum: angular(centre + stage.tolerance_plus, at),
        }
    });

    let mut notes = Vec::new();
    if stage.helix_angle != 0.0 && !contact_ratios.has_full_axial_overlap() {
        notes.push(format!(
            "overlap ratio {overlap:.3} is below 1: the stage is helical in form \
             but still transfers load like a spur gear"
        ));
    }
    if path.contact_ratio < 1.0 {
        notes.push(format!(
            "transverse contact ratio {:.3} is below 1: the mesh loses contact between teeth",
            path.contact_ratio
        ));
    }

    Ok(SpurResult {
        ratio: f64::from(stage.gears[1].teeth) / f64::from(stage.gears[0].teeth),
        centre_distance_nominal: mesh.a_w,
        centre_distance: centre,
        contact_ratios,
        efficiency: Directional::of(|d| efficiency(&path, &mesh, &g[0], stage.friction, d)),
        backlash,
        coprime: gcd(stage.gears[0].teeth, stage.gears[1].teeth) == 1,
        gears: [gears[0].clone(), gears[1].clone()],
        notes,
    })
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}
