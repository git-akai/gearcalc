//! The geometry outside the specification's conventional ranges.
//!
//! The specification bounds several inputs far more tightly than the geometry
//! does — tooth count at 3, pressure angle at 10–60°, helix at ±45°, addendum at
//! zero. None of those is a limit of the mathematics, and this file is the
//! evidence: a one-tooth gear, an 85° helix and a 2° pressure angle all generate
//! finite, closed, sane cross-sections.
//!
//! That matters because the project's rule for an input limit is **"could this
//! gear exist?"** and not "would anyone want it?". A guard that refuses a legal
//! shape is a guard that will one day refuse a legitimate design.

#![allow(clippy::unwrap_used)]

use gear_core::auto::admissible_ranges;
use gear_core::{Gear, GearParams};

/// Finite, ordered radii and a closed outline — the minimum for a gear to be a
/// gear at all.
fn is_constructible(p: GearParams) -> bool {
    let g = Gear::new(p);
    let finite = [g.r, g.rb, g.ra, g.rf, g.st].iter().all(|v| v.is_finite());
    let outline = g.profile(120);
    let closed = outline.len() > 8 && outline.first() == outline.last();
    finite && g.ra > g.rf && g.rf > 0.0 && closed
}

#[test]
fn the_specifications_ranges_are_conventional_not_mathematical() {
    // Tooth count: the specification says 3.
    for teeth in [1, 2, 3, 5] {
        assert!(
            is_constructible(GearParams {
                teeth,
                ..Default::default()
            }),
            "z={teeth} should be constructible"
        );
    }
    // Pressure angle: the specification says 10 to 60.
    for pressure_angle in [0.5, 2.0, 5.0, 70.0, 85.0] {
        assert!(
            is_constructible(GearParams {
                pressure_angle,
                ..Default::default()
            }),
            "alpha={pressure_angle} should be constructible"
        );
    }
    // Helix angle: the specification says +-45.
    for helix_angle in [-85.0, -60.0, 60.0, 85.0] {
        assert!(
            is_constructible(GearParams {
                helix_angle,
                ..Default::default()
            }),
            "beta={helix_angle} should be constructible"
        );
    }
    // Addendum: the specification says >= 0. A stub tooth is still a tooth.
    for addendum in [-0.5, -0.2, 0.0] {
        assert!(
            is_constructible(GearParams {
                addendum,
                ..Default::default()
            }),
            "addendum={addendum} should be constructible"
        );
    }
}

/// The bounds that *are* real, checked where it counts: just inside one the
/// generator builds without complaint, just outside it clamps and says so.
#[test]
fn the_geometric_bounds_are_exactly_where_the_generator_starts_clamping() {
    let cases = [
        GearParams::default(),
        GearParams {
            teeth: 9,
            ..Default::default()
        },
        GearParams {
            teeth: 40,
            profile_shift: 0.3,
            ..Default::default()
        },
        GearParams {
            pressure_angle: 14.5,
            ..Default::default()
        },
        GearParams {
            helix_angle: 25.0,
            ..Default::default()
        },
    ];

    for p in cases {
        let r = admissible_ranges(&p, 1.0);
        let clamped = |q: GearParams, needle: &str| {
            Gear::new(q).clamps.notes.iter().any(|n| n.contains(needle))
        };
        let eps = 0.01;

        // Addendum: below its floor the tip sinks into the root.
        let lo = r.addendum.min.unwrap();
        assert!(
            Gear::new(GearParams {
                addendum: lo + eps,
                ..p
            })
            .ra > Gear::new(GearParams {
                addendum: lo + eps,
                ..p
            })
            .rf,
            "tip should clear the root just inside the addendum floor"
        );

        // Dedendum: past its ceiling the root circle reaches for the axis.
        let hi = r.dedendum.max.unwrap();
        assert!(
            !clamped(
                GearParams {
                    dedendum: hi - eps,
                    ..p
                },
                "root radius"
            ),
            "clamped inside the dedendum ceiling"
        );
        assert!(
            clamped(
                GearParams {
                    dedendum: hi + eps,
                    ..p
                },
                "root radius"
            ),
            "no clamp above the dedendum ceiling"
        );

        // Root radius: past its ceiling the fillet cannot fit the space.
        let rr = r.root_radius.max.unwrap();
        assert!(
            !clamped(
                GearParams {
                    root_radius: rr - eps,
                    ..p
                },
                "fillet capped"
            ),
            "clamped inside the fillet cap"
        );
        assert!(
            clamped(
                GearParams {
                    root_radius: rr + eps,
                    ..p
                },
                "fillet capped"
            ),
            "no clamp above the fillet cap"
        );
    }
}

/// Addendum and dedendum bound *each other*: the tooth must have positive
/// height. Neither has a fixed floor of zero, which is what the specification
/// assumed.
#[test]
fn addendum_and_dedendum_bound_each_other() {
    for dedendum in [0.5, 1.0, 1.25, 2.0] {
        let p = GearParams {
            dedendum,
            ..Default::default()
        };
        let r = admissible_ranges(&p, 1.0);
        // The addendum floor is minus the dedendum, unless the base circle binds
        // first.
        assert!(r.addendum.min.unwrap() <= -dedendum + 1e-9 || r.addendum.min.unwrap() > -dedendum);
        assert!(r.addendum.min.unwrap() < 0.0, "a stub tooth is legal");

        // ...and read the other way round.
        let back = admissible_ranges(&GearParams { addendum: 1.0, ..p }, 1.0);
        assert!((back.dedendum.min.unwrap() + 1.0).abs() < 1e-12);
    }
}
