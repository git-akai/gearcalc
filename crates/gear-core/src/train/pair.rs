//! **The pair preset**: what a spur, helical, crossed or worm stage is built
//! from, and the readings and bounds a shape shares with it.
//!
//! [`PairStage`] holds a pair's inputs as a designer states them — two gears,
//! a module, a shaft angle, a distance and its clearance — and
//! [`super::shape::Shape::from_pair`] lays them out as two axes in the
//! ground, one mesh and one distance. It no longer solves anything: the shape
//! does, and a distance whose angle is not zero is routed to
//! [`super::crossed`] as the pair it is. What a worm stage adds is that its
//! first member states a pitch diameter where a gear states a helix angle, an
//! axial float, and the conventional proportions a worm and its wheel are
//! given ([`PairStage::worm`], carried on the shape as `Distance::worm`).
//!
//! What stays here besides the preset is what every shape reads through it:
//! [`ShiftAsked`] — who decides a shift and what it must satisfy, with the
//! search floor and the true minimum told apart — and [`Reading`]s of the
//! helix. A worm stage used to be a separate type with a separate result — no
//! profile shift, no addendum, members that were not gears — and its centre
//! distance could only be reached by resizing the worm. `docs/corrections.md`
//! records what that cost, and the audit's record (`docs/history/audit.md`,
//! F83) what deleting it moved: nothing.

use super::StageGear;
use crate::auto::automatic_profile_shift;
use crate::contact::LoadSharing;
use crate::params::Auto;

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
/// Two ways, and they want two different answers to the same question —
/// which is why this is a type rather than a boolean. A shift a designer
/// typed is the third way, and it is pinned before any bound is asked
/// ([`crate::auto::Cut::Pinned`]). See [`undercut_bound`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decided {
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
        Decided::Absorbed => Some(crate::auto::minimum_profile_shift(p, depth).with_cutter_radius),
    }
}

impl ShiftAsked {
    /// **The note a raised shift owes its reader.**
    ///
    /// `no undercut` bounds a shift a designer typed as well as one the stage
    /// chose, which is what lets it mean one thing everywhere — but a number
    /// that was not taken as given has to say so, or the field and the gear
    /// disagree in silence. It is the gear's own note, drawn under its own
    /// field, so it names no gear: it used to carry a tooth count for a list
    /// at the foot of the stage it no longer sits in.
    pub(crate) fn note(&self) -> Option<crate::note::Note> {
        self.raised.then(|| {
            crate::note::Note::new(crate::note::key::GEAR_SHIFT_RAISED_FOR_UNDERCUT).number(
                "shift",
                self.settled,
                4,
            )
        })
    }
}

/// A stage of two gears on shafts at any angle.
///
/// Spur when nothing is angled, helical when the teeth are, a **crossed gear
/// pair** when the shafts are, and a **worm stage** when the first member's
/// size is a diameter someone chose — one stage, as the specification has it,
/// with the shaft angle and which reading of the size is given as the inputs
/// that distinguish them. It
/// is not four types of stage: the tooth counts, the module, the shifts, the
/// materials and the tolerances mean the same thing throughout, and only the
/// *mesh* differs.
///
/// # The two helix angles come from the shaft angle
///
/// `β₁ + β₂ = Σ` — the relation crossed-axis screw gearing runs on
/// (docs/reference.md#crossed-axes) — so one number places them, and it can be
/// stated three ways: either member's helix angle, or the first member's
/// pitch diameter, since `d = z m_n / cos β` makes a diameter and a helix the
/// same freedom read as a size. Which reading a designer uses is the whole of
/// the difference between a worm and a crossed gear: a worm's pitch diameter
/// is a *free choice* that sets its lead angle, its efficiency and whether it
/// can be back-driven, while a gear's follows from its teeth. At `Σ = 0` it is
/// a parallel helical pair with its two hands opposed; the parallel case is
/// the shaft angle's zero rather than a separate construction.
///
/// [verified: a `Screw` built with `d₁ = z₁ m_n / cos β₁` reports
/// `γ₁ = 90° − β₁` exactly, and `β₂ = Σ − β₁`, over three tooth pairs × four
/// shaft angles × three helix angles.]
///
/// What *does* branch is the mesh, and it must: parallel axes touch along a
/// line and lose power to sliding along the profile, while crossed axes touch
/// at a point and slide lengthwise. Those are different mechanisms with
/// different formulas and different results (docs/reference.md#crossed-axes),
/// so a crossed pair's mesh report carries a contact ratio along the line of
/// action, no bending and two efficiencies — in the same [`super::MeshReport`]
/// a parallel pair's does, with [`super::PointContact`] where the other has
/// [`super::LineContact`].
///
/// # One relation among five inputs
///
/// The centre distance, the clearance, the two shifts and the size are bound
/// by `a = a₀(size, x₁ + x₂) + clearance`, so four of the five may be given and
/// the fifth follows — [`super::Stage::freedoms`] says so for every kind of
/// pair alike. Which one absorbs a given distance is a preference, not a law:
/// the shifts do wherever one of them is automatic, and the size only when
/// both shifts are pinned, because a shift moves the teeth and a size changes
/// them.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct PairStage {
    /// Normal module, mm. Shared by both gears.
    pub module: f64,
    /// Normal pressure angle, degrees. Shared.
    pub pressure_angle: f64,
    /// Shaft angle `Σ`, degrees. **Zero is a parallel-axis pair**; anything else
    /// crosses the shafts.
    #[cfg_attr(feature = "serde", serde(default))]
    pub shaft_angle: f64,
    /// **A worm and its wheel**, to a designer — one bit, carried on the
    /// shape as `Distance::worm`, and nothing in the model reads it: a
    /// worm is a helical gear with a few starts at a steep helix, its
    /// wheel a helical gear at the complementary one, and their mesh the
    /// crossed-axis mesh any two such gears have. What the bit decides is
    /// the little that is not geometry: the **automatic face width** where
    /// no rating sizes one — a worm and its wheel take a worm drive's
    /// conventional proportions ([`super::crossed::proportions`]), a
    /// crossed gear pair the width at which contact is just continuous —
    /// and **the words**, *starts*, *worm*, *wheel*, which the shape reads
    /// off the same bit ([`super::shape::Shape::member_names`]). A worm at
    /// a shaft angle of zero is a legal, if strange, helical pair and
    /// solves as one. [`Self::worm`] sets it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub worm: bool,
    /// **The first member's pitch diameter, mm** — a worm's reading of how
    /// big it is, and one of the three readings of the pair's one size freedom
    /// with the two members' helix angles ([`StageGear::helix_angle`]).
    ///
    /// Given, it decides both helix angles. Automatic with a helix given, it
    /// follows from that. Automatic with every size reading automatic, the
    /// size is solved to reach a given centre distance, but only where both
    /// shifts are pinned: a shift is the thing that absorbs a distance by
    /// preference ([`Self::first_pitch_diameter`]). A worm stage is the case
    /// that was built for — it has no profile shift by convention, so its
    /// size is what a housing decides — and a helical pair cut to fit a
    /// standard centre distance is the same request on parallel shafts.
    pub pitch_diameter: Auto<f64>,
    /// **The axial contact ratio** `ε_β` the pair is asked for, where it is
    /// asked for one.
    ///
    /// Automatic, it is an output — what the helix and the face width the
    /// mesh carries come to. Given, it is a constraint on whichever of the two
    /// is free: a floor under an automatic face width, beside the ratings'
    /// asks; or, with both widths given, the thing that decides the helix
    /// ([`super::helix_for_overlap`]). Read on parallel shafts alone — a
    /// point contact has no overlap — and the floor on a crossed pair's
    /// automatic width is nothing at all: its contact's pressure does not
    /// depend on the width, so nothing here can size one, and it stands at
    /// its box ([`super::key::GEAR_FACE_WIDTH_AS_ENTERED`]).
    pub overlap: Auto<f64>,
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
    /// and the backlash is a consequence; `solve_parallel` is where that
    /// happens and is the whole of the rule.
    pub clearance: Auto<f64>,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// What the stage is asked to optimise, and what it may not do to get
    /// there. See [`super::Optimisation`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: super::Optimisation,
    /// The transverse contact ratio the search may not take the mesh
    /// below ([`super::DEFAULT_MIN_CONTACT_RATIO`]).
    #[cfg_attr(feature = "serde", serde(default = "super::default_min_contact_ratio"))]
    pub min_contact_ratio: f64,
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
    /// Axial float of the first member along its own axis, mm.
    ///
    /// The dominant source of backlash in a worm drive, and **a displacement
    /// every helical gear has**: a rigid slide along the axis opens one flank
    /// exactly as far as it closes the other, by its component along the
    /// common normal, `j_axial sin β_b1`. On a spur gear that component is zero
    /// and the input is idle rather than wrong, which is why a spur kind need
    /// not show it and a worm kind must.
    #[cfg_attr(feature = "serde", serde(default))]
    pub axial_clearance: f64,
    pub gears: [StageGear; 2],
}

impl Default for PairStage {
    fn default() -> Self {
        // The words a pair shares with every shape default where the shape
        // does, once.
        let shape = super::shape::Shape::default();
        Self {
            module: 1.0,
            pressure_angle: crate::params::GearParams::default().pressure_angle,
            shaft_angle: 0.0,
            worm: false,
            pitch_diameter: Auto::automatic(17.0),
            overlap: super::shape::default_overlap(),
            sliding_friction: 0.08,
            static_friction: 0.16,
            thickness_mod: 1.0,
            optimisation: shape.optimisation,
            min_contact_ratio: super::DEFAULT_MIN_CONTACT_RATIO,
            centre_distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            load_sharing: shape.load_sharing,
            axial_clearance: 0.0,
            // Neither member states a helix, so the two share the shaft
            // angle evenly — straight teeth on parallel shafts.
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

impl PairStage {
    /// **The worm preset**: a single start of 7 mm pitch diameter at a
    /// right angle to a 40-tooth brass wheel, with the float a worm's thrust
    /// bearing leaves it.
    ///
    /// Two conventions of worm practice are set here as inputs rather than
    /// built in, so a designer can undo either:
    ///
    /// - **the worm carries no profile shift** — it is the tool its wheel is
    ///   cut by, so its shift is pinned at zero and a given centre distance is
    ///   absorbed by the *wheel's* shift, as DIN 3975 has it; pin the wheel's
    ///   too and the worm's size absorbs it instead;
    /// - **the face widths are automatic**, and the worm kind resolves them to
    ///   a worm drive's conventional proportions rather than to a rating
    ///   ([`Self::worm`], the field).
    #[must_use]
    pub fn worm() -> Self {
        // The helix boxes hold what 7 mm on one start gives — `cos β₁ = m/d₁`,
        // and the wheel's is the rest of the right angle — so a reading pinned
        // by relief stands where the diameter had it rather than at a zero.
        let worm_helix = (1.0_f64 / 7.0).acos().to_degrees();
        Self {
            shaft_angle: 90.0,
            worm: true,
            pitch_diameter: Auto::fixed(7.0),
            axial_clearance: 0.04,
            gears: [
                StageGear {
                    teeth: 1,
                    profile_shift: Auto::fixed(0.0),
                    // A worm's thread has no fillet of the rack's kind: its
                    // root is cut by the thread mill's own round, which this
                    // model does not describe, so the coefficient is nought.
                    root_radius: 0.0,
                    helix_angle: Auto::automatic(worm_helix),
                    face_width: Auto::automatic(10.0),
                    ..StageGear::default()
                },
                StageGear {
                    teeth: 40,
                    helix_angle: Auto::automatic(90.0 - worm_helix),
                    face_width: Auto::automatic(10.0),
                    material: "Brass C360".to_string(),
                    ..StageGear::default()
                },
            ],
            ..Self::default()
        }
    }

    /// **This pair with its first member's helix stated**, degrees — the other
    /// readings of the size left to follow. `β₂ = Σ − β₁`.
    #[must_use]
    pub fn with_first_helix(mut self, beta_deg: f64) -> Self {
        self.gears[0].helix_angle = Auto::fixed(beta_deg);
        self.gears[1].helix_angle.auto = true;
        self.pitch_diameter.auto = true;
        self
    }

    /// **This pair with what each member carries beyond half the shaft angle
    /// stated**, degrees: `β₁ = Σ/2 + β_add`, `β₂ = Σ/2 − β_add`. At `Σ = 0`
    /// it is the familiar shared helix with the hands opposed — the
    /// specification's own "Total Helix Angle = 0.5 × Axis Angle + Additional
    /// Helix Angle" — stated on the first member.
    #[must_use]
    pub fn with_additional_helix(self, add_deg: f64) -> Self {
        let half = self.shaft_angle / 2.0;
        self.with_first_helix(half + add_deg)
    }

    /// **This pair with every reading of its size automatic**, so a given
    /// centre distance with both shifts pinned decides it, or a given axial
    /// contact ratio with both widths given does.
    #[must_use]
    pub fn size_free(mut self) -> Self {
        self.pitch_diameter.auto = true;
        for g in &mut self.gears {
            g.helix_angle.auto = true;
        }
        self
    }

    /// **This pair with its first member's pitch diameter stated**, mm — a
    /// worm's reading, with both helix angles left to follow.
    #[must_use]
    pub fn with_first_diameter(mut self, d1: f64) -> Self {
        self.pitch_diameter = Auto::fixed(d1);
        for g in &mut self.gears {
            g.helix_angle.auto = true;
        }
        self
    }

    /// Whether the shafts cross. The parallel case is the zero of the shaft
    /// angle, and it is the *mesh* that differs, not the stage.
    #[must_use]
    pub fn is_crossed(&self) -> bool {
        self.shaft_angle != 0.0
    }
}

/// **The tests' door.** What the tests ask of a pair — its shifts, its
/// screw geometry, the gear it builds — they ask of the shape it lays out,
/// in the pair's own vocabulary; nothing here is a second chooser.
#[cfg(test)]
impl PairStage {
    /// The shape this pair lays out: two axes in ground at the shaft angle,
    /// one mesh, one distance.
    pub(crate) fn shape(&self) -> super::shape::Shape {
        super::shape::Shape::from(self)
    }

    /// The two profile shifts the shape settles on, at the shipped effort.
    pub(crate) fn shifts(&self) -> [f64; 2] {
        self.shifts_at(&crate::auto::Search::SHIPPED)
    }

    /// As [`Self::shifts`], at a stated search effort.
    pub(crate) fn shifts_at(&self, search: &crate::auto::Search) -> [f64; 2] {
        let x = self.shape().shifts_at(search);
        [x[0], x[1]]
    }

    /// The crossed-axis geometry this stage describes, at the shifts the
    /// shape decides — its zero-backlash distance is the stage's.
    pub(crate) fn geometry(&self) -> Result<crate::screw::Screw, super::TrainError> {
        self.shape().screw(0)
    }

    /// The first member's pitch diameter, mm, as the shape reads it from
    /// the helix the readings decide.
    pub(crate) fn first_pitch_diameter(&self) -> f64 {
        f64::from(self.gears[0].teeth.max(1)) * self.module
            / self.helix_angles()[0].to_radians().cos()
    }

    /// The two gears' helix angles, degrees, as the shape reads them.
    pub(crate) fn helix_angles(&self) -> [f64; 2] {
        let h = self.shape().helix_angles();
        [h[0], h[1]]
    }

    /// The gear the stage would build at a given shift.
    pub(crate) fn params_at(&self, i: usize, x: f64) -> crate::params::GearParams {
        self.shape().params_of(i, x)
    }

    /// `GearParams` for one gear, before any automatic value is resolved.
    pub(crate) fn base_params(&self, i: usize) -> crate::params::GearParams {
        self.shape().base_params_of(i)
    }
}

/// The pair presets' old entry point, kept for the tests written against
/// it: a pair through the shape.
#[cfg(test)]
pub(crate) fn solve_pair_stage(
    stage: &PairStage,
    loads: &super::StageLoads,
    lib: &crate::material::MaterialLibrary,
) -> Result<super::shape::ShapeResult, super::TrainError> {
    super::shape::solve_loads(
        &super::shape::Shape::from(stage),
        loads,
        lib,
        super::Reversal::default(),
    )
}
