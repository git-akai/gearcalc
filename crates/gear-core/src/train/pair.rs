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
//! given ([`PairKind`], carried on the shape as `Distance::worm`).
//!
//! What stays here besides the preset is what every shape reads through it:
//! [`ShiftAsked`] — who decides a shift and what it must satisfy, with the
//! search floor and the true minimum told apart — and [`Reading`]s of the
//! helix. A worm stage used to be a separate type with a separate result — no
//! profile shift, no addendum, members that were not gears — and its centre
//! distance could only be reached by resizing the worm. `docs/corrections.md`
//! records what that cost, and the audit's record (`docs/history/audit.md`,
//! F83) what deleting it moved: nothing.

use super::{Freedom, Reading, StageGear, TrainError};
use crate::auto::automatic_profile_shift;
use crate::contact::LoadSharing;
use crate::mesh::{Mesh, MeshKind};
use crate::params::{Auto, GearParams};
use crate::screw::{Screw, ScrewParams};
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

/// **Which pair this is, to a designer** — the layer over [`PairStage`] that
/// a preset is.
///
/// The model underneath is one model: a worm is a helical gear with a few
/// starts at a steep helix, its wheel a helical gear at the complementary one,
/// and their mesh the crossed-axis mesh any two such gears have. What the kind
/// decides is the little that is not geometry:
///
/// - **the preset** — `Spur` starts as 17/43 on parallel shafts, `Worm` as a
///   single start of 7 mm at a right angle to a 40-tooth brass wheel;
/// - **the automatic face width** where no rating sizes one — a worm and its
///   wheel take the conventional proportions of a worm drive
///   ([`super::crossed::proportions`]), a crossed gear pair the width at which
///   contact is just continuous;
/// - **the words** — *starts*, *worm*, *wheel* — and which inputs a panel puts
///   in front of the designer, which is the front end's to read off the kind
///   and nothing the core has to know.
///
/// Nothing in it is a constraint the model needs: a `Worm` at a shaft angle of
/// zero is a legal, if strange, helical pair and solves as one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum PairKind {
    /// A spur or helical pair, or a crossed gear pair.
    Spur,
    /// A worm and its wheel.
    Worm,
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
        Self {
            module: 1.0,
            pressure_angle: 20.0,
            shaft_angle: 0.0,
            pitch_diameter: Auto::automatic(17.0),
            overlap: Auto::automatic(1.0),
            sliding_friction: 0.08,
            static_friction: 0.16,
            thickness_mod: 1.0,
            optimisation: super::Optimisation::default(),
            centre_distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            load_sharing: LoadSharing::None,
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
    ///   ([`PairKind`]).
    #[must_use]
    pub fn worm() -> Self {
        // The helix boxes hold what 7 mm on one start gives — `cos β₁ = m/d₁`,
        // and the wheel's is the rest of the right angle — so a reading pinned
        // by relief stands where the diameter had it rather than at a zero.
        let worm_helix = (1.0_f64 / 7.0).acos().to_degrees();
        Self {
            shaft_angle: 90.0,
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

    /// **Whether the axial contact ratio is a reading of the size**: both
    /// face widths are given, so the width the mesh carries is known and a
    /// ratio can be turned into a helix at it, and the shafts are parallel so
    /// there is an overlap to reach. A fact about the widths, not about the
    /// ratio's own toggle — which is what lets relief count it among the
    /// readings the moment it is pinned.
    #[must_use]
    pub fn overlap_reads_size(&self) -> bool {
        !self.is_crossed() && self.gears.iter().all(|g| !g.face_width.auto)
    }

    /// **Whether the axial contact ratio decides the helix**: it is a reading
    /// of the size, and it is given.
    #[must_use]
    pub fn size_taken_by_overlap(&self) -> bool {
        !self.overlap.auto && self.overlap_reads_size()
    }

    /// The width the mesh carries as given — the narrower of two given faces.
    fn given_width(&self) -> f64 {
        self.gears[0]
            .face_width
            .manual
            .min(self.gears[1].face_width.manual)
    }

    /// The two gears' helix angles, degrees — from whichever reading of the
    /// size is given ([`super::Reading`]), or from what decides it where none
    /// is ([`Self::first_pitch_diameter`]).
    ///
    /// One place to ask, so the pair cannot disagree about a shaft angle they
    /// share — and so `β₁ + β₂ = Σ` holds by construction rather than by a test.
    /// A stated helix is kept in its own words, to the bit, rather than
    /// becoming a diameter and coming back through an arccosine.
    /// Either member's helix and the first member's diameter as readings of
    /// the one size, and the ratio where it decides it — what the shape
    /// declares to relief for every member; kept here for the crossed
    /// model's own reading of the pair.
    fn readings(&self) -> Vec<Reading> {
        let z1 = f64::from(self.gears[0].teeth.max(1)) * self.module;
        let mut out = vec![
            Reading::helix(0, &self.gears[0], |b| b),
            Reading::helix(1, &self.gears[1], |b| self.shaft_angle - b),
            Reading {
                freedom: Freedom::FirstPitchDiameter,
                // `cos β = z m_n / d`, clamped so a diameter below the tooth's
                // own reads as straight teeth rather than a NaN.
                helix: (!self.pitch_diameter.auto).then(|| {
                    (z1 / self.pitch_diameter.manual)
                        .clamp(-1.0, 1.0)
                        .acos()
                        .to_degrees()
                }),
            },
        ];
        if self.overlap_reads_size() {
            out.push(Reading::overlap(
                &self.overlap,
                self.module,
                self.given_width(),
            ));
        }
        out
    }

    #[must_use]
    pub fn helix_angles(&self) -> [f64; 2] {
        let first = super::stated_helix(&self.readings()).unwrap_or_else(|| {
            // A diameter, given or solved. `cos β = z m_n / d`, clamped so a
            // diameter below the tooth's own — which `geometry` refuses —
            // reads as a helix of zero rather than a NaN.
            let cos = (f64::from(self.gears[0].teeth.max(1)) * self.module
                / self.first_pitch_diameter())
            .clamp(-1.0, 1.0);
            cos.acos().to_degrees()
        });
        [first, self.shaft_angle - first]
    }

    /// Whether the shafts cross. The parallel case is the zero of the shaft
    /// angle, and it is the *mesh* that differs, not the stage.
    #[must_use]
    pub fn is_crossed(&self) -> bool {
        self.shaft_angle != 0.0
    }

    /// **The first member's pitch diameter, mm** — from whichever reading of
    /// the size is given, or from what decides it where none is.
    ///
    /// The readings are one number, so this is the one accessor the crossed
    /// geometry is built from whichever was stated: the diameter itself, to
    /// the bit, or `z₁ m_n / cos β₁` from the helix the readings state. With
    /// every reading automatic, what decides it is a given centre distance
    /// with **both shifts pinned** — a shift absorbs a distance by
    /// preference, since it moves the teeth where a size changes them, so
    /// while one is free the size has nothing to absorb (see
    /// [`Self::size_reaching`], which is also where the two-answers problem
    /// is dealt with) — and otherwise **the shaft angle shared evenly**,
    /// which at `Σ = 0` is a spur pair and at a right angle a 45°/45° crossed
    /// one.
    ///
    /// Falling back to the even split where the distance cannot be reached is
    /// `docs/rationale.md`'s clamp-rather-than-refuse: the stage still solves,
    /// at a distance the reported clearance then makes visible.
    #[must_use]
    pub fn first_pitch_diameter(&self) -> f64 {
        if !self.pitch_diameter.auto {
            return self.pitch_diameter.manual;
        }
        let z1 = f64::from(self.gears[0].teeth.max(1)) * self.module;
        let of_helix = |beta_deg: f64| z1 / beta_deg.to_radians().cos();
        if let Some(beta) = super::stated_helix(&self.readings()) {
            return of_helix(beta);
        }
        let shifts_pinned = self.gears.iter().all(|g| !g.profile_shift.auto);
        if shifts_pinned {
            // The branch is chosen by the diameter in the box — the designer's
            // own number, which is what `size_reaching` argues from — held to
            // the tooth's own diameter below which no pair exists.
            let from = self.pitch_diameter.manual.max(z1 * 1.000_001);
            if let Some(d1) = self
                .nominal_distance()
                .and_then(|target| self.size_reaching(target, from))
            {
                return d1;
            }
        }
        of_helix(self.shaft_angle / 2.0)
    }

    /// **The first member's size that puts this pair at `target`**, mm of pitch
    /// diameter — where one exists on the branch the stage is already on.
    ///
    /// # Two answers, and the branch is chosen by continuity
    ///
    /// On crossed shafts the distance has a **minimum** in the first member's
    /// diameter ([`Screw::least_distance_lead_angle`]): steepening the thread
    /// shrinks the worm and grows the wheel, and past the turning point the
    /// second wins. So a target above the minimum is reached by two worms, and
    /// picking one is a decision rather than a calculation.
    ///
    /// It is taken **on the side the designer's own number is on**, which is the
    /// only choice under which nudging the target moves the answer smoothly
    /// instead of jumping between a thin fast worm and a fat slow one. A target
    /// *below* the minimum is reached by neither and there is no answer to give.
    ///
    /// On parallel shafts there is no turning point — the distance only grows
    /// with the helix — and the one branch runs from the tooth's own diameter
    /// upward.
    fn size_reaching(&self, target: f64, from: f64) -> Option<f64> {
        let z1 = f64::from(self.gears[0].teeth.max(1));
        let floor = z1 * self.module;
        // **Degrees here, radians there.** `shaft_angle` is the designer's
        // number and `Screw`'s is the mathematics'.
        let turning = Screw::least_distance_lead_angle(
            self.gears[0].teeth.max(1),
            self.gears[1].teeth,
            self.shaft_angle.to_radians(),
        )
        .map(|least| z1 * self.module / least.sin());

        // The zero-backlash distance the whole stage would sit at with this
        // size — the mesh's own, whichever mesh it is — with the shifts as the
        // stage decides them. Both are pinned wherever this runs.
        let distance = |d1: f64| -> f64 {
            let mut probe = self.clone();
            probe.pitch_diameter = Auto::fixed(d1);
            probe.zero_backlash_distance().unwrap_or(f64::NAN)
        };

        // **`from` rather than `first_pitch_diameter()`** — that is what calls
        // this, and reading it back here would recurse forever. It is the
        // designer's own number, which is the whole point: the branch is chosen
        // by where they already are. `floor` is the diameter at which the
        // thread would wrap at a right angle, where `Screw::new` refuses.
        let grown = |from: f64| {
            // No upper bound in the geometry, so one is grown until it brackets
            // — the distance rises without bound on this branch, so it does.
            let mut top = from * 2.0;
            for _ in 0..60 {
                if distance(top) >= target || !distance(top).is_finite() {
                    break;
                }
                top *= 2.0;
            }
            top
        };
        let (lo, hi) = match turning {
            Some(turning) if from <= turning => (floor * (1.0 + 1e-9), turning),
            Some(turning) => (turning, grown(turning.max(from))),
            None => (floor * (1.0 + 1e-9), grown(from.max(floor * 2.0))),
        };
        crate::solve::brent(
            |d1| distance(d1) - target,
            lo,
            hi,
            crate::solve::Tol::default(),
        )
    }

    /// **The zero-backlash centre distance this stage sits at**, mm, with the
    /// shifts as it decides them — the parallel mesh's `a_w` or the crossed
    /// mesh's rack-law distance, whichever mesh it has.
    ///
    /// `None` where the pair cannot mesh at all.
    #[must_use]
    pub fn zero_backlash_distance(&self) -> Option<f64> {
        let x = self.chosen_at(&crate::auto::Search::SHIPPED).shifts;
        if self.is_crossed() {
            return self.screw_at(x).ok().map(|s| s.centre_distance);
        }
        let g = [0, 1].map(|i| Tooth::new(self.params_at(i, x[i])));
        Mesh::new(&g[0], &g[1], MeshKind::External)
            .ok()
            .map(|m| m.a_w)
    }

    /// The crossed-axis geometry this stage describes, at given shifts.
    ///
    /// # Errors
    ///
    /// [`TrainError::Screw`] if the pair cannot exist.
    pub fn screw_at(&self, shifts: [f64; 2]) -> Result<Screw, TrainError> {
        // Caught here rather than in `Screw::new`, because by then the helix
        // angle has become a diameter and the information is gone: `cos 90°` is
        // 6e-17, not zero, so the diameter comes out enormous rather than
        // infinite and passes every finiteness check downstream.
        if self.helix_angles()[0].abs() >= 90.0 {
            return Err(TrainError::Screw(
                crate::screw::ScrewError::FirstMemberIsADisc,
            ));
        }
        Screw::new(&ScrewParams {
            normal_module: self.module,
            normal_pressure_angle_rad: self.pressure_angle.to_radians(),
            shaft_angle_rad: self.shaft_angle.to_radians(),
            starts: self.gears[0].teeth,
            wheel_teeth: self.gears[1].teeth,
            worm_pitch_diameter: self.first_pitch_diameter(),
            profile_shifts: shifts,
        })
        .map_err(TrainError::Screw)
    }

    /// **This stage with its automatic size resolved to a number**, so the
    /// geometry below it is built once rather than solved again at every read
    /// of a helix angle.
    ///
    /// Only where every reading *was* automatic: a stated reading is kept in
    /// its own words, so a stated helix stays itself to the bit rather than
    /// becoming a diameter and coming back through an arccosine.
    #[must_use]
    pub fn sized(&self) -> Self {
        let any_stated =
            !self.pitch_diameter.auto || self.gears.iter().any(|g| !g.helix_angle.auto);
        if any_stated {
            return self.clone();
        }
        Self {
            pitch_diameter: Auto::fixed(self.first_pitch_diameter()),
            ..self.clone()
        }
    }

    /// The crossed-axis geometry this stage describes, at the shifts it
    /// decides — its zero-backlash distance is the stage's.
    ///
    /// # Errors
    ///
    /// As [`Self::screw_at`].
    pub fn geometry(&self) -> Result<Screw, TrainError> {
        self.screw_at(self.chosen_at(&crate::auto::Search::SHIPPED).shifts)
    }

    /// **The shift sum that puts this pair at `target`**, in normal modules —
    /// the parallel mesh's involute relation, or the crossed mesh's rack law
    /// ([`crate::screw::ScrewParams::profile_shifts`]). `None` where no sum
    /// reaches it.
    fn shift_sum_reaching(&self, target: f64) -> Option<f64> {
        if self.is_crossed() {
            // At zero shift, which is the only thing the reference depends on
            // — and asking `geometry()` here would ask the shifts, which is
            // what this is deciding.
            let reference = self.screw_at([0.0; 2]).ok()?.reference_distance;
            let sum = (target - reference) / self.module;
            return sum.is_finite().then_some(sum);
        }
        let rack = crate::plane::BasicRack::new(
            self.module,
            self.pressure_angle,
            self.helix_angles()[0].abs(),
        );
        let sum_z = f64::from(self.gears[0].teeth) + f64::from(self.gears[1].teeth);
        crate::mesh::shift_sum_for(rack.mt, rack.alpha_t, rack.alpha_n, sum_z, target)
    }

    /// The two profile shifts, chosen together where that is what the stage
    /// asked for.
    ///
    /// With the toggle off each gear answers on its own, as it always has: the
    /// manual value, or the least shift that clears undercut. With it on the
    /// pair is chosen at once, because a shift is only good or bad relative to
    /// the one it meshes with — and what a designer has already given is handed
    /// over as pinned rather than overridden.
    // **The tests\' door.** The solve reads `chosen_at`, because it needs
    // to know *how* the shifts were arrived at as well as what they are.
    #[cfg(test)]
    pub(super) fn shifts(&self) -> [f64; 2] {
        self.shifts_at(&crate::auto::Search::SHIPPED)
    }

    /// The **nominal** distance the shifts have to reach, where one was given —
    /// the distance typed less the clearance it is opened by.
    ///
    /// `None` where the stage is not in mode 3: with the distance automatic
    /// there is nothing to reach, and with the *clearance* automatic the
    /// designer is asking what gap their shifts leave rather than for shifts
    /// that make a gap. The same accessor, under the same name, is on every
    /// kind that has a centre distance.
    pub(super) fn nominal_distance(&self) -> Option<f64> {
        self.given_distance()
            .map(|running| MeshKind::External.nominal_of(running, self.clearance.manual))
    }

    /// The **running** distance a designer gave, where mode 3 is on — the
    /// number typed, which the pair is then judged against
    /// (`train::distance_notes`).
    pub(super) fn given_distance(&self) -> Option<f64> {
        (!self.centre_distance.auto && !self.clearance.auto).then_some(self.centre_distance.manual)
    }

    /// As [`Self::shifts`], at a stated search effort — which is what makes
    /// "the shipped effort is converged" a claim something can raise and check
    /// rather than a comment (`auto::Search`).
    /// As [`Self::shifts_at`], **and whether the optimiser actually chose**.
    ///
    /// Two outcomes look identical from the shifts alone: a search that agreed
    /// with the floor, and a search that found nothing admissible and left the
    /// floor alone. `super::Searched` tells them apart, and the solve says the
    /// second out loud.
    pub(super) fn chosen_at(&self, search: &crate::auto::Search) -> super::Chosen<2> {
        let asked = [0, 1].map(|i| self.gears[i].shift_asked(&self.base_params(i)));
        let floor = asked.map(|a| a.search_floor);
        let given = asked.map(|a| a.given);
        // **Mode 3 needs both of them given.** A distance with an *automatic*
        // clearance is the designer asking what gap their shifts leave — mode 2
        // — and solving the shifts from the distance would answer a question
        // they did not ask. So the sum is pinned only when the clearance is a
        // number they stated.
        let sum = self
            .nominal_distance()
            .and_then(|target| self.shift_sum_reaching(target));

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
            return super::Chosen {
                shifts: constrained().unwrap_or_else(|| asked.map(|a| a.settled)),
                how: super::Searched::NotAsked,
            };
        }
        let bounds = crate::auto::Bounds {
            floor,
            min_contact_ratio: self.optimisation.min_contact_ratio,
            // **The gap the designer nominated**, which is a number they
            // stated even where the *reported* clearance is derived from a
            // given distance. It is a guard on the trial mesh — the teeth
            // must not bottom out — so what it wants is the intended gap,
            // not whatever a candidate's shifts happen to leave.
            clearance: self.clearance.manual,
        };
        let pinned = crate::auto::Pinned { shift: given, sum };
        let pair = |x: [f64; 2]| [0, 1].map(|i| self.params_at(i, x[i]));
        // **One search, two meshes.** The floor, the pinning, the box and the
        // descent are the same; what a candidate is worth is the mesh's own
        // question — the loss integral along a line contact, the friction
        // balance along a point's — and the two meet at the parallel limit as
        // the meshes do.
        if self.is_crossed() {
            crate::auto::crossed_shifts_for_efficiency(
                &pair,
                &|x| self.screw_at(x).ok(),
                &bounds,
                &pinned,
                self.sliding_friction,
                search,
            )
        } else {
            crate::auto::shifts_for_efficiency(
                &pair,
                crate::mesh::MeshKind::External,
                &bounds,
                &pinned,
                self.sliding_friction,
                search,
            )
        }
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
        .map_or_else(
            // Nothing admissible. The constraints still stand — see above — but
            // the *objective* found no answer, and that is worth saying: it is
            // otherwise indistinguishable from a search that agreed.
            || super::Chosen {
                shifts: constrained().unwrap_or_else(|| asked.map(|a| a.settled)),
                how: super::Searched::FoundNothing,
            },
            |shifts| super::Chosen {
                shifts,
                how: super::Searched::Chose,
            },
        )
    }

    /// As [`Self::shifts`], at a stated search effort — which is what makes
    /// "the shipped effort is converged" a claim something can raise and check
    /// rather than a comment (`auto::Search`).
    #[cfg(test)]
    pub(super) fn shifts_at(&self, search: &crate::auto::Search) -> [f64; 2] {
        self.chosen_at(search).shifts
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

/// The pair presets' old entry point, kept for the tests written against
/// it: a pair through the shape.
#[cfg(test)]
pub(crate) fn solve_pair_stage(
    stage: &PairStage,
    kind: PairKind,
    loads: &super::StageLoads,
    lib: &crate::material::MaterialLibrary,
) -> Result<super::shape::ShapeResult, TrainError> {
    super::shape::solve_shape(
        &super::shape::Shape::from_pair(stage, kind),
        loads,
        lib,
        super::Reversal::default(),
    )
}
