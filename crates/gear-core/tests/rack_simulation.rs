//! The definitive check: simulate the cutter and bound the profile from both
//! sides. This is milestone 1's gate.
//!
//! Penetration alone is not sufficient — an arbitrarily undersized profile
//! passes it trivially — so deviation is checked too. Together they pin the
//! profile down uniquely.
//!
//! The prior work could afford 44 cases of this in Python. Here it is thousands.

#![allow(clippy::unwrap_used)]

use gear_core::verify::{check_cut, fillet_envelope_error, sdf_matches_polyline};
use gear_core::{Gear, GearParams};

/// No gear point may lie inside the cutter, at any phase. Exactly zero, not
/// "small" — any positive value is material the tool would have removed.
const PENETRATION_LIMIT: f64 = 1e-12;

/// Every generated point must be touched by the cutter at its closest approach.
/// The residual here is phase-discretisation, not geometry error; see
/// `phase_resolution_has_converged`.
const DEVIATION_LIMIT: f64 = 1e-3;

mod common;
use common::{Grid, PRESSURE_ANGLES};

/// The cutter check's grid. Kept at its own tooth counts and shifts — this is
/// the expensive test in the suite and the product is 1 080 cases — but drawn
/// through the shared builder so the axes are named and the cost is visible.
fn grid() -> Vec<GearParams> {
    Grid::new()
        .teeth(&[3, 5, 8, 9, 11, 13, 17, 23, 31, 47])
        .shifts(&[-0.5, -0.2, 0.0, 0.2, 0.5, 0.9])
        .pressure_angle(PRESSURE_ANGLES)
        .helix_angle(&[0.0, 20.0])
        .root_radius(&[0.0, 0.25, 0.38])
        .build()
}

#[test]
fn profile_is_bounded_from_both_sides_by_the_cutter() {
    let cases = grid();
    assert!(
        cases.len() >= 1000,
        "the gate calls for 1000+ cases, got {}",
        cases.len()
    );

    let mut worst_pen = 0.0_f64;
    let mut worst_dev = 0.0_f64;
    let mut worst_dev_case = String::new();

    for p in &cases {
        let g = Gear::new(*p);
        let rep = check_cut(&g, 150);
        assert!(
            rep.penetration <= PENETRATION_LIMIT,
            "penetration {:.3e} mm at z={} x={} a={} b={} rho={}",
            rep.penetration,
            p.teeth,
            p.profile_shift,
            p.pressure_angle,
            p.helix_angle,
            p.root_radius
        );
        if rep.deviation > worst_dev {
            worst_dev = rep.deviation;
            worst_dev_case = format!(
                "z={} x={} a={} b={} rho={}",
                p.teeth, p.profile_shift, p.pressure_angle, p.helix_angle, p.root_radius
            );
        }
        worst_pen = worst_pen.max(rep.penetration);
    }

    println!(
        "{} cases: worst penetration {worst_pen:.3e} mm, worst deviation {worst_dev:.3e} mm ({worst_dev_case})",
        cases.len()
    );
    assert!(
        worst_dev < DEVIATION_LIMIT,
        "worst deviation {worst_dev:.3e} mm at {worst_dev_case}"
    );
}

/// Independent of the envelope derivation entirely: every fillet point must lie
/// exactly `rho` from the path traced by the cutter's tip-round centre.
///
/// This is what proved the fillet correct while the rack simulation was still
/// misreporting, so it earns its place even though it overlaps `check_cut`.
#[test]
fn fillet_is_the_tip_round_envelope() {
    let mut worst = 0.0_f64;
    for p in grid().into_iter().step_by(7) {
        let g = Gear::new(p);
        let e = fillet_envelope_error(&g, 150, 20_000);
        assert!(
            e < 1e-6,
            "fillet is {e:.3e} mm off the tip-round centre path at z={} x={}",
            p.teeth,
            p.profile_shift
        );
        worst = worst.max(e);
    }
    println!("worst fillet envelope error: {worst:.3e} mm");
}

/// The analytic cutter distance is the thing `check_cut` trusts, so it is
/// cross-checked against an independently constructed polyline of the same
/// tooth. Agreement is limited by the polyline's own chord error, which is
/// exactly why the polyline is no longer used for the real check.
#[test]
fn analytic_cutter_distance_matches_an_independent_polyline() {
    let mut worst = 0.0_f64;
    for p in grid().into_iter().step_by(11) {
        let g = Gear::new(p);
        worst = worst.max(sdf_matches_polyline(&g, 400, 4000));
    }
    println!("worst analytic-vs-polyline disagreement: {worst:.3e} mm");
    assert!(
        worst < 1e-5,
        "analytic distance disagrees with the polyline by {worst:.3e} mm"
    );
}

/// Justifies `MAX_ROTATION_STEP` by measurement rather than by choice.
///
/// The reported deviation is a phase-discretisation artefact: halving the step
/// must not materially change it, or the sampling is too coarse to trust. This
/// is the convergence test the design called for in place of a tuned constant.
#[test]
fn phase_resolution_has_converged() {
    // Points per half-profile is the other resolution knob; vary it too, since
    // a finer profile samples the flank where the cutter is closest.
    for p in [
        GearParams {
            teeth: 31,
            profile_shift: -0.5,
            pressure_angle: 14.5,
            root_radius: 0.0,
            ..Default::default()
        },
        GearParams {
            teeth: 8,
            profile_shift: 0.0,
            ..Default::default()
        },
        GearParams {
            teeth: 3,
            profile_shift: 0.5,
            ..Default::default()
        },
    ] {
        let g = Gear::new(p);
        let coarse = check_cut(&g, 100);
        let fine = check_cut(&g, 300);
        // Both must satisfy the bound, and refining must not reveal a much
        // larger deviation hiding between samples.
        assert!(coarse.deviation < DEVIATION_LIMIT && fine.deviation < DEVIATION_LIMIT);
        assert!(
            fine.deviation < coarse.deviation.max(1e-5) * 3.0,
            "deviation is resolution-sensitive at z={}: {:.3e} -> {:.3e}",
            p.teeth,
            coarse.deviation,
            fine.deviation
        );
    }
}

/// The travel range is derived from the geometry, not guessed. Extending it must
/// not change the answer — if it does, the sweep was missing part of the
/// generating motion, which is the single error that hid two real failure modes
/// in the original suite.
#[test]
fn travel_padding_is_sufficient() {
    for p in [
        GearParams {
            teeth: 8,
            ..Default::default()
        },
        GearParams {
            teeth: 17,
            profile_shift: 0.3,
            ..Default::default()
        },
        GearParams {
            teeth: 5,
            profile_shift: -0.2,
            root_radius: 0.0,
            ..Default::default()
        },
    ] {
        let g = Gear::new(p);
        let (lo, hi) = gear_core::verify::rack_travel_range(&g);
        // The generating features must sit strictly inside the padded range.
        assert!(
            g.s_j - g.ac > lo && -g.ac < hi,
            "fillet generation outside the swept range"
        );
        assert!(
            0.0 > lo && g.r * g.half_pitch < hi,
            "root arc outside the swept range"
        );
        let span_pitches = (hi - lo) / (std::f64::consts::PI * g.mt);
        assert!(
            span_pitches > 2.0,
            "swept range is only {span_pitches:.2} pitches: the fillet is cut ~1.07 pitches away"
        );
    }
}
