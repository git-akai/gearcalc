//! **What can be done to a piece** — every edit of the graph's set
//! ([`Edit`]) that applies at one piece a designer has selected, generated
//! from the graph and tried on a copy, so an interface lists complete
//! outcomes rather than controls to fill in, and shows a refusal as a verb
//! it cannot press, with its reason (docs/reference.md#the-graph, *what a
//! piece offers*). What each would come to is [`super::preview()`]'s to say.
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
#[cfg_attr(feature = "serde", serde(deny_unknown_fields))]
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
                if let Some(distance) = s.indexed().distance_of(k) {
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
    //! piece of every train of the grid, an offer is refused
    //! exactly where its edit is, and made it changes the train and leaves
    //! nothing hanging. Complete: every edit a brute-force sweep finds the
    //! train makes — every gear at every mate and place, every move, join,
    //! hold, removal, ratio, step, coupling and stage at every index — is
    //! offered at the piece it names.

    use super::super::sweep::{starts as trains, step, targets, Lcg, DEPTH, WALKS};
    use super::super::testing::cased;
    use super::super::{solve_train, test_library, LoadRole, TrainError};
    use super::*;
    use std::panic::{catch_unwind, AssertUnwindSafe};

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
                    // The one key that says no cause — an edit asked of the
                    // wrong level — is never what a piece offers.
                    assert_ne!(
                        offer.refused,
                        Some(Note::new(super::super::EditRefused::WrongFamily.key())),
                        "{context}"
                    );
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
                    let at = s.indexed();
                    let meshes = at
                        .meshes_on(d)
                        .iter()
                        .copied()
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
        let sun = t.member(1, 0).expect("a set's sun");
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
            Err(super::super::EditRefused::RingCrossed)
        );
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

    /// **Every fatigue case that solves counts every gear's cycles, or says
    /// why it counts none** — its sweep unset, or at a body that does not
    /// turn — and never both (audit T13.5).
    fn counts_or_says_why(t: &Train, r: &super::super::TrainResult) -> Result<(), String> {
        use crate::note::key;
        for (i, c) in r.cases.iter().enumerate() {
            let case = &t.load_cases[i];
            if !c.solved || !case.enabled || case.counted().is_none() {
                continue;
            }
            let said = c
                .notes
                .iter()
                .any(|n| n.is(key::TRAIN_DUTY_UNSET) || n.is(key::TRAIN_DUTY_AT_STILL));
            let counted = r.members.iter().all(|m| {
                m.cases
                    .iter()
                    .find(|g| g.case == i)
                    .is_some_and(|g| g.cycles.is_some())
            });
            if said == counted {
                return Err(format!(
                    "case {i}: counts {counted} and says why not {said}"
                ));
            }
        }
        Ok(())
    }

    /// **A fresh case names only open ports** (audit T13.6): of either
    /// kind, off, its entries and its sweep at open ports — at numbers the
    /// graph does not list, parked, on an empty train.
    fn a_fresh_case_names_open_ports(t: &Train) -> Result<(), String> {
        let open: Vec<usize> = t.open_ports().iter().map(|p| p.body).collect();
        let listed = |b: usize| t.shape.bodies.iter().any(|x| x.body == b);
        for kind in super::super::CaseKind::BOTH {
            let (torque, speed) = kind.fresh_figures();
            let f = t.fresh_case(kind, torque, speed);
            let sweep = match f.duty {
                super::super::Duty::Intermittent { at, .. } => at,
                super::super::Duty::Continuous { .. } => None,
            };
            let fits = |b: usize| {
                if t.shape.members.is_empty() {
                    !listed(b)
                } else {
                    open.contains(&b)
                }
            };
            if f.enabled || !f.loads.iter().map(|l| l.at).chain(sweep).all(fits) {
                return Err(format!("fresh {kind:?}: {f:?}"));
            }
        }
        Ok(())
    }

    /// **A fresh case, switched off, moves no answer** (audit T13.6): the
    /// train with a fresh fatigue case pushed — the kind whose sweep a
    /// solve reads — solves exactly where it solved without it.
    fn a_fresh_case_moves_nothing(t: &Train, solved: bool) -> Result<(), String> {
        let kind = super::super::CaseKind::Fatigue;
        let (torque, speed) = kind.fresh_figures();
        let mut u = t.clone();
        u.load_cases.push(t.fresh_case(kind, torque, speed));
        if solve_train(&u, &test_library()).is_ok() == solved {
            Ok(())
        } else {
            Err("a fresh case, pushed, moved the answer".into())
        }
    }

    /// **A body the graph lists is bare only where something names it**
    /// (audit T13.12): a hold or a case's entry or sweep; one bare and
    /// named by nothing is a number nothing would read again.
    fn nothing_bare_unnamed(t: &Train) -> Result<(), String> {
        let named = |b: usize| {
            t.held.contains(&b)
                || t.load_cases.iter().any(|c| {
                    c.loads.iter().any(|l| l.at == b)
                        || matches!(c.duty, super::super::Duty::Intermittent { at: Some(at), .. } if at == b)
                })
        };
        match t
            .shape
            .bodies
            .iter()
            .find(|b| t.shape.is_bare(b.body) && !named(b.body))
        {
            Some(b) => Err(format!("body {} is bare and named by nothing", b.body)),
            None => Ok(()),
        }
    }

    /// What a walk asserts of the train after every step, `solved` saying
    /// whether the train before it solved: the train well formed; offers
    /// at the train and a solve that do not panic; a train that solved
    /// kept solving or refused by a named reason other than a wiring that
    /// describes no mechanism; every flow saying each body and mesh once;
    /// every fatigue case counting its cycles or saying why not; a fresh
    /// case fitting it; no step (`step`, as the walk logs it) locking a
    /// train that turned; no body bare and named by nothing; and the train
    /// the same after a trip through JSON.
    fn laws(t: &Train, solved: bool, step: &str) -> Result<bool, String> {
        let lib = test_library();
        t.check().map_err(|e| format!("check: {e:?}"))?;
        let _ = t.offers(Target::Train);
        let r = solve_train(t, &lib);
        match &r {
            Err(e) if solved && wiring(e) => return Err(format!("solved, then {e:?}")),
            // No step locks a train that turned (audit T13.9, `Locks`).
            Err(e @ TrainError::Overdetermined { .. }) if solved => {
                return Err(format!("solved, then {step} locked it: {e:?}"));
            }
            Ok(r) => {
                super::super::groupings::says_everything_once(t, r)?;
                super::super::rating_laws::held_tips_reach_past_nothing(t, r)?;
                counts_or_says_why(t, r)?;
            }
            Err(_) => {}
        }
        a_fresh_case_names_open_ports(t)?;
        a_fresh_case_moves_nothing(t, r.is_ok())?;
        nothing_bare_unnamed(t)?;
        let json = serde_json::to_string(t).unwrap();
        let back: Train = serde_json::from_str(&json).map_err(|e| format!("json: {e}"))?;
        if serde_json::to_string(&back).unwrap() != json {
            return Err("json: a round trip changed the train".into());
        }
        Ok(r.is_ok())
    }

    /// **What each case states, where**: its loads and reactions, figures
    /// and all, and its sweep — each body read through `map`, the number
    /// a body had before an edit to the one it has after.
    fn stated(t: &Train, map: &dyn Fn(usize) -> Option<usize>) -> Vec<String> {
        t.load_cases
            .iter()
            .map(|c| {
                let said: Vec<_> = c
                    .loads
                    .iter()
                    .filter(|l| l.role != LoadRole::Free)
                    .map(|l| (map(l.at), l.role, l.torque, l.speed))
                    .collect();
                let sweep = match c.duty {
                    super::super::Duty::Intermittent { at, .. } => at.map(map),
                    super::super::Duty::Continuous { .. } => None,
                };
                format!("{said:?} sweep {sweep:?}")
            })
            .collect()
    }

    /// **No edit drops or moves what a case states** (plan decision 6):
    /// every load, reaction and sweep of every case stands, after the edit,
    /// at the body its body became — the edit's own renumbering, so a
    /// figure moved to another body, or dropped, fails here where a count
    /// of them would not. A preset laid on at the chain's end carries the
    /// entries at the end it joins to its own output, which is the insert's
    /// stated rule (`Train::chain_on`), and an emptied train's first preset
    /// takes its parked cases up: those two are held to the count alone.
    fn keeps_what_it_states(
        before: &Train,
        after: &Train,
        map: &[Option<usize>],
        step: &str,
    ) -> Result<(), String> {
        let followed = |b: usize| map.get(b).copied().flatten();
        let (was, now) = (stated(before, &followed), stated(after, &|b| Some(b)));
        let carried = step.starts_with("Insert") && step.contains("at None")
            || before.shape.members.is_empty();
        let count = |t: &Train| -> Vec<usize> {
            t.load_cases
                .iter()
                .map(|c| c.loads.iter().filter(|l| l.role != LoadRole::Free).count())
                .collect()
        };
        if carried && count(before) == count(after) || was == now {
            Ok(())
        } else {
            Err(format!(
                "a stated figure moved or dropped: {was:?} -> {now:?}"
            ))
        }
    }

    /// **The walks `seeds`, `depth` steps each**, the train held to
    /// [`laws`] after every step and to [`keeps_what_it_states`] after
    /// every edit: each failure, with the walk's start and the steps that
    /// led to it.
    fn walk_failures(seeds: std::ops::Range<usize>, depth: usize) -> Vec<String> {
        let starts = trains();
        let mut failures: Vec<String> = Vec::new();
        for walk in seeds {
            let (name, start) = &starts[walk % starts.len()];
            let mut rng = Lcg(walk as u64);
            let mut t = start.clone();
            let mut steps: Vec<String> = Vec::new();
            let mut solved = solve_train(&t, &test_library()).is_ok();
            for _ in 0..depth {
                let made = catch_unwind(AssertUnwindSafe(|| {
                    let u = step(&t, &mut rng, &mut steps)?
                        .unwrap_or_else(|e| panic!("an offered edit refused: {e:?}"));
                    let last = steps.last().cloned().unwrap_or_default();
                    let held = laws(&u.train, solved, &last).and_then(|s| {
                        match &u.renumbered {
                            Some(map) => keeps_what_it_states(&t, &u.train, map, &last),
                            None => Ok(()),
                        }
                        .map(|()| s)
                    });
                    Some((held, u.train))
                }));
                match made {
                    Ok(None) => {}
                    Ok(Some((Ok(s), u))) => (t, solved) = (u, s),
                    Ok(Some((Err(e), _))) => {
                        failures.push(format!("walk {walk}, {name}: {steps:?}: {e}"));
                        break;
                    }
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
        failures
    }

    /// **A seeded walk over cased trains** — every start in [`trains`],
    /// [`WALKS`] walks of [`DEPTH`] steps each ([`step`]) — the train
    /// held to [`laws`] after every step, and the steps that led to any
    /// failure printed. A walk crosses what one edit on a fresh preset
    /// never reaches: an emptied train given a preset, a hold on a body a
    /// case names, a join onto a held body.
    #[test]
    fn a_walk_of_offered_edits_keeps_the_train_whole() {
        let failures = walk_failures(0..WALKS, DEPTH);
        assert!(
            failures.is_empty(),
            "{} of {WALKS} walks failed:\n{}",
            failures.len(),
            failures.join("\n")
        );
    }

    /// **A join never lines a ring's mesh up across crossed axes** (Q6): of
    /// the 36 walks of 0..3600 at depth 8 that ended in a join breaking the
    /// graph's invariant that a ring meshes on parallel axes (`RingCrossed`) —
    /// a ring added on a new axis, then two bodies joined so its distance is
    /// the crossed one. Before the invariant read a mesh's kind, the join was
    /// made and the ring wired as an external gear. The join is refused now,
    /// under the key adding a ring there is refused by.
    #[test]
    fn a_join_never_crosses_a_ring() {
        // Eight of them, the shortest; `a_deeper_walk_keeps_the_train_whole`
        // over 0..3600 walks the rest.
        const CROSSED: [usize; 8] = [583, 669, 711, 714, 1254, 1371, 1772, 2213];
        let failures: Vec<String> = CROSSED
            .iter()
            .flat_map(|&w| walk_failures(w..w + 1, 8))
            .collect();
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// **The walk, deeper and wider**, by hand: `WALK_SEEDS=from..to` and
    /// `WALK_DEPTH=n` (depth 8 where unset) — what a checker runs to look
    /// past the default walk's reach, e.g. `WALK_SEEDS=0..3600 cargo
    /// nextest run --run-ignored only -E 'test(a_deeper_walk)'`. Without
    /// `WALK_SEEDS` it walks nothing: CI's serial run of the ignored tests
    /// is for the timing canaries.
    #[test]
    #[ignore = "by hand, with WALK_SEEDS set: minutes"]
    fn a_deeper_walk_keeps_the_train_whole() {
        let Ok(seeds) = std::env::var("WALK_SEEDS") else {
            return;
        };
        let (from, to) = seeds.split_once("..").expect("WALK_SEEDS=from..to");
        let depth: usize = std::env::var("WALK_DEPTH").map_or(8, |d| d.parse().unwrap());
        let seeds = from.parse().unwrap()..to.parse().unwrap();
        let count = seeds.len();
        let failures = walk_failures(seeds, depth);
        assert!(
            failures.is_empty(),
            "{} of {count} walks failed:\n{}",
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
    /// emptied by the removals it offers in two orders: every case keeps
    /// both its entries — a removal that would take one away is refused
    /// (plan decision 6), and the one that empties the train parks them —
    /// the train offers its presets without panicking, and every preset
    /// laid in holds what it holds alone, reads its cases load in and
    /// reaction out, and solves them as it does alone.
    #[test]
    fn an_emptied_train_takes_the_next_preset_as_it_would_come() {
        let lib = test_library();
        let mut laid = 0;
        for p in Preset::ALL {
            for last in [false, true] {
                let t = emptied(cased(vec![p.build()]), last);
                assert!(t.shape.bodies.is_empty(), "{p:?}: {:?}", t.shape.bodies);
                assert!(!t.offers(Target::Train).is_empty());
                assert!(
                    t.load_cases.iter().all(|c| c.loads.len() == 2),
                    "{p:?} emptied (last {last}): {:?}",
                    t.load_cases
                );
                for q in Preset::ALL {
                    let mut u = t.clone();
                    u.edit(Edit::Insert {
                        shape: q.build(),
                        at: None,
                    })
                    .unwrap();
                    u.check().unwrap();
                    laid += 1;
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
        assert_eq!(laid, 2 * Preset::ALL.len() * Preset::ALL.len());
    }

    /// **A sweep is a stated figure, never moved** (plan decision 6, the
    /// orchestrator's decision (a)): a set's fatigue sweep measured at its
    /// released ring, the ring then held, is refused under `Loaded` — it
    /// moved the sweep to the case's reaction, a body the designer did not
    /// name — the train unchanged. A join that would hold a body a case
    /// reacts at is refused too: it once left the reaction on ground.
    #[test]
    fn a_hold_keeps_the_sweep_and_a_join_does_not_ground_a_reaction() {
        let mut t = cased(vec![Preset::Planetary.build()]);
        let ring = t.port(0, 3);
        t.edit(Edit::Release(ring)).unwrap();
        t.load_cases[1].duty = super::super::Duty::intermittent(Some(ring));
        let before = format!("{t:?}");
        assert_eq!(
            t.edit(Edit::Hold(ring)),
            Err(super::super::EditRefused::Loaded)
        );
        assert_eq!(format!("{t:?}"), before, "refused whole");
        // A set's held ring joined to the output of a pair after it, which
        // the case reacts at.
        let mut t = cased(vec![Preset::Planetary.build(), Preset::Spur.build()]);
        let (ring, output) = (t.port(0, 3), t.port(1, 2));
        assert!(t.held.contains(&ring));
        assert!(t.load_cases[0]
            .loads
            .iter()
            .any(|l| l.at == output && l.role == LoadRole::Reacted));
        let before = format!("{t:?}");
        assert_eq!(
            t.edit(Edit::Join { a: ring, b: output }),
            Err(super::super::EditRefused::Loaded)
        );
        assert_eq!(format!("{t:?}"), before, "refused whole");
    }

    /// **A load is never dropped by a join or a hold**: a hold at a body a
    /// case loads or reacts at, and a join that would put two entries of
    /// one case on one body, are refused under one key, the train
    /// unchanged — and offered as refused.
    #[test]
    fn a_load_is_never_dropped_by_a_join_or_a_hold() {
        let loaded = Some(Note::new(super::super::EditRefused::Loaded.key()));
        let refused = |t: &Train, at: Target, edit: Edit| {
            let mut u = t.clone();
            assert_eq!(
                u.edit(edit.clone()),
                Err(super::super::EditRefused::Loaded),
                "{edit:?}"
            );
            assert_eq!(
                format!("{u:?}"),
                format!("{t:?}"),
                "{edit:?}: refused whole"
            );
            let offer = t
                .offers(at)
                .into_iter()
                .find(|o| format!("{:?}", o.edit) == format!("{edit:?}"))
                .unwrap();
            assert_eq!(offer.refused, loaded, "{edit:?}");
        };
        // Two pairs chained, loaded at 1 and reacted at 3: one body would
        // carry both.
        let t = cased(vec![Preset::Spur.build(), Preset::Spur.build()]);
        refused(&t, Target::Body(1), Edit::Join { a: 1, b: 3 });
        // A pair: its input and its output each carry an entry.
        let t = cased(vec![Preset::Spur.build()]);
        refused(&t, Target::Body(1), Edit::Hold(1));
        refused(&t, Target::Body(2), Edit::Hold(2));
    }

    /// **A removal never drops or moves a load, and an insert undoes**
    /// (audit T13.4, plan decision 6). A cased pair with each preset laid on
    /// at its output — the reaction carried to the preset's output — and
    /// every removal offered anywhere: made, every load, reaction and sweep
    /// each case states stands at the body its body became, read through
    /// the removal's own renumbering ([`stated`]); where it would take one
    /// away, it is refused whole under `Loaded`. Then the reaction moved
    /// back to the pair's output, and the preset's gears removed through
    /// what is offered: the pair comes back, its cases field for field, and
    /// every case solves.
    #[test]
    fn a_removal_never_drops_a_load_and_an_insert_undoes() {
        let lib = test_library();
        let (mut refused, mut kept) = (0, 0);
        for p in Preset::ALL {
            let pair = cased(vec![Preset::Spur.build()]);
            let mut t = pair.clone();
            t.edit(Edit::Insert {
                shape: p.build(),
                at: None,
            })
            .unwrap();
            for at in targets(&t) {
                for o in t.offers(at) {
                    if !matches!(o.edit, Edit::Remove(_)) {
                        continue;
                    }
                    let mut u = t.clone();
                    let made = u.edit_renumbered(o.edit.clone());
                    let context = format!("{p:?} on a pair, {:?}", o.edit);
                    match made {
                        Ok(_) if u.shape.members.is_empty() => {}
                        Ok(map) => {
                            let followed = |b: usize| map.get(b).copied().flatten();
                            assert_eq!(
                                stated(&u, &|b| Some(b)),
                                stated(&t, &followed),
                                "{context}: a stated figure moved or dropped"
                            );
                            kept += 1;
                        }
                        Err(super::super::EditRefused::Loaded) => {
                            assert_eq!(format!("{u:?}"), format!("{t:?}"), "{context}");
                            refused += 1;
                        }
                        Err(_) => {}
                    }
                }
            }
            // The reaction back at the pair's output, then the preset's
            // gears taken off one offered removal at a time.
            let output = pair.chain_ends().unwrap().1;
            let moved = t.chain_ends().unwrap().1;
            for c in &mut t.load_cases {
                for l in &mut c.loads {
                    if l.at == moved {
                        l.at = output;
                    }
                }
                if let super::super::Duty::Intermittent { at, .. } = &mut c.duty {
                    if *at == Some(moved) {
                        *at = Some(output);
                    }
                }
            }
            let n = pair.shape.members.len();
            while t.shape.members.len() > n {
                let offered = |i: usize| {
                    let removal = format!("{:?}", Edit::Remove(Piece::Member(i)));
                    t.offers(Target::Member(i))
                        .iter()
                        .any(|o| o.refused.is_none() && format!("{:?}", o.edit) == removal)
                };
                let i = (n..t.shape.members.len())
                    .find(|&i| offered(i))
                    .unwrap_or_else(|| panic!("{p:?}: no removal of its gears offered"));
                t.edit(Edit::Remove(Piece::Member(i))).unwrap();
            }
            assert_eq!(
                format!("{:?}", t.shape),
                format!("{:?}", pair.shape),
                "{p:?}"
            );
            assert_eq!(t.load_cases, pair.load_cases, "{p:?}: the cases");
            let r = solve_train(&t, &lib).unwrap();
            assert!(r.cases.iter().all(|c| c.solved), "{p:?}: {:?}", r.cases);
        }
        assert!(refused > 0 && kept > 0, "refused {refused}, kept {kept}");
    }

    /// **A fresh case lands on open ports wherever the train has them**
    /// (audit T13.6): on every train the walk visits, and on each with its
    /// cases taken away — so no headline says where a case runs and, where
    /// the walk's edits left the chain without two ends, nothing but the
    /// open ports can — a fresh case of either kind names only open ports,
    /// or parked numbers on an empty train. Without the open ports to fall
    /// back on, a case landed past the last body and the train refused it.
    #[test]
    fn a_fresh_case_lands_on_open_ports() {
        let (mut checked, mut endless) = (0, 0);
        for (name, steps, t) in super::super::sweep::visited() {
            let mut bare = t.clone();
            bare.load_cases.clear();
            if bare.chain_ends().is_none() && !bare.shape.members.is_empty() {
                endless += 1;
            }
            for u in [&t, &bare] {
                a_fresh_case_names_open_ports(u)
                    .unwrap_or_else(|e| panic!("{name}: {steps:?}: {e}"));
                checked += 1;
            }
        }
        assert!(
            endless > 0 && checked > 0,
            "{endless} of {checked} without ends"
        );
    }

    /// **Every speed relation the graph states, as rows over its bodies**
    /// — read off the members, meshes, axes, couplings and holds directly,
    /// not through the train's kinematic system: each mesh
    /// `z_a (ω_a − ω_f) ± z_b (ω_b − ω_f) = 0` in the frame `f` both its
    /// axes stand still in (a ring's count negative), each coupling
    /// `ω_a = ω_b`, ground and each hold `ω = 0`; every row scaled to unit
    /// length. `None` where two axes of a mesh stand in no one frame this
    /// reading finds, or the graph lists no body.
    fn speed_rows(t: &Train) -> Option<(Vec<Vec<f64>>, usize)> {
        let s = &t.shape;
        let n = s.max_body() + 1;
        let axis = |b: usize| s.bodies.iter().find(|x| x.body == b).map(|x| x.axis);
        let carrier = |a: usize| s.axes[a].carried_by;
        let mut rows: Vec<Vec<f64>> = Vec::new();
        for m in &s.meshes {
            let (ma, mb) = (&s.members[m.a], &s.members[m.b]);
            let (xa, xb) = (axis(ma.body)?, axis(mb.body)?);
            let (ca, cb) = (carrier(xa), carrier(xb));
            // The frame: the carrier both axes share, or the carrier of the
            // one that orbits about the other's line.
            let frame = if ca == cb || cb == GROUND || axis(ca) == Some(xb) {
                ca
            } else if ca == GROUND || axis(cb) == Some(xa) {
                cb
            } else {
                return None;
            };
            let za = f64::from(ma.gear.teeth);
            let zb = f64::from(mb.gear.teeth);
            let sign = if ma.ring.is_some() || mb.ring.is_some() {
                -1.0
            } else {
                1.0
            };
            let mut row = vec![0.0; n];
            row[ma.body] += za;
            row[mb.body] += sign * zb;
            row[frame] -= za + sign * zb;
            rows.push(row);
        }
        for &[a, b] in &s.couplings {
            let mut row = vec![0.0; n];
            row[a] = 1.0;
            row[b] = -1.0;
            rows.push(row);
        }
        for h in std::iter::once(GROUND).chain(t.held.iter().copied().filter(|&h| h < n)) {
            let mut row = vec![0.0; n];
            row[h] = 1.0;
            rows.push(row);
        }
        for row in &mut rows {
            let norm = row.iter().map(|x| x * x).sum::<f64>().sqrt();
            row.iter_mut().for_each(|x| *x /= norm);
        }
        Some((rows, n))
    }

    /// **Which bodies can turn, by a singular value decomposition** of
    /// [`speed_rows`] in floating point (one-sided Jacobi): a body turns
    /// where the null space of the rows reaches it. The rank tolerance is
    /// the backward error of the decomposition, `τ = max(m, n)·ε·σ_max`, a
    /// singular value at or below it counted nought; a body's reach into the
    /// null space is the norm of its row of the null basis, and the basis
    /// computed is within `max(m, n)·ε·σ_max / δ` of the exact one, `δ` the
    /// least singular value above `τ` (Wedin) — so a reach above that turns.
    /// A different arithmetic and a different method from the train's exact
    /// elimination.
    fn turns_by_svd(t: &Train) -> Option<Vec<bool>> {
        let (rows, n) = speed_rows(t)?;
        let m = rows.len();
        // Columns of A, rotated in place, and V accumulating the rotations.
        let mut a: Vec<Vec<f64>> = (0..n)
            .map(|j| rows.iter().map(|r| r[j]).collect())
            .collect();
        let mut v: Vec<Vec<f64>> = (0..n)
            .map(|j| (0..n).map(|i| f64::from(u8::from(i == j))).collect())
            .collect();
        let dot = |x: &[f64], y: &[f64]| x.iter().zip(y).map(|(p, q)| p * q).sum::<f64>();
        // Jacobi converges quadratically; far fewer sweeps than this
        // settle any matrix of this size, and one that does not is said.
        const SWEEPS: usize = 64;
        // A column already below the rounding of the whole matrix — its
        // Frobenius norm squared is `m`, every row being a unit — has
        // nothing left to orthogonalise.
        let rows_f = f64::from(u32::try_from(m).unwrap());
        let floor = f64::EPSILON * f64::EPSILON * rows_f;
        let rounding = rows_f * f64::EPSILON;
        let mut settled = false;
        for _ in 0..SWEEPS {
            let mut rotated = false;
            for p in 0..n {
                for q in p + 1..n {
                    let (alpha, beta, gamma) =
                        (dot(&a[p], &a[p]), dot(&a[q], &a[q]), dot(&a[p], &a[q]));
                    // Orthogonal to the rounding of an `m`-term dot product.
                    if alpha <= floor
                        || beta <= floor
                        || gamma.abs() <= rounding * (alpha * beta).sqrt()
                    {
                        continue;
                    }
                    rotated = true;
                    let zeta = (beta - alpha) / (2.0 * gamma);
                    let tan = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                    let cos = 1.0 / (1.0 + tan * tan).sqrt();
                    let sin = cos * tan;
                    for w in [&mut a, &mut v] {
                        let (lo, hi) = w.split_at_mut(q);
                        for (x, y) in lo[p].iter_mut().zip(hi[0].iter_mut()) {
                            (*x, *y) = (cos * *x - sin * *y, sin * *x + cos * *y);
                        }
                    }
                }
            }
            if !rotated {
                settled = true;
                break;
            }
        }
        assert!(settled, "Jacobi did not settle in {SWEEPS} sweeps");
        let sigma: Vec<f64> = a.iter().map(|c| dot(c, c).sqrt()).collect();
        let most = sigma.iter().copied().fold(0.0, f64::max);
        let size = f64::from(u32::try_from(m.max(n)).unwrap());
        let tau = size * f64::EPSILON * most;
        let null: Vec<usize> = (0..n).filter(|&j| sigma[j] <= tau).collect();
        let gap = sigma
            .iter()
            .copied()
            .filter(|&x| x > tau)
            .fold(f64::INFINITY, f64::min);
        let reach = tau / gap;
        Some(
            (0..n)
                .map(|b| null.iter().map(|&j| v[j][b] * v[j][b]).sum::<f64>().sqrt() > reach)
                .collect(),
        )
    }

    /// **What a body is, by what the graph says is on it**: each gear on it
    /// that `before` had too — but a gear the edit moved, whose body it
    /// states anew — each planet gear on an axis it carries, and each gear
    /// on the body it is coupled to; resolved to the body that held the
    /// same in `before`. Empty for a body the edit added. Read off the two
    /// graphs, not the edit's renumbering.
    fn was(before: &Train, after: &Train, body: usize, edit: &Edit) -> Vec<usize> {
        let old = before.shape.members.len();
        let moved = match edit {
            Edit::Move { member, .. } => Some(*member),
            _ => None,
        };
        let kept = |i: &usize| *i < old && Some(*i) != moved;
        let axis_in =
            |t: &Train, b: usize| t.shape.bodies.iter().find(|x| x.body == b).map(|x| x.axis);
        let on = |t: &Train, b: usize| -> Vec<usize> {
            (0..t.shape.members.len())
                .filter(|&i| t.shape.members[i].body == b)
                .collect()
        };
        let mut out = Vec::new();
        for i in on(after, body).into_iter().filter(kept) {
            out.push(before.shape.members[i].body);
        }
        for (x, a) in after.shape.axes.iter().enumerate() {
            if a.carried_by != body {
                continue;
            }
            for i in (0..after.shape.members.len()).filter(kept) {
                if axis_in(after, after.shape.members[i].body) == Some(x) {
                    let then = axis_in(before, before.shape.members[i].body);
                    out.extend(then.map(|y| before.shape.axes[y].carried_by));
                }
            }
        }
        for &[x, y] in &after.shape.couplings {
            let other = if x == body {
                y
            } else if y == body {
                x
            } else {
                continue;
            };
            for i in on(after, other).into_iter().filter(kept) {
                let then = before.shape.members[i].body;
                out.extend(
                    before.shape.couplings.iter().filter_map(|&[p, q]| {
                        (p == then).then_some(q).or((q == then).then_some(p))
                    }),
                );
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }

    /// Whether `after` — `before` with `edit` made by its own rule — has a
    /// body standing still that turned before or is new and that it does
    /// not hold: by [`turns_by_svd`] and [`was`], sharing neither the
    /// train's kinematic system nor the edit's renumbering with the rule.
    fn stops(before: &Train, after: &Train, edit: &Edit) -> Option<bool> {
        let (turned, turns) = (turns_by_svd(before)?, turns_by_svd(after)?);
        Some(after.shape.bodies.iter().map(|b| b.body).any(|n| {
            let then = was(before, after, n, edit);
            let could = then.is_empty() || then.iter().any(|&o| turned.get(o) == Some(&true));
            !turns[n] && could && !after.held.contains(&n)
        }))
    }

    /// **An edit is refused as a lock exactly where it stops a body that
    /// could turn** (audit T13.9, the orchestrator's rule): on every start
    /// of the walk, a set released beside a pair after it (which still
    /// turns whatever meshes are added), three pairs in a chain, uncased, a
    /// held pair beside a free set, and every train the walks from seeds
    /// 10481, 50045 and 50568 visit —
    /// every offer at every piece not refused for another reason is made by
    /// its own rule, unchecked, and it is refused as `Locks` if and only if
    /// the train it makes has a body standing still that turned before or
    /// is new and is not held — what turns read by a floating-point null
    /// space of the speed relations the graph states ([`turns_by_svd`]),
    /// each body known by what is on it ([`was`]): neither the train's
    /// exact kinematic system nor the edit's renumbering. The mobility of
    /// the meshes without the holds, which the rule first read, misses a
    /// lock on a held set and refuses gears on a released one that still
    /// turns; a renumbering that follows no body where a join or a move
    /// took it misreads what stopped; each fails here.
    #[test]
    fn a_lock_is_refused_exactly_where_a_body_that_turned_stops() {
        let locks = Some(Note::new(super::super::EditRefused::Locks.key()));
        let mut released = cased(vec![Preset::Planetary.build(), Preset::Spur.build()]);
        let ring = released.port(0, 3);
        released.edit(Edit::Release(ring)).unwrap();
        let pairs = Train::chained(vec![Preset::Spur.build(); 3], |_| Vec::new());
        // A pair held at its input, its output standing still, beside a set
        // with nothing held, apart — built in code, since a hold that stops
        // a body is no edit's. Joining the pair's still output (2) to the
        // set's sun (3) stops the sun alone, the set turning on in its other
        // freedom: a join numbered by the lower body, the one that stood
        // still, which a renumbering that follows no body misreads.
        let mut set = Preset::Planetary.build();
        set.renumber_bodies(|b| b + 2);
        let apart = Train {
            load_cases: Vec::new(),
            reversed_bending: false,
            shape: super::super::graph::graph_of(&[Preset::Spur.build(), set], 1).shape,
            held: vec![1],
        };
        let walked = [10481, 50045, 50568].into_iter().flat_map(|seed| {
            super::super::sweep::walked(seed..seed + 1, 8)
                .into_iter()
                .map(|(name, steps, t)| (format!("{name}: {steps:?}"), t))
        });
        let fixtures = [
            ("a released set, a pair after it".to_owned(), released),
            ("three pairs".to_owned(), pairs),
            ("a held pair beside a free set".to_owned(), apart),
        ];
        let (mut refused, mut made, mut undecided) = (0, 0, 0);
        for (name, t) in trains().into_iter().chain(fixtures).chain(walked) {
            for at in targets(&t) {
                for o in t.offers(at) {
                    if o.refused.is_some() && o.refused != locks {
                        continue;
                    }
                    let context = format!("{name}: at {at:?}, {:?}", o.edit);
                    // A removal takes rows away and forces nothing still.
                    if matches!(o.edit, Edit::Remove(_)) {
                        assert_ne!(o.refused, locks, "{context}");
                        continue;
                    }
                    let mut u = t.clone();
                    let map = u
                        .make(o.edit.clone())
                        .unwrap_or_else(|e| panic!("{context}: {e:?}"));
                    // One entry per number the train had: a body the edit
                    // adds is the image of none of them.
                    assert_eq!(map.len(), t.max_body() + 1, "{context}: {map:?}");
                    let Some(stops) = stops(&t, &u, &o.edit) else {
                        undecided += 1;
                        continue;
                    };
                    assert_eq!(o.refused == locks, stops, "{context}");
                    if stops {
                        refused += 1;
                    } else {
                        made += 1;
                    }
                }
            }
        }
        assert!(
            refused > 100 && made > 1000 && undecided * 100 < made,
            "{refused} refused, {made} made, {undecided} undecided"
        );
    }
}
