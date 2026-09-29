//! **Degenerate input cannot panic.** Each public entry point that takes a
//! gear's parameters is fed the inputs that describe no gear at all — no
//! teeth, a module at or below nought, a not-a-number in each figure, a
//! right angle for each angle — and must return, whatever it returns. What it
//! should return (a refusal under rule 5) is T01.6's; this law asks only that
//! it does not panic.
//!
//! A panic this law finds and nothing yet cures is listed, by entry point
//! and input, with the task that cures it ([`known`]). The calls that panic
//! must be exactly the listed ones: a new panic fails the law, and so does
//! a listed call that stops panicking, so the list only shrinks.
//!
//! **The list is per profile**, because what panics is: a `debug_assert!`
//! fires only where debug assertions are on (the test profile CI runs), and
//! a call it stops may panic further on where they are off (`--release`).

#![allow(clippy::unwrap_used)]

use gear_core::auto;
use gear_core::gear::{self, Gear};
use gear_core::metrology;
use gear_core::ring::{self, Cutter, Ring};
use gear_core::{GearParams, Tooth};
use std::hint::black_box;
use std::panic::{catch_unwind, AssertUnwindSafe};

/// **Calls that panic today**: on `input`, each of `entries`, and the task
/// that removes the panic.
struct Known {
    input: &'static str,
    entries: &'static [&'static str],
    task: &'static str,
}

/// T01.6: `GearParams::check` at every boundary refuses these inputs, and
/// `Gear::profile` and `Ring::profile` never index an empty flank.
const T01_6: &str = "T01.6";

/// Where debug assertions are on, the flank junction's `debug_assert!`
/// (tooth.rs) stops every call that builds a tooth first; the rest are
/// `&r[1..]` on an empty flank (gear.rs, ring.rs).
const KNOWN_WITH_DEBUG_ASSERTIONS: &[Known] = &[
    Known {
        input: "teeth 0",
        entries: &[
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
        ],
        task: T01_6,
    },
    Known {
        input: "module 0",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
            "Ring::profile",
        ],
        task: T01_6,
    },
    Known {
        input: "module -1",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
        ],
        task: T01_6,
    },
    Known {
        input: "helix 90",
        entries: &["Gear::profile"],
        task: T01_6,
    },
    Known {
        input: "module NaN",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
            "Ring::profile",
        ],
        task: T01_6,
    },
    Known {
        input: "pressure_angle NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "profile_shift NaN",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
            "Ring::profile",
        ],
        task: T01_6,
    },
    Known {
        input: "helix_angle NaN",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
            "Ring::profile",
        ],
        task: T01_6,
    },
    Known {
        input: "dedendum NaN",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::profile",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
        ],
        task: T01_6,
    },
    Known {
        input: "thickness_mod NaN",
        entries: &[
            "Tooth::new",
            "Tooth::with_flank_clamped_at_base",
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span",
            "metrology::pin_geometry",
            "metrology::cutter_tip_width",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
            "Ring::profile",
        ],
        task: T01_6,
    },
    Known {
        input: "angular_shift NaN",
        entries: &[
            "Gear::new",
            "Gear::profile",
            "Gear::outline",
            "metrology::best_span_around",
            "metrology::pin_diameter_range_around",
        ],
        task: T01_6,
    },
];

/// Where debug assertions are off, the calls the junction's assertion
/// stopped run on, and those that reach an empty flank index it.
const KNOWN_WITHOUT_DEBUG_ASSERTIONS: &[Known] = &[
    Known {
        input: "teeth 0",
        entries: &["Gear::profile"],
        task: T01_6,
    },
    Known {
        input: "module 0",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "helix 90",
        entries: &["Gear::profile"],
        task: T01_6,
    },
    Known {
        input: "module NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "pressure_angle NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "profile_shift NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "helix_angle NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "dedendum NaN",
        entries: &["Gear::profile"],
        task: T01_6,
    },
    Known {
        input: "thickness_mod NaN",
        entries: &["Gear::profile", "Ring::profile"],
        task: T01_6,
    },
    Known {
        input: "angular_shift NaN",
        entries: &["Gear::profile"],
        task: T01_6,
    },
];

/// The list for the profile this runs in.
fn known() -> &'static [Known] {
    if cfg!(debug_assertions) {
        KNOWN_WITH_DEBUG_ASSERTIONS
    } else {
        KNOWN_WITHOUT_DEBUG_ASSERTIONS
    }
}

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

/// The panics found, as the [`Known`] rows that would list them.
fn table(failed: &[(String, String, String)]) -> String {
    let mut inputs: Vec<&str> = failed.iter().map(|(_, i, _)| i.as_str()).collect();
    inputs.dedup();
    inputs
        .iter()
        .map(|input| {
            let entries: Vec<String> = failed
                .iter()
                .filter(|(_, i, _)| i == input)
                .map(|(e, _, _)| format!("\"{e}\""))
                .collect();
            format!(
                "    Known {{ input: \"{input}\", entries: &[{}], task: T01_6 }},",
                entries.join(", ")
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
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
    let listed: Vec<(&str, &str)> = known()
        .iter()
        .flat_map(|k| k.entries.iter().map(|e| (*e, k.input)))
        .collect();
    let new: Vec<_> = failed
        .iter()
        .filter(|(e, i, _)| !listed.contains(&(e.as_str(), i.as_str())))
        .map(|(e, i, how)| format!("  {e} at {i}: {how}"))
        .collect();
    let cured: Vec<_> = listed
        .iter()
        .filter(|(e, i)| {
            !failed
                .iter()
                .any(|(x, y, _)| (x.as_str(), y.as_str()) == (*e, *i))
        })
        .collect();
    assert!(
        new.is_empty() && cured.is_empty(),
        "of {asked} calls ({} listed in this profile), panicking and not listed:\n{}\nlisted and now returning: {cured:?}\n\nthe panics as found, by input:\n{}",
        listed.len(),
        new.join("\n"),
        table(&failed)
    );
    for k in known() {
        assert!(!k.task.is_empty(), "{}: no task removes these", k.input);
    }
    assert_eq!(asked, degenerate().len() * entries().len());
}
