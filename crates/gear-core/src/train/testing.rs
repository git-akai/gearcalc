//! **The train's one test grid**, as `tests/common` is the gear's: every
//! arrangement a train can be built from, in every context a part meets,
//! each nameable, so a law about trains is swept over one list and a preset
//! added to [`Preset::ALL`] reaches every law with no edit here.
//!
//! - [`arrangements`]: every preset, and the arrangements no button lays out
//!   (the hula, a Ravigneaux set, a worm with a pair after it).
//! - [`Context`]: the arrangement alone, after a spur pair, and before a
//!   layshaft — a part whose members, meshes and bodies are not the graph's
//!   by the same index, and a train with an idle branch.
//! - [`grid`]: the two crossed, each cased between its chain's two ends.
//! - [`alone`]: one arrangement asked alone ([`Train::alone`]), the way the
//!   laws about a single part ask it.

#![allow(clippy::unwrap_used)]

use super::arrangements::{self as arr, Preset};
use super::{solve_alone, test_library, Alone, LoadCase, Shape, Train, TrainError};
use crate::params::Auto;

/// The torque a grid train's cases give, N·m: small enough that every
/// arrangement's teeth carry it at their preset widths.
pub const TORQUE_NM: f64 = 1.0;
/// The speed a grid train's cases give, rpm: enough to count cycles.
pub const SPEED_RPM: f64 = 1000.0;
/// The torque an arrangement asked [`alone`] is loaded with, N·m.
pub const ALONE_NM: f64 = 2.0;

/// **Every arrangement**, named: each preset in the menu's order, then the
/// arrangements a designer reaches by edits rather than a button. The hula's
/// grounded ring is left automatic, so the relief laws have a shift to turn
/// on it.
pub fn arrangements() -> Vec<(String, Shape)> {
    let mut hula = arr::hula([65, 61, 57, 61], [1.0, 1.0]);
    hula.members[2].gear.profile_shift = Auto::automatic(0.0);
    Preset::ALL
        .into_iter()
        .map(|p| (format!("{p:?}"), p.build()))
        .chain([
            ("Hula".to_owned(), hula),
            (
                "Ravigneaux".to_owned(),
                arr::ravigneaux([18, 30], [22, 18], 62, 3),
            ),
            (
                "WormAndPair".to_owned(),
                arr::worm_and_pair((1, 40), (17, 43)),
            ),
        ])
        .collect()
}

/// The arrangements' shapes alone, in [`arrangements`]' order.
pub fn shapes() -> Vec<Shape> {
    arrangements().into_iter().map(|(_, s)| s).collect()
}

/// **Where an arrangement sits in the train it is asked in.**
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Context {
    /// A train of one part.
    Alone,
    /// The second part, after a spur pair: its pieces are not the graph's
    /// by the same index.
    AfterSpur,
    /// The first part, with a layshaft after it: an idle branch downstream.
    BeforeLayshaft,
}

impl Context {
    pub const ALL: [Self; 3] = [Self::Alone, Self::AfterSpur, Self::BeforeLayshaft];

    /// The chain's shapes with `shape` placed.
    fn chain(self, shape: Shape) -> Vec<Shape> {
        match self {
            Self::Alone => vec![shape],
            Self::AfterSpur => vec![Preset::Spur.build(), shape],
            Self::BeforeLayshaft => vec![shape, Preset::Layshaft.build()],
        }
    }
}

/// The grid's two cases between two ports: an ultimate load at `input`
/// reacted at `output`, and the same load judged for fatigue over the
/// default duty.
pub fn cases(input: usize, output: usize) -> Vec<LoadCase> {
    vec![
        LoadCase::ultimate(input, output, TORQUE_NM, SPEED_RPM),
        LoadCase::fatigue(input, output, TORQUE_NM, SPEED_RPM),
    ]
}

/// **A train as the panel starts it**: `shapes` chained, [`cases`] between
/// the chain's two ends.
pub fn cased(shapes: Vec<Shape>) -> Train {
    Train::chained(shapes, |t| {
        t.chain_ends().map(|(a, b)| cases(a, b)).unwrap_or_default()
    })
}

/// **One entry of the grid**: an arrangement in a context.
#[derive(Clone, Debug)]
pub struct Entry {
    /// `"<arrangement> <context>"`, for a failure's message.
    pub name: String,
    pub shape: Shape,
    pub context: Context,
}

impl Entry {
    /// The train, cased between its chain's two ends ([`cases`]).
    pub fn train(&self) -> Train {
        self.train_of(self.shape.clone())
    }

    /// The same context around another shape — the arrangement with an
    /// input turned — cased as [`Self::train`].
    pub fn train_of(&self, shape: Shape) -> Train {
        cased(self.context.chain(shape))
    }

    /// The train with no load case: the graph and its holds alone, for a
    /// law about the graph that a rating would only slow.
    pub fn uncased(&self) -> Train {
        Train::chained(self.context.chain(self.shape.clone()), |_| Vec::new())
    }
}

/// **The grid**: every arrangement in every context, arrangement-major.
pub fn grid() -> Vec<Entry> {
    arrangements()
        .into_iter()
        .flat_map(|(arrangement, shape)| {
            Context::ALL.into_iter().map(move |context| Entry {
                name: format!("{arrangement} {context:?}"),
                shape: shape.clone(),
                context,
            })
        })
        .collect()
}

/// The grid's trains, cased, by name.
pub fn trains() -> Vec<(String, Train)> {
    grid()
        .into_iter()
        .map(|e| (e.name.clone(), e.train()))
        .collect()
}

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
