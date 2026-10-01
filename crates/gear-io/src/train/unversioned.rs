//! **A geartrain file as the tool wrote it before the format was numbered**:
//! read once by [`super::convert`] and never written.
//!
//! Such a file holds its train either as stages (`[[train.stages]]`, each a
//! shape whose bodies are the train's numbers) or as the one graph
//! (`[train.shape]`), and may leave out what a file of its time could. Each
//! field it may leave out is read at what every file that left it out meant,
//! written here as a literal: the converter is frozen, and a default the
//! tool moves later does not move what an old file said. Every field it may
//! not leave out is required, and every field it does not have is refused by
//! name.

use gear_core::contact::LoadSharing;
use gear_core::material::Overrides;
use gear_core::params::Auto;
use gear_core::ring::Cutter;
use gear_core::train::shape::{self, BodyOn};
use gear_core::train::{CaseKind, Duty, FaceSources, LoadRole};
use serde::Deserialize;

/// The document: its name and its train.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Document {
    pub name: String,
    pub train: Train,
}

/// The train, its graph as stages or as one shape.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Train {
    pub load_cases: Vec<LoadCase>,
    /// Absent, the reduced reversed-bending allowable is not asked for.
    #[serde(default)]
    pub reversed_bending: bool,
    /// Absent, nothing is held.
    #[serde(default)]
    pub held: Vec<usize>,
    /// The train as stages, each a shape of the train's bodies.
    pub stages: Option<Vec<Shape>>,
    /// The train as one graph.
    pub shape: Option<Shape>,
}

/// A shape, a stage's or the train's.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Shape {
    axes: Vec<Axis>,
    bodies: Vec<BodyOn>,
    members: Vec<Member>,
    meshes: Vec<Mesh>,
    distances: Vec<Distance>,
    /// Absent, nothing is coupled.
    #[serde(default)]
    couplings: Vec<[usize; 2]>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Axis {
    /// Absent, ground.
    #[serde(default)]
    carried_by: usize,
    count: u32,
    #[serde(default = "planet_clearance")]
    min_planet_clearance: f64,
}

/// A replicated axis's least gap between planets, mm, where a file gave none.
const fn planet_clearance() -> f64 {
    0.3
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Member {
    body: usize,
    gear: MemberGear,
    module: Auto<f64>,
    /// Normal pressure angle, degrees.
    #[serde(default = "pressure_angle")]
    pressure_angle: Auto<f64>,
    thickness_mod: Auto<f64>,
    ring: Option<Cutter>,
    pitch_diameter: Auto<f64>,
}

/// A member's pressure angle where a file gave none: its mesh group's, at 20°.
const fn pressure_angle() -> Auto<f64> {
    Auto {
        auto: true,
        manual: 20.0,
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct MemberGear {
    teeth: u32,
    profile_shift: Auto<f64>,
    #[serde(default = "on")]
    no_undercut: bool,
    working_depth: Auto<f64>,
    addendum: f64,
    #[serde(default = "on")]
    no_sharp_tip: bool,
    min_tip_width: f64,
    #[serde(default = "on")]
    no_tip_past_mate_flank: bool,
    dedendum: f64,
    root_radius: f64,
    /// Helix angle, degrees.
    helix_angle: Auto<f64>,
    face_width: Auto<f64>,
    face_sources: FaceSources,
    rim_thickness: Option<f64>,
    material: String,
    /// Absent, the library's figures throughout.
    #[serde(default)]
    material_overrides: Overrides,
}

/// A bound a file did not mention: on, as every gear then was.
const fn on() -> bool {
    true
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Mesh {
    a: usize,
    b: usize,
    sliding_friction: f64,
    static_friction: f64,
    #[serde(default = "overlap")]
    overlap: Auto<f64>,
    #[serde(default = "min_contact_ratio")]
    min_contact_ratio: f64,
    #[serde(default = "no_sharing")]
    load_sharing: LoadSharing,
    /// Absent, the shifts are not searched.
    #[serde(default)]
    search: bool,
}

/// A mesh's axial contact ratio where a file gave none: automatic, at one.
const fn overlap() -> Auto<f64> {
    Auto {
        auto: true,
        manual: 1.0,
    }
}

/// The search's floor on a mesh's transverse contact ratio where a file gave
/// none.
const fn min_contact_ratio() -> f64 {
    1.2
}

/// A mesh's load sharing where a file gave none: none.
const fn no_sharing() -> LoadSharing {
    LoadSharing::None
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Distance {
    axes: [usize; 2],
    /// The angle between the axes, degrees.
    angle: f64,
    worm: bool,
    distance: Auto<f64>,
    clearance: Auto<f64>,
    /// Absent, nought: the tips must not cross and nothing more.
    #[serde(default)]
    tip_clearance: f64,
    tolerance_plus: f64,
    tolerance_minus: f64,
    axial_clearance: f64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LoadCase {
    kind: CaseKind,
    enabled: bool,
    loads: Vec<Load>,
    duty: Duty,
    #[serde(default = "unit_factor")]
    application_factor: f64,
}

/// `K_A` where a file gave none: the torque entered is the design torque.
const fn unit_factor() -> f64 {
    1.0
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Load {
    at: usize,
    #[serde(default = "a_load")]
    role: LoadRole,
    torque: Auto<f64>,
    speed: Auto<f64>,
}

/// A port's role where a file gave none: a load.
const fn a_load() -> LoadRole {
    LoadRole::Load
}

impl From<Shape> for shape::Shape {
    fn from(s: Shape) -> Self {
        Self {
            axes: s
                .axes
                .into_iter()
                .map(|a| shape::Axis {
                    carried_by: a.carried_by,
                    count: a.count,
                    min_planet_clearance: a.min_planet_clearance,
                })
                .collect(),
            bodies: s.bodies,
            members: s.members.into_iter().map(Into::into).collect(),
            meshes: s
                .meshes
                .into_iter()
                .map(|m| shape::MeshInput {
                    a: m.a,
                    b: m.b,
                    sliding_friction: m.sliding_friction,
                    static_friction: m.static_friction,
                    overlap: m.overlap,
                    min_contact_ratio: m.min_contact_ratio,
                    load_sharing: m.load_sharing,
                    search: m.search,
                })
                .collect(),
            distances: s
                .distances
                .into_iter()
                .map(|d| shape::Distance {
                    axes: d.axes,
                    angle: d.angle,
                    worm: d.worm,
                    distance: d.distance,
                    clearance: d.clearance,
                    tip_clearance: d.tip_clearance,
                    tolerance_plus: d.tolerance_plus,
                    tolerance_minus: d.tolerance_minus,
                    axial_clearance: d.axial_clearance,
                })
                .collect(),
            couplings: s.couplings,
        }
    }
}

impl From<Member> for shape::Member {
    fn from(m: Member) -> Self {
        let g = m.gear;
        Self {
            body: m.body,
            gear: gear_core::train::MemberGear {
                teeth: g.teeth,
                profile_shift: g.profile_shift,
                no_undercut: g.no_undercut,
                working_depth: g.working_depth,
                addendum: g.addendum,
                no_sharp_tip: g.no_sharp_tip,
                min_tip_width: g.min_tip_width,
                no_tip_past_mate_flank: g.no_tip_past_mate_flank,
                dedendum: g.dedendum,
                root_radius: g.root_radius,
                helix_angle: g.helix_angle,
                // An automatic width's box was nought until the panel
                // seeded it, and nothing read it while a load sized the
                // gear: read as the width every gear is born with now.
                face_width: if g.face_width.auto && g.face_width.manual == 0.0 {
                    Auto::automatic(gear_core::train::DEFAULT_FACE_WIDTH)
                } else {
                    g.face_width
                },
                face_sources: g.face_sources,
                rim_thickness: g.rim_thickness,
                material: g.material,
                material_overrides: g.material_overrides,
            },
            module: m.module,
            pressure_angle: m.pressure_angle,
            thickness_mod: m.thickness_mod,
            ring: m.ring,
            pitch_diameter: m.pitch_diameter,
        }
    }
}

impl From<LoadCase> for gear_core::train::LoadCase {
    fn from(c: LoadCase) -> Self {
        Self {
            kind: c.kind,
            enabled: c.enabled,
            loads: c
                .loads
                .into_iter()
                .map(|l| gear_core::train::Load {
                    at: l.at,
                    role: l.role,
                    torque: l.torque,
                    speed: l.speed,
                })
                .collect(),
            duty: c.duty,
            application_factor: c.application_factor,
        }
    }
}
