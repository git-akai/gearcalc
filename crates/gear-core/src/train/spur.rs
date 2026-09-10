//! The parallel-axis stage: spur when the helix angle is zero, helical
//! otherwise.
//!
//! Split out of `train.rs` when the worm stage arrived and gave the division
//! something to divide. What stays in the parent module is everything a stage
//! of *any* kind produces — [`StageResult`], [`GearResult`], [`Backlash`] — and
//! the train that strings them together.

use super::{
    Backlash, Case, ContactRatios, GearResult, LoadCase, Loading, MemberRating, MeshReport,
    SpurResult, StageGear, StageTorques, TrainError, PROBE,
};
use crate::auto::automatic_profile_shift;
use crate::contact::{efficiency, ContactPath, Directional, LoadSharing};
use crate::material::{contact_modulus, Material, MaterialLibrary};
use crate::mesh::{Mesh, MeshKind, MeshSide};
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::strength::{bending_stress, contact_stress, Load, RootStressModel, PARALLEL_AXES};
use crate::tooth::Tooth;

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
/// *search* at the true minimum let the hula stage's split walk out to
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

/// **How a member's shift came to be**, which is what decides the undercut
/// bound it answers to.
///
/// Three ways, and they want three different answers to the same question —
/// which is why this is a type rather than a boolean about whether a shift was
/// typed. See [`undercut_bound`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decided {
    /// A designer typed it. Already held to its own bound when it was read.
    Given,
    /// A search is choosing it, and may pick any admissible value.
    Chosen,
    /// A relation left it — the planet's shift closing an epicyclic set's two
    /// centre distances, say. Nothing is free to move it.
    Absorbed,
}

/// **The undercut bound a member answers to**, or `None` where it answers to
/// none.
///
/// One question, three answers, and each of them is the honest one for how the
/// number arrived:
///
/// | how | bound | why |
/// |---|---|---|
/// | [`Decided::Chosen`] | `max(x_min, 0)` | a chooser should not thin a tooth that needed no help |
/// | [`Decided::Given`] | none | it was held to `x_min` when it was read; re-judging it here rejects legal designs |
/// | [`Decided::Absorbed`] | `x_min` | nothing can move it, so the only honest question is whether it *does* undercut |
///
/// The middle row is a bug this had twice: a search that re-judges a number it
/// was handed throws away every candidate built on a perfectly legal one. The
/// last row is the same mistake wearing different clothes — an absorbed shift
/// clamped to a chooser's floor would break the relation that produced it.
pub(crate) fn undercut_bound(
    no_undercut: bool,
    p: &crate::params::GearParams,
    depth: f64,
    how: Decided,
) -> Option<f64> {
    if !no_undercut {
        return None;
    }
    match how {
        Decided::Chosen => Some(automatic_profile_shift(p, depth)),
        Decided::Given => None,
        Decided::Absorbed => Some(crate::auto::minimum_profile_shift(p, depth).with_cutter_radius),
    }
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
    /// Whether a stage turns at all is decided at rest and against this; how
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
    /// **Read only where something is free to absorb it** — the automatic
    /// centre distance, which is this opened out, or the shifts when they are
    /// being chosen. Given a distance with nothing free, it is not read at all
    /// and the backlash is a consequence; `solve_spur_stage` is where that
    /// happens and is the whole of the rule.
    pub clearance: Auto<f64>,
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
            clearance: Auto::fixed(0.02),
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

    /// The two profile shifts, chosen together where that is what the stage
    /// asked for.
    ///
    /// With the toggle off each gear answers on its own, as it always has: the
    /// manual value, or the least shift that clears undercut. With it on the
    /// pair is chosen at once, because a shift is only good or bad relative to
    /// the one it meshes with — and what a designer has already given is handed
    /// over as pinned rather than overridden.
    pub(super) fn shifts(&self) -> [f64; 2] {
        self.shifts_at(&crate::auto::Search::SHIPPED)
    }

    /// As [`Self::shifts`], at a stated search effort — which is what makes
    /// "the shipped effort is converged" a claim something can raise and check
    /// rather than a comment (`auto::Search`).
    pub(super) fn shifts_at(&self, search: &crate::auto::Search) -> [f64; 2] {
        let asked = [0, 1].map(|i| self.gears[i].shift_asked(&self.base_params(i)));
        let floor = asked.map(|a| a.search_floor);
        let given = asked.map(|a| a.given);
        // **Mode 3 needs both of them given.** A distance with an *automatic*
        // clearance is the designer asking what gap their shifts leave — mode 2
        // — and solving the shifts from the distance would answer a question
        // they did not ask. So the sum is pinned only when the clearance is a
        // number they stated.
        let sum = (!self.centre_distance.auto && !self.clearance.auto)
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
                    self.centre_distance.manual - self.clearance.manual,
                )
            })
            .flatten();

        // **What the constraints alone imply**, with no objective involved.
        //
        // A given centre distance fixes the shift *sum*; what it leaves
        // undecided is the division, and with nothing to optimise that follows a
        // stated rule ([`crate::auto::divide_shift_sum`]) rather than a search.
        // A shift a designer *gave* is never one of the numbers being chosen: it
        // stands, and the other member absorbs the whole of the rest.
        //
        // `None` where no distance was given — there is then nothing to place —
        // or where no admissible pair of shifts reaches it, which is F55.
        let constrained = || -> Option<[f64; 2]> {
            let sum = sum?;
            match given {
                [Some(a), Some(b)] => Some([a, b]),
                [Some(a), None] => Some([a, sum - a]),
                [None, Some(b)] => Some([sum - b, b]),
                [None, None] => {
                    crate::auto::divide_shift_sum(&|i, x| self.params_at(i, x), 1.0, sum, floor)
                }
            }
        };

        // **A given centre distance is a constraint whether or not anything is
        // being optimised.** It used to be read only on the optimiser's path, so
        // the plainest thing a designer does — type a housing distance with
        // nothing asked to move — returned the undercut floor and ran at
        // whatever distance that happened to make. Mode 3 of the clearance
        // paradigm (`docs/reference.md#which-of-the-three-numbers-is-given-and-which-follows`) is the
        // rule: the distance and the clearance are given, so the shifts follow.
        if !self.optimisation.enabled {
            return constrained().unwrap_or_else(|| asked.map(|a| a.settled));
        }
        crate::auto::shifts_for_efficiency(
            &|x| [0, 1].map(|i| self.params_at(i, x[i])),
            crate::mesh::MeshKind::External,
            &crate::auto::Bounds {
                floor,
                min_contact_ratio: self.optimisation.min_contact_ratio,
                // **The gap the designer nominated**, which is a number they
                // stated even where the *reported* clearance is derived from a
                // given distance. It is a guard on the trial mesh — the teeth
                // must not bottom out — so what it wants is the intended gap,
                // not whatever a candidate's shifts happen to leave.
                clearance: self.clearance.manual,
            },
            &crate::auto::Pinned { shift: given, sum },
            self.sliding_friction,
            search,
        )
        // **Failing to optimise must not abandon a constraint.** The search has
        // its own conditions — a minimum contact ratio, a tool that leaves the
        // members alone — and where none of the candidates meets them it returns
        // nothing. Falling back to `settled` then threw away the *centre
        // distance* along with the optimisation, so a pair told to run at
        // 23.6866 mm with 0.2 of clearance ran at 23.6433 instead, and said so
        // only through a clearance readout nobody was watching.
        //
        // The objective is the thing being given up; the constraints are not.
        // So the fallback is what the constraints alone imply, and only where
        // *that* has no answer does the stage fall back to what it would have
        // built unasked.
        .or_else(constrained)
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
            addendum: g.addendum_asked(&with_shift).used,
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
            addendum: g.addendum,
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
    //
    // **Where the clearance is read is the rule**, and there is nothing else to
    // it. With the distance automatic it is the zero-backlash distance opened
    // out, and that opening *is* the backlash. With the distance given by hand
    // there is nothing left to move — the shifts sit where they sit, the
    // distance is whatever was typed — so the clearance is not read here, and
    // the backlash is a consequence rather than a choice. Unless the shifts are
    // being chosen, in which case *they* absorb it: `shifts_at` pins the sum a
    // clearance inside the given distance, so the designer gets both the housing
    // and the play. [`SpurResult::clearance`] reports what came of it, derived.
    //
    // This used to be a `clearance_taken()` returning zero in the given-distance
    // case, which reads as the rule and enforced none of it: **every caller of
    // it already ran where its condition held**, so the arm returning zero was
    // dead. Removing it moved no test and no recorded figure — which is what a
    // conditional that decides nothing does, while suggesting there are two
    // answers here.
    // The number in the box. Read here only where the distance is automatic —
    // which is the branch below — and there the clearance is what *makes* the
    // distance, so it is an input by definition. With both automatic nothing
    // would determine either, and `Stage::relieved` is what stops a designer
    // reaching that state; a hand-written document that does reach it gets the
    // value it wrote rather than a refusal.
    let clearance = stage.clearance.manual;
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
    let e_star = contact_modulus(&materials[0], &materials[1]);

    // The critical section, and the share of the load acting on it. With
    // sharing off — the default — this *is* `bending_section` and a share of
    // exactly 1, so the ordinary rating is untouched to the bit.
    let bending = [0usize, 1].map(|i| {
        super::Bending::of(
            &g[i],
            path.contact_ratio,
            stage.load_sharing,
            stage.gears[i].rim_thickness,
        )
    });
    let [Some(first), Some(second)] = bending else {
        return Err(TrainError::NoRootSection);
    };
    let bending = [first, second];
    let sections = [bending[0].section, bending[1].section];
    let load_share = [bending[0].share, bending[1].share];
    let rims = [bending[0].rim, bending[1].rim];

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
            bending_stress(
                &sections[i],
                li.tangential(&g[i]),
                li.face_width,
                RootStressModel::DolanBroghamer,
                rims[i],
            )
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
    // **A pair is one mesh, so each member is in a list of one** — and it says
    // so through the same type an epicyclic set's planet says it is in two.
    // What is this stage's own is that each load case is *evaluated* rather
    // than scaled: it has its own torque, and nothing here has to claim the
    // stresses are linear in it (`Loading::both_cases` is that claim, and the
    // stages that make it are the ones whose power split does not depend on the
    // magnitude passing through them).
    let rating = |i: usize,
                  at: &LoadCase<(crate::strength::ContactStress, [Option<f64>; 2])>,
                  measured_at: f64,
                  carried_at: f64| MemberRating {
        material: &materials[i],
        reversal,
        reverses,
        loadings: LoadCase::of(|case| {
            let (cs, sf) = at.get(case);
            vec![Loading {
                bending: sf[i],
                // **This gear's** governing point, not the pair's envelope: the
                // width a gear needs follows from the stress it is rated at.
                contact: cs.governing(i),
                measured_at,
                carried_at,
            }]
        }),
    };

    let mut notes = Vec::new();
    // What the rating has to say about each gear. Per gear, because that is
    // whose it is — and because two gears raising the same note would give one
    // stage-level list two entries with one key.
    let gear_notes = |i: usize| {
        let mut out = Vec::new();
        // ...and whether its rim is thinner than ISO 6336-3 will rate.
        out.extend(super::rim_below_minimum(rims[i]));
        // ...and whether the cutter has eaten into the flank, which no toggle
        // can prevent once a shift is given and `no undercut` is off.
        out.extend(super::undercut_note(&g[i]));
        // ...and whether this root is loaded both ways, which for a parallel
        // pair is the drive's doing alone.
        out.extend(reversal.note_for(reverses));
        // **A bound that moved this gear's own number belongs to this gear.**
        // A note naming an input is one the reader wants under that input, not
        // in a list at the foot of the stage where they have to match it back
        // up by tooth count.
        let g = &stage.gears[i];
        out.extend(g.shift_asked(&stage.base_params(i)).note(g.teeth));
        out.extend(
            g.addendum_asked(&GearParams {
                profile_shift: x[i],
                ..stage.base_params(i)
            })
            .note(g.teeth),
        );
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
        // invert, so it stands at the number in its box and the stage says so
        // (`FaceSources::width_for`). Said rather than divided by, which is
        // what it was: a zero width made every stress infinite.
        if g.face_width.auto && !g.face_sources.any() {
            notes
                .push(Note::new(key::STAGE_FACE_WIDTH_NO_SOURCE).text("gear", (i + 1).to_string()));
        }
        g.face_sources.width_for(
            &rating(i, &probed, PROBE, PROBE).asks(),
            g.face_width.manual,
        )
    });
    let wanted = asks[0].max(asks[1]);
    let widths = [0usize, 1].map(|i| stage.gears[i].face_width.resolve(wanted));

    // The spec is explicit: the *narrower* gear carries the mesh, so both gears
    // are rated at the smaller width regardless of which one owns it.
    let effective = widths[0].min(widths[1]);

    // Every rating again, at the width actually in force. Two evaluations
    // rather than one, and the same expression: a load case is a torque, and
    // nothing else about the stage knows which one it is looking at.
    let rate =
        |case: Case| -> Result<(crate::strength::ContactStress, [Option<f64>; 2]), TrainError> {
            let load = Load::new(torques.at(case), effective);
            let cs = contact_stress(&path, &operating, &g[0], PARALLEL_AXES, &load, e_star)
                .ok_or(TrainError::NoContact)?;
            let sf = [0usize, 1].map(|i| {
                let li = load.across_mesh(&g[0], &g[i]);
                // The share this tooth carries where it is rated — exactly 1 unless
                // a sharing model was asked for, so nothing scales by default.
                bending_stress(
                    &sections[i],
                    li.tangential(&g[i]),
                    li.face_width,
                    RootStressModel::DolanBroghamer,
                    rims[i],
                )
                .map(|s| s * load_share[i])
            });
            Ok((cs, sf))
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
        // Measured at the width in force and carried at it, so nothing scales
        // and the figures are the ones this stage's own arithmetic produced.
        let this = rating(i, &rated, effective, effective).rated();
        gears.push(GearResult::of(super::MemberFacts {
            profile_shift: p[i].profile_shift,
            params: &p[i],
            input: &stage.gears[i],
            rated: this,
            face_width: widths[i],
            torque: load_i.torque,
            back_driving_torque: torques.referred_like(load_i.torque),
            // Filled in by `solve_train`, which is the only level that knows
            // where this gear sits in the shaft line.
            speed: 0.0,
            material: materials[i].clone(),
            clamps: g[i].clamps.notes.clone(),
            notes: gear_notes(i),
        }));
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
    // **Per member, which is what the gap is.** A mesh has one gap and two ends
    // to see it from; which end is the *output* is a question about the drive,
    // and `MeshReport::backlash_by_drive` is the one place that turns the first
    // reading into the second. It used to be a `match` from `Drive` to
    // `MeshSide` written out here as well.
    let at_member = |at: MeshSide| {
        Backlash::banded(centre, stage.tolerance_minus, stage.tolerance_plus, |d| {
            angular(d, at)
        })
    };
    let backlash = [at_member(MeshSide::First), at_member(MeshSide::Second)];

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
    // What the sharing model has to say about this mesh, if anything — raised
    // where the section and the share are worked out, so no stage kind has to
    // remember to ask (`train::Bending`). One mesh, so one note at most.
    notes.extend(bending[0].note.clone());

    Ok(SpurResult {
        ratio: f64::from(stage.gears[1].teeth) / f64::from(stage.gears[0].teeth),
        centre_distance_nominal: mesh.a_w,
        centre_distance: centre,
        // The gap the pair runs at, which is the two distances above it and a
        // subtraction rather than the input echoed back.
        clearance: centre - mesh.a_w,
        mesh: MeshReport {
            operating_pressure_angle: mesh.alpha_w.to_degrees(),
            coprime: super::gcd(stage.gears[0].teeth, stage.gears[1].teeth) == 1,
            contact_ratios,
            // Breaking away is decided at rest, running is decided sliding —
            // one rule, applied to every stage kind (`Directional::once_moving`).
            // A parallel-axis mesh is never near the threshold, so this passes
            // the sliding figure through and always will; it is here so there is
            // no stage kind the rule has to be remembered for.
            efficiency: {
                let with =
                    |mu: f64| Directional::of(|d| efficiency(&path, &operating, &g[0], mu, d));
                with(stage.sliding_friction).once_moving(&with(stage.static_friction))
            },
            contact_stress_at_pitch_point: LoadCase {
                peak: rated.peak.0.at_pitch_point,
                cyclic: rated.cyclic.0.at_pitch_point,
            },
            relative_radius: rated.peak.0.relative_radius,
            // Per member, in the order the mesh was built — which
            // `MeshReport::backlash_by_drive` is the one place that turns into
            // the per-direction reading a stage reports.
            backlash,
            // A parallel-axis pair is external: its tips meet on the line of
            // centres or not at all, which `bottom_clearance` already asks.
            tips: None,
        },
        gears: [gears[0].clone(), gears[1].clone()],
        notes,
    })
}
