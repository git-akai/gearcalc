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

use crate::contact::Directional;
use crate::hula::{self, Offset, Split, Teeth};
use crate::mesh::{Mesh, MeshError, MeshKind, MeshSide};
use crate::note::Note;

use crate::ring::{mesh_with, Cutter, Ring};
use crate::tooth::Tooth;
use crate::train::StageGear;
use crate::GearParams;

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
    /// What decides the crank offset.
    pub offset: Offset,
    /// What decides each mesh's shift distribution.
    pub split: [Split; 2],
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

impl Default for HulaStage {
    fn default() -> Self {
        let gear = |teeth: u32| StageGear {
            teeth,
            addendum: crate::params::Auto::fixed(0.8),
            dedendum: 1.0,
            ..StageGear::default()
        };
        Self {
            module: [1.0, 1.0],
            pressure_angle: 20.0,
            helix_angle: 0.0,
            thickness_mod: [1.0, 1.0],
            // **The tips set this, not the far-side gap.** A fifth of a module
            // leaves the two tip circles overlapping where they cross, 134° from
            // the line of centres; three tenths clears at every tooth count
            // tried, and the margin is reported so a design can be taken closer.
            clearance: 0.3,
            running_clearance: 0.02,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            offset: Offset::Clearance,
            split: [Split::Pinion(0.0); 2],
            // **A shaper is coupled to the shift it has to cut**, not only to
            // the ring's size: the clearance drives the ring's shift up, and a
            // tool that reached its flank at one shift stops reaching it at a
            // larger one. Twelve teeth cuts the shipped rings clean over the
            // gaps worth running; a ring far from these counts will want its
            // own, and says so through its clamps rather than quietly coming out
            // without a fillet.
            cutter: [Cutter {
                teeth: 12,
                ..Cutter::default()
            }; 2],
            gears: [gear(19), gear(18), gear(17), gear(18)],
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
    /// Operating pressure angle, degrees.
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
    pub contact_ratio: f64,
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
    /// The offset actually run at, including the running clearance.
    pub offset: f64,
    /// Which mesh sits at the clearance minimum, when the offset came from it.
    pub binding_mesh: Option<usize>,
    /// Speed of the crank, rpm — the input, and the carrier of both meshes.
    pub crank_speed: f64,
    /// Angular backlash at whichever shaft is the **output**, degrees: gear 4
    /// driving forward, the crank driving backward. The same two plays subtend
    /// very different angles at the two, by exactly the reduction.
    pub backlash: Directional<super::Backlash>,
    pub meshes: [HulaMesh; 2],
    pub gears: [HulaGear; 4],
}

/// Solve a hula stage: the arrangement, then the parts, then the meshes.
///
/// # Errors
///
/// [`crate::hula::Error`], through [`super::TrainError::Hula`] — a drive that
/// cannot exist rather than a solve that gave up.
pub fn solve_hula_stage(stage: &HulaStage, input_speed: f64) -> Result<HulaResult, Error> {
    let teeth = Teeth(stage.gears.each_ref().map(|g| g.teeth));
    let set = hula::Set {
        teeth,
        module: stage.module,
        pressure_angle: stage.pressure_angle,
        helix_angle: stage.helix_angle,
        addendum: stage.gears.each_ref().map(|g| g.addendum.manual),
        clearance: stage.clearance,
        offset: stage.offset,
        split: stage.split,
    };
    let layout = hula::solve(&set)?;
    let pairs = [teeth.pair(0)?, teeth.pair(1)?];
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
    // Filled as each pair is built, since a gear belongs to exactly one mesh
    // and is finished the moment its own pair is.
    let mut gears: [Option<HulaGear>; 4] = [None, None, None, None];
    let speed = |i: usize| match i {
        0 => 0.0,
        1 | 2 => wobble_speed,
        _ => output_speed,
    };

    for (index, pair) in pairs.iter().enumerate() {
        let params = |i: usize| GearParams {
            module: stage.module[index],
            pressure_angle: stage.pressure_angle,
            helix_angle: stage.helix_angle,
            teeth: teeth.0[i],
            profile_shift: layout.shift[i],
            addendum: stage.gears[i].addendum.manual,
            dedendum: stage.gears[i].dedendum,
            root_radius: stage.gears[i].root_radius,
            thickness_mod: stage.thickness_mod[index],
            ..GearParams::default()
        };
        // The ring twice over, and neither reading is redundant. `Ring` is the
        // part as its shaper leaves it — the tip it really has, the fillet, the
        // clamps — while `Mesh` is the pair's rolling geometry, which is where
        // backlash lives for every stage here. Building the second from the
        // first's parameters is what keeps one relation in one place.
        let ring = Ring::cut_by(&params(pair.ring), &stage.cutter[index]);
        let pinion = Tooth::new(params(pair.pinion));
        let mesh = Mesh::new(&pinion, &Tooth::new(params(pair.ring)), MeshKind::Internal)
            .map_err(Error::Mesh)?;

        gears[pair.ring] = Some(HulaGear {
            teeth: teeth.0[pair.ring],
            ring: true,
            profile_shift: layout.shift[pair.ring],
            pitch_radius: ring.r,
            base_radius: ring.rb,
            tip_radius: ring.ra,
            root_radius: ring.rf,
            speed: speed(pair.ring),
            clamps: ring.clamps.clone(),
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
            clamps: pinion.clamps.notes.clone(),
        });

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
            contact_ratio: report.as_ref().map_or(0.0, |m| m.contact_ratio),
            trochoid_interference: report.as_ref().is_some_and(|m| m.trochoid_interference),
            involute_interference: report.as_ref().is_some_and(|m| m.involute_interference),
            tip_interference: report.as_ref().is_some_and(|m| m.tip_interference),
            tip_margin: report.as_ref().map_or(0.0, |m| m.tip_margin.to_degrees()),
            backlash: [backlash_of(MeshSide::First), backlash_of(MeshSide::Second)],
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

    Ok(HulaResult {
        ratio: layout.ratio.value(),
        ratio_products: [layout.ratio.numerator, layout.ratio.denominator],
        offset_nominal: layout.offset,
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

    fn stage() -> HulaStage {
        HulaStage::default()
    }

    /// The stage reports the arrangement's ratio, products and all — it does not
    /// recompute one.
    #[test]
    fn the_stage_reports_the_arrangements_ratio() {
        let r = solve_hula_stage(&stage(), 100.0).unwrap();
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
        let r = solve_hula_stage(&s, 100.0).unwrap();
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
        let r = solve_hula_stage(&stage(), 100.0).unwrap();
        assert_eq!(
            r.gears.each_ref().map(|g| g.ring),
            [true, false, false, true]
        );
    }

    /// Gear 1 is held, gears 2 and 3 are one body, and the output turns at the
    /// input over the ratio. Nothing here is a second kinematic model.
    #[test]
    fn the_speeds_are_the_arrangements() {
        let r = solve_hula_stage(&stage(), 3240.0).unwrap();
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
            let r = solve_hula_stage(&s, 100.0).unwrap();
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
        let r = solve_hula_stage(&s, 100.0).unwrap();
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
            let r = solve_hula_stage(&s, 100.0).unwrap();
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
            let r = solve_hula_stage(&s, 100.0).unwrap();
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
        let r = solve_hula_stage(&tight, 100.0).unwrap();
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
            let j = solve_hula_stage(&s, 100.0)
                .unwrap()
                .backlash
                .forward
                .nominal;
            assert!(j > last, "the output's play fell from {last} to {j}");
            last = j;
        }
    }

    /// **The shipped drive clears its tips, and a fifth of a module does not.**
    ///
    /// The far-side gap and the tip fouling are different bounds and the second
    /// is the one that binds here. The figures either side of it are tight — a
    /// tenth of a degree of pinion rotation — which is what makes this a gate on
    /// the arithmetic rather than on the sign: the two windows are on different
    /// wheels and one has to be carried onto the other before they can be
    /// compared, and a comparison that skips that step lands the wrong side of
    /// this boundary.
    ///
    /// Both figures are what rolling the outlines through a tooth measures
    /// (`gear-cli hulasweep`): 0.22 fouls by 3 µm and 0.3 does not foul.
    #[test]
    fn the_tips_are_what_the_clearance_has_to_clear() {
        let clear = solve_hula_stage(&stage(), 100.0).unwrap();
        for mesh in &clear.meshes {
            assert!(
                !mesh.tip_interference,
                "the shipped gap should clear the tips, margin {}",
                mesh.tip_margin
            );
        }
        let tight = solve_hula_stage(
            &HulaStage {
                clearance: 0.22,
                ..stage()
            },
            100.0,
        )
        .unwrap();
        assert!(
            tight.meshes.iter().any(|m| m.tip_interference),
            "a fifth of a module leaves the tips overlapping: margins {:?}",
            tight.meshes.each_ref().map(|m| m.tip_margin)
        );
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
            solve_hula_stage(&s, 100.0).unwrap_err(),
            Error::Drive(hula::Error::Locked)
        );
    }
}
