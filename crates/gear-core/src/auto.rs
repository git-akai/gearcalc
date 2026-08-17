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
use crate::profile::{Gear, POINTED_TOOTH_MAX_ROLL};
use crate::solve::{brent, Tol};

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
    let alpha_t = (p.pressure_angle.to_radians().tan() / beta.cos()).atan();
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
    let alpha_t = (an.tan() / beta.cos()).atan();
    let mt = p.module / beta.cos();
    let r = mt * f64::from(p.teeth) / 2.0;
    let xs = p.thickness_shift();
    let ta = an.tan();

    // Tooth thickness, both ends. `x_s` shifts the whole interval.
    let lo_t = (guard::MIN_TOOTH_THICKNESS_MODULES * beta.cos() - PI / 2.0) / (2.0 * ta) - xs;
    let hi_t = (guard::MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH * PI - PI / 2.0) / (2.0 * ta) - xs;

    // Cutter depth, both ends.
    let hi_d = p.dedendum - guard::MIN_CUTTER_DEPTH_MODULES;
    let lo_d = p.dedendum - guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r / p.module;

    let min = lo_t.max(lo_d);
    let max = hi_t.min(hi_d);

    let shift = minimum_profile_shift(p, working_depth);

    // Where the tooth comes to a point at the requested addendum. Monotone in
    // practice but not guaranteed, so it is bracketed on the admissible range
    // and simply absent if the tooth never points within it.
    let rb = r * alpha_t.cos();
    let half_tip_angle = |x: f64| {
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
        undercut: shift.with_cutter_radius,
        sharp_rack_undercut: shift.sharp_rack,
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

    /// Why `v` is outside, or `None` if it is inside.
    ///
    /// The message is produced here rather than in the view so that the two
    /// panels cannot word — or bound — the same condition differently.
    #[must_use]
    pub fn rejects(&self, v: f64) -> Option<String> {
        if !v.is_finite() {
            return Some("must be a number".into());
        }
        if let Some(lo) = self.min {
            if if self.exclusive_min { v <= lo } else { v < lo } {
                let how = if self.exclusive_min {
                    "greater than"
                } else {
                    "at least"
                };
                return Some(format!("must be {how} {lo}"));
            }
        }
        if let Some(hi) = self.max {
            if if self.exclusive_max { v >= hi } else { v > hi } {
                let how = if self.exclusive_max {
                    "less than"
                } else {
                    "at most"
                };
                return Some(format!("must be {how} {hi}"));
            }
        }
        None
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
#[must_use]
pub fn admissible_ranges(p: &GearParams, working_depth: f64) -> Ranges {
    use crate::params::guard;
    use std::f64::consts::PI;

    let beta = p.helix_angle.to_radians();
    let an = p
        .pressure_angle
        .to_radians()
        .max(guard::MIN_PRESSURE_ANGLE_DEG.to_radians());
    let alpha_t = (an.tan() / beta.cos()).atan();
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
pub fn addendum_for_tip_width(g: &Gear, min_tip_width: f64) -> Option<f64> {
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

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

            let just_under = Gear::new(GearParams {
                profile_shift: x - 1e-4,
                ..p
            });
            let just_over = Gear::new(GearParams {
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
                let g = Gear::new(base);
                let h_a = addendum_for_tip_width(&g, want).unwrap();

                let sized = Gear::new(GearParams {
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
        let g = Gear::new(base);
        let h_a = addendum_for_tip_width(&g, 0.0).unwrap();
        let pointed = Gear::new(GearParams {
            addendum: h_a,
            ..base
        });
        assert!(
            pointed.theta_a.abs() < 1e-9,
            "tip arc should have closed up"
        );

        // ...and asking for more addendum than that is refused by the generator's
        // own cap, so the two agree on where "pointed" is.
        let past = Gear::new(GearParams {
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
                Gear::new(GearParams {
                    profile_shift: x,
                    ..p
                })
                .clamps
                .notes
                .iter()
                .any(|n| n.contains("cutter depth") || n.contains("tooth thickness"))
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

    /// The specification's fixed `|x| <= 2` is not a conservative version of the
    /// real bound — it is loose in some places and tight in others, which is
    /// exactly what a constant cannot fix.
    #[test]
    fn the_fixed_plus_minus_two_is_wrong_in_both_directions() {
        // Too loose above: the real ceiling is the cutter depth.
        let d = admissible_profile_shift(&GearParams::default(), 1.0);
        let dmax = d.bound.max.unwrap();
        assert!(dmax < 2.0 && (dmax - 1.20).abs() < 1e-9, "max {dmax}");

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
                let capped = Gear::new(GearParams {
                    profile_shift: (pointed + 0.02).min(r.bound.max.unwrap()),
                    ..p
                });
                assert!(
                    capped.clamps.notes.iter().any(|n| n.contains("pointed")),
                    "z={teeth}: no pointed-tooth cap past {pointed}"
                );
            }
        }
    }

    /// Exclusivity is carried, not assumed. A module of exactly zero is not a
    /// gear; an addendum exactly at its floor is.
    #[test]
    fn bounds_carry_their_own_exclusivity_and_wording() {
        let strict = Bound::strictly(0.0, 90.0);
        assert!(strict.rejects(0.0).is_some(), "0 is outside (0, 90)");
        assert!(strict.rejects(90.0).is_some());
        assert!(strict.rejects(0.001).is_none());
        assert!(strict.rejects(20.0).is_none());
        assert!(strict.rejects(f64::NAN).is_some());

        let inclusive = Bound::between(Some(-1.25), None);
        assert!(
            inclusive.rejects(-1.25).is_none(),
            "the floor itself is legal"
        );
        assert!(inclusive.rejects(-1.26).is_some());
        assert!(inclusive.rejects(1e9).is_none(), "no ceiling");

        // The wording distinguishes the two, since it is the only thing the user
        // sees and both panels take it from here.
        assert!(strict.rejects(0.0).unwrap().contains("greater than"));
        assert!(inclusive.rejects(-2.0).unwrap().contains("at least"));
    }

    /// Every input has a bound, and they come from one place.
    #[test]
    fn every_input_is_bounded_and_the_defaults_are_inside() {
        let p = GearParams::default();
        let r = admissible_ranges(&p, 1.0);

        assert!(r.module.rejects(p.module).is_none());
        assert!(r.pressure_angle.rejects(p.pressure_angle).is_none());
        assert!(r.teeth.rejects(f64::from(p.teeth)).is_none());
        assert!(r.helix_angle.rejects(p.helix_angle).is_none());
        assert!(r.thickness_mod.rejects(p.thickness_mod).is_none());
        assert!(r.profile_shift.bound.rejects(p.profile_shift).is_none());
        assert!(r.addendum.rejects(p.addendum).is_none());
        assert!(r.dedendum.rejects(p.dedendum).is_none());
        assert!(r.root_radius.rejects(p.root_radius).is_none());

        // ...and the invariant ones reject what they must.
        assert!(r.module.rejects(0.0).is_some());
        assert!(r.teeth.rejects(0.0).is_some());
        assert!(r.pressure_angle.rejects(90.0).is_some());
        assert!(r.helix_angle.rejects(-90.0).is_some());
        assert!(r.thickness_mod.rejects(2.0).is_some());
    }

    #[test]
    fn an_unreachable_tip_width_is_refused_rather_than_approximated() {
        let g = Gear::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        // Wider than the tooth is anywhere on its flank.
        assert!(addendum_for_tip_width(&g, 100.0).is_none());
    }
}
