//! The lengthwise influence along a contact line: how a unit line load on one panel of the line
//! moves one body's surface at a point of the line, relative to a depth below it.
//!
//! A line load `q` (N/mm) is spread across the line as the Hertz half-ellipse of half-width `b`,
//! `φ(x) = (2/(π b)) √(1 − x²/b²)`. On the centre line, at distance `η` along the line from a
//! load element, Boussinesq gives the surface displacement `(1 − ν²)/(π E) ∫ φ(x) dx / √(x² + η²)`,
//! so a panel `[η₁, η₂]` of unit load moves the point by `(1 − ν²)/(π E) [G(η₂/b) − G(η₁/b)]`,
//! with one special function, odd in `r`:
//!
//! ```text
//! G(r)  = ∫ φ(t) asinh(r/|t|) dt  (b = 1)  = (4/π) ∫₀^{π/2} cos²θ asinh(r / sin θ) dθ
//! G′(r) = (4/(3π)) (1 + r²) R_D(0, r², 1 + r²)
//! G(r)  = ln 4r + 1/2 + 1/(16 r²) + O(r⁻⁴)   as r → ∞
//! G(r)  = (4/π) r ln(4/r) + O(r³ ln r)         as r → 0
//! ```
//!
//! The tooth model takes over at depth `h` below the contact, so a panel's influence is the
//! surface's displacement minus the same load's displacement at depth `h` under the point
//! (Boussinesq's interior field), per unit of `(1 + ν)/(2π E)`:
//!
//! ```text
//! ∫ φ(x) [2(1 − ν)(asinh(η₂/ρ) − asinh(η₁/ρ)) + h² (η₂/(ρ² R₂) − η₁/(ρ² R₁))] dx,
//! ρ² = x² + h²,  R_i² = ρ² + η_i²,
//! ```
//!
//! and over an infinite uniform line it integrates to the plane's depth-referenced line
//! compliance ([`HalfSpace::line_limit`]).
//!
//! Two exact forms of one integral ([`Route`]): near the point, the surface term through `G` and
//! the depth term by Gauss–Chebyshev across the width; separated (both ends on one side, the
//! nearer at least `b/2` away: [`SEPARATED_FROM`]), one integrand in which every difference is a
//! product or sum of terms of one sign. Far out the two terms of the first form each grow as
//! `ln r` while their difference decays as `−ν h²/r³`, so the first form would carry `G`'s error
//! amplified by up to `3e7` (`h < b`); the second has none to amplify.
//!
//! Ports the prototype's `kernel.G`, `G_direct`, `dG`, `panel`, `panel_sep`, `depth_panel`,
//! `n_width`, `line_limit` and `shape_C`, and `carlson.aspect` through [`patch_aspect`]; its
//! records are `tests/data/field_oracle/kernel.json`. Every length here is relative: the kernel is
//! unchanged when every length is scaled.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, LN_2, PI};
use std::sync::OnceLock;

use crate::elliptic::r_d;
use crate::hertz::{gauss_legendre, patch_aspect};

/// Where `G`'s table starts, in `u = ln r`. Below it `G` is `(4/π) r ln(4/r)`, whose remainder,
/// `O(r³ ln r)`, is below `1e-35` there.
const TABLE_FROM: f64 = -28.0;

/// Where `G`'s table ends, in `u = ln r`. Above it `G` is its asymptote `ln 4r + 1/2 + 1/(16 r²)`,
/// whose remainder, `O(r⁻⁴)`, is below `1e-24` there.
const TABLE_TO: f64 = 14.0;

/// The table's step in `u = ln r`. Cubic Hermite with the exact slope errs `step⁴ G⁗/384` at a
/// cell's middle: the prototype's step (round seven), equal to [`g_direct`] to `4e-14`
/// (the oracle's `G` basis); the step before it, `0.02`, erred `7.9e-11`.
const TABLE_STEP: f64 = 0.0025;

/// Gauss–Legendre nodes per cell of [`g_direct`]: 16 equal a tanh-sinh reference of the
/// definition to `2e-14` (the oracle's basis; 8 left `7.8e-13`).
const CELL_NODES: usize = 16;

/// The width rule's bound: `n` Gauss–Chebyshev nodes integrate the depth term to `O(E⁻²ⁿ)`, `E`
/// the Bernstein ellipse through its branch points `±i h`; `n` is the least that puts `E⁻²ⁿ`
/// below this.
const WIDTH_TOL: f64 = 1e-14;

/// The most nodes the width rule takes. The rule asks more only above `b/h ≈ 3.9`, where a strip
/// is wider than the depth its tooth model starts at and is no Hertz strip; there it errs as
/// `E^{−2·64}` (`3e-6` at `b/h = 10`) rather than growing without bound.
pub const MAX_WIDTH_NODES: usize = 64;

/// A panel whose ends lie on one side of the point, the nearer at least this many half-widths
/// away, takes the separated form, whose rule then needs at most 35 nodes for
/// [`SEPARATED_TOL`]: `E ≥ 1/2 + √(5/4)` at the nearer end.
pub const SEPARATED_FROM: f64 = 0.5;

/// The separated form's rule's bound, as [`WIDTH_TOL`], its branch points at `x = ±i |η_near|`.
const SEPARATED_TOL: f64 = 1e-14;

/// Below this `s = κ²` the shape factor is 1 in floating point: `C − 1 ≈ s (ln(16/s) − 3)/8`
/// (the expansion of `s R_D(0, 1, s)` about 0) is `7e-24` there, and far below it
/// `R_D(0, 1, s)`'s duplication need not converge in its step bound (`s < 4⁻²⁰⁰`).
const SHAPE_ONE: f64 = 1e-24;

/// One body's elastic half-space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HalfSpace {
    /// `E`, Young's modulus, MPa.
    modulus: f64,
    /// `ν`, Poisson's ratio.
    poisson: f64,
}

impl HalfSpace {
    /// `None` unless `E` is a finite positive modulus and `−1 < ν ≤ 1/2`.
    pub fn new(modulus: f64, poisson: f64) -> Option<Self> {
        (modulus.is_finite() && modulus > 0.0 && poisson > -1.0 && poisson <= 0.5)
            .then_some(Self { modulus, poisson })
    }

    /// `E`, MPa.
    pub fn modulus(self) -> f64 {
        self.modulus
    }

    /// `ν`.
    pub fn poisson(self) -> f64 {
        self.poisson
    }

    /// `(1 − ν²)/(π E)`: the surface term's factor.
    fn surface(self) -> f64 {
        let nu = self.poisson;
        (1.0 - nu * nu) / (PI * self.modulus)
    }

    /// `(1 + ν)/(2π E)`: the depth term's factor.
    fn interior(self) -> f64 {
        (1.0 + self.poisson) / (2.0 * PI * self.modulus)
    }

    /// An infinite uniform line of unit load (N/mm) and half-width `b`, relative to depth `h`:
    /// `2(1 − ν²)/(π E) [asinh(h/b) − ν/(1 − ν) · h/(√(h² + b²) + h)]`, mm²/N. For `b, h > 0`.
    pub fn line_limit(self, half_width: f64, depth: f64) -> f64 {
        let (b, h, nu) = (half_width, depth, self.poisson);
        2.0 * (1.0 - nu * nu) / (PI * self.modulus)
            * ((h / b).asinh() - nu / (1.0 - nu) * h / (h.hypot(b) + h))
    }
}

/// A panel of uniform line load on the contact line, as it acts at one point of the line.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Panel {
    /// `η₁`: where the panel starts, along the line from the point, mm.
    from: f64,
    /// `η₂`: where it ends, mm.
    to: f64,
    /// `b`: the Hertz half-width across the line of the load on it, mm.
    half_width: f64,
    /// `h`: the depth below the point the displacement is taken relative to, mm; `None` is the
    /// bare half-space, relative to nothing.
    depth: Option<f64>,
}

/// Which of the kernel's two exact forms a panel takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// The surface term through `G`, the depth term across the width by the width rule.
    Near,
    /// Both ends on one side of the point, the nearer at least [`SEPARATED_FROM`] half-widths
    /// away, with a depth reference: the cancellation-free integrand.
    Separated,
}

impl Panel {
    /// `None` unless both ends are finite, the half-width is a finite length `> 0`, and a depth,
    /// where there is one, is a finite length `> 0`.
    pub fn new(from: f64, to: f64, half_width: f64, depth: Option<f64>) -> Option<Self> {
        let length = |v: f64| v.is_finite() && v > 0.0;
        (from.is_finite() && to.is_finite() && length(half_width) && depth.is_none_or(length))
            .then_some(Self {
                from,
                to,
                half_width,
                depth,
            })
    }

    /// The form the kernel reads this panel with.
    pub fn route(&self) -> Route {
        let (from, to) = (self.from, self.to);
        let nearer = from.abs().min(to.abs());
        if self.depth.is_some() && from * to > 0.0 && nearer >= SEPARATED_FROM * self.half_width {
            Route::Separated
        } else {
            Route::Near
        }
    }

    /// The width rule's node count for this panel's depth term ([`width_rule`]). `None` for the
    /// bare half-space, which has no depth term.
    pub fn width_nodes(&self) -> Option<usize> {
        self.depth.map(|h| width_rule(self.half_width, h))
    }
}

/// The width rule: the least `n ≥ 2` Gauss–Chebyshev nodes with `E⁻²ⁿ ≤ 1e-14`,
/// `E = h/b + √(1 + (h/b)²)` the Bernstein ellipse through the depth term's branch points
/// `x = ±i h`, at most [`MAX_WIDTH_NODES`].
fn width_rule(half_width: f64, depth: f64) -> usize {
    let r = depth / half_width;
    nodes(r + (1.0 + r * r).sqrt(), WIDTH_TOL)
}

/// The least `n ≥ 2` with `E⁻²ⁿ` below `tol`, `n = ⌈ln(1/tol) / (2 ln E)⌉ + 1`, at most
/// [`MAX_WIDTH_NODES`].
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "an integer in [2, MAX_WIDTH_NODES], clamped before the cast"
)]
fn nodes(ellipse: f64, tol: f64) -> usize {
    let n = ((1.0 / tol).ln() / (2.0 * ellipse.ln())).ceil() + 1.0;
    n.clamp(2.0, MAX_WIDTH_NODES as f64) as usize
}

/// `G`, tabulated once, and the Gauss–Chebyshev rules across the width.
#[derive(Clone, Debug)]
pub struct Kernel {
    /// The table's step in `u = ln r`.
    step: f64,
    /// `(G, dG/du = r G′(r))` at `u = TABLE_FROM + i · step`.
    table: Vec<(f64, f64)>,
    /// Gauss–Chebyshev (second kind) rules for `φ`, by node count: `(t_k, w_k)` with
    /// `t_k = cos(kπ/(n + 1))` and `w_k = (2/(n + 1)) sin²(kπ/(n + 1))`, `k = 1 … n`.
    rules: Vec<Vec<(f64, f64)>>,
}

impl Kernel {
    /// The kernel, built on first use.
    pub fn shared() -> &'static Self {
        static KERNEL: OnceLock<Kernel> = OnceLock::new();
        KERNEL.get_or_init(|| Self::with_step(TABLE_STEP))
    }

    /// The kernel with `G` tabulated at `step` in `u = ln r`.
    #[expect(
        clippy::expect_used,
        reason = "G′ is R_D(0, r², 1 + r²): finite arguments, x + y > 0 and z > 0 for every r \
                  of the table"
    )]
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "the table's node count, a positive integer"
    )]
    pub(crate) fn with_step(step: f64) -> Self {
        let count = ((TABLE_TO - TABLE_FROM) / step).round() as usize + 1;
        let cells = gauss_legendre::<CELL_NODES>();
        let table = (0..count)
            .map(|i| {
                let r = (TABLE_FROM + i as f64 * step).exp();
                let slope = g_slope(r).expect("R_D's domain holds on the table");
                (g_direct_with(r, &cells), r * slope)
            })
            .collect();
        let rules = (0..=MAX_WIDTH_NODES)
            .map(|n| {
                let span = (n + 1) as f64;
                (1..=n)
                    .map(|k| {
                        let a = k as f64 * PI / span;
                        (a.cos(), 2.0 / span * a.sin().powi(2))
                    })
                    .collect()
            })
            .collect();
        Self { step, table, rules }
    }

    /// `G(r)`: the table's cubic Hermite in `u = ln r` with the exact slope, the asymptote above
    /// it and the small-`r` form below it; odd, `G(0) = 0`.
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a cell index in [0, len − 2], clamped before the cast"
    )]
    pub fn g(&self, r: f64) -> f64 {
        let x = r.abs();
        if x == 0.0 {
            return r;
        }
        let u = x.ln();
        let value = if u >= TABLE_TO {
            (4.0 * x).ln() + 0.5 + 1.0 / (16.0 * x * x)
        } else if u > TABLE_FROM {
            let at = (u - TABLE_FROM) / self.step;
            let cell = at.floor().min((self.table.len() - 2) as f64) as usize;
            let t = at - cell as f64;
            let (t2, t3) = (t * t, t * t * t);
            let ((g0, d0), (g1, d1)) = (self.table[cell], self.table[cell + 1]);
            (2.0 * t3 - 3.0 * t2 + 1.0) * g0
                + (t3 - 2.0 * t2 + t) * self.step * d0
                + (-2.0 * t3 + 3.0 * t2) * g1
                + (t3 - t2) * self.step * d1
        } else {
            // Below the table, and NaN, which the comparisons above pass here.
            4.0 / PI * x * (4.0 / x).ln()
        };
        value.copysign(r)
    }

    /// One body's centre-line displacement at the point minus its displacement at the panel's
    /// depth below the point, per unit line load (N/mm) on the panel: mm²/N. The bare half-space
    /// (no depth) is the surface's alone.
    pub fn panel(&self, panel: &Panel, body: HalfSpace) -> f64 {
        let Panel {
            from,
            to,
            half_width: b,
            depth,
        } = *panel;
        match (depth, panel.route()) {
            (Some(h), Route::Separated) => {
                let n = separated_rule(from, to, b);
                body.interior() * separated(from, to, b, h, body.poisson, self.rule(n))
            }
            _ => self.near(panel, body),
        }
    }

    /// The near form: the surface term through `G` less, with a depth, the depth term by the
    /// width rule.
    fn near(&self, panel: &Panel, body: HalfSpace) -> f64 {
        let Panel {
            from,
            to,
            half_width: b,
            depth,
        } = *panel;
        let surface = body.surface() * (self.g(to / b) - self.g(from / b));
        depth.map_or(surface, |h| {
            let rule = self.rule(width_rule(b, h));
            surface - body.interior() * depth_term(from, to, b, h, body.poisson, rule)
        })
    }

    /// The `n`-node rule.
    fn rule(&self, n: usize) -> &[(f64, f64)] {
        &self.rules[n]
    }
}

/// The depth term's integral across the width by `rule`, per unit of `(1 + ν)/(2π E)`.
fn depth_term(from: f64, to: f64, b: f64, h: f64, nu: f64, rule: &[(f64, f64)]) -> f64 {
    rule.iter()
        .map(|&(t, w)| {
            let x = b * t;
            let r2 = x * x + h * h;
            let rho = r2.sqrt();
            let (dist1, dist2) = ((r2 + from * from).sqrt(), (r2 + to * to).sqrt());
            w * (2.0 * (1.0 - nu) * ((to / rho).asinh() - (from / rho).asinh())
                + h * h * (to / (r2 * dist2) - from / (r2 * dist1)))
        })
        .sum()
}

/// The separated form's node count: as the width rule's, with the Bernstein ellipse through the
/// branch points `x = ±i |η_near|`; at most 35 where the form applies ([`SEPARATED_FROM`]).
fn separated_rule(from: f64, to: f64, b: f64) -> usize {
    let r = from.abs().min(to.abs()) / b;
    nodes(r + (1.0 + r * r).sqrt(), SEPARATED_TOL)
}

/// The separated panel, both ends of one sign, without cancellation. Across the strip:
///
/// ```text
/// (1 + ν)/(2π E) ∫ φ(x) {2(1 − ν)[A_s − A_ρ] − h² [η₂/(ρ² R₂) − η₁/(ρ² R₁)]} dx,
/// A_s = asinh(D/d_s),  A_ρ = asinh(D/d_R),  D = η₂² − η₁²,
/// d_s = η₂ s₁ + η₁ s₂,  d_R = η₂ R₁ + η₁ R₂,  s_i² = x² + η_i²  (terms of one sign),
/// ```
///
/// and with `a = D/d_s`, `c = D/d_R` (one sign):
/// `A_s − A_ρ = asinh((a − c)(a + c)/(a √(1 + c²) + c √(1 + a²)))`,
/// `a − c = D (d_R − d_s)/(d_s d_R)`, `d_R − d_s = h² [η₂/(R₁ + s₁) + η₁/(R₂ + s₂)]`, and
/// `h² [η₂/(ρ² R₂) − η₁/(ρ² R₁)] = h² D/(d_R R₁ R₂)`. What is left is the physical `−ν`.
/// The integrand is analytic but for branch points at `x = ±i |η_near|`, which set the rule.
/// Per unit of `(1 + ν)/(2π E)`.
fn separated(from: f64, to: f64, b: f64, h: f64, nu: f64, rule: &[(f64, f64)]) -> f64 {
    let d = (to - from) * (to + from);
    let (h2, e1s, e2s) = (h * h, from * from, to * to);
    rule.iter()
        .map(|&(t, w)| {
            let x2 = (b * t).powi(2);
            let (s1, s2) = ((x2 + e1s).sqrt(), (x2 + e2s).sqrt());
            let (r1, r2) = ((x2 + h2 + e1s).sqrt(), (x2 + h2 + e2s).sqrt());
            let (ds, dr) = (to * s1 + from * s2, to * r1 + from * r2);
            let (a, c) = (d / ds, d / dr);
            let a_minus_c = d * h2 * (to / (r1 + s1) + from / (r2 + s2)) / (ds * dr);
            let apart = (a_minus_c * (a + c)
                / (a * (1.0 + c * c).sqrt() + c * (1.0 + a * a).sqrt()))
            .asinh();
            w * (2.0 * (1.0 - nu) * apart - h2 * d / (dr * r1 * r2))
        })
        .sum()
}

/// `G(r)` by quadrature of its definition: the `ln sin θ` singularity at `θ = 0` taken out in
/// closed form (`−∫₀^{π/2} cos²θ ln sin θ dθ = (π/4) ln 2 + π/8`), the rest,
/// `cos²θ ln(r + √(r² + sin²θ))`, over cells doubling from `min(r, π/4)` to `π/2`, 16-point
/// Gauss–Legendre on each. Exact to rounding in absolute terms (a few `1e-16` at `r → 0`, where
/// `G` itself vanishes). Odd, `G(0) = 0`.
pub fn g_direct(r: f64) -> f64 {
    g_direct_with(r, &gauss_legendre::<CELL_NODES>())
}

fn g_direct_with(r: f64, cells: &[(f64, f64)]) -> f64 {
    let x = r.abs();
    if x == 0.0 {
        return r;
    }
    let f = |th: f64| {
        let s = th.sin();
        th.cos().powi(2) * (x + (x * x + s * s).sqrt()).ln()
    };
    // Cells double from `min(r, π/4)`: `⌈log₂(π/(2 min(r, π/4)))⌉` of them before `π/2`.
    let mut edges = vec![0.0];
    let mut a = x.min(FRAC_PI_4);
    while a < FRAC_PI_2 {
        edges.push(a);
        a *= 2.0;
    }
    edges.push(FRAC_PI_2);
    let total: f64 = edges
        .windows(2)
        .map(|e| {
            let (half, mid) = (0.5 * (e[1] - e[0]), 0.5 * (e[1] + e[0]));
            half * cells
                .iter()
                .map(|&(t, w)| w * f(mid + half * t))
                .sum::<f64>()
        })
        .sum();
    (4.0 / PI * (total + PI / 4.0 * LN_2 + PI / 8.0)).copysign(r)
}

/// `G′(r) = (4/(3π)) (1 + r²) R_D(0, r², 1 + r²)`, even. `None` at `r = 0`, where it grows as
/// `(4/π) ln(4/|r|)` without bound, and where `r²` is not finite.
pub fn g_slope(r: f64) -> Option<f64> {
    let r2 = r * r;
    r_d(0.0, r2, 1.0 + r2).map(|d| 4.0 / (3.0 * PI) * (1.0 + r2) * d)
}

/// `C = p₀/p_line`, the peak of the Hertz ellipse of curvature ratio `q = k_L/k_A` over the line
/// contact's at the same line load: `C² = 3/(s R_D(0, 1, s))`, `s = κ²` the ellipse's aspect
/// squared ([`patch_aspect`]). `C(0) = 1`, the line; `C(1) = 2/√π`, the circle. `None` for `q`
/// outside `[0, 1]`: which way a contact whose line is its sharper direction is read is its
/// caller's to say.
pub fn shape_factor(q: f64) -> Option<f64> {
    if !(0.0..=1.0).contains(&q) {
        return None;
    }
    let kappa = patch_aspect(q, 1.0)?;
    let s = kappa * kappa;
    if s < SHAPE_ONE {
        return Some(1.0);
    }
    r_d(0.0, 1.0, s).map(|d| (3.0 / (s * d)).sqrt())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::elliptic::r_f;
    use crate::field::oracle::{self, miss, num, Worst};
    use crate::field::wide::Wide;
    use serde_json::Value;

    const EPS: f64 = f64::EPSILON;

    fn oracle_file() -> Value {
        serde_json::from_str(include_str!("../../tests/data/field_oracle/kernel.json"))
            .expect("kernel.json parses")
    }

    fn steel() -> HalfSpace {
        HalfSpace::new(206_000.0, 0.3).expect("a half-space")
    }

    // ---------------------------------------------------------------- the oracle

    /// The README's rules for this step's records, written out: each record states its own, and
    /// [`reproduce`] asserts it is this one.
    const G_ABS: f64 = 1e-12;
    const CLOSED_REL: f64 = 1e-13;
    const PANEL_OF_SCALE: f64 = 1e-12;
    const SHAPE_REL: f64 = 1e-12;

    /// How one output is held: absolutely, relative to its size, or relative to its record's
    /// `scale` (a panel's larger term).
    #[derive(Clone, Copy, Debug)]
    enum Rule {
        Abs(f64),
        Rel(f64),
        OfScale(f64, f64),
    }

    fn bound(rule: Rule, oracle: f64) -> f64 {
        match rule {
            Rule::Abs(a) => a,
            Rule::Rel(r) => r * oracle.abs(),
            Rule::OfScale(r, scale) => r * scale,
        }
    }

    /// A value moved ten times its rule, written out from the constants above rather than read
    /// from [`bound`].
    fn ten_times(x: f64, rule: Rule) -> f64 {
        match rule {
            Rule::Abs(_) => x + 10.0 * G_ABS,
            Rule::Rel(r) if r == CLOSED_REL => x + 10.0 * CLOSED_REL * x.abs(),
            Rule::Rel(_) => x + 10.0 * SHAPE_REL * x.abs(),
            Rule::OfScale(_, scale) => x + 10.0 * PANEL_OF_SCALE * scale,
        }
    }

    /// [`bound`], to the bit, at literal values: what the 10× law cannot pin, a bound up to ten
    /// times too loose. Plants: every bound loosened 100× (none of the pins hold), and the
    /// relative rule floored at 1 absolute as the form step's is where a value passes zero (the
    /// pins below 1 in size fail).
    #[test]
    fn the_bound_is_the_readmes_rule() {
        let pins = |bound: fn(Rule, f64) -> f64| {
            [
                (Rule::Abs(G_ABS), 5.0, 1e-12),
                (Rule::Abs(G_ABS), 0.0, 1e-12),
                (Rule::Rel(CLOSED_REL), -20.0, 2e-12),
                (Rule::Rel(SHAPE_REL), 0.25, 2.5e-13),
                (Rule::OfScale(PANEL_OF_SCALE, 3e-7), -1e-9, 3e-19),
            ]
            .into_iter()
            .filter(|&(rule, x, b)| bound(rule, x) == b)
            .count()
        };
        assert_eq!(pins(bound), 5);
        assert_eq!(pins(|r, x| 100.0 * bound(r, x)), 0);
        assert_eq!(
            pins(|r, x| match r {
                Rule::Rel(rel) => rel * x.abs().max(1.0),
                _ => bound(r, x),
            }),
            4
        );
    }

    /// One oracle panel case: `(η₁, η₂, b, h, ν, E)`, `h = "inf"` the bare half-space.
    fn case_of(c: &Value) -> (Panel, HalfSpace) {
        let v = c.as_array().expect("a case");
        let depth = v[3].as_f64();
        assert!(depth.is_some() || v[3].as_str() == Some("inf"), "{c}");
        let panel = Panel::new(num(&v[0]), num(&v[1]), num(&v[2]), depth).expect("a panel");
        let body = HalfSpace::new(num(&v[5]), num(&v[4])).expect("a half-space");
        (panel, body)
    }

    fn route_name(r: Route) -> &'static str {
        match r {
            Route::Near => "G",
            Route::Separated => "sep",
        }
    }

    /// What [`reproduce`] saw: the values' worst miss, and the routes and node counts compared
    /// exactly, with how many of each differed.
    #[derive(Default)]
    struct Seen {
        worst: Worst,
        exact: usize,
        differ: usize,
        /// Panel values that missed, by route.
        panel_misses: (usize, usize),
    }

    /// Every record of `kernel.json` within its own rule: `G` (the table), `G_direct` and `G′` at
    /// 25 radii; Carlson's `R_F` and `R_D` (`elliptic.rs`'s); the line limit; each panel's value
    /// against `1e-12` of its `scale`, its route and its width rule's node count exactly; the
    /// shape factor and the aspect. `kernel` and `panel` are the port's, or a plant's.
    fn reproduce(
        kernel: &Kernel,
        panel: impl Fn(&Kernel, &Panel, HalfSpace) -> f64,
        perturb: impl Fn(f64, Rule) -> f64,
        bound: impl Fn(Rule, f64) -> f64,
    ) -> Seen {
        let file = oracle_file();
        let records = oracle::records(&file);
        assert_eq!(records.len(), 5);
        let mut seen = Seen::default();
        let list =
            |v: &Value| -> Vec<f64> { v.as_array().expect("a list").iter().map(num).collect() };
        for rec in records {
            let (i, o, tol) = (rec.inputs, rec.outputs, rec.tol);
            let mut worst = std::mem::take(&mut seen.worst);
            // Whether the value missed.
            let mut check = |name: String, port: f64, oracle: f64, rule: Rule| {
                let ratio = miss(perturb(port, rule), oracle, bound(rule, oracle));
                worst.see(ratio, || {
                    format!("{} {name}: {port:e} vs {oracle:e}", rec.id)
                });
                ratio > 1.0 || ratio.is_nan()
            };
            match rec.id {
                "kernel/G" => {
                    assert_eq!(num(&tol["G"]["abs"]), G_ABS);
                    assert_eq!(num(&tol["G_direct"]["abs"]), G_ABS);
                    assert_eq!(num(&tol["dG"]["rel"]), CLOSED_REL);
                    let rs = list(&i["r"]);
                    assert_eq!(rs.len(), 25);
                    for (k, &r) in rs.iter().enumerate() {
                        let slope = g_slope(r).expect("G′ at r > 0");
                        check(
                            format!("G({r})"),
                            kernel.g(r),
                            num(&o["G"][k]),
                            Rule::Abs(G_ABS),
                        );
                        check(
                            format!("G_direct({r})"),
                            g_direct(r),
                            num(&o["G_direct"][k]),
                            Rule::Abs(G_ABS),
                        );
                        check(
                            format!("G'({r})"),
                            slope,
                            num(&o["dG"][k]),
                            Rule::Rel(CLOSED_REL),
                        );
                    }
                }
                "kernel/carlson" => {
                    assert_eq!(num(&tol["rel"]), CLOSED_REL);
                    let args = i["args"].as_array().expect("args");
                    assert_eq!(args.len(), 4);
                    for (k, a) in args.iter().enumerate() {
                        let [x, y, z] = [num(&a[0]), num(&a[1]), num(&a[2])];
                        let (rf, rd) = (r_f(x, y, z).expect("R_F"), r_d(x, y, z).expect("R_D"));
                        check(
                            format!("R_F{a}"),
                            rf,
                            num(&o["rf"][k]),
                            Rule::Rel(CLOSED_REL),
                        );
                        check(
                            format!("R_D{a}"),
                            rd,
                            num(&o["rd"][k]),
                            Rule::Rel(CLOSED_REL),
                        );
                    }
                }
                "kernel/line_limit" => {
                    assert_eq!(num(&tol["rel"]), CLOSED_REL);
                    let cases = i["cases"].as_array().expect("cases");
                    assert_eq!(cases.len(), 3);
                    for (k, c) in cases.iter().enumerate() {
                        let body = HalfSpace::new(num(&c[3]), num(&c[2])).expect("a half-space");
                        let port = body.line_limit(num(&c[0]), num(&c[1]));
                        check(
                            format!("line{c}"),
                            port,
                            num(&o["v"][k]),
                            Rule::Rel(CLOSED_REL),
                        );
                    }
                }
                "kernel/panel" => {
                    assert_eq!(num(&tol["panel"]["abs_of_scale"]), PANEL_OF_SCALE);
                    let cases = i["cases"].as_array().expect("cases");
                    assert_eq!(cases.len(), 35);
                    for (k, c) in cases.iter().enumerate() {
                        let (p, body) = case_of(c);
                        let scale = num(&o["scale"][k]);
                        let rule = Rule::OfScale(PANEL_OF_SCALE, scale);
                        let (port, oracle) = (panel(kernel, &p, body), num(&o["panel"][k]));
                        if check(format!("panel{c}"), port, oracle, rule) {
                            match p.route() {
                                Route::Near => seen.panel_misses.0 += 1,
                                Route::Separated => seen.panel_misses.1 += 1,
                            }
                        }
                        let route = o["route"][k].as_str().expect("a route");
                        let width = o["n_width"][k]
                            .as_u64()
                            .map(|n| usize::try_from(n).expect("fits"));
                        seen.exact += 2;
                        seen.differ += usize::from(route_name(p.route()) != route)
                            + usize::from(p.width_nodes() != width);
                    }
                }
                "kernel/shape_C" => {
                    assert_eq!(num(&tol["C"]["rel"]), SHAPE_REL);
                    assert_eq!(num(&tol["aspect"]["rel"]), SHAPE_REL);
                    let qs = list(&i["q"]);
                    assert_eq!(qs.len(), 12);
                    let aspects = list(&o["aspect"]);
                    for (k, &q) in qs.iter().enumerate() {
                        let c = shape_factor(q).expect("q in [0, 1]");
                        check(format!("C({q})"), c, num(&o["C"][k]), Rule::Rel(SHAPE_REL));
                    }
                    // The aspect is recorded for q > 0 only.
                    let positive: Vec<f64> = qs.into_iter().filter(|&q| q > 0.0).collect();
                    assert_eq!(positive.len(), aspects.len());
                    for (&q, &a) in positive.iter().zip(&aspects) {
                        let port = patch_aspect(q, 1.0).expect("an aspect");
                        check(format!("aspect({q})"), port, a, Rule::Rel(SHAPE_REL));
                    }
                }
                other => panic!("a record this step does not know: {other}"),
            }
            seen.worst = worst;
        }
        seen
    }

    /// Every value [`reproduce`] compares: 3 × 25 of `G`, 2 × 4 Carlson, 3 line limits, 35
    /// panels, 12 shape factors and 11 aspects; and 2 × 35 exact (routes and node counts).
    const VALUES: usize = 75 + 8 + 3 + 35 + 12 + 11;
    const EXACT: usize = 70;

    fn port(kernel: &Kernel, p: &Panel, body: HalfSpace) -> f64 {
        kernel.panel(p, body)
    }

    #[test]
    fn the_oracle_records_reproduce() {
        let seen = reproduce(Kernel::shared(), port, |x, _| x, bound);
        eprintln!(
            "kernel.json: 5/5 records, {} values, worst {:.3} of the tolerance ({}); {} routes \
             and node counts equal",
            seen.worst.count, seen.worst.ratio, seen.worst.at, seen.exact
        );
        assert_eq!((seen.worst.count, seen.exact), (VALUES, EXACT));
        assert_eq!((seen.worst.missed, seen.differ), (0, 0));
        assert!(
            seen.worst.ratio <= 1.0,
            "worst {} at {}",
            seen.worst.ratio,
            seen.worst.at
        );
    }

    /// The tolerance's two laws: every output a few roundings off still reproduces, and ten times
    /// its rule off ([`ten_times`], written apart from [`bound`]) misses at every value. Its
    /// plant: every bound loosened 100×, under which the fault reproduces.
    #[test]
    fn the_oracle_tolerance_passes_rounding_and_fails_ten_times_itself() {
        let kernel = Kernel::shared();
        let rounded = reproduce(kernel, port, |x, _| x * (1.0 + 4.0 * EPS), bound);
        assert_eq!((rounded.worst.count, rounded.worst.missed), (VALUES, 0));
        let tenfold = reproduce(kernel, port, ten_times, bound);
        assert_eq!(
            (tenfold.worst.count, tenfold.worst.missed),
            (VALUES, VALUES)
        );
        let loose = reproduce(kernel, port, ten_times, |r, x| 100.0 * bound(r, x));
        assert_eq!(loose.worst.missed, 0, "the 100× bound passes the fault");
    }

    /// The plants a looser port passes. Version 1's port, every panel by the near form (surface
    /// minus depth through `G`): the separated records it misses are the ones whose terms cancel
    /// most, and no near record. Round six's `G` table (step 0.02, `7.9e-11` off): `G` misses. The
    /// round-six width rule (`1e-10`): node counts differ.
    #[test]
    fn the_oracle_fails_its_plants() {
        let kernel = Kernel::shared();
        let v1 = reproduce(kernel, Kernel::near, |x, _| x, bound);
        eprintln!(
            "the near form everywhere: {:?} panels missed (near, separated), worst {:.3e} ({})",
            v1.panel_misses, v1.worst.ratio, v1.worst.at
        );
        assert_eq!(v1.panel_misses.0, 0);
        assert!(v1.panel_misses.1 > 0);

        let coarse = Kernel::with_step(0.02);
        let six = reproduce(&coarse, port, |x, _| x, bound);
        eprintln!(
            "step 0.02: {} values missed, worst {:.3e} ({})",
            six.worst.missed, six.worst.ratio, six.worst.at
        );
        assert!(six.worst.missed > 0);

        let file = oracle_file();
        let rec = oracle::records(&file)
            .into_iter()
            .find(|r| r.id == "kernel/panel")
            .expect("panel");
        let six_rule = rec.inputs["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .zip(rec.outputs["n_width"].as_array().expect("n_width"))
            .filter(|(c, n)| {
                let (p, _) = case_of(c);
                p.depth.is_some_and(|h| {
                    let r = h / p.half_width;
                    Some(nodes(r + (1.0 + r * r).sqrt(), 1e-10))
                        != n.as_u64().and_then(|n| usize::try_from(n).ok())
                })
            })
            .count();
        eprintln!("the round-six width rule: {six_rule} node counts differ");
        assert!(six_rule > 0);
    }

    // ---------------------------------------------------------------- the 50-digit reference

    /// The panel's definition, surface minus depth with `η` integrated in closed form, over the
    /// strip's width by tanh-sinh at step `hs` in double-double (the prototype's
    /// `oracle-fix/check_panel_dec.py`, in 50 digits there): shares no code with the kernel.
    /// Returns the two terms, `(surface, depth)`; the panel is their difference.
    fn reference_terms(p: &Panel, body: HalfSpace, hs: f64) -> (Wide, Wide) {
        let w = Wide::of;
        let (e1, e2, b) = (w(p.from), w(p.to), w(p.half_width));
        let (nu, e) = (w(body.poisson), w(body.modulus));
        let pi = Wide::pi();
        let one = w(1.0);
        let cs = (one - nu * nu) / (pi * e);
        let cd = (one + nu) / (w(2.0) * pi * e);
        let surface = |x: Wide| cs * ((e2 / x).asinh() - (e1 / x).asinh());
        let depth = |x: Wide| {
            p.depth.map_or(w(0.0), |h| {
                let h = w(h);
                let r2 = x * x + h * h;
                let rho = r2.sqrt();
                let (d1, d2) = ((r2 + e1 * e1).sqrt(), (r2 + e2 * e2).sqrt());
                cd * (w(2.0) * (one - nu) * ((e2 / rho).asinh() - (e1 / rho).asinh())
                    + h * h * (e2 / (r2 * d2) - e1 / (r2 * d1)))
            })
        };
        // x on [0, b] (the integrand is even in x): x = b/(1 + e^{−2u}), u = (π/2) sinh t,
        // dx = (b/2)(π/2) cosh t / cosh² u dt; |t| ≤ 4.2, where the weight is below 1e-43.
        #[expect(clippy::cast_possible_truncation, reason = "a node count")]
        let reach = (4.2 / hs) as i32;
        let (mut s, mut d) = (w(0.0), w(0.0));
        for k in -reach..=reach {
            let t = w(f64::from(k)) * w(hs);
            let u = pi * w(0.5) * t.sinh();
            let e2u = (w(2.0) * u).exp();
            let from_zero = b / (one + one / e2u);
            let from_b = b / (e2u + one);
            if from_zero.hi == 0.0 || from_b.hi == 0.0 {
                continue;
            }
            let cu = u.cosh();
            let weight = pi * w(0.5) * t.cosh() / (cu * cu);
            let phi = w(2.0) / (pi * b) * (from_b * (w(2.0) * b - from_b)).sqrt() / b;
            s = s + weight * phi * surface(from_zero);
            d = d + weight * phi * depth(from_zero);
        }
        let factor = b * w(hs);
        (s * factor, d * factor)
    }

    fn reference(p: &Panel, body: HalfSpace, hs: f64) -> Wide {
        let (s, d) = reference_terms(p, body, hs);
        s - d
    }

    /// Every oracle panel against the double-double quadrature of its definition, to `1e-13` of
    /// its scale (the porting plan's gate; the records themselves are within `3.3e-14` of the
    /// prototype's 50-digit one). The reference at steps 1/32 and 1/64 agrees with itself far
    /// below that. Its plants: version 1's port (the near form everywhere) and round six's `G`
    /// table, each missing some panel; the round-six width rule on the near panels.
    #[test]
    fn every_panel_is_its_definitions_quadrature() {
        const GATE: f64 = 1e-13;
        let file = oracle_file();
        let rec = oracle::records(&file)
            .into_iter()
            .find(|r| r.id == "kernel/panel")
            .expect("panel");
        let cases = rec.inputs["cases"].as_array().expect("cases");
        let scales = rec.outputs["scale"].as_array().expect("scales");
        let kernel = Kernel::shared();
        let coarse = Kernel::with_step(0.02);
        let (mut worst, mut self_worst, mut compared) = (0.0_f64, 0.0_f64, 0);
        let (mut v1_missed, mut six_missed, mut rule_missed) = (0, 0, 0);
        for (c, s) in cases.iter().zip(scales) {
            let (p, body) = case_of(c);
            let scale = num(s);
            let fine = reference(&p, body, 1.0 / 64.0);
            let half = reference(&p, body, 1.0 / 32.0);
            self_worst = self_worst.max((fine - half).abs().to_f64() / scale);
            let off = |v: f64| (Wide::of(v) - fine).abs().to_f64() / scale;
            worst = worst.max(off(kernel.panel(&p, body)));
            compared += 1;
            v1_missed += usize::from(off(kernel.near(&p, body)) > GATE);
            six_missed += usize::from(off(coarse.panel(&p, body)) > GATE);
            if let (Some(h), Route::Near) = (p.depth, p.route()) {
                let r = h / p.half_width;
                let n = nodes(r + (1.0 + r * r).sqrt(), 1e-10);
                let surface = body.surface()
                    * (kernel.g(p.to / p.half_width) - kernel.g(p.from / p.half_width));
                let v = surface
                    - body.interior()
                        * depth_term(p.from, p.to, p.half_width, h, body.poisson, kernel.rule(n));
                rule_missed += usize::from(off(v) > GATE);
            }
        }
        eprintln!(
            "{compared} panels against the double-double reference: worst {worst:.2e} of scale \
             (the reference's own step change {self_worst:.1e}); plants missing: v1 {v1_missed}, \
             step 0.02 {six_missed}, width 1e-10 {rule_missed}"
        );
        assert_eq!(compared, 35);
        assert!(
            self_worst < 1e-20,
            "the reference has not converged: {self_worst}"
        );
        assert!(worst <= GATE, "worst {worst}");
        assert!(v1_missed > 0 && six_missed > 0 && rule_missed > 0);
    }

    // ---------------------------------------------------------------- the line limit

    /// The cases the line-limit and width-rule laws run over: `(b, h)` from a strip far narrower
    /// than its depth to one three times wider, on three bodies (the oracle's).
    fn strips() -> Vec<(f64, f64, HalfSpace)> {
        let bodies = [(206_000.0, 0.3), (190_000.0, 0.29), (96_500.0, 0.32)];
        let mut out = Vec::new();
        for (b, h) in [(0.05, 1.0), (0.2, 0.8), (0.5, 0.5), (1.0, 0.3), (3.0, 1.0)] {
            for (e, nu) in bodies {
                out.push((b, h, HalfSpace::new(e, nu).expect("a half-space")));
            }
        }
        out
    }

    /// `panel(−L, L)` against the line limit and its leading tail, `(residual, bound)`. Far along
    /// the line the depth-referenced kernel is `−(1 + ν) ν h²/(2π E η³)`, so the two tails beyond
    /// `±L` are `−(1 + ν) ν h²/(2π E L²)` and `panel(−L, L) = line + (1 + ν) ν h²/(2π E L²)`, to
    /// a remainder of relative order `(h² + b²)/L²` in the tail: at `L = 10⁴ max(b, h)` below
    /// `1e-8` of a tail below `1e-8` of the line, under rounding. The bound is the rounding of the
    /// two terms the near form subtracts: `(n + 16) ε (|surface| + |depth|)`, `n` the width rule's
    /// nodes (each node's sum adds one rounding, the rest a fixed few per term).
    fn line_tail(b: f64, h: f64, body: HalfSpace) -> f64 {
        let (l, nu) = (1e4 * b.max(h), body.poisson);
        (1.0 + nu) * nu * h * h / (2.0 * PI * body.modulus * l * l)
    }

    fn line_residual(
        kernel: &Kernel,
        (b, h, body): (f64, f64, HalfSpace),
        rule: usize,
        tail: bool,
    ) -> (f64, f64) {
        let l = 1e4 * b.max(h);
        let surface = body.surface() * (kernel.g(l / b) - kernel.g(-l / b));
        let depth = body.interior() * depth_term(-l, l, b, h, body.poisson, kernel.rule(rule));
        let tail = if tail { line_tail(b, h, body) } else { 0.0 };
        let residual = surface - depth - body.line_limit(b, h) - tail;
        let bound = (rule as f64 + 16.0) * EPS * (surface.abs() + depth.abs());
        (residual.abs(), bound)
    }

    /// **The line limit**: the kernel summed over an infinite uniform line is the plane's
    /// depth-referenced line compliance, with its tail in closed form, to rounding. The sum is
    /// the kernel's own panel (`panel(−L, L)`, which takes the near form).
    ///
    /// Plants, each missing: the tail left out, on every strip (the prototype's check read
    /// `3.1e-12` at `L = 10⁵ h` as agreement: a `1e-11` gate passes it); round two's 4-point width
    /// rule, on every strip; round six's (`1e-10`), on those where it takes fewer nodes than the
    /// rule and the depth term is not negligible.
    #[test]
    fn the_kernel_sums_to_the_line_limit() {
        let kernel = Kernel::shared();
        let (mut worst, mut ran) = (0.0_f64, 0);
        let mut missed = [0; 3];
        for s in strips() {
            let (b, h, body) = s;
            let l = 1e4 * b.max(h);
            let p = Panel::new(-l, l, b, Some(h)).expect("a panel");
            assert_eq!(p.route(), Route::Near);
            let n = width_rule(b, h);
            let (res, bound) = line_residual(kernel, s, n, true);
            assert_eq!(
                (kernel.panel(&p, body) - body.line_limit(b, h) - line_tail(b, h, body)).abs(),
                res,
                "the residual is the kernel's own panel's"
            );
            assert!(res <= bound, "{s:?}: {res:e} > {bound:e}");
            worst = worst.max(res / bound);
            ran += 1;
            let r = h / b;
            let six = nodes(r + (1.0 + r * r).sqrt(), 1e-10);
            for (k, (rule, tail)) in [(n, false), (4, true), (six, true)].into_iter().enumerate() {
                let (res, bound) = line_residual(kernel, s, rule, tail);
                missed[k] += usize::from(res > bound);
            }
        }
        eprintln!(
            "line limit: {ran} strips, worst {worst:.3} of the bound; plants missing \
             (no tail, 4-point, 1e-10): {missed:?}"
        );
        assert_eq!(ran, 15);
        assert_eq!(missed[..2], [15, 15]);
        assert!(missed[2] > 0, "{missed:?}");
    }

    // ---------------------------------------------------------------- the width rule

    /// The width rule's cases: `b/h` from `1e-3` to just inside the cap, and at each a panel on
    /// the point, one beside it, one far, one starting at the point, one ending just short of
    /// it, and a line `10⁵ h` long each way.
    fn width_cases() -> Vec<Panel> {
        let h = 1.0;
        let mut out = Vec::new();
        for bh in [1e-3, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 1.5, 2.0, 3.0, 3.86] {
            for (from, to) in [
                (-0.2, 0.2),
                (0.2, 0.6),
                (3.0, 3.4),
                (0.0, 0.01),
                (-3.0, -1e-3),
                (-1e5, 1e5),
            ] {
                out.push(Panel::new(from * h, to * h, bh * h, Some(h)).expect("a panel"));
            }
        }
        out
    }

    /// The depth term by `rule` against the double-double reference: `(error, bound)`. The bound
    /// is the rule's own, `1e-14` of the value, and the rounding of an `n`-node sum of terms
    /// each a dozen operations deep: `(n + 12) ε Σ w |f|`.
    fn width_error(p: &Panel, rule: &[(f64, f64)], reference: Wide) -> (f64, f64) {
        let body = steel();
        let h = p.depth.expect("a depth");
        let term = |r: &[(f64, f64)]| {
            body.interior() * depth_term(p.from, p.to, p.half_width, h, body.poisson, r)
        };
        let magnitude: f64 = rule
            .iter()
            .map(|node| term(std::slice::from_ref(node)).abs())
            .sum();
        let error = (Wide::of(term(rule)) - reference).abs().to_f64();
        let bound =
            WIDTH_TOL * reference.abs().to_f64() + (rule.len() as f64 + 12.0) * EPS * magnitude;
        (error, bound)
    }

    /// The width rule's count before the cap: `⌈ln(1/tol)/(2 ln E)⌉ + 1`.
    fn uncapped(half_width: f64, depth: f64) -> f64 {
        let r = depth / half_width;
        ((1.0 / WIDTH_TOL).ln() / (2.0 * (r + (1.0 + r * r).sqrt()).ln())).ceil() + 1.0
    }

    /// **The width rule to 1e-14**: the depth term across the strip by the rule's nodes is its
    /// double-double quadrature to `1e-14` of itself and rounding, on every strip up to the cap
    /// (`b/h ≤ 3.86`), panels at, beside, far from and ending at the point. The cap binds only
    /// where the rule asks more than [`MAX_WIDTH_NODES`]: `ln E < ln(10¹⁴)/126`, `b/h > 3.865`.
    ///
    /// Plants: round six's rule (`1e-10`) misses, and so does the rule two nodes short. One node
    /// short passes every case: the rule's `+ 1` is a margin for the error's constant, which
    /// these integrands do not use.
    #[test]
    fn the_width_rule_holds_its_bound() {
        let kernel = Kernel::shared();
        let (mut worst, mut ran) = (0.0_f64, 0);
        let (mut six_missed, mut short_missed, mut margin_missed) = (0, 0, 0);
        for p in width_cases() {
            let reference = reference_terms(&p, steel(), 1.0 / 64.0).1;
            let n = p.width_nodes().expect("a depth");
            assert_eq!(n as f64, uncapped(p.half_width, 1.0).max(2.0), "{p:?}");
            let (error, bound) = width_error(&p, kernel.rule(n), reference);
            assert!(error <= bound, "{p:?}: {error:e} > {bound:e}");
            worst = worst.max(error / bound);
            ran += 1;
            let r = p.depth.expect("a depth") / p.half_width;
            let six = nodes(r + (1.0 + r * r).sqrt(), 1e-10);
            let (e, b) = width_error(&p, kernel.rule(six), reference);
            six_missed += usize::from(e > b);
            let (e, b) = width_error(&p, kernel.rule(n - 2), reference);
            short_missed += usize::from(e > b);
            let (e, b) = width_error(&p, kernel.rule(n - 1), reference);
            margin_missed += usize::from(e > b);
        }
        eprintln!(
            "width rule: {ran} panels, worst {worst:.3} of the bound; plants missing: 1e-10 \
             {six_missed}, two nodes short {short_missed}, one short {margin_missed}"
        );
        assert_eq!(ran, 66);
        assert!(six_missed > 0 && short_missed > 0);
        assert_eq!(margin_missed, 0);
        assert_eq!((uncapped(3.86, 1.0), uncapped(3.87, 1.0)), (64.0, 65.0));
        assert_eq!(width_rule(3.87, 1.0), MAX_WIDTH_NODES);
        assert_eq!(width_rule(1e3, 1.0), MAX_WIDTH_NODES);
    }

    // ---------------------------------------------------------------- the two forms

    /// **The switch between the forms is continuous**: at the separated form's threshold both
    /// forms are exact, and they agree to the panel tolerance, `1e-12` of the larger of the near
    /// form's two terms, on either side of the point, over depths from a third of the width to
    /// thirty widths and panels from a hundredth of a width to fifty. The threshold is where the
    /// route flips: one ulp nearer takes the near form. The separated rule never takes more than
    /// 35 nodes, which the threshold gives.
    ///
    /// Plant: the separated form with round six's width bound (`1e-10`), a few nodes short:
    /// misses at the switch.
    #[test]
    fn the_forms_agree_at_the_switch() {
        let kernel = Kernel::shared();
        let body = steel();
        let b = 0.05;
        let start = SEPARATED_FROM * b;
        let (mut worst, mut ran, mut missed) = (0.0_f64, 0, 0);
        for h in [0.3 * b, b, 3.0 * b, 30.0 * b] {
            for length in [0.01 * b, 0.3 * b, 2.0 * b, 50.0 * b] {
                for (from, to) in [(start, start + length), (-start - length, -start)] {
                    let p = Panel::new(from, to, b, Some(h)).expect("a panel");
                    assert_eq!(p.route(), Route::Separated);
                    let nearer = if from > 0.0 {
                        Panel {
                            from: from.next_down(),
                            ..p
                        }
                    } else {
                        Panel {
                            to: to.next_up(),
                            ..p
                        }
                    };
                    assert_eq!(nearer.route(), Route::Near);
                    let surface = body.surface() * (kernel.g(to / b) - kernel.g(from / b));
                    let depth = body.interior()
                        * depth_term(from, to, b, h, body.poisson, kernel.rule(width_rule(b, h)));
                    let bound = PANEL_OF_SCALE * surface.abs().max(depth.abs());
                    let near = kernel.near(&p, body);
                    let n = separated_rule(from, to, b);
                    assert!(n <= 35);
                    let sep =
                        body.interior() * separated(from, to, b, h, body.poisson, kernel.rule(n));
                    assert_eq!(sep, kernel.panel(&p, body));
                    assert!((sep - near).abs() <= bound, "{p:?}: {sep:e} vs {near:e}");
                    worst = worst.max((sep - near).abs() / bound);
                    ran += 1;
                    let r = start / b;
                    let six = nodes(r + (1.0 + r * r).sqrt(), 1e-10);
                    let planted =
                        body.interior() * separated(from, to, b, h, body.poisson, kernel.rule(six));
                    missed += usize::from((planted - near).abs() > bound);
                }
            }
        }
        eprintln!("the switch: {ran} panels, worst {worst:.3} of the bound; the 1e-10 separated rule misses {missed}");
        assert_eq!(ran, 32);
        assert!(missed > 0);
        let r = SEPARATED_FROM;
        assert_eq!(nodes(r + (1.0 + r * r).sqrt(), SEPARATED_TOL), 35);
    }

    // ---------------------------------------------------------------- the Hertz ellipse

    /// A dense solve of `m x = r` for each right-hand side, by elimination with partial pivoting.
    fn solve(mut m: Vec<Vec<f64>>, mut rhs: Vec<Vec<f64>>) -> Vec<Vec<f64>> {
        let n = m.len();
        for c in 0..n {
            let p = (c..n)
                .max_by(|&i, &j| m[i][c].abs().total_cmp(&m[j][c].abs()))
                .expect("a row");
            m.swap(c, p);
            for r in &mut rhs {
                r.swap(c, p);
            }
            for r in c + 1..n {
                let (above, below) = m.split_at_mut(r);
                let (pivot, row) = (&above[c], &mut below[0]);
                let f = row[c] / pivot[c];
                for (x, p) in row[c..].iter_mut().zip(&pivot[c..]) {
                    *x -= f * p;
                }
                for x in &mut rhs {
                    x[r] -= f * x[c];
                }
            }
        }
        for x in &mut rhs {
            for r in (0..n).rev() {
                let tail: f64 = (r + 1..n).map(|k| m[r][k] * x[k]).sum();
                x[r] = (x[r] - tail) / m[r][r];
            }
        }
        rhs
    }

    /// What the ellipse gate varies: the shape factor it widens each strip by, and where on its
    /// panel each station sits (0 its middle, −½ its start).
    #[derive(Clone, Copy)]
    struct Line {
        shape: fn(f64) -> f64,
        station: f64,
    }

    const THE_LINE: Line = Line {
        shape: |q| shape_factor(q).expect("q in [0, 1]"),
        station: 0.0,
    };

    /// The coupled line solve along the major axis of an untruncated Hertz contact, bare
    /// half-spaces of steel both, gap `k_L y²/2` along the line: `N` uniform panels over 1.6 of
    /// the Hertz half-length each way, each station's strip of half-width
    /// `b = √(4 q/(π E* k_A))/C` (the local Hertz line's, widened by the shape factor), the
    /// active set, the approach `D` in closed form per iterate, and the widths to a fixed point.
    /// Returns `D/δ_H − 1` against the Hertz approach ([`crate::hertz::elliptical_contact`]).
    fn ellipse_error(n: usize, (kl, ka): (f64, f64), line: Line) -> f64 {
        const LOAD: f64 = 1000.0;
        // An unloaded station's strip: any width serves, its load is zero until the active set
        // admits it, and the next fixed-point pass then sets it.
        const Q_FLOOR: f64 = 1e-9;
        const STOP: f64 = 1e-12;
        let kernel = Kernel::shared();
        let body = steel();
        let nu = body.poisson;
        let es = body.modulus / (2.0 * (1.0 - nu * nu));
        let hertz = crate::hertz::elliptical_contact(kl, ka, LOAD, es).expect("an ellipse");
        let (a, approach) = (hertz.semi_x, hertz.approach);
        let c = (line.shape)(kl / ka);
        let half = 1.6 * a;
        let l = 2.0 * half / n as f64;
        let ys: Vec<f64> = (0..n)
            .map(|i| -half + l * (i as f64 + 0.5 + line.station))
            .collect();
        let mut q: Vec<f64> = ys
            .iter()
            .map(|y| 0.75 * LOAD / a * (1.0 - (y / a).powi(2)).max(0.0))
            .collect();
        let mut active = vec![true; n];
        let mut d = f64::NAN;
        for pass in 0.. {
            assert!(pass < 100, "the widths' fixed point did not settle");
            let b: Vec<f64> = q
                .iter()
                .map(|&qj| (4.0 * qj.max(Q_FLOOR) / (PI * es * ka)).sqrt() / c)
                .collect();
            let k: Vec<Vec<f64>> = ys
                .iter()
                .map(|yi| {
                    ys.iter()
                        .zip(&b)
                        .map(|(yj, &bj)| {
                            let p = Panel::new(yj - l / 2.0 - yi, yj + l / 2.0 - yi, bj, None)
                                .expect("a panel");
                            2.0 * kernel.panel(&p, body)
                        })
                        .collect()
                })
                .collect();
            let mut next = vec![0.0; n];
            for round in 0.. {
                assert!(round < n, "the active set did not settle");
                let idx: Vec<usize> = (0..n).filter(|&i| active[i]).collect();
                let m = idx
                    .iter()
                    .map(|&i| idx.iter().map(|&j| k[i][j]).collect())
                    .collect();
                let ones = vec![1.0; idx.len()];
                let gap = idx.iter().map(|&i| -kl * ys[i] * ys[i] / 2.0).collect();
                let x = solve(m, vec![ones, gap]);
                d = (LOAD / l - x[1].iter().sum::<f64>()) / x[0].iter().sum::<f64>();
                next = vec![0.0; n];
                for (s, &i) in idx.iter().enumerate() {
                    next[i] = d * x[0][s] + x[1][s];
                }
                let negative: Vec<usize> = idx.iter().copied().filter(|&i| next[i] < 0.0).collect();
                let pressing: Vec<usize> = (0..n)
                    .filter(|&i| {
                        !active[i]
                            && d - kl * ys[i] * ys[i] / 2.0
                                - k[i]
                                    .iter()
                                    .zip(&next)
                                    .map(|(kij, qj)| kij * qj)
                                    .sum::<f64>()
                                > 0.0
                    })
                    .collect();
                if negative.is_empty() && pressing.is_empty() {
                    break;
                }
                for &i in &negative {
                    active[i] = false;
                }
                if negative.is_empty() {
                    for &i in &pressing {
                        active[i] = true;
                    }
                }
            }
            let top = next.iter().copied().fold(0.0_f64, f64::max);
            let moved = next
                .iter()
                .zip(&q)
                .map(|(u, v)| (u - v).abs())
                .fold(0.0_f64, f64::max);
            q = next;
            if moved <= STOP * top {
                break;
            }
        }
        d / approach - 1.0
    }

    /// The orders a line's approach converges at, per doubling of `N` from 16 to 128, on the two
    /// curvature ratios: `log₂(e(N)/e(2N))`.
    fn orders(line: Line) -> Vec<(f64, f64)> {
        let mut out = Vec::new();
        for ratio in [(0.05, 0.5), (0.2, 0.4)] {
            let e: Vec<f64> = [16, 32, 64, 128]
                .iter()
                .map(|&n| ellipse_error(n, ratio, line))
                .collect();
            for w in e.windows(2) {
                out.push(((w[0] / w[1]).log2(), w[1]));
            }
        }
        out
    }

    /// **The Hertz ellipse at O(N⁻²)**: the line solve through the kernel reproduces the Hertz
    /// ellipse, its approach converging at second order in the panel count — the order each
    /// doubling from 16 to 128 shows rounds to 2 (half an order either side is the half-way to
    /// the neighbouring orders), on two curvature ratios, the error of one sign throughout.
    ///
    /// Plants: the shape factor left out (the strips as wide as a line's), a wrong limit; the
    /// shape factor `2e-4` high, a wrong limit a `|e(128)| < 1e-4` gate passes (asserted); the
    /// stations at their panels' starts, which converges at first order on the coarse grids.
    #[test]
    fn the_hertz_ellipse_converges_at_second_order() {
        let second = |o: &[(f64, f64)]| {
            o.iter().all(|&(p, _)| (p - 2.0).abs() < 0.5)
                && o.iter().all(|&(_, e)| e.signum() == o[0].1.signum())
        };
        let port = orders(THE_LINE);
        eprintln!(
            "ellipse: orders {:?}, e(128) {:.2e} and {:.2e}",
            port.iter().map(|o| o.0).collect::<Vec<_>>(),
            port[2].1,
            port[5].1
        );
        assert_eq!(port.len(), 6);
        assert!(second(&port), "{port:?}");

        let plants = [
            Line {
                shape: |_| 1.0,
                ..THE_LINE
            },
            Line {
                shape: |q| 1.0002 * shape_factor(q).expect("q in [0, 1]"),
                ..THE_LINE
            },
            Line {
                station: -0.5,
                ..THE_LINE
            },
        ];
        for (k, plant) in plants.into_iter().enumerate() {
            let o = orders(plant);
            let last = o.last().expect("an order").1;
            eprintln!(
                "plant {k}: orders {:?}, e(128) {last:.2e}",
                o.iter().map(|o| o.0).collect::<Vec<_>>()
            );
            assert!(!second(&o), "plant {k} passes");
            if k == 1 {
                assert!(
                    last.abs() < 1e-4,
                    "a near miss: a 1e-4 gate on e(128) passes it"
                );
            }
        }
    }

    // ---------------------------------------------------------------- G, and the shape factor

    /// **`G`'s table is its quadrature**: at every cell's middle across the table, where cubic
    /// Hermite errs most, the table equals [`g_direct`] to `G`'s oracle tolerance (`1e-12`; the
    /// prototype measured `4e-14`); at each end it meets its outer form to rounding (`8ε` of `G`,
    /// both forms exact there); `G` is odd and `G(0) = 0`; `G′` is the table's slope, and
    /// `None` at 0. Plant: round six's step, 0.02, which a `1e-10` gate passes (asserted).
    #[test]
    fn the_table_is_g() {
        let midpoints = |k: &Kernel| -> (usize, f64) {
            let cells = k.table.len() - 1;
            let worst = (0..cells)
                .map(|i| {
                    let r = (TABLE_FROM + (i as f64 + 0.5) * k.step).exp();
                    (k.g(r) - g_direct(r)).abs()
                })
                .fold(0.0_f64, f64::max);
            (cells, worst)
        };
        let kernel = Kernel::shared();
        let (cells, worst) = midpoints(kernel);
        eprintln!("G: {cells} cells, worst {worst:.1e} from the quadrature");
        assert_eq!(cells, 16_800);
        assert!(worst <= G_ABS, "{worst:e}");
        let (_, six) = midpoints(&Kernel::with_step(0.02));
        eprintln!("round six's step: worst {six:.1e}");
        assert!(six > G_ABS && six < 1e-10);

        for end in [TABLE_FROM, TABLE_TO] {
            let r = end.exp();
            for x in [r.next_down(), r, r.next_up()] {
                let g = kernel.g(x);
                assert!(
                    (g - g_direct(x)).abs() <= 8.0 * EPS * g.abs().max(1.0),
                    "{x:e}"
                );
            }
        }
        for r in [1e-13, 0.3, 7.0, 3e6] {
            assert_eq!(kernel.g(-r), -kernel.g(r));
            assert_eq!(g_direct(-r), -g_direct(r));
        }
        assert_eq!(
            (kernel.g(0.0), g_direct(0.0), g_slope(0.0)),
            (0.0, 0.0, None)
        );
    }

    /// The shape factor: `C(0) = 1` (the line) and `C(1) = 2/√π` (the circle, `R_D(0, 1, 1) =
    /// 3π/4`), increasing between, and continuous where `s = κ²` falls below [`SHAPE_ONE`]:
    /// `C` read through `R_D` just above it is 1 to two roundings. `None` outside `[0, 1]`. Plant:
    /// the switch at `s = 1e-12`, which a `1e-9` look at `C` near 0 passes (asserted).
    #[test]
    fn the_shape_factor_runs_from_the_line_to_the_circle() {
        assert_eq!(shape_factor(0.0), Some(1.0));
        let circle = 2.0 / PI.sqrt();
        let c1 = shape_factor(1.0).expect("C(1)");
        assert!(
            (c1 - circle).abs() <= 4.0 * EPS * circle,
            "{c1} vs {circle}"
        );
        let qs: Vec<f64> = (0..=40)
            .map(|k| 10f64.powf(-16.0 * f64::from(40 - k) / 40.0))
            .collect();
        let cs: Vec<f64> = qs.iter().map(|&q| shape_factor(q).expect("C")).collect();
        assert!(cs.windows(2).all(|w| w[0] <= w[1]), "{cs:?}");
        let through = |s: f64| (3.0 / (s * r_d(0.0, 1.0, s).expect("R_D"))).sqrt();
        let jump = |one: f64| (through(one) - 1.0).abs();
        assert!(jump(SHAPE_ONE) <= 2.0 * EPS, "{}", jump(SHAPE_ONE));
        let planted = 1e-12;
        assert!(jump(planted) > 2.0 * EPS && jump(planted) < 1e-9);
        for q in [-1e-300, 1.0 + EPS, f64::NAN, f64::INFINITY] {
            assert_eq!(shape_factor(q), None, "{q}");
        }
    }

    /// What is no panel and no half-space is refused: a panel with an end not a number, a width
    /// not a finite length `> 0`, or a depth not one (`None` is the bare half-space, not a depth
    /// of `∞`); a modulus not a finite positive one, a Poisson's ratio outside `(−1, 1/2]`.
    #[test]
    fn what_is_no_panel_is_refused() {
        let good = Panel::new(-0.1, 0.2, 0.05, Some(0.5));
        assert!(good.is_some() && Panel::new(-0.1, 0.2, 0.05, None).is_some());
        let refused = [
            (f64::NAN, 0.2, 0.05, Some(0.5)),
            (-0.1, f64::INFINITY, 0.05, Some(0.5)),
            (-0.1, 0.2, 0.0, Some(0.5)),
            (-0.1, 0.2, -0.05, Some(0.5)),
            (-0.1, 0.2, f64::INFINITY, Some(0.5)),
            (-0.1, 0.2, 0.05, Some(0.0)),
            (-0.1, 0.2, 0.05, Some(-0.5)),
            (-0.1, 0.2, 0.05, Some(f64::INFINITY)),
            (-0.1, 0.2, 0.05, Some(f64::NAN)),
        ];
        for (from, to, b, h) in refused {
            assert_eq!(Panel::new(from, to, b, h), None, "{from} {to} {b} {h:?}");
        }
        assert!(HalfSpace::new(206_000.0, 0.5).is_some() && HalfSpace::new(1.0, -0.99).is_some());
        for (e, nu) in [
            (0.0, 0.3),
            (-1.0, 0.3),
            (f64::INFINITY, 0.3),
            (f64::NAN, 0.3),
            (1.0, -1.0),
            (1.0, 0.5 + EPS),
            (1.0, f64::NAN),
        ] {
            assert_eq!(HalfSpace::new(e, nu), None, "{e} {nu}");
        }
    }
}
