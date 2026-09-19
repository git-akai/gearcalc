//! **The hula preset**: four gears — the grounded one, the two that ride the
//! wobble body, the output — on a crank, as a designer states them, and the
//! shape they lay out as ([`super::shape::Shape::from`]).
//!
//! A hula stage is a compound planet with one planet and both meshes
//! internal: a central axis carrying the grounded gear, the crank and the
//! output, a wobble axis the crank carries with both wobble gears on one
//! shaft, two internal meshes on the one distance between the axes — the
//! crank offset — which the tips size where it is automatic
//! ([`super::shape::Distance::tip_clearance`]). Which member of each pair is the ring is a
//! tooth count, the larger. Everything the stage's own solver once did —
//! the offset from the clearances, the shifts that reach it, the power flow
//! with its circulating power, the play at either shaft — the shape solves
//! as it solves any arrangement, and `hula_recorded` in `shape.rs` holds it
//! to that solver's figures, which are recorded in `docs/reference.md`.
//!
//! What stays here is the preset, and [`stage_efficiency`] — the closed form
//! this family's efficiency collapses to, kept as the independent statement
//! the per-mesh flow is checked against.

use super::{Optimisation, StageGear};
use crate::ring::Cutter;
use crate::Auto;

/// A hula stage as its inputs describe it.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
    /// **The axial contact ratio** `ε_β` the stage is asked for, where it is
    /// asked for one — the same input a pair has ([`super::PairStage::overlap`]),
    /// asked of both meshes: a floor under every automatic face width, or,
    /// with all four widths given, the thing that decides the helix, so that
    /// the narrower of the two meshes reaches it.
    pub overlap: Auto<f64>,
    /// `k` for each mesh's **pinion**. Its ring takes the same figure, because
    /// on a ring `k` describes the space, and a pinion and a ring that mesh
    /// want the same one rather than complementary ones.
    pub thickness_mod: [f64; 2],
    /// Coefficient of friction in each mesh.
    pub sliding_friction: [f64; 2],
    /// Coefficient of **static** friction in each mesh, for breaking away.
    ///
    /// Whether a stage turns at all is decided at rest and against this; how
    /// well it does once turning is decided against the sliding coefficient,
    /// which is lower. See [`Directional::once_moving`].
    pub static_friction: [f64; 2],
    /// The smallest far-side tip gap any mesh may run at, mm.
    ///
    /// A minimum, and checked whether or not the offset is taken from it: an
    /// offset that fails it describes a stage that could be built and would
    /// foul, which is a thing a designer is owed the number for rather than a
    /// refusal.
    pub clearance: f64,
    /// Added to the crank offset, mm — the running clearance.
    ///
    /// **One number, because there is one crank.** Both meshes are separated by
    /// the same distance by construction, so a clearance on each could be set
    /// to disagree about a distance that is physically single.
    pub running_clearance: Auto<f64>,
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
    /// What the stage is asked to optimise, and what it may not do to get
    /// there. See [`Optimisation`].
    ///
    /// A pair's shift *sum* is fixed by the offset the crank has to reach, so
    /// within a mesh only the division between ring and pinion is free — and
    /// that division is worth real efficiency. The contact ratio defaults lower
    /// here than the shared default, for a reason that is the stage's rather
    /// than a relaxation of the rule: a mesh of one tooth of difference has a
    /// very short path and sits just above continuous contact at every split it
    /// can be built at, so a pair's usual 1.2 of design margin would forbid the
    /// mechanism rather than constrain it.
    #[cfg_attr(feature = "serde", serde(default))]
    pub optimisation: Optimisation,
    /// How the load is divided while two tooth pairs are engaged.
    ///
    /// **Off by default, and it reaches bending only** — see
    /// [`super::PairStage::load_sharing`], which is the same input for the same
    /// reason. One switch for the stage rather than one per mesh: it selects a
    /// *model*, and a stage running two meshes under two different models of
    /// the same thing would be reporting a comparison rather than a design.
    #[cfg_attr(feature = "serde", serde(default))]
    pub load_sharing: crate::contact::LoadSharing,
    /// The shaper each mesh's ring is cut with.
    ///
    /// **A shaper has to be smaller than the ring it cuts**, and the rings here
    /// are small: a tool larger than its workpiece is clamped down to the
    /// ring's own count and then reaches none of its flank, leaving no fillet
    /// at all. Both defaults are well below the default rings for that reason.
    pub cutter: [Cutter; 2],
    /// The four gears in the order the arrangement is read: the grounded one, the two that
    /// ride the wobble body, then the output.
    pub gears: [StageGear; 4],
}

impl Default for HulaStage {
    fn default() -> Self {
        // **The addendum belongs to the difference, not to the stage.** Four
        // teeth of difference runs at a far lower operating pressure angle than
        // one does, so a tooth that cleared the involute interference limit at
        // one tooth of difference reaches past it here: on these counts the
        // limit sits between 0.70 and 0.75, measured, and the taller tooth
        // costs efficiency on the way as well — 0.70 keeps 79.6 % where 0.80
        // keeps 73.1 % and fouls (docs/reference.md#the-hula-stage). A stage
        // taken to another difference will want its own proportion, and the
        // interference row is what says so.
        let gear = |teeth: u32| StageGear {
            teeth,
            addendum: 0.7,
            dedendum: 1.0,
            ..StageGear::default()
        };
        Self {
            module: [1.0, 1.0],
            pressure_angle: 20.0,
            overlap: Auto::automatic(1.0),
            thickness_mod: [1.0, 1.0],
            sliding_friction: [0.08, 0.08],
            static_friction: [0.16, 0.16],
            // **The tips set this, not the far-side gap.** A fifth of a module
            // leaves the two tip circles overlapping where they cross, 134° from
            // the line of centres; three tenths clears at every tooth count
            // tried, and the margin is reported so a design can be taken closer.
            clearance: 0.3,
            running_clearance: Auto::fixed(0.02),
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            load_sharing: crate::contact::LoadSharing::None,
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
                // **The tool cuts to the depth these teeth have.** The shared
                // default is 1.25, the rack proportion, and it would sink the
                // rings a quarter of a module below the 1.0 dedendum the
                // external members here are cut to — a deeper space than the
                // pair needs and one more thing not to match.
                addendum: 1.0,
                ..Cutter::default()
            }; 2],
            // `N ± d` about 61, at four teeth of difference on each mesh —
            // the same `[N+d, N, N−d, N]` arrangement the counts have always
            // taken, at a size and a difference a real reducer is built at.
            gears: [gear(65), gear(61), gear(57), gear(61)],
        }
    }
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
pub fn stage_efficiency(ratio: f64, mesh: f64) -> f64 {
    1.0 / (ratio.abs() * (1.0 - mesh) + mesh)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **The hula tables `docs/reference.md#the-hula-stage` prints are the
    //! ones this code prints**, through the shape. Prose is the copy no test
    //! reads, and these sections had drifted before the tests existed. A
    //! canary rather than an invariant: the digits are free to move, and not
    //! free to move *quietly*. Asserted to half of the last digit each table
    //! prints, since that is the claim it makes.

    use super::super::shape::{solve_shape, Shape, ShapeResult};
    use super::super::{test_library, Reversal, StageBoundary, StageLoads, TrainError};
    use super::*;

    /// The hula's own arrangement: crank driven, grounded gear held, output
    /// out — the shape's shafts 1, 3 and 2.
    fn solve(stage: &HulaStage, speed: f64) -> Result<ShapeResult, TrainError> {
        solve_shape(
            &Shape::from(stage),
            &StageLoads::at(2.0, speed).under(StageBoundary::holding(5, &[3], 1, 2)),
            &test_library(),
            Reversal::default(),
        )
    }

    /// The two meshes alone, crank held: `η₀`.
    fn eta0(r: &ShapeResult) -> f64 {
        r.meshes.iter().map(|m| m.efficiency.forward).product()
    }

    fn alpha_w(r: &ShapeResult, mesh: usize) -> f64 {
        r.meshes[mesh].line.unwrap().operating_pressure_angle
    }

    /// One tooth of difference about 18, at 0.8 of a module, with a shaper
    /// that fits the rings this fixture builds — the fixture the tables were
    /// generated from.
    fn stage() -> HulaStage {
        let mut s = HulaStage::default();
        for (gear, count) in s.gears.iter_mut().zip([19, 18, 17, 18]) {
            gear.teeth = count;
            gear.addendum = 0.8;
        }
        for cutter in &mut s.cutter {
            cutter.teeth = 12;
        }
        s
    }

    #[test]
    fn the_documented_tables_are_the_ones_this_code_prints() {
        // | reduction | meshes, crank held | the stage |
        for (n, reduction, meshes, keeps) in [
            (12_u32, 144.0, 98.85, 37.9),
            (18, 324.0, 99.18, 27.4),
            (30, 900.0, 99.50, 18.1),
            (50, 2500.0, 99.69, 11.5),
        ] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                gear.teeth = count;
            }
            let r = solve(&s, 1000.0).unwrap();
            assert!(
                (r.ratio.abs() - reduction).abs() < 1e-9,
                "z{n}: {}",
                r.ratio
            );
            let got = (eta0(&r) * 100.0, r.efficiency.forward * 100.0);
            assert!(
                (got.0 - meshes).abs() < 0.005 && (got.1 - keeps).abs() < 0.05,
                "z{n}: the table says {meshes} % / {keeps} %, this gives {:.2} / {:.1}",
                got.0,
                got.1
            );
        }

        // | m₁/m₂ | offset | α_w mesh 1 | α_w mesh 2 | — the angles the
        // **running** meshes turn at, 0.02 mm inside the offset quoted.
        for (ratio, offset, first, second) in [
            (0.8_f64, 0.813, 61.7, 53.6),
            (0.9, 0.813, 57.8, 53.6),
            (1.0, 0.813, 53.6, 53.6),
            (1.1, 0.884, 53.3, 57.1),
            (1.3, 1.028, 52.7, 62.2),
        ] {
            let s = HulaStage {
                module: [ratio, 1.0],
                clearance: 0.30,
                ..stage()
            };
            let r = solve(&s, 1000.0)
                .unwrap_or_else(|e| panic!("m {ratio}: the table's row must solve: {e}"));
            let angles = [alpha_w(&r, 0), alpha_w(&r, 1)];
            let nominal = r.distances[0].nominal[0];
            assert!(
                (nominal - offset).abs() < 0.0005
                    && (angles[0] - first).abs() < 0.05
                    && (angles[1] - second).abs() < 0.05,
                "m {ratio}: the table says {offset} mm and {first}°/{second}°, \
                 this gives {nominal:.3} mm and {:.1}°/{:.1}°",
                angles[0],
                angles[1]
            );
        }
    }

    #[test]
    fn the_four_hula_studies_are_the_ones_this_code_prints() {
        // | arrangement | `D` | ratio | meshes | the stage |    — at N = 18.
        // Each is solved under the hula's own arrangement whatever its
        // counts make the rings, which is what the explicit boundary is for.
        for (name, counts, ratio, meshes, keeps) in [
            ("N+1/N/N−1/N", [19u32, 18, 17, 18], 324.0_f64, 99.18, 27.4),
            ("N/N+1/N/N−1", [18, 19, 18, 17], -323.0, 99.18, 27.2),
            ("N+1/N/N/N−1", [19, 18, 18, 17], -8.5, 99.18, 92.7),
            ("N/N+1/N/N+1", [18, 19, 18, 19], 9.8, 99.20, 93.5),
        ] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(counts) {
                gear.teeth = count;
            }
            let r = solve(&s, 1000.0).unwrap_or_else(|e| panic!("{name}: must solve: {e}"));
            assert!(
                (r.ratio - ratio).abs() < 0.05,
                "{name}: the table says a ratio of {ratio}, this gives {}",
                r.ratio
            );
            let got = (eta0(&r) * 100.0, r.efficiency.forward * 100.0);
            assert!(
                (got.0 - meshes).abs() < 0.005 && (got.1 - keeps).abs() < 0.05,
                "{name}: the table says {meshes} % / {keeps} %, this gives {:.2} / {:.1}",
                got.0,
                got.1
            );
        }

        // | `h_a` | involute interference | ε_α | the stage |   — on the shipped
        // N ± 4 about 61
        for (addendum, fouls, eps, keeps) in [
            (0.60_f64, false, 1.17, 90.4),
            (0.65, false, 1.26, 86.2),
            (0.70, false, 1.35, 81.9),
            (0.75, true, 1.44, 78.2),
            (0.80, true, 1.52, 75.0),
        ] {
            let mut s = HulaStage::default();
            for gear in &mut s.gears {
                gear.addendum = addendum;
            }
            let r = solve(&s, 1000.0).unwrap_or_else(|e| panic!("h_a {addendum}: must solve: {e}"));
            // The involute interference the table names is the pinion's flank
            // reached past its end by the ring's tip, on either mesh.
            let got_fouls = r.meshes.iter().any(|m| m.flank_interference[0]);
            let got_eps = r.meshes[0].line.unwrap().contact_ratios.transverse;
            let got_keeps = r.efficiency.forward * 100.0;
            assert!(
                got_fouls == fouls,
                "h_a {addendum}: the table says fouls={fouls}, this says {got_fouls}"
            );
            assert!(
                (got_eps - eps).abs() < 0.005 && (got_keeps - keeps).abs() < 0.05,
                "h_a {addendum}: the table says ε {eps} / {keeps} %, this gives {:.2} / {:.1}",
                got_eps,
                got_keeps
            );
        }

        // | d | reduction | α_w | the pair keeps | the stage keeps |  — z = 36,
        // h_a = 0.6, μ = 0.08, each pair optimised alone
        // | d | least loss (Σx, x_ring, x_pinion) | least shift | stage, best | stage, least |
        for (d, reduction, alpha, pair_keeps, best, least) in [
            (2u32, 324.0_f64, 33.0, 99.893, 58.39, 54.81),
            (3, 144.0, 25.2, 99.966, 90.57, 79.52),
            (4, 81.0, 20.9, 99.959, 93.25, 91.72),
            (5, 51.8, 18.8, 99.948, 94.25, 94.25),
        ] {
            let build = |optimise: bool| {
                let mut s = HulaStage {
                    sliding_friction: [0.08; 2],
                    ..HulaStage::default()
                };
                s.optimisation.enabled = optimise;
                for (gear, count) in s.gears.iter_mut().zip([36 + d, 36, 36 - d, 36]) {
                    gear.teeth = count;
                    gear.addendum = 0.6;
                }
                solve(&s, 1000.0).unwrap_or_else(|e| panic!("d {d}: must solve: {e}"))
            };
            let (on, off) = (build(true), build(false));
            assert!(
                (on.ratio.abs() - reduction).abs() < 0.05,
                "d {d}: the table says {reduction}, this gives {}",
                on.ratio
            );
            let aw = alpha_w(&on, 0);
            let pk = on.meshes[0].efficiency.forward * 100.0;
            assert!(
                (aw - alpha).abs() < 0.05 && (pk - pair_keeps).abs() < 0.0005,
                "d {d}: the table says α_w {alpha}° and the pair keeps {pair_keeps} %, \
                 this gives {aw:.1}° and {pk:.3} %"
            );
            let (got_best, got_least) = (
                on.efficiency.forward * 100.0,
                off.efficiency.forward * 100.0,
            );
            assert!(
                (got_best - best).abs() < 0.005 && (got_least - least).abs() < 0.005,
                "d {d}: the table says {best} % best and {least} % least, this gives \
                 {got_best:.2} % and {got_least:.2} %"
            );
            // **The two pairs land at the same operating angle** — the physical
            // claim the prose makes, and the one that makes equal modules cost
            // nothing.
            let twin = alpha_w(&on, 1);
            assert!(
                (aw - twin).abs() < 0.005,
                "d {d}: the two meshes should sit at one angle: {aw:.2}° and {twin:.2}°"
            );
        }

        // | d | least loss (Σx, x_ring, x_pinion) | least shift |  — the shift
        // columns of the same table, to the two decimals it prints.
        for (d, on_shifts, off_shifts) in [
            (2u32, [-0.19_f64, 0.37, 0.18], [-0.20_f64, 0.20, 0.00]),
            (3, [-0.09, 0.51, 0.42], [-0.11, 0.11, 0.00]),
            (4, [-0.03, 0.37, 0.34], [-0.05, 0.05, 0.00]),
            (5, [0.00, 0.00, 0.00], [0.00, 0.00, 0.00]),
        ] {
            let build = |optimise: bool| {
                let mut s = HulaStage {
                    sliding_friction: [0.08; 2],
                    ..HulaStage::default()
                };
                s.optimisation.enabled = optimise;
                for (gear, count) in s.gears.iter_mut().zip([36 + d, 36, 36 - d, 36]) {
                    gear.teeth = count;
                    gear.addendum = 0.6;
                }
                solve(&s, 1000.0).unwrap_or_else(|e| panic!("d {d}: must solve: {e}"))
            };
            for (optimise, want) in [(true, on_shifts), (false, off_shifts)] {
                let r = build(optimise);
                let (ring, pin) = (r.members[0].profile_shift, r.members[1].profile_shift);
                let got = [pin - ring, ring, pin];
                for (g, w) in got.iter().zip(want) {
                    assert!(
                        (g - w).abs() < 0.005,
                        "d {d} optimise={optimise}: the table says {want:?}, this gives \
                         [{:.2}, {:.2}, {:.2}]",
                        got[0],
                        got[1],
                        got[2]
                    );
                }
            }
        }
    }

    /// **The power circulates, and the figure says by how much.** On a pair
    /// everything crosses the one mesh, once; on a hula each mesh passes
    /// about `η |R − 1|` times the input to cancel to the output — 88× at
    /// 324 : 1 keeping 27 %, 9× at 8.5 : 1 keeping 93 % — and the stage's loss is
    /// each mesh's loss on the power crossing it, exactly, which is what
    /// `η = 1/[R(1 − η₀) + η₀]` folds into one line.
    #[test]
    fn the_circulating_power_is_the_reductions_worth() {
        let r = solve(&stage(), 1000.0).unwrap();
        let through = r.circulation.forward;
        assert!(through > 100.0, "324 : 1 circulates: {through}× the input");
        // The loss is each mesh's loss on the power crossing it: what the
        // teeth pass, less what comes out, over the input — to the
        // per-mesh accounting the flow keeps, exactly.
        let lost = 1.0 - r.efficiency.forward;
        let by_mesh: f64 = r
            .meshes
            .iter()
            .map(|m| (1.0 - m.efficiency.forward) * m.power_through.forward)
            .sum();
        assert!(
            (lost - by_mesh).abs() < 1e-9,
            "loss {lost} against the meshes' {by_mesh}"
        );
        assert!(
            (r.meshes[0].power_through.forward + r.meshes[1].power_through.forward - through).abs()
                < 1e-12
        );
        // ...and each mesh passes about `R η` times the input, on the
        // stage that cancels and on one that does not.
        let mut s = stage();
        for (gear, count) in s.gears.iter_mut().zip([19u32, 18, 18, 17]) {
            gear.teeth = count;
        }
        let plain = solve(&s, 1000.0).unwrap();
        // The output turns `R` times slower than the crank, so relative to
        // the crank the mesh sees the output's torque at `|1 − 1/R|` of the
        // crank's speed: `η |R − 1|` of the input, give or take the loss's
        // own share.
        for r in [&r, &plain] {
            let expect = (r.ratio - 1.0).abs() * r.efficiency.forward;
            for m in &r.meshes {
                let got = m.power_through.forward;
                assert!(
                    (got - expect).abs() / expect < 0.05,
                    "{} : 1 keeping {}: a mesh passes {got}× against about {expect}×",
                    r.ratio,
                    r.efficiency.forward
                );
            }
        }
    }

    /// **The power flow collapses to one relation**, and the shape's flow —
    /// mesh by mesh, with each mesh's loss in the direction it turns — agrees
    /// with it everywhere: `η = 1/[R(1 − η₀) + η₀]`, written from the torque
    /// shares by hand. Agreeing across three reductions and four friction
    /// coefficients says the closed form is the same statement, which is what
    /// makes it safe to design against.
    #[test]
    fn the_stage_efficiency_is_the_reduction_and_the_meshes() {
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
                let r = solve(&s, 1000.0).unwrap();
                let want = stage_efficiency(r.ratio, eta0(&r));
                assert!(
                    (r.efficiency.forward - want).abs() < 1e-9,
                    "z {n} mu {mu}: solve {} against the relation {want}",
                    r.efficiency.forward
                );
            }
        }
    }

    /// **Checked against a gearbox somebody built.** The bilateral drive
    /// gear is a 3K of this family reporting 89.0 % forward against 68.5 %
    /// uncorrected; read through the relation those are meshes at 99.73 %
    /// and 99.04 % at a reduction near fifty, and this stage at that
    /// reduction lands where the relation says it should for the teeth it
    /// actually has.
    #[test]
    fn the_relation_agrees_with_a_gearbox_somebody_built() {
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
        assert!((stage_efficiency(49.0, optimised) - 0.890).abs() < 1e-3);
        let mut s = HulaStage {
            sliding_friction: [0.010; 2],
            static_friction: [0.020; 2],
            ..stage()
        };
        for (gear, count) in s.gears.iter_mut().zip([8_u32, 7, 6, 7]) {
            gear.teeth = count;
        }
        let r = solve(&s, 1000.0).unwrap();
        assert!((r.ratio - 49.0).abs() < 1e-9, "ratio {}", r.ratio);
        assert!(
            r.efficiency.forward > 0.89 && r.efficiency.forward < 0.93,
            "a stage of this reduction with meshes this good keeps {}",
            r.efficiency.forward
        );
        let implied_here = 1.0 - implied(r.efficiency.forward, 49.0);
        assert!((stage_efficiency(49.0, implied_here) - r.efficiency.forward).abs() < 1e-9);
    }

    /// **A reduction that does not come from cancellation is efficient**, and
    /// the same code says so: both families have the same two meshes losing
    /// the same 0.85 % between them and differ only in whether the wobble
    /// body carries two faces of the same kind. Where it does the meshes
    /// nearly cancel and the stage keeps a quarter; where it does not, ninety-
    /// odd percent — an ordinary gearbox.
    #[test]
    fn a_reduction_that_does_not_come_from_cancellation_is_efficient() {
        let solve_z = |z: [u32; 4]| {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(z) {
                gear.teeth = count;
            }
            solve(&s, 1000.0).unwrap()
        };
        for z in [[19, 18, 17, 18], [17, 18, 19, 18]] {
            let r = solve_z(z);
            assert!(r.ratio.abs() > 300.0, "{z:?} reduces by {}", r.ratio);
            assert!(
                r.efficiency.forward < 0.35,
                "{z:?}: {} is too good",
                r.efficiency.forward
            );
        }
        for z in [[19, 18, 18, 17], [17, 18, 18, 19], [18, 17, 19, 18]] {
            let r = solve_z(z);
            assert!(r.ratio.abs() < 12.0, "{z:?} reduces by {}", r.ratio);
            assert!(
                r.efficiency.forward > 0.9,
                "{z:?}: {} is too poor",
                r.efficiency.forward
            );
        }
        let cancelling = eta0(&solve_z([19, 18, 17, 18]));
        let plain = eta0(&solve_z([19, 18, 18, 17]));
        assert!(
            (cancelling - plain).abs() < 0.005,
            "{cancelling} against {plain}"
        );
    }

    /// **A higher reduction costs efficiency**, monotonically, and none of
    /// these turns backwards.
    #[test]
    fn a_higher_reduction_costs_efficiency() {
        let mut last = 1.0;
        for n in [12_u32, 18, 30, 50] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
                gear.teeth = count;
            }
            let r = solve(&s, 100.0).unwrap();
            assert!(
                r.efficiency.forward < last,
                "z {n}: {} did not fall below {last}",
                r.efficiency.forward
            );
            assert_eq!(r.efficiency.backward, 0.0, "z {n} should be self-locking");
            last = r.efficiency.forward;
        }
        assert!(last < 0.2, "2500:1 should be dear: {last}");
    }

    /// **The play at the two shafts differs by the reduction**, and the
    /// output's play is the two meshes' plays referred through the body —
    /// mesh A's at the wobble gear, carried through mesh B by `z₃/z₄`, plus
    /// mesh B's own at the output.
    #[test]
    fn the_outputs_play_is_the_two_meshes_referred_through_the_body() {
        for teeth in [[19_u32, 18, 17, 18], [17, 18, 19, 18], [20, 18, 17, 18]] {
            let mut s = stage();
            for (gear, count) in s.gears.iter_mut().zip(teeth) {
                gear.teeth = count;
            }
            let shape = Shape::from(&s);
            let r = solve(&s, 100.0).unwrap();
            let want = r.ratio.abs();
            let got = r.backlash.backward.nominal / r.backlash.forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: {got} where the reduction is {want}"
            );
            // `backlash[0]` is the pinion's, `[1]` the ring's — the mesh was
            // built with the pinion first.
            let at = |mesh: usize, gear: usize| {
                r.meshes[mesh].backlash[usize::from(shape.members[gear].ring.is_some())].nominal
            };
            let want = at(0, 1) * f64::from(teeth[2]) / f64::from(teeth[3]) + at(1, 3);
            let got = r.backlash.forward.nominal;
            assert!(
                (got - want).abs() < 1e-9 * want,
                "{teeth:?}: {got} referred, {want} from the members"
            );
        }
    }

    /// **The offset clears the tips as well as the far side**, and either can
    /// be what binds: at the fixture's gap the far side asks for more and the
    /// tips have room; ask for a gap the tips cannot live with and the offset
    /// opens past it, the tips at their limit and the far-side gap larger than
    /// asked — the tool answering with what can be built.
    #[test]
    fn the_offset_clears_the_tips_as_well_as_the_far_side() {
        let s = stage();
        let shipped = solve(&s, 100.0).unwrap();
        for mesh in &shipped.meshes {
            let tips = mesh.tips.expect("every hula mesh is internal");
            assert!(
                !tips.tip_interference && tips.tip_margin > 0.1,
                "margin {}",
                tips.tip_margin
            );
        }
        let held = shipped.distances[0].sized_by.expect("the gap held it open");
        assert!(
            (shipped.meshes[held].tips.unwrap().far_gap - s.clearance).abs() < 1e-6,
            "the binding mesh should sit at the gap asked for"
        );
        let tight = HulaStage {
            clearance: 0.22,
            ..stage()
        };
        let opened = solve(&tight, 100.0).unwrap();
        for mesh in &opened.meshes {
            let tips = mesh.tips.expect("every hula mesh is internal");
            assert!(
                !tips.tip_interference,
                "the offset should have opened until the tips cleared"
            );
            assert!(
                tips.far_gap >= tight.clearance - 1e-6,
                "opening for the tips gives the far side more"
            );
        }
        let binding = opened.distances[0]
            .sized_by
            .expect("something held it open");
        assert!(
            opened.meshes[binding]
                .tips
                .is_some_and(|t| t.tip_margin.abs() < 1e-4),
            "the tips are what held it, so they sit at their limit: {:?}",
            opened.meshes[binding].tips
        );
    }
}
