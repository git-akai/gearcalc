//! Numerical helpers the **tests** check the closed forms against.
//!
//! Compiled only under `cfg(test)`, and deliberately naive: the job of anything
//! here is to be obviously right rather than fast or clever. These exist so a
//! derivation can be gated against a route that shares no algebra with it —
//! `docs/rationale.md`'s first testing rule — and they live in one place for the
//! same reason everything else in this crate does: [`crate::elliptic`] and
//! [`crate::hertz`] had a Simpson rule apiece, identical but for the name, and
//! two copies of one idea is where two answers come from.

/// Composite Simpson's rule over `[0, 1]`.
///
/// The interval is fixed because every integrand these tests reach it with has
/// already been substituted onto it — an improper or infinite range is made
/// finite by the *caller*, which is where the substitution belongs, since only
/// the caller knows what its integrand does at the ends.
///
/// `N` is even, so the alternating 4/2 weights land correctly and the rule is
/// the fourth-order one it claims to be.
pub fn simpson_unit_interval<F: Fn(f64) -> f64>(f: F) -> f64 {
    const N: usize = 200_000;
    #[allow(clippy::cast_precision_loss)]
    let h = 1.0 / N as f64;
    let mut sum = f(0.0) + f(1.0);
    for i in 1..N {
        #[allow(clippy::cast_precision_loss)]
        let x = i as f64 * h;
        sum += if i % 2 == 0 { 2.0 } else { 4.0 } * f(x);
    }
    sum * h / 3.0
}

/// `∫₀^∞ g(w) w^(−1/2) dw` for smooth `g` — the shape all three Hertz integrals
/// have.
///
/// Split at `w = 1` and substituted onto `[0,1]` twice: `w = u²` near the
/// origin, which cancels the `w^(−1/2)` outright rather than integrating through
/// it, and `w = 1/v²` on the tail, which turns the algebraic decay into a smooth
/// vanishing.
pub fn improper_sqrt<F: Fn(f64) -> f64>(g: F) -> f64 {
    let near = simpson_unit_interval(|u| 2.0 * g(u * u));
    let tail = simpson_unit_interval(|v| {
        if v == 0.0 {
            0.0
        } else {
            2.0 * g(1.0 / (v * v)) / (v * v)
        }
    });
    near + tail
}

/// **Work counted where it is done**, so a test can say what a solve cost in
/// units no machine changes: teeth cut, rings cut, and a search's objective
/// evaluations.
///
/// Wall-clock in a suite that runs tests in parallel measures the machine as
/// much as the code; a count measures the code alone, and the regressions it
/// is for — a candidate building teeth nothing then read — are regressions in
/// the count. Per thread, because a test runs on one and the counters are read
/// around a call that does not spawn.
pub mod work {
    use std::cell::Cell;

    thread_local! {
        static TEETH: Cell<u64> = const { Cell::new(0) };
        static RINGS: Cell<u64> = const { Cell::new(0) };
        static EVALUATIONS: Cell<u64> = const { Cell::new(0) };
        static TRIALS: Cell<u64> = const { Cell::new(0) };
        static SEARCHES: std::cell::RefCell<Vec<Walked>> = const { std::cell::RefCell::new(Vec::new()) };
    }

    /// One `auto::Search::maximise` call's control flow: what its structure
    /// says it evaluates, recorded where it runs so a test can hold the
    /// evaluation count to it.
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct Walked {
        /// Steps across each axis of the sweep (`Search::scan`).
        pub scan: u64,
        /// Free coordinates.
        pub dof: u32,
        /// Walks started (each evaluates its start once).
        pub walks: u64,
        /// Rounds of the walks together (each tries every direction once).
        pub rounds: u64,
    }

    impl Walked {
        /// What the search's definition evaluates: every point of the sweep's
        /// grid, `(scan + 1)^dof`; each walk's start; and in each round every
        /// direction of the `3^dof − 1` about the point.
        #[must_use]
        pub fn evaluations(self) -> u64 {
            (self.scan + 1).pow(self.dof) + self.walks + self.rounds * (3u64.pow(self.dof) - 1)
        }
    }

    pub fn walked(w: Walked) {
        SEARCHES.with(|s| s.borrow_mut().push(w));
    }

    /// The searches `f` ran on this thread, and what it returns.
    pub fn searches<T>(f: impl FnOnce() -> T) -> (Vec<Walked>, T) {
        let from = SEARCHES.with(|s| s.borrow().len());
        let out = f();
        (SEARCHES.with(|s| s.borrow()[from..].to_vec()), out)
    }

    /// What was counted on this thread.
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct Work {
        /// External teeth cut (`Tooth` construction).
        pub teeth: u64,
        /// Rings cut (`Ring` construction).
        pub rings: u64,
        /// Objective evaluations by `auto::Search::maximise`.
        pub evaluations: u64,
        /// Candidate trains a shape's search scored (`trial_efficiency`).
        pub trials: u64,
    }

    fn bump(c: &'static std::thread::LocalKey<Cell<u64>>) {
        c.with(|n| n.set(n.get() + 1));
    }

    pub fn tooth() {
        bump(&TEETH);
    }

    pub fn ring() {
        bump(&RINGS);
    }

    pub fn evaluation() {
        bump(&EVALUATIONS);
    }

    pub fn trial() {
        bump(&TRIALS);
    }

    /// The work `f` does on this thread, and what it returns.
    pub fn of<T>(f: impl FnOnce() -> T) -> (Work, T) {
        let read = || Work {
            teeth: TEETH.with(Cell::get),
            rings: RINGS.with(Cell::get),
            evaluations: EVALUATIONS.with(Cell::get),
            trials: TRIALS.with(Cell::get),
        };
        let before = read();
        let out = f();
        let after = read();
        (
            Work {
                teeth: after.teeth - before.teeth,
                rings: after.rings - before.rings,
                evaluations: after.evaluations - before.evaluations,
                trials: after.trials - before.trials,
            },
            out,
        )
    }
}
