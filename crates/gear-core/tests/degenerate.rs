//! **Degenerate input cannot panic, and the boundary refuses it by name.**
//! Each public entry point that takes a gear's parameters is fed the inputs
//! that describe no gear at all — no teeth, a module at or below nought, a
//! not-a-number in each figure, a right angle for each angle — and must
//! return, whatever it returns: the constructors are total, an outline of a
//! tooth with no flank is empty rather than an index past it, and the flank
//! junction's assertion holds only of input the boundary admits. What the
//! boundary does with each is the second law's: `GearParams::check`
//! (`gear_core::input`) refuses every one, naming the field it changed.
//!
//! The list of known panics this law carried (T16.21) is empty since the
//! validator landed (T01.6), in both profiles, and the list is gone with it:
//! a panic here is a failure, not an entry.

#![allow(clippy::unwrap_used)]

use gear_core::auto;
use gear_core::gear::{self, Gear};
use gear_core::metrology;
use gear_core::ring::{self, Cutter, Ring};
use gear_core::{GearParams, Tooth};
use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Every input that describes no gear, named: each on the default gear.
fn degenerate() -> Vec<(String, GearParams)> {
    let base = GearParams::default();
    let mut out = vec![
        ("teeth 0".to_owned(), GearParams { teeth: 0, ..base }),
        (
            "module 0".to_owned(),
            GearParams {
                module: 0.0,
                ..base
            },
        ),
        (
            "module -1".to_owned(),
            GearParams {
                module: -1.0,
                ..base
            },
        ),
        (
            "pressure_angle 90".to_owned(),
            GearParams {
                pressure_angle: 90.0,
                ..base
            },
        ),
        (
            "helix_angle 90".to_owned(),
            GearParams {
                helix_angle: 90.0,
                ..base
            },
        ),
    ];
    type Field = fn(&mut GearParams) -> &mut f64;
    let fields: [(&str, Field); 10] = [
        ("module", |p| &mut p.module),
        ("pressure_angle", |p| &mut p.pressure_angle),
        ("profile_shift", |p| &mut p.profile_shift),
        ("helix_angle", |p| &mut p.helix_angle),
        ("addendum", |p| &mut p.addendum),
        ("dedendum", |p| &mut p.dedendum),
        ("root_radius", |p| &mut p.root_radius),
        ("thickness_mod", |p| &mut p.thickness_mod),
        ("angular_shift", |p| &mut p.angular_shift),
        ("index_offset", |p| &mut p.index_offset),
    ];
    for (name, field) in fields {
        let mut p = base;
        *field(&mut p) = f64::NAN;
        out.push((format!("{name} NaN"), p));
    }
    out
}

type Entry = (&'static str, fn(GearParams));

/// What an entry returned, read so the call is made.
fn done<T>(t: T) {
    black_box(t);
}

/// Every public entry point that takes a gear's parameters, each asked the
/// way a caller asks it: built, then read.
fn entries() -> Vec<Entry> {
    vec![
        ("Tooth::new", |p| done(Tooth::new(p))),
        ("Tooth::with_flank_clamped_at_base", |p| {
            done(Tooth::with_flank_clamped_at_base(p));
        }),
        ("Tooth::tool_wanted_by", |p| done(Tooth::tool_wanted_by(&p))),
        ("Gear::new", |p| done(Gear::new(p))),
        ("Gear::profile", |p| {
            done(Gear::new(p).profile(64, gear_core::input::Budget::DEFAULT))
        }),
        ("Gear::outline", |p| {
            done(Gear::new(p).outline(
                gear_core::outline::DEFAULT_CHORD_TOLERANCE,
                gear_core::input::Budget::DEFAULT,
            ));
        }),
        ("gear::shift_at", |p| {
            done(gear::shift_at(&p, 1));
        }),
        ("auto::minimum_profile_shift", |p| {
            done(auto::minimum_profile_shift(&p, 2.0));
        }),
        ("auto::automatic_profile_shift", |p| {
            done(auto::automatic_profile_shift(&p, 2.0));
        }),
        ("auto::admissible_ranges", |p| {
            done(auto::admissible_ranges(&p, 2.0));
        }),
        ("auto::admissible_angular_shift", |p| {
            done(auto::admissible_angular_shift(&p));
        }),
        ("auto::root_radius_fits", |p| {
            done(auto::root_radius_fits(&p, 2.0));
        }),
        ("metrology::best_span", |p| {
            done(metrology::best_span(&Tooth::new(p)));
        }),
        ("metrology::pin_geometry", |p| {
            done(metrology::pin_geometry(&Tooth::new(p), 1.75));
        }),
        ("metrology::cutter_tip_width", |p| {
            done(metrology::cutter_tip_width(&Tooth::new(p)));
        }),
        ("metrology::best_span_around", |p| {
            done(metrology::best_span_around(&Gear::new(p)));
        }),
        ("metrology::pin_diameter_range_around", |p| {
            done(metrology::pin_diameter_range_around(&Gear::new(p)));
        }),
        ("Ring::cut_by", |p| {
            done(Ring::cut_by(&p, &Cutter::default()))
        }),
        ("Ring::profile", |p| {
            done(
                Ring::cut_by(&p, &Cutter::default()).profile(64, gear_core::input::Budget::DEFAULT),
            );
        }),
        ("ring::minimum_profile_shift", |p| {
            done(ring::minimum_profile_shift(&p, &Cutter::default()));
        }),
    ]
}

/// What asking `entry` with `p` did: `None` where it returned, or the
/// panic's message.
fn ask(entry: fn(GearParams), p: GearParams) -> Option<String> {
    catch_unwind(AssertUnwindSafe(|| entry(p))).err().map(|e| {
        let m = e
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| e.downcast_ref::<&str>().map(ToString::to_string))
            .unwrap_or_default();
        format!("panicked: {m}")
    })
}

#[test]
fn degenerate_input_cannot_panic() {
    // The panics are collected, not printed as they happen.
    std::panic::set_hook(Box::new(|_| {}));
    let mut failed: Vec<String> = Vec::new();
    let mut asked = 0;
    for (input, p) in degenerate() {
        for (name, entry) in entries() {
            if let Some(how) = ask(entry, p) {
                failed.push(format!("  {name} at {input}: {how}"));
            }
            asked += 1;
        }
    }
    let _ = std::panic::take_hook();
    assert!(
        failed.is_empty(),
        "of {asked} calls, {} panicked:\n{}",
        failed.len(),
        failed.join("\n")
    );
    assert_eq!(asked, degenerate().len() * entries().len());
}

/// **The boundary refuses every one of them, naming the field it changed**
/// — the one check every entry reads a gear through
/// (`GearParams::check`), and the only thing between these inputs and a
/// tooth built of them.
#[test]
fn every_degenerate_gear_is_refused_naming_its_field() {
    let mut refused = 0;
    for (input, p) in degenerate() {
        let field = input.split(' ').next().unwrap();
        let e = p
            .check()
            .expect_err(&format!("{input}: admitted, and describes no gear"));
        assert_eq!(e.field, field, "{input}: {e}");
        refused += 1;
    }
    assert_eq!(refused, degenerate().len());
    // ...and the default gear, on either side of nothing, is admitted.
    assert!(GearParams::default().check().is_ok());
}

/// **The pressure angle has one reading, and below its floor it is the
/// floor's tooth, said** (T01.5). Every generator, search and mesh reads
/// the angle through `GearParams::normal_pressure_angle_rad`, so at 0.1°,
/// 0.3° and 0.49° a tooth is the tooth at 0.5° — its profile point for
/// point, its thickness — and says it was raised; at 0.5° and 0.6° it is
/// the angle asked, and says nothing. Swept over tooth thickness
/// modifications either side of one, since the thickness-only shift
/// `π(k − 1)/(4 tan αₙ)` is the reader that went unguarded, and a ring
/// is held to the same floor. The floor is 0.5° by name here, so a
/// perturbed constant is caught; and the angle nought, which the boundary
/// refuses, still builds a tooth with no not-a-number in it.
#[test]
fn the_pressure_angle_is_read_one_way_and_held_at_its_floor() {
    const FLOOR: f64 = 0.5;
    let mut asked = 0;
    for k in [0.8, 0.9, 1.0, 1.1, 1.2] {
        for alpha in [0.1, 0.3, 0.49, 0.5, 0.6] {
            let p = GearParams {
                pressure_angle: alpha,
                thickness_mod: k,
                ..GearParams::default()
            };
            let t = Tooth::new(p);
            let held = alpha.max(FLOOR);
            let at = Tooth::new(GearParams {
                pressure_angle: held,
                ..p
            });
            assert_eq!(t.alpha_n, held.to_radians(), "α {alpha} k {k}");
            assert_eq!(t.st, at.st, "α {alpha} k {k}: thickness");
            let (a, b) = (
                Gear::new(p)
                    .profile(64, gear_core::input::Budget::DEFAULT)
                    .unwrap(),
                Gear::new(at.params)
                    .profile(64, gear_core::input::Budget::DEFAULT)
                    .unwrap(),
            );
            assert!(!a.is_empty() && a.iter().flatten().all(|x| x.is_finite()));
            assert_eq!(a, b, "α {alpha} k {k}: profile");
            let raised = t
                .clamps
                .fired(gear_core::note::key::CLAMP_PRESSURE_ANGLE_RAISED);
            assert_eq!(raised, alpha < FLOOR, "α {alpha} k {k}: said");
            let ring = Ring::cut_by(&p, &Cutter::default());
            assert_eq!(ring.alpha_n, held.to_radians(), "ring α {alpha} k {k}");
            assert_eq!(
                ring.clamps
                    .iter()
                    .any(|n| n.is(gear_core::note::key::CLAMP_PRESSURE_ANGLE_RAISED)),
                alpha < FLOOR,
                "ring α {alpha} k {k}: said"
            );
            asked += 1;
        }
    }
    assert_eq!(asked, 25);
    let flat = Tooth::new(GearParams {
        pressure_angle: 0.0,
        ..GearParams::default()
    });
    assert!(flat.st.is_finite() && flat.ra.is_finite() && flat.r_j.is_finite());
    assert!(!Gear::new(flat.params)
        .profile(64, gear_core::input::Budget::DEFAULT)
        .unwrap()
        .is_empty());
}

/// **The floor is read in one place**: every source of the crate but the
/// one that states it (`params.rs`) and the table that bounds by it
/// (`input.rs`) reads the angle through the accessor, never the constant.
#[test]
fn the_pressure_angle_floor_is_read_in_one_place() {
    let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = 0;
    let mut stack = vec![src];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            let name = path.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&path).unwrap();
            // A test module may name the floor it tests against.
            let production = text.split("#[cfg(test)]").next().unwrap();
            let reads = production
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .filter(|l| l.contains("MIN_PRESSURE_ANGLE_DEG"))
                .count();
            if !["params.rs", "input.rs"].contains(&name.as_str()) {
                assert_eq!(reads, 0, "{name} reads the floor itself");
            }
            files += 1;
        }
    }
    assert!(files > 40, "{files} sources read");
}
