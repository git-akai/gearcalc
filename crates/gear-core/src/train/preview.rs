//! **What an edit would do, before it is made** — the dry run the interface
//! shows beside an entry or a verb (docs/reference.md#the-graph, *what a
//! piece offers*). The edit is
//! made on a copy by the rule that would make it, both trains are solved,
//! and the difference is said as notes: the words stay in the catalogue and
//! the comparison in the core.

use super::{solve_train, EditRefused, PathReport, Train, TrainError, TrainResult};
use crate::material::MaterialLibrary;
use crate::note::{key, Explain, Note};

/// **What an edit would do** ([`preview`]).
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Preview {
    /// Why the edit would not be made, where it would not.
    pub refused: Option<Note>,
    /// **What it would change**: each kind of piece whose count moves —
    /// gears, meshes, axes, bodies, axis distances, couplings — the holds
    /// and the case entries, before and after; or that it changes nothing
    /// at all.
    pub changes: Vec<Note>,
    /// **What the headline path would come to** — the path the headline
    /// case walks first — its ratio and efficiency before and after, or
    /// that it would go, or appear.
    pub paths: Vec<Note>,
    /// Why the train would not solve after it, where it would not.
    pub unsolved: Option<Note>,
}

/// The kinds of piece and statement a preview counts, in the order it
/// says them.
const KINDS: [&str; 8] = [
    key::PREVIEW_GEARS,
    key::PREVIEW_MESHES,
    key::PREVIEW_AXES,
    key::PREVIEW_BODIES,
    key::PREVIEW_DISTANCES,
    key::PREVIEW_COUPLINGS,
    key::PREVIEW_HOLDS,
    key::PREVIEW_CASE_ENTRIES,
];

fn counts(t: &Train) -> [usize; 8] {
    let s = &t.shape;
    [
        s.members.len(),
        s.meshes.len(),
        s.axes.len(),
        s.bodies.len(),
        s.distances.len(),
        s.couplings.len(),
        t.held.len(),
        t.load_cases.iter().map(|c| c.loads.len()).sum(),
    ]
}

/// **Whether an edit changed nothing** — the train it left is the train it
/// was given, every input and every number in it.
pub(super) fn unchanged(before: &Train, after: &Train) -> bool {
    format!("{before:?}") == format!("{after:?}")
}

fn whole(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// **What an edit would do**: `after` — the train it would leave, or why
/// it would not be made — against `before`, both solved against `lib`.
///
/// The headline path is compared where the headline case keeps its
/// entries, which is when it is the same path whatever the edit numbered
/// again; where an entry went with a body, the old path is said to go and
/// any the case still walks to appear.
#[must_use]
pub fn preview(
    before: &Train,
    after: Result<&Train, EditRefused>,
    lib: &MaterialLibrary,
) -> Preview {
    let after = match after {
        Ok(t) => t,
        Err(refusal) => {
            return Preview {
                refused: Some(Note::new(refusal.key())),
                changes: Vec::new(),
                paths: Vec::new(),
                unsolved: None,
            }
        }
    };
    let mut changes: Vec<Note> = KINDS
        .iter()
        .zip(counts(before).into_iter().zip(counts(after)))
        .filter(|(_, (was, now))| was != now)
        .map(|(k, (was, now))| {
            Note::new(k)
                .count("before", whole(was))
                .count("after", whole(now))
        })
        .collect();
    if changes.is_empty() && unchanged(before, after) {
        changes.push(Note::new(key::PREVIEW_NOTHING));
    }

    let (was, now) = (solve_train(before, lib), solve_train(after, lib));
    let headline = |t: &Train| {
        t.load_cases
            .iter()
            .position(|c| c.enabled)
            .map(|i| (i, t.load_cases[i].loads.len()))
    };
    let first = |r: &Result<TrainResult, TrainError>| {
        r.as_ref().ok().and_then(|r| r.paths.first().cloned())
    };
    let ends =
        |note: Note, p: &PathReport| note.count("from", whole(p.from)).count("to", whole(p.to));
    // A ratio is two sides, read as the path's row reads it (`reads`): the
    // one a count, so it prints as one, and the other its figure.
    let figures = |note: Note, p: &PathReport, ratio: &str, efficiency: &str| {
        let (from, to) = (format!("{ratio}_from"), format!("{ratio}_to"));
        let (one, turns) = if p.reads.step_up {
            (from, to)
        } else {
            (to, from)
        };
        note.count(&one, 1).number(&turns, p.reads.turns, 4).number(
            efficiency,
            100.0 * p.efficiency.forward,
            2,
        )
    };
    let gone = |p: &PathReport| ends(Note::new(key::PREVIEW_PATH_GONE), p);
    let appears = |p: &PathReport| {
        figures(
            ends(Note::new(key::PREVIEW_PATH_NEW), p),
            p,
            "ratio",
            "efficiency",
        )
    };
    let paths = match (first(&was), first(&now)) {
        (Some(x), Some(y)) if headline(before) == headline(after) => {
            let note = ends(Note::new(key::PREVIEW_PATH), &y);
            let note = figures(note, &x, "ratio_before", "efficiency_before");
            vec![figures(note, &y, "ratio_after", "efficiency_after")]
        }
        (Some(x), Some(y)) => vec![gone(&x), appears(&y)],
        (Some(x), None) => vec![gone(&x)],
        (None, Some(y)) => vec![appears(&y)],
        (None, None) => Vec::new(),
    };
    Preview {
        refused: None,
        changes,
        paths,
        unsolved: now.err().map(|e| e.note()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    //! **A preview says what the edit it previews does**: its refusal and
    //! nothing else; the pieces it makes and takes, counted; nothing, where
    //! it changes nothing; and the headline path kept, lost or found.

    use super::super::arrangements::Preset;
    use super::super::{test_library, Edit, LoadCase, Piece, Place};
    use super::*;

    fn keys(notes: &[Note]) -> Vec<&str> {
        notes.iter().map(|n| n.key.as_str()).collect()
    }

    fn pair() -> Train {
        Train::chained(vec![Preset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(0, 2), 1.0, 1000.0)]
        })
    }

    #[test]
    fn a_refused_edit_previews_its_refusal_and_nothing_else() {
        let t = pair();
        let p = preview(&t, Err(EditRefused::Geared), &test_library());
        assert_eq!(p.refused, Some(Note::new(EditRefused::Geared.key())));
        assert!(p.changes.is_empty() && p.paths.is_empty() && p.unsolved.is_none());
    }

    /// **A preview counts what an edit makes**: a gear on a new axis at a
    /// pair's second gear — an idler off the path the case walks — is a
    /// gear, a mesh, an axis, a body and a distance more, and the path as
    /// it was.
    #[test]
    fn a_preview_counts_what_an_edit_makes() {
        let t = pair();
        let mut u = t.clone();
        u.edit(Edit::AddGear {
            mate: 1,
            on: Place::NewAxis,
            ring: false,
        })
        .unwrap();
        let p = preview(&t, Ok(&u), &test_library());
        assert_eq!(
            keys(&p.changes),
            [
                key::PREVIEW_GEARS,
                key::PREVIEW_MESHES,
                key::PREVIEW_AXES,
                key::PREVIEW_BODIES,
                key::PREVIEW_DISTANCES
            ]
        );
        let gears = &p.changes[0].values;
        assert_eq!(
            (gears["before"].as_str(), gears["after"].as_str()),
            ("2", "3")
        );
        assert_eq!(keys(&p.paths), [key::PREVIEW_PATH]);
        let path = &p.paths[0].values;
        for side in ["from", "to"] {
            assert_eq!(
                path[&format!("ratio_before_{side}")],
                path[&format!("ratio_after_{side}")],
                "{path:?}"
            );
        }
        assert_eq!(
            path["efficiency_before"], path["efficiency_after"],
            "{path:?}"
        );
        assert!(p.refused.is_none() && p.unsolved.is_none());
    }

    #[test]
    fn an_edit_that_changes_nothing_says_so() {
        let t = pair();
        let mut u = t.clone();
        u.edit(Edit::Release(1)).unwrap();
        let p = preview(&t, Ok(&u), &test_library());
        assert_eq!(keys(&p.changes), [key::PREVIEW_NOTHING]);
        assert_eq!(keys(&p.paths), [key::PREVIEW_PATH]);
    }

    /// **A path an edit takes is said to go, and one it makes to appear.**
    /// A chain's last gear removed takes its mate and the output body the
    /// case reacted at, and the case walks no path; a set with nothing held
    /// is a family and walks none, and its ring held gives it one.
    #[test]
    fn a_path_goes_and_a_path_appears() {
        let lib = test_library();
        let t = Train::chained(vec![Preset::Spur.build(), Preset::Spur.build()], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(1, 2), 1.0, 1000.0)]
        });
        let mut u = t.clone();
        u.edit(Edit::Remove(Piece::Member(3))).unwrap();
        let p = preview(&t, Ok(&u), &lib);
        assert_eq!(keys(&p.paths), [key::PREVIEW_PATH_GONE]);
        assert!(keys(&p.changes).contains(&key::PREVIEW_CASE_ENTRIES));

        let set = Preset::Planetary.build();
        let (sun, carrier, ring) = (1, 2, 3);
        let mut t = Train::chained(vec![set], |_| Vec::new());
        t.release(ring);
        t.load_cases = vec![LoadCase::ultimate(sun, carrier, 1.0, 1000.0)];
        let mut u = t.clone();
        u.edit(Edit::Hold(ring)).unwrap();
        let p = preview(&t, Ok(&u), &lib);
        assert_eq!(keys(&p.paths), [key::PREVIEW_PATH_NEW]);
        assert_eq!(keys(&p.changes), [key::PREVIEW_HOLDS]);
    }
}
