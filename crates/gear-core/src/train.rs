//! Geartrains: a stage at a time, and the accumulation along the shaft line.
//!
//! Only the parallel-axis spur/helical stage exists so far. Worm and planetary
//! stages get their own modules when they arrive, at which point this file
//! splits; the accumulation in [`solve_train`] is written against a trait-free
//! enum so that adding a stage kind is additive rather than a rewrite.
//!
//! # What is state and what is not
//!
//! Per `docs/DESIGN.md` §3.1 the input structs here are the *only* state. Every
//! result is recomputed from them, so nothing can go stale. Two consequences are
//! visible in the shapes below:
//!
//! - Values shared across a stage — normal module, pressure angle, helix angle —
//!   are stored **once on the stage**, not per gear, so the two cannot disagree
//!   (§3.2).
//! - Tooth thickness modification is stored as `k₁` alone, with `k₂ = 2 − k₁`
//!   derived, because a meshing pair must sum to 2. The invariant is unwritable
//!   rather than merely tested.

use crate::auto::{addendum_for_tip_width, automatic_profile_shift};
use crate::contact::{efficiency, ContactPath};
use crate::material::{contact_modulus, MaterialLibrary};
use crate::mesh::{Member, Mesh, MeshError, MeshKind};
use crate::params::{Auto, GearParams};
use crate::profile::Gear;
use crate::strength::{
    bending_section, bending_stress, contact_stress, min_face_width_bending,
    min_face_width_contact, Load, StressConcentration,
};

/// One gear of a stage.
///
/// Note what is *absent*: module, pressure angle and helix angle live on the
/// stage, because they are shared.
#[derive(Clone, Debug)]
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
        }
    }
}

/// A parallel-axis stage: spur when the helix angle is zero, helical otherwise.
///
/// Crossed axes are milestone 10 and are not accepted here; the specification's
/// "Axis Angle" input belongs with that work.
#[derive(Clone, Debug)]
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

/// The three contact ratios.
#[derive(Clone, Copy, Debug)]
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
pub struct Backlash {
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
}

/// What a stage does to one of its gears.
#[derive(Clone, Debug)]
pub struct GearResult {
    /// The shift in force, after any automatic calculation.
    pub profile_shift: f64,
    /// Likewise the addendum.
    pub addendum: f64,
    /// Likewise the face width.
    pub face_width: f64,
    /// Torque on this gear, N·m.
    pub torque: f64,
    /// Tooth root bending stress, MPa. `None` where the stress correction is
    /// undefined for this section — see [`crate::strength::bending_stress`].
    pub bending_stress: Option<f64>,
    /// Hertzian contact stress, MPa. Shared by the pair, so both gears report
    /// the same number; the *allowables* they are judged against differ.
    pub contact_stress: f64,
    /// Face width the bending stress would need against the fatigue allowable.
    pub min_face_width_bending: Option<f64>,
    /// ...and against contact.
    pub min_face_width_contact: f64,
    /// Guards that altered this gear's geometry.
    pub clamps: Vec<String>,
}

/// Everything one stage produces.
#[derive(Clone, Debug)]
pub struct StageResult {
    /// `z₂ / z₁`.
    pub ratio: f64,
    /// Zero-backlash centre distance, mm.
    pub centre_distance_nominal: f64,
    /// The centre distance actually used, including clearance.
    pub centre_distance: f64,
    pub contact_ratios: ContactRatios,
    /// Mesh efficiency, 0..1. Equal in both directions for a parallel-axis
    /// stage — see [`crate::contact::efficiency`].
    pub efficiency: f64,
    /// Angular backlash referred to each gear.
    pub backlash: [Backlash; 2],
    /// Whether the tooth counts share no factor — a hunting ratio, which spreads
    /// wear evenly instead of repeatedly pairing the same teeth.
    pub coprime: bool,
    pub gears: [GearResult; 2],
    /// Anything the stage had to say about the design.
    pub notes: Vec<String>,
}

/// Why a stage could not be solved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrainError {
    /// The pair cannot mesh, or the shifts put it outside the involute domain.
    Mesh(MeshError),
    /// No usable path of contact — the teeth do not reach each other.
    NoContact,
    /// A material name that is not in the library.
    UnknownMaterial(String),
    /// A tooth so undercut there is no root section left to rate.
    NoRootSection,
    /// The train has no stages, so there is nothing to accumulate.
    Empty,
}

impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mesh(e) => write!(f, "{e}"),
            Self::NoContact => write!(f, "the teeth never come into contact"),
            Self::UnknownMaterial(n) => write!(f, "no material named {n:?} in the library"),
            Self::NoRootSection => write!(f, "the tooth is too undercut to have a root section"),
            Self::Empty => write!(f, "the geartrain has no stages"),
        }
    }
}

impl std::error::Error for TrainError {}

impl SpurStage {
    /// `GearParams` for one gear, with the automatic values resolved.
    ///
    /// The two automatic calculations are ordered, and the order matters: the
    /// shift is chosen first because the addendum solve needs `ψ_b`, which
    /// depends on it. The reverse dependency does not exist — `minimum_profile_shift`
    /// touches only `r`, `α_t` and the cutter, none of which the addendum moves —
    /// so there is no loop to iterate.
    fn params(&self, i: usize) -> GearParams {
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
) -> Result<StageResult, TrainError> {
    let p = [stage.params(0), stage.params(1)];
    let g = [Gear::new(p[0]), Gear::new(p[1])];
    let mesh = Mesh::new(&g[0], &g[1], MeshKind::External).map_err(TrainError::Mesh)?;
    let path = ContactPath::new(&g[0], &g[1], &mesh).ok_or(TrainError::NoContact)?;

    let materials: Vec<&crate::Material> = stage
        .gears
        .iter()
        .map(|s| {
            lib.get(&s.material)
                .ok_or_else(|| TrainError::UnknownMaterial(s.material.clone()))
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
    let e_star = contact_modulus(materials[0], materials[1]);
    let probe_load = Load::new(input_torque, PROBE);

    let sections = [
        bending_section(&g[0], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
        bending_section(&g[1], path.contact_ratio).ok_or(TrainError::NoRootSection)?,
    ];
    let probe_contact =
        contact_stress(&path, &mesh, &g[0], &probe_load, e_star).ok_or(TrainError::NoContact)?;

    let mut widths = [0.0_f64; 2];
    for i in 0..2 {
        let load_i = probe_load.across_mesh(&g[0], &g[i]);
        let sf = bending_stress(&sections[i], &g[i], &load_i, StressConcentration::Iso6336);
        let allow = materials[i].fatigue_allowable.get();

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
    let cs = contact_stress(&path, &mesh, &g[0], &load, e_star).ok_or(TrainError::NoContact)?;

    let mut gears = Vec::with_capacity(2);
    for i in 0..2 {
        let load_i = load.across_mesh(&g[0], &g[i]);
        let sf = bending_stress(&sections[i], &g[i], &load_i, StressConcentration::Iso6336);
        let allow = materials[i].fatigue_allowable.get();
        gears.push(GearResult {
            profile_shift: p[i].profile_shift,
            addendum: p[i].addendum,
            face_width: widths[i],
            torque: load_i.torque,
            bending_stress: sf,
            contact_stress: cs.worst,
            min_face_width_bending: sf.map(|s| min_face_width_bending(s, effective, allow)),
            min_face_width_contact: min_face_width_contact(cs.worst, effective, allow),
            clamps: g[i].clamps.notes.clone(),
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
    let backlash = [Member::First, Member::Second].map(|m| Backlash {
        nominal: angular(centre, m),
        minimum: angular(centre - stage.tolerance_minus, m),
        maximum: angular(centre + stage.tolerance_plus, m),
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

    Ok(StageResult {
        ratio: f64::from(stage.gears[1].teeth) / f64::from(stage.gears[0].teeth),
        centre_distance_nominal: mesh.a_w,
        centre_distance: centre,
        contact_ratios,
        efficiency: efficiency(&path, &mesh, &g[0], stage.friction),
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

/// A whole geartrain.
#[derive(Clone, Debug)]
pub struct Train {
    /// Peak input speed, rpm.
    pub input_speed: f64,
    /// Peak input torque, N·m.
    pub input_torque: f64,
    pub stages: Vec<SpurStage>,
}

/// What a train produces.
#[derive(Clone, Debug)]
pub struct TrainResult {
    /// Product of the stage ratios.
    pub total_ratio: f64,
    /// Output speed, rpm — an *output*, per Q1.
    pub output_speed: f64,
    /// Output torque, N·m, after efficiency losses.
    pub output_torque: f64,
    /// Product of the stage efficiencies.
    pub total_efficiency: f64,
    /// Angular backlash referred to the **output** shaft, degrees.
    pub output_backlash: Backlash,
    pub stages: Vec<StageResult>,
}

/// Solve a whole train, propagating torque and accumulating backlash.
///
/// # Torque
///
/// `T_{k+1} = T_k · i_k · η_k`. Efficiency always *reduces* delivered torque,
/// whichever way the train is driven — that is the sign convention it is easy to
/// get wrong, so it is stated here and tested.
///
/// # Backlash
///
/// Referred to the output shaft, each stage's contribution is divided by the
/// ratio of everything downstream of it:
///
/// ```text
/// θ_out = Σ_k  j_θ,k / Π_{j>k} i_j
/// ```
///
/// The consequence worth surfacing: the **last** stage dominates, and backlash in
/// the first stage is nearly free. A train designed for low output backlash
/// should spend its tolerance budget at the output end.
///
/// # Errors
///
/// [`TrainError::Empty`] for a train with no stages, or whatever the first
/// failing stage reports.
pub fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<TrainResult, TrainError> {
    if train.stages.is_empty() {
        return Err(TrainError::Empty);
    }

    let mut torque = train.input_torque;
    let mut stages = Vec::with_capacity(train.stages.len());
    for stage in &train.stages {
        let r = solve_stage(stage, torque, lib)?;
        torque = torque * r.ratio * r.efficiency;
        stages.push(r);
    }

    let total_ratio: f64 = stages.iter().map(|s| s.ratio).product();
    let total_efficiency: f64 = stages.iter().map(|s| s.efficiency).product();

    // Each stage's backlash, seen from the output, divided by everything after it.
    let refer = |pick: fn(&Backlash) -> f64| {
        stages
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let downstream: f64 = stages[k + 1..].iter().map(|d| d.ratio).product();
                pick(&s.backlash[1]) / downstream
            })
            .sum()
    };

    Ok(TrainResult {
        total_ratio,
        output_speed: train.input_speed / total_ratio,
        output_torque: torque,
        total_efficiency,
        output_backlash: Backlash {
            nominal: refer(|b| b.nominal),
            minimum: refer(|b| b.minimum),
            maximum: refer(|b| b.maximum),
        },
        stages,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn library() -> MaterialLibrary {
        // A self-contained stand-in so gear-core keeps no dependency on gear-io.
        use crate::material::{Basis, Class, Material, Measure, Value};
        MaterialLibrary {
            materials: vec![Material {
                name: "4340 Hardened Steel".into(),
                class: Class::Steel,
                grade: "test".into(),
                condition: "test".into(),
                source: "test".into(),
                density: Value::datasheet(7850.0),
                elastic_modulus: Value::datasheet(190_000.0),
                poissons_ratio: Value::datasheet(0.29),
                ultimate_allowable: Value::datasheet(1365.0),
                ultimate_measure: Measure::Yield,
                fatigue_allowable: Value {
                    dry: 750.0,
                    conditioned: None,
                    basis: Basis::Estimated,
                    note: Some("test".into()),
                },
            }],
        }
    }

    fn two_stage() -> Train {
        Train {
            input_speed: 3000.0,
            input_torque: 2.0,
            stages: vec![
                SpurStage::default(),
                SpurStage {
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
                    ..SpurStage::default()
                },
            ],
        }
    }

    #[test]
    fn a_two_stage_train_computes_end_to_end() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        assert_eq!(r.stages.len(), 2);

        // 43/17 * 31/13
        let want = (43.0 / 17.0) * (31.0 / 13.0);
        assert!((r.total_ratio - want).abs() < 1e-12);
        assert!((r.output_speed - 3000.0 / want).abs() < 1e-9);

        // Every stage produced real numbers.
        for s in &r.stages {
            assert!(s.centre_distance > 0.0);
            assert!(s.contact_ratios.transverse > 1.0);
            assert!(s.efficiency > 0.9 && s.efficiency < 1.0);
            for g in &s.gears {
                assert!(g.face_width > 0.0);
                assert!(g.contact_stress > 0.0);
                assert!(g.bending_stress.unwrap() > 0.0);
            }
        }
    }

    /// Efficiency must always *reduce* delivered torque. Getting this sign wrong
    /// is the classic train-accumulation bug, and it hides because the ratio term
    /// is so much larger.
    #[test]
    fn efficiency_always_costs_torque() {
        let lib = library();
        let mut lossless = two_stage();
        for s in &mut lossless.stages {
            s.friction = 0.0;
        }
        let ideal = solve_train(&lossless, &lib).unwrap();
        let real = solve_train(&two_stage(), &lib).unwrap();

        assert!((ideal.total_efficiency - 1.0).abs() < 1e-12);
        assert!(real.output_torque < ideal.output_torque);
        // ...and the shortfall is exactly the product of the stage efficiencies.
        assert!((real.output_torque - ideal.output_torque * real.total_efficiency).abs() < 1e-9);
    }

    #[test]
    fn a_spur_stage_has_exactly_zero_overlap_and_a_helical_one_does_not() {
        let lib = library();
        let spur = solve_stage(&SpurStage::default(), 2.0, &lib).unwrap();
        assert_eq!(spur.contact_ratios.overlap, 0.0, "must be exactly zero");
        assert_eq!(spur.contact_ratios.total, spur.contact_ratios.transverse);
        assert!(!spur.contact_ratios.has_full_axial_overlap());

        let helical = solve_stage(
            &SpurStage {
                helix_angle: 20.0,
                ..SpurStage::default()
            },
            2.0,
            &lib,
        )
        .unwrap();
        assert!(helical.contact_ratios.overlap > 0.0);
        assert!(helical.contact_ratios.total > helical.contact_ratios.transverse);
    }

    /// The last stage dominates output backlash, which is the design consequence
    /// worth surfacing: tolerance spent at the input end is nearly free.
    #[test]
    fn backlash_referred_to_the_output_is_dominated_by_the_last_stage() {
        let lib = library();
        let base = two_stage();

        let loosen = |k: usize| {
            let mut t = base.clone();
            t.stages[k].clearance *= 4.0;
            solve_train(&t, &lib).unwrap().output_backlash.nominal
        };

        let reference = solve_train(&base, &lib).unwrap().output_backlash.nominal;
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
        let stage = SpurStage {
            thickness_mod: 1.3,
            ..SpurStage::default()
        };
        let k: Vec<f64> = (0..2).map(|i| stage.params(i).thickness_mod).collect();
        assert!((k[0] + k[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_automatic_face_width_is_the_larger_of_the_enabled_checks() {
        let lib = library();
        let auto_both = |bending: bool, contact: bool| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.auto_face_from_bending = bending;
                g.auto_face_from_contact = contact;
            }
            solve_stage(&s, 2.0, &lib).unwrap().gears[0].face_width
        };
        let b = auto_both(true, false);
        let c = auto_both(false, true);
        let both = auto_both(true, true);
        assert!((both - b.max(c)).abs() < 1e-9, "{both} vs max({b}, {c})");
        // Contact governs a lightly loaded steel pair, as it usually does.
        assert!(c > b);
    }

    #[test]
    fn an_unknown_material_is_named_rather_than_swallowed() {
        let mut s = SpurStage::default();
        s.gears[0].material = "unobtainium".into();
        let e = solve_stage(&s, 2.0, &library()).unwrap_err();
        assert!(matches!(e, TrainError::UnknownMaterial(ref n) if n == "unobtainium"));
        assert!(e.to_string().contains("unobtainium"));
    }

    #[test]
    fn an_empty_train_says_so() {
        let t = Train {
            input_speed: 1.0,
            input_torque: 1.0,
            stages: vec![],
        };
        assert_eq!(solve_train(&t, &library()).unwrap_err(), TrainError::Empty);
    }
}
