//! **The planetary preset**: one carrier, one sun, one ring, N planets — the
//! inputs a designer of a simple set states, and the shape they lay out.
//!
//! The set is not a stage type of its own. Its geometry, closure, power flow
//! and ratings are the one shape's ([`super::shape`]): a central axis with
//! the sun, the carrier and the ring on it, a planet axis carried by the
//! carrier and replicated `N` times, and two meshes on the one distance
//! between them. What this module keeps is the *vocabulary* — sun, planet,
//! ring, planets, the two meshes' friction by name — and the conversion into
//! that shape, [`super::shape::Shape::from`]. What used to be its own is now
//! said once for every shape: a planet is loaded from both sides because it
//! is in two meshes, and one tooth-thickness coefficient fixes all three
//! members because the sun's mate takes `2 − k` across an external mesh and
//! the ring shares the planet's across an internal one.
//!
//! The planets are assumed to share the load equally, and the shape says so
//! in its notes wherever an axis is replicated.

use crate::params::Auto;
use crate::planetary::{Arrangement, PlanetaryShaft};
use crate::ring::Cutter;
use crate::train::{Optimisation, StageGear};

/// A planetary stage as its inputs describe it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PlanetaryStage {
    /// Normal module, mm. Shared by all three members.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// **The axial contact ratio** `ε_β` the set is asked for, where it is
    /// asked for one — the same input a pair has ([`super::PairStage::overlap`]),
    /// asked of both meshes: a floor under every automatic face width, or,
    /// with all three widths given, the thing that decides the helix, so that
    /// the narrower of the two meshes reaches it.
    pub overlap: Auto<f64>,
    /// Coefficient of friction, sun-to-planet.
    pub sliding_friction_sun_planet: f64,
    /// Coefficient of **static** friction, for breaking away.
    ///
    /// Whether a stage turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`] — the static figure's
    /// only job is the sign, and it is never itself reported as an efficiency.
    pub static_friction_sun_planet: f64,
    /// ...and planet-to-ring.
    pub sliding_friction_planet_ring: f64,
    /// Coefficient of **static** friction, for breaking away.
    ///
    /// Whether a stage turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`] — the static figure's
    /// only job is the sign, and it is never itself reported as an efficiency.
    pub static_friction_planet_ring: f64,
    /// `k` for the **sun**. The planet takes `2 − k` (external pair) and the ring
    /// takes the planet's (internal pair), so all three follow from this one.
    pub thickness_mod: f64,
    /// How many planets. One is legal; it just has no neighbour to clear.
    pub planets: u32,
    /// What the set is asked to optimise, and what it may not do to get there.
    /// See [`Optimisation`]: the sun's and the ring's shifts are searched
    /// together and the planet's follows, and both meshes are held to the
    /// contact ratio since a set is only as continuous as its worse half.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: Optimisation,
    /// The transverse contact ratio the search may not take either mesh
    /// below ([`super::DEFAULT_MIN_CONTACT_RATIO`]).
    #[cfg_attr(feature = "serde", serde(default = "super::default_min_contact_ratio"))]
    pub min_contact_ratio: f64,
    /// How the load is divided while two tooth pairs are engaged.
    ///
    /// **Off by default, and it reaches bending only** — see
    /// [`super::PairStage::load_sharing`], which is the same input for the same
    /// reason. Both meshes take it: a set switching the model on and getting it
    /// on one mesh would be one stage answering two ways.
    #[cfg_attr(feature = "serde", serde(default))]
    pub load_sharing: crate::contact::LoadSharing,
    /// **The distance the sun runs from a planet, or automatic.**
    ///
    /// The same shape and the same decision every other stage's centre distance
    /// has. Automatic, the common distance is whatever the shifts leave — which
    /// is what this stage has always done. Given, each mesh has a shift sum it
    /// must reach to run at it, and both of those are closed form
    /// ([`crate::mesh::shift_sum_for`]), so a target makes the layout *easier*:
    /// it removes the iteration rather than adding to it.
    ///
    /// Two equations instead of one means **two** of the three shifts are
    /// absorbed rather than one, which is the same accounting a pair does — see
    /// [`crate::train::FreedomGroup`].
    pub centre_distance: Auto<f64>,
    /// The running clearance, mm — in **both** meshes, and always an input.
    ///
    /// One physical distance carries a sun–planet mesh that opens as the planet
    /// moves out and a planet–ring mesh that opens as it moves in, so a
    /// clearance in both is the two zero-backlash distances differing by `2c`
    /// ([`crate::mesh::MeshKind::run_at`]) — which is what the absorbing shift
    /// is solved to leave. That is why it cannot be derived: a given distance
    /// and given shifts leave one gap on each mesh, and there is no one number
    /// for this field to be. It carries the same `Auto` every distance's does so
    /// the front end can offer it the same way; [`super::Stage::freedoms`]
    /// says it may not be automatic, and relief pins it.
    pub clearance: Auto<f64>,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// Smallest acceptable gap between adjacent planets' tip circles, mm.
    pub min_planet_clearance: f64,
    /// The shaper the ring is cut with. A ring has no geometry without one.
    pub cutter: Cutter,
    /// The sun. Its profile shift is an input, automatic or manual.
    pub sun: StageGear,
    /// The planet. **Its profile shift is ignored**: the shift is what makes the
    /// two centre distances agree, so it is solved rather than chosen (docs/reference.md#planetary-sets).
    pub planet: StageGear,
    /// The ring. Its `dedendum` and `root_radius` are ignored — a ring's root
    /// circle is where its cutter reaches (docs/reference.md#internal-gears) — and its shift is a manual
    /// input, since the automatic rule is an undercut criterion for an external
    /// tooth.
    pub ring: StageGear,
}

impl Default for PlanetaryStage {
    fn default() -> Self {
        // The words a set shares with every shape default where the shape
        // does, once.
        let shape = super::shape::Shape::default();
        Self {
            module: 1.0,
            pressure_angle: crate::params::GearParams::default().pressure_angle,
            overlap: super::shape::default_overlap(),
            sliding_friction_sun_planet: 0.08,
            static_friction_sun_planet: 0.16,
            sliding_friction_planet_ring: 0.08,
            static_friction_planet_ring: 0.16,
            thickness_mod: 1.0,
            load_sharing: shape.load_sharing,
            planets: 3,
            centre_distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            optimisation: shape.optimisation,
            min_contact_ratio: super::DEFAULT_MIN_CONTACT_RATIO,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            min_planet_clearance: shape.min_planet_clearance,
            cutter: Cutter::default(),
            // `z_r = z_s + 2 z_p`, the ideal ring, on a sun small enough to
            // need shift — so a fresh set shows what the automatic shift does
            // rather than three zeros.
            sun: StageGear {
                teeth: 12,
                ..StageGear::default()
            },
            planet: StageGear {
                teeth: 30,
                ..StageGear::default()
            },
            ring: StageGear {
                teeth: 72,
                profile_shift: Auto::fixed(0.0),
                ..StageGear::default()
            },
        }
    }
}

const SHAFT_SUN: usize = 1;
const SHAFT_CARRIER: usize = 2;
const SHAFT_RING: usize = 3;

impl PlanetaryStage {
    /// **An arrangement as a boundary** — the set's own vocabulary for its
    /// three central shafts, turned into what its solver takes.
    ///
    /// For a set asked about alone: the harness, a test, the sweep. In a train
    /// the same thing is two [`super::ShaftConstraint`]s on the train, and the
    /// set never sees the words.
    #[must_use]
    pub fn boundary_for(arrangement: Arrangement) -> super::StageBoundary {
        let shaft = |m: PlanetaryShaft| match m {
            PlanetaryShaft::Sun => SHAFT_SUN,
            PlanetaryShaft::Carrier => SHAFT_CARRIER,
            PlanetaryShaft::Ring => SHAFT_RING,
        };
        let output = PlanetaryShaft::ALL
            .into_iter()
            .find(|&m| m != arrangement.input && m != arrangement.fixed)
            .unwrap_or(arrangement.input);
        super::StageBoundary::holding(
            5,
            &[shaft(arrangement.fixed)],
            shaft(arrangement.input),
            shaft(output),
        )
    }

    /// The three members in the order [`super::StageResult::members`] reports
    /// them: sun, planet, ring.
    #[must_use]
    pub fn members(&self) -> [&StageGear; 3] {
        [&self.sun, &self.planet, &self.ring]
    }
}
