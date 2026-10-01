//! Across the line: the frictionless contact of one strip, the section of the two bodies normal
//! to the contact line at a station, under a line load `q`; and the friction a panel of the line
//! carries.
//!
//! **The section.** The gap across the line is `h(t)`, `t` the across coordinate from the
//! section's minimum (`h(0) = h′(0) = 0`), and its curvature `h″` is piecewise constant: the two
//! flanks' relative curvature `k₀` plus each form's graph curvature where it lies (a round, a tip
//! relief), each a [`CurvatureStep`]. Where the curvature changes is a knot.
//!
//! **The strip** ([`Strip`]). The plane-strain contact of the two half-planes with that gap,
//! modulus `E*`, bounded at both ends, has half-width `c` and centre `m` from two equations in
//! `t = m + c cos φ` (Muskhelishvili; Johnson, *Contact Mechanics*, section 5.7):
//!
//! ```text
//! I₀ = ∫₀^π h′(m + c cos φ) dφ = 0,      q = (E*/2) c I₁,   I₁ = ∫₀^π h′(m + c cos φ) cos φ dφ,
//! p(θ) = (E*/2π) ∫₀^π h′(m + c cos φ) [cot((θ − φ)/2) + cot((θ + φ)/2)] dφ.
//! ```
//!
//! `h′` is linear between knots, so both integrals are closed forms piece by piece, and the
//! pressure, integrated by parts at each knot (where `h′` is continuous), is
//!
//! ```text
//! p(θ) = (E* c/2) [ sin θ (k_π + Σ Δk_j φ_j/π) + (1/π) Σ Δk_j (cos θ − cos φ_j) ln|sin((θ + φ_j)/2) / sin((θ − φ_j)/2)| ]
//! ```
//!
//! over the knots `t_j = m + c cos φ_j` inside the contact, `Δk_j` the curvature's step there
//! (above `t_j` less below), `k_π` the curvature at the end `t = m − c`. With no knot inside it is
//! Hertz, `p₀ sin θ`; a knot at an end adds nothing, so the pressure is continuous as a knot
//! enters; at a knot it has an `x ln x` kink and no singularity. The load on `θ ∈ [0, θ_a]`
//! (`t ≥ m + c cos θ_a`) is a closed form of the same terms ([`Strip::load_to`]).
//!
//! **The report** ([`StripReport`]) reads the pressure's maximum over each region of the strip —
//! the flank, where neither member is on its edge, and the edge beyond it — and the share of the
//! load on the edge. The pressure is analytic between knots, so each region is searched knot to
//! knot ([`PIECE_NODES`] cosine-graded samples, every sampled local maximum refined by golden
//! section to the angle's own resolution).
//!
//! **Creep** ([`Strip::creep`]). In steady rolling of like materials the stick zone `[d, x_l]` at
//! the leading edge carries a corrective traction that solves the normal problem's equation on
//! that zone, so the creepage is `ξ = μ I₀′/π` and the traction `Q = μP − μ(E*/2) c′ I₁′`, the
//! zone's own `I₀′` and `I₁′` (Johnson, section 8.2, generalised): the exact curve for any piecewise
//! quadratic strip, which is Carter's `Q/μP = 1 − (1 − ξ/ξ*)²` on Hertz.
//!
//! **A panel's friction** ([`panel_slide`], [`carter_slide`]): the means over a panel of the
//! traction's direction and of the friction power, for sliding affine along the panel, Coulomb's
//! and Carter's, in closed form without cancellation.
//!
//! Ports the prototype's `trace._pieces_of`, `_segments`, `_I`, `_conj`, `_strip`, `contact2d`,
//! `strip_report`, `max_pieces`, `panel_slide`, `_slide_moments` and `carter_slide`, and the exact
//! creep curve of its `t6_creep.py`; the records are `tests/data/field_oracle/across.json`. Every
//! length is relative: scaling every length by `k` and `q` by `k` leaves every pressure and share
//! unchanged.

use std::f64::consts::PI;

use crate::involute::inv_inverse;
use crate::solve::{brent, Tol};

/// Samples per piece of a region (knot to knot), cosine-graded towards both ends where the
/// pressure has its `x ln x` kinks; every sampled local maximum is then refined. A resolution:
/// the region maxima it gives equal a dense reference (4000 samples a region, refined) to
/// `1e-12` on 491 random strips (`region_maxima_are_a_dense_references`).
pub const PIECE_NODES: usize = 12;

/// The share of a golden-section bracket each round keeps, `1/φ`.
fn golden() -> f64 {
    (5.0_f64.sqrt() - 1.0) / 2.0
}

/// A step of the section's curvature: `dk` (1/mm) added where `from < t < to`; `None` is an end
/// that does not come (the step runs on to that side).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CurvatureStep {
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub dk: f64,
}

/// The gap across the line: `h(0) = h′(0) = 0` and `h″` piecewise constant.
#[derive(Clone, Debug, PartialEq)]
pub struct Profile {
    /// The knots in increasing `t`, `0` among them (where `h′` is anchored), each step's ends.
    knots: Vec<f64>,
    /// `h″` on each interval: `curvature[i]` between `knots[i − 1]` and `knots[i]`, the first
    /// below every knot and the last above.
    curvature: Vec<f64>,
    /// `h′` at each knot.
    slope: Vec<f64>,
}

impl Profile {
    /// The section of base curvature `k0` with `steps` added. `None` unless every number is
    /// finite and every step with both ends runs forward (`from < to`).
    pub fn new(k0: f64, steps: &[CurvatureStep]) -> Option<Self> {
        let finite_end = |e: Option<f64>| e.is_none_or(f64::is_finite);
        let sound = k0.is_finite()
            && steps.iter().all(|s| {
                s.dk.is_finite()
                    && finite_end(s.from)
                    && finite_end(s.to)
                    && s.from.zip(s.to).is_none_or(|(a, b)| a < b)
            });
        if !sound {
            return None;
        }
        let mut knots: Vec<f64> = std::iter::once(0.0)
            .chain(steps.iter().flat_map(|s| [s.from, s.to]).flatten())
            .collect();
        knots.sort_by(f64::total_cmp);
        knots.dedup();
        // Each interval's curvature, read at a point inside it.
        let inside = |i: usize| -> f64 {
            match i {
                0 => knots[0] - 1.0,
                i if i == knots.len() => knots[i - 1] + 1.0,
                i => 0.5 * (knots[i - 1] + knots[i]),
            }
        };
        let curvature: Vec<f64> = (0..=knots.len())
            .map(|i| {
                let t = inside(i);
                k0 + steps
                    .iter()
                    .filter(|s| s.from.is_none_or(|a| a < t) && s.to.is_none_or(|b| t < b))
                    .map(|s| s.dk)
                    .sum::<f64>()
            })
            .collect();
        let origin = knots.iter().position(|&t| t == 0.0)?;
        let mut slope = vec![0.0; knots.len()];
        for i in origin + 1..knots.len() {
            slope[i] = slope[i - 1] + curvature[i] * (knots[i] - knots[i - 1]);
        }
        for i in (0..origin).rev() {
            slope[i] = slope[i + 1] - curvature[i + 1] * (knots[i + 1] - knots[i]);
        }
        Some(Self {
            knots,
            curvature,
            slope,
        })
    }

    /// The interval `i`'s ends, `None` where it runs on.
    fn interval(&self, i: usize) -> (Option<f64>, Option<f64>) {
        let lo = i.checked_sub(1).map(|j| self.knots[j]);
        let hi = self.knots.get(i).copied();
        (lo, hi)
    }

    /// `h′ = a + k (t − t_r)` on interval `i`, `(t_r, a)` its right knot and the slope there (the
    /// last knot's for the interval above every knot).
    fn line_of(&self, i: usize) -> (f64, f64) {
        let r = i.min(self.knots.len() - 1);
        (self.knots[r], self.slope[r])
    }

    /// `h′(t)`.
    pub fn slope_at(&self, t: f64) -> f64 {
        let i = self.knots.partition_point(|&k| k < t);
        let (tr, a) = self.line_of(i);
        a + self.curvature[i] * (t - tr)
    }

    /// The knots where the curvature changes, with the change (above less below).
    fn changes(&self) -> impl Iterator<Item = (f64, f64)> + '_ {
        self.knots
            .iter()
            .enumerate()
            .map(|(j, &t)| (t, self.curvature[j + 1] - self.curvature[j]))
            .filter(|&(_, dk)| dk != 0.0)
    }

    /// The least and greatest curvature of the intervals meeting `(−r, r)`; `None` for `r` is
    /// every interval.
    fn curvature_within(&self, r: Option<f64>) -> (f64, f64) {
        let meets = |i: usize| {
            let (lo, hi) = self.interval(i);
            r.is_none_or(|r| lo.is_none_or(|a| a < r) && hi.is_none_or(|b| b > -r))
        };
        // The interval holding the origin meets every window.
        let origin = self.knots.partition_point(|&k| k <= 0.0);
        let k = self.curvature[origin];
        (0..self.curvature.len())
            .filter(|&i| meets(i))
            .map(|i| self.curvature[i])
            .fold((k, k), |(lo, hi), k| (lo.min(k), hi.max(k)))
    }

    /// `σ(t) = h′(t)/t`, `t ≠ 0`: the mean curvature from the origin to `t`, positive across the
    /// section's valley (where `h′` has the sign of `t`) and nowhere else.
    fn secant(&self, t: f64) -> f64 {
        self.slope_at(t) / t
    }

    /// The least and greatest of `σ` over `[−r, r]`, `None` for `r` the whole line. On each
    /// interval `h′ = α + k t`, so `σ = k + α/t` is monotone there and both are at a knot, at
    /// `±r`, or at the line's ends (where `σ` tends to the outer intervals' curvatures).
    fn secant_within(&self, r: Option<f64>) -> (f64, f64) {
        let ends = r.map_or([self.curvature[0], self.curvature[self.knots.len()]], |r| {
            [self.secant(-r), self.secant(r)]
        });
        self.knots
            .iter()
            .filter(|&&t| t != 0.0 && r.is_none_or(|r| t.abs() <= r))
            .map(|&t| self.secant(t))
            .fold(
                (ends[0].min(ends[1]), ends[0].max(ends[1])),
                |(lo, hi), s| (lo.min(s), hi.max(s)),
            )
    }

    /// `h′ = α + k t` on interval `i`: `(α, k)`.
    fn line_on(&self, i: usize) -> (f64, f64) {
        let (tr, a) = self.line_of(i);
        let k = self.curvature[i];
        (a - k * tr, k)
    }

    /// The lines `(α, k)` of `h′` just beyond `from` and just short of `−from` (`from ≥ 0`, a
    /// knot's distance from the origin or `0`): on the band from `from` to the next knot's
    /// distance, at `t` and at `−t`.
    fn band_lines(&self, from: f64) -> [(f64, f64); 2] {
        [
            self.line_on(self.knots.partition_point(|&k| k <= from)),
            self.line_on(self.knots.partition_point(|&k| k < -from)),
        ]
    }

    /// A bracket of the strip's half-width at load `q`, by construction, with the bound on its
    /// centre that holds across it ([`WidthBracket`]).
    ///
    /// At a root of `I₀`, `I₁ = ∫ h′(t) (t/c) dφ` (`t = m + c cos φ`; `I₀ = 0` adds `m/c` to
    /// `cos φ`), so the load is `(E*/2) ∫ σ(t) t² dφ`: where `σ ≥ σ₋` on the window it is at least
    /// `(πE*/4) σ₋ c²` (`∫ t² dφ = π (m² + c²/2)`), Hertz's at curvature `σ₋`, convex or not.
    /// And `m/c = −⟨cos φ⟩` weighted by `σ` (from `I₀ = ∫ σ t dφ = 0`), so where `σ` spans a
    /// ratio `ρ` on the window, `|m| ≤ μ c` ([`skew`]). So within a region `[−s, s]` of the
    /// valley, every half-width `c ≤ s/(1 + μ)` has its windows inside it, its centre in
    /// `[−μc, μc]` (`I₀` of the root's sign at both ends) and a load of at least
    /// `(πE*/4) σ₋ c²`, `σ₋` the least `σ` on `[−s, s]`. The least `s` whose `c = s/(1 + μ)`
    /// carries `q` so is `c_hi`: sought band by band between the knots' distances from the
    /// origin, `μ` from `σ` over the band's whole region, in closed form within each band
    /// (`s² σ(±s) = k s² ± α s`, a quadratic). Below it every window lies in `[−s, s]`, where
    /// the load is at most Hertz's at `K`, the greatest curvature there (`I₁ = c ∫ h″ sin² φ
    /// dφ`), so `c_lo` is Hertz's width at `K`.
    ///
    /// `None` where no region of the valley carries `q` by the bound: the gap's slope turns
    /// back before any half-width would. A contact carried mostly where `σ` has fallen far
    /// below its value at the origin can exist beyond that reach.
    fn width_bracket(&self, q: f64, modulus: f64) -> Option<WidthBracket> {
        self.width_bracket_by(q, modulus, Self::secant_within)
    }

    /// [`Profile::width_bracket`] with `σ`'s span over a band's region read by `region` (a
    /// plant's seam).
    fn width_bracket_by(
        &self,
        q: f64,
        modulus: f64,
        region: fn(&Self, Option<f64>) -> (f64, f64),
    ) -> Option<WidthBracket> {
        // `c² σ₋` that carries `q`.
        let load = 4.0 * q / (PI * modulus);
        let mut reach: Vec<f64> = self
            .knots
            .iter()
            .filter(|&&t| t != 0.0)
            .map(|t| t.abs())
            .collect();
        reach.sort_by(f64::total_cmp);
        reach.dedup();
        let starts = std::iter::once(0.0).chain(reach.iter().copied());
        let ends = reach.iter().copied().map(Some).chain(std::iter::once(None));
        let band = |from: f64, to: Option<f64>| -> Option<WidthBracket> {
            // The least `σ` at the knots passed (`None` before the first): not positive, the
            // band lies past the valley. Where `σ` falls from a passed knot to `±s` the band's
            // own line reads it; where it rises, its least is at that knot.
            let passed = self
                .knots
                .iter()
                .filter(|&&t| t != 0.0 && t.abs() <= from)
                .map(|&t| self.secant(t))
                .reduce(f64::min);
            if passed.is_some_and(|s| s <= 0.0) {
                return None;
            }
            let (least, greatest) = region(self, to);
            // Where `σ` is not positive over the band's whole region, `|m| ≤ c` (the sign of
            // `h′` alone) holds for every region inside the valley.
            let skew = if least > 0.0 {
                skew(greatest / least)
            } else {
                1.0
            };
            let need = (1.0 + skew) * (1.0 + skew) * load;
            // The reach `s` in the band with `s² σ ≥ need` at the knots passed and at `±s`, on
            // the intervals just beyond `±from` (no knot lies within the band).
            let [(a_right, k_right), (a_left, k_left)] = self.band_lines(from);
            let (right_from, right_to) = reaches(k_right, a_right, need)?;
            let (left_from, left_to) = reaches(k_left, -a_left, need)?;
            let s = passed
                .map_or(from, |p| from.max((need / p).sqrt()))
                .max(right_from)
                .max(left_from);
            let fits = [to, right_to, left_to]
                .into_iter()
                .flatten()
                .all(|end| s <= end);
            fits.then(|| WidthBracket {
                lo: (load / self.curvature_within(Some(s)).1).sqrt(),
                hi: s / (1.0 + skew),
                skew,
            })
        };
        starts.zip(ends).find_map(|(from, to)| band(from, to))
    }
}

/// A bracket of a strip's half-width ([`Profile::width_bracket`]).
#[derive(Clone, Copy, Debug, PartialEq)]
struct WidthBracket {
    /// `c` at which the load is at most `q`, mm.
    lo: f64,
    /// `c` at which it is at least `q`, mm.
    hi: f64,
    /// `μ`: at every half-width `c` up to `hi`, the centre lies in `[−μc, μc]`.
    skew: f64,
}

/// `μ`, the bound on `|m|/c` where `σ` spans a ratio `ρ = σ₊/σ₋` on the window. `|⟨cos φ⟩|`
/// over `[0, π]` with weights within that ratio is greatest with the greater weight on
/// `[0, S]`: `max_S (ρ − 1) sin S/(π + (ρ − 1) S) = cos S*`, `tan S* − S* = π/(ρ − 1)` (the
/// involute's inverse, [`inv_inverse`]); and never above `(ρ − 1)/π`, its limit as `ρ → 1`,
/// where the inverse runs out of range. `0` for `ρ = 1`, `1` as `ρ → ∞`.
fn skew(ratio: f64) -> f64 {
    let v = PI / (ratio - 1.0);
    let limit = 1.0 / v;
    inv_inverse(v).map_or(limit, f64::cos).min(limit).min(1.0)
}

/// The `s ≥ 0` where `k s² + β s ≥ need` (`need > 0`): `(from, to)`, `to` `None` where it runs
/// on; `None` where there is none. Each root in the form without a difference of near equals.
fn reaches(k: f64, beta: f64, need: f64) -> Option<(f64, Option<f64>)> {
    if k > 0.0 {
        let d = (beta * beta + 4.0 * k * need).sqrt();
        let from = if beta >= 0.0 {
            2.0 * need / (beta + d)
        } else {
            (d - beta) / (2.0 * k)
        };
        return Some((from, None));
    }
    let disc = beta * beta + 4.0 * k * need;
    if beta <= 0.0 || disc < 0.0 {
        return None;
    }
    let d = disc.sqrt();
    Some((
        2.0 * need / (beta + d),
        (k < 0.0).then(|| (beta + d) / (-2.0 * k)),
    ))
}

/// One `φ`-piece of `[0, π]` on which `h′(m + c cos φ) = a + b cos φ`.
#[derive(Clone, Copy, Debug)]
struct Segment {
    /// Where the piece starts, radians.
    from: f64,
    /// Where it ends, radians.
    to: f64,
    a: f64,
    b: f64,
}

/// The contact's angle of `t`, `cos φ = (t − m)/c`, for a `t` inside it.
fn angle(t: f64, m: f64, c: f64) -> f64 {
    ((t - m) / c).clamp(-1.0, 1.0).acos()
}

/// The pieces of `[0, π]` over `[m − c, m + c]`, in increasing `φ`. A piece that reaches an end of
/// the contact runs to `0` or `π` exactly: the end is not an angle computed through `acos` of
/// `(t − m)/c`, which is `±1` only to rounding and would leave a sliver `√ε` wide unintegrated.
fn segments(profile: &Profile, m: f64, c: f64) -> Vec<Segment> {
    let (lo, hi) = (m - c, m + c);
    let mut out: Vec<Segment> = (0..profile.curvature.len())
        .filter_map(|i| {
            let (a, b) = profile.interval(i);
            let (ta, tb) = (a.map_or(lo, |a| a.max(lo)), b.map_or(hi, |b| b.min(hi)));
            if tb <= ta {
                return None;
            }
            let from = if b.is_none_or(|b| b >= hi) {
                0.0
            } else {
                angle(tb, m, c)
            };
            let to = if a.is_none_or(|a| a <= lo) {
                PI
            } else {
                angle(ta, m, c)
            };
            let (tr, slope) = profile.line_of(i);
            let k = profile.curvature[i];
            Some(Segment {
                from,
                to,
                a: slope + k * (m - tr),
                b: k * c,
            })
        })
        .collect();
    out.reverse();
    out
}

/// `(I₀, I₁)` over `segments`: `Σ a Δφ + b Δsin φ` and `Σ a Δsin φ + b (Δφ/2 + Δsin 2φ/4)`.
fn moments(segments: &[Segment]) -> (f64, f64) {
    segments.iter().fold((0.0, 0.0), |(i0, i1), s| {
        let dp = s.to - s.from;
        let ds = s.to.sin() - s.from.sin();
        let ds2 = (2.0 * s.to).sin() - (2.0 * s.from).sin();
        (
            i0 + s.a * dp + s.b * ds,
            i1 + s.a * ds + s.b * (0.5 * dp + 0.25 * ds2),
        )
    })
}

/// A knot inside the contact: its angle and the curvature's step there (above less below).
#[derive(Clone, Copy, Debug, PartialEq)]
struct Knot {
    /// `φ_j`, radians.
    angle_rad: f64,
    /// `Δk_j`, 1/mm.
    step: f64,
}

/// Which end of the strip leads in rolling: where the stick zone sits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LeadingEdge {
    /// The end `t = m + c`.
    Upper,
    /// The end `t = m − c`.
    Lower,
}

impl LeadingEdge {
    fn sign(self) -> f64 {
        match self {
            Self::Upper => 1.0,
            Self::Lower => -1.0,
        }
    }
}

/// A point of the exact creep curve, per unit of the friction coefficient.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CreepPoint {
    /// `ξ/μ`: the creepage (sliding over rolling speed) over `μ`, in the rolling direction's
    /// sense, so the curve is the same either way: `≥ 0` from full stick to full slip.
    pub creepage: f64,
    /// `Q/(μP)`: the traction over the friction limit.
    pub traction: f64,
}

/// The `t`-interval of a section where neither member is on its edge: `None` ends run on. It may
/// be empty (`lo > hi`): no part of the section is flank.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FlankPart {
    pub lo: Option<f64>,
    pub hi: Option<f64>,
}

impl FlankPart {
    /// The whole section is flank.
    pub const WHOLE: Self = Self { lo: None, hi: None };
}

/// The greatest pressure on a region and where: `t` across the section.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Peak {
    /// MPa (N/mm²).
    pub pressure: f64,
    /// `t`, mm.
    pub at: f64,
}

/// A strip read by regions.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StripReport {
    /// `c`, mm.
    pub half_width: f64,
    /// `m`, mm.
    pub centre: f64,
    /// The greatest pressure on the strip, the larger of the regions'.
    pub peak: Peak,
    /// The greatest pressure on the flank part (a closed set: its boundary's pressure belongs to
    /// both regions). `None` where no part of the strip is flank.
    pub flank: Option<f64>,
    /// The greatest pressure on the edge part. `None` where no part of the strip is edge.
    pub edge: Option<f64>,
    /// The share of the strip's load on the edge part.
    pub edge_share: f64,
}

/// The frictionless contact of one strip (the module's doc).
#[derive(Clone, Debug, PartialEq)]
pub struct Strip {
    profile: Profile,
    /// `E*`, MPa.
    modulus: f64,
    /// `c`, mm.
    half_width: f64,
    /// `m`, mm.
    centre: f64,
    /// The knots inside the contact.
    knots: Vec<Knot>,
    /// `k_π`: the curvature at `t = m − c`.
    trailing: f64,
    /// `I₀` at the solution: zero, to the root's resolution.
    i0: f64,
    /// `I₁` at the solution.
    i1: f64,
}

impl Strip {
    /// The strip of `profile` under line load `q` (N/mm) with plane-strain modulus `modulus`
    /// (`E*`, MPa). Hertz's closed form where the curvature at the origin holds across Hertz's
    /// width (no knot inside it: then the equations' root is Hertz's, exactly); else the root of
    /// the two equations, each bracketed by construction ([`Profile::width_bracket`]: `c`
    /// between two Hertz widths, and `m` in `[−μc, μc]`, where `I₀` has the root's sign at
    /// both ends).
    ///
    /// `None` where `q` or `modulus` is not a finite number `> 0`, or no region of the
    /// section's valley carries the load by the bracket's bound ([`Profile::width_bracket`]).
    pub fn new(profile: &Profile, q: f64, modulus: f64) -> Option<Self> {
        Self::new_by(profile, q, modulus, Profile::width_bracket)
    }

    /// [`Strip::new`] with the half-width bracketed by `bracket` (a plant's seam).
    fn new_by(
        profile: &Profile,
        q: f64,
        modulus: f64,
        bracket: fn(&Profile, f64, f64) -> Option<WidthBracket>,
    ) -> Option<Self> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !(positive(q) && positive(modulus)) {
            return None;
        }
        let k = profile.slope_curvature_at_origin();
        if let Some(k) = k.filter(|&k| k > 0.0) {
            let c = (4.0 * q / (PI * modulus * k)).sqrt();
            if !profile.changes().any(|(t, _)| -c < t && t < c) {
                return Some(Self::from_root(profile, modulus, 0.0, c));
            }
        }
        Self::solved(profile, q, modulus, ROOT_RESOLUTION, bracket)
    }

    /// The root of the two equations, whatever the knots: [`Strip::new`] without Hertz's closed
    /// form (which this equals where it applies).
    #[cfg(test)]
    pub(crate) fn general(profile: &Profile, q: f64, modulus: f64) -> Option<Self> {
        Self::general_to(profile, q, modulus, ROOT_RESOLUTION)
    }

    /// [`Strip::general`] with the root finders' resolution, in units of `ε` of the bracket's
    /// scale.
    #[cfg(test)]
    pub(crate) fn general_to(
        profile: &Profile,
        q: f64,
        modulus: f64,
        resolution: f64,
    ) -> Option<Self> {
        Self::solved(profile, q, modulus, resolution, Profile::width_bracket)
    }

    /// The root of the two equations in `bracket`'s bracket, to `resolution`.
    fn solved(
        profile: &Profile,
        q: f64,
        modulus: f64,
        resolution: f64,
        bracket: fn(&Profile, f64, f64) -> Option<WidthBracket>,
    ) -> Option<Self> {
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !(positive(q) && positive(modulus)) {
            return None;
        }
        let WidthBracket { lo, hi, skew } = bracket(profile, q, modulus)?;
        let centre = |c: f64| centre_of(profile, c, skew, resolution);
        let excess = |c: f64| match centre(c) {
            Some(m) => 0.5 * modulus * c * moments(&segments(profile, m, c)).1 - q,
            None => f64::NAN, // absence: no centre is no value, which `brent` refuses
        };
        let (at_lo, at_hi) = (excess(lo), excess(hi));
        if !(at_lo.is_finite() && at_hi.is_finite()) {
            return None;
        }
        // The bracket holds the root by construction; an end with the other sign is that end
        // to rounding.
        let c = if at_lo >= 0.0 {
            lo
        } else if at_hi <= 0.0 {
            hi
        } else {
            brent(excess, lo, hi, tol(resolution * f64::EPSILON * lo))?
        };
        let m = centre(c)?;
        Some(Self::from_root(profile, modulus, m, c))
    }

    /// The strip at its root `(m, c)`.
    fn from_root(profile: &Profile, modulus: f64, m: f64, c: f64) -> Self {
        let (i0, i1) = moments(&segments(profile, m, c));
        let (lo, hi) = (m - c, m + c);
        let knots = profile
            .changes()
            .filter(|&(t, _)| lo < t && t < hi)
            .map(|(t, step)| Knot {
                angle_rad: angle(t, m, c),
                step,
            })
            .collect();
        // The interval holding `m − c` (the one above it where `m − c` is a knot).
        let trailing = profile.curvature[profile.knots.partition_point(|&k| k <= lo)];
        Self {
            profile: profile.clone(),
            modulus,
            half_width: c,
            centre: m,
            knots,
            trailing,
            i0,
            i1,
        }
    }

    /// `c`, mm.
    pub fn half_width(&self) -> f64 {
        self.half_width
    }

    /// `m`, mm.
    pub fn centre(&self) -> f64 {
        self.centre
    }

    /// `t = m + c cos θ`.
    pub fn across(&self, theta: f64) -> f64 {
        self.centre + self.half_width * theta.cos()
    }

    /// The contact's angle of `t`, held to `[0, π]`.
    pub fn angle_of(&self, t: f64) -> f64 {
        angle(t, self.centre, self.half_width)
    }

    /// The knots' angles inside the contact, increasing.
    fn cuts(&self) -> Vec<f64> {
        let mut cuts: Vec<f64> = self.knots.iter().map(|k| k.angle_rad).collect();
        cuts.sort_by(f64::total_cmp);
        cuts
    }

    /// `p(θ)`, MPa, `θ ∈ [0, π]` (the module's doc). At a knot its logarithm's factor vanishes
    /// with it (`x ln x → 0`).
    pub fn pressure(&self, theta: f64) -> f64 {
        let (st, ct) = theta.sin_cos();
        let mut sine = self.trailing;
        let mut logs = 0.0;
        for k in &self.knots {
            sine += k.step * k.angle_rad / PI;
            let x = ct - k.angle_rad.cos();
            if x != 0.0 {
                let ratio =
                    (0.5 * (theta + k.angle_rad)).sin() / (0.5 * (theta - k.angle_rad)).sin();
                logs += k.step * x * ratio.abs().ln();
            }
        }
        0.5 * self.modulus * self.half_width * (st * sine + logs / PI)
    }

    /// The load on `θ ∈ [0, θ_a]`, `t ≥ m + c cos θ_a`, N/mm: the pressure's integral in closed
    /// form. Swapping the order, `∫₀^θa sin²θ/(cos φ − cos θ) dθ = sin φ Λ(φ) + θ_a cos φ + sin θ_a`
    /// with `Λ(φ) = ln|sin((θ_a − φ)/2)/sin((θ_a + φ)/2)|`; integrating `h′ sin φ Λ` by parts at
    /// each knot,
    ///
    /// ```text
    /// F(θ_a) = (E* c/2π) [ θ_a I₁ + sin θ_a I₀ − (π/2) c k_π sin θ_a cos θ_a
    ///          + c Σ Δk_j ((cos φ_j − cos θ_a)² Λ(φ_j)/2 + sin θ_a (sin φ_j − cos θ_a φ_j)/2) ],
    /// ```
    ///
    /// Hertz's `(θ_a − sin θ_a cos θ_a)/π` of the load with no knot, and `(E*/2) c I₁` at `π`.
    pub fn load_to(&self, theta: f64) -> f64 {
        let (sa, ca) = theta.sin_cos();
        let c = self.half_width;
        let mut sum = theta * self.i1 + sa * self.i0 - 0.5 * PI * c * self.trailing * sa * ca;
        for k in &self.knots {
            let (sk, ck) = k.angle_rad.sin_cos();
            let x = ck - ca;
            let lambda = if x == 0.0 {
                0.0
            } else {
                ((0.5 * (theta - k.angle_rad)).sin() / (0.5 * (theta + k.angle_rad)).sin())
                    .abs()
                    .ln()
            };
            sum += c * k.step * (0.5 * x * x * lambda + 0.5 * sa * (sk - ca * k.angle_rad));
        }
        self.modulus * c / (2.0 * PI) * sum
    }

    /// The strip's load, `(E*/2) c I₁`: the line load it was solved for, to the root's
    /// resolution.
    pub fn load(&self) -> f64 {
        0.5 * self.modulus * self.half_width * self.i1
    }

    /// The greatest pressure on `[a, b]` (angles), searched knot to knot.
    fn peak_on(&self, a: f64, b: f64) -> (f64, f64) {
        let cuts = self.cuts();
        greatest_by_pieces(|th| self.pressure(th), a, b, &cuts, PIECE_NODES)
    }

    /// The greatest pressure on the strip and where.
    pub fn peak(&self) -> Peak {
        let (pressure, theta) = self.peak_on(0.0, PI);
        Peak {
            pressure,
            at: self.across(theta),
        }
    }

    /// The strip read by regions, `flank` the part where neither member is on its edge (`None`:
    /// no part of the section is).
    pub fn report(&self, flank: Option<FlankPart>) -> StripReport {
        self.report_by(flank, Self::peak_on)
    }

    /// [`Strip::report`] with the regions' maxima found by `peak_on(strip, a, b)` → `(p, θ)`.
    pub(crate) fn report_by(
        &self,
        flank: Option<FlankPart>,
        peak_on: impl Fn(&Self, f64, f64) -> (f64, f64),
    ) -> StripReport {
        // The flank part in angles, `[from, to]`; empty where it misses the contact.
        let (from, to) = flank.map_or((0.5 * PI, 0.5 * PI), |f| {
            let from = f.hi.map_or(0.0, |hi| self.angle_of(hi)); // absence: no upper end, the flank reaches the contact's (and `π` below)
            let to = f.lo.map_or(PI, |lo| self.angle_of(lo));
            if to < from {
                let mid = 0.5 * (from + to);
                (mid, mid)
            } else {
                (from, to)
            }
        });
        let regions = [(0.0, from, true), (from, to, false), (to, PI, true)];
        let mut peak: Option<(f64, f64)> = None;
        let (mut flank_peak, mut edge_peak): (Option<f64>, Option<f64>) = (None, None);
        for (a, b, edge) in regions {
            if b <= a {
                continue;
            }
            let (v, th) = peak_on(self, a, b);
            let slot = if edge {
                &mut edge_peak
            } else {
                &mut flank_peak
            };
            *slot = Some(slot.map_or(v, |s| s.max(v)));
            if peak.is_none_or(|(p, _)| v > p) {
                peak = Some((v, th));
            }
        }
        let total = self.load_to(PI);
        let edge_share = if to > from {
            (self.load_to(from) + (total - self.load_to(to))) / total
        } else {
            1.0
        };
        // Some region has width: `[0, π]` is split at two points.
        debug_assert!(peak.is_some());
        let (pressure, theta) = peak.unwrap_or_else(|| (self.pressure(from), from));
        StripReport {
            half_width: self.half_width,
            centre: self.centre,
            peak: Peak {
                pressure,
                at: self.across(theta),
            },
            flank: flank_peak,
            edge: edge_peak,
            edge_share,
        }
    }

    /// The exact creep curve at slip `s ∈ [0, 1]`, the share of the contact in slip: the stick
    /// zone runs from `d = x_t + 2cs` (along from the trailing end `x_t`) to the leading end
    /// `x_l`, centre `m′ = m + c s` towards the leading end and half-width `c′ = c (1 − s)`.
    /// `ξ/μ = ±I₀′/π` and `Q/μP = 1 − c′ I₁′/(c I₁)`, the zone's own moments (at `s = 1`, `I₀′ =
    /// π h′(x_l)`, `I₁′ = 0`). `s = 0` is full stick (`ξ = 0` to the root's resolution, `Q = 0`),
    /// `s = 1` full slip. `None` for `s` outside `[0, 1]`.
    pub fn creep(&self, lead: LeadingEdge, slip: f64) -> Option<CreepPoint> {
        if !(0.0..=1.0).contains(&slip) {
            return None;
        }
        let sign = lead.sign();
        let c = self.half_width;
        let zone = c * (1.0 - slip);
        let (i0, i1) = if zone > 0.0 {
            moments(&segments(
                &self.profile,
                self.centre + sign * c * slip,
                zone,
            ))
        } else {
            let lead_end = self.centre + sign * c;
            (PI * self.profile.slope_at(lead_end), 0.0)
        };
        Some(CreepPoint {
            creepage: sign * i0 / PI,
            traction: 1.0 - zone * i1 / (c * self.i1),
        })
    }
}

impl Profile {
    /// `h″` at the origin, where it is one value on both sides; `None` where a step ends there.
    fn slope_curvature_at_origin(&self) -> Option<f64> {
        let i = self.knots.partition_point(|&k| k < 0.0);
        (self.curvature[i] == self.curvature[i + 1]).then_some(self.curvature[i])
    }
}

/// The root finders' resolution, in `ε` of the bracket's scale: the floating-point floor.
const ROOT_RESOLUTION: f64 = 1.0;

fn tol(x_tol: f64) -> Tol {
    Tol {
        x_tol,
        ..Tol::default()
    }
}

/// `m` at half-width `c`: the root of `I₀` on `[−μc, μc]`, `μ` the bracket's `skew`, where
/// `I₀(−μc) ≤ 0 ≤ I₀(μc)` ([`Profile::width_bracket`]), so an end of the other sign is that
/// end to rounding.
fn centre_of(profile: &Profile, c: f64, skew: f64, resolution: f64) -> Option<f64> {
    let f = |m: f64| moments(&segments(profile, m, c)).0;
    let (lo, hi) = (-skew * c, skew * c);
    let (at_lo, at_hi) = (f(lo), f(hi));
    if at_lo >= 0.0 {
        Some(lo)
    } else if at_hi <= 0.0 {
        Some(hi)
    } else {
        brent(f, lo, hi, tol(resolution * f64::EPSILON * c))
    }
}

/// The greatest value of `f` on `[a, b]` and where, searched piece by piece between `cuts`: each
/// piece sampled at `nodes + 1` cosine-graded points (its ends included) and every sampled local
/// maximum refined by golden section within its two neighbours, to the angle's resolution
/// ([`golden_rounds`]). Inside a piece `f` is analytic; at its ends it may have kinks, so the
/// samples crowd there.
pub(crate) fn greatest_by_pieces(
    f: impl Fn(f64) -> f64,
    a: f64,
    b: f64,
    cuts: &[f64],
    nodes: usize,
) -> (f64, f64) {
    if b <= a {
        return (f(a), a);
    }
    let edges: Vec<f64> = std::iter::once(a)
        .chain(cuts.iter().copied().filter(|&x| a < x && x < b))
        .chain(std::iter::once(b))
        .collect();
    #[expect(clippy::cast_precision_loss, reason = "a node count")]
    let n = nodes as f64;
    let mut best = (f(a), a);
    let mut see = |v: f64, x: f64| {
        if v > best.0 {
            best = (v, x);
        }
    };
    for w in edges.windows(2) {
        let (u, v) = (w[0], w[1]);
        let xs: Vec<f64> = (0..=nodes)
            .map(|i| {
                #[expect(clippy::cast_precision_loss, reason = "a node index")]
                let i = i as f64;
                u + (v - u) * 0.5 * (1.0 - (PI * i / n).cos())
            })
            .collect();
        let ps: Vec<f64> = xs.iter().map(|&x| f(x)).collect();
        for i in 0..=nodes {
            let left = i == 0 || ps[i] >= ps[i - 1];
            let right = i == nodes || ps[i] >= ps[i + 1];
            if !(left && right) {
                continue;
            }
            see(ps[i], xs[i]);
            let (lo, hi) = (xs[i.saturating_sub(1)], xs[(i + 1).min(nodes)]);
            let (x, fx) = golden_section(&f, lo, hi);
            see(fx, x);
        }
    }
    best
}

/// Golden-section rounds that take a bracket of width `w` at `scale` to its floating-point
/// resolution, `ε·scale`: `⌈ln(w/(ε·scale))/ln φ⌉`, at most 75 for a bracket of `π` at `π`
/// (none for one already there). Not `√ε`: a piece's peak can sit within `√ε` of a knot, where
/// the pressure's `x ln x` is not flat.
fn golden_rounds(width: f64, scale: f64) -> u32 {
    let floor = f64::EPSILON * scale;
    if width <= floor {
        return 0;
    }
    #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "a positive count below 2^31 (ln of a ratio of finite doubles)"
    )]
    let rounds = ((width / floor).ln() / (1.0 / golden()).ln()).ceil() as u32;
    rounds
}

/// The greater of the last two golden-section points on `[lo, hi]`.
fn golden_section(f: &impl Fn(f64) -> f64, lo: f64, hi: f64) -> (f64, f64) {
    let r = golden();
    let (mut a, mut b) = (lo, hi);
    let (mut x1, mut x2) = (b - r * (b - a), a + r * (b - a));
    let (mut f1, mut f2) = (f(x1), f(x2));
    for _ in 0..golden_rounds(hi - lo, lo.abs().max(hi.abs())) {
        if f1 >= f2 {
            b = x2;
            x2 = x1;
            f2 = f1;
            x1 = b - r * (b - a);
            f1 = f(x1);
        } else {
            a = x1;
            x1 = x2;
            f1 = f2;
            x2 = a + r * (b - a);
            f2 = f(x2);
        }
    }
    if f1 >= f2 {
        (x1, f1)
    } else {
        (x2, f2)
    }
}

// ---------------------------------------------------------------------------------- a panel's friction

/// A vector in the tangent plane.
pub type Vec3 = [f64; 3];

fn dot(a: Vec3, b: Vec3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn norm(a: Vec3) -> f64 {
    dot(a, a).sqrt()
}

/// `a + s b`.
fn add(a: Vec3, b: Vec3, s: f64) -> Vec3 {
    [a[0] + s * b[0], a[1] + s * b[1], a[2] + s * b[2]]
}

fn scale(a: Vec3, s: f64) -> Vec3 {
    [s * a[0], s * a[1], s * a[2]]
}

fn cross(a: Vec3, b: Vec3) -> Vec3 {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// A panel's friction per unit `μ q`: the mean over the panel of the traction's direction (times
/// the law's factor) and of the friction power over `μ q` (the factor times the sliding speed).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slide {
    pub direction: Vec3,
    pub speed: f64,
}

/// The line `A + B u` about the foot of the origin on it: `w = u − u₀` at the panel's two ends
/// `w₁ < w₂` (`w₂ − w₁ = 1`), its offset `h` from the origin, `|B|` and `R_i = |A + B u_i|`.
struct Foot {
    b: f64,
    b2: f64,
    h: Vec3,
    hn: f64,
    w1: f64,
    w2: f64,
    r1: f64,
    r2: f64,
    /// Whether the offset's terms are taken: `|h| > floor·|B|`.
    offset: bool,
}

/// The relative offset `|h|/|B|` at or below which a panel's offset terms are dropped: none,
/// they are kept down to `h = 0`, where they vanish with `|h|`. A plant raises it.
const OFFSET_FLOOR: f64 = 0.0;

impl Foot {
    /// `None` for `B = 0`: the sliding is uniform over the panel. `floor` is the offset's
    /// ([`OFFSET_FLOOR`]).
    fn of(a: Vec3, b: Vec3, floor: f64) -> Option<Self> {
        let b2 = dot(b, b);
        (b2 != 0.0).then(|| {
            let bn = b2.sqrt();
            let u0 = -dot(a, b) / b2;
            let h = add(a, b, u0);
            let hn = norm(h);
            let (w1, w2) = (-0.5 - u0, 0.5 - u0);
            Self {
                b: bn,
                b2,
                h,
                hn,
                w1,
                w2,
                r1: (bn * w1).hypot(hn),
                r2: (bn * w2).hypot(hn),
                offset: hn > floor * bn,
            }
        })
    }

    /// Whether the zero of sliding's foot is off the panel: `w₁`, `w₂` of one sign, each
    /// difference written as a product of one-signed terms.
    fn one_sided(&self) -> bool {
        self.w1 * self.w2 > 0.0
    }

    /// `[asinh(|B| w/|h|)]` over the panel; `0` where `h = 0` (its factor `|h|` vanishes with it).
    fn d_asinh(&self) -> f64 {
        let (b, w1, w2, hn) = (self.b, self.w1, self.w2, self.hn);
        if self.one_sided() {
            (b * (w1 + w2) / (w2 * self.r1 + w1 * self.r2)).asinh()
        } else if self.offset {
            (b * w2 / hn).asinh() - (b * w1 / hn).asinh()
        } else {
            0.0
        }
    }

    /// `[w R]` over the panel.
    fn d_wr(&self) -> f64 {
        let (w1, w2) = (self.w1, self.w2);
        if self.one_sided() {
            (w1 + w2) * (self.b2 * (w1 * w1 + w2 * w2) + self.hn * self.hn)
                / (w2 * self.r2 + w1 * self.r1)
        } else {
            w2 * self.r2 - w1 * self.r1
        }
    }
}

/// **Coulomb over a panel**: the means over `u ∈ [−½, ½]` of the unit vector along the sliding
/// velocity `v = A + B u` and of `|v|`, exact. With `w = u − u₀`, `h` the line's offset:
/// `∫ (B w + h)/R dw = B R/|B|² + h asinh(|B| w/|h|)/|B|`, `∫ R dw = w R/2 + |h|² asinh(…)/(2|B|)`.
/// As `h → 0` the asinh terms vanish and the first is the integral of `sgn w`: the panel is split
/// at the zero of sliding. Each difference is written without cancellation, so a panel far from
/// the zero (`|B| ≪ |A|`) loses nothing. Where nothing slides (`A = B = 0`) the direction is
/// zero: the limit of creep's traction, which Coulomb's set-valued law admits there.
pub fn panel_slide(a: Vec3, b: Vec3) -> Slide {
    panel_slide_to(a, b, OFFSET_FLOOR)
}

/// [`panel_slide`] with the offset's floor `floor` (a plant's seam).
fn panel_slide_to(a: Vec3, b: Vec3, floor: f64) -> Slide {
    let Some(foot) = Foot::of(a, b, floor) else {
        let na = norm(a);
        let direction = if na > 0.0 {
            scale(a, 1.0 / na)
        } else {
            [0.0; 3]
        };
        return Slide {
            direction,
            speed: na,
        };
    };
    let (w1, w2, r1, r2) = (foot.w1, foot.w2, foot.r1, foot.r2);
    // B (R₂ − R₁)/|B|², with R₂ − R₁ = |B|² (w₂ − w₁)(w₂ + w₁)/(R₁ + R₂).
    let mut direction = scale(b, (w1 + w2) / (r1 + r2));
    let mut speed = 0.5 * foot.d_wr();
    if foot.offset {
        let da = foot.d_asinh();
        direction = add(direction, foot.h, da / foot.b);
        speed += foot.hn * foot.hn * da / (2.0 * foot.b);
    }
    Slide { direction, speed }
}

/// `(mean v|v|, mean |v|³)` over `τ ∈ [−½, ½]` for `v = V + B τ`, without cancellation:
/// `∫ v|v| = B [R³]/(3|B|²) + h ∫R`, `∫R = [wR]/2 + |h|² Δasinh/(2|B|)`,
/// `∫R³ = [wR³]/4 + 3|h|²[wR]/8 + 3|h|⁴ Δasinh/(8|B|)`; where `w₁`, `w₂` have one sign,
/// `[R³]/(3|B|²) = (w₁ + w₂)(R₁² + R₁R₂ + R₂²)/(3(R₁ + R₂))` and `[wR³] = [wR] R₂² + w₁ R₁ |B|²
/// (w₁ + w₂)`; with the zero inside, `|w| ≤ 1` and the direct differences lose nothing.
fn slide_moments(v: Vec3, b: Vec3, floor: f64) -> (Vec3, f64) {
    let Some(foot) = Foot::of(v, b, floor) else {
        let r = norm(v);
        return (scale(v, r), r * r * r);
    };
    let (w1, w2, r1, r2, b2) = (foot.w1, foot.w2, foot.r1, foot.r2, foot.b2);
    let h2 = foot.hn * foot.hn;
    let dwr = foot.d_wr();
    let (dr3, dwr3) = if foot.one_sided() {
        (
            (w1 + w2) * (r1 * r1 + r1 * r2 + r2 * r2) / (3.0 * (r1 + r2)),
            dwr * r2 * r2 + w1 * r1 * b2 * (w1 + w2),
        )
    } else {
        (
            (r2 * r2 * r2 - r1 * r1 * r1) / (3.0 * b2),
            w2 * r2 * r2 * r2 - w1 * r1 * r1 * r1,
        )
    };
    let (ir, ir3) = if foot.offset {
        let da = foot.d_asinh();
        (
            0.5 * dwr + h2 * da / (2.0 * foot.b),
            0.25 * dwr3 + 0.375 * h2 * dwr + 0.375 * h2 * h2 * da / foot.b,
        )
    } else {
        (0.5 * dwr, 0.25 * dwr3)
    };
    (add(scale(b, dr3), foot.h, ir), ir3)
}

/// **Carter's creep over a panel**: the means over `u ∈ [−½, ½]` of `f(|v|/s*) v/|v|` and
/// `f(|v|/s*) |v|`, `f(x) = 2x − x²` below 1 and 1 beyond (Johnson, section 8.2: `Q/μP = 1 − (1 −
/// ξ/ξ*)²`), `s* = v_r μ a k`, the sliding speed at full slip. The window `|v| < s*` is the
/// interval between the roots of `|B|² u² + 2(A·B) u + |A|² − s*²` (the stable form: the larger
/// root from the discriminant `|B|² s*² − |A × B|²`, the smaller from their product); each piece
/// of the panel is mapped onto `[−½, ½]` about its own centre: outside the window
/// [`panel_slide`], inside it `v(2/s* − |v|/s*²)` and `2|v|²/s* − |v|³/s*²` through the
/// moments. A panel the window misses is [`panel_slide`].
pub fn carter_slide(a: Vec3, b: Vec3, sstar: f64) -> Slide {
    carter_slide_to(a, b, sstar, OFFSET_FLOOR)
}

/// [`carter_slide`] with the offset's floor `floor` (a plant's seam).
fn carter_slide_to(a: Vec3, b: Vec3, sstar: f64, floor: f64) -> Slide {
    let b2 = dot(b, b);
    let na = norm(a);
    if b2 == 0.0 {
        if na == 0.0 {
            return Slide {
                direction: [0.0; 3],
                speed: 0.0,
            };
        }
        let f = if na < sstar {
            let x = na / sstar;
            2.0 * x - x * x
        } else {
            1.0
        };
        return Slide {
            direction: scale(a, f / na),
            speed: f * na,
        };
    }
    let ab = dot(a, b);
    let ax = cross(a, b);
    let disc = b2 * sstar * sstar - dot(ax, ax);
    if disc <= 0.0 || !disc.is_finite() {
        return panel_slide_to(a, b, floor);
    }
    let sq = disc.sqrt();
    let qq = -(ab + sq.copysign(ab));
    let (r1, r2) = (qq / b2, (na - sstar) * (na + sstar) / qq);
    let (ua, ub) = (r1.min(r2), r1.max(r2));
    // A window off the panel leaves one piece, the whole panel outside it: `panel_slide`.
    let mut direction = [0.0; 3];
    let mut speed = 0.0;
    let pieces = [
        (-0.5, ua.min(0.5), false),
        (ua.max(-0.5), ub.min(0.5), true),
        (ub.max(-0.5), 0.5, false),
    ];
    for (lo, hi, inside) in pieces {
        let d = hi - lo;
        if d <= 0.0 {
            continue;
        }
        let v = add(a, b, 0.5 * (lo + hi));
        let bp = scale(b, d);
        let (e, s) = if inside {
            let (m1, m3) = slide_moments(v, bp, floor);
            (
                add(scale(v, 2.0 / sstar), m1, -1.0 / (sstar * sstar)),
                2.0 * (dot(v, v) + dot(bp, bp) / 12.0) / sstar - m3 / (sstar * sstar),
            )
        } else {
            let s = panel_slide_to(v, bp, floor);
            (s.direction, s.speed)
        };
        direction = add(direction, e, d);
        speed += d * s;
    }
    Slide { direction, speed }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::field::oracle::{self, miss, num, Worst};
    use crate::hertz::gauss_legendre_rule;
    use serde_json::Value;

    const EPS: f64 = f64::EPSILON;

    fn oracle_file() -> Value {
        serde_json::from_str(include_str!("../../tests/data/field_oracle/across.json"))
            .expect("across.json parses")
    }

    fn bem_file() -> Value {
        serde_json::from_str(include_str!("../../tests/data/field_bem/across_bem.json"))
            .expect("across_bem.json parses")
    }

    /// The plane-strain modulus of steel on steel the records use, `E/(2(1 − ν²))`, MPa.
    const STEEL: f64 = 113_186.813_186_813_19;

    /// A record's end: a number, or `"-inf"`/`"inf"`, an end that does not come.
    fn end(v: &Value) -> Option<f64> {
        v.as_f64().or_else(|| {
            assert!(matches!(v.as_str(), Some("-inf" | "inf")), "{v}");
            None
        })
    }

    fn steps_of(v: &Value) -> Vec<CurvatureStep> {
        v.as_array()
            .expect("steps")
            .iter()
            .map(|s| CurvatureStep {
                from: end(&s[0]),
                to: end(&s[1]),
                dk: num(&s[2]),
            })
            .collect()
    }

    fn flank_of(v: &Value) -> Option<FlankPart> {
        Some(FlankPart {
            lo: end(&v[0]),
            hi: end(&v[1]),
        })
    }

    /// One strip's inputs.
    #[derive(Clone, Debug)]
    struct Case {
        q: f64,
        k0: f64,
        steps: Vec<CurvatureStep>,
        flank: Option<FlankPart>,
    }

    impl Case {
        fn strip(&self) -> Strip {
            let profile = Profile::new(self.k0, &self.steps).expect("a profile");
            Strip::new(&profile, self.q, STEEL).expect("a strip")
        }

        /// Every length times `k` (and `q`, a load per length).
        fn scaled(&self, k: f64) -> Self {
            Self {
                q: self.q * k,
                k0: self.k0 / k,
                steps: self
                    .steps
                    .iter()
                    .map(|s| CurvatureStep {
                        from: s.from.map(|a| a * k),
                        to: s.to.map(|b| b * k),
                        dk: s.dk / k,
                    })
                    .collect(),
                flank: self.flank.map(|f| FlankPart {
                    lo: f.lo.map(|a| a * k),
                    hi: f.hi.map(|b| b * k),
                }),
            }
        }
    }

    // ------------------------------------------------------------------ what shares no code

    /// `h′(t)` from the steps directly: `k₀ t` plus each step's `dk` times the signed length of
    /// its overlap with the interval from `0` to `t`. Shares nothing with [`Profile`]'s knots.
    fn slope_direct(c: &Case, t: f64) -> f64 {
        let (lo, hi) = (t.min(0.0), t.max(0.0));
        let overlap = |s: &CurvatureStep| {
            let a = s.from.map_or(lo, |a| a.max(lo));
            let b = s.to.map_or(hi, |b| b.min(hi));
            (b - a).max(0.0)
        };
        c.k0 * t + t.signum() * c.steps.iter().map(|s| s.dk * overlap(s)).sum::<f64>()
    }

    /// Where `h′` is not smooth inside `(m − c, m + c)`, as angles, with `0` and `π`.
    fn angles_of(c: &Case, m: f64, w: f64) -> Vec<f64> {
        let mut out = vec![0.0, PI];
        for s in &c.steps {
            for t in [s.from, s.to].into_iter().flatten() {
                if m - w < t && t < m + w {
                    out.push(((t - m) / w).acos());
                }
            }
        }
        out.sort_by(f64::total_cmp);
        out
    }

    /// `∫₀^π g(φ) dφ` by Gauss–Legendre (32 nodes) on each piece between `cuts`.
    fn by_pieces(cuts: &[f64], g: impl Fn(f64) -> f64) -> (f64, f64) {
        let rule = gauss_legendre_rule(32);
        let (mut sum, mut size) = (0.0, 0.0);
        for w in cuts.windows(2) {
            let (h, mid) = (0.5 * (w[1] - w[0]), 0.5 * (w[0] + w[1]));
            for &(x, wt) in &rule {
                let v = g(mid + h * x);
                sum += h * wt * v;
                size += h * wt * v.abs();
            }
        }
        (sum, size)
    }

    /// The two equations' residuals at `(m, c)`, each over its own scale: `I₀` and
    /// `(E*/2) c I₁ − q`, by quadrature of [`slope_direct`].
    fn residuals(case: &Case, m: f64, c: f64) -> (f64, f64) {
        let cuts = angles_of(case, m, c);
        let (i0, s0) = by_pieces(&cuts, |p| slope_direct(case, m + c * p.cos()));
        let (i1, _) = by_pieces(&cuts, |p| slope_direct(case, m + c * p.cos()) * p.cos());
        (i0 / s0, (0.5 * STEEL * c * i1 - case.q) / case.q)
    }

    /// The residuals' bound: rounding over the quadrature's 32 nodes a piece and the slope's
    /// few terms, `1e-13` (≈ 450 ε). The prototype's ends miss it by `1e-9` (below).
    const RESIDUAL: f64 = 1e-13;

    /// `∫_u^v g` by tanh-sinh at step `1/64`, `g` handed the point and its distances to both
    /// ends, so an end's `x ln x` costs nothing; exact to rounding for a `g` analytic inside.
    fn tanh_sinh(u: f64, v: f64, g: impl Fn(f64) -> f64) -> f64 {
        let hs = 1.0 / 64.0;
        let len = v - u;
        let mut sum = 0.0;
        for k in -256..=256 {
            let t = f64::from(k) * hs;
            let s = 0.5 * PI * t.sinh();
            let e = (2.0 * s).exp();
            let (from_u, to_v) = (len / (1.0 + 1.0 / e), len / (e + 1.0));
            if from_u == 0.0 || to_v == 0.0 {
                continue;
            }
            let x = if from_u < to_v { u + from_u } else { v - to_v };
            let w = 0.5 * PI * t.cosh() / (s.cosh() * s.cosh());
            sum += w * g(x);
        }
        sum * len * 0.5 * hs
    }

    /// The load on `[0, θ_a]` as the pressure's quadrature, piece by piece between knots.
    fn load_by_quadrature(strip: &Strip, theta: f64) -> f64 {
        let cuts: Vec<f64> = std::iter::once(0.0)
            .chain(strip.cuts().into_iter().filter(|&x| x < theta))
            .chain(std::iter::once(theta))
            .collect();
        let c = strip.half_width();
        cuts.windows(2)
            .map(|w| tanh_sinh(w[0], w[1], |th| strip.pressure(th) * c * th.sin()))
            .sum()
    }

    /// The prototype's load rule: Gauss–Legendre 12 on each piece between knots.
    fn load_by_twelve(strip: &Strip, a: f64, b: f64) -> f64 {
        let rule = gauss_legendre_rule(12);
        let cuts: Vec<f64> = std::iter::once(a)
            .chain(strip.cuts().into_iter().filter(|&x| a < x && x < b))
            .chain(std::iter::once(b))
            .collect();
        let c = strip.half_width();
        cuts.windows(2)
            .map(|w| {
                let (h, mid) = (0.5 * (w[1] - w[0]), 0.5 * (w[0] + w[1]));
                rule.iter()
                    .map(|&(x, wt)| {
                        let th = mid + h * x;
                        h * wt * strip.pressure(th) * c * th.sin()
                    })
                    .sum::<f64>()
            })
            .sum()
    }

    /// The edge share by the prototype's rule, on the same regions as [`Strip::report`].
    fn edge_share_by_twelve(strip: &Strip, flank: Option<FlankPart>) -> f64 {
        let edge = std::cell::Cell::new(0.0);
        strip.report_by(flank, |s, a, b| {
            if a == 0.0 || b == PI {
                // An edge region: it touches an end of the contact.
                edge.set(edge.get() + load_by_twelve(s, a, b));
            }
            (s.pressure(a), a)
        });
        edge.get() / strip.load()
    }

    /// A deterministic generator (splitmix64), uniform on `[lo, hi)`.
    struct Draw(u64);

    impl Draw {
        fn next(&mut self) -> f64 {
            self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
            let mut z = self.0;
            z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
            z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
            z ^= z >> 31;
            #[expect(clippy::cast_precision_loss, reason = "53 bits of a draw")]
            let u = (z >> 11) as f64 / (1u64 << 53) as f64;
            u
        }

        fn uniform(&mut self, lo: f64, hi: f64) -> f64 {
            lo + (hi - lo) * self.next()
        }

        fn pick(&mut self, from: &[f64]) -> f64 {
            #[expect(
                clippy::cast_possible_truncation,
                clippy::cast_sign_loss,
                clippy::cast_precision_loss,
                reason = "an index below a short list's length"
            )]
            let i = ((self.next() * from.len() as f64) as usize).min(from.len() - 1);
            from[i]
        }
    }

    /// The verifier's generator of realistic strips (round six's `t6_strip.py`): a round on one
    /// member, crossing Hertz's width; with `both`, the other member's round below it and a tip
    /// relief, always; else each sometimes.
    fn random_case(d: &mut Draw, both: bool) -> Case {
        let k0 = d.uniform(0.1, 1.5);
        let q = d.uniform(20f64.ln(), 800f64.ln()).exp();
        let re = d.pick(&[0.05, 0.1, 0.2, 0.45]);
        let c2 = d.uniform(0.3, 1.0);
        let w = re * d.uniform(0.5, 1.2);
        let ch = (4.0 * q / (PI * STEEL * k0)).sqrt();
        let s = d.uniform(-1.5 * ch, 1.5 * ch);
        let mut steps = vec![CurvatureStep {
            from: Some(s),
            to: Some(s + w),
            dk: c2 / re,
        }];
        let mut flank = FlankPart {
            lo: None,
            hi: Some(s),
        };
        if both || d.next() < 0.4 {
            let re2 = d.pick(&[0.05, 0.1, 0.2]);
            let w2 = re2 * d.uniform(0.5, 1.2);
            let s2 = d.uniform(-1.5 * ch, s);
            steps.push(CurvatureStep {
                from: Some(s2 - w2),
                to: Some(s2),
                dk: d.uniform(0.3, 1.0) / re2,
            });
            flank.lo = Some(s2);
        }
        if both || d.next() < 0.3 {
            steps.push(CurvatureStep {
                from: None,
                to: Some(d.uniform(-ch, ch)),
                dk: d.uniform(0.05, 1.0),
            });
        }
        Case {
            q,
            k0,
            steps,
            flank: Some(flank),
        }
    }

    /// Random strips with a knot inside the contact (the general solve), 265 of the verifier's
    /// generator and 226 of both rounds and relief: the 491 round six measured.
    fn random_strips() -> Vec<(Case, Strip)> {
        let mut out = Vec::new();
        for (seed, both, count) in [(6, false, 265), (7, true, 226)] {
            let mut d = Draw(seed);
            let mut taken = 0;
            while taken < count {
                let case = random_case(&mut d, both);
                let strip = case.strip();
                if !strip.knots.is_empty() {
                    out.push((case, strip));
                    taken += 1;
                }
            }
        }
        out
    }

    // ------------------------------------------------------------------ the oracle
    //
    // The prototype's records, and the test on them, go with the prototype. What holds each
    // record's function then shares neither code nor method with it:
    // - `contact2d`'s `c` and `m` (`_pieces_of`, `_segments`, `_I`, `_strip`):
    //   `the_strip_solves_its_equations` (the equations by quadrature of `h′` from the steps),
    //   `hertz_off_a_step` and `the_strip_is_homogeneous`;
    // - the pressure and its maxima (`_conj`, `max_pieces`, `strip_report`):
    //   `the_boundary_elements_agree` (contact-verify5's BEM) and
    //   `region_maxima_are_a_dense_references`;
    // - the edge share: `the_load_is_the_pressures_integral` (tanh-sinh) and the BEM;
    // - `panel_slide` and `carter_slide`: `both_slides_are_their_quadrature`;
    // - the creep curve: `the_creep_curve_is_carters_on_hertz` and
    //   `the_exact_creep_curve_is_monotone`.

    /// The README's rules for this step's records (each record states its own, asserted below).
    const C_ABS: f64 = 1e-13;
    const M_ABS: f64 = 1e-12;
    const P_REL: f64 = 1e-10;
    const SHARE_ABS: f64 = 1e-10;
    const WHERE_OF_C: f64 = 1e-4;
    const SLIDE_ABS: f64 = 1e-13;

    /// What [`reproduce`] saw.
    #[derive(Default)]
    struct Seen {
        worst: Worst,
        /// The values that reproduce, alone.
        reproduced: Worst,
        /// The values of records that solve their own strip and whose share is their pressure's
        /// integral: what the prototype's two flaws do not reach.
        clean: Worst,
        /// Values that missed and are refuted: the record is not the solution of its own strip
        /// (its `(c, m)` fail the equations), or its edge share is not its pressure's integral.
        refuted: usize,
        /// Records with a refuted value.
        refuted_records: Vec<String>,
        /// Values that missed and are not refuted.
        unexplained: Vec<String>,
    }

    /// Every record of `across.json`: per strip `c`, `m`, the peak (`contact2d`'s and the
    /// report's) and where, each region's peak (the record's `0.0` is no such region: `None`),
    /// the edge share; the 30 panels' Coulomb and Carter means. `perturb` moves every port value
    /// (the tolerance's laws).
    fn reproduce(perturb: impl Fn(f64, f64) -> f64) -> Seen {
        let file = oracle_file();
        let records = oracle::records(&file);
        assert_eq!(records.len(), 44);
        let mut seen = Seen::default();
        for rec in records {
            let (i, o, tol) = (rec.inputs, rec.outputs, rec.tol);
            if rec.id == "across/slide" {
                assert_eq!(num(&tol["abs"]), SLIDE_ABS);
                let cases = i["cases"].as_array().expect("cases");
                assert_eq!(cases.len(), 30);
                for (k, c) in cases.iter().enumerate() {
                    let v = |x: &Value| -> Vec3 { [num(&x[0]), num(&x[1]), num(&x[2])] };
                    let (a, b, s) = (v(&c["A"]), v(&c["B"]), num(&c["sstar"]));
                    let out = &o["cases"][k];
                    for (name, slide) in [
                        ("coulomb", panel_slide(a, b)),
                        ("carter", carter_slide(a, b, s)),
                    ] {
                        let values = slide.direction.into_iter().chain([slide.speed]);
                        let names = ["_dir", "_dir", "_dir", "_speed"];
                        for (j, (port, n)) in values.zip(names).enumerate() {
                            let key = format!("{name}{n}");
                            let oracle = if j < 3 {
                                num(&out[&key][j])
                            } else {
                                num(&out[&key])
                            };
                            let r = miss(perturb(port, SLIDE_ABS), oracle, SLIDE_ABS);
                            seen.worst.see(r, || format!("slide {k} {key}"));
                            if r <= 1.0 {
                                seen.reproduced.see(r, || format!("slide {k} {key}"));
                            }
                            if r > 1.0 {
                                seen.unexplained.push(format!("slide {k} {key}"));
                            }
                        }
                    }
                }
                continue;
            }
            assert_eq!(num(&tol["c"]["abs"]), C_ABS);
            assert_eq!(num(&tol["m"]["abs"]), M_ABS);
            for key in ["pmax", "pflank", "pedge"] {
                assert_eq!(num(&tol[key]["rel"]), P_REL);
            }
            assert_eq!(num(&tol["fedge"]["abs"]), SHARE_ABS);
            assert_eq!(num(&tol["t_pmax"]["rel"]), WHERE_OF_C);
            let case = Case {
                q: num(&i["q"]),
                k0: num(&i["k0"]),
                steps: steps_of(&i["steps"]),
                flank: flank_of(&i["flank"]),
            };
            assert_eq!(num(&i["Es"]), STEEL);
            let strip = case.strip();
            let report = strip.report(case.flank);
            let peak = strip.peak();
            let (c, m) = (strip.half_width(), strip.centre());
            let rec_c = num(&o["c"]);
            let rec_m = num(&o["m"]);
            let at = |v: f64| perturb(v, 1.0);
            let p_bound = |x: f64| P_REL * x.abs();
            let where_bound = WHERE_OF_C * rec_c;
            let region = |v: &Value| (num(v) != 0.0).then(|| num(v));
            let mut compared: Vec<(&str, f64, f64, f64)> = vec![
                ("c", at(c), rec_c, C_ABS),
                ("m", perturb(m, M_ABS), rec_m, M_ABS),
                (
                    "pmax2d",
                    at(peak.pressure),
                    num(&o["pmax2d"]),
                    p_bound(num(&o["pmax2d"])),
                ),
                ("t_pmax2d", at(peak.at), num(&o["t_pmax2d"]), where_bound),
                (
                    "pmax",
                    at(report.peak.pressure),
                    num(&o["pmax"]),
                    p_bound(num(&o["pmax"])),
                ),
                ("t_pmax", at(report.peak.at), num(&o["t_pmax"]), where_bound),
                (
                    "fedge",
                    perturb(report.edge_share, SHARE_ABS),
                    num(&o["fedge"]),
                    SHARE_ABS,
                ),
            ];
            for (key, port) in [("pflank", report.flank), ("pedge", report.edge)] {
                let rec_v = region(&o[key]);
                assert_eq!(port.is_some(), rec_v.is_some(), "{} {key}", rec.id);
                if let (Some(p), Some(r)) = (port, rec_v) {
                    compared.push((key, at(p), r, p_bound(r)));
                }
            }
            // Whether the record solves its own strip: its `(c, m)` against the equations, by
            // quadrature, and the port's.
            let (r0, r1) = residuals(&case, rec_m, rec_c);
            let record_solves = r0.abs().max(r1.abs()) <= RESIDUAL;
            let (p0, p1) = residuals(&case, m, c);
            assert!(
                p0.abs().max(p1.abs()) <= RESIDUAL,
                "{}: {p0:e} {p1:e}",
                rec.id
            );
            // Whether the record's edge share is its pressure's integral.
            let share_by_quadrature = {
                let total = load_by_quadrature(&strip, PI);
                let edge = |th: f64| load_by_quadrature(&strip, th);
                let FlankPart { lo, hi } = case.flank.expect("a flank");
                let from = hi.map_or(0.0, |hi| strip.angle_of(hi));
                let to = lo.map_or(PI, |lo| strip.angle_of(lo));
                if to > from {
                    (edge(from) + total - edge(to)) / total
                } else {
                    1.0
                }
            };
            assert!(
                (share_by_quadrature - report.edge_share).abs() <= 1e-13,
                "{}: {share_by_quadrature} vs {}",
                rec.id,
                report.edge_share
            );
            let share_refuted = (share_by_quadrature - num(&o["fedge"])).abs() > SHARE_ABS;
            let mut refuted_here = false;
            for (key, port, oracle, bound) in compared {
                let r = miss(port, oracle, bound);
                seen.worst
                    .see(r, || format!("{} {key}: {port:e} vs {oracle:e}", rec.id));
                if r <= 1.0 {
                    seen.reproduced.see(r, || format!("{} {key}", rec.id));
                }
                if record_solves && !share_refuted {
                    seen.clean.see(r, || format!("{} {key}", rec.id));
                }
                if r > 1.0 || r.is_nan() {
                    let refuted = !record_solves || (key == "fedge" && share_refuted);
                    if refuted {
                        seen.refuted += 1;
                        refuted_here = true;
                    } else {
                        seen.unexplained.push(format!("{} {key}", rec.id));
                    }
                }
            }
            if refuted_here {
                seen.refuted_records.push(rec.id.to_string());
            }
        }
        seen
    }

    /// Every value [`reproduce`] compares: 240 slide means (30 panels, two laws, a direction and
    /// a speed); per strip 7, and a region's peak
    /// where the record has the region.
    fn values() -> usize {
        let file = oracle_file();
        let regions: usize = oracle::records(&file)
            .iter()
            .filter(|r| r.id != "across/slide")
            .map(|r| {
                ["pflank", "pedge"]
                    .iter()
                    .filter(|k| num(&r.outputs[**k]) != 0.0)
                    .count()
            })
            .sum();
        240 + 43 * 7 + regions
    }

    /// **Every record reproduces or is refuted by the physics.** A value reproduces within its
    /// record's rule; where one does not, the record is not right: either its `(c, m)` do not
    /// solve its own strip's equations (by quadrature of `h′`, to `1e-13`; the port's do), the
    /// prototype's ends cut through `acos` of a rounded `±1`, or its edge share is not the
    /// integral of its own pressure (by tanh-sinh, to `1e-13`; the port's closed form is), the
    /// prototype's Gauss–Legendre 12 on pieces whose ends carry `x ln x`. Counted exactly.
    #[test]
    fn the_oracle_records_reproduce_or_are_refuted() {
        let seen = reproduce(|x, _| x);
        eprintln!(
            "across.json: {} values; {} reproduce, worst {:.3} of the tolerance ({}); on the \
             records neither flaw reaches {}, worst {:.2e} ({}); {} refuted in {} records: {:?}",
            seen.worst.count,
            seen.reproduced.count,
            seen.reproduced.ratio,
            seen.reproduced.at,
            seen.clean.count,
            seen.clean.ratio,
            seen.clean.at,
            seen.refuted,
            seen.refuted_records.len(),
            seen.refuted_records
        );
        assert_eq!(seen.worst.count, values());
        assert_eq!(seen.reproduced.count + seen.refuted, values());
        assert_eq!(seen.clean.missed, 0);
        assert!(seen.unexplained.is_empty(), "{:?}", seen.unexplained);
        assert_eq!(
            (seen.refuted, seen.refuted_records.len()),
            (REFUTED_VALUES, REFUTED_RECORDS)
        );
    }

    /// The counts the oracle test holds: what the two prototype flaws reach.
    const REFUTED_VALUES: usize = 65;
    const REFUTED_RECORDS: usize = 23;

    /// The tolerance's two laws: a few roundings off still reproduces (nothing new missed), and
    /// ten times each rule off misses at every value.
    #[test]
    fn the_oracle_tolerance_passes_rounding_and_fails_ten_times_itself() {
        let rounded = reproduce(|x, _| x * (1.0 + 4.0 * EPS));
        assert!(rounded.unexplained.is_empty(), "{:?}", rounded.unexplained);
        assert_eq!(rounded.refuted, REFUTED_VALUES);
        // Ten times the rule: the absolute rules by their own size, the relative and the
        // location's by theirs (the location's is 1e-4 of `c`, at most 1e-4 × 0.1 mm).
        let tenfold = reproduce(|x, abs| {
            if abs == 1.0 {
                x * (1.0 + 10.0 * P_REL) + 10.0 * WHERE_OF_C * 0.1
            } else {
                x + 10.0 * abs
            }
        });
        assert_eq!(
            tenfold.unexplained.len() + tenfold.refuted,
            tenfold.worst.count
        );
    }

    // ------------------------------------------------------------------ the equations

    /// **The strip solves its equations**, read by a method that shares no code with it: `I₀ =
    /// 0` and `(E*/2) c I₁ = q` by Gauss–Legendre over `φ` of `h′` summed from the steps
    /// directly, to `1e-13` of each one's scale, on the 491 random strips, the oracle's 43, the
    /// ring's section concave beyond its round (at 100 N/mm, and at 1700 and 2000, where the
    /// contact runs past the round onto the concave flank) and the near miss of a run with a
    /// knot inside it ([`near_miss`]). Its plant: the prototype's own `(c, m)` (the records),
    /// whose ends `acos` cut a `√ε` sliver from: nine of the 43 miss by up to `2e-9`.
    #[test]
    fn the_strip_solves_its_equations() {
        let mut worst = 0.0_f64;
        let mut count = 0;
        let mut check = |case: &Case, m: f64, c: f64| {
            let (r0, r1) = residuals(case, m, c);
            count += 1;
            worst = worst.max(r0.abs()).max(r1.abs());
        };
        for (case, strip) in random_strips() {
            check(&case, strip.centre(), strip.half_width());
        }
        let sections = [100.0, 1700.0, 2000.0]
            .map(ring_at)
            .into_iter()
            .chain(NEAR_MISS_LOADS.map(near_miss));
        for case in sections {
            let strip = case.strip();
            check(&case, strip.centre(), strip.half_width());
        }
        let file = oracle_file();
        let mut prototype_misses = 0;
        let mut prototype_worst = 0.0_f64;
        for rec in oracle::records(&file) {
            if rec.id == "across/slide" {
                continue;
            }
            let case = Case {
                q: num(&rec.inputs["q"]),
                k0: num(&rec.inputs["k0"]),
                steps: steps_of(&rec.inputs["steps"]),
                flank: None,
            };
            let strip = case.strip();
            check(&case, strip.centre(), strip.half_width());
            let (r0, r1) = residuals(&case, num(&rec.outputs["m"]), num(&rec.outputs["c"]));
            let r = r0.abs().max(r1.abs());
            prototype_worst = prototype_worst.max(r);
            prototype_misses += usize::from(r > RESIDUAL);
        }
        eprintln!(
            "{count} strips: worst residual {worst:.2e}; the prototype's (c, m): {prototype_misses} \
             of 43 miss, worst {prototype_worst:.2e}"
        );
        assert_eq!(count, 491 + 3 + 3 + 43);
        assert!(worst <= RESIDUAL, "{worst:e}");
        assert!(prototype_misses > 0 && prototype_worst > 10.0 * RESIDUAL);
    }

    /// The ring's section (`gap.json`'s ring, pair 0, valley point 3) at line load `q`: concave
    /// (`k₀ < 0`) beyond a round (its run, [`RING_RUN`]) that holds the contact to 1640 N/mm.
    fn ring_at(q: f64) -> Case {
        Case {
            q,
            k0: -0.048_367_718_543_815_094,
            steps: vec![CurvatureStep {
                from: Some(-0.061_147_195_323_074_58),
                to: Some(0.111_499_084_453_049_85),
                dk: 4.994_284_347_581_456,
            }],
            flank: Some(FlankPart {
                lo: None,
                hi: Some(-0.061_147_195_323_074_58),
            }),
        }
    }

    /// The ring's run: its round, where the curvature is positive.
    const RING_RUN: (f64, f64) = (-0.061_147_195_323_074_58, 0.111_499_084_453_049_85);

    /// The near miss (the checker's): a run `(−0.03, 0.05)` of curvature `4.95` with a second
    /// step on `(0.01, 0.05)`, concave (`−0.05`) beyond, at line load `q`. Its contact sits
    /// inside the run to 300 N/mm, within `2c` of its nearer end from 100 N/mm: what refuses
    /// a bracket that asks the curvature to be positive over `[−2c, 2c]`.
    fn near_miss(q: f64) -> Case {
        let step = |from: f64, to: f64, dk: f64| CurvatureStep {
            from: Some(from),
            to: Some(to),
            dk,
        };
        Case {
            q,
            k0: -0.05,
            steps: vec![step(-0.03, 0.05, 5.0), step(0.01, 0.05, 2.0)],
            flank: Some(FlankPart {
                lo: Some(-0.03),
                hi: Some(0.01),
            }),
        }
    }

    /// The near miss's run.
    const NEAR_MISS_RUN: (f64, f64) = (-0.03, 0.05);

    /// The near miss turned over, with a soft step `(0.04, 0.06)` past its run's nearer end: its
    /// contact leans right, its farthest knot is on the right, and beyond it the right side is
    /// the one that carries the bound (a section that reads each band's lines on both sides).
    fn soft_right(q: f64) -> Case {
        let step = |from: f64, to: f64, dk: f64| CurvatureStep {
            from: Some(from),
            to: Some(to),
            dk,
        };
        Case {
            q,
            k0: -0.05,
            steps: vec![
                step(-0.05, 0.03, 5.0),
                step(-0.05, -0.01, 2.0),
                step(0.04, 0.06, 0.5),
            ],
            flank: None,
        }
    }

    /// The run of [`soft_right`].
    const SOFT_RIGHT_RUN: (f64, f64) = (-0.05, 0.03);

    /// The loads the checker found the near miss refused at, N/mm.
    const NEAR_MISS_LOADS: [f64; 3] = [100.0, 176.0, 250.0];

    /// Loads at which the near miss's bracket reads `σ` past its run (its band reaches `0.05`,
    /// so `|m| ≤ μc` is the section's own and not its twin's) while its contact stays inside.
    const NEAR_EDGE_LOADS: [f64; 4] = [350.0, 360.0, 370.0, 380.0];

    /// `case`'s convex twin on its run `(lo, hi)`: the same section there, the tails' curvature
    /// `k₀` raised to `−k₀` by a step beyond each end (the run's knots, curvature and slopes are
    /// the section's to the bit).
    fn twin(case: &Case, run: (f64, f64)) -> Case {
        let mut steps = case.steps.clone();
        steps.extend([
            CurvatureStep {
                from: None,
                to: Some(run.0),
                dk: -2.0 * case.k0,
            },
            CurvatureStep {
                from: Some(run.1),
                to: None,
                dk: -2.0 * case.k0,
            },
        ]);
        Case {
            steps,
            ..case.clone()
        }
    }

    /// The previous bracket (the plant): Hertz's width at the least curvature within `2c` of
    /// the origin, band by band, the first that fits; the centre anywhere in `[−c, c]`.
    fn convex_over_twice_the_width(p: &Profile, q: f64, modulus: f64) -> Option<WidthBracket> {
        let hertz = |k: f64| (4.0 * q / (PI * modulus * k)).sqrt();
        let mut reach: Vec<f64> = p.knots.iter().map(|t| t.abs()).collect();
        reach.sort_by(f64::total_cmp);
        reach.dedup();
        let hi = reach
            .into_iter()
            .filter(|&r| r > 0.0)
            .map(Some)
            .chain(std::iter::once(None))
            .filter_map(|r| {
                let (least, _) = p.curvature_within(r);
                let c = (least > 0.0).then(|| hertz(least))?;
                r.is_none_or(|r| 2.0 * c <= r).then_some(c)
            })
            .next()?;
        let (_, greatest) = p.curvature_within(Some(2.0 * hi));
        Some(WidthBracket {
            lo: hertz(greatest),
            hi,
            skew: 1.0,
        })
    }

    /// The twin law's tolerance on `c`, `m` (over `c`) and the peak pressure, relative: two
    /// roots of one pair of equations, found from different brackets. Each `c` lies within its
    /// finder's `ε c` of where its load crosses `q`; that crossing moves by half the load's
    /// relative error (`c dF/dc ≈ 2F`): its rounding over at most eight terms (`8ε`) and the
    /// centre's own `ε c` read through `∂I₁/∂m ≤ K π` against `I₁ ≥ κ c π/2` (`2(K/κ) ε`,
    /// `K/κ < 1.5` on these sections), `≤ 5.5ε`; the peak, `m` and `c` with it. Both: `≤ 13ε`.
    const TWIN: f64 = 16.0 * EPS;

    /// What [`twin_law`] saw.
    #[derive(Default)]
    struct TwinSeen {
        /// Loads where the twin's contact stays inside the run: the strips compared.
        inside: usize,
        /// Of those, the strips with a knot inside the contact (the general solve).
        general: usize,
        /// Loads where it leaves the run: the strip's own equations read instead.
        beyond: usize,
        /// The worst miss of the twin, in [`TWIN`]s, and where.
        worst: Worst,
        /// The worst residual of a strip beyond its run.
        residual: f64,
        /// The strips refused: section and load.
        refused: Vec<(&'static str, f64)>,
    }

    /// The twin law over the ring's section (1 to 10⁴ N/mm, eight loads a decade), the near
    /// miss (1 to 1778 N/mm, [`NEAR_MISS_LOADS`] and [`NEAR_EDGE_LOADS`]) and [`soft_right`] (1
    /// to 1778 N/mm), each strip solved with `bracket`;
    /// `perturb` moves the strip's `c`, `m` and peak by a share of [`TWIN`] (the tolerance's
    /// laws).
    fn twin_law(
        bracket: fn(&Profile, f64, f64) -> Option<WidthBracket>,
        perturb: impl Fn(f64) -> f64,
    ) -> TwinSeen {
        let decade = |top: i32| (0..=top).map(|k| 10f64.powf(f64::from(k) / 8.0));
        let ring = decade(32).map(|q| ("ring", ring_at(q), RING_RUN));
        let near = decade(26)
            .chain(NEAR_MISS_LOADS)
            .chain(NEAR_EDGE_LOADS)
            .map(|q| ("near miss", near_miss(q), NEAR_MISS_RUN));
        let soft = decade(26).map(|q| ("soft right", soft_right(q), SOFT_RIGHT_RUN));
        let mut seen = TwinSeen::default();
        for (name, case, run) in ring.chain(near).chain(soft) {
            let solve = |c: &Case| {
                let profile = Profile::new(c.k0, &c.steps).expect("a profile");
                Strip::new_by(&profile, c.q, STEEL, bracket)
            };
            let (Some(strip), Some(twin)) = (solve(&case), solve(&twin(&case, run))) else {
                seen.refused.push((name, case.q));
                continue;
            };
            let (c, m) = (twin.half_width(), twin.centre());
            if run.0 < m - c && m + c < run.1 {
                seen.inside += 1;
                seen.general += usize::from(!strip.knots.is_empty());
                let p = twin.peak().pressure;
                let miss = (perturb(strip.half_width()) - c)
                    .abs()
                    .max((perturb(strip.centre()) - m).abs())
                    / c;
                let miss = miss.max((perturb(strip.peak().pressure) - p).abs() / p) / TWIN;
                seen.worst
                    .see(miss, || format!("{name} at {} N/mm", case.q));
            } else {
                seen.beyond += 1;
                let (r0, r1) = residuals(&case, strip.centre(), strip.half_width());
                seen.residual = worse(worse(seen.residual, r0.abs()), r1.abs());
            }
        }
        seen
    }

    /// **A section concave beyond its run is its convex twin's strip wherever the twin's
    /// contact stays inside the run**: the equations read `h′` only on the contact, where the
    /// two are one section. Swept over the ring's section (inside its round to 1640 N/mm; on
    /// the concave flank beyond, to 10⁴ N/mm, where the strip must still solve its own
    /// equations, to `1e-13`) and over the near miss (inside its run to 380 N/mm, with a knot
    /// inside the contact from 44 N/mm, and its bracket reading `σ` past the run from 350), and
    /// over the near miss turned over with a soft step past its run ([`soft_right`]): `c`, `m`
    /// and the peak equal the twin's within [`TWIN`], and nothing is refused. Plant: the previous bracket, convex over `[−2c, 2c]`, which holds
    /// every load of the ring inside its round (a gate on the ring alone passes it: there the
    /// strip is Hertz's) and refuses the near miss and its turned-over twin from 100 N/mm and
    /// the ring beyond its round.
    #[test]
    fn a_section_concave_beyond_its_run_is_its_convex_twins_strip() {
        let seen = twin_law(Profile::width_bracket, |x| x);
        eprintln!(
            "twin: {} inside ({} general), worst {:.3} of the tolerance ({}); {} beyond, worst \
             residual {:.2e}; refused {:?}",
            seen.inside,
            seen.general,
            seen.worst.ratio,
            seen.worst.at,
            seen.beyond,
            seen.residual,
            seen.refused
        );
        assert!(seen.refused.is_empty(), "{:?}", seen.refused);
        assert_eq!(
            (seen.inside, seen.general, seen.beyond),
            (26 + 28 + 21, 14 + 7, 7 + 6 + 6)
        );
        assert_eq!(seen.worst.count, seen.inside);
        assert!(seen.worst.ratio <= 1.0 && seen.residual <= RESIDUAL);
        // The plant: the previous bracket.
        let plant = twin_law(convex_over_twice_the_width, |x| x);
        let ring_inside = |q: f64| q < 1640.0;
        assert!(plant.refused.iter().all(|&(name, q)| {
            name == "ring" && !ring_inside(q) || name != "ring" && q >= 99.0
        }));
        for q in NEAR_MISS_LOADS {
            assert!(plant.refused.contains(&("near miss", q)), "{q}");
        }
        assert_eq!(plant.refused.len(), 7 + (11 + 3 + 4) + 11);
    }

    /// The twin law's tolerance: a few roundings off passes, and ten times it off misses at
    /// every strip compared.
    #[test]
    fn the_twin_tolerance_passes_rounding_and_fails_ten_times_itself() {
        let rounded = twin_law(Profile::width_bracket, |x| x * (1.0 + 4.0 * EPS));
        assert!(rounded.worst.ratio <= 1.0, "{}", rounded.worst.ratio);
        let tenfold = twin_law(Profile::width_bracket, |x| x * (1.0 + 10.0 * TWIN));
        assert_eq!(tenfold.worst.missed, tenfold.inside);
    }

    /// **Hertz off a step.** With every knot at or beyond Hertz's edge the general solve is
    /// Hertz: `c`, `m = 0`, the pressure at 64 angles, the peak, and the load to 16 angles, each
    /// to `1e-12` of Hertz's own scale; and moving the knot across the edge by `1e-9 c` (Hertz's
    /// closed form on one side, the general solve on the other) moves nothing by more. Plant:
    /// the centre sought across `[−c, c]` (the bound the sign of `h′` alone gives) by root
    /// finders stopped at `1e5 ε` of the bracket (`2e-11`), which misses; with the bracket's
    /// own bound on the centre, which closes on Hertz's `m = 0` wherever `σ` is one value
    /// across the bracket's region, the coarse finders miss nothing.
    #[test]
    fn hertz_off_a_step() {
        let anywhere = |p: &Profile, q: f64, e: f64| {
            let b = p.width_bracket(q, e)?;
            Some(WidthBracket { skew: 1.0, ..b })
        };
        let worst_of =
            |resolution: f64, bracket: fn(&Profile, f64, f64) -> Option<WidthBracket>| {
                let mut worst = 0.0_f64;
                for (q, k0, dk) in [(300.0, 0.4, 3.0), (60.0, 0.8, 20.0), (20.0, 1.5, 2.2)] {
                    let ch = (4.0 * q / (PI * STEEL * k0)).sqrt();
                    let (p0, c_h) = ((q * STEEL * k0 / PI).sqrt(), ch);
                    for (side, at) in [(1.0, 1.0), (-1.0, 1.0), (1.0, 1.5), (-1.0, 1.0 + 1e-9)] {
                        let s = side * at * ch;
                        let steps = if side > 0.0 {
                            [CurvatureStep {
                                from: Some(s),
                                to: None,
                                dk,
                            }]
                        } else {
                            [CurvatureStep {
                                from: None,
                                to: Some(s),
                                dk,
                            }]
                        };
                        let profile = Profile::new(k0, &steps).expect("a profile");
                        let g = Strip::solved(&profile, q, STEEL, resolution, bracket)
                            .expect("a strip");
                        worst = worst
                            .max((g.half_width() - c_h).abs() / c_h)
                            .max(g.centre().abs() / c_h);
                        for k in 0..=64 {
                            let th = PI * f64::from(k) / 64.0;
                            let hertz = p0 * th.sin();
                            worst = worst.max((g.pressure(th) - hertz).abs() / p0);
                        }
                        worst = worst.max((g.peak().pressure - p0).abs() / p0);
                        for k in 0..=16 {
                            let th = PI * f64::from(k) / 16.0;
                            let hertz = q * (th - th.sin() * th.cos()) / PI;
                            worst = worst.max((g.load_to(th) - hertz).abs() / q);
                        }
                    }
                    // The knot across the edge: the closed form outside, the general solve inside.
                    for side in [1.0, -1.0] {
                        let strip_at = |at: f64| {
                            let s = side * at * ch;
                            let step = if side > 0.0 {
                                CurvatureStep {
                                    from: Some(s),
                                    to: None,
                                    dk,
                                }
                            } else {
                                CurvatureStep {
                                    from: None,
                                    to: Some(s),
                                    dk,
                                }
                            };
                            let profile = Profile::new(k0, &[step]).expect("a profile");
                            let strip = Strip::new(&profile, q, STEEL).expect("a strip");
                            (strip.knots.len(), strip.peak().pressure, strip.half_width())
                        };
                        let (outside, inside) = (strip_at(1.0 + 1e-9), strip_at(1.0 - 1e-9));
                        assert_eq!((outside.0, inside.0), (0, 1));
                        worst = worst
                            .max((outside.1 - inside.1).abs() / p0)
                            .max((outside.2 - inside.2).abs() / c_h);
                    }
                }
                worst
            };
        let worst = worst_of(ROOT_RESOLUTION, Profile::width_bracket);
        let coarse = worst_of(1e5, Profile::width_bracket);
        let plant = worst_of(1e5, anywhere);
        eprintln!(
            "Hertz off a step: worst {worst:.2e}; at 1e5 ε roots {coarse:.2e}, the centre \
             across [−c, c] {plant:.2e}"
        );
        assert!(worst <= 1e-12 && coarse <= 1e-12, "{worst:e} {coarse:e}");
        assert!(plant > 1e-12, "{plant:e}");
    }

    /// **The load is the pressure's integral.** [`Strip::load_to`], a closed form, equals the
    /// pressure integrated by tanh-sinh piece by piece between knots to `1e-13` of the load, at
    /// 33 angles of each of the 491 random strips and at each of its knots; Hertz's share
    /// `(θ − sin θ cos θ)/π` to `1e-15`; and the whole is `q` to `1e-13`. Plant: the prototype's
    /// rule, Gauss–Legendre 12 between knots, misses (`x ln x` at the pieces' ends).
    #[test]
    fn the_load_is_the_pressures_integral() {
        let (mut worst, mut twelve, mut whole) = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut count = 0;
        for (case, strip) in random_strips() {
            let q = strip.load();
            whole = whole.max((q - case.q).abs() / case.q);
            let angles = (0..=32)
                .map(|k| PI * f64::from(k) / 32.0)
                .chain(strip.cuts());
            for th in angles {
                let quad = load_by_quadrature(&strip, th);
                worst = worst.max((strip.load_to(th) - quad).abs() / q);
                twelve = twelve.max((load_by_twelve(&strip, 0.0, th) - quad).abs() / q);
                count += 1;
            }
        }
        let mut hertz = 0.0_f64;
        let profile = Profile::new(0.4, &[]).expect("a profile");
        let strip = Strip::new(&profile, 300.0, STEEL).expect("a strip");
        for k in 0..=64 {
            let th = PI * f64::from(k) / 64.0;
            let exact = (th - th.sin() * th.cos()) / PI;
            hertz = hertz.max((strip.load_to(th) / 300.0 - exact).abs());
        }
        assert!(count > 491 * 33);
        eprintln!(
            "load: {count} angles, worst {worst:.2e} of q; GL 12 {twelve:.2e}; whole {whole:.2e}; \
             Hertz {hertz:.2e}"
        );
        assert!(worst <= 1e-13 && whole <= 1e-13 && hertz <= 1e-15);
        assert!(twelve > 10.0 * SHARE_ABS);
    }

    /// The regions' maxima by a dense scan, sharing no search with the port: 4000 samples a
    /// region, then the same scan again within the best sample's two neighbours, four times
    /// (each narrowing `2000×`, so the last spacing is `π/(4000·2000⁴) ≈ 5e-17`, below the
    /// angle's `ε`).
    fn dense(strip: &Strip, a: f64, b: f64) -> (f64, f64) {
        let n = 4000;
        let mut best = (strip.pressure(a), a);
        let (mut lo, mut hi) = (a, b);
        for _ in 0..5 {
            for k in 0..=n {
                let th = lo + (hi - lo) * f64::from(k) / f64::from(n);
                let v = strip.pressure(th);
                if v > best.0 {
                    best = (v, th);
                }
            }
            let h = (hi - lo) / f64::from(n);
            (lo, hi) = ((best.1 - h).max(a), (best.1 + h).min(b));
        }
        best
    }

    /// **Golden section finds a maximum to its argument's resolution**, and its round count is
    /// the fewest that reach it: on `−|θ − θ₀|` (a kink, so the value is no flatter than the
    /// argument) at `θ₀` spread over `[0, π]`, the argument within `4ε·π`, the value the best it
    /// evaluated, and the bracket after
    /// [`golden_rounds`] at most `ε·scale` where one round fewer is not. Plant: a search stopped
    /// at `√ε` (the textbook stop) misses by `1e-8`; one asked for the minimum lands on an end.
    #[test]
    fn golden_section_finds_the_peak_to_resolution() {
        let r = golden();
        let mut ran = 0;
        for k in 1..64 {
            let x0 = PI * f64::from(k) / 64.0 + 1e-3 * f64::from(k % 7);
            let seen = std::cell::Cell::new(f64::NEG_INFINITY);
            let f = |x: f64| {
                let v = -(x - x0).abs();
                seen.set(seen.get().max(v));
                v
            };
            let (x, fx) = golden_section(&f, 0.0, PI);
            assert!((x - x0).abs() <= 4.0 * EPS * PI, "{x} {x0}");
            // The best point it evaluated is the one it returns.
            assert_eq!(fx, seen.get());
            assert_eq!(fx, -(x - x0).abs());
            ran += 1;
        }
        assert_eq!(ran, 63);
        for (w, scale) in [(PI, PI), (1e-3, 2.0), (1e-9, 1.0), (3.0, 1e3)] {
            let n = golden_rounds(w, scale);
            let after = |n: u32| w * r.powi(i32::try_from(n).expect("a small count"));
            assert!(
                after(n) <= EPS * scale && after(n - 1) > EPS * scale,
                "{w} {scale} {n}"
            );
        }
        assert_eq!(golden_rounds(EPS, 1.0), 0);
        // Off the origin, its work is those rounds and no more: one evaluation a round, two to
        // start.
        let calls = std::cell::Cell::new(0_u32);
        let (lo, hi) = (2.0, 2.0 + 1e-3);
        let (x, _) = golden_section(
            &|x: f64| {
                calls.set(calls.get() + 1);
                -(x - 2.000_3).abs()
            },
            lo,
            hi,
        );
        assert!((x - 2.000_3).abs() <= 4.0 * EPS * hi);
        assert_eq!(calls.get(), golden_rounds(hi - lo, hi) + 2);
        // Plants: a search stopped at √ε, and one that keeps the lesser point.
        let stopped = |f: &dyn Fn(f64) -> f64, lo: f64, hi: f64, keep_greater: bool| {
            let (mut a, mut b) = (lo, hi);
            while b - a > EPS.sqrt() * hi {
                let (x1, x2) = (b - r * (b - a), a + r * (b - a));
                if (f(x1) >= f(x2)) == keep_greater {
                    b = x2;
                } else {
                    a = x1;
                }
            }
            0.5 * (a + b)
        };
        let x0 = 1.234_567;
        let f = |x: f64| -(x - x0).abs();
        assert!((stopped(&f, 0.0, PI, true) - x0).abs() > 1e3 * EPS * PI);
        assert!((stopped(&f, 0.0, PI, false) - x0).abs() > 1.0);
    }

    /// **Where Hertz holds, the strip is Hertz's to the bit**: on a section with no knot in its
    /// width, `Strip::new` returns the closed form `c = √(4q/(πE*k))`, `m = 0` exactly, and the
    /// general solve (the same strip without the closed form) agrees to `1e-13`. Plant: the
    /// general solve returned in its place, which the `1e-13` alone passes and the identity does
    /// not.
    #[test]
    fn where_hertz_holds_the_strip_is_hertzs() {
        let mut identical_general = 0;
        let mut ran = 0;
        for (q, k0) in [
            (300.0, 0.4),
            (60.0, 0.8),
            (20.0, 1.5),
            (1.0, 7.0),
            (900.0, 0.05),
        ] {
            let c = (4.0 * q / (PI * STEEL * k0)).sqrt();
            for steps in [
                vec![],
                vec![CurvatureStep {
                    from: Some(2.0 * c),
                    to: None,
                    dk: 3.0,
                }],
            ] {
                let profile = Profile::new(k0, &steps).expect("a profile");
                let s = Strip::new(&profile, q, STEEL).expect("a strip");
                assert_eq!((s.half_width(), s.centre()), (c, 0.0), "{q} {k0}");
                let g = Strip::general(&profile, q, STEEL).expect("a strip");
                assert!((g.half_width() / c - 1.0).abs() <= 1e-13 && g.centre().abs() <= 1e-13 * c);
                identical_general += usize::from(g.half_width() == c && g.centre() == 0.0);
                ran += 1;
            }
        }
        assert_eq!(ran, 10);
        // The plant: the general solve is not the closed form to the bit on every one.
        assert!(identical_general < ran, "{identical_general}");
    }

    /// Round five's search (`trace_round5.py`'s `contact2d`): 48 samples at the midpoints of
    /// equal parts of the region, the two best local maxima refined within their neighbours.
    fn round_five(strip: &Strip, a: f64, b: f64) -> (f64, f64) {
        let n = 48;
        let xs: Vec<f64> = (0..n)
            .map(|k| a + (b - a) * (f64::from(k) + 0.5) / f64::from(n))
            .collect();
        let ps: Vec<f64> = xs.iter().map(|&x| strip.pressure(x)).collect();
        let last = xs.len() - 1;
        let mut peaks: Vec<usize> = (0..=last)
            .filter(|&i| (i == 0 || ps[i] >= ps[i - 1]) && (i == last || ps[i] >= ps[i + 1]))
            .collect();
        peaks.sort_by(|&i, &j| ps[j].total_cmp(&ps[i]));
        let mut best = (ps[0], xs[0]);
        for &i in peaks.iter().take(2) {
            let lo = if i > 0 { xs[i - 1] } else { a };
            let hi = if i < last { xs[i + 1] } else { b };
            let (x, v) = golden_section(&|th| strip.pressure(th), lo, hi);
            for (v, x) in [(v, x), (ps[i], xs[i])] {
                if v > best.0 {
                    best = (v, x);
                }
            }
        }
        best
    }

    /// The report's region maxima against [`dense`]: the worst `report/dense − 1` either way.
    fn against_dense(
        strips: &[(Case, Strip)],
        search: impl Fn(&Strip, f64, f64) -> (f64, f64),
    ) -> (f64, usize) {
        let mut worst = 0.0_f64;
        let mut regions = 0;
        for (case, strip) in strips {
            let report = strip.report_by(case.flank, &search);
            let reference = strip.report_by(case.flank, dense);
            let (whole, whole_dense) = (search(strip, 0.0, PI).0, dense(strip, 0.0, PI).0);
            for (r, d) in [
                (Some(whole), Some(whole_dense)),
                (report.flank, reference.flank),
                (report.edge, reference.edge),
                (Some(report.peak.pressure), Some(reference.peak.pressure)),
            ] {
                if let (Some(r), Some(d)) = (r, d) {
                    regions += 1;
                    worst = worst.max((r / d - 1.0).abs());
                }
            }
        }
        (worst, regions)
    }

    /// The verifier's two-round strip (contact-verify6, its maxima section: both members' rounds in one strip),
    /// its round's start swept over `[−0.1033, −0.1032]` in 200 steps, with the two points
    /// `8e-17` apart that round five read 2.35 % apart.
    fn two_round_sweep() -> Vec<(Case, Strip)> {
        let at = |s: f64| Case {
            q: 200.0,
            k0: 0.5,
            steps: vec![
                CurvatureStep {
                    from: Some(s),
                    to: Some(s + 0.06),
                    dk: 10.0,
                },
                CurvatureStep {
                    from: Some(-0.09),
                    to: Some(-0.04),
                    dk: 8.0,
                },
            ],
            flank: Some(FlankPart {
                lo: Some(-0.04),
                hi: Some(s),
            }),
        };
        let starts = (0..=200)
            .map(|i| -0.1033 + 1e-4 * f64::from(i) / 200.0)
            .chain([-0.103_262_572_252_744_6, -0.103_262_572_252_744_52]);
        starts
            .map(|s| {
                let case = at(s);
                let strip = case.strip();
                (case, strip)
            })
            .collect()
    }

    /// **The regions' maxima are a dense reference's** to `1e-12` on the 491 random strips (round
    /// six measured `5e-14`) and the verifier's two-round strip swept across its round's start:
    /// both regions, the larger of them, and the whole strip searched at once. Plant: round
    /// five's search (48 samples, the two best local maxima refined), which holds every random
    /// strip and misses the two-round one.
    #[test]
    fn region_maxima_are_a_dense_references() {
        let random = random_strips();
        let sweep = two_round_sweep();
        let (worst, regions) = against_dense(&random, Strip::peak_on);
        let (worst_sweep, regions_sweep) = against_dense(&sweep, Strip::peak_on);
        let (five, _) = against_dense(&random, round_five);
        let (five_sweep, _) = against_dense(&sweep, round_five);
        eprintln!(
            "{} random strips, {regions} regions: worst {worst:.2e} (round five {five:.2e}); the \
             two-round sweep, {regions_sweep} regions: {worst_sweep:.2e} (round five \
             {five_sweep:.2e})",
            random.len()
        );
        assert!(regions > random.len() && regions_sweep >= sweep.len());
        assert!(
            worst <= 1e-12 && worst_sweep <= 1e-12,
            "{worst:e} {worst_sweep:e}"
        );
        assert!(five <= 1e-12 && five_sweep > 1e-12);
    }

    /// **The boundary elements agree.** The peak pressure and the edge share of nine strips
    /// against contact-verify5's BEM (`tests/data/field_bem/`, its README), within ten times
    /// each figure's own uncertainty and never looser than `1e-5` (the porting plan's gate),
    /// figures whose uncertainty is above `1e-6` of them not read. Plant: the prototype's edge
    /// share (Gauss–Legendre 12 between knots) misses, and on one strip by less than `1e-5`: the
    /// plan's bound alone would pass it.
    #[test]
    fn the_boundary_elements_agree() {
        let file = bem_file();
        let strips = file["strips"].as_array().expect("strips");
        assert_eq!(strips.len(), 9);
        let (mut used, mut worst) = (0, 0.0_f64);
        let (mut twelve_misses, mut twelve_near) = (0, 0);
        for s in strips {
            let case = Case {
                q: num(&s["q"]),
                k0: num(&s["k0"]),
                steps: steps_of(&s["steps"]),
                flank: flank_of(&s["flank"]),
            };
            assert_eq!(num(&s["Es"]), STEEL);
            let strip = case.strip();
            let report = strip.report(case.flank);
            let twelve = edge_share_by_twelve(&strip, case.flank);
            for (key, port, plant) in [
                ("pmax", report.peak.pressure, None),
                ("fedge", report.edge_share, Some(twelve)),
            ] {
                let (value, unc) = (num(&s[key]["value"]), num(&s[key]["uncertainty"]));
                if unc > 1e-6 * value.abs().max(1.0) {
                    continue;
                }
                used += 1;
                let bound = (10.0 * unc).max(EPS).min(1e-5 * value.abs().max(1.0));
                let r = (port - value).abs() / bound;
                worst = worst.max(r);
                assert!(r <= 1.0, "{} {key}: {port} vs {value} ± {unc}", s["name"]);
                if let Some(p) = plant {
                    let d = (p - value).abs();
                    twelve_misses += usize::from(d > bound);
                    twelve_near += usize::from(d > bound && d < 1e-5);
                }
            }
        }
        eprintln!(
            "BEM: {used} figures, worst {worst:.3} of the bound; GL 12 misses {twelve_misses} \
             ({twelve_near} within 1e-5)"
        );
        assert_eq!(used, 17);
        assert!(twelve_misses > 0 && twelve_near > 0);
    }

    // ------------------------------------------------------------------ creep

    /// Carter's law, `1 − (1 − x)²` below 1.
    fn carter(x: f64) -> f64 {
        let x = x.min(1.0);
        1.0 - (1.0 - x) * (1.0 - x)
    }

    /// **The creep curve is Carter's on Hertz**, `Q/μP = 1 − (1 − ξ/ξ*)²` with `ξ* = μ c k`, to
    /// `1e-15` at 201 slips both ways on three Hertz strips; full stick at `ξ = 0`, `Q = 0` and
    /// full slip at `ξ*`. Plant: the zone's `I₀′/π` without the rolling direction's sense (a curve
    /// read in one direction only passes it): the lower leading edge misses.
    #[test]
    fn the_creep_curve_is_carters_on_hertz() {
        let worst_of = |signed: bool| {
            let mut worst = 0.0_f64;
            for (q, k) in [(300.0, 0.4), (60.0, 0.8), (500.0, 12.0)] {
                let profile = Profile::new(k, &[]).expect("a profile");
                let strip = Strip::new(&profile, q, STEEL).expect("a strip");
                let star = strip.half_width() * k;
                for lead in [LeadingEdge::Upper, LeadingEdge::Lower] {
                    for i in 0..=200 {
                        let s = f64::from(i) / 200.0;
                        let p = strip.creep(lead, s).expect("a slip");
                        let xi = if signed {
                            p.creepage
                        } else {
                            p.creepage * lead.sign()
                        };
                        worst = worst.max((p.traction - carter(xi / star)).abs());
                    }
                }
            }
            worst
        };
        let worst = worst_of(true);
        let unsigned = worst_of(false);
        eprintln!("creep on Hertz: worst {worst:.2e}; unsigned {unsigned:.2e}");
        assert!(worst <= 1e-15, "{worst:e}");
        assert!(unsigned > 1e-15);
    }

    /// **The exact creep curve is monotone**: `ξ` and `Q` rise with the slip zone on the 491
    /// random strips both ways, at 400 slips and 100 more within `1e-5` of full slip (where `Q`
    /// rises by `2(1 − s) Δs`, down to `1e-14` a step), each step down at most two roundings of
    /// its scale; `Q` runs from 0 at full stick to 1 at full slip. Plant: the curve with an
    /// alternating ripple of `1e-12`, which round six's check (`1e-9` allowed) passed.
    #[test]
    fn the_exact_creep_curve_is_monotone() {
        let slips: Vec<f64> = (0..400)
            .map(|i| f64::from(i) / 400.0)
            .chain((0..=100).map(|i| 1.0 - 1e-5 * (1.0 - f64::from(i) / 100.0)))
            .collect();
        let falls = |ripple: f64| {
            let mut falls = 0;
            let mut ends = 0.0_f64;
            for (_, strip) in random_strips() {
                for lead in [LeadingEdge::Upper, LeadingEdge::Lower] {
                    let curve: Vec<(f64, f64)> = slips
                        .iter()
                        .enumerate()
                        .map(|(i, &s)| {
                            let p = strip.creep(lead, s).expect("a slip");
                            let sign = if i % 2 == 0 { 1.0 } else { -1.0 };
                            (p.creepage, p.traction + sign * ripple)
                        })
                        .collect();
                    let full = curve[curve.len() - 1];
                    for w in curve.windows(2) {
                        let (a, b) = (w[0], w[1]);
                        falls += usize::from(b.0 < a.0 - 2.0 * EPS * full.0.abs());
                        falls += usize::from(b.1 < a.1 - 2.0 * EPS);
                    }
                    ends = ends
                        .max(curve[0].1.abs() - ripple)
                        .max((full.1 - 1.0).abs());
                }
            }
            (falls, ends)
        };
        let (smooth, ends) = falls(0.0);
        let (rippled, _) = falls(1e-12);
        eprintln!("creep: {smooth} falls, ends {ends:.2e}; rippled {rippled} falls");
        assert_eq!(smooth, 0);
        assert!(ends <= 4.0 * EPS);
        assert!(rippled > 0);
    }

    // ------------------------------------------------------------------ a panel's friction

    /// Panels: random sliding (half with `B` a component-wise multiple of `A`), and every third a
    /// spur line's, `|B|` `1e-16 … 1e-12 |A|` parallel to `A` (with or without `1e-17` noise),
    /// `s*` within a factor `√10` of `|A|`.
    fn panels() -> Vec<(Vec3, Vec3, f64, bool)> {
        let mut d = Draw(3);
        (0..3000)
            .map(|i| {
                let a: Vec3 = [
                    d.uniform(-1.0, 1.0),
                    d.uniform(-1.0, 1.0),
                    d.uniform(-1.0, 1.0),
                ];
                if i % 3 == 0 {
                    let e = 10f64.powf(d.uniform(-16.0, -12.0));
                    let sign = if d.next() < 0.5 { 1.0 } else { -1.0 };
                    let noise = if i % 2 == 1 { 1e-17 } else { 0.0 };
                    let b = a.map(|x| x * e * sign + noise * d.uniform(-1.0, 1.0));
                    let s = norm(a) * 10f64.powf(d.uniform(-0.5, 0.5));
                    (a, b, s, true)
                } else {
                    let b: Vec3 = if d.next() < 0.5 {
                        a.map(|x| x * d.uniform(-3.0, 3.0))
                    } else {
                        [
                            d.uniform(-2.0, 2.0),
                            d.uniform(-2.0, 2.0),
                            d.uniform(-2.0, 2.0),
                        ]
                    };
                    (a, b, d.uniform(0.05, 2.0), false)
                }
            })
            .collect()
    }

    /// The halvings a graded piece takes toward each of its ends: its innermost interval is
    /// `2^−60 ≈ 1e-18` of it, where a bounded integrand holds less than `1e-18`, and every
    /// interval but that one lies at least its own length from the end, so the transition of
    /// width `|h|/|B|` at the foot is resolved at any offset.
    const GRADES: i32 = 60;

    /// `[p, q]` in intervals graded geometrically toward both ends: each half split at `2^−j`
    /// of its length from its end, `j ≤` [`GRADES`].
    fn graded(p: f64, q: f64) -> Vec<(f64, f64)> {
        let half = 0.5 * (q - p);
        [(p, 1.0), (q, -1.0)]
            .into_iter()
            .flat_map(|(end, dir)| {
                let at = move |j: i32| end + dir * half * 0.5f64.powi(j);
                (0..GRADES)
                    .map(move |j| (at(j + 1), at(j)))
                    .chain(std::iter::once((at(GRADES), end)))
                    .map(|(x, y)| (x.min(y), x.max(y)))
            })
            .collect()
    }

    /// `[p, q]` in eight equal parts: the previous reference (with Gauss–Legendre 64), which
    /// misses the panels near the foot by up to `1e-8`.
    fn in_eight(p: f64, q: f64) -> Vec<(f64, f64)> {
        (0..8)
            .map(|k| {
                let at = |k: i32| p + (q - p) * f64::from(k) / 8.0;
                (at(k), at(k + 1))
            })
            .collect()
    }

    /// Both laws' means by quadrature: Gauss–Legendre 30 on each piece of `[−½, ½]` split at
    /// the window's ends and at the foot of the zero of sliding, [`graded`] toward both ends.
    fn slides_by_quadrature(a: Vec3, b: Vec3, s: f64) -> (Slide, Slide) {
        slides_on(a, b, s, graded, 30)
    }

    /// Both laws' means by Gauss–Legendre `nodes` on the intervals `parts` makes of each piece
    /// of `[−½, ½]` between the window's ends and the foot of the zero of sliding.
    fn slides_on(
        a: Vec3,
        b: Vec3,
        s: f64,
        parts: fn(f64, f64) -> Vec<(f64, f64)>,
        nodes: usize,
    ) -> (Slide, Slide) {
        let mut cuts = vec![-0.5, 0.5];
        let b2 = dot(b, b);
        if b2 > 0.0 {
            let u0 = -dot(a, b) / b2;
            let h2 = dot(add(a, b, u0), add(a, b, u0));
            cuts.push(u0);
            if s * s > h2 {
                let half = ((s * s - h2) / b2).sqrt();
                cuts.extend([u0 - half, u0 + half]);
            }
        }
        cuts.retain(|&u| (-0.5..=0.5).contains(&u));
        cuts.sort_by(f64::total_cmp);
        let rule = gauss_legendre_rule(nodes);
        // `[Coulomb's direction, speed, Carter's direction, speed]`, summed node by node into
        // each interval, interval by interval into each piece, and piece by piece: the
        // rounding grows with the longest of the three sums, not with their product.
        type Sums = [f64; 8];
        let plus = |x: Sums, y: Sums| -> Sums { std::array::from_fn(|i| x[i] + y[i]) };
        let interval = |lo: f64, hi: f64| -> Sums {
            let (h, mid) = (0.5 * (hi - lo), 0.5 * (lo + hi));
            rule.iter().fold([0.0; 8], |sum, &(x, wt)| {
                let v = add(a, b, mid + h * x);
                let r = norm(v);
                if r == 0.0 {
                    return sum;
                }
                let f = carter(r / s);
                let (d, k) = (h * wt / r, h * wt * f / r);
                let node = [
                    v[0] * d,
                    v[1] * d,
                    v[2] * d,
                    h * wt * r,
                    v[0] * k,
                    v[1] * k,
                    v[2] * k,
                    h * wt * f * r,
                ];
                plus(sum, node)
            })
        };
        let t = cuts.windows(2).fold([0.0; 8], |total, w| {
            let piece = parts(w[0], w[1])
                .into_iter()
                .fold([0.0; 8], |sum, (lo, hi)| plus(sum, interval(lo, hi)));
            plus(total, piece)
        });
        (
            Slide {
                direction: [t[0], t[1], t[2]],
                speed: t[3],
            },
            Slide {
                direction: [t[4], t[5], t[6]],
                speed: t[7],
            },
        )
    }

    /// Round six's moments: the window's differences taken directly wherever the zero of
    /// sliding is, about its foot (the cancellation round seven removed).
    fn carter_slide_direct(a: Vec3, b: Vec3, sstar: f64) -> Slide {
        let foot = Foot::of(a, b, OFFSET_FLOOR).expect("B ≠ 0");
        let (w1, w2, r1, r2, b2) = (foot.w1, foot.w2, foot.r1, foot.r2, foot.b2);
        let h2 = foot.hn * foot.hn;
        let da = if foot.hn > 0.0 {
            (foot.b * w2 / foot.hn).asinh() - (foot.b * w1 / foot.hn).asinh()
        } else {
            0.0
        };
        let dwr = w2 * r2 - w1 * r1;
        let dr3 = (r2 * r2 * r2 - r1 * r1 * r1) / (3.0 * b2);
        let dwr3 = w2 * r2 * r2 * r2 - w1 * r1 * r1 * r1;
        let ir = 0.5 * dwr + h2 * da / (2.0 * foot.b);
        let ir3 = 0.25 * dwr3 + 0.375 * h2 * dwr + 0.375 * h2 * h2 * da / foot.b;
        let m1 = add(scale(b, dr3), foot.h, ir);
        let ss = sstar;
        let inside = (dot(a, a).sqrt() < ss) && (add(a, b, 0.5)).iter().all(|x| x.is_finite());
        if !inside {
            return carter_slide(a, b, sstar);
        }
        Slide {
            direction: add(scale(a, 2.0 / ss), m1, -1.0 / (ss * ss)),
            speed: 2.0 * (dot(a, a) + b2 / 12.0) / ss - ir3 / (ss * ss),
        }
    }

    /// The greater of two misses, a miss that is not a number kept (`f64::max` drops it).
    fn worse(a: f64, b: f64) -> f64 {
        if b > a || b.is_nan() {
            b
        } else {
            a
        }
    }

    fn slide_miss(p: Slide, q: Slide) -> f64 {
        (0..3)
            .map(|k| (p.direction[k] - q.direction[k]).abs())
            .fold((p.speed - q.speed).abs(), worse)
    }

    /// Panels with the zero of sliding on them a small offset off the origin, where the offset's
    /// `asinh` terms carry the means: `B` random, the foot `u₀` on the panel, `A = −u₀ B + δ |B|
    /// e` (`e ⊥ B` a unit vector) with `δ = |h|/|B|` log-uniform on `[1e-16, 1e-3]`; every fourth
    /// with `h = 0` exactly (`B` and `u₀` dyadic, so `A`, the foot and the offset are exact);
    /// `s*` log-uniform on `[1e-3, 2] |B|`.
    fn near_foot_panels() -> Vec<(Vec3, Vec3, f64)> {
        let mut d = Draw(11);
        (0..800)
            .map(|i| {
                let (a, b) = if i % 4 == 0 {
                    let mut eighths = || (d.uniform(-16.0, 16.0)).round() / 8.0;
                    let b: Vec3 = [1.0 + eighths().abs(), eighths(), eighths()];
                    let u0 = d.uniform(-28.0, 28.0).round() / 64.0;
                    (b.map(|x| -u0 * x), b)
                } else {
                    let mut unit = || d.uniform(-2.0, 2.0);
                    let b: Vec3 = [unit(), unit(), unit()];
                    let e = cross(b, [unit(), unit(), unit()]);
                    let e = scale(e, 1.0 / norm(e));
                    let u0 = d.uniform(-0.45, 0.45);
                    let delta = 10f64.powf(d.uniform(-16.0, -3.0));
                    (add(scale(b, -u0), e, delta * norm(b)), b)
                };
                let s = norm(b) * 10f64.powf(d.uniform(-3.0, 0.3));
                (a, b, s)
            })
            .collect()
    }

    /// The checker's plant on the offset's terms: dropped at `|h| ≤ 1e-8 |B|`.
    const PLANT_FLOOR: f64 = 1e-8;

    /// **Both slides are their quadrature**, split at the window and the zero of sliding and
    /// graded toward each cut ([`graded`]), to `1e-13`: on 3000 panels, a thousand of them a
    /// spur line's, and 800 with the zero of sliding on the panel at `|h|/|B|` from `1e-16` to
    /// `1e-3` (200 at `h = 0` exactly). Plants: round six's window integrals, taken about the
    /// foot of the origin, which hold every panel but the spur lines'; and the offset's terms
    /// dropped at `|h| ≤ 1e-8 |B|` ([`PLANT_FLOOR`]), which holds all 3000 (a gate on them alone
    /// passes it) and misses near the foot. The reference's own: the previous one, eight equal
    /// parts a piece, misses near the foot, where the graded one resolves the transition.
    #[test]
    fn both_slides_are_their_quadrature() {
        let (mut worst, mut direct_spur, mut direct_other) = (0.0_f64, 0.0_f64, 0.0_f64);
        let mut floor_far = 0.0_f64;
        let mut windows = 0;
        let both = |a: Vec3, b: Vec3, s: f64, floor: f64| {
            (panel_slide_to(a, b, floor), carter_slide_to(a, b, s, floor))
        };
        let miss = |(p, q): (Slide, Slide), (c, k): (Slide, Slide)| {
            worse(slide_miss(p, c), slide_miss(q, k))
        };
        let all = panels();
        assert_eq!(all.len(), 3000);
        for (a, b, s, spur) in all {
            let reference = slides_by_quadrature(a, b, s);
            worst = worse(
                worst,
                miss((panel_slide(a, b), carter_slide(a, b, s)), reference),
            );
            floor_far = worse(floor_far, miss(both(a, b, s, PLANT_FLOOR), reference));
            let (_, creep) = reference;
            let whole_window = norm(a) < s && norm(add(a, b, 0.5)) < s && norm(add(a, b, -0.5)) < s;
            if whole_window {
                windows += 1;
                let d = slide_miss(carter_slide_direct(a, b, s), creep);
                if spur {
                    direct_spur = worse(direct_spur, d);
                } else {
                    direct_other = worse(direct_other, d);
                }
            }
        }
        let (mut near, mut floor_near, mut in_parts) = (0.0_f64, 0.0_f64, 0.0_f64);
        let (mut on_panel, mut exact) = (0, 0);
        for (a, b, s) in near_foot_panels() {
            let foot = Foot::of(a, b, OFFSET_FLOOR).expect("B ≠ 0");
            on_panel += usize::from(!foot.one_sided());
            exact += usize::from(foot.hn == 0.0);
            let reference = slides_by_quadrature(a, b, s);
            let port = both(a, b, s, OFFSET_FLOOR);
            near = worse(near, miss(port, reference));
            floor_near = worse(floor_near, miss(both(a, b, s, PLANT_FLOOR), reference));
            in_parts = worse(in_parts, miss(port, slides_on(a, b, s, in_eight, 64)));
        }
        eprintln!(
            "slides: worst {worst:.2e}, near the foot {near:.2e}; {windows} panels inside the \
             window: round six's {direct_other:.2e} off spur lines, {direct_spur:.2e} on them; \
             offset floor {floor_far:.2e}, near the foot {floor_near:.2e}; eight parts near the \
             foot {in_parts:.2e}"
        );
        assert_eq!((on_panel, exact), (800, 200));
        assert!(windows > 100, "{windows}");
        assert!(
            worst <= SLIDE_ABS && near <= SLIDE_ABS,
            "{worst:e} {near:e}"
        );
        assert!(direct_other <= SLIDE_ABS && direct_spur > SLIDE_ABS);
        assert!(floor_far <= SLIDE_ABS && floor_near > 10.0 * SLIDE_ABS);
        assert!(in_parts > 10.0 * SLIDE_ABS);
    }

    // ------------------------------------------------------------------ homogeneity

    /// **The strip is homogeneous**: every length ×10 and ×0.1 (and `q`, a load per length)
    /// leaves `c/k`, `m/k`, every pressure, the edge share and the creep curve equal to `1e-13`
    /// on the 491 random strips. Plant: the root finders' floor absolute in mm (`1e-14`, the
    /// prototype's), which misses at ×0.1.
    #[test]
    fn the_strip_is_homogeneous() {
        let worst_of = |absolute: bool| {
            let mut worst = 0.0_f64;
            let mut count = 0;
            for (case, strip) in random_strips().into_iter().step_by(7) {
                count += 1;
                let base = strip.report(case.flank);
                for k in [10.0, 0.1] {
                    let scaled = case.scaled(k);
                    let profile = Profile::new(scaled.k0, &scaled.steps).expect("a profile");
                    let resolution = if absolute {
                        1e-14 / (EPS * strip.half_width() * k)
                    } else {
                        ROOT_RESOLUTION
                    };
                    let s =
                        Strip::general_to(&profile, scaled.q, STEEL, resolution).expect("a strip");
                    let r = s.report(scaled.flank);
                    let c = base.half_width;
                    let rel = |a: f64, b: f64| (a - b).abs() / b.abs();
                    worst = worst
                        .max(rel(r.half_width / k, c))
                        .max((r.centre / k - base.centre).abs() / c)
                        .max(rel(r.peak.pressure, base.peak.pressure))
                        .max((r.edge_share - base.edge_share).abs());
                    for slip in [0.25, 0.75] {
                        let (p, q) = (
                            s.creep(LeadingEdge::Upper, slip).expect("a slip"),
                            strip.creep(LeadingEdge::Upper, slip).expect("a slip"),
                        );
                        worst = worst
                            .max((p.traction - q.traction).abs())
                            .max((p.creepage - q.creepage).abs() / (c * case.k0));
                    }
                }
            }
            assert_eq!(count, 71);
            worst
        };
        let worst = worst_of(false);
        let absolute = worst_of(true);
        eprintln!("homogeneity: worst {worst:.2e}; an absolute floor {absolute:.2e}");
        assert!(worst <= 1e-13, "{worst:e}");
        assert!(absolute > 1e-13);
    }

    /// **A panel that slides uniformly is its one point**, and the panels about it close on it:
    /// with `B = 0`, Coulomb's direction is `A/|A|` at speed `|A|`, Carter's is `f(|A|/s*)` of
    /// that, `f(x) = 2x − x²` below `1` (a creepage read against `s*`, never against `1`); and a
    /// panel with `B = δ` differs by `O(δ²)` (the mean over `u ∈ [−½, ½]` cancels the odd term),
    /// within `|B|²/|A|²` relative at `δ = 1e-5 |A|`. Where nothing slides the direction is zero.
    /// Plant: Carter's factor read at `|A|` rather than `|A|/s*` fails the point on seven of
    /// the nine (the other two are at full slip both ways).
    #[test]
    fn a_uniform_panel_is_its_point() {
        let carter = |x: f64| if x < 1.0 { 2.0 * x - x * x } else { 1.0 };
        let (mut ran, mut plant_misses) = (0, 0);
        for a in [[0.3, -0.1, 0.0], [2.0, 0.5, -1.0], [1e-7, 0.0, 3e-8]] {
            let na = norm(a);
            let unit = scale(a, 1.0 / na);
            let p = panel_slide(a, [0.0; 3]);
            assert_eq!((p.direction, p.speed), (unit, na));
            for sstar in [0.25 * na, 0.9 * na, 3.0 * na] {
                let c = carter_slide(a, [0.0; 3], sstar);
                let f = carter(na / sstar);
                for (d, u) in c.direction.iter().zip(unit) {
                    assert!((d - f * u).abs() <= 4.0 * EPS);
                }
                assert!((c.speed / (f * na) - 1.0).abs() <= 4.0 * EPS);
                // The plant: the factor read at |A|.
                plant_misses += usize::from((carter(na) - f).abs() > 1e-2);
                let b = scale([0.2, 0.7, -0.4], 1e-5 * na / norm([0.2, 0.7, -0.4]));
                let near = carter_slide(a, b, sstar);
                let bound = 4.0 * (norm(b) / na).powi(2) + 16.0 * EPS;
                assert!((near.speed / c.speed - 1.0).abs() <= bound);
                for (n, d) in near.direction.iter().zip(c.direction) {
                    assert!((n - d).abs() <= bound);
                }
                let near_p = panel_slide(a, b);
                assert!((near_p.speed / na - 1.0).abs() <= bound);
                ran += 1;
            }
        }
        assert_eq!((ran, plant_misses), (9, 7));
        let still = panel_slide([0.0; 3], [0.0; 3]);
        assert_eq!((still.direction, still.speed), ([0.0; 3], 0.0));
        let still = carter_slide([0.0; 3], [0.0; 3], 1.0);
        assert_eq!((still.direction, still.speed), ([0.0; 3], 0.0));
    }

    /// **The strip reads its section only within its bracket's region**: a step far beyond it
    /// (at 5 to 6 mm, stiffening by `50/mm`, or turning the gap's slope back by `−50/mm`) leaves
    /// `c` and `m` the same to the bit, on the near miss at the loads it was refused at and
    /// where its bracket reads past its run, and on the ring beyond its round (1700 and 2000
    /// N/mm). Plant: `σ`'s span read at every knot of the section as well as over the band's
    /// region (a valid, looser bound: the twin and the equations pass it), which moves on
    /// every one.
    #[test]
    fn the_strip_reads_its_section_only_within_its_brackets_region() {
        let whole: fn(&Profile, f64, f64) -> Option<WidthBracket> = |p, q, e| {
            p.width_bracket_by(q, e, |p, r| {
                p.knots
                    .iter()
                    .filter(|&&t| t != 0.0)
                    .map(|&t| p.secant(t))
                    .fold(p.secant_within(r), |(lo, hi), s| (lo.min(s), hi.max(s)))
            })
        };
        let cases: Vec<Case> = NEAR_MISS_LOADS
            .into_iter()
            .chain(NEAR_EDGE_LOADS)
            .map(near_miss)
            .chain([1700.0, 2000.0].map(ring_at))
            .collect();
        let moved = |bracket: fn(&Profile, f64, f64) -> Option<WidthBracket>| {
            let mut moved = 0;
            let mut ran = 0;
            for case in &cases {
                let solve = |c: &Case| {
                    let p = Profile::new(c.k0, &c.steps).expect("a profile");
                    let s = Strip::new_by(&p, c.q, STEEL, bracket).expect("a strip");
                    (s.half_width().to_bits(), s.centre().to_bits())
                };
                let base = solve(case);
                for dk in [50.0, -50.0] {
                    let mut far = case.clone();
                    far.steps.push(CurvatureStep {
                        from: Some(5.0),
                        to: Some(6.0),
                        dk,
                    });
                    moved += usize::from(solve(&far) != base);
                    ran += 1;
                }
            }
            (moved, ran)
        };
        assert_eq!(moved(Profile::width_bracket), (0, 18));
        assert_eq!(moved(whole), (18, 18));
    }

    /// **Each band's lines are the section's slope there**: for every band of the near miss,
    /// [`soft_right`], the ring's section and the 491 random strips, `α + k t` is `h′(t)` and
    /// `α′ + k′(−t)` is `h′(−t)` at 16 points `t` across the band (to the slope's rounding),
    /// the open outermost band included. Plant: the interval holding `from` itself, which is the
    /// one below it where `from` is a knot on the right: right on every band but the one past a
    /// right knot, so a check that skips the outermost band, or the left side, passes it.
    #[test]
    fn each_bands_lines_are_its_slope() {
        let inner = |p: &Profile, from: f64| {
            [
                p.line_on(p.knots.partition_point(|&k| k < from)),
                p.line_on(p.knots.partition_point(|&k| k < -from)),
            ]
        };
        let sections: Vec<Case> = [near_miss(1.0), soft_right(1.0), ring_at(1.0)]
            .into_iter()
            .chain(random_strips().into_iter().map(|(c, _)| c))
            .collect();
        let (mut ran, mut worst, mut plant_misses) = (0, 0.0_f64, 0);
        for case in &sections {
            let p = Profile::new(case.k0, &case.steps).expect("a profile");
            let mut reach: Vec<f64> = p
                .knots
                .iter()
                .filter(|&&t| t != 0.0)
                .map(|t| t.abs())
                .collect();
            reach.sort_by(f64::total_cmp);
            reach.dedup();
            let far = reach.last().copied().unwrap_or_default();
            let starts = std::iter::once(0.0).chain(reach.iter().copied());
            let ends = reach.iter().copied().map(Some).chain(std::iter::once(None));
            for (from, to) in starts.zip(ends) {
                // The open band read over one unit past its start.
                let width = to.map_or(1.0, |to| to - from);
                // Each side's miss over its terms' size: a line is anchored at a knot, so its
                // rounding is the knot's (`far`, the farthest), not `t`'s.
                let side = |(a, k): (f64, f64), t: f64| {
                    let h = p.slope_at(t);
                    (a + k * t - h).abs() / (a.abs() + k.abs() * (t.abs() + far) + h.abs())
                };
                let off =
                    |[right, left]: [(f64, f64); 2], t: f64| worse(side(right, t), side(left, -t));
                let mut plant_off = 0.0_f64;
                for j in 1..=16 {
                    let t = from + width * f64::from(j) / 17.0;
                    worst = worse(worst, off(p.band_lines(from), t));
                    plant_off = worse(plant_off, off(inner(&p, from), t));
                    ran += 1;
                }
                plant_misses += usize::from(plant_off > 1e-12);
            }
        }
        eprintln!(
            "band lines: {ran} points, worst {worst:.2e}; the plant misses {plant_misses} bands"
        );
        assert!(ran > 491 * 16);
        assert!(worst <= 8.0 * EPS, "{worst:e}");
        assert!(plant_misses > 0);
    }

    /// **`reaches` is its quadratic's solution set**: on a grid of `k` (both signs and zero),
    /// `β` (both signs, to `10⁸` against `need`) and `need`, each end of the interval solves
    /// `k s² + β s = need` to rounding of its terms, the quadratic is at least `need` at the
    /// interval's midpoint (or past its start where it runs on) and below it just short of its
    /// start, and `None` comes only where the quadratic stays below `need` for every `s ≥ 0`;
    /// a double root (`β² = −4 k need`) is its one point. Plant: the textbook root
    /// `(−β + √(β² + 4k need))/(2k)`, which loses every digit at `β ≫ √(k need)`.
    #[test]
    fn reaches_is_its_quadratics_solution_set() {
        let f = |k: f64, b: f64, s: f64| k * s * s + b * s;
        let scale = |k: f64, b: f64, s: f64, need: f64| (k * s * s).abs() + (b * s).abs() + need;
        let (mut ran, mut nones) = (0, 0);
        for k in [-3.0, -1e-3, 0.0, 1e-3, 2.0, 7e4] {
            for b in [-1e8, -5.0, -1e-3, 0.0, 1e-3, 0.5, 5.0, 1e8] {
                for need in [1e-9, 1.0, 40.0] {
                    ran += 1;
                    let Some((from, to)) = reaches(k, b, need) else {
                        nones += 1;
                        // None: the quadratic's greatest over `s ≥ 0` is short of `need`.
                        let top = if k < 0.0 && b > 0.0 {
                            -b * b / (4.0 * k)
                        } else {
                            0.0
                        };
                        assert!(k <= 0.0 && top < need, "{k} {b} {need}");
                        continue;
                    };
                    for end in std::iter::once(from).chain(to) {
                        let r = (f(k, b, end) - need).abs() / scale(k, b, end, need);
                        assert!(r <= 8.0 * EPS, "{k} {b} {need}: {end} {r:e}");
                    }
                    let inside = to.map_or(from * 2.0 + 1.0, |to| 0.5 * (from + to));
                    assert!(f(k, b, inside) >= need, "{k} {b} {need}");
                    assert!(f(k, b, from * (1.0 - 1e-6)) < need, "{k} {b} {need}");
                }
            }
        }
        assert_eq!((ran, nones), (144, 43));
        assert_eq!(reaches(-1.0, 2.0, 1.0), Some((1.0, Some(1.0))));
        // The plant: the textbook root at k = 1, β = 1e8, need = 1 (the root is 1e-8).
        let textbook = (-1e8 + (1e16_f64 + 4.0).sqrt()) / 2.0;
        let (root, _) = reaches(1.0, 1e8, 1.0).expect("a root");
        assert!((root * 1e8 - 1.0).abs() <= 4.0 * EPS);
        assert!((textbook * 1e8 - 1.0).abs() > 0.1, "{textbook:e}");
    }

    /// **`skew` bounds the centre, and closely**: it is the greatest `|⟨cos φ⟩|` over `[0, π]`
    /// with weights within a ratio `ρ`, attained by the weight `ρ` on `[0, S*]` and `1` beyond,
    /// so it is at least that step's mean (to rounding) and at least the mean of 2000 random
    /// weights within `ρ`, and no more than the greatest of the step means over 4000 `S`;
    /// `0` at `ρ = 1`, `1` as `ρ → ∞`, monotone between. Plant: the weight split at `cos φ = 0`
    /// (`(2/π)(ρ − 1)/(ρ + 1)`), a bound the step's mean exceeds wherever `ρ − 1` is felt (six
    /// of the eight).
    #[test]
    fn skew_bounds_the_centre() {
        // The step weight's mean of cos φ: ρ on [0, S], 1 beyond.
        let step = |rho: f64, s: f64| (rho - 1.0) * s.sin() / (PI + (rho - 1.0) * s);
        let mut d = Draw(17);
        let (mut ran, mut below_split) = (0, 0);
        let mut last = 0.0;
        for rho in [1.0 + 1e-14, 1.0 + 1e-9, 1.01, 1.269, 2.0, 10.0, 1e3, 1e8] {
            let mu = skew(rho);
            assert!(mu >= last && mu <= 1.0, "{rho}");
            last = mu;
            let best = (0..=4000)
                .map(|i| step(rho, PI * f64::from(i) / 4000.0))
                .fold(0.0, f64::max);
            assert!(
                mu >= best - 4.0 * EPS && mu <= best + 1e-6 * best + EPS,
                "{rho}: {mu} {best}"
            );
            // S* from its equation, and the step there.
            let s_star = mu.acos();
            assert!(
                (step(rho, s_star) - mu).abs() <= 1e-9 * mu + 4.0 * EPS,
                "{rho}"
            );
            for _ in 0..2000 {
                // A random weight within ρ, piecewise constant on 16 parts of [0, π].
                let w: Vec<f64> = (0..16).map(|_| 1.0 + (rho - 1.0) * d.next()).collect();
                let part = |j: usize| {
                    #[expect(clippy::cast_precision_loss, reason = "a part index")]
                    let j = j as f64;
                    PI * j / 16.0
                };
                let (mut num, mut den) = (0.0, 0.0);
                for (j, wj) in w.iter().enumerate() {
                    num += wj * (part(j + 1).sin() - part(j).sin());
                    den += wj * PI / 16.0;
                }
                assert!((num / den).abs() <= mu + 4.0 * EPS, "{rho}");
                ran += 1;
            }
            let split = 2.0 / PI * (rho - 1.0) / (rho + 1.0);
            below_split += usize::from(split < best - 1e-12);
        }
        assert_eq!(ran, 8 * 2000);
        assert_eq!(skew(1.0), 0.0);
        assert_eq!(skew(f64::INFINITY), 1.0);
        // The plant: the split at cos φ = 0 is below the step's mean wherever ρ > 1 is felt.
        assert_eq!(below_split, 6);
    }

    /// What is no strip is refused: a load or modulus that is not a finite number `> 0`, a
    /// step that runs backwards or holds no number, a section concave at its origin, and one
    /// whose valley (where the gap's slope has the sign of `t`) ends before any window carries
    /// the load.
    #[test]
    fn what_is_no_strip_is_refused() {
        let p = Profile::new(0.5, &[]).expect("a profile");
        for (q, e) in [(0.0, STEEL), (-1.0, STEEL), (f64::NAN, STEEL), (10.0, 0.0)] {
            assert!(Strip::new(&p, q, e).is_none(), "{q} {e}");
        }
        let step = |from, to, dk| CurvatureStep { from, to, dk };
        assert!(Profile::new(0.5, &[step(Some(1.0), Some(0.5), 1.0)]).is_none());
        assert!(Profile::new(0.5, &[step(Some(0.0), Some(0.5), f64::NAN)]).is_none());
        assert!(Profile::new(f64::INFINITY, &[]).is_none());
        let concave = Profile::new(-0.5, &[]).expect("a profile");
        assert!(Strip::new(&concave, 10.0, STEEL).is_none());
        // Convex only within 1e-4 of the origin: its valley ends at 2e-3, short of 100 N/mm.
        let narrow = Profile::new(-0.5, &[step(Some(-1e-4), Some(1e-4), 10.0)]).expect("a profile");
        assert!(Strip::new(&narrow, 100.0, STEEL).is_none());
        assert!(Strip::new(&narrow, 1e-6, STEEL).is_some());
    }
}
