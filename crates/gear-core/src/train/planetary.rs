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
//! §4.9 warns that this is easy to get silently wrong, so both appear in the
//! result and the reversal is stated rather than folded into a number.
//!
//! **The tooth thickness invariants differ between the two meshes.** An external
//! pair needs `k₁ + k₂ = 2`; an internal pair needs `k₁ = k₂` (§4.11). A
//! planetary set has one of each, sharing the planet — so one stored `k` fixes
//! all three, and the invariant is unwritable rather than merely tested.
//!
//! # What is assumed, and said out loud
//!
//! **The planets are assumed to share the load equally.** Real sets do not
//! without a floating member or deliberate compliance; a mesh-load factor is the
//! usual remedy and it is a rating factor of exactly the kind §4.7 declines. So
//! the assumption is stated in the result's notes rather than absorbed into a
//! coefficient, and a designer who needs the derating can apply it knowingly.

use super::{Backlash, ContactRatios, GearResult, TrainError};
use crate::auto::{addendum_for_tip_width, admissible_ranges, automatic_profile_shift};
use crate::contact::{efficiency, ContactPath, Directional};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{Member as MeshMember, Mesh, MeshKind};
use crate::params::{Auto, GearParams};
use crate::planetary::{self, Arrangement, Member, Rack, Teeth};
use crate::profile::Gear;
use crate::ring::{Cutter, Ring};
use crate::strength::{
    bending_section, bending_stress, contact_stress, min_face_width_bending,
    min_face_width_contact, ring_bending_section, Load, StressConcentration, PARALLEL_AXES,
};
use crate::train::StageGear;

/// A planetary stage as its inputs describe it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PlanetaryStage {
    /// Normal module, mm. Shared by all three members.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// Helix angle, degrees. Shared; the internal pair takes the same hand and
    /// the external pair the opposite, which is what the meshes require.
    pub helix_angle: f64,
    /// Coefficient of friction, sun-to-planet.
    pub friction_sun_planet: f64,
    /// ...and planet-to-ring.
    pub friction_planet_ring: f64,
    /// `k` for the **sun**. The planet takes `2 − k` (external pair) and the ring
    /// takes the planet's (internal pair), so all three follow from this one.
    pub thickness_mod: f64,
    /// How many planets. One is legal; it just has no neighbour to clear.
    pub planets: u32,
    /// Which shaft drives and which is held.
    pub arrangement: Arrangement,
    /// Added to the common centre distance, mm — the running clearance.
    pub clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// Smallest acceptable gap between adjacent planets' tip circles, mm.
    pub min_planet_clearance: f64,
    /// The shaper the ring is cut with. A ring has no geometry without one.
    pub cutter: Cutter,
    /// The sun. Its profile shift is an input, automatic or manual.
    pub sun: StageGear,
    /// The planet. **Its profile shift is ignored**: the shift is what makes the
    /// two centre distances agree, so it is solved rather than chosen (§4.8).
    pub planet: StageGear,
    /// The ring. Its `dedendum` and `root_radius` are ignored — a ring's root
    /// circle is where its cutter reaches (§4.11) — and its shift is a manual
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
            friction_sun_planet: 0.06,
            friction_planet_ring: 0.06,
            thickness_mod: 1.0,
            planets: 3,
            arrangement: Arrangement {
                input: Member::Sun,
                fixed: Member::Ring,
            },
            clearance: 0.02,
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

/// What one of the two meshes did.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MeshReport {
    pub contact_ratios: ContactRatios,
    /// Mesh efficiency, both drive senses. Equal for a parallel-axis pair, and
    /// arrived at rather than copied.
    pub efficiency: Directional<f64>,
    /// Hertzian contact stress, MPa — shared by the pair.
    pub contact_stress: f64,
    /// Relative radius of curvature at the governing point, mm.
    pub relative_radius: f64,
    /// Angular backlash at each member, degrees: the first is the pinion-side
    /// member (sun, then planet), the second the other.
    pub backlash: [Backlash; 2],
}

/// The planet's own answers, which are not the shape of the other two.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PlanetResult {
    /// Everything a gear in a stage reports.
    pub gear: GearResult,
    /// The shift the common-centre-distance solve required.
    pub profile_shift: f64,
    /// `|a_sun-planet − a_planet-ring|` at that shift, mm. Reported rather than
    /// asserted: it is the one number that says the solve closed.
    pub shift_residual: f64,
    /// Speed in the fixed frame, rpm.
    pub speed_absolute: f64,
    /// Speed **relative to the carrier**, rpm — what its teeth actually see.
    pub speed_relative: f64,
    /// Bending is fully reversed: the sun drives one flank and the ring the
    /// other. Always true, and carried so a reader meets it beside the stress
    /// rather than in the documentation.
    pub fully_reversed: bool,
    /// The bending allowable a fully reversed load leaves, MPa, and where the
    /// figure comes from.
    pub reversed_allowable: crate::material::Value,
    /// Face width the reversed bending would need against that allowable.
    pub min_face_width_reversed: Option<f64>,
}

/// Everything a planetary stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct PlanetaryResult {
    pub arrangement: Arrangement,
    /// The shaft the other two leave over.
    pub output: Member,
    /// Speed reduction, input over output. Negative when the output reverses.
    pub ratio: f64,
    /// The common centre distance, zero-backlash, mm — sun-to-planet and
    /// planet-to-ring, which the planet's shift has made the same number.
    pub centre_distance_nominal: f64,
    /// ...and the one actually used, including clearance.
    pub centre_distance: f64,
    /// Fixed-carrier efficiency `η₀`, the product of the two mesh efficiencies.
    /// The quantity §4.5.2 requires, because the meshes slide at their speeds
    /// relative to the carrier rather than to ground.
    pub fixed_carrier_efficiency: Directional<f64>,
    /// Whole-set efficiency, driving forward and backward. Driving backward means
    /// the output shaft becomes the input with the same shaft held.
    pub efficiency: Directional<f64>,
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
    pub notes: Vec<String>,
}

impl PlanetaryStage {
    /// `GearParams` for one member, with the thickness invariants applied.
    ///
    /// The sun's `k` is the input; the planet takes `2 − k` because they mesh
    /// externally, and the ring takes the planet's because they mesh internally.
    /// One number, three consistent values, no assertion needed.
    fn params(&self, member: Member, teeth: u32, shift: f64, addendum: f64) -> GearParams {
        let (k, helix) = match member {
            Member::Sun => (self.thickness_mod, self.helix_angle),
            // The planet opposes the sun's hand, as an external pair must.
            Member::Carrier => (2.0 - self.thickness_mod, -self.helix_angle),
            // ...and the ring shares the planet's, as an internal pair must.
            Member::Ring => (2.0 - self.thickness_mod, -self.helix_angle),
        };
        GearParams {
            module: self.module,
            pressure_angle: self.pressure_angle,
            teeth,
            helix_angle: helix,
            profile_shift: shift,
            addendum,
            dedendum: match member {
                Member::Sun => self.sun.dedendum,
                Member::Carrier => self.planet.dedendum,
                Member::Ring => self.ring.dedendum,
            },
            root_radius: match member {
                Member::Sun => self.sun.root_radius,
                Member::Carrier => self.planet.root_radius,
                Member::Ring => self.ring.root_radius,
            },
            thickness_mod: k,
        }
    }

    fn rack(&self) -> Rack {
        Rack::new(self.module, self.pressure_angle, self.helix_angle)
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
    input_torque: f64,
    lib: &MaterialLibrary,
) -> Result<PlanetaryResult, TrainError> {
    let teeth = stage.teeth();
    let rack = stage.rack();
    let mut notes = Vec::new();

    // ---- shifts. The sun's and ring's are inputs; the planet's is solved.
    let sun_base = stage.params(Member::Sun, teeth.sun, 0.0, stage.sun.addendum.manual);
    let sun_shift = stage
        .sun
        .profile_shift
        .resolve(automatic_profile_shift(&sun_base, stage.sun.working_depth));
    let ring_shift = stage.ring.profile_shift.manual;

    // Thickness shifts, since only `x + x_s` reaches the answer (§4.1).
    let thickness = |shift: f64, member: Member| -> f64 {
        shift + stage.params(member, 1, shift, 1.0).thickness_shift()
    };
    let set = planetary::Set {
        rack,
        teeth,
        planets: stage.planets,
        sun_shift: thickness(sun_shift, Member::Sun),
        ring_shift: thickness(ring_shift, Member::Ring),
        // Filled once the planet exists; clearance only reads it.
        planet_tip_diameter: 0.0,
    };
    let layout = planetary::solve(&set).ok_or(TrainError::NoContact)?;
    // The solve works in thickness shifts, so take the thickness modification
    // back out to get the planet's profile shift proper.
    let planet_shift =
        layout.planet_shift - stage.params(Member::Carrier, 1, 0.0, 1.0).thickness_shift();

    // ---- the three members.
    let addendum_of = |member: Member, g: &StageGear, teeth: u32, shift: f64| -> f64 {
        let with_shift = stage.params(member, teeth, shift, g.addendum.manual);
        if g.addendum.auto {
            addendum_for_tip_width(&Gear::new(with_shift), g.min_tip_width)
                .unwrap_or(with_shift.addendum)
        } else {
            g.addendum.manual
        }
    };
    let sun_params = stage.params(
        Member::Sun,
        teeth.sun,
        sun_shift,
        addendum_of(Member::Sun, &stage.sun, teeth.sun, sun_shift),
    );
    let planet_params = stage.params(
        Member::Carrier,
        teeth.planet,
        planet_shift,
        addendum_of(Member::Carrier, &stage.planet, teeth.planet, planet_shift),
    );
    let ring_params = stage.params(
        Member::Ring,
        teeth.ring,
        ring_shift,
        stage.ring.addendum.manual,
    );

    let sun = Gear::new(sun_params);
    let planet = Gear::new(planet_params);
    let ring = Ring::new(&ring_params, &stage.cutter);
    // The mesh reads the ring through `Gear` arithmetic: a ring's shift enters
    // its space exactly as an external gear's enters its tooth (§4.11).
    let ring_as_gear = Gear::new(ring_params);

    // ---- the two meshes.
    let sp_mesh = Mesh::new(&sun, &planet, MeshKind::External).map_err(TrainError::Mesh)?;
    let sp_path = ContactPath::new(&sun, planet.ra, &sp_mesh).ok_or(TrainError::NoContact)?;
    let pr_mesh =
        Mesh::new(&planet, &ring_as_gear, MeshKind::Internal).map_err(TrainError::Mesh)?;
    let pr_path = ContactPath::new(&planet, ring.ra, &pr_mesh).ok_or(TrainError::NoContact)?;

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
    let sp_eff =
        Directional::of(|d| efficiency(&sp_path, &sp_mesh, &sun, stage.friction_sun_planet, d));
    let pr_eff =
        Directional::of(|d| efficiency(&pr_path, &pr_mesh, &planet, stage.friction_planet_ring, d));
    let eta0 = Directional::of(|d| sp_eff.get(d) * pr_eff.get(d));

    let forward = planetary::power(
        teeth,
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
        teeth,
        reversed,
        forward.speeds[out],
        forward.torques[out].abs() * if forward.speeds[out] < 0.0 { -1.0 } else { 1.0 },
        eta0.backward,
    );
    let set_efficiency = Directional {
        forward: forward.efficiency,
        backward: backward.map_or(0.0, |b| b.efficiency),
    };

    // ---- loads. Each mesh carries its member's torque divided among N planets.
    let planets = f64::from(stage.planets.max(1));
    let sun_torque_per_mesh = (forward.torques[0] / planets).abs();
    let ring_torque_per_mesh = (forward.torques[2] / planets).abs();

    // ---- face widths and stresses. `b_min` does not depend on the width it was
    // measured at (§4.7), so one probe evaluation gives every minimum.
    const PROBE: f64 = 10.0;
    let sp_e = contact_modulus(&mats[0], &mats[1]);
    let pr_e = contact_modulus(&mats[1], &mats[2]);

    let sp_probe = contact_stress(
        &sp_path,
        &sp_mesh,
        &sun,
        PARALLEL_AXES,
        &Load::new(sun_torque_per_mesh, PROBE),
        sp_e,
    )
    .ok_or(TrainError::NoContact)?;
    let pr_probe = contact_stress(
        &pr_path,
        &pr_mesh,
        &planet,
        PARALLEL_AXES,
        &Load::new(ring_torque_per_mesh / pr_mesh.ratio(), PROBE),
        pr_e,
    )
    .ok_or(TrainError::NoContact)?;

    let sun_section =
        bending_section(&sun, sp_path.contact_ratio).ok_or(TrainError::NoRootSection)?;
    let planet_section =
        bending_section(&planet, sp_path.contact_ratio).ok_or(TrainError::NoRootSection)?;
    let ring_section =
        ring_bending_section(&ring, pr_path.contact_ratio).ok_or(TrainError::NoRootSection)?;

    let width_for = |g: &StageGear, sigma_f: Option<f64>, sigma_h: f64, allow: f64| -> f64 {
        let mut want = 0.0_f64;
        if g.auto_face_from_bending {
            if let Some(sf) = sigma_f {
                want = want.max(min_face_width_bending(sf, PROBE, allow));
            }
        }
        if g.auto_face_from_contact {
            want = want.max(min_face_width_contact(sigma_h, PROBE, allow));
        }
        g.face_width.resolve(want)
    };

    let probe_load_sp = Load::new(sun_torque_per_mesh, PROBE);
    let sun_sf = bending_stress(
        &sun_section,
        &sun,
        &probe_load_sp,
        StressConcentration::Iso6336,
    );
    let planet_sf = bending_stress(
        &planet_section,
        &planet,
        &probe_load_sp.across_mesh(&sun, &planet),
        StressConcentration::Iso6336,
    );
    let probe_load_pr = Load::new(ring_torque_per_mesh / pr_mesh.ratio(), PROBE);
    let ring_sf = bending_stress(
        &ring_section,
        &planet,
        &probe_load_pr,
        StressConcentration::Iso6336,
    );

    let widths = [
        width_for(
            &stage.sun,
            sun_sf,
            sp_probe.worst,
            mats[0].fatigue_allowable.value,
        ),
        width_for(
            &stage.planet,
            planet_sf,
            sp_probe.worst.max(pr_probe.worst),
            crate::material::reversed_bending_allowable(&mats[1]).value,
        ),
        width_for(
            &stage.ring,
            ring_sf,
            pr_probe.worst,
            mats[2].fatigue_allowable.value,
        ),
    ];
    // The narrower member carries the mesh, per the specification.
    let sp_width = widths[0].min(widths[1]);
    let pr_width = widths[1].min(widths[2]);

    let sp_load = Load::new(sun_torque_per_mesh, sp_width);
    let pr_load = Load::new(ring_torque_per_mesh / pr_mesh.ratio(), pr_width);
    let sp_cs = contact_stress(&sp_path, &sp_mesh, &sun, PARALLEL_AXES, &sp_load, sp_e)
        .ok_or(TrainError::NoContact)?;
    let pr_cs = contact_stress(&pr_path, &pr_mesh, &planet, PARALLEL_AXES, &pr_load, pr_e)
        .ok_or(TrainError::NoContact)?;

    // ---- centre distance and backlash.
    let centre = layout.centre_distance + stage.clearance;
    let angular = |mesh: &Mesh, a: f64, at: MeshMember| -> f64 {
        mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees()
    };
    let backlash_of = |mesh: &Mesh, at: MeshMember| Backlash {
        nominal: angular(mesh, centre, at),
        minimum: angular(mesh, centre - stage.tolerance_minus, at),
        maximum: angular(mesh, centre + stage.tolerance_plus, at),
    };

    // ---- layout.
    let planet_tip = 2.0 * planet.ra;
    let clearance = (stage.planets > 1).then(|| {
        2.0 * layout.centre_distance * (std::f64::consts::PI / planets).sin() - planet_tip
    });

    // ---- what the answer does not include.
    notes.push(format!(
        "the {} planets are assumed to share the load equally; real sets do not \
         without a floating member, and the usual remedy is a mesh-load factor \
         this project declines to apply on the designer's behalf",
        stage.planets
    ));
    notes.push(
        "angular backlash is reported per mesh; the figure referred to the output \
         shaft needs a kinematic referral this stage does not yet derive"
            .to_string(),
    );
    if !layout.equal_spacing {
        notes.push(format!(
            "{} planets cannot be spaced evenly: (z_sun + z_ring) is not divisible by it",
            stage.planets
        ));
    }
    if let Some(gap) = clearance {
        if gap < stage.min_planet_clearance {
            notes.push(format!(
                "adjacent planets clear by {gap:.3} mm, below the {:.3} mm asked for",
                stage.min_planet_clearance
            ));
        }
    }
    if ring.clamps.iter().any(|c| c.contains("tip radius raised")) {
        notes.push(
            "the ring's addendum was clamped at its base circle: it needs more teeth \
             or a shorter addendum"
                .to_string(),
        );
    }

    let gear_result = |member: Member,
                       input: &StageGear,
                       params: &GearParams,
                       width: f64,
                       torque: f64,
                       sigma_f: Option<f64>,
                       sigma_h: f64,
                       material: &Material,
                       clamps: Vec<String>|
     -> GearResult {
        let allow = material.fatigue_allowable.value;
        GearResult {
            profile_shift: params.profile_shift,
            addendum: params.addendum,
            face_width: width,
            torque,
            speed: forward.speeds[member.index_pub()],
            tooth_cycles: 0.0,
            bending_stress: sigma_f,
            contact_stress: sigma_h,
            min_face_width_bending: sigma_f.map(|s| min_face_width_bending(s, width, allow)),
            min_face_width_contact: min_face_width_contact(sigma_h, width, allow),
            clamps,
            material: material.clone(),
            ranges: admissible_ranges(params, input.working_depth),
        }
    };

    let scale = |probe: Option<f64>, width: f64| probe.map(|s| s * PROBE / width);
    let sun_stress = scale(sun_sf, sp_width);
    let planet_stress = scale(planet_sf, sp_width);
    let ring_stress = scale(ring_sf, pr_width);

    let planet_relative = forward.speeds[0] - forward.speeds[1];
    let reversed_allowable = crate::material::reversed_bending_allowable(&mats[1]);

    Ok(PlanetaryResult {
        arrangement: stage.arrangement,
        output: forward.output,
        ratio: forward.ratio,
        centre_distance_nominal: layout.centre_distance,
        centre_distance: centre,
        fixed_carrier_efficiency: eta0,
        efficiency: set_efficiency,
        speeds: forward.speeds,
        torques: forward.torques,
        sun_planet: MeshReport {
            contact_ratios: ratios(sp_path.contact_ratio, sp_width, stage),
            efficiency: sp_eff,
            contact_stress: sp_cs.worst,
            relative_radius: sp_cs.relative_radius,
            backlash: [
                backlash_of(&sp_mesh, MeshMember::First),
                backlash_of(&sp_mesh, MeshMember::Second),
            ],
        },
        planet_ring: MeshReport {
            contact_ratios: ratios(pr_path.contact_ratio, pr_width, stage),
            efficiency: pr_eff,
            contact_stress: pr_cs.worst,
            relative_radius: pr_cs.relative_radius,
            backlash: [
                backlash_of(&pr_mesh, MeshMember::First),
                backlash_of(&pr_mesh, MeshMember::Second),
            ],
        },
        equal_spacing: layout.equal_spacing,
        simultaneous_meshing: layout.simultaneous_meshing,
        planet_clearance: clearance,
        planet_clearance_ok: clearance.is_none_or(|g| g >= stage.min_planet_clearance),
        sun_coprime_with_planets: gcd(teeth.sun, stage.planets.max(1)) == 1,
        ring_coprime_with_planets: gcd(teeth.ring, stage.planets.max(1)) == 1,
        sun: gear_result(
            Member::Sun,
            &stage.sun,
            &sun_params,
            widths[0],
            forward.torques[0] / planets,
            sun_stress,
            sp_cs.worst,
            &mats[0],
            sun.clamps.notes.clone(),
        ),
        planet: PlanetResult {
            gear: gear_result(
                Member::Carrier,
                &stage.planet,
                &planet_params,
                widths[1],
                sp_load.across_mesh(&sun, &planet).torque,
                planet_stress,
                sp_cs.worst.max(pr_cs.worst),
                &mats[1],
                planet.clamps.notes.clone(),
            ),
            profile_shift: planet_shift,
            shift_residual: layout.residual,
            speed_absolute: forward.speeds[1],
            speed_relative: planet_relative,
            fully_reversed: true,
            min_face_width_reversed: planet_stress
                .map(|s| min_face_width_bending(s, widths[1], reversed_allowable.value)),
            reversed_allowable,
        },
        planets: stage.planets,
        ring: gear_result(
            Member::Ring,
            &stage.ring,
            &ring_params,
            widths[2],
            forward.torques[2] / planets,
            ring_stress,
            pr_cs.worst,
            &mats[2],
            ring.clamps.clone(),
        ),
        notes,
    })
}

/// The three contact ratios for one mesh of the set.
fn ratios(transverse: f64, width: f64, stage: &PlanetaryStage) -> ContactRatios {
    let beta = stage.helix_angle.to_radians();
    let overlap = width * beta.sin().abs() / (std::f64::consts::PI * stage.module);
    ContactRatios {
        transverse,
        overlap,
        total: transverse + overlap,
    }
}

const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::train::test_library;

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
            2.0,
            &test_library(),
        )
        .unwrap()
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
        assert!(ideal.planet.profile_shift.abs() < 1e-12);
        assert!((ideal.centre_distance_nominal - 21.0).abs() < 1e-12);
    }

    /// The classical ratios, through the whole stage rather than the bare
    /// algebra — so a wiring error between them would show.
    #[test]
    fn the_stage_reports_the_classical_ratios() {
        let want = [
            (Member::Sun, Member::Ring, Member::Carrier, 3.5),
            (Member::Sun, Member::Carrier, Member::Ring, -2.5),
            (Member::Ring, Member::Sun, Member::Carrier, 1.4),
        ];
        for (input, fixed, output, ratio) in want {
            let stage = PlanetaryStage {
                arrangement: Arrangement { input, fixed },
                ..stage_of(24, 18, 60, 0.0)
            };
            let r = solve_planetary_stage(&stage, 3000.0, 2.0, &test_library()).unwrap();
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
                input: Member::Sun,
                fixed: Member::Carrier,
            },
            ..stage_of(24, 18, 60, 0.0)
        };
        let r = solve_planetary_stage(&stage, 3000.0, 2.0, &test_library()).unwrap();
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
                res.sun.bending_stress.expect("the sun is always rated"),
                res.ring.bending_stress.expect("and so is the ring"),
            );
            assert!(
                ring_s < sun_s,
                "z={s}/{p}/{r}: ring {ring_s} vs sun {sun_s}"
            );
        }
    }

    /// The planet is the special case of §4.9: fully reversed, judged against a
    /// smaller allowable that says where it came from, and turning at a speed
    /// measured **relative to the carrier**.
    #[test]
    fn the_planet_is_reported_as_the_special_case_it_is() {
        let r = solved(24, 18, 60);
        assert!(r.planet.fully_reversed);
        let allow = &r.planet.reversed_allowable;
        assert_eq!(allow.basis, crate::material::Basis::Derived);
        assert!(allow.note.is_some(), "a derived value must say what from");
        assert!(
            allow.value < r.planet.gear.material.fatigue_allowable.value,
            "a reversed allowable must be the smaller"
        );
        // Its speed relative to the carrier is not its speed in the fixed frame,
        // and with the ring held neither is zero.
        assert!(r.planet.speed_relative.abs() > 0.0);
        assert!((r.planet.speed_relative - r.planet.speed_absolute).abs() > 1e-9);
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
        let r = solve_planetary_stage(&one, 3000.0, 2.0, &test_library()).unwrap();
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
            let r = solve_planetary_stage(&stage, 3000.0, 2.0, &test_library())
                .unwrap_or_else(|e| panic!("helix={helix}: {e}"));
            assert!(r.sun.bending_stress.is_some(), "helix={helix}: sun");
            assert!(
                r.planet.gear.bending_stress.is_some(),
                "helix={helix}: planet"
            );
            assert!(r.ring.bending_stress.is_some(), "helix={helix}: ring");
            assert!(r.sun_planet.contact_ratios.overlap > 0.0, "helix={helix}");
            assert!(r.planet.shift_residual < 1e-12);
        }
    }

    /// Tooth counts that admit no planet shift are refused, not fudged into an
    /// answer. Most combinations are impossible (§4.8) and that is the common
    /// case rather than an exceptional one.
    #[test]
    fn an_impossible_set_is_refused() {
        assert!(
            solve_planetary_stage(&stage_of(24, 18, 200, 0.0), 3000.0, 2.0, &test_library())
                .is_err()
        );
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
            let sun = stage.params(Member::Sun, 24, 0.0, 1.0).thickness_mod;
            let planet = stage.params(Member::Carrier, 18, 0.0, 1.0).thickness_mod;
            let ring = stage.params(Member::Ring, 60, 0.0, 1.0).thickness_mod;
            assert!(
                (sun + planet - 2.0).abs() < 1e-15,
                "external pair must sum to two"
            );
            assert!((planet - ring).abs() < 1e-15, "internal pair must match");
            // ...and it still solves.
            assert!(solve_planetary_stage(&stage, 3000.0, 2.0, &test_library()).is_ok());
        }
    }
}
