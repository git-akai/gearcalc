//! Scalar root finding, and two bracketed maximisers.
//!
//! Two solvers cover every transcendental step in this crate (docs/rationale.md#where-closed-form-is-impossible).
//! Both are **bracketed**, so neither can diverge: a Newton step that leaves the
//! bracket is replaced by a bisection step. That property is not a nicety here —
//! the involute function's series seed diverges above roughly 60°, which is
//! inside the pressure-angle range this tool allows.
//!
//! Neither returns a "best effort" answer on failure. A solve that did not
//! bracket a root, that stepped onto a point where the function has no value
//! (a residual that is not finite, or `None` from a `_partial` form's), or
//! that ran out of iterations without converging returns `None`, and the
//! caller reports the geometry as impossible rather than propagating a NaN
//! into a stress figure. A residual with no value somewhere says so with
//! `None` through the `_partial` forms, never with a NaN standing for it. Both halves of that were promised
//! here long before the second was true: falling out of the loop used to return
//! the last iterate.

/// Convergence settings.
///
/// `x_tol` is a **floor** on the abscissa tolerance, not the whole of it: both
/// solvers add a relative term, `2ε|x|`, so the same settings mean the same
/// thing on a root of 1e-3 and one of 1e+3. That was not true of
/// [`newton_bracketed`], which tested an absolute step alone — and since a root
/// of order 100 mm has an ulp of 1.4e-14, the 1e-15 floor could never be met
/// and the loop ran to its bound every time (`docs/corrections.md`).
///
/// `max_iter` is a safety stop, not an expected limit — bisection alone halves
/// the bracket each step, so 200 iterations is far beyond what double precision
/// can use. Reaching it means the iteration is not converging, and both solvers
/// then return `None` rather than the last iterate.
#[derive(Clone, Copy, Debug)]
pub struct Tol {
    pub x_tol: f64,
    pub max_iter: u32,
}

impl Default for Tol {
    fn default() -> Self {
        Self {
            x_tol: 1e-15,
            max_iter: 200,
        }
    }
}

/// Brent's method: inverse quadratic interpolation with a guaranteed bisection
/// fallback. Requires `f(lo)` and `f(hi)` to straddle zero.
///
/// Returns `None` if the interval is not bracketed or `f` is not finite on it.
pub fn brent<F>(f: F, lo: f64, hi: f64, tol: Tol) -> Option<f64>
where
    F: Fn(f64) -> f64,
{
    brent_bracket(f, lo, hi, tol).map(|[b, _]| b)
}

/// [`brent`]'s final bracket: `[b, c]`, the root estimate and the far end of
/// the bracket it closed, with `f(b)` and `f(c)` of opposite signs or `f(b)`
/// zero (then `c = b`). For a caller that needs the root **on one side** — a
/// clearance that must not be negative — rather than wherever the last step
/// landed.
pub fn brent_bracket<F>(f: F, lo: f64, hi: f64, tol: Tol) -> Option<[f64; 2]>
where
    F: Fn(f64) -> f64,
{
    brent_bracket_partial(|x| Some(f(x)), lo, hi, tol)
}

/// [`brent`] for a residual that has no value somewhere: `f` answers `None`
/// there, and the solve answers `None` if it steps onto such a point. A
/// non-finite value is read the same way.
pub fn brent_partial<F>(f: F, lo: f64, hi: f64, tol: Tol) -> Option<f64>
where
    F: Fn(f64) -> Option<f64>,
{
    brent_bracket_partial(f, lo, hi, tol).map(|[b, _]| b)
}

/// [`brent_bracket`] for a residual that has no value somewhere, as
/// [`brent_partial`] reads it.
pub fn brent_bracket_partial<F>(f: F, lo: f64, hi: f64, tol: Tol) -> Option<[f64; 2]>
where
    F: Fn(f64) -> Option<f64>,
{
    let f = |x: f64| f(x).filter(|v| v.is_finite());
    let (mut a, mut b) = (lo, hi);
    let (mut fa, mut fb) = (f(a)?, f(b)?);
    if fa == 0.0 {
        return Some([a, a]);
    }
    if fb == 0.0 {
        return Some([b, b]);
    }
    if (fa < 0.0) == (fb < 0.0) {
        return None; // not bracketed
    }

    let (mut c, mut fc) = (a, fa);
    let mut d = b - a;
    let mut e = d;

    for _ in 0..tol.max_iter {
        if (fb < 0.0) == (fc < 0.0) {
            c = a;
            fc = fa;
            d = b - a;
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }

        let tol1 = 2.0 * f64::EPSILON * b.abs() + 0.5 * tol.x_tol;
        let xm = 0.5 * (c - b);
        if fb == 0.0 {
            return Some([b, b]);
        }
        if xm.abs() <= tol1 {
            return Some([b, c]);
        }

        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            let s = fb / fa;
            let (mut p, mut q);
            if (a - c).abs() < f64::EPSILON * a.abs().max(1.0) {
                // secant
                p = 2.0 * xm * s;
                q = 1.0 - s;
            } else {
                // inverse quadratic
                let qq = fa / fc;
                let r = fb / fc;
                p = s * (2.0 * xm * qq * (qq - r) - (b - a) * (r - 1.0));
                q = (qq - 1.0) * (r - 1.0) * (s - 1.0);
            }
            if p > 0.0 {
                q = -q;
            }
            p = p.abs();
            let bound = (3.0 * xm * q - (tol1 * q).abs()).min((e * q).abs());
            if 2.0 * p < bound {
                e = d;
                d = p / q;
            } else {
                d = xm;
                e = d;
            }
        } else {
            d = xm;
            e = d;
        }

        a = b;
        fa = fb;
        b += if d.abs() > tol1 { d } else { tol1.copysign(xm) };
        fb = f(b)?;
    }
    // The iteration bound is a safety stop, so reaching it is a failure to
    // converge and not an answer. Returning the last iterate here contradicted
    // this module's own promise that neither solver gives a best effort — and a
    // promise nothing enforces is the one that gets relied on.
    None
}

/// Newton's method with a maintained bracket.
///
/// Takes a Newton step when it lands inside the current bracket and makes
/// progress; otherwise bisects. Converges quadratically where Newton behaves and
/// is still guaranteed where it does not.
///
/// `f` must straddle zero on `[lo, hi]`. `guess` is clamped into the bracket.
pub fn newton_bracketed<F, D>(f: F, df: D, lo: f64, hi: f64, guess: f64, tol: Tol) -> Option<f64>
where
    F: Fn(f64) -> f64,
    D: Fn(f64) -> f64,
{
    newton_bracketed_partial(|x| Some(f(x)), |x| Some(df(x)), lo, hi, guess, tol)
}

/// [`newton_bracketed`] for a function or slope that has no value somewhere,
/// as [`brent_partial`] reads one: a step onto a point with no value ends the
/// solve with `None`, and a point with no slope is bisected.
pub fn newton_bracketed_partial<F, D>(
    f: F,
    df: D,
    lo: f64,
    hi: f64,
    guess: f64,
    tol: Tol,
) -> Option<f64>
where
    F: Fn(f64) -> Option<f64>,
    D: Fn(f64) -> Option<f64>,
{
    let f = |x: f64| f(x).filter(|v| v.is_finite());
    let df = |x: f64| df(x).filter(|v| v.is_finite());
    let (flo, fhi) = (f(lo)?, f(hi)?);
    if flo == 0.0 {
        return Some(lo);
    }
    if fhi == 0.0 {
        return Some(hi);
    }
    if (flo < 0.0) == (fhi < 0.0) {
        return None;
    }

    // Orient so that f is negative at `low` and positive at `high`.
    let (mut low, mut high) = if flo < 0.0 { (lo, hi) } else { (hi, lo) };

    let mut x = guess.clamp(lo.min(hi), lo.max(hi));
    let mut step_prev = (hi - lo).abs();
    let mut step = step_prev;
    let mut fx = f(x)?;
    let mut dfx = df(x);

    for _ in 0..tol.max_iter {
        // Bisect when the Newton step would leave the bracket, or is not at
        // least halving the interval.
        let newton = dfx.filter(|&d| {
            let out_of_range = ((x - high) * d - fx) * ((x - low) * d - fx) > 0.0;
            let too_slow = (2.0 * fx).abs() > (step_prev * d).abs();
            d != 0.0 && !out_of_range && !too_slow
        });
        if let Some(dfx) = newton {
            step_prev = step;
            step = fx / dfx;
            x -= step;
        } else {
            step_prev = step;
            step = 0.5 * (high - low);
            x = low + step;
        }

        // Relative, as Brent's is: an absolute floor alone is a different
        // claim at every magnitude, and an unreachable one above about 10.
        if step.abs() < 2.0 * f64::EPSILON * x.abs() + tol.x_tol {
            return Some(x);
        }

        fx = f(x)?;
        dfx = df(x);
        if fx < 0.0 {
            low = x;
        } else {
            high = x;
        }
    }
    None
}

/// **The maximum of a unimodal `f` on `[lo, hi]`**, by golden-section search:
/// `(x, f(x))`.
///
/// Bracketed like the root finders, so it cannot leave the interval: each
/// step keeps the sub-interval that holds the larger of two interior values.
/// `f` must have one interior maximum on the bracket; `None` where a value is
/// not finite, where the search runs out of iterations, or where the maximum
/// sits at an end — a bracket that did not hold it.
pub fn maximise<F>(f: F, lo: f64, hi: f64, tol: Tol) -> Option<(f64, f64)>
where
    F: Fn(f64) -> f64,
{
    // 1/φ, the golden ratio's reciprocal: each step keeps this share.
    let r = (5.0_f64.sqrt() - 1.0) / 2.0;
    let (mut a, mut b) = (lo, hi);
    let (mut c, mut d) = (b - r * (b - a), a + r * (b - a));
    let (mut fc, mut fd) = (f(c), f(d));
    for _ in 0..tol.max_iter {
        if !fc.is_finite() || !fd.is_finite() {
            return None;
        }
        if (b - a).abs() <= tol.x_tol + 4.0 * f64::EPSILON * (a.abs() + b.abs()) {
            let (x, fx) = if fc > fd { (c, fc) } else { (d, fd) };
            let edge = 2.0 * (b - a).abs().max(tol.x_tol);
            let at_end = (x - lo).abs() <= edge || (hi - x).abs() <= edge;
            return (!at_end).then_some((x, fx));
        }
        if fc > fd {
            b = d;
            d = c;
            fd = fc;
            c = b - r * (b - a);
            fc = f(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + r * (b - a);
            fd = f(d);
        }
    }
    None
}

/// **The greatest value of `f` on `[lo, hi]`, ends included**: `(x, f(x))`,
/// or `None` where `f` has no value anywhere it was asked.
///
/// Golden section, as [`maximise`], for an `f` that rises to one peak and
/// falls on the bracket, or does only one of the two — the ends are compared
/// with what the search settles on, so a greatest value at an end is found as
/// one inside is. A point where `f` has no value ranks below every value: it is
/// never the answer, and a search is never steered toward it.
///
/// **The trip count is derived, not a stopping test.** Near a smooth peak `f`
/// is flat to its own rounding, `ε`, within `√ε` of the peak's argument, so
/// narrowing the bracket past `√ε` of its magnitude cannot change the value
/// found. Each step keeps `1/φ` of the bracket, so it takes
/// `⌈ln((hi − lo)/(√ε·max(|lo|, |hi|))) / ln φ⌉` steps, 38 at most on a
/// bracket that starts at zero.
pub fn greatest<F>(f: F, lo: f64, hi: f64) -> Option<(f64, f64)>
where
    F: Fn(f64) -> Option<f64>,
{
    let at = |x: f64| f(x).map(|v| (x, v));
    let better = |a: Option<(f64, f64)>, b: Option<(f64, f64)>| match (a, b) {
        (Some(p), Some(q)) => Some(if q.1 > p.1 { q } else { p }),
        (p, q) => p.or(q),
    };
    let ends = better(at(lo), at(hi));
    let width = hi - lo;
    let scale = f64::EPSILON.sqrt() * lo.abs().max(hi.abs());
    if width <= scale {
        return ends;
    }
    // 1/φ, the share of the bracket each step keeps.
    let r = (5.0_f64.sqrt() - 1.0) / 2.0;
    let steps = ((width / scale).ln() / (1.0 / r).ln()).ceil();
    let (mut a, mut b) = (lo, hi);
    let (mut c, mut d) = (b - r * (b - a), a + r * (b - a));
    let (mut fc, mut fd) = (at(c), at(d));
    let mut step = 0.0;
    while step < steps {
        // `None` ranks below every value: the side holding a value is kept.
        if fc.map(|p| p.1) > fd.map(|q| q.1) {
            b = d;
            d = c;
            fd = fc;
            c = b - r * (b - a);
            fc = at(c);
        } else {
            a = c;
            c = d;
            fc = fd;
            d = a + r * (b - a);
            fd = at(d);
        }
        step += 1.0;
    }
    better(ends, better(fc, fd))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// **The greatest value is found inside and at either end**, to the
    /// value's rounding, in the derived number of steps: a peak inside, a
    /// function rising to its right end, one falling from its left, and one
    /// with no value on part of the bracket.
    #[test]
    fn the_greatest_value_is_found_inside_and_at_either_end() {
        let peak = |x: f64| Some(-(x - 0.3) * (x - 0.3) + 2.0);
        let (x, v) = greatest(peak, 0.0, 1.0).unwrap();
        assert!((v - 2.0).abs() <= 4.0 * f64::EPSILON * 2.0, "{v}");
        assert!((x - 0.3).abs() <= 4.0 * f64::EPSILON.sqrt(), "{x}");
        assert_eq!(
            greatest(|x: f64| Some(x.exp()), 0.0, 2.0).unwrap(),
            (2.0, 2.0_f64.exp())
        );
        assert_eq!(greatest(|x: f64| Some(-x), 1.0, 3.0).unwrap(), (1.0, -1.0));
        // No value past 0.6: the peak at 0.5 is still found.
        let partial = |x: f64| (x < 0.6).then(|| 1.0 - (x - 0.5).abs());
        let (x, v) = greatest(partial, 0.0, 1.0).unwrap();
        assert!(
            (x - 0.5).abs() <= 4.0 * f64::EPSILON.sqrt() && (v - 1.0).abs() < 1e-7,
            "{x} {v}"
        );
        assert_eq!(greatest(|_: f64| None::<f64>, 0.0, 1.0), None);
    }

    #[test]
    fn brent_finds_a_polynomial_root() {
        // (x - 2)(x + 3) = x^2 + x - 6
        let r = brent(|x| x * x + x - 6.0, 0.0, 10.0, Tol::default()).unwrap();
        assert!((r - 2.0).abs() < 1e-12, "got {r}");
    }

    #[test]
    fn brent_rejects_an_unbracketed_interval() {
        assert!(brent(|x| x * x + 1.0, -1.0, 1.0, Tol::default()).is_none());
    }

    #[test]
    fn brent_handles_a_root_at_a_bracket_end() {
        assert_eq!(brent(|x| x, 0.0, 1.0, Tol::default()), Some(0.0));
        assert_eq!(brent(|x| x, -1.0, 0.0, Tol::default()), Some(0.0));
    }

    #[test]
    fn newton_matches_brent_on_a_transcendental() {
        let f = |x: f64| x.cos() - x;
        let df = |x: f64| -x.sin() - 1.0;
        let a = brent(f, 0.0, 2.0, Tol::default()).unwrap();
        let b = newton_bracketed(f, df, 0.0, 2.0, 0.0, Tol::default()).unwrap();
        assert!((a - b).abs() < 1e-12, "{a} vs {b}");
        assert!(f(b).abs() < 1e-12);
    }

    #[test]
    fn newton_survives_a_zero_derivative_at_the_guess() {
        // f' = 0 exactly at the initial guess; must fall back to bisection.
        let f = |x: f64| x * x * x - 1.0;
        let df = |x: f64| 3.0 * x * x;
        let r = newton_bracketed(f, df, -0.5, 3.0, 0.0, Tol::default()).unwrap();
        assert!((r - 1.0).abs() < 1e-10, "got {r}");
    }

    #[test]
    fn newton_rejects_an_unbracketed_interval() {
        assert!(
            newton_bracketed(|x| x * x + 1.0, |x| 2.0 * x, -1.0, 1.0, 0.0, Tol::default())
                .is_none()
        );
    }

    /// **Running out of iterations is a failure, not an answer.**
    ///
    /// Both solvers used to fall out of their loop and return the last iterate,
    /// which contradicts this module's own first paragraph. Nothing exercised
    /// it, which is exactly why it survived: the one guarantee the docstring
    /// makes was the one thing unenforced.
    #[test]
    fn a_solve_that_runs_out_of_iterations_says_so() {
        let stingy = Tol {
            x_tol: 0.0,
            max_iter: 2,
        };
        // A perfectly ordinary root, denied the steps to reach it.
        assert_eq!(brent(|x| x * x - 2.0, 0.0, 10.0, stingy), None);
        assert_eq!(
            newton_bracketed(|x| x * x - 2.0, |x| 2.0 * x, 0.0, 10.0, 9.0, stingy),
            None
        );
        // ...and with room, both find it.
        let d = Tol::default();
        let r = brent(|x| x * x - 2.0, 0.0, 10.0, d).unwrap();
        assert!((r - std::f64::consts::SQRT_2).abs() < 1e-12);
    }

    /// **The convergence test is relative, so it means the same thing at every
    /// magnitude.**
    ///
    /// `newton_bracketed` tested an absolute step against `x_tol = 1e-15`. A
    /// root of order 100 has an ulp of 1.4e-14, so that test could never be met
    /// and the solve ran its full iteration bound — which, now that exhaustion
    /// returns `None`, would be an outright failure rather than a slow success.
    #[test]
    fn a_large_root_converges_rather_than_exhausting_the_bound() {
        for scale in [1e-3_f64, 1.0, 1e3, 1e6] {
            let f = |x: f64| x * x - 2.0 * scale * scale;
            let df = |x: f64| 2.0 * x;
            let want = scale * std::f64::consts::SQRT_2;
            for r in [
                brent(f, 0.0, 10.0 * scale, Tol::default()),
                newton_bracketed(f, df, 0.0, 10.0 * scale, 0.0, Tol::default()),
            ] {
                let r = r.unwrap_or_else(|| panic!("no root at scale {scale}"));
                assert!(
                    (r - want).abs() / want < 1e-14,
                    "scale {scale}: {r} vs {want}"
                );
            }
        }
    }

    #[test]
    fn the_maximiser_finds_an_interior_peak_and_refuses_an_edge() {
        let tol = Tol {
            x_tol: 1e-12,
            max_iter: 200,
        };
        let (x, fx) = maximise(|x: f64| -(x - 0.3).powi(2) + 2.0, 0.0, 1.0, tol).unwrap();
        assert!(
            (x - 0.3).abs() < 1e-6 && (fx - 2.0).abs() < 1e-12,
            "{x} {fx}"
        );
        assert!(
            maximise(|x: f64| x, 0.0, 1.0, tol).is_none(),
            "a peak at the end"
        );
        assert!(maximise(|_| f64::NAN, 0.0, 1.0, tol).is_none());
    }
}
