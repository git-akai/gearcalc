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
//! docs/reference.md#contact-stress requires of the unified contact model and the reason
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
//! `κ ≈ 1.0339(R_y/R_x)^0.636` and its relatives — so docs/rationale.md#where-closed-form-is-impossible's rule excludes them,
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

/// The peak pressure a contact presses with, once the **bodies' own extent** is
/// allowed to truncate the elastic patch.
///
/// ```text
/// σ_H = max( σ_elliptical , σ_line )        σ_line = √( (P/L) · (1/R_y) · E*/π )
/// ```
///
/// `curvature_along` and `curvature_across` are the two relative principal
/// curvatures from [`relative_curvatures`], flatter first — `along` is the
/// direction the patch lengthens in, `across` the one it is pinched in. `load`
/// is the total normal force in N, `line_length` how much contact line the two
/// bodies actually have in the `along` direction (mm), and `e_star` the
/// effective contact modulus.
///
/// # Why the two models are a `max` and not a choice
///
/// They are the same contact seen under two different limits, and which one is
/// the truth is decided by the *bodies*, not by the elasticity. The elliptical
/// solution assumes half-spaces of unlimited extent: as `curvature_along → 0`
/// its patch lengthens without bound and its peak pressure falls to zero. A real
/// tooth is not unlimited — its contact line runs out at the face — so once the
/// ellipse is longer than `line_length` the load is carried on the length that
/// exists, which is the line-contact term. The two cross exactly once, and the
/// larger is the physical one on each side of the crossing.
///
/// Near the crossing the truth sits slightly **above** both, since a truncated
/// ellipse concentrates load more than a uniform line does. That is the honest
/// limit of this expression rather than something papered over.
///
/// # One home, because it had two answers
///
/// The parallel path has had this `max` since general contact arrived, where it
/// collapses to the line term for every uncrowned mesh (`curvature_along` is
/// [`PARALLEL_AXES`](crate::strength::PARALLEL_AXES), a named zero, so the
/// elliptical term is *exactly* zero and the line term is returned bit for bit).
/// The crossed path evaluated the ellipse **alone** — correct at a worm's 90°,
/// where the patch is a fraction of the face, and badly optimistic as the shafts
/// come parallel: at `Σ = 0.5°` on a 10 mm face the ellipse wants to be 42 mm
/// long and reports 369 MPa where the line it actually has carries 618. Both
/// paths ask this one function (`docs/corrections.md`).
///
/// # Both degenerate ends are values, and only `0/0` is not
///
/// This module's convention is that a degenerate case is a *value* rather than
/// an error: at `curvature_along = 0` the ellipse is infinitely long and presses
/// with **exactly zero**. A zero `line_length` is the same statement from the
/// other end — a face of no width carries its load on no line — and its value is
/// an infinite pressure. Both are honest, and a stage that resolves to a zero
/// face width because no rating was enabled to size it therefore still reports,
/// with the infinity saying plainly what a zero-width gear does.
///
/// The one case with no answer is a zero load on a zero line, which is `0/0` and
/// means nothing at all.
///
/// # Errors
///
/// `None` if `e_star` or `curvature_across` is not positive, if `load` or
/// `line_length` is negative or not finite, or for the `0/0` above.
#[must_use]
pub fn peak_pressure(
    curvature_along: f64,
    curvature_across: f64,
    load: f64,
    line_length: f64,
    e_star: f64,
) -> Option<f64> {
    let positive = |v: f64| v.is_finite() && v > 0.0;
    let non_negative = |v: f64| v.is_finite() && v >= 0.0;
    if !positive(e_star) || !positive(curvature_across) {
        return None;
    }
    // `curvature_along` reaches only the elliptical term, which would answer a
    // NaN with `None` and let the line term carry the result — so it is checked
    // here rather than left to fall through as a silently ignored argument.
    if !non_negative(load) || !non_negative(line_length) || !non_negative(curvature_along) {
        return None;
    }
    if line_length == 0.0 && load == 0.0 {
        return None;
    }
    let line = (load / line_length * curvature_across * e_star / PI).sqrt();
    // A patch that cannot exist — no load, say — carries no pressure, which the
    // line term still can. So a failed ellipse contributes zero rather than
    // refusing the whole answer.
    let elliptical = elliptical_contact(curvature_along, curvature_across, load, e_star)
        .map_or(0.0, |c| c.max_pressure);
    Some(line.max(elliptical))
}

/// The two **relative** principal curvatures of a contacting pair, from each
/// body's own principal curvatures and the angle between their principal
/// planes.
///
/// This is the step between a pair of surfaces and [`elliptical_contact`],
/// which wants the relative curvatures and nothing else. Each body is given as
/// `(1/R', 1/R'')` in 1/mm — its two principal curvatures, positive for a
/// convex surface — and `skew` is the angle from body 1's first principal plane
/// to body 2's.
///
/// ```text
/// 2A + 2B = Σ curvatures
/// 2B − 2A = √[ (Δ₁ + Δ₂)² − 4 Δ₁ Δ₂ sin²ψ ],     Δᵢ = 1/Rᵢ′ − 1/Rᵢ″
/// ```
///
/// The returned pair is `(2A, 2B)` with `2A ≤ 2B`: the flatter direction first,
/// which is the one that degenerates.
///
/// # Why it is written with `sin²ψ`
///
/// The textbook form of the second line carries `cos 2ψ`, and the two are the
/// same identity apart. This one is **exact at `ψ = 0`**: the radicand collapses
/// to `(Δ₁ + Δ₂)²`, whose square root is that sum to the last bit, so the
/// flatter relative curvature comes back as **exactly zero** rather than as a
/// rounding-sized positive number. Parallel cylinders therefore reach line
/// contact exactly, which is the property the whole docs/reference.md#contact-stress unification rests on —
/// with the `cos 2ψ` form the same case would arrive as an ellipse a few
/// hundred metres long, and while that is harmless in the `max`, "harmless"
/// is not the claim being made.
///
/// # Errors
///
/// `None` if either relative curvature comes out negative — the surfaces are
/// conformal in that direction and do not touch at a point at all, which
/// Hertz's theory does not cover.
#[must_use]
pub fn relative_curvatures(
    body_1: (f64, f64),
    body_2: (f64, f64),
    skew: f64,
) -> Option<(f64, f64)> {
    let finite = |v: f64| v.is_finite();
    if !finite(body_1.0) || !finite(body_1.1) || !finite(body_2.0) || !finite(body_2.1) {
        return None;
    }

    let sum = body_1.0 + body_1.1 + body_2.0 + body_2.1;
    let d1 = body_1.0 - body_1.1;
    let d2 = body_2.0 - body_2.1;

    let s = skew.sin();
    let radicand = (d1 + d2) * (d1 + d2) - 4.0 * d1 * d2 * s * s;
    // Rounding can put a vanishing radicand a hair below zero; the difference
    // it represents is zero either way.
    let difference = radicand.max(0.0).sqrt();

    let flatter = 0.5 * (sum - difference);
    let sharper = 0.5 * (sum + difference);
    if flatter < 0.0 || sharper <= 0.0 {
        return None;
    }
    Some((flatter, sharper))
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
    use std::f64::consts::FRAC_PI_2;

    /// Parallel cylinders are the gear case, and the relative curvature must
    /// come out as the flat direction being **exactly** flat — not nearly. This
    /// is what lets a crossed-axis stage at zero shaft angle return the same
    /// number a parallel one does.
    #[test]
    fn parallel_cylinders_are_exactly_line_contact() {
        for (r1, r2) in [(10.0, 10.0), (3.0, 25.0), (1.5, 0.4), (100.0, 2.0)] {
            let (flat, sharp) = relative_curvatures((1.0 / r1, 0.0), (1.0 / r2, 0.0), 0.0).unwrap();
            assert_eq!(
                flat, 0.0,
                "R={r1}/{r2}: the flat direction must be exactly 0"
            );
            // and the sharp one is the relative curvature the line-contact
            // formula in `strength` uses
            assert!((sharp - (1.0 / r1 + 1.0 / r2)).abs() < 1e-15 * sharp);
        }
    }

    /// Cylinders crossed at a right angle: the classic case, where the two
    /// relative curvatures are simply the two bodies' own.
    #[test]
    fn cylinders_crossed_at_a_right_angle_keep_their_own_curvatures() {
        for (r1, r2) in [(10.0, 10.0), (3.0, 25.0), (0.4, 1.5)] {
            let (flat, sharp) =
                relative_curvatures((1.0 / r1, 0.0), (1.0 / r2, 0.0), FRAC_PI_2).unwrap();
            let (lo, hi) = if r1 >= r2 {
                (1.0 / r1, 1.0 / r2)
            } else {
                (1.0 / r2, 1.0 / r1)
            };
            assert!(
                (flat - lo).abs() < 1e-15 * hi,
                "R={r1}/{r2}: {flat} vs {lo}"
            );
            assert!((sharp - hi).abs() < 1e-15 * hi);
        }
    }

    /// Equal cylinders crossed at a right angle press a **circular** patch —
    /// the standard result, and a check that runs the whole way through to a
    /// contact solution rather than stopping at the curvatures.
    #[test]
    fn equal_crossed_cylinders_press_a_circle() {
        let r = 8.0;
        let (cx, cy) = relative_curvatures((1.0 / r, 0.0), (1.0 / r, 0.0), FRAC_PI_2).unwrap();
        let c = elliptical_contact(cx, cy, 400.0, 113_000.0).unwrap();
        assert!(
            (c.semi_x - c.semi_y).abs() < 1e-12 * c.semi_x,
            "{} vs {}",
            c.semi_x,
            c.semi_y
        );
        // and it is the sphere-on-sphere answer for the same relative radius
        let sphere = elliptical_contact(1.0 / r, 1.0 / r, 400.0, 113_000.0).unwrap();
        assert!((c.max_pressure - sphere.max_pressure).abs() < 1e-12 * sphere.max_pressure);
    }

    /// Against an independent route: build each body's curvature as a 2×2 form,
    /// rotate the second one, add them, and take the eigenvalues. That is the
    /// derivation the closed form summarises, done by different arithmetic, and
    /// it is what actually tests the skew term.
    #[test]
    fn the_closed_form_matches_the_eigenvalues_of_the_summed_curvature_forms() {
        for skew_deg in [0.0, 15.0, 30.0, 45.0, 60.0, 90.0, 120.0, 175.0] {
            let skew = f64::to_radians(skew_deg);
            for (b1, b2) in [
                ((0.1, 0.0), (0.25, 0.0)),
                ((0.4, 0.05), (0.2, 0.02)),
                ((0.3, 0.3), (0.1, 0.0)),
                ((1.0 / 7.0, 1.0 / 90.0), (1.0 / 20.0, 0.0)),
            ] {
                let (flat, sharp) = relative_curvatures(b1, b2, skew).unwrap();

                // Body 1 in its own frame, body 2 rotated into it. The quadratic
                // form of a surface with principal curvatures (k', k'') is
                // diag(k'/2, k''/2); the gap is the sum of the two.
                let (c, s) = (skew.cos(), skew.sin());
                let (p, q) = (b2.0 / 2.0, b2.1 / 2.0);
                let a11 = b1.0 / 2.0 + p * c * c + q * s * s;
                let a22 = b1.1 / 2.0 + p * s * s + q * c * c;
                let a12 = (p - q) * s * c;

                let mean = 0.5 * (a11 + a22);
                let spread = (0.25 * (a11 - a22) * (a11 - a22) + a12 * a12).sqrt();
                let (lo, hi) = (2.0 * (mean - spread), 2.0 * (mean + spread));

                assert!(
                    (flat - lo).abs() < 1e-13 * hi.max(1e-9),
                    "skew={skew_deg} b1={b1:?} b2={b2:?}: flat {flat} vs eigenvalue {lo}"
                );
                assert!(
                    (sharp - hi).abs() < 1e-13 * hi,
                    "skew={skew_deg}: sharp {sharp} vs eigenvalue {hi}"
                );
            }
        }
    }

    /// Two invariants worth pinning: the curvatures sum to the same total
    /// however the bodies are turned, and turning them by a straight angle
    /// changes nothing, because a principal plane has no direction.
    #[test]
    fn the_sum_is_invariant_and_a_straight_angle_changes_nothing() {
        let (b1, b2) = ((0.3, 0.05), (0.12, 0.02));
        let total = b1.0 + b1.1 + b2.0 + b2.1;
        for skew_deg in [0.0, 23.0, 90.0, 137.0] {
            let skew = f64::to_radians(skew_deg);
            let (flat, sharp) = relative_curvatures(b1, b2, skew).unwrap();
            assert!(
                (flat + sharp - total).abs() < 1e-15 * total,
                "skew={skew_deg}"
            );

            let turned = relative_curvatures(b1, b2, skew + PI).unwrap();
            assert!((turned.0 - flat).abs() < 1e-15 * total);
            assert!((turned.1 - sharp).abs() < 1e-15 * total);
        }
        // and the pair is symmetric in the two bodies
        let a = relative_curvatures(b1, b2, 0.7).unwrap();
        let b = relative_curvatures(b2, b1, 0.7).unwrap();
        assert!((a.0 - b.0).abs() < 1e-15 * total && (a.1 - b.1).abs() < 1e-15 * total);
    }

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
            let m =
                crate::testing::improper_sqrt(|w| (a * a + w).powf(-1.5) * (b * b + w).powf(-0.5));
            let n =
                crate::testing::improper_sqrt(|w| (a * a + w).powf(-0.5) * (b * b + w).powf(-1.5));
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

    /// **The two models cross exactly once, and the larger is the answer on
    /// each side.** Sweeping the flat direction from a point contact down to a
    /// line one, the elliptical term falls to zero while the line term does not
    /// move at all — so there is one crossing, and `peak_pressure` must track
    /// the upper envelope through it without a step.
    #[test]
    fn the_governing_model_changes_once_and_the_answer_does_not_step() {
        let (across, load, line, e_star) = (0.5, 250.0, 10.0, 113_000.0);
        let line_only = (load / line * across * e_star / PI).sqrt();

        let mut crossings = 0;
        let mut previous: Option<(f64, bool)> = None;
        let mut along = 1.0;
        while along > 1e-14 {
            let got = peak_pressure(along, across, load, line, e_star).unwrap();
            let ellipse = elliptical_contact(along, across, load, e_star)
                .unwrap()
                .max_pressure;
            assert!(
                (got - line_only.max(ellipse)).abs() < 1e-12 * got,
                "along={along}: {got} is not the larger of {ellipse} and {line_only}"
            );
            let line_governs = line_only > ellipse;
            if let Some((was, then)) = previous {
                if then != line_governs {
                    crossings += 1;
                }
                // Continuity through the seam: the envelope of two continuous
                // curves is continuous, so no step may appear at the crossing.
                assert!(
                    (got - was).abs() < 0.25 * was,
                    "along={along}: {got} stepped from {was}"
                );
            }
            previous = Some((got, line_governs));
            along /= 1.3;
        }
        assert_eq!(crossings, 1, "the two models must cross exactly once");
        // ...and at the line-contact limit the line term is all that is left.
        let at_limit = peak_pressure(0.0, across, load, line, e_star).unwrap();
        assert!((at_limit - line_only).abs() < 1e-15 * line_only);
    }

    /// Both degenerate ends are values; only `0/0` has no answer.
    #[test]
    fn the_degenerate_ends_are_values_and_only_nothing_over_nothing_is_not() {
        // No line to press on, but a load to press with: unbounded pressure.
        assert_eq!(
            peak_pressure(0.0, 0.5, 250.0, 0.0, 113_000.0),
            Some(f64::INFINITY)
        );
        // No load at all: no pressure, which is a number.
        assert_eq!(peak_pressure(0.0, 0.5, 0.0, 10.0, 113_000.0), Some(0.0));
        // No load and no line: nothing at all.
        assert_eq!(peak_pressure(0.0, 0.5, 0.0, 0.0, 113_000.0), None);
        // ...and the ordinary refusals.
        assert!(peak_pressure(0.0, 0.0, 250.0, 10.0, 113_000.0).is_none());
        assert!(peak_pressure(0.0, 0.5, -1.0, 10.0, 113_000.0).is_none());
        assert!(peak_pressure(0.0, 0.5, 250.0, 10.0, 0.0).is_none());
        assert!(peak_pressure(f64::NAN, 0.5, 250.0, 10.0, 113_000.0).is_none());
    }
}
