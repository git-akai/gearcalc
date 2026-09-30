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
//! Within the near form's terms, the differences that cancel are removed. On one side of the
//! point its depth term is `asinh(D/d_R)` and `h² D/(d_R R₁ R₂)`, `D = (η₂ − η₁)(η₂ + η₁)`,
//! `d_R = η₂ R₁ + η₁ R₂`, rather than differences of `asinh(η/ρ)` and of `η/R`; its surface term,
//! where the farther end is within `4.15` times the nearer, is `∫ G′` by Gauss–Legendre
//! ([`SLOPE_NODES`]) rather than `G(r₂) − G(r₁)`; across the point every difference is a sum.
//! Two differences remain, each bounded. The near form is its surface term less its depth term,
//! two values of one sign, so it errs a few roundings of the larger of them rather than of the
//! value: its gate is `1e-13` of that scale, and the separated form takes over where the two
//! would amplify `G`'s error. Beyond the `4.15` ratio the surface is `G(r₂) − G(r₁)`, which
//! cancels by a factor of at most 2.9 with its nearer end within `b/2` of the point, growing to
//! 23 at the table's end ([`SLOPE_NODES`]). `G` itself is exact to a few roundings of itself at
//! every `r`: the table holds `G/asinh r`, which is linear in `ln r` below it, and its nodes are
//! a quadrature of positive terms ([`g_direct`]).
//!
//! Ports the prototype's `kernel.G`, `G_direct`, `dG`, `panel`, `panel_sep`, `depth_panel`,
//! `n_width`, `line_limit` and `shape_C`, and `carlson.aspect` through [`patch_aspect`]; its
//! records are `tests/data/field_oracle/kernel.json`. Every length here is relative: the kernel is
//! unchanged when every length is scaled.

use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI};
use std::sync::OnceLock;

use crate::elliptic::r_d;
use crate::hertz::{gauss_legendre, gauss_legendre_rule, patch_aspect};

/// Where `G`'s table starts, in `u = ln r`. Below it `G` is `(4/π) r ln(4/r)`, whose remainder,
/// `O(r³ ln r)`, is below `1e-24` of `G` there.
const TABLE_FROM: f64 = -28.0;

/// Where `G`'s table ends, in `u = ln r`. Above it `G` is its asymptote `ln 4r + 1/2 + 1/(16 r²)`,
/// whose remainder, `O(r⁻⁴)`, is below `1e-24` there.
const TABLE_TO: f64 = 14.0;

/// The table's step in `u = ln r`. The table holds `F = G/asinh r`, which is `(4/π)(ln 4 − u)`,
/// linear, below it and tends to 1 above it, so cubic Hermite with the exact slope errs at most
/// `step⁴ max|∂⁴F/∂u⁴ / F| / 384` of `G` in a cell; `max|∂⁴F/∂u⁴ / F| = 0.288`, at `u ≈ 0.05`,
/// so `1.8e-15` of `G` here. (`G` itself, tabulated, errs up to `0.87 step⁴/384` of itself as
/// `r → 0`, where it is `(4/π) r ln(4/r)`.)
const TABLE_STEP: f64 = 0.00125;

/// Gauss–Legendre nodes per cell of [`g_direct`]. Each cell's rule converges as `ρ⁻³²`, `ρ ≥ 4.3`
/// the Bernstein ellipse through the integrand's nearest branch point, `ψ = i asinh r`
/// (`4.3⁻³² ≈ 4e-21`), so the sum is exact to rounding.
const CELL_NODES: usize = 16;

/// The width rule's bound: `n` Gauss–Chebyshev nodes integrate the depth term to `O(E⁻²ⁿ)`, `E`
/// the Bernstein ellipse through its branch points `±i h`; `n` is the least count that puts
/// `E⁻²ⁿ` below this, plus one node of margin for the error's constant ([`nodes`]).
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

/// The bound of the rule that integrates `G′` along a panel on one side of the point, as
/// [`WIDTH_TOL`]: `G′`'s branch point is the point, `r = 0`, so the Bernstein ellipse is the
/// panel's through it, `E = k + √(k² − 1)`, `k = |η₁ + η₂|/(η₂ − η₁)`.
const SLOPE_TOL: f64 = 1e-14;

/// The most Gauss–Legendre nodes the surface term takes along a panel on one side of the point:
/// at most this many reach [`SLOPE_TOL`] where the farther end is within `4.15` times the nearer
/// (`E ≥ 2.93`). Beyond it the surface is `G(r₂) − G(r₁)`, whose terms cancel by at most
/// `(G₂ + G₁)/(G₂ − G₁)` at `r₂ = 4.15 r₁`: 1.7 … 2.9 for a nearer end within `b/2` (the near
/// form's, with a depth), growing as `2 ln 4r/ln 4.15` beyond (23 at the table's end, `r₁ = e^14`,
/// on a bare panel four times as long as its distance), each factor on `G`'s own few roundings.
const SLOPE_NODES: usize = 16;

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
    /// `2(1 − ν²)/(π E) [asinh(h/b) − ν/(1 − ν) · h/(√(h² + b²) + h)]`, mm²/N. `None` unless `b`
    /// and `h` are finite lengths `> 0`.
    pub fn line_limit(self, half_width: f64, depth: f64) -> Option<f64> {
        let (b, h, nu) = (half_width, depth, self.poisson);
        let length = |v: f64| v.is_finite() && v > 0.0;
        (length(b) && length(h)).then(|| {
            2.0 * (1.0 - nu * nu) / (PI * self.modulus)
                * ((h / b).asinh() - nu / (1.0 - nu) * h / (h.hypot(b) + h))
        })
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
    /// `None` unless the half-width is a finite length `> 0`, the panel runs forward (`from <
    /// to`: a panel of no length loads nothing and a reversed one is the same panel negated),
    /// both ends are finite in half-widths, and a depth, where there is one, is a finite length
    /// `> 0`.
    pub fn new(from: f64, to: f64, half_width: f64, depth: Option<f64>) -> Option<Self> {
        let length = |v: f64| v.is_finite() && v > 0.0;
        let finite = |end: f64| (end / half_width).is_finite();
        let ends = from < to && finite(from) && finite(to);
        (length(half_width) && ends && depth.is_none_or(length)).then_some(Self {
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
}

/// The width rule's Gauss–Chebyshev node count for [`WIDTH_TOL`] ([`nodes`]), `E = h/b +
/// √(1 + (h/b)²)` the Bernstein ellipse through the depth term's branch points `x = ±i h`.
fn width_rule(half_width: f64, depth: f64) -> usize {
    let r = depth / half_width;
    nodes(r + (1.0 + r * r).sqrt(), WIDTH_TOL)
}

/// A rule's node count for the Bernstein ellipse `E` and bound `tol`: the least count with
/// `E⁻²ⁿ ≤ tol`, `⌈ln(1/tol) / (2 ln E)⌉`, plus one node of margin for the error's constant,
/// `n = ⌈ln(1/tol) / (2 ln E)⌉ + 1`, held to `[2, MAX_WIDTH_NODES]`.
#[expect(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "an integer in [2, MAX_WIDTH_NODES], clamped before the cast"
)]
fn nodes(ellipse: f64, tol: f64) -> usize {
    let n = ((1.0 / tol).ln() / (2.0 * ellipse.ln())).ceil() + 1.0;
    n.clamp(2.0, MAX_WIDTH_NODES as f64) as usize
}

/// The kernel's tables, built once: `F = G/asinh r` in `u = ln r`, whose cubic Hermite gives
/// `G` ([`Kernel::g`]); the Gauss–Chebyshev rules across the width, for the depth term and the
/// separated form; and the Gauss–Legendre rules along a panel, for the surface term's `∫ G′`.
#[derive(Clone, Debug)]
pub struct Kernel {
    /// The table's step in `u = ln r`.
    step: f64,
    /// `(F, dF/du)`, `F = G/asinh r`, at `u = TABLE_FROM + i · step`.
    table: Vec<(f64, f64)>,
    /// Gauss–Chebyshev (second kind) rules for `φ`, by node count: `(t_k, w_k)` with
    /// `t_k = cos(kπ/(n + 1))` and `w_k = (2/(n + 1)) sin²(kπ/(n + 1))`, `k = 1 … n`.
    rules: Vec<Vec<(f64, f64)>>,
    /// Gauss–Legendre rules on `[−1, 1]` for `∫ G′` along a panel, by node count, to
    /// [`SLOPE_NODES`].
    slopes: Vec<Vec<(f64, f64)>>,
}

impl Kernel {
    /// The kernel, built on first use.
    pub fn shared() -> &'static Self {
        static KERNEL: OnceLock<Kernel> = OnceLock::new();
        KERNEL.get_or_init(|| Self::with_step(TABLE_STEP))
    }

    /// The kernel with `F = G/asinh r` tabulated at `step` in `u = ln r`.
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
                let (g, area) = (g_direct_with(r, &cells), r.asinh());
                // dF/du = r G′/A − G r/(√(1 + r²) A²), A = asinh r.
                let dfdu = r * slope(r) / area - g * r / (r.hypot(1.0) * area * area);
                (g / area, dfdu)
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
        let slopes = (0..=SLOPE_NODES).map(gauss_legendre_rule).collect();
        Self {
            step,
            table,
            rules,
            slopes,
        }
    }

    /// `G(r)`: `asinh r` times the table's cubic Hermite of `G/asinh r` in `u = ln r` with the
    /// exact slope, the asymptote above it and the small-`r` form below it; odd, `G(0) = 0`.
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
            let ((f0, d0), (f1, d1)) = (self.table[cell], self.table[cell + 1]);
            let f = (2.0 * t3 - 3.0 * t2 + 1.0) * f0
                + (t3 - 2.0 * t2 + t) * self.step * d0
                + (-2.0 * t3 + 3.0 * t2) * f1
                + (t3 - t2) * self.step * d1;
            f * x.asinh()
        } else {
            // Below the table, and NaN, which the comparisons above pass here. `ln 4 − ln r`, not
            // `ln(4/r)`, which overflows for a subnormal `r`.
            4.0 / PI * x * (4.0_f64.ln() - x.ln())
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

    /// The near form: the surface term ([`Kernel::surface`]) less, with a depth, the depth term by
    /// the width rule.
    fn near(&self, panel: &Panel, body: HalfSpace) -> f64 {
        let Panel {
            from,
            to,
            half_width: b,
            depth,
        } = *panel;
        let surface = body.surface() * self.surface(from, to, b);
        depth.map_or(surface, |h| {
            let rule = self.rule(width_rule(b, h));
            surface - body.interior() * depth_term(from, to, b, h, body.poisson, rule)
        })
    }

    /// `G(η₂/b) − G(η₁/b)`, the surface term per unit of `(1 − ν²)/(π E)`. On one side of the
    /// point, where [`SLOPE_NODES`] Gauss–Legendre nodes reach [`SLOPE_TOL`], `∫ G′` along the
    /// panel by that rule: the two values of `G` would cancel there by `G/(G′ Δr)`, `2e4` on a
    /// panel `3e-5 b` long `0.4 b` from the point. Otherwise the difference, which
    /// across the point is a sum and on one side cancels by at most [`SLOPE_NODES`]' factor.
    fn surface(&self, from: f64, to: f64, b: f64) -> f64 {
        if from * to > 0.0 {
            let k = (from + to).abs() / (to - from);
            let n = nodes(k + (k * k - 1.0).sqrt(), SLOPE_TOL);
            if n <= SLOPE_NODES {
                let (mid, half) = ((from + to) / (2.0 * b), (to - from) / (2.0 * b));
                let sum: f64 = self.slopes[n]
                    .iter()
                    .map(|&(t, w)| w * slope(mid + half * t))
                    .sum();
                return half * sum;
            }
        }
        self.g(to / b) - self.g(from / b)
    }

    /// The `n`-node rule.
    fn rule(&self, n: usize) -> &[(f64, f64)] {
        &self.rules[n]
    }
}

/// The depth term's integral across the width by `rule`, per unit of `(1 + ν)/(2π E)`:
/// `∫ φ [2(1 − ν)(asinh(η₂/ρ) − asinh(η₁/ρ)) + h² (η₂/(ρ² R₂) − η₁/(ρ² R₁))]`. Across the point
/// each difference is a sum of two terms of one sign; on one side it is taken without
/// cancellation: `asinh(η₂/ρ) − asinh(η₁/ρ) = asinh(D/d_R)` and `η₂/R₂ − η₁/R₁ = ρ² D/(d_R R₁ R₂)`,
/// `D = (η₂ − η₁)(η₂ + η₁)`, `d_R = η₂ R₁ + η₁ R₂`, whose terms are of one sign there. `d_R`
/// vanishes only where `ρ` does, at `x = ±i h`, so the width rule serves both.
fn depth_term(from: f64, to: f64, b: f64, h: f64, nu: f64, rule: &[(f64, f64)]) -> f64 {
    let (h2, d) = (h * h, (to - from) * (to + from));
    let one_side = from * to > 0.0;
    rule.iter()
        .map(|&(t, w)| {
            let x = b * t;
            let rho2 = x * x + h2;
            let (dist1, dist2) = ((rho2 + from * from).sqrt(), (rho2 + to * to).sqrt());
            let (apart, over) = if one_side {
                let dr = to * dist1 + from * dist2;
                ((d / dr).asinh(), h2 * d / (dr * dist1 * dist2))
            } else {
                let rho = rho2.sqrt();
                let apart = (to / rho).asinh() - (from / rho).asinh();
                (apart, h2 * (to / (rho2 * dist2) - from / (rho2 * dist1)))
            };
            w * (2.0 * (1.0 - nu) * apart + over)
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

/// `G(r)` by quadrature, integrated by parts so that every term is positive. With
/// `Φ(x) = ∫₀ˣ φ = (x √(1 − x²) + asin x)/π`, `Φ(1) = 1/2`, and `x = sin ψ`,
///
/// ```text
/// G(r) = 2 ∫₀¹ φ(x) asinh(r/x) dx = asinh r + 2r ∫₀¹ Φ(x) dx/(x √(x² + r²))
///      = asinh r + (2r/π) ∫₀^{π/2} cos ψ (cos ψ + ψ/sin ψ) dψ / √(sin²ψ + r²),
/// ```
///
/// the integrand bounded (`2/r` at `ψ = 0`) and analytic but for branch points at
/// `ψ = ±i asinh r`, over cells doubling from `min(r, π/4)` to `π/2`, 16-point Gauss–Legendre on
/// each, summed with compensation. Exact to a few roundings of `G` itself at every `r`, since
/// no term is of the other sign. Odd, `G(0) = 0`.
pub fn g_direct(r: f64) -> f64 {
    g_direct_with(r, &gauss_legendre::<CELL_NODES>())
}

fn g_direct_with(r: f64, cells: &[(f64, f64)]) -> f64 {
    let x = r.abs();
    if x == 0.0 {
        return r;
    }
    let f = |psi: f64| {
        let (s, c) = psi.sin_cos();
        c * (c + psi / s) / s.hypot(x)
    };
    // Cells double from `min(r, π/4)`: `⌈log₂(π/(2 min(r, π/4)))⌉` of them before `π/2`.
    let mut edges = vec![0.0];
    let mut a = x.min(FRAC_PI_4);
    while a < FRAC_PI_2 {
        edges.push(a);
        a *= 2.0;
    }
    edges.push(FRAC_PI_2);
    let total = compensated(edges.windows(2).flat_map(|e| {
        let (half, mid) = (0.5 * (e[1] - e[0]), 0.5 * (e[1] + e[0]));
        cells
            .iter()
            .map(move |&(t, w)| half * w * f(mid + half * t))
    }));
    (x.asinh() + 2.0 * x / PI * total).copysign(r)
}

/// Neumaier's compensated sum: each addition's rounding is carried apart and added once at the
/// end, so the sum errs two roundings of itself rather than one per term.
fn compensated(terms: impl IntoIterator<Item = f64>) -> f64 {
    let (mut total, mut carry) = (0.0_f64, 0.0_f64);
    for term in terms {
        let next = total + term;
        carry += if total.abs() >= term.abs() {
            (total - next) + term
        } else {
            (term - next) + total
        };
        total = next;
    }
    total + carry
}

/// `G′(r) = (4/(3π)) (1 + r²) R_D(0, r², 1 + r²)`, even. `None` at `r = 0`, where it grows as
/// `(4/π) ln(4/|r|)` without bound, and where `r²` is not finite.
pub fn g_slope(r: f64) -> Option<f64> {
    let r2 = r * r;
    r_d(0.0, r2, 1.0 + r2).map(|d| 4.0 / (3.0 * PI) * (1.0 + r2) * d)
}

/// `G′(r)` for `r ≠ 0`, by `G`'s three forms: the slopes of the outer forms beyond the table's
/// ends (`(4/π)(ln(4/r) − 1)` below, `1/r − 1/(8 r³)` above, each to `1e-24` of itself there),
/// [`g_slope`]'s closed form between, where `R_D`'s arguments are finite and `x + y > 0`.
#[expect(
    clippy::expect_used,
    reason = "R_D(0, r², 1 + r²) for e^−28 < |r| < e^14: finite arguments, x + y > 0, z > 0"
)]
fn slope(r: f64) -> f64 {
    let x = r.abs();
    let u = x.ln();
    if u >= TABLE_TO {
        1.0 / x - 1.0 / (8.0 * x * x * x)
    } else if u > TABLE_FROM {
        g_slope(x).expect("R_D's domain holds on the table")
    } else {
        4.0 / PI * (4.0_f64.ln() - x.ln() - 1.0)
    }
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
    use crate::hertz::elliptical_contact;
    use serde_json::Value;
    use std::f64::consts::LN_2;

    const EPS: f64 = f64::EPSILON;

    /// How many roundings of `G` [`g_direct`] errs by at most: each term's own dozen (the node,
    /// its sine and cosine, the quotient, the sum, the product, the hypotenuse, the division,
    /// the weight and the half-width, each moving the term by at most its own rounding), two
    /// for the compensated sum, and six for `asinh r` and the terms' combination. Every term is
    /// positive, so none of these is amplified.
    const DIRECT_ROUNDINGS: f64 = 20.0;

    fn oracle_file() -> Value {
        serde_json::from_str(include_str!("../../tests/data/field_oracle/kernel.json"))
            .expect("kernel.json parses")
    }

    fn steel() -> HalfSpace {
        HalfSpace::new(206_000.0, 0.3).expect("a half-space")
    }

    /// The width rule's node count for a panel's depth term; `None` for the bare half-space.
    fn nodes_of(p: &Panel) -> Option<usize> {
        p.depth.map(|h| width_rule(p.half_width, h))
    }

    // ---------------------------------------------------------------- the oracle
    //
    // The prototype's records, and the laws on them, go with the prototype. What holds each
    // record's function then shares neither code nor method with it:
    // - G, G_direct, G′: `g_is_its_definitions_quadrature` (double-double, in x = r sinh v) and
    //   `the_table_is_g`;
    // - Carlson's R_F and R_D: `elliptic.rs`'s laws (each definition's quadrature; the complete
    //   integrals against the arithmetic–geometric mean);
    // - the line limit: `the_kernel_sums_to_the_line_limit` and
    //   `the_line_limit_deep_in_the_tooth_is_weber_and_banascheks`;
    // - the panels, their routes and node counts: `every_panel_is_its_definitions_quadrature`,
    //   `the_forms_agree_at_the_switch` and `the_width_rule_holds_its_bound`;
    // - C and the aspect: `the_shape_factor_is_the_ellipses_peak_over_the_lines`,
    //   `the_shape_factor_runs_from_the_line_to_the_circle`,
    //   `the_hertz_ellipse_converges_at_second_order`, and `hertz.rs`'s aspect laws.

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
                        let port = body.line_limit(num(&c[0]), num(&c[1])).expect("a strip");
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
                            + usize::from(nodes_of(&p) != width);
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
        let v1 = reproduce(kernel, near_by_differences, |x, _| x, bound);
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

    // ---------------------------------------------------------------- the double-double references

    /// How far along `t` tanh-sinh reaches: at `|t| = 4.2` its weight is below `1e-43`.
    const REACH: f64 = 4.2;

    /// `∫₀^len f` by tanh-sinh at step `hs` in double-double: `x = len/(1 + e^{−2u})`,
    /// `u = (π/2) sinh t`. `f` is handed `x` and `len − x`, each without cancellation, so an
    /// integrable singularity at either end costs nothing.
    fn tanh_sinh(len: Wide, hs: f64, f: impl Fn(Wide, Wide) -> Wide) -> Wide {
        let (one, two) = (Wide::of(1.0), Wide::of(2.0));
        let half_pi = Wide::pi() * Wide::of(0.5);
        #[expect(clippy::cast_possible_truncation, reason = "a node count")]
        let reach = (REACH / hs) as i32;
        let mut sum = Wide::of(0.0);
        for k in -reach..=reach {
            let t = Wide::of(f64::from(k) * hs);
            let u = half_pi * t.sinh();
            let e2u = (two * u).exp();
            let (from_zero, to_end) = (len / (one + one / e2u), len / (e2u + one));
            if from_zero.hi == 0.0 || to_end.hi == 0.0 {
                continue;
            }
            let cu = u.cosh();
            sum = sum + half_pi * t.cosh() / (cu * cu) * f(from_zero, to_end);
        }
        sum * len * Wide::of(0.5 * hs)
    }

    /// `∫ φ(x) g(|x|) dx` across a strip of half-width `b`, `φ(x) = (2/(π b²)) √(b² − x²)`.
    fn strip(b: Wide, hs: f64, g: impl Fn(Wide) -> Wide) -> Wide {
        let two = Wide::of(2.0);
        let phi = |x: Wide, to_b: Wide| two / (Wide::pi() * b * b) * (to_b * (b + x)).sqrt();
        two * tanh_sinh(b, hs, |x, to_b| phi(x, to_b) * g(x))
    }

    /// A panel's definition in its three parts, each across the strip in double-double with its
    /// factor and `η` integrated in closed form: the surface term `S`, and the depth term's two,
    /// `D_ρ` (its `asinh`s) and `D_h` (its `h²` term). The panel is `S − D_ρ − D_h`. Shares no
    /// code with the kernel (the prototype's `oracle-fix/check_panel_dec.py` did this in 50
    /// digits).
    struct Terms {
        surface: Wide,
        asinh: Wide,
        h2: Wide,
    }

    impl Terms {
        fn value(&self) -> Wide {
            self.surface - self.asinh - self.h2
        }

        /// The larger of the two terms the form `route` subtracts: the near form's surface and
        /// depth; the separated form's `S − D_ρ` (its `asinh` difference) and `D_h`. An exact
        /// evaluation by that form errs a few roundings of it.
        fn scale(&self, route: Route) -> f64 {
            let (a, b) = match route {
                Route::Near => (self.surface, self.asinh + self.h2),
                Route::Separated => (self.surface - self.asinh, self.h2),
            };
            a.abs().to_f64().max(b.abs().to_f64())
        }
    }

    fn reference_terms(p: &Panel, body: HalfSpace, hs: f64) -> Terms {
        let w = Wide::of;
        let (e1, e2, b) = (w(p.from), w(p.to), w(p.half_width));
        let (nu, e) = (w(body.poisson), w(body.modulus));
        let (one, two, pi) = (w(1.0), w(2.0), Wide::pi());
        let cs = (one - nu * nu) / (pi * e);
        let cd = (one + nu) / (two * pi * e);
        let surface = strip(b, hs, |x| cs * ((e2 / x).asinh() - (e1 / x).asinh()));
        let Some(h) = p.depth else {
            return Terms {
                surface,
                asinh: w(0.0),
                h2: w(0.0),
            };
        };
        let h = w(h);
        let asinh = strip(b, hs, |x| {
            let rho = (x * x + h * h).sqrt();
            cd * two * (one - nu) * ((e2 / rho).asinh() - (e1 / rho).asinh())
        });
        let h2 = strip(b, hs, |x| {
            let r2 = x * x + h * h;
            let (d1, d2) = ((r2 + e1 * e1).sqrt(), (r2 + e2 * e2).sqrt());
            cd * h * h * (e2 / (r2 * d2) - e1 / (r2 * d1))
        });
        Terms { surface, asinh, h2 }
    }

    /// `(G(r), G′(r))`, `r > 0`, by their definitions in double-double, in `v = asinh(x/r)` so
    /// that the scale `r` is the variable's own at every `r`:
    ///
    /// ```text
    /// G(r)  = 2 ∫₀¹ φ(x) asinh(r/x) dx  = 2r ∫₀^V φ(r sinh v) asinh(1/sinh v) cosh v dv,
    /// G′(r) = 2 ∫₀¹ φ(x) dx/√(x² + r²) = 2  ∫₀^V φ(r sinh v) dv,     V = asinh(1/r),
    /// ```
    ///
    /// `φ(x) = (2/π) √(1 − x²)`. Shares neither code nor variable with [`g_direct`] (θ, cells of
    /// Gauss–Legendre), the table or [`g_slope`] (Carlson's `R_D`).
    fn g_reference(r: f64, hs: f64) -> (Wide, Wide) {
        let (r, one, two) = (Wide::of(r), Wide::of(1.0), Wide::of(2.0));
        let phi = |v: Wide| {
            let x = r * v.sinh();
            two / Wide::pi() * (one - x * x).sqrt()
        };
        let end = (one / r).asinh();
        let g = tanh_sinh(end, hs, |v, _| phi(v) * (one / v.sinh()).asinh() * v.cosh());
        let slope = tanh_sinh(end, hs, |v, _| phi(v));
        (two * r * g, two * slope)
    }

    // ---------------------------------------------------------------- G

    /// `max |∂⁴F/∂u⁴ / F|` over the table, `F = G/asinh r`, `u = ln r`: `0.288` at `u ≈ 0.05`,
    /// rounded up. [`the_table_is_g`] holds it against the third differences of the table's
    /// exact slope.
    const F4_REL: f64 = 0.29;

    /// Cubic Hermite of `F` with the exact slope, at `step` in `u`, relative to `G`:
    /// `step⁴ max|∂⁴F/∂u⁴ / F| / 384`.
    fn hermite(step: f64) -> f64 {
        step.powi(4) * F4_REL / 384.0
    }

    /// [`g_direct`]'s rounding at `G = g`: [`DIRECT_ROUNDINGS`] of it.
    fn direct_rounding(g: f64) -> f64 {
        DIRECT_ROUNDINGS * EPS * g.abs()
    }

    /// The table's rounding at `u = ln r`, relative to `G`: the cell's position `u − TABLE_FROM`
    /// errs `(|u| + 21) ε` (`ln r`'s own and the subtraction's, whose result is at most 42), which
    /// moves `ln G` by at most as much (`|d ln F/du| = |r G′/G − r/(√(1 + r²) asinh r)| ≤ 1`,
    /// both terms in `[0, 1]`); the interpolant's basis, sum and product with `asinh r`, twelve.
    fn table_rounding(r: f64) -> f64 {
        (r.ln().abs() + 21.0 + 12.0) * EPS
    }

    /// What `G` is held to at `r`: each of its three forms to what that form guarantees. Beyond
    /// the table's ends, the outer forms, whose remainders are below `1e-24` of `G` there, to
    /// their few roundings (`8 ε |G|`); in the table, the Hermite bound, the nodes' rounding
    /// (the Hermite weights of the two values sum to 1), and the table's own.
    fn g_bound(r: f64, g: f64) -> f64 {
        let u = r.ln();
        if u >= TABLE_TO || u <= TABLE_FROM {
            8.0 * EPS * g.abs()
        } else {
            (hermite(TABLE_STEP) + table_rounding(r)) * g.abs() + direct_rounding(g)
        }
    }

    /// The radii the `G` laws run over: four to a decade, from `1.3e-14`, below the table, to
    /// `1.3e6`, above it; none of them the oracle's (`10^{k/2}`).
    fn radii() -> Vec<f64> {
        (-56..=24)
            .map(|k| 10f64.powf(f64::from(k) / 4.0 + 0.125))
            .collect()
    }

    /// `G′`'s closed form with its leading constant `4/(3π)` typed to 11 digits.
    fn slope_typed(r: f64) -> f64 {
        let r2 = r * r;
        0.424_413_181_58 * (1.0 + r2) * r_d(0.0, r2, 1.0 + r2).expect("R_D")
    }

    /// The prototype's `G_direct`, and this port's before it: the `ln sin θ` singularity taken
    /// out in closed form over the whole range, `G = (4/π)[∫ cos²θ ln(r + √(r² + sin²θ)) +
    /// (π/4) ln 2 + π/8]`, whose two parts cancel to `G ~ r ln(1/r)` as `r → 0`: exact only in
    /// absolute terms there.
    fn g_direct_split(r: f64) -> f64 {
        let x = r.abs();
        let cells = gauss_legendre::<CELL_NODES>();
        let f = |th: f64| {
            let s = th.sin();
            th.cos().powi(2) * (x + (x * x + s * s).sqrt()).ln()
        };
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

    /// `G` itself tabulated in `u = ln r` with its exact slope `r G′`, as this port's table was
    /// before it held `G/asinh r`, at `step`: cubic Hermite errs up to `0.87 step⁴/384` of `G` as
    /// `r → 0`, where `∂⁴G/∂u⁴ → G (ln 4 − u − 4)/(ln 4 − u)`.
    struct GItself {
        step: f64,
        table: Vec<(f64, f64)>,
    }

    impl GItself {
        #[expect(clippy::cast_possible_truncation, reason = "a node count")]
        #[expect(clippy::cast_sign_loss, reason = "a node count")]
        fn new(step: f64) -> Self {
            let count = ((TABLE_TO - TABLE_FROM) / step).round() as usize + 1;
            let table = (0..count)
                .map(|i| {
                    let r = (TABLE_FROM + i as f64 * step).exp();
                    (g_direct(r), r * g_slope(r).expect("G′"))
                })
                .collect();
            Self { step, table }
        }

        #[expect(clippy::cast_possible_truncation, reason = "a cell index")]
        #[expect(clippy::cast_sign_loss, reason = "a cell index")]
        fn g(&self, r: f64) -> f64 {
            let u = r.ln();
            if !(TABLE_FROM..TABLE_TO).contains(&u) {
                return Kernel::shared().g(r);
            }
            let at = (u - TABLE_FROM) / self.step;
            let cell = at.floor().min((self.table.len() - 2) as f64) as usize;
            let t = at - cell as f64;
            let ((g0, d0), (g1, d1)) = (self.table[cell], self.table[cell + 1]);
            (2.0 * t.powi(3) - 3.0 * t * t + 1.0) * g0
                + (t.powi(3) - 2.0 * t * t + t) * self.step * d0
                + (-2.0 * t.powi(3) + 3.0 * t * t) * g1
                + (t.powi(3) - t * t) * self.step * d1
        }
    }

    /// Each radius with its reference `(G, G′)`, at steps 1/64 and 1/32.
    struct GCase {
        r: f64,
        g: Wide,
        slope: Wide,
        coarse: (Wide, Wide),
    }

    fn g_cases() -> Vec<GCase> {
        radii()
            .into_iter()
            .map(|r| {
                let (g, slope) = g_reference(r, 1.0 / 64.0);
                GCase {
                    r,
                    g,
                    slope,
                    coarse: g_reference(r, 1.0 / 32.0),
                }
            })
            .collect()
    }

    /// Which of `G`'s evaluations: the table (with its outer forms), [`g_direct`], [`g_slope`].
    const TABLE: usize = 0;
    const DIRECT: usize = 1;
    const SLOPE: usize = 2;

    /// Each value's own bound at `r`, its reference `x`: `G` by [`g_bound`], [`g_direct`] by
    /// [`direct_rounding`], `G′` by the closed-form rule (`1e-13`: a few dozen roundings of
    /// Carlson's duplication).
    fn g_rule(k: usize, r: f64, x: f64) -> f64 {
        match k {
            TABLE => g_bound(r, x),
            DIRECT => direct_rounding(x),
            _ => CLOSED_REL * x.abs(),
        }
    }

    /// The three evaluations, the port's or a plant's, at `r`.
    type Evaluations<'a> = [&'a dyn Fn(f64) -> f64; 3];

    /// A bound on one evaluation, by its index, `r` and its reference.
    type GBound<'a> = &'a dyn Fn(usize, f64, f64) -> f64;

    /// How many of each evaluation miss the reference by more than `bound` allows, and the worst
    /// ratio of each; `perturb` moves a value first (by its index and reference).
    fn g_missed(
        cases: &[GCase],
        at: Evaluations,
        perturb: impl Fn(usize, &GCase, f64) -> f64,
        bound: impl Fn(usize, f64, f64) -> f64,
    ) -> ([usize; 3], [f64; 3]) {
        let (mut missed, mut worst) = ([0; 3], [0.0_f64; 3]);
        for c in cases {
            for (k, f) in at.iter().enumerate() {
                let reference = if k == SLOPE { c.slope } else { c.g };
                let port = perturb(k, c, f(c.r));
                let off = (Wide::of(port) - reference).abs().to_f64();
                let ratio = off / bound(k, c.r, reference.to_f64());
                missed[k] += usize::from(ratio > 1.0 || ratio.is_nan());
                worst[k] = worst[k].max(ratio);
            }
        }
        (missed, worst)
    }

    /// **`G`, `G_direct` and `G′` are their definitions' quadratures**: at 81 radii from below the
    /// table to above it, the table (and its outer forms), [`g_direct`] and [`g_slope`] each
    /// equal a double-double quadrature of the definition, in a variable none of them uses, to
    /// the bound its own form guarantees, relative to `G` at every `r`; the reference at steps
    /// 1/32 and 1/64 agrees with itself to `1e-20`. The tolerance's two laws: every value a few
    /// roundings off passes, ten times its bound off misses everywhere.
    ///
    /// Plants, each missing somewhere and each passing the looser gate beside it (asserted):
    /// the prototype's tail below the table, linear in `r` (`G(r₀) r/r₀`: under `1e-12`
    /// absolute); `G` itself tabulated at the previous step, `0.0025` (under `1e-13` of `G`,
    /// the panel gate's own size); the prototype's `G_direct`, the `ln sin θ` split (under
    /// `1e-12` absolute); [`g_direct`] at 8 nodes a cell (under `1e-9` of `G`); `G′` with
    /// `4/(3π)` typed to 11 digits (under `1e-10` relative).
    #[test]
    fn g_is_its_definitions_quadrature() {
        let kernel = Kernel::shared();
        let cases = g_cases();
        assert_eq!(cases.len(), 81);
        let rel = |a: Wide, b: Wide| ((a - b) / a).abs().to_f64();
        let self_worst = cases
            .iter()
            .map(|c| rel(c.g, c.coarse.0).max(rel(c.slope, c.coarse.1)))
            .fold(0.0_f64, f64::max);
        assert!(self_worst < 1e-20, "the reference: {self_worst:e}");

        let table = |r: f64| kernel.g(r);
        let slope = |r: f64| g_slope(r).expect("G′ at r > 0");
        let port: Evaluations = [&table, &g_direct, &slope];
        let as_is = |_: usize, _: &GCase, x: f64| x;
        let (missed, worst) = g_missed(&cases, port, as_is, g_rule);
        eprintln!(
            "G: 81 radii, worst of the bound: table {:.3}, direct {:.3}, slope {:.3} (the \
             reference's own step change {self_worst:.1e})",
            worst[0], worst[1], worst[2]
        );
        assert_eq!(missed, [0; 3]);

        let rounded = |_: usize, _: &GCase, x: f64| x * (1.0 + 4.0 * EPS);
        assert_eq!(g_missed(&cases, port, rounded, g_rule).0, [0; 3]);
        let ten_times = |k: usize, c: &GCase, x: f64| {
            let reference = if k == SLOPE { c.slope } else { c.g };
            x + 10.0 * g_rule(k, c.r, reference.to_f64())
        };
        assert_eq!(g_missed(&cases, port, ten_times, g_rule).0, [81; 3]);

        let r0 = TABLE_FROM.exp();
        let linear = |r: f64| {
            if r < r0 {
                kernel.g(r0) * r / r0
            } else {
                kernel.g(r)
            }
        };
        let itself = GItself::new(0.0025);
        let itself_table = |r: f64| itself.g(r);
        let eight = |r: f64| g_direct_with(r, &gauss_legendre::<8>());
        let absolute = |a: f64| move |_: usize, _: f64, _: f64| a;
        let relative = |a: f64| move |_: usize, _: f64, x: f64| a * x.abs();
        // (the evaluations, which of them is planted, the looser gate it passes)
        let plants: [(Evaluations, usize, GBound); 5] = [
            ([&linear, &g_direct, &slope], TABLE, &absolute(1e-12)),
            ([&itself_table, &g_direct, &slope], TABLE, &relative(1e-13)),
            ([&table, &g_direct_split, &slope], DIRECT, &absolute(1e-12)),
            ([&table, &eight, &slope], DIRECT, &relative(1e-9)),
            ([&table, &g_direct, &slope_typed], SLOPE, &relative(1e-10)),
        ];
        let (mut caught, mut largest) = ([0; 5], [0.0; 5]);
        for (k, (at, planted, looser)) in plants.into_iter().enumerate() {
            let (missed, _) = g_missed(&cases, at, as_is, g_rule);
            assert_eq!(
                missed.iter().sum::<usize>(),
                missed[planted],
                "plant {k} moves only what it plants"
            );
            caught[k] = missed[planted];
            largest[k] = g_missed(&cases, at, as_is, relative(1.0)).1[planted];
            let (near_miss, _) = g_missed(&cases, at, as_is, looser);
            assert_eq!(near_miss[planted], 0, "plant {k} passes the looser gate");
        }
        eprintln!(
            "G plants missing (tail, G itself at 0.0025, the ln sin θ split, 8 nodes, 11 \
             digits): {caught:?}, largest errors of G {:?}",
            largest.map(|x: f64| format!("{x:.1e}"))
        );
        assert!(caught.iter().all(|&m| m > 0), "{caught:?}");
    }

    /// **The compensated sum keeps what rounding drops**: `1`, then a thousand `2⁻⁶⁰` (each below
    /// half an ulp of 1, so a plain sum drops every one), then `−1`, sums to `1000 · 2⁻⁶⁰`
    /// exactly, in either order; a plain sum gives 0 (asserted).
    #[test]
    fn the_compensated_sum_keeps_what_rounding_drops() {
        let tiny = 2.0_f64.powi(-60);
        let terms = || {
            std::iter::once(1.0)
                .chain(std::iter::repeat_n(tiny, 1000))
                .chain(std::iter::once(-1.0))
        };
        assert_eq!(compensated(terms()), 1000.0 * tiny);
        let reversed: Vec<f64> = terms().collect::<Vec<_>>().into_iter().rev().collect();
        assert_eq!(compensated(reversed), 1000.0 * tiny);
        assert_eq!(terms().sum::<f64>(), 0.0);
        assert_eq!(compensated(std::iter::empty()), 0.0);
    }

    // ---------------------------------------------------------------- the panels

    /// The panels the definition's quadrature is held on, in half-widths (`b = 0.05`): at depths
    /// from a third of the width to a thousand widths, panels over the point, starting at it,
    /// ending just short of it, a thousandth of a width long on it, either side of the separated
    /// form's threshold, beside it, and far out (to 2000 widths, where the near form's terms
    /// cancel to `1e4` times the value and more); two on the bare half-space. The bodies turn
    /// with the cases, `ν` from 0 to 1/2.
    fn panel_cases() -> Vec<(Panel, HalfSpace)> {
        let b = 0.05;
        let bodies = [
            (206_000.0, 0.3),
            (96_500.0, 0.32),
            (190_000.0, 0.29),
            (1.0, 0.5),
            (200_000.0, 0.0),
        ];
        let spans = [
            (-0.2, 0.2),
            (0.0, 2.0),
            (-1.2, 1e-3),
            (-1e-3, 1e-3),
            (0.4999, 0.9),
            (0.5, 0.9),
            (-0.9, -0.5),
            (1.1, 1.8),
            (-4.06, -4.01),
            (21.7, 35.9),
            (-153.2, -152.9),
            (-693.0, -691.0),
            (-2000.0, -1980.0),
        ];
        let depths = [Some(0.3), Some(1.0), Some(4.0), Some(30.0), Some(1000.0)];
        let cases = depths
            .into_iter()
            .flat_map(|d| spans.map(|s| (s, d)))
            .chain([((-2.0, 2.0), None), ((0.5, 3.0), None)]);
        cases
            .enumerate()
            .map(|(k, ((from, to), depth))| {
                let (e, nu) = bodies[k % bodies.len()];
                let panel = Panel::new(from * b, to * b, b, depth.map(|h: f64| h * b));
                (
                    panel.expect("a panel"),
                    HalfSpace::new(e, nu).expect("a half-space"),
                )
            })
            .collect()
    }

    /// The short panels a graded solve asks for, which the grid above misses: panels on one side
    /// of the point, the nearer end `0.02 … 0.45 b` from it and `3e-5 … 1 b` long, at depths from
    /// a third of the width to three hundred widths, alternately either side; panels on the point
    /// `3e-5 b` long (a graded end at `N` 96) and `5.7e-4 b` (at `N` 48); one of them bare, with a
    /// bare panel short beside the point and two far out; and the panel of `s0bb` at `N` 96 that
    /// the prototype's near form errs `7.8e-11` of scale on (`b 0.0593`, `h 0.520`).
    fn short_cases() -> Vec<(Panel, HalfSpace)> {
        let b = 0.05;
        let steel = steel();
        let mut out = vec![(
            Panel::new(0.025_126_7, 0.025_128_8, 0.0593, Some(0.520)).expect("a panel"),
            steel,
        )];
        let mut side = 1.0;
        for depth in [0.3, 3.0, 30.0, 300.0] {
            for nearer in [0.02, 0.2, 0.45] {
                for length in [3e-5, 1e-3, 0.1, 1.0] {
                    let (from, to) = if side > 0.0 {
                        (nearer, nearer + length)
                    } else {
                        (-nearer - length, -nearer)
                    };
                    side = -side;
                    let panel = Panel::new(from * b, to * b, b, Some(depth * b));
                    out.push((panel.expect("a panel"), steel));
                }
            }
        }
        for (length, depth) in [
            (3e-5, Some(0.3)),
            (3e-5, Some(8.8)),
            (3e-5, Some(300.0)),
            (5.7e-4, Some(0.3)),
            (5.7e-4, Some(8.8)),
            (3e-5, None),
        ] {
            let panel = Panel::new(-length / 2.0 * b, length / 2.0 * b, b, depth.map(|h| h * b));
            out.push((panel.expect("a panel"), steel));
        }
        for (from, to) in [(0.2, 0.2 + 1e-3), (100.0, 100.001), (-3000.2, -3000.0)] {
            out.push((
                Panel::new(from * b, to * b, b, None).expect("a panel"),
                steel,
            ));
        }
        out
    }

    /// The gate: a panel within `1e-13` of its form's scale ([`Terms::scale`]) of the
    /// definition's quadrature.
    const PANEL_GATE: f64 = 1e-13;

    /// The prototype's near form, and this port's before it: the surface as `G(η₂/b) − G(η₁/b)`
    /// and the depth term as differences of `asinh` and of `η/R`, on one side of the point as
    /// across it.
    fn near_by_differences(kernel: &Kernel, p: &Panel, body: HalfSpace) -> f64 {
        let (b, nu, from, to) = (p.half_width, body.poisson, p.from, p.to);
        let surface = body.surface() * (kernel.g(to / b) - kernel.g(from / b));
        p.depth.map_or(surface, |h| {
            let depth: f64 = kernel
                .rule(width_rule(b, h))
                .iter()
                .map(|&(t, w)| {
                    let x = b * t;
                    let r2 = x * x + h * h;
                    let rho = r2.sqrt();
                    let (d1, d2) = ((r2 + from * from).sqrt(), (r2 + to * to).sqrt());
                    w * (2.0 * (1.0 - nu) * ((to / rho).asinh() - (from / rho).asinh())
                        + h * h * (to / (r2 * d2) - from / (r2 * d1)))
                })
                .sum();
            surface - body.interior() * depth
        })
    }

    /// The prototype's `panel`: the separated form where the port takes it, the near form by
    /// differences elsewhere.
    fn prototype_panel(kernel: &Kernel, p: &Panel, body: HalfSpace) -> f64 {
        match p.route() {
            Route::Separated => kernel.panel(p, body),
            Route::Near => near_by_differences(kernel, p, body),
        }
    }

    /// Whether the surface of `p` takes `∫ G′` ([`Kernel::surface`]'s rule).
    fn by_slope(p: &Panel) -> bool {
        let k = (p.from + p.to).abs() / (p.to - p.from);
        p.from * p.to > 0.0 && nodes(k + (k * k - 1.0).sqrt(), SLOPE_TOL) <= SLOPE_NODES
    }

    /// **Every panel is its definition's quadrature**: on the 67 panels of [`panel_cases`] and
    /// the 58 short ones a graded solve asks for ([`short_cases`]), the kernel's value is the
    /// double-double quadrature of the definition to `1e-13` of the scale of the form it takes
    /// (the porting plan's gate), and the reference at steps 1/64 and 1/128 agrees with itself
    /// to `1e-20` of it. The tolerance's two laws: a value a few roundings off passes, ten times
    /// the gate off misses on every panel. The short panels' surfaces take both of its forms
    /// (counted).
    ///
    /// Plants, each missing some panel: the prototype's `panel` (the near form by differences
    /// of `G` and of `asinh`), which the short panels fail and every panel of the first grid
    /// passes (asserted: the grid this gate had); version 1's port (the near form by
    /// differences everywhere), which the far panels' cancellation fails; round six's `G` table
    /// (step 0.02); round six's width rule (`1e-10`) on the near panels.
    #[test]
    fn every_panel_is_its_definitions_quadrature() {
        let kernel = Kernel::shared();
        let coarse = Kernel::with_step(0.02);
        let (mut worst, mut self_worst, mut cancels) = (0.0_f64, 0.0_f64, 0.0_f64);
        let (mut compared, mut rounded, mut tenfold) = (0, 0, 0);
        let (mut v1_missed, mut six_missed, mut rule_missed) = (0, 0, 0);
        let (mut prototype_missed, mut prototype_grid) = (0, 0);
        let mut forms = (0, 0);
        let grid = panel_cases();
        let first = grid.len();
        for (k, (p, body)) in grid.into_iter().chain(short_cases()).enumerate() {
            let terms = reference_terms(&p, body, 1.0 / 128.0);
            let (fine, scale) = (terms.value(), terms.scale(p.route()));
            let half = reference_terms(&p, body, 1.0 / 64.0).value();
            self_worst = self_worst.max((fine - half).abs().to_f64() / scale);
            let near_scale = terms.scale(Route::Near);
            cancels = cancels.max(near_scale / fine.abs().to_f64());
            let off = |v: f64| (Wide::of(v) - fine).abs().to_f64() / scale;
            let port = kernel.panel(&p, body);
            worst = worst.max(off(port));
            compared += 1;
            rounded += usize::from(off(port * (1.0 + 4.0 * EPS)) <= PANEL_GATE);
            tenfold += usize::from(off(port + 10.0 * PANEL_GATE * scale) > PANEL_GATE);
            v1_missed += usize::from(off(near_by_differences(kernel, &p, body)) > PANEL_GATE);
            let prototype = usize::from(off(prototype_panel(kernel, &p, body)) > PANEL_GATE);
            prototype_missed += prototype;
            prototype_grid += if k < first { prototype } else { 0 };
            six_missed += usize::from(off(coarse.panel(&p, body)) > PANEL_GATE);
            if k >= first {
                if by_slope(&p) {
                    forms.0 += 1;
                } else {
                    forms.1 += 1;
                }
            }
            if let (Some(h), Route::Near) = (p.depth, p.route()) {
                let r = h / p.half_width;
                let n = nodes(r + (1.0 + r * r).sqrt(), 1e-10);
                let surface = body.surface() * kernel.surface(p.from, p.to, p.half_width);
                let v = surface
                    - body.interior()
                        * depth_term(p.from, p.to, p.half_width, h, body.poisson, kernel.rule(n));
                rule_missed += usize::from(off(v) > PANEL_GATE);
            }
        }
        eprintln!(
            "{compared} panels against the double-double reference: worst {worst:.2e} of scale \
             (the reference's own step change {self_worst:.1e}; the near form's terms reach \
             {cancels:.1e} times the value); the short panels' surfaces by G′ and by G: \
             {forms:?}; plants missing: the prototype's panel {prototype_missed} ({prototype_grid} \
             of the first grid), v1 {v1_missed}, step 0.02 {six_missed}, width 1e-10 \
             {rule_missed}"
        );
        assert_eq!((first, compared), (67, 125));
        assert!(
            self_worst < 1e-20,
            "the reference has not converged: {self_worst}"
        );
        assert!(worst <= PANEL_GATE, "worst {worst}");
        assert_eq!((rounded, tenfold), (125, 125));
        assert!(forms.0 > 0 && forms.1 > 0, "{forms:?}");
        assert_eq!(
            prototype_grid, 0,
            "the prototype's panel passes the first grid"
        );
        assert!(prototype_missed > 0 && v1_missed > 0 && six_missed > 0 && rule_missed > 0);
    }

    /// `∫ G′` along the panel `[from, to]` (in half-widths `b`) by the `n`-node Gauss–Legendre
    /// rule, as the kernel's surface takes it.
    fn along(from: f64, to: f64, b: f64, n: usize) -> f64 {
        let (mid, half) = ((from + to) / (2.0 * b), (to - from) / (2.0 * b));
        let sum: f64 = gauss_legendre_rule(n)
            .iter()
            .map(|&(t, w)| w * slope(mid + half * t))
            .sum();
        half * sum
    }

    /// **The surface's two forms agree at their switch**: on a panel on one side of the point
    /// where [`SLOPE_NODES`] nodes just reach [`SLOPE_TOL`] (`E = SLOPE_TOL^{−1/(2(n − 1))}`,
    /// `n` the most nodes), the rule and `G`'s difference both equal the double-double
    /// quadrature of the surface term to the panel gate of it, over nearer ends from `1e-6` to
    /// `1e3` half-widths either side. The kernel takes the rule there at its most nodes, the
    /// difference on a panel `1e-9` longer, and on a panel a millionth as long the rule at the
    /// count its own ellipse gives, each compared to the bit. Plant: the rule three nodes short
    /// at the switch, which misses.
    #[test]
    fn the_surface_forms_agree_at_their_switch() {
        let kernel = Kernel::shared();
        let body = steel();
        let b = 0.05;
        let edge = SLOPE_TOL.powf(-1.0 / (2.0 * (SLOPE_NODES as f64 - 1.0)));
        let k = 0.5 * (edge + 1.0 / edge);
        let ratio = (k + 1.0) / (k - 1.0);
        let (mut worst, mut ran, mut short) = (0.0_f64, 0, 0);
        for nearer in [1e-6, 1e-3, 0.1, 0.45, 3.0, 1e3] {
            for side in [1.0, -1.0] {
                let (a, c) = (nearer * b, nearer * b * ratio * (1.0 - 1e-12));
                let (from, to) = if side > 0.0 { (a, c) } else { (-c, -a) };
                let p = Panel::new(from, to, b, None).expect("a panel");
                assert!(by_slope(&p), "{p:?}");
                let longer = if side > 0.0 {
                    Panel::new(from, to * (1.0 + 1e-9), b, None)
                } else {
                    Panel::new(from * (1.0 + 1e-9), to, b, None)
                };
                let longer = longer.expect("a panel");
                assert!(!by_slope(&longer));
                let reference = reference_terms(&p, body, 1.0 / 64.0).surface;
                let scale = reference.abs().to_f64();
                let off =
                    |v: f64| (Wide::of(body.surface() * v) - reference).abs().to_f64() / scale;
                let rule = kernel.surface(from, to, b);
                let difference = kernel.g(to / b) - kernel.g(from / b);
                assert_eq!(rule, along(from, to, b, SLOPE_NODES), "{p:?}");
                let (e1, e2) = (longer.from, longer.to);
                let taken = kernel.surface(e1, e2, b);
                assert_eq!(taken, kernel.g(e2 / b) - kernel.g(e1 / b), "{longer:?}");
                let (s1, s2) = (from, from + (to - from) * 1e-6);
                let k_short = (s1 + s2).abs() / (s2 - s1);
                let n = nodes(k_short + (k_short * k_short - 1.0).sqrt(), SLOPE_TOL);
                assert_eq!(kernel.surface(s1, s2, b), along(s1, s2, b, n), "{s1} {s2}");
                assert!(off(rule) <= PANEL_GATE, "{p:?}: {:e}", off(rule));
                assert!(
                    off(difference) <= PANEL_GATE,
                    "{p:?}: {:e}",
                    off(difference)
                );
                worst = worst.max(off(rule)).max(off(difference));
                ran += 1;
                short += usize::from(off(along(from, to, b, SLOPE_NODES - 3)) > PANEL_GATE);
            }
        }
        eprintln!(
            "the surface's switch (r₂/r₁ = {ratio:.3}): {ran} panels, worst {worst:.2e} of the \
             surface; three nodes short misses {short}"
        );
        assert_eq!(ran, 12);
        assert!(short > 0);
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
        let residual = surface - depth - body.line_limit(b, h).expect("a strip") - tail;
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
                (kernel.panel(&p, body)
                    - body.line_limit(b, h).expect("a strip")
                    - line_tail(b, h, body))
                .abs(),
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

    /// **Deep in the tooth, the line limit is Weber and Banaschek's**: for `b ≪ h` the plane's
    /// depth-referenced line compliance is the classical `2(1 − ν²)/(π E) [ln(2h/b) − ν/(2(1 −
    /// ν))]` (Weber and Banaschek, 1953; the contact term of the tooth-stiffness literature),
    /// derived apart from the kernel. The line limit's exact form exceeds it by
    /// `2(1 − ν²)/(π E) (b/h)² [1/4 + ν/(8(1 − ν))]` (from `asinh(1/ε) = ln(2/ε) + ε²/4 + O(ε⁴)`
    /// and `1/(√(1 + ε²) + 1) = 1/2 − ε²/8 + O(ε⁴)`, `ε = b/h`), to a remainder below
    /// `0.2 ε⁴` of the factor (`3/32 + ν/(16(1 − ν))`, `ν ≤ 1/2`) and the roundings of the two
    /// values: on five bodies (`ν` 0 to 1/2) at `b/h` from 0.1 to 0.003.
    ///
    /// Plants: `ν/(1 − ν)` read as `ν`, which misses everywhere but at `ν = 0`; the exact tail
    /// `h/(√(h² + b²) + h)` taken at its deep limit `1/2`, which misses wherever `ν > 0` and
    /// passes a check that the two agree to `1e-3` (asserted).
    #[test]
    fn the_line_limit_deep_in_the_tooth_is_weber_and_banascheks() {
        let bodies = [
            (206_000.0, 0.3),
            (96_500.0, 0.32),
            (190_000.0, 0.29),
            (1.0, 0.5),
            (200_000.0, 0.0),
        ];
        let limit = |b: f64, h: f64, nu: f64, e: f64, tail: fn(f64, f64) -> f64, ratio: f64| {
            2.0 * (1.0 - nu * nu) / (PI * e) * ((h / b).asinh() - ratio * tail(b, h))
        };
        let exact: fn(f64, f64) -> f64 = |b, h| h / (h.hypot(b) + h);
        let deep: fn(f64, f64) -> f64 = |_, _| 0.5;
        let (mut ran, mut slip, mut halved, mut loose) = (0, 0, 0, 0);
        for (e, nu) in bodies {
            let body = HalfSpace::new(e, nu).expect("a half-space");
            let k = 2.0 * (1.0 - nu * nu) / (PI * e);
            let coefficient = 0.25 + nu / (8.0 * (1.0 - nu));
            for eps in [0.1, 0.03, 0.01, 0.003] {
                let (b, h): (f64, f64) = (eps, 1.0);
                let classical = k * ((2.0 * h / b).ln() - nu / (2.0 * (1.0 - nu)));
                let held = |line: f64| {
                    let excess = (line - classical) / (k * eps * eps);
                    let rounding = 16.0 * EPS * (line.abs() + classical.abs()) / (k * eps * eps);
                    (excess - coefficient).abs() <= 0.2 * eps * eps + rounding
                };
                let line = body.line_limit(b, h).expect("a strip");
                let same = limit(b, h, nu, e, exact, nu / (1.0 - nu));
                assert!((line - same).abs() <= 4.0 * EPS * line.abs());
                assert!(held(line), "{e} {nu} {eps}");
                ran += 1;
                slip += usize::from(!held(limit(b, h, nu, e, exact, nu)));
                let half = limit(b, h, nu, e, deep, nu / (1.0 - nu));
                halved += usize::from(!held(half));
                loose += usize::from((half - line).abs() > 1e-3 * line.abs());
            }
        }
        eprintln!("Weber–Banaschek: {ran} strips; plants missing: ν for ν/(1 − ν) {slip}, the deep tail {halved}");
        assert_eq!(ran, 20);
        assert_eq!((slip, halved, loose), (16, 16, 0));
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
            let terms = reference_terms(&p, steel(), 1.0 / 64.0);
            let reference = terms.asinh + terms.h2;
            let n = nodes_of(&p).expect("a depth");
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

    /// A line's approach error `e(N)` at `N` 16, 32, 64 and 128, on the two curvature ratios.
    fn errors(line: Line) -> [[f64; 4]; 2] {
        [(0.05, 0.5), (0.2, 0.4)]
            .map(|ratio| [16, 32, 64, 128].map(|n| ellipse_error(n, ratio, line)))
    }

    /// The orders each doubling shows, `log₂(e(N)/e(2N))`, all rounding to 2 (half an order
    /// either side is the half-way to the neighbouring orders), and the error of one sign.
    fn second_order(e: &[f64; 4]) -> bool {
        e.windows(2)
            .all(|w| ((w[0] / w[1]).log2() - 2.0).abs() < 0.5)
            && e.iter().all(|x| x.signum() == e[0].signum())
    }

    /// Richardson's extrapolations `R(N) = (4 e(2N) − e(N))/3`, `N` 16, 32, 64: what is left
    /// once the `N⁻²` term is taken out.
    fn richardson(e: &[f64; 4]) -> [f64; 3] {
        [0, 1, 2].map(|k| (4.0 * e[k + 1] - e[k]) / 3.0)
    }

    /// Whether the limit is Hertz's: `R(N)` keeps one sign and at least halves at each doubling,
    /// so the remainder it carries vanishes at least at first order. A limit `ε` off Hertz's
    /// makes `R(N)` settle on `ε` rather than on 0: it stalls, or crosses zero.
    fn at_hertz(e: &[f64; 4]) -> bool {
        let r = richardson(e);
        r.iter().all(|x| x.signum() == r[0].signum())
            && r.windows(2).all(|w| w[0].abs() >= 2.0 * w[1].abs())
    }

    /// **The Hertz ellipse at O(N⁻²), in the limit**: the line solve through the kernel
    /// reproduces the Hertz ellipse, its approach converging at second order in the panel count
    /// ([`second_order`], on each doubling from 16 to 128) to Hertz's approach itself
    /// ([`at_hertz`]: Richardson's extrapolation converges on zero), on two curvature ratios.
    ///
    /// Plants: the shape factor left out (the strips as wide as a line's), `2e-4` high, `1e-5`
    /// high and low, and `2e-5` high, each a wrong limit, and the stations at their panels'
    /// starts, which converges at first order on the coarse grids. The shape factor `1e-5` and
    /// `2e-5` off keeps second order (asserted: the gate before the limit passed them); the
    /// limit catches them. The resolution in `C` lies between `2e-6` (asserted to
    /// pass: its limit, `≈ 6e-7`, is below the extrapolations' own change at `N` 128) and `1e-5`.
    #[test]
    fn the_hertz_ellipse_converges_at_second_order() {
        let gate = |e: &[[f64; 4]; 2]| e.iter().all(|e| second_order(e) && at_hertz(e));
        let port = errors(THE_LINE);
        let list = |v: &[f64]| v.iter().map(|x| format!("{x:.3e}")).collect::<Vec<_>>();
        let seen = |e: &[[f64; 4]; 2]| e.map(|e| (list(&e), list(&richardson(&e))));
        eprintln!("ellipse: (e(N), R(N)) {:?}", seen(&port));
        assert!(gate(&port), "{port:?}");

        // (the plant, whether the second-order gate alone passes it)
        let plants: [(Line, bool); 6] = [
            (
                Line {
                    shape: |_| 1.0,
                    ..THE_LINE
                },
                false,
            ),
            (
                Line {
                    shape: |q| 1.0002 * shape_factor(q).expect("C"),
                    ..THE_LINE
                },
                false,
            ),
            (
                Line {
                    shape: |q| (1.0 + 1e-5) * shape_factor(q).expect("C"),
                    ..THE_LINE
                },
                true,
            ),
            (
                Line {
                    shape: |q| (1.0 - 1e-5) * shape_factor(q).expect("C"),
                    ..THE_LINE
                },
                true,
            ),
            (
                Line {
                    shape: |q| (1.0 + 2e-5) * shape_factor(q).expect("C"),
                    ..THE_LINE
                },
                true,
            ),
            (
                Line {
                    station: -0.5,
                    ..THE_LINE
                },
                false,
            ),
        ];
        let mut failed = 0;
        for (k, (plant, keeps_order)) in plants.into_iter().enumerate() {
            let e = errors(plant);
            eprintln!("plant {k}: (e(N), R(N)) {:?}", seen(&e));
            assert!(!gate(&e), "plant {k} passes");
            failed += 1;
            if keeps_order {
                assert!(
                    e.iter().all(second_order),
                    "plant {k}: the second-order gate alone passes it"
                );
            }
        }
        assert_eq!(failed, 6);
        let fine = errors(Line {
            shape: |q| (1.0 + 2e-6) * shape_factor(q).expect("C"),
            ..THE_LINE
        });
        assert!(gate(&fine), "C 2e-6 high is below the resolution: {fine:?}");
    }

    // ---------------------------------------------------------------- G, and the shape factor

    /// **`G`'s table is its quadrature**: at every cell's middle across the table, where cubic
    /// Hermite errs most, the table equals [`g_direct`] to the Hermite bound and both sides'
    /// rounding, relative to `G`; at each end it meets its outer form to rounding (`8ε` of `G`,
    /// both forms exact there); `G` is odd and `G(0) = 0`, and `G′` is `None` at 0. The Hermite
    /// bound's `max|∂⁴F/∂u⁴ / F|` ([`F4_REL`]) bounds every third difference of the table's
    /// exact slope `dF/du` over `step³` (each is `∂⁴F/∂u⁴` somewhere in its three cells) over
    /// `F`, and is within 5 % of the largest. `G′` beyond the ends is the outer forms' slopes,
    /// which meet the closed form at each end to its roundings (`16ε`; the far slope's
    /// `1/(8 r³)` is `8.6e-14` of it there). Plant: `F` at twice the step, `0.0025`, which a
    /// `1e-13` gate relative to `G` passes (asserted).
    #[test]
    fn the_table_is_g() {
        let midpoints = |k: &Kernel| -> (usize, f64, f64) {
            let cells = k.table.len() - 1;
            let (mut worst, mut largest) = (0.0_f64, 0.0_f64);
            for i in 0..cells {
                let r = (TABLE_FROM + (i as f64 + 0.5) * k.step).exp();
                let (g, direct) = (k.g(r), g_direct(r));
                let bound = (hermite(TABLE_STEP) + table_rounding(r)) * g.abs()
                    + 2.0 * direct_rounding(direct);
                worst = worst.max((g - direct).abs() / bound);
                largest = largest.max((g - direct).abs() / direct.abs());
            }
            (cells, worst, largest)
        };
        let kernel = Kernel::shared();
        let (cells, worst, largest) = midpoints(kernel);
        eprintln!("G: {cells} cells, worst {worst:.3} of the bound ({largest:.1e} of G)");
        assert_eq!(cells, 33_600);
        assert!(worst <= 1.0, "{worst}");
        let (_, twice, twice_largest) = midpoints(&Kernel::with_step(2.0 * TABLE_STEP));
        eprintln!("twice the step: worst {twice:.1e} of the bound ({twice_largest:.1e} of G)");
        assert!(twice > 1.0 && twice_largest < 1e-13);

        let third = kernel
            .table
            .windows(4)
            .map(|w| {
                let d3 = w[3].1 - 3.0 * w[2].1 + 3.0 * w[1].1 - w[0].1;
                (d3 / kernel.step.powi(3) / w[1].0.min(w[2].0)).abs()
            })
            .fold(0.0_f64, f64::max);
        eprintln!("max |∂⁴F/∂u⁴ / F| over the table's third differences: {third:.4}");
        assert!(third <= F4_REL && F4_REL <= 1.05 * third, "{third}");

        for end in [TABLE_FROM, TABLE_TO] {
            let r = end.exp();
            for x in [r.next_down(), r, r.next_up()] {
                let g = kernel.g(x);
                assert!((g - g_direct(x)).abs() <= 8.0 * EPS * g.abs(), "{x:e}");
                let closed = g_slope(x).expect("G′");
                assert!(
                    (slope(x) - closed).abs() <= 16.0 * EPS * closed,
                    "G′({x:e})"
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

    /// `C` read as its closed form at an aspect `κ`: `√(3/(κ² R_D(0, 1, κ²)))`.
    fn c_at(kappa: f64) -> f64 {
        let s = kappa * kappa;
        (3.0 / (s * r_d(0.0, 1.0, s).expect("R_D"))).sqrt()
    }

    /// **The shape factor is the Hertz ellipse's peak over the line's**, by its definition and
    /// through `hertz.rs`'s elliptical contact rather than `C`'s closed form: for curvatures
    /// `k_L = q k_A`, the ellipse's peak `p₀` and its semi-axis across, `c`, carry a line load
    /// at the centre `q₀ = (π/2) p₀ c`, which a line contact of curvature `k_A` presses with
    /// `p_line = √(q₀ E* k_A/π)`; `C = p₀/p_line`. The two readings share the aspect `κ`, and
    /// part by exactly `√(q/g(κ))`, `g(κ) = R_D(κ², 0, 1)/R_D(1, 0, κ²)` the curvature ratio
    /// `κ` belongs to, which the aspect's root makes `q` to its stopping tolerance. The gate is
    /// ten times that part, as the root left it at each `q`, and both readings' roundings
    /// (`32 ε`), on `q` from `1e-16` to 1.
    ///
    /// Plants: `C` at an aspect `1e-11` off (a root stopped early), which misses wherever `C`
    /// moves with `κ` and passes the ellipse gate's reach (`1e-6`, asserted); the switch to 1
    /// at `s = 1e-12` rather than [`SHAPE_ONE`], which misses where `s` lies between.
    #[test]
    fn the_shape_factor_is_the_ellipses_peak_over_the_lines() {
        let (load, e_star, k_a) = (1000.0, 113_000.0, 0.5);
        let qs: Vec<f64> = (0..=32)
            .map(|k| 10f64.powf(-f64::from(k) / 2.0))
            .chain((1..20).map(|k| f64::from(k) / 20.0))
            .collect();
        let (mut ran, mut worst, mut early, mut switch, mut reach) = (0, 0.0_f64, 0, 0, 0);
        for &q in &qs {
            let patch = elliptical_contact(q * k_a, k_a, load, e_star).expect("an ellipse");
            let (p0, across) = (patch.max_pressure, patch.semi_y);
            let centre = FRAC_PI_2 * p0 * across;
            let hertz = p0 / (centre * e_star * k_a / PI).sqrt();
            let kappa = patch_aspect(q, 1.0).expect("an aspect");
            let k2 = kappa * kappa;
            let g = r_d(k2, 0.0, 1.0).expect("R_D") / r_d(1.0, 0.0, k2).expect("R_D");
            let c = shape_factor(q).expect("q in [0, 1]");
            let bound = 10.0 * (0.5 * (q / g).ln().abs() + 32.0 * EPS) * c;
            assert!((hertz - c).abs() <= bound, "q {q}: {hertz} vs {c}");
            worst = worst.max((hertz - c).abs() / bound);
            ran += 1;
            let planted = c_at(kappa * (1.0 + 1e-11));
            early += usize::from((hertz - planted).abs() > bound);
            reach += usize::from((hertz - planted).abs() > 1e-6 * c);
            let switched = if k2 < 1e-12 { 1.0 } else { c };
            switch += usize::from((hertz - switched).abs() > bound);
        }
        eprintln!(
            "C by the ellipse: {ran} ratios, worst {worst:.3} of the bound; plants missing: an \
             aspect 1e-11 off {early}, the switch at 1e-12 {switch}"
        );
        assert_eq!(ran, 52);
        assert!(early > 0 && switch > 0 && reach == 0);
    }

    /// What is no panel and no half-space is refused: a panel of no length or reversed (the
    /// separated form's `D/d_R` is `0/0` on one of no length), an end not a number or not finite
    /// in half-widths, a width not a finite length `> 0`, or a depth not one (`None` is the bare
    /// half-space, not a depth of `∞`); a modulus not a finite positive one, a Poisson's ratio
    /// outside `(−1, 1/2]`; a line limit on a width or depth not a finite length `> 0`. A
    /// half-space holds what it was given; a panel a rounding long is a panel, and finite.
    #[test]
    fn what_is_no_panel_is_refused() {
        let good = Panel::new(-0.1, 0.2, 0.05, Some(0.5));
        assert!(good.is_some() && Panel::new(-0.1, 0.2, 0.05, None).is_some());
        let refused = [
            (0.045, 0.045, 0.05, Some(0.2)),
            (0.045, 0.045, 0.05, None),
            (0.0, 0.0, 0.05, Some(0.2)),
            (0.2, -0.1, 0.05, Some(0.5)),
            (1e300, 2e300, 1e-10, Some(0.5)),
            (-1e300, 0.1, 1e-10, None),
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
        let kernel = Kernel::shared();
        for (from, h) in [
            (0.045, Some(0.2)),
            (0.045, None),
            (0.0, Some(0.2)),
            (-0.01, None),
        ] {
            let p = Panel::new(from, from.next_up(), 0.05, h).expect("a panel");
            assert!(kernel.panel(&p, steel()).is_finite(), "{p:?}");
        }
        for (e, nu) in [(206_000.0, 0.5), (1.0, -0.99), (96_500.0, 0.32)] {
            let body = HalfSpace::new(e, nu).expect("a half-space");
            assert_eq!((body.modulus(), body.poisson()), (e, nu));
        }
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
        let body = steel();
        assert!(body.line_limit(0.05, 0.5).is_some());
        let no_strip = [
            (0.0, 0.5),
            (-0.05, 0.5),
            (f64::NAN, 0.5),
            (f64::INFINITY, 0.5),
            (0.05, 0.0),
            (0.05, -0.5),
            (0.05, f64::INFINITY),
            (0.05, f64::NAN),
        ];
        for (b, h) in no_strip {
            assert_eq!(body.line_limit(b, h), None, "{b} {h}");
        }
    }
}
