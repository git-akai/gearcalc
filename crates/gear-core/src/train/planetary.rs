//! **The set's arrangement in its own words** — sun, carrier and ring, the
//! words a designer of a simple set uses, turned into the holds and the
//! case a train of one set is asked with.
//!
//! The set is not a stage type of its own, and has not been one since the
//! shape absorbed it: its geometry, closure, power flow and ratings are the
//! one shape's ([`super::shape`]), and the preset that lays one out is
//! `arrangements::planetary` — a central axis with the sun, the carrier and
//! the ring on it, a planet axis carried by the carrier and replicated `N`
//! times, and two meshes on the one distance between them. What was left
//! here was a second vocabulary for the same inputs — a module and a
//! pressure angle for the stage where the shape has one per member, a
//! friction per mesh named by the members it joins — and it went with the
//! types. This is what remains: the three roles, as a hold and a case.

use crate::planetary::{Arrangement, PlanetaryShaft};

const SLOT_SUN: usize = 1;
const SLOT_CARRIER: usize = 2;
const SLOT_RING: usize = 3;

/// **A train of one simple set, asked in the set's own words** — the shaft
/// `fixed` held, every case loaded at `input` and reacted at the shaft the
/// two leave over ([`super::Train::arranged`]). Bodies are the set's slots,
/// as [`super::Train::alone`] numbers them: the sun, the carrier and the
/// ring first. For a set asked about alone — the harness, a test, the
/// sweep; in a train the same thing is a hold and a case, and the set never
/// sees the words.
impl super::Train {
    #[must_use]
    pub fn arranged_as(self, arrangement: Arrangement) -> Self {
        let slot = |m: PlanetaryShaft| match m {
            PlanetaryShaft::Sun => SLOT_SUN,
            PlanetaryShaft::Carrier => SLOT_CARRIER,
            PlanetaryShaft::Ring => SLOT_RING,
        };
        let output = PlanetaryShaft::ALL
            .into_iter()
            .find(|&m| m != arrangement.input && m != arrangement.fixed)
            .unwrap_or(arrangement.input);
        self.arranged(
            &[slot(arrangement.fixed)],
            slot(arrangement.input),
            slot(output),
        )
    }
}
