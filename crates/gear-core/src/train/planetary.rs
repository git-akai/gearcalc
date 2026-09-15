//! The planetary stage: one carrier, one sun, one ring, and N planets.
//!
//! Most of what this needs already existed, and the point of the module is that
//! it *reuses* it rather than restating it. The planet's shift comes from
//! [`crate::planetary::solve`], the kinematics and efficiency from
//! [`crate::planetary::power`], the internal mesh from [`crate::ring`], and both
//! meshes' contact and bending from [`crate::strength`] through the same signed
//! machinery an external pair uses.
//!
//! # What is genuinely its own
//!
//! **The planet is loaded from both sides.** The sun drives one flank and the
//! ring the other, so its bending is fully reversed where the sun's and ring's
//! are one-directional. Its speed is also the odd one out: what fatigues it is
//! its rotation *relative to the carrier*, not its rotation in the fixed frame.
//! docs/reference.md#trains warns that this is easy to get silently wrong, so both appear in the
//! result and the reversal is stated rather than folded into a number.
//!
//! **The tooth thickness invariants differ between the two meshes.** An external
//! pair needs `k₁ + k₂ = 2`; an internal pair needs `k₁ = k₂` (docs/reference.md#internal-gears). A
//! planetary set has one of each, sharing the planet — so one stored `k` fixes
//! all three, and the invariant is unwritable rather than merely tested.
//!
//! # What is assumed, and said out loud
//!
//! **The planets are assumed to share the load equally.** Real sets do not
//! without a floating member or deliberate compliance; a mesh-load factor is the
//! usual remedy and it is a rating factor of exactly the kind docs/reference.md#contact-stress declines. So
//! the assumption is stated in the result's notes rather than absorbed into a
//! coefficient, and a designer who needs the derating can apply it knowingly.

use super::{
    Backlash, ContactRatios, GearResult, LoadCase, Loading, MemberRating, MeshReport, StageTorques,
    TrainError, Widths, PROBE,
};
use crate::contact::{efficiency, ContactPath, Directional};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::plane::BasicRack;
use crate::planetary::{self, Arrangement, PlanetaryShaft, Teeth};
use crate::ring::{Cutter, Ring};
use crate::strength::{bending_stress, contact_stress, Load, RootStressModel, PARALLEL_AXES};
use crate::tooth::Tooth;
use crate::train::{Optimisation, StageGear};

/// A planetary stage as its inputs describe it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    /// Helix angle, degrees. Shared; the internal pair takes the same hand and
    /// the external pair the opposite, which is what the meshes require.
    pub helix_angle: f64,
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
    /// Which shaft drives and which is held.
    pub arrangement: Arrangement,
    /// What the set is asked to optimise, and what it may not do to get there.
    /// See [`Optimisation`]: the sun's and the ring's shifts are searched
    /// together and the planet's follows, and both meshes are held to the
    /// contact ratio since a set is only as continuous as its worse half.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: Optimisation,
    /// How the load is divided while two tooth pairs are engaged.
    ///
    /// **Off by default, and it reaches bending only** — see
    /// [`super::PairStage::load_sharing`], which is the same input for the same
    /// reason. Both meshes take it: a set switching the model on and getting it
    /// on one mesh would be one stage answering two ways.
    #[cfg_attr(feature = "serde", serde(default))]
    pub load_sharing: crate::contact::LoadSharing,
    /// Added to the common centre distance, mm — the running clearance.
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
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            helix_angle: 0.0,
            sliding_friction_sun_planet: 0.08,
            static_friction_sun_planet: 0.16,
            sliding_friction_planet_ring: 0.08,
            static_friction_planet_ring: 0.16,
            thickness_mod: 1.0,
            load_sharing: crate::contact::LoadSharing::None,
            planets: 3,
            arrangement: Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
            },
            centre_distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            optimisation: Optimisation::default(),
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            min_planet_clearance: 0.3,
            cutter: Cutter::default(),
            sun: StageGear {
                teeth: 24,
                ..StageGear::default()
            },
            planet: StageGear {
                teeth: 18,
                ..StageGear::default()
            },
            ring: StageGear {
                teeth: 60,
                profile_shift: Auto::fixed(0.0),
                ..StageGear::default()
            },
        }
    }
}

/// The planet's own answers, which are not the shape of the other two.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PlanetResult {
    /// Everything a gear in a stage reports — including **the shift the
    /// common-centre-distance solve required**, which is this member's
    /// `profile_shift` like any other's, and its speed in the fixed frame.
    /// What is left below is what no `GearResult` has a field for.
    pub gear: GearResult,
    /// `|a_sun-planet − a_planet-ring|` at that shift, mm. Reported rather than
    /// asserted: it is the one number that says the solve closed.
    pub shift_residual: f64,
    /// Speed **relative to the carrier**, rpm — what its teeth actually see.
    ///
    /// The planet is the one member whose fixed-frame speed is not the whole
    /// story, and it is not the sun's relative speed either: the two differ by
    /// `z_s/z_p` ([`crate::planetary::Power::planet_speed`]).
    pub speed_relative: f64,
}

/// Everything a planetary stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PlanetaryResult {
    pub arrangement: Arrangement,
    /// The shaft the other two leave over.
    pub output: PlanetaryShaft,
    /// Speed reduction, input over output. Negative when the output reverses.
    pub ratio: f64,
    /// The common centre distance, zero-backlash, mm — sun-to-planet and
    /// planet-to-ring, which the planet's shift has made the same number.
    /// **What portion of the running distance is clearance**, mm — derived, as
    /// every other kind's is.
    ///
    /// `centre_distance − centre_distance_nominal`, so it cannot disagree with
    /// the two numbers it sits between. A set had no distance *input* until F39's
    /// third item, so this was the one kind with no gap to report; with one, the
    /// same three modes apply here as anywhere else.
    pub clearance: f64,
    pub centre_distance_nominal: f64,
    /// ...and the one actually used, including clearance.
    pub centre_distance: f64,
    /// Fixed-carrier efficiency `η₀`, the product of the two mesh efficiencies.
    /// The quantity docs/reference.md#planetary-sets requires, because the meshes slide at their speeds
    /// relative to the carrier rather than to ground.
    pub fixed_carrier_efficiency: Directional<f64>,
    /// Whole-set efficiency, driving forward and backward. Driving backward means
    /// the output shaft becomes the input with the same shaft held.
    pub efficiency: Directional<f64>,
    /// Angular backlash at whichever shaft is the **output**, degrees: the
    /// output shaft driving forward, the input shaft driving backward — the same
    /// convention every other stage kind uses.
    pub backlash: Directional<Backlash>,
    /// Speeds `[sun, carrier, ring]`, rpm. The held shaft is exactly zero.
    pub speeds: [f64; 3],
    /// Torques `[sun, carrier, ring]`, N·m. They sum to zero.
    pub torques: [f64; 3],
    pub sun_planet: MeshReport,
    pub planet_ring: MeshReport,
    /// Planets can be spaced evenly: `(z_s + z_r) mod N = 0`.
    pub equal_spacing: bool,
    /// Every planet meshes at the same phase — rarely true, and not a fault.
    pub simultaneous_meshing: bool,
    /// Gap between adjacent planets' tip circles, mm. `None` for a single planet.
    pub planet_clearance: Option<f64>,
    /// Whether that gap meets [`PlanetaryStage::min_planet_clearance`].
    pub planet_clearance_ok: bool,
    /// Whether the tooth counts share no factor with the planet count, which
    /// spreads wear instead of repeatedly pairing the same teeth.
    pub sun_coprime_with_planets: bool,
    pub ring_coprime_with_planets: bool,
    pub sun: GearResult,
    pub planet: PlanetResult,
    pub ring: GearResult,
    /// How many planets the set has — kept because the train needs it to count
    /// tooth cycles, and the stage's inputs are not in reach by then.
    pub planets: u32,
    /// Anything the stage had to say — including what it did not model.
    pub notes: Vec<Note>,
}

impl PlanetaryStage {
    /// `GearParams` for one member, with the thickness invariants applied.
    ///
    /// The sun's `k` is the input; the planet takes `2 − k` because they mesh
    /// externally, and the ring takes the planet's because they mesh internally.
    /// One number, three consistent values, no assertion needed.
    fn params(&self, member: PlanetaryShaft, teeth: u32, shift: f64, addendum: f64) -> GearParams {
        let (k, helix) = match member {
            PlanetaryShaft::Sun => (self.thickness_mod, self.helix_angle),
            // The planet opposes the sun's hand, as an external pair must.
            PlanetaryShaft::Carrier => (2.0 - self.thickness_mod, -self.helix_angle),
            // ...and the ring shares the planet's, as an internal pair must.
            PlanetaryShaft::Ring => (2.0 - self.thickness_mod, -self.helix_angle),
        };
        GearParams {
            // A stage member is concentric: the eccentric feature is the gear
            // tab's, and `..Default::default()` here would silently invent one
            // the day a stage grew the input.
            angular_shift: 0.0,
            index_offset: 0.0,
            module: self.module,
            pressure_angle: self.pressure_angle,
            teeth,
            helix_angle: helix,
            profile_shift: shift,
            addendum,
            dedendum: match member {
                PlanetaryShaft::Sun => self.sun.dedendum,
                PlanetaryShaft::Carrier => self.planet.dedendum,
                PlanetaryShaft::Ring => self.ring.dedendum,
            },
            root_radius: match member {
                PlanetaryShaft::Sun => self.sun.root_radius,
                PlanetaryShaft::Carrier => self.planet.root_radius,
                PlanetaryShaft::Ring => self.ring.root_radius,
            },
            thickness_mod: k,
        }
    }

    fn rack(&self) -> BasicRack {
        BasicRack::new(self.module, self.pressure_angle, self.helix_angle)
    }

    fn teeth(&self) -> Teeth {
        Teeth {
            sun: self.sun.teeth,
            planet: self.planet.teeth,
            ring: self.ring.teeth,
        }
    }
}

/// Solve a planetary stage.
///
/// # Errors
///
/// [`TrainError`] when the tooth counts admit no planet shift, when either mesh
/// cannot be formed, when a material is not in the library, or when a member is
/// too undercut to rate.
#[allow(clippy::too_many_lines)]
pub fn solve_planetary_stage(
    stage: &PlanetaryStage,
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<PlanetaryResult, TrainError> {
    solve_planetary_stage_with(stage, input_speed, torques, lib, super::Reversal::default())
}

/// The same, told how the train treats a root loaded on both flanks.
///
/// **A planet's root always is**, whatever the drive does: the sun drives one
/// flank and the ring the other. So a planetary set is the stage kind where this
/// is never idle — with the correction off, the planet is rated against the
/// material's plain fatigue figure and the stage says the reversal is there and
/// uncorrected.
///
/// # Errors
///
/// **The set as it would be built at a given pair of shifts.**
///
/// The sun's and the ring's shifts are the free ones; the planet's follows from
/// them, because the two centre distances are one number
/// (docs/reference.md#planetary-sets). Everything downstream — the
/// stresses, the kinematics, the efficiency — reads the geometry from here, and
/// so does the search that chooses those two shifts, so what is optimised is
/// what is built.
pub(super) struct Built {
    pub(super) sun: Tooth,
    pub(super) planet: Tooth,
    pub(super) ring: Ring,
    pub(super) ring_as_gear: Tooth,
    pub(super) layout: crate::planetary::Layout,
    pub(super) sp_mesh: Mesh,
    pub(super) sp_path: ContactPath,
    pub(super) pr_mesh: Mesh,
    pub(super) pr_path: ContactPath,
}

impl PlanetaryStage {
    /// [`Built`] at the two given shifts, or why the set has no geometry there.
    pub(super) fn built(&self, shifts: [f64; 3]) -> Result<Built, TrainError> {
        let stage = self;
        let teeth = stage.teeth();
        let rack = stage.rack();

        // Thickness shifts, since only `x + x_s` reaches the answer (docs/reference.md#tooth-thickness-and-its-equivalent-shift).
        let member_of = |i: usize| match i {
            0 => PlanetaryShaft::Sun,
            1 => PlanetaryShaft::Carrier,
            _ => PlanetaryShaft::Ring,
        };
        let modification = |i: usize| stage.params(member_of(i), 1, 0.0, 1.0).thickness_shift();
        let set = crate::planetary::Set {
            rack,
            teeth,
            planets: stage.planets,
            shift: std::array::from_fn(|i| shifts[i] + modification(i)),
            absorber: stage.absorber(),
            // **The nominal distance**, which is what the shifts have to reach;
            // the clearance is added to it afterwards, exactly as a pair's is.
            distance: (!stage.centre_distance.auto)
                .then_some(stage.centre_distance.manual - stage.clearance.manual),
            // Filled once the planet exists; clearance only reads it.
            planet_tip_diameter: 0.0,
        };
        let layout = crate::planetary::solve(&set).ok_or(TrainError::NoContact)?;
        // The solve works in thickness shifts, so take the modification back out
        // to get each member's profile shift proper. Only the absorber's has
        // actually moved, but taking it out of all three is the same expression
        // as putting it in and cannot drift from it.
        let solved: [f64; 3] = std::array::from_fn(|i| layout.shift[i] - modification(i));
        let [sun_shift, planet_shift, ring_shift] = solved;

        let addendum_of = |member: PlanetaryShaft, g: &StageGear, teeth: u32, shift: f64| -> f64 {
            g.addendum_asked(&stage.params(member, teeth, shift, g.addendum))
                .used
        };
        let sun_params = stage.params(
            PlanetaryShaft::Sun,
            teeth.sun,
            sun_shift,
            addendum_of(PlanetaryShaft::Sun, &stage.sun, teeth.sun, sun_shift),
        );
        let planet_params = stage.params(
            PlanetaryShaft::Carrier,
            teeth.planet,
            planet_shift,
            addendum_of(
                PlanetaryShaft::Carrier,
                &stage.planet,
                teeth.planet,
                planet_shift,
            ),
        );
        let ring_params = stage.params(
            PlanetaryShaft::Ring,
            teeth.ring,
            ring_shift,
            stage.ring.addendum,
        );

        let sun = Tooth::new(sun_params);
        let planet = Tooth::new(planet_params);
        let ring = Ring::cut_by(&ring_params, &stage.cutter);
        // The mesh reads the ring through `Tooth` arithmetic: a ring's shift enters
        // its space exactly as an external gear's enters its tooth (docs/reference.md#internal-gears).
        let ring_as_gear = Tooth::new(ring_params);

        let sp_mesh = Mesh::new(&sun, &planet, MeshKind::External).map_err(TrainError::Mesh)?;
        let sp_path = ContactPath::new(&sun, planet.ra, &sp_mesh).ok_or(TrainError::NoContact)?;
        let pr_mesh =
            Mesh::new(&planet, &ring_as_gear, MeshKind::Internal).map_err(TrainError::Mesh)?;
        let pr_path = ContactPath::new(&planet, ring.ra, &pr_mesh).ok_or(TrainError::NoContact)?;

        Ok(Built {
            sun,
            planet,
            ring,
            ring_as_gear,
            layout,
            sp_mesh,
            sp_path,
            pr_mesh,
            pr_path,
        })
    }

    /// **The sun's and the ring's shifts**, chosen together where the stage
    /// asked for that.
    ///
    /// Off, each is what it always was: the ring's as given, the sun's the
    /// least that clears undercut. On, the pair is searched for the fixed-carrier
    /// efficiency `η₀` — the product of the two mesh efficiencies — because
    /// [`crate::planetary::power`] rises with `η₀` in either direction, so the
    /// most efficient basic train is the most efficient set and the power flow
    /// need not be run inside the search.
    ///
    /// Unlike a pair, these two are not free of each other: the planet's shift
    /// absorbs whatever they ask for, and where it cannot the set has no
    /// geometry and the point is simply not admissible.
    /// **Which member's shift closes the set**, from the toggles rather than
    /// from a control of its own.
    ///
    /// The two centre distances have to agree, which is one relation among
    /// three shifts: two are a design and the third is what they leave. The
    /// member that absorbs it is one left automatic, and the planet is
    /// preferred because it is the member in *both* meshes — the one whose
    /// shift no single mesh's operating angle is a statement about, and the one
    /// this stage has always used. Pinning the planet is therefore how a
    /// designer asks for the sun to close it instead, exactly as pinning one of
    /// a hula stage's two members names the other as the one the crank
    /// supplies.
    ///
    /// With all three given the set is over-specified and something has to give
    /// way; the planet does, and the front end relieves the over-specification
    /// as it is created so this is the transient rather than the design.
    #[must_use]
    pub fn absorber(&self) -> crate::planetary::Member {
        use crate::planetary::Member;
        if self.planet.profile_shift.auto {
            Member::Planet
        } else if self.sun.profile_shift.auto {
            Member::Sun
        } else if self.ring.profile_shift.auto {
            Member::Ring
        } else {
            Member::Planet
        }
    }

    /// What each member asks of its shift, in the solve's order.
    fn asked(&self) -> [super::ShiftAsked; 3] {
        let sun_base = self.params(PlanetaryShaft::Sun, self.sun.teeth, 0.0, self.sun.addendum);
        let planet_base = self.params(
            PlanetaryShaft::Carrier,
            self.planet.teeth,
            0.0,
            self.planet.addendum,
        );
        [
            self.sun.shift_asked(&sun_base),
            self.planet.shift_asked(&planet_base),
            // **A ring is never asked about undercut** — its flank is its
            // shaper's rather than a rack's — so its shift is given or it is
            // the set's to choose, and nothing floors it either way.
            super::ShiftAsked {
                search_floor: None,
                given: (!self.ring.profile_shift.auto).then_some(self.ring.profile_shift.manual),
                settled: if self.ring.profile_shift.auto {
                    0.0
                } else {
                    self.ring.profile_shift.manual
                },
                raised: false,
            },
        ]
    }

    /// The **nominal** distance the shifts have to reach, where one was given —
    /// the distance typed less the clearance it is opened by, exactly as a
    /// parallel pair reads its own.
    fn nominal_distance(&self) -> Option<f64> {
        (!self.centre_distance.auto && !self.clearance.auto)
            .then_some(self.centre_distance.manual - self.clearance.manual)
    }

    /// **The three shifts that put both meshes at `target`.**
    ///
    /// Two closed-form sums and one freedom, taken as the planet's shift. A
    /// shift a designer *gave* fixes that freedom — through whichever mesh it is
    /// in — and where none was given the freedom follows the same rule a pair's
    /// division does: the even split of the external mesh's sum, projected onto
    /// what the two members can be cut at ([`crate::auto::divide_shift_sum`]),
    /// so a member against its undercut floor holds there while the other
    /// absorbs.
    ///
    /// `None` where no such geometry exists — the distance is below a base
    /// circle limit, or no admissible pair of external shifts reaches its sum.
    fn shifts_reaching(&self, target: f64, asked: &[super::ShiftAsked; 3]) -> Option<[f64; 3]> {
        let rack = self.rack();
        let teeth = self.teeth();
        let sum_x = |sum_z: f64| {
            crate::mesh::shift_sum_for(rack.mt, rack.alpha_t, rack.alpha_n, sum_z, target)
        };
        let sum_ext = f64::from(teeth.sun) + f64::from(teeth.planet);
        let sum_int = f64::from(teeth.ring) - f64::from(teeth.planet);
        let (s_ext, s_int) = (sum_x(sum_ext)?, sum_x(-sum_int)?);

        let given = [0, 1, 2].map(|i| asked[i].given);
        let x_p = match given {
            [_, Some(p), _] => p,
            [Some(sun), None, _] => s_ext - sun,
            [None, None, Some(ring)] => s_int + ring,
            [None, None, None] => {
                let at = |i: usize, x: f64| {
                    let (member, count) = if i == 0 {
                        (PlanetaryShaft::Sun, teeth.sun)
                    } else {
                        (PlanetaryShaft::Carrier, teeth.planet)
                    };
                    let g = if i == 0 { &self.sun } else { &self.planet };
                    self.params(member, count, x, g.addendum)
                };
                let floor = [asked[0].search_floor, asked[1].search_floor];
                crate::auto::divide_shift_sum(&at, 1.0, s_ext, floor)?[1]
            }
        };
        Some([s_ext - x_p, x_p, x_p - s_int])
    }

    // **The tests\' door.** The solve reads `chosen_at`, because it needs
    // to know *how* the shifts were arrived at as well as what they are.
    #[cfg(test)]
    pub(super) fn shifts(&self) -> [f64; 3] {
        self.shifts_at(&crate::auto::Search::SHIPPED)
    }

    /// As [`Self::shifts`], at a stated search effort — see `auto::Search`, and
    /// `PairStage::shifts_at` for why the effort is a parameter at all.
    /// As [`Self::shifts_at`], **and whether the optimiser actually chose** —
    /// see `super::Searched`, and `PairStage::chosen_at` for why the two
    /// outcomes that look alike have to be told apart.
    pub(super) fn chosen_at(&self, search: &crate::auto::Search) -> super::Chosen<3> {
        let asked = self.asked();
        let absorbed = self.absorber().index();
        let plain: [f64; 3] = std::array::from_fn(|i| asked[i].settled);

        // **A given centre distance is two equations, and it leaves one
        // freedom.** Each mesh has a shift sum it must reach to run at that
        // distance and both are closed form, so the sun's and the ring's shifts
        // are read off the planet's — which is the coordinate this stage has
        // always treated as the special one, being the member in both meshes.
        //
        // Honoured whether or not the stage is optimising, for the reason the
        // parallel pair's is: the relation belongs to the geometry and never to
        // the optimiser. **What is not yet done is searching that one freedom**
        // — with a target the objective is a function of the planet's shift
        // alone, which is a one-dimensional search this pass does not add.
        if let Some(target) = self.nominal_distance() {
            if let Some(x) = self.shifts_reaching(target, &asked) {
                return super::Chosen {
                    shifts: x,
                    how: super::Searched::NotAsked,
                };
            }
        }

        if !self.optimisation.enabled {
            return super::Chosen {
                shifts: plain,
                how: super::Searched::NotAsked,
            };
        }
        // **The absorber is not searched**: its shift is what the other two
        // leave, so it is neither given nor free and the search never places a
        // number in it. What it settles at comes back out of `built`, which is
        // where the equality is closed.
        let given: [Option<f64>; 3] =
            std::array::from_fn(|i| (i == absorbed).then_some(plain[i]).or(asked[i].given));
        let freedoms = crate::auto::Freedoms::new(given);
        let eta0 = |x: [f64; 3]| -> Option<f64> {
            let b = self.built(x).ok()?;
            // **The sun and the planet both have to be cuttable**, and the
            // planet especially: its shift is not chosen but absorbed, so a sun
            // and a ring that ask for more than a planet can carry is exactly
            // the combination this has to refuse. The ring is asked the one
            // question it can answer — whether its cutter left the shape the
            // shift asked for — and the set's own internal bounds are the
            // interference flags the mesh reports.
            // **Each member answers to the bound its own arrival earns it** —
            // chosen, given, or absorbed (`train::undercut_bound`). The absorber
            // is the interesting one: nothing can move it, so the only honest
            // question is whether it actually undercuts.
            let how = |i: usize| {
                if i == absorbed {
                    super::Decided::Absorbed
                } else if asked[i].given.is_some() {
                    super::Decided::Given
                } else {
                    super::Decided::Chosen
                }
            };
            let floors = [
                super::undercut_bound(
                    self.sun.no_undercut,
                    &b.sun.params,
                    self.sun.dedendum,
                    how(0),
                ),
                super::undercut_bound(
                    self.planet.no_undercut,
                    &b.planet.params,
                    self.planet.dedendum,
                    how(1),
                ),
            ];
            // **What a set contributes is which meshes it has and how they are
            // assembled**; what is asked of each of them belongs to the mesh
            // (`auto::MeshTrial`) and is the same question a pair asks. Asked
            // here for itself, this set answered two of the three: a ring was
            // asked nothing at all, and no member was asked whether its teeth
            // reach past the root circle they run into.
            let sun = crate::auto::Cut::ByRack {
                tooth: &b.sun,
                floor: floors[0],
            };
            let planet = crate::auto::Cut::ByRack {
                tooth: &b.planet,
                floor: floors[1],
            };
            let trial = |members, mesh, path, friction| {
                crate::auto::MeshTrial {
                    members,
                    mesh,
                    path,
                    min_contact_ratio: self.optimisation.min_contact_ratio,
                    friction,
                }
                .efficiency()
            };
            Some(
                trial(
                    [sun, planet],
                    &b.sp_mesh,
                    &b.sp_path,
                    self.sliding_friction_sun_planet,
                )? * trial(
                    [planet, crate::auto::Cut::ByShaper { ring: &b.ring }],
                    &b.pr_mesh,
                    &b.pr_path,
                    self.sliding_friction_planet_ring,
                )?,
            )
        };
        if freedoms.count() == 0 {
            // Every shift given by hand: nothing was asked, so nothing failing
            // to move says anything.
            return super::Chosen {
                shifts: plain,
                how: super::Searched::NotAsked,
            };
        }
        // **Where each member's shift can sit, asked of the tool that cuts it.**
        //
        // The sun and the planet are rack-cut and answer `auto::searchable_shift`
        // whole. A ring is asked **one** of its questions and not the other, and
        // that is the same split `auto::member_is_buildable` already makes: its
        // *space* is generated the way a tooth is and takes the identical
        // expression, so the thickness pair bounds it unchanged; its *fillet* is
        // its shaper's tip round rather than an input of its own, so "does the
        // round asked for still fit" is a question it has no answer to.
        //
        // Asking it anyway is what this used to do, and it capped a ring near
        // 1.2 modules where these sets want 1.9 — the shift bound of a rack that
        // is not cutting it.
        let member_kind = |i: usize| {
            [
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
                PlanetaryShaft::Ring,
            ][i]
        };
        let params = |i: usize, x: f64| {
            let (teeth, addendum) = [
                (self.sun.teeth, self.sun.addendum),
                (self.planet.teeth, self.planet.addendum),
                (self.ring.teeth, self.ring.addendum),
            ][i];
            self.params(member_kind(i), teeth, x, addendum)
        };
        let Some(per_member) = (0..3)
            // **Each member asked of the tool that cuts it.**
            .map(|i| match member_kind(i) {
                // Shaper-cut. Its *space* is generated the way a tooth is and
                // takes the identical expression, so `admissible_profile_shift`
                // already bounds it — the same two guards read on the space,
                // which is what `Ring::cut_by` says where it applies them. What
                // a ring has no answer to is the round: that is its cutter's,
                // and it is asked of the cut instead (`ring_is_cut_as_asked`).
                PlanetaryShaft::Ring => {
                    let p = params(i, 0.0);
                    let b = crate::auto::admissible_ranges(&p, p.dedendum)
                        .profile_shift
                        .bound;
                    let (lo, hi) = (b.min?, b.max?);
                    (lo < hi).then_some((lo, hi))
                }
                // Rack-cut, and asked all of it.
                _ => crate::auto::searchable_shift(&|x| params(i, x), asked[i].search_floor),
            })
            .collect::<Option<Vec<_>>>()
        else {
            // No member has an interval to search in, which is the same finding
            // as a search that combed one and found nothing.
            return super::Chosen {
                shifts: plain,
                how: super::Searched::FoundNothing,
            };
        };
        let box_ = freedoms.boxes([per_member[0], per_member[1], per_member[2]]);
        search
            .maximise(&box_, &|free| eta0(freedoms.place(free)))
            .map(|free| freedoms.place(&free))
            .map_or_else(
                || super::Chosen {
                    shifts: plain,
                    how: super::Searched::FoundNothing,
                },
                |shifts| super::Chosen {
                    shifts,
                    how: super::Searched::Chose,
                },
            )
    }

    /// As [`Self::shifts`], at a stated search effort — see `auto::Search`, and
    /// `PairStage::shifts_at` for why the effort is a parameter at all.
    #[cfg(test)]
    pub(super) fn shifts_at(&self, search: &crate::auto::Search) -> [f64; 3] {
        self.chosen_at(search).shifts
    }
}

/// As [`solve_planetary_stage`].
pub fn solve_planetary_stage_with(
    stage: &PlanetaryStage,
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
    reversal: super::Reversal,
) -> Result<PlanetaryResult, TrainError> {
    let input_torque = torques.peak_forward;
    let teeth = stage.teeth();
    // The sun and the planet are rack-cut and can be raised to clear undercut;
    // a ring is not asked. Collected here and handed to the member each one is
    // about, so a note naming an input reaches the reader under that input.
    let member_notes: Vec<(usize, Note)> = [
        (
            &stage.sun,
            stage.params(
                PlanetaryShaft::Sun,
                stage.sun.teeth,
                0.0,
                stage.sun.addendum,
            ),
        ),
        (
            &stage.planet,
            stage.params(
                PlanetaryShaft::Carrier,
                stage.planet.teeth,
                0.0,
                stage.planet.addendum,
            ),
        ),
    ]
    .into_iter()
    .enumerate()
    .filter_map(|(i, (g, p))| g.shift_asked(&p).note(g.teeth).map(|n| (i, n)))
    .collect();
    let mut notes: Vec<Note> = Vec::new();

    // ---- the set as built, at the shifts the stage settled on.
    let chosen = stage.chosen_at(&crate::auto::Search::SHIPPED);
    let shifts = chosen.shifts;
    // ...and the other end of the tooth, which needs those shifts to be known.
    let mut member_notes = member_notes;
    member_notes.extend(
        [
            (&stage.sun, PlanetaryShaft::Sun, stage.sun.teeth, shifts[0]),
            (
                &stage.planet,
                PlanetaryShaft::Carrier,
                stage.planet.teeth,
                shifts[1],
            ),
        ]
        .into_iter()
        .enumerate()
        .filter_map(|(i, (g, member, teeth, x))| {
            g.addendum_asked(&stage.params(member, teeth, x, g.addendum))
                .note(teeth)
                .map(|n| (i, n))
        }),
    );
    let notes_for = |which: usize| -> Vec<Note> {
        member_notes
            .iter()
            .filter(|(i, _)| *i == which)
            .map(|(_, n)| n.clone())
            .collect()
    };
    let Built {
        sun,
        planet,
        ring,
        ring_as_gear,
        layout,
        sp_mesh,
        sp_path,
        pr_mesh,
        pr_path,
    } = stage.built(shifts)?;
    let sun_params = sun.params;
    let planet_params = planet.params;
    let ring_params = ring_as_gear.params;

    // ---- materials.
    let material_of = |g: &StageGear| -> Result<Material, TrainError> {
        lib.get(&g.material)
            .ok_or_else(|| TrainError::UnknownMaterial(g.material.clone()))
            .map(|m| m.overridden(&g.material_overrides))
    };
    let mats = [
        material_of(&stage.sun)?,
        material_of(&stage.planet)?,
        material_of(&stage.ring)?,
    ];

    // ---- kinematics and efficiency.
    //
    // `eta0` twice: once on the sliding coefficients, once on the static ones.
    // A set is nowhere near its own threshold, so the second only ever confirms
    // the first — it is computed anyway so no stage kind is the exception
    // (`Directional::once_moving`).
    let sun_planet = |mu: f64| Directional::of(|d| efficiency(&sp_path, &sp_mesh, &sun, mu, d));
    let planet_ring = |mu: f64| Directional::of(|d| efficiency(&pr_path, &pr_mesh, &planet, mu, d));
    let combined =
        |sp: &Directional<f64>, pr: &Directional<f64>| Directional::of(|d| sp.get(d) * pr.get(d));

    let sp_eff = sun_planet(stage.sliding_friction_sun_planet);
    let pr_eff = planet_ring(stage.sliding_friction_planet_ring);
    let eta0 = combined(&sp_eff, &pr_eff);
    let eta0_at_rest = combined(
        &sun_planet(stage.static_friction_sun_planet),
        &planet_ring(stage.static_friction_planet_ring),
    );

    let forward = planetary::power(
        planetary::basic_ratio(teeth),
        stage.arrangement,
        input_speed,
        input_torque,
        eta0.forward,
    )
    .ok_or(TrainError::NoContact)?;
    // Driving backward: the output shaft becomes the input, the same shaft held.
    //
    // **Its torque has to be a driving one.** The forward solution leaves that
    // shaft carrying a *reaction* torque, opposite in sign to its speed; passing
    // that back in makes the rolling power come out the wrong way round, which
    // picks the wrong branch of `η₀^w` and returns an efficiency **above one**.
    // So the magnitude is carried over and the sign is taken from the speed,
    // which is what "this shaft is now driving" means.
    let out = forward.output.index_pub();
    let reversed = Arrangement {
        input: forward.output,
        fixed: stage.arrangement.fixed,
    };
    let backward = planetary::power(
        planetary::basic_ratio(teeth),
        reversed,
        forward.speeds[out],
        forward.torques[out].abs() * if forward.speeds[out] < 0.0 { -1.0 } else { 1.0 },
        eta0.backward,
    );
    // The same two solves on the static basic efficiency, for the sign only.
    // A set is a train of parallel meshes and never sits near its threshold, so
    // this confirms rather than decides — but it is the rule every stage kind
    // obeys, and a stage that skipped it would be the one place a reader has to
    // remember why.
    let at_rest = Directional {
        forward: planetary::power(
            planetary::basic_ratio(teeth),
            stage.arrangement,
            input_speed,
            input_torque,
            eta0_at_rest.forward,
        )
        .map_or(0.0, |p| p.efficiency),
        backward: planetary::power(
            planetary::basic_ratio(teeth),
            reversed,
            forward.speeds[out],
            forward.torques[out].abs() * if forward.speeds[out] < 0.0 { -1.0 } else { 1.0 },
            eta0_at_rest.backward,
        )
        .map_or(0.0, |b| b.efficiency),
    };
    let set_efficiency = Directional {
        forward: forward.efficiency,
        backward: backward.map_or(0.0, |b| b.efficiency),
    }
    .once_moving(&at_rest);

    // ---- loads. Each mesh carries its member's torque divided among N planets.
    let planets = f64::from(stage.planets.max(1));

    // **The reverse is the same construction with the roles swapped**, and the
    // torques it produces are the ones a back-driving load puts on each member.
    //
    // They used to be the *forward* distribution scaled by the ratio of the two
    // stage torques (`StageTorques::referred_like`), which is exact wherever the
    // forward torque is a geometric projection or the two directional
    // efficiencies agree. An epicyclic set is neither: which shaft is driving
    // decides where `η₀` multiplies, so the distribution itself changes shape.
    // Measured on the shipped set, driving backward puts **1.922 N·m** on the
    // sun where scaling the forward answer says 2.000 — 3.9 %, and 1.6 % on the
    // ring.
    //
    // `backward` was already solved, for its efficiency; this reads the torques
    // it had all along. Normalised so the shaft the load was referred to carries
    // what `back_driving_torques` referred there, since a power flow is linear
    // in the torque through it.
    let in_i = stage.arrangement.input.index_pub();
    let backward_share = |shaft: usize| -> Option<f64> {
        let (b, applied) = (backward.as_ref()?, torques.peak_backward?);
        let at_input = b.torques[in_i];
        if at_input == 0.0 {
            return Some(0.0);
        }
        Some((b.torques[shaft] / planets) * (applied / at_input))
    };

    // **What each mesh carries, in each load case.** Driving forward it is the
    // shaft's torque shared among the planets; being driven it is the reverse
    // solve's, which is a different *shape* and not merely a different size. So
    // the peak is taken per mesh, after each direction's own distribution —
    // `StageTorques::on_mesh` argues the order, and this set is one of the two
    // kinds where the two orders differ.
    let sun_torque_per_mesh = (forward.torques[0] / planets).abs();
    let ring_torque_per_mesh = (forward.torques[2] / planets).abs();
    let sp_torque = torques.on_mesh(
        sun_torque_per_mesh,
        backward_share(PlanetaryShaft::Sun.index_pub()),
    );
    // Quoted at the planet, which is the member the ring mesh's loads are read
    // through, so both directions take the same projection.
    let pr_torque = torques.on_mesh(
        ring_torque_per_mesh / pr_mesh.ratio(),
        backward_share(PlanetaryShaft::Ring.index_pub()).map(|t| t / pr_mesh.ratio()),
    );

    // ---- face widths and stresses. `b_min` does not depend on the width it was
    // measured at (docs/reference.md#contact-stress), so one probe evaluation gives every minimum.
    let sp_e = contact_modulus(&mats[0], &mats[1]);
    let pr_e = contact_modulus(&mats[1], &mats[2]);

    let sp_probe = contact_stress(
        &sp_path,
        &sp_mesh,
        &sun,
        PARALLEL_AXES,
        &Load::new(sp_torque.peak, PROBE),
        sp_e,
    )
    .ok_or(TrainError::NoContact)?;
    let pr_probe = contact_stress(
        &pr_path,
        &pr_mesh,
        &planet,
        PARALLEL_AXES,
        &Load::new(pr_torque.peak, PROBE),
        pr_e,
    )
    .ok_or(TrainError::NoContact)?;

    // **Four bendings, because there are four (member, mesh) pairs** — the sun
    // and the planet in the sun mesh, the planet and the ring in the ring mesh.
    // Each carries its own section, its own share of the mesh load, and what
    // the sharing model has to say about that mesh.
    let sharing = stage.load_sharing;
    let sun_bending = super::Bending::of(
        &sun,
        sp_path.contact_ratio,
        sharing,
        stage.sun.rim_thickness,
    )
    .ok_or(TrainError::NoRootSection)?;
    let planet_bending = super::Bending::of(
        &planet,
        sp_path.contact_ratio,
        sharing,
        stage.planet.rim_thickness,
    )
    .ok_or(TrainError::NoRootSection)?;
    // The planet's *other* root: a different section under a different force,
    // since the sun and the ring load its two flanks and neither is its rating
    // by right.
    let planet_ring_bending = super::Bending::of(
        &planet,
        pr_path.contact_ratio,
        sharing,
        stage.planet.rim_thickness,
    );
    // **A ring that cannot be rated for bending refuses the rating, not the
    // stage.**
    //
    // The commonest way to get here is a ring with no fillet, and its geometry
    // exists perfectly well: it draws, it exports, it meshes, and every figure
    // that does not need a notch is still answerable — the ratios, both contact
    // stresses, the efficiencies, the cycles, and the sun and planet's own
    // bending. What cannot be had is `Y_S`, whose input is a fillet radius, and
    // a stage that threw all of the rest away over one missing input would be
    // deciding for the designer instead of informing them.
    //
    // Nothing downstream needs telling: a bending stress is already an `Option`
    // for the worm stage's sake, so the width it would have asked for is simply
    // not asked for, and the member's own clamps say why the fillet is missing.
    let ring_bending = super::Bending::of(
        &ring,
        pr_path.contact_ratio,
        sharing,
        stage.ring.rim_thickness,
    );

    // Every rating is linear or square-root in the torque, and the set's power
    // split does not depend on the *magnitude* passing through it, so once each
    // mesh's peak torque is known the other case is a scale rather than a second
    // kinematic solve. **The two directions are not** — they are two solves, and
    // `sp_torque` / `pr_torque` are where that was done.
    let sp_scale = sp_torque.as_fraction_of_peak();
    let pr_scale = pr_torque.as_fraction_of_peak();

    // An automatic width with every source switched off stands at the number in
    // its box, as in the spur stage (`FaceSources::width_for`).
    //
    // **Which members are loaded on both flanks.** The planet structurally —
    // sun on one flank, ring on the other, whatever the drive does — and all
    // three when the drive itself reverses. A reversing drive does not make a
    // planet *more* reversed, so the two do not stack: `Reversal::reverses`
    // takes the member's own answer or the drive's, never both.
    let reverses = [
        reversal.reverses(false),
        reversal.reverses(true),
        reversal.reverses(false),
    ];
    // What the rating has to say about each member, kept **on** the member: two
    // of them raising the same note would give one list two entries under one
    // key, which is not something a keyed list can draw.
    // The rim under each member, where one was described. Per member rather
    // than per mesh, which is why the planet's two `Bending`s give one entry.
    let rims = [
        sun_bending.rim,
        planet_bending.rim,
        ring_bending.as_ref().and_then(|b| b.rim),
    ];
    // The two rack-cut members; a ring is not asked about undercut.
    let cut_by_a_rack = [Some(&sun), Some(&planet), None];
    let gear_notes = |i: usize| {
        let mut out = Vec::new();
        // ...and whether this member's rim is thinner than the clause will
        // rate. One rim per member, so unlike the notch it is asked once.
        out.extend(super::rim_below_minimum(rims[i]));
        out.extend(cut_by_a_rack[i].and_then(super::undercut_note));
        out.extend(reversal.note_for(reverses[i]));
        // **A bound that moved this member's own number belongs to it**, not to
        // a list at the foot of the stage that a reader has to match back up by
        // tooth count. The ring has none: it is asked neither question.
        out.extend(notes_for(i));
        out
    };

    let mut no_source = Vec::new();
    let mut ask_of = |name: &str, g: &StageGear, asks: &LoadCase<Widths>| -> f64 {
        if g.face_width.auto && !g.face_sources.any() {
            no_source
                .push(Note::new(key::STAGE_FACE_WIDTH_NO_SOURCE).text("gear", name.to_string()));
        }
        g.face_sources.width_for(asks, g.face_width.manual)
    };

    let probe_load_sp = Load::new(sp_torque.peak, PROBE);
    let probe_load_pr = Load::new(pr_torque.peak, PROBE);
    // The share this tooth carries where it is rated — exactly 1 unless a
    // sharing model was asked for, so nothing scales by default.
    // **`F_t` is the mesh's**, so a member is named only to say which reference
    // cylinder it is read at — and the ring, which has no rack-cut tooth, is no
    // longer handed the planet's to stand in for one.
    let stress_at = |b: &super::Bending, ft: f64, load: &Load| {
        bending_stress(
            &b.section,
            ft,
            load.face_width,
            RootStressModel::DolanBroghamer,
            b.rim,
        )
        .map(|s| s * b.share)
    };
    // One tangential force per mesh, read at either member's reference cylinder
    // because they are equal there — so the ring's rating asks nothing of the
    // planet beyond the mesh they are both in.
    let ft_sp = probe_load_sp.tangential(&sun);
    let ft_pr = probe_load_pr.tangential(&planet);
    let sun_sf = stress_at(&sun_bending, ft_sp, &probe_load_sp);
    let planet_sf = stress_at(&planet_bending, ft_sp, &probe_load_sp);
    let ring_sf = ring_bending
        .as_ref()
        .and_then(|b| stress_at(b, ft_pr, &probe_load_pr));
    let planet_ring_sf = planet_ring_bending
        .as_ref()
        .and_then(|b| stress_at(b, ft_pr, &probe_load_pr));

    // **What each member's ratings come to**, from the probe pass — the same
    // four questions every stage kind asks of every member it builds, asked
    // through the one type that answers them (`train::MemberRating`). Bending
    // takes whatever the reversal leaves; contact takes the material's own
    // figure, since pitting is compressive on whichever flank carries it, and
    // both of those live in there rather than in a pair of closures here.
    //
    // **The set is three members over two meshes**, written as which meshes each
    // member is in. The planet is the one in both, and it is the reason this is
    // a list: taking the worse of two figures was written out for contact, left
    // out for bending, and is one fold over the list for either.
    let loading = |bending, contact, carried_at| Loading {
        bending,
        contact,
        measured_at: PROBE,
        carried_at,
    };
    let rating = |i: usize, sp: f64, pr: f64| MemberRating {
        material: &mats[i],
        reversal,
        reverses: reverses[i],
        loadings: Loading::both_cases(&match i {
            0 => vec![(loading(sun_sf, sp_probe.governing(0), sp), sp_scale)],
            1 => vec![
                (loading(planet_sf, sp_probe.governing(1), sp), sp_scale),
                (loading(planet_ring_sf, pr_probe.governing(0), pr), pr_scale),
            ],
            _ => vec![(loading(ring_sf, pr_probe.governing(1), pr), pr_scale)],
        }),
    };
    let ratings: [MemberRating; 3] = std::array::from_fn(|i| rating(i, PROBE, PROBE));
    let asks = [
        ask_of("sun", &stage.sun, &ratings[0].asks()),
        ask_of("planet", &stage.planet, &ratings[1].asks()),
        ask_of("ring", &stage.ring, &ratings[2].asks()),
    ];
    notes.extend(no_source);
    // What the sharing model has to say, mesh by mesh: two meshes can be in
    // different bands, and a set with one extrapolating and one not should say
    // which. The note carries its own contact ratio, so two entries are two
    // findings rather than a repeat.
    for b in [Some(&sun_bending), planet_ring_bending.as_ref()] {
        notes.extend(b.and_then(|b| b.note.clone()));
    }
    // **A member's automatic width is the largest requirement of any mesh it is
    // in**, because the narrower face carries the pair — see the spur stage for
    // the fault this avoids. The planet is in both meshes, so it answers to
    // both; the sun and the ring answer to their own.
    let mesh_ask = [asks[0].max(asks[1]), asks[1].max(asks[2])];
    let wanted = [mesh_ask[0], mesh_ask[0].max(mesh_ask[1]), mesh_ask[1]];
    let widths = [
        stage.sun.face_width.resolve(wanted[0]),
        stage.planet.face_width.resolve(wanted[1]),
        stage.ring.face_width.resolve(wanted[2]),
    ];
    // The narrower member carries the mesh, per the specification.
    let sp_width = widths[0].min(widths[1]);
    let pr_width = widths[1].min(widths[2]);

    // **At the peak each mesh actually carries**, so the figures the mesh report
    // scales from are the ones the worse direction produces. The torque a member
    // is *labelled* with is its forward one, which is a different question and
    // is taken from `sp_forward` below.
    let sp_load = Load::new(sp_torque.peak, sp_width);
    let pr_load = Load::new(pr_torque.peak, pr_width);
    let sp_forward = Load::new(sun_torque_per_mesh, sp_width);
    let sp_cs = contact_stress(&sp_path, &sp_mesh, &sun, PARALLEL_AXES, &sp_load, sp_e)
        .ok_or(TrainError::NoContact)?;
    let pr_cs = contact_stress(&pr_path, &pr_mesh, &planet, PARALLEL_AXES, &pr_load, pr_e)
        .ok_or(TrainError::NoContact)?;

    // ---- centre distance and backlash.
    let centre = layout.centre_distance + stage.clearance.manual;
    let angular = |mesh: &Mesh, a: f64, at: MeshSide| -> f64 {
        mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees()
    };
    let backlash_of = |mesh: &Mesh, at: MeshSide| {
        Backlash::banded(centre, stage.tolerance_minus, stage.tolerance_plus, |d| {
            angular(mesh, d, at)
        })
    };

    // ---- layout.
    let planet_tip = 2.0 * planet.ra;
    let clearance = (stage.planets > 1).then(|| {
        2.0 * layout.centre_distance * (std::f64::consts::PI / planets).sin() - planet_tip
    });

    // ---- backlash, referred to whichever shaft is the output.
    //
    // Both meshes sit at the same centre distance, since the set is coaxial. In
    // the carrier's frame each mesh's play lets its members slip:
    //
    //     r′_s(θ_s − θ_c) + r′_p1(θ_p − θ_c) = δ₁      sun–planet
    //     r′_p2(θ_p − θ_c) − r′_r(θ_r − θ_c) = δ₂      planet–ring
    //
    // `r′_p1 ≠ r′_p2` in general — the two meshes have different operating
    // pressure angles, so the planet's operating radius differs between them,
    // and assuming one radius is the mistake waiting to be made here.
    // Eliminating the planet leaves one constraint on the three shafts:
    //
    //     z_s(θ_s − θ_c) + z_r(θ_r − θ_c) = Δ,   Δ = [(z_s+z_p)δ₁ − (z_r−z_p)δ₂]/a
    //
    // which is Willis at Δ = 0. Hold the two shafts that are not the output and
    // the third moves by `|Δ| / Z`, with `Z` its own coefficient above. The two
    // plays are independent, so their extremes add.
    //
    // The check that this is right: the same play measured at two different
    // output shafts must differ by exactly the ratio between them, and those
    // ratios come from `planetary::power` by a route sharing none of this.
    let zs = f64::from(teeth.sun);
    let zp = f64::from(teeth.planet);
    let zr = f64::from(teeth.ring);
    let coefficient = |m: PlanetaryShaft| match m {
        PlanetaryShaft::Sun => zs,
        PlanetaryShaft::Carrier => zs + zr,
        PlanetaryShaft::Ring => zr,
    };
    let referred = |at: PlanetaryShaft, a: f64| -> f64 {
        let j1 = sp_mesh.backlash(a).unwrap_or(0.0);
        let j2 = pr_mesh.backlash(a).unwrap_or(0.0);
        let delta = ((zs + zp) * j1.abs() + (zr - zp) * j2.abs()) / a;
        (delta / coefficient(at)).to_degrees()
    };
    let backlash_at = |at: PlanetaryShaft| Backlash {
        nominal: referred(at, centre),
        minimum: referred(at, centre - stage.tolerance_minus),
        maximum: referred(at, centre + stage.tolerance_plus),
    };
    let set_backlash = Directional {
        // Forward the output shaft is where the play shows; backward the shaft
        // that was driving becomes the one being measured.
        forward: backlash_at(forward.output),
        backward: backlash_at(stage.arrangement.input),
    };

    // ---- what the answer does not include.
    notes.push(Note::new(key::STAGE_PLANETS_SHARE_LOAD_EQUALLY).count("planets", stage.planets));
    // What the distance has to say — the same two findings every kind with a
    // centre distance can reach, in the same words (`train::distance_notes`).
    notes.extend(super::distance_notes(
        stage.nominal_distance(),
        centre,
        layout.centre_distance,
    ));
    notes.extend(chosen.how.note());
    if !layout.equal_spacing {
        notes.push(Note::new(key::STAGE_PLANETS_NOT_EVENLY_SPACED).count("planets", stage.planets));
    }
    if let Some(gap) = clearance {
        if gap < stage.min_planet_clearance {
            notes.push(
                Note::new(key::STAGE_PLANET_CLEARANCE_BELOW_MINIMUM)
                    .number("gap", gap, 3)
                    .number("minimum", stage.min_planet_clearance, 3),
            );
        }
    }
    // Asked by key. This used to search the clamp's *text* for "tip radius
    // raised", which is a sentence doing a symbol's job — one rewording, or one
    // translation, from silently doing nothing.
    if ring.clamps.iter().any(|c| c.is(key::CLAMP_RING_TIP_RAISED)) {
        notes.push(Note::new(key::STAGE_RING_ADDENDUM_CLAMPED));
    }

    // **The width a member is rated at is its mesh's, not its own.** The
    // narrower face carries the pair, so that is the width the load is spread
    // over — and it is the width the reported minimum has to be inverted at, or
    // a member wider than its mate is told it needs more face than it does, in
    // proportion to how much wider it is. A member in two meshes answers to each
    // at that mesh's own width, which is what `Loading` carries and why the
    // planet no longer needs a width picked for it.
    // **A member's speed is its own.** It used to be read out of the shaft array
    // by role, which gave the planet the *carrier's* — the shaft it rides rather
    // than the one it spins on, and on a set with the ring held not even the
    // same sign.
    let gear_result = |speed: f64,
                       input: &StageGear,
                       params: &GearParams,
                       which: usize,
                       torque: f64,
                       back_driving_torque: Option<f64>,
                       clamps: Vec<Note>,
                       notes: Vec<Note>|
     -> GearResult {
        // A load case is a scale on the torque, and every rating is linear or
        // square-root in it — so the peak and cyclic figures are the same
        // expression evaluated at the two scales rather than a second solve.
        // Which is what `MemberRating` is, for every stage kind at once.
        GearResult::of(super::MemberFacts {
            recommended_face_width: None,
            profile_shift: params.profile_shift,
            params,
            input,
            rated: rating(which, sp_width, pr_width).rated(),
            face_width: widths[which],
            torque,
            back_driving_torque,
            speed,
            material: mats[which].clone(),
            clamps,
            notes,
        })
    };

    // **The planet's own rotation**, from the kinematics rather than from the
    // shaft beside it: its absolute speed is not the carrier's, and what its
    // teeth see is not the sun's speed relative to the carrier
    // ([`crate::planetary::Power::planet_speed`]).
    let (planet_absolute, planet_relative) = forward.planet_speed(teeth);

    Ok(PlanetaryResult {
        arrangement: stage.arrangement,
        output: forward.output,
        ratio: forward.ratio,
        clearance: centre - layout.centre_distance,
        centre_distance_nominal: layout.centre_distance,
        centre_distance: centre,
        fixed_carrier_efficiency: eta0,
        efficiency: set_efficiency,
        backlash: set_backlash,
        speeds: forward.speeds,
        torques: forward.torques,
        sun_planet: MeshReport {
            flank_interference: sp_mesh.flank_interference([sun.flank_ends(), planet.flank_ends()]),
            operating_pressure_angle: sp_mesh.alpha_w.to_degrees(),
            coprime: super::gcd(teeth.sun, teeth.planet) == 1,
            contact_ratios: ContactRatios::of(
                sp_path.contact_ratio,
                sp_width,
                stage.helix_angle,
                stage.module,
            ),
            efficiency: sp_eff,
            contact_stress_at_pitch_point: LoadCase::of(|c| {
                sp_cs.at_pitch_point * sp_scale.get(c).sqrt()
            }),
            relative_radius: sp_cs.relative_radius,
            backlash: [
                backlash_of(&sp_mesh, MeshSide::First),
                backlash_of(&sp_mesh, MeshSide::Second),
            ],
            // Sun to planet is an external mesh.
            tips: None,
        },
        planet_ring: MeshReport {
            // **The ring answers as a ring**, not as the `Tooth` the mesh
            // arithmetic reads it through: its flank runs outwards from its tip
            // and ends at its shaper's fillet, which only a `Ring` knows.
            flank_interference: pr_mesh
                .flank_interference([planet.flank_ends(), ring.flank_ends()]),
            operating_pressure_angle: pr_mesh.alpha_w.to_degrees(),
            coprime: super::gcd(teeth.planet, teeth.ring) == 1,
            contact_ratios: ContactRatios::of(
                pr_path.contact_ratio,
                pr_width,
                stage.helix_angle,
                stage.module,
            ),
            efficiency: pr_eff,
            contact_stress_at_pitch_point: LoadCase::of(|c| {
                pr_cs.at_pitch_point * pr_scale.get(c).sqrt()
            }),
            relative_radius: pr_cs.relative_radius,
            backlash: [
                backlash_of(&pr_mesh, MeshSide::First),
                backlash_of(&pr_mesh, MeshSide::Second),
            ],
            // **And planet to ring is not**, which is the whole of what this
            // field is for: the set has an internal mesh in it and had never
            // been asked the three questions one answers.
            tips: super::TipRoom::of(&ring, &planet),
        },
        equal_spacing: layout.equal_spacing,
        simultaneous_meshing: layout.simultaneous_meshing,
        planet_clearance: clearance,
        planet_clearance_ok: clearance.is_none_or(|g| g >= stage.min_planet_clearance),
        sun_coprime_with_planets: super::gcd(teeth.sun, stage.planets.max(1)) == 1,
        ring_coprime_with_planets: super::gcd(teeth.ring, stage.planets.max(1)) == 1,
        sun: gear_result(
            forward.speeds[PlanetaryShaft::Sun.index_pub()],
            &stage.sun,
            &sun_params,
            0,
            forward.torques[0] / planets,
            backward_share(PlanetaryShaft::Sun.index_pub()),
            sun.clamps.notes.clone(),
            gear_notes(0),
        ),
        planet: PlanetResult {
            gear: gear_result(
                planet_absolute,
                &stage.planet,
                &planet_params,
                1,
                sp_forward.across_mesh(&sun, &planet).torque,
                // **A planet is not one of the three shafts**, so its share is
                // the sun's carried across the mesh they share — the same
                // projection its forward torque takes, on the backward figure.
                backward_share(PlanetaryShaft::Sun.index_pub())
                    .map(|t| Load::new(t, sp_width).across_mesh(&sun, &planet).torque),
                planet.clamps.notes.clone(),
                gear_notes(1),
            ),
            shift_residual: layout.residual,
            speed_relative: planet_relative,
        },
        planets: stage.planets,
        ring: gear_result(
            forward.speeds[PlanetaryShaft::Ring.index_pub()],
            &stage.ring,
            &ring_params,
            2,
            forward.torques[2] / planets,
            backward_share(PlanetaryShaft::Ring.index_pub()),
            ring.clamps.clone(),
            gear_notes(2),
        ),
        notes,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::train::test_library;

    /// **A probe width leaves no trace.**
    ///
    /// An epicyclic set rates once at `PROBE` and scales to the width each mesh
    /// carries — bending inversely with the width, contact with its square root
    /// (`Loading::at_width`). So the stress it reports must be the stress a
    /// direct evaluation at that width gives, and this asks for one.
    ///
    /// **Nothing asked before, and the reason is a coincidence of two
    /// constants**: `PROBE` is 10.0 and `StageGear`'s default face width is
    /// 10.0, so every shipped case scales by exactly one and the exponent could
    /// be anything. Perturbing it to 0.51 left all 558 tests and all 27 golden
    /// files unchanged. *Two unrelated numbers that happen to be equal will hide
    /// whatever lies between them.*
    ///
    /// Widths well away from the probe on both sides, so a scale that is wrong
    /// in either direction shows.
    #[test]
    fn a_probe_width_leaves_no_trace() {
        let lib = test_library();
        let mut checked = 0u32;
        for face in [2.5_f64, 10.0, 40.0] {
            let stage = PlanetaryStage {
                sun: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().sun
                },
                planet: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().planet
                },
                ring: StageGear {
                    face_width: Auto::fixed(face),
                    ..PlanetaryStage::default().ring
                },
                ..PlanetaryStage::default()
            };
            let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("face {face}: {e}"));
            let b = stage
                .built([
                    r.sun.profile_shift,
                    r.planet.gear.profile_shift,
                    r.ring.profile_shift,
                ])
                .expect("the set has geometry");

            // The sun mesh, evaluated where it is carried rather than scaled
            // there. Same `contact_stress`; what it does not share is the
            // scaling under test.
            let planets = f64::from(stage.planets.max(1));
            let load = Load::new((r.torques[0] / planets).abs(), face);
            let e_star = contact_modulus(
                &lib.get(&stage.sun.material).expect("a material").clone(),
                &lib.get(&stage.planet.material).expect("a material").clone(),
            );
            let direct =
                contact_stress(&b.sp_path, &b.sp_mesh, &b.sun, PARALLEL_AXES, &load, e_star)
                    .expect("the sun mesh has contact");

            let got = r.sun.contact_stress.peak;
            assert!(
                (got - direct.governing(0)).abs() < 1e-9 * direct.governing(0),
                "face {face}: the sun reports {got} MPa where a direct evaluation \
                 at that width gives {}",
                direct.governing(0)
            );
            checked += 1;
        }
        assert_eq!(checked, 3, "a width went unrun");
    }

    /// **A planet's root answers to both of its meshes.**
    ///
    /// The sun loads one flank and the ring the other, and it had only the
    /// sun's. The two tangential forces are equal — it is the same planet
    /// transmitting through — so what separates them is the section each mesh's
    /// contact ratio puts the load at, and the **width each mesh carries it
    /// over**: the narrower of that pair's two faces. A narrow ring is the
    /// ordinary way for the second mesh to be the worse one, and it is what the
    /// second fixture is.
    ///
    /// Both orderings are exercised deliberately. A test that only ever met the
    /// sun-governed case would pass against a stage that had gone back to
    /// looking at one mesh, which is exactly what the first draft of this did.
    #[test]
    fn a_planets_root_answers_to_both_of_its_meshes() {
        let lib = test_library();
        let mut sun_won = false;
        let mut ring_won = false;
        for ring_face in [10.0_f64, 3.0] {
            let stage = PlanetaryStage {
                ring: StageGear {
                    teeth: 60,
                    profile_shift: Auto::fixed(0.0),
                    face_width: Auto::fixed(ring_face),
                    ..StageGear::default()
                },
                ..PlanetaryStage::default()
            };
            let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("ring face {ring_face}: {e}"));
            let got = r
                .planet
                .gear
                .bending_stress
                .peak
                .expect("a planet has a root section");

            // The two contributions, rebuilt from what the result reports rather
            // than from the expression that produced them: each mesh's own
            // section under its own load, over the width that mesh carries.
            let built = stage.built(stage.shifts()).unwrap();
            let planets = f64::from(stage.planets);
            let sp_width = r.planet.gear.face_width.min(r.sun.face_width);
            let pr_width = r.planet.gear.face_width.min(r.ring.face_width);
            let each = |contact_ratio: f64, torque: f64, b: f64| {
                let section =
                    crate::strength::bending_section(&built.planet, contact_ratio).unwrap();
                bending_stress(
                    &section,
                    Load::new(torque, b).tangential(&built.planet),
                    b,
                    RootStressModel::DolanBroghamer,
                    None,
                )
                .unwrap()
            };
            let from_sun = each(
                built.sp_path.contact_ratio,
                Load::new((r.torques[0] / planets).abs(), sp_width)
                    .across_mesh(&built.sun, &built.planet)
                    .torque,
                sp_width,
            );
            let from_ring = each(
                built.pr_path.contact_ratio,
                (r.torques[2] / planets).abs() / built.pr_mesh.ratio(),
                pr_width,
            );
            assert!(
                (got - from_sun.max(from_ring)).abs() < 1e-9 * got,
                "ring face {ring_face}: reported {got}, sun {from_sun}, ring {from_ring}"
            );
            sun_won |= from_sun > from_ring;
            ring_won |= from_ring > from_sun;
        }
        assert!(
            sun_won && ring_won,
            "one fixture each way, or the max is never asked a question"
        );
    }

    /// **A ring that cannot be rated for bending costs the rating, not the set.**
    ///
    /// A planetary set gives its ring `k = 2 − k_stage`, so an ordinary stage
    /// thickness modification of 1.4 puts the ring at 0.6 — thick enough that
    /// the cutter which would leave its space comes to a point before its own
    /// tip, and no fillet is generated. There is then no notch, so no `Y_S`, so
    /// no bending number for that member.
    ///
    /// None of which stops the set existing. The geometry draws, exports and
    /// meshes, and the ratio, both contact stresses, the efficiencies, the
    /// cycles and the other two members' bending are all still answerable. A
    /// stage that threw those away over one missing input would be deciding for
    /// the designer rather than informing them, which is the opposite of what
    /// this tool is for.
    ///
    /// Run against the code this replaced, every case below is
    /// `Err(NoRootSection)` — and the message blamed undercut, which nothing
    /// here is.
    #[test]
    fn a_ring_with_no_notch_costs_its_bending_rather_than_the_stage() {
        let lib = test_library();
        for k in [1.3_f64, 1.4, 1.5, 1.7] {
            let stage = PlanetaryStage {
                thickness_mod: k,
                ..Default::default()
            };
            let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("k={k}: the set should still solve, got {e}"));

            assert!(
                r.ring.bending_stress.peak.is_none(),
                "k={k}: a ring with no notch cannot have a bending stress"
            );
            // ...and everything that never needed the notch is still there.
            assert!(r.ratio.is_finite() && r.ratio != 0.0, "k={k}: no ratio");
            assert!(
                r.sun.bending_stress.peak.is_some(),
                "k={k}: the sun's own bending went with it"
            );
            for (name, gear) in [("sun", &r.sun), ("ring", &r.ring)] {
                assert!(
                    gear.contact_stress.peak > 0.0,
                    "k={k}: {name} lost its contact stress"
                );
            }
            // The member says why, so the blank is read rather than guessed at.
            assert!(
                !r.ring.clamps.is_empty(),
                "k={k}: the ring reports no reason for having no fillet"
            );
        }
    }

    fn stage_of(sun: u32, planet: u32, ring: u32, helix: f64) -> PlanetaryStage {
        PlanetaryStage {
            helix_angle: helix,
            sun: StageGear {
                teeth: sun,
                ..StageGear::default()
            },
            planet: StageGear {
                teeth: planet,
                ..StageGear::default()
            },
            ring: StageGear {
                teeth: ring,
                profile_shift: Auto::fixed(0.0),
                ..StageGear::default()
            },
            ..PlanetaryStage::default()
        }
    }

    fn solved(sun: u32, planet: u32, ring: u32) -> PlanetaryResult {
        solve_planetary_stage(
            &stage_of(sun, planet, ring, 0.0),
            3000.0,
            StageTorques::just(2.0),
            &test_library(),
        )
        .unwrap()
    }

    /// **With a distance given, only one shift is free** — asserted against what
    /// the set actually does, not against the number the declaration carries.
    ///
    /// A set's two distances must agree, which is one relation among its three
    /// shifts. Give it a distance as well and there is a second — each mesh must
    /// reach *that* distance — so one shift is a design and two are absorbed.
    /// `Stage::freedoms` says so by reading the distance's toggle, and this is
    /// what makes that reading true rather than declared: give **two** shifts at
    /// a given distance and one of them cannot survive.
    ///
    /// Checking the declaration against the declaration is what
    /// `docs/corrections.md` calls a check built from the thing under test. The
    /// first version of this did exactly that and passed against a limit hard
    /// -wired to the wrong number.
    #[test]
    fn a_given_distance_leaves_a_set_one_free_shift() {
        let lib = super::super::test_library();
        let base = PlanetaryStage::default();
        let free = solve_planetary_stage(&base, 3000.0, StageTorques::just(2.0), &lib)
            .expect("the shipped set solves");
        let asked = free.centre_distance + 0.1;

        // One shift given — the ring's, as the shipped set has it — and every
        // given number stands.
        let mut one = base.clone();
        one.centre_distance = Auto::fixed(asked);
        let r = solve_planetary_stage(&one, 3000.0, StageTorques::just(2.0), &lib)
            .expect("one free shift is enough");
        assert!((r.centre_distance - asked).abs() < 1e-9);
        assert!((r.ring.profile_shift - one.ring.profile_shift.manual).abs() < 1e-12);

        // A second shift given, and it cannot also stand: three relations'
        // worth of demands on two freedoms.
        let mut two = one.clone();
        two.sun.profile_shift = Auto::fixed(r.sun.profile_shift + 0.25);
        let over = solve_planetary_stage(&two, 3000.0, StageTorques::just(2.0), &lib)
            .expect("it still builds; it just cannot honour everything");
        let honoured = [
            (over.centre_distance - asked).abs() < 1e-9,
            (over.sun.profile_shift - two.sun.profile_shift.manual).abs() < 1e-9,
            (over.ring.profile_shift - two.ring.profile_shift.manual).abs() < 1e-9,
        ];
        assert!(
            honoured.iter().any(|ok| !ok),
            "a distance and two shifts were all honoured, so the set has more \
             freedom than its declaration claims: {honoured:?}"
        );

        // ...and the declaration says the same thing, read from the toggles.
        use super::super::{Freedom, Stage};
        let flat = Stage::Planetary(Box::new(one.clone()));
        let shifts = flat
            .freedoms()
            .into_iter()
            .find(|g| g.order.iter().all(|f| matches!(f, Freedom::Shift(_))))
            .expect("a set declares a group over its shifts");
        assert_eq!(shifts.given_at_most, 1, "a given distance leaves one shift");

        let mut loose = one.clone();
        loose.centre_distance = Auto::automatic(0.0);
        let free_group = Stage::Planetary(Box::new(loose))
            .freedoms()
            .into_iter()
            .find(|g| g.order.iter().all(|f| matches!(f, Freedom::Shift(_))))
            .expect("a set declares a group over its shifts");
        assert_eq!(
            free_group.given_at_most, 2,
            "with the distance free, two shifts are a design"
        );
    }

    /// **A set runs at the centre distance it was given**, and both of its
    /// meshes do — which is the whole of F39's third item.
    ///
    /// A planetary set had no distance input at all: the common distance was
    /// whatever the shifts left. Given one, each mesh has a shift sum it must
    /// reach to run at it, and both of those are closed form — so a target makes
    /// the layout *easier* and the Newton iteration disappears.
    ///
    /// Three claims. The distance asked for is the distance run at; the two
    /// meshes agree there to the bit (`residual`, which is the layout's own
    /// measure of whether it closed); and a shift the designer **gave** is
    /// untouched, since with one freedom left it is the freedom.
    #[test]
    fn a_set_runs_at_the_centre_distance_it_was_given() {
        let lib = super::super::test_library();
        let base = PlanetaryStage::default();
        let free = solve_planetary_stage(&base, 3000.0, StageTorques::just(2.0), &lib)
            .expect("the shipped set solves");

        let mut checked = 0u32;
        for step in -2..=4 {
            let asked = free.centre_distance + 0.2 * f64::from(step);
            let mut stage = base.clone();
            stage.centre_distance = Auto::fixed(asked);
            let Ok(r) = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &lib) else {
                // A distance no set can reach is refused, not answered — which
                // is the honest end of the range rather than a gap in it.
                continue;
            };
            checked += 1;

            assert!(
                (r.centre_distance - asked).abs() < 1e-9,
                "asked {asked}, ran at {}",
                r.centre_distance
            );
            assert!(
                (r.clearance - stage.clearance.manual).abs() < 1e-9,
                "the clearance asked for should be the clearance left: {}",
                r.clearance
            );

            // Both meshes at that distance, measured by the layout rather than
            // asserted from the same expression that placed the shifts.
            let built = stage.built(stage.shifts()).expect("it just solved");
            assert!(
                built.layout.residual < 1e-12,
                "the two meshes disagree by {} at a given distance",
                built.layout.residual
            );

            // The ring's shift is given on the shipped set, so it is the one
            // freedom a target leaves and must come back untouched.
            assert!(
                (r.ring.profile_shift - stage.ring.profile_shift.manual).abs() < 1e-12,
                "a given shift moved: {} for {}",
                r.ring.profile_shift,
                stage.ring.profile_shift.manual
            );
        }
        assert!(checked >= 5, "only {checked} distances were reachable");
    }

    /// **The constraint that makes it a planetary set**: sun-to-planet and
    /// planet-to-ring are one distance measured twice, and the planet's shift is
    /// what makes them agree.
    #[test]
    fn the_two_centre_distances_are_one_number() {
        for (s, p, r) in [
            (24u32, 18u32, 60u32),
            (17, 17, 52),
            (20, 20, 62),
            (30, 15, 62),
        ] {
            let res = solved(s, p, r);
            assert!(
                res.planet.shift_residual < 1e-12,
                "z={s}/{p}/{r}: residual {} mm",
                res.planet.shift_residual
            );
            assert!(res.centre_distance > 0.0);
        }
        // The ideal ring needs no shift at all, and gets exactly none.
        let ideal = solved(24, 18, 60);
        assert!(ideal.planet.gear.profile_shift.abs() < 1e-12);
        assert!((ideal.centre_distance_nominal - 21.0).abs() < 1e-12);
    }

    /// The classical ratios, through the whole stage rather than the bare
    /// algebra — so a wiring error between them would show.
    /// **Any one of the three shifts can close the set**, and the other two are
    /// then exactly what was asked for.
    ///
    /// The relation is that the two centre distances agree, so whichever member
    /// absorbs it, the answer has to satisfy the same equality — which is what
    /// `residual` reports and what this checks rather than checking the wiring.
    /// The two that did not absorb must come back untouched, since a shift a
    /// designer gave is not the solve's to move.
    #[test]
    fn whichever_shift_is_left_automatic_is_the_one_that_closes_the_set() {
        use crate::planetary::Member;
        let lib = test_library();
        let base = PlanetaryStage::default();
        // The shifts the default set settles at, so each variant below asks for
        // values a set of these counts can actually be built at.
        let settled = base.shifts();

        for absorber in [Member::Sun, Member::Planet, Member::Ring] {
            let mut s = PlanetaryStage::default();
            // Pin every member but the one meant to absorb.
            for (i, gear) in [&mut s.sun, &mut s.planet, &mut s.ring]
                .into_iter()
                .enumerate()
            {
                gear.profile_shift = if i == absorber.index() {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(settled[i])
                };
            }
            assert_eq!(
                s.absorber(),
                absorber,
                "the member left automatic should be the one that absorbs"
            );
            let r = solve_planetary_stage(&s, 100.0, StageTorques::just(2.0), &lib)
                .unwrap_or_else(|e| panic!("{absorber:?} could not close the set: {e:?}"));

            // The equality actually closed...
            assert!(
                r.planet.shift_residual.abs() < 1e-9,
                "{absorber:?}: the two centre distances differ by {}",
                r.planet.shift_residual
            );
            // ...and the given members were left exactly as given.
            let got = [
                r.sun.profile_shift,
                r.planet.gear.profile_shift,
                r.ring.profile_shift,
            ];
            for i in 0..3 {
                if i == absorber.index() {
                    continue;
                }
                assert!(
                    (got[i] - settled[i]).abs() < 1e-9,
                    "{absorber:?}: member {i} was given {} and came back {}",
                    settled[i],
                    got[i]
                );
            }
        }
    }

    /// **The default set is the planet's**, which is what it has always been.
    #[test]
    fn the_planet_closes_the_set_unless_it_is_pinned() {
        use crate::planetary::Member;
        assert_eq!(PlanetaryStage::default().absorber(), Member::Planet);
        let mut s = PlanetaryStage::default();
        s.planet.profile_shift = Auto::fixed(0.0);
        assert_eq!(
            s.absorber(),
            Member::Sun,
            "pinning the planet hands it to the sun"
        );
        s.sun.profile_shift = Auto::fixed(0.0);
        // The shipped ring's shift is given, so with the other two pinned as
        // well nothing is left automatic and the set is over-specified — the
        // planet gives way, and the panel relieves it as it is created.
        assert_eq!(
            s.absorber(),
            Member::Planet,
            "over-specified, the planet gives way"
        );
        s.ring.profile_shift = Auto::automatic(0.0);
        assert_eq!(
            s.absorber(),
            Member::Ring,
            "...and a ring left free takes it instead"
        );
    }

    #[test]
    fn the_stage_reports_the_classical_ratios() {
        let want = [
            (
                PlanetaryShaft::Sun,
                PlanetaryShaft::Ring,
                PlanetaryShaft::Carrier,
                3.5,
            ),
            (
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
                PlanetaryShaft::Ring,
                -2.5,
            ),
            (
                PlanetaryShaft::Ring,
                PlanetaryShaft::Sun,
                PlanetaryShaft::Carrier,
                1.4,
            ),
        ];
        for (input, fixed, output, ratio) in want {
            let stage = PlanetaryStage {
                arrangement: Arrangement { input, fixed },
                ..stage_of(24, 18, 60, 0.0)
            };
            let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &test_library())
                .unwrap();
            assert_eq!(r.output, output);
            assert!(
                (r.ratio - ratio).abs() < 1e-12,
                "{input:?}/{fixed:?}: {}",
                r.ratio
            );
        }
    }

    /// **A held carrier makes the set two meshes in series**, so its efficiency
    /// must be exactly the product of theirs — through the stage, not just the
    /// algebra.
    #[test]
    fn a_held_carrier_gives_exactly_the_product_of_the_mesh_efficiencies() {
        let stage = PlanetaryStage {
            arrangement: Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Carrier,
            },
            ..stage_of(24, 18, 60, 0.0)
        };
        let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &test_library())
            .unwrap();
        let product = r.sun_planet.efficiency.forward * r.planet_ring.efficiency.forward;
        assert!((r.fixed_carrier_efficiency.forward - product).abs() < 1e-15);
        assert!(
            (r.efficiency.forward - product).abs() < 1e-12,
            "{}",
            r.efficiency.forward
        );
    }

    /// **The internal mesh is the gentler one**, in both the ways it should be:
    /// more contact and less pressure. Both were proved as laws in `ring.rs`;
    /// asserting them here says the stage wired the two meshes the right way
    /// round, which no amount of core testing would catch.
    #[test]
    fn the_internal_mesh_carries_better_than_the_external_one() {
        for (s, p, r) in [(24u32, 18u32, 60u32), (17, 17, 52), (30, 15, 62)] {
            let res = solved(s, p, r);
            assert!(
                res.planet_ring.contact_ratios.transverse
                    > res.sun_planet.contact_ratios.transverse,
                "z={s}/{p}/{r}: internal contact ratio {} not above external {}",
                res.planet_ring.contact_ratios.transverse,
                res.sun_planet.contact_ratios.transverse
            );
            assert!(
                res.planet_ring.relative_radius > res.sun_planet.relative_radius,
                "z={s}/{p}/{r}: internal relative radius should be the larger"
            );
            // ...and a ring's tooth is the stronger, so it carries the less
            // bending stress. Every member is rated: a ring's critical section
            // sits on its involute flank for most tooth counts, and that used to
            // withhold the figure entirely — see
            // `the_rating_is_continuous_across_the_flank_fillet_transition`.
            let (sun_s, ring_s) = (
                res.sun
                    .bending_stress
                    .peak
                    .expect("the sun is always rated"),
                res.ring.bending_stress.peak.expect("and so is the ring"),
            );
            assert!(
                ring_s < sun_s,
                "z={s}/{p}/{r}: ring {ring_s} vs sun {sun_s}"
            );
        }
    }

    /// **The backlash referral, against the kinematics.**
    ///
    /// The same play measured at two different output shafts must differ by
    /// exactly the ratio between them — and those ratios come from
    /// `planetary::power`, which shares none of the referral's algebra. That is
    /// what makes this a check rather than a restatement.
    ///
    /// It is also the law the train-level test uses on a multi-stage train
    /// ("backlash at the two ends differs by exactly the total ratio"), asked of
    /// one stage with three shafts instead of a line of two-shaft ones.
    #[test]
    fn backlash_referred_to_two_shafts_differs_by_exactly_their_ratio() {
        let lib = test_library();
        for (s, p, r) in [(24u32, 18u32, 60u32), (17, 17, 52), (30, 15, 62)] {
            // Ring held: the sun and the carrier are the two possible outputs.
            let sun_in = PlanetaryStage {
                arrangement: Arrangement {
                    input: PlanetaryShaft::Sun,
                    fixed: PlanetaryShaft::Ring,
                },
                ..stage_of(s, p, r, 0.0)
            };
            let carrier_in = PlanetaryStage {
                arrangement: Arrangement {
                    input: PlanetaryShaft::Carrier,
                    fixed: PlanetaryShaft::Ring,
                },
                ..stage_of(s, p, r, 0.0)
            };
            let a = solve_planetary_stage(&sun_in, 3000.0, StageTorques::just(2.0), &lib).unwrap();
            let b =
                solve_planetary_stage(&carrier_in, 3000.0, StageTorques::just(2.0), &lib).unwrap();

            // `a` outputs at the carrier, `b` at the sun.
            let at_carrier = a.backlash.forward.nominal;
            let at_sun = b.backlash.forward.nominal;
            assert!(at_carrier > 0.0 && at_sun > 0.0);
            assert!(
                (at_sun - at_carrier * a.ratio).abs() < 1e-9 * at_sun,
                "z={s}/{p}/{r}: {at_sun} vs {at_carrier} x {}",
                a.ratio
            );
            // ...and the shaft that turns faster carries the looser play.
            assert!(at_sun > at_carrier);
        }
    }

    /// Both meshes contribute, and more play in either loosens the output.
    ///
    /// A referral that dropped one mesh would still satisfy the ratio law above,
    /// since that law is about *where* the play is measured rather than where it
    /// came from — so it needs saying separately.
    #[test]
    fn both_meshes_contribute_to_the_output_backlash() {
        let lib = test_library();
        let base = stage_of(24, 18, 60, 0.0);
        let tight = solve_planetary_stage(&base, 3000.0, StageTorques::just(2.0), &lib).unwrap();

        // More clearance opens both meshes, so the output must loosen.
        let loose = PlanetaryStage {
            clearance: Auto::fixed(base.clearance.manual + 0.05),
            ..base.clone()
        };
        let loose = solve_planetary_stage(&loose, 3000.0, StageTorques::just(2.0), &lib).unwrap();
        assert!(
            loose.backlash.forward.nominal > tight.backlash.forward.nominal,
            "{} should exceed {}",
            loose.backlash.forward.nominal,
            tight.backlash.forward.nominal
        );

        // And the tolerance band brackets the nominal, as it does everywhere else.
        let b = &tight.backlash.forward;
        assert!(b.minimum < b.nominal && b.nominal < b.maximum);

        // At the zero-backlash centre distance there is no play at all.
        let exact = PlanetaryStage {
            clearance: Auto::fixed(0.0),
            tolerance_plus: 0.0,
            tolerance_minus: 0.0,
            ..base
        };
        let exact = solve_planetary_stage(&exact, 3000.0, StageTorques::just(2.0), &lib).unwrap();
        assert!(
            exact.backlash.forward.nominal < 1e-12,
            "zero clearance must give zero play, got {}",
            exact.backlash.forward.nominal
        );
    }

    /// The planet turns at a speed measured **relative to the carrier**, which
    /// is what its teeth actually see.
    #[test]
    fn the_planet_is_reported_as_the_special_case_it_is() {
        let r = solved(24, 18, 60);
        assert!(r.planet.speed_relative.abs() > 0.0);
        assert!((r.planet.speed_relative - r.planet.gear.speed).abs() > 1e-9);
    }

    /// **A planet's root is loaded both ways, and what to do about it is asked
    /// rather than assumed.**
    ///
    /// The derate is a convention — a fraction on an allowable a part is sized
    /// against — so it is a switch, off by default, and the stage says which of
    /// its members the reversal reaches either way. It used to be applied to the
    /// planet silently, and to the planet alone, so a reversing *drive* derated
    /// nothing while a set nobody had told anything about derated one member.
    #[test]
    fn a_reversed_root_is_corrected_only_when_the_train_asks() {
        let lib = test_library();
        let stage = PlanetaryStage::default();
        let solve = |reversal: crate::train::Reversal| {
            solve_planetary_stage_with(&stage, 3000.0, StageTorques::just(2.0), &lib, reversal)
                .unwrap()
        };
        // **On the member, not the stage.** Three members raising one note is
        // exactly what a stage-level list could not carry: one key, three
        // entries, and a keyed list in the front end that cannot draw it.
        let fired = |r: &PlanetaryResult, k: &str| {
            [&r.sun, &r.planet.gear, &r.ring]
                .iter()
                .filter(|g| g.notes.iter().any(|n| n.is(k)))
                .count()
        };

        // Off: nothing is derated, and the planet's reversal is disclosed.
        let plain = solve(crate::train::Reversal::default());
        assert_eq!(fired(&plain, key::STAGE_REVERSED_BENDING_UNCORRECTED), 1);
        assert_eq!(fired(&plain, key::STAGE_REVERSED_BENDING_APPLIED), 0);

        // On: the same member is derated, and the note says so instead.
        let corrected = solve(crate::train::Reversal {
            drive_reverses: false,
            correct: true,
        });
        assert_eq!(fired(&corrected, key::STAGE_REVERSED_BENDING_APPLIED), 1);
        assert_eq!(
            fired(&corrected, key::STAGE_REVERSED_BENDING_UNCORRECTED),
            0
        );

        // A smaller allowable asks for more face, and only for the planet.
        let width = |r: &PlanetaryResult, g: &GearResult| {
            let _ = r;
            g.min_face_width.cyclic.bending.unwrap()
        };
        assert!(
            width(&corrected, &corrected.planet.gear) > width(&plain, &plain.planet.gear) * 1.2,
            "the correction must reach the planet's minimum width"
        );
        for (name, a, b) in [
            ("sun", &corrected.sun, &plain.sun),
            ("ring", &corrected.ring, &plain.ring),
        ] {
            assert_eq!(
                width(&corrected, a).to_bits(),
                width(&plain, b).to_bits(),
                "{name}: a one-way root must not be derated"
            );
        }

        // A reversing **drive** reverses all three, and does not stack with the
        // planet's own — which is the whole point of asking `reverses` once.
        let driven = solve(crate::train::Reversal {
            drive_reverses: true,
            correct: true,
        });
        assert_eq!(fired(&driven, key::STAGE_REVERSED_BENDING_APPLIED), 3);
        // ...and no member's own list carries one note twice, which is the shape
        // that broke the panel: a keyed list cannot draw two of one key.
        for g in [&driven.sun, &driven.planet.gear, &driven.ring] {
            let mut keys: Vec<&str> = g.notes.iter().map(|n| n.key.as_str()).collect();
            let before = keys.len();
            keys.sort_unstable();
            keys.dedup();
            assert_eq!(before, keys.len(), "a member repeated a note key");
        }
        assert_eq!(
            width(&driven, &driven.planet.gear).to_bits(),
            width(&corrected, &corrected.planet.gear).to_bits(),
            "a reversing drive cannot make a planet more reversed than it is"
        );
    }

    /// Layout is arithmetic on the tooth counts, and it reaches the result.
    #[test]
    fn the_layout_checks_reach_the_result() {
        let r = solved(24, 18, 60);
        assert!(r.equal_spacing, "(24+60)/3 = 28");
        assert!(r.planet_clearance.unwrap() > 0.0);
        assert!(r.planet_clearance_ok);

        // A single planet has no neighbour to clear, and says so rather than
        // reporting a gap of nothing.
        let one = PlanetaryStage {
            planets: 1,
            ..stage_of(24, 18, 60, 0.0)
        };
        let r =
            solve_planetary_stage(&one, 3000.0, StageTorques::just(2.0), &test_library()).unwrap();
        assert!(r.planet_clearance.is_none());
        assert!(r.planet_clearance_ok);
    }

    /// **Helical works, to parity with spur.** Every figure a spur set reports,
    /// a helical one reports too — including the ring's bending, which goes
    /// through the virtual spur ring.
    #[test]
    fn a_helical_set_reports_everything_a_spur_one_does() {
        for helix in [10.0, 20.0, 30.0] {
            let stage = stage_of(24, 18, 60, helix);
            let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &test_library())
                .unwrap_or_else(|e| panic!("helix={helix}: {e}"));
            assert!(r.sun.bending_stress.peak.is_some(), "helix={helix}: sun");
            assert!(
                r.planet.gear.bending_stress.peak.is_some(),
                "helix={helix}: planet"
            );
            assert!(r.ring.bending_stress.peak.is_some(), "helix={helix}: ring");
            assert!(r.sun_planet.contact_ratios.overlap > 0.0, "helix={helix}");
            assert!(r.planet.shift_residual < 1e-12);
        }
    }

    /// Tooth counts that admit no planet shift are refused, not fudged into an
    /// answer. Most combinations are impossible (docs/reference.md#planetary-sets) and that is the common
    /// case rather than an exceptional one.
    #[test]
    fn an_impossible_set_is_refused() {
        assert!(solve_planetary_stage(
            &stage_of(24, 18, 200, 0.0),
            3000.0,
            StageTorques::just(2.0),
            &test_library()
        )
        .is_err());
    }

    /// The thickness invariants differ between the two meshes and both hold from
    /// one stored `k`: the external pair sums to two, the internal pair matches.
    #[test]
    fn one_thickness_modification_satisfies_both_invariants() {
        for k in [0.9, 1.0, 1.15] {
            let stage = PlanetaryStage {
                thickness_mod: k,
                ..stage_of(24, 18, 60, 0.0)
            };
            let sun = stage
                .params(PlanetaryShaft::Sun, 24, 0.0, 1.0)
                .thickness_mod;
            let planet = stage
                .params(PlanetaryShaft::Carrier, 18, 0.0, 1.0)
                .thickness_mod;
            let ring = stage
                .params(PlanetaryShaft::Ring, 60, 0.0, 1.0)
                .thickness_mod;
            assert!(
                (sun + planet - 2.0).abs() < 1e-15,
                "external pair must sum to two"
            );
            assert!((planet - ring).abs() < 1e-15, "internal pair must match");
            // ...and it still solves.
            assert!(solve_planetary_stage(
                &stage,
                3000.0,
                StageTorques::just(2.0),
                &test_library()
            )
            .is_ok());
        }
    }
    /// **The set's shifts follow the same rule as a pair's**: off, the sun sits
    /// at its undercut minimum and the ring where it was put; on, the two are
    /// searched together and the set keeps more of its power.
    ///
    /// `η₀` is the thing maximised and the set efficiency is what has to rise,
    /// which is the claim that [`crate::planetary::power`] is monotone in `η₀`
    /// being checked rather than assumed.
    #[test]
    fn choosing_the_shifts_for_efficiency_leaves_the_set_more_of_its_power() {
        let lib = test_library();
        // Both free: a shift given by hand is a constraint, and a set with two
        // of them has nothing left to search.
        let free = || {
            let mut s = stage_of(24, 18, 60, 0.0);
            s.sun.profile_shift = Auto::automatic(0.0);
            s.ring.profile_shift = Auto::automatic(0.0);
            s
        };
        let solve = |on: bool| {
            solve_planetary_stage(
                &PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    ..free()
                },
                3000.0,
                StageTorques::just(2.0),
                &lib,
            )
            .expect("the set solves")
        };
        let plain = solve(false);
        let tuned = solve(true);

        assert!(
            (tuned.sun.profile_shift - plain.sun.profile_shift).abs() > 1e-6
                || (tuned.ring.profile_shift - plain.ring.profile_shift).abs() > 1e-6,
            "the search moved nothing: sun {} ring {}",
            tuned.sun.profile_shift,
            tuned.ring.profile_shift
        );
        assert!(
            tuned.fixed_carrier_efficiency.forward > plain.fixed_carrier_efficiency.forward,
            "eta0 {:.6} should beat {:.6}",
            tuned.fixed_carrier_efficiency.forward,
            plain.fixed_carrier_efficiency.forward
        );
        assert!(
            tuned.efficiency.forward > plain.efficiency.forward,
            "the set efficiency {:.6} should beat {:.6}, or power is not monotone in eta0",
            tuned.efficiency.forward,
            plain.efficiency.forward
        );
        // Both meshes stay continuous by at least the margin asked for.
        for eps in [
            tuned.sun_planet.contact_ratios.transverse,
            tuned.planet_ring.contact_ratios.transverse,
        ] {
            let asked = free().optimisation.min_contact_ratio;
            assert!(eps >= asked - 1e-3, "contact ratio {eps} under {asked}");
        }
    }

    /// A shift given by hand is a constraint the search may not overrule.
    #[test]
    fn a_given_shift_survives_the_search() {
        let mut stage = PlanetaryStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..stage_of(24, 18, 60, 0.0)
        };
        stage.sun.profile_shift = Auto::automatic(0.0);
        stage.ring.profile_shift = Auto::fixed(0.25);
        let r = solve_planetary_stage(&stage, 3000.0, StageTorques::just(2.0), &test_library())
            .expect("solves");
        assert!((r.ring.profile_shift - 0.25).abs() < 1e-9);
    }
}
