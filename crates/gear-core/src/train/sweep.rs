//! **The trains the laws are swept over**: every arrangement in every
//! context a part meets ([`grid`]), and the seeded walk of offered edits that
//! starts from them ([`step`]). In the library rather than behind
//! `cfg(test)` so `gear-cli identity` records the structure of exactly the
//! trains the laws visit.
//!
//! - [`arrangements`]: every preset, and the arrangements no button lays out
//!   (the hula, a Ravigneaux set, a worm with a pair after it).
//! - [`Context`]: the arrangement alone, after a spur pair, and before a
//!   layshaft — a part whose members, meshes and bodies are not the graph's
//!   by the same index, and a train with an idle branch.
//! - [`grid`]: the two crossed, each cased between its chain's two ends.
//! - [`starts`], [`Lcg`] and [`step`]: the walk — [`WALKS`] walks of
//!   [`DEPTH`] steps, walk `w` from start `w mod n` with seed `w`.

use super::arrangements::{self as arr, Preset};
use super::{CaseKind, Edit, EditRefused, LoadCase, Offer, Shape, Target, Train};
use crate::params::Auto;

/// The torque a grid train's cases give, N·m: small enough that every
/// arrangement's teeth carry it at their preset widths.
pub const TORQUE_NM: f64 = 1.0;
/// The speed a grid train's cases give, rpm: enough to count cycles.
pub const SPEED_RPM: f64 = 1000.0;

/// **Every arrangement**, named: each preset in the menu's order, then the
/// arrangements a designer reaches by edits rather than a button. The hula's
/// grounded ring is left automatic, so the relief laws have a shift to turn
/// on it.
#[must_use]
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
#[must_use]
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
#[must_use]
pub fn cases(input: usize, output: usize) -> Vec<LoadCase> {
    vec![
        LoadCase::ultimate(input, output, TORQUE_NM, SPEED_RPM),
        LoadCase::fatigue(input, output, TORQUE_NM, SPEED_RPM),
    ]
}

/// **A train as the panel starts it**: `shapes` chained, [`cases`] between
/// the chain's two ends.
#[must_use]
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
    #[must_use]
    pub fn train(&self) -> Train {
        self.train_of(self.shape.clone())
    }

    /// The same context around another shape — the arrangement with an
    /// input turned — cased as [`Self::train`].
    #[must_use]
    pub fn train_of(&self, shape: Shape) -> Train {
        cased(self.context.chain(shape))
    }

    /// The train with no load case: the graph and its holds alone, for a
    /// law about the graph that a rating would only slow.
    #[must_use]
    pub fn uncased(&self) -> Train {
        Train::chained(self.context.chain(self.shape.clone()), |_| Vec::new())
    }
}

/// **The grid**: every arrangement in every context, arrangement-major.
#[must_use]
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
#[must_use]
pub fn trains() -> Vec<(String, Train)> {
    grid()
        .into_iter()
        .map(|e| (e.name.clone(), e.train()))
        .collect()
}

// ---------------------------------------------------------------- the walk ---

/// Walks the seeded walk takes.
pub const WALKS: usize = 200;
/// Steps each walk takes.
pub const DEPTH: usize = 4;

/// **Where the walk starts**: every train of the [`grid`], and, but before a
/// layshaft, the same with its first gear's helix given.
#[must_use]
pub fn starts() -> Vec<(String, Train)> {
    let mut out = Vec::new();
    for e in grid() {
        out.push((e.name.clone(), e.train()));
        if e.context == Context::BeforeLayshaft {
            continue;
        }
        out.push((
            format!("{}, helix 20", e.name),
            e.train_of(e.shape.clone().with_first_helix(20.0)),
        ));
    }
    out
}

/// Every piece of the train, and the train.
#[must_use]
pub fn targets(t: &Train) -> Vec<Target> {
    let s = &t.shape;
    std::iter::once(Target::Train)
        .chain((0..s.members.len()).map(Target::Member))
        .chain((0..s.meshes.len()).map(Target::Mesh))
        .chain(s.bodies.iter().map(|b| Target::Body(b.body)))
        .chain((0..s.axes.len()).map(Target::Axis))
        .chain((0..s.distances.len()).map(Target::Distance))
        .chain((0..s.couplings.len()).map(Target::Coupling))
        .collect()
}

/// A fixed linear congruential generator (Knuth's MMIX constants): the
/// walk's one source of choice, so a walk replays from its seed.
pub struct Lcg(pub u64);

impl Lcg {
    /// A choice among `n`, 0 where there is none to make.
    #[allow(
        clippy::cast_possible_truncation,
        reason = "the top 31 bits of a u64 fit a usize on every target this crate builds for"
    )]
    pub fn pick(&mut self, n: usize) -> usize {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 33) as usize % n.max(1)
    }
}

/// One step of a walk: a case added or its duty switched, a hold, a
/// release or a join at a body, or any offer the graph makes at any
/// piece — each as the panel asks it, and said in `log`. `None` where the
/// choice made has nothing to offer; the edit's refusal where an offered
/// edit is not made, which the walk's laws call a failure.
pub fn step(t: &Train, rng: &mut Lcg, log: &mut Vec<String>) -> Option<Result<Train, EditRefused>> {
    let mut u = t.clone();
    match rng.pick(8) {
        0 => {
            let kind = [CaseKind::Ultimate, CaseKind::Fatigue][rng.pick(2)];
            log.push(format!("fresh_case({kind:?})"));
            let case = u.fresh_case(kind, TORQUE_NM, SPEED_RPM);
            u.load_cases.push(case);
            Some(Ok(u))
        }
        1 if !u.load_cases.is_empty() => {
            let (case, intermittent) = (rng.pick(u.load_cases.len()), rng.pick(2) == 0);
            log.push(format!("set_duty({case}, {intermittent})"));
            Some(u.set_duty(case, intermittent).map(|()| u))
        }
        2 | 3 if !u.shape.bodies.is_empty() => {
            let b = u.shape.bodies[rng.pick(u.shape.bodies.len())].body;
            let open: Vec<Edit> = t
                .offers(Target::Body(b))
                .into_iter()
                .filter(|o| o.refused.is_none())
                .map(|o| o.edit)
                .filter(|e| matches!(e, Edit::Hold(_) | Edit::Release(_) | Edit::Join { .. }))
                .collect();
            let edit = open.get(rng.pick(open.len()))?.clone();
            log.push(format!("{edit:?}"));
            Some(u.edit(edit).map(|()| u))
        }
        _ => {
            let at = targets(t);
            let at = at[rng.pick(at.len())];
            let open: Vec<Offer> = t
                .offers(at)
                .into_iter()
                .filter(|o| o.refused.is_none())
                .collect();
            let offer = open.get(rng.pick(open.len()))?;
            let named = match &offer.edit {
                Edit::Insert { at, .. } => format!("Insert {:?} at {at:?}", offer.preset),
                e => format!("{e:?}"),
            };
            log.push(format!("{named} @ {at:?}"));
            Some(u.edit(offer.edit.clone()).map(|()| u))
        }
    }
}

/// **Every train the walk visits**, in order: for each walk its start, then
/// the train after each step that made one, with the steps that led there —
/// a step with nothing to offer leaves the train where it was, and a
/// refused offer ends its walk. What `gear-cli identity` records; the
/// walk's laws take the same steps from the same seeds.
#[must_use]
pub fn visited() -> Vec<(String, Vec<String>, Train)> {
    let starts = starts();
    let mut out = Vec::new();
    for walk in 0..WALKS {
        let (name, start) = &starts[walk % starts.len()];
        let mut rng = Lcg(walk as u64);
        let mut t = start.clone();
        let mut steps: Vec<String> = Vec::new();
        out.push((format!("walk {walk}, {name}"), steps.clone(), t.clone()));
        for _ in 0..DEPTH {
            match step(&t, &mut rng, &mut steps) {
                None => {}
                Some(Ok(u)) => {
                    out.push((format!("walk {walk}, {name}"), steps.clone(), u.clone()));
                    t = u;
                }
                Some(Err(_)) => break,
            }
        }
    }
    out
}
