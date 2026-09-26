//! **What can be done to a piece** — every edit of the graph's set
//! ([`Edit`]) that applies at one piece a designer has selected, generated
//! from the graph and tried on a copy, so an interface lists complete
//! outcomes rather than controls to fill in, and shows a refusal as a verb
//! it cannot press, with its reason (docs/reference.md#the-graph, *what a
//! piece offers*). What each would come to is [`super::preview`]'s to say.
//!
//! **An offer is its edit**: refused exactly where [`Train::edit`] refuses
//! it, and never one that would change nothing — a gear alone on its body
//! moved to a body of its own is on one already. Which edits a piece
//! offers is read off the graph by the rule each edit states: a gear meshes
//! across an axis distance, so it is offered on the axes a distance joins
//! to its mate's; a gear moves among the bodies of its axis; a step goes on
//! a carried axis and a coupling from a body on one. The laws below hold
//! the reading to the edits themselves, swept by brute force.

use super::arrangements::Preset;
use super::preview::unchanged;
use super::{Edit, Piece, Place, Train};
use crate::kinematics::GROUND;
use crate::note::Note;

/// **A piece of the graph a designer can select**, by the graph's index —
/// a body by its number — or the train, where nothing is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub enum Target {
    /// Nothing selected: the presets a train takes at its output.
    Train,
    Member(usize),
    Mesh(usize),
    Body(usize),
    Axis(usize),
    /// An axis distance — a centre, in the list's grouping.
    Distance(usize),
    Coupling(usize),
}

/// **One edit a piece offers**, and why it would be refused, where it would.
#[derive(Clone, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Offer {
    pub edit: Edit,
    /// The preset an [`Edit::Insert`] lays in, for a menu to name it by.
    pub preset: Option<Preset>,
    /// The refusal's catalogue key ([`super::EditRefused::key`]).
    pub refused: Option<Note>,
}

impl Train {
    /// **Every edit the graph offers at `at`**, in the order a menu lists
    /// them — what it adds, then the presets it takes, then what it moves,
    /// joins and holds, then what it takes away — each tried on a copy
    /// ([the module](self)).
    #[must_use]
    pub fn offers(&self, at: Target) -> Vec<Offer> {
        self.candidates(at)
            .into_iter()
            .filter_map(|(edit, preset)| {
                let mut after = self.clone();
                match after.edit(edit.clone()) {
                    Err(refusal) => Some(Offer {
                        edit,
                        preset,
                        refused: Some(Note::new(refusal.key())),
                    }),
                    Ok(()) if unchanged(self, &after) => None,
                    Ok(()) => Some(Offer {
                        edit,
                        preset,
                        refused: None,
                    }),
                }
            })
            .collect()
    }

    /// The edits [`Self::offers`] tries at `at`, before trying them.
    fn candidates(&self, at: Target) -> Vec<(Edit, Option<Preset>)> {
        let s = &self.shape;
        let axis_of = |body: usize| s.bodies.iter().find(|b| b.body == body).map(|b| b.axis);
        // The axes an axis distance joins `axis` to: where a gear on it
        // can mesh.
        let across = |axis: usize| -> Vec<usize> {
            s.distances
                .iter()
                .filter_map(|d| match d.axes {
                    [x, y] if x == axis => Some(y),
                    [x, y] if y == axis => Some(x),
                    _ => None,
                })
                .collect()
        };
        let members_on = |axis: usize| {
            (0..s.members.len()).filter(move |&i| axis_of(s.members[i].body) == Some(axis))
        };
        let carried = |axis: usize| s.axes[axis].carried_by != GROUND;
        let plain = |edit: Edit| (edit, None);
        let gear_and_ring = |mate: usize, on: Place| {
            [false, true].map(|ring| plain(Edit::AddGear { mate, on, ring }))
        };
        let presets = |at: Option<usize>| {
            Preset::ALL.map(|p| {
                (
                    Edit::Insert {
                        shape: p.build(),
                        at,
                    },
                    Some(p),
                )
            })
        };

        let mut out = Vec::new();
        match at {
            Target::Train => out.extend(presets(None)),
            Target::Member(i) if i < s.members.len() => {
                let body = s.members[i].body;
                let axis = axis_of(body);
                out.extend(gear_and_ring(i, Place::NewAxis));
                for other in axis.map(across).unwrap_or_default() {
                    out.extend(gear_and_ring(i, Place::NewBody(other)));
                    for b in s.bodies.iter().filter(|b| b.axis == other) {
                        out.extend(gear_and_ring(i, Place::Body(b.body)));
                    }
                }
                out.push(plain(Edit::Move {
                    member: i,
                    to: None,
                }));
                for b in s
                    .bodies
                    .iter()
                    .filter(|b| Some(b.axis) == axis && b.body != body)
                {
                    out.push(plain(Edit::Move {
                        member: i,
                        to: Some(b.body),
                    }));
                }
                out.push(plain(Edit::Remove(Piece::Member(i))));
            }
            Target::Mesh(k) if k < s.meshes.len() => {
                let m = s.meshes[k];
                if let Some(distance) = s.distance_of(k) {
                    for shared in [s.members[m.a].body, s.members[m.b].body] {
                        out.push(plain(Edit::AddRatio { distance, shared }));
                    }
                }
                out.push(plain(Edit::Remove(Piece::Mesh(k))));
            }
            Target::Body(b) => {
                let Some(axis) = axis_of(b) else {
                    return out;
                };
                for other in across(axis) {
                    for mate in members_on(other) {
                        out.extend(gear_and_ring(mate, Place::Body(b)));
                    }
                }
                if carried(axis) {
                    out.push(plain(Edit::Couple { body: b }));
                }
                out.extend(presets(Some(b)));
                for member in members_on(axis).filter(|&i| s.members[i].body != b) {
                    out.push(plain(Edit::Move {
                        member,
                        to: Some(b),
                    }));
                }
                for x in s.bodies.iter().filter(|x| x.body != b) {
                    out.push(plain(Edit::Join { a: b, b: x.body }));
                }
                out.push(plain(if self.held.contains(&b) {
                    Edit::Release(b)
                } else {
                    Edit::Hold(b)
                }));
                for (c, _) in s
                    .couplings
                    .iter()
                    .enumerate()
                    .filter(|(_, x)| x.contains(&b))
                {
                    out.push(plain(Edit::Remove(Piece::Coupling(c))));
                }
                out.push(plain(Edit::Remove(Piece::Body(b))));
            }
            Target::Axis(a) if a < s.axes.len() => {
                for other in across(a) {
                    for mate in members_on(other) {
                        out.extend(gear_and_ring(mate, Place::NewBody(a)));
                    }
                }
                if carried(a) {
                    out.push(plain(Edit::AddStep { axis: a }));
                }
                out.push(plain(Edit::Remove(Piece::Axis(a))));
            }
            Target::Distance(d) if d < s.distances.len() => {
                for x in s
                    .bodies
                    .iter()
                    .filter(|x| s.distances[d].axes.contains(&x.axis))
                {
                    out.push(plain(Edit::AddRatio {
                        distance: d,
                        shared: x.body,
                    }));
                }
            }
            Target::Coupling(c) if c < s.couplings.len() => {
                out.push(plain(Edit::Remove(Piece::Coupling(c))));
            }
            _ => {}
        }
        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **An offer is its edit, and every edit is offered.** Sound: on every
    //! piece of every preset alone and after a pair, an offer is refused
    //! exactly where its edit is, and made it changes the train and leaves
    //! nothing hanging. Complete: every edit a brute-force sweep finds the
    //! train makes — every gear at every mate and place, every move, join,
    //! hold, removal, ratio, step, coupling and stage at every index — is
    //! offered at the piece it names.

    use super::super::{solve_train, test_library, CaseKind, LoadCase, Shape, TrainError};
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

    /// The figures of the panel's fresh case: a load small enough that
    /// every preset's teeth carry it, at a speed that counts cycles.
    const TORQUE: f64 = 0.1;
    const SPEED: f64 = 30_000.0;

    /// **A train as the panel starts it**: its presets chained, an
    /// ultimate and a fatigue case between the chain's two ends.
    fn cased(stages: Vec<Shape>) -> Train {
        Train::chained(stages, |t| {
            t.chain_ends()
                .map(|(a, b)| {
                    vec![
                        LoadCase::ultimate(a, b, TORQUE, SPEED),
                        LoadCase::fatigue(a, b, TORQUE, SPEED),
                    ]
                })
                .unwrap_or_default()
        })
    }

    /// Every preset alone and after a pair, spur and with its first gear's
    /// helix given, each with its conventional cases.
    fn trains() -> Vec<(String, Train)> {
        let mut out = Vec::new();
        for p in Preset::ALL {
            for before in [None, Some(Preset::Spur)] {
                for helix in [None, Some(20.0)] {
                    let last = p.build();
                    let last = helix.map_or(last.clone(), |b| last.with_first_helix(b));
                    let stages: Vec<Shape> = before
                        .map(Preset::build)
                        .into_iter()
                        .chain(std::iter::once(last))
                        .collect();
                    out.push((
                        format!("{p:?} after {before:?}, helix {helix:?}"),
                        cased(stages),
                    ));
                }
            }
        }
        out
    }

    /// Every piece of the train, and the train.
    fn targets(t: &Train) -> Vec<Target> {
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

    #[test]
    fn an_offer_is_its_edit() {
        let mut offered = 0;
        for (name, t) in trains() {
            for at in targets(&t) {
                for offer in t.offers(at) {
                    let mut u = t.clone();
                    let made = u.edit(offer.edit.clone());
                    let context = format!("{name}: at {at:?}, {:?}", offer.edit);
                    assert_eq!(
                        made.err().map(|e| Note::new(e.key())),
                        offer.refused,
                        "{context}"
                    );
                    if offer.refused.is_none() {
                        assert!(!unchanged(&t, &u), "{context}: changes nothing");
                        u.check().unwrap_or_else(|e| panic!("{context}: {e:?}"));
                    }
                    offered += 1;
                }
            }
        }
        assert!(offered > 3000, "only {offered} offers");
    }

    /// **Every edit the train makes is offered at every piece it names** —
    /// a gear at its mate and at the body or axis it goes on, a move at the
    /// gear and the body it goes to, a ratio at its distance and at each
    /// mesh there on the body it shares, a coupling's removal at the
    /// coupling and at both bodies it joins — swept over every index each edit
    /// takes, the places a gear goes included, without reading the graph
    /// the way [`Train::offers`] does.
    #[test]
    fn every_edit_the_train_makes_is_offered() {
        let mut made = 0;
        for (name, t) in trains() {
            let s = &t.shape;
            let bodies: Vec<usize> = s.bodies.iter().map(|b| b.body).collect();
            let places: Vec<Place> = std::iter::once(Place::NewAxis)
                .chain((0..s.axes.len()).map(Place::NewBody))
                .chain(bodies.iter().map(|&b| Place::Body(b)))
                .collect();
            let mut sweep: Vec<(Edit, Vec<Target>)> = Vec::new();
            for mate in 0..s.members.len() {
                for &on in &places {
                    let at = match on {
                        Place::NewAxis => None,
                        Place::NewBody(a) => Some(Target::Axis(a)),
                        Place::Body(b) => Some(Target::Body(b)),
                    };
                    for ring in [false, true] {
                        let edit = Edit::AddGear { mate, on, ring };
                        sweep.push((
                            edit,
                            [Some(Target::Member(mate)), at]
                                .into_iter()
                                .flatten()
                                .collect(),
                        ));
                    }
                }
                for to in std::iter::once(None).chain(bodies.iter().map(|&b| Some(b))) {
                    let at = [Some(Target::Member(mate)), to.map(Target::Body)];
                    sweep.push((
                        Edit::Move { member: mate, to },
                        at.into_iter().flatten().collect(),
                    ));
                }
                sweep.push((
                    Edit::Remove(Piece::Member(mate)),
                    vec![Target::Member(mate)],
                ));
            }
            for k in 0..s.meshes.len() {
                sweep.push((Edit::Remove(Piece::Mesh(k)), vec![Target::Mesh(k)]));
            }
            for d in 0..s.distances.len() {
                for &shared in &bodies {
                    let on_shared = |i: usize| s.members[i].body == shared;
                    let meshes = s
                        .meshes_on(d)
                        .into_iter()
                        .filter(|&k| on_shared(s.meshes[k].a) || on_shared(s.meshes[k].b));
                    let at = std::iter::once(Target::Distance(d)).chain(meshes.map(Target::Mesh));
                    sweep.push((
                        Edit::AddRatio {
                            distance: d,
                            shared,
                        },
                        at.collect(),
                    ));
                }
            }
            for a in 0..s.axes.len() {
                sweep.push((Edit::AddStep { axis: a }, vec![Target::Axis(a)]));
                sweep.push((Edit::Remove(Piece::Axis(a)), vec![Target::Axis(a)]));
            }
            for (c, joined) in s.couplings.iter().enumerate() {
                let at = std::iter::once(Target::Coupling(c)).chain(joined.map(Target::Body));
                sweep.push((Edit::Remove(Piece::Coupling(c)), at.collect()));
            }
            for &b in &bodies {
                let at = vec![Target::Body(b)];
                sweep.push((Edit::Couple { body: b }, at.clone()));
                sweep.push((Edit::Hold(b), at.clone()));
                sweep.push((Edit::Release(b), at.clone()));
                sweep.push((Edit::Remove(Piece::Body(b)), at.clone()));
                for &x in &bodies {
                    sweep.push((Edit::Join { a: b, b: x }, at.clone()));
                }
                for p in Preset::ALL {
                    let shape = p.build();
                    sweep.push((Edit::Insert { shape, at: Some(b) }, at.clone()));
                }
            }
            for p in Preset::ALL {
                let shape = p.build();
                sweep.push((Edit::Insert { shape, at: None }, vec![Target::Train]));
            }
            let offered: Vec<(Target, Vec<String>)> = targets(&t)
                .into_iter()
                .map(|at| {
                    let offers = t.offers(at).into_iter();
                    let open = offers.filter(|o| o.refused.is_none());
                    (at, open.map(|o| format!("{:?}", o.edit)).collect())
                })
                .collect();
            for (edit, at) in sweep {
                let mut u = t.clone();
                if u.edit(edit.clone()).is_err() || unchanged(&t, &u) {
                    continue;
                }
                for piece in at {
                    let (_, open) = offered.iter().find(|(x, _)| *x == piece).unwrap();
                    assert!(
                        open.contains(&format!("{edit:?}")),
                        "{name}: {edit:?} is made but not offered at {piece:?}"
                    );
                }
                made += 1;
            }
        }
        assert!(made > 1000, "only {made} edits made");
    }

    /// **No offered gear meshes in two frames.** A gear on the input shaft
    /// meshing a set's sun — which meshes its planets in the carrier's
    /// frame — was offered and made, and the train then failed as a whole
    /// with a wiring fault said to be a preset's. It is refused now, with
    /// its own reason; the frame per mesh that would let it stand is the
    /// graph's redesign.
    #[test]
    fn a_gear_meshing_in_two_frames_is_refused() {
        let t = cased(vec![Preset::Spur.build(), Preset::Planetary.build()]);
        let sun = t.member(1, 0);
        let edit = Edit::AddGear {
            mate: sun,
            on: Place::Body(1),
            ring: false,
        };
        let offer = t
            .offers(Target::Member(sun))
            .into_iter()
            .find(|o| format!("{:?}", o.edit) == format!("{edit:?}"))
            .unwrap();
        assert_eq!(
            offer.refused,
            Some(Note::new("ui.train_edit_refused_two_frames"))
        );
    }

    /// **A ring on crossed shafts is no mesh**, and is refused: the screw
    /// model has no internal kind.
    #[test]
    fn a_ring_across_crossed_shafts_is_refused() {
        let t = cased(vec![Preset::Worm.build()]);
        let mut u = t.clone();
        assert_eq!(
            u.edit(Edit::AddGear {
                mate: 0,
                on: Place::Body(2),
                ring: true,
            }),
            Err(super::super::EditRefused::WrongFamily)
        );
    }

    /// A fixed linear congruential generator (Knuth's MMIX constants): the
    /// walk's one source of choice, so a failure replays from its seed.
    struct Lcg(u64);

    impl Lcg {
        fn pick(&mut self, n: usize) -> usize {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            usize::try_from(self.0 >> 33).unwrap() % n.max(1)
        }
    }

    /// Whether a solve failed as a wiring that describes no mechanism —
    /// which an edit the graph offers must never make.
    fn wiring(e: &TrainError) -> bool {
        match e {
            TrainError::Wiring(_) => true,
            TrainError::InPart { cause, .. } => wiring(cause),
            _ => false,
        }
    }

    /// One step of a walk: a case added or its duty switched, a hold, a
    /// release or a join at a body, or any offer the graph makes at any
    /// piece — each as the panel asks it. `None` where the choice made
    /// has nothing to offer.
    fn step(t: &Train, rng: &mut Lcg, log: &mut Vec<String>) -> Option<Train> {
        let mut u = t.clone();
        match rng.pick(8) {
            0 => {
                let kind = [CaseKind::Ultimate, CaseKind::Fatigue][rng.pick(2)];
                log.push(format!("fresh_case({kind:?})"));
                let case = u.fresh_case(kind, TORQUE, SPEED);
                u.load_cases.push(case);
                Some(u)
            }
            1 if !u.load_cases.is_empty() => {
                let (case, intermittent) = (rng.pick(u.load_cases.len()), rng.pick(2) == 0);
                log.push(format!("set_duty({case}, {intermittent})"));
                u.set_duty(case, intermittent);
                Some(u)
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
                u.edit(edit).unwrap();
                Some(u)
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
                u.edit(offer.edit.clone()).unwrap();
                Some(u)
            }
        }
    }

    /// What a walk asserts of the train after every step, `solved` saying
    /// whether the train before it solved: the train well formed; offers
    /// at the train and a solve that do not panic; a train that solved
    /// kept solving or refused by a named reason other than a wiring that
    /// describes no mechanism; every flow saying each body and mesh once;
    /// and the train the same after a trip through JSON.
    fn laws(t: &Train, solved: bool) -> Result<bool, String> {
        let lib = test_library();
        t.check().map_err(|e| format!("check: {e:?}"))?;
        let _ = t.offers(Target::Train);
        let r = solve_train(t, &lib);
        match &r {
            Err(e) if solved && wiring(e) => return Err(format!("solved, then {e:?}")),
            Ok(r) => super::super::groupings::says_everything_once(t, r)?,
            Err(_) => {}
        }
        #[cfg(feature = "serde")]
        {
            let json = serde_json::to_string(t).unwrap();
            let back: Train = serde_json::from_str(&json).map_err(|e| format!("json: {e}"))?;
            if serde_json::to_string(&back).unwrap() != json {
                return Err("json: a round trip changed the train".into());
            }
        }
        Ok(r.is_ok())
    }

    /// **A seeded walk over cased trains** — every start in [`trains`],
    /// [`WALKS`] walks of [`DEPTH`] steps each ([`step`]) — the train
    /// held to [`laws`] after every step, and the steps that led to any
    /// failure printed. A walk crosses what one edit on a fresh preset
    /// never reaches: an emptied train given a preset, a hold on a body a
    /// case names, a join onto a held body.
    #[test]
    fn a_walk_of_offered_edits_keeps_the_train_whole() {
        const WALKS: usize = 200;
        const DEPTH: usize = 4;
        let starts = trains();
        let mut failures: Vec<String> = Vec::new();
        for walk in 0..WALKS {
            let (name, start) = &starts[walk % starts.len()];
            let mut rng = Lcg(walk as u64);
            let mut t = start.clone();
            let mut steps: Vec<String> = Vec::new();
            let mut solved = solve_train(&t, &test_library()).is_ok();
            for _ in 0..DEPTH {
                let made = catch_unwind(AssertUnwindSafe(|| {
                    let u = step(&t, &mut rng, &mut steps)?;
                    Some((laws(&u, solved), u))
                }));
                match made {
                    Ok(None) => {}
                    Ok(Some((held, u))) => match held {
                        Ok(s) => (t, solved) = (u, s),
                        Err(e) => {
                            failures.push(format!("walk {walk}, {name}: {steps:?}: {e}"));
                            break;
                        }
                    },
                    Err(panic) => {
                        let what = panic
                            .downcast_ref::<String>()
                            .cloned()
                            .or_else(|| panic.downcast_ref::<&str>().map(ToString::to_string));
                        failures.push(format!(
                            "walk {walk}, {name}: {steps:?} then panicked: {what:?}"
                        ));
                        break;
                    }
                }
            }
        }
        assert!(
            failures.is_empty(),
            "{} of {WALKS} walks failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// The train emptied by offered removals, each time the first (or the
    /// last) one offered anywhere, until no gear is left.
    fn emptied(mut t: Train, last: bool) -> Train {
        while !t.shape.members.is_empty() {
            let removals: Vec<Edit> = targets(&t)
                .into_iter()
                .flat_map(|at| t.offers(at))
                .filter(|o| o.refused.is_none() && matches!(o.edit, Edit::Remove(_)))
                .map(|o| o.edit)
                .collect();
            let edit = if last {
                removals.last()
            } else {
                removals.first()
            };
            t.edit(edit.unwrap().clone()).unwrap();
        }
        t
    }

    /// What a case says, entry by entry, and where its sweep is measured.
    fn said(t: &Train) -> Vec<String> {
        t.load_cases
            .iter()
            .map(|c| {
                let entries: Vec<_> = c.loads.iter().map(|l| (l.at, l.role)).collect();
                format!("{entries:?}, {:?}", c.duty)
            })
            .collect()
    }

    /// **An emptied train lists nothing, and the next preset takes its
    /// cases up as a fresh train of it would have them.** Every preset,
    /// emptied by the removals it offers in two orders: the train offers
    /// its presets without panicking, and every preset laid in holds what
    /// it holds alone, reads its cases load in and reaction out, and
    /// solves them as it does alone — wherever the removals kept both of
    /// a case's entries. A removal that takes the body an entry is at
    /// drops the entry, which is a rule of its own (audit T13.4); there
    /// the preset laid in is only held well formed.
    #[test]
    fn an_emptied_train_takes_the_next_preset_as_it_would_come() {
        let lib = test_library();
        let mut whole = 0;
        for p in Preset::ALL {
            for last in [false, true] {
                let t = emptied(cased(vec![p.build()]), last);
                assert!(t.shape.bodies.is_empty(), "{p:?}: {:?}", t.shape.bodies);
                assert!(!t.offers(Target::Train).is_empty());
                let kept = t.load_cases.iter().all(|c| c.loads.len() == 2);
                whole += usize::from(kept);
                for q in Preset::ALL {
                    let mut u = t.clone();
                    u.edit(Edit::Insert {
                        shape: q.build(),
                        at: None,
                    })
                    .unwrap();
                    u.check().unwrap();
                    if !kept {
                        continue;
                    }
                    let fresh = cased(vec![q.build()]);
                    let context = format!("{p:?} emptied (last {last}), then {q:?}");
                    assert_eq!(u.held, fresh.held, "{context}");
                    assert_eq!(said(&u), said(&fresh), "{context}");
                    assert_eq!(u.headline(), fresh.headline(), "{context}");
                    let solved = |t: &Train| {
                        solve_train(t, &lib)
                            .map(|r| r.cases.iter().map(|c| c.solved).collect::<Vec<_>>())
                    };
                    assert_eq!(solved(&u), solved(&fresh), "{context}");
                }
            }
        }
        assert!(whole >= Preset::ALL.len(), "only {whole} emptied whole");
    }

    /// **A hold leaves no case anything to say of the body.** A pair's
    /// output held takes its reaction and its sweep with it. A set's
    /// fatigue sweep measured at its ring, the ring then held: the sweep
    /// moves to the case's reaction and the set counts cycles, where it
    /// counted none, saying nothing. A join that would hold a body a case
    /// reacts at is refused — it once left the reaction on ground.
    #[test]
    fn a_hold_moves_the_sweep_and_a_join_does_not_ground_a_reaction() {
        let lib = test_library();
        let mut t = cased(vec![Preset::Planetary.build()]);
        let ring = t.port(0, 3);
        let reaction = t.load_cases[1].loads[1].at;
        t.edit(Edit::Release(ring)).unwrap();
        t.load_cases[1].duty = super::super::Duty::intermittent(ring);
        t.edit(Edit::Hold(ring)).unwrap();
        assert!(
            matches!(t.load_cases[1].duty, super::super::Duty::Intermittent { at, .. } if at == reaction)
        );
        let r = solve_train(&t, &lib).unwrap();
        for m in &r.members {
            let cycles = m.cases[1].cycles.unwrap();
            assert!(cycles.bending > 0.0, "{cycles:?}");
        }
        // A hold at a pair's reacted output is made, and the case keeps
        // nothing there: its sweep goes to what is left, the load.
        let mut t = cased(vec![Preset::Spur.build()]);
        t.edit(Edit::Hold(2)).unwrap();
        for c in &t.load_cases {
            assert!(c.loads.iter().all(|l| l.at == 1), "{c:?}");
            assert!(matches!(
                c.duty,
                super::super::Duty::Intermittent { at: 1, .. }
            ));
        }
        // A pair chained on a pair at its input, its far end held, then
        // that end joined to the first pair's reacted output.
        let mut t = cased(vec![Preset::Spur.build()]);
        t.edit(Edit::Insert {
            shape: Preset::Spur.build(),
            at: Some(1),
        })
        .unwrap();
        t.edit(Edit::Hold(3)).unwrap();
        let before = format!("{t:?}");
        assert_eq!(
            t.edit(Edit::Join { a: 2, b: 3 }),
            Err(super::super::EditRefused::Loaded)
        );
        assert_eq!(format!("{t:?}"), before, "refused whole");
    }
}
