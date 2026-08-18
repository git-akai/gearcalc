//! Geartrains: a stage at a time, and the accumulation along the shaft line.
//!
//! Each stage kind has its own module — `spur` and `worm` — and what stays here
//! is the vocabulary they share ([`Backlash`], [`TrainError`], the duty cycle)
//! and the train that strings them together.
//!
//! **Each kind keeps its own result type**, and that was a decision rather than
//! an oversight. A worm stage has no bending stress, no minimum face width from
//! contact — a point contact does not care how wide the tooth is — and two
//! efficiencies rather than one. Forcing that into [`StageResult`] would have
//! meant four `Option`s and a comment apologising for each. A result shaped like
//! the answer says the same thing without the apology.
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

use crate::auto::Ranges;
use crate::contact::{Directional, Drive};
use crate::material::{Material, MaterialLibrary};
use crate::mesh::MeshError;

mod spur;
mod worm;

pub use spur::{solve_stage, SpurStage, StageGear};
pub use worm::{
    solve_worm_stage, WormContact, WormMember, WormMemberResult, WormResult, WormStage,
};

/// The three contact ratios.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
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
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Backlash {
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
}

/// What a stage does to one of its gears.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct GearResult {
    /// The shift in force, after any automatic calculation.
    pub profile_shift: f64,
    /// Likewise the addendum.
    pub addendum: f64,
    /// Likewise the face width.
    pub face_width: f64,
    /// Torque on this gear, N·m.
    pub torque: f64,
    /// Rotational speed, rpm.
    pub speed: f64,
    /// Tooth load cycles over the duty the train describes.
    ///
    /// One cycle per revolution for a simple gear: a given tooth meets the mate
    /// once per turn. Sun and ring gears in a planetary stage see `N_planets`
    /// per revolution, and a planet is a special case again — DESIGN.md §4.9.
    pub tooth_cycles: f64,
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
    /// The material as used, after any overrides — what the numbers were
    /// actually computed from, rather than what the library holds.
    pub material: Material,
    /// What this gear's geometry allows its own inputs to be.
    ///
    /// Computed from the **resolved** parameters, so an automatic profile shift
    /// or addendum is already folded in. The UI bounds its fields by these
    /// rather than by constants — see `docs/DESIGN.md` §4.3.1.
    pub ranges: Ranges,
}

/// Everything a parallel-axis stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct SpurResult {
    /// `z₂ / z₁`.
    pub ratio: f64,
    /// Zero-backlash centre distance, mm.
    pub centre_distance_nominal: f64,
    /// The centre distance actually used, including clearance.
    pub centre_distance: f64,
    pub contact_ratios: ContactRatios,
    /// Mesh efficiency, 0..1, in both drive directions.
    ///
    /// The two are equal for a parallel-axis stage, and they are computed
    /// independently rather than copied — see
    /// [`crate::contact::efficiency`] for why they come out that way.
    pub efficiency: Directional<f64>,
    /// Angular backlash at whichever gear is the output in each direction,
    /// degrees: gear 2 driving forward, gear 1 driving backward. The same tooth
    /// gap subtends a different angle at each, so these differ whenever the
    /// tooth counts do.
    pub backlash: Directional<Backlash>,
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
    /// The screw pair cannot exist — see [`crate::screw::ScrewError`].
    Screw(crate::screw::ScrewError),
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
            Self::Screw(e) => match e {
                crate::screw::ScrewError::NotPositive => {
                    write!(f, "a module, diameter or tooth count is not positive")
                }
                crate::screw::ScrewError::WormTooThin => write!(
                    f,
                    "the worm is too thin for that many starts at that module: \
                     the thread would have to wrap at ninety degrees or more"
                ),
                crate::screw::ScrewError::ShaftAngleImpossible => {
                    write!(f, "that shaft angle leaves the wheel with no lead angle")
                }
                crate::screw::ScrewError::AxesAreParallel => write!(
                    f,
                    "parallel axes: a worm stage needs crossed shafts, and a \
                     parallel pair is a spur stage"
                ),
            },
            Self::NoContact => write!(f, "the teeth never come into contact"),
            Self::UnknownMaterial(n) => write!(f, "no material named {n:?} in the library"),
            Self::NoRootSection => write!(f, "the tooth is too undercut to have a root section"),
            Self::Empty => write!(f, "the geartrain has no stages"),
        }
    }
}

impl std::error::Error for TrainError {}

/// A self-contained material library for tests, so `gear-core` keeps no
/// dependency on `gear-io`. Shared by every stage kind's tests.
#[cfg(test)]
pub(super) fn test_library() -> MaterialLibrary {
    use crate::material::{Basis, Class, Measure, Value};
    let steel = Material {
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
            value: 750.0,
            basis: Basis::Estimated,
            note: Some("test".into()),
        },
    };
    let bronze = Material {
        name: "Brass C360".into(),
        class: Class::Brass,
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

/// A stage of a geartrain, of whichever kind.
///
/// Serialised with a `kind` tag alongside the stage's own fields, so a train
/// file says what each stage is rather than relying on position.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum Stage {
    Spur(SpurStage),
    Worm(WormStage),
}

impl Default for Stage {
    fn default() -> Self {
        Self::Spur(SpurStage::default())
    }
}

/// What a stage produced, of whichever kind.
///
/// **Each kind keeps its own shape.** A worm stage has no bending stress, no
/// minimum face width from contact and two efficiencies; a spur stage has all
/// three and one. What the train needs from either is small enough to read
/// through the accessors below — ratio, efficiency, the backlash at the output
/// member — so the accumulation never asks what kind it was, without every
/// result having to pretend to be the same shape.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum StageResult {
    // Both variants are boxed. A result carries material records, admissible
    // ranges and notes, so the kinds differ in size by an order of magnitude and
    // will keep doing so as more arrive; a `Vec<StageResult>` would otherwise
    // pay the largest of them for every stage whatever its kind. The boxes are
    // invisible to readers and to serde.
    Spur(Box<SpurResult>),
    Worm(Box<WormResult>),
}

impl StageResult {
    /// `z₂/z₁`.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Spur(r) => r.ratio,
            Self::Worm(r) => r.ratio,
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
            Self::Spur(r) => r.efficiency,
            Self::Worm(r) => r.efficiency,
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
            Self::Spur(r) => r.backlash,
            Self::Worm(r) => r.backlash,
        }
    }

    /// The parallel-axis result, if that is what this is.
    #[must_use]
    pub fn as_spur(&self) -> Option<&SpurResult> {
        match self {
            Self::Spur(r) => Some(r),
            Self::Worm(_) => None,
        }
    }

    /// The worm result, if that is what this is.
    #[must_use]
    pub fn as_worm(&self) -> Option<&WormResult> {
        match self {
            Self::Worm(r) => Some(r),
            Self::Spur(_) => None,
        }
    }

    /// Write in the speeds and cycles, which only the whole shaft line knows.
    ///
    /// A worm's "tooth cycles" are revolutions: its thread is engaged
    /// continuously rather than meeting a mate once per turn, so the count is
    /// the same arithmetic but means something looser. It is reported because a
    /// duty cycle has to be reported somewhere, not because a worm thread has a
    /// fatigue life this crate can rate.
    fn set_kinematics(&mut self, speeds: [f64; 2], cycles: [f64; 2]) {
        match self {
            Self::Spur(r) => {
                for (i, g) in r.gears.iter_mut().enumerate() {
                    g.speed = speeds[i];
                    g.tooth_cycles = cycles[i];
                }
            }
            Self::Worm(r) => {
                for (i, m) in r.members.iter_mut().enumerate() {
                    m.speed = speeds[i];
                    m.tooth_cycles = cycles[i];
                }
                // Sliding needs a shaft speed, so it could only be filled here.
                r.sliding_velocity = r.sliding_ratio
                    * (speeds[0] / 60.0 * std::f64::consts::TAU)
                    * (r.members[0].pitch_diameter / 2.0);
            }
        }
    }
}

/// Solve one stage of whichever kind, given the torque on its input member.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any(
    stage: &Stage,
    input_torque: f64,
    lib: &MaterialLibrary,
) -> Result<StageResult, TrainError> {
    match stage {
        Stage::Spur(s) => solve_stage(s, input_torque, lib).map(|r| StageResult::Spur(Box::new(r))),
        Stage::Worm(s) => {
            solve_worm_stage(s, input_torque, lib).map(|r| StageResult::Worm(Box::new(r)))
        }
    }
}

/// How the train is used, which is what turns a ratio into a tooth count.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Actuation {
    /// A limited sweep, repeated. The range is measured **at the output**, so
    /// every gear's revolutions are worked *backwards* from there — upstream
    /// gears turn further, not less.
    Intermittent {
        /// Output sweep per actuation, degrees.
        range_degrees: f64,
        actuations: u32,
    },
    /// Continuous running at a fraction of peak speed.
    Continuous {
        /// Percentage of the peak input speed.
        operating_percent: f64,
        runtime_hours: f64,
    },
}

impl Default for Actuation {
    fn default() -> Self {
        Self::Intermittent {
            range_degrees: 25.0,
            actuations: 1000,
        }
    }
}

/// A whole geartrain.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Train {
    /// Peak input speed, rpm.
    pub input_speed: f64,
    /// Peak input torque, N·m.
    pub input_torque: f64,
    pub actuation: Actuation,
    pub stages: Vec<Stage>,
}

/// What a train produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct TrainResult {
    /// Product of the stage ratios.
    pub total_ratio: f64,
    /// Output speed, rpm — an *output*, per Q1.
    pub output_speed: f64,
    /// Output torque, N·m, after efficiency losses.
    pub output_torque: f64,
    /// Product of the stage efficiencies, in both drive directions.
    ///
    /// A train containing a self-locking stage cannot be back-driven at all, and
    /// [`Directional::self_locking`] on this pair says so.
    pub total_efficiency: Directional<f64>,
    /// Angular backlash referred to whichever shaft is the output, degrees: the
    /// last shaft driving forward, the first driving backward.
    pub backlash: Directional<Backlash>,
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
        let r = solve_any(stage, torque, lib)?;
        // Forward is the direction a train propagates torque in; the backward
        // figure is reported, not applied.
        torque = torque * r.ratio() * r.efficiency().forward;
        stages.push(r);
    }

    let total_ratio: f64 = stages.iter().map(StageResult::ratio).product();
    let total_efficiency = Directional::of(|d| {
        stages
            .iter()
            .map(|s| *s.efficiency().get(d))
            .product::<f64>()
    });

    // --- speeds and tooth cycles, which need the whole shaft line. None of this
    // asks what kind of stage it is looking at.
    let ratios: Vec<f64> = stages.iter().map(StageResult::ratio).collect();
    for (k, s) in stages.iter_mut().enumerate() {
        let upstream: f64 = ratios[..k].iter().product();
        let speed_in = train.input_speed / upstream;
        let speeds = [speed_in, speed_in / ratios[k]];

        // The reduction between each member and the output. Member 0 of a stage
        // sits before that stage's own mesh, member 1 after it.
        let cycles = [0usize, 1].map(|i| {
            let to_output: f64 = if i == 0 {
                ratios[k..].iter().product()
            } else {
                ratios[k + 1..].iter().product()
            };
            match train.actuation {
                Actuation::Intermittent {
                    range_degrees,
                    actuations,
                } => (range_degrees / 360.0) * to_output * f64::from(actuations),
                Actuation::Continuous {
                    operating_percent,
                    runtime_hours,
                } => speeds[i] * 60.0 * runtime_hours * operating_percent / 100.0,
            }
        });
        s.set_kinematics(speeds, cycles);
    }

    // Each stage's backlash, referred to whichever shaft is the output.
    //
    // Driven forward that is the last shaft, so a stage's contribution is
    // divided by everything downstream of it. Driven backward the *input* shaft
    // is the output, so the contribution is multiplied by everything upstream
    // instead: those shafts turn faster, and the same play is a larger angle
    // there. The last stage dominates either way, and by more going backward.
    let refer = |drive: Drive, pick: fn(&Backlash) -> f64| -> f64 {
        stages
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let stage = pick(s.backlash().get(drive));
                match drive {
                    Drive::Forward => {
                        let downstream: f64 =
                            stages[k + 1..].iter().map(StageResult::ratio).product();
                        stage / downstream
                    }
                    Drive::Backward => {
                        let upstream: f64 = stages[..k].iter().map(StageResult::ratio).product();
                        stage * upstream
                    }
                }
            })
            .sum()
    };

    Ok(TrainResult {
        total_ratio,
        output_speed: train.input_speed / total_ratio,
        output_torque: torque,
        total_efficiency,
        backlash: Directional::of(|d| Backlash {
            nominal: refer(d, |b| b.nominal),
            minimum: refer(d, |b| b.minimum),
            maximum: refer(d, |b| b.maximum),
        }),
        stages,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::material::Overrides;
    use crate::params::Auto;
    use crate::profile::Gear;

    fn library() -> MaterialLibrary {
        super::test_library()
    }

    /// These tests build spur trains, so they know the kind and say so once.
    fn spur(r: &StageResult) -> &SpurResult {
        r.as_spur().expect("this train's stages are all spur")
    }

    /// ...and the same for reaching into a stage's inputs.
    fn spur_input(s: &mut Stage) -> &mut SpurStage {
        match s {
            Stage::Spur(st) => st,
            Stage::Worm(_) => panic!("this train's stages are all spur"),
        }
    }

    fn two_stage() -> Train {
        Train {
            input_speed: 3000.0,
            input_torque: 2.0,
            actuation: Actuation::default(),
            stages: vec![
                Stage::Spur(SpurStage::default()),
                Stage::Spur(SpurStage {
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
                }),
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
        for s in r.stages.iter().map(spur) {
            assert!(s.centre_distance > 0.0);
            assert!(s.contact_ratios.transverse > 1.0);
            assert!(s.efficiency.forward > 0.9 && s.efficiency.forward < 1.0);
            assert_eq!(
                s.efficiency.forward, s.efficiency.backward,
                "a parallel-axis stage is as efficient driven either way"
            );
            for g in &s.gears {
                assert!(g.face_width > 0.0);
                assert!(g.contact_stress > 0.0);
                assert!(g.bending_stress.unwrap() > 0.0);
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
        assert!(!r.total_efficiency.self_locking());
    }

    /// Efficiency must always *reduce* delivered torque. Getting this sign wrong
    /// is the classic train-accumulation bug, and it hides because the ratio term
    /// is so much larger.
    #[test]
    fn efficiency_always_costs_torque() {
        let lib = library();
        let mut lossless = two_stage();
        for s in &mut lossless.stages {
            spur_input(s).friction = 0.0;
        }
        let ideal = solve_train(&lossless, &lib).unwrap();
        let real = solve_train(&two_stage(), &lib).unwrap();

        assert!((ideal.total_efficiency.forward - 1.0).abs() < 1e-12);
        assert!(real.output_torque < ideal.output_torque);
        // ...and the shortfall is exactly the product of the stage efficiencies.
        assert!(
            (real.output_torque - ideal.output_torque * real.total_efficiency.forward).abs() < 1e-9
        );
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
            spur_input(&mut t.stages[k]).clearance *= 4.0;
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

    /// Intermittent duty is measured at the OUTPUT, so upstream gears turn
    /// further, not less. Getting the direction backwards would silently
    /// under-count cycles on exactly the gears that see the most.
    #[test]
    fn intermittent_cycles_are_worked_backwards_from_the_output() {
        let mut t = two_stage();
        t.actuation = Actuation::Intermittent {
            range_degrees: 360.0,
            actuations: 100,
        };
        let r = solve_train(&t, &library()).unwrap();

        // The output gear turns exactly once per actuation.
        let last = &spur(&r.stages[1]).gears[1];
        assert!((last.tooth_cycles - 100.0).abs() < 1e-9);

        // Every gear upstream turns more than the one after it.
        let seq = [
            spur(&r.stages[0]).gears[0].tooth_cycles,
            spur(&r.stages[0]).gears[1].tooth_cycles,
            spur(&r.stages[1]).gears[1].tooth_cycles,
        ];
        for w in seq.windows(2) {
            assert!(w[0] > w[1], "cycles must fall towards the output: {seq:?}");
        }
        // The input gear sees the whole train ratio's worth.
        assert!((seq[0] - 100.0 * r.total_ratio).abs() < 1e-9);

        // The two gears meshing with each other turn at different speeds but
        // share a mesh, so their cycle counts differ by exactly that stage ratio.
        let s0 = spur(&r.stages[0]);
        assert!((s0.gears[0].tooth_cycles / s0.gears[1].tooth_cycles - s0.ratio).abs() < 1e-9);
    }

    #[test]
    fn continuous_cycles_follow_each_gears_own_speed() {
        let mut t = two_stage();
        t.actuation = Actuation::Continuous {
            operating_percent: 50.0,
            runtime_hours: 2.0,
        };
        let r = solve_train(&t, &library()).unwrap();

        // Input gear: 3000 rpm * 60 min * 2 h * 50%.
        let want = 3000.0 * 60.0 * 2.0 * 0.5;
        assert!((spur(&r.stages[0]).gears[0].tooth_cycles - want).abs() < 1e-6);
        // Speeds fall through the train, and cycles follow them.
        assert!((spur(&r.stages[0]).gears[0].speed - 3000.0).abs() < 1e-9);
        assert!((spur(&r.stages[1]).gears[1].speed - r.output_speed).abs() < 1e-9);
        assert!(
            spur(&r.stages[1]).gears[1].tooth_cycles < spur(&r.stages[0]).gears[0].tooth_cycles
        );
    }

    /// The automatic addendum, exercised through a whole stage rather than in
    /// isolation: set a minimum tip width, solve the stage, and measure the tip
    /// width off the gear the stage actually built.
    #[test]
    fn the_stages_automatic_addendum_produces_the_requested_tip_width() {
        for want in [0.05, 0.15, 0.3] {
            let mut stage = SpurStage::default();
            for g in &mut stage.gears {
                g.addendum = Auto::automatic(1.0);
                g.min_tip_width = want;
            }
            let r = solve_stage(&stage, 2.0, &library()).unwrap();

            for i in 0..2 {
                let built = Gear::new(stage.params(i));
                let got = 2.0 * built.ra * built.theta_a;
                assert!(
                    (got - want).abs() < 1e-9,
                    "gear {i}: wanted tip width {want}, built {got}"
                );
                // ...and the addendum reported is the one that produced it.
                assert!((r.gears[i].addendum - built.params.addendum).abs() < 1e-12);
            }
        }
    }

    /// Setting the centre distance by hand takes clearance out of the picture,
    /// which is what the specification requires and what changes the backlash.
    #[test]
    fn a_manual_centre_distance_ignores_the_clearance() {
        let lib = library();
        let auto = solve_stage(&SpurStage::default(), 2.0, &lib).unwrap();

        // The same distance, set by hand, with a clearance that must be ignored.
        let manual = solve_stage(
            &SpurStage {
                centre_distance: Auto::fixed(auto.centre_distance_nominal),
                clearance: 0.5,
                ..SpurStage::default()
            },
            2.0,
            &lib,
        )
        .unwrap();

        assert!((manual.centre_distance - auto.centre_distance_nominal).abs() < 1e-12);
        // At the zero-backlash distance there is, by construction, no backlash.
        assert!(manual.backlash.forward.nominal.abs() < 1e-9);
        // Whereas the automatic one carries its clearance into real backlash.
        assert!(auto.backlash.forward.nominal > 0.0);
    }

    /// An override has to reach the arithmetic, not just the display. Doubling
    /// the fatigue allowable must halve the face width contact asks for, since
    /// `b_min ∝ (σ_H/σ_allow)²` and the automatic width is sized by it.
    #[test]
    fn a_material_override_changes_the_answer() {
        let lib = library();
        let auto_width = |o: Overrides| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.auto_face_from_bending = false;
                g.material_overrides = o;
            }
            solve_stage(&s, 2.0, &lib).unwrap()
        };

        let base = auto_width(Overrides::default());
        let doubled = auto_width(Overrides {
            fatigue_allowable: Some(2.0 * 750.0),
            ..Default::default()
        });

        let ratio = base.gears[0].face_width / doubled.gears[0].face_width;
        assert!(
            (ratio - 4.0).abs() < 1e-9,
            "doubling the allowable should quarter the width: ratio {ratio}"
        );

        // ...and the reported material says the number came from the user.
        assert_eq!(
            doubled.gears[0].material.fatigue_allowable.basis,
            crate::material::Basis::Overridden
        );
        assert_eq!(
            base.gears[0].material.fatigue_allowable.basis,
            crate::material::Basis::Estimated
        );
    }

    /// Overriding the modulus moves contact stress, and by the right law.
    #[test]
    fn overriding_the_modulus_moves_contact_stress_as_the_square_root() {
        let lib = library();
        let at = |e: Option<f64>| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.material_overrides = Overrides {
                    elastic_modulus: e,
                    ..Default::default()
                };
            }
            solve_stage(&s, 2.0, &lib).unwrap().gears[0].contact_stress
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
            actuation: Actuation::default(),
            stages: vec![],
        };
        assert_eq!(solve_train(&t, &library()).unwrap_err(), TrainError::Empty);
    }
}
