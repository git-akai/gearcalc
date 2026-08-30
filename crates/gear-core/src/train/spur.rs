//! The parallel-axis stage: spur when the helix angle is zero, helical
//! otherwise.
//!
//! Split out of `train.rs` when the worm stage arrived and gave the division
//! something to divide. What stays in the parent module is everything a stage
//! of *any* kind produces — [`StageResult`], [`GearResult`], [`Backlash`] — and
//! the train that strings them together.

use super::{
    allowable, Backlash, Case, ContactRatios, Cycles, GearResult, LoadCase, SpurResult,
    StageTorques, TrainError, Widths,
};
use crate::auto::{addendum_for_tip_width, admissible_ranges, automatic_profile_shift};
use crate::contact::{efficiency, ContactPath, Directional, Drive};
use crate::material::{contact_modulus, Material, MaterialLibrary, Overrides};
use crate::mesh::{Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::strength::{
    bending_section, bending_stress, contact_stress, min_face_width_bending,
    min_face_width_contact, Load, StressConcentration, PARALLEL_AXES,
};
use crate::tooth::Tooth;

/// One gear of a stage.
///
/// Note what is *absent*: module, pressure angle and helix angle live on the
/// stage, because they are shared.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct StageGear {
    pub teeth: u32,
    /// Automatic uses [`minimum_profile_shift`] at `working_depth`.
    pub profile_shift: Auto<f64>,
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
    /// Automatic uses [`addendum_for_tip_width`] at `min_tip_width`.
    pub addendum: Auto<f64>,
    /// Minimum transverse tooth tip width, mm.
    pub min_tip_width: f64,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Automatic takes the larger of the enabled minimums below.
    pub face_width: Auto<f64>,
    /// Which of the four ratings an automatic face width is sized from.
    pub face_sources: super::FaceSources,
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
            working_depth: Auto::automatic(1.0),
            addendum: Auto::fixed(1.0),
            min_tip_width: 0.1,
            dedendum: 1.25,
            root_radius: 0.38,
            face_width: Auto::fixed(10.0),
            face_sources: super::FaceSources::default(),
            material: "4340 Hardened Steel".to_string(),
            material_overrides: Overrides::default(),
        }
    }
}

/// A stage of two gears on shafts at any angle.
///
/// Spur when nothing is angled, helical when the teeth are, and a **crossed
/// gear pair** when the shafts are — one stage, as the specification has it,
/// with the shaft angle as the input that distinguishes them. It is not three
/// kinds of stage: the tooth counts, the module, the materials and the
/// tolerances mean the same thing throughout, and only the *mesh* differs.
///
/// # The two helix angles come from the shaft angle
///
/// ```text
/// β₁ = Σ/2 + β_add,     β₂ = Σ/2 − β_add
/// ```
///
/// so `β₁ + β₂ = Σ` — the relation crossed-axis screw gearing runs on (docs/reference.md#crossed-axes)
/// — and at `Σ = 0` it collapses to `β₁ = −β₂ = β_add`, a parallel helical pair
/// with its two hands opposed. The parallel case is the shaft angle's zero
/// rather than a separate construction, which is the specification's own
/// reading: "Total Helix Angle = 0.5 × Axis Angle + Additional Helix Angle".
///
/// What *does* branch is the mesh, and it must: parallel axes touch along a
/// line and lose power to sliding along the profile, while crossed axes touch
/// at a point and slide lengthwise. Those are different mechanisms with
/// different formulas and different results (docs/reference.md#crossed-axes), so a crossed stage
/// answers with the screw result — no contact ratio, no bending, two
/// efficiencies — and says so.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct SpurStage {
    /// Normal module, mm. Shared by both gears.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// Shaft angle `Σ`, degrees. **Zero is a parallel-axis pair**; anything else
    /// crosses the shafts.
    #[cfg_attr(feature = "serde", serde(default))]
    pub shaft_angle: f64,
    /// Additional helix angle, degrees — what each gear carries *beyond* half
    /// the shaft angle. Gear 2 takes it with the opposite sign, so at `Σ = 0`
    /// this is the familiar shared helix angle with opposed hands.
    pub additional_helix: f64,
    /// Coefficient of friction for the mesh.
    pub sliding_friction: f64,
    /// Coefficient of **static** friction, for breaking away.
    ///
    /// Whether a drive turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`] — the static figure's
    /// only job is the sign, and it is never itself reported as an efficiency.
    pub static_friction: f64,
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
            shaft_angle: 0.0,
            additional_helix: 0.0,
            sliding_friction: 0.06,
            static_friction: 0.16,
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
    /// The two gears' helix angles, degrees: `Σ/2 ± β_add`.
    ///
    /// One place to ask, so the pair cannot disagree about a shaft angle they
    /// share — and so `β₁ + β₂ = Σ` holds by construction rather than by a test.
    #[must_use]
    pub fn helix_angles(&self) -> [f64; 2] {
        let half = self.shaft_angle / 2.0;
        [half + self.additional_helix, half - self.additional_helix]
    }

    /// Whether the shafts cross. The parallel case is the zero of the shaft
    /// angle, and it is the *mesh* that differs, not the stage.
    #[must_use]
    pub fn is_crossed(&self) -> bool {
        self.shaft_angle != 0.0
    }

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
            // A stage member is concentric: the eccentric feature is the gear
            // tab's, and `..Default::default()` here would silently invent one
            // the day a stage grew the input.
            angular_shift: 0.0,
            index_offset: 0.0,
            module: self.module,
            pressure_angle: self.pressure_angle,
            teeth: g.teeth,
            // Opposite hands mesh; the stage stores the magnitude once.
            helix_angle: self.helix_angles()[i],
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

        let x = g.profile_shift.resolve(automatic_profile_shift(
            &base,
            g.working_depth.resolve(g.dedendum),
        ));
        let with_shift = GearParams {
            profile_shift: x,
            ..base
        };

        let addendum = if g.addendum.auto {
            addendum_for_tip_width(&Tooth::new(with_shift), g.min_tip_width)
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
pub fn solve_spur_stage(
    stage: &SpurStage,
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<SpurResult, TrainError> {
    let p = [stage.params(0), stage.params(1)];
    let g = [Tooth::new(p[0]), Tooth::new(p[1])];
    let mesh = Mesh::new(&g[0], &g[1], MeshKind::External).map_err(TrainError::Mesh)?;

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

    // --- the pair as it actually runs.
    //
    // `mesh` is the **zero-backlash** pair, which is where the profile shifts put
    // it; `operating` is that plus the assembly clearance, which is where the
    // teeth actually touch. Everything about contact — the path, the operating
    // radii, the relative curvature, the Hertz stress and the efficiency
    // integral — belongs to the second, and only backlash belongs to the first,
    // which measures play *against* the zero-backlash reference. Rating at
    // `mesh` was rating a pair nobody builds: the clearance is not a tolerance
    // to be ignored, it is the reason there is any backlash to report.
    //
    // A crossed stage reaches the same place by a different road (docs/reference.md#centre-distance-and-backlash): its line
    // of action slides instead of turning, so `Screw::path_of_contact_at` takes
    // the distance rather than the pair being re-described at it.
    let operating = mesh.at(centre).map_err(TrainError::Mesh)?;
    let path = ContactPath::new(&g[0], g[1].ra, &operating).ok_or(TrainError::NoContact)?;
    let _ = clearance;

    // --- face width. `b_min` does not depend on the `b` it was measured at
    // (docs/reference.md#contact-stress), so one evaluation at any width gives every minimum, and
    // nothing has to be iterated.
    const PROBE: f64 = 10.0;
    let e_star = contact_modulus(&materials[0], &materials[1]);

    let sections = [
        bending_section(&g[0], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
        bending_section(&g[1], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
    ];

    // Every rating at a probe width, one set per load case. `b_min` does not
    // depend on the `b` it was measured at, so this is still one evaluation per
    // case and nothing iterates.
    let probe = |case| -> Result<(f64, [Option<f64>; 2]), TrainError> {
        let load = Load::new(torques.at(case), PROBE);
        let cs = contact_stress(&path, &operating, &g[0], PARALLEL_AXES, &load, e_star)
            .ok_or(TrainError::NoContact)?;
        let sf = [0usize, 1].map(|i| {
            let li = load.across_mesh(&g[0], &g[i]);
            bending_stress(&sections[i], &g[i], &li, StressConcentration::Iso6336)
        });
        Ok((cs.worst, sf))
    };
    let probed = LoadCase {
        peak: probe(Case::Peak)?,
        cyclic: probe(Case::Cyclic)?,
    };

    let probe_widths = |i: usize| -> LoadCase<Widths> {
        LoadCase::of(|case| {
            let (cs, sf) = probed.get(case);
            let allow = allowable(&materials[i], case);
            Widths {
                bending: sf[i].map(|s| min_face_width_bending(s, PROBE, allow)),
                contact: min_face_width_contact(*cs, PROBE, allow),
            }
        })
    };

    let mut notes = Vec::new();
    let widths = [0usize, 1].map(|i| {
        let g = &stage.gears[i];
        // An automatic width with every source switched off has nothing to
        // invert, and comes out zero. Said rather than divided by: the input
        // that produced it is on screen, and this is what it did.
        if g.face_width.auto && !g.face_sources.any() {
            notes
                .push(Note::new(key::STAGE_FACE_WIDTH_NO_SOURCE).text("gear", (i + 1).to_string()));
        }
        g.face_width
            .resolve(g.face_sources.largest_of(&probe_widths(i)))
    });

    // The spec is explicit: the *narrower* gear carries the mesh, so both gears
    // are rated at the smaller width regardless of which one owns it.
    let effective = widths[0].min(widths[1]);

    // Every rating again, at the width actually in force. Two evaluations
    // rather than one, and the same expression: a load case is a torque, and
    // nothing else about the stage knows which one it is looking at.
    let rate = |case: Case| -> Result<(crate::strength::ContactStress, [Option<f64>; 2], Load), TrainError> {
        let load = Load::new(torques.at(case), effective);
        let cs = contact_stress(&path, &operating, &g[0], PARALLEL_AXES, &load, e_star)
            .ok_or(TrainError::NoContact)?;
        let sf = [0usize, 1].map(|i| {
            let li = load.across_mesh(&g[0], &g[i]);
            bending_stress(&sections[i], &g[i], &li, StressConcentration::Iso6336)
        });
        Ok((cs, sf, load))
    };
    let rated = LoadCase {
        peak: rate(Case::Peak)?,
        cyclic: rate(Case::Cyclic)?,
    };
    let load = rated.peak.2;

    let mut gears = Vec::with_capacity(2);
    for i in 0..2 {
        let load_i = load.across_mesh(&g[0], &g[i]);
        gears.push(GearResult {
            profile_shift: p[i].profile_shift,
            addendum: p[i].addendum,
            face_width: widths[i],
            torque: load_i.torque,
            back_driving_torque: torques
                .peak_backward
                .map(|t| Load::new(t, effective).across_mesh(&g[0], &g[i]).torque),
            // Filled in by `solve_train`, which is the only level that knows the
            // duty cycle and where this gear sits in the shaft line.
            speed: 0.0,
            tooth_cycles: Cycles {
                bending: 0.0,
                contact: 0.0,
            },
            bending_stress: LoadCase {
                peak: rated.peak.1[i],
                cyclic: rated.cyclic.1[i],
            },
            contact_stress: LoadCase {
                peak: rated.peak.0.worst,
                cyclic: rated.cyclic.0.worst,
            },
            min_face_width: LoadCase::of(|case| {
                let (cs, sf, _) = rated.get(case);
                let allow = allowable(&materials[i], case);
                Widths {
                    bending: sf[i].map(|s| min_face_width_bending(s, effective, allow)),
                    contact: min_face_width_contact(cs.worst, effective, allow),
                }
            }),
            clamps: g[i].clamps.notes.clone(),
            material: materials[i].clone(),
            ranges: admissible_ranges(
                &p[i],
                stage.gears[i]
                    .working_depth
                    .resolve(stage.gears[i].dedendum),
            ),
        });
    }

    // --- contact ratios. eps_beta needs the face width, which is why it could
    // not exist before this milestone.
    let beta = stage.additional_helix.to_radians();
    let overlap = effective * beta.sin().abs() / (std::f64::consts::PI * stage.module);
    let contact_ratios = ContactRatios {
        transverse: path.contact_ratio,
        overlap,
        total: path.contact_ratio + overlap,
    };

    // --- backlash at the three centre distances.
    let angular =
        |a: f64, at: MeshSide| -> f64 { mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees() };
    // Reported by direction rather than by member: the output of a forward
    // drive is gear 2, of a backward drive gear 1, and the same gap subtends a
    // different angle at each.
    let backlash = Directional::of(|d| {
        let at = match d {
            Drive::Forward => MeshSide::Second,
            Drive::Backward => MeshSide::First,
        };
        Backlash {
            nominal: angular(centre, at),
            minimum: angular(centre - stage.tolerance_minus, at),
            maximum: angular(centre + stage.tolerance_plus, at),
        }
    });

    if stage.additional_helix != 0.0 && !contact_ratios.has_full_axial_overlap() {
        notes.push(Note::new(key::STAGE_OVERLAP_BELOW_ONE).number("ratio", overlap, 3));
    }
    if path.contact_ratio < 1.0 {
        notes.push(
            Note::new(key::STAGE_TRANSVERSE_CONTACT_RATIO_BELOW_ONE).number(
                "ratio",
                path.contact_ratio,
                3,
            ),
        );
    }

    Ok(SpurResult {
        ratio: f64::from(stage.gears[1].teeth) / f64::from(stage.gears[0].teeth),
        centre_distance_nominal: mesh.a_w,
        centre_distance: centre,
        contact_ratios,
        // Breaking away is decided at rest, running is decided sliding — one
        // rule, applied to every stage kind (`Directional::once_moving`). A
        // parallel-axis mesh is never near the threshold, so this passes the
        // sliding figure through and always will; it is here so there is no
        // stage kind the rule has to be remembered for.
        efficiency: {
            let with = |mu: f64| Directional::of(|d| efficiency(&path, &operating, &g[0], mu, d));
            with(stage.sliding_friction).once_moving(&with(stage.static_friction))
        },
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
