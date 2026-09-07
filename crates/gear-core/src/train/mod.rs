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

use crate::auto::Ranges;
use crate::contact::{Directional, Drive};
use crate::material::{Material, MaterialLibrary};
use crate::mesh::MeshError;
use crate::note::{key, Note};

mod hula;
mod planetary;
mod spur;
mod worm;

pub use hula::{drive_efficiency, solve_hula_stage, HulaGear, HulaMesh, HulaResult, HulaStage};
pub use planetary::{
    solve_planetary_stage, solve_planetary_stage_with, MeshReport, PlanetResult, PlanetaryResult,
    PlanetaryStage,
};
pub(crate) use spur::ShiftAsked;
pub use spur::{solve_spur_stage, solve_spur_stage_with, SpurStage, StageGear};
pub use worm::{
    solve_crossed_stage, solve_worm_stage, FirstMemberSizing, WormContact, WormMember,
    WormMemberResult, WormResult, WormStage,
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

/// The note a **rating** raises about one member: its notch parameter sits
/// outside the band the `Y_S` fit is stated for.
///
/// `Y_S` is an empirical fit over `q_s = s_Fn / 2ρ_F`, stated for `1 ≤ q_s < 8`.
/// Outside it the formula still evaluates and
/// [`stress_correction`](crate::strength::RootSection::stress_correction)
/// clamps to the boundary — which for a notch *sharper* than the fit covers
/// **under-predicts** the stress, since `Y_S` rises with `q_s`. That is the
/// unconservative direction, and it is why
/// [`notch_parameter_in_range`](crate::strength::RootSection::notch_parameter_in_range)
/// exists.
///
/// It existed and nothing asked it. `docs/rationale.md` has said all along that
/// the range is "reported, not assumed", and that a result leaving the band
/// says so "instead of quietly returning a boundary value" — a promise with
/// nothing enforcing it, which is the shape of half of `docs/corrections.md`.
/// This is what asks.
///
/// Not a *clamp* on the part, so not in the member's clamp list: no geometry was
/// moved. It is the stage saying which member it is reporting a fitted number
/// outside the fit for.
pub(crate) fn notch_outside_fit(section: &crate::strength::RootSection) -> Option<Note> {
    (!section.notch_parameter_in_range())
        .then(|| Note::new(key::STAGE_NOTCH_OUTSIDE_FIT).number("q", section.notch_parameter, 2))
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
    /// From contact.
    pub contact: f64,
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
    /// The shift in force, after any automatic calculation.
    pub profile_shift: f64,
    /// Likewise the addendum.
    pub addendum: f64,
    /// Likewise the face width.
    pub face_width: f64,
    /// Torque on this gear, N·m, driving forward at peak.
    pub torque: f64,
    /// Torque on this gear from a back-driving load, N·m.
    ///
    /// `None` where none is reacted here — which is every gear of a train that
    /// can be back-driven at all, and every gear upstream of whatever stage
    /// stops one that cannot.
    pub back_driving_torque: Option<f64>,
    /// Rotational speed, rpm.
    pub speed: f64,
    /// Tooth load cycles over the duty the train describes.
    ///
    /// One cycle per revolution for a simple gear: a given tooth meets the mate
    /// once per turn. Sun and ring gears in a planetary stage see `N_planets`
    /// per revolution, and a planet is a special case again — docs/reference.md#trains.
    pub tooth_cycles: Cycles,
    /// Tooth root bending stress, MPa, for each load case. `None` where the
    /// stress correction is undefined for this section — see
    /// [`crate::strength::bending_stress`] — or where the case carries no load.
    pub bending_stress: LoadCase<Option<f64>>,
    /// Hertzian contact stress, MPa, for each load case — at **this gear's**
    /// governing point.
    ///
    /// The two gears of a mesh share one patch and one pressure at any instant,
    /// so this is not a per-tooth curvature effect. They differ because they are
    /// rated at different *moments*: each gear's dedendum is loaded alone at one
    /// end of the path, and that is where its own pitting is assessed. See
    /// [`crate::strength::ContactStress::governing`].
    pub contact_stress: LoadCase<f64>,
    /// The face width each rating would need: two per case, four in all.
    pub min_face_width: LoadCase<Widths>,
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

/// Everything a parallel-axis stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct SpurResult {
    /// `z₂ / z₁`.
    pub ratio: f64,
    /// Zero-backlash centre distance, mm.
    pub centre_distance_nominal: f64,
    /// **The clearance the stage opened by**, which is zero where nothing was
    /// free to absorb it — see [`SpurStage::clearance_taken`], the one place
    /// that is decided. Reported so a reader is told the input went unread
    /// rather than left to work it out, and so the panel can grey the field by
    /// reading the answer instead of knowing the rule a second time.
    pub clearance: f64,
    /// The centre distance actually used, including clearance.
    pub centre_distance: f64,
    /// **Operating pressure angle `α_w`, degrees** — the angle the profile
    /// shifts put the pair at, measured at the zero-backlash distance beside
    /// it rather than at the running one.
    ///
    /// The nominal one because it is the design quantity: it is what the shifts
    /// decide and what interference and tip thickness are judged against, and
    /// an assembly clearance of a few hundredths moves it without changing any
    /// of that. Reported by every stage that has a parallel-axis mesh, in the
    /// same units and from the same place ([`crate::mesh::Mesh::alpha_w`]); a
    /// crossed pair has no such angle, its line of action sliding rather than
    /// turning (docs/reference.md#crossed-axes).
    pub operating_pressure_angle: f64,
    pub contact_ratios: ContactRatios,
    /// Hertzian contact stress at the pitch point, MPa, in both load cases.
    ///
    /// The one figure the two members genuinely share: same patch, same normal
    /// force, same `E*`. Each gear's own rating sits on [`GearResult`] and is
    /// this or worse, depending on where its dedendum is loaded alone.
    pub contact_stress_at_pitch_point: LoadCase<f64>,
    /// Relative radius of curvature at the worst point on the path, mm.
    pub relative_radius: f64,
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
    NoContact,
    /// A material name that is not in the library.
    UnknownMaterial(String),
    /// A tooth so undercut there is no root section left to rate.
    NoRootSection,
    /// The train has no stages, so there is nothing to accumulate.
    Empty,
    /// A hula drive that has no geometry — see [`crate::hula::Error`].
    Hula(crate::hula::Error),
}

impl From<hula::Error> for TrainError {
    fn from(e: hula::Error) -> Self {
        match e {
            hula::Error::Mesh(m) => Self::Mesh(m),
            hula::Error::Drive(d) => Self::Hula(d),
        }
    }
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
            Self::UnknownMaterial(n) => {
                Note::new(key::ERROR_TRAIN_UNKNOWN_MATERIAL).text("name", n.clone())
            }
            Self::NoRootSection => Note::new(key::ERROR_TRAIN_NO_ROOT_SECTION),
            Self::Empty => Note::new(key::ERROR_TRAIN_EMPTY),
        }
    }
}

/// English, for the CLI and for `Debug`. **Not** what the browser renders — see
/// [`TrainError::note`].
impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mesh(e) => write!(f, "{e}"),
            Self::Hula(e) => write!(f, "the drive has no geometry: {e:?}"),
            Self::Screw(e) => match e {
                crate::screw::ScrewError::NotPositive => {
                    write!(f, "a module, diameter or tooth count is not positive")
                }
                crate::screw::ScrewError::WormTooThin => write!(
                    f,
                    "the first member has no lead angle: sized by diameter, the worm \
                     is too thin for that many starts at that module and its thread \
                     would have to wrap at ninety degrees or more; sized by helix \
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
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum Stage {
    Spur(SpurStage),
    Worm(WormStage),
    // Boxed for the same reason `StageResult`'s variants are: a planetary stage
    // carries three gears, a cutter and an arrangement where a spur stage carries
    // two gears, so a `Vec<Stage>` would otherwise pay the largest of them for
    // every stage whatever its kind. Invisible to readers and to serde.
    Planetary(Box<PlanetaryStage>),
    Hula(Box<HulaStage>),
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
    Spur(Box<SpurResult>),
    Worm(Box<WormResult>),
    Planetary(Box<PlanetaryResult>),
    Hula(Box<HulaResult>),
}

impl StageResult {
    /// `z₂/z₁`.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Spur(r) => r.ratio,
            Self::Worm(r) => r.ratio,
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
            Self::Spur(r) => r.efficiency,
            Self::Worm(r) => r.efficiency,
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
            Self::Spur(r) => r.backlash,
            Self::Worm(r) => r.backlash,
            Self::Planetary(r) => r.backlash,
            Self::Hula(r) => r.backlash,
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

    /// The parallel-axis result, if that is what this is.
    #[must_use]
    pub fn as_spur(&self) -> Option<&SpurResult> {
        match self {
            Self::Spur(r) => Some(r),
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

    /// The worm result, if that is what this is.
    #[must_use]
    pub fn as_worm(&self) -> Option<&WormResult> {
        match self {
            Self::Worm(r) => Some(r),
            _ => None,
        }
    }

    /// Write in the speeds and cycles, which only the whole shaft line knows.
    ///
    /// The cycles arriving here are **revolutions**, fractional; each stage kind
    /// scales them into the count its members actually see and rounds at the end
    /// with [`loaded_cycles`].
    ///
    /// A worm's "tooth cycles" are revolutions: its thread is engaged
    /// continuously rather than meeting a mate once per turn, so the count is
    /// the same arithmetic but means something looser. It is reported because a
    /// duty cycle has to be reported somewhere, not because a worm thread has a
    /// fatigue life this crate can rate.
    fn set_kinematics(&mut self, speeds: [f64; 2], cycles: [(f64, Option<(f64, f64)>); 2]) {
        match self {
            Self::Spur(r) => {
                for (i, g) in r.gears.iter_mut().enumerate() {
                    g.speed = speeds[i];
                    g.tooth_cycles = loaded_cycles(cycles[i].0, cycles[i].1);
                }
            }
            Self::Worm(r) => {
                for (i, m) in r.members.iter_mut().enumerate() {
                    m.speed = speeds[i];
                    m.tooth_cycles = loaded_cycles(cycles[i].0, cycles[i].1);
                }
                // Sliding needs a shaft speed, so it could only be filled here.
                r.sliding_velocity = r.sliding_ratio
                    * (speeds[0] / 60.0 * std::f64::consts::TAU)
                    * (r.members[0].pitch_diameter / 2.0);
            }
            // A planetary's speeds are not the train's two-member pattern — it
            // has three shafts and its own kinematics already set them, so only
            // the cycles are filled here. Sun and ring meet a planet `N` times
            // per revolution; the planet is the special case of docs/reference.md#trains, and what
            // fatigues it is its rotation **relative to the carrier**.
            // A hula sets its own speeds — four gears on three shafts, and its
            // own kinematics filled them. Nothing else is filled here: a cycle
            // count is what a fatigue rating consumes, and this stage has none
            // to consume it, so the arm is empty rather than filling a field
            // nothing reads.
            Self::Hula(_) => {}
            Self::Planetary(r) => {
                let n = f64::from(r.planets.max(1));
                // Sun and ring meet a planet `N` times per revolution and the
                // planet turns relative to the carrier, so each member's
                // revolutions scale before they are counted — including, for a
                // reversing drive, the per-actuation figure the rounding is
                // applied to.
                let scaled = |f: f64| (cycles[0].0 * f, cycles[0].1.map(|(e, a)| (e * f, a)));
                let (sun_turns, sun_each) = scaled(n);
                r.sun.tooth_cycles = loaded_cycles(sun_turns, sun_each);
                r.ring.tooth_cycles = loaded_cycles(sun_turns, sun_each);
                let carrier_relative =
                    (r.planet.speed_relative / r.speeds[0].abs().max(f64::MIN_POSITIVE)).abs();
                let (p_turns, p_each) = scaled(carrier_relative);
                r.planet.gear.tooth_cycles = loaded_cycles(p_turns, p_each);
            }
        }
    }
}

/// Which ratings an automatic face width is sized from.
///
/// Four toggles rather than two, and shaped like the answer they select from so
/// the UI can walk them rather than naming each: a rating exists for every
/// combination of what fails (bending or contact) and what it is rated against
/// (the peak or the cyclic load).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct FaceSources {
    pub bending: LoadCase<bool>,
    pub contact: LoadCase<bool>,
}

impl Default for FaceSources {
    fn default() -> Self {
        Self {
            bending: LoadCase {
                peak: true,
                cyclic: true,
            },
            // **Neither contact rating sizes a width by default.** Both are
            // offered and both are computed; what they are not is *assumed*.
            //
            // The peak case is the weaker of the two: a Hertzian pressure is not
            // a tensile stress, and the library's `ultimate_allowable` is a
            // tensile figure — a flank under a single overload fails by
            // subsurface shear, at a contact pressure well above it. Comparing
            // them is arithmetic with no mechanism behind it.
            //
            // The cyclic case is sounder — the fatigue allowable is a flank
            // figure — but it is the one that *dominates*, by an order of
            // magnitude: on the reference train it asks 8.5 mm where bending
            // asks 0.9. A default that decides the answer is a default making
            // the design decision, so both are left to the designer and bending
            // is what a fresh stage is sized from.
            contact: LoadCase {
                peak: false,
                cyclic: false,
            },
        }
    }
}

impl FaceSources {
    /// The largest width any **enabled** rating asks for.
    ///
    /// Zero when none is enabled, which resolves to a zero face width — a
    /// degenerate gear rather than a wrong one, and said in the stage's notes
    /// rather than divided by.
    #[must_use]
    pub fn largest_of(&self, asks: &LoadCase<Widths>) -> f64 {
        let mut want = 0.0_f64;
        for case in [Case::Peak, Case::Cyclic] {
            let w = asks.get(case);
            if *self.bending.get(case) {
                if let Some(b) = w.bending {
                    want = want.max(b);
                }
            }
            if *self.contact.get(case) {
                want = want.max(w.contact);
            }
        }
        want
    }

    /// Whether anything at all is selected.
    #[must_use]
    pub fn any(&self) -> bool {
        self.bending.peak || self.bending.cyclic || self.contact.peak || self.contact.cyclic
    }
}

/// How a stage's gears are treated for **reversed bending**.
///
/// Assembled by [`solve_train`] and handed down, because both halves of it are
/// facts about the train rather than about any one stage: whether the drive
/// reverses, and whether the designer asked for the correction at all.
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
/// whatever the drive does. Every other gear does when the *drive* reverses,
/// which is the same flag that already splits the contact cycles between the two
/// flanks. One rule, so a note cannot appear where a cycle count does not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reversal {
    /// The drive reverses between actuations, so every gear's root is loaded
    /// both ways.
    pub drive_reverses: bool,
    /// Judge a reversed root against the reduced allowable.
    pub correct: bool,
}

impl Reversal {
    /// Whether a member's root is loaded both ways. `always` is the member's own
    /// structural answer — true for a planet, false for everything else.
    #[must_use]
    pub fn reverses(self, always: bool) -> bool {
        always || self.drive_reverses
    }

    /// What a member's reversal is worth saying about it, if anything.
    ///
    /// One home, so a stage cannot report the correction on one member and stay
    /// silent about it on another — and so the sentence is the same whichever
    /// kind of stage raises it.
    #[must_use]
    pub fn note_for(self, reverses: bool) -> Option<Note> {
        if !reverses {
            return None;
        }
        Some(if self.correct {
            Note::new(key::STAGE_REVERSED_BENDING_APPLIED).number(
                "fraction",
                crate::material::REVERSED_BENDING_FRACTION,
                2,
            )
        } else {
            Note::new(key::STAGE_REVERSED_BENDING_UNCORRECTED)
        })
    }

    /// The **bending** allowable a member is judged against, MPa.
    ///
    /// Only the cyclic case can be reduced: a peak load is survived once and has
    /// no reversal to endure. And only bending — pitting is compressive whichever
    /// flank carries it, so a contact rating keeps the material's own figure.
    #[must_use]
    pub fn bending_allowable(self, m: &Material, case: Case, reverses: bool) -> f64 {
        if self.correct && reverses && case == Case::Cyclic {
            crate::material::reversed_bending_allowable(m).value
        } else {
            allowable(m, case)
        }
    }
}

/// The torques one stage sees, at its **first** member, one per load case.
///
/// Assembled by [`solve_train`], which is the only level that knows where a
/// stage sits in the shaft line and what reaches it from each end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageTorques {
    /// Peak, delivered forward from the input, N·m.
    pub peak_forward: f64,
    /// Peak, delivered backward from the output, N·m — `None` where no
    /// back-driving load is reacted here. See [`back_driving_torques`].
    pub peak_backward: Option<f64>,
    /// The torque the train runs at, forward, N·m. May be zero.
    pub cyclic: f64,
}

impl StageTorques {
    /// The torque to rate a load case at.
    ///
    /// The peak case takes whichever direction loads the teeth harder. Both have
    /// already been attenuated by the efficiencies on their own way here, so
    /// either can be the larger and either can be absent.
    #[must_use]
    pub fn at(&self, case: Case) -> f64 {
        match case {
            Case::Peak => self
                .peak_forward
                .abs()
                .max(self.peak_backward.unwrap_or(0.0).abs()),
            Case::Cyclic => self.cyclic.abs(),
        }
    }

    /// Everything at one torque, and nothing back-driving — the shape a caller
    /// wants when it is asking about a single load.
    #[must_use]
    pub fn just(torque: f64) -> Self {
        Self {
            peak_forward: torque,
            peak_backward: None,
            cyclic: torque,
        }
    }
}

/// Which allowable a load case is judged against.
///
/// **This is the whole reason the two cases are separate.** A peak load has to be
/// survived once, so the ultimate is the right bar; a cyclic one has to be
/// survived for the duty, so the fatigue figure is. Rating a peak against a
/// fatigue allowable asks the wrong question, and it is what this crate did
/// before the cases existed.
#[must_use]
pub fn allowable(material: &Material, case: Case) -> f64 {
    match case {
        Case::Peak => material.ultimate_allowable.value,
        Case::Cyclic => material.fatigue_allowable.value,
    }
}

/// A quantity evaluated for each **load case** the train describes.
///
/// Named for the engineering term — a defined set of loads applied for analysis
/// — and deliberately *not* for a duty cycle, which is a fraction of time spent
/// running and a different idea with different implications. One of this train's
/// inputs genuinely is that; these are not it.
///
/// The two differ in one thing beyond their torque, and it is the thing that
/// matters: **which allowable they are rated against.** A peak load has to be
/// survived once, so it is judged against the ultimate; a cyclic one has to be
/// survived indefinitely, so it is judged against the fatigue figure. Rating a
/// peak against a fatigue allowable — which is what this crate did before the
/// two cases existed — asks the wrong question and answers it confidently.
///
/// Built through [`LoadCase::of`] for the same reason [`Directional`] is: there
/// is no path by which a stage reports one case and not the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LoadCase<T> {
    /// The worst single application: survive it once.
    pub peak: T,
    /// The load it runs at: survive it for the duty.
    pub cyclic: T,
}

/// Which load case a quantity belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Peak,
    Cyclic,
}

impl<T> LoadCase<T> {
    /// Ask the same question of both cases.
    pub fn of(mut f: impl FnMut(Case) -> T) -> Self {
        Self {
            peak: f(Case::Peak),
            cyclic: f(Case::Cyclic),
        }
    }

    /// The value for one case.
    pub const fn get(&self, case: Case) -> &T {
        match case {
            Case::Peak => &self.peak,
            Case::Cyclic => &self.cyclic,
        }
    }

    /// The same pair with each value mapped.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> LoadCase<U> {
        LoadCase {
            peak: f(self.peak),
            cyclic: f(self.cyclic),
        }
    }
}

/// How often a tooth is loaded, which is not one number once the drive reverses.
///
/// Both counts are always reported and are the **same number** when the drive
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
    /// Engagements the tooth root sees. A reversing drive loads **both** flanks
    /// in bending, so every engagement counts.
    pub bending: f64,
    /// Engagements one flank sees. A reversing drive shares them between the two
    /// flanks, so each takes half; otherwise it is the same number as `bending`.
    pub contact: f64,
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
/// `None` for a continuous drive: there is no actuation to round within, so the
/// total rounds once and the two counts are equal.
#[must_use]
pub fn loaded_cycles(revolutions: f64, per_actuation: Option<(f64, f64)>) -> Cycles {
    match per_actuation {
        Some((each, actuations)) => {
            let bending = each.ceil() * actuations;
            Cycles {
                bending,
                contact: bending / 2.0,
            }
        }
        None => {
            let n = revolutions.ceil();
            Cycles {
                bending: n,
                contact: n,
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
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<StageResult, TrainError> {
    solve_any_with(stage, input_speed, torques, lib, Reversal::default())
}

/// The same, told how the train treats reversed bending.
///
/// A second entry point rather than a fourth argument on the first, because a
/// stage asked about in isolation — by a test, by the CLI, by the sweep — has no
/// train to inherit that from and should not have to invent one. The plain call
/// is this one at [`Reversal::default`]: no reversing drive, no correction.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any_with(
    stage: &Stage,
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
    reversal: Reversal,
) -> Result<StageResult, TrainError> {
    match stage {
        // One stage kind, two meshes. Crossing the shafts changes what the
        // teeth do to each other — a line contact becomes a point, and the
        // sliding changes direction — so a crossed pair answers with the screw
        // result. The *inputs* stay one set, as the specification has them.
        Stage::Spur(s) if s.is_crossed() => {
            solve_crossed_stage(s, torques, lib).map(|r| StageResult::Worm(Box::new(r)))
        }
        Stage::Spur(s) => {
            solve_spur_stage_with(s, torques, lib, reversal).map(|r| StageResult::Spur(Box::new(r)))
        }
        Stage::Worm(s) => solve_worm_stage(s, torques, lib).map(|r| StageResult::Worm(Box::new(r))),
        // A planetary needs a speed as well as a torque: its efficiency depends
        // on which shaft is held, and that is a kinematic question. The train
        // supplies the speed it has reached by this point.
        Stage::Planetary(s) => solve_planetary_stage_with(s, input_speed, torques, lib, reversal)
            .map(|r| StageResult::Planetary(Box::new(r))),
        // A hula needs a speed and a torque for the same reason a planetary
        // does: its efficiency is a power flow, and a power flow is not a
        // property of the teeth alone.
        Stage::Hula(s) => solve_hula_stage(s, input_speed, torques.peak_forward)
            .map(|r| StageResult::Hula(Box::new(r)))
            .map_err(TrainError::from),
    }
}

/// How the train is used, which is what turns a ratio into a tooth count.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Actuation {
    /// A limited sweep, repeated. The range is measured **at the output**, so
    /// every gear's revolutions are worked *backwards* from there — upstream
    /// gears turn further, not less.
    Intermittent {
        /// Output sweep per actuation, degrees.
        range_degrees: f64,
        actuations: u32,
        /// Whether the drive reverses between actuations.
        ///
        /// It changes nothing but the **cycle count**, and it changes that in
        /// two ways ([`loaded_cycles`]): each actuation's revolutions round up
        /// on their own rather than the total rounding once, because a partial
        /// sweep still loads the teeth it reaches and every tooth must meet the
        /// worst of them; and the contact count halves, because the two flanks
        /// share the engagements while both take the full bending.
        reversing: bool,
    },
    /// Continuous running at the speed it actually runs at.
    Continuous {
        /// The input speed the train runs at, rpm. Bounded by the peak, and
        /// **absolute** rather than a percentage of it for the reason
        /// [`Train::operating_torque_percent`] gives: the crate will not assert
        /// a relation between torque and speed on the user's behalf.
        operating_speed: f64,
        runtime_hours: f64,
    },
}

impl Default for Actuation {
    fn default() -> Self {
        Self::Intermittent {
            range_degrees: 25.0,
            actuations: 1000,
            reversing: false,
        }
    }
}

/// A whole geartrain.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Train {
    /// Peak input speed, rpm.
    pub input_speed: f64,
    /// Peak input torque, N·m.
    pub input_torque: f64,
    /// Peak torque applied at the **output** shaft, N·m, trying to drive the
    /// train backwards.
    ///
    /// A load case of its own rather than a sign on the input: it enters at the
    /// far end and is attenuated by each stage's *backward* efficiency on the
    /// way up. See [`back_driving_torques`] for where it is reacted, and where
    /// it therefore reaches no number at all.
    pub back_driving_torque: f64,
    /// The torque the train runs at, N·m — the load its fatigue life is spent
    /// against, as opposed to the peak it must merely survive.
    ///
    /// Bounded by [`Self::input_torque`] and meaningful down to and including
    /// **zero**: a train that only ever sees its peak has no cyclic case.
    pub operating_torque: f64,
    pub actuation: Actuation,
    /// Judge a root that is loaded on **both** flanks against the reduced
    /// bending allowable.
    ///
    /// Off by default. A planet's root is always loaded both ways and a
    /// reversing drive loads every root both ways, but what to do about it is a
    /// convention that multiplies a stress — so the train asks rather than
    /// assumes, and says where reversal is present and uncorrected. See
    /// [`Reversal`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub reversed_bending: bool,
    pub stages: Vec<Stage>,
}

impl Train {
    /// What fraction of peak the operating torque is, as a percentage.
    ///
    /// Reported rather than entered. The two are given **separately and
    /// absolutely** because this crate has no basis for a relation between
    /// torque and speed: an electric motor makes them inversely proportional,
    /// efficiencies bend that, and another power source need not obey it at all.
    /// One percentage driving both would assert a relationship nothing here can
    /// stand behind — so the user states each, and the ratio between them is an
    /// *output*, computed where every other number is.
    ///
    /// `None` at zero peak, where there is no fraction to take.
    #[must_use]
    pub fn operating_torque_percent(&self) -> Option<f64> {
        (self.input_torque != 0.0).then(|| 100.0 * self.cyclic_torque() / self.input_torque)
    }

    /// The operating torque as the solve uses it: never above the peak.
    ///
    /// Clamped rather than refused (docs/rationale.md), and clamped **here** so
    /// that the figure the train is solved at, the percentage reported beside
    /// the input, and the note that says it happened cannot disagree.
    #[must_use]
    pub fn cyclic_torque(&self) -> f64 {
        self.operating_torque
            .clamp(-self.input_torque.abs(), self.input_torque.abs())
    }

    /// The operating speed as the solve uses it, rpm — likewise never above the
    /// peak. `None` for an intermittent drive, which has no operating speed:
    /// it turns through its range and stops.
    #[must_use]
    pub fn cyclic_speed(&self) -> Option<f64> {
        match self.actuation {
            Actuation::Continuous {
                operating_speed, ..
            } => Some(operating_speed.clamp(-self.input_speed.abs(), self.input_speed.abs())),
            Actuation::Intermittent { .. } => None,
        }
    }
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
    /// The operating torque as a percentage of the peak — see
    /// [`Train::operating_torque_percent`] for why this is an output and not an
    /// input. `None` at zero peak.
    pub operating_torque_percent: Option<f64>,
    /// What the train as a whole wants read: an input clamped against its peak,
    /// and where a back-driving load is reacted. Facts about the shaft line, so
    /// no stage is in a position to say them.
    pub notes: Vec<Note>,
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
/// Where a back-driving load is reacted, and what each stage feels of it.
///
/// # The model
///
/// A load exists only where something reacts it. A torque applied at the output
/// shaft works its way *upstream*, attenuated at each stage by that stage's
/// backward ratio and backward efficiency — until it reaches a stage that cannot
/// be back-driven at all. That stage holds it: everything upstream sees nothing,
/// and the reaction is the load this stage carries.
///
/// If the chain reaches the input still turning something, then **nothing
/// reacted it**: the train is back-drivable, the load simply drives it, and the
/// case is zero everywhere. A back-driving torque on a back-drivable train is an
/// input that moves no number, and this is why.
///
/// Each stage's figure is referred to its own **input** shaft, which is the shaft
/// every stage solver takes its torque on. That referral is a division by the
/// ratio and nothing else: the mesh force is set by the torque at the wheel, and
/// the losses sit between the mesh and the shaft beyond it, not before it.
fn back_driving_torques(stages: &[StageResult], applied: f64) -> (Vec<Option<f64>>, Option<usize>) {
    let none = || vec![None; stages.len()];
    if applied == 0.0 {
        return (none(), None);
    }
    let mut torques = none();
    let mut at_output = applied;
    for (k, s) in stages.iter().enumerate().rev() {
        let referred = at_output / s.ratio();
        torques[k] = Some(referred);
        let backward = s.efficiency().backward;
        if backward <= 0.0 {
            // Self-locking: this stage is where the load stops.
            return (torques, Some(k));
        }
        at_output = referred * backward;
    }
    // The load reached the input with something still to turn.
    (none(), None)
}

/// # Errors
///
/// [`TrainError::Empty`] for a train with no stages, or whatever the first
/// failing stage reports.
pub fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<TrainResult, TrainError> {
    if train.stages.is_empty() {
        return Err(TrainError::Empty);
    }

    // How every stage treats a root loaded on both flanks. Both halves are the
    // train's to know: the drive's own reversal is the same flag that splits the
    // contact cycles, and whether to correct for it at all is one switch for the
    // whole train rather than a decision taken per stage.
    let reversal = Reversal {
        drive_reverses: matches!(
            train.actuation,
            Actuation::Intermittent {
                reversing: true,
                ..
            }
        ),
        correct: train.reversed_bending,
    };

    // --- what each stage is loaded by.
    //
    // Two propagations, in opposite directions, and only the forward one can
    // start: a stage's ratio and efficiency do not depend on the torque through
    // it, but the *backward* torque at a stage depends on every efficiency
    // downstream, which is not known until those stages have been solved. So the
    // train is solved once to learn the shaft line and again to rate it. The
    // second pass is not a refinement of the first — it is the same arithmetic
    // with the load it was missing.
    let forward =
        |torques: &dyn Fn(usize) -> StageTorques| -> Result<Vec<StageResult>, TrainError> {
            let mut speed = train.input_speed;
            let mut out = Vec::with_capacity(train.stages.len());
            for (k, stage) in train.stages.iter().enumerate() {
                let r = solve_any_with(stage, speed, torques(k), lib, reversal)?;
                speed /= r.ratio();
                out.push(r);
            }
            Ok(out)
        };

    // Forward torques are the same in both passes, so they are worked out once
    // from the ratios and efficiencies the first pass reports.
    let first = forward(&|_| StageTorques::just(train.input_torque))?;
    let mut fwd = Vec::with_capacity(first.len());
    let cyclic_torque = train.cyclic_torque();
    let (mut peak, mut cyclic) = (train.input_torque, cyclic_torque);
    for r in &first {
        fwd.push((peak, cyclic));
        peak = peak * r.ratio() * r.efficiency().forward;
        cyclic = cyclic * r.ratio() * r.efficiency().forward;
    }
    let (backward, reacted_at) = back_driving_torques(&first, train.back_driving_torque);
    let mut stages = forward(&|k| StageTorques {
        peak_forward: fwd[k].0,
        peak_backward: backward[k],
        cyclic: fwd[k].1,
    })?;
    let torque = peak;

    // --- what the train wants read, as opposed to what a stage does.
    let mut notes = Vec::new();
    if train.operating_torque != cyclic_torque {
        notes.push(Note::new(key::TRAIN_OPERATING_TORQUE_CLAMPED).number(
            "torque",
            cyclic_torque,
            4,
        ));
    }
    if let (
        Actuation::Continuous {
            operating_speed, ..
        },
        Some(used),
    ) = (&train.actuation, train.cyclic_speed())
    {
        if *operating_speed != used {
            notes.push(Note::new(key::TRAIN_OPERATING_SPEED_CLAMPED).number("speed", used, 1));
        }
    }
    if train.back_driving_torque != 0.0 {
        notes.push(match reacted_at {
            Some(k) => {
                Note::new(key::TRAIN_BACK_DRIVING_REACTED_AT).text("stage", (k + 1).to_string())
            }
            None => Note::new(key::TRAIN_BACK_DRIVING_NOT_REACTED),
        });
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

        // The reduction between each member and the output. MeshSide 0 of a stage
        // sits before that stage's own mesh, member 1 after it.
        // Revolutions, and — for an intermittent drive — how they divide into
        // actuations, which is what a reversing drive needs in order to round
        // within one rather than over all of them.
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
                    reversing,
                } => {
                    let each = (range_degrees / 360.0) * to_output;
                    let n = f64::from(actuations);
                    (each * n, reversing.then_some((each, n)))
                }
                // A continuous drive turns at the speed it runs at, for as long
                // as it runs. There is no actuation to round within, so reversing
                // has nothing to mean here — the toggle is offered only where it
                // does.
                Actuation::Continuous { runtime_hours, .. } => {
                    // Each shaft's own speed, scaled from the peak the train was
                    // laid out at to the speed it actually runs at.
                    let scale = if train.input_speed == 0.0 {
                        0.0
                    } else {
                        train.cyclic_speed().unwrap_or(0.0) / train.input_speed
                    };
                    (speeds[i] * scale * 60.0 * runtime_hours, None)
                }
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
        operating_torque_percent: train.operating_torque_percent(),
        notes,
        stages,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::material::Overrides;
    use crate::params::Auto;
    use crate::tooth::Tooth;

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
            _ => panic!("this train's stages are all spur"),
        }
    }

    fn two_stage() -> Train {
        Train {
            input_speed: 3000.0,
            input_torque: 2.0,
            back_driving_torque: 0.0,
            operating_torque: 2.0,
            reversed_bending: false,
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
            assert!(s.contact_stress_at_pitch_point.peak > 0.0);
            for g in &s.gears {
                assert!(g.face_width > 0.0);
                assert!(g.bending_stress.peak.unwrap() > 0.0);
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
            spur_input(s).sliding_friction = 0.0;
            spur_input(s).static_friction = 0.0;
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
            let stage = SpurStage {
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
            solve_spur_stage(&stage, StageTorques::just(2.0), &lib)
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
        let stage = |clearance: f64| SpurStage {
            clearance,
            ..SpurStage::default()
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solve_spur_stage(&stage(clearance), StageTorques::just(2.0), &lib).unwrap();
            let eps = r.contact_ratios.transverse;
            let bending = r.gears[0].bending_stress.peak.expect("a rateable tooth");
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
        let spur = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        assert_eq!(spur.contact_ratios.overlap, 0.0, "must be exactly zero");
        assert_eq!(spur.contact_ratios.total, spur.contact_ratios.transverse);
        assert!(!spur.contact_ratios.has_full_axial_overlap());

        let helical = solve_spur_stage(
            &SpurStage {
                additional_helix: 20.0,
                ..SpurStage::default()
            },
            StageTorques::just(2.0),
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
        let k: Vec<f64> = (0..2)
            .map(|i| stage.params_at(i, stage.shifts()[i]).thickness_mod)
            .collect();
        assert!((k[0] + k[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_automatic_face_width_is_the_larger_of_the_enabled_checks() {
        let lib = library();
        let off = LoadCase {
            peak: false,
            cyclic: false,
        };
        let width = |sources: FaceSources| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib)
                .unwrap()
                .gears[0]
                .face_width
        };
        // One source at a time, then every combination of them: the width is the
        // largest of whatever is enabled, and that is the whole rule.
        let each: Vec<f64> = [
            FaceSources {
                bending: LoadCase {
                    peak: true,
                    cyclic: false,
                },
                contact: off,
            },
            FaceSources {
                bending: LoadCase {
                    peak: false,
                    cyclic: true,
                },
                contact: off,
            },
            FaceSources {
                bending: off,
                contact: LoadCase {
                    peak: true,
                    cyclic: false,
                },
            },
            FaceSources {
                bending: off,
                contact: LoadCase {
                    peak: false,
                    cyclic: true,
                },
            },
        ]
        .into_iter()
        .map(width)
        .collect();
        // The law is about what is *enabled*, so it is checked against every
        // source switched on rather than against the default — which is a
        // separate decision, and is asserted as one below.
        let on = LoadCase {
            peak: true,
            cyclic: true,
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

    /// Intermittent duty is measured at the OUTPUT, so upstream gears turn
    /// further, not less. Getting the direction backwards would silently
    /// under-count cycles on exactly the gears that see the most.
    #[test]
    fn intermittent_cycles_are_worked_backwards_from_the_output() {
        let mut t = two_stage();
        t.actuation = Actuation::Intermittent {
            range_degrees: 360.0,
            actuations: 100,
            reversing: false,
        };
        let r = solve_train(&t, &library()).unwrap();

        // The output gear turns exactly once per actuation. A whole number of
        // revolutions, so the rounding has nothing to do and the count is the
        // revolutions exactly — which is the case worth putting first, because
        // it is where a count and a revolution coincide.
        let last = &spur(&r.stages[1]).gears[1];
        assert_eq!(last.tooth_cycles.bending, 100.0);
        // Not reversing, so one flank takes every engagement.
        assert_eq!(last.tooth_cycles.contact, last.tooth_cycles.bending);

        // Every gear upstream turns more than the one after it.
        let seq = [
            spur(&r.stages[0]).gears[0].tooth_cycles.bending,
            spur(&r.stages[0]).gears[1].tooth_cycles.bending,
            spur(&r.stages[1]).gears[1].tooth_cycles.bending,
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
        let (a, b) = (
            s0.gears[0].tooth_cycles.bending,
            s0.gears[1].tooth_cycles.bending,
        );
        assert!(
            (a / b - s0.ratio).abs() < (1.0 + s0.ratio) / b,
            "{a}/{b} = {} against a stage ratio of {}",
            a / b,
            s0.ratio
        );
    }

    #[test]
    fn continuous_cycles_follow_each_gears_own_speed() {
        let mut t = two_stage();
        t.actuation = Actuation::Continuous {
            operating_speed: 1500.0,
            runtime_hours: 2.0,
        };
        let r = solve_train(&t, &library()).unwrap();

        // Input gear: 1500 rpm * 60 min * 2 h — its own speed, for as long as
        // the train runs. Rounded up, as every count is.
        let want = (1500.0_f64 * 60.0 * 2.0).ceil();
        let first = spur(&r.stages[0]).gears[0].tooth_cycles;
        assert!((first.bending - want).abs() < 1e-6);
        // A continuous drive has no actuation to reverse within, so the two
        // counts agree.
        assert_eq!(first.bending, first.contact);
        // Speeds fall through the train, and cycles follow them.
        assert!((spur(&r.stages[0]).gears[0].speed - 3000.0).abs() < 1e-9);
        assert!((spur(&r.stages[1]).gears[1].speed - r.output_speed).abs() < 1e-9);
        assert!(spur(&r.stages[1]).gears[1].tooth_cycles.bending < first.bending);
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
            let r = solve_spur_stage(&stage, StageTorques::just(2.0), &library()).unwrap();

            for i in 0..2 {
                let built = Tooth::new(stage.params_at(i, stage.shifts()[i]));
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
    /// anything. These sit roughly ten times what each search costs (0.7 ms,
    /// 10 ms and 1.8 ms), which is loose enough for a busy machine and tight
    /// enough to catch the kind of regression that has actually happened here:
    /// 800 ms, 100 ms and 68 ms at various points, every time because something
    /// was built per candidate that nothing then read.
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

        let pair = SpurStage {
            optimisation: tuned,
            ..SpurStage::default()
        };
        each("pair's", 10, &|| {
            solve_spur_stage(&pair, StageTorques::just(2.0), &lib).unwrap();
        });

        let mut set = PlanetaryStage {
            optimisation: tuned,
            ..PlanetaryStage::default()
        };
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        each("epicyclic set's", 60, &|| {
            solve_planetary_stage(&set, 3000.0, StageTorques::just(2.0), &lib).unwrap();
        });

        let drive = HulaStage {
            optimisation: tuned,
            ..HulaStage::default()
        };
        each("eccentric drive's", 20, &|| {
            solve_hula_stage(&drive, 1000.0, 2.0).unwrap();
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
                ..SpurStage::default().gears[0].clone()
            };
            SpurStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                gears: [gear(9), gear(37)],
                ..SpurStage::default()
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
                    SpurStage {
                        optimisation: Optimisation::default(),
                        ..s
                    }
                    .shifts()
                );
                continue;
            }
            assert!(
                sum <= last + 1e-9,
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
        let stage = |on: bool| SpurStage {
            gears: [
                StageGear {
                    teeth: 17,
                    ..SpurStage::default().gears[0].clone()
                },
                StageGear {
                    teeth: 43,
                    ..SpurStage::default().gears[1].clone()
                },
            ],
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            ..SpurStage::default()
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
            1.0 - solve_spur_stage(&stage(on), StageTorques::just(2.0), &lib)
                .unwrap()
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
    /// an epicyclic set, and the eccentric drive checked its pinion for a
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

        let spur = SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..SpurStage::default()
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

        // The eccentric drive's rack-generated members are its pinions; its
        // rings are the shaper's and are not asked.
        let drive = HulaStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..HulaStage::default()
        };
        let r = solve_hula_stage(&drive, 1000.0, 2.0).expect("the drive solves");
        for (i, g) in r.gears.iter().enumerate() {
            if g.ring {
                continue;
            }
            cuttable(
                &GearParams {
                    module: drive.module[i / 2],
                    pressure_angle: drive.pressure_angle,
                    helix_angle: drive.helix_angle,
                    teeth: g.teeth,
                    profile_shift: g.profile_shift,
                    addendum: drive.gears[i].addendum.manual,
                    dedendum: drive.gears[i].dedendum,
                    root_radius: drive.gears[i].root_radius,
                    thickness_mod: drive.thickness_mod[i / 2],
                    ..GearParams::default()
                },
                "the drive's pinion",
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
        let free = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        // A housing the pair can actually meet: a clearance inside it is the
        // distance the automatic solve already closes to.
        let asked = free.centre_distance_nominal + 0.05;
        let at = |on: bool| SpurStage {
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            clearance: 0.05,
            ..SpurStage::default()
        };

        // Nothing free: the input is not read, and the answer reports that.
        let pinned = solve_spur_stage(&at(false), StageTorques::just(2.0), &lib).unwrap();
        assert!(pinned.clearance == 0.0, "read {}", pinned.clearance);

        // The shifts free: they take it, and the backlash is the one asked for.
        let chosen = solve_spur_stage(&at(true), StageTorques::just(2.0), &lib).unwrap();
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
                &SpurStage {
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    clearance: 0.05,
                    ..SpurStage::default()
                },
                StageTorques::just(2.0),
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
        let free = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        let asked = free.centre_distance_nominal + 0.4;
        let stage = SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            ..SpurStage::default()
        };
        let r = solve_spur_stage(&stage, StageTorques::just(2.0), &lib).unwrap();
        assert!(
            (r.centre_distance - asked).abs() < 1e-9,
            "asked for {asked}, ran at {}",
            r.centre_distance
        );
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
            ..SpurStage::default().gears[0].clone()
        };
        let tuned = |gears: [StageGear; 2]| SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            gears,
            ..SpurStage::default()
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
        let auto = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();

        // The same distance, set by hand, with a clearance that must be ignored.
        let manual = solve_spur_stage(
            &SpurStage {
                centre_distance: Auto::fixed(auto.centre_distance_nominal),
                clearance: 0.5,
                ..SpurStage::default()
            },
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();

        assert!((manual.centre_distance - auto.centre_distance_nominal).abs() < 1e-12);
        // At the zero-backlash distance there is, by construction, no backlash.
        assert!(manual.backlash.forward.nominal.abs() < 1e-9);
        // Whereas the automatic one carries its clearance into real backlash.
        assert!(auto.backlash.forward.nominal > 0.0);
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
        let off = LoadCase {
            peak: false,
            cyclic: false,
        };
        let auto_width = |sources: FaceSources, o: Overrides| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
                g.material_overrides = o;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib).unwrap()
        };
        let contact_only = |case: Case| FaceSources {
            bending: off,
            contact: LoadCase::of(|c| c == case),
        };

        for (case, other) in [(Case::Cyclic, Case::Peak), (Case::Peak, Case::Cyclic)] {
            let base = auto_width(contact_only(case), Overrides::default());
            // Twice **this material's** figure, read from the answer rather than
            // written down again: a test that repeats the library's numbers
            // stops testing the arithmetic the moment the library moves.
            let doubled = 2.0 * allowable(&base.gears[0].material, case);
            let over = match case {
                Case::Peak => Overrides {
                    ultimate_allowable: Some(doubled),
                    ..Default::default()
                },
                Case::Cyclic => Overrides {
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
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.material_overrides = Overrides {
                    elastic_modulus: e,
                    ..Default::default()
                };
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib)
                .unwrap()
                .contact_stress_at_pitch_point
                .peak
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
        let e = solve_spur_stage(&s, StageTorques::just(2.0), &library()).unwrap_err();
        assert!(matches!(e, TrainError::UnknownMaterial(ref n) if n == "unobtainium"));
        assert!(e.to_string().contains("unobtainium"));
    }

    #[test]
    fn an_empty_train_says_so() {
        let t = Train {
            input_speed: 1.0,
            input_torque: 1.0,
            back_driving_torque: 0.0,
            operating_torque: 1.0,
            reversed_bending: false,
            actuation: Actuation::default(),
            stages: vec![],
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
        for (train, what) in [
            (two_stage(), "two spur stages"),
            (
                Train {
                    actuation: Actuation::Continuous {
                        operating_speed: 2400.0,
                        runtime_hours: 1000.0,
                    },
                    ..two_stage()
                },
                "continuous",
            ),
            (
                Train {
                    reversed_bending: false,
                    actuation: Actuation::Intermittent {
                        range_degrees: 25.0,
                        actuations: 1000,
                        reversing: false,
                    },
                    ..two_stage()
                },
                "intermittent",
            ),
            (
                Train {
                    reversed_bending: false,
                    actuation: Actuation::Intermittent {
                        range_degrees: 25.0,
                        actuations: 1000,
                        reversing: true,
                    },
                    ..two_stage()
                },
                "reversing",
            ),
        ] {
            let r = solve_train(&train, &lib).expect(what);

            // The revolutions each member turns, before anything rounds them.
            let ratios: Vec<f64> = r.stages.iter().map(StageResult::ratio).collect();
            for (k, s) in r.stages.iter().enumerate() {
                let upstream: f64 = ratios[..k].iter().product();
                let speed_in = train.input_speed / upstream;
                let speeds = [speed_in, speed_in / ratios[k]];
                let expected = [0usize, 1].map(|i| {
                    let to_output: f64 = if i == 0 {
                        ratios[k..].iter().product()
                    } else {
                        ratios[k + 1..].iter().product()
                    };
                    match train.actuation {
                        Actuation::Intermittent {
                            range_degrees,
                            actuations,
                            reversing,
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
                        Actuation::Continuous {
                            operating_speed,
                            runtime_hours,
                        } => {
                            let scale = operating_speed / train.input_speed;
                            let n = (speeds[i] * scale * 60.0 * runtime_hours).ceil();
                            Cycles {
                                bending: n,
                                contact: n,
                            }
                        }
                    }
                });

                let Some(sp) = s.as_spur() else { continue };
                for (i, (g, want)) in sp.gears.iter().zip(expected).enumerate() {
                    let got = g.tooth_cycles;
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
            let mut stage = SpurStage::default();
            for (g, o) in stage.gears.iter_mut().zip(over) {
                g.face_width = Auto::automatic(0.0);
                g.material_overrides = o;
            }
            let r = solve_spur_stage(&stage, StageTorques::just(2.0), &lib).unwrap();
            let effective = r.gears[0].face_width.min(r.gears[1].face_width);
            assert!(effective > 0.0);

            for (i, g) in r.gears.iter().enumerate() {
                let sources = &stage.gears[i].face_sources;
                for case in [Case::Peak, Case::Cyclic] {
                    let asks = g.min_face_width.get(case);
                    if *sources.contact.get(case) {
                        assert!(
                            effective >= asks.contact * (1.0 - 1e-9),
                            "gear {i} {case:?} contact needs {} mm, mesh carries {effective}",
                            asks.contact
                        );
                    }
                    if let (true, Some(b)) = (*sources.bending.get(case), asks.bending) {
                        assert!(
                            effective >= b * (1.0 - 1e-9),
                            "gear {i} {case:?} bending needs {b} mm, mesh carries {effective}"
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
            let mut s = SpurStage::default();
            for (g, o) in s.gears.iter_mut().zip(over) {
                g.face_width = if auto {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(10.0)
                };
                g.material_overrides = o;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib).unwrap()
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
        let pitch = |r: &SpurResult| r.contact_stress_at_pitch_point.peak;
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
            base.gears[0].contact_stress.peak,
            base.gears[1].contact_stress.peak,
        );
        assert!(
            a != b,
            "17/43 is not symmetric, so its two gears are not rated alike: {a} and {b}"
        );
        for g in &base.gears {
            assert!(g.contact_stress.peak >= pitch(&base));
        }
        // ...and the envelope is the worse of them, which is what the *mesh*
        // would be rated on with no member named.
        assert!((a.max(b) - base.gears[0].contact_stress.peak.max(b)).abs() < 1e-12);

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
        let half = allowable(0.5 * super::allowable(&wide.gears[1].material, Case::Cyclic));
        let derated = solved(false, [Overrides::default(), half]);
        assert_eq!(
            derated.contact_stress_at_pitch_point, wide.contact_stress_at_pitch_point,
            "an allowable is not a stress and must not move one"
        );
        let (was, now) = (
            wide.gears[1].min_face_width.cyclic.contact,
            derated.gears[1].min_face_width.cyclic.contact,
        );
        assert!(
            (now / was - 4.0).abs() < 1e-9,
            "halving the allowable should quadruple the width: {was} to {now}"
        );
        assert_eq!(
            derated.gears[0].min_face_width.cyclic.contact,
            wide.gears[0].min_face_width.cyclic.contact,
            "and it must not reach the other gear"
        );
    }

    /// **A load exists only where it is reacted.**
    ///
    /// A back-driving torque on a train of ordinary spur stages reaches no
    /// number: every stage can be driven backward, so the load simply turns the
    /// train and nothing holds it. Put one self-locking stage in the way and the
    /// load stops there — that stage and everything downstream of it carry it,
    /// and everything upstream still sees nothing.
    #[test]
    fn a_back_driving_load_is_carried_only_where_something_reacts_it() {
        let lib = test_library();
        let mut t = two_stage();
        t.back_driving_torque = 5.0;
        let r = solve_train(&t, &lib).unwrap();
        for s in &r.stages {
            for g in &spur(s).gears {
                assert_eq!(
                    g.back_driving_torque, None,
                    "a back-drivable train reacts nothing"
                );
            }
        }

        // The same load against a stage that cannot be driven backward. A worm
        // with enough friction locks, and then the load stops there: the worm
        // stage carries it, and the spur stage ahead of it carries none.
        t.stages.push(Stage::Worm(WormStage {
            sliding_friction: 0.3,
            static_friction: 0.3,
            ..WormStage::default()
        }));
        let r = solve_train(&t, &lib).unwrap();
        let worm = r.stages[2].as_worm().expect("the third stage is a worm");
        assert!(
            worm.efficiency.self_locking(),
            "this worm was meant to lock: backward efficiency {}",
            worm.efficiency.backward
        );
        for m in &worm.members {
            assert!(
                m.back_driving_torque.is_some_and(|t| t > 0.0),
                "the stage that reacts the load carries it"
            );
        }
        for s in &r.stages[..2] {
            for g in &spur(s).gears {
                assert_eq!(
                    g.back_driving_torque, None,
                    "nothing upstream of a self-locking stage sees the load"
                );
            }
        }

        // ...and the train says which stage held it, rather than leaving the
        // reader to infer it from a column of dashes.
        assert!(r
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_BACK_DRIVING_REACTED_AT)));

        // **The two torques are two facts, and the forward one is not the peak
        // case.** Put the locking stage first, so the spur stages after it carry
        // the load as well: a gear's `torque` must stay the torque it sees
        // driving forward even when the back-driving figure is the larger of the
        // two, which is what the peak *rating* uses.
        let mut t = two_stage();
        t.back_driving_torque = 500.0;
        t.stages.insert(
            0,
            Stage::Worm(WormStage {
                sliding_friction: 0.3,
                static_friction: 0.3,
                ..WormStage::default()
            }),
        );
        let r = solve_train(&t, &lib).unwrap();
        let forward_only = solve_train(
            &Train {
                back_driving_torque: 0.0,
                ..t.clone()
            },
            &lib,
        )
        .unwrap();
        let mut seen = 0;
        for (loaded, plain) in r.stages[1..].iter().zip(&forward_only.stages[1..]) {
            for (g, unloaded) in spur(loaded).gears.iter().zip(&spur(plain).gears) {
                let back = g
                    .back_driving_torque
                    .expect("carried downstream of the lock");
                assert!(back > g.torque, "this fixture is meant to load it backward");
                assert_eq!(
                    g.torque, unloaded.torque,
                    "a back-driving load must not move the forward torque"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 4, "both spur stages, both gears");
    }
}
