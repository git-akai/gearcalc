//! The hula stage: a drive's geometry, the parts it describes, and what those
//! parts do when they are put together.
//!
//! [`crate::hula`] solves the arrangement — one crank offset and the four
//! profile shifts that let both meshes run at it. This builds the gears those
//! numbers describe and asks each pair what it thinks, because a drive that
//! closes algebraically can still be one whose teeth foul, and at one tooth of
//! difference that is the likely outcome rather than the unlucky one.
//!
//! # What this stage reads, and what it does not
//!
//! A [`StageGear`] describes a gear for every stage kind here, so it carries
//! more than this one asks of it. Read: the tooth count, the addendum, the
//! dedendum and the root radius. Not read: the **profile shift**, because a
//! shift is what the offset spends and [`crate::hula::Split`] says which member
//! is given it; and everything a rating needs — face width, material,
//! overrides — which belong with the load cases and arrive with them.
//!
//! # The three shafts
//!
//! A hula is an epicyclic set: gear 1 is held, the crank is the carrier and the
//! input, and gear 4 is the output; the wobble body carrying gears 2 and 3 is
//! the planet. Its basic ratio is `i₀ = z₂z₄/(z₁z₃)`, which is the same two
//! products the reduction is written in.
//!
//! That basic ratio is also what an efficiency would be read through. The
//! power-flow solve in [`crate::planetary::power`] is about a three-shaft set
//! with a basic ratio and a fixed-carrier efficiency, and asks nothing of a sun
//! or a ring but their tooth counts — so it is a solve this arrangement can be
//! put through, given a basic ratio rather than a set of planetary counts to
//! derive one from.

use crate::contact::{efficiency, ContactPath, Directional, Drive};
use crate::hula::{self, Offset, Split, Teeth};
use crate::mesh::{Mesh, MeshError, MeshKind, MeshSide};
use crate::note::Note;
use crate::planetary::{self, Arrangement, PlanetaryShaft};

use crate::ring::{mesh_with, Cutter, Ring};
use crate::tooth::Tooth;
use crate::train::{Optimisation, StageGear};
use crate::{Auto, GearParams};

/// Why a hula stage could not be solved.
///
/// Its own type rather than a widening of [`super::TrainError`]: a drive that
/// does not exist and a pair that cannot mesh are different findings, and the
/// train has no arm to put either in until this stage is one of its kinds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// The arrangement has no geometry — see [`crate::hula::Error`].
    Drive(hula::Error),
    /// A pair the arrangement produced will not mesh.
    Mesh(MeshError),
}

impl From<hula::Error> for Error {
    fn from(e: hula::Error) -> Self {
        Self::Drive(e)
    }
}

/// A hula stage as its inputs describe it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct HulaStage {
    /// Normal module of each mesh, mm. Two, because the pairs need not share
    /// one — only the offset they run at.
    pub module: [f64; 2],
    /// Normal pressure angle, degrees. Shared by both meshes.
    pub pressure_angle: f64,
    /// Helix angle, degrees. Shared.
    pub helix_angle: f64,
    /// `k` for each mesh's **pinion**. Its ring takes the same figure, because
    /// on a ring `k` describes the space, and a pinion and a ring that mesh
    /// want the same one rather than complementary ones.
    pub thickness_mod: [f64; 2],
    /// Coefficient of friction in each mesh.
    pub sliding_friction: [f64; 2],
    /// Coefficient of **static** friction in each mesh, for breaking away.
    ///
    /// Whether a drive turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`].
    pub static_friction: [f64; 2],
    /// The smallest far-side tip gap any mesh may run at, mm.
    ///
    /// A minimum, and checked whether or not the offset is taken from it: an
    /// offset that fails it describes a drive that could be built and would
    /// foul, which is a thing a designer is owed the number for rather than a
    /// refusal.
    pub clearance: f64,
    /// Added to the crank offset, mm — the running clearance.
    ///
    /// **One number, because there is one crank.** Both meshes are separated by
    /// the same distance by construction, so a clearance on each could be set
    /// to disagree about a distance that is physically single.
    pub running_clearance: f64,
    pub tolerance_plus: f64,
    pub tolerance_minus: f64,
    /// **The crank offset, or automatic.**
    ///
    /// The same shape every stage's centre distance has, because it is the same
    /// decision: automatic derives the distance from the clearances the parts
    /// have to keep, and a number given by hand is the distance to run at. The
    /// arrangement below keeps its own vocabulary for that — [`Offset`] says
    /// *what decides* the offset rather than how it was entered — and this is
    /// the one place the two are translated.
    pub offset: Auto<f64>,
    /// What the drive is asked to optimise, and what it may not do to get
    /// there. See [`Optimisation`].
    ///
    /// A pair's shift *sum* is fixed by the offset the crank has to reach, so
    /// within a mesh only the division between ring and pinion is free — and
    /// that division is worth real efficiency. The contact ratio defaults lower
    /// here than the shared default, for a reason that is the drive's rather
    /// than a relaxation of the rule: a mesh of one tooth of difference has a
    /// very short path and sits just above continuous contact at every split it
    /// can be built at, so a pair's usual 1.2 of design margin would forbid the
    /// mechanism rather than constrain it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: Optimisation,
    /// The shaper each mesh's ring is cut with.
    ///
    /// **A shaper has to be smaller than the ring it cuts**, and the rings here
    /// are small: a tool larger than its workpiece is clamped down to the
    /// ring's own count and then reaches none of its flank, leaving no fillet
    /// at all. Both defaults are well below the default rings for that reason.
    pub cutter: [Cutter; 2],
    /// The four gears, in [`Teeth`]'s order: the grounded one, the two that
    /// ride the wobble body, then the output.
    pub gears: [StageGear; 4],
}

impl HulaStage {
    /// **The minimum clearance this drive is actually held to.**
    ///
    /// It is what *sets* the crank offset, so it is read only while the offset
    /// is being derived. Given the offset by hand, the far-side gap is whatever
    /// that offset leaves — reported, but not asked for — and the input goes
    /// unread.
    #[must_use]
    pub fn clearance_taken(&self) -> f64 {
        if self.offset.auto {
            self.clearance
        } else {
            0.0
        }
    }
}

impl Default for HulaStage {
    fn default() -> Self {
        // **The addendum belongs to the difference, not to the drive.** Four
        // teeth of difference runs at a far lower operating pressure angle than
        // one does, so the tooth that cleared at 0.8 module reaches past the
        // interference limit here; 0.7 clears both meshes, leaves a transverse
        // contact ratio of 1.37, and is worth six points of efficiency over 0.8
        // into the bargain. A drive taken to another difference will want its
        // own, and the interference readout is what says so.
        let gear = |teeth: u32| StageGear {
            teeth,
            addendum: crate::params::Auto::fixed(0.7),
            dedendum: 1.0,
            ..StageGear::default()
        };
        Self {
            module: [1.0, 1.0],
            pressure_angle: 20.0,
            helix_angle: 0.0,
            thickness_mod: [1.0, 1.0],
            sliding_friction: [0.08, 0.08],
            static_friction: [0.16, 0.16],
            // **The tips set this, not the far-side gap.** A fifth of a module
            // leaves the two tip circles overlapping where they cross, 134° from
            // the line of centres; three tenths clears at every tooth count
            // tried, and the margin is reported so a design can be taken closer.
            clearance: 0.3,
            running_clearance: 0.02,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            offset: Auto::automatic(0.0),
            optimisation: Optimisation {
                min_contact_ratio: 1.0,
                ..Optimisation::default()
            },
            // **A shaper is coupled to the shift it has to cut**, not only to
            // the ring's size: the clearance drives the ring's shift up, and a
            // tool that reached its flank at one shift stops reaching it at a
            // larger one. Twenty teeth cuts the shipped rings clean over the
            // gaps worth running, and is a tool somebody stocks; a ring far
            // from these counts will want its own, and says so through its
            // clamps rather than quietly coming out without a fillet.
            cutter: [Cutter {
                teeth: 20,
                ..Cutter::default()
            }; 2],
            // `N ± d` about 61, at four teeth of difference on each mesh —
            // the same `[N+d, N, N−d, N]` arrangement the counts have always
            // taken, at a size and a difference a real reducer is built at.
            gears: [gear(65), gear(61), gear(57), gear(61)],
        }
    }
}

/// One gear of a solved drive.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct HulaGear {
    pub teeth: u32,
    /// Whether this member is the ring of its pair — an outcome of the tooth
    /// counts rather than an input.
    pub ring: bool,
    /// The shift in force, after the split has been applied.
    pub profile_shift: f64,
    pub pitch_radius: f64,
    pub base_radius: f64,
    /// Tip radius, mm. **Smaller** than the pitch radius on a ring.
    pub tip_radius: f64,
    /// Root radius, mm. Larger than the pitch radius on a ring.
    pub root_radius: f64,
    /// Speed, rpm. Gear 1 is held, so its speed is zero; gears 2 and 3 share
    /// the wobble body's.
    pub speed: f64,
    /// Anything clamped while this gear was built.
    pub clamps: Vec<Note>,
}

/// One mesh of a solved drive.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct HulaMesh {
    /// Operating pressure angle `α_w`, degrees — see
    /// [`crate::train::SpurResult::operating_pressure_angle`], which defines it
    /// for every parallel-axis mesh here.
    ///
    /// A one-tooth-difference pair opened far enough to clear itself runs at an
    /// angle no ordinary pair would — above 50° on the shipped proportions —
    /// which is a property of the arrangement rather than a fault, and is
    /// reported for that reason.
    pub operating_pressure_angle: f64,
    /// Far-side tip gap, mm: the room the wobble body has on the side away from
    /// the mesh, which is what the shift was spent on.
    pub clearance: f64,
    /// The same gap measured on the parts as cut, mm. It differs from the one
    /// above only where a tip was clamped, and then the difference is what the
    /// clamp cost.
    pub clearance_as_cut: f64,
    /// The three contact ratios, as every other mesh here reports them. This
    /// arrangement runs its transverse one just above continuous contact by
    /// construction, so the axial term is what a helical drive of this kind has
    /// to buy its overlap with — and reading only the transverse one hid that.
    pub contact_ratios: super::ContactRatios,
    /// The pinion's tip reaches past where the ring's flank ends.
    pub trochoid_interference: bool,
    /// The ring's tip reaches below where the pinion's flank ends.
    pub involute_interference: bool,
    /// **The tips foul away from the line of action** — the condition that
    /// decides a small tooth difference, where the tip circles cross far from
    /// the line of centres and the mesh itself is perfectly conjugate.
    pub tip_interference: bool,
    /// How much room the tips have where their circles cross, as an angle of
    /// pinion rotation, degrees. Negative is the overlap.
    pub tip_margin: f64,
    /// Angular backlash at each member, degrees — the pinion's first, then the
    /// ring's: nominal at the running offset, then at each end of the tolerance
    /// band. The same gap subtends a different angle at each, so the two differ
    /// whenever the tooth counts do.
    pub backlash: [super::Backlash; 2],
    /// **This pair's own efficiency with the crank held**, 0..1, both
    /// directions — what the teeth lose, and nothing about the arrangement they
    /// sit in.
    ///
    /// It is emphatically **not** the drive's: the two of these multiply to
    /// [`HulaResult::fixed_carrier_efficiency`], and the drive's own is that
    /// figure put through the reduction, which on a high-ratio arrangement
    /// takes a pair losing under a percent to a drive losing tens of them
    /// ([`HulaResult::efficiency`]). Reported per mesh for the same reason a
    /// planetary set reports it per mesh: a stage that loses more than it should
    /// loses it in one of its meshes, and a single product cannot say which.
    pub efficiency: Directional<f64>,
}

/// What a hula stage came to.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct HulaResult {
    /// Input revolutions per output revolution. Negative means the output turns
    /// against the crank.
    pub ratio: f64,
    /// `z₂z₄` and `D = z₂z₄ − z₁z₃`, the two products the ratio is written in.
    /// Reported because `D` is the design rule: its size is the reduction and
    /// its sign is the direction.
    pub ratio_products: [i64; 2],
    /// The zero-backlash crank offset, mm.
    pub offset_nominal: f64,
    /// **The minimum far-side clearance the drive was held to**, zero where the
    /// crank offset was given instead and the input went unread — the same
    /// question every stage answers about its own clearance (see
    /// [`super::SpurStage::clearance_taken`]).
    pub clearance: f64,
    /// The offset actually run at, including the running clearance.
    pub offset: f64,
    /// Which mesh sits at the clearance minimum, when the offset came from it.
    pub binding_mesh: Option<usize>,
    /// Speed of the crank, rpm — the input, and the carrier of both meshes.
    pub crank_speed: f64,
    /// Mesh efficiency with the **crank held**, 0..1, both directions: the two
    /// pairs' own, multiplied.
    ///
    /// **It is not the drive's efficiency**, and on a high-ratio arrangement it
    /// is nowhere near it: the meshes lose under a percent while the drive loses
    /// tens of them, because power circulates. Both figures are reported for
    /// exactly that reason — one is not a stand-in for the other, and reading
    /// the mesh figure as the drive's is the mistake this pair of fields exists
    /// to prevent.
    pub fixed_carrier_efficiency: Directional<f64>,
    /// The drive's own efficiency, 0..1, in both directions.
    ///
    /// It follows from the reduction and the meshes alone:
    ///
    /// ```text
    /// η = 1 / [ R(1 − η₀) + η₀ ]
    /// ```
    ///
    /// — which is [`drive_efficiency`] and is worth reading before choosing
    /// tooth counts, because it says what a design *can* reach before any of it
    /// is drawn. At `R = 49` a mesh pair losing 0.27 % gives 88 %; the same pair
    /// at `R = 324` gives 53 %, and losing 0.85 % instead gives 27 %.
    ///
    /// **Backward is zero where the drive cannot be back-driven**, which on this
    /// arrangement is the ordinary case rather than the exception: an
    /// efficiency below a half forward means a reversed power flow with no
    /// self-consistent solution, and the classical `2 − 1/η` for such a set is
    /// negative there. Reported the way a self-locking worm reports it.
    pub efficiency: Directional<f64>,
    /// Torque on each shaft — the grounded gear, the crank, the output — in
    /// whatever unit the input torque was given. They sum to zero.
    pub shaft_torques: [f64; 3],
    /// Angular backlash at whichever shaft is the **output**, degrees: gear 4
    /// driving forward, the crank driving backward. The same two plays subtend
    /// very different angles at the two, by exactly the reduction.
    pub backlash: Directional<super::Backlash>,
    pub meshes: [HulaMesh; 2],
    pub gears: [HulaGear; 4],
}

/// What a reduction of `ratio` can reach, given meshes that keep `mesh` of what
/// passes through them.
///
/// ```text
/// η = 1 / [ R(1 − η₀) + η₀ ]
/// ```
///
/// The whole power flow collapses to this for the arrangement here — carrier
/// driving, one central member held, the other the output — and it is the most
/// useful thing this module knows, because it answers the design question
/// before anything is drawn: *what would the teeth have to be worth?*
///
/// Read it and the trade is plain. The loss term carries `R`, so a reduction
/// multiplies the mesh loss before it reaches the output: at `R = 324` a mesh
/// pair losing 0.85 % keeps 27 % of the input, and it would have to lose under
/// 0.04 % to keep 90 %. Halve the reduction and the same teeth do far better.
/// This is why a gearbox of this family is built at a few tens to one and not a
/// few hundreds, and why the ones that reach both are a different mechanism.
#[must_use]
pub fn drive_efficiency(ratio: f64, mesh: f64) -> f64 {
    1.0 / (ratio.abs() * (1.0 - mesh) + mesh)
}

/// Solve a hula stage: the arrangement, then the parts, then the meshes.
///
/// # Errors
///
/// [`crate::hula::Error`], through [`super::TrainError::Hula`] — a drive that
/// cannot exist rather than a solve that gave up.
pub fn solve_hula_stage(
    stage: &HulaStage,
    input_speed: f64,
    input_torque: f64,
) -> Result<HulaResult, Error> {
    let teeth = Teeth(stage.gears.each_ref().map(|g| g.teeth));
    // The given shift is the named member's own, so the arrangement is handed
    // the value from where a reader entered it rather than from a second copy.
    let pair_of = |mesh: usize| {
        let (a, b) = (mesh * 2, mesh * 2 + 1);
        if teeth.0[a] > teeth.0[b] {
            (a, b)
        } else {
            (b, a)
        }
    };
    let built = |mesh: usize, shift: [f64; 4], i: usize| GearParams {
        module: stage.module[mesh],
        pressure_angle: stage.pressure_angle,
        helix_angle: stage.helix_angle,
        teeth: teeth.0[i],
        profile_shift: shift[i],
        addendum: stage.gears[i].addendum.manual,
        dedendum: stage.gears[i].dedendum,
        root_radius: stage.gears[i].root_radius,
        thickness_mod: stage.thickness_mod[mesh],
        ..GearParams::default()
    };

    // **Which member carries a mesh's one free number is not an input any
    // more; the `auto` toggles say it.** The crank fixes the *difference* of a
    // pair's two shifts, so exactly one of them is left over: the member a
    // designer gave carries it, and the other follows. With both left automatic
    // the number is nobody's in particular and the pinion carries it — the
    // rack-cut member, the one with an undercut bound to answer to.
    //
    // Both given over-specifies the mesh, since the difference is already the
    // crank's. The ring is taken as the given one and the pinion follows, which
    // is what the panel's relief is written against: it turns the other member
    // back to automatic as a shift is pinned, so this arm is the transient
    // rather than the design.
    let carrier = |mesh: usize| {
        let (ring, pinion) = pair_of(mesh);
        if stage.gears[ring].profile_shift.auto {
            pinion
        } else {
            ring
        }
    };
    let split_of = |mesh: usize, value: f64| {
        let (ring, _) = pair_of(mesh);
        if carrier(mesh) == ring {
            Split::Ring(value)
        } else {
            Split::Pinion(value)
        }
    };
    // What the carrier settles at when nothing searches for it, and what it may
    // not be moved off when a designer gave it — the same two answers every
    // stage reads out of the shift controls.
    let asked = |mesh: usize| {
        let i = carrier(mesh);
        stage.gears[i].shift_asked(&built(mesh, [0.0; 4], i))
    };
    let given: [f64; 2] = std::array::from_fn(|mesh| asked(mesh).settled);
    let pinned: [Option<f64>; 2] = std::array::from_fn(|mesh| asked(mesh).given);
    // A carrier whose given shift had to be raised says so, on the gear whose
    // shift it is — the channel this stage already reports a clamped part on.
    let raised: [Option<crate::note::Note>; 2] =
        std::array::from_fn(|mesh| asked(mesh).note(teeth.0[carrier(mesh)]));
    let asked_offset = if stage.offset.auto {
        Offset::Clearance
    } else {
        Offset::Given(stage.offset.manual)
    };
    let set_with = |value: [f64; 2], offset: Offset| hula::Set {
        teeth,
        module: stage.module,
        pressure_angle: stage.pressure_angle,
        helix_angle: stage.helix_angle,
        addendum: stage.gears.each_ref().map(|g| g.addendum.manual),
        clearance: stage.clearance,
        offset,
        split: std::array::from_fn(|mesh| split_of(mesh, value[mesh])),
    };
    let set_at = |value: [f64; 2]| set_with(value, asked_offset);
    // **The offset has to clear the tips as well as the far side**, and that
    // bound belongs to the pair rather than to the arrangement — so it is
    // supplied to the solve from the parts a trial offset would produce, not
    // rewritten inside it.
    let pairs = [teeth.pair(0)?, teeth.pair(1)?];
    let tip_room = |mesh: usize, shift: [f64; 4]| {
        let pair = pairs[mesh];
        let ring = Ring::cut_by(&built(mesh, shift, pair.ring), &stage.cutter[mesh]);
        let pinion = Tooth::new(built(mesh, shift, pair.pinion));
        mesh_with(&ring, &pinion).map_or(f64::NAN, |m| m.tip_margin)
    };
    // **The split is the free variable, and the crank is what ties the two
    // meshes together.** A pair's shift sum is fixed by the offset it has to
    // reach, so within a mesh only the division is left — but the two meshes
    // share one crank, so the two divisions are searched as a pair rather than
    // one at a time.
    //
    // The objective is the product of the two mesh efficiencies, which is what
    // the drive's own efficiency rises with, so the power flow does not have to
    // be run inside the search.
    let split_at = if stage.optimisation.enabled {
        // **The crank is solved once a round, and each mesh is chosen alone.**
        //
        // Two facts make this cheap, and both are the mechanism's rather than
        // the search's. The first: where the offset comes from the clearances,
        // finding it is a bracketed root-find whose every step builds both
        // pairs, and the split moves it only through the tip geometry — a weak
        // coupling. So the offset is solved at the split in hand, the splits are
        // chosen at *that* offset where the solve is closed and costs nothing,
        // and the offset is solved again at what they chose.
        //
        // The second: at a held offset the two meshes do not touch. Each pair's
        // shift sum is the crank's, and its division changes nothing outside its
        // own mesh — so the product of the two efficiencies is largest when each
        // factor is, and what looked like one search in two variables is two
        // searches in one. That is not an approximation of the joint search; it
        // is the same answer for a thirteenth of the work.
        //
        // Nested and joint, this cost eight tenths of a second for a panel that
        // re-solves on every keystroke.
        const ROUNDS: usize = 3;
        /// The split has moved by less than a tooth is cut to; another round
        /// would return the same answer.
        const SETTLED: f64 = 1e-3;

        // One mesh's efficiency at a held offset, or `None` where that mesh
        // cannot be built there.
        let eta_one = |value: [f64; 2], offset: Offset, index: usize| -> Option<f64> {
            let layout = hula::solve_with(&set_with(value, offset), &tip_room).ok()?;
            let pair = pairs[index];
            let params = |i: usize| built(index, layout.shift, i);
            let ring = Ring::cut_by(&params(pair.ring), &stage.cutter[index]);
            let pinion_params = params(pair.pinion);
            let pinion = Tooth::new(pinion_params);
            // The pinion is rack-generated and so is asked the four questions
            // every chosen shift is asked; the ring is shaper-cut and is not,
            // and the pair's own bound is the tip margin below. Whether the
            // undercut two of the four are asked at all is the pinion's own
            // `no undercut`, exactly as it is on a pair.
            let pinion_floor = stage.gears[pair.pinion].no_undercut.then(|| {
                crate::auto::automatic_profile_shift(&pinion_params, pinion_params.dedendum)
            });
            if !crate::auto::member_is_buildable(&pinion, pinion_floor) {
                return None;
            }
            // A split that fouls the tips at this offset is not admissible at
            // it — read off the pair already in hand rather than through
            // `tip_room`, which would cut the ring a second time, and cutting a
            // ring is the dearest thing here. The round that follows may open
            // the crank far enough to take it, and then it is offered again on
            // its merits.
            if mesh_with(&ring, &pinion).is_none_or(|m| m.tip_margin < 0.0) {
                return None;
            }
            let mesh =
                Mesh::new(&pinion, &Tooth::new(params(pair.ring)), MeshKind::Internal).ok()?;
            let path = ContactPath::new(&pinion, ring.ra, &mesh)?;
            if path.contact_ratio < stage.optimisation.min_contact_ratio {
                return None;
            }
            Some(efficiency(
                &path,
                &mesh,
                &pinion,
                stage.sliding_friction[index],
                Drive::Forward,
            ))
        };

        let mut at = given;
        for _ in 0..ROUNDS {
            let Ok(layout) = hula::solve_with(&set_at(at), &tip_room) else {
                break;
            };
            let held = Offset::Given(layout.offset);
            let mut next = at;
            for index in 0..2 {
                // A split given by hand is a constraint, and that mesh has
                // nothing left to search.
                if pinned[index].is_some() {
                    continue;
                }
                if let Some(free) = crate::auto::maximise(1, &|free| {
                    let mut trial = next;
                    trial[index] = free[0];
                    eta_one(trial, held, index)
                }) {
                    next[index] = free[0];
                }
            }
            let settled = next.iter().zip(&at).all(|(a, b)| (a - b).abs() < SETTLED);
            at = next;
            if settled {
                break;
            }
        }
        at
    } else {
        given
    };
    let set = set_at(split_at);
    let layout = hula::solve_with(&set, &tip_room)?;
    let offset = layout.offset + stage.running_clearance;

    // Kinematics. The crank is the carrier of both meshes; the wobble body
    // follows from the first mesh with gear 1 held, and the output from the
    // ratio the two products give — the same expression, not a second one.
    let z = teeth.0.map(f64::from);
    let wobble_speed = input_speed * (1.0 - z[0] / z[1]);
    let output_speed = input_speed / layout.ratio.value();

    let mut meshes = Vec::with_capacity(2);
    // Kept past the loop: the pair's rolling geometry is what refers a play to
    // a shaft, and that cannot be done one mesh at a time.
    let mut rolling = Vec::with_capacity(2);
    // Each pair's own efficiency, with the crank held: sliding, then static.
    let mut basic: Vec<Directional<(f64, f64)>> = Vec::with_capacity(2);
    // Filled as each pair is built, since a gear belongs to exactly one mesh
    // and is finished the moment its own pair is.
    let mut gears: [Option<HulaGear>; 4] = [None, None, None, None];
    let speed = |i: usize| match i {
        0 => 0.0,
        1 | 2 => wobble_speed,
        _ => output_speed,
    };

    for (index, pair) in pairs.iter().enumerate() {
        let params = |i: usize| built(index, layout.shift, i);
        // The ring twice over, and neither reading is redundant. `Ring` is the
        // part as its shaper leaves it — the tip it really has, the fillet, the
        // clamps — while `Mesh` is the pair's rolling geometry, which is where
        // backlash lives for every stage here. Building the second from the
        // first's parameters is what keeps one relation in one place.
        let ring = Ring::cut_by(&params(pair.ring), &stage.cutter[index]);
        let pinion = Tooth::new(params(pair.pinion));
        let mesh = Mesh::new(&pinion, &Tooth::new(params(pair.ring)), MeshKind::Internal)
            .map_err(Error::Mesh)?;

        // The raised-shift note belongs to whichever member carries this mesh's
        // free number, and rides that gear's clamps — the channel this stage
        // already reports a part that did not come out as asked on.
        let with_raised = |mut notes: Vec<crate::note::Note>, gear: usize| {
            if gear == carrier(index) {
                if let Some(n) = raised[index].clone() {
                    notes.push(n);
                }
            }
            notes
        };

        gears[pair.ring] = Some(HulaGear {
            teeth: teeth.0[pair.ring],
            ring: true,
            profile_shift: layout.shift[pair.ring],
            pitch_radius: ring.r,
            base_radius: ring.rb,
            tip_radius: ring.ra,
            root_radius: ring.rf,
            speed: speed(pair.ring),
            clamps: with_raised(ring.clamps.clone(), pair.ring),
        });
        gears[pair.pinion] = Some(HulaGear {
            teeth: teeth.0[pair.pinion],
            ring: false,
            profile_shift: layout.shift[pair.pinion],
            pitch_radius: pinion.r,
            base_radius: pinion.rb,
            tip_radius: pinion.ra,
            root_radius: pinion.rf,
            speed: speed(pair.pinion),
            clamps: with_raised(pinion.clamps.notes.clone(), pair.pinion),
        });

        // The pair's own loss, with the crank held. The path of contact is the
        // pinion's, and the ring enters through its tip radius and the mesh's
        // signed tooth sum rather than through a case of its own — including
        // when it lies wholly on one side of the pitch point, which at one tooth
        // of difference it does.
        let path = ContactPath::new(&pinion, ring.ra, &mesh);
        let pair_efficiency = Directional::of(|d| {
            path.as_ref().map_or((0.0, 0.0), |p| {
                (
                    efficiency(p, &mesh, &pinion, stage.sliding_friction[index], d),
                    efficiency(p, &mesh, &pinion, stage.static_friction[index], d),
                )
            })
        });
        basic.push(pair_efficiency);

        rolling.push(mesh);
        let report = mesh_with(&ring, &pinion);
        let angular =
            |a: f64, at: MeshSide| mesh.angular_backlash(a, at).unwrap_or(0.0).to_degrees();
        let backlash_of = |at: MeshSide| super::Backlash {
            nominal: angular(offset, at),
            minimum: angular(offset - stage.tolerance_minus, at),
            maximum: angular(offset + stage.tolerance_plus, at),
        };

        meshes.push(HulaMesh {
            operating_pressure_angle: layout.alpha_w[index].to_degrees(),
            clearance: layout.clearance[index],
            clearance_as_cut: ring.ra - pinion.ra + offset,
            contact_ratios: super::ContactRatios::of(
                report.as_ref().map_or(0.0, |m| m.contact_ratio),
                // The narrower member carries the mesh, as it does everywhere
                // else here.
                stage.gears[pair.ring]
                    .face_width
                    .manual
                    .min(stage.gears[pair.pinion].face_width.manual),
                stage.helix_angle,
                stage.module[index],
            ),
            trochoid_interference: report.as_ref().is_some_and(|m| m.trochoid_interference),
            involute_interference: report.as_ref().is_some_and(|m| m.involute_interference),
            tip_interference: report.as_ref().is_some_and(|m| m.tip_interference),
            tip_margin: report.as_ref().map_or(0.0, |m| m.tip_margin.to_degrees()),
            backlash: [backlash_of(MeshSide::First), backlash_of(MeshSide::Second)],
            // The sliding figure of the pair the fixed-carrier product is
            // taken from, rather than a second run of the same integral: one
            // number, read twice, so a mesh row and the drive's own efficiency
            // cannot disagree about what this pair loses.
            efficiency: Directional::of(|d| pair_efficiency.get(d).0),
        });
    }

    let gears = gears.map(|g| g.expect("every gear belongs to a mesh"));

    // ---- backlash, referred to whichever shaft is the output.
    //
    // Both meshes sit at the same centre distance — the crank's — so in the
    // crank's frame each mesh's play lets its members slip. Writing each pair
    // with its ring first, and `θ_w` for the body carrying gears 2 and 3:
    //
    //     r′₁(θ₁ − θ_c) − r′₂(θ_w − θ_c) = ±δ_A        gears 1 and 2
    //     r′₃(θ_w − θ_c) − r′₄(θ₄ − θ_c) = ±δ_B        gears 3 and 4
    //
    // `r′₂ ≠ r′₃` in general — the two are different gears on one body, with
    // different tooth counts, modules and operating pressure angles, and
    // assuming one radius for the body is the mistake waiting to be made here.
    // Eliminating `θ_w` leaves one constraint on the three shafts:
    //
    //     r′₁r′₃(θ₁ − θ_c) − r′₂r′₄(θ₄ − θ_c) = Δ,     Δ = r′₃δ_A + r′₂δ_B
    //
    // which is Willis at Δ = 0. Hold the two shafts that are not the output and
    // the third moves by `|Δ|/Z`, with `Z` its own coefficient: `r′₂r′₄` at the
    // output, `r′₂r′₄ − r′₁r′₃` at the crank. The two plays are independent, so
    // their extremes add.
    //
    // The check that this is right: the same play measured at the two shafts
    // must differ by exactly the reduction between them — and the coefficients
    // here are built from operating radii while the reduction is counted in
    // teeth, so the two routes share nothing.
    let operating_radius = |gear: usize, a: f64| {
        let mesh = gear / 2;
        a * z[gear] / (z[mesh * 2] - z[mesh * 2 + 1]).abs()
    };
    let referred = |a: f64| -> Directional<f64> {
        let play: Vec<f64> = rolling
            .iter()
            .map(|m| m.backlash(a).unwrap_or(0.0).abs())
            .collect();
        let r: Vec<f64> = (0..4).map(|g| operating_radius(g, a)).collect();
        let delta = r[2] * play[0] + r[1] * play[1];
        Directional {
            forward: (delta / (r[1] * r[3])).to_degrees(),
            backward: (delta / (r[1] * r[3] - r[0] * r[2]).abs()).to_degrees(),
        }
    };
    let band = |pick: fn(&Directional<f64>) -> f64| super::Backlash {
        nominal: pick(&referred(offset)),
        minimum: pick(&referred(offset - stage.tolerance_minus)),
        maximum: pick(&referred(offset + stage.tolerance_plus)),
    };
    let backlash = Directional {
        forward: band(|d| d.forward),
        backward: band(|d| d.backward),
    };

    // ---- efficiency.
    //
    // The drive is a three-shaft epicyclic: gears 1 and 4 are the two central
    // members on the fixed axis, the crank is the carrier, and the wobble body
    // is the planet. Its basic ratio is the same two products the reduction is
    // written in, so the power flow is `planetary::power` — a solve about three
    // shafts, a basic ratio and a fixed-carrier efficiency, which asks nothing
    // else of the set it is given.
    //
    // The role names are that solve's: `Sun` and `Ring` are the two central
    // members, not two tooth forms. Here both may be rings, or both external,
    // and the solve is indifferent — which is the point of handing it a ratio.
    const GROUNDED: PlanetaryShaft = PlanetaryShaft::Sun;
    const OUTPUT: PlanetaryShaft = PlanetaryShaft::Ring;
    const CRANK: PlanetaryShaft = PlanetaryShaft::Carrier;
    let products = (layout.ratio.numerator, layout.ratio.denominator);
    let basic_ratio = products.0 as f64 / (products.0 - products.1) as f64;
    let product = |pick: fn(&(f64, f64)) -> f64| {
        Directional::of(|d| basic.iter().map(|m| pick(m.get(d))).product())
    };
    let fixed_carrier_efficiency = product(|e| e.0);
    // The same product on the static coefficients. It decides a sign rather
    // than a figure — whether the drive breaks away at all — and is kept beside
    // the sliding one so no stage kind is the exception.
    let at_rest_meshes = product(|e| e.1);
    let flow = |input: PlanetaryShaft, speed: f64, torque: f64, eta0: f64| {
        planetary::power(
            basic_ratio,
            Arrangement {
                input,
                fixed: GROUNDED,
            },
            speed,
            torque,
            eta0,
        )
    };
    // Driving backward the output shaft becomes the input, at **the speed and
    // torque the forward solve gave it**; the same shaft stays held. Where no
    // branch has the output absorbing there is no back-driven state at all —
    // the drive is self-locking, and that is a refusal rather than a low number.
    let reversed = |eta0: f64, forward: &planetary::Power| {
        let out = OUTPUT.index_pub();
        let speed = forward.speeds[out];
        flow(
            OUTPUT,
            speed,
            forward.torques[out].abs() * if speed < 0.0 { -1.0 } else { 1.0 },
            eta0,
        )
    };
    let forward = flow(
        CRANK,
        input_speed,
        input_torque,
        fixed_carrier_efficiency.forward,
    );
    let drive_efficiency = Directional {
        forward: forward.as_ref().map_or(0.0, |p| p.efficiency),
        backward: forward
            .as_ref()
            .and_then(|f| reversed(fixed_carrier_efficiency.backward, f))
            .map_or(0.0, |p| p.efficiency),
    }
    // **Whether it turns at all is decided at rest**, against the static
    // coefficients: the same two solves on the higher friction, for the sign
    // only.
    .once_moving(&Directional {
        forward: flow(CRANK, input_speed, input_torque, at_rest_meshes.forward)
            .map_or(0.0, |p| p.efficiency),
        backward: forward
            .as_ref()
            .and_then(|f| reversed(at_rest_meshes.backward, f))
            .map_or(0.0, |p| p.efficiency),
    });

    Ok(HulaResult {
        fixed_carrier_efficiency,
        efficiency: drive_efficiency,
        shaft_torques: forward.as_ref().map_or([0.0; 3], |p| p.torques),
        ratio: layout.ratio.value(),
        ratio_products: [layout.ratio.numerator, layout.ratio.denominator],
        offset_nominal: layout.offset,
        clearance: stage.clearance_taken(),
        offset,
        binding_mesh: layout.binding,
        crank_speed: input_speed,
        backlash,
        meshes: [meshes[0].clone(), meshes[1].clone()],
        gears,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The arrangement these tests were written against: `N ± 1` about 18,
    /// whose ratio, speeds and losses the assertions below name outright.
    ///
    /// Pinned here rather than taken from [`HulaStage::default`], so that what
    /// the tool ships with is free to change without changing what a test
    /// claims — a test that asserts 324:1 has to be the one saying which counts
    /// give 324:1.
    fn stage() -> HulaStage {
        let mut s = HulaStage::default();
        for (gear, count) in s.gears.iter_mut().zip([19, 18, 17, 18]) {
            gear.teeth = count;
            // The taller tooth this difference needs: at one tooth of
            // difference the operating pressure angle is far higher and the
            // path far shorter, so the 0.7 that suits the shipped arrangement
            // leaves this one below continuous contact.
            gear.addendum = crate::params::Auto::fixed(0.8);
        }
        // **The tool follows the rings this fixture builds**, which are a third
        // the size of the shipped ones. The stocked default would be larger
        // than the 19-tooth ring here and would cut it no fillet at all — the
        // fault `a_shaper_larger_than_its_ring_is_reported` exists to catch,
        // which every test on this fixture would then be quietly running into.
        for cutter in &mut s.cutter {
            cutter.teeth = 12;
        }
        s
    }

    /// The stage reports the arrangement's ratio, products and all — it does not
    /// recompute one.
    #[test]
    fn the_stage_reports_the_arrangements_ratio() {
        let r = solve_hula_stage(&stage(), 100.0, 2.0).unwrap();
        assert_eq!(r.ratio_products, [324, 1]);
        assert!((r.ratio - 324.0).abs() < 1e-12);
    }

    /// **The parts are built at the offset the drive solved**, and the running
    /// clearance is the whole of the difference.
    ///
    /// The gap as cut is read off the tips the shaper actually left, while the
    /// solved one comes from the ideal tips; they part company exactly when a
    /// tip was clamped, so this gates the clamp and the offset at once.
    #[test]
    fn the_gap_as_cut_is_the_solved_gap_plus_the_running_clearance() {
        let s = stage();
        let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
        for m in &r.meshes {
            assert!(
                (m.clearance_as_cut - m.clearance - s.running_clearance).abs() < 1e-9,
                "as cut {} against {} + {}",
                m.clearance_as_cut,
                m.clearance,
                s.running_clearance
            );
        }
        assert!((r.offset - r.offset_nominal - s.running_clearance).abs() < 1e-12);
    }

    /// Which members are rings is read off the counts, not declared.
    #[test]
    fn the_rings_are_the_members_with_more_teeth() {
        let r = solve_hula_stage(&stage(), 100.0, 2.0).unwrap();
        assert_eq!(
            r.gears.each_ref().map(|g| g.ring),
            [true, false, false, true]
        );
    }

    /// Gear 1 is held, gears 2 and 3 are one body, and the output turns at the
    /// input over the ratio. Nothing here is a second kinematic model.
    #[test]
    fn the_speeds_are_the_arrangements() {
        let r = solve_hula_stage(&stage(), 3240.0, 2.0).unwrap();
        assert_eq!(r.gears[0].speed, 0.0);
        assert!((r.gears[1].speed - r.gears[2].speed).abs() < 1e-12);
        assert!((r.gears[3].speed - 3240.0 / r.ratio).abs() < 1e-9);
        // The wobble body turns once backwards per z2 crank turns.
        assert!((r.gears[1].speed + 3240.0 / 18.0).abs() < 1e-9);
    }

    /// Backlash comes from the pair's own rolling geometry, so it rises with the
    /// running clearance and the tolerance band brackets it.
    #[test]
    fn backlash_rises_with_the_running_clearance() {
        let mut last = 0.0;
        for step in 0..8 {
            let s = HulaStage {
                running_clearance: f64::from(step) * 0.01,
                ..stage()
            };
            let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
            let j = r.meshes[0].backlash[0].nominal;
            assert!(j >= last, "backlash fell from {last} to {j}");
            assert!(
                r.meshes[0].backlash[0].minimum <= j && j <= r.meshes[0].backlash[0].maximum,
                "the tolerance band must bracket the nominal"
            );
            last = j;
        }
        assert!(last > 0.0, "no backlash at any clearance");
    }

    /// **The stage ships buildable**, tool included.
    ///
    /// A default is the one configuration every reader sees first and no test
    /// otherwise asserts, so the shaper and the rings can drift apart in it
    /// silently — as they did the moment the tool was sized for the rings this
    /// module's *fixtures* build rather than for the ones it ships.
    #[test]
    fn the_shipped_drive_is_one_its_own_tools_can_cut() {
        let r =
            solve_hula_stage(&HulaStage::default(), 1000.0, 2.0).expect("the shipped drive solves");
        for gear in &r.gears {
            assert!(
                gear.clamps.is_empty(),
                "z{} came out clamped: {:?}",
                gear.teeth,
                gear.clamps
            );
        }
    }

    /// **A shaper larger than its ring cuts nothing**, and the part says so.
    ///
    /// The tool is clamped down to the ring's own tooth count and then reaches
    /// none of its flank, so no fillet is generated. It is an ordinary mistake
    /// on a drive whose rings are this small, which is why the shipped cutters
    /// are well below the shipped rings.
    #[test]
    fn a_shaper_larger_than_its_ring_is_reported() {
        let s = HulaStage {
            cutter: [Cutter {
                teeth: 40,
                ..Cutter::default()
            }; 2],
            ..stage()
        };
        let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
        let rings: Vec<&HulaGear> = r.gears.iter().filter(|g| g.ring).collect();
        for ring in rings {
            assert!(
                !ring.clamps.is_empty(),
                "a ring cut by a tool bigger than itself should have said so"
            );
        }
    }

    /// **The same play, measured at the two shafts, differs by the reduction.**
    ///
    /// The two figures come from coefficients built out of operating radii;
    /// the reduction is counted in teeth. That they agree says the operating
    /// radii scale as the tooth counts within each mesh — which is the step
    /// that would go wrong if a reference radius were used for one member and
    /// an operating one for another.
    #[test]
    fn the_play_at_the_two_shafts_differs_by_the_reduction() {
        for teeth in [[19_u32, 18, 17, 18], [17, 18, 19, 18], [20, 18, 17, 18]] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(teeth) {
                gear.teeth = count;
            }
            let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
            let want = r.ratio.abs();
            let got = r.backlash.backward.nominal / r.backlash.forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: play differs by {got} where the reduction is {want}"
            );
        }
    }

    /// **The output's play, reached the other way round.**
    ///
    /// The stage refers the two plays with coefficients built from operating
    /// radii and linear backlash. This rebuilds the same figure from the
    /// per-member angular backlash each mesh already reports — mesh A's play at
    /// the wobble body, carried through mesh B by `z₃/z₄`, plus mesh B's own at
    /// the output. Different function, different quantities, same answer, which
    /// is what says the two plays are combined the right way round rather than
    /// merely added.
    #[test]
    fn the_outputs_play_is_the_two_meshes_referred_through_the_body() {
        for teeth in [[19_u32, 18, 17, 18], [17, 18, 19, 18], [20, 18, 17, 18]] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(teeth) {
                gear.teeth = count;
            }
            let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
            // `backlash[0]` is the pinion's, `[1]` the ring's — the mesh was
            // built with the pinion first.
            let at = |mesh: usize, gear: usize| {
                r.meshes[mesh].backlash[usize::from(r.gears[gear].ring)].nominal
            };
            let want = at(0, 1) * f64::from(teeth[2]) / f64::from(teeth[3]) + at(1, 3);
            let got = r.backlash.forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: {got} referred, {want} from the members"
            );
        }
    }

    /// The output's play is the two meshes' plays referred through the drive,
    /// so it rises with either of them and vanishes with both.
    #[test]
    fn the_outputs_play_is_the_two_meshes_referred() {
        let tight = HulaStage {
            running_clearance: 0.0,
            tolerance_plus: 0.0,
            tolerance_minus: 0.0,
            ..stage()
        };
        let r = solve_hula_stage(&tight, 100.0, 2.0).unwrap();
        assert!(
            r.backlash.forward.nominal.abs() < 1e-12,
            "a drive with no clearance should have no play, not {}",
            r.backlash.forward.nominal
        );

        let mut last = 0.0;
        for step in 1..8 {
            let s = HulaStage {
                running_clearance: f64::from(step) * 0.01,
                ..stage()
            };
            let j = solve_hula_stage(&s, 100.0, 2.0)
                .unwrap()
                .backlash
                .forward
                .nominal;
            assert!(j > last, "the output's play fell from {last} to {j}");
            last = j;
        }
    }

    /// **The offset clears the tips as well as the far side.**
    ///
    /// The two are different bounds and either can be the one that binds. At
    /// the shipped gap the far side asks for more, so the gap is what a reader
    /// gets and the tips have room to spare. Ask for a gap the tips cannot
    /// live with and the offset opens past it: the tips come to rest exactly at
    /// their limit, and the far-side gap that results is *larger* than the one
    /// asked for, which is the tool answering with what can be built rather
    /// than with what was requested.
    ///
    /// Both figures are what rolling the outlines through a tooth measures
    /// (`gear-cli hulasweep`): a fifth of a module fouled by 3 µm before the
    /// offset was allowed to answer to it.
    #[test]
    fn the_offset_clears_the_tips_as_well_as_the_far_side() {
        let s = stage();
        let shipped = solve_hula_stage(&s, 100.0, 2.0).unwrap();
        for mesh in &shipped.meshes {
            assert!(!mesh.tip_interference, "margin {}", mesh.tip_margin);
            assert!(
                mesh.tip_margin > 0.1,
                "the far side should be what binds here, not the tips: {}",
                mesh.tip_margin
            );
        }
        let held = shipped.binding_mesh.expect("the gap held it open");
        assert!(
            (shipped.meshes[held].clearance - s.clearance).abs() < 1e-9,
            "the binding mesh should sit at the gap asked for"
        );

        // Now a gap the tips cannot live with.
        let tight = HulaStage {
            clearance: 0.22,
            ..stage()
        };
        let opened = solve_hula_stage(&tight, 100.0, 2.0).unwrap();
        assert!(
            opened.offset_nominal > shipped.offset_nominal - 1e-9
                || opened.meshes.iter().all(|m| !m.tip_interference),
            "the tips must be clear whatever was asked for"
        );
        for mesh in &opened.meshes {
            assert!(
                !mesh.tip_interference,
                "the offset should have opened until the tips cleared: {}",
                mesh.tip_margin
            );
            assert!(
                mesh.clearance >= tight.clearance - 1e-9,
                "opening for the tips gives the far side more, never less: {} against {}",
                mesh.clearance,
                tight.clearance
            );
        }
        let binding = opened.binding_mesh.expect("something held it open");
        assert!(
            opened.meshes[binding].tip_margin.abs() < 1e-6,
            "the tips are what held it, so they sit at their limit: {}",
            opened.meshes[binding].tip_margin
        );
    }

    /// **The meshes lose a little and the drive loses a lot**, and the second
    /// does not follow from the first by reading it twice.
    ///
    /// Power circulates: at 324:1 the two pairs lose 0.85 % between them while
    /// the drive loses nearly three quarters of what it is given. That gap is
    /// the whole reason both figures are reported, and reading the mesh figure
    /// as the drive's is the mistake the pair of them exists to prevent.
    #[test]
    fn the_meshes_lose_a_little_and_the_drive_loses_a_lot() {
        let r = solve_hula_stage(&stage(), 100.0, 2.0).unwrap();
        let meshes = r.fixed_carrier_efficiency;
        assert!(
            meshes.forward > 0.95 && meshes.forward < 1.0,
            "two parallel-axis meshes lose a little, not nothing and not much: {}",
            meshes.forward
        );
        assert!(
            (meshes.forward - meshes.backward).abs() < 1e-12,
            "a parallel-axis mesh loses the same either way"
        );
        assert!(
            r.efficiency.forward > 0.0 && r.efficiency.forward < 0.5,
            "the drive loses far more than its meshes: {}",
            r.efficiency.forward
        );
        assert!(
            1.0 - r.efficiency.forward > 20.0 * (1.0 - meshes.forward),
            "the circulating power is the point: drive {} against meshes {}",
            r.efficiency.forward,
            meshes.forward
        );
    }

    /// **A higher reduction costs efficiency**, monotonically, because the
    /// nearer the two meshes come to cancelling the more power goes round
    /// between them before any of it reaches the output.
    #[test]
    fn a_higher_reduction_costs_efficiency() {
        let mut last = 1.0;
        for n in [12_u32, 18, 30, 50] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                gear.teeth = count;
            }
            let r = solve_hula_stage(&s, 100.0, 2.0).unwrap();
            assert!(
                r.efficiency.forward < last,
                "z {n}: {} did not fall below {last}",
                r.efficiency.forward
            );
            // ...and none of them turns backwards.
            assert_eq!(
                r.efficiency.backward, 0.0,
                "z {n} should be self-locking at {}",
                r.efficiency.forward
            );
            last = r.efficiency.forward;
        }
        assert!(last < 0.2, "2500:1 should be dear: {last}");
    }

    /// **A path that never reaches the pitch point is still a path.**
    ///
    /// These pairs run at an operating pressure angle high enough to put the
    /// pitch point outside both tip circles, so contact happens entirely on one
    /// side of it and sliding never reverses along the path. The loss integral
    /// is written to carry that — the familiar mesh is the case where the two
    /// ends have opposite signs — and a mesh efficiency comes out rather than
    /// nothing.
    #[test]
    fn a_mesh_whose_contact_never_reaches_the_pitch_point_still_has_a_loss() {
        let r = solve_hula_stage(&stage(), 100.0, 2.0).unwrap();
        assert!(
            r.fixed_carrier_efficiency.forward < 1.0,
            "a loss of exactly nothing means the path was refused, not computed"
        );
        for mesh in &r.meshes {
            assert!(
                mesh.contact_ratios.transverse > 1.0,
                "and the pair carries its load continuously: {}",
                mesh.contact_ratios.transverse
            );
        }
    }

    /// **A reduction that does not come from cancellation is efficient**, and
    /// the same code says so.
    ///
    /// This is the check that the low figure above is the mechanism rather than
    /// the model. Both families here have the *same* two meshes losing the same
    /// 0.85 % between them; they differ only in whether the wobble body carries
    /// two faces of the same kind. Where it does, the two meshes nearly cancel,
    /// `D = ±1`, the ratio is `z²` and the drive keeps a quarter of what it is
    /// given. Where it does not, `D ≈ 2z`, the ratio is about `z/2` and the
    /// drive keeps ninety-odd percent — an ordinary gearbox.
    ///
    /// It is the published behaviour of a Wolfrom set: efficiency falls as the
    /// reduction rises, because the reduction *is* the cancellation and
    /// cancellation is what circulates the power. A three-ring reducer reaches
    /// a high ratio at high efficiency by not doing this — its rings translate
    /// on a parallelogram of cranks instead of rotating, so its reduction comes
    /// from one mesh's tooth difference with nothing to cancel against.
    #[test]
    fn a_reduction_that_does_not_come_from_cancellation_is_efficient() {
        let solve = |z: [u32; 4]| {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(z) {
                gear.teeth = count;
            }
            solve_hula_stage(&s, 1000.0, 2.0).unwrap()
        };
        // Both faces of the wobble body the same kind: the meshes cancel.
        for z in [[19, 18, 17, 18], [17, 18, 19, 18]] {
            let r = solve(z);
            assert!(r.ratio.abs() > 300.0, "{z:?} reduces by {}", r.ratio);
            assert!(
                r.efficiency.forward < 0.35,
                "{z:?}: cancellation costs, {} is too good",
                r.efficiency.forward
            );
        }
        // One of each: nothing cancels, and the same meshes keep their loss.
        for z in [[19, 18, 18, 17], [17, 18, 18, 19], [18, 17, 19, 18]] {
            let r = solve(z);
            assert!(r.ratio.abs() < 12.0, "{z:?} reduces by {}", r.ratio);
            assert!(
                r.efficiency.forward > 0.9,
                "{z:?}: nothing circulates here, {} is too poor",
                r.efficiency.forward
            );
        }
        // ...and it is the same meshes throughout, so the difference is the
        // arrangement and not the teeth.
        let cancelling = solve([19, 18, 17, 18]).fixed_carrier_efficiency.forward;
        let plain = solve([19, 18, 18, 17]).fixed_carrier_efficiency.forward;
        assert!(
            (cancelling - plain).abs() < 0.005,
            "the meshes should lose alike: {cancelling} against {plain}"
        );
    }

    /// **The power flow collapses to one relation**, and the solve agrees with
    /// it everywhere.
    ///
    /// `η = 1/[R(1 − η₀) + η₀]` is written from the torque shares by hand; the
    /// solve reaches the same number through Willis, the two candidate signs of
    /// the rolling power and an energy condition. Agreeing across three
    /// reductions and four friction coefficients says the closed form is the
    /// same statement, which is what makes it safe to design against.
    #[test]
    fn the_drive_efficiency_is_the_reduction_and_the_meshes() {
        for n in [7_u32, 12, 18] {
            for mu in [0.08, 0.04, 0.02, 0.01] {
                let mut s = HulaStage {
                    sliding_friction: [mu; 2],
                    static_friction: [mu * 2.0; 2],
                    ..stage()
                };
                for (gear, count) in s.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                    gear.teeth = count;
                }
                let r = solve_hula_stage(&s, 1000.0, 2.0).unwrap();
                let want = drive_efficiency(r.ratio, r.fixed_carrier_efficiency.forward);
                assert!(
                    (r.efficiency.forward - want).abs() < 1e-9,
                    "z {n} mu {mu}: solve {} against the relation {want}",
                    r.efficiency.forward
                );
            }
        }
    }

    /// **Checked against a gearbox somebody built.**
    ///
    /// The bilateral drive gear is a 3K of this family, optimised for
    /// efficiency by choice of profile shift and tooth count, and it reports
    /// 89.0 % forward — against 68.5 % for the same gearbox with uncorrected
    /// teeth. Reading those through the relation gives mesh efficiencies of
    /// 99.73 % and 99.04 % at a reduction near fifty, which is an ordinary pair
    /// and a good one; and this stage at that reduction and that mesh figure
    /// comes out at 88.5 %.
    ///
    /// It is not a reproduction of their gearbox — theirs has a carrier and
    /// planets and its shifts are freer than a shared crank offset allows — but
    /// it is the same arithmetic reaching the same place from tooth counts this
    /// module chose independently, which is the most that can be asked of a
    /// model against a published number.
    #[test]
    fn the_relation_agrees_with_a_gearbox_somebody_built() {
        // What the published pair of figures implies about the meshes.
        let implied = |eta: f64, ratio: f64| (1.0 / eta - 1.0) / (ratio - 1.0);
        let optimised = 1.0 - implied(0.890, 49.0);
        let uncorrected = 1.0 - implied(0.685, 49.0);
        assert!(
            (optimised - 0.9973).abs() < 5e-4,
            "89.0 % at 49:1 wants meshes at {optimised}"
        );
        assert!(
            (uncorrected - 0.9904).abs() < 5e-4,
            "68.5 % at 49:1 wants meshes at {uncorrected}"
        );
        // ...and this stage, at that reduction and that mesh efficiency.
        assert!(
            (drive_efficiency(49.0, optimised) - 0.890).abs() < 1e-3,
            "the relation should return the figure it was read from"
        );
        let mut s = HulaStage {
            sliding_friction: [0.010; 2],
            static_friction: [0.020; 2],
            ..stage()
        };
        for (gear, count) in s.gears.iter_mut().zip([8_u32, 7, 6, 7]) {
            gear.teeth = count;
        }
        let r = solve_hula_stage(&s, 1000.0, 2.0).unwrap();
        assert!((r.ratio - 49.0).abs() < 1e-9, "ratio {}", r.ratio);
        // **Above the published figure, and that is the agreement rather than a
        // gap in it.** These are six- and seven-tooth pinions, and their shifts
        // are left automatic, so they carry the shift such counts need to exist
        // at all — meshes better than the gearbox's 99.73 %, and by the relation
        // a drive that keeps more than its 89 %. What the comparison establishes
        // is the relation, which the three assertions above check against the
        // published pair directly; this one checks the stage lands where the
        // relation says it should for the teeth it actually has.
        assert!(
            r.efficiency.forward > 0.89 && r.efficiency.forward < 0.93,
            "a drive of this reduction with meshes this good keeps {}",
            r.efficiency.forward
        );
        let implied_here = 1.0 - implied(r.efficiency.forward, 49.0);
        assert!(
            (drive_efficiency(49.0, implied_here) - r.efficiency.forward).abs() < 1e-9,
            "the solve and the relation have to be the same statement"
        );
    }

    /// **A hula is a stage of a train**, and the train does not have to know
    /// which kind it is.
    ///
    /// Ratio, efficiency and backlash reach the accumulation through the same
    /// three accessors every other kind answers, and a drive that cannot be
    /// built refuses through the same channel — the error travelling rather
    /// than being re-diagnosed at the boundary.
    #[test]
    fn a_hula_is_a_stage_a_train_can_carry() {
        use crate::train::{solve_any, Stage, StageTorques};
        let lib = crate::train::test_library();
        let torques = StageTorques {
            peak_forward: 2.0,
            peak_backward: None,
            cyclic: 1.0,
        };
        let stage = Stage::Hula(Box::default());
        let r = solve_any(&stage, 1000.0, torques, &lib).expect("a stage a train can solve");
        // The shipped arrangement's own ratio, `z₂z₄/(z₂z₄ − z₁z₃)` at 61 ± 4.
        assert!((r.ratio() - 3721.0 / 16.0).abs() < 1e-9);
        assert!(r.efficiency().forward > 0.5 && r.efficiency().forward < 1.0);
        assert!(r.backlash().forward.nominal > 0.0);
        assert!(r.as_hula().is_some(), "and it says which kind it is");

        // ...and a drive with no geometry refuses through the train's channel.
        // `z₁z₃ = z₂z₄`, so the two meshes cancel and there is no ratio.
        let mut locked = HulaStage::default();
        for (gear, count) in locked.gears.iter_mut().zip([19, 18, 18, 19]) {
            gear.teeth = count;
        }
        let e = solve_any(&Stage::Hula(Box::new(locked)), 1000.0, torques, &lib)
            .expect_err("meshes that cancel are not a stage");
        assert!(
            matches!(
                e,
                crate::train::TrainError::Hula(crate::hula::Error::Locked)
            ),
            "{e:?}"
        );
    }

    /// **The two meshes want the same module**, and the shared offset is why.
    ///
    /// Their modules are separate inputs and nothing in the arithmetic ties
    /// them, so it is worth knowing that moving them apart only costs. Both
    /// meshes run at one offset, and `e ≥ a_ref cos α_t` for each, so:
    ///
    /// - **below equality** the larger mesh still binds, the offset does not
    ///   move, and the smaller mesh's reference centre distance falls away from
    ///   it — its operating pressure angle climbs and the other's stands still;
    /// - **above it** the enlarged mesh binds instead and drags the offset up,
    ///   which pushes the *other* mesh's angle out by exactly what the first one
    ///   gained.
    ///
    /// Either way one mesh sits at its limit and the other is pushed off the
    /// pitch point, so the worse of the two is least where they are equal. That
    /// is a corner where both bounds are active at once, and it is what makes
    /// the design space `(z, d, addendum, shaper, two divisions)` and nothing
    /// more.
    #[test]
    fn the_two_meshes_want_the_same_module() {
        let worst_angle = |ratio: f64| {
            let mut s = HulaStage {
                module: [ratio, 1.0],
                clearance: 0.30,
                ..HulaStage::default()
            };
            for (gear, count) in s.gears.iter_mut().zip([19_u32, 18, 17, 18]) {
                gear.teeth = count;
                gear.addendum = crate::params::Auto::fixed(0.6);
                gear.profile_shift = crate::params::Auto::fixed(-0.2);
            }
            for c in &mut s.cutter {
                c.teeth = 14;
            }
            let r = solve_hula_stage(&s, 1000.0, 2.0).expect("a drive at every module ratio");
            (
                r.meshes[0]
                    .operating_pressure_angle
                    .max(r.meshes[1].operating_pressure_angle),
                r.offset_nominal,
            )
        };
        let (equal, equal_offset) = worst_angle(1.0);
        for ratio in [0.8, 0.9, 1.1, 1.3] {
            let (angle, offset) = worst_angle(ratio);
            assert!(
                angle > equal + 1e-9,
                "m₁/m₂ = {ratio} should cost: {angle}° against {equal}°"
            );
            // ...and which way it costs, since the two sides fail differently.
            if ratio < 1.0 {
                assert!(
                    (offset - equal_offset).abs() < 1e-9,
                    "below equality the other mesh still binds, so the offset holds"
                );
            } else {
                assert!(
                    offset > equal_offset + 1e-9,
                    "above it the enlarged mesh binds and drags the offset up"
                );
            }
        }
    }

    /// An arrangement whose meshes cancel is refused by the stage, as by the
    /// drive: the error travels rather than being re-diagnosed.
    #[test]
    fn a_locked_arrangement_is_refused_by_the_stage() {
        // z2 z4 = z1 z3: 18*19 = 19*18, so the two meshes step by the same
        // amount and cancel.
        let mut s = stage();
        s.gears[2].teeth = 18;
        s.gears[3].teeth = 19;
        assert_eq!(
            solve_hula_stage(&s, 100.0, 2.0).unwrap_err(),
            Error::Drive(hula::Error::Locked)
        );
    }
    /// **At a held crank, one mesh's split says nothing about the other's.**
    ///
    /// This is what lets the search choose each mesh alone rather than the two
    /// together — a thirteenth of the work — so it is gated rather than
    /// asserted: moving one mesh's split must leave the other mesh's operating
    /// pressure angle, contact ratio and shifts exactly where they were.
    ///
    /// It holds because each pair's shift *sum* belongs to the crank, which is
    /// held here, and the division is internal to its own pair.
    #[test]
    fn a_held_crank_leaves_each_mesh_to_itself() {
        let base = HulaStage::default();
        let settled = solve_hula_stage(&base, 1000.0, 2.0).expect("the drive solves");
        let at = |split: f64| {
            let mut s = HulaStage {
                offset: Auto::fixed(settled.offset_nominal),
                ..HulaStage::default()
            };
            // Mesh 0's pinion carries its free number, mesh 1's is left
            // alone. Pinning the pinion's shift is what names it as the given
            // member now — there is no separate control saying so.
            let (a, b) = (0, 1);
            let named = if s.gears[a].teeth > s.gears[b].teeth {
                b
            } else {
                a
            };
            s.gears[named].profile_shift = Auto::fixed(split);
            solve_hula_stage(&s, 1000.0, 2.0).expect("solves")
        };
        let (lo, hi) = (at(0.0), at(0.4));
        assert!(
            (lo.meshes[0].operating_pressure_angle - hi.meshes[0].operating_pressure_angle).abs()
                < 1e-9,
            "the crank fixes each pair's sum, so mesh 0's angle should not move either"
        );
        for field in [
            (
                lo.meshes[1].operating_pressure_angle,
                hi.meshes[1].operating_pressure_angle,
            ),
            (
                lo.meshes[1].contact_ratios.transverse,
                hi.meshes[1].contact_ratios.transverse,
            ),
            (lo.gears[2].profile_shift, hi.gears[2].profile_shift),
            (lo.gears[3].profile_shift, hi.gears[3].profile_shift),
        ] {
            assert!(
                (field.0 - field.1).abs() < 1e-12,
                "mesh 1 moved with mesh 0's split: {} against {}",
                field.0,
                field.1
            );
        }
        // ...and mesh 0 did move, or the test proved nothing.
        assert!((lo.gears[0].profile_shift - hi.gears[0].profile_shift).abs() > 1e-6);
    }

    /// **The split is worth something**, and choosing it is additive: a drive
    /// that did not ask keeps the split it was given.
    ///
    /// Where the crank offset is given, the shift *sum* each mesh must reach is
    /// given with it, so the search can move only the division — and the drive
    /// keeps more of its power for it. Where the offset is instead solved for a
    /// clearance the sum is free too, which is why that case is checked
    /// separately rather than folded into one assertion.
    #[test]
    fn choosing_the_split_leaves_the_drive_more_of_its_power() {
        let run = |offset: Auto<f64>, on: bool| {
            solve_hula_stage(
                &HulaStage {
                    offset,
                    optimisation: Optimisation {
                        enabled: on,
                        ..Optimisation::default()
                    },
                    ..HulaStage::default()
                },
                3000.0,
                2.0,
            )
        };
        // The offset the default drive settles at, then held there.
        let free = run(HulaStage::default().offset, false).expect("the ordinary drive solves");
        let held = Auto::fixed(free.offset_nominal);

        let plain = run(held, false).expect("the held drive solves");
        let tuned = run(held, true).expect("the tuned drive solves");

        // The signed sum, since a ring's shift enters a mesh negatively — that
        // signed quantity is what an offset fixes, and the plain sum is not.
        let sum = |r: &HulaResult, mesh: usize| {
            [mesh * 2, mesh * 2 + 1]
                .map(|i| {
                    if r.gears[i].ring {
                        -r.gears[i].profile_shift
                    } else {
                        r.gears[i].profile_shift
                    }
                })
                .iter()
                .sum::<f64>()
        };
        for mesh in 0..2 {
            assert!(
                (sum(&tuned, mesh) - sum(&plain, mesh)).abs() < 1e-6,
                "mesh {mesh}: a given offset fixes the sum, {} moved from {}",
                sum(&tuned, mesh),
                sum(&plain, mesh)
            );
        }
        assert!(
            tuned
                .gears
                .iter()
                .zip(&plain.gears)
                .any(|(t, p)| (t.profile_shift - p.profile_shift).abs() > 1e-6),
            "the search moved no shift at all"
        );
        assert!(
            tuned.efficiency.forward > plain.efficiency.forward,
            "{:.6} should beat {:.6}",
            tuned.efficiency.forward,
            plain.efficiency.forward
        );

        // And with the offset solved for clearance instead, the sum is free as
        // well — the search may take a different crank, and still has to pay
        // for itself.
        let loose = run(HulaStage::default().offset, true).expect("the tuned drive solves");
        assert!(loose.efficiency.forward > free.efficiency.forward);
    }
}
