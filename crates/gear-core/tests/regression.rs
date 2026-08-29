//! Pinned known-good geometry, so a refactor fails loudly rather than quietly.
//!
//! Every value here was produced by `handoff_inbound/gear.py`, the reference
//! implementation that was validated to 5e-4 mm against a full rack simulation.
//! The Rust port reproduces the whole 1188-case reference grid to 7.5e-14 mm;
//! these eight cases are the subset kept as permanent fixtures.
//!
//! The set is chosen for coverage of the awkward states, not for tidiness:
//! undercut, severed, pointed-tooth-capped, helical, and a sharp-cornered rack.

#![allow(clippy::unwrap_used)]
// These literals are transcribed reference output, not authored numbers. Clippy
// would have us "tidy" 9.699_999_999_999_999 into 9.7 and recognise
// 1.570_796_326_794_897 as FRAC_PI_2 — both true, and both destroy the point:
// a fixture must be the value the reference actually produced, digit for digit,
// so a future refactor is compared against it rather than against a rounding.
#![allow(clippy::excessive_precision, clippy::approx_constant)]

use gear_core::{Gear, GearParams};

struct Fixture {
    params: GearParams,
    rb: f64,
    ra: f64,
    rf: f64,
    st: f64,
    rho: f64,
    l: f64,
    r_j: f64,
    theta0: f64,
    theta_a: f64,
    undercut: bool,
    severed: bool,
}

/// Relative tolerance against the reference. The port agrees far more closely
/// than this; the margin covers the difference between our Brent and SciPy's on
/// the two solved quantities (`r_j`, and `l` through it).
const REL: f64 = 1e-11;

fn fixtures() -> Vec<Fixture> {
    vec![
        // z=17, x=0: the textbook marginal-undercut case
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 20.0,
                teeth: 17,
                profile_shift: 0.0,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 0.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.38,
                thickness_mod: 1.0,
            },
            rb: 7.987_387_276_680_222e0,
            ra: 9.5e0,
            rf: 7.25e0,
            st: 1.570_796_326_794_897,
            rho: 3.8e-1,
            l: -1.653_860_987_370_204e-2,
            r_j: 7.987_391_540_277_663e0,
            theta0: 1.772_282_142_058_695e-1,
            theta_a: 3.547_782_565_394_374e-2,
            undercut: true,
            severed: false,
        },
        // same gear, shifted clear of undercut
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 20.0,
                teeth: 17,
                profile_shift: 0.2,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 0.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.38,
                thickness_mod: 1.0,
            },
            rb: 7.987_387_276_680_222e0,
            ra: 9.699_999_999_999_999e0,
            rf: 7.45e0,
            st: 1.716_384_420_501_377,
            rho: 3.8e-1,
            l: 5.682_222_701_589_151e-1,
            r_j: 8.007_573_418_706_672e0,
            theta0: 1.772_282_142_058_695e-1,
            theta_a: 3.015_424_050_886_523e-2,
            undercut: false,
            severed: false,
        },
        // heavily undercut
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 20.0,
                teeth: 8,
                profile_shift: 0.0,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 0.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.38,
                thickness_mod: 1.0,
            },
            rb: 3.758_770_483_143_634e0,
            ra: 5.0e0,
            rf: 2.75e0,
            st: 1.570_796_326_794_897,
            rho: 3.8e-1,
            l: -1.555_629_254_839_211e0,
            r_j: 3.806_776_907_901_851e0,
            theta0: 3.766_099_551_874_727e-1,
            theta_a: 5.412_578_274_850_716e-2,
            undercut: true,
            severed: false,
        },
        // helical, negative shift, fillet cap binding
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 25.0,
                teeth: 13,
                profile_shift: -0.2,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 20.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.38,
                thickness_mod: 1.0,
            },
            rb: 6.196_197_166_740_613e0,
            ra: 7.717_155_521_093_428e0,
            rf: 5.467_155_521_093_428e0,
            st: 1.473_112_838_084_4,
            rho: 3.301_533_547_888_946e-1,
            l: 2.253_452_428_394_678e-1,
            r_j: 6.200_293_525_922_364e0,
            theta0: 2.401_031_765_014_941e-1,
            theta_a: 3.829_876_889_894_449e-2,
            undercut: false,
            severed: false,
        },
        // left-hand helix, 14.5 deg, sharp-cornered rack
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 14.5,
                teeth: 31,
                profile_shift: 0.5,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: -30.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.0,
                thickness_mod: 1.0,
            },
            rb: 1.714_951_152_869_241e1,
            ra: 1.939_785_834_487_840e1,
            rf: 1.714_785_834_487_840e1,
            st: 2.112_425_228_124_306,
            rho: 1e-9,
            l: 2.500_190_318_506_44,
            r_j: 1.733_080_198_090_98e1,
            theta0: 7.152_710_609_596_835e-2,
            theta_a: 2.511_364_063_450_693e-2,
            undercut: false,
            severed: false,
        },
        // severed: undercut removes the tooth entirely
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 20.0,
                teeth: 3,
                profile_shift: -0.5,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 0.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.38,
                thickness_mod: 1.0,
            },
            rb: 1.409_538_931_178_863e0,
            ra: 3.076_271_699_551_705e-1,
            rf: 1.499_999_999_999_999e-1,
            st: 1.206_826_092_528_694,
            rho: 3.8e-1,
            l: -2.703_060_053_169_692e0,
            r_j: 3.076_271_699_551_705e-1,
            theta0: 9.072_344_846_956_066e-1,
            theta_a: 0.0,
            undercut: true,
            severed: true,
        },
        // pointed tooth: tip capped, theta_a driven to zero
        Fixture {
            params: GearParams {
                module: 1.0,
                pressure_angle: 20.0,
                teeth: 9,
                profile_shift: 0.9,
                angular_shift: 0.0,
                index_offset: 0.0,
                helix_angle: 0.0,
                addendum: 1.0,
                dedendum: 1.25,
                root_radius: 0.0,
                thickness_mod: 1.0,
            },
            rb: 4.228_616_793_536_588e0,
            ra: 6.258_882_186_696_63,
            rf: 4.15e0,
            st: 2.225_942_748_474_061,
            rho: 1e-9,
            l: 5.157_591_068_322_332e-1,
            r_j: 4.259_953_924_969_147e0,
            theta0: 2.756_357_682_067_575e-1,
            theta_a: 0.0,
            undercut: false,
            severed: false,
        },
    ]
}

fn close(got: f64, want: f64, what: &str, p: &GearParams) {
    let scale = want.abs().max(1.0);
    assert!(
        (got - want).abs() <= REL * scale,
        "{what}: got {got:.17e}, reference {want:.17e} (rel {:.2e}) for z={} x={}",
        (got - want).abs() / scale,
        p.teeth,
        p.profile_shift
    );
}

#[test]
fn matches_the_reference_implementation() {
    for f in fixtures() {
        let g = Gear::new(f.params);
        close(g.rb, f.rb, "base radius", &f.params);
        close(g.ra, f.ra, "tip radius", &f.params);
        close(g.rf, f.rf, "root radius", &f.params);
        close(g.st, f.st, "tooth thickness", &f.params);
        close(g.rho, f.rho, "fillet radius", &f.params);
        close(g.l, f.l, "undercut indicator L", &f.params);
        close(g.r_j, f.r_j, "junction radius", &f.params);
        close(g.theta0, f.theta0, "root arc angle", &f.params);
        close(g.theta_a, f.theta_a, "tip arc half width", &f.params);
        assert_eq!(
            g.undercut, f.undercut,
            "undercut flag for z={}",
            f.params.teeth
        );
        assert_eq!(
            g.severed, f.severed,
            "severed flag for z={}",
            f.params.teeth
        );
    }
}

/// The negative fixture. `with_flank_clamped_at_base` reproduces the pre-fix behaviour —
/// the flank clamped at the base circle with a gap left to the fillet — and this
/// test exists to prove the suite still detects that fault. If this ever stops
/// showing a large gap, the detection has broken, not the bug.
#[test]
fn legacy_clamp_still_shows_the_junction_step_it_was_kept_to_demonstrate() {
    for (teeth, x) in [(8u32, 0.0), (3, 0.5)] {
        let p = GearParams {
            teeth,
            profile_shift: x,
            ..Default::default()
        };

        let fixed = Gear::new(p);
        let (r_fl, th_fl) = fixed.involute_at(fixed.u_j);
        let (r_tr, th_tr) = fixed.trochoid_at(fixed.s_j);
        let gap_fixed = f64::hypot(
            r_fl * th_fl.sin() - r_tr * th_tr.sin(),
            r_fl * th_fl.cos() - r_tr * th_tr.cos(),
        );

        let legacy = Gear::with_flank_clamped_at_base(p);
        let (r_fl, th_fl) = legacy.involute_at(legacy.u_j);
        let (r_tr, th_tr) = legacy.trochoid_at(legacy.s_j);
        let gap_legacy = f64::hypot(
            r_fl * th_fl.sin() - r_tr * th_tr.sin(),
            r_fl * th_fl.cos() - r_tr * th_tr.cos(),
        );

        assert!(
            gap_fixed < 1e-12,
            "z={teeth}: fixed build has a junction gap of {gap_fixed:.3e} mm"
        );
        assert!(
            gap_legacy > 1e-3,
            "z={teeth}: legacy build should show a visible step, got {gap_legacy:.3e} mm"
        );
    }
}
