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
use gear_core::{inv, GearParams, Tooth};

mod common;
use common::{Grid, MODULES, PRESSURE_ANGLES, THICKNESS_MODS};

/// Tooth counts a measurement is actually taken on — enough teeth for a span to
/// exist, up to where the pin leaves the flank. The module and the thickness
/// modification are turned here where they were not before: both move a span
/// and an over-pins figure directly, which is the whole subject of this file.
fn grid() -> Vec<GearParams> {
    Grid::new()
        .teeth(&[9, 12, 17, 20, 21, 31, 44, 63])
        .shifts(&[-0.3, 0.0, 0.2, 0.5])
        .pressure_angle(PRESSURE_ANGLES)
        .helix_angle(&[0.0, 15.0, -25.0])
        .module(&[MODULES[0], MODULES[3]])
        .thickness_mod(&[THICKNESS_MODS[0], THICKNESS_MODS[1]])
        .build()
}

// --------------------------------------------------------------------- //
//  span measurement
// --------------------------------------------------------------------- //

/// The general derivation must reproduce the standard formula
/// `W_k = m cos αₙ [π(k−0.5) + z inv α_t] + 2 x m sin αₙ` exactly.
///
/// # ...and `x` there is the **thickness** shift, which is a stronger claim
///
/// The textbook form is written for an unmodified rack, and this test only ever
/// ran on one, because `thickness_mod` was fixed at 1 in every grid the crate
/// had. Turning that axis makes the identity fail — correctly, and informatively:
/// a span is a *thickness* measurement, so it takes `x + x_s` where a radial
/// quantity takes `x` alone (docs/reference.md#tooth-thickness-and-its-equivalent-shift). Substituting it is not a repair to keep the
/// test green; it is the assertion the test should always have made, since it
/// checks that a thickness modification enters the span **exactly** as an
/// equivalent profile shift rather than merely closely.
#[test]
fn span_reduces_to_the_textbook_formula() {
    let mut worst = 0.0_f64;
    for p in grid() {
        let g = Tooth::new(p);
        let z = f64::from(p.teeth);
        let an = p.pressure_angle.to_radians();
        let x_thick = p.profile_shift + p.thickness_shift();
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
                + 2.0 * x_thick * p.module * an.sin();
            // Relative, because the module axis spans 0.5 to 12 mm and an
            // absolute tolerance would be a different claim at each end.
            let d = (got - want).abs() / want.abs().max(f64::MIN_POSITIVE);
            worst = worst.max(d);
            assert!(
                d < 1e-14,
                "z={} x={} k_thick={} k={k}: {got} vs {want}",
                p.teeth,
                p.profile_shift,
                p.thickness_mod
            );
        }
    }
    println!("worst relative span disagreement with the textbook form: {worst:.3e}");
}

/// A span is a chord along the base tangent, so consecutive spans must differ by
/// exactly one **normal base pitch**. That is an independent property of the
/// formula, not a restatement of it.
#[test]
fn consecutive_spans_differ_by_one_base_pitch() {
    for p in grid() {
        let g = Tooth::new(p);
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
        let g = Tooth::new(p);
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
fn distance_to_flank(g: &Tooth, px: f64, py: f64) -> f64 {
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
        let g = Tooth::new(p);
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
        let g = Tooth::new(GearParams {
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
    let g = Tooth::new(GearParams {
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
            Tooth::new(GearParams {
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
            let at_zero = cutter_tip_width(&Tooth::new(GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            }));
            for helix_angle in [10.0_f64, 25.0, -35.0] {
                let w = cutter_tip_width(&Tooth::new(GearParams {
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

// ------------------------------------------------------ internal gears ---

/// **The pin is genuinely tangent to a ring's generated flank**, measured the
/// same way the external check measures it: the minimum distance from the pin
/// centre to a densely sampled flank must be the pin's radius.
///
/// This shares no algebra with `between_pins` — it samples `Ring::involute_at`
/// and takes a minimum — so it tests the derivation rather than restating it. It
/// is the check that would catch the sign of the `d_p` term being wrong, which is
/// the one thing that distinguishes the internal relation from the external one.
#[test]
fn a_pin_is_genuinely_tangent_to_a_rings_generated_flank() {
    use gear_core::metrology::between_pins;
    use gear_core::ring::{Cutter, Ring};

    let cases = [
        (43u32, 0.0, 20.0, 1.7),
        (60, 0.0, 20.0, 1.8),
        (60, 0.3, 20.0, 1.8),
        (43, -0.2, 20.0, 1.6),
        (90, 0.0, 25.0, 1.9),
    ];
    let mut worst = 0.0_f64;
    for (teeth, x, pressure_angle, dp) in cases {
        let params = GearParams {
            teeth,
            profile_shift: x,
            pressure_angle,
            ..Default::default()
        };
        let ring = Ring::cut_by(&params, &Cutter::default());
        let m =
            between_pins(&ring, dp).unwrap_or_else(|e| panic!("z={teeth} x={x} dp={dp}: {e:?}"));

        // The pin centre lies on the tooth-space centreline, pi/z from the tooth
        // centre — the same place it sits on an external gear.
        let half = std::f64::consts::PI / f64::from(teeth);
        let (px, py) = (
            m.pin_centre_radius * half.sin(),
            m.pin_centre_radius * half.cos(),
        );

        // Minimum distance to the ring's own flank, sampled.
        const N: usize = 40_000;
        let mut best = f64::INFINITY;
        for i in 0..=N {
            let u = ring.u_tip + (ring.u_j - ring.u_tip) * (i as f64 / N as f64);
            let (r, th) = ring.involute_at(u);
            best = best.min(f64::hypot(r * th.sin() - px, r * th.cos() - py));
        }
        let err = (best - dp / 2.0).abs();
        assert!(
            err < 1e-6,
            "z={teeth} x={x} dp={dp}: pin centre is {best:.10} from the flank, expected {:.10}",
            dp / 2.0
        );
        worst = worst.max(err);
    }
    println!("worst internal pin tangency error: {worst:.3e} mm");
}

/// **A larger pin sits *deeper* in a ring**, which is the opposite of an external
/// gear — and the cheapest check that the sign of the `d_p` term is the right way
/// round.
///
/// A ring's tooth widens outward, so its space narrows outward: a big pin cannot
/// get as far out. On an external gear the space narrows inward and a big pin
/// rides higher. Getting this backwards still produces a plausible number.
#[test]
fn a_bigger_pin_sits_deeper_in_a_ring_and_higher_in_a_gear() {
    use gear_core::metrology::between_pins;
    use gear_core::ring::{Cutter, Ring};

    let ring = Ring::cut_by(
        &GearParams {
            teeth: 60,
            ..Default::default()
        },
        &Cutter::default(),
    );
    let mut previous = f64::INFINITY;
    for dp in [1.4, 1.6, 1.8, 2.0] {
        let m = between_pins(&ring, dp).unwrap();
        assert!(
            m.pin_centre_radius < previous,
            "dp={dp}: pin centre radius {} should fall",
            m.pin_centre_radius
        );
        // ...and the contact is *above* the centre, since the pin wedges outward.
        assert!(m.contact_radius > m.pin_centre_radius);
        previous = m.pin_centre_radius;
    }

    let g = Tooth::new(GearParams {
        teeth: 60,
        ..Default::default()
    });
    let mut previous = f64::NEG_INFINITY;
    for dp in [1.4, 1.6, 1.8, 2.0] {
        let (r_m, contact) = pin_geometry(&g, dp).unwrap();
        assert!(r_m > previous, "dp={dp}: external pin centre should rise");
        assert!(contact < r_m, "an external pin touches below its centre");
        previous = r_m;
    }
}

/// The pin diameter **subtracts** for a ring and **adds** for an external gear,
/// because one is measured between inner surfaces and the other across outer
/// ones. Stated as a test because it is the kind of sign that survives review.
#[test]
fn the_pin_diameter_subtracts_inside_and_adds_outside() {
    use gear_core::metrology::{between_pins, over_pins, PinCount};
    use gear_core::ring::{Cutter, Ring};

    let dp = 1.8;
    let ring = Ring::cut_by(
        &GearParams {
            teeth: 60,
            ..Default::default()
        },
        &Cutter::default(),
    );
    let m = between_pins(&ring, dp).unwrap();
    assert!((m.nominal - (2.0 * m.pin_centre_radius - dp)).abs() < 1e-12);
    assert!(m.nominal < 2.0 * m.pin_centre_radius);

    let g = Tooth::new(GearParams {
        teeth: 60,
        ..Default::default()
    });
    let o = over_pins(&g, dp, PinCount::Two).unwrap();
    assert!((o.nominal - (2.0 * o.pin_centre_radius + dp)).abs() < 1e-12);
    assert!(o.nominal > 2.0 * o.pin_centre_radius);
}

/// An odd tooth count leaves no space diametrically opposite, so the measurement
/// takes the same `cos(π/2z)` chord an external gear does — and comes out
/// smaller than the even-count value it would otherwise have.
#[test]
fn an_odd_toothed_ring_measures_across_a_chord() {
    use gear_core::metrology::between_pins;
    use gear_core::ring::{Cutter, Ring};

    let of = |teeth: u32| {
        let ring = Ring::cut_by(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        );
        between_pins(&ring, 1.8).map(|m| (m.nominal, m.pin_centre_radius))
    };
    let (odd, r_odd) = of(61).unwrap();
    let half = std::f64::consts::PI / (2.0 * 61.0);
    assert!((odd - (2.0 * r_odd * half.cos() - 1.8)).abs() < 1e-12);
    // The chord is shorter than the diameter, so an odd ring measures less than
    // the same pin circle would across a full diameter.
    assert!(odd < 2.0 * r_odd - 1.8);
}

/// **The same arithmetic failure means opposite things inside and out.**
///
/// A negative involute target says the pin centre would have to sit inside the
/// base circle. On an external gear that is a pin too *small* to bridge the
/// flanks; inside a ring, whose space narrows outward, it is a pin too *large* to
/// find a seat. Reporting the external diagnosis for a ring sends the designer
/// to make the pin bigger, which is the wrong way.
#[test]
fn a_pin_that_cannot_seat_is_diagnosed_by_kind() {
    use gear_core::metrology::{between_pins, MeasurementError};
    use gear_core::ring::{Cutter, Ring};

    // A small ring with a fat pin: no seat.
    let ring = Ring::cut_by(
        &GearParams {
            teeth: 9,
            ..Default::default()
        },
        &Cutter::default(),
    );
    assert_eq!(
        between_pins(&ring, 1.75).unwrap_err(),
        MeasurementError::PinTooLarge,
        "a ring's space narrows outward, so this is a pin too large"
    );

    // The external counterpart of the same arithmetic is a pin too small.
    let g = Tooth::new(GearParams {
        teeth: 9,
        ..Default::default()
    });
    assert_eq!(
        pin_geometry(&g, 0.2).unwrap_err(),
        MeasurementError::PinTooSmall
    );

    // And a sensible ring with a sensible pin measures.
    let ring = Ring::cut_by(
        &GearParams {
            teeth: 60,
            ..Default::default()
        },
        &Cutter::default(),
    );
    assert!(between_pins(&ring, 1.75).is_ok());
}
