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
