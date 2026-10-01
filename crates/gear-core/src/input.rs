//! **What an input may hold** — one table, a row per number of every kind of
//! input: the field's name on the wire, the bound its value is held to, and
//! what is done with a value past it.
//!
//! Every number that enters — a gear's parameters, a cutter, a train's
//! graph, holds and load cases, a material and its per-gear replacements —
//! is read against its row once, where it enters: a file read
//! (`gear_io`), every wasm entry, the CLI, and the core's own public
//! entries ([`crate::train::solve_train`], [`crate::train::Train::validate`],
//! [`crate::train::Shape::validate`]). The rows are the bounds the panel
//! shows ([`crate::auto::Ranges`] reads its invariant bounds from here), so a
//! box and the core cannot disagree about what a field may hold.
//!
//! # What a row does with a value past its bound
//!
//! Rule 5: a value that describes nothing is **refused**, naming the field
//! by its path ([`Refused`]); a value that describes something that can
//! nearly be made is **held** at the bound where the model reads it, and the
//! model says so ([`Held`]: a pressure angle below half a degree, an
//! application factor below one). A value no field means anything at — a
//! not-a-number, an infinity — is refused in every row, and an index is
//! refused where it names nothing in its list.
//!
//! # What a row does not know
//!
//! A bound that depends on the whole geometry — the shift that keeps a tooth
//! off its root, the dedendum the rack reaches — is the generator's to clamp
//! and say ([`crate::auto::admissible_ranges`]); a graph that describes no
//! train, whatever its numbers, is [`crate::train::Shape::invariants`]'.

use crate::auto::Bound;
use crate::material::{Material, MaterialLibrary, Overrides};
use crate::note::{key, Explain, Note};
use crate::params::GearParams;
use crate::ring::Cutter;
use crate::train::shape::{Axis, BodyOn, Distance, Member, MeshInput, Shape};
use crate::train::{Duty, Load, LoadCase, Train};

// ------------------------------------------------------------ the scale ---

/// **The degree of the widest product the model forms before a root or a
/// quotient brings it back**: the contact width's `b (σ_H/σ_allow)²`, where
/// `σ_H² = F·E*/(b·ρ)` and `F = T/r` — a torque, a modulus, three lengths
/// and an allowable twice, then the width again: eight figures' exponents.
pub const PRODUCT_DEGREE: u64 = 8;

/// **The binary exponent every figure's magnitude is held within**, ±127:
/// a product of [`PRODUCT_DEGREE`] figures each within `2^±E` lies within
/// `2^±8E`, a normal double where `8E ≤ 1022`, the normal doubles' own
/// exponent range. A figure is a length, a torque, a speed, a duration, a
/// stress, a friction, a hardness, a coefficient in modules; a count and
/// an angle have their own bounds.
pub const SCALE_EXPONENT: u64 = 1022 / PRODUCT_DEGREE;

/// `f64`'s exponent bias: the stored exponent of 1.
const EXPONENT_BIAS: u64 = 1023;

/// **The largest magnitude a figure may have**, `2^127` ≈ 1.7e38.
pub const CEILING: f64 = f64::from_bits((EXPONENT_BIAS + SCALE_EXPONENT) << 52);

/// **The smallest non-zero magnitude a figure may have**, `2^−127` ≈ 5.9e−39.
pub const FLOOR: f64 = f64::from_bits((EXPONENT_BIAS - SCALE_EXPONENT) << 52);

/// **The smallest module the model is built for**, 1 µm: where its
/// compatibility tolerance ([`crate::params::compat::SAME_RACK`], 1e−9 mm)
/// stops being what it is written as, a millionth of anything a designer
/// could mean ([`crate::params::compat::DESIGN_MARGIN`]). Below it the
/// model's absolute tolerances are no longer small beside the teeth: the
/// homogeneity law (`train/homogeneity.rs`) holds every preset from it up.
pub const SMALLEST_MODULE: f64 =
    crate::params::compat::SAME_RACK * crate::params::compat::DESIGN_MARGIN;

// ------------------------------------------------------------- the bounds ---

/// `m`: at least [`SMALLEST_MODULE`], at most [`CEILING`].
pub const MODULE: Bound = Bound::between(Some(SMALLEST_MODULE), Some(CEILING));
/// `0 < α < 90°`: at nought the thickness-equivalent shift diverges, at 90°
/// the base circle does.
pub const PRESSURE_ANGLE: Bound = Bound::strictly(0.0, 90.0);
/// `|β| < 90°`: the transverse module diverges at the limit.
pub const HELIX_ANGLE: Bound = Bound::strictly(-90.0, 90.0);
/// **An angle between two axes, one period**, degrees: a turn more is the
/// same pair of axes.
pub const SHAFT_ANGLE: Bound = Bound::between(Some(-180.0), Some(180.0));
/// `0 < k < 2`: a rack whose tooth or space has no width is not a rack.
pub const THICKNESS_MOD: Bound = Bound::strictly(0.0, 2.0);
/// **The bound on every count**: a gear's or a cutter's teeth, an axis's
/// planets, a duty's actuations. At least one, since none is no gear, no
/// tool, no axis and no duty; at most what the wire's `u32` carries.
pub const COUNT: Bound = Bound::between(Some(1.0), Some(u32::MAX as f64));
/// A figure of either sign: a shift, an addendum, a torque, a speed.
pub const FIGURE: Bound = Bound::between(Some(-CEILING), Some(CEILING));
/// A figure that may be nought and not less: a clearance a floor is asked
/// at, a tolerance's size, a friction coefficient.
pub const NOT_NEGATIVE: Bound = Bound::between(Some(0.0), Some(CEILING));
/// A figure that has to be there to mean anything: a face, a rim, a pin, a
/// distance, a modulus, an allowable.
pub const POSITIVE: Bound = Bound::between(Some(FLOOR), Some(CEILING));
/// **Poisson's ratio of an isotropic solid**, `−1 < ν ≤ ½`: its bulk and
/// shear moduli positive. The incompressible limit is a solid — its bulk
/// modulus `E / 3(1 − 2ν)` is unbounded, and the contact model multiplies
/// by `1 − 2ν` and never divides by it (`hertz.rs`). The one bound on ν:
/// the first-yield factor reads it too.
pub const POISSONS_RATIO: Bound = Bound {
    min: Some(-1.0),
    max: Some(0.5),
    exclusive_min: true,
    exclusive_max: false,
};

/// Where the tooth's flank stops being usable: below it the pressure angle
/// is held here and the tooth says so (`clamp.pressure_angle_raised`).
pub const PRESSURE_ANGLE_HELD: Bound =
    Bound::between(Some(crate::params::guard::MIN_PRESSURE_ANGLE_DEG), None);
/// `K_A ≥ 1` by its definition: a factor below it is held at 1 and the case
/// says so (`train.application_factor_held`).
pub const APPLICATION_FACTOR_HELD: Bound = Bound::between(Some(1.0), None);

// ---------------------------------------------------------------- a row ---

/// **What the model does with a value inside a row's bound but past this
/// one**: holds it here where it reads it, and says `note`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Held {
    pub to: Bound,
    pub note: &'static str,
}

/// The lengths of the lists an index is read against. A gear's rows read
/// none of them.
#[derive(Clone, Copy, Debug, Default)]
pub struct Lists {
    pub axes: usize,
    pub bodies: usize,
    pub members: usize,
    /// A train with no gear: its load cases wait, by number, for the bodies
    /// the next arrangement brings.
    pub empty: bool,
}

impl Lists {
    fn of(shape: &Shape) -> Self {
        Self {
            axes: shape.axes.len(),
            bodies: shape.bodies.len(),
            members: shape.members.len(),
            empty: shape.members.is_empty(),
        }
    }
}

/// **One row**: a number of a record of type `T`, where it is on the wire
/// relative to the record, the bound it is held to (`None` only where the
/// row does not read the number: an automatic box's seed, which must still
/// be finite), and what the model holds it to inside that.
pub struct Row<T> {
    pub field: &'static str,
    /// What the number is: a figure, or an index — a list's position or a
    /// body's number — which is read, for every record, before any figure
    /// is, since nothing can be read off a graph whose indices name nothing.
    pub kind: Kind,
    /// The number, or `None` where the field is an option not given.
    pub get: fn(&T) -> Option<f64>,
    pub bound: fn(&Lists, &T) -> Option<Bound>,
    pub held: Option<Held>,
}

/// **What a row's number is.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A figure: a length, an angle, a count, a ratio, a load.
    Figure,
    /// A position in one of the input's lists, or a body's number: what
    /// the graph is read by.
    Index,
}

/// A row with a fixed bound and nothing held.
macro_rules! row {
    ($field:literal, $get:expr, $bound:expr) => {
        Row {
            field: $field,
            kind: Kind::Figure,
            get: $get,
            bound: |_, _| Some($bound),
            held: None,
        }
    };
}

/// A row of an [`crate::params::Auto`]'s number held to its bound only
/// where it is given: an automatic box holds the solve's seed, which the
/// solve replaces before it reads it — nought, often, where nothing has
/// been solved yet. Finite either way.
macro_rules! given {
    ($field:literal, $auto:ident $(. $path:ident)*, $bound:expr) => {
        Row {
            field: $field,
            kind: Kind::Figure,
            get: |x| Some(x.$auto$(.$path)*.manual),
            bound: |_, x| (!x.$auto$(.$path)*.auto).then_some($bound),
            held: None,
        }
    };
}

/// The index bound on a list of `len`: `0 ..= len − 1`, and nothing where
/// the list is empty.
fn index(len: usize) -> Bound {
    Bound::between(Some(0.0), Some(len as f64 - 1.0))
}

/// A body's number: `1 ..= n`, the train's bodies numbered from one
/// without a gap.
fn body(n: usize) -> Bound {
    Bound::between(Some(1.0), Some(n as f64))
}

/// A body's number or ground's, `0 ..= n`.
fn body_or_ground(n: usize) -> Bound {
    Bound::between(Some(0.0), Some(n as f64))
}

/// A body a load case names: a listed one, or on a train with no gear any
/// number past ground, since the case waits for the bodies to come.
fn case_body(l: &Lists) -> Bound {
    if l.empty {
        COUNT
    } else {
        body(l.bodies)
    }
}

// ------------------------------------------------------------ the table ---

/// A gear's parameters ([`GearParams`]), as the gear tab and a train's
/// member cut them.
pub const GEAR: &[Row<GearParams>] = &[
    row!("module", |p| Some(p.module), MODULE),
    Row {
        field: "pressure_angle",
        kind: Kind::Figure,
        get: |p| Some(p.pressure_angle),
        bound: |_, _| Some(PRESSURE_ANGLE),
        held: Some(Held {
            to: PRESSURE_ANGLE_HELD,
            note: key::CLAMP_PRESSURE_ANGLE_RAISED,
        }),
    },
    row!("teeth", |p| Some(f64::from(p.teeth)), COUNT),
    row!("profile_shift", |p| Some(p.profile_shift), FIGURE),
    row!("helix_angle", |p| Some(p.helix_angle), HELIX_ANGLE),
    row!("addendum", |p| Some(p.addendum), FIGURE),
    row!("dedendum", |p| Some(p.dedendum), FIGURE),
    // A tool's tip round is a radius: nought is a sharp tool (held to the
    // least the cut admits, `Rack`), and one below nought describes none.
    row!("root_radius", |p| Some(p.root_radius), NOT_NEGATIVE),
    row!("thickness_mod", |p| Some(p.thickness_mod), THICKNESS_MOD),
    row!("angular_shift", |p| Some(p.angular_shift), FIGURE),
    row!("index_offset", |p| Some(p.index_offset), FIGURE),
];

/// A pinion cutter ([`Cutter`]): a ring's tool.
pub const CUTTER: &[Row<Cutter>] = &[
    row!("teeth", |c| Some(f64::from(c.teeth)), COUNT),
    row!("addendum", |c| Some(c.addendum), FIGURE),
    row!("tip_round", |c| Some(c.tip_round), NOT_NEGATIVE),
];

/// An axis ([`Axis`]).
pub const AXIS: &[Row<Axis>] = &[
    Row {
        field: "carried_by",
        kind: Kind::Index,
        get: |a| Some(a.carried_by as f64),
        bound: |l, _| Some(body_or_ground(l.bodies)),
        held: None,
    },
    row!("count", |a| Some(f64::from(a.count)), COUNT),
    row!(
        "min_planet_clearance",
        |a| Some(a.min_planet_clearance),
        NOT_NEGATIVE
    ),
];

/// A body on an axis ([`BodyOn`]).
pub const BODY: &[Row<BodyOn>] = &[
    Row {
        field: "body",
        kind: Kind::Index,
        get: |b| Some(b.body as f64),
        bound: |l, _| Some(body(l.bodies)),
        held: None,
    },
    Row {
        field: "axis",
        kind: Kind::Index,
        get: |b| Some(b.axis as f64),
        bound: |l, _| Some(index(l.axes)),
        held: None,
    },
];

/// A train's member ([`Member`]) and its gear. Its ring's cutter is
/// [`CUTTER`]'s, under `ring`; its replaced material figures
/// [`OVERRIDES`]', under `gear.material_overrides`.
pub const MEMBER: &[Row<Member>] = &[
    Row {
        field: "body",
        kind: Kind::Index,
        get: |m| Some(m.body as f64),
        bound: |l, _| Some(body(l.bodies)),
        held: None,
    },
    row!("module.manual", |m| Some(m.module.manual), MODULE),
    Row {
        field: "pressure_angle.manual",
        kind: Kind::Figure,
        get: |m| Some(m.pressure_angle.manual),
        bound: |_, _| Some(PRESSURE_ANGLE),
        held: Some(Held {
            to: PRESSURE_ANGLE_HELD,
            note: key::CLAMP_PRESSURE_ANGLE_RAISED,
        }),
    },
    given!("thickness_mod.manual", thickness_mod, THICKNESS_MOD),
    given!("pitch_diameter.manual", pitch_diameter, POSITIVE),
    row!("gear.teeth", |m| Some(f64::from(m.gear.teeth)), COUNT),
    row!(
        "gear.profile_shift.manual",
        |m| Some(m.gear.profile_shift.manual),
        FIGURE
    ),
    row!(
        "gear.working_depth.manual",
        |m| Some(m.gear.working_depth.manual),
        FIGURE
    ),
    row!("gear.addendum", |m| Some(m.gear.addendum), FIGURE),
    row!(
        "gear.min_tip_width",
        |m| Some(m.gear.min_tip_width),
        NOT_NEGATIVE
    ),
    row!("gear.dedendum", |m| Some(m.gear.dedendum), FIGURE),
    row!(
        "gear.root_radius",
        |m| Some(m.gear.root_radius),
        NOT_NEGATIVE
    ),
    given!("gear.helix_angle.manual", gear.helix_angle, HELIX_ANGLE),
    // Read where given, and where no rating sizes it: an automatic width
    // with no source stands at its box (`FaceSources::width_for`).
    Row {
        field: "gear.face_width.manual",
        kind: Kind::Figure,
        get: |m| Some(m.gear.face_width.manual),
        bound: |_, m| (!m.gear.face_width.auto || !m.gear.face_sources.any()).then_some(POSITIVE),
        held: None,
    },
    row!("gear.rim_thickness", |m| m.gear.rim_thickness, POSITIVE),
];

/// A mesh ([`MeshInput`]).
pub const MESH: &[Row<MeshInput>] = &[
    Row {
        field: "a",
        kind: Kind::Index,
        get: |k| Some(k.a as f64),
        bound: |l, _| Some(index(l.members)),
        held: None,
    },
    Row {
        field: "b",
        kind: Kind::Index,
        get: |k| Some(k.b as f64),
        bound: |l, _| Some(index(l.members)),
        held: None,
    },
    row!(
        "sliding_friction",
        |k| Some(k.sliding_friction),
        NOT_NEGATIVE
    ),
    row!("static_friction", |k| Some(k.static_friction), NOT_NEGATIVE),
    row!("overlap.manual", |k| Some(k.overlap.manual), NOT_NEGATIVE),
    row!("min_contact_ratio", |k| Some(k.min_contact_ratio), POSITIVE),
];

/// An axis distance ([`Distance`]).
pub const DISTANCE: &[Row<Distance>] = &[
    Row {
        field: "axes.0",
        kind: Kind::Index,
        get: |d| Some(d.axes[0] as f64),
        bound: |l, _| Some(index(l.axes)),
        held: None,
    },
    Row {
        field: "axes.1",
        kind: Kind::Index,
        get: |d| Some(d.axes[1] as f64),
        bound: |l, _| Some(index(l.axes)),
        held: None,
    },
    row!("angle", |d| Some(d.angle), SHAFT_ANGLE),
    given!("distance.manual", distance, POSITIVE),
    row!("clearance.manual", |d| Some(d.clearance.manual), FIGURE),
    row!("tip_clearance", |d| Some(d.tip_clearance), NOT_NEGATIVE),
    row!("tolerance_plus", |d| Some(d.tolerance_plus), NOT_NEGATIVE),
    row!("tolerance_minus", |d| Some(d.tolerance_minus), NOT_NEGATIVE),
    row!("axial_clearance", |d| Some(d.axial_clearance), NOT_NEGATIVE),
];

/// An offset coupling's two ends: two bodies.
pub const COUPLING: &[Row<[usize; 2]>] = &[
    Row {
        field: "0",
        kind: Kind::Index,
        get: |c| Some(c[0] as f64),
        bound: |l, _| Some(body(l.bodies)),
        held: None,
    },
    Row {
        field: "1",
        kind: Kind::Index,
        get: |c| Some(c[1] as f64),
        bound: |l, _| Some(body(l.bodies)),
        held: None,
    },
];

/// A body the train holds.
pub const HOLD: &[Row<usize>] = &[Row {
    field: "",
    kind: Kind::Index,
    get: |&b| Some(b as f64),
    bound: |l, _| Some(body(l.bodies)),
    held: None,
}];

/// A load case ([`LoadCase`]); its loads are [`LOAD`]'s and its duty
/// [`DUTY`]'s.
pub const CASE: &[Row<LoadCase>] = &[Row {
    field: "application_factor",
    kind: Kind::Figure,
    get: |c| Some(c.application_factor),
    bound: |_, _| Some(FIGURE),
    held: Some(Held {
        to: APPLICATION_FACTOR_HELD,
        note: key::TRAIN_APPLICATION_FACTOR_HELD,
    }),
}];

/// One load of a case ([`Load`]).
pub const LOAD: &[Row<Load>] = &[
    Row {
        field: "at",
        kind: Kind::Index,
        get: |l| Some(l.at as f64),
        bound: |l, _| Some(case_body(l)),
        held: None,
    },
    row!("torque.manual", |l| Some(l.torque.manual), FIGURE),
    row!("speed.manual", |l| Some(l.speed.manual), FIGURE),
];

/// A fatigue case's duty ([`Duty`]), each variant's under its own name.
pub const DUTY: &[Row<Duty>] = &[
    row!(
        "intermittent.range_degrees",
        |d| match *d {
            Duty::Intermittent { range_degrees, .. } => Some(range_degrees),
            Duty::Continuous { .. } => None,
        },
        POSITIVE
    ),
    Row {
        field: "intermittent.at",
        kind: Kind::Index,
        get: |d| match *d {
            Duty::Intermittent { at, .. } => at.map(|a| a as f64),
            Duty::Continuous { .. } => None,
        },
        bound: |l, _| Some(case_body(l)),
        held: None,
    },
    row!(
        "intermittent.actuations",
        |d| match *d {
            Duty::Intermittent { actuations, .. } => Some(f64::from(actuations)),
            Duty::Continuous { .. } => None,
        },
        COUNT
    ),
    row!(
        "continuous.runtime_hours",
        |d| match *d {
            Duty::Continuous { runtime_hours } => Some(runtime_hours),
            Duty::Intermittent { .. } => None,
        },
        POSITIVE
    ),
];

/// A material ([`Material`]): each figure's `value`.
pub const MATERIAL: &[Row<Material>] = &[
    row!("density.value", |m| Some(m.density.value), POSITIVE),
    row!(
        "elastic_modulus.value",
        |m| Some(m.elastic_modulus.value),
        POSITIVE
    ),
    row!(
        "poissons_ratio.value",
        |m| Some(m.poissons_ratio.value),
        POISSONS_RATIO
    ),
    row!(
        "ultimate_allowable.value",
        |m| Some(m.ultimate_allowable.value),
        POSITIVE
    ),
    Row {
        field: "fatigue_allowable.value",
        kind: Kind::Figure,
        get: |m| Some(m.fatigue_allowable.value),
        bound: |_, m| Some(cyclic(m.ultimate_allowable.value)),
        held: None,
    },
    row!(
        "contact_fatigue_allowable.value",
        |m| m.contact_fatigue_allowable.as_ref().map(|v| v.value),
        POSITIVE
    ),
    row!(
        "contact_estimate.hardness.value",
        |m| m.contact_estimate.as_ref().map(|e| e.hardness.value),
        POSITIVE
    ),
];

/// **A cyclic allowable**: [`POSITIVE`]'s floor, and no more than the static one it is
/// a fraction of — a root that survives more cycles than one at a higher
/// stress than it survives once describes no material.
fn cyclic(ultimate: f64) -> Bound {
    Bound::between(Some(FLOOR), Some(ultimate))
}

/// A member's replaced material figures ([`Overrides`]), each held to its
/// material's row where given.
pub const OVERRIDES: &[Row<Overrides>] = &[
    row!("density", |o| o.density, POSITIVE),
    row!("elastic_modulus", |o| o.elastic_modulus, POSITIVE),
    row!("poissons_ratio", |o| o.poissons_ratio, POISSONS_RATIO),
    row!("ultimate_allowable", |o| o.ultimate_allowable, POSITIVE),
    row!("fatigue_allowable", |o| o.fatigue_allowable, POSITIVE),
    row!(
        "contact_fatigue_allowable",
        |o| o.contact_fatigue_allowable,
        POSITIVE
    ),
];

/// **A figure a request asks beside the gear**, by its name on the wire:
/// a measurement's pin, a drawing's chord tolerance and point count, the
/// depth an undercut is asked at, an eccentric gear's throw and its mate.
pub type Asked = Row<Option<f64>>;

/// The pin or ball measured over: it has to be there.
pub const PIN_DIAMETER: Asked = row!("pin_diameter", |v| *v, POSITIVE);
/// How far a drawn outline may stand off its curve, mm.
pub const CHORD_TOLERANCE: Asked = row!("chord_tolerance", |v| *v, POSITIVE);
/// The depth, in modules, an undercut is asked at.
pub const WORKING_DEPTH: Asked = row!("working_depth", |v| *v, FIGURE);
/// An eccentric gear's commanded centre-distance throw, mm, signed.
pub const ECCENTRIC_THROW: Asked = row!("eccentric_throw", |v| *v, FIGURE);
/// The mate's teeth.
pub const MATE_TEETH: Asked = row!("mate.teeth", |v| *v, COUNT);
/// The mate's shift.
pub const MATE_SHIFT: Asked = row!("mate.profile_shift", |v| *v, FIGURE);
/// **Points a tooth a screen profile is drawn with**: at least one, and at
/// most [`POINTS_PER_TOOTH_MAX`].
pub const POINTS_PER_TOOTH: Asked = row!(
    "points_per_tooth",
    |v| *v,
    Bound::between(Some(1.0), Some(POINTS_PER_TOOTH_MAX))
);

/// **The most points a tooth's screen profile is drawn with**, 2¹⁶. The
/// profile exists to be drawn on a screen, and a tooth's outline drawn
/// across the largest display runs to some 2·10⁴ pixels (three widths of
/// an 8K panel), so 2¹⁶ points is past one a pixel on any of them; a
/// number past it asks for memory and for nothing the eye can see.
pub const POINTS_PER_TOOTH_MAX: f64 = 65_536.0;

/// **One asked figure read against its row**, at `at`; `None` (not asked)
/// passes.
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn asked(row: &Asked, value: Option<f64>, at: &str) -> Result<(), Refused> {
    check(std::slice::from_ref(row), &Lists::default(), &value, at)
}

// --------------------------------------------------------- the refusal ---

/// **A value refused where it entered**, naming the field by its path on
/// the wire — the record's keys and list indices from what was checked,
/// joined by `.` (`shape.members.2.gear.teeth`). An entry that checks a
/// record inside a request says where the record is ([`Self::within`]).
#[derive(Clone, Debug)]
pub struct Refused {
    pub field: String,
    pub value: f64,
    pub reason: Reason,
}

/// **Why a value was refused.**
#[derive(Clone, Copy, Debug)]
pub enum Reason {
    /// Not a finite number, which no field holds.
    NotFinite,
    /// Outside its row's bound.
    Outside(Bound),
    /// **An output past its budget** ([`Budget`]): what the value would
    /// have the model build — a drawing's points, a DXF's text, an eccentric
    /// gear's teeth — is `size` bytes, and the budget `budget`. Predicted
    /// from the value before anything is built, so it is the same refusal
    /// in every build.
    PastBudget { size: u64, budget: u64 },
    /// **Within its budget, and still more than the machine's memory
    /// holds**: the allocator's refusal, where memory is short of what the
    /// budget admits.
    PastMemory,
}

/// Equal where every figure is the same double, a not-a-number included:
/// two refusals of one value at one field are one refusal.
impl PartialEq for Refused {
    fn eq(&self, other: &Self) -> bool {
        let reason = |r: &Reason| match r {
            Reason::NotFinite => (0, None),
            Reason::Outside(b) => (
                1,
                Some((
                    b.min.map(f64::to_bits),
                    b.max.map(f64::to_bits),
                    b.exclusive_min,
                    b.exclusive_max,
                )),
            ),
            Reason::PastBudget { size, budget } => {
                (2, Some((Some(*size), Some(*budget), false, false)))
            }
            Reason::PastMemory => (3, None),
        };
        self.field == other.field
            && self.value.to_bits() == other.value.to_bits()
            && reason(&self.reason) == reason(&other.reason)
    }
}

impl Eq for Refused {}

impl Refused {
    /// The same refusal, its field placed under `prefix` — the record's
    /// path in the request that held it.
    #[must_use]
    pub fn within(self, prefix: &str) -> Self {
        Self {
            field: join(prefix, &self.field),
            ..self
        }
    }

    /// **An output of `size` bytes past a budget of `budget`**, refused
    /// naming the `field` whose value `n` asked for it.
    #[must_use]
    pub fn past_budget(field: &str, n: f64, size: u64, budget: u64) -> Self {
        Self {
            field: field.to_owned(),
            value: n,
            reason: Reason::PastBudget { size, budget },
        }
    }

    /// **A list `n` long that the machine's memory could not hold**,
    /// refused naming `field` (a gear's `teeth`), its value `n`.
    #[must_use]
    pub fn past_memory(field: &str, n: f64) -> Self {
        Self {
            field: field.to_owned(),
            value: n,
            reason: Reason::PastMemory,
        }
    }

    /// The bound it is outside, where that is why.
    #[must_use]
    pub fn bound(&self) -> Option<Bound> {
        match self.reason {
            Reason::Outside(b) => Some(b),
            Reason::NotFinite | Reason::PastBudget { .. } | Reason::PastMemory => None,
        }
    }
}

impl std::fmt::Display for Refused {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.reason {
            Reason::NotFinite => write!(f, "{} is not a finite number", self.field),
            Reason::Outside(b) => write!(
                f,
                "{} is {}, outside {}",
                self.field,
                figure(self.value),
                interval(&b)
            ),
            Reason::PastBudget { size, budget } => write!(
                f,
                "{} is {}: the output would be {size} bytes, past its budget of {budget}",
                self.field,
                figure(self.value)
            ),
            Reason::PastMemory => write!(
                f,
                "{} is {}: more than this machine's memory holds",
                self.field,
                figure(self.value)
            ),
        }
    }
}

impl std::error::Error for Refused {}

impl Explain for Refused {
    fn note(&self) -> Note {
        let field = self.field.clone();
        match self.reason {
            Reason::NotFinite => Note::new(key::ERROR_INPUT_NOT_FINITE).text("field", field),
            Reason::Outside(b) => Note::new(key::ERROR_INPUT_OUT_OF_RANGE)
                .text("field", field)
                .text("value", figure(self.value))
                .text("bound", interval(&b)),
            Reason::PastBudget { size, budget } => Note::new(key::ERROR_OUTPUT_PAST_BUDGET)
                .text("field", field)
                .text("value", figure(self.value))
                .text("size", size.to_string())
                .text("budget", budget.to_string()),
            Reason::PastMemory => Note::new(key::ERROR_INPUT_PAST_MEMORY)
                .text("field", field)
                .text("value", figure(self.value)),
        }
    }
}

// ---------------------------------------------------------- the budget ---

/// **The most one output may take, in bytes as the core builds it** — a
/// drawing's points or vertices, a DXF's text, an eccentric gear's teeth
/// and the centre distance read at each. A setting, stated and
/// overridable (every function that builds one is handed it; the harness
/// reads `GEARCALC_OUTPUT_BUDGET`), and not engineering: it bounds what one
/// request may have the machine build.
///
/// It is checked from the size an output **will** have, predicted from the
/// value that asks for it before anything is built — so the refusal is the
/// same in every build and never the allocator's to give: Linux's
/// overcommit grants a list its memory cannot back, and then the process is
/// ended by whatever notices, not refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Budget {
    pub bytes: usize,
}

impl Budget {
    /// **2²⁷ bytes, 128 MiB**, on two bases, both met. As a drawing, 2²⁷
    /// bytes of points (`[f64; 2]`, 16 bytes each) is 8 388 608 of them, the
    /// smallest power of two holding one for every pixel of a 4K display
    /// (3840 × 2160 = 8 294 400): past it no screen the panel is drawn on
    /// shows a difference. As memory, an output crosses the browser held as
    /// the core's list and as its JSON (a double's 8 bytes written as at most
    /// 24 characters, three a byte), so one at the budget takes 4 × 128 MiB
    /// at most — an eighth of the 4 GiB a wasm32 module addresses.
    pub const DEFAULT: Self = Self { bytes: 1 << 27 };

    /// **`n` items of `each` bytes admitted, or refused** naming `field`
    /// at the `value` that asked for them.
    ///
    /// # Errors
    ///
    /// [`Refused::past_budget`], its size counted in `u64` — what a
    /// 32-bit target's `usize` could not say of a drawing of billions of
    /// teeth — and saturating past it.
    pub fn admit(self, n: usize, each: usize, field: &str, value: f64) -> Result<(), Refused> {
        // `usize` into `u64` on every target this builds for.
        let (size, budget) = ((n as u64).saturating_mul(each as u64), self.bytes as u64);
        if size <= budget {
            Ok(())
        } else {
            Err(Refused::past_budget(field, value, size, budget))
        }
    }

    /// **Room in `v` for `n` more, admitted by the budget first**: the
    /// list's whole length is what is charged, and only then is memory
    /// asked for — exactly `n` into an empty list, as `Vec` grows after —
    /// refused naming `field` if the allocator will not give it even so.
    ///
    /// # Errors
    ///
    /// [`Refused::past_budget`], or [`Refused::past_memory`] beneath it.
    pub fn room<T>(self, v: &mut Vec<T>, n: usize, field: &str, value: f64) -> Result<(), Refused> {
        self.admit(
            v.len().saturating_add(n),
            std::mem::size_of::<T>(),
            field,
            value,
        )?;
        v.try_reserve(n)
            .map_err(|_| Refused::past_memory(field, value))
    }
}

/// A figure as it was given, exactly: its shortest decimal that reads back
/// to the same double, in scientific form where the plain one would run
/// past fifteen digits.
#[must_use]
pub fn figure(v: f64) -> String {
    let plain = v == 0.0 || (1e-4..1e15).contains(&v.abs());
    if plain {
        format!("{v}")
    } else {
        format!("{v:e}")
    }
}

/// A bound in interval notation — `(0, 90)`, `[1, ∞)` — and `∅` where it
/// admits nothing (an index into an empty list).
#[must_use]
pub fn interval(b: &Bound) -> String {
    if let (Some(lo), Some(hi)) = (b.min, b.max) {
        if lo > hi {
            return "∅".to_owned();
        }
    }
    let lo = b.min.map_or_else(
        || "(−∞".to_owned(), // absence: an unbounded side is its infinity
        |x| format!("{}{}", if b.exclusive_min { "(" } else { "[" }, figure(x)),
    );
    let hi = b.max.map_or_else(
        || "∞)".to_owned(), // absence: an unbounded side is its infinity
        |x| format!("{}{}", figure(x), if b.exclusive_max { ")" } else { "]" }),
    );
    format!("{lo}, {hi}")
}

/// `prefix.field`, or whichever is not empty.
fn join(prefix: &str, field: &str) -> String {
    match (prefix.is_empty(), field.is_empty()) {
        (true, _) => field.to_owned(),
        (_, true) => prefix.to_owned(),
        _ => format!("{prefix}.{field}"),
    }
}

// ----------------------------------------------------------- the check ---

/// **What a walk does at each row it meets**: the row's path (made only
/// where asked), its value and its bound — go on, or refuse.
type Visit<'a> = &'a mut dyn FnMut(&dyn Fn() -> String, f64, Option<Bound>) -> Result<(), Refused>;

/// The check every entry makes: [`verdict`] at each row.
fn checking() -> impl FnMut(&dyn Fn() -> String, f64, Option<Bound>) -> Result<(), Refused> {
    |field: &dyn Fn() -> String, value, bound| verdict(field, value, bound)
}

/// **One field the table reads**, as a walk meets it: its path, its value
/// and its bound — `None` where the row does not bound it (an automatic
/// box's seed).
#[derive(Clone, Debug)]
pub struct Field {
    pub path: String,
    pub value: f64,
    pub bound: Option<Bound>,
}

/// **`record` read against `rows`**, at `at`: the first value that is not a
/// finite number or is outside its row's bound, refused.
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn check<T>(rows: &[Row<T>], lists: &Lists, record: &T, at: &str) -> Result<(), Refused> {
    check_of(None, rows, lists, record, at, &mut checking())
}

/// [`check`], of the rows of one kind only where `kind` names one, each
/// handed to `visit`.
fn check_of<T>(
    kind: Option<Kind>,
    rows: &[Row<T>],
    lists: &Lists,
    record: &T,
    at: &str,
    visit: Visit<'_>,
) -> Result<(), Refused> {
    for row in rows.iter().filter(|r| kind.is_none_or(|k| r.kind == k)) {
        let Some(value) = (row.get)(record) else {
            continue;
        };
        visit(&|| join(at, row.field), value, (row.bound)(lists, record))?;
    }
    Ok(())
}

/// Every record of `list` read against `rows` of `kind` (all, where
/// `None`), each at `at.i`.
fn each<T>(
    kind: Option<Kind>,
    rows: &[Row<T>],
    lists: &Lists,
    list: &[T],
    at: &str,
    visit: Visit<'_>,
) -> Result<(), Refused> {
    for (i, x) in list.iter().enumerate() {
        check_of(kind, rows, lists, x, &join(at, &i.to_string()), &mut *visit)?;
    }
    Ok(())
}

/// **One number read against a bound**: refused where it is not finite, or
/// outside the bound where there is one — the verdict every row gives.
fn verdict(field: &dyn Fn() -> String, value: f64, bound: Option<Bound>) -> Result<(), Refused> {
    let reason = if !value.is_finite() {
        Reason::NotFinite
    } else {
        match bound {
            Some(b) if !b.admits(value) => Reason::Outside(b),
            _ => return Ok(()),
        }
    };
    Err(Refused {
        field: field(),
        value,
        reason,
    })
}

/// **One figure outside any record, read against the bound the table holds
/// for what it is** — a harness's positional argument: a torque against
/// [`FIGURE`], a face against [`POSITIVE`], a shaft angle against
/// [`SHAFT_ANGLE`].
///
/// # Errors
///
/// [`Refused`], naming `field`.
pub fn scalar(field: &str, value: f64, bound: Bound) -> Result<(), Refused> {
    verdict(&|| field.to_owned(), value, Some(bound))
}

/// Every field a walk meets, collected rather than checked.
fn collected(walk: impl FnOnce(Visit<'_>) -> Result<(), Refused>) -> Vec<Field> {
    let mut out = Vec::new();
    let mut visit = |path: &dyn Fn() -> String, value: f64, bound: Option<Bound>| {
        out.push(Field {
            path: path(),
            value,
            bound,
        });
        Ok(())
    };
    // The collecting visit refuses nothing.
    let _ = walk(&mut visit);
    out
}

impl GearParams {
    /// **These parameters as the boundary reads them** ([`GEAR`]).
    ///
    /// # Errors
    ///
    /// [`Refused`], naming the field.
    pub fn check(&self) -> Result<(), Refused> {
        check(GEAR, &Lists::default(), self, "")
    }

    /// Every field [`Self::check`] reads, with its bound.
    #[must_use]
    pub fn fields(&self) -> Vec<Field> {
        collected(|v| check_of(None, GEAR, &Lists::default(), self, "", v))
    }
}

impl Cutter {
    /// **This cutter as the boundary reads it** ([`CUTTER`]).
    ///
    /// # Errors
    ///
    /// [`Refused`], naming the field.
    pub fn check(&self) -> Result<(), Refused> {
        check(CUTTER, &Lists::default(), self, "")
    }

    /// Every field [`Self::check`] reads, with its bound.
    #[must_use]
    pub fn fields(&self) -> Vec<Field> {
        collected(|v| check_of(None, CUTTER, &Lists::default(), self, "", v))
    }
}

impl Material {
    /// **This material as the boundary reads it** ([`MATERIAL`]).
    ///
    /// # Errors
    ///
    /// [`Refused`], naming the field.
    pub fn check(&self) -> Result<(), Refused> {
        check(MATERIAL, &Lists::default(), self, "")
    }
}

impl MaterialLibrary {
    /// **Every material of the library as the boundary reads it**, each at
    /// `material.i` — the library's key on the wire.
    ///
    /// # Errors
    ///
    /// [`Refused`], naming the field.
    pub fn check(&self) -> Result<(), Refused> {
        library_of(self, &mut checking())
    }

    /// Every field [`Self::check`] reads, with its bound.
    #[must_use]
    pub fn fields(&self) -> Vec<Field> {
        collected(|v| library_of(self, v))
    }
}

/// [`MaterialLibrary::check`]'s walk.
fn library_of(lib: &MaterialLibrary, visit: Visit<'_>) -> Result<(), Refused> {
    each(
        None,
        MATERIAL,
        &Lists::default(),
        &lib.materials,
        "material",
        visit,
    )
}

/// **A graph's every number**, each at its path from the shape: its axes,
/// bodies, members (their rings' cutters and replaced material figures
/// among them), meshes, distances and couplings — every index read against
/// the list it names into before any figure is read.
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn shape(s: &Shape) -> Result<(), Refused> {
    shape_of(Some(Kind::Index), s, &mut checking())?;
    shape_of(Some(Kind::Figure), s, &mut checking())
}

/// [`shape`], of the rows of one kind.
fn shape_of(kind: Option<Kind>, s: &Shape, visit: Visit<'_>) -> Result<(), Refused> {
    let l = Lists::of(s);
    each(kind, AXIS, &l, &s.axes, "axes", &mut *visit)?;
    each(kind, BODY, &l, &s.bodies, "bodies", &mut *visit)?;
    for (i, m) in s.members.iter().enumerate() {
        let at = format!("members.{i}");
        check_of(kind, MEMBER, &l, m, &at, &mut *visit)?;
        if let Some(c) = &m.ring {
            check_of(kind, CUTTER, &l, c, &join(&at, "ring"), &mut *visit)?;
        }
        check_of(
            kind,
            OVERRIDES,
            &l,
            &m.gear.material_overrides,
            &join(&at, "gear.material_overrides"),
            &mut *visit,
        )?;
    }
    each(kind, MESH, &l, &s.meshes, "meshes", &mut *visit)?;
    each(kind, DISTANCE, &l, &s.distances, "distances", &mut *visit)?;
    each(kind, COUPLING, &l, &s.couplings, "couplings", visit)
}

/// **A train's every number**, each at its path from the train: its
/// graph's ([`shape`], under `shape`), its holds, and its load cases —
/// their factors, loads and duties — every index before any figure
/// ([`graph`], then [`figures`]).
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn train(t: &Train) -> Result<(), Refused> {
    graph(t)?;
    figures(t)
}

/// **A train's every index** — what its graph, its holds and its cases are
/// read by — so its graph can be read, whatever its figures hold.
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn graph(t: &Train) -> Result<(), Refused> {
    train_of(Some(Kind::Index), t, &mut checking())
}

/// **A train's every figure**: what its solve reads once its graph can be.
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn figures(t: &Train) -> Result<(), Refused> {
    train_of(Some(Kind::Figure), t, &mut checking())
}

/// **Every field [`train`] reads**, with its path from the train and its
/// bound — what a law sets to each bound's ends.
#[must_use]
pub fn fields(t: &Train) -> Vec<Field> {
    collected(|v| train_of(None, t, v))
}

/// [`train`], of the rows of one kind.
fn train_of(kind: Option<Kind>, t: &Train, visit: Visit<'_>) -> Result<(), Refused> {
    let mut within = |path: &dyn Fn() -> String, value: f64, bound: Option<Bound>| {
        visit(&|| join("shape", &path()), value, bound)
    };
    shape_of(kind, &t.shape, &mut within)?;
    let l = Lists::of(&t.shape);
    each(kind, HOLD, &l, &t.held, "held", &mut *visit)?;
    for (c, case) in t.load_cases.iter().enumerate() {
        let at = format!("load_cases.{c}");
        check_of(kind, CASE, &l, case, &at, &mut *visit)?;
        each(
            kind,
            LOAD,
            &l,
            &case.loads,
            &join(&at, "loads"),
            &mut *visit,
        )?;
        check_of(kind, DUTY, &l, &case.duty, &join(&at, "duty"), &mut *visit)?;
    }
    Ok(())
}

/// **A material as a member uses it** — its library's, with the member's
/// replacements laid over it ([`Material::overridden`]). Each replacement
/// is held to its own row ([`OVERRIDES`], read with the graph) and the
/// library to its own ([`MaterialLibrary::check`]); what only the two
/// together can break is the cyclic allowable against the static one, and
/// the replacement that moved it is named, at `at` (the member's
/// `gear.material_overrides`).
///
/// # Errors
///
/// [`Refused`], naming the field.
pub fn resolved(m: &Material, o: &Overrides, at: &str) -> Result<(), Refused> {
    let used = m.overridden(o);
    let (ultimate, fatigue) = (used.ultimate_allowable.value, used.fatigue_allowable.value);
    let within = cyclic(ultimate);
    if within.admits(fatigue) || !(ultimate.is_finite() && fatigue.is_finite()) {
        // A figure no row admits is the library's or the replacement's own
        // row to name.
        return used.check();
    }
    Err(
        if o.fatigue_allowable.is_some() || o.ultimate_allowable.is_none() {
            Refused {
                field: join(at, "fatigue_allowable"),
                value: fatigue,
                reason: Reason::Outside(within),
            }
        } else {
            Refused {
                field: join(at, "ultimate_allowable"),
                value: ultimate,
                reason: Reason::Outside(Bound::between(Some(fatigue), Some(CEILING))),
            }
        },
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Every bound the table holds a value to, by name.
    fn bounds() -> Vec<(&'static str, Bound)> {
        vec![
            ("module", MODULE),
            ("pressure angle", PRESSURE_ANGLE),
            ("helix angle", HELIX_ANGLE),
            ("thickness mod", THICKNESS_MOD),
            ("shaft angle", SHAFT_ANGLE),
            ("count", COUNT),
            ("figure", FIGURE),
            ("positive", POSITIVE),
            ("not negative", NOT_NEGATIVE),
            ("poisson", POISSONS_RATIO),
            ("index of 4", index(4)),
            ("body of 3", body(3)),
            ("body or ground of 3", body_or_ground(3)),
            ("cyclic under 470", cyclic(470.0)),
            ("pressure angle held", PRESSURE_ANGLE_HELD),
            ("application factor held", APPLICATION_FACTOR_HELD),
        ]
    }

    /// **Every bound holds its ends as it says**: an exclusive end refused
    /// and the next double inside admitted; an inclusive end admitted and
    /// the next double outside refused; nothing not finite admitted. The
    /// near misses are the ends themselves — a pressure angle of exactly
    /// 90°, a count of exactly nought, a Poisson's ratio of exactly ½ —
    /// where a check written `<` for `≤` passes every value but one.
    #[test]
    fn every_bound_holds_its_ends() {
        let mut ends = 0;
        for (name, b) in bounds() {
            if let Some(lo) = b.min {
                if b.exclusive_min {
                    assert!(!b.admits(lo), "{name}: its exclusive floor {lo}");
                    assert!(b.admits(lo.next_up()), "{name}: just above {lo}");
                } else {
                    assert!(b.admits(lo), "{name}: its floor {lo}");
                    assert!(!b.admits(lo.next_down()), "{name}: just below {lo}");
                }
                ends += 1;
            }
            if let Some(hi) = b.max {
                if b.exclusive_max {
                    assert!(!b.admits(hi), "{name}: its exclusive ceiling {hi}");
                    assert!(b.admits(hi.next_down()), "{name}: just below {hi}");
                } else {
                    assert!(b.admits(hi), "{name}: its ceiling {hi}");
                    assert!(!b.admits(hi.next_up()), "{name}: just above {hi}");
                }
                ends += 1;
            }
            for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
                assert!(!b.admits(x), "{name}: {x}");
            }
        }
        // Every bound two-sided but the two a model holds a value to from
        // one side.
        assert_eq!(ends, 2 * bounds().len() - 2);
        // An index into an empty list names nothing, and says so.
        assert!(!index(0).admits(0.0));
        assert_eq!(interval(&index(0)), "∅");
    }

    /// **The bounds the panel shows are the table's**: the gear's invariant
    /// ranges ([`crate::auto::admissible_ranges`]) read here, so a box and
    /// the core cannot hold one field to two bounds.
    #[test]
    fn the_ranges_the_panel_shows_are_the_table() {
        let r = crate::auto::admissible_ranges(&GearParams::default(), 1.25);
        assert_eq!(r.module, MODULE);
        assert_eq!(r.pressure_angle, PRESSURE_ANGLE);
        assert_eq!(r.teeth, COUNT);
        assert_eq!(r.helix_angle, HELIX_ANGLE);
        assert_eq!(r.thickness_mod, THICKNESS_MOD);
        let bound_of = |field: &str| {
            GEAR.iter()
                .find(|r| r.field == field)
                .and_then(|r| (r.bound)(&Lists::default(), &GearParams::default()))
        };
        assert_eq!(bound_of("module"), Some(r.module));
        assert_eq!(bound_of("pressure_angle"), Some(r.pressure_angle));
        assert_eq!(bound_of("teeth"), Some(r.teeth));
        assert_eq!(bound_of("helix_angle"), Some(r.helix_angle));
        assert_eq!(bound_of("thickness_mod"), Some(r.thickness_mod));
    }

    /// **What the table says the model holds, the model holds and says**:
    /// a pressure angle below half a degree is raised there, noted, and
    /// the tooth is the one at half a degree; a load case's factor below
    /// one is held at one, noted. At the bound, nothing is said. Every
    /// held row of the table is one of these three.
    #[test]
    fn a_held_row_is_held_by_the_model_and_said() {
        use crate::tooth::Tooth;
        use crate::train::{solve_train, test_library, Train};
        let held: Vec<(&str, Held)> = GEAR
            .iter()
            .filter_map(|r| r.held.map(|h| (r.field, h)))
            .chain(MEMBER.iter().filter_map(|r| r.held.map(|h| (r.field, h))))
            .chain(CASE.iter().filter_map(|r| r.held.map(|h| (r.field, h))))
            .collect();
        assert_eq!(
            held.iter().map(|(f, _)| *f).collect::<Vec<_>>(),
            [
                "pressure_angle",
                "pressure_angle.manual",
                "application_factor"
            ]
        );
        let floor = PRESSURE_ANGLE_HELD.min.unwrap();
        let at = |a: f64| {
            Tooth::new(GearParams {
                pressure_angle: a,
                ..GearParams::default()
            })
        };
        let below = at(0.3);
        assert!(below.clamps.fired(key::CLAMP_PRESSURE_ANGLE_RAISED));
        assert!((below.alpha_n - floor.to_radians()).abs() < 1e-15);
        assert!(!at(floor).clamps.fired(key::CLAMP_PRESSURE_ANGLE_RAISED));
        assert!(!at(floor.next_up())
            .clamps
            .fired(key::CLAMP_PRESSURE_ANGLE_RAISED));

        let case_at = |k: f64| {
            let mut t = Train::alone(&crate::train::arrangements::pair([17, 43]), 2.0, 100.0);
            for c in &mut t.load_cases {
                c.application_factor = k;
            }
            let r = solve_train(&t, &test_library()).unwrap();
            r.cases[0]
                .notes
                .iter()
                .any(|n| n.is(key::TRAIN_APPLICATION_FACTOR_HELD))
        };
        assert!(case_at(0.5));
        assert!(!case_at(APPLICATION_FACTOR_HELD.min.unwrap()));
    }

    /// **A refusal is said by the field's path, the value as given and the
    /// bound in interval notation** — a huge value in scientific form, not
    /// three hundred digits.
    #[test]
    fn a_refusal_names_its_field_its_value_and_its_bound() {
        let p = GearParams {
            module: -1e300,
            ..GearParams::default()
        };
        let e = p.check().unwrap_err().within("params");
        let n = e.note();
        assert_eq!(n.key, key::ERROR_INPUT_OUT_OF_RANGE);
        assert_eq!(n.values["field"], "params.module");
        assert_eq!(n.values["value"], "-1e300");
        assert_eq!(n.values["bound"], "[0.001, 1.7014118346046923e38]");
        let e = GearParams {
            teeth: 0,
            ..GearParams::default()
        }
        .check()
        .unwrap_err();
        assert_eq!(e.note().values["bound"], "[1, 4294967295]");
        let e = GearParams {
            dedendum: f64::NAN,
            ..GearParams::default()
        }
        .check()
        .unwrap_err();
        assert_eq!(e.note().key, key::ERROR_INPUT_NOT_FINITE);
        assert_eq!(e.note().values["field"], "dedendum");
        assert_eq!(interval(&HELIX_ANGLE), "(-90, 90)");
        assert_eq!(interval(&POISSONS_RATIO), "(-1, 0.5]");
        // A list of one admits its one index, and says so; of none, nothing.
        assert_eq!(interval(&index(1)), "[0, 0]");
        assert!(index(1).admits(0.0) && !index(1).admits(1.0));
    }

    /// **A member's material, its replacements laid over the library's, is
    /// held to a cyclic allowable no more than the static one** — and the
    /// replacement that moved it is named: a fatigue figure raised past the
    /// library's ultimate names the fatigue figure; an ultimate lowered under
    /// the library's fatigue figure names the ultimate; both raised together
    /// stand, as does the library as it is.
    #[test]
    fn a_replaced_allowable_is_held_to_its_static_one_and_named() {
        use crate::train::{solve_train, test_library, Train};
        let lib = test_library();
        let at = "shape.members.0.gear.material_overrides";
        let solve = |o: Overrides| {
            let mut s = crate::train::arrangements::pair([17, 43]);
            s.members[0].gear.material_overrides = o;
            solve_train(&Train::alone(&s, 2.0, 100.0), &lib)
        };
        let material = lib
            .get(&crate::train::MemberGear::default().material)
            .unwrap();
        let (ultimate, fatigue) = (
            material.ultimate_allowable.value,
            material.fatigue_allowable.value,
        );
        let refused = |o: Overrides| match solve(o) {
            Err(crate::train::TrainError::Input(r)) => r,
            other => panic!("{o:?}: refused, not {:?}", other.map(|_| ())),
        };
        let r = refused(Overrides {
            fatigue_allowable: Some(ultimate * 1.01),
            ..Overrides::default()
        });
        assert_eq!(r.field, format!("{at}.fatigue_allowable"));
        assert_eq!(r.bound(), Some(cyclic(ultimate)));
        let r = refused(Overrides {
            ultimate_allowable: Some(fatigue * 0.99),
            ..Overrides::default()
        });
        assert_eq!(r.field, format!("{at}.ultimate_allowable"));
        assert_eq!(r.value, fatigue * 0.99);
        assert!(solve(Overrides {
            ultimate_allowable: Some(ultimate * 2.0),
            fatigue_allowable: Some(ultimate * 1.5),
            ..Overrides::default()
        })
        .is_ok());
        assert!(solve(Overrides {
            fatigue_allowable: Some(ultimate),
            ..Overrides::default()
        })
        .is_ok());
        assert!(solve(Overrides::default()).is_ok());
    }

    /// **Two refusals of one value at one field are one refusal** — a
    /// not-a-number included, which no `==` on doubles calls equal — and a
    /// refusal at another field, of another value or against another bound
    /// is another. What lets a train's error, which carries one, be compared.
    #[test]
    fn a_refusal_is_its_field_its_value_and_its_bound() {
        let at = |p: GearParams| p.check().unwrap_err();
        let nan = GearParams {
            module: f64::NAN,
            ..GearParams::default()
        };
        assert_eq!(at(nan), at(nan));
        let zero = GearParams {
            module: 0.0,
            ..GearParams::default()
        };
        assert_eq!(at(zero), at(zero));
        assert_ne!(at(zero), at(nan));
        assert_ne!(
            at(zero),
            at(GearParams {
                module: -1.0,
                ..GearParams::default()
            })
        );
        assert_ne!(at(zero), at(zero).within("params"));
        let other_bound = Refused {
            reason: Reason::Outside(POSITIVE),
            ..at(GearParams {
                teeth: 0,
                ..GearParams::default()
            })
        };
        assert_ne!(
            other_bound,
            at(GearParams {
                teeth: 0,
                ..GearParams::default()
            })
        );
    }
    /// The values a bound's ends admit: each end itself where inclusive,
    /// the next double inside where exclusive.
    fn ends(b: Bound) -> Vec<f64> {
        let mut out = Vec::new();
        if let Some(lo) = b.min {
            out.push(if b.exclusive_min { lo.next_up() } else { lo });
        }
        if let Some(hi) = b.max {
            out.push(if b.exclusive_max { hi.next_down() } else { hi });
        }
        out
    }

    /// `v` with the leaf at the `.`-joined `path` set to `x`.
    fn set(v: &mut serde_json::Value, path: &str, x: serde_json::Value) {
        let mut at = v;
        for step in path.split('.') {
            at = match step.parse::<usize>() {
                Ok(i) if at.is_array() => &mut at[i],
                _ => &mut at[step],
            };
        }
        *at = x;
    }

    /// The leaf of `v` at the `.`-joined `path`.
    fn leaf<'a>(v: &'a serde_json::Value, path: &str) -> Option<&'a serde_json::Value> {
        path.split('.')
            .try_fold(v, |at, step| match step.parse::<usize>() {
                Ok(i) if at.is_array() => at.get(i),
                _ => at.get(step),
            })
    }

    /// A row's name, its list indices dropped: `shape.members.#.module.manual`.
    fn row_name(path: &str) -> String {
        path.split('.')
            .map(|s| if s.parse::<usize>().is_ok() { "#" } else { s })
            .collect::<Vec<_>>()
            .join(".")
    }

    /// **Every number in a train's report is finite at each end of every
    /// row's bound** — each row set to each of its ends in turn, the rest
    /// as the arrangement lays them, over arrangements that between them
    /// hold every row the train's tables have (a ring's cutter, a worm's
    /// distance, a coupling, a hold, both duties, every replaced material
    /// figure, a rim): the train solves with every number finite, or is
    /// refused. A row whose box is automatic is asked given too — the
    /// helix, the thickness, the pitch diameter, the distance, the face
    /// with no rating to size it. A refusal at an end is the model's
    /// answer to a design no one can make — a 0.1 mm tip width on a 1 µm
    /// module, a 7 mm worm of a 1e38 mm module — and whether the model is
    /// right at the module's floor is the homogeneity law's to hold, every
    /// millimetre scaled with it (`train/homogeneity.rs`, from
    /// [`SMALLEST_MODULE`] up).
    #[test]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a bound's end set as the integer the wire carries"
    )]
    fn every_report_is_finite_at_every_rows_ends() {
        use crate::train::arrangements::Preset;
        use crate::train::{solve_train, test_library, Duty, LoadCase, Train};
        let lib = test_library();
        let mut rows: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let (mut solved, mut refused) = (0, 0);
        let mut faults = Vec::new();
        let prepared = |shape: crate::train::Shape| {
            let mut t = crate::train::sweep::cased(vec![shape]);
            let continuous = LoadCase {
                duty: Duty::Continuous {
                    runtime_hours: 100.0,
                },
                ..t.load_cases[1].clone()
            };
            t.load_cases.push(continuous);
            let material = lib.get(&t.shape.members[0].gear.material).unwrap().clone();
            let g = &mut t.shape.members[0].gear;
            g.rim_thickness = Some(5.0);
            g.material_overrides = Overrides {
                density: Some(material.density.value),
                elastic_modulus: Some(material.elastic_modulus.value),
                poissons_ratio: Some(material.poissons_ratio.value),
                ultimate_allowable: Some(material.ultimate_allowable.value),
                fatigue_allowable: Some(material.fatigue_allowable.value),
                contact_fatigue_allowable: Some(500.0),
                ..Overrides::default()
            };
            t
        };
        let judge = |name: &str, t: &Train, path: &str, x: f64, faults: &mut Vec<String>| -> bool {
            match solve_train(t, &lib) {
                Ok(r) => {
                    let bad = crate::finite::non_finite(&r);
                    if !bad.is_empty() {
                        faults.push(format!(
                            "{name}: {path} = {x:e}: {} not finite, first {}",
                            bad.len(),
                            bad[0]
                        ));
                    }
                    true
                }
                Err(_) => false,
            }
        };
        for preset in [
            Preset::Spur,
            Preset::Worm,
            Preset::Planetary,
            Preset::Planocentric,
        ] {
            let name = format!("{preset:?}");
            let base = prepared(preset.build());
            base.validate().unwrap_or_else(|e| panic!("{name}: {e:?}"));
            let json = serde_json::to_value(&base).unwrap();
            for f in fields(&base) {
                // A box the solve fills is asked given too.
                let (json, bound) = match (f.bound, f.path.strip_suffix(".manual")) {
                    (Some(b), _) => (json.clone(), b),
                    (None, Some(auto)) => {
                        let mut given = json.clone();
                        set(
                            &mut given,
                            &format!("{auto}.auto"),
                            serde_json::Value::Bool(false),
                        );
                        let t: Train = serde_json::from_value(given.clone()).unwrap();
                        let Some(b) = fields(&t)
                            .into_iter()
                            .find(|g| g.path == f.path)
                            .and_then(|g| g.bound)
                        else {
                            continue;
                        };
                        (given, b)
                    }
                    (None, None) => continue,
                };
                rows.insert(row_name(&f.path));
                for x in ends(bound) {
                    let mut v = json.clone();
                    // A count or an index crosses as an integer, a figure as
                    // a number: the leaf as the train wrote it says which.
                    let integer = leaf(&json, &f.path).is_some_and(serde_json::Value::is_u64);
                    let x_leaf = if integer {
                        serde_json::json!(x as u64)
                    } else {
                        serde_json::json!(x)
                    };
                    set(&mut v, &f.path, x_leaf);
                    let Ok(t) = serde_json::from_value::<Train>(v) else {
                        continue;
                    };
                    if judge(&name, &t, &f.path, x, &mut faults) {
                        solved += 1;
                    } else {
                        refused += 1;
                    }
                }
            }
        }
        assert!(
            faults.is_empty(),
            "{} faults:\n{}",
            faults.len(),
            faults.join("\n")
        );
        // Every row the train's tables have was set to its ends somewhere.
        fn names<T>(prefix: &str, rows: &[Row<T>]) -> Vec<String> {
            rows.iter()
                .map(|r| row_name(&join(prefix, r.field)))
                .collect()
        }
        let every: Vec<String> = [
            names("shape.axes.#", AXIS),
            names("shape.bodies.#", BODY),
            names("shape.members.#", MEMBER),
            names("shape.members.#.ring", CUTTER),
            names("shape.members.#.gear.material_overrides", OVERRIDES),
            names("shape.meshes.#", MESH),
            names("shape.distances.#", DISTANCE),
            names("shape.couplings.#", COUPLING),
            names("held.#", HOLD),
            names("load_cases.#", CASE),
            names("load_cases.#.loads.#", LOAD),
            names("load_cases.#.duty", DUTY),
        ]
        .concat();
        let missed: Vec<&String> = every.iter().filter(|r| !rows.contains(*r)).collect();
        assert!(missed.is_empty(), "rows never asked: {missed:?}");
        eprintln!("{} rows, {solved} solved, {refused} refused", rows.len());
        assert!(solved > 0 && refused > 0);
    }

    /// **Every number a train's report holds is finite at each end of every
    /// material figure's bound** — each figure of the library's every
    /// material (its modulus, ν, allowables, the cyclic one up to its static
    /// one, a flank figure and a hardness where it has them) set to each end
    /// in turn and a pair and a worm, whose gears are of two of them, solved.
    #[test]
    fn every_report_is_finite_at_every_material_figures_ends() {
        use crate::train::{arrangements::Preset, solve_train, test_library, Train};
        // Every figure present: the steel given a published flank figure.
        let mut base = test_library();
        base.materials[0].contact_fatigue_allowable =
            Some(crate::material::Value::datasheet(1000.0));
        let json = serde_json::to_value(&base).unwrap();
        let trains = [Preset::Spur, Preset::Worm].map(|p| Train::alone(&p.build(), 2.0, 100.0));
        let (mut asked, mut faults) = (0, Vec::new());
        let mut named = std::collections::BTreeSet::new();
        for f in base.fields() {
            let Some(b) = f.bound else { continue };
            named.insert(row_name(&f.path));
            for x in ends(b) {
                let mut v = json.clone();
                set(&mut v, &f.path, serde_json::json!(x));
                let lib: MaterialLibrary = serde_json::from_value(v).unwrap();
                if lib.check().is_err() {
                    continue;
                }
                for t in &trains {
                    if let Ok(r) = solve_train(t, &lib) {
                        let bad = crate::finite::non_finite(&r);
                        if !bad.is_empty() {
                            faults.push(format!("{} = {x:e}: {bad:?}", f.path));
                        }
                    }
                    asked += 1;
                }
            }
        }
        assert!(faults.is_empty(), "{}", faults.join("\n"));
        let every: Vec<String> = MATERIAL
            .iter()
            .map(|r| row_name(&join("material.#", r.field)))
            .collect();
        let missed: Vec<&String> = every.iter().filter(|r| !named.contains(*r)).collect();
        assert!(missed.is_empty(), "figures never asked: {missed:?}");
        assert!(asked > 0);
    }
}
