//! **What can be done to a piece** — every edit of the graph's set
//! ([`Edit`]) that applies at one piece a designer has selected, generated
//! from the graph and tried on a copy, so an interface lists complete
//! outcomes rather than controls to fill in, and shows a refusal as a verb
//! it cannot press, with its reason (`train-graph-plan.md`, *select, act,
//! and see before you click*). What each would come to is
//! [`super::preview`]'s to say.
//!
//! **An offer is its edit**: refused exactly where [`Train::edit`] refuses
//! it, and never one that would change nothing — a gear alone on its body
//! moved to a body of its own is on one already. Which edits a piece
//! offers is read off the graph by the rule each edit states: a gear meshes
//! across an axis distance, so it is offered on the axes a distance joins
//! to its mate's; a gear moves among the bodies of its axis; a step goes on
//! a carried axis and a coupling from a body on one. The laws below hold
//! the reading to the edits themselves, swept by brute force.

use super::arrangements::StagePreset;
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
    /// Nothing selected: the stages a train takes at its output.
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
    pub preset: Option<StagePreset>,
    /// The refusal's catalogue key ([`super::EditRefused::key`]).
    pub refused: Option<Note>,
}

impl Train {
    /// **Every edit the graph offers at `at`**, in the order a menu lists
    /// them — what it adds, then the stages it takes, then what it moves,
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
    fn candidates(&self, at: Target) -> Vec<(Edit, Option<StagePreset>)> {
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
            StagePreset::ALL.map(|p| {
                (
                    Edit::Insert {
                        stage: p.build(),
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

    use super::super::edits::well_formed;
    use super::super::Shape;
    use super::*;

    fn trains() -> Vec<(String, Train)> {
        let mut out = Vec::new();
        for p in StagePreset::ALL {
            for before in [None, Some(StagePreset::Spur)] {
                let stages: Vec<Shape> = before
                    .into_iter()
                    .chain(std::iter::once(p))
                    .map(StagePreset::build)
                    .collect();
                out.push((
                    format!("{p:?} after {before:?}"),
                    Train::chained(stages, |_| Vec::new()),
                ));
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
                        well_formed(&u).unwrap_or_else(|e| panic!("{context}: {e}"));
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
                for p in StagePreset::ALL {
                    let stage = p.build();
                    sweep.push((Edit::Insert { stage, at: Some(b) }, at.clone()));
                }
            }
            for p in StagePreset::ALL {
                let stage = p.build();
                sweep.push((Edit::Insert { stage, at: None }, vec![Target::Train]));
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
}
