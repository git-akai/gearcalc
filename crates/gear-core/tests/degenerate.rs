//! **Degenerate input cannot panic.** Each public entry point that takes a
//! gear's parameters is fed the inputs that describe no gear at all — no
//! teeth, a module at or below nought, a not-a-number in each figure, a
//! right angle for each angle — and must return, whatever it returns. What it
//! should return (a refusal under rule 5) is T01.6's; this law asks only that
//! it does not panic.
//!
//! A panic this law finds and nothing yet cures is listed by its site in
//! [`KNOWN`] with the task that cures it. Every panic must be at a listed
//! site, and every listed site must still be reached, so the list only
//! shrinks.

#![allow(clippy::unwrap_used)]

use gear_core::auto;
use gear_core::gear::{self, Gear};
use gear_core::metrology;
use gear_core::ring::{self, Cutter, Ring};
use gear_core::{GearParams, Tooth};
use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// **The panics the law finds today, by site**: the start of the message and
/// the task that removes it. Both are T01.6's: `GearParams::check` at every
/// boundary refuses what reaches the first, and `Gear::profile` and
/// `Ring::profile` never index an empty flank.
const KNOWN: &[(&str, &str)] = &[
    // tooth.rs, `debug_assert!` on the junction the flank solve found.
    ("panicked: unsolved flank junction", "T01.6"),
    // gear.rs and ring.rs, `&r[1..]` on a flank that came out empty.
    ("panicked: range start index 1 out of range", "T01.6"),
];

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
            "pressure angle 90".to_owned(),
            GearParams {
                pressure_angle: 90.0,
                ..base
            },
        ),
        (
            "helix 90".to_owned(),
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
        ("Gear::profile", |p| done(Gear::new(p).profile(64))),
        ("Gear::outline", |p| {
            done(Gear::new(p).outline(gear_core::outline::DEFAULT_CHORD_TOLERANCE));
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
            done(Ring::cut_by(&p, &Cutter::default()).profile(64));
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
    let mut failed: Vec<(String, String, String)> = Vec::new();
    let mut asked = 0;
    for (input, p) in degenerate() {
        for (name, entry) in entries() {
            if let Some(how) = ask(entry, p) {
                failed.push((name.to_owned(), input.clone(), how));
            }
            asked += 1;
        }
    }
    let _ = std::panic::take_hook();
    let site = |how: &str| KNOWN.iter().position(|(start, _)| how.starts_with(start));
    let new: Vec<_> = failed
        .iter()
        .filter(|(_, _, how)| site(how).is_none())
        .collect();
    let mut at = vec![0; KNOWN.len()];
    for (_, _, how) in &failed {
        if let Some(k) = site(how) {
            at[k] += 1;
        }
    }
    let cured: Vec<_> = KNOWN
        .iter()
        .zip(&at)
        .filter(|(_, n)| **n == 0)
        .map(|(k, _)| k)
        .collect();
    assert!(
        new.is_empty() && cured.is_empty(),
        "of {asked} calls, failing at no listed site:\n{}\nlisted sites now never reached: {cured:?}",
        new.iter()
            .map(|(e, i, h)| format!("  {e} at {i}: {h}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert_eq!(asked, degenerate().len() * entries().len());
}
