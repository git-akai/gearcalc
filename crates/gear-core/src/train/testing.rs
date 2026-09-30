//! **The train's one test grid**, as `tests/common` is the gear's: every
//! arrangement a train can be built from, in every context a part meets,
//! each nameable, so a law about trains is swept over one list and a preset
//! added to [`Preset::ALL`] reaches every law with no edit here. The grid
//! itself is [`super::sweep`]'s, where `gear-cli identity` reads it too;
//! this adds what only a test asks.
//!
//! - [`alone`]: one arrangement asked alone ([`Train::alone`]), the way the
//!   laws about a single part ask it.

#![allow(clippy::unwrap_used)]

use super::arrangements::Preset;
pub use super::sweep::{arrangements, cased, grid, shapes, trains, Context, SPEED_RPM, TORQUE_NM};
use super::{solve_alone, test_library, Alone, Shape, Train, TrainError};

/// The torque an arrangement asked [`alone`] is loaded with, N·m.
pub const ALONE_NM: f64 = 2.0;

/// **An arrangement asked alone** — a train of one ([`Train::alone`]) loaded
/// with [`ALONE_NM`] held still at its conventional input and reacted at
/// its output — solved and read as that part ([`solve_alone`]), or the
/// test fails naming the shape.
pub fn alone(shape: &Shape) -> Alone {
    try_alone(shape).unwrap_or_else(|e| panic!("{e}: {shape:?}"))
}

/// [`alone`], its refusal handed back.
///
/// # Errors
///
/// Whatever the part reports.
pub fn try_alone(shape: &Shape) -> Result<Alone, TrainError> {
    try_alone_at(shape, ALONE_NM, 0.0)
}

/// [`try_alone`] at a given torque and speed.
///
/// # Errors
///
/// Whatever the part reports.
pub fn try_alone_at(shape: &Shape, torque: f64, speed: f64) -> Result<Alone, TrainError> {
    solve_alone(&Train::alone(shape, torque, speed), &test_library())
}

/// **The grid covers what it says**: every preset, the three arrangements
/// no button lays out, in three contexts each; every train is whole
/// ([`Train::check`]) and cased.
#[test]
fn the_grid_is_every_arrangement_in_every_context() {
    let g = grid();
    assert_eq!(g.len(), (Preset::ALL.len() + 3) * Context::ALL.len());
    for e in &g {
        let t = e.train();
        t.check()
            .unwrap_or_else(|err| panic!("{}: {err:?}", e.name));
        assert_eq!(t.load_cases.len(), 2, "{}: not cased", e.name);
        let parts = t.parts().len();
        let own = e.shape.parts().len();
        let expected = match e.context {
            Context::Alone => own,
            Context::AfterSpur | Context::BeforeLayshaft => own + 1,
        };
        assert_eq!(parts, expected, "{}: parts", e.name);
    }
}
