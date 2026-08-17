//! General Hertzian contact — the elliptical solution, of which line contact is
//! a limit.
//!
//! Two bodies touching at a point are, near that point, two quadratic surfaces.
//! Their gap is `h = x²/(2R_x) + y²/(2R_y)` in the principal directions, where
//! each `1/R` is the **relative** curvature — the sum of the two bodies' own
//! curvatures in that direction. Hertz's solution says the contact patch is an
//! ellipse carrying a semi-ellipsoidal pressure
//!
//! ```text
//! p(x,y) = p₀ √(1 − x²/a² − y²/b²),      P = ⅔ π a b p₀
//! ```
//!
//! and the elastic conditions that fix `a` and `b` are, in Carlson form,
//!
//! ```text
//! 1/(2R_x) = (p₀ a b / 3E*) · R_D(b², 0, a²)
//! 1/(2R_y) = (p₀ a b / 3E*) · R_D(a², 0, b²)
//! δ        = (p₀ a b /  E*) · R_F(a², b², 0)
//! ```
//!
//! Written this way there is **no major-axis branch**: the two conditions are
//! the same expression with the arguments exchanged, which is the property
//! DESIGN.md §4.7 requires of the unified contact model and the reason
//! [`crate::elliptic`] exists.
//!
//! # The one solve
//!
//! Dividing the first condition by the second removes the load, the moduli and
//! the size, leaving the ellipse's aspect ratio `κ = b/a` fixed implicitly by
//! the ratio of the curvatures alone:
//!
//! ```text
//! (1/R_x)/(1/R_y) = R_D(κ², 0, 1) / R_D(1, 0, κ²)  ≡  g(κ)
//! ```
//!
//! `g` is monotone from `g(0) = 0` to `g(1) = 1` and has no closed inverse. The
//! published closed forms for it are **fits** — Hamrock–Dowson's
//! `κ ≈ 1.0339(R_y/R_x)^0.636` and its relatives — so §5's rule excludes them,
//! and this is the sixth bracketed scalar solve rather than a correlation. It
//! is done in `ln κ`, so the tolerance is relative: near the line-contact limit
//! `g(κ) ≈ κ² ln(2/κ)`, and a solve with an absolute tolerance would lose the
//! small-`κ` end exactly where the model is meant to stay continuous.
//!
//! Once `κ` is known the size follows in closed form, `a³ = P R_D(κ²,0,1) /
//! (π E* /R_x)`, taking `x` as the direction of *smaller* curvature. Which
//! direction that is comes from comparing two numbers, not from a second
//! formula; the two orderings meet at `κ = 1`, where the contact is circular.
//!
//! # Line contact is the degenerate value, and it is reachable
//!
//! At `1/R_x = 0` — the flat direction of every parallel-axis gear mesh — the
//! ellipse is infinitely long, and a finite load spread over it gives **zero**
//! peak pressure. That is returned as `0.0`, exactly, rather than as a `NaN`
//! from a `0/0` or an error from a solve that could not bracket. It is what
//! makes `σ_H = max(σ_elliptical, σ_line)` collapse to the line term for every
//! mesh the tool supports today, without a branch and without moving a digit.

use crate::elliptic::{r_d, r_f};
use crate::solve::{brent, Tol};
use std::f64::consts::PI;

/// The contact patch and the pressure in it.
#[derive(Clone, Copy, Debug)]
pub struct EllipticalContact {
    /// Semi-axis along the direction of `curvature_x`, mm.
    ///
    /// Infinite in the line-contact limit, which is the honest value: the patch
    /// is bounded by the bodies' own length there, not by elasticity.
    pub semi_x: f64,
    /// Semi-axis along the direction of `curvature_y`, mm.
    pub semi_y: f64,
    /// Peak pressure at the centre of the patch, MPa. This is the Hertzian
    /// contact stress.
    pub max_pressure: f64,
    /// Relative approach of the two bodies, mm.
    ///
    /// Not used by any rating in this crate. It is here because it is the only
    /// output that exercises `R_F`, and because it has an exact textbook value
    /// for spheres (`δ = a²/R`) which makes it a free check on the whole
    /// assembly rather than on the parts.
    pub approach: f64,
}

impl EllipticalContact {
    /// The longer semi-axis, mm.
    #[must_use]
    pub fn semi_major(&self) -> f64 {
        self.semi_x.max(self.semi_y)
    }

    /// The shorter semi-axis, mm.
    #[must_use]
    pub fn semi_minor(&self) -> f64 {
        self.semi_x.min(self.semi_y)
    }
}

/// Hertzian point contact from the two relative principal curvatures.
///
/// `curvature_x` and `curvature_y` are `1/R` in the two principal directions,
/// in 1/mm, each the **sum** of the two bodies' curvatures in that direction.
/// `load` is the total normal force in N, `e_star` the effective contact
/// modulus in MPa, from [`crate::material::contact_modulus`] — the same `E*`
/// the line-contact formula in [`crate::strength`] takes.
///
/// # The degenerate case is a value, not an error
///
/// A zero curvature in one direction returns an infinite semi-axis and zero
/// pressure — the line-contact limit, reached exactly rather than approached.
///
/// # Errors
///
/// `None` if a curvature is negative, if both are zero (the bodies do not
/// touch at a point at all), or if the load or modulus is not positive.
///
/// # Examples
///
/// A sphere on a flat, which has a closed form to check against:
///
/// ```
/// use gear_core::hertz::elliptical_contact;
///
/// let (r, load, e_star) = (10.0, 100.0, 110_000.0);
/// let c = elliptical_contact(1.0 / r, 1.0 / r, load, e_star).unwrap();
///
/// let a = (3.0 * load * r / (4.0 * e_star)).cbrt();      // textbook Hertz
/// assert!((c.semi_x - a).abs() < 1e-12 * a);
/// assert!((c.semi_x - c.semi_y).abs() < 1e-15 * a);      // circular
/// ```
#[must_use]
pub fn elliptical_contact(
    curvature_x: f64,
    curvature_y: f64,
    load: f64,
    e_star: f64,
) -> Option<EllipticalContact> {
    // Written as predicates rather than negated comparisons so that a NaN falls
    // out here rather than propagating into a stress figure.
    let flat_or_curved = |v: f64| v.is_finite() && v >= 0.0;
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if !flat_or_curved(curvature_x) || !flat_or_curved(curvature_y) {
        return None;
    }
    if !positive(load) || !positive(e_star) {
        return None;
    }
    // The larger curvature sets the short axis; the smaller one, which may be
    // zero, sets the long axis. Which is which is a comparison of two numbers,
    // not a second formula — the two orderings agree at kappa = 1.
    let (c_long, c_short) = if curvature_x <= curvature_y {
        (curvature_x, curvature_y)
    } else {
        (curvature_y, curvature_x)
    };
    if c_short <= 0.0 {
        return None;
    }

    let kappa = aspect_ratio(c_long / c_short)?;
    if kappa <= 0.0 {
        // Line contact: infinitely long patch, zero peak pressure.
        let (semi_x, semi_y) = if curvature_x <= curvature_y {
            (f64::INFINITY, 0.0)
        } else {
            (0.0, f64::INFINITY)
        };
        return Some(EllipticalContact {
            semi_x,
            semi_y,
            max_pressure: 0.0,
            approach: 0.0,
        });
    }

    let shape = r_d(kappa * kappa, 0.0, 1.0)?;
    let major = (load * shape / (PI * e_star * c_long)).cbrt();
    let minor = kappa * major;

    let max_pressure = 3.0 * load / (2.0 * PI * major * minor);
    let approach = max_pressure * major * minor * r_f(major * major, minor * minor, 0.0)? / e_star;

    let (semi_x, semi_y) = if curvature_x <= curvature_y {
        (major, minor)
    } else {
        (minor, major)
    };
    Some(EllipticalContact {
        semi_x,
        semi_y,
        max_pressure,
        approach,
    })
}

/// The ellipse's aspect ratio `κ = b/a ∈ [0,1]` for a curvature ratio
/// `q = (1/R_long)/(1/R_short) ∈ [0,1]`.
///
/// Both endpoints are exact and neither is a special case bolted on: `q = 1` is
/// a circular contact and `q = 0` is line contact, and the solve is only
/// consulted strictly between them.
fn aspect_ratio(q: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&q) {
        return None;
    }
    if q == 0.0 {
        return Some(0.0);
    }
    if q == 1.0 {
        return Some(1.0);
    }

    // In ln kappa, so the tolerance is relative. g is monotone, so the residual
    // is too, and Brent cannot leave the bracket.
    let residual = |w: f64| -> f64 {
        let kappa = w.exp();
        match curvature_ratio(kappa) {
            Some(g) => g.ln() - q.ln(),
            None => f64::NAN,
        }
    };

    // kappa² must stay normal for R_D to be evaluable, which is the only floor
    // there is: it comes from the type, not from a choice.
    let floor = f64::MIN_POSITIVE.ln() / 2.0;
    let mut lo = -1.0_f64;
    while residual(lo) > 0.0 {
        lo *= 2.0;
        if lo <= floor {
            // Below anything f64 can express as an aspect ratio. The ellipse is
            // degenerate to the precision available, which is line contact.
            return Some(0.0);
        }
    }

    let w = brent(
        residual,
        lo,
        0.0,
        Tol {
            x_tol: 1e-14,
            max_iter: 200,
        },
    )?;
    Some(w.exp())
}

/// `g(κ) = R_D(κ²,0,1) / R_D(1,0,κ²)` — the curvature ratio an ellipse of
/// aspect ratio `κ` belongs to. Monotone from 0 at `κ = 0` to 1 at `κ = 1`.
fn curvature_ratio(kappa: f64) -> Option<f64> {
    let k2 = kappa * kappa;
    Some(r_d(k2, 0.0, 1.0)? / r_d(1.0, 0.0, k2)?)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// Sphere on sphere and sphere on flat, against the textbook closed forms.
    /// These need no gear and share nothing with the algebra above.
    #[test]
    fn spheres_reproduce_the_textbook_closed_form() {
        for (r1, r2) in [(10.0, f64::INFINITY), (10.0, 10.0), (3.0, 25.0), (0.5, 2.0)] {
            let r_rel = 1.0 / (1.0 / r1 + 1.0 / r2);
            for load in [1.0, 100.0, 5_000.0] {
                for e_star in [1_700.0, 110_000.0] {
                    let c = elliptical_contact(1.0 / r_rel, 1.0 / r_rel, load, e_star).unwrap();

                    let a = (3.0 * load * r_rel / (4.0 * e_star)).cbrt();
                    let p0 = 3.0 * load / (2.0 * PI * a * a);
                    let delta = a * a / r_rel;

                    assert!(
                        (c.semi_x - a).abs() < 1e-12 * a,
                        "R={r_rel} P={load} E*={e_star}: a {} vs {a}",
                        c.semi_x
                    );
                    assert!((c.semi_y - a).abs() < 1e-12 * a, "must be circular");
                    assert!(
                        (c.max_pressure - p0).abs() < 1e-12 * p0,
                        "p0 {} vs {p0}",
                        c.max_pressure
                    );
                    assert!(
                        (c.approach - delta).abs() < 1e-11 * delta,
                        "delta {} vs {delta}",
                        c.approach
                    );
                }
            }
        }
    }

    /// The elastic conditions the solution is *defined* by, checked with the
    /// returned patch by direct quadrature. This shares no code with the
    /// duplication algorithm or the aspect-ratio solve, so it tests the whole
    /// assembly — including the constant in front, which is the easiest thing
    /// to get wrong and the hardest to notice.
    #[test]
    fn the_patch_satisfies_the_elastic_conditions_it_was_derived_from() {
        for (cx, cy) in [
            (0.1, 0.1),
            (0.05, 0.4),
            (0.4, 0.05),
            (0.01, 1.0),
            (0.002, 0.5),
            (1.0 / 3.0, 1.0 / 7.0),
        ] {
            let (load, e_star) = (250.0, 113_000.0);
            let c = elliptical_contact(cx, cy, load, e_star).unwrap();
            let (a, b, p0) = (c.semi_x, c.semi_y, c.max_pressure);

            // 1/(2R_x) = (p0 a b / 2E*) ∫₀^∞ dw (a²+w)^(−3/2)(b²+w)^(−1/2) w^(−1/2)
            let m = improper_sqrt(|w| (a * a + w).powf(-1.5) * (b * b + w).powf(-0.5));
            let n = improper_sqrt(|w| (a * a + w).powf(-0.5) * (b * b + w).powf(-1.5));
            let coefficient = p0 * a * b / (2.0 * e_star);

            assert!(
                (coefficient * m - cx / 2.0).abs() < 1e-8 * cx,
                "curvature_x: {} vs {}",
                coefficient * m,
                cx / 2.0
            );
            assert!(
                (coefficient * n - cy / 2.0).abs() < 1e-8 * cy,
                "curvature_y: {} vs {}",
                coefficient * n,
                cy / 2.0
            );
            // and the load really is what the pressure integrates to
            let total = 2.0 / 3.0 * PI * a * b * p0;
            assert!((total - load).abs() < 1e-9 * load);
        }
    }

    /// Contact is a property of the pair, not of which direction the caller
    /// listed first — the same argument as the existing "which gear is called
    /// 1" test in `strength`.
    #[test]
    fn naming_the_directions_the_other_way_round_changes_nothing() {
        for (cx, cy) in [(0.05, 0.4), (0.3, 0.3), (1e-4, 0.9), (0.7, 0.02)] {
            let a = elliptical_contact(cx, cy, 500.0, 113_000.0).unwrap();
            let b = elliptical_contact(cy, cx, 500.0, 113_000.0).unwrap();
            assert!((a.max_pressure - b.max_pressure).abs() < 1e-12 * a.max_pressure);
            assert!((a.semi_x - b.semi_y).abs() < 1e-12 * a.semi_x);
            assert!((a.semi_y - b.semi_x).abs() < 1e-12 * a.semi_y);
            assert!((a.approach - b.approach).abs() < 1e-12 * a.approach);
        }
    }

    /// The aspect-ratio solve, round-tripped over six decades. This is the
    /// measurement behind the claim that solving in `ln κ` keeps the small end:
    /// with an absolute tolerance the last two cases would be noise.
    #[test]
    fn the_aspect_ratio_round_trips_to_the_line_contact_limit() {
        for kappa in [1.0, 0.9, 0.5, 0.1, 1e-2, 1e-3, 1e-4, 1e-6] {
            let q = curvature_ratio(kappa).unwrap();
            let back = aspect_ratio(q).unwrap();
            assert!(
                (back - kappa).abs() < 1e-11 * kappa,
                "kappa {kappa} -> q {q} -> {back}"
            );
        }
    }

    /// The degenerate end, which is the whole point of the formulation: a flat
    /// direction gives an infinite patch and zero pressure, exactly, and the
    /// approach to it is continuous rather than a jump at zero.
    #[test]
    fn a_flat_direction_is_line_contact_reached_exactly() {
        let c = elliptical_contact(0.0, 0.5, 250.0, 113_000.0).unwrap();
        assert_eq!(c.max_pressure, 0.0);
        assert!(c.semi_x.is_infinite());
        assert_eq!(c.semi_y, 0.0);

        // and coming down to it, the peak pressure falls monotonically to zero
        let mut previous = f64::INFINITY;
        let mut cx = 0.5;
        while cx > 1e-12 {
            let c = elliptical_contact(cx, 0.5, 250.0, 113_000.0).unwrap();
            assert!(
                c.max_pressure < previous,
                "pressure must fall as the flat direction flattens: {} at cx={cx}",
                c.max_pressure
            );
            assert!(c.max_pressure.is_finite() && c.max_pressure > 0.0);
            previous = c.max_pressure;
            cx /= 10.0;
        }
        assert!(
            previous < 400.0,
            "should be well on its way down: {previous}"
        );
    }

    /// Hertz's scalings, which are what a reader will sanity-check the numbers
    /// against: the patch grows as the cube root of the load and the pressure
    /// with its remaining third.
    #[test]
    fn the_load_scalings_are_hertzian() {
        let base = elliptical_contact(0.1, 0.4, 100.0, 113_000.0).unwrap();
        let heavy = elliptical_contact(0.1, 0.4, 800.0, 113_000.0).unwrap();
        assert!(
            (heavy.semi_x / base.semi_x - 2.0).abs() < 1e-9,
            "a ∝ P^(1/3)"
        );
        assert!((heavy.semi_y / base.semi_y - 2.0).abs() < 1e-9);
        assert!(
            (heavy.max_pressure / base.max_pressure - 2.0).abs() < 1e-9,
            "p0 ∝ P^(1/3)"
        );
    }

    #[test]
    fn impossible_contacts_are_refused_rather_than_returning_nonsense() {
        assert!(elliptical_contact(0.0, 0.0, 100.0, 113_000.0).is_none());
        assert!(elliptical_contact(-0.1, 0.5, 100.0, 113_000.0).is_none());
        assert!(elliptical_contact(0.1, 0.5, 0.0, 113_000.0).is_none());
        assert!(elliptical_contact(0.1, 0.5, 100.0, 0.0).is_none());
        assert!(elliptical_contact(f64::NAN, 0.5, 100.0, 113_000.0).is_none());
    }

    /// `∫₀^∞ g(w) w^(−1/2) dw` for smooth `g` — the shape all three Hertz
    /// integrals have.
    ///
    /// Split at `w = 1` and substituted onto `[0,1]` twice: `w = u²` near the
    /// origin, which cancels the `w^(−1/2)` outright rather than integrating
    /// through it, and `w = 1/v²` on the tail, which turns the algebraic decay
    /// into a smooth vanishing. Test-only, and deliberately naive: its job is
    /// to be obviously right, not fast.
    fn improper_sqrt<F: Fn(f64) -> f64>(g: F) -> f64 {
        const N: usize = 200_000;
        #[allow(clippy::cast_precision_loss)]
        let h = 1.0 / N as f64;
        let integrate = |f: &dyn Fn(f64) -> f64| {
            let mut sum = f(0.0) + f(1.0);
            for i in 1..N {
                #[allow(clippy::cast_precision_loss)]
                let x = i as f64 * h;
                sum += if i % 2 == 0 { 2.0 } else { 4.0 } * f(x);
            }
            sum * h / 3.0
        };
        let near = integrate(&|u: f64| 2.0 * g(u * u));
        let tail = integrate(&|v: f64| {
            if v == 0.0 {
                0.0
            } else {
                2.0 * g(1.0 / (v * v)) / (v * v)
            }
        });
        near + tail
    }
}
