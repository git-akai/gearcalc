//! **The arrangements the one shape makes free**, each written as the list
//! of what sits where and nothing else — no solve, no kinematics, no model.
//!
//! A spur pair and a planetary set are the presets a panel offers
//! ([`super::PairStage`], [`super::PlanetaryStage`]); these are the ones the
//! shape reaches with no code of their own, kept here so the harness
//! (`gear-cli kinematics`) and the suite can name them, and so a future
//! preset is a function of tooth counts rather than a type of its own. Each is a
//! textbook arrangement, and the test beside each holds the ratio the
//! textbook gives.
//!
//! [`Builder`] is the whole vocabulary: an axis, carried or not and
//! replicated or not; a shaft on an axis; a gear or a ring on a shaft; a
//! mesh between two members; a distance between two axes. Shafts are
//! numbered as the wiring numbers them, ground being 0, so the numbers a
//! builder hands back are the ones a train's constraints address.

use super::shape::{Axis, Distance, Member, MeshInput, ShaftOn, Shape};
use super::StageGear;
use crate::kinematics::Shaft;
use crate::params::Auto;
use crate::ring::Cutter;

/// Sliding and static friction every mesh here runs with — the set's.
const FRICTION: (f64, f64) = (0.08, 0.16);

/// A shape assembled a piece at a time, at one module and one pressure
/// angle, every distance automatic and every clearance the shipped 0.02 mm.
#[derive(Clone, Debug)]
pub struct Builder {
    shape: Shape,
    module: f64,
}

impl Builder {
    /// An empty shape at this module, 20° pressure angle.
    #[must_use]
    pub fn new(module: f64) -> Self {
        let set = super::PlanetaryStage::default();
        Self {
            shape: Shape {
                pressure_angle: set.pressure_angle,
                overlap: set.overlap,
                optimisation: set.optimisation,
                load_sharing: set.load_sharing,
                min_planet_clearance: set.min_planet_clearance,
                axes: Vec::new(),
                shafts: Vec::new(),
                members: Vec::new(),
                meshes: Vec::new(),
                distances: Vec::new(),
            },
            module,
        }
    }

    /// An axis fixed in ground.
    pub fn axis(&mut self) -> usize {
        self.carried_axis(None, 1)
    }

    /// An axis carried round by `carrier`, `count` times.
    pub fn carried_axis(&mut self, carrier: Option<Shaft>, count: u32) -> usize {
        self.shape.axes.push(Axis {
            carried_by: carrier,
            count,
        });
        self.shape.axes.len() - 1
    }

    /// A shaft on an axis, numbered as the wiring numbers it.
    pub fn shaft(&mut self, axis: usize) -> Shaft {
        self.shape.shafts.push(ShaftOn { axis });
        self.shape.shafts.len()
    }

    /// An external gear on a shaft; the member's index.
    pub fn gear(&mut self, shaft: Shaft, teeth: u32) -> usize {
        self.member(shaft, teeth, None)
    }

    /// A ring on a shaft, cut by the shipped cutter, its shift automatic
    /// like every other member's here — a second ring at one carrier radius
    /// is closed by its shift, which a ring given at zero could not do; the
    /// member's index.
    pub fn ring(&mut self, shaft: Shaft, teeth: u32) -> usize {
        self.member(shaft, teeth, Some(Cutter::default()))
    }

    fn member(&mut self, shaft: Shaft, teeth: u32, ring: Option<Cutter>) -> usize {
        self.shape.members.push(Member {
            shaft,
            gear: StageGear {
                teeth,
                ..StageGear::default()
            },
            module: self.module,
            thickness_mod: 1.0,
            ring,
            pitch_diameter: Auto::automatic(0.0),
        });
        self.shape.members.len() - 1
    }

    /// Two members in mesh. On an internal mesh the ring goes second, as
    /// [`MeshInput`] has it.
    pub fn mesh(&mut self, a: usize, b: usize) -> &mut Self {
        let (a, b) = if self.shape.members[a].ring.is_some() {
            (b, a)
        } else {
            (a, b)
        };
        self.shape.meshes.push(MeshInput {
            a,
            b,
            sliding_friction: FRICTION.0,
            static_friction: FRICTION.1,
        });
        self
    }

    /// An automatic distance between two axes, at the shipped clearance.
    pub fn distance(&mut self, axes: [usize; 2]) -> &mut Self {
        self.crossed(axes, 0.0)
    }

    /// An automatic distance between two axes at a shaft angle, degrees —
    /// the point contact of crossed shafts where the angle is not nought.
    pub fn crossed(&mut self, axes: [usize; 2], angle: f64) -> &mut Self {
        self.shape.distances.push(Distance {
            axes,
            angle,
            worm: false,
            distance: Auto::automatic(0.0),
            clearance: Auto::fixed(0.02),
            tip_clearance: 0.0,
            tolerance_plus: 0.02,
            tolerance_minus: 0.02,
            axial_clearance: 0.0,
        });
        self
    }

    #[must_use]
    pub fn build(self) -> Shape {
        self.shape
    }
}

/// **A layshaft transmission**: an input shaft and an output shaft on one
/// centreline, a layshaft beside them, and a pair of gears per ratio at the
/// one distance between the two axes. The input's gear drives the layshaft;
/// the engaged pair drives the output shaft; every other pair's output-side
/// gear idles on a shaft of its own, coaxial with the output. `pairs` are
/// `(on the layshaft, on the output side)` in order, `engaged` the index of
/// the one driving; the first pair is the input's constant mesh.
///
/// Shafts: input 1, output 2, layshaft 3, then one idler per disengaged
/// pair — the output before the layshaft so that the convention's "first
/// free port after the input" is the output and not the layshaft.
#[must_use]
pub fn layshaft(input: (u32, u32), pairs: &[(u32, u32)], engaged: usize) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let lay = b.axis();
    let input_shaft = b.shaft(centre);
    let output = b.shaft(centre);
    let layshaft = b.shaft(lay);
    let driving = b.gear(input_shaft, input.0);
    let driven = b.gear(layshaft, input.1);
    b.mesh(driving, driven);
    for (i, &(on_lay, on_out)) in pairs.iter().enumerate() {
        let shaft = if i == engaged {
            output
        } else {
            b.shaft(centre)
        };
        let a = b.gear(layshaft, on_lay);
        let z = b.gear(shaft, on_out);
        b.mesh(a, z);
    }
    b.distance([centre, lay]);
    b.build()
}

/// **A Wolfrom (3K) set**: one planet meshing two rings at one carrier
/// radius, one ring held, the other the output, the carrier the input. No
/// sun; the ratio is `z_r2 / (z_r2 − z_r1)` at one planet, which is what
/// makes a difference of a few teeth a large reduction.
///
/// Shafts: carrier 1, second ring 2, first ring 3, planet 4; members: the
/// planet, the first ring, the second. The first ring listed is the one the
/// shape holds by convention and the output is the next free port after the
/// driven one, so the textbook arrangement is the one it takes with nothing
/// stated.
#[must_use]
pub fn wolfrom(planet: u32, rings: [u32; 2], count: u32) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let carrier = b.shaft(centre);
    let orbit = b.carried_axis(Some(carrier), count);
    let r2 = b.shaft(centre);
    let r1 = b.shaft(centre);
    let p = b.shaft(orbit);
    let planet = b.gear(p, planet);
    let ring1 = b.ring(r1, rings[0]);
    let ring2 = b.ring(r2, rings[1]);
    b.mesh(planet, ring1)
        .mesh(planet, ring2)
        .distance([centre, orbit]);
    b.build()
}

/// **A stepped-planet set**: two gears on each planet shaft, the first
/// meshing the sun and the first ring, the second the second ring — a
/// compound set at one carrier radius, which the second ring's shift
/// closes.
///
/// Shafts: sun 1, second ring 2, carrier 3, first ring 4, planet 5; the
/// first ring is listed first among the rings and so held by convention,
/// and the second ring is the next free port after the sun, so the
/// convention gives the compound reduction; a hold on the carrier or a
/// drive elsewhere gives the others.
#[must_use]
pub fn stepped(sun: u32, planets: [u32; 2], rings: [u32; 2], count: u32) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let s = b.shaft(centre);
    let r2 = b.shaft(centre);
    let carrier = b.shaft(centre);
    let orbit = b.carried_axis(Some(carrier), count);
    let r1 = b.shaft(centre);
    let p = b.shaft(orbit);
    let sun = b.gear(s, sun);
    let p1 = b.gear(p, planets[0]);
    let p2 = b.gear(p, planets[1]);
    let ring1 = b.ring(r1, rings[0]);
    let ring2 = b.ring(r2, rings[1]);
    b.mesh(sun, p1)
        .mesh(p1, ring1)
        .mesh(p2, ring2)
        .distance([centre, orbit]);
    b.build()
}

/// **A planocentric (cycloid-style involute) reducer**: one planet on an
/// eccentric carrier meshing one ring a tooth or two larger; the ring held,
/// the carrier the input, the planet's own rotation the output. The ratio
/// is `−z_p / (z_r − z_p)`.
///
/// Shafts: carrier 1, ring 2, planet 3.
#[must_use]
pub fn planocentric(planet: u32, ring: u32) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let carrier = b.shaft(centre);
    let orbit = b.carried_axis(Some(carrier), 1);
    let r = b.shaft(centre);
    let p = b.shaft(orbit);
    let planet = b.gear(p, planet);
    let ring = b.ring(r, ring);
    b.mesh(planet, ring).distance([centre, orbit]);
    b.build()
}

/// **A set with meshed planets** — the "gutter": a sun meshing planet A,
/// planet A meshing planet B, planet B meshing the ring, the two planets on
/// their own carried axes. It reverses the simple set's sense: with the
/// ring held the carrier turns against the sun, at `1 − z_r/z_s`.
///
/// Shafts: sun 1, carrier 2, ring 3, planet A 4, planet B 5.
#[must_use]
pub fn meshed_planets(sun: u32, planets: [u32; 2], ring: u32, count: u32) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let s = b.shaft(centre);
    let carrier = b.shaft(centre);
    let r = b.shaft(centre);
    let axis_a = b.carried_axis(Some(carrier), count);
    let axis_b = b.carried_axis(Some(carrier), count);
    let pa = b.shaft(axis_a);
    let pb = b.shaft(axis_b);
    let sun = b.gear(s, sun);
    let a = b.gear(pa, planets[0]);
    let z = b.gear(pb, planets[1]);
    let ring = b.ring(r, ring);
    b.mesh(sun, a)
        .mesh(a, z)
        .mesh(z, ring)
        .distance([centre, axis_a])
        .distance([centre, axis_b])
        .distance([axis_a, axis_b]);
    b.build()
}

/// **A Ravigneaux set**: a small sun meshing the long planets, which mesh
/// the ring; a large sun meshing the short planets, which mesh the long
/// ones — two planet axes on one carrier, and the planet–planet mesh a
/// distance between two carried axes.
///
/// Shafts: small sun 1, carrier 2, large sun 3, ring 4, long planet 5,
/// short planet 6 — the carrier the output by convention, whichever sun
/// drives and whichever is held.
#[must_use]
pub fn ravigneaux(suns: [u32; 2], planets: [u32; 2], ring: u32, count: u32) -> Shape {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let s1 = b.shaft(centre);
    let carrier = b.shaft(centre);
    let s2 = b.shaft(centre);
    let r = b.shaft(centre);
    let long_axis = b.carried_axis(Some(carrier), count);
    let short_axis = b.carried_axis(Some(carrier), count);
    let pl = b.shaft(long_axis);
    let ps = b.shaft(short_axis);
    let small_sun = b.gear(s1, suns[0]);
    let large_sun = b.gear(s2, suns[1]);
    let long = b.gear(pl, planets[0]);
    let short = b.gear(ps, planets[1]);
    let ring = b.ring(r, ring);
    b.mesh(small_sun, long)
        .mesh(long, ring)
        .mesh(large_sun, short)
        .mesh(short, long)
        .distance([centre, long_axis])
        .distance([centre, short_axis])
        .distance([long_axis, short_axis]);
    b.build()
}

/// **A worm feeding a spur pair in one stage**: the worm on its own axis at
/// a right angle to a wheel shaft that also carries a pinion, and the gear
/// the pinion drives on a third axis parallel to it. Two distances, one at
/// an angle; a point contact and a line contact in one shape, which is what
/// a crossed distance being a mesh like any other buys.
///
/// Shafts: worm 1, output 2, wheel 3; members: worm, wheel, pinion, gear.
#[must_use]
pub fn worm_and_pair(worm: (u32, u32), pair: (u32, u32)) -> Shape {
    let mut b = Builder::new(1.0);
    let worm_axis = b.axis();
    let wheel_axis = b.axis();
    let out_axis = b.axis();
    let worm_shaft = b.shaft(worm_axis);
    let output = b.shaft(out_axis);
    let wheel_shaft = b.shaft(wheel_axis);
    let w = b.gear(worm_shaft, worm.0);
    let wheel = b.gear(wheel_shaft, worm.1);
    let pinion = b.gear(wheel_shaft, pair.0);
    let gear = b.gear(output, pair.1);
    b.mesh(w, wheel)
        .mesh(pinion, gear)
        .crossed([worm_axis, wheel_axis], 90.0)
        .distance([wheel_axis, out_axis]);
    // Sized as a worm and its wheel — the preset's word, which gives the
    // two their conventional proportions and their names — and a worm's
    // size is its diameter, not a helix: seven millimetres at one start is
    // the worm preset's.
    b.shape.distances[0].worm = true;
    b.shape.members[0].pitch_diameter = Auto::fixed(7.0);
    b.build()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! Each arrangement solves through the shape under its textbook
    //! boundary and gives the ratio the textbook gives, with every distance
    //! closed — the laws the set's tests hold, asked of the arrangements
    //! the set's kind could not name.

    use super::super::shape::{solve_shape, ShapeResult};
    use super::super::{test_library as library, Reversal, StageBoundary, StageLoads};
    use super::*;

    fn solve(shape: &Shape, held: &[Shaft], input: Shaft, output: Shaft) -> ShapeResult {
        let boundary = StageBoundary::holding(shape.shafts.len() + 1, held, input, output);
        solve_shape(
            shape,
            &StageLoads::at(2.0, 3000.0).under(boundary),
            &library(),
            Reversal::default(),
        )
        .unwrap()
    }

    /// Every mesh on every distance runs at that distance, opened by the
    /// clearance its own way.
    fn every_distance_closes(r: &ShapeResult) {
        for d in &r.distances {
            for nominal in &d.nominal {
                assert!(
                    ((d.running - nominal).abs() - d.clearance.abs()).abs() < 1e-9,
                    "nominal {nominal} at running {} with clearance {}",
                    d.running,
                    d.clearance
                );
            }
        }
    }

    /// **A planet's teeth carry one instance's share, on every mesh it is
    /// in** — a planet–planet mesh included. A planet is a free idler: the
    /// tooth load its second mesh carries is its first mesh's less that
    /// mesh's loss, so a planet's reported torque — the larger of its two —
    /// is its sun's tooth load, itself one instance's share, read across the
    /// sun mesh, to within the mesh's efficiency. The mesh between two
    /// planets was divided by what its members see — one path each — rather
    /// than by its own count, and the meshed planets' and the Ravigneaux's
    /// planets carried `N` times their share on that mesh.
    #[test]
    fn a_planet_carries_one_instances_share_on_every_mesh() {
        let cases = [
            (meshed_planets(24, [18, 18], 96, 3), 1, 3),
            (ravigneaux([18, 30], [22, 18], 62, 3), 1, 3),
        ];
        for (shape, input, held) in cases {
            let r = solve(&shape, &[held], input, 2);
            let sun = 0;
            for (i, m) in shape.members.iter().enumerate() {
                let axis = shape.shafts[m.shaft - 1].axis;
                if shape.axes[axis].carried_by.is_none() {
                    continue;
                }
                // The sun's tooth load — one instance's — read across to
                // this planet.
                let share = r.members[sun].cases[0].torque * f64::from(m.gear.teeth)
                    / f64::from(shape.members[sun].gear.teeth);
                let got = r.members[i].cases[0].torque;
                assert!(
                    got <= share * 1.0001 && got >= share * 0.9,
                    "member {i}: {got} against one instance's share {share}"
                );
            }
        }
    }

    /// **A point contact and a line contact in one stage** multiply as any
    /// two meshes do: the ratio is the worm's times the pair's, the
    /// efficiency the product of the two meshes' own, the wheel is rated by
    /// contact alone and the pinion on its shaft by bending as well, and
    /// the stage locks backward exactly where the worm does.
    #[test]
    fn a_worm_and_a_spur_pair_share_one_stage() {
        let shape = worm_and_pair((1, 40), (17, 43));
        let r = solve(&shape, &[], 1, 2);
        assert!(
            (r.ratio.abs() - 40.0 * 43.0 / 17.0).abs() < 1e-9,
            "{}",
            r.ratio
        );
        every_distance_closes(&r);
        assert!(r.meshes[0].point.is_some() && r.meshes[1].line.is_some());
        let product = r.meshes[0].efficiency.forward * r.meshes[1].efficiency.forward;
        assert!(
            (r.efficiency.forward - product).abs() < 1e-9,
            "{} vs {product}",
            r.efficiency.forward
        );
        assert_eq!(
            r.efficiency.backward <= 0.0,
            r.meshes[0].efficiency.backward <= 0.0,
            "the stage locks where its worm does"
        );
        assert!(r.members[1].cases[0].bending_stress.is_none());
        assert!(r.members[2].cases[0].bending_stress.is_some());
        assert!(r.members[1].cases[0].contact_stress > 0.0);
        // The wheel and the pinion turn as one: the same shaft.
        assert_eq!(r.members[1].cases[0].speed, r.members[2].cases[0].speed);
    }

    #[test]
    fn a_layshaft_transmission_is_the_engaged_pair_times_the_constant_mesh() {
        // 17/43 into the layshaft; three ratios, the second engaged.
        let pairs = [(19, 41), (31, 29), (43, 17)];
        for engaged in 0..pairs.len() {
            let shape = layshaft((17, 43), &pairs, engaged);
            let r = solve(&shape, &[], 1, 2);
            let (on_lay, on_out) = pairs[engaged];
            let want = (43.0 / 17.0) * (f64::from(on_out) / f64::from(on_lay));
            assert!(
                (r.ratio - want).abs() < 1e-12,
                "pair {engaged}: {} vs {want}",
                r.ratio
            );
            every_distance_closes(&r);
            // Four meshes at one distance: the input's, and one per ratio.
            assert_eq!(r.distances.len(), 1);
            assert_eq!(r.distances[0].nominal.len(), 4);
            // The idlers turn and carry nothing.
            let idlers: Vec<Shaft> = (4..=shape.shafts.len()).collect();
            for c in &r.cases {
                for &s in &idlers {
                    assert!(c.speeds[s] != 0.0 || c.case == 2, "idler {s} stands still");
                    assert!(c.torques[s].abs() < 1e-12, "idler {s} carries torque");
                }
            }
        }
    }

    #[test]
    fn a_wolfrom_set_reduces_by_the_ring_difference() {
        // Carrier in, first ring held, second ring out: i = z_r2 / (z_r2 − z_r1).
        let shape = wolfrom(18, [60, 61], 3);
        let r = solve(&shape, &[3], 1, 2);
        assert!((r.ratio - 61.0).abs() < 1e-12, "{}", r.ratio);
        every_distance_closes(&r);
        assert_eq!(r.layouts[0].count, 3);
    }

    #[test]
    fn a_stepped_planet_set_carries_the_second_ring_at_the_first_radius() {
        // Sun in, first ring held, second ring out. Willis on the compound
        // planet: ω_r2 − ω_c = (ω_s − ω_c) · (−z_s/z_p1) · (z_p2/z_r2).
        let (zs, zp1, zr1, zp2, zr2) = (24, 18, 60, 17, 59);
        let shape = stepped(zs, [zp1, zp2], [zr1, zr2], 3);
        let r = solve(&shape, &[4], 1, 2);
        let e1 = -f64::from(zs) / f64::from(zp1) * f64::from(zp1) / f64::from(zr1);
        let e2 = -f64::from(zs) / f64::from(zp1) * f64::from(zp2) / f64::from(zr2);
        // With ring 1 held, ω_c/ω_s = e1/(e1 − 1); then ω_r2 = ω_c + e2(ω_s − ω_c).
        let wc = e1 / (e1 - 1.0);
        let wr2 = wc + e2 * (1.0 - wc);
        assert!(
            (r.ratio - 1.0 / wr2).abs() < 1e-9,
            "{} vs {}",
            r.ratio,
            1.0 / wr2
        );
        every_distance_closes(&r);
    }

    #[test]
    fn a_planocentric_reducer_is_minus_the_planet_count_over_the_difference() {
        // Carrier in, ring held, the planet's own turn out.
        for (zp, zr) in [(30, 33), (40, 42)] {
            let shape = planocentric(zp, zr);
            let r = solve(&shape, &[2], 1, 3);
            let want = -f64::from(zr - zp) / f64::from(zp);
            assert!(
                (1.0 / r.ratio - want).abs() < 1e-12,
                "{zp}/{zr}: {} vs {}",
                1.0 / r.ratio,
                want
            );
            every_distance_closes(&r);
            // One planet: no replicated axis, so nothing to lay out.
            assert!(r.layouts.is_empty());
        }
    }

    /// **A distance the tips size**: a planocentric at one tooth of
    /// difference cannot run at the half-module the shifts leave — the
    /// pinion's tip would stand inside the ring's on the far side — so the
    /// distance opens out to where the tips clear by what was asked, the
    /// shifts reach it, and the report says which mesh held it open.
    #[test]
    fn a_four_tooth_planocentric_is_sized_by_its_tips() {
        // The hula stage's second mesh, on its own: 57 in 61, teeth cut to
        // 0.7 of a module, three tenths of far-side gap asked.
        let mut shape = planocentric(57, 61);
        for m in &mut shape.members {
            m.gear.addendum = 0.7;
            m.gear.dedendum = 1.0;
        }
        shape.members[1].ring = Some(Cutter {
            teeth: 20,
            addendum: 1.0,
            ..Cutter::default()
        });
        shape.distances[0].tip_clearance = 0.3;
        let r = solve(&shape, &[2], 1, 3);
        let d = &r.distances[0];
        assert_eq!(d.sized_by, Some(0), "the one mesh sized it");
        assert!(
            d.running > 2.0,
            "opened past the two modules the counts leave: {}",
            d.running
        );
        every_distance_closes(&r);
        // The far-side gap is what was asked, to the solver's tolerance, and
        // the tips do not cross.
        let (pinion, ring) = (&r.members[0], &r.members[1]);
        let ring_tip = crate::ring::Ring::cut_by(&ring.params, &shape.members[1].ring.unwrap()).ra;
        let far = ring_tip - crate::tooth::Tooth::new(pinion.params).ra + d.running;
        assert!((far - 0.3).abs() < 1e-6 || far > 0.3, "far-side gap {far}");
        assert_eq!(r.meshes[0].tips.map(|t| t.tip_interference), Some(false));
        assert!((1.0 / r.ratio + 4.0 / 57.0).abs() < 1e-12);
        // ...and a distance the designer states is not sized: it leaves what
        // it leaves, and says so through the mesh's own room.
        shape.distances[0].distance = Auto::fixed(2.0);
        let r = solve(&shape, &[2], 1, 3);
        assert_eq!(r.distances[0].sized_by, None);
    }

    /// **The shape's sizing is the hula stage's**, on the mesh the hula's
    /// harness reports held open by its tips: `gear-cli hula 18 0.2` runs
    /// 19/18 at a crank offset of 0.726026 mm (0.746026 at zero backlash),
    /// its tip margin at nought and its far-side gap 0.2779 with 0.2 asked.
    /// A planocentric of that pair, cut to the hula's proportions, is sized
    /// to the same offset by the shape — two solvers, one bound.
    #[test]
    fn the_shape_sizes_a_distance_where_the_hula_stage_does() {
        let hula = super::super::HulaStage::default();
        let mut shape = planocentric(18, 19);
        for (m, g) in shape
            .members
            .iter_mut()
            .zip([&hula.gears[1], &hula.gears[0]])
        {
            m.gear = StageGear {
                teeth: m.gear.teeth,
                ..g.clone()
            };
        }
        shape.members[1].ring = Some(Cutter {
            teeth: 14,
            ..hula.cutter[0]
        });
        shape.distances[0].tip_clearance = 0.2;
        let r = solve(&shape, &[2], 1, 3);
        let d = &r.distances[0];
        assert_eq!(d.sized_by, Some(0));
        assert!(
            (d.running - 0.726_026).abs() < 2e-5,
            "the hula's crank offset: {}",
            d.running
        );
        assert!(
            (r.members[1].profile_shift - 0.4519).abs() < 2e-4,
            "the ring's shift"
        );
    }

    #[test]
    fn meshed_planets_reverse_the_simple_set() {
        // Sun in, ring held, carrier out: 1 − z_r/z_s, negative.
        let (zs, zr) = (24, 96);
        let shape = meshed_planets(zs, [18, 18], zr, 3);
        let r = solve(&shape, &[3], 1, 2);
        assert!(
            (r.ratio - (1.0 - f64::from(zr) / f64::from(zs))).abs() < 1e-12,
            "{}",
            r.ratio
        );
        every_distance_closes(&r);
        assert_eq!(r.layouts.len(), 2);
    }

    #[test]
    fn a_ravigneaux_set_gives_both_textbook_reductions() {
        // The ring the long planets' sun and count make ideal, `z_s1 + 2 z_pl`;
        // the same counts `tools/train_kinematics.py` lays out.
        let (zs1, zs2, zpl, zps, zr) = (18, 30, 22, 18, 62);
        let shape = ravigneaux([zs1, zs2], [zpl, zps], zr, 3);
        // Small sun in, carrier held, ring out: a simple reversing pair through
        // the long planet, −z_r/z_s1.
        let r = solve(&shape, &[2], 1, 4);
        assert!(
            (r.ratio + f64::from(zr) / f64::from(zs1)).abs() < 1e-12,
            "{}",
            r.ratio
        );
        every_distance_closes(&r);
        // Large sun in, carrier held, ring out: through both planets, +z_r/z_s2.
        let r = solve(&shape, &[2], 3, 4);
        assert!(
            (r.ratio - f64::from(zr) / f64::from(zs2)).abs() < 1e-12,
            "{}",
            r.ratio
        );
        every_distance_closes(&r);
        // Small sun in, ring held, carrier out: 1 + z_r/z_s1.
        let r = solve(&shape, &[4], 1, 2);
        assert!(
            (r.ratio - (1.0 + f64::from(zr) / f64::from(zs1))).abs() < 1e-12,
            "{}",
            r.ratio
        );
        every_distance_closes(&r);
        assert_eq!(r.distances.len(), 3);
        // Two replicated axes, each laid out: the short planets stand nearer
        // the centre than the long ones and have a clearance of their own.
        assert_eq!(r.layouts.len(), 2);
        assert_eq!((r.layouts[0].axis, r.layouts[1].axis), (1, 2));
        assert!(r.layouts.iter().all(|l| l.count == 3 && l.clearance > 0.0));
    }
}
