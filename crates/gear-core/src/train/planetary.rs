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
    allowable, Backlash, Case, ContactRatios, Cycles, GearResult, LoadCase, StageTorques,
    TrainError, Widths,
};
use crate::auto::{addendum_for_tip_width, admissible_ranges, automatic_profile_shift};
use crate::contact::{efficiency, ContactPath, Directional};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::planetary::{self, Arrangement, PlanetaryShaft, Rack, Teeth};
use crate::ring::{Cutter, Ring};
use crate::strength::{
    bending_section, bending_stress, contact_stress, min_face_width_bending,
    min_face_width_contact, ring_bending_section, Load, StressConcentration, PARALLEL_AXES,
};
use crate::tooth::Tooth;
use crate::train::StageGear;

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
    /// Whether a drive turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`] — the static figure's
    /// only job is the sign, and it is never itself reported as an efficiency.
    pub static_friction_sun_planet: f64,
    /// ...and planet-to-ring.
    pub sliding_friction_planet_ring: f64,
    /// Coefficient of **static** friction, for breaking away.
    ///
    /// Whether a drive turns at all is decided at rest and against this; how
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
            sliding_friction_sun_planet: 0.06,
            static_friction_sun_planet: 0.16,
            sliding_friction_planet_ring: 0.06,
            static_friction_planet_ring: 0.16,
            thickness_mod: 1.0,
            planets: 3,
            arrangement: Arrangement {
                input: PlanetaryShaft::Sun,
                fixed: PlanetaryShaft::Ring,
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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MeshReport {
    pub contact_ratios: ContactRatios,
    /// Mesh efficiency, both drive senses. Equal for a parallel-axis pair, and
    /// arrived at rather than copied.
    pub efficiency: Directional<f64>,
    /// Hertzian contact stress at the pitch point, MPa, in both load cases.
    ///
    /// The one figure both members of the mesh share. Each member's own rating —
    /// taken where its dedendum is loaded alone — sits on its `GearResult`.
    pub contact_stress_at_pitch_point: LoadCase<f64>,
    /// Relative radius of curvature at the governing point, mm.
    pub relative_radius: f64,
    /// Angular backlash at each member, degrees: the first is the pinion-side
    /// member (sun, then planet), the second the other.
    pub backlash: [Backlash; 2],
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
    ///
    /// This is the allowable [`Self::gear`]'s **cyclic** bending is rated
    /// against — there is no second minimum face width to report beside it, and
    /// there used to be, which was one derate written down in two places.
    pub reversed_allowable: crate::material::Value,
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
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<PlanetaryResult, TrainError> {
    let input_torque = torques.peak_forward;
    let teeth = stage.teeth();
    let rack = stage.rack();
    let mut notes = Vec::new();

    // ---- shifts. The sun's and ring's are inputs; the planet's is solved.
    let sun_base = stage.params(
        PlanetaryShaft::Sun,
        teeth.sun,
        0.0,
        stage.sun.addendum.manual,
    );
    let sun_shift = stage.sun.profile_shift.resolve(automatic_profile_shift(
        &sun_base,
        stage.sun.working_depth.resolve(stage.sun.dedendum),
    ));
    let ring_shift = stage.ring.profile_shift.manual;

    // Thickness shifts, since only `x + x_s` reaches the answer (docs/reference.md#tooth-thickness-and-its-equivalent-shift).
    let thickness = |shift: f64, member: PlanetaryShaft| -> f64 {
        shift + stage.params(member, 1, shift, 1.0).thickness_shift()
    };
    let set = planetary::Set {
        rack,
        teeth,
        planets: stage.planets,
        sun_shift: thickness(sun_shift, PlanetaryShaft::Sun),
        ring_shift: thickness(ring_shift, PlanetaryShaft::Ring),
        // Filled once the planet exists; clearance only reads it.
        planet_tip_diameter: 0.0,
    };
    let layout = planetary::solve(&set).ok_or(TrainError::NoContact)?;
    // The solve works in thickness shifts, so take the thickness modification
    // back out to get the planet's profile shift proper.
    let planet_shift = layout.planet_shift
        - stage
            .params(PlanetaryShaft::Carrier, 1, 0.0, 1.0)
            .thickness_shift();

    // ---- the three members.
    let addendum_of = |member: PlanetaryShaft, g: &StageGear, teeth: u32, shift: f64| -> f64 {
        let with_shift = stage.params(member, teeth, shift, g.addendum.manual);
        if g.addendum.auto {
            addendum_for_tip_width(&Tooth::new(with_shift), g.min_tip_width)
                .unwrap_or(with_shift.addendum)
        } else {
            g.addendum.manual
        }
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
        stage.ring.addendum.manual,
    );

    let sun = Tooth::new(sun_params);
    let planet = Tooth::new(planet_params);
    let ring = Ring::cut_by(&ring_params, &stage.cutter);
    // The mesh reads the ring through `Tooth` arithmetic: a ring's shift enters
    // its space exactly as an external gear's enters its tooth (docs/reference.md#internal-gears).
    let ring_as_gear = Tooth::new(ring_params);

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
    // The same two solves on the static basic efficiency, for the sign only.
    // A set is a train of parallel meshes and never sits near its threshold, so
    // this confirms rather than decides — but it is the rule every stage kind
    // obeys, and a stage that skipped it would be the one place a reader has to
    // remember why.
    let at_rest = Directional {
        forward: planetary::power(
            teeth,
            stage.arrangement,
            input_speed,
            input_torque,
            eta0_at_rest.forward,
        )
        .map_or(0.0, |p| p.efficiency),
        backward: planetary::power(
            teeth,
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
    let sun_torque_per_mesh = (forward.torques[0] / planets).abs();
    let ring_torque_per_mesh = (forward.torques[2] / planets).abs();

    // ---- face widths and stresses. `b_min` does not depend on the width it was
    // measured at (docs/reference.md#contact-stress), so one probe evaluation gives every minimum.
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

    // Every rating is linear or square-root in the torque, and the planetary's
    // power split is **not** a function of its magnitude: `w = sgn(T_s(ω_s − ω_c))`
    // and back-driving reverses both factors, so the branch is the same one. A
    // load case is therefore a scale on the forward torques rather than a second
    // kinematic solve.
    let scale_case = |case: Case| {
        if torques.peak_forward == 0.0 {
            0.0
        } else {
            torques.at(case) / torques.peak_forward.abs()
        }
    };

    // An automatic width with every source switched off has nothing to invert
    // and comes out zero. Said rather than divided by, as in the spur stage.
    let mut no_source = Vec::new();
    let mut ask_of = |name: &str, g: &StageGear, asks: &LoadCase<Widths>| -> f64 {
        if g.face_width.auto && !g.face_sources.any() {
            no_source
                .push(Note::new(key::STAGE_FACE_WIDTH_NO_SOURCE).text("gear", name.to_string()));
        }
        g.face_sources.largest_of(asks)
    };

    /// The four widths one member's ratings ask for.
    fn asks_of(
        sigma_f: Option<f64>,
        sigma_h: f64,
        probe: f64,
        allow: &dyn Fn(Case) -> f64,
        scale: &dyn Fn(Case) -> f64,
    ) -> LoadCase<Widths> {
        LoadCase::of(|case| {
            let k = scale(case);
            let a = allow(case);
            Widths {
                bending: sigma_f.map(|s| min_face_width_bending(s * k, probe, a)),
                contact: min_face_width_contact(sigma_h * k.sqrt(), probe, a),
            }
        })
    }

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

    // The planet's bending allowable is already the fully reversed one: sun and
    // ring load opposite flanks of the same tooth, whatever the drive does. A
    // reversing *drive* does not make it more reversed, so the two do not stack
    // — the derate stands and the contact halving is applied to the cycles.
    let planet_allow = |case: Case| match case {
        Case::Cyclic => crate::material::reversed_bending_allowable(&mats[1]).value,
        Case::Peak => mats[1].ultimate_allowable.value,
    };
    let asks = [
        ask_of(
            "sun",
            &stage.sun,
            &asks_of(
                sun_sf,
                sp_probe.governing(0),
                PROBE,
                &|c| allowable(&mats[0], c),
                &scale_case,
            ),
        ),
        ask_of(
            "planet",
            &stage.planet,
            &asks_of(
                planet_sf,
                sp_probe.governing(1).max(pr_probe.governing(0)),
                PROBE,
                &planet_allow,
                &scale_case,
            ),
        ),
        ask_of(
            "ring",
            &stage.ring,
            &asks_of(
                ring_sf,
                pr_probe.governing(1),
                PROBE,
                &|c| allowable(&mats[2], c),
                &scale_case,
            ),
        ),
    ];
    notes.extend(no_source);
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

    let sp_load = Load::new(sun_torque_per_mesh, sp_width);
    let pr_load = Load::new(ring_torque_per_mesh / pr_mesh.ratio(), pr_width);
    let sp_cs = contact_stress(&sp_path, &sp_mesh, &sun, PARALLEL_AXES, &sp_load, sp_e)
        .ok_or(TrainError::NoContact)?;
    let pr_cs = contact_stress(&pr_path, &pr_mesh, &planet, PARALLEL_AXES, &pr_load, pr_e)
        .ok_or(TrainError::NoContact)?;

    // ---- centre distance and backlash.
    let centre = layout.centre_distance + stage.clearance;
    let angular = |mesh: &Mesh, a: f64, at: MeshSide| -> f64 {
        mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees()
    };
    let backlash_of = |mesh: &Mesh, at: MeshSide| Backlash {
        nominal: angular(mesh, centre, at),
        minimum: angular(mesh, centre - stage.tolerance_minus, at),
        maximum: angular(mesh, centre + stage.tolerance_plus, at),
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

    let gear_result = |member: PlanetaryShaft,
                       input: &StageGear,
                       params: &GearParams,
                       width: f64,
                       torque: f64,
                       sigma_f: Option<f64>,
                       sigma_h: f64,
                       material: &Material,
                       allow: &dyn Fn(Case) -> f64,
                       clamps: Vec<Note>|
     -> GearResult {
        // A load case is a scale on the torque, and every rating is linear or
        // square-root in it — so the peak and cyclic figures are the same
        // expression evaluated at the two scales rather than a second solve.
        //
        // **The allowable comes in rather than being read off the material**, so
        // that the width the member was given and the minimum reported here are
        // sized against the same figure. The planet is the member this matters
        // for: its bending is fully reversed whatever the drive does, so it is
        // rated against a derated allowable, and reading the plain one here
        // would have reported a minimum the stage had not used.
        let by_case = |case: Case| (scale_case(case), allow(case));
        GearResult {
            profile_shift: params.profile_shift,
            addendum: params.addendum,
            face_width: width,
            torque,
            back_driving_torque: torques
                .peak_backward
                .map(|t| torque * (t.abs() / torques.peak_forward.abs().max(f64::MIN_POSITIVE))),
            speed: forward.speeds[member.index_pub()],
            tooth_cycles: Cycles {
                bending: 0.0,
                contact: 0.0,
            },
            bending_stress: LoadCase::of(|c| sigma_f.map(|s| s * by_case(c).0)),
            contact_stress: LoadCase::of(|c| sigma_h * by_case(c).0.sqrt()),
            min_face_width: LoadCase::of(|c| {
                let (k, a) = by_case(c);
                Widths {
                    bending: sigma_f.map(|s| min_face_width_bending(s * k, width, a)),
                    contact: min_face_width_contact(sigma_h * k.sqrt(), width, a),
                }
            }),
            clamps,
            material: material.clone(),
            ranges: admissible_ranges(params, input.working_depth.resolve(input.dedendum)),
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
        backlash: set_backlash,
        speeds: forward.speeds,
        torques: forward.torques,
        sun_planet: MeshReport {
            contact_ratios: ratios(sp_path.contact_ratio, sp_width, stage),
            efficiency: sp_eff,
            contact_stress_at_pitch_point: LoadCase::of(|c| {
                sp_cs.at_pitch_point * scale_case(c).sqrt()
            }),
            relative_radius: sp_cs.relative_radius,
            backlash: [
                backlash_of(&sp_mesh, MeshSide::First),
                backlash_of(&sp_mesh, MeshSide::Second),
            ],
        },
        planet_ring: MeshReport {
            contact_ratios: ratios(pr_path.contact_ratio, pr_width, stage),
            efficiency: pr_eff,
            contact_stress_at_pitch_point: LoadCase::of(|c| {
                pr_cs.at_pitch_point * scale_case(c).sqrt()
            }),
            relative_radius: pr_cs.relative_radius,
            backlash: [
                backlash_of(&pr_mesh, MeshSide::First),
                backlash_of(&pr_mesh, MeshSide::Second),
            ],
        },
        equal_spacing: layout.equal_spacing,
        simultaneous_meshing: layout.simultaneous_meshing,
        planet_clearance: clearance,
        planet_clearance_ok: clearance.is_none_or(|g| g >= stage.min_planet_clearance),
        sun_coprime_with_planets: gcd(teeth.sun, stage.planets.max(1)) == 1,
        ring_coprime_with_planets: gcd(teeth.ring, stage.planets.max(1)) == 1,
        sun: gear_result(
            PlanetaryShaft::Sun,
            &stage.sun,
            &sun_params,
            widths[0],
            forward.torques[0] / planets,
            sun_stress,
            sp_cs.governing(0),
            &mats[0],
            &|c| allowable(&mats[0], c),
            sun.clamps.notes.clone(),
        ),
        planet: PlanetResult {
            gear: gear_result(
                PlanetaryShaft::Carrier,
                &stage.planet,
                &planet_params,
                widths[1],
                sp_load.across_mesh(&sun, &planet).torque,
                planet_stress,
                // The planet is member 2 of the sun mesh and member 1 of the
                // ring mesh, so its own root is loaded alone at a different end
                // of each path. It takes the worse of its two.
                sp_cs.governing(1).max(pr_cs.governing(0)),
                &mats[1],
                &planet_allow,
                planet.clamps.notes.clone(),
            ),
            profile_shift: planet_shift,
            shift_residual: layout.residual,
            speed_absolute: forward.speeds[1],
            speed_relative: planet_relative,
            fully_reversed: true,
            reversed_allowable,
        },
        planets: stage.planets,
        ring: gear_result(
            PlanetaryShaft::Ring,
            &stage.ring,
            &ring_params,
            widths[2],
            forward.torques[2] / planets,
            ring_stress,
            pr_cs.governing(1),
            &mats[2],
            &|c| allowable(&mats[2], c),
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
            StageTorques::just(2.0),
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
            clearance: base.clearance + 0.05,
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
            clearance: 0.0,
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

    /// The planet is the special case of docs/reference.md#trains: fully reversed, judged against a
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
}
