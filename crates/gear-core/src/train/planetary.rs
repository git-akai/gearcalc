//! **The set's arrangement as a boundary** — sun, carrier and ring, the
//! words a designer of a simple set uses, turned into what a lone stage's
//! solver takes.
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
//! types. This is what remains: the three roles, as a boundary.

use crate::planetary::{Arrangement, PlanetaryShaft};

const SLOT_SUN: usize = 1;
const SLOT_CARRIER: usize = 2;
const SLOT_RING: usize = 3;

/// **An arrangement as a boundary** — the set's own vocabulary for its
/// three central bodies, turned into what its solver takes.
///
/// For a set asked about alone: the harness, a test, the sweep. In a train
/// the same thing is two [`super::BodyConstraint`]s on the train, and the
/// set never sees the words.
#[must_use]
pub fn boundary_for(arrangement: Arrangement) -> super::StageBoundary {
    let slot = |m: PlanetaryShaft| match m {
        PlanetaryShaft::Sun => SLOT_SUN,
        PlanetaryShaft::Carrier => SLOT_CARRIER,
        PlanetaryShaft::Ring => SLOT_RING,
    };
    let output = PlanetaryShaft::ALL
        .into_iter()
        .find(|&m| m != arrangement.input && m != arrangement.fixed)
        .unwrap_or(arrangement.input);
    super::StageBoundary::holding(
        5,
        &[slot(arrangement.fixed)],
        slot(arrangement.input),
        slot(output),
    )
}
