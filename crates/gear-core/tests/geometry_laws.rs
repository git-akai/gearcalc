//! Invariants the generated profile must satisfy, independent of how it is built.
//!
//! These are cheap and run over a wide grid, so they are the first thing to fail
//! when something breaks.
//!
//! One invariant is deliberately *absent*: `theta` is **not** monotone along the
//! profile. Undercut gears are legitimately re-entrant — the fillet curls back
//! under the flank — and asserting otherwise produced 161 false failures in the
//! prior work. The correct statement is monotone **radius** (the profile is a
//! graph over `r`, so it cannot self-intersect) plus `0 <= theta <= π/z`.

#![allow(clippy::unwrap_used)]

use gear_core::gear::Gear;
use gear_core::note::key;
use gear_core::{inv, inv_from_roll, GearParams, Tooth};

mod common;
use common::{
    Grid, AWKWARD_SHIFTS, AWKWARD_TEETH, HELIX_ANGLES, MODULES, PRESSURE_ANGLES, ROOT_RADII,
};

/// The awkward regions: tiny tooth counts, both signs of shift, sharp and
/// rounded racks, both helix hands. One grid, shared — see `common`.
fn grid() -> Vec<GearParams> {
    Grid::new()
        .teeth(AWKWARD_TEETH)
        .shifts(AWKWARD_SHIFTS)
        .pressure_angle(PRESSURE_ANGLES)
        .helix_angle(HELIX_ANGLES)
        .root_radius(ROOT_RADII)
        .build()
}

#[test]
fn flank_and_fillet_meet_exactly() {
    let mut worst = 0.0_f64;
    for p in grid() {
        let g = Tooth::new(p);
        if g.severed {
            continue; // no flank exists
        }
        let (r1, t1) = g.involute_at(g.u_j);
        let (r2, t2) = g.trochoid_at(g.s_j);
        let gap = f64::hypot(r1 * t1.sin() - r2 * t2.sin(), r1 * t1.cos() - r2 * t2.cos());
        assert!(
            gap < 1e-9,
            "junction gap {gap:.3e} mm at z={} x={} rho={}",
            p.teeth,
            p.profile_shift,
            p.root_radius
        );
        worst = worst.max(gap);
    }
    println!("worst junction gap: {worst:.3e} mm");
}

#[test]
fn radius_is_monotone_and_theta_stays_in_the_half_pitch() {
    for p in grid() {
        let g = Tooth::new(p);
        let (r, th) = g.half_profile(600);
        for w in r.windows(2) {
            assert!(
                w[1] <= w[0] + 1e-9,
                "radius increased along the profile at z={} x={}",
                p.teeth,
                p.profile_shift
            );
        }
        for &t in &th {
            assert!(
                (-1e-12..=g.half_pitch + 1e-9).contains(&t),
                "theta {t} outside [0, pi/z] at z={} x={}",
                p.teeth,
                p.profile_shift
            );
        }
    }
}

#[test]
fn involute_tooth_thickness_law_holds_on_the_flank() {
    let mut worst = 0.0_f64;
    for p in grid() {
        let g = Tooth::new(p);
        if g.severed {
            continue;
        }
        let (lo, hi) = (g.r_j * 1.001, g.ra * 0.999);
        if hi <= lo {
            continue;
        }
        for i in 0..=8 {
            let rr = lo + (hi - lo) * f64::from(i) / 8.0;
            let a_r = (g.rb / rr).min(1.0).acos();
            // s(r)/r = s_p/R + 2(inv a_t - inv a_r)
            let want = g.st / g.r + 2.0 * (inv(g.alpha_t) - inv(a_r));
            let u = (((rr / g.rb).powi(2) - 1.0).max(0.0)).sqrt();
            let got = 2.0 * g.involute_at(u).1;
            worst = worst.max((got - want).abs());
            assert!(
                (got - want).abs() < 1e-12,
                "thickness law off by {:.3e} at r={rr} z={}",
                (got - want).abs(),
                p.teeth
            );
        }
    }
    println!("worst thickness-law residual: {worst:.3e} rad");
}

/// The fillet fit cap is algebraically equivalent to `ac/R <= pi/z`, so
/// satisfying it *guarantees* a non-negative root arc. If the cap is ever
/// rewritten, this is what catches getting it wrong.
#[test]
fn fillet_cap_guarantees_a_nonnegative_root_arc() {
    for p in grid() {
        let g = Tooth::new(p);
        assert!(
            g.theta0 <= g.half_pitch + 1e-9,
            "root arc is negative ({} > {}) at z={} x={} rho={}",
            g.theta0,
            g.half_pitch,
            p.teeth,
            p.profile_shift,
            p.root_radius
        );
    }
}

#[test]
fn profile_is_closed_and_has_one_period_per_tooth() {
    for p in [
        GearParams::default(),
        GearParams {
            teeth: 8,
            ..Default::default()
        },
        GearParams {
            teeth: 3,
            profile_shift: -0.5,
            ..Default::default()
        },
    ] {
        let g = Tooth::new(p);
        let pts = Gear::new(p).profile(400);
        let first = pts.first().unwrap();
        let last = pts.last().unwrap();
        assert!((first[0] - last[0]).abs() < 1e-15 && (first[1] - last[1]).abs() < 1e-15);
        // every point lies between the root and tip radii
        for q in &pts {
            let r = f64::hypot(q[0], q[1]);
            assert!(
                r >= g.rf - 1e-9 && r <= g.ra + 1e-9,
                "point at r={r} outside [rf, ra]"
            );
        }
    }
}

// --------------------------------------------------------------------- //
//  tooth thickness modification
// --------------------------------------------------------------------- //

/// The whole justification for implementing thickness modification as a scalar:
/// it is exactly an extra thickness-only profile shift.
#[test]
fn thickness_modification_equals_an_equivalent_profile_shift() {
    for k in [0.6_f64, 0.85, 1.0, 1.2, 1.45] {
        for pressure_angle in [14.5_f64, 20.0, 25.0, 30.0] {
            for profile_shift in [-0.5_f64, 0.0, 0.7] {
                let modified = GearParams {
                    pressure_angle,
                    profile_shift,
                    thickness_mod: k,
                    ..Default::default()
                };
                let x_s = modified.thickness_shift();
                let equivalent = GearParams {
                    pressure_angle,
                    profile_shift: profile_shift + x_s,
                    thickness_mod: 1.0,
                    ..Default::default()
                };
                let a = Tooth::new(modified);
                let b = Tooth::new(equivalent);
                assert!(
                    (a.st - b.st).abs() < 1e-14,
                    "k={k} a={pressure_angle}: st {} vs {}",
                    a.st,
                    b.st
                );
            }
        }
    }
}

/// **A thickness modification changes a thickness, and nothing radial.**
///
/// The rule the whole crate is built on: *radial* quantities take plain `x` and
/// *thickness* quantities take `x + x_s` — `docs/reference.md#tooth-thickness-and-its-equivalent-shift`
/// argues it, and everything from the root circle to the cutter's plunge rests
/// on the separation.
///
/// # One law, two kinds, and they are asserted at the same standard now
///
/// A ring says the same thing in `ring::tests::a_thickness_modification_moves_no_radius_on_a_ring`,
/// and it says it far better: three tooth counts by three shifts by four values
/// of `k`, the cutter's plunge as well as the radii, and a carve-out for the one
/// place the rule genuinely bends — a space that closes on itself, where the
/// root is truncated by the profile rather than placed by the tool.
///
/// This half asserted the same law on **one gear**: the default tooth count, the
/// default shift, one module, no helix, three values of `k`. Everything the law
/// could plausibly interact with was at its default, which is the fault
/// `tests/common` exists for — *an axis nobody turns is an axis nobody tests* —
/// met in the weaker of two statements of one rule.
///
/// The two are not merged, and deliberately: a ring needs a cutter to be cut by
/// and has a limit an external gear does not. What they now share is the
/// standard.
#[test]
fn thickness_modification_leaves_radial_dimensions_alone() {
    let mut checked = 0u32;
    for base_params in grid() {
        let base = Tooth::new(base_params);
        for k in [0.6_f64, 0.85, 1.0, 1.15, 1.4] {
            let g = Tooth::new(GearParams {
                thickness_mod: k,
                ..base_params
            });
            // **The pitch and base circles are exact, always.** They are
            // `m·z/2` and `r·cos α_t` — the tooth form does not enter them, so
            // no guard can move them and there is no case to carve out.
            for (name, got, want) in [
                ("pitch radius", g.r, base.r),
                ("base radius", g.rb, base.rb),
            ] {
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "z={} x={} a={} b={}: {name} moved from {want} to {got} at k={k}",
                    base_params.teeth,
                    base_params.profile_shift,
                    base_params.pressure_angle,
                    base_params.helix_angle
                );
            }

            // **The tip and root are exact too, unless the profile truncated
            // them.** A tooth thin enough is *severed* — undercut so deep the
            // flank is cut away — or comes to a point before its addendum, and
            // then the radius is set by the shape that is left rather than by
            // the number asked for. How much is left is a function of the tooth
            // thickness, so of `k`; that is a different guard with its own note,
            // and it is the exact mirror of the one a ring has, where a space
            // closing on itself truncates the root.
            //
            // The sweep found it: three teeth at 14.5° and a shift of −0.5 are
            // severed, and the tip sits at 0.173 mm at `k = 0.6` against
            // 0.208 mm at 1.0. One law, two kinds, one place each where it bends
            // and the same reason both times.
            let truncated = |t: &Tooth| {
                t.severed
                    || t.clamps
                        .notes
                        .iter()
                        .any(|n| n.is(key::CLAMP_TIP_CAPPED_POINTED))
            };
            if truncated(&g) || truncated(&base) {
                assert!(
                    g.ra > 0.0 && g.rf > 0.0 && g.ra.is_finite() && g.rf.is_finite(),
                    "z={} k={k}: a truncated tooth has no radii: ra={} rf={}",
                    base_params.teeth,
                    g.ra,
                    g.rf
                );
                checked += 1;
                continue;
            }
            for (name, got, want) in [
                ("tip radius", g.ra, base.ra),
                ("root radius", g.rf, base.rf),
            ] {
                assert_eq!(
                    got.to_bits(),
                    want.to_bits(),
                    "z={} x={} a={} b={}: {name} moved from {want} to {got} at k={k}",
                    base_params.teeth,
                    base_params.profile_shift,
                    base_params.pressure_angle,
                    base_params.helix_angle
                );
            }
            // ...while the thing it *is* about did move, or this is asserting
            // that a control does nothing.
            if (k - 1.0).abs() > 1e-12 {
                assert!(
                    (g.st - base.st).abs() > 1e-9,
                    "z={} k={k}: the tooth thickness did not move either",
                    base_params.teeth
                );
            }
            checked += 1;
        }
    }
    assert!(checked > 500, "only {checked} gears were swept");
}

/// A meshing pair must satisfy `k1 + k2 = 2`. The consequence relied on
/// downstream is that their equivalent shifts cancel, so thickness modification
/// cannot move the centre distance.
#[test]
fn paired_thickness_modifications_cancel() {
    for k in [0.5_f64, 0.9, 1.0, 1.35, 1.8] {
        for pressure_angle in [14.5_f64, 20.0, 25.0] {
            let a = GearParams {
                pressure_angle,
                thickness_mod: k,
                ..Default::default()
            };
            let b = GearParams {
                pressure_angle,
                thickness_mod: 2.0 - k,
                ..Default::default()
            };
            let sum = a.thickness_shift() + b.thickness_shift();
            assert!(
                sum.abs() < 1e-15,
                "k={k}: equivalent shifts sum to {sum:.3e}, not 0"
            );
        }
    }
}

/// Degenerate input is clamped, never rejected — and every clamp is recorded.
#[test]
fn degenerate_input_is_clamped_and_reported() {
    // pressure angle of zero: no involute exists
    let g = Tooth::new(GearParams {
        pressure_angle: 0.0,
        ..Default::default()
    });
    assert!(g.clamps.any() && g.rb > 0.0);

    // dedendum deeper than the pitch radius
    let g = Tooth::new(GearParams {
        teeth: 4,
        dedendum: 40.0,
        ..Default::default()
    });
    assert!(
        g.clamps.any() && g.rf > 0.0,
        "root radius must stay positive"
    );

    // fillet far larger than the tooth space
    let g = Tooth::new(GearParams {
        root_radius: 20.0,
        ..Default::default()
    });
    assert!(g.clamps.any() && g.rho <= g.bd);

    assert!(g.ra.is_finite() && g.rf.is_finite() && g.rho.is_finite());
}

/// **A shifted internal pair has exactly zero backlash at the centre distance
/// the mesh computes** — measured off the two real profiles, not from the
/// relation that produced the centre distance.
///
/// This is the check that decides what a profile shift and a thickness
/// modification *mean* on a ring. `Mesh::new` flips gear 2's `x` and `x_s`
/// together, which is consistent only if both describe the ring's **space** —
/// the place the pinion's tooth goes — rather than its tooth. Read the other way
/// round the same pair comes out 0.63 mm from zero at `k = 1.2`, so this is not
/// a convention that could be chosen either way.
///
/// It is also the reason the internal pair invariant is `k₁ = k₂` where an
/// external one needs `k₁ + k₂ = 2`: equal thickness modifications are what
/// leave the centre distance at its reference value.
#[test]
fn an_internal_pair_has_zero_backlash_at_the_centre_distance_the_mesh_gives() {
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::ring::{Cutter, Ring};

    let params = |teeth: u32, x: f64, k: f64| GearParams {
        teeth,
        profile_shift: x,
        thickness_mod: k,
        ..Default::default()
    };

    for (z1, z2, x1, x2, k1, k2) in [
        (17u32, 51u32, 0.0, 0.0, 1.0, 1.0),
        (17, 51, 0.3, 0.3, 1.0, 1.0), // equal shifts: reference centres
        (17, 51, 0.0, 0.4, 1.0, 1.0),
        (17, 51, -0.2, 0.25, 1.0, 1.0),
        (17, 51, 0.0, 0.0, 1.15, 1.15), // equal k: reference centres
        (17, 51, 0.0, 0.0, 1.2, 0.9),
        (20, 60, 0.15, -0.1, 1.05, 1.1),
        (25, 41, 0.0, 0.3, 1.0, 1.0),
    ] {
        let pinion = Tooth::new(params(z1, x1, k1));
        let ring = Ring::cut_by(&params(z2, x2, k2), &Cutter::default());
        // The mesh reads the ring through the same `Tooth` arithmetic, because a
        // ring's shift and thickness enter its space exactly as they enter an
        // external gear's tooth.
        let ring_as_gear = Tooth::new(params(z2, x2, k2));
        let mesh = Mesh::new(&pinion, &ring_as_gear, MeshKind::Internal).unwrap();

        let (r1, r2) = mesh.operating_radii();
        let r2 = r2.abs();
        assert!(
            (r2 - r1 - mesh.a_w).abs() < 1e-12,
            "operating radii must differ by the centre distance for an internal pair"
        );

        // Pinion tooth against ring space, both at their own operating circle.
        let u1 = (((r1 / pinion.rb).powi(2) - 1.0).max(0.0)).sqrt();
        let tooth = 2.0 * r1 * (pinion.psi_b - inv_from_roll(u1));
        let space = ring.space_width_at(r2);
        assert!(
            (space - tooth).abs() < 1e-11,
            "z={z1}/{z2} x={x1}/{x2} k={k1}/{k2}: space {space} vs tooth {tooth}, \
             backlash {} mm at a_w = {}",
            space - tooth,
            mesh.a_w
        );

        // Equal shifts and equal thickness modifications each leave the centre
        // distance exactly where the reference geometry put it.
        if (x1 - x2).abs() < 1e-15 && (k1 - k2).abs() < 1e-15 {
            assert!(
                (mesh.a_w - mesh.a_ref).abs() < 1e-12,
                "z={z1}/{z2}: equal shifts must not move the centre distance"
            );
        }
    }
}

/// **What is drawn is a curve, and it is the involute of the ring's own base
/// circle.**
///
/// Both halves of that sentence are checked from the sampled polyline alone,
/// sharing no code with what produced it:
///
/// 1. *A curve.* The points must arrive at the density that was asked for.
///    Nothing else here noticed when a 600-point request came back with seven,
///    because every surviving point was still exactly on the profile — the
///    outline was not wrong, it was **absent**, and assertions about where
///    points lie cannot see that. The statement that does is about the gaps
///    between them.
///
/// 2. *An involute.* An involute is traced by a string unwound from the base
///    circle, so at radius `r` the taut string — the curve's **normal** — has
///    length `√(r² − r_b²)`, and the tangent there stands exactly that far from
///    the centre. Each short chord of the sampled flank is that tangent, so
///    measuring the centre's distance to it and comparing against the roll
///    length of its own midpoint tests the defining property, using nothing but
///    the drawn points and the base radius. A flank flattened into a couple of
///    straight segments is out by tens of micrometres against a bound of one.
///
///    (The first version of this asserted the distance was `r_b` itself, on the
///    strength of "an involute's chord is tangent to its base circle". It is the
///    *normal* that is tangent to the base circle; the tangent is perpendicular
///    to it. The failure was the test's, and the geometry it accused was right.)
///
/// The cutter that cannot cut is included deliberately: a tool whose corner
/// rounds overlap generates no fillet, and the flank then runs to the root
/// circle. That is a different shape, but it is still a *drawn* one, and it was
/// the case that failed.
#[test]
fn a_rings_drawn_profile_is_dense_and_lies_on_its_base_circles_involute() {
    use gear_core::ring::{Cutter, Ring};

    const PER_TOOTH: usize = 600;

    for teeth in [40u32, 60, 90] {
        for tip_round in [0.0_f64, 0.2, 0.38] {
            // A sharp-cornered shaper undercuts a ring — its trochoid crosses
            // below the junction — so on that one the sampled points below the
            // junction are fillet rather than flank. The density half of this
            // test still covers it; the involute half would need exactly the
            // section bookkeeping it is meant not to trust.
            let involute_law = tip_round > 0.0;
            let cutter = Cutter {
                teeth: 20,
                addendum: 1.25,
                tip_round,
            };
            let ring = Ring::cut_by(
                &GearParams {
                    teeth,
                    ..Default::default()
                },
                &cutter,
            );
            let what = format!("z={teeth} tip_round={tip_round}");
            let pts = ring.profile(PER_TOOTH);

            assert!(
                pts.iter().flatten().all(|v| v.is_finite()),
                "{what}: the profile contains a non-finite coordinate"
            );

            // 1. Density. The closing point makes it one more than a whole
            //    number of teeth; a tenth either way is generous, and the
            //    failure this exists for was a factor of forty.
            #[allow(clippy::cast_precision_loss)]
            let per_tooth = (pts.len() - 1) as f64 / f64::from(teeth);
            assert!(
                (0.9 * PER_TOOTH as f64..=1.1 * PER_TOOTH as f64).contains(&per_tooth),
                "{what}: asked for {PER_TOOTH} points a tooth and got {per_tooth:.1}"
            );

            // ...spread evenly, not bunched into one section with the rest bare.
            // The bound is the mean spacing times a factor, so it states a
            // property of the sampling rather than a length anyone chose.
            let pitch_arc = 2.0 * std::f64::consts::PI * ring.r / f64::from(teeth);
            let allowed = 20.0 * pitch_arc / per_tooth;
            let worst_gap = pts
                .windows(2)
                .map(|w| f64::hypot(w[1][0] - w[0][0], w[1][1] - w[0][1]))
                .fold(0.0_f64, f64::max);
            assert!(
                worst_gap < allowed,
                "{what}: a {worst_gap:.4} mm gap between neighbouring points, \
                 against {allowed:.4} mm allowed"
            );

            // 2. The involute law, on the band that is a generated flank: above
            //    the tip, below wherever the flank hands over, and clear of the
            //    generation limit where the cutter's own involute has run out.
            let flank_low = ring.ra.max(ring.generation_limit()) + 0.05;
            let flank_high = ring.involute_at(ring.u_j).0 - 0.05;
            let mut checked = 0;
            for w in pts.windows(2).filter(|_| involute_law) {
                let radii = [f64::hypot(w[0][0], w[0][1]), f64::hypot(w[1][0], w[1][1])];
                if !radii.iter().all(|r| (flank_low..=flank_high).contains(r)) {
                    continue;
                }
                // Distance from the centre to the line through the two points.
                let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
                let len = f64::hypot(dx, dy);
                if len < 1e-12 {
                    continue;
                }
                let distance = (w[0][0] * dy - w[0][1] * dx).abs() / len;
                let mid = 0.5 * (radii[0] + radii[1]);
                let roll = (mid * mid - ring.rb * ring.rb).sqrt();
                assert!(
                    (distance - roll).abs() < 1e-3,
                    "{what}: a flank chord at r={mid:.4} stands {distance:.4} mm off \
                     the centre, against the {roll:.4} mm its roll length demands"
                );
                checked += 1;
            }
            assert!(
                !involute_law || checked > 50,
                "{what}: only {checked} flank chords were testable — the band is \
                 too thin to be evidence"
            );
        }
    }
}

/// **Every length scales with the module and every angle does not.**
///
/// A gear's geometry is homogeneous of degree one in `m`: the module is the only
/// dimensional input, so doubling it must double every radius and leave every
/// angle exactly where it was. That is an exact law, checkable without knowing a
/// single answer — the strongest kind this project has — and until this test
/// nothing asserted it. Nothing could: `module` was `1.0` in every profile-law
/// case in the repository, so the axis that carries the law was never turned.
///
/// # What is deliberately not asserted, and why it is worth saying
///
/// The **roll parameters** `u_j` and `u_tip` are excluded, and not because they
/// are wrong. `u` is recovered from a radius by `√((r/r_b)² − 1)`, which near
/// `u → 0` amplifies a relative error by `1/u²` — the involute is tangent to its
/// own base circle there, so the radius carries almost no information about the
/// roll. On an undercut tooth `u_j ≈ 1e-3` and a 3e-14 residual in `r_j` becomes
/// 3e-8 in `u_j`: conditioning, not a defect, and it goes the same way whichever
/// route is taken to `u`.
///
/// So the law belongs on the lengths and the angles, which are what the geometry
/// is stated in, and a roll parameter is the wrong instrument for it. Recorded
/// here rather than discovered later as an unreproducible flake.
#[test]
fn every_length_scales_with_the_module_and_every_angle_is_invariant() {
    let mut worst_length = 0.0_f64;
    let mut worst_angle = 0.0_f64;
    let mut where_length = String::new();

    for base in Grid::new()
        .teeth(&[5, 9, 17, 23, 40])
        .shifts(&[-0.4, 0.0, 0.3])
        .pressure_angle(PRESSURE_ANGLES)
        .helix_angle(&[0.0, 20.0])
        .root_radius(ROOT_RADII)
        .thickness_mod(&[0.8, 1.0, 1.2])
        .build()
    {
        let unit = Tooth::new(GearParams {
            module: 1.0,
            ..base
        });
        for &m in MODULES {
            let g = Tooth::new(GearParams { module: m, ..base });

            // Lengths: exactly `m` times the unit gear's.
            for (name, got, want) in [
                ("r", g.r, unit.r),
                ("rb", g.rb, unit.rb),
                ("rf", g.rf, unit.rf),
                ("ra", g.ra, unit.ra),
                ("rho", g.rho, unit.rho),
                ("st", g.st, unit.st),
                ("bd", g.bd, unit.bd),
                ("bc", g.bc, unit.bc),
                ("ac", g.ac, unit.ac),
                ("r_j", g.r_j, unit.r_j),
                ("s_j", g.s_j, unit.s_j),
                ("l", g.l, unit.l),
            ] {
                if !got.is_finite() || !want.is_finite() {
                    continue;
                }
                let target = want * m;
                let rel = (got - target).abs() / target.abs().max(f64::MIN_POSITIVE);
                if rel > worst_length {
                    worst_length = rel;
                    where_length = format!("{name} at m={m}, {base:?}");
                }
            }

            // Angles: the same number, whatever the module.
            for (_name, got, want) in [
                ("psi_p", g.psi_p, unit.psi_p),
                ("psi_b", g.psi_b, unit.psi_b),
                ("theta_a", g.theta_a, unit.theta_a),
                ("theta0", g.theta0, unit.theta0),
                ("half_pitch", g.half_pitch, unit.half_pitch),
                ("alpha_t", g.alpha_t, unit.alpha_t),
            ] {
                if got.is_finite() && want.is_finite() {
                    worst_angle = worst_angle.max((got - want).abs());
                }
            }

            // ...and the discrete verdicts are facts about the shape, so they
            // cannot depend on how large it is drawn.
            assert_eq!(
                g.undercut, unit.undercut,
                "undercut moved with m={m}: {base:?}"
            );
            assert_eq!(
                g.severed, unit.severed,
                "severed moved with m={m}: {base:?}"
            );
        }
    }

    assert!(
        worst_length < 1e-12,
        "a length is not homogeneous in the module: {worst_length:e} relative at {where_length}"
    );
    assert!(
        worst_angle < 1e-14,
        "an angle moved with the module by {worst_angle:e} rad"
    );
}

/// **A tooth that is not undercut is never severed**, which is what lets the
/// severing scan be skipped for one.
///
/// The reason is structural rather than observed, and the sweep is here to
/// corroborate it rather than to establish it. Severing is the two fillets
/// meeting across the tooth space, and `Rack::wanted_by` caps the tool's tip
/// round at `FILLET_FRACTION_OF_MAX` — 95 % — of the round that would exactly
/// fill that space. The fillets are therefore held apart by the five per cent
/// the cap is short of one, and the only way the trochoid reaches the
/// centreline is if the flank has been consumed, which is undercut.
///
/// What matters is that the margin is a *fraction of the space*: it narrows
/// with the tooth without ever closing. So this sweeps toward the corner where
/// the space is smallest — a tenth of the nominal pressure angle, a twentieth
/// of the nominal thickness, a root round larger than the rack would carry —
/// and asks only that it stay positive. An earlier version of this test asserted
/// a floor of a thousandth of a radian; widening the sweep by one parameter
/// walked the margin to 6e-4 and would have failed it, on geometry that is
/// perfectly sound. A bound has to be the one the reasoning gives, or it is a
/// record of where the sweep happened to stop.
///
/// The scan is written out here rather than called, because inside the crate it
/// no longer runs for these teeth — which is the point.
/// **A tooth reported unsevered does not cross its own centreline.**
///
/// Severing is the trochoid reaching the tooth's centreline, and the crate finds
/// it by **solving** for the minimum of `θ` — a root of `dθ/ds`, which is exact
/// — and testing its sign. This is the independent check on that: a dense scan
/// over undercut teeth, asserting nothing the crate called unsevered dips below
/// zero anywhere.
///
/// It is worth having because the solve replaced a 2000-point scan, and a scan
/// is what this is. The count that scan used could not be justified — measured,
/// the negative excursion narrows to a twentieth of a thousandth of the interval
/// as a tooth approaches severing, where 2000 samples resolve half a thousandth,
/// and no larger count closes that gap because the window shrinks to zero. What
/// a scan can still do is confirm the solve from outside, which is what it does
/// here.
///
/// It is the same shape as `only_an_undercut_tooth_can_be_severed` above and
/// asks the opposite question of the other half of the population.
///
/// Only unsevered teeth can be re-scanned: severing overwrites `s_j` with the
/// crossing it found and sets `u_j` to NaN, so the original interval is gone.
/// That is the case that matters — a *missed* crossing is the fault, and a
/// found one has already been bracketed by a guaranteed solve.
#[test]
fn a_tooth_called_unsevered_is_not_severed_at_a_finer_scan() {
    const DENSE: usize = 40_000;
    let mut undercut = 0u32;
    let mut worst = f64::INFINITY;
    let mut at = String::new();

    for z in [3u32, 5, 7, 9, 12, 17, 24] {
        for k in -40..=20 {
            let x = f64::from(k) * 0.05;
            for &(alpha, ded, kt, rho, beta, m) in &[
                (20.0, 1.25, 1.0, 0.38, 0.0, 1.0),
                (14.5, 1.25, 1.0, 0.38, 0.0, 1.0),
                (25.0, 1.25, 1.0, 0.0, 0.0, 1.0),
                (20.0, 1.6, 1.0, 0.9, 0.0, 1.0),
                (20.0, 1.0, 1.7, 0.2, 0.0, 1.0),
                (20.0, 1.25, 1.0, 0.38, 30.0, 1.0),
                (20.0, 1.25, 1.0, 0.9, -45.0, 8.0),
                (25.0, 1.0, 1.7, 0.2, 15.0, 0.4),
            ] {
                let g = Tooth::new(GearParams {
                    module: m,
                    teeth: z,
                    profile_shift: x,
                    pressure_angle: alpha,
                    helix_angle: beta,
                    dedendum: ded,
                    thickness_mod: kt,
                    root_radius: rho,
                    ..Default::default()
                });
                if !g.undercut || g.severed {
                    continue;
                }
                undercut += 1;
                // The interval the crate scans, sampled a hundred times finer.
                let mut min_th = f64::INFINITY;
                for i in 0..DENSE {
                    #[allow(clippy::cast_precision_loss)]
                    let t = i as f64 / (DENSE - 1) as f64;
                    let th = g.trochoid_at(g.s_j + t * (0.0 - g.s_j)).1;
                    min_th = min_th.min(th);
                }
                if min_th < worst {
                    worst = min_th;
                    at = format!(
                        "z={z} x={x:+.2} a={alpha} ded={ded} k={kt} rho={rho} b={beta} m={m}"
                    );
                }
                assert!(
                    min_th > 0.0,
                    "{at}: called unsevered, but a {DENSE}-point scan finds \
                     theta = {min_th} on the trochoid — the 2000-point scan \
                     missed a crossing"
                );
            }
        }
    }

    assert!(
        undercut > 200,
        "only {undercut} undercut-but-unsevered teeth in the sweep; \
         this test needs a population to mean anything"
    );
    // Reported rather than asserted against a floor, for the reason the test
    // above gives at length: the margin is a fraction of the space, so it
    // narrows with the tooth and a threshold would record this sweep instead.
    println!("{undercut} undercut, unsevered teeth; closest approach {worst:.3e} rad at {at}");
}

#[test]
fn only_an_undercut_tooth_can_be_severed() {
    let mut worst = f64::INFINITY;
    let mut count = 0u32;
    let mut at = String::new();
    for z in [5u32, 7, 9, 12, 17, 24, 31, 40, 60, 90, 140, 200] {
        for k in -40..=40 {
            let x = f64::from(k) * 0.05;
            // Everything that moves the trochoid or narrows the space: the
            // pressure angle sets the fillet's slope, the dedendum how deep the
            // tool reaches, the thickness modification how much room is left,
            // the root round how much of it the fillet claims — and the helix
            // and module, which the transverse section carries.
            for &(alpha, ded, kt, rho, beta, m) in &[
                (20.0, 1.25, 1.0, 0.38, 0.0, 1.0),
                (14.5, 1.25, 1.0, 0.38, 0.0, 1.0),
                (25.0, 1.25, 1.0, 0.0, 0.0, 1.0),
                (20.0, 1.6, 1.0, 0.9, 0.0, 1.0),
                (20.0, 1.0, 1.7, 0.2, 0.0, 1.0),
                (20.0, 1.25, 1.0, 0.38, 30.0, 1.0),
                (20.0, 1.25, 1.0, 0.9, -45.0, 8.0),
                (25.0, 1.0, 1.7, 0.2, 15.0, 0.4),
                // The corner: a thin tooth in a low-angle rack with a round
                // bigger than the space would carry, where the cap binds hardest.
                (10.0, 1.25, 0.05, 1.4, 0.0, 1.0),
                (12.0, 2.0, 0.1, 2.0, 0.0, 1.0),
                (14.5, 2.5, 0.2, 1.4, 0.0, 1.0),
            ] {
                let t = Tooth::new(GearParams {
                    teeth: z,
                    profile_shift: x,
                    pressure_angle: alpha,
                    dedendum: ded,
                    thickness_mod: kt,
                    root_radius: rho,
                    helix_angle: beta,
                    module: m,
                    ..GearParams::default()
                });
                if t.undercut {
                    continue;
                }
                count += 1;
                assert!(
                    !t.severed,
                    "z{z} x{x} a{alpha} d{ded} k{kt} rho{rho} b{beta} m{m}: severed without undercut"
                );
                let n = 2000usize;
                let mut min_th = f64::INFINITY;
                for i in 0..n {
                    #[allow(clippy::cast_precision_loss)]
                    let f = i as f64 / (n - 1) as f64;
                    min_th = min_th.min(t.trochoid_at(t.s_j + f * (0.0 - t.s_j)).1);
                }
                if min_th < worst {
                    worst = min_th;
                    at = format!("z{z} x{x:.2} a{alpha} d{ded} k{kt} rho{rho} b{beta} m{m}");
                }
            }
        }
    }
    assert!(
        count > 6_000,
        "the sweep has to be wide to corroborate anything: {count}"
    );
    assert!(
        worst > 0.0,
        "the trochoid reached the centreline at {at} ({worst:e}), so the cap does not \
         hold the fillets apart and the guard is wrong"
    );
}

/// **The trochoid's `θ` turns once, which is what lets severing be solved.**
///
/// `Tooth::sever` finds the least `θ` as a root of `dθ/ds` rather than as the
/// smallest of a sample. That is only valid where the slope changes sign at most
/// once on the interval — otherwise a bracketed solve lands on whichever turning
/// point it happens to bracket, and the minimum it reports is not the minimum.
///
/// So the shape is asserted rather than assumed, over the population that
/// reaches this code: every undercut tooth in the sweep, counting sign changes
/// of a finite difference. Zero changes is a monotone `θ` and is admissible too
/// — the code falls back to the endpoints when the slope does not bracket.
///
/// If this ever fails, `sever` needs the bracket narrowing to the branch it
/// means, not a finer sample count.
#[test]
fn the_trochoid_turns_once() {
    let mut worst = 0usize;
    let mut at = String::new();
    let mut cases = 0u32;
    for z in [3u32, 5, 7, 9, 12, 17, 24, 40] {
        for k in -60..=20 {
            let x = f64::from(k) * 0.05;
            for &(alpha, ded, kt, rho, beta) in &[
                (20.0, 1.25, 1.0, 0.38, 0.0),
                (14.5, 1.25, 1.0, 0.38, 0.0),
                (25.0, 1.25, 1.0, 0.0, 0.0),
                (20.0, 1.6, 1.0, 0.9, 0.0),
                (20.0, 1.0, 1.7, 0.2, 0.0),
                (20.0, 1.25, 1.0, 0.38, 30.0),
            ] {
                let g = Tooth::new(GearParams {
                    teeth: z,
                    profile_shift: x,
                    pressure_angle: alpha,
                    dedendum: ded,
                    thickness_mod: kt,
                    root_radius: rho,
                    helix_angle: beta,
                    ..Default::default()
                });
                if !g.undercut || g.severed {
                    continue;
                }
                cases += 1;
                const N: u32 = 4000;
                let mut prev: Option<bool> = None;
                let mut changes = 0usize;
                for i in 0..N {
                    let t = |j: u32| g.s_j + (f64::from(j) / f64::from(N)) * (0.0 - g.s_j);
                    let rising = g.trochoid_at(t(i + 1)).1 > g.trochoid_at(t(i)).1;
                    if prev.is_some_and(|p| p != rising) {
                        changes += 1;
                    }
                    prev = Some(rising);
                }
                if changes > worst {
                    worst = changes;
                    at = format!("z={z} x={x:+.2} a={alpha} ded={ded} k={kt} rho={rho} b={beta}");
                }
                assert!(
                    changes <= 1,
                    "{at}: dtheta/ds changes sign {changes} times — theta is not \
                     unimodal here, so solving for its minimum is not valid"
                );
            }
        }
    }
    assert!(
        cases > 500,
        "only {cases} undercut teeth; too few to mean anything"
    );
    println!("{cases} undercut teeth; most turns in theta: {worst} at {at}");
}
