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
use crate::contact::{efficiency, ContactPath, Directional, Drive, LoadSharing};
use crate::material::{contact_modulus, Material, MaterialLibrary, Overrides};
use crate::mesh::{Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::strength::{
    bending_section_shared, bending_stress, contact_stress, min_face_width_bending,
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
    /// The shift, and who decides it: **automatic means the stage does**, not
    /// that undercut does. What it resolves to when nothing else constrains it
    /// is [`StageGear::no_undercut`]'s business.
    pub profile_shift: Auto<f64>,
    /// **The shift may not go below the least that clears undercut.**
    ///
    /// A constraint rather than a source, which is what lets it combine with
    /// everything else: it bounds a shift a designer typed, a shift the stage
    /// solved from a centre distance or a crank offset, and a shift the
    /// efficiency search chose, all in the same words.
    ///
    /// The bound is the **true** minimum from [`minimum_profile_shift`], which
    /// on a comfortable tooth count is negative — so a deliberate negative
    /// shift is left alone and only a genuinely undercut one is raised. That is
    /// deliberate: negative shift is a decision about centre distance or
    /// balance, and this is a question about undercut. Where the *stage* is
    /// choosing and nothing else decides, the answer is instead
    /// [`automatic_profile_shift`] — the same bound taken no lower than zero,
    /// because a shift chosen for no reason should not thin a tooth that needed
    /// no help.
    ///
    /// Off, the gear may undercut, and the searches stop asking
    /// ([`crate::auto::member_is_buildable`]).
    ///
    /// **Meaningless on a ring**, whose flank is its shaper's rather than a
    /// rack's, and which is never asked — see `member_is_buildable`.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub no_undercut: bool,
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

/// A shift clears undercut unless it is told not to — and a document written
/// before the question was asked separately meant exactly that.
#[cfg(feature = "serde")]
const fn yes() -> bool {
    true
}

/// **What a gear's two shift controls come to**, read once so every stage reads
/// them the same way.
///
/// The two are different kinds of thing and this is where that is written down:
/// `profile_shift.auto` says *who decides*, `no_undercut` says *what the answer
/// must satisfy however it is decided*. Every stage here needs all three of the
/// values below — the bound for its search, what it may not overrule, and what
/// to report when there is nothing to search — and each had been working them
/// out for itself.
///
/// # The bound is not one number, and that is not an inconsistency
///
/// Undercut asks one question, but *choosing* a shift and *checking* a given
/// one want different answers to it, and [`automatic_profile_shift`] says why:
/// the true minimum is negative on any comfortable tooth count, so applying it
/// literally would thin a tooth that needed no help, for nothing.
///
/// So a search is floored at `max(x_min, 0)` — shift when the geometry demands
/// it, otherwise leave the tooth alone — while a number a designer typed is
/// held only to `x_min` itself. A deliberate −0.3 on a 43-tooth gear is a
/// decision about centre distance or balance, not a mistake about undercut, and
/// survives; a −2.0 there genuinely undercuts and is raised.
///
/// Getting this wrong is measurable rather than a matter of taste: flooring the
/// *search* at the true minimum let the eccentric drive's split walk out to
/// −1.79 and come back with **less** drive efficiency than it started with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShiftAsked {
    /// What a search choosing this shift may not go below — `max(x_min, 0)`,
    /// see above — or `None` where there is no such bound to apply.
    ///
    /// `None` covers **two** cases and they are the same statement: the gear
    /// may undercut, or the gear is not being chosen at all. A shift a designer
    /// gave was already held to its own bound when it was read, and a search
    /// that re-judges it against the *chooser's* bound rejects designs that are
    /// perfectly legal — a 43-tooth wheel pinned at −0.5 is nowhere near
    /// undercut and sits a whole module below `max(x_min, 0)`, so every
    /// candidate built on it was thrown away and the optimiser fell back to
    /// doing nothing, silently. A pinned gear is a constraint on the search,
    /// not a candidate of it.
    pub search_floor: Option<f64>,
    /// The shift a designer gave, which no search may overrule. `None` where it
    /// was left automatic — and a given shift arrives already raised to its
    /// floor, so a pinned value and a reported one cannot disagree.
    pub given: Option<f64>,
    /// What the gear settles at when nothing else decides: the given value, or
    /// [`automatic_profile_shift`] where the bound is on, or zero where it is
    /// off and nothing is asked of the shift at all.
    pub settled: f64,
    /// Whether a given shift had to be raised to reach its floor, and so
    /// whether the number in hand is the number that was typed.
    pub raised: bool,
}

impl ShiftAsked {
    /// **The note a raised shift owes its reader.**
    ///
    /// `no undercut` bounds a shift a designer typed as well as one the stage
    /// chose, which is what lets it mean one thing everywhere — but a number
    /// that was not taken as given has to say so, or the field and the gear
    /// disagree in silence. The gear is named by its tooth count, which every
    /// stage kind has and none has to invent a scheme for.
    pub(crate) fn note(&self, teeth: u32) -> Option<crate::note::Note> {
        self.raised.then(|| {
            crate::note::Note::new(crate::note::key::STAGE_SHIFT_RAISED_FOR_UNDERCUT)
                .count("teeth", teeth)
                .number("shift", self.settled, 4)
        })
    }
}

impl StageGear {
    /// [`ShiftAsked`], for this gear at its own working depth.
    pub(crate) fn shift_asked(&self, base: &crate::params::GearParams) -> ShiftAsked {
        let depth = self.working_depth.resolve(self.dedendum);
        if !self.no_undercut {
            // Nothing asked of the shift. Given, it is taken as typed; left to
            // the stage with no objective either, there is no reason to move
            // the tooth at all.
            let given = (!self.profile_shift.auto).then_some(self.profile_shift.manual);
            return ShiftAsked {
                search_floor: None,
                given,
                settled: given.unwrap_or(0.0),
                raised: false,
            };
        }
        if self.profile_shift.auto {
            let floor = automatic_profile_shift(base, depth);
            return ShiftAsked {
                search_floor: Some(floor),
                given: None,
                settled: floor,
                raised: false,
            };
        }
        // Given, and held to the true minimum rather than to the search's — a
        // negative shift somebody meant is not an undercut one. Nothing is
        // choosing it, so it carries no search bound.
        let typed = self.profile_shift.manual;
        let used = typed.max(crate::auto::minimum_profile_shift(base, depth).with_cutter_radius);
        ShiftAsked {
            search_floor: None,
            given: Some(used),
            settled: used,
            raised: used > typed,
        }
    }
}

impl Default for StageGear {
    fn default() -> Self {
        Self {
            teeth: 17,
            profile_shift: Auto::automatic(0.0),
            no_undercut: true,
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
    /// Added to the centre distance, mm — the assembly clearance, and so the
    /// backlash.
    ///
    /// Read only where something is free to absorb it; see
    /// [`Self::clearance_taken`], which is the whole of that rule.
    pub clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// What the stage is asked to optimise, and what it may not do to get
    /// there. See [`super::Optimisation`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: super::Optimisation,
    /// How the load is divided while two tooth pairs are engaged.
    ///
    /// **Off by default, and it reaches bending only.** A contact rating is
    /// already taken where one tooth carries everything, so sharing cannot move
    /// it; a bending rating's worst point is a product of the form factor and
    /// the share, so it can. See
    /// [`bending_section_shared`](crate::strength::bending_section_shared).
    ///
    /// The model behind it is an explicitly uncalibrated placeholder rather
    /// than a stiffness calculation, which is why it is offered as a choice a
    /// designer makes rather than applied on their behalf — the same treatment
    /// every other estimate in this crate gets.
    #[cfg_attr(feature = "serde", serde(default))]
    pub load_sharing: LoadSharing,
    pub gears: [StageGear; 2],
}

impl Default for SpurStage {
    fn default() -> Self {
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            shaft_angle: 0.0,
            additional_helix: 0.0,
            sliding_friction: 0.08,
            static_friction: 0.16,
            thickness_mod: 1.0,
            optimisation: super::Optimisation::default(),
            centre_distance: Auto::automatic(0.0),
            clearance: 0.02,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            load_sharing: LoadSharing::None,
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

    /// **The clearance this stage actually opens by**, which is the one it was
    /// given wherever anything is free to absorb it.
    ///
    /// With the centre distance automatic, the distance itself absorbs it: it is
    /// the zero-backlash distance opened out, and that opening *is* the
    /// backlash. With the distance given by hand there is nothing left to move
    /// — the shifts sit at their undercut minimum, the distance is whatever was
    /// typed, and the backlash is a consequence rather than a choice — so the
    /// clearance is not read at all.
    ///
    /// Unless the shifts are being chosen, in which case they are what absorbs
    /// it: the sum is pinned so the pair closes to zero backlash a clearance
    /// *inside* the given distance, and the designer gets both the housing they
    /// specified and the play they asked for.
    ///
    /// One rule, in one place, and [`SpurResult::clearance`] reports what came
    /// of it — so the panel greys the input out by reading the answer rather
    /// than by knowing the rule a second time.
    #[must_use]
    pub fn clearance_taken(&self) -> f64 {
        if self.centre_distance.auto || self.optimisation.enabled {
            self.clearance
        } else {
            0.0
        }
    }

    /// The two profile shifts, chosen together where that is what the stage
    /// asked for.
    ///
    /// With the toggle off each gear answers on its own, as it always has: the
    /// manual value, or the least shift that clears undercut. With it on the
    /// pair is chosen at once, because a shift is only good or bad relative to
    /// the one it meshes with — and what a designer has already given is handed
    /// over as pinned rather than overridden.
    /// What the shift controls have to say for themselves — empty unless one
    /// of them raised a given value.
    pub(super) fn shift_notes(&self) -> Vec<Note> {
        (0..2)
            .filter_map(|i| {
                self.gears[i]
                    .shift_asked(&self.base_params(i))
                    .note(self.gears[i].teeth)
            })
            .collect()
    }

    pub(super) fn shifts(&self) -> [f64; 2] {
        let asked = [0, 1].map(|i| self.gears[i].shift_asked(&self.base_params(i)));
        let floor = asked.map(|a| a.search_floor);
        let given = asked.map(|a| a.given);
        if !self.optimisation.enabled {
            return asked.map(|a| a.settled);
        }
        let sum = (!self.centre_distance.auto)
            .then(|| {
                let rack = crate::plane::BasicRack::new(
                    self.module,
                    self.pressure_angle,
                    self.helix_angles()[0].abs(),
                );
                let sum_z = f64::from(self.gears[0].teeth) + f64::from(self.gears[1].teeth);
                crate::mesh::shift_sum_for(
                    rack.mt,
                    rack.alpha_t,
                    rack.alpha_n,
                    sum_z,
                    self.centre_distance.manual - self.clearance_taken(),
                )
            })
            .flatten();
        crate::auto::shifts_for_efficiency(
            &|x| [0, 1].map(|i| self.params_at(i, x[i])),
            crate::mesh::MeshKind::External,
            &crate::auto::Bounds {
                floor,
                min_contact_ratio: self.optimisation.min_contact_ratio,
                clearance: self.clearance_taken(),
            },
            &crate::auto::Pinned { shift: given, sum },
            self.sliding_friction,
        )
        .unwrap_or_else(|| asked.map(|a| a.settled))
    }

    /// The gear the stage would build at a given shift — the automatic
    /// addendum resolved, because a shift changes the tip width and so the
    /// tooth that shift produces.
    ///
    /// [`Self::params`] is this at the shift the stage settled on, and the
    /// optimiser searches over it, so the geometry that is rated is the
    /// geometry that is built.
    pub(super) fn params_at(&self, i: usize, x: f64) -> GearParams {
        let g = &self.gears[i];
        let with_shift = GearParams {
            profile_shift: x,
            ..self.base_params(i)
        };
        GearParams {
            addendum: if g.addendum.auto {
                addendum_for_tip_width(&Tooth::new(with_shift), g.min_tip_width)
                    .unwrap_or(with_shift.addendum)
            } else {
                g.addendum.manual
            },
            ..with_shift
        }
    }

    /// `GearParams` for one gear, before any automatic value is resolved.
    pub(super) fn base_params(&self, i: usize) -> GearParams {
        let g = &self.gears[i];
        GearParams {
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
    solve_spur_stage_with(stage, torques, lib, super::Reversal::default())
}

/// The same, told how the train treats a root loaded on both flanks.
///
/// A parallel-axis gear reverses only when the **drive** does, so with an
/// ordinary one-way drive this is the call above and nothing moves.
///
/// # Errors
///
/// As [`solve_spur_stage`].
pub fn solve_spur_stage_with(
    stage: &SpurStage,
    torques: StageTorques,
    lib: &MaterialLibrary,
    reversal: super::Reversal,
) -> Result<SpurResult, TrainError> {
    // The shifts once, not once per gear: with the optimiser on, `shifts` is a
    // search, and asking each gear for its own would run it twice for one
    // answer.
    let x = stage.shifts();
    let p = [stage.params_at(0, x[0]), stage.params_at(1, x[1])];
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

    // --- centre distance and the clearance it is opened by.
    let clearance = stage.clearance_taken();
    let centre = if stage.centre_distance.auto {
        mesh.a_w + clearance
    } else {
        stage.centre_distance.manual
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

    // --- face width. `b_min` does not depend on the `b` it was measured at
    // (docs/reference.md#contact-stress), so one evaluation at any width gives every minimum, and
    // nothing has to be iterated.
    const PROBE: f64 = 10.0;
    let e_star = contact_modulus(&materials[0], &materials[1]);

    // The critical section, and the share of the load acting on it. With
    // sharing off — the default — this *is* `bending_section` and a share of
    // exactly 1, so the ordinary rating is untouched to the bit.
    let shared =
        [0usize, 1].map(|i| bending_section_shared(&g[i], path.contact_ratio, stage.load_sharing));
    let sections = [
        shared[0].ok_or(TrainError::NoRootSection)?.0,
        shared[1].ok_or(TrainError::NoRootSection)?.0,
    ];
    let load_share = [
        shared[0].ok_or(TrainError::NoRootSection)?.1,
        shared[1].ok_or(TrainError::NoRootSection)?.1,
    ];

    // Every rating at a probe width, one set per load case. `b_min` does not
    // depend on the `b` it was measured at, so this is still one evaluation per
    // case and nothing iterates.
    let probe = |case| -> Result<(crate::strength::ContactStress, [Option<f64>; 2]), TrainError> {
        let load = Load::new(torques.at(case), PROBE);
        let cs = contact_stress(&path, &operating, &g[0], PARALLEL_AXES, &load, e_star)
            .ok_or(TrainError::NoContact)?;
        let sf = [0usize, 1].map(|i| {
            let li = load.across_mesh(&g[0], &g[i]);
            // The share this tooth carries where it is rated — exactly 1 unless
            // a sharing model was asked for, so nothing scales by default.
            bending_stress(&sections[i], &g[i], &li, StressConcentration::Iso6336)
                .map(|s| s * load_share[i])
        });
        Ok((cs, sf))
    };
    let probed = LoadCase {
        peak: probe(Case::Peak)?,
        cyclic: probe(Case::Cyclic)?,
    };

    // A parallel-axis gear's root is loaded both ways only when the drive
    // reverses; nothing about the pair itself reverses it.
    let reverses = reversal.reverses(false);
    let probe_widths = |i: usize| -> LoadCase<Widths> {
        LoadCase::of(|case| {
            let (cs, sf) = probed.get(case);
            Widths {
                bending: sf[i].map(|s| {
                    let allow = reversal.bending_allowable(&materials[i], case, reverses);
                    min_face_width_bending(s, PROBE, allow)
                }),
                // **This gear's** governing point, not the pair's envelope: the
                // width a gear needs follows from the stress it is rated at. And
                // the material's own allowable whatever the drive does: pitting
                // is compressive on whichever flank carries it.
                contact: min_face_width_contact(
                    cs.governing(i),
                    PROBE,
                    allowable(&materials[i], case),
                ),
            }
        })
    };

    let mut notes = stage.shift_notes();
    // The `Y_S` fit is stated over a band, and a section outside it is reported
    // rather than silently taking the boundary value — see `notch_outside_fit`.
    // What the rating has to say about each gear. Per gear, because that is
    // whose it is — and because two gears raising the same note would give one
    // stage-level list two entries with one key.
    let gear_notes = |i: usize| {
        let mut out = Vec::new();
        out.extend(super::notch_outside_fit(&sections[i]));
        // ...and whether this root is loaded both ways, which for a parallel
        // pair is the drive's doing alone.
        out.extend(reversal.note_for(reverses));
        out
    };
    // **What the mesh needs, not what one gear needs.** The narrower face
    // carries the pair, so a width that satisfies only its own gear satisfies
    // nothing: give gear 2 a weaker material and it asks for more, and sizing
    // gear 1 to its own smaller figure pulls the effective width — and gear 2
    // with it — under what gear 2 required. Each gear's toggles still choose
    // which of *its* ratings count; the width they resolve to is the largest ask
    // in the mesh.
    let asks = [0usize, 1].map(|i| {
        let g = &stage.gears[i];
        // An automatic width with every source switched off has nothing to
        // invert, and comes out zero. Said rather than divided by: the input
        // that produced it is on screen, and this is what it did.
        if g.face_width.auto && !g.face_sources.any() {
            notes
                .push(Note::new(key::STAGE_FACE_WIDTH_NO_SOURCE).text("gear", (i + 1).to_string()));
        }
        g.face_sources.largest_of(&probe_widths(i))
    });
    let wanted = asks[0].max(asks[1]);
    let widths = [0usize, 1].map(|i| stage.gears[i].face_width.resolve(wanted));

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
            // The share this tooth carries where it is rated — exactly 1 unless
            // a sharing model was asked for, so nothing scales by default.
            bending_stress(&sections[i], &g[i], &li, StressConcentration::Iso6336)
                .map(|s| s * load_share[i])
        });
        Ok((cs, sf, load))
    };
    let rated = LoadCase {
        peak: rate(Case::Peak)?,
        cyclic: rate(Case::Cyclic)?,
    };
    // **The rating and the reported torque are different questions.** The peak
    // *case* is rated at whichever direction loads the teeth harder, which is
    // what `rated.peak` used; the torque a gear is labelled with is the one it
    // carries driving **forward**, with the back-driving figure reported beside
    // it rather than folded into it.
    let load = Load::new(torques.peak_forward, effective);

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
            contact_stress: LoadCase::of(|case| rated.get(case).0.governing(i)),
            min_face_width: LoadCase::of(|case| {
                let (cs, sf, _) = rated.get(case);
                Widths {
                    bending: sf[i].map(|s| {
                        let allow = reversal.bending_allowable(&materials[i], case, reverses);
                        min_face_width_bending(s, effective, allow)
                    }),
                    contact: min_face_width_contact(
                        cs.governing(i),
                        effective,
                        allowable(&materials[i], case),
                    ),
                }
            }),
            clamps: g[i].clamps.notes.clone(),
            notes: gear_notes(i),
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
    let contact_ratios = ContactRatios::of(
        path.contact_ratio,
        effective,
        stage.additional_helix,
        stage.module,
    );

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
        notes.push(Note::new(key::STAGE_OVERLAP_BELOW_ONE).number(
            "ratio",
            contact_ratios.overlap,
            3,
        ));
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
    // **The sharing ramp outside the band it was described in.** It is a
    // first-order stand-in for a spur mesh with a single-pair zone; at
    // `ε_n ≥ 2` there is no such zone, the ramp never reaches a full share, and
    // it relieves the tooth by about a third. That is a large number from an
    // uncalibrated model, in the unconservative direction — exactly what
    // `docs/rationale.md` refuses to let pass silently — so the stage says so
    // where the figure is shown. The model is still the one the designer asked
    // for; what they are owed is knowing it is extrapolating.
    if !matches!(stage.load_sharing, LoadSharing::None) {
        let cos_bb = crate::metrology::base_helix_angle(&g[0]).cos();
        let eps_n = path.contact_ratio / (cos_bb * cos_bb);
        if eps_n >= 2.0 {
            notes.push(Note::new(key::STAGE_LOAD_SHARING_OUT_OF_BAND).number("ratio", eps_n, 3));
        }
    }

    Ok(SpurResult {
        ratio: f64::from(stage.gears[1].teeth) / f64::from(stage.gears[0].teeth),
        centre_distance_nominal: mesh.a_w,
        centre_distance: centre,
        operating_pressure_angle: mesh.alpha_w.to_degrees(),
        clearance,
        contact_ratios,
        contact_stress_at_pitch_point: LoadCase {
            peak: rated.peak.0.at_pitch_point,
            cyclic: rated.cyclic.0.at_pitch_point,
        },
        relative_radius: rated.peak.0.relative_radius,
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
