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

use gear_core::{inv, Gear, GearParams};

/// A grid spanning the awkward regions: tiny tooth counts, both signs of shift,
/// sharp and rounded racks, both helix hands.
fn grid() -> Vec<GearParams> {
    let mut v = Vec::new();
    for teeth in [3u32, 5, 7, 9, 12, 17, 23, 40, 80] {
        for xi in [-5i32, -3, 0, 3, 6, 9] {
            for pressure_angle in [14.5_f64, 20.0, 25.0] {
                for helix_angle in [0.0_f64, 15.0, -30.0] {
                    for root_radius in [0.0_f64, 0.2, 0.38] {
                        v.push(GearParams {
                            teeth,
                            profile_shift: f64::from(xi) * 0.1,
                            pressure_angle,
                            helix_angle,
                            root_radius,
                            ..Default::default()
                        });
                    }
                }
            }
        }
    }
    v
}

#[test]
fn flank_and_fillet_meet_exactly() {
    let mut worst = 0.0_f64;
    for p in grid() {
        let g = Gear::new(p);
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
        let g = Gear::new(p);
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
        let g = Gear::new(p);
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
        let g = Gear::new(p);
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
        let g = Gear::new(p);
        let pts = g.profile(400);
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
                let a = Gear::new(modified);
                let b = Gear::new(equivalent);
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

/// Thickness modification is a *thickness* quantity. It must leave every radial
/// dimension untouched — that separation is the rule the implementation rests on.
#[test]
fn thickness_modification_leaves_radial_dimensions_alone() {
    let base = Gear::new(GearParams::default());
    for k in [0.7_f64, 1.0, 1.3] {
        let g = Gear::new(GearParams {
            thickness_mod: k,
            ..Default::default()
        });
        assert!(
            (g.r - base.r).abs() < 1e-15,
            "pitch radius moved with k={k}"
        );
        assert!(
            (g.rb - base.rb).abs() < 1e-15,
            "base radius moved with k={k}"
        );
        assert!(
            (g.ra - base.ra).abs() < 1e-15,
            "tip radius moved with k={k}"
        );
        assert!(
            (g.rf - base.rf).abs() < 1e-15,
            "root radius moved with k={k}"
        );
    }
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
    let g = Gear::new(GearParams {
        pressure_angle: 0.0,
        ..Default::default()
    });
    assert!(g.clamps.any() && g.rb > 0.0);

    // dedendum deeper than the pitch radius
    let g = Gear::new(GearParams {
        teeth: 4,
        dedendum: 40.0,
        ..Default::default()
    });
    assert!(
        g.clamps.any() && g.rf > 0.0,
        "root radius must stay positive"
    );

    // fillet far larger than the tooth space
    let g = Gear::new(GearParams {
        root_radius: 20.0,
        ..Default::default()
    });
    assert!(g.clamps.any() && g.rho <= g.bd);

    assert!(g.ra.is_finite() && g.rf.is_finite() && g.rho.is_finite());
}
