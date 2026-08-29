//! The involute function and its inverse.
//!
//! `inv α = tan α − α` is the backbone of involute gear geometry. It has no
//! algebraic inverse, and that single fact is the reason three of the five
//! transcendental solves in this crate exist (docs/rationale.md#where-closed-form-is-impossible).

use crate::solve::{newton_bracketed, Tol};

/// The involute function, `inv α = tan α − α`, with `α` in radians.
#[must_use]
pub fn inv(alpha: f64) -> f64 {
    alpha.tan() - alpha
}

/// The involute function expressed through the roll parameter `u = tan α`.
///
/// Cheaper and better conditioned than `inv(atan(u))` when the roll parameter is
/// what you already have, which on a generated flank it usually is.
#[must_use]
pub fn inv_from_roll(u: f64) -> f64 {
    u - u.atan()
}

/// The largest angle the inverse will search.
///
/// `inv` rises without bound as `α → π/2`, so the bracket has to stop somewhere.
/// At this value `inv α ≈ 1e12`, which is about eleven orders of magnitude beyond
/// anything a gear produces — a 60° pressure angle, the most this tool allows,
/// gives `inv α ≈ 0.51`. The bound exists to keep `tan` away from its pole, not
/// to express a physical limit.
const ALPHA_MAX: f64 = std::f64::consts::FRAC_PI_2 - 1e-12;

/// Inverse involute: solve `inv α = v` for `α ∈ [0, π/2)`.
///
/// Returns `None` when `v < 0`. That is not an error case to paper over — `inv α`
/// is non-negative for `α ≥ 0`, so a negative argument means the caller asked for
/// a centre distance smaller than the base circles permit. Planetary ring-gear
/// searches request exactly this for most candidate tooth counts (docs/reference.md#primitives
/// docs/reference.md#planetary-sets), and the honest answer is "impossible", not a NaN.
///
/// # Method
///
/// Seeded from the series `tan α − α = α³/3 + 2α⁵/15 + …`, inverted to
/// `α ≈ (3v)^⅓ − (2/5)v`, then refined by Newton with `d(inv α)/dα = tan²α`.
///
/// The refinement is **bracketed**, and that matters: the bare seed-plus-Newton
/// scheme reaches machine precision in two to four steps up to roughly 60° but
/// diverges above it, and operating pressure angles do exceed that under positive
/// profile shift.
#[must_use]
pub fn inv_inverse(v: f64) -> Option<f64> {
    if v < 0.0 || !v.is_finite() {
        return None;
    }
    if v == 0.0 {
        return Some(0.0);
    }

    // Series seed. The 2/5 is the second Taylor coefficient of the inversion,
    // not a fitted constant.
    let seed = ((3.0 * v).cbrt() - 0.4 * v).clamp(f64::EPSILON, ALPHA_MAX);

    newton_bracketed(
        |a| inv(a) - v,
        |a| {
            let t = a.tan();
            t * t
        },
        0.0,
        ALPHA_MAX,
        seed,
        Tol::default(),
    )
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn inv_round_trips_across_the_whole_allowed_range() {
        // The tool allows pressure angles to 60°; operating angles run higher.
        for deg in 1..=85 {
            let a = f64::from(deg).to_radians();
            let back = inv_inverse(inv(a)).unwrap();
            assert!(
                (back - a).abs() < 1e-12,
                "{deg}°: round trip gave {}°",
                back.to_degrees()
            );
        }
    }

    #[test]
    fn inv_inverse_does_not_diverge_where_the_bare_series_does() {
        // 75° is where seed-plus-unbracketed-Newton runs away (docs/reference.md#primitives).
        let a = 75_f64.to_radians();
        let back = inv_inverse(inv(a)).unwrap();
        assert!((back - a).abs() < 1e-12, "got {}°", back.to_degrees());
    }

    #[test]
    fn negative_argument_has_no_solution() {
        assert!(inv_inverse(-1e-9).is_none());
        assert!(inv_inverse(-1.0).is_none());
    }

    #[test]
    fn zero_maps_to_zero() {
        assert_eq!(inv_inverse(0.0), Some(0.0));
    }

    #[test]
    fn inv_matches_a_known_value() {
        // inv(20°) = 0.0149043838673 (standard tables give 0.0149044)
        assert!((inv(20_f64.to_radians()) - 0.014_904_383_867_336).abs() < 1e-14);
        // inv(14.5°) and inv(25°), likewise standard
        assert!((inv(14.5_f64.to_radians()) - 0.005_544_842_816_712).abs() < 1e-14);
        assert!((inv(25_f64.to_radians()) - 0.029_975_345_156_416).abs() < 1e-14);
    }

    #[test]
    fn roll_form_agrees_with_the_angle_form() {
        for u in [0.05_f64, 0.3, 0.7, 1.5, 3.0] {
            assert!((inv_from_roll(u) - inv(u.atan())).abs() < 1e-14);
        }
    }
}
