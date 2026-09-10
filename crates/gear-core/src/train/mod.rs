//! Geartrains: a stage at a time, and the accumulation along the shaft line.
//!
//! Each stage kind has its own module — `spur` and `worm` — and what stays here
//! is the vocabulary they share ([`Backlash`], [`TrainError`], the duty cycle)
//! and the train that strings them together.
//!
//! **Each kind keeps its own result type**, and that was a decision rather than
//! an oversight. A worm stage has no bending stress, no minimum face width from
//! contact — a point contact does not care how wide the tooth is — and two
//! efficiencies rather than one. Forcing that into [`StageResult`] would have
//! meant four `Option`s and a comment apologising for each. A result shaped like
//! the answer says the same thing without the apology.
//!
//! # What is state and what is not
//!
//! Per `docs/rationale.md#inputs-are-the-only-state` the input structs here are the *only* state. Every
//! result is recomputed from them, so nothing can go stale. Two consequences are
//! visible in the shapes below:
//!
//! - Values shared across a stage — normal module, pressure angle, helix angle —
//!   are stored **once on the stage**, not per gear, so the two cannot disagree
//!   (docs/rationale.md#inputs-are-the-only-state).
//! - Tooth thickness modification is stored as `k₁` alone, with `k₂ = 2 − k₁`
//!   derived, because a meshing pair must sum to 2. The invariant is unwritable
//!   rather than merely tested.

use crate::auto::{addendum_for_tip_width, automatic_profile_shift, Ranges};
use crate::contact::{Directional, Drive};
use crate::material::{Material, MaterialLibrary, Overrides};
use crate::mesh::MeshError;
use crate::note::{key, Note};
use crate::params::{Auto, GearParams};
use crate::tooth::Tooth;

mod hula;
mod planetary;
mod spur;
mod worm;

pub use hula::{
    solve_hula_stage, solve_hula_stage_with, stage_efficiency, HulaGear, HulaMesh, HulaResult,
    HulaStage,
};
pub use planetary::{
    solve_planetary_stage, solve_planetary_stage_with, PlanetResult, PlanetaryResult,
    PlanetaryStage,
};
pub use spur::{solve_spur_stage, solve_spur_stage_with, SpurStage};
pub(crate) use spur::{undercut_bound, Decided, ShiftAsked};
pub use worm::{
    solve_crossed_stage, solve_worm_stage, FirstMemberSizing, WormContact, WormMember,
    WormMemberResult, WormResult, WormStage,
};

/// The three contact ratios.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ContactRatios {
    /// Transverse, `ε_α` — profile overlap.
    pub transverse: f64,
    /// Overlap, `ε_β = b sin β / (π m_n)` — axial overlap. Exactly zero for a
    /// spur stage.
    pub overlap: f64,
    /// Total, `ε_γ = ε_α + ε_β`.
    pub total: f64,
}

impl ContactRatios {
    /// The three, from the transverse one and the mesh they belong to.
    ///
    /// `ε_β = b sin β / (π m_n)` and `ε_γ = ε_α + ε_β` — one line each, but
    /// written out once per stage kind they were three copies of the same two
    /// lines, and a fourth stage away from being four. The width is the mesh's
    /// **effective** one, the narrower of the two members, because that is the
    /// width that carries the pair.
    #[must_use]
    pub fn of(transverse: f64, width: f64, helix_angle: f64, normal_module: f64) -> Self {
        let overlap =
            width * helix_angle.to_radians().sin().abs() / (std::f64::consts::PI * normal_module);
        Self {
            transverse,
            overlap,
            total: transverse + overlap,
        }
    }

    /// Whether at least one contact line is engaged at all times.
    ///
    /// Below this a gear is helical in form but still transfers load like a spur
    /// gear — abrupt engagement, no smoothing — which is usually not what the
    /// helix angle was chosen for, and is invisible without the check.
    #[must_use]
    pub fn has_full_axial_overlap(&self) -> bool {
        self.overlap >= 1.0
    }
}

/// Angular backlash at one gear, in degrees.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Backlash {
    pub nominal: f64,
    pub minimum: f64,
    pub maximum: f64,
}

impl Backlash {
    /// The band a centre-distance tolerance opens around a nominal distance.
    ///
    /// **One construction, because the sign convention is one claim.** A
    /// *smaller* centre distance leaves less room and so less play, and a larger
    /// one more — so `minus` gives the minimum and `plus` the maximum, and
    /// getting that backwards produces a band that reads perfectly well and is
    /// inside out.
    ///
    /// It was written out four times, once per stage kind, each closing over its
    /// own way of turning a distance into an angle. That is the part that
    /// genuinely differs — a parallel mesh, a screw pair and a crank each reach
    /// it differently — so it is the argument, and the three lines around it are
    /// not.
    pub fn banded(nominal: f64, minus: f64, plus: f64, angular: impl Fn(f64) -> f64) -> Self {
        Self {
            nominal: angular(nominal),
            minimum: angular(nominal - minus),
            maximum: angular(nominal + plus),
        }
    }
}

/// **What one parallel-axis mesh reports**, for any stage kind that has more
/// than one of them.
///
/// A stage with a single mesh puts these on its own result, because there is no
/// ambiguity about whose they are; a stage with two has to say which mesh each
/// belongs to, and both of them were saying it in the same six fields. The
/// planetary set's `sun_planet`/`planet_ring` and the hula stage's two pairs are
/// the same report, so it is one type.
///
/// A crossed pair has none of this — its line of action slides rather than
/// turning, so there is no operating pressure angle and no contact ratio to
/// report (docs/reference.md#crossed-axes).
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct MeshReport {
    /// Operating pressure angle `α_w`, degrees — see
    /// [`SpurResult::operating_pressure_angle`], which defines it for every
    /// parallel-axis mesh here.
    pub operating_pressure_angle: f64,
    /// Whether the two members' tooth counts share no factor — a hunting pair,
    /// which spreads wear evenly instead of repeatedly bringing the same two
    /// teeth together.
    ///
    /// A property of a *mesh*, which is why it is here: a pair reports one
    /// ([`SpurResult::coprime`], its one mesh being the stage), and a set with
    /// two meshes has two answers. An epicyclic set's separate question — each
    /// central member against the *planet count* — is a different check with a
    /// different reason, and it stays where it is.
    pub coprime: bool,
    pub contact_ratios: ContactRatios,
    /// Mesh efficiency, both drive senses. Equal for a parallel-axis pair, and
    /// arrived at rather than copied.
    pub efficiency: Directional<f64>,
    /// Hertzian contact stress at the pitch point, MPa, in both load cases.
    ///
    /// The one figure both members of the mesh share. Each member's own rating —
    /// taken where its dedendum is loaded alone — sits on its [`GearResult`].
    pub contact_stress_at_pitch_point: LoadCase<f64>,
    /// Relative radius of curvature at the governing point, mm.
    pub relative_radius: f64,
    /// Angular backlash at each member, degrees, in the order the mesh was
    /// built: the pinion-side member first, then the other.
    pub backlash: [Backlash; 2],
    /// **Where an internal mesh's tips are**, and `None` for an external one,
    /// which has no such question.
    pub tips: Option<TipRoom>,
}

/// **The three ways an internal mesh's teeth can foul**, and the room the third
/// leaves.
///
/// An external pair has none of them: its two members curve opposite ways, so a
/// tip meets a flank where the teeth mesh and nowhere else, and the one question
/// left is whether the tips bottom out — which is a radial comparison every
/// mesh already makes ([`crate::mesh::Mesh::bottom_clearance`]). An internal
/// pair's members curve the *same* way, and that opens three more:
///
/// - the pinion's tip reaching past where the ring's flank ends, into the fillet
///   its shaper left (**trochoid**);
/// - the ring's tip reaching below where the pinion's flank ends, or not
///   reaching its involute at all (**involute**) — the one a **full-depth**
///   internal pair fails as a matter of course, which is why internal gears are
///   not built full-depth (`crate::ring::mesh_with`);
/// - and the tips fouling **away from the line of action** entirely (**tip**),
///   which the first two cannot see: they ask what happens where the teeth mesh,
///   and this asks whether two teeth try to occupy the same place somewhere
///   else. It is what decides a small tooth difference, where the tip circles
///   cross far from the line of centres and the mesh itself is perfectly
///   conjugate.
///
/// # Why it is on the mesh report
///
/// It was a hula stage's, in four fields of its own, and the epicyclic set with
/// the same internal mesh in it reported nothing — so a designer was told
/// whether the teeth foul or not according to which stage kind they had picked.
/// The set's shipped proportions fail the involute question and had never said
/// so. *A constraint belongs to the mesh*, and so does what it found: a ring's
/// tip is the mesh's business wherever the ring is.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TipRoom {
    /// The pinion's tip reaches past where the ring's flank ends.
    pub trochoid_interference: bool,
    /// The ring's tip reaches below where the pinion's flank ends.
    pub involute_interference: bool,
    /// The tips foul away from the line of action.
    pub tip_interference: bool,
    /// How much room the tips have where their circles cross, as an angle of
    /// **pinion** rotation, degrees. Negative is the overlap, and infinite where
    /// the tip circles do not cross at all — the ordinary case, where there is
    /// no place for the tips to meet.
    pub tip_margin: f64,
}

impl TipRoom {
    /// Read off the pair, at the centre distance its own shifts give it.
    ///
    /// `None` where the two cannot be meshed at all, which the caller already
    /// knows by other means — every caller here has built the mesh.
    pub(crate) fn of(ring: &crate::ring::Ring, pinion: &crate::Tooth) -> Option<Self> {
        crate::ring::mesh_with(ring, pinion).map(|m| Self {
            trochoid_interference: m.trochoid_interference,
            involute_interference: m.involute_interference,
            tip_interference: m.tip_interference,
            tip_margin: m.tip_margin.to_degrees(),
        })
    }

    /// Whether anything fouls — the question a search asks, as against the four
    /// numbers a reader is given.
    #[must_use]
    pub fn clear(&self) -> bool {
        !self.trochoid_interference && !self.involute_interference && !self.tip_interference
    }
}

impl MeshReport {
    /// The one gap, read by **drive direction** rather than by member.
    ///
    /// `backlash` is per member — "the one gap seen from each of its ends" — and
    /// a *stage* reports it per direction, because the output of a forward drive
    /// is the second member and of a backward drive the first. Two indexings of
    /// two numbers, and this is the conversion, in one place: it was written out
    /// in the spur stage as a `match` inside a `Directional::of`, which is the
    /// same mapping stated a second time.
    #[must_use]
    pub fn backlash_by_drive(&self) -> Directional<Backlash> {
        Directional {
            forward: self.backlash[1],
            backward: self.backlash[0],
        }
    }
}

/// **A tooth the cutter has eaten into**, where that is a finding rather than a
/// clamp.
///
/// Severing truncates the profile, so `Tooth` records it as a clamp and every
/// stage's member list has carried it. Undercut short of severing alters
/// nothing — the tooth is exactly the one the inputs describe — which is why it
/// is not a clamp, and why nothing was reporting it: a gear tab has shown it
/// since undercut existed, and the same gear inside a geartrain said nothing at
/// all. So it goes where a remark about a member goes, beside the notch band
/// and the reversed root.
///
/// It matters most exactly where a stage cannot prevent it. `no undercut` bounds
/// a shift somebody chooses; a shift that a *relation* leaves over answers to no
/// bound at all — a hula pinion whose ring was pinned, an epicyclic absorber —
/// so the control can be on, the tooth undercut, and the two never meet.
///
/// Nothing for a ring: its flank is its shaper's, and undercut is not a question
/// that can be asked of it.
pub(crate) fn undercut_note(tooth: &crate::tooth::Tooth) -> Option<Note> {
    (tooth.undercut && !tooth.severed).then(|| Note::new(key::CLAMP_TOOTH_UNDERCUT))
}

/// The face width a pair of ratings asks for, at one load case.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Widths {
    /// From bending. `None` where the section has no rating.
    pub bending: Option<f64>,
    /// From contact. `None` where no contact *rating* sizes this member's face.
    ///
    /// Optional for the same reason `bending` is, and it took a crossed pair to
    /// make the case reachable: inverting a stress for a width assumes the
    /// stress depends on the width, and a **point** contact's peak pressure does
    /// not depend on it at all. A crossed pair's automatic width comes from
    /// continuity instead — `ε = 1`, a *geometric* minimum, reported as its own
    /// figure and labelled as the other kind of answer because the two differ by
    /// 2.4× (`docs/state.md`). Putting that number here would be the mixing this
    /// project refuses; putting a zero here would be a width nobody needs, which
    /// `docs/corrections.md` has already been caught by once.
    pub contact: Option<f64>,
}

/// The face width every rating is first evaluated at, mm.
///
/// Any width will do, and that is the point: `b_min` does not depend on the `b`
/// it was measured at (docs/reference.md#contact-stress), so one evaluation
/// gives every minimum and nothing has to be iterated to find the width a
/// member needs before it is given one.
pub(crate) const PROBE: f64 = 10.0;

/// **What one member bends at in one mesh**: the critical section, the share of
/// the mesh load acting on it, and what the sharing model has to say about
/// being asked outside the band it was described in.
///
/// All three are per *(member, mesh)*, which is why they travel together and
/// why this is one place rather than one per stage kind. It had been none: the
/// pair asked for the section and the share inline and raised the note itself,
/// and the two epicyclic kinds asked for neither — so the one estimate this
/// crate ships reached one stage of three, and a set that switched the model on
/// got it on the members it happened to share code with.
pub(crate) struct Bending {
    pub section: crate::strength::RootSection,
    /// Exactly 1 where no sharing model was asked for, so the ordinary rating
    /// is untouched to the bit.
    pub share: f64,
    /// **`Y_B`** — the one factor of `σ_F0` that is the member's *blank's*
    /// rather than its root section's. `None` where nobody described a rim,
    /// which is every gear this crate rated before it existed.
    pub rim: Option<crate::strength::RimSupport>,
    /// **The ramp outside the band it was described in.** It is a first-order
    /// stand-in for a mesh with a single-pair zone; at `ε_n ≥ 2` there is no
    /// such zone and the ramp never reaches a full share. **What that does to
    /// the figure has no fixed direction**: measured across high-contact-ratio
    /// spur designs it runs from a 24 % relief to a 15 % *increase*, because
    /// the swept maximum is a product of a form factor rising toward the tip
    /// and a share falling away there, and which wins is the tooth's business.
    /// A large number either way from an uncalibrated model — exactly what
    /// `docs/rationale.md` refuses to let pass silently — so it is said where
    /// the figure is shown. The model is still the one the designer asked for;
    /// what they are owed is knowing it is extrapolating.
    ///
    /// A *mesh's* finding, so a stage raises it once per mesh rather than once
    /// per member; a member's own findings — its notch band, its rim — go in
    /// its own list.
    pub note: Option<Note>,
}

impl Bending {
    /// **For any member that bends**, rack-cut or shaper-cut.
    ///
    /// `rim` is the thickness of the rim under its teeth, mm, or `None` where
    /// nobody said — see [`crate::strength::RimSupport`]. Which reference that
    /// thickness is measured against is the member's own business
    /// ([`ToothOutline::rim_support`]), as the direction its load point travels
    /// is; this was two functions and they differed in nothing else.
    pub(crate) fn of<T: crate::strength::ToothOutline>(
        member: &T,
        contact_ratio: f64,
        model: crate::contact::LoadSharing,
        rim: Option<f64>,
    ) -> Option<Self> {
        let (section, share) =
            crate::strength::bending_section_shared(member, contact_ratio, model)?;
        let cos_bb = member.base_helix_angle().cos();
        Some(Self::new(
            section,
            share,
            model,
            contact_ratio / (cos_bb * cos_bb),
            rim.map(|s| member.rim_support(s)),
        ))
    }

    fn new(
        section: crate::strength::RootSection,
        share: f64,
        model: crate::contact::LoadSharing,
        eps_n: f64,
        rim: Option<crate::strength::RimSupport>,
    ) -> Self {
        let out_of_band = !matches!(model, crate::contact::LoadSharing::None) && eps_n >= 2.0;
        Self {
            section,
            share,
            rim,
            note: out_of_band
                .then(|| Note::new(key::STAGE_LOAD_SHARING_OUT_OF_BAND).number("ratio", eps_n, 3)),
        }
    }
}

/// The note a rating raises about one member's **rim**: it is thinner than the
/// clause will rate.
///
/// ISO 6336-3:2019, 9.3 says a backup ratio at or below 0,5 (external) or a rim
/// below 1,75 normal modules (internal) "shall be avoided" rather than giving a
/// value for it. `Y_B` still evaluates there and this crate still reports it —
/// the fit is a logarithm and does not fall over — but a design that far in is
/// past where the standard will go, and it says so instead of returning a
/// number that looks like the others.
///
/// A **member's** finding rather than a mesh's, which is why it is here and not
/// in [`Bending::note`]: two members of one mesh have two rims and one sharing
/// model between them.
pub(crate) fn rim_below_minimum(rim: Option<crate::strength::RimSupport>) -> Option<Note> {
    rim.filter(|r| !r.in_range())
        .map(|r| Note::new(key::STAGE_RIM_BELOW_MINIMUM).number("ratio", r.ratio(), 2))
}

/// **What one mesh does to one member**: the two stresses it produces there,
/// and the widths they belong to.
///
/// A member is not always in one mesh. A planet is in two, and a stage kind
/// nobody has written yet may put a member in more — so a rating is taken over
/// *however many there are* rather than over a named pair, and adding a mesh to
/// a member is adding an entry to a list rather than an arm to an expression.
///
/// Both widths are here because they are genuinely two questions. A mesh
/// carries a member at the narrower of its two members' faces, which differs
/// from mesh to mesh; and the stresses may have been evaluated somewhere else
/// entirely — at [`PROBE`], before any width was settled.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Loading {
    /// Root bending stress this mesh produces on the member, MPa at
    /// [`Self::measured_at`], under the torque the stage solved at. `None`
    /// where the section has no rating — a ring with no fillet is the ordinary
    /// way to get here.
    pub bending: Option<f64>,
    /// The member's **governing** contact stress in this mesh, MPa at
    /// [`Self::measured_at`]: where its own dedendum is loaded alone.
    pub contact: f64,
    /// The face width both figures were evaluated at, mm.
    pub measured_at: f64,
    /// The width this mesh actually carries the member at, mm.
    ///
    /// Equal to [`Self::measured_at`] in two quite different cases, and it is
    /// worth knowing which: during the probe pass, when no width has been
    /// settled and none is needed ([`MemberRating::asks`]); and for a stage
    /// that evaluated its stresses at the width it ended with, where there is
    /// nothing to scale and the figures are the ones its own arithmetic
    /// produced, bit for bit.
    pub carried_at: f64,
}

impl Loading {
    /// The two stresses as they stand at [`Self::carried_at`].
    ///
    /// Bending is inversely linear in width and contact goes as the inverse
    /// square root of it, so a change of width is a scale rather than a second
    /// solve — and where the two widths are equal this is the identity, which
    /// is what lets a stage that evaluated at its final width keep its own
    /// digits to the bit.
    fn at_width(self) -> (Option<f64>, f64) {
        let by = self.measured_at / self.carried_at;
        (self.bending.map(|s| s * by), self.contact * by.sqrt())
    }

    /// **The same loading under `k` times the torque.**
    ///
    /// Bending is linear in torque and contact goes as its square root
    /// (docs/reference.md#load-cases), so where a stage's power split does not
    /// depend on the *magnitude* of what passes through it — which is every
    /// kind here, and is a fact about each kind rather than about gearing — the
    /// second load case is this rather than a second solve. A stage whose flow
    /// does not have that property builds each case's loadings itself, which is
    /// why they are held per case rather than as one list and a factor.
    fn under(self, k: f64) -> Self {
        Self {
            bending: self.bending.map(|s| s * k),
            contact: self.contact * k.sqrt(),
            ..self
        }
    }

    /// Both load cases, for a stage whose ratings scale with the torque.
    ///
    /// **The scale is the mesh's, not the stage's.** Each loading is evaluated
    /// at the torque its own mesh carries and carries the fraction of that the
    /// other case is ([`LoadCase::as_fraction_of_peak`]) — because which
    /// direction loads a mesh hardest is a fact about that mesh, and a set's two
    /// meshes need not agree about it (`StageTorques::on_mesh`).
    pub(crate) fn both_cases(loadings: &[(Self, LoadCase<f64>)]) -> LoadCase<Vec<Self>> {
        LoadCase::of(|case| {
            loadings
                .iter()
                .map(|(l, scale)| l.under(*scale.get(case)))
                .collect()
        })
    }
}

/// **What one member's ratings come to**, over every mesh it is in.
///
/// Every stage kind asks the same four questions of every member it builds —
/// two stresses, each against two load cases, and the width each of those would
/// need — and each of them had been writing the arithmetic out for itself. What
/// genuinely differs between kinds is what the meshes do to the member and
/// which allowable a reversed root answers to, and both arrive here as values.
///
/// **The worst mesh wins, figure by figure.** A planet's root is loaded by the
/// sun on one flank and the ring on the other, at different tangential forces
/// through different sections, and its flanks pit at whichever end of whichever
/// path is worst — so neither figure is the sun mesh's by right. Taking the
/// worse of the two was written out for contact and simply omitted for bending;
/// here it is one fold over a list, which is the same answer for two meshes and
/// an answer at all for three.
pub(crate) struct MemberRating<'a> {
    /// The material as used, after any overrides.
    pub material: &'a Material,
    /// How the train treats a root loaded on both flanks.
    pub reversal: Reversal,
    /// ...and whether this member's is.
    pub reverses: bool,
    /// **Every mesh this member is in, in each load case.** One for an ordinary
    /// gear, two for a planet, and a list rather than a pair so that neither is
    /// the special case.
    ///
    /// Per case rather than one list and a factor, because "the second case is
    /// the first times a number" is a claim about a *stage's power flow* rather
    /// than about gearing. It holds for every kind here and
    /// [`Loading::both_cases`] is how they say so; a kind whose flow does not
    /// scale with what passes through it builds each case for itself, and needs
    /// nothing added here to do it.
    pub loadings: LoadCase<Vec<Loading>>,
}

/// The three rating fields of a [`GearResult`], over every mesh a member is in.
pub(crate) struct Rated {
    pub bending_stress: LoadCase<Option<f64>>,
    pub contact_stress: LoadCase<f64>,
    pub min_face_width: LoadCase<Widths>,
}

impl MemberRating<'_> {
    /// The rating: each mesh at the width it carries the member at, and the
    /// worst of them.
    pub(crate) fn rated(&self) -> Rated {
        // A bending stress that no mesh could rate stays absent; one that any
        // mesh could rate is that mesh's worst, and a mesh with no rating does
        // not make an absence out of a figure another mesh has.
        let worst = |case: Case| -> (Option<f64>, f64, f64) {
            let mut bending: Option<f64> = None;
            let mut contact = 0.0_f64;
            let mut width = 0.0_f64;
            for l in self.loadings.get(case) {
                let (b, c) = l.at_width();
                if let Some(b) = b {
                    bending = Some(bending.map_or(b, |had: f64| had.max(b)));
                }
                if c >= contact {
                    contact = c;
                }
                width = width.max(l.carried_at);
            }
            (bending, contact, width)
        };
        Rated {
            bending_stress: LoadCase::of(|c| worst(c).0),
            contact_stress: LoadCase::of(|c| worst(c).1),
            min_face_width: LoadCase::of(|c| {
                // **Each figure is inverted at the width it was taken at**, and
                // the widest mesh is the one that answers: a minimum is what
                // this member would need, and it needs enough for every mesh it
                // is in.
                let (bending, contact, width) = worst(c);
                Widths {
                    // Two allowables, because a reversed root endures less
                    // bending while its flank pits exactly as it did.
                    bending: bending.map(|s| {
                        crate::strength::min_face_width_bending(
                            s,
                            width,
                            self.reversal
                                .bending_allowable(self.material, c, self.reverses),
                        )
                    }),
                    contact: Some(crate::strength::min_face_width_contact(
                        contact,
                        width,
                        allowable(self.material, c),
                    )),
                }
            }),
        }
    }

    /// The four widths this member's ratings ask for.
    ///
    /// Takes no width, because the answer does not depend on one: a minimum
    /// width is a stress inverted, and the stress it inverts scales with the
    /// width it was measured at by exactly the amount that cancels
    /// (docs/reference.md#contact-stress). So a stage can size a member from a
    /// probe pass, before it has a width to size it at.
    pub(crate) fn asks(&self) -> LoadCase<Widths> {
        self.rated().min_face_width
    }
}

/// **What a gear's addendum comes to**, once its bound has had its say.
///
/// The same shape as [`ShiftAsked`] and for the same reason: two stages were
/// working it out for themselves, in the same four lines, and the answer has
/// two halves — the number to build with, and whether it is the number that was
/// asked for.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AddendumAsked {
    /// The coefficient to build with, in modules.
    pub used: f64,
    /// Whether the bound cut it down, and so whether the tooth is the one that
    /// was asked for.
    pub clamped: bool,
}
impl AddendumAsked {
    /// The note a clamped addendum owes its reader, if it was clamped.
    pub(crate) fn note(&self, teeth: u32) -> Option<crate::note::Note> {
        self.clamped.then(|| {
            crate::note::Note::new(crate::note::key::STAGE_ADDENDUM_HELD_TO_TIP_WIDTH)
                .count("teeth", teeth)
                .number("addendum", self.used, 4)
        })
    }

    /// **The same finding where the stage cannot act on it**, which is a report
    /// rather than a clamp.
    ///
    /// A hula stage solves its crank offset from a gap written in the
    /// tips, in closed form with an analytic derivative. An addendum that moved
    /// with the shift — which moves with the offset — would put a tip-width
    /// solve inside that root-find and take the derivative away with it. Not
    /// every bound an input creates needs a solver behind it: this one says
    /// what the tooth would have to be and leaves the number alone.
    pub(crate) fn warning(&self, teeth: u32) -> Option<crate::note::Note> {
        self.clamped.then(|| {
            crate::note::Note::new(crate::note::key::STAGE_ADDENDUM_ABOVE_TIP_WIDTH)
                .count("teeth", teeth)
                .number("addendum", self.used, 4)
        })
    }
}

/// A shift clears undercut unless it is told not to — and a document written
/// before the question was asked separately meant exactly that.
#[cfg(feature = "serde")]
const fn yes() -> bool {
    true
}
/// A coefficient that used to be an `Auto` and is now a number.
///
/// Read either shape: a document written before the addendum's bound was split
/// from its value holds `{ auto, manual }`, and the number it meant is the
/// `manual` one — with `auto` on, it meant "and hold it to the tip width",
/// which [`StageGear::no_sharp_tip`] says now and defaults to.
#[cfg(feature = "serde")]
fn coefficient<'de, D: serde::Deserializer<'de>>(d: D) -> Result<f64, D::Error> {
    #[derive(serde::Deserialize)]
    #[serde(untagged)]
    enum Either {
        Number(f64),
        WasAuto { manual: f64 },
    }
    Ok(match <Either as serde::Deserialize>::deserialize(d)? {
        Either::Number(v) | Either::WasAuto { manual: v } => v,
    })
}

// -------------------------------------------------- the shared member ---
//
// `StageGear` is what *every* stage kind describes a member with, so it lives
// here with the rest of the shared vocabulary rather than in the kind that
// happened to need it first. It was declared in `spur.rs` and re-exported from
// this module, which read as though the parallel-axis stage owned it — and this
// module's own comment says it holds "what every stage kind shares".

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
    /// Addendum coefficient, in modules, as asked for.
    ///
    /// Plain, because the only thing an automatic addendum ever computed was
    /// the tallest tooth that keeps a tip [`Self::min_tip_width`] wide — which
    /// is a **bound on the number**, not a source for it, and now says so.
    #[cfg_attr(feature = "serde", serde(deserialize_with = "coefficient"))]
    pub addendum: f64,
    /// **The tooth may not be taller than its tip is wide.**
    ///
    /// The same shape as [`Self::no_undercut`], on the other end of the tooth:
    /// a constraint the number answers to however it arrived, rather than a
    /// mode the number is only read in. It was the latter, and so
    /// `min_tip_width` went unread on every addendum a designer typed — a
    /// tooth could come to a point and nothing said so.
    ///
    /// Off, the addendum stands as asked and the tip is whatever it is.
    #[cfg_attr(feature = "serde", serde(default = "yes"))]
    pub no_sharp_tip: bool,
    /// Minimum transverse tooth tip width, mm.
    pub min_tip_width: f64,
    pub dedendum: f64,
    pub root_radius: f64,
    /// Automatic takes the larger of the enabled minimums below.
    pub face_width: Auto<f64>,
    /// Which of the four ratings an automatic face width is sized from.
    pub face_sources: FaceSources,
    /// **Thickness of the rim under this member's teeth, mm** — `s_R`, and with
    /// it the rim thickness factor `Y_B` (ISO 6336-3:2019, Clause 9).
    ///
    /// `None` is the default and means *nobody said*, which is not the same
    /// claim as a thick rim even though both rate at `Y_B = 1`: a gear whose rim
    /// was never described cannot be told it is too thin, and one that was can.
    /// Given, it de-rates the root — a thin rim moves the failure out of the
    /// fillet and through the rim — and never relieves it.
    ///
    /// Measured against the whole tooth depth on a rack-cut member and against
    /// the normal module on a ring, which is the clause's own distinction and
    /// the only one: see [`RimSupport`](crate::strength::RimSupport).
    ///
    /// It reaches bending alone. A rim under the teeth has nothing to do with
    /// the pressure between two flanks, so no contact rating reads it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub rim_thickness: Option<f64>,
    /// Name of a material in the library.
    pub material: String,
    /// Properties replaced for this gear only. Empty means "as the library
    /// says" — see [`Overrides`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub material_overrides: Overrides,
}

impl StageGear {
    /// **The addendum this gear builds at**, at a given shift.
    ///
    /// The bound is an upper one — a taller tooth is a sharper tooth — so the
    /// answer is the smaller of what was asked and what the tip width allows.
    /// It depends on the shift, which is why it is asked per built tooth rather
    /// than once per gear.
    ///
    /// A tooth already thinner than `min_tip_width` at its base circle has no
    /// admissible addendum at all ([`addendum_for_tip_width`] returns `None`);
    /// there is nothing to clamp to, so the number stands and the tooth's own
    /// `pointed` reporting is what says it is wrong.
    pub(crate) fn addendum_asked(&self, at_shift: &crate::params::GearParams) -> AddendumAsked {
        let asked = self.addendum;
        if !self.no_sharp_tip {
            return AddendumAsked {
                used: asked,
                clamped: false,
            };
        }
        let ceiling = addendum_for_tip_width(
            &Tooth::new(GearParams {
                addendum: asked,
                ..*at_shift
            }),
            self.min_tip_width,
        );
        let used = ceiling.map_or(asked, |c| asked.min(c));
        AddendumAsked {
            used,
            clamped: used < asked - 1e-12,
        }
    }

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
            addendum: 1.0,
            no_sharp_tip: true,
            min_tip_width: 0.1,
            dedendum: 1.25,
            root_radius: 0.38,
            face_width: Auto::fixed(10.0),
            face_sources: FaceSources::default(),
            // A rim nobody described: `Y_B` is 1, and the gear cannot be told
            // its rim is thin because it has not said what its rim is.
            rim_thickness: None,
            material: "4340 Hardened Steel".to_string(),
            material_overrides: Overrides::default(),
        }
    }
}

/// What a stage does to one of its gears.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct GearResult {
    /// The shift in force, after any automatic calculation.
    pub profile_shift: f64,
    /// Likewise the addendum.
    pub addendum: f64,
    /// Likewise the face width.
    pub face_width: f64,
    /// Torque on this gear, N·m, driving forward at peak.
    pub torque: f64,
    /// Torque on this gear from a back-driving load, N·m.
    ///
    /// `None` where none is reacted here — which is every gear of a train that
    /// can be back-driven at all, and every gear upstream of whatever stage
    /// stops one that cannot.
    pub back_driving_torque: Option<f64>,
    /// Rotational speed, rpm.
    pub speed: f64,
    /// Tooth load cycles over the duty the train describes.
    ///
    /// One cycle per revolution for a simple gear: a given tooth meets the mate
    /// once per turn. Sun and ring gears in a planetary stage see `N_planets`
    /// per revolution, and a planet is a special case again — docs/reference.md#trains.
    pub tooth_cycles: Cycles,
    /// Tooth root bending stress, MPa, for each load case. `None` where the
    /// stress correction is undefined for this section — see
    /// [`crate::strength::bending_stress`] — or where the case carries no load.
    pub bending_stress: LoadCase<Option<f64>>,
    /// Hertzian contact stress, MPa, for each load case — at **this gear's**
    /// governing point.
    ///
    /// The two gears of a mesh share one patch and one pressure at any instant,
    /// so this is not a per-tooth curvature effect. They differ because they are
    /// rated at different *moments*: each gear's dedendum is loaded alone at one
    /// end of the path, and that is where its own pitting is assessed. See
    /// [`crate::strength::ContactStress::governing`].
    pub contact_stress: LoadCase<f64>,
    /// The face width each rating would need: two per case, four in all.
    pub min_face_width: LoadCase<Widths>,
    /// Guards that altered this gear's geometry.
    pub clamps: Vec<crate::note::Note>,
    /// What the **rating** has to say about this gear, as against what was done
    /// to its geometry.
    ///
    /// Kept apart from [`Self::clamps`] because nothing here moved a dimension:
    /// a root loaded on both flanks and a notch outside the `Y_S` fit's band are
    /// statements about how the number was arrived at, not about the part.
    ///
    /// **Per gear rather than per stage**, and that is not filing. A stage note
    /// naming a member has to carry the member's name in its own text, and two
    /// members raising the same note give one list two entries with one key —
    /// which a keyed list in the front end cannot render (`docs/corrections.md`).
    /// A note that belongs to a gear belongs *on* the gear.
    #[cfg_attr(feature = "serde", serde(default))]
    pub notes: Vec<crate::note::Note>,
    /// The material as used, after any overrides — what the numbers were
    /// actually computed from, rather than what the library holds.
    pub material: Material,
    /// What this gear's geometry allows its own inputs to be.
    ///
    /// Computed from the **resolved** parameters, so an automatic profile shift
    /// or addendum is already folded in. The UI bounds its fields by these
    /// rather than by constants — see `docs/reference.md#input-ranges`.
    pub ranges: Ranges,
}

/// The facts a stage has about one of its members, gathered so that assembling
/// a [`GearResult`] is one expression rather than one per stage kind.
///
/// # Why this exists
///
/// Three kinds built a `GearResult` field by field, listing the same seventeen
/// names each time. Sixteen agreed. The seventeenth did not: `back_driving_torque`
/// was the mesh projection of the backward torque in the spur stage and this
/// gear's forward torque scaled by `|t| / |forward|` in the other two — the same
/// number wherever the projection is linear, which it is, **except in sign**.
/// A stage with a negative ratio gave a signed figure from one kind and a
/// magnitude from another, for the field beside `torque`, which is signed.
///
/// That is the fault `docs/corrections.md` opens with: a duplicated formula is a
/// place where two answers can differ, and nothing compared these two.
pub(crate) struct MemberFacts<'a> {
    /// The shift in force — which for a hula gear is the layout's, not the
    /// parameters', so it is given rather than read.
    pub profile_shift: f64,
    pub params: &'a GearParams,
    pub input: &'a StageGear,
    pub rated: Rated,
    /// The width this member is *rated at*, which is its mesh's rather than its
    /// own ([`Widths`]).
    pub face_width: f64,
    /// Driving forward, at peak.
    pub torque: f64,
    /// Filled here where the stage knows it, and by [`TrainResult`] where the
    /// shaft line does. A planet's is neither its carrier's nor its sun's.
    pub speed: f64,
    /// This member's share of a back-driving load, if one is reacted here.
    ///
    /// **Given rather than derived**, and that is the finding. Three kinds
    /// referred the load by scaling this member's *forward* torque
    /// ([`StageTorques::referred_like`]), which is exact wherever the forward
    /// torque is a geometric projection or the two directional efficiencies
    /// agree — true of every parallel-axis kind. A worm stage is neither: its
    /// wheel's forward torque carries a forward efficiency of 62 % that a
    /// backward load does not share, and its backward efficiency is zero. So it
    /// supplies its own, and the constructor holds no formula that a kind could
    /// need to disagree with.
    pub back_driving_torque: Option<f64>,
    pub material: Material,
    pub clamps: Vec<Note>,
    pub notes: Vec<Note>,
}

impl GearResult {
    /// One member's result, from what the stage knows about it.
    ///
    /// Every stage kind comes through here, so a field cannot be filled two ways
    /// — see [`MemberFacts`] for the one that was.
    pub(crate) fn of(f: MemberFacts) -> Self {
        Self {
            profile_shift: f.profile_shift,
            addendum: f.params.addendum,
            face_width: f.face_width,
            torque: f.torque,
            back_driving_torque: f.back_driving_torque,
            speed: f.speed,
            // Filled by `set_kinematics`, which is the only level that knows the
            // duty cycle and where this gear sits in the shaft line.
            tooth_cycles: Cycles::default(),
            bending_stress: f.rated.bending_stress,
            contact_stress: f.rated.contact_stress,
            min_face_width: f.rated.min_face_width,
            clamps: f.clamps,
            notes: f.notes,
            material: f.material,
            ranges: crate::auto::admissible_ranges(
                f.params,
                f.input.working_depth.resolve(f.input.dedendum),
            ),
        }
    }

    /// **Whether this is the gear that was asked for.**
    ///
    /// False where a guard moved a dimension ([`Self::clamps`]), and false where
    /// a bound overrode a number a designer typed or reported that it would have
    /// to be — the shift raised to clear undercut, and the addendum that a tip
    /// width cannot carry. Those two live in [`Self::notes`] because that is
    /// where a note naming an *input* belongs, but they are statements about the
    /// part rather than about the rating.
    ///
    /// The rest of `notes` is deliberately not consulted. A notch outside the
    /// `Y_S` fit's band or a root loaded on both flanks are remarks about how a
    /// number was *arrived at*, and a candidate design is not worse for carrying
    /// one.
    ///
    /// One home, because a search choosing between designs wants exactly this
    /// question and there is more than one such search. Reading the clamp list
    /// alone was the same question asked with half the evidence, and it changed
    /// its answer the day the two notes moved out of that list into the one the
    /// spur stage had always put them in.
    #[must_use]
    pub fn as_asked(&self) -> bool {
        self.clamps.is_empty()
            && !self.notes.iter().any(|n| {
                n.is(key::STAGE_SHIFT_RAISED_FOR_UNDERCUT)
                    || n.is(key::STAGE_ADDENDUM_ABOVE_TIP_WIDTH)
                    || n.is(key::STAGE_ADDENDUM_HELD_TO_TIP_WIDTH)
            })
    }
}

/// Everything a parallel-axis stage produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct SpurResult {
    /// `z₂ / z₁`.
    pub ratio: f64,
    /// Zero-backlash centre distance, mm.
    pub centre_distance_nominal: f64,
    /// **The clearance the pair actually runs at**, which is
    /// `centre_distance − centre_distance_nominal` and is derived rather than
    /// echoed.
    ///
    /// A centre distance is the **true** distance and a clearance is what
    /// portion of it is clearance, so these three numbers are two facts and a
    /// subtraction — and reporting the input back is a fourth place the same
    /// quantity can be said, which was wrong whenever the distance was given.
    /// A pair told to run at 30.3 mm whose shifts put it at 30.0057 has 0.294 mm
    /// of clearance; it used to report 0.02, the number in the box, or zero.
    ///
    /// Derived, so the panel can still grey the input by reading the answer,
    /// and so the answer cannot disagree with the two numbers above it.
    pub clearance: f64,
    /// The centre distance actually used, including clearance.
    pub centre_distance: f64,
    /// **What this pair's one mesh reports**, in the type every other kind
    /// reports a parallel-axis mesh in.
    ///
    /// Seven fields used to sit here loose — the operating pressure angle, the
    /// three contact ratios, whether the pair hunts, the shared contact stress
    /// and relative radius, the efficiency and the backlash — which made the
    /// parallel-axis stage the one kind not using the type named for what a
    /// parallel-axis mesh reports. The front end had the same split: a
    /// `meshRows` snippet for the kinds carrying a `MeshReport`, and the same
    /// rows written out again for this one.
    ///
    /// A spur pair's efficiency and backlash **are** its mesh's — one mesh, no
    /// carrier — so they are read through here rather than stored a second
    /// time, and `StageResult::efficiency` is where every kind is made to agree
    /// about which level it is being asked for.
    pub mesh: MeshReport,
    pub gears: [GearResult; 2],
    /// Anything the stage had to say about the design.
    pub notes: Vec<crate::note::Note>,
}

/// Why a stage could not be solved.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TrainError {
    /// The screw pair cannot exist — see [`crate::screw::ScrewError`].
    Screw(crate::screw::ScrewError),
    /// The pair cannot mesh, or the shifts put it outside the involute domain.
    Mesh(MeshError),
    /// No usable path of contact — the teeth do not reach each other.
    NoContact,
    /// A material name that is not in the library.
    UnknownMaterial(String),
    /// A tooth so undercut there is no root section left to rate.
    NoRootSection,
    /// The train has no stages, so there is nothing to accumulate.
    Empty,
    /// A hula stage that has no geometry — see [`crate::hula::Error`].
    Hula(crate::hula::Error),
    /// **Which stage could not be solved**, wrapped around why.
    ///
    /// A train is a chain: a stage that fails takes the shaft line with it, so
    /// every stage after it has no speed or torque to be solved at and the
    /// second pass — which rates *every* stage against the efficiencies
    /// downstream — cannot run at all. So the whole train has no answer, and
    /// the least a reader is owed is which of its stages is the one to look at.
    InStage {
        /// Zero-based, as the stages are indexed; the front end numbers from 1.
        stage: usize,
        cause: Box<TrainError>,
    },
}

/// So a hula stage can report its findings in the vocabulary every other stage
/// kind reports in. It had a refusal type of its own, from before the train had
/// an arm to put one in; now that it has, the arrangement's own error is a
/// [`TrainError`] like a mesh's, and the stage returns the same `Result` as
/// every other solver here — which is what lets it name a material the library
/// does not have, or a member too undercut to rate, without inventing a second
/// place to say so.
impl From<crate::hula::Error> for TrainError {
    fn from(e: crate::hula::Error) -> Self {
        Self::Hula(e)
    }
}

impl crate::note::Explain for TrainError {
    /// Why the stage could not be solved, as a key and its values.
    ///
    /// The nested cases delegate rather than restating: a mesh that will not
    /// mesh says so once, wherever it is asked.
    fn note(&self) -> crate::note::Note {
        use crate::note::{key, Note};
        match self {
            Self::Mesh(e) => e.note(),
            // The drive diagnoses itself; the train carries the note rather
            // than restating it.
            Self::Hula(e) => e.note(),
            Self::Screw(e) => e.note(),
            Self::NoContact => Note::new(key::ERROR_TRAIN_NO_CONTACT),
            Self::UnknownMaterial(n) => {
                Note::new(key::ERROR_TRAIN_UNKNOWN_MATERIAL).text("name", n.clone())
            }
            Self::NoRootSection => Note::new(key::ERROR_TRAIN_NO_ROOT_SECTION),
            Self::Empty => Note::new(key::ERROR_TRAIN_EMPTY),
            // The stage number belongs to the reader rather than to the reason,
            // so the note is the cause's and the number reaches the front end
            // through the error's own shape.
            Self::InStage { cause, .. } => cause.note(),
        }
    }
}

/// English, for the CLI and for `Debug`. **Not** what the browser renders — see
/// [`TrainError::note`].
impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Mesh(e) => write!(f, "{e}"),
            Self::Hula(e) => write!(f, "the arrangement has no geometry: {e:?}"),
            Self::Screw(e) => match e {
                crate::screw::ScrewError::NotPositive => {
                    write!(f, "a module, diameter or tooth count is not positive")
                }
                crate::screw::ScrewError::WormTooThin => write!(
                    f,
                    "the first member has no lead angle: sized by diameter, the worm \
                     is too thin for that many starts at that module and its thread \
                     would have to wrap at ninety degrees or more; sized by helix \
                     angle, a zero helix makes it a spur gear, which has no lead at \
                     all — put the helical member first and the spur one second"
                ),
                crate::screw::ScrewError::ShaftAngleImpossible => {
                    write!(f, "that shaft angle leaves the wheel with no lead angle")
                }
                crate::screw::ScrewError::FirstMemberIsADisc => write!(
                    f,
                    "the first member's helix angle reaches ninety degrees: its teeth \
                     would run circumferentially and its pitch diameter is unbounded, \
                     which is a disc rather than a gear"
                ),
                crate::screw::ScrewError::AxesAreParallel => write!(
                    f,
                    "parallel axes: a worm stage needs crossed shafts, and a \
                     parallel pair is a spur stage"
                ),
            },
            Self::NoContact => write!(f, "the teeth never come into contact"),
            Self::UnknownMaterial(n) => write!(f, "no material named {n:?} in the library"),
            Self::NoRootSection => write!(f, "the tooth is too undercut to have a root section"),
            Self::Empty => write!(f, "the geartrain has no stages"),
            Self::InStage { stage, cause } => write!(f, "stage {}: {cause}", stage + 1),
        }
    }
}

impl std::error::Error for TrainError {}

/// A self-contained material library for tests, so `gear-core` keeps no
/// dependency on `gear-io`. Shared by every stage kind's tests.
#[cfg(test)]
pub(super) fn test_library() -> MaterialLibrary {
    use crate::material::{Basis, Family, Measure, Value};
    let steel = Material {
        name: "4340 Hardened Steel".into(),
        class: Family::Steel,
        grade: "test".into(),
        condition: "test".into(),
        source: "test".into(),
        density: Value::datasheet(7850.0),
        elastic_modulus: Value::datasheet(190_000.0),
        poissons_ratio: Value::datasheet(0.29),
        ultimate_allowable: Value::datasheet(1365.0),
        ultimate_measure: Measure::Yield,
        fatigue_allowable: Value {
            value: 750.0,
            basis: Basis::Estimated,
            note: Some("test".into()),
        },
    };
    let bronze = Material {
        name: "Brass C360".into(),
        class: Family::Brass,
        elastic_modulus: Value::datasheet(97_000.0),
        poissons_ratio: Value::datasheet(0.321),
        ultimate_allowable: Value::datasheet(310.0),
        fatigue_allowable: Value {
            value: 140.0,
            basis: Basis::Estimated,
            note: Some("test".into()),
        },
        ..steel.clone()
    };
    MaterialLibrary {
        materials: vec![steel, bronze],
    }
}

/// **What a stage is asked to optimise, and what it may not do to get there.**
///
/// Every stage with shifts to choose carries the same two decisions, and they
/// were the same two fields written out three times — which is three places to
/// edit, three serde defaults to keep in step, and a fourth stage away from
/// being four. The searches differ in what is free and what it is worth; this
/// does not differ at all.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(default))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Optimisation {
    /// **Choose the automatic shifts for efficiency rather than for undercut.**
    ///
    /// Off, so a stage answers as it always has. On, the shifts a designer has
    /// left automatic are chosen to make the stage lose least, with the undercut
    /// shift as a *floor* rather than as the answer
    /// (docs/reference.md#efficiency-parallel-axes).
    ///
    /// What is already given constrains it rather than being overruled by it: a
    /// manual shift is that gear's, and a manual centre distance or crank offset
    /// fixes a shift sum. Enough of them leave nothing to choose, which is a
    /// design fully specified rather than an error.
    pub enabled: bool,
    /// **The transverse contact ratio the optimiser may not go below.**
    ///
    /// Sliding loss falls monotonically with the length of the path, so the
    /// least-loss pair is always the one whose teeth barely reach: this is the
    /// constraint that answers rather than the optimum, which is why it is an
    /// input and not a constant. 1.2 is the usual design minimum; a mesh of one
    /// tooth of difference sits just above continuous contact at every shift it
    /// can be built at, and asks for less.
    ///
    /// It bounds the *optimiser* only. A design specified by hand is reported as
    /// it is, with the existing note below 1.
    pub min_contact_ratio: f64,
}

impl Default for Optimisation {
    fn default() -> Self {
        Self {
            enabled: false,
            min_contact_ratio: 1.2,
        }
    }
}

/// A stage of a geartrain, of whichever kind.
///
/// Serialised with a `kind` tag alongside the stage's own fields, so a train
/// file says what each stage is rather than relying on position.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum Stage {
    Spur(SpurStage),
    Worm(WormStage),
    // Boxed for the same reason `StageResult`'s variants are: a planetary stage
    // carries three gears, a cutter and an arrangement where a spur stage carries
    // two gears, so a `Vec<Stage>` would otherwise pay the largest of them for
    // every stage whatever its kind. Invisible to readers and to serde.
    Planetary(Box<PlanetaryStage>),
    Hula(Box<HulaStage>),
}

impl Default for Stage {
    fn default() -> Self {
        Self::Spur(SpurStage::default())
    }
}

/// One of a stage's constrainable inputs, named so a caller can find it.
///
/// `Shift(i)` indexes the stage's members in the order
/// [`StageResult::members`] reports them — a pair's two gears, a set's sun,
/// planet and ring, a hula stage's four — so a caller that can walk members can
/// resolve one of these without knowing the kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Freedom {
    /// The distance the stage's members run at.
    CentreDistance,
    /// One member's profile shift.
    Shift(usize),
}

/// **A set of inputs bound by one relation, and how many of them may be given.**
///
/// A geometric relation among `n` inputs leaves `n − 1` of them free, so giving
/// `n` is not a tighter specification — it is a contradiction, and one of the
/// numbers would have to be ignored. This says which inputs are in that
/// argument and how many may stand.
///
/// # Why the *stage* declares this rather than the front end
///
/// It was three functions in TypeScript, one per stage kind, each restating a
/// relation the core already enforces. That is two faults at once: an
/// engineering rule written outside Rust, and the same idea written once per
/// kind — so a fifth kind arrives with no relief at all, and a rule that changes
/// changes in one of four places. It is also untestable there, and was untested.
///
/// The stage kinds genuinely differ in *what* is related, which is why this is a
/// declaration and not a constant: a pair relates its distance to its two
/// shifts, an epicyclic set relates its three shifts to each other through the
/// two distances that must agree, and a hula stage relates each mesh's pair
/// separately because its crank fixes their difference one mesh at a time.
///
/// What does **not** differ is the resolution: too many given means the first
/// one in `order` that the designer is not this moment touching goes back to
/// automatic. Least precious first, and stated by the stage.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct FreedomGroup {
    /// How many of `order` may be given before the set is over-determined.
    pub given_at_most: usize,
    /// The inputs in the argument, **relief order, least precious first**.
    pub order: Vec<Freedom>,
}

impl Stage {
    /// The input a [`Freedom`] names, to be read or written.
    ///
    /// The one place the core's member order — [`StageResult::members`]' —
    /// meets each kind's own fields. `None` where a kind has no such input,
    /// which is the same thing its [`Self::freedoms`] says by not mentioning it.
    fn auto_mut(&mut self, f: Freedom) -> Option<&mut Auto<f64>> {
        match (self, f) {
            (Self::Spur(s), Freedom::CentreDistance) => Some(&mut s.centre_distance),
            (Self::Spur(s), Freedom::Shift(i)) => s.gears.get_mut(i).map(|g| &mut g.profile_shift),
            (Self::Planetary(p), Freedom::Shift(i)) => match i {
                0 => Some(&mut p.sun.profile_shift),
                1 => Some(&mut p.planet.profile_shift),
                2 => Some(&mut p.ring.profile_shift),
                _ => None,
            },
            (Self::Hula(h), Freedom::Shift(i)) => h.gears.get_mut(i).map(|g| &mut g.profile_shift),
            _ => None,
        }
    }

    /// **This stage with its over-determined inputs relieved.**
    ///
    /// A designer who pins a pair's distance *and* both its shifts has asked for
    /// a contradiction: the three are bound by one relation, so one of them
    /// would have to be ignored. Rather than accept an input and quietly
    /// disregard it, the first one in relief order that the designer is **not**
    /// this moment pinning goes back to automatic, visibly.
    ///
    /// `just` is the input they have this moment given, and is never the one
    /// relieved. Every group is resolved, because a stage can have more than one
    /// argument going on at once.
    ///
    /// # Why this is here and not in the panel
    ///
    /// It was three functions in TypeScript, one per stage kind. Rule 1 puts an
    /// engineering rule in Rust, rule 4 says one idea belongs in one place, and
    /// neither was being followed — nor was any of it tested. What decided it is
    /// that the rule is not the panel's to know: **it is the same relation the
    /// solve enforces**, read from the other end, and the two must agree or a
    /// designer is offered an input the solve will disregard.
    ///
    /// Nothing here decides a *value*. It only says which inputs are still being
    /// read, which is why it can be a pure function of the inputs.
    #[must_use]
    pub fn relieved(&self, just: Freedom) -> Self {
        let mut out = self.clone();
        for group in self.freedoms() {
            // Counted with the same accessor that does the relieving, so a
            // freedom this kind does not have is absent from both.
            let mut given = 0usize;
            for f in &group.order {
                if out.auto_mut(*f).is_some_and(|a| !a.auto) {
                    given += 1;
                }
            }
            for f in &group.order {
                if given <= group.given_at_most {
                    break;
                }
                if *f == just {
                    continue;
                }
                if let Some(a) = out.auto_mut(*f) {
                    if !a.auto {
                        a.auto = true;
                        given -= 1;
                    }
                }
            }
        }
        out
    }

    /// **Every argument this stage's inputs can get into with each other.**
    ///
    /// Empty where a kind has no such relation — a screw stage has no profile
    /// shift, so nothing inside it is free to absorb a distance and there is
    /// nothing to relieve. That is a fact about a worm and is why its mode 3 is
    /// an open question rather than an oversight.
    ///
    /// A stage may have **more than one** group: a hula stage's crank fixes the
    /// difference of each mesh's two shifts, which is one relation per mesh
    /// rather than one for the stage.
    #[must_use]
    pub fn freedoms(&self) -> Vec<FreedomGroup> {
        // **Every group here is a single relation**, so exactly one of its
        // inputs is the one the others decide. Written once rather than as a
        // count per kind, because a count per kind is a count to get wrong —
        // and the first draft did, by one, on the kind with the most tests.
        let one_relation = |order: Vec<Freedom>| FreedomGroup {
            given_at_most: order.len() - 1,
            order,
        };
        match self {
            // A pair's distance and its two shifts: `a = f(x₁ + x₂) + clearance`
            // is one relation, so two of the three may be given. The distance
            // comes first because it is the one a designer expects to give way
            // when they pin both shifts.
            Self::Spur(s) => vec![one_relation(
                std::iter::once(Freedom::CentreDistance)
                    .chain((0..s.gears.len()).map(Freedom::Shift))
                    .collect(),
            )],
            // **An epicyclic set has two shifts to give, not three.** Its two
            // centre distances have to agree, which is one relation among the
            // three shifts. It has no distance input of its own — the common
            // distance falls out — which is F39's third item.
            Self::Planetary(_) => vec![one_relation((0..3).map(Freedom::Shift).collect())],
            // **One relation per mesh.** The crank offset fixes the difference
            // of a pair's two shifts, so pinning both over-specifies that mesh
            // — the same triangle a pair's distance and two shifts make, one
            // freedom smaller — and the other mesh is a separate argument.
            Self::Hula(h) => (0..h.gears.len() / 2)
                .map(|m| one_relation(vec![Freedom::Shift(2 * m), Freedom::Shift(2 * m + 1)]))
                .collect(),
            Self::Worm(_) => Vec::new(),
        }
    }
}

/// What a stage produced, of whichever kind.
///
/// **Each kind keeps its own shape.** A worm stage has no bending stress, no
/// minimum face width from contact and two efficiencies; a spur stage has all
/// three and one. What the train needs from either is small enough to read
/// through the accessors below — ratio, efficiency, the backlash at the output
/// member — so the accumulation never asks what kind it was, without every
/// result having to pretend to be the same shape.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(tag = "kind", rename_all = "snake_case"))]
pub enum StageResult {
    // Both variants are boxed. A result carries material records, admissible
    // ranges and notes, so the kinds differ in size by an order of magnitude and
    // will keep doing so as more arrive; a `Vec<StageResult>` would otherwise
    // pay the largest of them for every stage whatever its kind. The boxes are
    // invisible to readers and to serde.
    Spur(Box<SpurResult>),
    Worm(Box<WormResult>),
    Planetary(Box<PlanetaryResult>),
    Hula(Box<HulaResult>),
}

impl StageResult {
    /// `z₂/z₁`.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Spur(r) => r.ratio,
            Self::Worm(r) => r.ratio,
            Self::Planetary(r) => r.ratio,
            Self::Hula(r) => r.ratio,
        }
    }

    /// Mesh efficiency, both directions.
    ///
    /// A parallel-axis stage puts the same number in both and a worm does not;
    /// the train does not have to know which is which. It takes `.forward` to
    /// propagate torque, and reports the pair.
    #[must_use]
    pub fn efficiency(&self) -> Directional<f64> {
        match self {
            Self::Spur(r) => r.mesh.efficiency,
            Self::Worm(r) => r.efficiency,
            Self::Planetary(r) => r.efficiency,
            Self::Hula(r) => r.efficiency,
        }
    }

    /// Angular backlash at whichever member is the *output* in each direction.
    ///
    /// The same tooth gap seen from two lever arms: it subtends a larger angle
    /// at the smaller member, so a pair with different tooth counts genuinely
    /// reports two different numbers.
    #[must_use]
    pub fn backlash(&self) -> Directional<Backlash> {
        match self {
            Self::Spur(r) => r.mesh.backlash_by_drive(),
            Self::Worm(r) => r.backlash,
            Self::Planetary(r) => r.backlash,
            Self::Hula(r) => r.backlash,
        }
    }

    /// **Every member of this stage that is a gear**, whatever kind it is.
    ///
    /// The kind-independent accessors beside this one — [`Self::ratio`],
    /// [`Self::efficiency`], [`Self::backlash`] — say what every stage has. This
    /// says what every *member* has, and it is the one that was missing: a sweep
    /// over "every number every member reports" had to know the five kinds and
    /// name their fields, which is how a formula comes to be written five times
    /// and one of them to be wrong (`docs/corrections.md`, and F30 of the audit
    /// that added this).
    ///
    /// A worm stage contributes **nothing** here, and that is the qualification
    /// rather than an omission: a worm is a thread and its wheel is the envelope
    /// of one, so neither is a gear in the sense the rest of this vocabulary
    /// means. A crossed *gear* pair contributes both of its members
    /// ([`WormMemberResult::gear`]).
    #[must_use]
    pub fn members(&self) -> Vec<&GearResult> {
        match self {
            Self::Spur(r) => r.gears.iter().collect(),
            Self::Planetary(r) => vec![&r.sun, &r.planet.gear, &r.ring],
            Self::Hula(r) => r.gears.iter().map(|g| &g.gear).collect(),
            Self::Worm(r) => r.members.iter().filter_map(|m| m.gear.as_ref()).collect(),
        }
    }

    /// **Every parallel-axis mesh this stage has**, in the order it built them.
    ///
    /// The companion of [`Self::members`], and for the same reason: a question
    /// about *a mesh* — is contact continuous, do the tips foul, how much play
    /// is there — is asked of a stage by asking each of its meshes, and a walk
    /// that names the kinds is a walk that forgets one. A pair has one, either
    /// epicyclic kind has two, and a screw stage has **none**: its pair is not a
    /// parallel-axis mesh and reports [`WormResult::crossed`] instead.
    #[must_use]
    pub fn meshes(&self) -> Vec<&MeshReport> {
        match self {
            Self::Spur(r) => vec![&r.mesh],
            Self::Planetary(r) => vec![&r.sun_planet, &r.planet_ring],
            Self::Hula(r) => r.meshes.iter().map(|m| &m.report).collect(),
            Self::Worm(_) => Vec::new(),
        }
    }

    /// The hula result, if that is what this is.
    #[must_use]
    pub fn as_hula(&self) -> Option<&HulaResult> {
        match self {
            Self::Hula(r) => Some(r),
            _ => None,
        }
    }

    /// The parallel-axis result, if that is what this is.
    #[must_use]
    pub fn as_spur(&self) -> Option<&SpurResult> {
        match self {
            Self::Spur(r) => Some(r),
            _ => None,
        }
    }

    /// The planetary result, if that is what this is.
    #[must_use]
    pub fn as_planetary(&self) -> Option<&PlanetaryResult> {
        match self {
            Self::Planetary(r) => Some(r),
            _ => None,
        }
    }

    /// The worm result, if that is what this is.
    #[must_use]
    pub fn as_worm(&self) -> Option<&WormResult> {
        match self {
            Self::Worm(r) => Some(r),
            _ => None,
        }
    }

    /// Write in the speeds and cycles, which only the whole shaft line knows.
    ///
    /// The cycles arriving here are **revolutions**, fractional; each stage kind
    /// scales them into the count its members actually see and rounds at the end
    /// with [`loaded_cycles`].
    ///
    /// A worm's "tooth cycles" are revolutions: its thread is engaged
    /// continuously rather than meeting a mate once per turn, so the count is
    /// the same arithmetic but means something looser. It is reported because a
    /// duty cycle has to be reported somewhere, not because a worm thread has a
    /// fatigue life this crate can rate.
    fn set_kinematics(&mut self, speeds: [f64; 2], cycles: [(f64, Option<(f64, f64)>); 2]) {
        match self {
            Self::Spur(r) => {
                for (i, g) in r.gears.iter_mut().enumerate() {
                    g.speed = speeds[i];
                    g.tooth_cycles = loaded_cycles(cycles[i].0, cycles[i].1);
                }
            }
            Self::Worm(r) => {
                for (i, m) in r.members.iter_mut().enumerate() {
                    m.speed = speeds[i];
                    m.tooth_cycles = loaded_cycles(cycles[i].0, cycles[i].1);
                }
                // Sliding needs a shaft speed, so it could only be filled here.
                r.sliding_velocity = r.sliding_ratio
                    * (speeds[0] / 60.0 * std::f64::consts::TAU)
                    * (r.members[0].pitch_diameter / 2.0);
            }
            // An epicyclic set's speeds are not the train's two-member pattern —
            // it has three shafts and its own kinematics already set them, so
            // only the cycles are filled here, through the one rule both kinds
            // obey ([`engagements`]). Its members' revolutions scale before they
            // are counted, including — for a reversing drive — the per-actuation
            // figure the rounding is applied to.
            Self::Planetary(r) => {
                let n = f64::from(r.planets.max(1));
                let carrier = r.speeds[crate::planetary::PlanetaryShaft::Carrier.index_pub()];
                let input = r.speeds[r.arrangement.input.index_pub()];
                let count = |member: f64, at: &mut GearResult| {
                    let f = engagements(member, carrier, input, n);
                    at.tooth_cycles =
                        loaded_cycles(cycles[0].0 * f, cycles[0].1.map(|(e, a)| (e * f, a)));
                };
                count(r.speeds[0], &mut r.sun);
                count(r.speeds[2], &mut r.ring);
                let planet = r.planet.gear.speed;
                count(planet, &mut r.planet.gear);
            }
            // A hula sets its own speeds — four gears on three shafts, and its
            // own kinematics filled them. The crank is the carrier of both
            // meshes *and* the shaft the train counted revolutions on, and the
            // drive has one wobble body, so the rule reads as "how far each gear
            // turns against the crank".
            Self::Hula(r) => {
                let crank = r.crank_speed;
                for g in &mut r.gears {
                    let f = engagements(g.gear.speed, crank, crank, 1.0);
                    g.gear.tooth_cycles =
                        loaded_cycles(cycles[0].0 * f, cycles[0].1.map(|(e, a)| (e * f, a)));
                }
            }
        }
    }
}

/// Which ratings an automatic face width is sized from.
///
/// Four toggles rather than two, and shaped like the answer they select from so
/// the UI can walk them rather than naming each: a rating exists for every
/// combination of what fails (bending or contact) and what it is rated against
/// (the peak or the cyclic load).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct FaceSources {
    pub bending: LoadCase<bool>,
    pub contact: LoadCase<bool>,
}

impl Default for FaceSources {
    fn default() -> Self {
        Self {
            bending: LoadCase {
                peak: true,
                cyclic: true,
            },
            // **Neither contact rating sizes a width by default.** Both are
            // offered and both are computed; what they are not is *assumed*.
            //
            // The peak case is the weaker of the two: a Hertzian pressure is not
            // a tensile stress, and the library's `ultimate_allowable` is a
            // tensile figure — a flank under a single overload fails by
            // subsurface shear, at a contact pressure well above it. Comparing
            // them is arithmetic with no mechanism behind it.
            //
            // The cyclic case is sounder — the fatigue allowable is a flank
            // figure — but it is the one that *dominates*, by an order of
            // magnitude: on the reference train it asks 8.5 mm where bending
            // asks 0.9. A default that decides the answer is a default making
            // the design decision, so both are left to the designer and bending
            // is what a fresh stage is sized from.
            contact: LoadCase {
                peak: false,
                cyclic: false,
            },
        }
    }
}

impl FaceSources {
    /// **The width to size a member to**: the largest an *enabled* rating asks
    /// for, or the width the member was given where none is enabled.
    ///
    /// Nothing enabled used to come out **zero**, and the stage then divided by
    /// it — every stress infinite, every minimum width a NaN. Those cross the
    /// boundary as `null` and draw as blanks, so a reader saw the note and no
    /// figures, which is the right outcome reached by accident. The note's
    /// promise is that the input is *said* rather than divided by, and this is
    /// what keeps it: an automatic width with nothing to choose between has
    /// nothing to choose, so it stands at the number already in its box, which
    /// is on screen beside the toggle that stopped deciding it.
    ///
    /// A width a designer **types** as zero is a different thing — it describes
    /// a gear with no face, and this cannot rescue it. See `docs/state.md`.
    #[must_use]
    pub fn width_for(&self, asks: &LoadCase<Widths>, given: f64) -> f64 {
        if !self.any() {
            return given;
        }
        let mut want = 0.0_f64;
        for case in [Case::Peak, Case::Cyclic] {
            let w = asks.get(case);
            if *self.bending.get(case) {
                if let Some(b) = w.bending {
                    want = want.max(b);
                }
            }
            if *self.contact.get(case) {
                if let Some(c) = w.contact {
                    want = want.max(c);
                }
            }
        }
        want
    }

    /// Whether anything at all is selected.
    #[must_use]
    pub fn any(&self) -> bool {
        self.bending.peak || self.bending.cyclic || self.contact.peak || self.contact.cyclic
    }
}

/// How a stage's gears are treated for **reversed bending**.
///
/// Assembled by [`solve_train`] and handed down, because both halves of it are
/// facts about the train rather than about any one stage: whether the drive
/// reverses, and whether the designer asked for the correction at all.
///
/// # Why the correction is asked for rather than applied
///
/// A root loaded on both flanks endures less than one loaded on a single flank,
/// and the usual allowance is a fraction on the *allowable*
/// ([`REVERSED_BENDING_FRACTION`](crate::material::REVERSED_BENDING_FRACTION)).
/// That fraction is a convention: it multiplies a stress a part is sized
/// against, which is exactly what `docs/rationale.md` refuses to apply on a
/// designer's behalf. So it is a switch, off by default, and where it is off the
/// stage **says** that reversal is present and uncorrected rather than leaving
/// the reader to notice.
///
/// # Which members reverse
///
/// A planet always does — the sun drives one flank and the ring the other,
/// whatever the drive does. Every other gear does when the *drive* reverses,
/// which is the same flag that already splits the contact cycles between the two
/// flanks. One rule, so a note cannot appear where a cycle count does not.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Reversal {
    /// The drive reverses between actuations, so every gear's root is loaded
    /// both ways.
    pub drive_reverses: bool,
    /// Judge a reversed root against the reduced allowable.
    pub correct: bool,
}

impl Reversal {
    /// Whether a member's root is loaded both ways. `always` is the member's own
    /// structural answer — true for a planet, false for everything else.
    #[must_use]
    pub fn reverses(self, always: bool) -> bool {
        always || self.drive_reverses
    }

    /// What a member's reversal is worth saying about it, if anything.
    ///
    /// One home, so a stage cannot report the correction on one member and stay
    /// silent about it on another — and so the sentence is the same whichever
    /// kind of stage raises it.
    #[must_use]
    pub fn note_for(self, reverses: bool) -> Option<Note> {
        if !reverses {
            return None;
        }
        Some(if self.correct {
            Note::new(key::STAGE_REVERSED_BENDING_APPLIED).number(
                "fraction",
                crate::material::REVERSED_BENDING_FRACTION,
                2,
            )
        } else {
            Note::new(key::STAGE_REVERSED_BENDING_UNCORRECTED)
        })
    }

    /// The **bending** allowable a member is judged against, MPa.
    ///
    /// Only the cyclic case can be reduced: a peak load is survived once and has
    /// no reversal to endure. And only bending — pitting is compressive whichever
    /// flank carries it, so a contact rating keeps the material's own figure.
    #[must_use]
    pub fn bending_allowable(self, m: &Material, case: Case, reverses: bool) -> f64 {
        if self.correct && reverses && case == Case::Cyclic {
            crate::material::reversed_bending_allowable(m).value
        } else {
            allowable(m, case)
        }
    }
}

/// The torques one stage sees, at its **first** member, one per load case.
///
/// Assembled by [`solve_train`], which is the only level that knows where a
/// stage sits in the shaft line and what reaches it from each end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StageTorques {
    /// Peak, delivered forward from the input, N·m.
    pub peak_forward: f64,
    /// Peak, delivered backward from the output, N·m — `None` where no
    /// back-driving load is reacted here. See [`back_driving_torques`].
    pub peak_backward: Option<f64>,
    /// The torque the train runs at, forward, N·m. May be zero.
    pub cyclic: f64,
}

impl StageTorques {
    /// **What one mesh carries, in each load case** — a load case being a torque
    /// *and a direction*.
    ///
    /// `forward` is what this mesh carries driving forward and `backward` what
    /// it carries being driven, each already distributed by the stage's own
    /// construction. The peak is the worse of them and the operating case is
    /// the forward one at the duty torque, since that is the load the train
    /// runs at.
    ///
    /// # The order matters
    ///
    /// The peak is taken **after** each direction's distribution rather than
    /// before it. Taking the larger of the two *shaft* torques first and pushing
    /// that one magnitude through the forward construction is the same answer
    /// only where the distribution does not depend on which way the stage is
    /// driven — true of a parallel-axis mesh, which carries one tangential force
    /// whichever way it turns, and false of an epicyclic set, where which shaft
    /// drives decides on which side `η₀` multiplies, and of a screw pair, whose
    /// output torque carries a forward efficiency a backward load does not
    /// share. Both were rated the wrong way round; `docs/corrections.md` records
    /// what it cost.
    ///
    /// # Zero is a load
    ///
    /// A stage may be driven forward at zero torque and back-driven at a real
    /// one, so the operating fraction is taken against the *forward* peak and
    /// the case torques against nothing at all. A stage carrying nothing in
    /// either direction rates at zero, which is the answer and not a failure.
    #[must_use]
    pub fn on_mesh(&self, forward: f64, backward: Option<f64>) -> LoadCase<f64> {
        LoadCase {
            peak: forward.abs().max(backward.unwrap_or(0.0).abs()),
            cyclic: forward.abs() * self.duty_fraction(),
        }
    }

    /// The duty torque as a fraction of the peak driving **forward** — what a
    /// forward-carried load scales by to reach the operating case. Zero peak is
    /// zero duty, which is the only reading that does not divide by it.
    fn duty_fraction(&self) -> f64 {
        if self.peak_forward == 0.0 {
            0.0
        } else {
            (self.cyclic / self.peak_forward).abs()
        }
    }

    /// The torque to rate a load case at, on the shaft the stage was handed.
    ///
    /// [`Self::on_mesh`] asked of the input shaft itself, which is the mesh load
    /// wherever the distribution is a direction-independent projection.
    #[must_use]
    pub fn at(&self, case: Case) -> f64 {
        *self
            .on_mesh(self.peak_forward, self.peak_backward)
            .get(case)
    }

    /// Everything at one torque, and nothing back-driving — the shape a caller
    /// wants when it is asking about a single load.
    #[must_use]
    /// A member's share of the back-driving load, referred as its forward
    /// torque was.
    ///
    /// The load enters at the far end and `back_driving_torques` refers it to
    /// **this stage's input shaft** before anything else happens to it — so
    /// within a stage the question is only how the input shaft's torque reaches
    /// a member, and the answer is the same path the forward torque took.
    ///
    /// Valid where that path is a geometric projection (a parallel-axis mesh
    /// carries one tangential force, so `T_i = T_in · z_i/z_in` with no
    /// efficiency in it) or where the two directional efficiencies agree — which
    /// covers every kind whose meshes are parallel. **A worm is the exception**
    /// and says so where it computes its own.
    pub fn referred_like(&self, member_torque: f64) -> Option<f64> {
        self.peak_backward.map(|t| {
            if self.peak_forward == 0.0 {
                0.0
            } else {
                member_torque * (t / self.peak_forward)
            }
        })
    }

    pub fn just(torque: f64) -> Self {
        Self {
            peak_forward: torque,
            peak_backward: None,
            cyclic: torque,
        }
    }
}

/// Which allowable a load case is judged against.
///
/// **This is the whole reason the two cases are separate.** A peak load has to be
/// survived once, so the ultimate is the right bar; a cyclic one has to be
/// survived for the duty, so the fatigue figure is. Rating a peak against a
/// fatigue allowable asks the wrong question, and it is what this crate did
/// before the cases existed.
#[must_use]
pub fn allowable(material: &Material, case: Case) -> f64 {
    match case {
        Case::Peak => material.ultimate_allowable.value,
        Case::Cyclic => material.fatigue_allowable.value,
    }
}

/// A quantity evaluated for each **load case** the train describes.
///
/// Named for the engineering term — a defined set of loads applied for analysis
/// — and deliberately *not* for a duty cycle, which is a fraction of time spent
/// running and a different idea with different implications. One of this train's
/// inputs genuinely is that; these are not it.
///
/// The two differ in one thing beyond their torque, and it is the thing that
/// matters: **which allowable they are rated against.** A peak load has to be
/// survived once, so it is judged against the ultimate; a cyclic one has to be
/// survived indefinitely, so it is judged against the fatigue figure. Rating a
/// peak against a fatigue allowable — which is what this crate did before the
/// two cases existed — asks the wrong question and answers it confidently.
///
/// Built through [`LoadCase::of`] for the same reason [`Directional`] is: there
/// is no path by which a stage reports one case and not the other.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct LoadCase<T> {
    /// The worst single application: survive it once.
    pub peak: T,
    /// The load it runs at: survive it for the duty.
    pub cyclic: T,
}

/// Which load case a quantity belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Case {
    Peak,
    Cyclic,
}

impl<T> LoadCase<T> {
    /// Ask the same question of both cases.
    pub fn of(mut f: impl FnMut(Case) -> T) -> Self {
        Self {
            peak: f(Case::Peak),
            cyclic: f(Case::Cyclic),
        }
    }

    /// The value for one case.
    pub const fn get(&self, case: Case) -> &T {
        match case {
            Case::Peak => &self.peak,
            Case::Cyclic => &self.cyclic,
        }
    }

    /// The same pair with each value mapped.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> LoadCase<U> {
        LoadCase {
            peak: f(self.peak),
            cyclic: f(self.cyclic),
        }
    }
}

impl LoadCase<f64> {
    /// Each case as a multiple of the peak — what a figure evaluated at the peak
    /// scales by to reach the other case.
    ///
    /// A rating is evaluated once, at the worst torque the mesh carries, and the
    /// operating case is that scaled; so this is the companion of
    /// [`StageTorques::on_mesh`], which says what those two torques are. **A
    /// zero peak is zero everywhere** — a mesh carrying nothing is a load and
    /// not a division to guard against.
    #[must_use]
    pub fn as_fraction_of_peak(&self) -> Self {
        if self.peak == 0.0 {
            Self {
                peak: 0.0,
                cyclic: 0.0,
            }
        } else {
            Self {
                peak: 1.0,
                cyclic: self.cyclic / self.peak,
            }
        }
    }
}

/// How often a tooth is loaded, which is not one number once the drive reverses.
///
/// Both counts are always reported and are the **same number** when the drive
/// does not reverse — the ordinary case as a value rather than behind a flag, so
/// a reader can quote a range unconditionally.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Cycles {
    /// Engagements the tooth root sees. A reversing drive loads **both** flanks
    /// in bending, so every engagement counts.
    pub bending: f64,
    /// Engagements one flank sees. A reversing drive shares them between the two
    /// flanks, so each takes half; otherwise it is the same number as `bending`.
    pub contact: f64,
}

/// The greatest common divisor of two tooth counts.
///
/// One home, because it was two: a coprime check is what says whether a pair
/// hunts, and every stage kind that has a mesh asks it.
pub(crate) const fn gcd(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// **How often one member of an epicyclic set is engaged**, per revolution of
/// the shaft the train counted revolutions on.
///
/// One rule, and both epicyclic kinds here obey it: a member's teeth are
/// engaged once per revolution **relative to the carrier**, once for each
/// parallel mesh path the set has. In the carrier's frame the arm stands still
/// and everything else turns past it, which is what makes the relative speed the
/// one that counts — and it is the same sentence for a sun, a ring, a planet, a
/// grounded gear and a wobble body, so no member is the exception it has to be
/// remembered for.
///
/// It had been three sentences. A sun and a ring counted the *input shaft's*
/// revolutions, which on an ordinary set with the ring held over-counts the sun
/// by `(z_s + z_r)/z_r` and the ring — whose teeth are loaded while it does not
/// turn at all — by the whole of `(z_s + z_r)/z_s`, three and a half times on
/// the shipped counts. Only the planet was carrier-relative, and it was
/// referred to the *sun's* speed rather than to the input's, so it was right
/// only in the arrangements where those are the same shaft.
///
/// A ratio of speeds, so their magnitude cancels — but a train whose input shaft
/// does not turn has no ratio to take, and answers zero.
fn engagements(member: f64, carrier: f64, input: f64, paths: f64) -> f64 {
    if input == 0.0 {
        return 0.0;
    }
    ((member - carrier) / input).abs() * paths
}

/// How many times a tooth is loaded, from how many times it comes round.
///
/// # Why this rounds, and why the rounding is here
///
/// A tooth is either loaded or it is not: two thirds of an engagement is one
/// engagement as far as the tooth is concerned, so a fractional count is
/// rounded **up**. That is a statement about how a gear is loaded, and it
/// belongs in the model.
///
/// It lived in `TrainPanel.svelte` as `Math.ceil` on the way to the screen —
/// harmless while it was only display, because rounding an integer up gives the
/// same integer wherever you do it, and against this project's standing rule the
/// whole time. Two things made it worth moving: the CLI printed the *unrounded*
/// figure while the browser printed the rounded one, so one model already had
/// two answers; and a reversing drive rounds at a different point in the
/// arithmetic, which makes *where* this happens a modelling decision rather than
/// a formatting one.
///
/// # What reversing changes, and why it is two changes
///
/// **Where the rounding happens.** Not reversing, the whole duty rounds once:
/// `ceil(revolutions_per_actuation × actuations)`. Reversing, each actuation
/// rounds on its own: `ceil(revolutions_per_actuation) × actuations`. A sweep of
/// less than a full turn loads only some of the teeth — but which ones is not
/// knowable, the teeth are radially symmetric, and every one of them has to meet
/// the worst case. So each actuation costs a whole engagement to whichever teeth
/// it reaches, and rounding the total instead would spread a fraction that is
/// not divisible.
///
/// **And the contact count halves.** Reversing puts the load on the other flank
/// on the way back, so a given flank sees half the engagements while the root
/// sees all of them.
///
/// `None` for a continuous drive: there is no actuation to round within, so the
/// total rounds once and the two counts are equal.
#[must_use]
pub fn loaded_cycles(revolutions: f64, per_actuation: Option<(f64, f64)>) -> Cycles {
    match per_actuation {
        Some((each, actuations)) => {
            let bending = each.ceil() * actuations;
            Cycles {
                bending,
                contact: bending / 2.0,
            }
        }
        None => {
            let n = revolutions.ceil();
            Cycles {
                bending: n,
                contact: n,
            }
        }
    }
}

/// Solve one stage of whichever kind, given the torque on its input member.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any(
    stage: &Stage,
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
) -> Result<StageResult, TrainError> {
    solve_any_with(stage, input_speed, torques, lib, Reversal::default())
}

/// The same, told how the train treats reversed bending.
///
/// A second entry point rather than a fourth argument on the first, because a
/// stage asked about in isolation — by a test, by the CLI, by the sweep — has no
/// train to inherit that from and should not have to invent one. The plain call
/// is this one at [`Reversal::default`]: no reversing drive, no correction.
///
/// # Errors
///
/// Whatever the stage kind reports.
pub fn solve_any_with(
    stage: &Stage,
    input_speed: f64,
    torques: StageTorques,
    lib: &MaterialLibrary,
    reversal: Reversal,
) -> Result<StageResult, TrainError> {
    match stage {
        // One stage kind, two meshes. Crossing the shafts changes what the
        // teeth do to each other — a line contact becomes a point, and the
        // sliding changes direction — so a crossed pair answers with the screw
        // result. The *inputs* stay one set, as the specification has them.
        Stage::Spur(s) if s.is_crossed() => {
            solve_crossed_stage(s, torques, lib).map(|r| StageResult::Worm(Box::new(r)))
        }
        Stage::Spur(s) => {
            solve_spur_stage_with(s, torques, lib, reversal).map(|r| StageResult::Spur(Box::new(r)))
        }
        Stage::Worm(s) => solve_worm_stage(s, torques, lib).map(|r| StageResult::Worm(Box::new(r))),
        // A planetary needs a speed as well as a torque: its efficiency depends
        // on which shaft is held, and that is a kinematic question. The train
        // supplies the speed it has reached by this point.
        Stage::Planetary(s) => solve_planetary_stage_with(s, input_speed, torques, lib, reversal)
            .map(|r| StageResult::Planetary(Box::new(r))),
        // A hula needs a speed and a torque for the same reason a planetary
        // does: its efficiency is a power flow, and a power flow is not a
        // property of the teeth alone.
        Stage::Hula(s) => solve_hula_stage_with(s, input_speed, torques, lib, reversal)
            .map(|r| StageResult::Hula(Box::new(r))),
    }
}

/// How the train is used, which is what turns a ratio into a tooth count.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Actuation {
    /// A limited sweep, repeated. The range is measured **at the output**, so
    /// every gear's revolutions are worked *backwards* from there — upstream
    /// gears turn further, not less.
    Intermittent {
        /// Output sweep per actuation, degrees.
        range_degrees: f64,
        actuations: u32,
        /// Whether the drive reverses between actuations.
        ///
        /// It changes nothing but the **cycle count**, and it changes that in
        /// two ways ([`loaded_cycles`]): each actuation's revolutions round up
        /// on their own rather than the total rounding once, because a partial
        /// sweep still loads the teeth it reaches and every tooth must meet the
        /// worst of them; and the contact count halves, because the two flanks
        /// share the engagements while both take the full bending.
        reversing: bool,
    },
    /// Continuous running at the speed it actually runs at.
    Continuous {
        /// The input speed the train runs at, rpm. Bounded by the peak, and
        /// **absolute** rather than a percentage of it for the reason
        /// [`Train::operating_torque_percent`] gives: the crate will not assert
        /// a relation between torque and speed on the user's behalf.
        operating_speed: f64,
        runtime_hours: f64,
    },
}

impl Default for Actuation {
    fn default() -> Self {
        Self::Intermittent {
            range_degrees: 25.0,
            actuations: 1000,
            reversing: false,
        }
    }
}

/// A whole geartrain.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Train {
    /// Peak input speed, rpm.
    pub input_speed: f64,
    /// Peak input torque, N·m.
    pub input_torque: f64,
    /// Peak torque applied at the **output** shaft, N·m, trying to drive the
    /// train backwards.
    ///
    /// A load case of its own rather than a sign on the input: it enters at the
    /// far end and is attenuated by each stage's *backward* efficiency on the
    /// way up. See [`back_driving_torques`] for where it is reacted, and where
    /// it therefore reaches no number at all.
    pub back_driving_torque: f64,
    /// The torque the train runs at, N·m — the load its fatigue life is spent
    /// against, as opposed to the peak it must merely survive.
    ///
    /// Bounded by [`Self::input_torque`] and meaningful down to and including
    /// **zero**: a train that only ever sees its peak has no cyclic case.
    pub operating_torque: f64,
    pub actuation: Actuation,
    /// Judge a root that is loaded on **both** flanks against the reduced
    /// bending allowable.
    ///
    /// Off by default. A planet's root is always loaded both ways and a
    /// reversing drive loads every root both ways, but what to do about it is a
    /// convention that multiplies a stress — so the train asks rather than
    /// assumes, and says where reversal is present and uncorrected. See
    /// [`Reversal`].
    #[cfg_attr(feature = "serde", serde(default))]
    pub reversed_bending: bool,
    pub stages: Vec<Stage>,
}

impl Train {
    /// What fraction of peak the operating torque is, as a percentage.
    ///
    /// Reported rather than entered. The two are given **separately and
    /// absolutely** because this crate has no basis for a relation between
    /// torque and speed: an electric motor makes them inversely proportional,
    /// efficiencies bend that, and another power source need not obey it at all.
    /// One percentage driving both would assert a relationship nothing here can
    /// stand behind — so the user states each, and the ratio between them is an
    /// *output*, computed where every other number is.
    ///
    /// `None` at zero peak, where there is no fraction to take.
    #[must_use]
    pub fn operating_torque_percent(&self) -> Option<f64> {
        (self.input_torque != 0.0).then(|| 100.0 * self.cyclic_torque() / self.input_torque)
    }

    /// The operating torque as the solve uses it: never above the peak.
    ///
    /// Clamped rather than refused (docs/rationale.md), and clamped **here** so
    /// that the figure the train is solved at, the percentage reported beside
    /// the input, and the note that says it happened cannot disagree.
    #[must_use]
    pub fn cyclic_torque(&self) -> f64 {
        self.operating_torque
            .clamp(-self.input_torque.abs(), self.input_torque.abs())
    }

    /// The operating speed as the solve uses it, rpm — likewise never above the
    /// peak. `None` for an intermittent drive, which has no operating speed:
    /// it turns through its range and stops.
    #[must_use]
    pub fn cyclic_speed(&self) -> Option<f64> {
        match self.actuation {
            Actuation::Continuous {
                operating_speed, ..
            } => Some(operating_speed.clamp(-self.input_speed.abs(), self.input_speed.abs())),
            Actuation::Intermittent { .. } => None,
        }
    }
}

/// What a train produces.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct TrainResult {
    /// Product of the stage ratios.
    pub total_ratio: f64,
    /// Output speed, rpm — an *output*, per Q1.
    pub output_speed: f64,
    /// Output torque, N·m, after efficiency losses.
    pub output_torque: f64,
    /// Product of the stage efficiencies, in both drive directions.
    ///
    /// A train containing a self-locking stage cannot be back-driven at all, and
    /// [`Directional::self_locking`] on this pair says so.
    pub total_efficiency: Directional<f64>,
    /// Angular backlash referred to whichever shaft is the output, degrees: the
    /// last shaft driving forward, the first driving backward.
    pub backlash: Directional<Backlash>,
    /// The operating torque as a percentage of the peak — see
    /// [`Train::operating_torque_percent`] for why this is an output and not an
    /// input. `None` at zero peak.
    pub operating_torque_percent: Option<f64>,
    /// What the train as a whole wants read: an input clamped against its peak,
    /// and where a back-driving load is reacted. Facts about the shaft line, so
    /// no stage is in a position to say them.
    pub notes: Vec<Note>,
    pub stages: Vec<StageResult>,
}

/// Solve a whole train, propagating torque and accumulating backlash.
///
/// # Torque
///
/// `T_{k+1} = T_k · i_k · η_k`. Efficiency always *reduces* delivered torque,
/// whichever way the train is driven — that is the sign convention it is easy to
/// get wrong, so it is stated here and tested.
///
/// # Backlash
///
/// Referred to the output shaft, each stage's contribution is divided by the
/// ratio of everything downstream of it:
///
/// ```text
/// θ_out = Σ_k  j_θ,k / Π_{j>k} i_j
/// ```
///
/// The consequence worth surfacing: the **last** stage dominates, and backlash in
/// the first stage is nearly free. A train designed for low output backlash
/// should spend its tolerance budget at the output end.
///
/// Where a back-driving load is reacted, and what each stage feels of it.
///
/// # The model
///
/// A load exists only where something reacts it. A torque applied at the output
/// shaft works its way *upstream*, attenuated at each stage by that stage's
/// backward ratio and backward efficiency — until it reaches a stage that cannot
/// be back-driven at all. That stage holds it: everything upstream sees nothing,
/// and the reaction is the load this stage carries.
///
/// If the chain reaches the input still turning something, then **nothing
/// reacted it**: the train is back-drivable, the load simply drives it, and the
/// case is zero everywhere. A back-driving torque on a back-drivable train is an
/// input that moves no number, and this is why.
///
/// Each stage's figure is referred to its own **input** shaft, which is the shaft
/// every stage solver takes its torque on. That referral is a division by the
/// ratio and nothing else: the mesh force is set by the torque at the wheel, and
/// the losses sit between the mesh and the shaft beyond it, not before it.
fn back_driving_torques(stages: &[StageResult], applied: f64) -> (Vec<Option<f64>>, Option<usize>) {
    let none = || vec![None; stages.len()];
    if applied == 0.0 {
        return (none(), None);
    }
    let mut torques = none();
    let mut at_output = applied;
    for (k, s) in stages.iter().enumerate().rev() {
        let referred = at_output / s.ratio();
        torques[k] = Some(referred);
        let backward = s.efficiency().backward;
        if backward <= 0.0 {
            // Self-locking: this stage is where the load stops.
            return (torques, Some(k));
        }
        at_output = referred * backward;
    }
    // The load reached the input with something still to turn.
    (none(), None)
}

/// # Errors
///
/// [`TrainError::Empty`] for a train with no stages, or whatever the first
/// failing stage reports.
pub fn solve_train(train: &Train, lib: &MaterialLibrary) -> Result<TrainResult, TrainError> {
    if train.stages.is_empty() {
        return Err(TrainError::Empty);
    }

    // How every stage treats a root loaded on both flanks. Both halves are the
    // train's to know: the drive's own reversal is the same flag that splits the
    // contact cycles, and whether to correct for it at all is one switch for the
    // whole train rather than a decision taken per stage.
    let reversal = Reversal {
        drive_reverses: matches!(
            train.actuation,
            Actuation::Intermittent {
                reversing: true,
                ..
            }
        ),
        correct: train.reversed_bending,
    };

    // --- what each stage is loaded by.
    //
    // Two propagations, in opposite directions, and only the forward one can
    // start: a stage's ratio and efficiency do not depend on the torque through
    // it, but the *backward* torque at a stage depends on every efficiency
    // downstream, which is not known until those stages have been solved. So the
    // train is solved once to learn the shaft line and again to rate it. The
    // second pass is not a refinement of the first — it is the same arithmetic
    // with the load it was missing.
    let forward =
        |torques: &dyn Fn(usize) -> StageTorques| -> Result<Vec<StageResult>, TrainError> {
            let mut speed = train.input_speed;
            let mut out = Vec::with_capacity(train.stages.len());
            for (k, stage) in train.stages.iter().enumerate() {
                let r = solve_any_with(stage, speed, torques(k), lib, reversal).map_err(|e| {
                    TrainError::InStage {
                        stage: k,
                        cause: Box::new(e),
                    }
                })?;
                speed /= r.ratio();
                out.push(r);
            }
            Ok(out)
        };

    // Forward torques are the same in both passes, so they are worked out once
    // from the ratios and efficiencies the first pass reports.
    let first = forward(&|_| StageTorques::just(train.input_torque))?;
    let mut fwd = Vec::with_capacity(first.len());
    let cyclic_torque = train.cyclic_torque();
    let (mut peak, mut cyclic) = (train.input_torque, cyclic_torque);
    for r in &first {
        fwd.push((peak, cyclic));
        peak = peak * r.ratio() * r.efficiency().forward;
        cyclic = cyclic * r.ratio() * r.efficiency().forward;
    }
    let (backward, reacted_at) = back_driving_torques(&first, train.back_driving_torque);
    let mut stages = forward(&|k| StageTorques {
        peak_forward: fwd[k].0,
        peak_backward: backward[k],
        cyclic: fwd[k].1,
    })?;
    let torque = peak;

    // --- what the train wants read, as opposed to what a stage does.
    let mut notes = Vec::new();
    if train.operating_torque != cyclic_torque {
        notes.push(Note::new(key::TRAIN_OPERATING_TORQUE_CLAMPED).number(
            "torque",
            cyclic_torque,
            4,
        ));
    }
    if let (
        Actuation::Continuous {
            operating_speed, ..
        },
        Some(used),
    ) = (&train.actuation, train.cyclic_speed())
    {
        if *operating_speed != used {
            notes.push(Note::new(key::TRAIN_OPERATING_SPEED_CLAMPED).number("speed", used, 1));
        }
    }
    if train.back_driving_torque != 0.0 {
        notes.push(match reacted_at {
            Some(k) => {
                Note::new(key::TRAIN_BACK_DRIVING_REACTED_AT).text("stage", (k + 1).to_string())
            }
            None => Note::new(key::TRAIN_BACK_DRIVING_NOT_REACTED),
        });
    }

    let total_ratio: f64 = stages.iter().map(StageResult::ratio).product();
    let total_efficiency = Directional::of(|d| {
        stages
            .iter()
            .map(|s| *s.efficiency().get(d))
            .product::<f64>()
    });

    // --- speeds and tooth cycles, which need the whole shaft line. None of this
    // asks what kind of stage it is looking at.
    let ratios: Vec<f64> = stages.iter().map(StageResult::ratio).collect();
    for (k, s) in stages.iter_mut().enumerate() {
        let upstream: f64 = ratios[..k].iter().product();
        let speed_in = train.input_speed / upstream;
        let speeds = [speed_in, speed_in / ratios[k]];

        // The reduction between each member and the output. MeshSide 0 of a stage
        // sits before that stage's own mesh, member 1 after it.
        // Revolutions, and — for an intermittent drive — how they divide into
        // actuations, which is what a reversing drive needs in order to round
        // within one rather than over all of them.
        let cycles = [0usize, 1].map(|i| {
            let to_output: f64 = if i == 0 {
                ratios[k..].iter().product()
            } else {
                ratios[k + 1..].iter().product()
            };
            match train.actuation {
                Actuation::Intermittent {
                    range_degrees,
                    actuations,
                    reversing,
                } => {
                    let each = (range_degrees / 360.0) * to_output;
                    let n = f64::from(actuations);
                    (each * n, reversing.then_some((each, n)))
                }
                // A continuous drive turns at the speed it runs at, for as long
                // as it runs. There is no actuation to round within, so reversing
                // has nothing to mean here — the toggle is offered only where it
                // does.
                Actuation::Continuous { runtime_hours, .. } => {
                    // Each shaft's own speed, scaled from the peak the train was
                    // laid out at to the speed it actually runs at.
                    let scale = if train.input_speed == 0.0 {
                        0.0
                    } else {
                        train.cyclic_speed().unwrap_or(0.0) / train.input_speed
                    };
                    (speeds[i] * scale * 60.0 * runtime_hours, None)
                }
            }
        });
        s.set_kinematics(speeds, cycles);
    }

    // Each stage's backlash, referred to whichever shaft is the output.
    //
    // Driven forward that is the last shaft, so a stage's contribution is
    // divided by everything downstream of it. Driven backward the *input* shaft
    // is the output, so the contribution is multiplied by everything upstream
    // instead: those shafts turn faster, and the same play is a larger angle
    // there. The last stage dominates either way, and by more going backward.
    let refer = |drive: Drive, pick: fn(&Backlash) -> f64| -> f64 {
        stages
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let stage = pick(s.backlash().get(drive));
                match drive {
                    Drive::Forward => {
                        let downstream: f64 =
                            stages[k + 1..].iter().map(StageResult::ratio).product();
                        stage / downstream
                    }
                    Drive::Backward => {
                        let upstream: f64 = stages[..k].iter().map(StageResult::ratio).product();
                        stage * upstream
                    }
                }
            })
            .sum()
    };

    Ok(TrainResult {
        total_ratio,
        output_speed: train.input_speed / total_ratio,
        output_torque: torque,
        total_efficiency,
        backlash: Directional::of(|d| Backlash {
            nominal: refer(d, |b| b.nominal),
            minimum: refer(d, |b| b.minimum),
            maximum: refer(d, |b| b.maximum),
        }),
        operating_torque_percent: train.operating_torque_percent(),
        notes,
        stages,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn library() -> MaterialLibrary {
        super::test_library()
    }

    /// These tests build spur trains, so they know the kind and say so once.
    fn spur(r: &StageResult) -> &SpurResult {
        r.as_spur().expect("this train's stages are all spur")
    }

    /// ...and the same for reaching into a stage's inputs.
    fn spur_input(s: &mut Stage) -> &mut SpurStage {
        match s {
            Stage::Spur(st) => st,
            _ => panic!("this train's stages are all spur"),
        }
    }

    /// **A back-driving load is the load, and the reverse is the forward
    /// construction with the roles swapped.**
    ///
    /// It enters at the output. The member *on* that shaft carries the applied
    /// torque itself — nothing has happened to it yet — and the member at the
    /// other end carries it referred by the ratio and cut by the mesh's loss
    /// **the backward way**, exactly as driving forward the output member
    /// carries the input's referred by the ratio and cut by the forward loss.
    /// One construction, read from either end.
    ///
    /// Two things this has caught, in opposite directions. It once *divided* by
    /// the backward efficiency, and a worm's is **zero** whenever it self-locks
    /// — the case a worm is chosen for — so the wheel reported **2.2e307 N·m**:
    /// finite, so it crossed the boundary as a number rather than as the `null`
    /// an infinity becomes, and drew on screen as a figure. The correction
    /// dropped the factor altogether, which left the *worm* claiming a shaft
    /// torque a locked mesh does not deliver.
    ///
    /// **Both ends of the friction range**, because a self-locking worm alone
    /// cannot tell "times `η_backward`" from "times nought": the loose fixture
    /// is what makes the factor a factor.
    #[test]
    fn a_self_locking_worm_reports_the_load_it_reacts() {
        let lib = library();
        // A worm that locks, and one loose enough to be driven backward — with a
        // locking worm behind it either way, since a load exists only where
        // something reacts it and a train that can be back-driven end to end
        // carries none of it.
        for friction in [0.16_f64, 0.02] {
            for applied in [0.5_f64, 3.0, 12.5] {
                let mut train = two_stage();
                train.stages = vec![
                    Stage::Worm(WormStage::default()),
                    Stage::Worm(WormStage {
                        sliding_friction: friction,
                        static_friction: friction,
                        ..WormStage::default()
                    }),
                ];
                train.back_driving_torque = applied;

                let r = solve_train(&train, &lib).expect("a train that solves");
                let w = r.stages[1].as_worm().expect("a worm stage");
                let locks = w.efficiency.locked().backward;
                assert_eq!(
                    locks,
                    friction > 0.1,
                    "the fixture must span both sides of the threshold"
                );

                let wheel = w.members[1]
                    .back_driving_torque
                    .expect("the stage reacts the load, so its output member carries it");
                assert!(
                    (wheel - applied).abs() < 1e-9,
                    "the wheel is on the output shaft and the load is {applied} N·m, \
                     but it reports {wheel}"
                );

                // ...and the worm carries it referred by the ratio and attenuated
                // by the loss the mesh takes carrying it that way.
                let worm = w.members[0]
                    .back_driving_torque
                    .expect("likewise the input member");
                let want = wheel / w.ratio * w.efficiency.backward.max(0.0);
                assert!(
                    (worm - want).abs() < 1e-9 * applied,
                    "the worm reports {worm} where {wheel} at the wheel over a \
                     ratio of {} at {:.4} backward efficiency is {want}",
                    w.ratio,
                    w.efficiency.backward
                );
                assert_eq!(
                    locks,
                    worm == 0.0,
                    "a locked mesh delivers nothing to the worm's shaft, and an \
                     unlocked one delivers something: {worm} N·m at μ = {friction}"
                );
            }
        }
    }

    /// **Crossing the shafts does not stop a gear being a gear.**
    ///
    /// A crossed pair is a spur stage with an axis angle, and it is solved by
    /// translating it into the equivalent screw pair — which carries no tooth
    /// form, because a worm is a thread. So the translation dropped the shift,
    /// the dedendum and the root round on the way, and with them everything the
    /// members had to say about themselves: **the same pair reported
    /// `clamp.tooth_undercut` with its shafts parallel and nothing at all with
    /// them crossed.**
    ///
    /// `docs/corrections.md` records that gap being closed once already — "a gear
    /// in a geartrain never said it was undercut … reported now on every
    /// rack-cut member of every stage kind". It reached four kinds of five.
    ///
    /// The shaft angle is the helix angle doubled, so a crossed pair's teeth are
    /// genuinely *different* teeth from the parallel pair's — at 45° of helix a
    /// 9-tooth pinion is not undercut at all, because the transverse pressure
    /// angle has risen to 27°. So this is not an equality between the two
    /// answers. It is the claim that **the crossed member reports its own
    /// tooth's clamps**, checked at a shaft angle mild enough that the tooth is
    /// undercut on both sides of the comparison.
    #[test]
    fn a_crossed_pair_says_what_a_parallel_one_says_about_its_teeth() {
        let lib = library();
        let pair = |shaft_angle: f64| {
            let mut sp = SpurStage {
                shaft_angle,
                ..SpurStage::default()
            };
            // Small enough at zero shift to be eaten into by a standard rack.
            sp.gears[0].teeth = 9;
            sp.gears[0].profile_shift = Auto::fixed(0.0);
            sp.gears[0].no_undercut = false;
            sp.gears[1].teeth = 23;
            let mut t = two_stage();
            t.stages = vec![Stage::Spur(sp)];
            t
        };

        let flat = solve_train(&pair(0.0), &lib).expect("the parallel pair solves");
        let spur = flat.stages[0]
            .as_spur()
            .expect("parallel answers as a spur result");
        let said: Vec<&str> = spur.gears[0]
            .clamps
            .iter()
            .chain(&spur.gears[0].notes)
            .map(|n| n.key.as_str())
            .collect();
        assert!(
            said.contains(&"clamp.tooth_undercut"),
            "the fixture must undercut its pinion or it checks nothing: {said:?}"
        );

        // 20°: a 10° helix, so the transverse geometry is near enough the
        // parallel one that the same pinion is still undercut.
        let angled = solve_train(&pair(20.0), &lib).expect("the crossed pair solves");
        let screw = angled.stages[0]
            .as_worm()
            .expect("a crossed pair answers as a screw result");
        let gear = screw.members[0]
            .gear
            .as_ref()
            .expect("a crossed pair's members are gears");
        let crossed: Vec<&str> = gear
            .clamps
            .iter()
            .chain(&gear.notes)
            .map(|n| n.key.as_str())
            .collect();
        assert!(
            crossed.contains(&"clamp.tooth_undercut"),
            "the same pinion, shafts crossed, says {crossed:?} — an axis angle \
             is not what decides whether a cutter ate into a flank"
        );
    }

    /// **Every member of a stage that reacts a load reports its share of it.**
    ///
    /// A walk over `StageResult::members()` rather than over five named field
    /// paths, which is what that accessor is for: the fault it is looking for is
    /// a member quietly missing a figure, and a sweep that names the kinds can
    /// only miss it in the kind nobody named. F30 — a self-locking worm's wheel
    /// reporting 2.2e307 N·m — was in the fifth.
    ///
    /// The claim is deliberately the weak one: **present, finite, and signed
    /// like the stage's.** A *quantitative* law across kinds does not exist, and
    /// finding that out is what this test cost. A parallel-axis member's forward
    /// torque is a geometric projection with no efficiency in it, so the ratio
    /// of backward to forward is the same for both members. A screw pair's
    /// output torque carries a forward efficiency that the backward load does
    /// not share, so its two members differ by exactly `1/η_forward` — by
    /// construction, and correctly. See [`StageTorques::referred_like`].
    #[test]
    fn every_member_of_a_reacting_stage_reports_its_share() {
        let lib = library();
        let mut train = two_stage();
        train.back_driving_torque = 0.5;
        // **A self-locking stage at the input end**, or nothing reacts the load
        // and the case is correctly zero at every gear — which would leave this
        // walking an empty list and passing for the wrong reason. The load
        // enters at the output and walks up; a worm stops it, and every stage
        // between it and the output carries it.
        train.stages.insert(0, Stage::Worm(WormStage::default()));
        train.stages.push(Stage::Planetary(Box::default()));
        train.stages.push(Stage::Spur(SpurStage {
            shaft_angle: 90.0,
            ..SpurStage::default()
        }));

        let r = solve_train(&train, &lib).expect("a train that solves");
        let (mut checked, mut kinds) = (0u32, 0u32);
        for (k, stage) in r.stages.iter().enumerate() {
            let members = stage.members();
            if members.is_empty() {
                continue;
            }
            kinds += 1;
            for g in members {
                let back = g.back_driving_torque.unwrap_or_else(|| {
                    panic!("stage {k}: a member of a stage that reacts the load reports none")
                });
                checked += 1;
                assert!(
                    back.is_finite(),
                    "stage {k}: a member reports {back} N·m of back-driving torque"
                );
                assert!(
                    g.torque == 0.0 || back.signum() == g.torque.signum(),
                    "stage {k}: {back} against a forward torque of {} — a reacted \
                     load that turns a gear the other way from the drive is a \
                     claim, not a rounding",
                    g.torque
                );
            }
        }
        assert!(kinds >= 3, "only {kinds} stage kinds contributed members");
        assert!(checked >= 6, "only {checked} members carried the load");
    }

    /// **The search refines what its sweep found, and a narrow box does not stop
    /// it.**
    ///
    /// Checked against a scan of the same admissible interval at the search's own
    /// stopping distance — an instrument that shares no step, no direction and no
    /// budget with the walk, which is the kind of check this repository trusts.
    ///
    /// **The fault it is for.** The walk's first step is a fraction of the
    /// sweep's spacing, and the sweep's spacing is a fraction of the *box*. Pin a
    /// centre distance and the box is a fraction of a module wide, at which point
    /// that first step falls **below the distance the walk stops at** — so
    /// `step > resolution` was false before the body ran, the walk took no step
    /// at all, and the answer was the best of thirteen grid points. On 9/37 at a
    /// shift sum of 0.56 the optimum sits four ten-thousandths above the undercut
    /// floor, between two of them: **3.6e-5** of efficiency, on the path a given
    /// centre distance takes.
    ///
    /// The tolerance is the resolution's own worth. Near the floor the efficiency
    /// moves by a few parts in a million across one thousandth of a module, and
    /// no search that stops there can do better — which is a statement about the
    /// stopping distance rather than about the walk.
    #[test]
    fn the_search_beats_a_scan_of_the_same_interval() {
        use crate::auto::{Bounds, Pinned, Search};
        let lib = library();
        let mut worst = 0.0_f64;
        for teeth in [[9_u32, 37], [12, 29]] {
            let stage = SpurStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..SpurStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..SpurStage::default()
            };
            let asked = [0, 1].map(|i| stage.gears[i].shift_asked(&stage.base_params(i)));
            let bounds = Bounds {
                floor: asked.map(|a| a.search_floor),
                min_contact_ratio: stage.optimisation.min_contact_ratio,
                clearance: stage.clearance,
            };
            let pair = |q: [f64; 2]| [0, 1].map(|i| stage.params_at(i, q[i]));
            // The stage's own answer at a shift pair, and `None` where the pair
            // is one no search may choose — asked of the search itself with both
            // shifts pinned, so the scan and the walk agree about what is
            // admissible and differ only in how they look.
            let at = |x: [f64; 2]| {
                crate::auto::shifts_for_efficiency(
                    &pair,
                    crate::mesh::MeshKind::External,
                    &bounds,
                    &Pinned {
                        shift: [Some(x[0]), Some(x[1])],
                        sum: None,
                    },
                    stage.sliding_friction,
                    &Search::SHIPPED,
                )?;
                let fixed = SpurStage {
                    gears: [0, 1].map(|i| StageGear {
                        profile_shift: Auto::fixed(x[i]),
                        ..stage.gears[i].clone()
                    }),
                    optimisation: Optimisation::default(),
                    ..stage.clone()
                };
                solve_spur_stage(&fixed, StageTorques::just(2.0), &lib)
                    .ok()
                    .map(|r| r.mesh.efficiency.forward)
            };

            for k in 0..=8 {
                let sum = 0.5 + f64::from(k) * 0.075;
                let found = crate::auto::shifts_for_efficiency(
                    &pair,
                    crate::mesh::MeshKind::External,
                    &bounds,
                    &Pinned {
                        shift: [None, None],
                        sum: Some(sum),
                    },
                    stage.sliding_friction,
                    &Search::SHIPPED,
                )
                .and_then(at);
                // The scan, over the whole interval either shift could take, at
                // the step the walk stops at.
                let mut best: Option<f64> = None;
                let mut x0 = -0.5;
                while x0 < 1.5 {
                    if let Some(e) = at([x0, sum - x0]) {
                        best = Some(best.map_or(e, |b: f64| b.max(e)));
                    }
                    x0 += Search::SHIPPED.resolution;
                }
                let (Some(found), Some(best)) = (found, best) else {
                    continue;
                };
                worst = worst.max(best - found);
                assert!(
                    best - found < 1e-5,
                    "{teeth:?} at a shift sum of {sum}: the search keeps {found} \
                     where a scan of the same interval finds {best}"
                );
            }
        }
        assert!(
            worst > 0.0,
            "the scan never beat the search anywhere, so it is not measuring what \
             it is meant to"
        );
    }

    /// **Every kind that searches asks the same of its meshes.**
    ///
    /// A constraint belongs to the mesh, not to the arrangement around it: a
    /// mesh whose teeth reach past the root circle they run into bottoms out
    /// whether a carrier is turning about it or not. Three kinds each wrote the
    /// question out for themselves and between them answered it three ways — the
    /// pair asked about bottoming, neither epicyclic kind did; the pair and the
    /// set asked whether each member could be cut, and a ring was asked by
    /// nobody.
    ///
    /// `auto::MeshTrial` is the one place now, and this is the claim that says
    /// so from the outside: **whatever each kind chose, the mesh it chose is one
    /// the shared contract admits.** A kind that stops asking chooses something
    /// that fails here; a kind added later that never asks fails here the first
    /// time its answer is pushed against a bound.
    #[test]
    fn every_kind_that_searches_asks_the_same_of_its_meshes() {
        use crate::auto::Search;
        let lib = library();
        let mut checked = 0u32;

        // --- a pair, rebuilt through the stage's own constructors.
        for teeth in [[9_u32, 37], [17, 43], [12, 29]] {
            let stage = SpurStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..SpurStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..SpurStage::default()
            };
            let x = stage.shifts_at(&Search::SHIPPED);
            let g = [0, 1].map(|i| Tooth::new(stage.params_at(i, x[i])));
            let zero = crate::mesh::Mesh::new(&g[0], &g[1], crate::mesh::MeshKind::External)
                .expect("the pair meshes");
            let mesh = zero
                .at(zero.a_w + stage.clearance)
                .expect("...at its running distance");
            for (i, gap) in mesh
                .bottom_clearance([g[0].ra, g[1].ra], [g[0].rf, g[1].rf])
                .iter()
                .enumerate()
            {
                assert!(
                    *gap > 0.0,
                    "{teeth:?}: gear {i}'s tip bottoms out by {} mm at the shifts \
                     the search chose",
                    -gap
                );
                checked += 1;
            }
        }

        // --- an epicyclic set, both of its meshes.
        for (sun, planet) in [(17_u32, 17_u32), (24, 18), (13, 25)] {
            let mut set = PlanetaryStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..PlanetaryStage::default()
            };
            set.sun.teeth = sun;
            set.planet.teeth = planet;
            set.ring.teeth = sun + 2 * planet;
            set.sun.profile_shift = Auto::automatic(0.0);
            set.ring.profile_shift = Auto::automatic(0.0);
            let b = set
                .built(set.shifts_at(&Search::SHIPPED))
                .expect("the set has geometry");
            let gaps = [
                b.sp_mesh
                    .bottom_clearance([b.sun.ra, b.planet.ra], [b.sun.rf, b.planet.rf]),
                b.pr_mesh
                    .bottom_clearance([b.planet.ra, b.ring.ra], [b.planet.rf, b.ring.rf]),
            ];
            for (m, mesh) in gaps.iter().enumerate() {
                for (i, gap) in mesh.iter().enumerate() {
                    assert!(
                        *gap > 0.0,
                        "{sun}/{planet}: mesh {m} member {i} bottoms out by {} mm",
                        -gap
                    );
                    checked += 1;
                }
            }
        }

        // --- a hula stage, from what it reports rather than from what built it.
        // Its two meshes share one crank, so the offset *is* their centre
        // distance, and the two radii are on the members' own cards.
        for n in [12_u32, 18, 30] {
            let mut stage = HulaStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..HulaStage::default()
            };
            for (gear, count) in stage.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                gear.teeth = count;
            }
            let Ok(r) = solve_hula_stage(&stage, 1000.0, StageTorques::just(2.0), &lib) else {
                continue;
            };
            for mesh in 0..2 {
                // The ring is the member of the pair with more teeth, which the
                // stage's own inputs say and its cards echo as the larger tip.
                let (ring, pinion) = {
                    let (a, b) = (&r.gears[mesh * 2], &r.gears[mesh * 2 + 1]);
                    if stage.gears[mesh * 2].teeth > stage.gears[mesh * 2 + 1].teeth {
                        (a, b)
                    } else {
                        (b, a)
                    }
                };
                let gaps = [
                    (ring.root_radius - r.offset) - pinion.tip_radius,
                    (ring.tip_radius - r.offset) - pinion.root_radius,
                ];
                for (i, gap) in gaps.iter().enumerate() {
                    assert!(
                        *gap > 0.0,
                        "N {n} mesh {mesh} member {i} bottoms out by {} mm",
                        -gap
                    );
                    checked += 1;
                }
            }
        }

        assert!(checked >= 20, "only {checked} tooth spaces were measured");
    }

    /// **A search may not choose a part its tool has to alter.**
    ///
    /// `member_is_buildable` says this of a rack-cut member and declines to say
    /// it of a ring, on the reading that a rack's four questions mean nothing to
    /// one. True — and it left a ring asked *nothing at all*, so the search was
    /// free to walk past the shift where the shaper stops leaving the space that
    /// shift asked for. It did: **26 of these 30 sets** came back with a ring
    /// the cutter had capped, the shipped 13/25 choosing 2.35 modules where its
    /// space caps at 1.94. The efficiency reported for them is the efficiency of
    /// a part nobody makes.
    ///
    /// A ring is asked of its **cutter** instead (`auto::ring_is_cut_as_asked`),
    /// which is the one question it has an answer to, and it is free: every
    /// caller has already cut the ring to get the mesh it is scoring.
    ///
    /// Both epicyclic kinds, because the last time a rule reached one of them
    /// and not the other it cost a second entry in `docs/corrections.md`.
    #[test]
    fn a_search_chooses_only_parts_its_tool_leaves_alone() {
        let lib = library();
        let mut checked = 0u32;
        for sun in [11_u32, 13, 17, 19, 24, 31] {
            for planet in [14_u32, 17, 18, 21, 25] {
                let mut set = PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: true,
                        ..Optimisation::default()
                    },
                    ..PlanetaryStage::default()
                };
                set.sun.teeth = sun;
                set.planet.teeth = planet;
                set.ring.teeth = sun + 2 * planet;
                set.sun.profile_shift = Auto::automatic(0.0);
                set.ring.profile_shift = Auto::automatic(0.0);
                let Ok(r) = solve_planetary_stage(&set, 1000.0, StageTorques::just(2.0), &lib)
                else {
                    continue;
                };
                checked += 1;
                for (name, g) in [
                    ("sun", &r.sun),
                    ("planet", &r.planet.gear),
                    ("ring", &r.ring),
                ] {
                    assert!(
                        g.clamps.is_empty(),
                        "{sun}/{planet}: the search chose a {name} at x = {} that its \
                         tool had to alter — {:?}",
                        g.profile_shift,
                        g.clamps.iter().map(|c| c.key.clone()).collect::<Vec<_>>()
                    );
                }
            }
        }
        assert!(checked >= 25, "only {checked} sets solved at all");
    }

    /// **Ask for the centre distance the tool chose and get the gears it chose.**
    ///
    /// A stage with the shift optimiser on and no centre distance picks both;
    /// with a distance given it picks the shifts that reach it. Those are the
    /// same question asked twice, so at the distance the free search *itself*
    /// settled on the two must agree — and the second must not be worse anywhere
    /// short of it, since a nearer distance is a shift sum the free search
    /// passed through on its way.
    ///
    /// **It did not.** `auto::maximise` opened on a grid at multiples of half a
    /// module across a fixed `±3`; pin the sum and the admissible interval in one
    /// shift can be a tenth of that and fall between two grid points, at which
    /// point the search reports *no admissible pair* and the stage drops to its
    /// undercut floor. A 9/37 pair asked for the 24.42 mm it had just
    /// recommended came back at **97.289 %** against the **97.706 %** it offered
    /// unasked, at shifts that do not reach the distance at all. The sweep's
    /// resolution had become a feasibility test (`docs/corrections.md`).
    ///
    /// The box is the members' own admissible interval now
    /// (`auto::searchable_shift`), so the sweep spans the answer by construction.
    #[test]
    fn a_given_distance_gets_the_gears_the_free_search_would_choose() {
        let lib = library();
        for teeth in [[9_u32, 37], [17, 43], [12, 29], [23, 61]] {
            let stage = SpurStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..SpurStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..SpurStage::default()
            };
            let free = solve_spur_stage(&stage, StageTorques::just(2.0), &lib)
                .expect("the pair solves with the distance free");

            let at = |a: f64| {
                solve_spur_stage(
                    &SpurStage {
                        centre_distance: Auto::fixed(a),
                        ..stage.clone()
                    },
                    StageTorques::just(2.0),
                    &lib,
                )
                .expect("...and with it given")
            };

            // At the distance it chose: the same answer, to the search's own
            // stopping distance in the shifts it is reading.
            let given = at(free.centre_distance);
            for i in 0..2 {
                let (a, b) = (free.gears[i].profile_shift, given.gears[i].profile_shift);
                assert!(
                    (a - b).abs() < crate::auto::Search::SHIPPED.resolution,
                    "{teeth:?} gear {i}: free chose {a} and {:.6} mm given chose {b}",
                    free.centre_distance
                );
            }

            // ...and on the way there, every distance is answered rather than
            // refused, with an efficiency that climbs toward the free answer
            // rather than collapsing to the floor.
            let mut last = 0.0;
            let steps = 12;
            for k in 1..=steps {
                let a = free.centre_distance
                    - (free.centre_distance - stage.module * 0.5 * f64::from(teeth[0] + teeth[1]))
                        * f64::from(steps - k)
                        / f64::from(steps);
                let here = at(a).mesh.efficiency.forward;
                assert!(
                    here > last - 1e-9,
                    "{teeth:?}: {a:.4} mm gives {here}, below the {last} a tighter \
                     distance gave — the search dropped out somewhere between"
                );
                last = here;
            }
            assert!(
                (last - free.mesh.efficiency.forward).abs() < 1e-6,
                "{teeth:?}: the last step reaches {last} where the free answer is {}",
                free.mesh.efficiency.forward
            );
        }
    }

    /// **The search is converged where its coordinates are the problem's, and
    /// not where they are not.**
    ///
    /// `auto::Search` carries six numbers that say how hard to look, and the
    /// claim beside them was that raising them together buys nothing — "the
    /// pair's answer not at all to eight decimals, the set's by 2e-7". That was
    /// a comment: the numbers were constants inside the loop, so nothing could
    /// raise them and nothing could check it. They are a value for this reason
    /// alone, and checking it found the claim half true.
    ///
    /// **The claim is about the objective, not about the point.** These surfaces
    /// have flat ridges and their optima sit against constraints, so two
    /// different shifts can be equally good and demanding that the *shift* not
    /// move would assert something the model does not say. What a converged
    /// search means is that more effort finds nothing better — and nothing
    /// worse either, since more effort searching the same set cannot lose an
    /// answer it already had, and a search that does depends on its own step
    /// size for more than speed.
    ///
    /// **A pair converges and a set does not**, and the difference is
    /// coordinates. A pair is searched in its *own* two directions — the shift
    /// sum, which sets the operating pressure angle, and the division, which
    /// moves the path's ends against each other — so the flat direction is an
    /// axis and a walk climbs rather than zig-zags. A set is searched in two of
    /// its three raw shifts, which is nobody's natural coordinate: its
    /// admissible region is bounded by a curve and the optimum lies against it,
    /// so the walk slides. `AUDIT.md` F50 carries the size, the sweep and what
    /// is to be done; the bound below is a **canary on a known fault**, pinned
    /// so it can only get smaller.
    #[test]
    fn the_search_is_converged_not_budgeted() {
        use crate::auto::Search;
        let lib = library();
        // Fourteen times the work: a sweep of 18 a side, four starts, nine times
        // the walk and a third of the stopping distance.
        let hard = Search::refined(3);
        // A part in a million of efficiency — three orders below anything a mesh
        // is measured to. Measured across the pairs below, the worst is 4.1e-7
        // at 9/20 and every shift agrees to within one step of the search's own
        // resolution, so this is a ceiling on the last refinement rather than a
        // tolerance chosen to be met.
        let converged = 1e-6;

        // --- pairs. 9/37 is the fixture whose surface has a second summit
        // beyond a trough, so it is the one that punishes a short walk.
        let mut worst_pair = 0.0_f64;
        for teeth in [
            [9_u32, 37],
            [9, 20],
            [11, 41],
            [12, 29],
            [13, 31],
            [17, 43],
            [17, 17],
            [20, 20],
            [23, 61],
            [31, 37],
            [41, 43],
            [10, 51],
            [14, 22],
            [16, 33],
        ] {
            let stage = SpurStage {
                gears: [0, 1].map(|i| StageGear {
                    teeth: teeth[i],
                    ..SpurStage::default().gears[i].clone()
                }),
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                ..SpurStage::default()
            };
            // Scored by solving the stage at the shifts each search chose, so
            // the objective is the one the tool reports rather than a second
            // spelling of it.
            let at = |x: [f64; 2]| {
                let fixed = SpurStage {
                    gears: [0, 1].map(|i| StageGear {
                        profile_shift: Auto::fixed(x[i]),
                        ..stage.gears[i].clone()
                    }),
                    optimisation: Optimisation::default(),
                    ..stage.clone()
                };
                solve_spur_stage(&fixed, StageTorques::just(2.0), &lib)
                    .map(|r| r.mesh.efficiency.forward)
            };
            let (Ok(shipped), Ok(refined)) = (
                at(stage.shifts_at(&Search::SHIPPED)),
                at(stage.shifts_at(&hard)),
            ) else {
                panic!("{teeth:?}: both answers must be buildable pairs");
            };
            worst_pair = worst_pair.max((refined - shipped).abs());
            assert!(
                (refined - shipped).abs() < converged,
                "{teeth:?}: fourteen times the work moves the pair's efficiency \
                 by {}, which is more than the search claims to be converged to",
                refined - shipped
            );
        }
        assert!(
            worst_pair > 0.0,
            "every pair gave bit-identical answers at both efforts, so this \
             fixture never made the search work"
        );

        // --- sets. Two free shifts tied by one centre distance, and the harder
        // of the two searches: its admissible region is bounded by a curve its
        // planet's absorption draws, so a walk slides along that curve instead
        // of climbing an axis, and a walk that slides is a walk that costs.
        //
        // **Its budget is asked for separately**, and that is the claim worth
        // making: quadrupled, it moves nothing *at all*. A guard says that; a
        // truncation cannot, and this was a truncation — a pool shared across
        // the starts, spent entire by the first of six, leaving 3.2e-4 of `η₀`
        // unfound on the shipped set (F50, `docs/corrections.md`).
        //
        // What is left over is a little wider than a pair's, and it is the last
        // step of a different grid rather than under-search: it moves **both
        // ways** as the sweep is refined, by 5e-7 either side.
        let settled = 2e-6;
        let mut worst_set = 0.0_f64;
        for sun in [11_u32, 13, 17, 19, 24, 31] {
            for planet in [14_u32, 17, 18, 21, 25] {
                let mut set = PlanetaryStage {
                    optimisation: Optimisation {
                        enabled: true,
                        ..Optimisation::default()
                    },
                    ..PlanetaryStage::default()
                };
                set.sun.teeth = sun;
                set.planet.teeth = planet;
                set.ring.teeth = sun + 2 * planet;
                set.sun.profile_shift = Auto::automatic(0.0);
                set.ring.profile_shift = Auto::automatic(0.0);
                let eta0 = |x: [f64; 3]| {
                    let b = set.built(x).ok()?;
                    let one = |path, mesh, g, mu| {
                        crate::contact::efficiency(
                            path,
                            mesh,
                            g,
                            mu,
                            crate::contact::Drive::Forward,
                        )
                    };
                    Some(
                        one(
                            &b.sp_path,
                            &b.sp_mesh,
                            &b.sun,
                            set.sliding_friction_sun_planet,
                        ) * one(
                            &b.pr_path,
                            &b.pr_mesh,
                            &b.planet,
                            set.sliding_friction_planet_ring,
                        ),
                    )
                };
                let (Some(shipped), Some(refined)) = (
                    eta0(set.shifts_at(&Search::SHIPPED)),
                    eta0(set.shifts_at(&hard)),
                ) else {
                    continue;
                };
                worst_set = worst_set.max((refined - shipped).abs());
                assert!(
                    (refined - shipped).abs() < settled,
                    "{sun}/{planet}: fourteen times the work moves `η₀` by {}",
                    refined - shipped
                );
                // ...and the budget alone, which must not move it by a bit. It
                // is the one number here that is a guard rather than a
                // resolution, so it is the one that can be asked for exactly.
                let generous = eta0(set.shifts_at(&Search {
                    budget: Search::SHIPPED.budget * 4,
                    ..Search::SHIPPED
                }));
                assert_eq!(
                    generous,
                    Some(shipped),
                    "{sun}/{planet}: four times the budget moves the answer, so it \
                     is a ceiling the walk is running into rather than a guard"
                );
            }
        }
        assert!(
            worst_set > 0.0,
            "every set gave bit-identical answers at both efforts, so this \
             fixture never made the search work"
        );
    }

    /// **A member is rated at the load it carries, whichever way it carries it.**
    ///
    /// Every rating here is linear in the member's own torque, or the square root
    /// of it, so with the duty torque set to the peak the two load cases stand in
    /// exactly the ratio of the two torques the member reports — its forward one,
    /// and the worse of that and its share of a back-driving load. Checkable from
    /// the outputs alone, without knowing what any of them should be.
    ///
    /// **The fault it is for.** A load case was collapsed to one magnitude *at
    /// the stage's input shaft* and that magnitude pushed through the forward
    /// construction. That is the same answer only where the distribution does not
    /// depend on direction — a parallel-axis mesh carries one tangential force
    /// whichever way it turns — and an epicyclic set is not such a stage: which
    /// shaft drives decides on which side `η₀` multiplies. Measured on the shipped
    /// set, a back-driven ring was rated **6.0 % low** in bending and 3.0 % low in
    /// contact; on the hula stage, whose reduction is far larger, **41 % low** and
    /// 23 % low. The member torques themselves had already been put right
    /// (`docs/corrections.md`); the ratings had not, and nothing compared them.
    ///
    /// Every kind that reports per-member stresses is walked, through
    /// `StageResult::members()` rather than five named paths, for the reason that
    /// accessor exists.
    #[test]
    fn a_member_is_rated_at_the_load_it_carries() {
        let lib = library();
        // **Both sides of the maximum.** A small load leaves every member loaded
        // hardest driving forward, where the law reads "the two cases are equal"
        // and is a control; a large one puts every member on the backward
        // distribution, which is the case the fault was in.
        let (mut checked, mut dominated, mut forward_won) = (0u32, 0u32, 0u32);
        for applied in [1.0e2_f64, 1.0e10] {
            let mut train = two_stage();
            train.back_driving_torque = applied;
            train.operating_torque = train.input_torque;
            train.stages.insert(0, Stage::Worm(WormStage::default()));
            train.stages.push(Stage::Planetary(Box::default()));
            train.stages.push(Stage::Hula(Box::default()));

            let r = solve_train(&train, &lib).expect("a train that solves");
            for (k, stage) in r.stages.iter().enumerate() {
                for (i, g) in stage.members().iter().enumerate() {
                    let Some(back) = g.back_driving_torque else {
                        continue;
                    };
                    let forward = g.torque.abs();
                    assert!(
                        forward > 0.0,
                        "stage {k} member {i} carries nothing driving forward, so \
                     this law has no ratio to check"
                    );
                    let want = forward.max(back.abs()) / forward;
                    if want > 1.0 {
                        dominated += 1;
                    } else {
                        forward_won += 1;
                    }
                    checked += 1;
                    if let (Some(peak), Some(cyclic)) =
                        (g.bending_stress.peak, g.bending_stress.cyclic)
                    {
                        assert!(
                            (peak / cyclic - want).abs() < 1e-9 * want,
                            "stage {k} member {i}: bending {peak} against {cyclic} is \
                         {}, where {forward} N·m forward and {back} N·m backward \
                         make {want}",
                            peak / cyclic
                        );
                    }
                    let (peak, cyclic) = (g.contact_stress.peak, g.contact_stress.cyclic);
                    assert!(
                        (peak / cyclic - want.sqrt()).abs() < 1e-9 * want,
                        "stage {k} member {i}: contact {peak} against {cyclic} is {}, \
                     where the torques make {}",
                        peak / cyclic,
                        want.sqrt()
                    );
                }
            }
        }
        assert!(checked >= 18, "only {checked} members carried the load");
        assert!(
            dominated >= 9 && forward_won >= 9,
            "{dominated} members were loaded harder backward and {forward_won} \
             harder forward — both sides of the maximum have to be reached or \
             this only checks one of them"
        );
    }

    /// **A back-driving load reaches the wheel undiminished**, and the only
    /// thing the direction changes about the rating is which flank carries it.
    ///
    /// A screw pair is the kind whose distribution depends on direction: driving
    /// forward the wheel carries the worm's torque stepped up *and cut by the
    /// mesh's own loss*, while a back-driving load arrives at the wheel already.
    /// So a stage rated at `max(T_in, T_back)` and then stepped up and cut once
    /// is rating a back-driven pair at `η_forward` of its load — 62 % of it on
    /// the shipped worm, and 85 % of the pressure once a cube root has been
    /// through it.
    ///
    /// Asserted by putting the same torque on the wheel twice, from either end.
    /// The two answers are **not** identical and should not be: back-driving
    /// loads the other flank, which flips the normal term in the balance and
    /// leaves the friction term alone, and this pair's two flanks differ by half
    /// a percent. What makes this a test rather than a tolerance is the second
    /// bound — the gap between the two readings has to stay far smaller than the
    /// gap the old fault opened, or "the same load, the other flank" and "the
    /// wrong load" are not being told apart.
    #[test]
    fn a_back_driving_load_reaches_the_wheel_undiminished() {
        let lib = library();
        let load = 400.0;

        let driven = |input_torque: f64, back: f64| {
            let mut train = two_stage();
            train.input_torque = input_torque;
            train.operating_torque = input_torque;
            train.back_driving_torque = back;
            train.stages = vec![Stage::Worm(WormStage::default())];
            let r = solve_train(&train, &lib).expect("a train that solves");
            let w = r.stages[0].as_worm().expect("a worm stage");
            (w.contact.peak.max_pressure, w.ratio, w.efficiency.forward)
        };

        // Driven backward at `load` on the wheel, with nothing at all coming the
        // other way — zero is a torque, and a stage carrying none of it forward
        // still carries this.
        let (from_output, ratio, eta) = driven(0.0, load);
        // ...and driven forward hard enough to put the same torque on the wheel.
        let (from_input, _, _) = driven(load / (ratio * eta), 0.0);

        let apart = (from_output / from_input - 1.0).abs();
        assert!(
            apart < 0.02,
            "the same {load} N·m on the wheel rates at {from_output} MPa reached              from the output and {from_input} MPa reached from the input"
        );
        // The fault this is really about: `η` of the load, under a cube root.
        let old_fault = 1.0 - eta.cbrt();
        assert!(
            apart < old_fault / 5.0,
            "a flank swap moves the rating by {apart}, and rating the load at              η_forward would move it by {old_fault} — too close to tell apart"
        );

        // And it is the load itself that arrives, so the rating follows Hertz's
        // cube root of it exactly rather than approximately.
        let (doubled, _, _) = driven(0.0, 2.0 * load);
        assert!(
            (doubled / from_output - 2.0_f64.cbrt()).abs() < 1e-12,
            "twice the back-driving load should be 2^(1/3) of the pressure: {}",
            doubled / from_output
        );
    }

    /// **A pair that transmits nothing still has its flanks pressed.**
    ///
    /// A screw mesh's rating used to be taken from the torque on its **wheel**,
    /// `T_in · i · η_forward` — and `Directional::once_moving` clamps a locked
    /// pair's efficiency to zero, so that product is zero and the pair reported
    /// no flank load at all. It is not zero: something is holding the wheel, and
    /// whatever holds it is pressing the teeth.
    ///
    /// Reached at a helix split of 9° / 81° on a crossed 17/23 pair, which
    /// `gear-cli crossed 17 23 90` prints — the sliding is over six times the
    /// pitch-line speed there and the mesh cannot drive forward at µ = 0.06.
    /// Rated from the torque the stage was **given**, on the member it was given
    /// on, the answer is in line with its neighbours in that sweep rather than
    /// absent from it.
    #[test]
    fn a_pair_that_transmits_nothing_still_has_its_flanks_pressed() {
        let lib = library();
        let locked = WormStage {
            shaft_angle: 90.0,
            starts: 17,
            wheel_teeth: 23,
            sizing: FirstMemberSizing::HelixAngle(9.0),
            ..WormStage::default()
        };
        let r = solve_worm_stage(&locked, StageTorques::just(2.0), &lib)
            .expect("a locked pair is still a pair");
        assert_eq!(
            r.efficiency.forward, 0.0,
            "this split is meant to be the forward-locked one"
        );
        assert!(
            r.contact.peak.max_pressure > 100.0,
            "a locked pair's flanks are pressed by whatever holds them: {} MPa",
            r.contact.peak.max_pressure
        );
        // Not merely non-zero: the same 2 N·m through a split that *does* drive
        // presses about as hard, because the flank load comes from the input
        // torque either way and the geometry has not changed much.
        let driving = WormStage {
            sizing: FirstMemberSizing::HelixAngle(18.0),
            ..locked
        };
        let d = solve_worm_stage(&driving, StageTorques::just(2.0), &lib).expect("and this one");
        let ratio = r.contact.peak.max_pressure / d.contact.peak.max_pressure;
        assert!(
            (0.5..2.0).contains(&ratio),
            "the locked split rates at {} MPa against the driving split's {}",
            r.contact.peak.max_pressure,
            d.contact.peak.max_pressure
        );
    }

    /// **Zero is a torque.**
    ///
    /// A train has an operating torque as well as a peak, and nothing stops it
    /// being nought: a mechanism that is held rather than driven runs at no load
    /// and still has to be rated for the peak it sees. Every parallel-axis kind
    /// answered; a worm stage returned `NoContact` and took the whole train down
    /// with it, because the Hertz point solution refused a zero force where the
    /// limit is a closed form — the patch closes to a point and the pressure with
    /// it (`crate::hertz::elliptical_contact`).
    ///
    /// Asserted on every kind rather than on the one that failed, since what is
    /// being claimed is a property of the tool and not a patch to a stage.
    #[test]
    fn a_stage_carrying_nothing_is_a_stage() {
        let lib = library();
        for stage in [
            Stage::Spur(SpurStage::default()),
            Stage::Worm(WormStage::default()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::default()),
        ] {
            let mut train = two_stage();
            train.stages = vec![stage];
            train.operating_torque = 0.0;
            let r = solve_train(&train, &lib)
                .unwrap_or_else(|e| panic!("a stage at no operating load: {e:?}"));
            // A worm's members are not gears, so the walk below is empty there
            // and the claim is the stage's own — which is the one that failed.
            if let Some(w) = r.stages[0].as_worm() {
                assert_eq!(w.contact.cyclic.max_pressure, 0.0);
                assert!(w.contact.peak.max_pressure > 0.0);
            }
            for (i, g) in r.stages[0].members().iter().enumerate() {
                assert_eq!(
                    g.contact_stress.cyclic, 0.0,
                    "member {i} carries nothing and reports a stress"
                );
                assert!(
                    g.bending_stress.cyclic.is_none_or(|s| s == 0.0),
                    "member {i} carries nothing and reports {:?}",
                    g.bending_stress.cyclic
                );
                assert!(
                    g.contact_stress.peak > 0.0,
                    "member {i} still has a peak to survive"
                );
            }
        }
    }

    /// **An internal mesh is asked what an internal mesh is asked, whatever is
    /// turning around it — and an external one is not asked it at all.**
    ///
    /// The three ways an internal pair's teeth can foul were four fields on a
    /// *hula stage's* mesh row, and the epicyclic set with the same ring mesh in
    /// it reported nothing: a designer was told whether the teeth foul according
    /// to which stage kind they had picked. They are on [`MeshReport`] now, and
    /// this is the walk that says every kind gets them — through
    /// [`StageResult::meshes`], which is the same shape as `members()` and
    /// exists so a walk cannot forget a kind by naming them.
    ///
    /// `None` on an external mesh is the other half of the claim, and it is not
    /// three answers of `false`: an external pair's members curve opposite ways,
    /// its tip circles cross on the line of centres or not at all, and the
    /// question does not arise. *An absent thing is not a zero-length thing.*
    #[test]
    fn an_internal_mesh_is_asked_what_an_internal_mesh_is_asked() {
        let lib = library();
        // Which of each kind's meshes have a ring in them, in the order
        // `meshes()` returns them: a pair none, a set its second, a hula both.
        for (stage, internal) in [
            (Stage::Spur(SpurStage::default()), vec![false]),
            (Stage::Planetary(Box::default()), vec![false, true]),
            (Stage::Hula(Box::default()), vec![true, true]),
            (Stage::Worm(WormStage::default()), vec![]),
        ] {
            let mut train = two_stage();
            train.stages = vec![stage];
            let r = solve_train(&train, &lib).expect("every shipped kind solves");
            let meshes = r.stages[0].meshes();
            assert_eq!(
                meshes.len(),
                internal.len(),
                "the walk found a different number of meshes than the kind has"
            );
            for (mesh, is_internal) in meshes.iter().zip(&internal) {
                assert_eq!(
                    mesh.tips.is_some(),
                    *is_internal,
                    "tip room is an internal mesh's question and only an internal mesh's"
                );
            }
        }
    }

    /// **A full-depth internal pair interferes, and a shipped epicyclic set is
    /// one.**
    ///
    /// `ring::mesh_with` has said so since it existed — the ring's tip can only
    /// touch the pinion's involute while `√(r_a2² − r_b2²) ≥ a sin α_w`, and a
    /// standard ring misses it — but nothing ever put the question to a *set*,
    /// whose default ring is full-depth at zero shift. So the answer here is
    /// `true`, and it is asserted rather than fixed: it is a statement about
    /// what the shipped proportions are, and the remedy is the ring's addendum,
    /// which is an input.
    ///
    /// Both halves are pinned, because a canary that only says "it interferes"
    /// would pass if every set interfered for a new reason. Shortening the ring
    /// clears it, which is the same remedy `ring.rs` records and is where the
    /// rule of thumb about internal tooth differences comes from.
    #[test]
    fn a_shipped_sets_full_depth_ring_interferes_and_a_shorter_tooth_clears_it() {
        let lib = library();
        let solved = |addendum: f64| {
            let mut set = PlanetaryStage::default();
            set.ring.addendum = addendum;
            crate::train::solve_planetary_stage(&set, 3000.0, StageTorques::just(2.0), &lib)
                .expect("the shipped set solves")
        };
        let full = solved(1.0).planet_ring.tips.expect("an internal mesh");
        assert!(
            full.involute_interference,
            "a full-depth ring should interfere: {full:?}"
        );
        assert!(
            !full.trochoid_interference && !full.tip_interference,
            "and it should be the involute one alone that bites: {full:?}"
        );
        let short = solved(0.75).planet_ring.tips.expect("an internal mesh");
        assert!(
            short.clear(),
            "shortening the ring's tooth should clear it: {short:?}"
        );
    }

    /// **A parallel-axis pair's two members share one tangential force**, so the
    /// load they react is in the ratio of their tooth counts — and nothing about
    /// efficiency enters, because within a stage the load has not been
    /// attenuated yet.
    ///
    /// The quantitative half of the walk above, asserted where a quantitative
    /// law exists.
    #[test]
    fn a_parallel_pairs_reacted_load_is_in_the_tooth_count_ratio() {
        let lib = library();
        let mut train = two_stage();
        train.back_driving_torque = 0.5;
        train.stages.insert(0, Stage::Worm(WormStage::default()));

        let r = solve_train(&train, &lib).expect("a train that solves");
        let mut checked = 0u32;
        for (k, stage) in r.stages.iter().enumerate() {
            let Some(spur) = stage.as_spur() else {
                continue;
            };
            let (Some(a), Some(b)) = (
                spur.gears[0].back_driving_torque,
                spur.gears[1].back_driving_torque,
            ) else {
                continue;
            };
            checked += 1;
            let want = spur.ratio;
            assert!(
                (b / a - want).abs() < 1e-9 * want.abs(),
                "stage {k}: the two members react {a} and {b}, a ratio of {}, \
                 where the pair steps up by {want}",
                b / a
            );
        }
        assert!(
            checked >= 2,
            "only {checked} parallel stages carried the load"
        );
    }

    /// **The declared freedoms describe members that exist, and each group has
    /// something to relieve.**
    ///
    /// The structural half. A declaration nothing checks is data, and this is
    /// the cheap part of checking it: a `Shift(i)` that named a member the stage
    /// does not have would resolve to nothing in the front end and relieve
    /// silently, and a group whose limit equals its size can never fire at all.
    #[test]
    fn every_declared_freedom_names_a_member_the_stage_has() {
        let hula = HulaStage::default();
        let members = |s: &Stage| match s {
            Stage::Spur(s) => s.gears.len(),
            Stage::Planetary(_) => 3,
            Stage::Hula(h) => h.gears.len(),
            Stage::Worm(_) => 2,
        };
        for stage in [
            Stage::Spur(SpurStage::default()),
            Stage::Worm(WormStage::default()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::new(hula)),
        ] {
            let n = members(&stage);
            let groups = stage.freedoms();
            let mut seen = Vec::new();
            for g in &groups {
                assert!(
                    g.given_at_most < g.order.len(),
                    "a group that cannot be over-determined is not a group: {g:?}"
                );
                assert!(
                    !g.order.is_empty(),
                    "a group with no inputs relieves nothing: {g:?}"
                );
                for f in &g.order {
                    if let Freedom::Shift(i) = f {
                        assert!(*i < n, "Shift({i}) but this stage has {n} members");
                    }
                    assert!(
                        !seen.contains(f),
                        "{f:?} is in two groups, so relieving one could break the other"
                    );
                    seen.push(*f);
                }
            }
        }
    }

    /// **Relieving an over-determined stage**, on every kind that has a relation
    /// — behaviour that had no test at all while it lived in the panel.
    ///
    /// Three claims, and the third is the one that makes it a rule rather than a
    /// habit: pinning everything relieves down to the limit; the input the
    /// designer has *this moment* pinned is never the one taken away; and a
    /// stage already within its limit is returned untouched, so relief cannot
    /// undo a design that was never over-determined.
    #[test]
    fn an_over_determined_stage_relieves_to_its_limit_and_keeps_what_was_just_pinned() {
        let pin_everything = |stage: &Stage| {
            let mut s = stage.clone();
            for g in s.freedoms() {
                for f in g.order {
                    if let Some(a) = s.auto_mut(f) {
                        *a = Auto::fixed(0.1);
                    }
                }
            }
            s
        };

        let mut hula = HulaStage::default();
        hula.gears[0].profile_shift = Auto::automatic(0.0);
        for stage in [
            Stage::Spur(SpurStage::default()),
            Stage::Worm(WormStage::default()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::new(hula)),
        ] {
            let groups = stage.freedoms();
            if groups.is_empty() {
                // A kind with no relation cannot be over-determined, and relief
                // must leave it exactly as it was.
                let just = Freedom::Shift(0);
                assert_eq!(
                    stage.relieved(just).freedoms(),
                    groups,
                    "a stage with no freedoms should come back unchanged"
                );
                continue;
            }

            let over = pin_everything(&stage);
            for group in &groups {
                // Each input in turn is the one just pinned.
                for just in &group.order {
                    let mut relieved = over.relieved(*just);

                    let given = group
                        .order
                        .iter()
                        .filter(|f| relieved.auto_mut(**f).is_some_and(|a| !a.auto))
                        .count();
                    assert!(
                        given <= group.given_at_most,
                        "{group:?} left {given} given after relief"
                    );
                    assert!(
                        relieved.auto_mut(*just).is_some_and(|a| !a.auto),
                        "{just:?} was just pinned and must not be the one relieved"
                    );
                }
            }

            // Already inside the limit: nothing moves. Asserted against every
            // freedom as `just`, so it cannot pass by picking a lucky one.
            let mut settled = stage.clone();
            if let Some(f) = groups[0].order.first() {
                if let Some(a) = settled.auto_mut(*f) {
                    *a = Auto::fixed(0.1);
                }
            }
            for group in &groups {
                for just in &group.order {
                    let mut before = settled.clone();
                    let mut after = settled.relieved(*just);
                    for f in &group.order {
                        assert_eq!(
                            before.auto_mut(*f).map(|a| a.auto),
                            after.auto_mut(*f).map(|a| a.auto),
                            "{f:?} moved on a stage that was already within its limit"
                        );
                    }
                }
            }
        }
    }

    /// **The declared limit is the stage's actual freedom** — asserted by giving
    /// exactly that many and requiring every one of them to be honoured, then
    /// giving one more and requiring that it cannot be.
    ///
    /// This is what stops the declaration being a number somebody wrote down. A
    /// pair's group says two of `{a, x₁, x₂}` may be given: with the distance
    /// and the clearance given the shifts move to reach it, and with **both
    /// shifts** given as well the distance can no longer be what was asked at
    /// the clearance that was asked, because nothing is left to absorb the
    /// difference.
    ///
    /// Stated as *the clearance the pair actually runs at*, which is the
    /// quantity the relation is about, rather than as a shift value — so it says
    /// the same thing whatever the division rule decides.
    #[test]
    fn the_declared_limit_is_the_freedom_the_stage_actually_has() {
        let lib = library();
        let clearance = 0.02_f64;
        let asked = 24.4199_f64 + clearance;

        let solve = |pin_shifts: bool| {
            let mut sp = SpurStage {
                clearance,
                centre_distance: Auto::fixed(asked),
                ..SpurStage::default()
            };
            sp.gears[0].teeth = 9;
            sp.gears[1].teeth = 37;
            if pin_shifts {
                // Two shifts a designer might well type, and nowhere near the
                // pair the given distance wants.
                sp.gears[0].profile_shift = Auto::fixed(0.20);
                sp.gears[1].profile_shift = Auto::fixed(0.20);
            }
            let mut t = two_stage();
            t.stages = vec![Stage::Spur(sp)];
            let r = solve_train(&t, &lib).expect("the stage solves either way");
            let s = r.stages[0].as_spur().expect("a spur stage");
            (s.clearance, s.centre_distance)
        };

        // At the limit — two given, the shifts free — every given number stands.
        let (got, distance) = solve(false);
        assert!(
            (got - clearance).abs() < 1e-6 && (distance - asked).abs() < 1e-9,
            "two given should all be honoured: clearance {got} at {distance}"
        );

        // One more, and the relation cannot hold. The distance is still what was
        // typed — it is the *clearance* that gives, which is the arm of the
        // contradiction the panel resolves by relieving the distance.
        let (over, distance) = solve(true);
        assert!(
            (distance - asked).abs() < 1e-9,
            "the given distance is still the distance"
        );
        assert!(
            (over - clearance).abs() > 1e-3,
            "with both shifts pinned as well, the clearance cannot also be {clearance}: got {over}"
        );

        // ...and the group says exactly that many.
        let groups = Stage::Spur(SpurStage::default()).freedoms();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].given_at_most, 2);
        assert_eq!(groups[0].order[0], Freedom::CentreDistance);
    }

    /// **A given distance and a given clearance decide the shifts — with the
    /// optimiser off as much as on.**
    ///
    /// This is mode 3 of the clearance paradigm
    /// (`docs/reference.md#which-of-the-three-numbers-is-given-and-which-follows`): of the three related
    /// numbers, any two given leave the third derived, so a housing distance and
    /// a running clearance fix the shift sum and the stage must hit it.
    ///
    /// It used to hold **only with the optimiser on**. `shifts_at` returned the
    /// undercut floor before ever looking at the distance, so the plainest thing
    /// a designer does — type a distance with nothing asked to move — ran the
    /// pair at whatever distance the floor happened to make and reported the
    /// shortfall as clearance. On a 9/37 pair at 24.42 mm that was 0.47 mm of
    /// "clearance" nobody asked for.
    ///
    /// The claim asserted is the identity `clearance == what was asked`, which
    /// says the sum was reached without naming any shift — the division is a
    /// separate rule ([`crate::auto::divide_shift_sum`]) and this is true
    /// whatever it decides. **Both paths are checked against the same distances**,
    /// so a search and a stated rule cannot come to different sums.
    #[test]
    fn a_given_distance_and_clearance_decide_the_shifts_either_way() {
        let lib = library();
        let mut checked = 0u32;
        for (z1, z2, distances) in [
            (9u32, 37u32, [23.4866_f64, 23.9532, 24.4199]),
            (17, 43, [30.3922, 30.7645, 31.1367]),
        ] {
            for a in distances {
                for clearance in [0.0_f64, 0.02, 0.20] {
                    let mut sums = Vec::new();
                    for optimiser in [false, true] {
                        let mut sp = SpurStage {
                            clearance,
                            centre_distance: Auto::fixed(a + clearance),
                            ..SpurStage::default()
                        };
                        sp.gears[0].teeth = z1;
                        sp.gears[1].teeth = z2;
                        sp.optimisation.enabled = optimiser;
                        let mut t = two_stage();
                        t.stages = vec![Stage::Spur(sp)];
                        let Ok(r) = solve_train(&t, &lib) else {
                            continue;
                        };
                        let s = r.stages[0].as_spur().expect("a spur stage");

                        checked += 1;
                        assert!(
                            (s.clearance - clearance).abs() < 1e-6,
                            "{z1}/{z2} at a={a} clearance={clearance} optimiser={optimiser}:                              the shifts leave {} of clearance, so the pair does not run at the                              distance it was given",
                            s.clearance
                        );
                        sums.push(s.gears[0].profile_shift + s.gears[1].profile_shift);
                    }
                    // One relation, two ways of satisfying it: the searched
                    // division and the stated one must reach the *same* sum,
                    // because the sum is not either one's to choose.
                    if let [off, on] = sums[..] {
                        assert!(
                            (off - on).abs() < 1e-6,
                            "{z1}/{z2} at a={a}: the optimiser reaches a sum of {on}                              and the stated division {off}"
                        );
                    }
                }
            }
        }
        assert!(checked >= 30, "only {checked} cases reached the distance");
    }

    /// **The reported clearance is the gap the stage runs at.**
    ///
    /// A centre distance is the true distance and a clearance is what portion of
    /// it is clearance, so the three numbers are two facts and a subtraction.
    /// `clearance` was the *input* echoed back instead, gated by whether
    /// anything was free to absorb it — which is right only when the distance is
    /// automatic, because then the running distance was built by adding it.
    ///
    /// Given a distance, it was wrong every way it could be: a pair told to run
    /// at 30.3 mm whose shifts put it at 30.0057 has 0.294 mm of clearance, and
    /// it reported 0.02 with the optimiser on and 0.000 with it off.
    ///
    /// Asserted as the identity rather than against those figures, so it says
    /// the same thing on every kind and at every distance.
    #[test]
    fn the_reported_clearance_is_the_gap_the_stage_runs_at() {
        let lib = library();
        let mut checked = 0u32;
        for distance in [None, Some(30.3_f64), Some(30.5)] {
            for clearance in [0.0_f64, 0.02, 0.20] {
                for optimiser in [false, true] {
                    let mut sp = SpurStage {
                        clearance,
                        ..SpurStage::default()
                    };
                    sp.optimisation.enabled = optimiser;
                    if let Some(a) = distance {
                        sp.centre_distance = Auto::fixed(a);
                    }
                    let mut t = two_stage();
                    t.stages = vec![Stage::Spur(sp), Stage::Worm(WormStage::default())];
                    let Ok(r) = solve_train(&t, &lib) else {
                        continue;
                    };

                    let s = r.stages[0].as_spur().expect("a spur stage");
                    checked += 1;
                    assert!(
                        (s.clearance - (s.centre_distance - s.centre_distance_nominal)).abs()
                            < 1e-12,
                        "spur at a={distance:?} clearance={clearance} optimiser={optimiser}: \
                         reports {} of clearance between {} and {}",
                        s.clearance,
                        s.centre_distance_nominal,
                        s.centre_distance
                    );
                    // ...and the same identity on the kind that has no shift to
                    // absorb anything, which is where an echoed input and a
                    // derived gap part company hardest.
                    let w = r.stages[1].as_worm().expect("a worm stage");
                    assert!(
                        (w.clearance - (w.centre_distance - w.centre_distance_nominal)).abs()
                            < 1e-12,
                        "worm: reports {} between {} and {}",
                        w.clearance,
                        w.centre_distance_nominal,
                        w.centre_distance
                    );
                }
            }
        }
        assert!(checked >= 12, "only {checked} configurations solved");
    }

    /// **A tolerance band opens the way round it says it does**, on every kind
    /// that reports one.
    ///
    /// Less centre distance is less room and so less play. That is one claim,
    /// and it was written out four times — once per stage kind, each closing
    /// over its own way of turning a distance into an angle. Getting it
    /// backwards in one of them would have produced a band that reads perfectly
    /// well and is inside out, and nothing anywhere asserted the direction.
    ///
    /// `Backlash::banded` is the one construction now; this is the claim it
    /// makes, checked through all four kinds rather than at the constructor,
    /// because the argument each passes is the part that could still be wrong.
    #[test]
    fn a_tolerance_band_widens_with_the_centre_distance() {
        let lib = library();
        let mut train = two_stage();
        train.stages = vec![
            Stage::Spur(SpurStage::default()),
            Stage::Worm(WormStage::default()),
            Stage::Planetary(Box::default()),
            Stage::Hula(Box::default()),
            Stage::Spur(SpurStage {
                shaft_angle: 90.0,
                ..SpurStage::default()
            }),
        ];
        let r = solve_train(&train, &lib).expect("a train of every kind");

        let mut checked = 0u32;
        let mut check = |what: &str, b: &Backlash| {
            checked += 1;
            assert!(
                b.minimum <= b.nominal && b.nominal <= b.maximum,
                "{what}: the band runs {} … {} … {}, which is not an order",
                b.minimum,
                b.nominal,
                b.maximum
            );
            assert!(
                b.maximum > b.minimum,
                "{what}: a tolerance that opens nothing — {} either way",
                b.minimum
            );
        };

        for (k, stage) in r.stages.iter().enumerate() {
            let d = stage.backlash();
            check(&format!("stage {k} forward"), &d.forward);
            check(&format!("stage {k} backward"), &d.backward);
        }
        assert!(checked >= 10, "only {checked} bands checked");
    }

    /// **A set driven backward distributes its torque the way it distributes it
    /// driven backward**, not the way it does driven forward.
    ///
    /// The reverse of a mechanism is the same construction with the roles
    /// swapped, so the answer is the reverse *solve* — which this stage already
    /// performs, for its efficiency, and whose torques it discarded. What it
    /// reported instead was the forward distribution scaled by the ratio of the
    /// two stage torques, which is exact wherever the forward torque is a
    /// geometric projection or the two directional efficiencies agree. An
    /// epicyclic set is neither: **which shaft drives decides where `η₀`
    /// multiplies.** The shipped set's ring came out 6 % low.
    ///
    /// The law is the degenerate case rather than the figure: **at zero friction
    /// the two distributions coincide**, because with `η₀ = 1` there is no loss
    /// for the direction to place. So the difference *is* the efficiency, and
    /// that is checkable without knowing either number.
    /// **And the hula stage, which is an epicyclic power flow and had the same
    /// fault.** Its two central gears stand in for the sun and the ring.
    #[test]
    fn a_back_driven_hula_distributes_torque_by_its_own_solve() {
        let lib = library();
        let ratios = |mu: f64| {
            let h = HulaStage {
                sliding_friction: [mu; 2],
                static_friction: [mu; 2],
                ..HulaStage::default()
            };
            let mut t = two_stage();
            t.back_driving_torque = 0.5;
            t.stages = vec![Stage::Worm(WormStage::default()), Stage::Hula(Box::new(h))];
            let r = solve_train(&t, &lib).expect("a train that solves");
            let s = r.stages[1].as_hula().expect("a hula stage");
            let back = |g: &HulaGear| g.gear.back_driving_torque.expect("the stage reacts it");
            (
                s.gears[3].gear.torque / s.gears[0].gear.torque,
                back(&s.gears[3]) / back(&s.gears[0]),
            )
        };
        let (forward, backward) = ratios(0.0);
        assert!(
            (forward - backward).abs() < 1e-9,
            "with no friction the stage distributes torque alike either way, \
             but forward gives {forward} and backward {backward}"
        );
        let (forward, backward) = ratios(0.08);
        assert!(
            (forward - backward).abs() / forward.abs() > 1e-3,
            "with friction the two directions must place the loss differently, \
             but both give {forward}"
        );
    }

    #[test]
    fn a_back_driven_set_distributes_torque_by_its_own_solve() {
        let lib = library();
        let ratios = |mu: f64| {
            let mut set = PlanetaryStage {
                sliding_friction_sun_planet: mu,
                sliding_friction_planet_ring: mu,
                ..PlanetaryStage::default()
            };
            set.static_friction_sun_planet = mu;
            set.static_friction_planet_ring = mu;
            let mut t = two_stage();
            t.back_driving_torque = 0.5;
            // A self-locking stage at the input end, so the set reacts the load.
            t.stages = vec![
                Stage::Worm(WormStage::default()),
                Stage::Planetary(Box::new(set)),
            ];
            let r = solve_train(&t, &lib).expect("a train that solves");
            let p = r.stages[1].as_planetary().expect("a planetary stage");
            let back = |g: &GearResult| g.back_driving_torque.expect("the set reacts the load");
            (p.ring.torque / p.sun.torque, back(&p.ring) / back(&p.sun))
        };

        // Frictionless: nothing for the direction to place, so the two
        // distributions are the same one.
        let (forward, backward) = ratios(0.0);
        assert!(
            (forward - backward).abs() < 1e-9,
            "with no friction the set distributes torque alike either way, \
             but forward gives {forward} and backward {backward}"
        );

        // With friction they part, and that difference is the whole finding:
        // scaling the forward answer would have kept them equal at every `mu`.
        let (forward, backward) = ratios(0.06);
        assert!(
            (forward - backward).abs() / forward > 1e-3,
            "with friction the two directions must place the loss differently, \
             but both give {forward}"
        );
    }

    fn two_stage() -> Train {
        Train {
            input_speed: 3000.0,
            input_torque: 2.0,
            back_driving_torque: 0.0,
            operating_torque: 2.0,
            reversed_bending: false,
            actuation: Actuation::default(),
            stages: vec![
                Stage::Spur(SpurStage::default()),
                Stage::Spur(SpurStage {
                    gears: [
                        StageGear {
                            teeth: 13,
                            ..StageGear::default()
                        },
                        StageGear {
                            teeth: 31,
                            ..StageGear::default()
                        },
                    ],
                    ..SpurStage::default()
                }),
            ],
        }
    }

    #[test]
    fn a_two_stage_train_computes_end_to_end() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        assert_eq!(r.stages.len(), 2);

        // 43/17 * 31/13
        let want = (43.0 / 17.0) * (31.0 / 13.0);
        assert!((r.total_ratio - want).abs() < 1e-12);
        assert!((r.output_speed - 3000.0 / want).abs() < 1e-9);

        // Every stage produced real numbers.
        for s in r.stages.iter().map(spur) {
            assert!(s.centre_distance > 0.0);
            assert!(s.mesh.contact_ratios.transverse > 1.0);
            assert!(s.mesh.efficiency.forward > 0.9 && s.mesh.efficiency.forward < 1.0);
            assert_eq!(
                s.mesh.efficiency.forward, s.mesh.efficiency.backward,
                "a parallel-axis stage is as efficient driven either way"
            );
            assert!(s.mesh.contact_stress_at_pitch_point.peak > 0.0);
            for g in &s.gears {
                assert!(g.face_width > 0.0);
                assert!(g.bending_stress.peak.unwrap() > 0.0);
            }
        }
    }

    /// **The two backlash figures are one gap seen from the two ends.**
    ///
    /// Referred to the output shaft or to the input shaft, the same play must
    /// differ by exactly the total ratio — every stage's contribution scales the
    /// same way, because a stage's own two figures are its gap at two lever arms
    /// whose ratio *is* that stage's ratio. If a stage ever got that wrong the
    /// products would stop matching, which no per-stage check would catch.
    #[test]
    fn backlash_at_the_two_ends_differs_by_exactly_the_total_ratio() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        for (forward, backward) in [
            (r.backlash.forward.nominal, r.backlash.backward.nominal),
            (r.backlash.forward.maximum, r.backlash.backward.maximum),
        ] {
            assert!(forward > 0.0);
            assert!(
                (backward - forward * r.total_ratio).abs() < 1e-9 * backward,
                "{backward} vs {forward} x {}",
                r.total_ratio
            );
        }
        // ...and the input end is the looser one, because it turns faster.
        assert!(r.backlash.backward.nominal > r.backlash.forward.nominal);
    }

    /// A train of parallel-axis stages is as efficient driven either way, and
    /// cannot lock. Both are consequences of the meshes, not rules the train
    /// applies.
    #[test]
    fn a_parallel_axis_train_reports_equal_efficiencies_and_cannot_lock() {
        let r = solve_train(&two_stage(), &library()).unwrap();
        assert_eq!(r.total_efficiency.forward, r.total_efficiency.backward);
        assert!(!r.total_efficiency.locked().backward);
    }

    /// Efficiency must always *reduce* delivered torque. Getting this sign wrong
    /// is the classic train-accumulation bug, and it hides because the ratio term
    /// is so much larger.
    #[test]
    fn efficiency_always_costs_torque() {
        let lib = library();
        let mut lossless = two_stage();
        for s in &mut lossless.stages {
            spur_input(s).sliding_friction = 0.0;
            spur_input(s).static_friction = 0.0;
        }
        let ideal = solve_train(&lossless, &lib).unwrap();
        let real = solve_train(&two_stage(), &lib).unwrap();

        assert!((ideal.total_efficiency.forward - 1.0).abs() < 1e-12);
        assert!(real.output_torque < ideal.output_torque);
        // ...and the shortfall is exactly the product of the stage efficiencies.
        assert!(
            (real.output_torque - ideal.output_torque * real.total_efficiency.forward).abs() < 1e-9
        );
    }

    /// **An automatic profile shift asks about the depth the tooth actually
    /// has.**
    ///
    /// `working_depth` is the depth the undercut question is asked at, and it
    /// now follows the **dedendum** by default instead of sitting at a fixed
    /// module. The two are different questions — "is the flank undercut within a
    /// module of depth?" against "is it undercut at all?" — and at α = 20° with
    /// a sharp rack they part company at 18 teeth and 22 (docs/reference.md#automatic-values). Following the
    /// dedendum also means a gear cut shallower is asked about its own depth
    /// rather than about a convention.
    ///
    /// Gated because nothing did: when the default moved, every figure in
    /// `gear-cli train` moved with it and the suite stayed green. Asserted as
    /// the law rather than the numbers — a deeper cut can only need more shift
    /// to stay clear of undercut — plus the one identity that pins it, which is
    /// that fixing `working_depth` at the dedendum's value reproduces automatic
    /// exactly.
    #[test]
    fn an_automatic_profile_shift_follows_the_dedendum() {
        let lib = library();
        let shift_of = |dedendum: f64, working: Auto<f64>| {
            let stage = SpurStage {
                gears: [
                    StageGear {
                        teeth: 15,
                        dedendum,
                        working_depth: working,
                        profile_shift: Auto::automatic(0.0),
                        ..Default::default()
                    },
                    StageGear {
                        teeth: 43,
                        dedendum,
                        working_depth: working,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            };
            solve_spur_stage(&stage, StageTorques::just(2.0), &lib)
                .expect("a solvable stage")
                .gears[0]
                .profile_shift
        };

        // Deeper teeth, more shift. A law: undercut is a question about how far
        // down the flank has to stay clean.
        let mut previous = f64::NEG_INFINITY;
        for dedendum in [1.0_f64, 1.25, 1.5, 1.9] {
            let x = shift_of(dedendum, Auto::automatic(0.0));
            assert!(
                x > previous,
                "dedendum {dedendum}: a deeper cut cannot need less shift — \
                 {x} against {previous}"
            );
            previous = x;
        }

        // ...and automatic *is* the dedendum, not something near it.
        for dedendum in [1.0_f64, 1.25, 1.9] {
            let automatic = shift_of(dedendum, Auto::automatic(0.0));
            let named = shift_of(dedendum, Auto::fixed(dedendum));
            assert!(
                (automatic - named).abs() < 1e-12,
                "dedendum {dedendum}: automatic gave {automatic}, the dedendum \
                 by hand gave {named}"
            );
        }

        // The old default is still reachable and still different, so this is a
        // change of default rather than a loss of the control.
        let classical = shift_of(1.25, Auto::fixed(1.0));
        let now = shift_of(1.25, Auto::automatic(0.0));
        assert!(
            now > classical + 1e-6,
            "asking a module deep should ask for less shift than asking 1.25 \
             deep: {classical} against {now}"
        );
    }

    /// **A parallel stage is rated where it runs, not where it was designed.**
    ///
    /// `Mesh::a_w` is the **zero-backlash** centre distance — where the profile
    /// shifts put the pair — and a real one runs at that plus its assembly
    /// clearance. Every contact quantity belongs to the second: the path
    /// shortens, the operating pressure angle opens, the relative curvature
    /// grows. Rating at `a_w` was rating a pair nobody assembles, and the
    /// clearance is not a detail to round away — it is the reason the stage has
    /// any backlash to report at all.
    ///
    /// The direction is the law and is asserted as one: separating the centres
    /// can only shorten the path of contact. Everything downstream follows —
    /// less load sharing, so more bending stress — which is why the numbers
    /// moved when this landed (docs/reference.md#centre-distance-and-backlash).
    ///
    /// Backlash is deliberately *not* in this test's scope: it measures play
    /// against the zero-backlash reference and keeps the design mesh. That
    /// division is gated in `mesh.rs`.
    #[test]
    fn a_parallel_stage_is_rated_at_the_centre_distance_it_runs_at() {
        let lib = library();
        let stage = |clearance: f64| SpurStage {
            clearance,
            ..SpurStage::default()
        };
        let mut previous: Option<(f64, f64)> = None;
        for clearance in [0.0_f64, 0.02, 0.1, 0.3] {
            let r = solve_spur_stage(&stage(clearance), StageTorques::just(2.0), &lib).unwrap();
            let eps = r.mesh.contact_ratios.transverse;
            let bending = r.gears[0].bending_stress.peak.expect("a rateable tooth");
            if let Some((was_eps, was_bending)) = previous {
                assert!(
                    eps < was_eps,
                    "clearance {clearance}: separating the centres can only \
                     shorten the path — ε {eps} against {was_eps}"
                );
                assert!(
                    bending > was_bending,
                    "clearance {clearance}: a shorter path is less load sharing, \
                     so the tooth carries more — {bending} against {was_bending}"
                );
            }
            previous = Some((eps, bending));
        }
    }

    #[test]
    fn a_spur_stage_has_exactly_zero_overlap_and_a_helical_one_does_not() {
        let lib = library();
        let spur = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        assert_eq!(
            spur.mesh.contact_ratios.overlap, 0.0,
            "must be exactly zero"
        );
        assert_eq!(
            spur.mesh.contact_ratios.total,
            spur.mesh.contact_ratios.transverse
        );
        assert!(!spur.mesh.contact_ratios.has_full_axial_overlap());

        let helical = solve_spur_stage(
            &SpurStage {
                additional_helix: 20.0,
                ..SpurStage::default()
            },
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();
        assert!(helical.mesh.contact_ratios.overlap > 0.0);
        assert!(helical.mesh.contact_ratios.total > helical.mesh.contact_ratios.transverse);
    }

    /// The last stage dominates output backlash, which is the design consequence
    /// worth surfacing: tolerance spent at the input end is nearly free.
    #[test]
    fn backlash_referred_to_the_output_is_dominated_by_the_last_stage() {
        let lib = library();
        let base = two_stage();

        let loosen = |k: usize| {
            let mut t = base.clone();
            spur_input(&mut t.stages[k]).clearance *= 4.0;
            solve_train(&t, &lib).unwrap().backlash.forward.nominal
        };

        let reference = solve_train(&base, &lib).unwrap().backlash.forward.nominal;
        let first = loosen(0) - reference;
        let last = loosen(1) - reference;
        assert!(first > 0.0 && last > 0.0);
        assert!(
            last > first * 2.0,
            "the last stage should dominate: {last} vs {first}"
        );
    }

    #[test]
    fn thickness_modification_cannot_break_its_own_invariant() {
        let stage = SpurStage {
            thickness_mod: 1.3,
            ..SpurStage::default()
        };
        let k: Vec<f64> = (0..2)
            .map(|i| stage.params_at(i, stage.shifts()[i]).thickness_mod)
            .collect();
        assert!((k[0] + k[1] - 2.0).abs() < 1e-15);
    }

    #[test]
    fn the_automatic_face_width_is_the_larger_of_the_enabled_checks() {
        let lib = library();
        let off = LoadCase {
            peak: false,
            cyclic: false,
        };
        let width = |sources: FaceSources| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib)
                .unwrap()
                .gears[0]
                .face_width
        };
        // One source at a time, then every combination of them: the width is the
        // largest of whatever is enabled, and that is the whole rule.
        let each: Vec<f64> = [
            FaceSources {
                bending: LoadCase {
                    peak: true,
                    cyclic: false,
                },
                contact: off,
            },
            FaceSources {
                bending: LoadCase {
                    peak: false,
                    cyclic: true,
                },
                contact: off,
            },
            FaceSources {
                bending: off,
                contact: LoadCase {
                    peak: true,
                    cyclic: false,
                },
            },
            FaceSources {
                bending: off,
                contact: LoadCase {
                    peak: false,
                    cyclic: true,
                },
            },
        ]
        .into_iter()
        .map(width)
        .collect();
        // The law is about what is *enabled*, so it is checked against every
        // source switched on rather than against the default — which is a
        // separate decision, and is asserted as one below.
        let on = LoadCase {
            peak: true,
            cyclic: true,
        };
        let all = width(FaceSources {
            bending: on,
            contact: on,
        });
        let largest = each.iter().copied().fold(0.0_f64, f64::max);
        assert!((all - largest).abs() < 1e-9, "{all} vs max{each:?}");

        // Peak and cyclic are the same torque on this train, so what separates
        // them is only the allowable — and the ultimate is above the fatigue
        // figure, so the cyclic case is the one that asks for more face.
        assert!(each[1] > each[0] && each[3] > each[2]);
        // Contact governs a lightly loaded steel pair, as it usually does.
        assert!(each[3] > each[1]);

        // **A fresh stage is sized from bending alone.** Both contact ratings
        // are computed and both are offered; neither decides a width until a
        // designer says so, because the cyclic one dominates by an order of
        // magnitude and a default that picks the answer is a default making the
        // design decision.
        let d = FaceSources::default();
        assert_eq!(d.bending, on, "bending is what sizes a fresh stage");
        assert_eq!(d.contact, off, "neither contact rating is assumed");
        assert!(
            (width(d) - each[0].max(each[1])).abs() < 1e-9,
            "the default width is the bending pair and nothing else"
        );

        // Nothing enabled asks for nothing, which is a degenerate gear rather
        // than a divide by zero.
        assert_eq!(
            width(FaceSources {
                bending: off,
                contact: off
            }),
            0.0
        );
    }

    /// Intermittent duty is measured at the OUTPUT, so upstream gears turn
    /// further, not less. Getting the direction backwards would silently
    /// under-count cycles on exactly the gears that see the most.
    #[test]
    fn intermittent_cycles_are_worked_backwards_from_the_output() {
        let mut t = two_stage();
        t.actuation = Actuation::Intermittent {
            range_degrees: 360.0,
            actuations: 100,
            reversing: false,
        };
        let r = solve_train(&t, &library()).unwrap();

        // The output gear turns exactly once per actuation. A whole number of
        // revolutions, so the rounding has nothing to do and the count is the
        // revolutions exactly — which is the case worth putting first, because
        // it is where a count and a revolution coincide.
        let last = &spur(&r.stages[1]).gears[1];
        assert_eq!(last.tooth_cycles.bending, 100.0);
        // Not reversing, so one flank takes every engagement.
        assert_eq!(last.tooth_cycles.contact, last.tooth_cycles.bending);

        // Every gear upstream turns more than the one after it.
        let seq = [
            spur(&r.stages[0]).gears[0].tooth_cycles.bending,
            spur(&r.stages[0]).gears[1].tooth_cycles.bending,
            spur(&r.stages[1]).gears[1].tooth_cycles.bending,
        ];
        for w in seq.windows(2) {
            assert!(w[0] > w[1], "cycles must fall towards the output: {seq:?}");
        }
        // The input gear sees the whole train ratio's worth — **rounded up**,
        // because these are engagements rather than revolutions and a tooth
        // three quarters of the way through one has still been loaded by it.
        assert_eq!(seq[0], (100.0 * r.total_ratio).ceil());

        // The two gears meshing with each other turn at different speeds but
        // share a mesh, so their cycle counts differ by that stage ratio — and
        // only **up to the rounding**, now that each count is a whole number of
        // engagements rather than a revolution count.
        //
        // The bound is derived rather than chosen: rounding adds less than one
        // to each, so `a/b` moves from `A/B` by less than `(1 + ratio)/b`. On
        // this train that is 6e-8, which is why the old exact-to-1e-9 assertion
        // was the thing that had to change and not the arithmetic.
        let s0 = spur(&r.stages[0]);
        let (a, b) = (
            s0.gears[0].tooth_cycles.bending,
            s0.gears[1].tooth_cycles.bending,
        );
        assert!(
            (a / b - s0.ratio).abs() < (1.0 + s0.ratio) / b,
            "{a}/{b} = {} against a stage ratio of {}",
            a / b,
            s0.ratio
        );
    }

    #[test]
    fn continuous_cycles_follow_each_gears_own_speed() {
        let mut t = two_stage();
        t.actuation = Actuation::Continuous {
            operating_speed: 1500.0,
            runtime_hours: 2.0,
        };
        let r = solve_train(&t, &library()).unwrap();

        // Input gear: 1500 rpm * 60 min * 2 h — its own speed, for as long as
        // the train runs. Rounded up, as every count is.
        let want = (1500.0_f64 * 60.0 * 2.0).ceil();
        let first = spur(&r.stages[0]).gears[0].tooth_cycles;
        assert!((first.bending - want).abs() < 1e-6);
        // A continuous drive has no actuation to reverse within, so the two
        // counts agree.
        assert_eq!(first.bending, first.contact);
        // Speeds fall through the train, and cycles follow them.
        assert!((spur(&r.stages[0]).gears[0].speed - 3000.0).abs() < 1e-9);
        assert!((spur(&r.stages[1]).gears[1].speed - r.output_speed).abs() < 1e-9);
        assert!(spur(&r.stages[1]).gears[1].tooth_cycles.bending < first.bending);
    }

    /// **An epicyclic member is engaged once per turn against the carrier**, per
    /// planet — for every member and both kinds, which is the whole of
    /// [`engagements`].
    ///
    /// Checked against arithmetic the stage shares nothing with: the counts a
    /// member's own speed and the carrier's give, taken from the result's speed
    /// array rather than from the rule that filled the cycles. Three things
    /// would have failed here before it existed, and each was a different way of
    /// counting the wrong revolutions:
    ///
    /// - the **ring** of a set with the ring held is loaded while it does not
    ///   turn at all, and it counted the *sun's* revolutions — three and a half
    ///   times too many on these counts;
    /// - the **sun** counted its own rather than its rotation against the
    ///   carrier, which is 1.4 times too many;
    /// - the **planet** was referred to the sun's speed rather than to the input
    ///   shaft's, right only where those are the same shaft.
    #[test]
    fn an_epicyclic_members_cycles_are_its_turns_against_the_carrier() {
        let lib = library();
        for (name, stage) in [
            (
                "epicyclic",
                Stage::Planetary(Box::new(PlanetaryStage {
                    planets: 3,
                    ..PlanetaryStage::default()
                })),
            ),
            ("hula", Stage::Hula(Box::default())),
        ] {
            let train = Train {
                actuation: Actuation::Continuous {
                    operating_speed: 3000.0,
                    runtime_hours: 1.0,
                },
                stages: vec![stage],
                ..two_stage()
            };
            let r = solve_train(&train, &lib).unwrap_or_else(|e| panic!("{name}: {e}"));
            // The revolutions the input shaft turns over the duty, which is what
            // every member's count is a multiple of.
            let turns = 3000.0 * 60.0;

            let want = |member: f64, carrier: f64, paths: f64| {
                (turns * ((member - carrier) / 3000.0).abs() * paths).ceil()
            };
            match &r.stages[0] {
                StageResult::Planetary(p) => {
                    let carrier = p.speeds[1];
                    let n = f64::from(p.planets);
                    for (which, got, speed) in [
                        ("sun", p.sun.tooth_cycles.bending, p.speeds[0]),
                        ("ring", p.ring.tooth_cycles.bending, p.speeds[2]),
                        (
                            "planet",
                            p.planet.gear.tooth_cycles.bending,
                            p.planet.gear.speed,
                        ),
                    ] {
                        let expected = want(speed, carrier, n);
                        assert!(
                            (got - expected).abs() <= 1.0,
                            "{which}: {got} engagements against {expected}"
                        );
                    }
                    // **A shaft that does not turn is still loaded**, which is
                    // the case the old rule could not state: it counted the
                    // input's revolutions, so a held ring came out as though it
                    // turned with the sun. It meets a planet once per *carrier*
                    // turn instead, which is `z_s/(z_s + z_r)` of that.
                    assert_eq!(p.speeds[2], 0.0, "the ring is the held shaft here");
                    let zs = f64::from(PlanetaryStage::default().sun.teeth);
                    let zr = f64::from(PlanetaryStage::default().ring.teeth);
                    assert!(
                        (p.ring.tooth_cycles.bending
                            - (turns * zs / (zs + zr) * f64::from(p.planets)).ceil())
                        .abs()
                            <= 1.0,
                        "a held ring counts carrier turns: {}",
                        p.ring.tooth_cycles.bending
                    );
                }
                StageResult::Hula(h) => {
                    for g in &h.gears {
                        let expected = want(g.gear.speed, h.crank_speed, 1.0);
                        assert!(
                            (g.gear.tooth_cycles.bending - expected).abs() <= 1.0,
                            "z{}: {} engagements against {expected}",
                            g.teeth,
                            g.gear.tooth_cycles.bending
                        );
                    }
                    // The grounded gear stands still and is engaged once a crank
                    // turn, which is the whole duty's worth of revolutions.
                    assert!(
                        (h.gears[0].gear.tooth_cycles.bending - turns).abs() <= 1.0,
                        "the grounded gear meets the wobble body once a crank turn"
                    );
                }
                _ => panic!("{name}: wrong kind"),
            }
        }
    }

    /// **The tip width is a bound on the addendum, not a target for it.**
    ///
    /// Exercised through a whole stage rather than in isolation: ask for a
    /// minimum tip width, solve, and measure the tip off the gear the stage
    /// actually built. Two things have to hold and they are different
    /// statements — the tip is never narrower than was asked for, and where the
    /// bound bit it bit *exactly*, leaving the tallest tooth that keeps the tip
    /// rather than an arbitrary shorter one.
    ///
    /// It used to be a target, because the addendum's only automatic value was
    /// the tooth this bound allows — so an addendum a designer typed went
    /// unbounded and one they left automatic was overwritten. Neither is what
    /// either control was for.
    #[test]
    fn the_tip_width_bounds_the_addendum_and_bites_exactly() {
        for want in [0.05, 0.15, 0.3] {
            for asked in [0.8, 1.0, 1.6] {
                let mut stage = SpurStage::default();
                for g in &mut stage.gears {
                    g.addendum = asked;
                    g.min_tip_width = want;
                }
                let r = solve_spur_stage(&stage, StageTorques::just(2.0), &library()).unwrap();

                for i in 0..2 {
                    let built = Tooth::new(stage.params_at(i, stage.shifts()[i]));
                    let got = 2.0 * built.ra * built.theta_a;
                    assert!(
                        got > want - 1e-9,
                        "gear {i} at h={asked}: tip {got} is under the {want} asked for"
                    );
                    let clamped = built.params.addendum < asked - 1e-12;
                    assert!(
                        !clamped || (got - want).abs() < 1e-9,
                        "gear {i} at h={asked}: held down to {} but the tip is {got}, not {want}",
                        built.params.addendum
                    );
                    assert!(
                        clamped || (built.params.addendum - asked).abs() < 1e-12,
                        "gear {i} at h={asked}: unclamped, so the addendum should be as asked"
                    );
                    // ...and the addendum reported is the one that produced it.
                    assert!((r.gears[i].addendum - built.params.addendum).abs() < 1e-12);
                }
            }
        }
    }

    /// **An automatic width with nothing to size it is said, not divided by.**
    ///
    /// The note has promised exactly that since it was written, and the stage
    /// then resolved the width to zero and divided by it: every stress came out
    /// infinite and every minimum width a NaN. Both cross the boundary as JSON
    /// `null` and draw as blanks, so the browser was honest by accident — while
    /// the CLI printed `inf`, and the generated TypeScript said `number` of a
    /// field that could arrive `null`.
    ///
    /// Asked of all three stage kinds that have the control, because it is one
    /// rule and this is the shape of a bound reaching the search it was written
    /// in and no other.
    #[test]
    fn a_width_with_no_rating_to_size_it_stands_where_it_was() {
        let lib = library();
        let off = FaceSources {
            bending: LoadCase {
                peak: false,
                cyclic: false,
            },
            contact: LoadCase {
                peak: false,
                cyclic: false,
            },
        };
        const GIVEN: f64 = 7.5;
        let gear = || StageGear {
            face_width: Auto::automatic(GIVEN),
            face_sources: off,
            ..StageGear::default()
        };

        let mut spur = SpurStage::default();
        for g in &mut spur.gears {
            *g = StageGear {
                teeth: g.teeth,
                ..gear()
            };
        }
        let mut set = PlanetaryStage::default();
        for g in [&mut set.sun, &mut set.planet, &mut set.ring] {
            *g = StageGear {
                teeth: g.teeth,
                profile_shift: g.profile_shift,
                ..gear()
            };
        }
        let mut hula = HulaStage::default();
        for g in &mut hula.gears {
            *g = StageGear {
                teeth: g.teeth,
                addendum: g.addendum,
                dedendum: g.dedendum,
                ..gear()
            };
        }

        let spur_r = solve_spur_stage(&spur, StageTorques::just(2.0), &lib).unwrap();
        let set_r = solve_planetary_stage(&set, 3000.0, StageTorques::just(2.0), &lib).unwrap();
        let hula_r = solve_hula_stage(&hula, 1000.0, StageTorques::just(2.0), &lib).unwrap();

        let members: Vec<&GearResult> = spur_r
            .gears
            .iter()
            .chain([&set_r.sun, &set_r.planet.gear, &set_r.ring])
            .chain(hula_r.gears.iter().map(|g| &g.gear))
            .collect();
        assert_eq!(members.len(), 9);
        for g in members {
            assert!(
                (g.face_width - GIVEN).abs() < 1e-12,
                "a width nothing sizes stands at the number in its box, not {}",
                g.face_width
            );
            // ...and with a width, every figure taken at one is a number.
            assert!(
                g.contact_stress.peak.is_finite()
                    && g.min_face_width.peak.contact.is_some_and(f64::is_finite),
                "contact: {} and {:?}",
                g.contact_stress.peak,
                g.min_face_width.peak.contact
            );
            if let Some(s) = g.bending_stress.peak {
                assert!(s.is_finite(), "bending: {s}");
            }
        }
        // The note still fires — the point is that it is now the *only* thing
        // that happens, not that it stopped happening.
        for (what, notes) in [("spur", &spur_r.notes), ("epicyclic", &set_r.notes)] {
            assert!(
                notes.iter().any(|n| n.is(key::STAGE_FACE_WIDTH_NO_SOURCE)),
                "{what} should still say no rating sizes the width"
            );
        }
    }

    /// **The sharing model reaches every member of every kind that has one.**
    ///
    /// It was a spur input only, so the one estimate this crate ships reached
    /// one stage of three — and a ring had no shared section at all, so even
    /// that stage would have rated one member of an internal mesh under the
    /// model and the other without.
    ///
    /// Asked as *does switching it on move the number*, member by member, since
    /// a member the input never reaches reports the same stress either way and
    /// looks exactly like one the model happens not to relieve. Above a virtual
    /// contact ratio of 2 there is no single-pair zone, the ramp never reaches a
    /// full share, and every member it reaches is relieved — so the fixtures are
    /// chosen to put each mesh there, which for a spur pair and an epicyclic set
    /// is an ordinary high-contact-ratio tooth.
    #[test]
    fn the_sharing_model_reaches_every_member_that_bends() {
        use crate::contact::LoadSharing;
        let lib = library();
        let tall = |g: &StageGear| StageGear {
            addendum: 1.35,
            ..g.clone()
        };
        let bending_of = |sharing: LoadSharing| {
            let mut spur = SpurStage {
                load_sharing: sharing,
                ..SpurStage::default()
            };
            for g in &mut spur.gears {
                *g = tall(g);
            }
            let mut set = PlanetaryStage {
                load_sharing: sharing,
                ..PlanetaryStage::default()
            };
            set.sun = tall(&set.sun);
            set.planet = tall(&set.planet);
            set.ring = tall(&set.ring);
            // **A hula stage needs a taller tooth than it can be built with**,
            // and that is the point of the row below rather than a defect in
            // the fixture: at 1.1 modules its meshes reach `ε_n ≈ 2.02` and its
            // teeth foul, which the stage reports. The rating path is the one a
            // buildable stage uses, so this is what says the input reaches it.
            let mut hula = HulaStage {
                load_sharing: sharing,
                ..HulaStage::default()
            };
            for g in &mut hula.gears {
                g.addendum = 1.1;
            }

            let s = solve_spur_stage(&spur, StageTorques::just(2.0), &lib).unwrap();
            let p = solve_planetary_stage(&set, 3000.0, StageTorques::just(2.0), &lib).unwrap();
            let h = solve_hula_stage(&hula, 1000.0, StageTorques::just(2.0), &lib).unwrap();
            let mut out: Vec<(String, Option<f64>)> = Vec::new();
            for (i, g) in s.gears.iter().enumerate() {
                out.push((format!("spur {i}"), g.bending_stress.peak));
            }
            for (what, g) in [
                ("sun", &p.sun),
                ("planet", &p.planet.gear),
                ("ring", &p.ring),
            ] {
                out.push((what.to_string(), g.bending_stress.peak));
            }
            for g in &h.gears {
                out.push((format!("hula z{}", g.teeth), g.gear.bending_stress.peak));
            }
            out
        };

        let off = bending_of(LoadSharing::None);
        let on = bending_of(LoadSharing::LinearRamp);
        assert_eq!(off.len(), on.len());
        let mut rated = 0;
        for ((what, a), (_, b)) in off.iter().zip(&on) {
            let (Some(a), Some(b)) = (a, b) else {
                continue; // a member with no notch has no bending either way
            };
            rated += 1;
            // **That it moved is the claim; which way is not.** This test is
            // about *reach* — every member that bends must see the model — and
            // the direction was asserted for years only because it happened to
            // hold. It does not in general: the swept maximum is a product of a
            // form factor rising toward the tip and a share falling away there,
            // so a member whose factor rises steeply enough is governed near its
            // tip, at a partial share, above what the unshared convention (full
            // load at the single-pair boundary) assumes. A planetary ring is
            // that member — its `Y_F` runs 2.49 to 0.14 across its flank — and
            // it comes out 2.4 % *higher* with sharing on. That is the sweep
            // doing its job rather than a fault in it, and the unshared
            // convention being the approximation it is documented as.
            assert!(
                (b - a).abs() / a > 1e-9,
                "{what}: sharing must reach a tooth that bends — {a} to {b}"
            );
        }
        assert!(
            rated >= 8,
            "most members should have a bending rating: {off:?}"
        );
    }

    /// **...and below that band it changes nothing, which is not the same as
    /// not reaching them.**
    ///
    /// The ramp's worst point is the largest `Y_F · Y_S · share`, and below
    /// `ε_n = 2` the single-pair boundary is in the sweep with a share of
    /// exactly 1 — so the maximum is the point the unshared rating already
    /// took, and the answer is the one already reported. A hula stage is the
    /// case worth pinning: **its meshes cannot reach the band at any proportion
    /// it can be built at**, running just above continuous contact by
    /// construction, so the control is offered and provably cannot bite there.
    #[test]
    fn below_the_band_the_model_reports_the_tooth_it_was_given() {
        use crate::contact::LoadSharing;
        let lib = library();
        let solve = |sharing| {
            let stage = HulaStage {
                load_sharing: sharing,
                ..HulaStage::default()
            };
            solve_hula_stage(&stage, 1000.0, StageTorques::just(2.0), &lib).unwrap()
        };
        let off = solve(LoadSharing::None);
        let on = solve(LoadSharing::LinearRamp);
        for (a, b) in off.gears.iter().zip(&on.gears) {
            assert_eq!(
                a.gear.bending_stress.peak, b.gear.bending_stress.peak,
                "z{}: below the band the model has nothing to find",
                a.teeth
            );
        }
        // ...and the reason, rather than the symptom: the shipped stage's
        // meshes are nowhere near the band. If a hula stage ever is, this fails
        // and the paragraph above needs rewriting.
        for m in &off.meshes {
            assert!(
                m.report.contact_ratios.transverse < 2.0,
                "a hula mesh above the band would change the claim: {}",
                m.report.contact_ratios.transverse
            );
        }
    }

    /// **Every search runs on every keystroke**, so each has to cost like an
    /// input and not like a build.
    ///
    /// All three, because the point is the slowest one: the pair's search was
    /// what this was written for, and by the time the epicyclic set had its own
    /// it was twice as dear and ungated. Each stage names its own bound, since
    /// what they do differs — an epicyclic candidate solves a planet and cuts a
    /// ring where a pair's builds two teeth.
    ///
    /// The bounds are loose, but only by about a decade. Wall-clock in a suite
    /// that runs its tests in parallel measures the machine as much as the
    /// code, so a bound near the measurement would fail on a loaded one and
    /// teach a reader to ignore it — while a bound far above it stops catching
    /// anything. They sit at roughly five times what each search costs (8 ms,
    /// 39 ms and 2.6 ms), which is loose enough for a busy machine and tight
    /// enough to catch the kind of regression that has actually happened here:
    /// 800 ms, 100 ms and 68 ms at various points, every time because something
    /// was built per candidate that nothing then read.
    ///
    /// # These went up, and why
    ///
    /// They were 10 / 60 / 20 ms against 0.7 / 10 / 1.8, and the first of those
    /// measurements was of a search doing **a sixth of its own work**:
    /// `auto::Search::budget` was a pool shared across the starts, so the first
    /// walk spent it and the other five never ran. Per walk they all run, which
    /// costs a pair eight times what it was paying and buys it nothing — its
    /// answer is the same at a fifth of the guard — while it is the whole of an
    /// epicyclic set's missing 3.2e-4.
    ///
    /// **A ceiling raised to fit a change needs its reason written down**, so:
    /// the number went up because the work went up, the work went up because it
    /// was being silently skipped, and the multiplier came *down* from ten to
    /// five so the gate did not go slack while the measurement grew. The
    /// optimiser is off by default, so nothing pays this unless it was asked
    /// for.
    #[test]
    fn every_search_is_quick_enough_to_type_over() {
        let lib = library();
        let tuned = Optimisation {
            enabled: true,
            ..Optimisation::default()
        };
        let each = |name: &str, ceiling: u64, f: &dyn Fn()| {
            let start = std::time::Instant::now();
            for _ in 0..5 {
                f();
            }
            let took = start.elapsed() / 5;
            assert!(
                took < std::time::Duration::from_millis(ceiling),
                "the {name} search took {took:?}, over its {ceiling} ms"
            );
        };

        let pair = SpurStage {
            optimisation: tuned,
            ..SpurStage::default()
        };
        each("pair's", 40, &|| {
            solve_spur_stage(&pair, StageTorques::just(2.0), &lib).unwrap();
        });

        let mut set = PlanetaryStage {
            optimisation: tuned,
            ..PlanetaryStage::default()
        };
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        each("epicyclic set's", 200, &|| {
            solve_planetary_stage(&set, 3000.0, StageTorques::just(2.0), &lib).unwrap();
        });

        let drive = HulaStage {
            optimisation: tuned,
            ..HulaStage::default()
        };
        each("hula stage's", 20, &|| {
            solve_hula_stage(&drive, 1000.0, StageTorques::just(2.0), &lib).unwrap();
        });
    }

    /// **The root round a designer asked for bounds the shift.**
    ///
    /// The tip round a cutter can leave shrinks as the shift rises — it bites
    /// less deep and the space it cuts narrows — so a fillet specified at one
    /// shift becomes unbuildable at a larger one. Without that bound the search
    /// pushes the shift up until some other limit stops it and hands back a
    /// tooth nobody can cut.
    ///
    /// Two things are asked of it: the pair it returns carries the round it was
    /// given, and asking for more round never buys more shift. Where the round
    /// cannot be cut at *any* admissible shift the search finds nothing and the
    /// stage falls back to the shifts it would have had, which the panel's
    /// existing note against the input is what explains.
    #[test]
    fn a_larger_root_round_holds_the_shift_down() {
        let stage = |rho: f64| {
            let gear = |teeth: u32| StageGear {
                teeth,
                root_radius: rho,
                ..SpurStage::default().gears[0].clone()
            };
            SpurStage {
                optimisation: Optimisation {
                    enabled: true,
                    ..Optimisation::default()
                },
                gears: [gear(9), gear(37)],
                ..SpurStage::default()
            }
        };
        let mut last = f64::INFINITY;
        let mut fell = false;
        for k in 0..=8 {
            let rho = f64::from(k) * 0.05;
            let s = stage(rho);
            let x = s.shifts();
            let sum = x[0] + x[1];
            // Where it optimised at all, the teeth it chose can be cut.
            let cuttable = (0..2).all(|i| {
                let p = s.params_at(i, x[i]);
                crate::auto::root_radius_fits(&p, p.dedendum)
            });
            if !cuttable {
                // The round is unreachable at every shift; nothing was chosen.
                assert_eq!(
                    x,
                    SpurStage {
                        optimisation: Optimisation::default(),
                        ..s
                    }
                    .shifts()
                );
                continue;
            }
            // **To the search's own stopping distance**, not to the bit. The
            // claim is about the *bound* — a larger round admits less shift —
            // and what reads it is a search that stops when its answer has
            // settled to `auto::Search::resolution`. Asserting past that asserts
            // where the walk happened to halt, which is not a fact about
            // gearing.
            assert!(
                sum <= last + crate::auto::Search::SHIPPED.resolution,
                "round {rho} bought shift: sum {sum} against {last}"
            );
            fell = fell || sum < last - 1e-6;
            last = sum;
        }
        assert!(fell, "the round never bound the shift at all");
    }

    /// The efficiency toggle is **additive**: a stage that never asked for it
    /// answers exactly as it did before the toggle existed, and a stage that
    /// does is moved somewhere else.
    #[test]
    fn a_stage_that_did_not_ask_keeps_the_shifts_it_had() {
        let stage = |on: bool| SpurStage {
            gears: [
                StageGear {
                    teeth: 17,
                    ..SpurStage::default().gears[0].clone()
                },
                StageGear {
                    teeth: 43,
                    ..SpurStage::default().gears[1].clone()
                },
            ],
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            ..SpurStage::default()
        };
        let plain = stage(false).shifts();
        let tuned = stage(true).shifts();
        assert!(
            (tuned[0] + tuned[1]) - (plain[0] + plain[1]) > 0.05,
            "the toggle should move the pair off its undercut floor, {tuned:?} from {plain:?}"
        );
        // ...and where it moves it, the pair loses less than it did.
        let lib = library();
        let loss = |on: bool| {
            1.0 - solve_spur_stage(&stage(on), StageTorques::just(2.0), &lib)
                .unwrap()
                .mesh
                .efficiency
                .forward
        };
        assert!(
            loss(true) < loss(false),
            "optimised loss {:.5} should beat the undercut minimum {:.5}",
            loss(true),
            loss(false)
        );
    }

    /// **Whatever a stage chooses, it can be cut.**
    ///
    /// The bound that matters is not any single one but that every stage asks
    /// the same questions. They did not: the root round bounded a pair and not
    /// an epicyclic set, and the hula stage checked its pinion for a
    /// pointed tip but never for the round it was given — so it was returning a
    /// pinion nobody could cut, and a worse answer for it.
    ///
    /// This asks the invariant rather than the wiring, so a stage added later
    /// that forgets `member_is_buildable` fails here rather than shipping.
    #[test]
    fn every_stage_chooses_a_member_that_can_be_cut() {
        use crate::GearParams;
        let cuttable = |p: &GearParams, what: &str| {
            let t = Tooth::new(*p);
            assert!(
                crate::auto::member_is_buildable(
                    &t,
                    Some(crate::auto::automatic_profile_shift(p, p.dedendum))
                ),
                "{what} at x {} cannot be cut: undercut {} severed {} round {}",
                p.profile_shift,
                t.undercut,
                t.severed,
                crate::auto::root_radius_fits(p, p.dedendum)
            );
        };

        let spur = SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..SpurStage::default()
        };
        for (i, x) in spur.shifts().iter().enumerate() {
            cuttable(&spur.params_at(i, *x), "the pair's gear");
        }

        let mut set = PlanetaryStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..PlanetaryStage::default()
        };
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        let built = set.built(set.shifts()).expect("the set has geometry");
        cuttable(&built.sun.params, "the sun");
        cuttable(&built.planet.params, "the planet");

        // The hula stage's rack-generated members are its pinions; its
        // rings are the shaper's and are not asked.
        let drive = HulaStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            ..HulaStage::default()
        };
        let r = solve_hula_stage(&drive, 1000.0, StageTorques::just(2.0), &test_library())
            .expect("the stage solves");
        for (i, g) in r.gears.iter().enumerate() {
            if g.ring {
                continue;
            }
            cuttable(
                &GearParams {
                    module: drive.module[i / 2],
                    pressure_angle: drive.pressure_angle,
                    helix_angle: drive.helix_angle,
                    teeth: g.teeth,
                    profile_shift: g.gear.profile_shift,
                    addendum: drive.gears[i].addendum,
                    dedendum: drive.gears[i].dedendum,
                    root_radius: drive.gears[i].root_radius,
                    thickness_mod: drive.thickness_mod[i / 2],
                    ..GearParams::default()
                },
                "the stage's pinion",
            );
        }
    }

    /// **The clearance is taken by whatever is free to absorb it.**
    ///
    /// With the distance automatic, the distance absorbs it — it is the
    /// zero-backlash distance opened out, and that opening is the backlash.
    /// With the distance given and the shifts pinned at their undercut minimum
    /// nothing is left to move, so the input goes unread and the answer says so
    /// by reporting zero. With the distance given *and* the shifts being chosen,
    /// the shifts absorb it: the pair closes to zero backlash a clearance inside
    /// the housing, so the designer gets both the distance they specified and
    /// the play they asked for.
    #[test]
    fn the_clearance_is_taken_by_whatever_is_free_to_absorb_it() {
        let lib = library();
        let free = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        // A housing the pair can actually meet: a clearance inside it is the
        // distance the automatic solve already closes to.
        let asked = free.centre_distance_nominal + 0.05;
        let at = |on: bool| SpurStage {
            optimisation: Optimisation {
                enabled: on,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            clearance: 0.05,
            ..SpurStage::default()
        };

        // **Nothing free to absorb it, so the shifts do not move — and the gap
        // is whatever the housing leaves.** Here that is the 0.05 the fixture
        // built the housing out of, so the number matches the input by
        // arithmetic rather than because the input was read.
        //
        // This used to assert `clearance == 0.0`, on the reading that an unread
        // input should report as nothing. It is a *gap*, and the gap is
        // 0.05: a centre distance is the true distance and a clearance is what
        // portion of it is clearance, so the pair cannot run 0.05 mm wide of its
        // own nominal and report none.
        let pinned = solve_spur_stage(&at(false), StageTorques::just(2.0), &lib).unwrap();
        assert!(
            (pinned.clearance - (pinned.centre_distance - pinned.centre_distance_nominal)).abs()
                < 1e-12,
            "read {} against a gap of {}",
            pinned.clearance,
            pinned.centre_distance - pinned.centre_distance_nominal
        );
        assert!(
            (pinned.clearance - 0.05).abs() < 1e-9,
            "the housing is 0.05 outside the nominal, so the gap is 0.05, not {}",
            pinned.clearance
        );

        // The shifts free: they take it, and the backlash is the one asked for.
        let chosen = solve_spur_stage(&at(true), StageTorques::just(2.0), &lib).unwrap();
        assert!((chosen.clearance - 0.05).abs() < 1e-12);
        assert!(
            (chosen.centre_distance - asked).abs() < 1e-9,
            "the housing still holds: {}",
            chosen.centre_distance
        );
        assert!(
            (chosen.centre_distance - chosen.centre_distance_nominal - 0.05).abs() < 1e-6,
            "the pair should close to zero backlash 0.05 inside the housing, not {}",
            chosen.centre_distance - chosen.centre_distance_nominal
        );

        // And with the distance automatic it is read either way, as it always was.
        for on in [false, true] {
            let r = solve_spur_stage(
                &SpurStage {
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    clearance: 0.05,
                    ..SpurStage::default()
                },
                StageTorques::just(2.0),
                &lib,
            )
            .unwrap();
            assert!((r.clearance - 0.05).abs() < 1e-12);
        }
    }

    /// A centre distance the designer typed is a **constraint on the pair**, not
    /// a suggestion the optimiser may overrule: the shifts it chooses still add
    /// up to the housing it was given, and only their split is free.
    #[test]
    fn a_given_centre_distance_still_sets_the_distance() {
        let lib = library();
        let free = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();
        let asked = free.centre_distance_nominal + 0.4;
        let stage = SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            centre_distance: Auto::fixed(asked),
            ..SpurStage::default()
        };
        let r = solve_spur_stage(&stage, StageTorques::just(2.0), &lib).unwrap();
        assert!(
            (r.centre_distance - asked).abs() < 1e-9,
            "asked for {asked}, ran at {}",
            r.centre_distance
        );
    }

    /// **A train that cannot be solved says which stage stopped it.**
    ///
    /// The chain is why it has no answer at all: a stage that fails takes the
    /// shaft line with it, so the stages after it have no speed to be solved at
    /// and the pass that rates every stage against the efficiencies downstream
    /// cannot run. That is a fair reason to return nothing, and no reason at
    /// all to leave a reader hunting for which of five stages is the one to
    /// edit — the number was there at the point of failure and simply was not
    /// kept.
    #[test]
    fn a_train_that_fails_names_the_stage_that_failed() {
        let mut train = two_stage();
        train.stages.push(Stage::Spur(SpurStage::default()));
        assert!(
            solve_train(&train, &library()).is_ok(),
            "three good stages solve"
        );

        // A centre distance of zero is not a mesh, and it is the middle stage's.
        let Stage::Spur(s) = &mut train.stages[1] else {
            panic!("the stage this test set up is a spur one")
        };
        s.centre_distance = Auto::fixed(0.0);

        let e = solve_train(&train, &library()).expect_err("a mesh at no distance is not a train");
        let TrainError::InStage { stage, cause } = &e else {
            panic!("the failure should name the stage it happened in, got {e:?}")
        };
        assert_eq!(*stage, 1, "the middle stage is the one that failed");
        // ...and the reason is the stage's own, unchanged by being carried.
        assert!(
            matches!(**cause, TrainError::Mesh(_)),
            "the cause should be the mesh's, got {cause:?}"
        );
        // The note a reader sees is the cause's, so nothing is invented to
        // carry the number.
        use crate::note::Explain;
        assert_eq!(e.note().key, cause.note().key);
        assert!(e.to_string().starts_with("stage 2: "), "{e}");
    }

    /// Both shifts given leaves the optimiser nothing to choose, and it says so
    /// by handing back what it was given rather than by failing.
    ///
    /// **A negative shift somebody meant is not an undercut one.** The bound a
    /// given value is held to is the true minimum, so −0.1 on a 43-tooth wheel
    /// — whose flank is clear down to −1.76 — is a decision about centre
    /// distance and is left exactly where it was put. Only a value that
    /// genuinely undercuts is raised, which the second half asks of a 17-tooth
    /// pinion, clear only above +0.006.
    #[test]
    fn a_fully_specified_pair_is_left_alone() {
        let given = |teeth: u32, x: f64| StageGear {
            teeth,
            profile_shift: Auto::fixed(x),
            ..SpurStage::default().gears[0].clone()
        };
        let tuned = |gears: [StageGear; 2]| SpurStage {
            optimisation: Optimisation {
                enabled: true,
                ..Optimisation::default()
            },
            gears,
            ..SpurStage::default()
        };
        assert_eq!(
            tuned([given(17, 0.3), given(43, -0.1)]).shifts(),
            [0.3, -0.1]
        );

        // ...and the same pinion asked for a shift its own flank will not carry.
        let raised = tuned([given(17, -0.1), given(43, -0.1)]).shifts();
        assert!(
            raised[0] > -0.1 && raised[0] < 0.01,
            "a pinion below its undercut minimum should be raised to it, not past it: {raised:?}"
        );
        assert!(
            (raised[1] + 0.1).abs() < 1e-12,
            "and the wheel, which undercuts nowhere near here, left alone: {raised:?}"
        );

        // With the constraint off, the number stands however it undercuts.
        let loose = |teeth: u32, x: f64| StageGear {
            no_undercut: false,
            ..given(teeth, x)
        };
        assert_eq!(
            tuned([loose(17, -0.1), loose(43, -0.1)]).shifts(),
            [-0.1, -0.1]
        );
    }

    /// Setting the centre distance by hand takes clearance out of the picture,
    /// which is what the specification requires and what changes the backlash.
    #[test]
    fn a_manual_centre_distance_ignores_the_clearance() {
        let lib = library();
        let auto = solve_spur_stage(&SpurStage::default(), StageTorques::just(2.0), &lib).unwrap();

        // The same distance, set by hand, with a clearance that must be ignored.
        let manual = solve_spur_stage(
            &SpurStage {
                centre_distance: Auto::fixed(auto.centre_distance_nominal),
                clearance: 0.5,
                ..SpurStage::default()
            },
            StageTorques::just(2.0),
            &lib,
        )
        .unwrap();

        assert!((manual.centre_distance - auto.centre_distance_nominal).abs() < 1e-12);
        // At the zero-backlash distance there is, by construction, no backlash.
        assert!(manual.mesh.backlash_by_drive().forward.nominal.abs() < 1e-9);
        // Whereas the automatic one carries its clearance into real backlash.
        assert!(auto.mesh.backlash_by_drive().forward.nominal > 0.0);
    }

    /// An override has to reach the arithmetic, not just the display — and each
    /// load case has to read **its own** allowable.
    ///
    /// Doubling an allowable must quarter the face width contact asks for, since
    /// `b_min ∝ (σ_H/σ_allow)²`. Which allowable does it is the whole point of
    /// separating the cases: the peak case is judged against the ultimate and
    /// the cyclic one against the fatigue figure, so each override moves exactly
    /// one of the two widths and leaves the other alone.
    #[test]
    fn a_material_override_changes_the_answer() {
        let lib = library();
        let off = LoadCase {
            peak: false,
            cyclic: false,
        };
        let auto_width = |sources: FaceSources, o: Overrides| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.face_width = Auto::automatic(0.0);
                g.face_sources = sources;
                g.material_overrides = o;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib).unwrap()
        };
        let contact_only = |case: Case| FaceSources {
            bending: off,
            contact: LoadCase::of(|c| c == case),
        };

        for (case, other) in [(Case::Cyclic, Case::Peak), (Case::Peak, Case::Cyclic)] {
            let base = auto_width(contact_only(case), Overrides::default());
            // Twice **this material's** figure, read from the answer rather than
            // written down again: a test that repeats the library's numbers
            // stops testing the arithmetic the moment the library moves.
            let doubled = 2.0 * allowable(&base.gears[0].material, case);
            let over = match case {
                Case::Peak => Overrides {
                    ultimate_allowable: Some(doubled),
                    ..Default::default()
                },
                Case::Cyclic => Overrides {
                    fatigue_allowable: Some(doubled),
                    ..Default::default()
                },
            };

            let width = |sources, o| auto_width(sources, o).gears[0].face_width;
            let ratio = base.gears[0].face_width / width(contact_only(case), over);
            assert!(
                (ratio - 4.0).abs() < 1e-9,
                "{case:?}: doubling the allowable should quarter the width: ratio {ratio}"
            );
            // ...and it moved only the case it belongs to.
            let untouched = width(contact_only(other), Overrides::default());
            assert_eq!(untouched, width(contact_only(other), over));
        }

        // ...and the reported material says the number came from the user.
        let doubled = auto_width(
            FaceSources::default(),
            Overrides {
                fatigue_allowable: Some(2.0 * 750.0),
                ..Default::default()
            },
        );
        assert_eq!(
            doubled.gears[0].material.fatigue_allowable.basis,
            crate::material::Basis::Overridden
        );
        assert_eq!(
            auto_width(FaceSources::default(), Overrides::default()).gears[0]
                .material
                .fatigue_allowable
                .basis,
            crate::material::Basis::Estimated
        );
    }

    /// Overriding the modulus moves contact stress, and by the right law.
    #[test]
    fn overriding_the_modulus_moves_contact_stress_as_the_square_root() {
        let lib = library();
        let at = |e: Option<f64>| {
            let mut s = SpurStage::default();
            for g in &mut s.gears {
                g.material_overrides = Overrides {
                    elastic_modulus: e,
                    ..Default::default()
                };
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib)
                .unwrap()
                .mesh
                .contact_stress_at_pitch_point
                .peak
        };
        let base = at(None);
        let quarter = at(Some(190_000.0 / 4.0));
        assert!(
            (base / quarter - 2.0).abs() < 1e-9,
            "sigma_H goes as sqrt(E*): {base} vs {quarter}"
        );
    }

    #[test]
    fn an_unknown_material_is_named_rather_than_swallowed() {
        let mut s = SpurStage::default();
        s.gears[0].material = "unobtainium".into();
        let e = solve_spur_stage(&s, StageTorques::just(2.0), &library()).unwrap_err();
        assert!(matches!(e, TrainError::UnknownMaterial(ref n) if n == "unobtainium"));
        assert!(e.to_string().contains("unobtainium"));
    }

    #[test]
    fn an_empty_train_says_so() {
        let t = Train {
            input_speed: 1.0,
            input_torque: 1.0,
            back_driving_torque: 0.0,
            operating_torque: 1.0,
            reversed_bending: false,
            actuation: Actuation::default(),
            stages: vec![],
        };
        assert_eq!(solve_train(&t, &library()).unwrap_err(), TrainError::Empty);
    }

    /// **A tooth cycle count is what the browser used to round, and now what the
    /// model returns.**
    ///
    /// The `Math.ceil` in `TrainPanel.svelte` was a modelling decision on the
    /// side of the boundary with no tests, and the CLI printed the *unrounded*
    /// figure beside it — one model with two answers, which is the fault this
    /// crate spends most of its gates on.
    ///
    /// The revolutions are written out here rather than referenced, so the check
    /// is against the shaft line rather than against the code that counts it.
    /// Every count must be whole, and must be the ceiling taken in the right
    /// place: over the whole duty for a one-way drive, and **within one
    /// actuation** for a reversing one, where a tooth part way through an
    /// actuation has still been loaded by that actuation and by every one after.
    #[test]
    fn every_cycle_count_is_the_ceiling_of_the_revolutions_it_replaced() {
        let lib = test_library();
        for (train, what) in [
            (two_stage(), "two spur stages"),
            (
                Train {
                    actuation: Actuation::Continuous {
                        operating_speed: 2400.0,
                        runtime_hours: 1000.0,
                    },
                    ..two_stage()
                },
                "continuous",
            ),
            (
                Train {
                    reversed_bending: false,
                    actuation: Actuation::Intermittent {
                        range_degrees: 25.0,
                        actuations: 1000,
                        reversing: false,
                    },
                    ..two_stage()
                },
                "intermittent",
            ),
            (
                Train {
                    reversed_bending: false,
                    actuation: Actuation::Intermittent {
                        range_degrees: 25.0,
                        actuations: 1000,
                        reversing: true,
                    },
                    ..two_stage()
                },
                "reversing",
            ),
        ] {
            let r = solve_train(&train, &lib).expect(what);

            // The revolutions each member turns, before anything rounds them.
            let ratios: Vec<f64> = r.stages.iter().map(StageResult::ratio).collect();
            for (k, s) in r.stages.iter().enumerate() {
                let upstream: f64 = ratios[..k].iter().product();
                let speed_in = train.input_speed / upstream;
                let speeds = [speed_in, speed_in / ratios[k]];
                let expected = [0usize, 1].map(|i| {
                    let to_output: f64 = if i == 0 {
                        ratios[k..].iter().product()
                    } else {
                        ratios[k + 1..].iter().product()
                    };
                    match train.actuation {
                        Actuation::Intermittent {
                            range_degrees,
                            actuations,
                            reversing,
                        } => {
                            let each = (range_degrees / 360.0) * to_output;
                            let n = f64::from(actuations);
                            if reversing {
                                let bending = each.ceil() * n;
                                Cycles {
                                    bending,
                                    contact: bending / 2.0,
                                }
                            } else {
                                let n = (each * n).ceil();
                                Cycles {
                                    bending: n,
                                    contact: n,
                                }
                            }
                        }
                        Actuation::Continuous {
                            operating_speed,
                            runtime_hours,
                        } => {
                            let scale = operating_speed / train.input_speed;
                            let n = (speeds[i] * scale * 60.0 * runtime_hours).ceil();
                            Cycles {
                                bending: n,
                                contact: n,
                            }
                        }
                    }
                });

                let Some(sp) = s.as_spur() else { continue };
                for (i, (g, want)) in sp.gears.iter().zip(expected).enumerate() {
                    let got = g.tooth_cycles;
                    assert_eq!(got, want, "{what}, stage {k} gear {i}");
                    assert_eq!(
                        got.bending,
                        got.bending.trunc(),
                        "{what}: {} is not a whole count",
                        got.bending
                    );
                }
            }
        }
    }

    /// **An automatic face width has to satisfy the mesh, not one gear.**
    ///
    /// The narrower face carries the pair, so a width sized to its own gear's
    /// requirement satisfies nothing: give one gear a weaker material and it
    /// asks for more face, while the other — sized to its own smaller figure —
    /// pulls the effective width, and the weak gear with it, under what the weak
    /// gear needed.
    ///
    /// Stated as the invariant rather than as the arithmetic: **at an automatic
    /// width, every enabled rating is met**. That is what the control claims to
    /// do, and it is false in both directions if either gear is sized alone.
    #[test]
    fn an_automatic_face_width_satisfies_every_enabled_rating_of_both_gears() {
        let lib = library();
        let weak = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            ..Default::default()
        };
        // A matched pair, then each gear in turn made much weaker than the
        // other, so whichever gear governs the mesh is the one that changes.
        for over in [
            [Overrides::default(), Overrides::default()],
            [weak(250.0), Overrides::default()],
            [Overrides::default(), weak(250.0)],
        ] {
            let mut stage = SpurStage::default();
            for (g, o) in stage.gears.iter_mut().zip(over) {
                g.face_width = Auto::automatic(0.0);
                g.material_overrides = o;
            }
            let r = solve_spur_stage(&stage, StageTorques::just(2.0), &lib).unwrap();
            let effective = r.gears[0].face_width.min(r.gears[1].face_width);
            assert!(effective > 0.0);

            for (i, g) in r.gears.iter().enumerate() {
                let sources = &stage.gears[i].face_sources;
                for case in [Case::Peak, Case::Cyclic] {
                    let asks = g.min_face_width.get(case);
                    if *sources.contact.get(case) {
                        if let Some(c) = asks.contact {
                            assert!(
                                effective >= c * (1.0 - 1e-9),
                                "gear {i} {case:?} contact needs {c} mm, mesh carries {effective}"
                            );
                        }
                    }
                    if let (true, Some(b)) = (*sources.bending.get(case), asks.bending) {
                        assert!(
                            effective >= b * (1.0 - 1e-9),
                            "gear {i} {case:?} bending needs {b} mm, mesh carries {effective}"
                        );
                    }
                }
            }
        }
    }

    /// **The two gears of a mesh are rated at different *points*, not at
    /// different curvatures.**
    ///
    /// The mechanism matters, because the wrong one is very plausible: two teeth
    /// in mesh do have different flank curvatures, so it looks as though they
    /// should carry different stresses. They do not — Hertz reaches the contact
    /// through the *gap*, which depends on the individual radii only as their
    /// sum, and each body is then a half-space under a shared pressure. At one
    /// instant there is one pressure.
    ///
    /// What separates them is *when* each is rated: each gear's dedendum carries
    /// the load alone at one end of the path, and that is where its own pitting
    /// is assessed. So this pins three things — the shared figure is shared and
    /// reaches both materials, the two governing figures differ and each is its
    /// own end of the path, and an allowable moves a width and never a stress.
    #[test]
    fn the_two_gears_of_a_mesh_are_rated_at_different_points() {
        let lib = library();
        // At a **fixed** width: an automatic one is inverted from the stress, so
        // it lands the stress on the allowable and hides the material.
        let solved = |auto: bool, over: [Overrides; 2]| {
            let mut s = SpurStage::default();
            for (g, o) in s.gears.iter_mut().zip(over) {
                g.face_width = if auto {
                    Auto::automatic(0.0)
                } else {
                    Auto::fixed(10.0)
                };
                g.material_overrides = o;
            }
            solve_spur_stage(&s, StageTorques::just(2.0), &lib).unwrap()
        };
        let modulus = |e: f64| Overrides {
            elastic_modulus: Some(e),
            ..Default::default()
        };

        // --- the shared figure. Softening **either** gear softens the pair, so
        // neither material is being ignored, and `1/E*` is symmetric in the two
        // so it does not matter which was softened.
        let base = solved(false, [Overrides::default(), Overrides::default()]);
        let soft_first = solved(false, [modulus(70_000.0), Overrides::default()]);
        let soft_second = solved(false, [Overrides::default(), modulus(70_000.0)]);
        let pitch = |r: &SpurResult| r.mesh.contact_stress_at_pitch_point.peak;
        for (r, which) in [(&soft_first, "gear 1"), (&soft_second, "gear 2")] {
            assert!(
                pitch(r) < pitch(&base),
                "softening {which} must soften the pair: {} against {}",
                pitch(r),
                pitch(&base)
            );
        }
        assert!(
            (pitch(&soft_first) - pitch(&soft_second)).abs() < 1e-9,
            "1/E* is symmetric in the two gears"
        );

        // --- the two ratings. On this pair they differ, and each is at least
        // the shared figure — a gear is rated at the worse of the pitch point
        // and its own end of the path, never below it.
        let (a, b) = (
            base.gears[0].contact_stress.peak,
            base.gears[1].contact_stress.peak,
        );
        assert!(
            a != b,
            "17/43 is not symmetric, so its two gears are not rated alike: {a} and {b}"
        );
        for g in &base.gears {
            assert!(g.contact_stress.peak >= pitch(&base));
        }
        // ...and the envelope is the worse of them, which is what the *mesh*
        // would be rated on with no member named.
        assert!((a.max(b) - base.gears[0].contact_stress.peak.max(b)).abs() < 1e-12);

        // --- the allowable. At a fixed width again, and for a reason worth
        // stating: with an *automatic* width the allowable does reach the stress,
        // because it moves the width the pair is rated at. Held still, it moves
        // only what it should.
        let allowable = |x: f64| Overrides {
            ultimate_allowable: Some(x),
            fatigue_allowable: Some(x),
            ..Default::default()
        };
        let wide = base;
        let half = allowable(0.5 * super::allowable(&wide.gears[1].material, Case::Cyclic));
        let derated = solved(false, [Overrides::default(), half]);
        assert_eq!(
            derated.mesh.contact_stress_at_pitch_point, wide.mesh.contact_stress_at_pitch_point,
            "an allowable is not a stress and must not move one"
        );
        let contact_width = |r: &SpurResult| {
            r.gears[1]
                .min_face_width
                .cyclic
                .contact
                .expect("a spur member is contact-rated")
        };
        let (was, now) = (contact_width(&wide), contact_width(&derated));
        assert!(
            (now / was - 4.0).abs() < 1e-9,
            "halving the allowable should quadruple the width: {was} to {now}"
        );
        assert_eq!(
            derated.gears[0].min_face_width.cyclic.contact,
            wide.gears[0].min_face_width.cyclic.contact,
            "and it must not reach the other gear"
        );
    }

    /// **A load exists only where it is reacted.**
    ///
    /// A back-driving torque on a train of ordinary spur stages reaches no
    /// number: every stage can be driven backward, so the load simply turns the
    /// train and nothing holds it. Put one self-locking stage in the way and the
    /// load stops there — that stage and everything downstream of it carry it,
    /// and everything upstream still sees nothing.
    #[test]
    fn a_back_driving_load_is_carried_only_where_something_reacts_it() {
        let lib = test_library();
        let mut t = two_stage();
        t.back_driving_torque = 5.0;
        let r = solve_train(&t, &lib).unwrap();
        for s in &r.stages {
            for g in &spur(s).gears {
                assert_eq!(
                    g.back_driving_torque, None,
                    "a back-drivable train reacts nothing"
                );
            }
        }

        // The same load against a stage that cannot be driven backward. A worm
        // with enough friction locks, and then the load stops there: the worm
        // stage carries it, and the spur stage ahead of it carries none.
        t.stages.push(Stage::Worm(WormStage {
            sliding_friction: 0.3,
            static_friction: 0.3,
            ..WormStage::default()
        }));
        let r = solve_train(&t, &lib).unwrap();
        let worm = r.stages[2].as_worm().expect("the third stage is a worm");
        assert!(
            worm.efficiency.locked().backward,
            "this worm was meant to lock: backward efficiency {}",
            worm.efficiency.backward
        );
        // The wheel is on the shaft the load enters by and carries all of it;
        // the worm is at the far end of a mesh that cannot pass it, so its shaft
        // carries **none** — which is what a locked stage means and is not the
        // same as the case being absent (`a_self_locking_worm_reports_the_load_it_reacts`).
        assert_eq!(
            worm.members
                .iter()
                .map(|m| m.back_driving_torque)
                .collect::<Vec<_>>(),
            vec![Some(0.0), Some(5.0)],
            "the stage that reacts the load carries it, on the member the load \
             is on"
        );
        for s in &r.stages[..2] {
            for g in &spur(s).gears {
                assert_eq!(
                    g.back_driving_torque, None,
                    "nothing upstream of a self-locking stage sees the load"
                );
            }
        }

        // ...and the train says which stage held it, rather than leaving the
        // reader to infer it from a column of dashes.
        assert!(r
            .notes
            .iter()
            .any(|n| n.is(key::TRAIN_BACK_DRIVING_REACTED_AT)));

        // **The two torques are two facts, and the forward one is not the peak
        // case.** Put the locking stage first, so the spur stages after it carry
        // the load as well: a gear's `torque` must stay the torque it sees
        // driving forward even when the back-driving figure is the larger of the
        // two, which is what the peak *rating* uses.
        let mut t = two_stage();
        t.back_driving_torque = 500.0;
        t.stages.insert(
            0,
            Stage::Worm(WormStage {
                sliding_friction: 0.3,
                static_friction: 0.3,
                ..WormStage::default()
            }),
        );
        let r = solve_train(&t, &lib).unwrap();
        let forward_only = solve_train(
            &Train {
                back_driving_torque: 0.0,
                ..t.clone()
            },
            &lib,
        )
        .unwrap();
        let mut seen = 0;
        for (loaded, plain) in r.stages[1..].iter().zip(&forward_only.stages[1..]) {
            for (g, unloaded) in spur(loaded).gears.iter().zip(&spur(plain).gears) {
                let back = g
                    .back_driving_torque
                    .expect("carried downstream of the lock");
                assert!(back > g.torque, "this fixture is meant to load it backward");
                assert_eq!(
                    g.torque, unloaded.torque,
                    "a back-driving load must not move the forward torque"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 4, "both spur stages, both gears");
    }
}
