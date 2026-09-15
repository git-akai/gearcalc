//! The automatic parameter calculations.
//!
//! Several of the specification's inputs carry an "automatic" toggle: the solver
//! works the value out, and the field is locked while it does. The toggle itself
//! is [`crate::params::Auto`]; this module is the arithmetic behind two of them.
//!
//! Both are design decisions rather than measurements, and both make a hidden
//! assumption visible — which is why they are here rather than folded silently
//! into the profile generator.

use crate::involute::{inv, inv_from_roll};
use crate::params::GearParams;
use crate::solve::{brent, Tol};
use crate::tooth::{Tooth, POINTED_TOOTH_MAX_ROLL};

/// The smallest profile shift that avoids undercut at a stated depth.
///
/// Two figures, because the honest answer depends on the cutter and the
/// difference is the whole point of the control.
#[derive(Clone, Copy, Debug)]
pub struct MinimumShift {
    /// Using the root radius coefficient actually entered — the real answer.
    pub with_cutter_radius: f64,
    /// The same question asked of a sharp-cornered rack, `ρ = 0`.
    ///
    /// This is the assumption behind the classical "17 teeth at 20°" rule, and
    /// it is **more demanding** than reality: a real cutter's tip round ends the
    /// straight flank higher up, so there is less undercut than the sharp-rack
    /// figure predicts. Reported alongside so the gap is visible rather than
    /// arguable.
    pub sharp_rack: f64,
}

/// Minimum profile shift to keep the flank free of undercut down to
/// `working_depth` (in modules).
///
/// # The formula, and what "working depth" means
///
/// Undercut begins where the rack's straight flank runs out before reaching the
/// base tangent point. Writing that condition at a cutter depth of
/// `m(h_w − x) − ρ` and solving for `x`:
///
/// ```text
/// x_min = h_w − [ ρ + sin α_t ( r sin α_t − ρ ) ] / m
/// ```
///
/// Closed form, and exactly invertible, because the undercut indicator is linear
/// in `x`.
///
/// `h_w` is **the depth at which the undercut question is asked**, not a
/// constraint on the form radius. That distinction is the whole content of the
/// control: asking "is the flank undercut within one module of depth?" and "is
/// it undercut at all?" are different questions with different answers, and the
/// classical rule silently answers the first.
///
/// # What it exposes
///
/// With `ρ = 0` it reduces to `x_min = h_w − z sin²α_t / 2`, so `x = 0` needs
/// `z ≥ 2 h_w / sin²α_t`. At α = 20°:
///
/// | `h_w` | `z_min` |
/// |---|---|
/// | 1.00 module | 17.10 → **18 teeth**, the classical rule |
/// | 1.25 module, a full standard dedendum | 21.37 → **22 teeth** |
///
/// **A stage's `working_depth` follows its own dedendum**, and is `Auto` so it
/// can be told otherwise. It used to be a fixed 1 module — the classical rule —
/// and the two ask different questions, as the table shows. The dedendum is the
/// one the profile generator answers, so following it makes the automatic shift
/// and the `undercut` flag agree by construction rather than by coincidence:
/// `a_gear_at_the_minimum_shift_is_on_the_edge_of_undercut` could only be
/// written by passing `p.dedendum` in by hand, which was the model telling us
/// what its default should be. A gear cut shallower is now asked about the depth
/// it actually has instead of about a convention.
///
/// And with a real cutter tip radius the answer moves the other way: at
/// `ρ = 0.38` (the ISO 53 basic rack), `h_w = 1`, α = 20°, the threshold falls to
/// 12.82 — **13 teeth**. Two assumptions buried in one piece of conventional
/// wisdom, pulling in opposite directions.
///
/// Note this depends only on quantities that are themselves independent of `x`
/// — `r`, `α_t` and the cutter — so there is no circularity in using it to
/// choose `x`.
#[must_use]
pub fn minimum_profile_shift(p: &GearParams, working_depth: f64) -> MinimumShift {
    let beta = p.helix_angle.to_radians();
    let alpha_t = crate::plane::transverse_pressure_angle(p.pressure_angle.to_radians(), beta);
    let mt = p.module / beta.cos();
    let r = mt * f64::from(p.teeth) / 2.0;
    let sa = alpha_t.sin();

    // The cutter tip radius is a transverse length: the coefficient is in normal
    // modules, so it scales by m_t, matching `profile`.
    let rho = p.root_radius * mt;

    let at = |rho: f64| working_depth - (rho + sa * (r * sa - rho)) / p.module;

    MinimumShift {
        with_cutter_radius: at(rho),
        sharp_rack: at(0.0),
    }
}

/// The shift the automatic toggle should apply: enough to avoid undercut, and no
/// more.
///
/// [`minimum_profile_shift`] is a **lower bound**, and on any gear with a
/// comfortable tooth count it is negative — −1.76 at z = 43, α = 20°, `h_w` = 1.
/// Applying that literally would thin a tooth that needed no help, for nothing.
///
/// It is worse than merely pointless in a pair. Operating pressure angle comes
/// from `inv α_w = inv α_t + 2Σx tan α_n / Σz`, so a sufficiently negative `Σx`
/// drives `inv α_w` below zero and the mesh leaves the involute domain — there
/// is no centre distance at which those two gears run. A 17:43 pair with both
/// shifts set to their minimum does exactly that.
///
/// So the automatic value is `max(x_min, 0)`: shift when the geometry demands
/// it, otherwise leave it alone. Deliberate negative shift remains available by
/// switching the toggle off, which is the right place for it — it is a decision
/// about centre distance or balance, not about undercut.
#[must_use]
pub fn automatic_profile_shift(p: &GearParams, working_depth: f64) -> f64 {
    minimum_profile_shift(p, working_depth)
        .with_cutter_radius
        .max(0.0)
}

/// What profile shifts a gear can be built at, and the design thresholds inside
/// that range.
///
/// Two tiers, deliberately, because they answer different questions.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct ShiftRange {
    /// What can be built: below it the tooth is thinner than the guards allow or
    /// the cutter runs past the centre; above it the cutter no longer reaches
    /// the root.
    pub bound: Bound,
    /// Below this the flank is undercut at the stated working depth — the same
    /// figure [`minimum_profile_shift`] returns. **Advisory, not a limit:** an
    /// undercut gear is a real gear, and this crate generates it exactly.
    pub undercut: f64,
    /// The undercut threshold a sharp-cornered rack would give. Reported so the
    /// classical rule's hidden assumption stays visible.
    pub sharp_rack_undercut: f64,
    /// Above this the tooth would be pointed at the requested addendum, so the
    /// tip radius is capped and a clamp note is raised. `None` when the tooth
    /// never comes to a point anywhere in the buildable range.
    pub pointed: Option<f64>,
    /// Above this the cutter has to reach **deeper than the dedendum asked
    /// for**, because a shift moves the tool out and the depth `m(h_f − x)` runs
    /// out before the shift does.
    ///
    /// A threshold rather than a limit, and it was a limit until it was measured
    /// (`docs/corrections.md`). The dedendum is a request for where the root
    /// should sit, and the tool is derived from it — the same reading a ring has
    /// always had, where the root circle is wherever the cutter reaches and
    /// there is no dedendum input at all. So a shift past here is buildable, by
    /// a deeper tool, and what the designer is owed is to be told that the tool
    /// is no longer the one specified rather than to be refused a gear that can
    /// be cut.
    ///
    /// Treating it as a limit is also what made the buildable range **step by
    /// 62 %** as the angular shift left zero: an eccentric gear's teeth share
    /// one tool and cannot each own a dedendum, so that path had already derived
    /// the depth, while a concentric gear refused. One reading now, and this
    /// threshold is where the two used to part.
    pub shallow_cut: f64,
}

/// How far the extreme teeth of an eccentric gear are shifted from the nominal:
/// `+Δx` at θ = 0, and `Δx·cos(2π⌊z/2⌋/z) ≤ 0` at the far tooth — the same
/// folding [`crate::gear::Gear::new`] applies so that mirror-pair
/// teeth share a shift. `(0.0, 0.0)` for a concentric gear, so every `+ off`
/// downstream is `+ 0.0` and the concentric answer is unchanged to the bit.
fn shift_offsets(p: &GearParams) -> (f64, f64) {
    let e = p.angular_shift.abs();
    let z = f64::from(p.teeth.max(1));
    let c_lo = (std::f64::consts::TAU * (z / 2.0).floor() / z).cos();
    (e * c_lo, e)
}

/// The profile shifts this gear can actually be built at.
///
/// # Why this exists
///
/// The specification gives profile shift a fixed range of `|x| ≤ 2`. That is not
/// merely arbitrary — it is **wrong in three different directions**, because
/// every real bound depends on parameters a constant cannot see:
///
/// - The upper bound is always the cutter depth, `h_f − 0.05`: **1.20** at the
///   default dedendum and 0.95 at `h_f = 1`. Anything entered between there and
///   2 silently has its dedendum raised.
/// - The lower bound is tooth thickness, and it swings across the allowed
///   pressure-angle range: **−3.00** at 14.5°, −2.13 at 20°, **−1.34** at 30°.
/// - Thickness modification moves it again — `k = 1.3` takes the floor to −2.78
///   — and `|x| ≤ 2` cannot know that either.
///
/// # Closed form
///
/// Every guard that bounds `x` is **linear in `x`**, so the admissible interval
/// is an intersection of half-lines and needs no solve:
///
/// ```text
/// thickness:  0.02 m ≤ s_t ≤ 0.95 π m_t     s_t = m(π/2 + 2(x + x_s) tan α_n)/cos β
/// depth:      0.05 m ≤ m(h_f − x) ≤ 0.9 r
/// ```
///
/// [verified against the generator: at z = 17, α = 20°, `h_f` = 1.25 it predicts
/// 1.200 and −2.130, and the generator builds cleanly at 1.19 and −2.12 while
/// raising *"cutter depth was ≤ 0"* at 1.21 and *"tooth thickness raised"* at
/// −2.14.]
///
/// # The two tiers are not interchangeable
///
/// `min` and `max` are **degeneracy** limits — where the geometry stops being
/// constructible at all, per the guards in [`crate::params`]. They are not
/// design advice, and a shift near either end produces a legal but absurd tooth.
/// [`ShiftRange::undercut`] and [`ShiftRange::pointed`] are the design-relevant
/// thresholds, and they sit *inside* the range rather than bounding it: an
/// undercut gear is perfectly real and this crate generates it exactly.
///
/// # What no per-gear range can express
///
/// A meshing pair must also satisfy `inv α_w ≥ 0`, which constrains the **sum**
/// of both shifts. No per-gear interval can state that, and violating it is
/// reported by [`crate::Mesh::new`] instead.
#[must_use]
pub fn admissible_profile_shift(p: &GearParams, working_depth: f64) -> ShiftRange {
    use crate::params::guard;
    use std::f64::consts::PI;

    let beta = p.helix_angle.to_radians();
    let an = p
        .pressure_angle
        .to_radians()
        .max(guard::MIN_PRESSURE_ANGLE_DEG.to_radians());
    let alpha_t = crate::plane::transverse_pressure_angle(an, beta);
    let mt = p.module / beta.cos();
    let r = mt * f64::from(p.teeth) / 2.0;
    let xs = p.thickness_shift();
    let ta = an.tan();

    // Tooth thickness, both ends. `x_s` shifts the whole interval.
    let lo_t = (guard::MIN_TOOTH_THICKNESS_MODULES * beta.cos() - PI / 2.0) / (2.0 * ta) - xs;
    let hi_t = (guard::MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH * PI - PI / 2.0) / (2.0 * ta) - xs;

    // Where the dedendum asked for stops being deep enough to cut at all, so a
    // deeper tool takes over. A threshold, not a bound — `root_off_axis` below
    // is the only limit the depth still imposes, and it subsumes the old
    // `dedendum − 0.9 r/m` floor because it is read off the tool actually used.
    let shallow_cut = p.dedendum - guard::MIN_CUTTER_DEPTH_MODULES;

    // A gear is cut across the interval `[x̄ + off_lo, x̄ + off_hi]` — a single
    // point for a concentric gear, where both offsets are exactly zero — and
    // every tooth in it must be buildable. So each bound binds at whichever end
    // reaches it first, and there is **one expression** rather than a branch on
    // whether the interval has width.
    //
    // It used to be a branch, and the two arms disagreed about the shallow cut:
    // a concentric gear was refused where an eccentric one deepened its tool, so
    // the ceiling stepped from 1.200 to 1.942 as `Δx` left zero. The shallow cut
    // is a *threshold* now (see [`ShiftRange::shallow_cut`]) and the tool is
    // derived from the dedendum at both ends, so nothing steps.
    let (off_lo, off_hi) = shift_offsets(p);

    // The tool the whole gear shares: deep enough for the tooth cut at the
    // largest shift, and never shallower than the dedendum asked for.
    let d_shared = p
        .dedendum
        .max(guard::MIN_CUTTER_DEPTH_MODULES + p.profile_shift + off_hi);
    // ...and that tool has to leave the *deepest* cut's root off the axis, which
    // is the tooth at the smallest shift. This is the only ceiling the depth
    // still imposes, and it is a floor on the shift rather than a ceiling.
    let root_off_axis = d_shared - guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r / p.module - off_lo;

    let min = (lo_t - off_lo).max(root_off_axis);
    let max = hi_t - off_hi;

    let shift = minimum_profile_shift(p, working_depth);

    // Where the tooth comes to a point at the requested addendum. Monotone in
    // practice but not guaranteed, so it is bracketed on the admissible range
    // and simply absent if the tooth never points within it. For an eccentric
    // gear it is the *high* tooth, shift `x̄ + off_hi`, that points first — so the
    // threshold is expressed on `x̄` with that offset folded in.
    let rb = r * alpha_t.cos();
    let half_tip_angle = |x: f64| {
        let x = x + off_hi;
        let st = p.module * (PI / 2.0 + 2.0 * (x + xs) * ta) / beta.cos();
        let psi_b = st / (2.0 * r) + inv(alpha_t);
        let ra = r + p.module * (p.addendum + x);
        if ra <= rb {
            return psi_b;
        }
        psi_b - inv_from_roll(((ra / rb).powi(2) - 1.0).sqrt())
    };
    let pointed = if half_tip_angle(min) * half_tip_angle(max) < 0.0 {
        brent(half_tip_angle, min, max, Tol::default())
    } else {
        None
    };

    ShiftRange {
        bound: Bound::between(Some(min), Some(max)),
        // The high tooth, shift `x̄ + off_hi`, exhausts the depth first — the
        // same end that points first, and for the same reason: the tool has
        // moved out furthest there.
        shallow_cut: shallow_cut - off_hi,
        // The low tooth, shift `x̄ + off_lo`, undercuts first.
        undercut: shift.with_cutter_radius - off_lo,
        sharp_rack_undercut: shift.sharp_rack - off_lo,
        pointed,
    }
}

/// A bound on one input. `None` on a side that is genuinely unbounded.
///
/// Exclusivity matters and is carried rather than assumed: a module of exactly
/// zero collapses every radius, so `m > 0`, while an addendum exactly at its
/// floor is a legal (if pointless) tooth.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Bound {
    pub min: Option<f64>,
    pub max: Option<f64>,
    #[cfg_attr(feature = "serde", serde(default))]
    pub exclusive_min: bool,
    #[cfg_attr(feature = "serde", serde(default))]
    pub exclusive_max: bool,
}

impl Bound {
    /// Inclusive on both sides.
    #[must_use]
    pub const fn between(min: Option<f64>, max: Option<f64>) -> Self {
        Self {
            min,
            max,
            exclusive_min: false,
            exclusive_max: false,
        }
    }

    /// Exclusive on both sides.
    #[must_use]
    pub const fn strictly(min: f64, max: f64) -> Self {
        Self {
            min: Some(min),
            max: Some(max),
            exclusive_min: true,
            exclusive_max: true,
        }
    }

    /// Whether `v` is inside this bound.
    ///
    /// A predicate rather than a message. It used to return the sentence too —
    /// "must be at least 0.5" and its three siblings — on the reasoning that
    /// producing them here stopped two panels wording the same condition
    /// differently. That never happened: the front end wrote its own copies and
    /// used those, so the English here was shown to nobody and drifted freely
    /// (docs/rationale.md#no-english-in-gear-core-and-no-engineering-in-the-catalogue). `gear-core` holds no English, and the surviving copy is
    /// the one with a catalogue behind it — `outside()` in `web/src/core.ts`,
    /// reading `ui.validation_*`.
    ///
    /// Exclusivity lives in the fields, so a caller that needs to say *why*
    /// reads those rather than parsing a sentence.
    #[must_use]
    pub fn admits(&self, v: f64) -> bool {
        if !v.is_finite() {
            return false;
        }
        if let Some(lo) = self.min {
            if if self.exclusive_min { v <= lo } else { v < lo } {
                return false;
            }
        }
        if let Some(hi) = self.max {
            if if self.exclusive_max { v >= hi } else { v > hi } {
                return false;
            }
        }
        true
    }
}

/// Every input range the geometry decides, rather than convention.
///
/// # What these are for
///
/// **Only to stop a gear that cannot exist**, not one that is merely strange or
/// useless. A one-tooth gear, an 85° helix, a negative addendum and a pressure
/// angle of 2° are all peculiar and all perfectly constructible — the generator
/// builds every one of them, finite and closed. They are not this crate's to
/// forbid.
///
/// So the bounds here mark impossibility: a tip inside its own root, a root
/// circle at or through the axis, a fillet that cannot fit the space it must sit
/// in. The parameters that stay fixed in the UI are the ones whose bounds are
/// *also* impossibility, and simply do not vary: `m > 0`, `z ≥ 1`, `0 < α < 90°`,
/// `|β| < 90°`, and `0 < k < 2` (a rack whose tooth or space has non-positive
/// width is not a rack).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct Ranges {
    /// `m > 0`: at zero every radius collapses.
    pub module: Bound,
    /// `0 < α < 90°`: at zero the thickness-equivalent shift diverges, at 90°
    /// the base circle does.
    pub pressure_angle: Bound,
    /// `z ≥ 1`.
    pub teeth: Bound,
    /// `|β| < 90°`: the transverse module diverges at the limit.
    pub helix_angle: Bound,
    /// `0 < k < 2`: a rack whose tooth or space has non-positive width is not a
    /// rack.
    pub thickness_mod: Bound,
    pub profile_shift: ShiftRange,
    /// Lower bound only. There is no upper: too much addendum gives a pointed
    /// tooth, which the generator caps and reports rather than refuses.
    pub addendum: Bound,
    pub dedendum: Bound,
    /// Upper bound only, from the fillet fit.
    pub root_radius: Bound,
    /// How far the shift may vary around the revolution — see
    /// [`admissible_angular_shift`]. Symmetric about zero, and the one bound
    /// that is about the *tool* the teeth share rather than about any one tooth.
    pub angular_shift: Bound,
}

/// Combine two bounds into the interval that satisfies both.
fn tighter(a: Bound, b: Bound) -> Bound {
    let both = |x: Option<f64>, y: Option<f64>, f: fn(f64, f64) -> f64| match (x, y) {
        (Some(x), Some(y)) => Some(f(x, y)),
        (x, y) => x.or(y),
    };
    Bound {
        min: both(a.min, b.min, f64::max),
        max: both(a.max, b.max, f64::min),
        exclusive_min: a.exclusive_min || b.exclusive_min,
        exclusive_max: a.exclusive_max || b.exclusive_max,
    }
}

/// The ranges the geometry imposes on this gear's remaining parameters.
///
/// All closed form. Each bound is the point at which the generator's own guards
/// begin to clamp, so **inside the range implies no clamp note** — a property
/// the tests assert directly against the generator rather than against algebra.
///
/// - **Addendum**, lower: the tip must be outside the root, `r_a > r_f`, which
///   reduces to the pleasingly simple `h_a > −h_f` — the tooth must have
///   positive height. Also `r_a > r_b`, which binds only at extreme negative
///   addendum.
/// - **Dedendum**, lower: the same condition read the other way, `h_f > −h_a`.
///   Upper: the root circle must stay off the axis, `m(h_f − x) < r`.
/// - **Root radius**, upper: the tip round must fit both the cutter depth and
///   the tooth space. The space limit is
///   `ρ_max = w_tip cos α_t / (2(1 − sin α_t))` — the fit the prior work records
///   as easy to get wrong, since the plausible `w_tip/(2 cos α_t)` silently
///   shrinks every profile-shifted fillet.
///
/// # Gear gears
///
/// A gear whose shift varies (`angular_shift ≠ 0`) is cut across `x̄ ± Δx`, and
/// every tooth of it must be buildable — so the shift-dependent bounds close in
/// on the interval rather than a point. `profile_shift` pulls in by `Δx` on each
/// side (its cutter-depth ceiling drops out — the shared tool follows the shift
/// up), and `addendum` / `dedendum` / `root_radius` are each taken at the
/// tighter of the two extremes against the shared cutter depth. A concentric
/// gear is `Δx = 0` and this is the interval collapsed to a point, unchanged to
/// the bit.
#[must_use]
pub fn admissible_ranges(p: &GearParams, working_depth: f64) -> Ranges {
    let base = ranges_at_shift(p, working_depth);

    let (off_lo, off_hi) = shift_offsets(p);

    // `addendum`, `dedendum` and `root_radius` are read off the tooth being
    // cut — its shift, and the cutter depth. A gear's teeth span
    // `[x̄ + off_lo, x̄ + off_hi]` and share one cutter deep enough for the high
    // tooth (docs/reference.md#angularly-varying-profile-shift), so each of these is the tighter of its values at the two
    // shift extremes, taken against that shared depth. A concentric gear's
    // interval is a **point** — both offsets are exactly zero and `d_shared` is
    // exactly the dedendum — so the two evaluations are the same call and the
    // answer is `base` to the bit. That is why there is no longer an early
    // return for it: an `if` that has to be exactly equivalent to the general
    // path is a second place the general path can be wrong.
    // `profile_shift` is already the interval's own window — see
    // `admissible_profile_shift`, which `ranges_at_shift` calls.
    let x_lo = p.profile_shift + off_lo;
    let x_hi = p.profile_shift + off_hi;
    let d_shared = p
        .dedendum
        .max(crate::params::guard::MIN_CUTTER_DEPTH_MODULES + x_hi);
    let at = |x: f64| {
        ranges_at_shift(
            &GearParams {
                profile_shift: x,
                dedendum: d_shared,
                angular_shift: 0.0,
                ..*p
            },
            working_depth,
        )
    };
    let lo = at(x_lo);
    let hi = at(x_hi);

    Ranges {
        addendum: tighter(lo.addendum, hi.addendum),
        dedendum: tighter(lo.dedendum, hi.dedendum),
        root_radius: tighter(lo.root_radius, hi.root_radius),
        angular_shift: admissible_angular_shift(p),
        ..base
    }
}

/// How much the shift may vary around the revolution: `|Δx| ≤` this.
///
/// # Why the amplitude needs a bound of its own
///
/// Every other bound closes onto the swept interval `x̄ ± Δx`, which bounds the
/// *shift* given an amplitude. None of them bounds the amplitude given a shift,
/// and the amplitude has its own limit for a reason none of them can see:
/// **the teeth have to share one tool.**
///
/// One hob cuts the whole gear, so its depth is set by the tooth that needs
/// most — `MIN_CUTTER_DEPTH + x_hi` at least — and that same depth is then
/// driven into the tooth cut at the *smallest* shift, whose root sinks to
/// `r − m(depth − x_lo)`. Past some amplitude those two demands cross: no
/// single depth both reaches the high tooth and keeps the low tooth's root off
/// the axis, and there is no such gear. Both conditions are linear in `Δx`, so
/// the bound is closed form and is the tighter of them.
///
/// Unbounded, this was reachable and observable: at `z = 5`, `Δx = 2.4` the
/// drawn root stood **1.94 mm** clear of the teeth it belonged to, because the
/// tool settled for the deep tooth was re-clamped when the shallow one was
/// built against it (`docs/corrections.md`). The bound is what makes that
/// region unreachable rather than merely unlikely.
///
/// Symmetric, because the geometry is: [`shift_offsets`] takes `|Δx|`, so the
/// sign chooses which side of the gear is the thick one and nothing else.
#[must_use]
pub fn admissible_angular_shift(p: &GearParams) -> Bound {
    use crate::params::guard;

    let beta = p.helix_angle.to_radians();
    let r = p.module / beta.cos() * f64::from(p.teeth) / 2.0;
    let headroom = guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r / p.module;

    // Per unit amplitude: how far the extremes spread, and how far the deep
    // tooth sinks below the nominal. `c_lo ≤ 0` for every `z ≥ 2`; at `z = 1`
    // there is one tooth, so it neither spreads nor sinks and only the gear's
    // own dedendum can bind.
    let z = f64::from(p.teeth.max(1));
    let c_lo = (std::f64::consts::TAU * (z / 2.0).floor() / z).cos();
    let (spread, sink) = (1.0 - c_lo, -c_lo);

    // The tool must reach the high tooth without driving the low tooth's root
    // into the axis...
    let by_spread = if spread > 0.0 {
        (headroom - guard::MIN_CUTTER_DEPTH_MODULES) / spread
    } else {
        f64::INFINITY
    };
    // ...and where the dedendum alone already sets the depth, the low tooth
    // still has to survive it.
    let by_sink = if sink > 0.0 {
        (headroom - p.dedendum + p.profile_shift) / sink
    } else {
        f64::INFINITY
    };

    let amplitude = by_spread.min(by_sink).max(0.0);
    Bound::between(Some(-amplitude), Some(amplitude))
}

/// The ranges for a gear cut at one shift — every tooth of a concentric gear,
/// and each extreme of an eccentric one (called twice, [`admissible_ranges`]).
fn ranges_at_shift(p: &GearParams, working_depth: f64) -> Ranges {
    use crate::params::guard;
    use std::f64::consts::PI;

    let beta = p.helix_angle.to_radians();
    let an = p
        .pressure_angle
        .to_radians()
        .max(guard::MIN_PRESSURE_ANGLE_DEG.to_radians());
    let alpha_t = crate::plane::transverse_pressure_angle(an, beta);
    let mt = p.module / beta.cos();
    let r = mt * f64::from(p.teeth) / 2.0;
    let rb = r * alpha_t.cos();
    let x = p.profile_shift;

    // Addendum: tip outside the root, and outside the base circle.
    let above_root = -p.dedendum;
    let above_base = (rb * (1.0 + guard::TIP_ABOVE_BASE_FRACTION) - r) / p.module - x;

    // Dedendum: positive height, and a root circle that does not reach the axis.
    let root_positive = x + guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r / p.module;

    // Root radius: the tip round must fit the cutter depth and the tooth space.
    let bd = p.module * (p.dedendum - x);
    let st = p.module * (PI / 2.0 + 2.0 * (x + p.thickness_shift()) * an.tan()) / beta.cos();
    let w_tip = (PI * mt - st) - 2.0 * bd * alpha_t.tan();
    let rho_fit = if w_tip > 0.0 {
        w_tip * alpha_t.cos() / (2.0 * (1.0 - alpha_t.sin()))
    } else {
        0.0
    };
    let rho_max = guard::FILLET_FRACTION_OF_MAX * bd.min(rho_fit);

    Ranges {
        // Invariant bounds. They do not vary, but they live here so that there
        // is exactly one place any input limit is written down.
        module: Bound {
            min: Some(0.0),
            max: None,
            exclusive_min: true,
            exclusive_max: false,
        },
        pressure_angle: Bound::strictly(0.0, 90.0),
        teeth: Bound::between(Some(1.0), None),
        helix_angle: Bound::strictly(-90.0, 90.0),
        thickness_mod: Bound::strictly(0.0, 2.0),

        profile_shift: admissible_profile_shift(p, working_depth),
        addendum: Bound::between(Some(above_root.max(above_base)), None),
        dedendum: Bound::between(Some(-p.addendum), Some(root_positive)),
        root_radius: Bound::between(Some(0.0), Some((rho_max / mt).max(0.0))),
        angular_shift: admissible_angular_shift(p),
    }
}

/// Addendum coefficient that leaves the tooth tip exactly `min_tip_width` wide.
///
/// The transverse thickness at radius `r'` is `s(r') = 2r'(ψ_b − inv α_{r'})`
/// with `cos α_{r'} = r_b/r'`. Setting `s = s_min` is transcendental but strictly
/// decreasing in `r'`, and bracketed by construction between the base circle and
/// the pointed-tooth radius where `s = 0`. So it is a bracketed solve that cannot
/// fail to converge.
///
/// Returns the coefficient in **normal modules**, ready to put in
/// [`GearParams::addendum`], since `r_a = r + m(h_a + x)`.
///
/// `None` when the tooth is thinner than `min_tip_width` even at the base circle
/// — there is no radius at which it is wide enough, which is a real answer and
/// not a failure to converge.
///
/// # Which plane
///
/// This is the **transverse** tip width, because that is what `s(r')` measures
/// directly. For a spur gear the distinction does not arise; for a helical gear
/// the normal tip width is smaller, so a helical tooth sized to a transverse
/// minimum is slightly sharper in the normal plane than the number suggests.
///
/// Note the contrast with [`crate::metrology::cutter_tip_width`], which is
/// reported in the normal plane so that it stays helix-independent. They measure
/// different things: that one is the rack's tip, this one is the gear's.
#[must_use]
pub fn addendum_for_tip_width(g: &Tooth, min_tip_width: f64) -> Option<f64> {
    // s(u) in terms of the involute roll parameter: r' = r_b sqrt(1+u^2).
    let width = |u: f64| 2.0 * g.rb * f64::hypot(1.0, u) * (g.psi_b - inv_from_roll(u));

    // The tooth is at its widest where the involute starts.
    if width(0.0) < min_tip_width {
        return None;
    }
    // ...and vanishes at the pointed-tooth radius, which brackets the search.
    let u_point = brent(
        |u| g.psi_b - inv_from_roll(u),
        0.0,
        POINTED_TOOTH_MAX_ROLL,
        Tol::default(),
    )?;

    // A non-positive minimum width has its root exactly at the far end of the
    // bracket, where there is no sign change for a bracketed solver to find. The
    // pointed tooth is the answer, and it is already in hand.
    let u = if min_tip_width > 0.0 {
        brent(|u| width(u) - min_tip_width, 0.0, u_point, Tol::default())?
    } else {
        u_point
    };

    let ra = g.rb * f64::hypot(1.0, u);
    Some((ra - g.r) / g.params.module - g.params.profile_shift)
}

/// What is already decided about a pair's two profile shifts.
///
/// Two shifts is two unknowns, and each of the three things a designer can pin
/// — either shift, or the centre distance, which fixes their signed sum —
/// removes one. So **at most two may be given**; a third is a contradiction
/// rather than a tighter design, and the caller is expected to have dropped one
/// before asking.
/// **Whether the root round a designer asked for still fits at this shift.**
///
/// The tip round the cutter can leave shrinks as the shift rises: the cutter
/// bites less deep (`m(h_f − x)`) and the space it cuts narrows, so a fillet
/// specified at one shift becomes unbuildable at a larger one. That makes the
/// root radius a bound on the shift, and one the search has to respect for the
/// same reason it respects undercut — the shifts it hands back have to describe
/// a tooth somebody can cut.
///
/// It reads the ceiling from [`admissible_ranges`] rather than restating it, so
/// there is one place the limit is written and the note the panel already shows
/// against the input is the same limit the search is obeying. A designer who
/// wants the shift to go further relieves it by asking for a smaller round,
/// which the note is what tells them.
#[must_use]
pub fn root_radius_fits(p: &GearParams, working_depth: f64) -> bool {
    admissible_ranges(p, working_depth)
        .root_radius
        .admits(p.root_radius)
}

/// **The interval a search may choose one member's shift from**, or `None` where
/// there is none.
///
/// # It is narrower than what a designer may type, and deliberately
///
/// [`admissible_profile_shift`] answers *could this gear exist?* — the range the
/// generator builds without refusing, which is the range the gear card draws.
/// A search is held to a narrower question, *could this gear be cut as
/// specified?*, because the shifts it hands back are shifts nobody asked for:
/// past a certain shift the cutter's tip round no longer fits the space and the
/// generator **clamps** it, which is a fair answer to a number a designer typed
/// and not one to a number the tool chose on their behalf
/// ([`member_is_buildable`] says the same of undercut and a severed tip).
///
/// # Why it is bisected rather than derived
///
/// The bound is closed form — `ρ_max` is `k · min(m(h_f − x), ρ_fit)` and both
/// terms are linear in `x` — but writing that here would be a second statement
/// of a rule [`admissible_ranges`] already makes, and *one idea written down
/// twice is a place two answers can differ*. So the residual is read off that
/// one rule and a bracketed solve finds where it vanishes: `ρ_max` falls
/// monotonically as the cutter is drawn out, so the root is unique and the
/// bracket is the admissible interval itself.
///
/// `at(x)` builds this member at a trial shift, which the caller can do and this
/// cannot — the same shape [`shifts_for_efficiency`] takes its pair in.
#[must_use]
pub fn searchable_shift(at: &dyn Fn(f64) -> GearParams, floor: Option<f64>) -> Option<(f64, f64)> {
    let round_fits = |x: f64| {
        let p = at(x);
        let m = admissible_ranges(&p, p.dedendum).root_radius.max?;
        Some(m - p.root_radius)
    };
    let base = at(0.0);
    let bound = admissible_ranges(&base, base.dedendum).profile_shift.bound;
    let lo = bound.min?.max(floor.unwrap_or(f64::NEG_INFINITY));
    let hi = bound.max?;
    if lo >= hi {
        return None;
    }
    if round_fits(hi)? >= 0.0 {
        return Some((lo, hi));
    }
    if round_fits(lo)? < 0.0 {
        // The round asked for does not fit anywhere this member could be cut,
        // which is an answer about the round rather than about the shift.
        return None;
    }
    // `NaN` where the bound is unanswerable, so the bracket fails rather than
    // reading an absence as a sign change.
    let ceiling = brent(
        |x| round_fits(x).unwrap_or(f64::NAN),
        lo,
        hi,
        Tol::default(),
    )?;
    (lo < ceiling).then_some((lo, ceiling))
}

/// **The two shifts a given centre distance implies, divided as evenly as the
/// members allow.**
///
/// A centre distance fixes the *sum* and says nothing about the division. With
/// the optimiser on, the division is what `shifts_for_efficiency` searches over.
/// With it off there is no objective to search against, and the stage still has
/// to answer — so the division follows a stated rule instead.
///
/// # The rule: the even split, projected onto what the members can be cut at
///
/// Each member contributes to the sum — `x₁` and `sign · x₂`, so a ring is the
/// same expression — and the even split gives each half. Where that lands
/// outside a member's own interval it is pushed back in, and the other member
/// absorbs the difference. Written as one projection rather than as a walk:
/// gear 1's admissible contribution is
///
/// ```text
/// [max(lo₁, sum − hi₂),  min(hi₁, sum − lo₂)]
/// ```
///
/// — its own interval intersected with what leaves the other inside its own —
/// and the answer is `sum/2` clamped into that. An **empty** interval is the
/// honest "no admissible pair of shifts reaches this distance", which is a
/// refusal rather than a clamp because the input describes no shape.
///
/// # What it does, read on a 9/37 pair
///
/// The nine-tooth pinion needs about 0.47 modules to clear undercut and the
/// thirty-seven wants none, so at the nominal distance they sit at 0.47 and 0.
/// Open the distance and the *pinion cannot move* — it is against its floor —
/// so the wheel takes all of it, until the two are level and from there they
/// rise together. Close it instead and the wheel gives the shift back alone,
/// until it reaches its own floor and there is nothing left to give.
///
/// That is the behaviour a designer describes, and it falls out of the
/// projection rather than being coded as three cases.
///
/// # The interval is `searchable_shift`'s, not `admissible_profile_shift`'s
///
/// These are shifts the tool chooses, not shifts a designer typed, so the
/// narrower question applies: *could this member be cut as specified?* — see
/// [`searchable_shift`], which is also where the caller's undercut floor enters.
#[must_use]
pub fn divide_shift_sum(
    at: &dyn Fn(usize, f64) -> GearParams,
    sign: f64,
    sum: f64,
    floor: [Option<f64>; 2],
) -> Option<[f64; 2]> {
    // Each member's interval, as a **contribution** to the sum. A negative
    // `sign` maps gear 2's interval through a reflection, which reverses it.
    let contribution = |i: usize| -> Option<(f64, f64)> {
        let (lo, hi) = searchable_shift(&|x| at(i, x), floor[i])?;
        let k = if i == 0 { 1.0 } else { sign };
        Some(if k >= 0.0 {
            (lo * k, hi * k)
        } else {
            (hi * k, lo * k)
        })
    };
    let (lo1, hi1) = contribution(0)?;
    let (lo2, hi2) = contribution(1)?;

    let lo = lo1.max(sum - hi2);
    let hi = hi1.min(sum - lo2);
    // A **positive** test, so a NaN anywhere upstream reads as "no interval"
    // rather than as an empty one that happens to compare false — the same
    // reason `Screw::new` guards with predicates rather than with negations.
    if lo <= hi {
        let c1 = (sum / 2.0).clamp(lo, hi);
        return Some([c1, (sum - c1) / sign]);
    }
    None
}

/// **Which of a fixed set of numbers a search still has to choose.**
///
/// A stage hands its searcher some numbers already decided and some not, and
/// then has to map whatever the search hands back onto the full set. That
/// mapping was written out twice, identically, in two stages — and it is the
/// kind of thing that stays identical right up until one of them is edited.
///
/// `None` is a number left free; `Some` is one a designer gave, which a search
/// may not overrule.
#[derive(Debug, Clone, Copy)]
pub struct Freedoms<const N: usize> {
    given: [Option<f64>; N],
}

impl<const N: usize> Freedoms<N> {
    /// From what is already decided.
    #[must_use]
    pub const fn new(given: [Option<f64>; N]) -> Self {
        Self { given }
    }

    /// How many numbers are left to choose — the dimension to search in, and
    /// zero where the design is already fully specified.
    #[must_use]
    pub fn count(&self) -> usize {
        self.given.iter().filter(|g| g.is_none()).count()
    }

    /// The box the search sweeps, from a bound on each of the `N` numbers —
    /// only the ones left free, in the order [`Self::place`] reads them.
    ///
    /// A search's opening sweep is a resolution across an interval, so the
    /// interval has to be the one the number can take. Where it is a guess
    /// instead, the resolution becomes a feasibility test and a narrow
    /// admissible set falls between two grid points (`docs/corrections.md`).
    #[must_use]
    pub fn boxes(&self, per_number: [(f64, f64); N]) -> Vec<(f64, f64)> {
        self.given
            .iter()
            .zip(per_number)
            .filter_map(|(g, b)| g.is_none().then_some(b))
            .collect()
    }

    /// The full set, with `free` read into the places left open in order.
    ///
    /// # Panics
    ///
    /// If `free` is shorter than [`Self::count`], which is a caller that
    /// searched in the wrong number of dimensions.
    #[must_use]
    pub fn place(&self, free: &[f64]) -> [f64; N] {
        let mut next = 0;
        self.given.map(|g| {
            g.unwrap_or_else(|| {
                let v = free[next];
                next += 1;
                v
            })
        })
    }
}

/// **Whether a rack-generated member can be built at the shift it is given.**
///
/// Four things stop a tooth existing, and they are the same four whatever chose
/// the shift: it is below the least that clears undercut, the flank is undercut
/// anyway, the tooth comes to a point before its tip, or the root round asked
/// for no longer fits the space. One place for them, so that a bound added here
/// reaches every stage that chooses shifts rather than only the search it was
/// written in — which is how the root round came to bound a pair and not an
/// epicyclic set.
///
/// A **ring is not asked**: its root and its fillet are its shaper's rather than
/// inputs of its own (docs/reference.md#internal-gears), so three of the four
/// mean nothing there and the fourth is asked of the tool instead. A stage's
/// internal meshes carry their own bounds — the tip margin, and the
/// interference flags a `RingMesh` reports — which are about the pair rather
/// than about one member.
///
/// Takes the tooth rather than its parameters because every caller has already
/// built one — undercut and a severed tip are read off the form — and building
/// it twice per candidate is a cost a search pays a thousand times over.
///
/// # The floor is optional, and that is the `no undercut` constraint
///
/// `Some(x_min)` asks all four; `None` asks the last two only. Undercut is a
/// **design choice** rather than a fault — a designer who wants an undercut
/// tooth is entitled to one, and the first two questions are that one question
/// asked twice, numerically and off the form, so they are relieved together or
/// not at all. The other two are not a choice: a severed tooth and a root round
/// that will not fit are shapes no cutter leaves, and nothing relieves them.
#[must_use]
pub fn member_is_buildable(tooth: &Tooth, floor: Option<f64>) -> bool {
    let undercut_ok =
        floor.is_none_or(|f| tooth.params.profile_shift >= f - 1e-12 && !tooth.undercut);
    undercut_ok && !tooth.severed && root_radius_fits(&tooth.params, tooth.params.dedendum)
}

/// **Whether a shaper-cut member is the part its cutter would leave.**
///
/// The counterpart of [`member_is_buildable`], and the answer to the question
/// that one declines: *a ring is not asked* the four a rack asks, because its
/// root and its fillet are its shaper's rather than inputs of its own — so it is
/// asked of the **tool** instead, and this is where.
///
/// [`crate::Ring`] records every guard that altered its geometry, so the
/// question is already answered by the time a candidate exists: a space capped
/// against the pitch, a space raised off zero, a tip lifted to the base circle,
/// a root past where the two flanks close. **Any of them means the tool did not
/// leave the shape the shift asked for**, which is exactly what
/// [`member_is_buildable`] refuses on a rack-cut member and for the same reason:
/// a search may hand back a part somebody has to make, and a shift it has to be
/// talked out of is not one.
///
/// # Why this is not a list of guards
///
/// It asks whether *anything* was altered rather than naming which alterations
/// count. A list would be a list to keep in step, and the distinction it would
/// draw does not exist: every entry in `clamps` is by definition a place the
/// geometry is not what was asked for. A cutter too large for its ring is
/// altered as surely as a space too wide, and a set whose tool cannot cut it has
/// nothing to optimise — it falls back to the plain shifts, which is what an
/// inadmissible pair does too, and the clamp is still reported to the reader.
///
/// # It costs nothing
///
/// Every caller has already built the ring — its clamps are read off the same
/// construction the mesh and the path come from — so this is a field lookup
/// rather than a second cut.
#[must_use]
pub fn ring_is_cut_as_asked(ring: &crate::ring::Ring) -> bool {
    ring.clamps.is_empty()
}

/// **A member of a mesh a search is considering, and the tool that cut it.**
///
/// The tool is the parameter rather than the branch: a rack-cut member answers
/// the four questions [`member_is_buildable`] asks, a shaper-cut one answers of
/// its cutter ([`ring_is_cut_as_asked`]), and nothing above here needs to know
/// which it is holding.
#[derive(Clone, Copy, Debug)]
pub enum Cut<'a> {
    /// Rack-generated. `floor` is the least shift that clears undercut, and
    /// `None` where the designer has said this member may undercut.
    ByRack {
        tooth: &'a Tooth,
        floor: Option<f64>,
    },
    /// Rack-generated at a shift the designer **gave**, which a search may not
    /// overrule and so does not judge: a clamp the tool raised on this member
    /// is a fact about the design, not about the candidate. A worm's thread is
    /// the case — its round is capped at every shift, and a search that asked
    /// the pinned worm to be as asked refused every wheel.
    Pinned { tooth: &'a Tooth },
    /// Shaper-generated — a ring, whose root and fillet are its cutter's.
    ByShaper { ring: &'a crate::ring::Ring },
}

impl Cut<'_> {
    /// Where this member's usable flank begins and ends — whichever tool cut it.
    #[must_use]
    pub fn flank_ends(&self) -> crate::mesh::FlankEnds {
        match self {
            Self::ByRack { tooth, .. } | Self::Pinned { tooth } => tooth.flank_ends(),
            Self::ByShaper { ring } => ring.flank_ends(),
        }
    }

    /// Whether this member is the part the shift asked for.
    #[must_use]
    pub fn is_as_asked(&self) -> bool {
        match self {
            Self::ByRack { tooth, floor } => member_is_buildable(tooth, *floor),
            // Given, so not judged — see the variant.
            Self::Pinned { .. } => true,
            Self::ByShaper { ring } => ring_is_cut_as_asked(ring),
        }
    }

    /// Tip radius, mm.
    #[must_use]
    pub fn tip_radius(&self) -> f64 {
        match self {
            Self::ByRack { tooth, .. } | Self::Pinned { tooth } => tooth.ra,
            Self::ByShaper { ring } => ring.ra,
        }
    }

    /// Root radius, mm.
    #[must_use]
    pub fn root_radius(&self) -> f64 {
        match self {
            Self::ByRack { tooth, .. } | Self::Pinned { tooth } => tooth.rf,
            Self::ByShaper { ring } => ring.rf,
        }
    }
}

/// **One mesh offered to a search: may it be chosen, and what does it lose?**
///
/// # Why this is not a stage's question
///
/// Three stage kinds each wrote this out, and between them they answered it
/// three different ways: the pair asked whether its teeth bottom out, the
/// epicyclic set and the hula stage did not; the pair and the set asked whether
/// each member could be cut, and a ring was asked nothing by anybody
/// (`docs/corrections.md`). **A constraint belongs to the mesh, not to the
/// arrangement it sits in** — a mesh that bottoms out is a mesh that bottoms
/// out whether a carrier is turning around it or not — so it is asked once here
/// and every kind that builds a mesh gets it.
///
/// What a stage kind still owns is what it genuinely does own: how its meshes
/// are *assembled* — at a clearance-opened centre distance, from a shaper cut,
/// around a crank — and which of them a candidate has. Those are the mechanics.
/// This is what is asked of the result.
///
/// # It costs nothing to ask
///
/// Every field is a reference to something the caller has already built to score
/// the candidate at all. Nothing here cuts a tooth or solves a mesh.
pub struct MeshTrial<'a> {
    /// The two members, in the order the mesh was built.
    pub members: [Cut<'a>; 2],
    /// The mesh as it **runs**, opened by whatever clearance the stage assembles
    /// at — not as the shifts leave it, or the search would optimise a contact
    /// ratio nobody measures.
    pub mesh: &'a crate::mesh::Mesh,
    /// The path of contact on that mesh, read through member 1.
    pub path: &'a crate::contact::ContactPath,
    /// The transverse contact ratio this mesh must keep. Loss falls
    /// monotonically with path length, so without it the least-loss mesh is
    /// always the one whose teeth barely reach and the constraint *is* the
    /// answer; how much margin a design wants over continuous contact is the
    /// stage's decision, not this one's.
    pub min_contact_ratio: f64,
    pub friction: f64,
}

impl MeshTrial<'_> {
    /// What this mesh keeps, driving forward — or `None` where it is not a mesh
    /// a search may choose at all.
    ///
    /// The five refusals are the five ways a mesh can fail to be one: a member
    /// the tool would not leave as asked, a tooth that reaches past the root
    /// circle it runs into, a tip that fouls something it is not meshing with,
    /// a tip that reaches past the **flank** it is meshing with, and contact
    /// that does not stay continuous.
    ///
    /// The fourth is the newest and was the one hole an external pair had: the
    /// interference condition was asked of internal meshes under two classical
    /// names and of external ones not at all, though a long addendum on a small
    /// pinion is exactly where it bites.
    #[must_use]
    pub fn efficiency(&self) -> Option<f64> {
        if !self.members.iter().all(Cut::is_as_asked) {
            return None;
        }
        let gap = self.mesh.bottom_clearance(
            self.members.map(|m| m.tip_radius()),
            self.members.map(|m| m.root_radius()),
        );
        if gap.iter().any(|g| *g < 0.0) {
            return None;
        }
        if !self.tips_are_clear() {
            return None;
        }
        // **The mate's tip must not reach past a flank's usable end**, asked of
        // both members and of **every** mesh. `tips_are_clear` above is the
        // internal pair's other question — whether two tips foul away from the
        // line of action — and is `true` by construction on an external pair;
        // this one is not, and had never been asked of one at all.
        if self
            .mesh
            .flank_interference(self.members.map(|m| m.flank_ends()))
            .iter()
            .any(|&bad| bad)
        {
            return None;
        }
        if self.path.contact_ratio < self.min_contact_ratio {
            return None;
        }
        let (Cut::ByRack { tooth, .. } | Cut::Pinned { tooth }) = self.members[0] else {
            // The path and the efficiency integral are read through member 1,
            // which every mesh here builds pinion-first — a ring is never it.
            return None;
        };
        Some(crate::contact::efficiency(
            self.path,
            self.mesh,
            tooth,
            self.friction,
            crate::contact::Drive::Forward,
        ))
    }

    /// **Whether either tip reaches something it is not meshing with.**
    ///
    /// The bottoming check above asks whether a tip reaches the *root circle*
    /// across the line of centres, which is a question about the two members'
    /// radii and nothing else. An internal pair has three more, and they are
    /// three because an internal mesh's two members curve the same way:
    ///
    /// - the pinion's tip reaching past where the ring's flank ends, into the
    ///   fillet its shaper left (**trochoid interference**);
    /// - the ring's tip reaching below where the pinion's flank ends, or not
    ///   reaching its involute at all (**involute interference**);
    /// - and the tips fouling **away from the line of action** entirely, which
    ///   the first two cannot see: they ask what happens where the teeth mesh,
    ///   and this asks whether two teeth try to occupy the same place somewhere
    ///   else. It is the condition that decides small tooth differences, where
    ///   the tip circles cross far from the line of centres and the mesh itself
    ///   is perfectly conjugate.
    ///
    /// An external pair has none of them — its tip circles cross on the line of
    /// centres or not at all — so this is not a branch on the stage kind but on
    /// **what cut the members**, which is the same parameter [`Cut`] already
    /// carries. `None` for an internal pair that cannot be meshed at all is a
    /// refusal too: a search may not choose a pair whose geometry has no answer.
    ///
    /// # Where each kind used to ask it
    ///
    /// The hula stage asked the third, itself, immediately before building this
    /// — and neither of the other two. The epicyclic set asked none of the
    /// three, so its planet-ring mesh could be chosen fouling. That is the same
    /// shape as the gap `MeshTrial` was extracted to close, one layer in:
    /// *a constraint belongs to the mesh*, and a ring's tip is the mesh's
    /// business wherever the ring is.
    ///
    /// The four numbers themselves are [`crate::train::TipRoom`], which is what
    /// the mesh **reports**; this is the one bit of it a search needs, so the
    /// reader and the constraint cannot come to different conclusions.
    ///
    /// # It is asked at the zero-backlash distance, which is the tighter one
    ///
    /// [`crate::ring::mesh_with`] solves the pair's own centre distance from its
    /// shifts, and a stage then assembles at that distance *opened* by its
    /// running clearance. Opening it can only move the tips apart, so a pair
    /// that clears here clears where it runs. The bias is stated in
    /// `docs/state.md` with the rest.
    fn tips_are_clear(&self) -> bool {
        let ([Cut::ByRack { tooth, .. } | Cut::Pinned { tooth }, Cut::ByShaper { ring }]
        | [Cut::ByShaper { ring }, Cut::ByRack { tooth, .. } | Cut::Pinned { tooth }]) =
            self.members
        else {
            return true;
        };
        crate::train::TipRoom::of(ring, tooth).is_some_and(|t| t.tips_clear())
    }
}

/// **What the search may not do**, as opposed to what it is trying to achieve.
///
/// Each field eliminates a range of shifts rather than reshaping the surface
/// being searched, which is the whole of how constraints enter here: the
/// optimum is wherever it is, and these say which parts of the plane the answer
/// is not allowed to come from.
#[derive(Debug, Clone, Copy, Default)]
pub struct Bounds {
    /// The least shift each gear may take — the undercut minimum, which is a
    /// floor and not an answer, and `None` where the designer has said this
    /// gear may undercut ([`member_is_buildable`]).
    pub floor: [Option<f64>; 2],
    /// The transverse contact ratio the pair must keep. Loss falls
    /// monotonically with path length, so without this the least-loss pair is
    /// always the one whose teeth barely reach and the constraint *is* the
    /// answer; it belongs to the stage, not to this function, because how much
    /// margin a design wants over continuous contact is a design decision.
    pub min_contact_ratio: f64,
    /// How far the stage opens the zero-backlash distance to assemble, so the
    /// pair is rated where it actually touches.
    pub clearance: f64,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Pinned {
    /// A shift a designer gave, per gear.
    pub shift: [Option<f64>; 2],
    /// The signed shift sum a given centre distance implies, if one was given.
    pub sum: Option<f64>,
}

impl Pinned {
    /// How many of the two unknowns are still free.
    #[must_use]
    pub fn freedoms(&self) -> usize {
        2 - usize::from(self.shift[0].is_some())
            - usize::from(self.shift[1].is_some())
            - usize::from(self.sum.is_some())
    }

    /// **The pair's two shifts, from the numbers left free.**
    ///
    /// `sign` is the mesh's, so gear 2 enters the sum negated for a ring and the
    /// two orderings are one expression rather than a branch on the kind.
    fn place(&self, sign: f64, free: &[f64]) -> [f64; 2] {
        match (self.shift[0], self.shift[1], self.sum) {
            (Some(a), Some(b), _) => [a, b],
            (Some(a), None, Some(s)) => [a, (s - a) / sign],
            (None, Some(b), Some(s)) => [s - sign * b, b],
            (Some(a), None, None) => [a, free[0]],
            (None, Some(b), None) => [free[0], b],
            // Only the sum is pinned: the free coordinate is the division, taken
            // as gear 1's shift with gear 2's following.
            (None, None, Some(s)) => [free[0], (s - free[0]) / sign],
            // **The sum and the division, not the two shifts.** Those are the
            // pair's own coordinates: the sum alone sets the operating pressure
            // angle and so the length of the path, while the division only
            // moves the path's two ends against each other. In the shifts
            // themselves that structure lies along a diagonal, and a search
            // that moves one shift at a time can only zig-zag up it — here each
            // axis is one of the two effects, and the flat one is flat.
            (None, None, None) => [
                (free[0] + free[1]) / 2.0,
                (free[0] - free[1]) / (2.0 * sign),
            ],
        }
    }

    /// **...and the box those free numbers live in**, given what each gear's own
    /// shift may be.
    ///
    /// This is [`Self::place`] read backwards, and it is written directly
    /// beneath it on the same seven cases for that reason: the two are one
    /// decision seen from its two ends, and forty lines apart they are two
    /// places to edit and one to forget. `None` where the cases leave the free
    /// coordinate nothing to take.
    fn box_of(&self, sign: f64, per_gear: [(f64, f64); 2]) -> Option<Vec<(f64, f64)>> {
        let (a, b) = (per_gear[0], per_gear[1]);
        // Gear 2 as it enters the *sum*, negated for a ring and reordered so the
        // interval still runs low to high.
        let (lo, hi) = (sign * b.0, sign * b.1);
        let signed = (lo.min(hi), lo.max(hi));
        let overlap = |x: (f64, f64), y: (f64, f64)| -> Option<(f64, f64)> {
            let (lo, hi) = (x.0.max(y.0), x.1.min(y.1));
            (lo < hi).then_some((lo, hi))
        };
        match (self.shift[0], self.shift[1], self.sum) {
            // Nothing free, so nothing to sweep.
            (Some(_), Some(_), _) | (Some(_), None, Some(_)) | (None, Some(_), Some(_)) => {
                Some(Vec::new())
            }
            // One shift free: its own interval, and nothing else bears on it.
            (Some(_), None, None) => Some(vec![b]),
            (None, Some(_), None) => Some(vec![a]),
            // The sum is pinned, so gear 2 follows gear 1 — which makes gear 2's
            // interval a second bound on gear 1 rather than an axis of its own.
            (None, None, Some(s)) => Some(vec![overlap(a, (s - signed.1, s - signed.0))?]),
            // Both free, in the pair's own coordinates. The rectangle that
            // *contains* the rotated interval, since a point outside the
            // admissible set is refused by the objective anyway — a tight hull
            // would be a second statement of the same constraint.
            (None, None, None) => Some(vec![
                (a.0 + signed.0, a.1 + signed.1),
                (a.0 - signed.1, a.1 - signed.0),
            ]),
        }
    }
}

/// The profile shifts a pair loses least at, given what is already pinned and a
/// floor neither may go below.
///
/// # What the floor is
///
/// [`automatic_profile_shift`] — the least shift that clears undercut. It is a
/// **floor and not an answer**: an external pair loses least well above it,
/// because the loss is an integral along the whole path of contact and positive
/// shift shortens the path (docs/reference.md#efficiency-parallel-axes). On
/// 17/43 the difference is 1.6 points of mesh efficiency and a contact ratio of
/// 1.48 against 3.04 — which is the trade a designer is being handed, not one
/// this decides for them.
///
/// # The pair's own two directions, and both of them are searched
///
/// Moving the two shifts *apart* at a fixed sum leaves the operating pressure
/// angle where it was, so only the path's two ends move; moving them *together*
/// changes the operating pressure angle, and with it the base pitch, the
/// operating radii and both ends at once. That is why the search is run in the
/// sum and the division rather than in the two shifts — one of the two is flat,
/// and in the shifts themselves it lies along a diagonal.
///
/// The division has a stationary condition in closed form
/// ([`crate::contact::split_residual`]) and **this does not use it**, which the
/// documents said otherwise of until it was checked. Two reasons, both recorded
/// in `AUDIT.md`: as derived it assumes both tips move at `m` per unit shift,
/// which `no_sharp_tip` makes false on the shipped default; and wiring the
/// corrected form in trades a bounded one-dimensional search for five bracketed
/// solves — the interval's two ends, the cap's onset, and a stationary point per
/// smooth piece — which is not a saving in anything but a claim.
///
/// Both directions are cheap either way: a trial pair is two `Tooth`s and a
/// path.
///
/// # Errors
///
/// `None` when no admissible pair exists — every candidate undercut, pointed, or
/// with contact that does not stay continuous.
#[must_use]
pub fn shifts_for_efficiency(
    pair: &dyn Fn([f64; 2]) -> [GearParams; 2],
    kind: crate::mesh::MeshKind,
    bounds: &Bounds,
    pinned: &Pinned,
    friction: f64,
    search: &Search,
) -> Option<[f64; 2]> {
    let Bounds {
        floor,
        min_contact_ratio,
        clearance,
    } = *bounds;
    let sign = kind.sign();
    let loss_at = |x: [f64; 2]| -> Option<f64> {
        // The floor is this caller's own bound and costs a comparison, so a
        // candidate below it is dropped before anything is built for it. That
        // is not a second opinion about what is admissible — `member_is_buildable`
        // asks the same question below and is the one that answers it — it is
        // declining to build a tooth already known to be out of bounds, which
        // over a search of some hundreds of candidates is most of the work.
        if [0, 1]
            .iter()
            .any(|&i| floor[i].is_some_and(|f| x[i] < f - 1e-12))
        {
            return None;
        }
        let [pa, pb] = pair(x);
        let (a, b) = (Tooth::new(pa), Tooth::new(pb));
        // **The pair as it runs, not as its shifts leave it.** The stage opens
        // the zero-backlash distance by its assembly clearance and rates
        // contact there (docs/reference.md#centre-distance-and-backlash); so
        // does this, or the search would optimise a contact ratio nobody
        // measures and settle just under the floor it was given.
        //
        // That opening is the whole of what a *pair* contributes: how its mesh
        // is assembled. What is then asked of the mesh belongs to the mesh
        // ([`MeshTrial`]) and is the same question every kind asks.
        let zero_backlash = crate::mesh::Mesh::new(&a, &b, kind).ok()?;
        let mesh = zero_backlash.at(zero_backlash.a_w + clearance).ok()?;
        let path = crate::contact::ContactPath::new(&a, b.ra, &mesh)?;
        MeshTrial {
            members: [
                member(&a, floor[0], pinned.shift[0]),
                member(&b, floor[1], pinned.shift[1]),
            ],
            mesh: &mesh,
            path: &path,
            min_contact_ratio,
            friction,
        }
        .efficiency()
    };

    // The pair that a set of free coordinates describes, given what is pinned.
    if pinned.freedoms() == 0 {
        let x = pinned.place(sign, &[]);
        return loss_at(x).map(|_| x);
    }

    // **The box the search sweeps is the one the shifts can actually take**, per
    // member, from [`searchable_shift`] — the same rule the objective enforces
    // rather than a second reading of it — and turned into the free
    // coordinates' own by [`Pinned::box_of`].
    //
    // It used to sweep a fixed `±3` modules, which is a guess at where shifts
    // live rather than a statement of it, and the guess is what
    // `docs/corrections.md` records: pin the sum and the admissible interval can
    // be narrower than the sweep's own step, at which point the search reports
    // that no admissible pair exists because its grid fell either side of one.
    let box_ = pinned.box_of(sign, sides(pair, &floor, pinned)?)?;
    search
        .maximise(&box_, &|free| loss_at(pinned.place(sign, free)))
        .map(|free| pinned.place(sign, &free))
}

/// **The interval each member's shift may be searched over** — and for a
/// member whose shift was given, the given number alone, since an interval
/// the search will never sweep is not one it should have to exist. A pinned
/// worm's round is capped at every shift, so its interval is empty, and asking
/// for it refused the whole search before the wheel was looked at.
fn sides(
    pair: &dyn Fn([f64; 2]) -> [GearParams; 2],
    floor: &[Option<f64>; 2],
    pinned: &Pinned,
) -> Option<[(f64, f64); 2]> {
    let side = |i: usize| match pinned.shift[i] {
        Some(x) => Some((x, x)),
        None => searchable_shift(&|x| pair([x, x])[i], floor[i]),
    };
    Some([side(0)?, side(1)?])
}

/// A member of a trial pair: judged where a search is choosing its shift,
/// taken as given where the designer did.
fn member<'a>(tooth: &'a Tooth, floor: Option<f64>, given: Option<f64>) -> Cut<'a> {
    if given.is_some() {
        Cut::Pinned { tooth }
    } else {
        Cut::ByRack { tooth, floor }
    }
}

/// **The same choice on crossed shafts**: the shifts a crossed pair loses
/// least at, by the friction balance along its line of action.
///
/// [`shifts_for_efficiency`] with the mesh swapped — the same floor, the same
/// pinning, the same box, the same search, and the same five refusals asked of
/// a point contact: a member the tool would not leave as asked, a tip that
/// bottoms in the mate's root, a tip that reaches past the flank it meshes with
/// ([`crate::screw::CrossedPath::flank_interference`]), and contact that does
/// not stay continuous; the internal pair's tip question does not arise. The
/// objective is [`crate::screw::CrossedPath::efficiency`], forward, on the zone
/// the *teeth* leave — not the faces, which the stage sizes to the answer.
///
/// The shift sum enters through the rack law
/// ([`crate::screw::ScrewParams::profile_shifts`]), so `screw_at` is the pair
/// at those shifts and the zero-backlash distance is its own; the stage opens
/// it by the clearance as it does everywhere.
///
/// # Errors
///
/// `None` when no admissible pair exists.
#[must_use]
pub fn crossed_shifts_for_efficiency(
    pair: &dyn Fn([f64; 2]) -> [GearParams; 2],
    screw_at: &dyn Fn([f64; 2]) -> Option<crate::screw::Screw>,
    bounds: &Bounds,
    pinned: &Pinned,
    friction: f64,
    search: &Search,
) -> Option<[f64; 2]> {
    let Bounds {
        floor,
        min_contact_ratio,
        clearance,
    } = *bounds;
    // Both members external: the sign is the pair's own.
    let sign = crate::mesh::MeshKind::External.sign();
    let loss_at = |x: [f64; 2]| -> Option<f64> {
        if [0, 1]
            .iter()
            .any(|&i| floor[i].is_some_and(|f| x[i] < f - 1e-12))
        {
            return None;
        }
        let [pa, pb] = pair(x);
        let (a, b) = (Tooth::new(pa), Tooth::new(pb));
        let members = [
            member(&a, floor[0], pinned.shift[0]),
            member(&b, floor[1], pinned.shift[1]),
        ];
        if !members.iter().all(Cut::is_as_asked) {
            return None;
        }
        let s = screw_at(x)?;
        let centre = s.centre_distance + clearance;
        // A tip reaching the mate's root across the line of centres — the
        // radial comparison `Mesh::bottom_clearance` makes, on a crossed pair's
        // own distance.
        if centre - a.ra - b.rf < 0.0 || centre - b.ra - a.rf < 0.0 {
            return None;
        }
        let path = s.path_of_contact_at(a.ra, b.ra, centre)?;
        if path
            .flank_interference(&s, [a.flank_ends(), b.flank_ends()])
            .iter()
            .any(|&bad| bad)
        {
            return None;
        }
        if path.contact_ratio < min_contact_ratio {
            return None;
        }
        path.efficiency(&s, friction, crate::contact::Drive::Forward, SEARCH_SAMPLES)
    };

    if pinned.freedoms() == 0 {
        let x = pinned.place(sign, &[]);
        return loss_at(x).map(|_| x);
    }
    let box_ = pinned.box_of(sign, sides(pair, &floor, pinned)?)?;
    search
        .maximise(&box_, &|free| loss_at(pinned.place(sign, free)))
        .map(|free| pinned.place(sign, &free))
}

/// Quadrature points for the friction balance **inside the search**.
///
/// The stage reports the balance at 2048 points, where the trapezium rule's
/// residual is below 1e-9 (`train::crossed`). A search does not need that: it
/// resolves shifts to `Search::resolution`, and the objective moves by
/// hundredths of a point per module of shift, so a residual of 1e-7 is a
/// hundredth of the step it could ever tell apart. The rule is second order,
/// so an eighth of the points buys that sixty-four times over.
const SEARCH_SAMPLES: usize = 256;

/// **Coordinate descent over a few free numbers**, refined about the best.
///
/// The shift surfaces this searches are smooth and shallow, with the optimum
/// pushed against a constraint rather than sitting in a bowl, so a bounded
/// sweep on each axis in turn converges in a handful of passes and needs no
/// derivative in a direction that has none. `objective` returns `None` where
/// the point is not admissible at all, which is how a constraint eliminates a
/// range rather than penalising one.
///
/// It is here, rather than inside the one function that first needed it,
/// because every stage that chooses shifts for efficiency chooses a different
/// number of them against a different objective: a pair has its mesh, and an
/// epicyclic has two meshes whose shifts a shared centre distance ties
/// together, so only the search is common.
#[must_use]
pub fn maximise(
    box_: &[(f64, f64)],
    objective: &dyn Fn(&[f64]) -> Option<f64>,
) -> Option<Vec<f64>> {
    Search::SHIPPED.maximise(box_, objective)
}

/// **How hard [`maximise`] looks**, as a value rather than as six constants
/// inside its loop.
///
/// # Why these are a parameter
///
/// "This search is converged" is a claim *about* these numbers, and a claim
/// nothing can raise is a claim nothing can check. They were first chosen when a
/// candidate cost fifty times what it does now, so they are worth re-asking
/// rather than inheriting — and asking means running the same searches at a
/// budget nobody would ship and comparing. That is
/// `the_search_is_converged_not_budgeted`, which is the only reason this type
/// exists; [`Search::SHIPPED`] is what every caller uses.
///
/// None of them is a model constant. Each says how much work to spend, and the
/// answer they are spending it on is decided by the objective and its
/// constraints alone — so raising any of them may only sharpen an answer, never
/// move it somewhere else.
///
/// # What checking it found
///
/// The comment this replaces claimed all three of the crate's searches were
/// converged — "the pair's answer not at all to eight decimals, the set's by
/// 2e-7". **Half of that is true.** A pair's is converged: over fourteen tooth
/// pairs, fourteen times the work moves its efficiency by at most 4.1e-7 and
/// every shift by less than one step of `resolution`. An epicyclic set's is
/// not: over thirty sets it moves `η₀` by up to **2.4e-4**, with the sun's shift
/// **0.30 modules** away — a visibly different gear — and it moves in *both*
/// directions, so more effort sometimes finds a worse answer than the shipped
/// budget did. No budget fixes that.
///
/// The difference is coordinates rather than effort. A pair is searched in the
/// pair's own two directions, the shift sum and the division, so its flat
/// direction is an axis; a set is searched in two of its three raw shifts,
/// which are nobody's natural coordinate, and its walk slides along a curved
/// bound instead of climbing. `AUDIT.md` F50 carries the sweep and what is to
/// be done about it.
#[derive(Clone, Copy, Debug)]
pub struct Search {
    /// Steps across each axis of the opening sweep, so `scan + 1` points a side.
    ///
    /// **Across the caller's own box**, not across a guess at one. It used to
    /// sweep a fixed `±3` modules at this many steps a side, which put its grid
    /// at multiples of half a module — and where a caller's admissible interval
    /// is narrower than that and falls between two of them, the sweep collected
    /// *nothing* and the search reported that no admissible point exists. A
    /// resolution had become a feasibility test (`docs/corrections.md`).
    pub scan: i32,
    /// Below this the answer has stopped moving in any units a tooth is cut in
    /// — a thousandth of a module is finer than the tolerance any of this is
    /// ground to, and the surface is flat at that scale anyway.
    pub resolution: f64,
    /// A guard on **one walk**, so a walk that cannot converge stops rather than
    /// running for ever.
    ///
    /// # It is per walk because a pool is not a guard
    ///
    /// This was a pool shared across every start, and the measurement that
    /// settled it is the plainest in the crate: a walk terminates on its own
    /// resolution after **80 to 608** evaluations, so the first walk of six spent
    /// the entire pool of 220 and **the other five never ran at all**. On the
    /// shipped epicyclic set that cost the answer — the third start is the one
    /// that finds the better ridge, and it was never reached, leaving **3.2e-4**
    /// of `η₀` on the table.
    ///
    /// A pool is also an order dependence: which starts run is decided by how
    /// expensive the earlier ones happened to be. Per walk, every start runs,
    /// and the number below is a ceiling nothing reaches rather than a share
    /// everything divides.
    ///
    /// # Why not simply a bigger pool
    ///
    /// Because a pool sized for the kind that needs the most is spent by the
    /// kinds that do not: at ten times the pool a pair's search cost **24×** what
    /// it had, which is what a keystroke does not have
    /// (`every_search_is_quick_enough_to_type_over`). A guard per walk costs each
    /// kind what its own walks cost and nothing more.
    ///
    /// **Gated rather than tuned.** `the_search_is_converged_not_budgeted`
    /// quadruples this and demands the same answers, which is the claim a guard
    /// makes and a truncation cannot.
    pub budget: usize,
    /// How many of the sweep's best points are walked from.
    pub starts: usize,
    /// The walk's first step, as a fraction of the sweep's own spacing.
    ///
    /// **The sweep chose the region; the walk only refines inside it.** A first
    /// step near the spacing lets a walk cross into a neighbouring basin on a
    /// marginal improvement and then settle there, which on 9/37 costs the
    /// summit — so it starts well below that and climbs the ridge it was put on.
    pub first_step: f64,
}

impl Search {
    /// What every caller in the crate uses.
    pub const SHIPPED: Self = Self {
        scan: 12,
        resolution: 1e-3,
        budget: 2_000,
        starts: 2,
        first_step: 1.0 / 8.0,
    };

    /// The same search asked to work `k` times as hard in every direction at
    /// once: a finer sweep, more starts of it, a longer walk and a finer stop.
    ///
    /// One knob, because the claim being checked is about the whole of the
    /// effort and raising one number at a time would let another bind instead.
    #[must_use]
    pub fn refined(k: u32) -> Self {
        let k = k.max(1) as usize;
        Self {
            scan: Self::SHIPPED.scan * i32::try_from(k).unwrap_or(i32::MAX),
            budget: Self::SHIPPED.budget * k * k,
            starts: Self::SHIPPED.starts * k,
            #[allow(clippy::cast_precision_loss)]
            resolution: Self::SHIPPED.resolution / k as f64,
            ..Self::SHIPPED
        }
    }

    /// **How many times harder than the shipped search this is**, as the `k` a
    /// caller passed [`Self::refined`].
    ///
    /// Recovered from `starts`, which `refined` scales by exactly `k`. It exists
    /// so a caller whose search has *outer* structure — the hula stage solves a
    /// crank and chooses the splits, and does that a few times round — can scale
    /// that structure by the same effort rather than carrying an effort of its
    /// own that nothing lines up with.
    #[must_use]
    pub fn effort(&self) -> usize {
        (self.starts / Self::SHIPPED.starts).max(1)
    }

    /// As [`maximise`], at this effort.
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub fn maximise(
        &self,
        box_: &[(f64, f64)],
        objective: &dyn Fn(&[f64]) -> Option<f64>,
    ) -> Option<Vec<f64>> {
        let Self {
            scan,
            resolution,
            budget,
            starts: start_count,
            first_step,
        } = *self;
        let dof = box_.len();

        // **Every direction, not one axis at a time.** These surfaces have flat
        // ridges and their optima sit against constraints, and at such a corner no
        // single coordinate can improve while a diagonal still can — a search that
        // moved one number at a time would stop short and report the point it
        // stopped at.
        let mut directions: Vec<Vec<f64>> = vec![vec![]];
        for _ in 0..dof {
            directions = directions
                .iter()
                .flat_map(|d| {
                    [-1.0, 0.0, 1.0].map(|c| {
                        let mut next = d.clone();
                        next.push(c);
                        next
                    })
                })
                .collect();
        }
        directions.retain(|d| d.iter().any(|c| *c != 0.0));

        // **Sweep the box, then walk from the best few points of it.**
        //
        // One walk is not enough and neither is one start. Constraints carve the
        // admissible set into regions, and because the answer lies *against* a
        // constraint rather than in a bowl, the ridge a walk ends on is decided by
        // the point it began at — the sweep's own best is regularly on a different
        // ridge from the highest one. Walking from several of its best points and
        // keeping the best result is what makes the answer the surface's rather
        // than the starting point's.
        // **The box is the caller's**, so the sweep's step is a fraction of the
        // interval a number can actually take rather than of a guess at one. The
        // smallest interval decides the walk's own scale, since a step that
        // crosses a narrow axis in one stride is not a refinement of anything.
        let step_on = |axis: usize| (box_[axis].1 - box_[axis].0) / f64::from(scan);
        let spacing = (0..dof).map(step_on).fold(f64::INFINITY, f64::min);
        let mut scanned: Vec<(Vec<f64>, f64)> = Vec::new();
        let mut corner = vec![0i32; dof];
        loop {
            let at: Vec<f64> = corner
                .iter()
                .enumerate()
                .map(|(axis, c)| box_[axis].0 + f64::from(*c) * step_on(axis))
                .collect();
            if let Some(value) = objective(&at) {
                scanned.push((at, value));
            }
            let mut axis = 0;
            while axis < dof {
                corner[axis] += 1;
                if corner[axis] <= scan {
                    break;
                }
                corner[axis] = 0;
                axis += 1;
            }
            if axis == dof {
                break;
            }
        }
        // **The best points of the sweep, and its outermost ones.**
        //
        // Loss falls with the length of the path and the path shortens as the shifts
        // grow, so the answer is regularly the largest shifts a design admits —
        // pressed against whichever bound stops them, with a dip in between that a
        // walk started anywhere inside will settle into instead. Measured on 9/37:
        // a local peak at a sum of 1.1, a trough at 1.4, and the true summit at
        // 1.6 where the root round runs out. Starting from the extremes of the
        // admissible set as well as from its best interior points is what reaches
        // the second one.
        scanned.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut starts: Vec<Vec<f64>> = scanned
            .iter()
            .take(start_count)
            .map(|(at, _)| at.clone())
            .collect();
        for axis in 0..dof {
            for far in [f64::max, f64::min] {
                if let Some((at, _)) = scanned.iter().reduce(|a, b| {
                    if far(a.0[axis], b.0[axis]) == a.0[axis] {
                        a
                    } else {
                        b
                    }
                }) {
                    starts.push(at.clone());
                }
            }
        }
        // **Dedup needs the neighbours it compares to be neighbours.** These are
        // grid points gathered from two different rules — the sweep's best few
        // and its outermost — so the same point arrives twice and not next to
        // itself, which `Vec::dedup` cannot see. On the shipped epicyclic set
        // that was one walk in six spent re-walking a point already walked.
        // Sorted first, on the coordinates themselves, since they are exact grid
        // values rather than anything a tolerance would have to soften.
        starts.sort_by(|a, b| {
            a.iter()
                .zip(b)
                .map(|(x, y)| x.total_cmp(y))
                .find(|o| o.is_ne())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        starts.dedup();

        let mut best: Option<(Vec<f64>, f64)> = None;
        for from in starts {
            let Some(value) = objective(&from) else {
                continue;
            };
            // **Per walk, not a pool.** Shared, the first walk's cost decided how
            // many walks there were — see [`Search::budget`].
            let mut spent = 0usize;
            let (mut at, mut here) = (from, value);
            // **The sweep chose the region; the walk only refines inside it.** A
            // first step near the sweep's own spacing lets a walk cross into a
            // neighbouring basin on a marginal improvement and then settle there,
            // which on 9/37 costs the summit — so it starts well below that spacing
            // and climbs the ridge it was put on.
            //
            // **...but never below the distance it stops at**, or it stops before
            // it starts: `step > resolution` is false before the body runs and
            // the walk is a no-op, leaving the sweep's grid point unrefined. That
            // happens whenever the box is narrow — a fraction of a module wide,
            // which is exactly what pinning a centre distance leaves — and it is
            // where this search was quietly a thirteen-point grid. On 9/37 at a
            // shift sum of 0.56 the optimum sits four ten-thousandths above the
            // undercut floor, between two of those points, and the grid cannot
            // see it (`docs/corrections.md`).
            let mut step = (spacing * first_step).max(resolution);
            while step >= resolution && spent < budget {
                // Every direction is tried from the *same* point and the best taken,
                // rather than the first that happens to improve: otherwise the step
                // a direction is judged by depends on which came before it, and the
                // walk wanders instead of climbing.
                let mut local: Option<(Vec<f64>, f64)> = None;
                for d in &directions {
                    let trial: Vec<f64> = at.iter().zip(d).map(|(a, c)| a + c * step).collect();
                    spent += 1;
                    let Some(value) = objective(&trial) else {
                        continue;
                    };
                    if local.as_ref().is_none_or(|l| value > l.1) {
                        local = Some((trial, value));
                    }
                }
                match local {
                    Some((to, value)) if value > here => {
                        at = to;
                        here = value;
                    }
                    _ => step /= 2.0,
                }
            }
            if best.as_ref().is_none_or(|b| here > b.1) {
                best = Some((at, here));
            }
        }
        best.map(|(at, _)| at)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::note::key;

    /// **The division is the evenest one the members allow** — checked against
    /// a scan that shares none of its arithmetic.
    ///
    /// [`divide_shift_sum`] answers by projection: it intersects two intervals
    /// and clamps the midpoint into the result. The property that is *meant* is
    /// "as even as the members allow", and those are only the same thing if the
    /// projection is right — so this asks a brute scan of the same interval
    /// which point minimises the spread, and requires the two to agree.
    ///
    /// A scan is the right instrument here rather than a second formula: it
    /// shares no bound, no midpoint and no clamp with the thing it checks.
    #[test]
    fn the_shift_sum_is_divided_as_evenly_as_the_members_allow() {
        let pair = |z1: u32, z2: u32| {
            move |i: usize, x: f64| GearParams {
                teeth: if i == 0 { z1 } else { z2 },
                profile_shift: x,
                ..Default::default()
            }
        };
        let mut checked = 0u32;
        for (z1, z2) in [(9u32, 37u32), (17, 43), (13, 13), (20, 31)] {
            let at = pair(z1, z2);
            // The floors a stage would hand it: the undercut shift on each.
            let floor = [0, 1].map(|i| {
                let p = at(i, 0.0);
                Some(automatic_profile_shift(&p, p.dedendum))
            });
            for step in 0..24 {
                let sum = -0.4 + 0.12 * f64::from(step);
                let Some(got) = divide_shift_sum(&at, 1.0, sum, floor) else {
                    continue;
                };
                checked += 1;

                assert!(
                    (got[0] + got[1] - sum).abs() < 1e-9,
                    "{z1}/{z2} sum {sum}: {got:?} does not add up"
                );

                // The independent answer: scan the interval and keep the point
                // with the least spread among the ones that are admissible.
                let (lo0, hi0) = searchable_shift(&|x| at(0, x), floor[0]).unwrap();
                let (lo1, hi1) = searchable_shift(&|x| at(1, x), floor[1]).unwrap();
                let mut best: Option<(f64, f64)> = None;
                const N: u32 = 20_000;
                for k in 0..=N {
                    let x0 = lo0 + (hi0 - lo0) * f64::from(k) / f64::from(N);
                    let x1 = sum - x0;
                    if x1 < lo1 || x1 > hi1 {
                        continue;
                    }
                    let spread = (x0 - x1).abs();
                    if best.is_none_or(|(b, _)| spread < b) {
                        best = Some((spread, x0));
                    }
                }
                let (_, scanned) = best.expect("the projection found a point, so the scan must");
                // One scan step of the wider interval, which is what a grid can
                // resolve — the claim is the location, not the resolution.
                let step0 = (hi0 - lo0) / f64::from(N);
                assert!(
                    (got[0] - scanned).abs() <= 2.0 * step0,
                    "{z1}/{z2} sum {sum}: projected {} but the evenest is {scanned}",
                    got[0]
                );
            }
        }
        assert!(checked >= 40, "only {checked} sums had an answer");
    }

    /// **What that rule does on a pair whose members differ**, stated as the
    /// behaviour a designer sees rather than as a formula.
    ///
    /// A nine-tooth pinion needs about 0.47 modules to clear undercut and a
    /// thirty-seven-tooth wheel needs none. So as a centre distance is opened
    /// from nominal the pinion **cannot move** — it is against its floor — and
    /// the wheel takes all of it, until the two are level and from there they
    /// rise together. Three claims, none of which mentions the projection.
    #[test]
    fn the_member_at_its_floor_stays_there_until_the_other_catches_up() {
        let at = |i: usize, x: f64| GearParams {
            teeth: if i == 0 { 9 } else { 37 },
            profile_shift: x,
            ..Default::default()
        };
        let floor = [0, 1].map(|i| {
            let p = at(i, 0.0);
            Some(automatic_profile_shift(&p, p.dedendum))
        });
        let pinion_floor = floor[0].unwrap();

        let x = |sum: f64| divide_shift_sum(&at, 1.0, sum, floor).expect("reachable");

        // Just above the floor sum: the pinion is pinned, the wheel absorbs.
        let low = x(pinion_floor + 0.05);
        assert!(
            (low[0] - pinion_floor).abs() < 1e-9 && low[1] > 0.0,
            "the pinion should hold at its floor while the wheel takes the rest: {low:?}"
        );

        // Level, and beyond it they move together.
        let level = x(2.0 * pinion_floor);
        assert!(
            (level[0] - level[1]).abs() < 1e-6,
            "at twice the floor the two should be level: {level:?}"
        );
        let high = x(2.0 * pinion_floor + 0.6);
        assert!(
            (high[0] - high[1]).abs() < 1e-6,
            "past level they should rise together: {high:?}"
        );
        assert!(
            high[0] > level[0] && high[1] > level[1],
            "a wider distance shifts both further out: {level:?} then {high:?}"
        );

        // Neither member ever moves backwards as the sum grows. Sums past the
        // point where the members run out have no answer at all, and a skip is
        // the honest reading of that — the claim is about the ones that do.
        let mut previous = x(pinion_floor);
        let mut walked = 0u32;
        for step in 1..20 {
            let Some(now) =
                divide_shift_sum(&at, 1.0, pinion_floor + 0.08 * f64::from(step), floor)
            else {
                break;
            };
            assert!(
                now[0] >= previous[0] - 1e-9 && now[1] >= previous[1] - 1e-9,
                "a wider distance should not pull a member back: {previous:?} then {now:?}"
            );
            previous = now;
            walked += 1;
        }
        assert!(
            walked >= 10,
            "only {walked} sums were reachable to walk over"
        );
    }

    /// **A search may not choose a mesh whose tips foul**, and the same pair
    /// with a shorter ring tooth is admissible again.
    ///
    /// The constraint and the four numbers a reader is given come from one place
    /// ([`crate::train::TipRoom`]), so this pins the half a search uses. Stated
    /// as a *difference* between two rings rather than as "the shipped one is
    /// refused": what is being claimed is that the tip room decides the answer,
    /// and a test that only watched the full-depth pair be refused would pass if
    /// everything were refused.
    #[test]
    fn a_mesh_whose_tips_foul_is_not_one_a_search_may_choose() {
        use crate::ring::{Cutter, Ring};
        let pinion = Tooth::new(GearParams {
            teeth: 20,
            ..Default::default()
        });
        let trial = |addendum: f64| -> (bool, Option<f64>) {
            let ring = Ring::cut_by(
                &GearParams {
                    teeth: 60,
                    addendum,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let as_gear = Tooth::new(GearParams {
                teeth: 60,
                addendum,
                ..Default::default()
            });
            let mesh = crate::mesh::Mesh::new(&pinion, &as_gear, crate::mesh::MeshKind::Internal)
                .expect("a 60/20 internal pair meshes");
            let path = crate::contact::ContactPath::new(&pinion, ring.ra, &mesh)
                .expect("and it reaches contact");
            // **The whole question**, which is now two: the tips crossing away
            // from the line of action is an internal pair's alone, and a tip
            // reaching past a flank's usable end is every mesh's.
            let tips = crate::train::TipRoom::of(&ring, &pinion).expect("an internal mesh");
            let flanks = mesh.flank_interference([pinion.flank_ends(), ring.flank_ends()]);
            (
                tips.tips_clear() && flanks == [false, false],
                MeshTrial {
                    members: [
                        Cut::ByRack {
                            tooth: &pinion,
                            floor: None,
                        },
                        Cut::ByShaper { ring: &ring },
                    ],
                    mesh: &mesh,
                    path: &path,
                    min_contact_ratio: 1.0,
                    friction: 0.08,
                }
                .efficiency(),
            )
        };
        // Full depth: the ring's tip would have to touch the pinion inside its
        // own base circle, and `ring.rs` records that this is what internal
        // pairs do at full depth rather than an unlucky choice of counts.
        let (clear, eta) = trial(1.0);
        assert!(!clear, "a full-depth 60/20 internal pair should foul");
        assert!(eta.is_none(), "and a search may not choose it: {eta:?}");
        // Shortened: the same pair, admissible, and losing something sensible.
        let (clear, eta) = trial(0.8);
        assert!(clear, "a shorter ring tooth should clear it");
        let eta = eta.expect("and then the mesh is one a search may choose");
        assert!(
            (0.9..1.0).contains(&eta),
            "an internal pair should keep most of what it is given: {eta}"
        );
    }

    /// The tooth-count thresholds in the design document, reproduced from the
    /// formula rather than quoted. These are the numbers the control exists to
    /// expose, so they are worth pinning.
    #[test]
    fn the_sharp_rack_reproduces_the_classical_tooth_count_rule() {
        // z_min is where x_min crosses zero. Solve it by bisection on z so the
        // test shares no algebra with the implementation.
        let threshold = |working_depth: f64, root_radius: f64| {
            let (mut lo, mut hi) = (3.0_f64, 400.0_f64);
            for _ in 0..200 {
                let mid = 0.5 * (lo + hi);
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let p = GearParams {
                    teeth: mid as u32,
                    root_radius,
                    ..Default::default()
                };
                // Evaluate continuously in z by scaling the radius term directly.
                let alpha_t = p.pressure_angle.to_radians();
                let sa = alpha_t.sin();
                let r = p.module * mid / 2.0;
                let rho = root_radius * p.module;
                let x_min = working_depth - (rho + sa * (r * sa - rho)) / p.module;
                if x_min > 0.0 {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            0.5 * (lo + hi)
        };

        assert!((threshold(1.0, 0.0) - 17.10).abs() < 0.01);
        assert!((threshold(1.25, 0.0) - 21.37).abs() < 0.01);
        assert!((threshold(1.0, 0.38) - 12.82).abs() < 0.01);
    }

    #[test]
    fn minimum_shift_matches_the_closed_form_and_the_cutter_helps() {
        let p = GearParams {
            teeth: 12,
            ..Default::default()
        };
        let m = minimum_profile_shift(&p, 1.0);

        // Sharp rack: x_min = h_w - z sin^2(a)/2, computed independently.
        let sa = p.pressure_angle.to_radians().sin();
        let want = 1.0 - f64::from(p.teeth) * sa * sa / 2.0;
        assert!(
            (m.sharp_rack - want).abs() < 1e-12,
            "{} vs {want}",
            m.sharp_rack
        );

        // A real cutter tip round always needs LESS shift than a sharp rack.
        assert!(m.with_cutter_radius < m.sharp_rack);
    }

    #[test]
    fn a_gear_at_the_minimum_shift_is_on_the_edge_of_undercut() {
        // The independent check: build the gear at x_min and ask the profile
        // generator — which knows nothing about this module — whether it is
        // undercut. `working_depth` equal to the dedendum makes the two ask the
        // same question.
        for teeth in [9_u32, 12, 17, 25, 40] {
            let p = GearParams {
                teeth,
                dedendum: 1.25,
                ..Default::default()
            };
            let x = minimum_profile_shift(&p, p.dedendum).with_cutter_radius;

            let just_under = Tooth::new(GearParams {
                profile_shift: x - 1e-4,
                ..p
            });
            let just_over = Tooth::new(GearParams {
                profile_shift: x + 1e-4,
                ..p
            });
            assert!(
                just_under.undercut,
                "z={teeth}: should undercut below x_min"
            );
            assert!(
                !just_over.undercut,
                "z={teeth}: should not undercut above x_min"
            );
        }
    }

    /// The strongest check available: set the addendum to what this returns,
    /// generate the gear, and measure the tip width off the result.
    #[test]
    fn the_computed_addendum_produces_the_requested_tip_width() {
        for teeth in [9_u32, 17, 40] {
            for want in [0.05, 0.1, 0.3] {
                let base = GearParams {
                    teeth,
                    ..Default::default()
                };
                let g = Tooth::new(base);
                let h_a = addendum_for_tip_width(&g, want).unwrap();

                let sized = Tooth::new(GearParams {
                    addendum: h_a,
                    ..base
                });
                // Transverse tip width from the generated geometry.
                let got = 2.0 * sized.ra * sized.theta_a;
                assert!(
                    (got - want).abs() < 1e-9,
                    "z={teeth} want {want}: got {got}"
                );
            }
        }
    }

    #[test]
    fn a_zero_tip_width_lands_on_the_pointed_tooth_radius() {
        let base = GearParams {
            teeth: 17,
            ..Default::default()
        };
        let g = Tooth::new(base);
        let h_a = addendum_for_tip_width(&g, 0.0).unwrap();
        let pointed = Tooth::new(GearParams {
            addendum: h_a,
            ..base
        });
        assert!(
            pointed.theta_a.abs() < 1e-9,
            "tip arc should have closed up"
        );

        // ...and asking for more addendum than that is refused by the generator's
        // own cap, so the two agree on where "pointed" is.
        let past = Tooth::new(GearParams {
            addendum: h_a + 0.5,
            ..base
        });
        assert!(
            (past.ra - pointed.ra).abs() < 1e-6,
            "the cap should hold it"
        );
    }

    /// The automatic value is a *choice*, not the bound itself. Applying the raw
    /// minimum to a gear that does not need shifting thins it for nothing, and in
    /// a pair can push the mesh out of the involute domain altogether.
    #[test]
    fn the_automatic_shift_never_thins_a_tooth_that_did_not_need_it() {
        for teeth in [9_u32, 12, 17, 25, 43, 100] {
            let p = GearParams {
                teeth,
                ..Default::default()
            };
            let bound = minimum_profile_shift(&p, 1.0).with_cutter_radius;
            let applied = automatic_profile_shift(&p, 1.0);

            assert!(applied >= 0.0, "z={teeth}: automatic shift went negative");
            assert!(
                applied >= bound - 1e-15,
                "z={teeth}: below the undercut bound"
            );
            if bound > 0.0 {
                assert!(
                    (applied - bound).abs() < 1e-15,
                    "z={teeth}: should sit on the bound"
                );
            }
        }
        // The case that motivated it: a comfortable tooth count has a large
        // negative bound.
        let big = minimum_profile_shift(
            &GearParams {
                teeth: 43,
                ..Default::default()
            },
            1.0,
        );
        assert!(big.with_cutter_radius < -1.5);
    }

    /// The closed-form bounds against the generator itself: just inside each
    /// bound the geometry builds with no such clamp, just outside it clamps.
    /// That is what makes them the *real* limits rather than a tidier constant.
    ///
    /// **The dedendum note is not one of those clamps, and that is the two-tier
    /// distinction.** `clamp.dedendum_raised` fires wherever the shift has moved
    /// the tool out past the depth the dedendum asked for, which is a legal
    /// gear cut by a deeper hob — [`ShiftRange::shallow_cut`] reports where that
    /// begins, and the range deliberately runs past it. Only the thickness
    /// clamps mark geometry that stops being constructible, so only they belong
    /// here. Including the dedendum note is what made this test assert a limit
    /// the eccentric path did not honour, and the bound then stepped by 62 % as
    /// `Δx` left zero (`docs/corrections.md`).
    #[test]
    fn the_admissible_range_is_exactly_where_the_generator_starts_clamping() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                ..Default::default()
            },
            GearParams {
                pressure_angle: 14.5,
                ..Default::default()
            },
            GearParams {
                pressure_angle: 30.0,
                ..Default::default()
            },
            GearParams {
                thickness_mod: 1.3,
                ..Default::default()
            },
            GearParams {
                dedendum: 1.0,
                helix_angle: 20.0,
                ..Default::default()
            },
        ] {
            let r = admissible_profile_shift(&p, 1.0).bound;
            let (lo, hi) = (r.min.unwrap(), r.max.unwrap());
            assert!(lo < hi, "empty range for {p:?}");

            let clamps = |x: f64| {
                Tooth::new(GearParams {
                    profile_shift: x,
                    ..p
                })
                .clamps
                .notes
                .iter()
                .any(|n| {
                    n.is(key::CLAMP_TOOTH_THICKNESS_RAISED)
                        || n.is(key::CLAMP_TOOTH_THICKNESS_CAPPED)
                })
            };
            let eps = 0.01;
            assert!(
                !clamps(hi - eps),
                "clamped inside the top of the range: {p:?}"
            );
            assert!(
                clamps(hi + eps),
                "no clamp above the top of the range: {p:?}"
            );
            assert!(!clamps(lo + eps), "clamped inside the bottom: {p:?}");
            assert!(clamps(lo - eps), "no clamp below the bottom: {p:?}");
        }
    }

    /// **An eccentric gear's shift window is where *every* tooth still builds.**
    ///
    /// Its teeth are cut across `x̄ ± Δx`, so the buildable interval closes in —
    /// and against the *generator*, as for a concentric gear: just inside the
    /// window no distinct tooth carries a thickness clamp, just outside one
    /// does. A concentric gear is `Δx = 0` and the window is unchanged to the
    /// bit.
    #[test]
    fn an_eccentric_gears_shift_window_is_where_every_tooth_builds() {
        use crate::gear::Gear;

        // Concentric first: the window must be the single-gear one, exactly.
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                pressure_angle: 14.5,
                ..Default::default()
            },
        ] {
            let flat = GearParams {
                angular_shift: 0.0,
                ..p
            };
            let a = admissible_profile_shift(&flat, 1.0).bound;
            let b = admissible_profile_shift(&p, 1.0).bound;
            assert_eq!(a.min.unwrap().to_bits(), b.min.unwrap().to_bits());
            assert_eq!(a.max.unwrap().to_bits(), b.max.unwrap().to_bits());
        }

        let a_tooth_is_thin = |p: &GearParams, x: f64| {
            Gear::new(GearParams {
                profile_shift: x,
                ..*p
            })
            .distinct()
            .any(|g| {
                g.clamps.fired(key::CLAMP_TOOTH_THICKNESS_RAISED)
                    || g.clamps.fired(key::CLAMP_TOOTH_THICKNESS_CAPPED)
            })
        };

        for p in [
            GearParams {
                pressure_angle: 25.0,
                teeth: 23,
                addendum: 0.8,
                dedendum: 1.0,
                angular_shift: 1.0,
                index_offset: 1.0,
                ..Default::default()
            },
            GearParams {
                teeth: 24, // even: the far tooth sits exactly at 180°
                angular_shift: 0.6,
                ..Default::default()
            },
            GearParams {
                pressure_angle: 14.5,
                teeth: 31,
                angular_shift: 0.5,
                ..Default::default()
            },
        ] {
            let w = admissible_profile_shift(&p, p.dedendum).bound;
            let (lo, hi) = (w.min.unwrap(), w.max.unwrap());
            assert!(lo < hi, "empty window for {p:?}");
            let eps = 0.01;

            assert!(
                !a_tooth_is_thin(&p, lo + eps),
                "a tooth is thin inside the floor: {p:?}"
            );
            assert!(
                a_tooth_is_thin(&p, lo - eps),
                "no thin tooth below the floor: {p:?}"
            );
            assert!(
                !a_tooth_is_thin(&p, hi - eps),
                "a tooth is thin inside the ceiling: {p:?}"
            );
            assert!(
                a_tooth_is_thin(&p, hi + eps),
                "no thin tooth above the ceiling: {p:?}"
            );
        }
    }

    /// The reported eccentric case — a normal shift and a large amplitude —
    /// stays inside its window, and the fillet-radius ceiling drops to the
    /// **shared** cutter's tip round, which is what an eccentric gear is
    /// actually cut with.
    #[test]
    fn the_reported_eccentric_case_is_buildable_and_its_hints_are_the_shared_tool() {
        let p = GearParams {
            pressure_angle: 25.0,
            teeth: 23,
            profile_shift: 0.2,
            addendum: 0.8,
            dedendum: 1.0,
            angular_shift: 1.0,
            index_offset: 1.0,
            ..Default::default()
        };
        let r = admissible_ranges(&p, p.dedendum);
        assert!(
            r.profile_shift.bound.admits(p.profile_shift),
            "x̄ = 0.2 rejected: {:?}",
            r.profile_shift.bound
        );
        // The single-gear ceiling is ~0.48 module; the shared tool the high
        // teeth force is ~0.05.
        let ceiling = r.root_radius.max.unwrap();
        assert!(
            (0.03..0.07).contains(&ceiling),
            "root-radius ceiling {ceiling} is not the shared tool's"
        );
        let flat = admissible_ranges(
            &GearParams {
                angular_shift: 0.0,
                ..p
            },
            p.dedendum,
        );
        assert!(
            flat.root_radius.max.unwrap() > 0.4,
            "the concentric ceiling should be the roomy single-tooth one"
        );
    }

    /// The specification's fixed `|x| <= 2` is not a conservative version of the
    /// real bound — it is loose in some places and tight in others, which is
    /// exactly what a constant cannot fix.
    #[test]
    fn the_fixed_plus_minus_two_is_wrong_in_both_directions() {
        // Too loose above: the real ceiling is the tooth thickness, and the
        // depth arrives before it as a *threshold* — the cutter goes deeper than
        // the dedendum asked for from 1.20 up, and the tooth runs out of
        // thickness at 1.94. Both are below 2, so the constant is loose against
        // either; which of them is the limit is the two-tier question, and the
        // depth is not one because a deeper tool cuts the gear.
        let d = admissible_profile_shift(&GearParams::default(), 1.0);
        let dmax = d.bound.max.unwrap();
        assert!(dmax < 2.0, "max {dmax}");
        assert!(
            (d.shallow_cut - 1.20).abs() < 1e-9,
            "shallow cut {}",
            d.shallow_cut
        );
        assert!(
            d.shallow_cut < dmax,
            "the depth threshold must sit inside the range, not bound it"
        );

        // Too tight below at a low pressure angle...
        let low = admissible_profile_shift(
            &GearParams {
                pressure_angle: 14.5,
                ..Default::default()
            },
            1.0,
        );
        let lmin = low.bound.min.unwrap();
        assert!(lmin < -2.0, "14.5 deg floor {lmin} should be below -2");

        // ...and too loose below at a high one.
        let high = admissible_profile_shift(
            &GearParams {
                pressure_angle: 30.0,
                ..Default::default()
            },
            1.0,
        );
        let hmin = high.bound.min.unwrap();
        assert!(hmin > -2.0, "30 deg floor {hmin} should be above -2");

        // And blind to thickness modification entirely.
        let thick = admissible_profile_shift(
            &GearParams {
                thickness_mod: 1.3,
                ..Default::default()
            },
            1.0,
        );
        assert!(
            thick.bound.min.unwrap() < d.bound.min.unwrap() - 0.5,
            "k should move the floor"
        );
    }

    /// The design thresholds sit INSIDE the buildable range, not at its edges:
    /// an undercut gear is a real gear and this crate generates it exactly.
    #[test]
    fn the_advisory_thresholds_are_inside_the_range_not_bounds_on_it() {
        for teeth in [9_u32, 13, 17, 40] {
            let p = GearParams {
                teeth,
                ..Default::default()
            };
            let r = admissible_profile_shift(&p, 1.0);
            assert!(
                r.undercut > r.bound.min.unwrap() && r.undercut < r.bound.max.unwrap(),
                "z={teeth}"
            );
            // A real cutter always needs less shift than a sharp rack.
            assert!(r.undercut < r.sharp_rack_undercut);

            if let Some(pointed) = r.pointed {
                assert!(pointed > r.bound.min.unwrap() && pointed <= r.bound.max.unwrap());
                // Just past it the generator caps the tip, and says so.
                let capped = Tooth::new(GearParams {
                    profile_shift: (pointed + 0.02).min(r.bound.max.unwrap()),
                    ..p
                });
                assert!(
                    capped.clamps.fired(key::CLAMP_TIP_CAPPED_POINTED),
                    "z={teeth}: no pointed-tooth cap past {pointed}"
                );
            }
        }
    }

    /// Exclusivity is carried, not assumed. A module of exactly zero is not a
    /// gear; an addendum exactly at its floor is.
    #[test]
    fn bounds_carry_their_own_exclusivity() {
        let strict = Bound::strictly(0.0, 90.0);
        assert!(!strict.admits(0.0), "0 is outside (0, 90)");
        assert!(!strict.admits(90.0));
        assert!(strict.admits(0.001));
        assert!(strict.admits(20.0));
        assert!(!strict.admits(f64::NAN));

        let inclusive = Bound::between(Some(-1.25), None);
        assert!(inclusive.admits(-1.25), "the floor itself is legal");
        assert!(!inclusive.admits(-1.26));
        assert!(inclusive.admits(1e9), "no ceiling");

        // Exclusivity is carried in the fields, which is what a caller reads to
        // say *why* a value was refused. It used to be asserted by searching the
        // rejection sentence for "greater than" — a test on wording, of a
        // sentence nothing ever displayed.
        assert!(strict.exclusive_min && strict.exclusive_max);
        assert!(!inclusive.exclusive_min && !inclusive.exclusive_max);
    }

    /// Every input has a bound, and they come from one place.
    #[test]
    fn every_input_is_bounded_and_the_defaults_are_inside() {
        let p = GearParams::default();
        let r = admissible_ranges(&p, 1.0);

        assert!(r.module.admits(p.module));
        assert!(r.pressure_angle.admits(p.pressure_angle));
        assert!(r.teeth.admits(f64::from(p.teeth)));
        assert!(r.helix_angle.admits(p.helix_angle));
        assert!(r.thickness_mod.admits(p.thickness_mod));
        assert!(r.profile_shift.bound.admits(p.profile_shift));
        assert!(r.addendum.admits(p.addendum));
        assert!(r.dedendum.admits(p.dedendum));
        assert!(r.root_radius.admits(p.root_radius));

        // ...and the invariant ones reject what they must.
        assert!(!r.module.admits(0.0));
        assert!(!r.teeth.admits(0.0));
        assert!(!r.pressure_angle.admits(90.0));
        assert!(!r.helix_angle.admits(-90.0));
        assert!(!r.thickness_mod.admits(2.0));
    }

    #[test]
    fn an_unreachable_tip_width_is_refused_rather_than_approximated() {
        let g = Tooth::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        // Wider than the tooth is anywhere on its flank.
        assert!(addendum_for_tip_width(&g, 100.0).is_none());
    }

    /// **The buildable range is continuous in the eccentricity.**
    ///
    /// It was not. `admissible_profile_shift` branched on `off_hi == 0.0`, and
    /// the two arms disagreed about what happens when the shift outruns the
    /// dedendum: the concentric one refused, the eccentric one deepened the
    /// tool. So the ceiling jumped 1.200 → 1.942 at `Δx = 1e-14`, where nothing
    /// physical happens (`docs/corrections.md`).
    ///
    /// The gate is a **law rather than a tolerance**, which is what this project
    /// asks of a limit check: an amplitude of `Δx` can move a bound by at most
    /// the amount it moves the teeth, so the gap must fall *with* `Δx`. Halving
    /// the amplitude must at least halve the gap. A step does not do that at any
    /// amplitude, so no threshold has to be chosen and none is.
    #[test]
    fn the_shift_range_is_continuous_in_the_angular_shift() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                pressure_angle: 14.5,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                dedendum: 1.0,
                profile_shift: 0.3,
                ..Default::default()
            },
            GearParams {
                teeth: 23,
                pressure_angle: 25.0,
                addendum: 0.8,
                dedendum: 1.0,
                thickness_mod: 1.2,
                ..Default::default()
            },
        ] {
            let at = |amp: f64| {
                let r = admissible_profile_shift(
                    &GearParams {
                        angular_shift: amp,
                        ..p
                    },
                    1.0,
                );
                (r.bound.min.unwrap(), r.bound.max.unwrap())
            };
            let (lo0, hi0) = at(0.0);

            // The bound moves *linearly* with the amplitude — the teeth spread
            // by `Δx`, so the window closes by `Δx` — which means the honest
            // instrument is the ratio, not the gap. A continuous bound holds
            // `gap/Δx` bounded all the way down; a step sends it to infinity,
            // because the gap it leaves does not shrink at all. Nothing has to
            // be tolerated: the two behaviours differ by orders of magnitude at
            // the smallest amplitude tried.
            for step in 0..14 {
                let amp = 0.1 * 0.5_f64.powi(step);
                let (lo, hi) = at(amp);
                let gap = (lo - lo0).abs().max((hi - hi0).abs());
                assert!(
                    gap <= 2.0 * amp,
                    "{p:?}: the range steps rather than closing — it moves {gap:e} \
                     for an amplitude of {amp:e}, a ratio of {}",
                    gap / amp
                );
            }
        }
    }
    /// The contact ratio the chooser is not allowed to go below, shared with
    /// the grid it is checked against so the two agree on what is admissible.
    const MIN_RATIO: f64 = 1.2;

    /// How much of the grid's best the chooser is allowed to leave on the table.
    ///
    /// Not a fudge for a search that might be wrong: these optima lie **on the
    /// boundary** of the admissible set — pressed against the undercut floor,
    /// the contact ratio, or the root round — and a search that has to stay
    /// inside it approaches such a point rather than landing on it, the more so
    /// where the boundary is curved and the ridge along it is flat. A tenth of a
    /// thousandth of a point of efficiency is some four orders below what
    /// knowing the friction coefficient to ±10 % is worth, and two orders below
    /// the smallest gap this chooser exists to close. What the gate still
    /// catches is a search on the wrong ridge entirely, which is what every
    /// earlier version of it was doing.
    const SLACK: f64 = 1e-4;

    /// **The chooser finds what a sweep finds**, on the pairs the reference
    /// documents — including one whose pinion needs shift to exist at all.
    ///
    /// Checked against the efficiency itself over a grid, not against the
    /// search's own arithmetic: nothing on that grid may beat what it returns.
    #[test]
    fn the_shifts_it_chooses_are_the_ones_that_lose_least() {
        for (z1, z2) in [(9_u32, 37_u32), (17, 43), (13, 37)] {
            let base = |teeth: u32| GearParams {
                teeth,
                ..GearParams::default()
            };
            let (b1, b2) = (base(z1), base(z2));
            let floor = [
                automatic_profile_shift(&b1, b1.dedendum),
                automatic_profile_shift(&b2, b2.dedendum),
            ];
            let pair = |x: [f64; 2]| {
                [0, 1].map(|i| GearParams {
                    profile_shift: x[i],
                    ..if i == 0 { b1 } else { b2 }
                })
            };
            let chosen = shifts_for_efficiency(
                &pair,
                crate::mesh::MeshKind::External,
                &Bounds {
                    floor: floor.map(Some),
                    min_contact_ratio: MIN_RATIO,
                    clearance: 0.0,
                },
                &Pinned::default(),
                0.08,
                &Search::SHIPPED,
            )
            .expect("a pair that can be built");

            let eta_at = |x: [f64; 2]| {
                let g = |i: usize, teeth: u32| {
                    Tooth::new(GearParams {
                        teeth,
                        profile_shift: x[i],
                        ..GearParams::default()
                    })
                };
                let (a, b) = (g(0, z1), g(1, z2));
                if a.undercut || b.undercut || a.severed || b.severed {
                    return None;
                }
                // The same bound the chooser obeys: a fillet that no longer
                // fits is a tooth nobody can cut, whichever side finds it.
                if !root_radius_fits(&a.params, a.params.dedendum)
                    || !root_radius_fits(&b.params, b.params.dedendum)
                {
                    return None;
                }
                let mesh = crate::mesh::Mesh::new(&a, &b, crate::mesh::MeshKind::External).ok()?;
                let path = crate::contact::ContactPath::new(&a, b.ra, &mesh)?;
                (path.contact_ratio >= MIN_RATIO).then(|| {
                    crate::contact::efficiency(
                        &path,
                        &mesh,
                        &a,
                        0.08,
                        crate::contact::Drive::Forward,
                    )
                })
            };
            let here = eta_at(chosen).expect("the chosen pair builds");
            assert!(
                chosen[0] >= floor[0] - 1e-9 && chosen[1] >= floor[1] - 1e-9,
                "z{z1}/z{z2}: {chosen:?} is under the undercut floor {floor:?}"
            );
            for i in -10..=40 {
                for j in -25..=40 {
                    let x = [f64::from(i) * 0.05, f64::from(j) * 0.05];
                    if let Some(eta) = eta_at(x) {
                        assert!(
                            eta <= here + SLACK,
                            "z{z1}/z{z2}: {x:?} keeps {eta}, better than the chosen {chosen:?} \
                             at {here} by {:e}",
                            eta - here
                        );
                    }
                }
            }
        }
    }

    /// **What is pinned stays pinned**, and the freedom count says how much is
    /// left to choose.
    #[test]
    fn a_pinned_shift_is_the_shift_it_was_given() {
        let (b1, b2) = (
            GearParams {
                teeth: 17,
                ..GearParams::default()
            },
            GearParams {
                teeth: 43,
                ..GearParams::default()
            },
        );
        let floor = [0.0, 0.0];
        let pinned = Pinned {
            shift: [Some(0.3), None],
            sum: None,
        };
        assert_eq!(pinned.freedoms(), 1);
        let x = shifts_for_efficiency(
            &|x: [f64; 2]| {
                [0, 1].map(|i| GearParams {
                    profile_shift: x[i],
                    ..if i == 0 { b1 } else { b2 }
                })
            },
            crate::mesh::MeshKind::External,
            &Bounds {
                floor: floor.map(Some),
                min_contact_ratio: MIN_RATIO,
                clearance: 0.0,
            },
            &pinned,
            0.08,
            &Search::SHIPPED,
        )
        .unwrap();
        assert!((x[0] - 0.3).abs() < 1e-12, "the given shift moved: {x:?}");

        // ...and a given sum leaves only the division.
        let both = Pinned {
            shift: [Some(0.3), None],
            sum: Some(0.9),
        };
        assert_eq!(both.freedoms(), 0);
        let x = shifts_for_efficiency(
            &|x: [f64; 2]| {
                [0, 1].map(|i| GearParams {
                    profile_shift: x[i],
                    ..if i == 0 { b1 } else { b2 }
                })
            },
            crate::mesh::MeshKind::External,
            &Bounds {
                floor: floor.map(Some),
                min_contact_ratio: MIN_RATIO,
                clearance: 0.0,
            },
            &both,
            0.08,
            &Search::SHIPPED,
        )
        .unwrap();
        assert!(
            (x[0] - 0.3).abs() < 1e-12 && (x[1] - 0.6).abs() < 1e-12,
            "two pins determine the pair: {x:?}"
        );
    }
}
