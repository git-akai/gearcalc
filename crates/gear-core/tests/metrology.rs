//! Milestone 2's gate: every formula that has a textbook special case is checked
//! against it, and over-pins is checked against the geometry itself.
//!
//! The second kind matters more. A formula agreeing with a remembered table
//! value proves only that two sources agree; a pin proved tangent to the
//! generated flank proves the formula describes the gear we actually made. The
//! design document records a case where a remembered "textbook" figure was the
//! thing that was wrong.

#![allow(clippy::unwrap_used)]

use gear_core::mesh::{Mesh, MeshKind};
use gear_core::metrology::{
    best_span, cutter_tip_width, over_pins, pin_geometry, span_over_teeth, MeasurementError,
    PinCount,
};
use gear_core::{inv, Gear, GearParams};

fn grid() -> Vec<GearParams> {
    let mut v = Vec::new();
    for teeth in [9u32, 12, 17, 20, 21, 31, 44, 63] {
        for xi in [-3i32, 0, 2, 5] {
            for pressure_angle in [14.5_f64, 20.0, 25.0] {
                for helix_angle in [0.0_f64, 15.0, -25.0] {
                    v.push(GearParams {
                        teeth,
                        profile_shift: f64::from(xi) * 0.1,
                        pressure_angle,
                        helix_angle,
                        ..Default::default()
                    });
                }
            }
        }
    }
    v
}

// --------------------------------------------------------------------- //
//  span measurement
// --------------------------------------------------------------------- //

/// The general derivation must reproduce the standard formula
/// `W_k = m cos αₙ [π(k−0.5) + z inv α_t] + 2 x m sin αₙ` exactly.
#[test]
fn span_reduces_to_the_textbook_formula() {
    let mut worst = 0.0_f64;
    for p in grid() {
        let g = Gear::new(p);
        let z = f64::from(p.teeth);
        let an = p.pressure_angle.to_radians();
        for k in 2..=5u32 {
            let got = span_over_teeth(&g, k).nominal;
            // The textbook form is stated for spur gears; the helical case is
            // covered by `consecutive_spans_differ_by_one_base_pitch` instead.
            if p.helix_angle != 0.0 {
                continue;
            }
            let want = p.module
                * an.cos()
                * (std::f64::consts::PI * (f64::from(k) - 0.5) + z * inv(g.alpha_t))
                + 2.0 * p.profile_shift * p.module * an.sin();
            let d = (got - want).abs();
            worst = worst.max(d);
            assert!(
                d < 1e-12,
                "z={} x={} k={k}: {got} vs {want}",
                p.teeth,
                p.profile_shift
            );
        }
    }
    println!("worst span disagreement with the textbook form: {worst:.3e} mm");
}

/// A span is a chord along the base tangent, so consecutive spans must differ by
/// exactly one **normal base pitch**. That is an independent property of the
/// formula, not a restatement of it.
#[test]
fn consecutive_spans_differ_by_one_base_pitch() {
    for p in grid() {
        let g = Gear::new(p);
        let pbn = std::f64::consts::PI * p.module * p.pressure_angle.to_radians().cos();
        for k in 2..=6u32 {
            let d = span_over_teeth(&g, k + 1).nominal - span_over_teeth(&g, k).nominal;
            assert!(
                (d - pbn).abs() < 1e-12,
                "z={} k={k}: step {d}, base pitch {pbn}",
                p.teeth
            );
        }
    }
}

#[test]
fn best_span_lands_on_the_usable_flank() {
    for p in grid() {
        let g = Gear::new(p);
        match best_span(&g) {
            Ok(s) => {
                assert!(s.teeth_spanned >= 1 && s.teeth_spanned <= p.teeth);
                assert!(
                    s.contact_radius >= g.r_j && s.contact_radius <= g.ra,
                    "z={} contact {} outside [{}, {}]",
                    p.teeth,
                    s.contact_radius,
                    g.r_j,
                    g.ra
                );
                assert!(s.nominal > 0.0 && s.limits.is_none());
            }
            Err(e) => assert_eq!(e, MeasurementError::NoValidSpan),
        }
    }
}

// --------------------------------------------------------------------- //
//  over pins — the independent check
// --------------------------------------------------------------------- //

/// Distance from a point to the generated involute flank, by direct search.
///
/// Deliberately independent of the over-pins derivation: it samples the flank
/// the profile generator actually produces.
fn distance_to_flank(g: &Gear, px: f64, py: f64) -> f64 {
    let n = 400_000;
    let (lo, hi) = (0.0_f64, g.u_tip.max(0.1));
    let mut best = f64::INFINITY;
    for i in 0..=n {
        #[allow(clippy::cast_precision_loss)]
        let u = lo + (hi - lo) * (i as f64 / n as f64);
        let (r, th) = g.involute_at(u);
        let d = f64::hypot(px - r * th.sin(), py - r * th.cos());
        if d < best {
            best = d;
        }
    }
    best
}

/// **The gate.** The pin centre the formula returns must sit exactly one pin
/// radius from the real flank. If it does, the pin is genuinely tangent and the
/// measurement is real.
#[test]
fn pin_is_genuinely_tangent_to_the_generated_flank() {
    let cases = [
        (20u32, 0.0, 20.0, 1.728),
        (21, 0.3, 20.0, 1.75),
        (13, -0.2, 20.0, 1.6),
        (31, 0.0, 25.0, 1.8),
        (44, 0.4, 14.5, 1.9),
    ];
    let mut worst = 0.0_f64;
    for (teeth, x, pressure_angle, dp) in cases {
        let p = GearParams {
            teeth,
            profile_shift: x,
            pressure_angle,
            ..Default::default()
        };
        let g = Gear::new(p);
        let (r_m, _) = pin_geometry(&g, dp).unwrap();

        // The pin centre lies on the tooth-space centreline, pi/z from the
        // tooth centre.
        let half = std::f64::consts::PI / f64::from(teeth);
        let d = distance_to_flank(&g, r_m * half.sin(), r_m * half.cos());

        let err = (d - dp / 2.0).abs();
        assert!(
            err < 1e-8,
            "z={teeth} x={x} dp={dp}: pin centre is {d:.10} from the flank, expected {:.10}",
            dp / 2.0
        );
        worst = worst.max(err);
    }
    println!("worst pin tangency error: {worst:.3e} mm");
}

/// Two-pin and three-pin geometry are related: for an odd tooth count the third
/// pin sits exactly opposite the bisector of the adjacent pair, so the three-pin
/// value is the mean of the pin-centre circle's diameter and its chord — a
/// relation that holds only if both formulas are right.
#[test]
fn two_and_three_pin_agree_on_the_pin_centre_circle() {
    for teeth in [12u32, 13, 20, 21, 31, 44] {
        let g = Gear::new(GearParams {
            teeth,
            ..Default::default()
        });
        let dp = 1.75;
        let (Ok(two), Ok(three)) = (
            over_pins(&g, dp, PinCount::Two),
            over_pins(&g, dp, PinCount::Three),
        ) else {
            continue;
        };
        assert!(
            (two.pin_centre_radius - three.pin_centre_radius).abs() < 1e-15,
            "pin centre radius must not depend on how many pins are used"
        );
        let r_m = two.pin_centre_radius;
        let pi = std::f64::consts::PI;
        let z = f64::from(teeth);
        let expect_three = if teeth.is_multiple_of(2) {
            2.0 * r_m * (pi / z).cos() + dp
        } else {
            r_m * (1.0 + (pi / z).cos()) + dp
        };
        assert!((three.nominal - expect_three).abs() < 1e-12);
        // three pins always measures less than two across the same circle
        assert!(three.nominal < two.nominal, "z={teeth}");
    }
}

#[test]
fn unusable_pins_are_rejected_rather_than_measured() {
    let g = Gear::new(GearParams {
        teeth: 17,
        ..Default::default()
    });
    // Far too small: falls between the flanks into the root.
    assert!(matches!(
        over_pins(&g, 0.05, PinCount::Two),
        Err(MeasurementError::PinTooSmall)
    ));
    // Far too large: contact runs past the tip.
    assert!(over_pins(&g, 5.0, PinCount::Two).is_err());
    // A sane pin works.
    assert!(over_pins(&g, 1.75, PinCount::Two).is_ok());
}

// --------------------------------------------------------------------- //
//  backlash, against a direct computation
// --------------------------------------------------------------------- //

/// The exact backlash law, checked against tooth thicknesses computed directly
/// at the operating pitch circles. This is the independent derivation, not a
/// rearrangement: it never mentions `α_w`.
#[test]
fn backlash_matches_a_direct_tooth_thickness_computation() {
    let mut worst = 0.0_f64;
    for (z1, z2, x1, x2, k1, an, beta) in [
        (17u32, 31u32, 0.2, 0.1, 1.0, 20.0, 0.0),
        (13, 47, -0.1, 0.4, 1.15, 25.0, 15.0),
        (9, 9, 0.5, 0.5, 0.8, 20.0, 30.0),
        (23, 23, 0.0, 0.0, 1.0, 14.5, 0.0),
    ] {
        let mk = |z: u32, x: f64, k: f64, hel: f64| {
            Gear::new(GearParams {
                teeth: z,
                profile_shift: x,
                thickness_mod: k,
                pressure_angle: an,
                helix_angle: hel,
                ..Default::default()
            })
        };
        let g1 = mk(z1, x1, k1, beta);
        let g2 = mk(z2, x2, 2.0 - k1, -beta);
        let m = Mesh::new(&g1, &g2, MeshKind::External).unwrap();

        for da in [-0.05_f64, 0.0, 0.02, 0.1, 0.4] {
            let a = m.a_w + da;
            let formula = m.backlash(a).unwrap();

            // direct: circular pitch at the operating circle, minus both tooth
            // thicknesses there
            let ap = m.pressure_angle_at(a).unwrap();
            let sz = f64::from(z1) + f64::from(z2);
            let mut direct = 2.0 * std::f64::consts::PI * a / sz;
            for (g, z) in [(&g1, f64::from(z1)), (&g2, f64::from(z2))] {
                let psi = g.st / (2.0 * g.r);
                let r_op = a * z / sz;
                direct -= 2.0 * r_op * (psi + inv(g.alpha_t) - inv(ap));
            }

            let d = (formula - direct).abs();
            worst = worst.max(d);
            assert!(
                d < 1e-12,
                "z={z1}/{z2} da={da}: formula {formula:.15}, direct {direct:.15}"
            );
        }
    }
    println!("worst backlash disagreement with the direct computation: {worst:.3e} mm");
}

// --------------------------------------------------------------------- //

/// The cutter tip width is a normal-plane quantity, so it must not move when
/// only the helix angle changes. That is the property the spec calls out.
#[test]
fn cutter_tip_width_is_independent_of_helix_angle() {
    for teeth in [12u32, 17, 40] {
        for x in [-0.2_f64, 0.0, 0.4] {
            let at_zero = cutter_tip_width(&Gear::new(GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            }));
            for helix_angle in [10.0_f64, 25.0, -35.0] {
                let w = cutter_tip_width(&Gear::new(GearParams {
                    teeth,
                    profile_shift: x,
                    helix_angle,
                    ..Default::default()
                }));
                assert!(
                    (w - at_zero).abs() < 1e-12,
                    "z={teeth} x={x} beta={helix_angle}: {w} vs {at_zero}"
                );
            }
        }
    }
}
