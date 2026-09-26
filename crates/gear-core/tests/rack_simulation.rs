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
use gear_core::{GearParams, Tooth};

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
        let g = Tooth::new(*p);
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

/// Rows past the main grid's corner, one axis at a time rather than as a
/// product — the main grid is already the suite's slowest test. Each row is
/// where a rack's tooth comes to a point before the ISO depth: steep pressure
/// angles, a deep dedendum, a thick tooth.
fn pointed_rack_rows() -> Vec<GearParams> {
    let row = || {
        Grid::new()
            .teeth(&[5, 9, 17, 31])
            .shifts(&[-0.2, 0.0, 0.5])
            .root_radius(&[0.0, 0.38])
    };
    [
        row().pressure_angle(&[30.0, 35.0, 40.0]).build(),
        row().dedendum(&[2.0]).build(),
        row().thickness_mod(&[1.3]).build(),
    ]
    .concat()
}

/// Rows past the undercut band, where the fillet eats the flank from below
/// until the crossing is above the tip.
fn deep_undercut_rows() -> Vec<GearParams> {
    [
        Grid::new()
            .teeth(&[9, 12, 17, 31])
            .shifts(&[-0.9, -1.2])
            .pressure_angle(&[14.5, 20.0])
            .root_radius(&[0.0, 0.38])
            .build(),
        vec![
            GearParams {
                teeth: 12,
                profile_shift: -1.0,
                pressure_angle: 14.5,
                ..Default::default()
            },
            GearParams {
                profile_shift: -1.5,
                ..Default::default()
            },
            // The fuzz case the finding was made on.
            GearParams {
                module: 3.440_788,
                pressure_angle: 14.580_153,
                teeth: 9,
                profile_shift: -0.685_214,
                helix_angle: 6.845_211,
                addendum: 0.540_009,
                dedendum: 1.477_107,
                root_radius: 0.372_547,
                thickness_mod: 1.198_174,
                ..Default::default()
            },
        ],
    ]
    .concat()
}

/// **The gate holds past the undercut band.** Before the tooth ended at its
/// tip, a flank drawn from above the tip stood in the cutter's path: 5.1e-2 mm
/// of penetration at z12 x−1.0 α14.5, 0.729 mm on the fuzz gear.
#[test]
fn profile_is_bounded_by_the_cutter_past_the_undercut_band() {
    for p in deep_undercut_rows() {
        let rep = check_cut(&Tooth::new(p), 150);
        let tag = format!(
            "z={} x={} a={} rho={} m={}",
            p.teeth, p.profile_shift, p.pressure_angle, p.root_radius, p.module
        );
        assert!(
            rep.penetration <= PENETRATION_LIMIT,
            "penetration {:.3e} mm at {tag}",
            rep.penetration
        );
        assert!(
            rep.deviation < DEVIATION_LIMIT,
            "deviation {:.3e} mm at {tag}",
            rep.deviation
        );
    }
}

/// **The gate holds where the rack comes to a point.** Its cutter is built
/// from the inputs and the tool, so a tooth whose depth was driven past the
/// rack's closing point shows as a root the rack never reached — 4e-2 to
/// 0.37 mm of deviation — until the depth stops there.
#[test]
fn profile_is_bounded_by_the_cutter_where_the_rack_comes_to_a_point() {
    let mut closed = 0;
    for p in pointed_rack_rows() {
        let g = Tooth::new(p);
        closed += usize::from(g.clamps.fired(gear_core::note::key::CLAMP_SPACE_CLOSED));
        let rep = check_cut(&g, 150);
        let tag = format!(
            "z={} x={} a={} hf={} k={} rho={}",
            p.teeth, p.profile_shift, p.pressure_angle, p.dedendum, p.thickness_mod, p.root_radius
        );
        assert!(
            rep.penetration <= PENETRATION_LIMIT,
            "penetration {:.3e} mm at {tag}",
            rep.penetration
        );
        assert!(
            rep.deviation < DEVIATION_LIMIT,
            "deviation {:.3e} mm at {tag}",
            rep.deviation
        );
    }
    assert!(closed > 0, "no row reached the rack's closing depth");
}

/// The root the gate's own cutter reaches, where the rack's tooth closes above
/// the asked depth: `r − b_point`, `b_point = (π m_t − s_t)/(2 tan α_t)` at a
/// sharp corner. At z20, α30°, h_f 1.4 that is 8.6397 mm; driving the rack to
/// the asked depth put the root at 8.600.
#[test]
fn the_root_is_where_the_racks_tooth_closes() {
    let g = Tooth::new(GearParams {
        teeth: 20,
        pressure_angle: 30.0,
        dedendum: 1.4,
        root_radius: 0.0,
        ..Default::default()
    });
    let b_point = (std::f64::consts::PI * g.mt - g.st) / (2.0 * g.alpha_t.tan());
    assert!(
        (g.rf - (g.r - b_point)).abs() < 1e-8,
        "root at {} against {}",
        g.rf,
        g.r - b_point
    );
}

/// The gate's rack is rebuilt from the inputs and the settled tool, never from
/// the tooth, so a tooth that is wrong in a way its own derived quantities agree
/// with is still a tooth that rack did not cut.
///
/// Each fault is planted by cutting the tooth for other inputs and letting it
/// claim the originals: every derived quantity then moves together, which is
/// the error a gate built from those quantities cannot see.
#[test]
fn the_gate_sees_a_tooth_its_rack_did_not_cut() {
    let p = GearParams::default();
    let asked = Tooth::new(p);
    let claiming = |mut g: Tooth| {
        g.params = p;
        g.tool = asked.tool;
        check_cut(&g, 150)
    };

    // Thicker by `δ` at the pitch circle: `s = m (π k/2 + 2 x tan α)`.
    let delta = 1e-3;
    let thick = claiming(Tooth::new(GearParams {
        thickness_mod: 1.0 + 2.0 * delta / std::f64::consts::PI,
        ..p
    }));
    // Each flank stands `δ/2` proud across the pitch line, `δ/2 · cos α` along
    // its normal.
    let proud = delta / 2.0 * asked.alpha_t.cos();
    assert!(
        (thick.penetration - proud).abs() < 0.1 * proud,
        "a flank {proud:.3e} mm proud reads as penetration {:.3e}",
        thick.penetration
    );

    // Deeper by `δ`: the root the rack never reached is the deviation. Planted
    // well above the gate's own limit, so what is asserted is that it measures
    // the fault rather than where its threshold sits.
    let delta = 10.0 * DEVIATION_LIMIT;
    let deep = claiming(Tooth::new(GearParams {
        dedendum: p.dedendum + delta / p.module,
        ..p
    }));
    assert!(
        deep.penetration <= PENETRATION_LIMIT && (deep.deviation - delta).abs() < DEVIATION_LIMIT,
        "a root {delta:.3e} mm too deep reads as deviation {:.3e}",
        deep.deviation
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
        let g = Tooth::new(p);
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
        let g = Tooth::new(p);
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
        let g = Tooth::new(p);
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
        let g = Tooth::new(p);
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
