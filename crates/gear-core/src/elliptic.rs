//! Carlson symmetric elliptic integrals.
//!
//! These exist here for one reason: **general Hertzian contact**, and
//! specifically the requirement that it not branch (docs/reference.md#contact-stress). The
//! classical elliptical-contact solution is written with `K(e)` and `E(e)`,
//! which forces the caller to know which semi-axis of the contact ellipse is
//! the major one before it can evaluate anything. That is a branch, and it sits
//! exactly where this project needs continuity: a parallel-axis gear mesh is
//! the *degenerate* ellipse, infinitely long, and every mesh the tool supports
//! today lands on it.
//!
//! Carlson's integrals are the standard remedy. `R_F` is symmetric in all three
//! arguments and `R_D` in its first two, so "which axis is major" is not a
//! question the formulas can ask, and both are well conditioned as an argument
//! goes to zero — which is the limit that matters.
//!
//! ```text
//! R_F(x,y,z) = ½ ∫₀^∞ dt [(t+x)(t+y)(t+z)]^(−1/2)
//! R_D(x,y,z) = (3/2) ∫₀^∞ dt [(t+x)(t+y)]^(−1/2) (t+z)^(−3/2)
//! ```
//!
//! `R_D` is the one that carries the contact ellipse's shape; `R_F` sets its
//! size. Both are homogeneous — degree −1/2 and −3/2 respectively — which is
//! what lets the aspect-ratio solve of docs/reference.md#contact-stress be posed in a single dimensionless
//! variable.
//!
//! # How they are computed
//!
//! By **duplication**, which is Carlson's own algorithm and the reason no
//! tables appear here. The transformation
//!
//! ```text
//! λ = √x√y + √y√z + √z√x,    (x,y,z) → ((x+λ)/4, (y+λ)/4, (z+λ)/4)
//! ```
//!
//! leaves the integral's value unchanged (up to a term that is accumulated
//! exactly, in `R_D`'s case) and drives the three arguments together
//! geometrically. Once they agree closely enough, a short Taylor series in
//! their deviations from the mean finishes the job. The series coefficients are
//! exact rationals — they are the terms of that expansion, not a fit — so they
//! are written as fractions rather than decimals.
//!
//! Neither function panics and neither returns a "best effort" answer: an
//! argument outside the domain gives `None`, in keeping with the rest of the
//! crate, because a `NaN` reaching a stress figure is the failure mode that
//! matters here.

/// Relative deviation from the mean at which duplication stops and the series
/// takes over.
///
/// The truncated series omits terms of order `δ⁶`, so the stopping deviation
/// that puts the truncation error at the rounding floor is `ε^(1/6)` ≈ 2.4e-3.
/// It is written out rather than computed because `powf` is not `const`, and a
/// test asserts it really is the sixth root of the machine epsilon rather than
/// a number someone liked.
const TOL: f64 = 2.4e-3;

/// Safety stop on the duplication loop.
///
/// Each step contracts the spread of the arguments by roughly a factor of four,
/// so the count needed is `O(log(max/min))` — a handful for the arguments a
/// contact problem produces. This is a guard against a pathological input, not
/// an expected limit; the same convention as [`crate::solve`].
const MAX_STEPS: u32 = 200;

/// Carlson's `R_F` — the completely symmetric elliptic integral of the first
/// kind.
///
/// ```text
/// R_F(x,y,z) = ½ ∫₀^∞ dt [(t+x)(t+y)(t+z)]^(−1/2)
/// ```
///
/// # Domain
///
/// All three arguments must be non-negative and finite, and **at most one may
/// be zero** — with two zeros the integral diverges. Anything else returns
/// `None`.
///
/// # Examples
///
/// ```
/// use gear_core::elliptic::r_f;
///
/// // Equal arguments collapse to a power: R_F(x,x,x) = 1/√x.
/// let v = r_f(4.0, 4.0, 4.0).unwrap();
/// assert!((v - 0.5).abs() < 1e-15);
/// ```
#[must_use]
pub fn r_f(x: f64, y: f64, z: f64) -> Option<f64> {
    if !domain_ok(x, y, z) {
        return None;
    }
    // Two zeros make the integral divergent; one is fine and is the case the
    // contact problem actually uses.
    if x + y <= 0.0 || y + z <= 0.0 || z + x <= 0.0 {
        return None;
    }

    let (mut x, mut y, mut z) = (x, y, z);
    let mut steps = 0;
    let (mu, dx, dy, dz) = loop {
        let lambda = duplication_lambda(x, y, z);
        x = 0.25 * (x + lambda);
        y = 0.25 * (y + lambda);
        z = 0.25 * (z + lambda);

        let mu = (x + y + z) / 3.0;
        let (dx, dy, dz) = ((mu - x) / mu, (mu - y) / mu, (mu - z) / mu);
        if deviation(dx, dy, dz) <= TOL {
            break (mu, dx, dy, dz);
        }
        steps += 1;
        if steps >= MAX_STEPS {
            return None;
        }
    };

    // The elementary symmetric polynomials in the deviations. `dx + dy + dz`
    // is zero by construction, which is why only these two appear.
    let e2 = dx * dy - dz * dz;
    let e3 = dx * dy * dz;
    let series = 1.0 - e2 / 10.0 + e3 / 14.0 + e2 * e2 / 24.0 - 3.0 * e2 * e3 / 44.0;
    Some(series / mu.sqrt())
}

/// Carlson's `R_D` — the symmetric elliptic integral of the second kind.
///
/// ```text
/// R_D(x,y,z) = (3/2) ∫₀^∞ dt [(t+x)(t+y)]^(−1/2) (t+z)^(−3/2)
/// ```
///
/// Symmetric in `x` and `y` only: `z` is the argument carrying the `−3/2`
/// power, so it is distinguished by the integral itself rather than by a choice
/// made in the caller.
///
/// # Domain
///
/// `x` and `y` must be non-negative with **at most one of them zero**, and `z`
/// must be strictly positive. Anything else returns `None`.
///
/// # The limit that matters
///
/// `R_D(κ², 0, 1)` grows like `−3 ln κ` as `κ → 0` while `R_D(1, 0, κ²)` grows
/// like `3/κ²`, so their ratio — which is what fixes the contact ellipse's
/// aspect ratio — vanishes like `−κ² ln κ`. It is finite and monotone all the
/// way down; it is the *ratio* that degenerates, not either integral, which is
/// the property that keeps the line-contact limit reachable rather than
/// asymptotic.
///
/// # Examples
///
/// ```
/// use gear_core::elliptic::r_d;
///
/// let v = r_d(4.0, 4.0, 4.0).unwrap();
/// assert!((v - 0.125).abs() < 1e-15);        // R_D(x,x,x) = x^(−3/2)
/// ```
#[must_use]
pub fn r_d(x: f64, y: f64, z: f64) -> Option<f64> {
    if !domain_ok(x, y, z) {
        return None;
    }
    if x + y <= 0.0 || z <= 0.0 {
        return None;
    }

    let (mut x, mut y, mut z) = (x, y, z);
    // The `−3/2` argument means each duplication leaves behind a term that has
    // to be carried rather than absorbed, unlike `R_F`. `factor` is the weight
    // that term takes at the step it was produced.
    let mut carried = 0.0;
    let mut factor = 1.0;
    let mut steps = 0;
    let (mu, dx, dy, dz) = loop {
        let sz = z.sqrt();
        let lambda = duplication_lambda(x, y, z);
        carried += factor / (sz * (z + lambda));
        factor *= 0.25;

        x = 0.25 * (x + lambda);
        y = 0.25 * (y + lambda);
        z = 0.25 * (z + lambda);

        // `z` is weighted three times here, matching the power it carries.
        let mu = (x + y + 3.0 * z) / 5.0;
        let (dx, dy, dz) = ((mu - x) / mu, (mu - y) / mu, (mu - z) / mu);
        if deviation(dx, dy, dz) <= TOL {
            break (mu, dx, dy, dz);
        }
        steps += 1;
        if steps >= MAX_STEPS {
            return None;
        }
    };

    let ea = dx * dy;
    let eb = dz * dz;
    let ec = ea - eb;
    let ed = ea - 6.0 * eb;
    let ee = ed + ec + ec;
    let series = 1.0
        + ed * (-(3.0 / 14.0) + (9.0 / 88.0) * ed - (9.0 / 52.0) * dz * ee)
        + dz * (ee / 6.0 + dz * (-(9.0 / 22.0) * ec + dz * (3.0 / 26.0) * ea));
    Some(3.0 * carried + factor * series / (mu * mu.sqrt()))
}

/// The duplication step's `λ`.
fn duplication_lambda(x: f64, y: f64, z: f64) -> f64 {
    let (sx, sy, sz) = (x.sqrt(), y.sqrt(), z.sqrt());
    sx * (sy + sz) + sy * sz
}

/// How far the arguments still are from their mean, relatively.
fn deviation(dx: f64, dy: f64, dz: f64) -> f64 {
    dx.abs().max(dy.abs()).max(dz.abs())
}

/// Finite and non-negative. The per-function domain rules are separate, because
/// they differ.
fn domain_ok(x: f64, y: f64, z: f64) -> bool {
    [x, y, z]
        .iter()
        .all(|v| v.is_finite() && !v.is_sign_negative())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    /// The stopping tolerance is claimed to be the sixth root of the machine
    /// epsilon. Assert that rather than trusting the comment: if someone
    /// "tidies" it to a round number the truncation error moves off the
    /// rounding floor silently.
    #[test]
    fn the_stopping_tolerance_is_the_sixth_root_of_epsilon() {
        let derived = f64::EPSILON.powf(1.0 / 6.0);
        assert!(
            TOL <= derived && TOL > derived / 2.0,
            "TOL {TOL} should be just under {derived}"
        );
    }

    #[test]
    fn equal_arguments_give_the_elementary_powers() {
        for x in [0.25_f64, 1.0, 2.0, 4.0, 100.0] {
            let f = r_f(x, x, x).unwrap();
            let d = r_d(x, x, x).unwrap();
            assert!((f - x.powf(-0.5)).abs() < 1e-15 * f, "R_F({x},{x},{x})");
            assert!((d - x.powf(-1.5)).abs() < 1e-15 * d, "R_D({x},{x},{x})");
        }
    }

    /// One zero argument is legal and has a closed form in both cases, which is
    /// the cheapest check that the degenerate end is handled at all.
    #[test]
    fn one_zero_argument_gives_the_closed_forms() {
        for y in [0.5_f64, 1.0, 3.0, 25.0] {
            let f = r_f(0.0, y, y).unwrap();
            let d = r_d(0.0, y, y).unwrap();
            assert!(
                (f - PI / (2.0 * y.sqrt())).abs() < 1e-14 * f,
                "R_F(0,{y},{y})"
            );
            assert!(
                (d - 3.0 * PI / (4.0 * y * y.sqrt())).abs() < 1e-13 * d,
                "R_D(0,{y},{y})"
            );
        }
    }

    /// Homogeneity is what lets the aspect-ratio solve be posed in one
    /// dimensionless variable, so it is load-bearing rather than decorative.
    #[test]
    fn both_are_homogeneous_in_their_arguments() {
        let (x, y, z) = (0.3, 1.7, 4.1);
        for lambda in [1e-6_f64, 0.5, 3.0, 1e6] {
            let f = r_f(lambda * x, lambda * y, lambda * z).unwrap();
            let d = r_d(lambda * x, lambda * y, lambda * z).unwrap();
            let f0 = r_f(x, y, z).unwrap() * lambda.powf(-0.5);
            let d0 = r_d(x, y, z).unwrap() * lambda.powf(-1.5);
            assert!((f - f0).abs() < 1e-14 * f, "R_F at lambda={lambda}");
            assert!((d - d0).abs() < 1e-14 * d, "R_D at lambda={lambda}");
        }
    }

    /// The whole reason these were chosen over `K`/`E`: no argument is
    /// privileged, so there is no major-axis branch for a caller to get wrong.
    #[test]
    fn the_symmetries_hold_exactly_as_the_definitions_claim() {
        let (x, y, z) = (0.2, 1.3, 5.0);
        let f = r_f(x, y, z).unwrap();
        for (a, b, c) in [(x, z, y), (y, x, z), (y, z, x), (z, x, y), (z, y, x)] {
            assert!(
                (r_f(a, b, c).unwrap() - f).abs() < 1e-15 * f,
                "R_F symmetry"
            );
        }
        // R_D is symmetric in its first two arguments only.
        let d = r_d(x, y, z).unwrap();
        assert!(
            (r_d(y, x, z).unwrap() - d).abs() < 1e-15 * d,
            "R_D symmetry"
        );
        assert!(
            (r_d(x, z, y).unwrap() - d).abs() > 1e-3 * d,
            "R_D must NOT be symmetric in its third argument"
        );
    }

    /// Against a direct quadrature of the defining integrals — an independent
    /// route that shares no code with the duplication algorithm.
    ///
    /// The infinite range is split at `t = 1` and each half substituted so both
    /// integrands are smooth on `[0,1]`: `t = u²` on the near half, which also
    /// removes the `1/√t` behaviour at the origin, and `t = 1/w²` on the far
    /// half, which turns the `t^(−3/2)` tail into a constant.
    #[test]
    fn both_match_a_direct_quadrature_of_their_own_definition() {
        for (x, y, z) in [
            (1.0, 2.0, 3.0),
            (0.5, 0.5, 4.0),
            (0.01, 1.0, 1.0),
            (1.0, 1.0, 100.0),
            (2.5, 0.3, 0.7),
            (1e-4, 1.0, 1.0),
        ] {
            let f = r_f(x, y, z).unwrap();
            let near = crate::testing::simpson_unit_interval(|u| {
                2.0 * u / ((u * u + x) * (u * u + y) * (u * u + z)).sqrt()
            });
            let far = crate::testing::simpson_unit_interval(|w| {
                let (a, b, c) = (1.0 + x * w * w, 1.0 + y * w * w, 1.0 + z * w * w);
                2.0 / (a * b * c).sqrt()
            });
            let quad = 0.5 * (near + far);
            assert!(
                (f - quad).abs() < 1e-11 * f,
                "R_F({x},{y},{z}): {f} vs quadrature {quad}"
            );

            let d = r_d(x, y, z).unwrap();
            let near = crate::testing::simpson_unit_interval(|u| {
                2.0 * u / (((u * u + x) * (u * u + y)).sqrt() * (u * u + z).powf(1.5))
            });
            let far = crate::testing::simpson_unit_interval(|w| {
                let (a, b, c) = (1.0 + x * w * w, 1.0 + y * w * w, 1.0 + z * w * w);
                2.0 * w * w / ((a * b).sqrt() * c.powf(1.5))
            });
            let quad = 1.5 * (near + far);
            assert!(
                (d - quad).abs() < 1e-11 * d,
                "R_D({x},{y},{z}): {d} vs quadrature {quad}"
            );
        }
    }

    /// The classical complete integrals, recovered from the symmetric ones and
    /// checked against the arithmetic–geometric mean — a completely different
    /// algorithm.
    ///
    /// This is the equivalence the design claims when it says Carlson's form is
    /// the same theory written without the branch, and it exercises the
    /// zero-argument case that the quadrature above deliberately avoids.
    #[test]
    fn the_classical_complete_integrals_come_out_of_them() {
        for m in [0.0, 0.1, 0.25, 0.5, 0.75, 0.9, 0.99] {
            let k_carlson = r_f(0.0, 1.0 - m, 1.0).unwrap();
            let e_carlson = k_carlson - (m / 3.0) * r_d(0.0, 1.0 - m, 1.0).unwrap();
            let (k_agm, e_agm) = complete_by_agm(m);
            assert!(
                (k_carlson - k_agm).abs() < 1e-13 * k_agm,
                "K({m}): {k_carlson} vs AGM {k_agm}"
            );
            assert!(
                (e_carlson - e_agm).abs() < 1e-13 * e_agm,
                "E({m}): {e_carlson} vs AGM {e_agm}"
            );
        }
    }

    /// The line-contact limit is approached, not jumped to: both integrals stay
    /// finite and monotone as an argument runs down to 1e-12, and neither
    /// produces a `NaN` on the way. This is the conditioning claim in the
    /// module documentation, and it is the property the general Hertz solve
    /// will stand on.
    #[test]
    fn the_degenerate_limit_stays_finite_and_monotone() {
        let mut previous_ratio = f64::INFINITY;
        let mut kappa = 1.0;
        while kappa > 1e-12 {
            let shape = r_d(kappa * kappa, 0.0, 1.0).unwrap();
            let size = r_d(1.0, 0.0, kappa * kappa).unwrap();
            assert!(shape.is_finite() && shape > 0.0, "R_D(k²,0,1) at {kappa}");
            assert!(size.is_finite() && size > 0.0, "R_D(1,0,k²) at {kappa}");
            let ratio = shape / size;
            assert!(
                ratio < previous_ratio,
                "the curvature ratio must fall monotonically toward zero: \
                 {ratio} at kappa={kappa} did not beat {previous_ratio}"
            );
            previous_ratio = ratio;
            kappa /= 4.0;
        }
        assert!(previous_ratio < 1e-20, "should be heading to zero");
    }

    /// A divergent integral is refused rather than returned as an infinity, and
    /// a negative argument is refused rather than turned into a `NaN` by the
    /// first square root.
    #[test]
    fn arguments_outside_the_domain_are_refused() {
        assert!(r_f(0.0, 0.0, 1.0).is_none(), "two zeros diverge");
        assert!(r_f(-1.0, 1.0, 1.0).is_none());
        assert!(r_f(f64::NAN, 1.0, 1.0).is_none());
        assert!(r_f(f64::INFINITY, 1.0, 1.0).is_none());
        assert!(r_d(0.0, 0.0, 1.0).is_none(), "two zeros diverge");
        assert!(
            r_d(1.0, 1.0, 0.0).is_none(),
            "the third argument cannot be zero"
        );
        assert!(r_d(1.0, -1.0, 1.0).is_none());
        // and the legal degenerate cases are not caught by those guards
        assert!(r_f(0.0, 1.0, 1.0).is_some());
        assert!(r_d(0.0, 1.0, 1.0).is_some());
    }

    /// `K(m)` and `E(m)` by the arithmetic–geometric mean, which shares nothing
    /// with the duplication algorithm above.
    fn complete_by_agm(m: f64) -> (f64, f64) {
        let mut a = 1.0_f64;
        let mut b = (1.0 - m).sqrt();
        let mut c = m.sqrt();
        // The n = 0 term of the sum below is c₀²/2 = m/2.
        let mut sum = 0.5 * c * c;
        let mut weight = 0.5;
        for _ in 0..60 {
            if c.abs() < 1e-18 {
                break;
            }
            let a_next = 0.5 * (a + b);
            let b_next = (a * b).sqrt();
            c = 0.5 * (a - b);
            a = a_next;
            b = b_next;
            weight *= 2.0;
            sum += weight * c * c;
        }
        let k = PI / (2.0 * a);
        (k, k * (1.0 - sum))
    }
}
