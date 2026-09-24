//! **Who decides a shift, and what it must satisfy** — read once so every
//! shape reads it the same way.
//!
//! A pair of gears was a stage type here, then a vocabulary the shape was
//! filled in from, and is now `arrangements::pair` — a list of what sits
//! where, like every other preset, with a worm's conventional proportions
//! one bit on a distance (`Distance::worm`). What stays is what every shape
//! reads through this module: [`ShiftAsked`] — the search floor and the
//! true minimum told apart — and the undercut bound a search or an absorber
//! is held to.
//!
//! A worm stage used to be a separate type with a separate result — no
//! profile shift, no addendum, members that were not gears — and its centre
//! distance could only be reached by resizing the worm. `docs/corrections.md`
//! records what that cost, and the audit's record (`docs/history/audit.md`,
//! F83) what deleting it moved: nothing.

use crate::auto::automatic_profile_shift;

/// **What a gear's two shift controls come to**, read once so every shape reads
/// them the same way.
///
/// The two are different kinds of thing and this is where that is written down:
/// `profile_shift.auto` says *who decides*, `no_undercut` says *what the answer
/// must satisfy however it is decided*. Every shape needs all three of the
/// values below — the bound for its search, what it may not overrule, and what
/// to report when there is nothing to search — and each had been working them
/// out for itself.
///
/// # The bound is not one number, and that is not an inconsistency
///
/// Undercut asks one question, but *choosing* a shift and *checking* a given
/// one want different answers to it, and [`automatic_profile_shift`] says why:
/// the true minimum is negative on any comfortable tooth count, so applying it
/// literally would thin a tooth that needed no help, for nothing.
///
/// So a search is floored at `max(x_min, 0)` — shift when the geometry demands
/// it, otherwise leave the tooth alone — while a number a designer typed is
/// held only to `x_min` itself. A deliberate −0.3 on a 43-tooth gear is a
/// decision about centre distance or balance, not a mistake about undercut, and
/// survives; a −2.0 there genuinely undercuts and is raised.
///
/// Getting this wrong is measurable rather than a matter of taste: flooring the
/// *search* at the true minimum let the hula stage's split walk out to
/// −1.79 and come back with **less** drive efficiency than it started with.
#[derive(Clone, Copy, Debug)]
pub(crate) struct ShiftAsked {
    /// What a search choosing this shift may not go below — `max(x_min, 0)`,
    /// see above — or `None` where there is no such bound to apply.
    ///
    /// `None` covers **two** cases and they are the same statement: the gear
    /// may undercut, or the gear is not being chosen at all. A shift a designer
    /// gave was already held to its own bound when it was read, and a search
    /// that re-judges it against the *chooser's* bound rejects designs that are
    /// perfectly legal — a 43-tooth wheel pinned at −0.5 is nowhere near
    /// undercut and sits a whole module below `max(x_min, 0)`, so every
    /// candidate built on it was thrown away and the optimiser fell back to
    /// doing nothing, silently. A pinned gear is a constraint on the search,
    /// not a candidate of it.
    pub search_floor: Option<f64>,
    /// The shift a designer gave, which no search may overrule. `None` where it
    /// was left automatic — and a given shift arrives already raised to its
    /// floor, so a pinned value and a reported one cannot disagree.
    pub given: Option<f64>,
    /// What the gear settles at when nothing else decides: the given value, or
    /// [`automatic_profile_shift`] where the bound is on, or zero where it is
    /// off and nothing is asked of the shift at all.
    pub settled: f64,
    /// Whether a given shift had to be raised to reach its floor, and so
    /// whether the number in hand is the number that was typed.
    pub raised: bool,
}

/// **How a member's shift came to be**, which is what decides the undercut
/// bound it answers to.
///
/// Two ways, and they want two different answers to the same question —
/// which is why this is a type rather than a boolean. A shift a designer
/// typed is the third way, and it is pinned before any bound is asked
/// ([`crate::auto::Cut::Pinned`]). See [`undercut_bound`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Decided {
    /// A search is choosing it, and may pick any admissible value.
    Chosen,
    /// A relation left it — the planet's shift closing an epicyclic set's two
    /// centre distances, say. Nothing is free to move it.
    Absorbed,
}

/// **The undercut bound a member answers to**, or `None` where it answers to
/// none.
///
/// One question, three answers, and each of them is the honest one for how the
/// number arrived:
///
/// | how | bound | why |
/// |---|---|---|
/// | [`Decided::Chosen`] | `max(x_min, 0)` | a chooser should not thin a tooth that needed no help |
/// | [`Decided::Given`] | none | it was held to `x_min` when it was read; re-judging it here rejects legal designs |
/// | [`Decided::Absorbed`] | `x_min` | nothing can move it, so the only honest question is whether it *does* undercut |
///
/// The middle row is a bug this had twice: a search that re-judges a number it
/// was handed throws away every candidate built on a perfectly legal one. The
/// last row is the same mistake wearing different clothes — an absorbed shift
/// clamped to a chooser's floor would break the relation that produced it.
pub(crate) fn undercut_bound(
    no_undercut: bool,
    p: &crate::params::GearParams,
    depth: f64,
    how: Decided,
) -> Option<f64> {
    if !no_undercut {
        return None;
    }
    match how {
        Decided::Chosen => Some(automatic_profile_shift(p, depth)),
        Decided::Absorbed => Some(crate::auto::minimum_profile_shift(p, depth).with_cutter_radius),
    }
}

impl ShiftAsked {
    /// **The note a raised shift owes its reader.**
    ///
    /// `no undercut` bounds a shift a designer typed as well as one the search
    /// chose, which is what lets it mean one thing everywhere — but a number
    /// that was not taken as given has to say so, or the field and the gear
    /// disagree in silence. It is the gear's own note, drawn under its own
    /// field, so it names no gear: it used to carry a tooth count for a list
    /// at the foot of the stage card it no longer sits in.
    pub(crate) fn note(&self) -> Option<crate::note::Note> {
        self.raised.then(|| {
            crate::note::Note::new(crate::note::key::GEAR_SHIFT_RAISED_FOR_UNDERCUT).number(
                "shift",
                self.settled,
                4,
            )
        })
    }
}
