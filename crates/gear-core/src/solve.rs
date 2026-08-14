//! Scalar root finding.
//!
//! Two solvers cover every transcendental step in this crate (DESIGN.md §5).
//! Both are **bracketed**, so neither can diverge: a Newton step that leaves the
//! bracket is replaced by a bisection step. That property is not a nicety here —
//! the involute function's series seed diverges above roughly 60°, which is
//! inside the pressure-angle range this tool allows.
//!
//! Neither returns a "best effort" answer on failure. A solve that did not
//! bracket a root returns `None`, and the caller reports the geometry as
//! impossible rather than propagating a NaN into a stress figure.

/// Convergence settings.
///
/// `x_tol` is an absolute tolerance on the abscissa. The defaults target machine
/// precision; `max_iter` is a safety stop, not an expected limit — bisection
/// alone halves the bracket each step, so 200 iterations is far beyond what
/// double precision can use.
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
    let (mut a, mut b) = (lo, hi);
    let (mut fa, mut fb) = (f(a), f(b));
    if !fa.is_finite() || !fb.is_finite() {
        return None;
    }
    if fa == 0.0 {
        return Some(a);
    }
    if fb == 0.0 {
        return Some(b);
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
        if xm.abs() <= tol1 || fb == 0.0 {
            return Some(b);
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
        fb = f(b);
        if !fb.is_finite() {
            return None;
        }
    }
    Some(b)
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
    let (flo, fhi) = (f(lo), f(hi));
    if !flo.is_finite() || !fhi.is_finite() {
        return None;
    }
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
    let mut fx = f(x);
    let mut dfx = df(x);

    for _ in 0..tol.max_iter {
        // Bisect when the Newton step would leave the bracket, or is not at
        // least halving the interval.
        let newton_out_of_range = ((x - high) * dfx - fx) * ((x - low) * dfx - fx) > 0.0;
        let newton_too_slow = (2.0 * fx).abs() > (step_prev * dfx).abs();
        if !dfx.is_finite() || dfx == 0.0 || newton_out_of_range || newton_too_slow {
            step_prev = step;
            step = 0.5 * (high - low);
            x = low + step;
        } else {
            step_prev = step;
            step = fx / dfx;
            x -= step;
        }

        if step.abs() < tol.x_tol {
            return Some(x);
        }

        fx = f(x);
        if !fx.is_finite() {
            return None;
        }
        dfx = df(x);
        if fx < 0.0 {
            low = x;
        } else {
            high = x;
        }
    }
    Some(x)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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
}
