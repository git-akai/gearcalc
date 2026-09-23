//! What the note catalogue *claims*, measured.
//!
//! A message that states a number is a documented figure like any other, and
//! documented figures drift. `stage.load_sharing_out_of_band` said the ramp
//! "relieves the tooth by about a third" for two model revisions after that
//! stopped being true — it now runs from a 24 % relief to a 15 % **increase**,
//! and the sign was the part nobody was watching.
//!
//! This lives in `gear-io` because that is where the catalogue is: the crate
//! that owns the sentence owns the check on it.

use gear_core::contact::LoadSharing;
use gear_core::train::{arrangements, solve_alone, StageGear, Train};

/// **The sharing note's number, and its lack of a sign.**
///
/// The message tells a reader the ramp is extrapolating and that what it does
/// to the figure has no fixed direction. Both halves are asserted: that the
/// effect reaches the tens of percent the message quotes, and that it is
/// genuinely seen in *both* directions — because "it relieves the tooth" was
/// true when it was written and is the kind of claim that quietly stops being.
#[test]
fn the_sharing_note_quotes_a_number_the_sweep_still_produces() {
    let lib = gear_io::default_library();
    let rated = |teeth: u32, addendum: f64, model: LoadSharing| {
        let g = StageGear {
            teeth,
            addendum,
            profile_shift: gear_core::params::Auto::fixed(0.0),
            ..Default::default()
        };
        let stage = {
            let mut s = arrangements::pair([g.teeth, g.teeth]);
            s.set_load_sharing(model);
            s.members[0].gear = g.clone();
            s.members[1].gear = g;
            s
        };
        solve_alone(&Train::alone(&stage, 2.0, 0.0), &lib)
            .ok()
            .and_then(|r| r.members()[0].cases[0].bending_stress)
    };

    let mut seen: Vec<f64> = Vec::new();
    for (teeth, addendum) in [(60u32, 1.35_f64), (60, 1.4), (40, 1.35), (100, 1.4)] {
        let (Some(off), Some(on)) = (
            rated(teeth, addendum, LoadSharing::None),
            rated(teeth, addendum, LoadSharing::LinearRamp),
        ) else {
            continue;
        };
        seen.push(1.0 - on / off);
    }
    assert!(seen.len() >= 4, "the band should be reachable: {seen:?}");

    let lo = seen.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = seen.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    assert!(
        lo < 0.0,
        "sharing must still be seen *raising* a figure — the message says so, \
         and it is the half that was wrong for two revisions: {seen:?}"
    );
    assert!(hi > 0.0, "...and relieving one: {seen:?}");
    // The message quotes 24 % and 15 %; hold the size loosely, the sign exactly.
    assert!(
        (0.15..0.35).contains(&hi) && (-0.30..-0.05).contains(&lo),
        "the quoted magnitudes have moved: relief {:.1}%, increase {:.1}%",
        100.0 * hi,
        -100.0 * lo
    );

    // ...and below the band the model changes nothing at all, which is the
    // other half of what the note's existence implies.
    let (Some(a), Some(b)) = (
        rated(17, 1.0, LoadSharing::None),
        rated(17, 1.0, LoadSharing::LinearRamp),
    ) else {
        panic!("an ordinary pair should rate")
    };
    assert!(
        (a - b).abs() < 1e-12,
        "below the band sharing is the identity: {a} vs {b}"
    );
}
