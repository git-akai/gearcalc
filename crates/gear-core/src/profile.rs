//! Involute gear cross-section generation.
//!
//! A port of the validated `handoff_inbound/gear.py`, which was checked to
//! 5e-4 mm against a full simulation of the generating rack. Geometry is
//! produced the way a real gear is cut: an involute flank from the rack's
//! straight flank, and a trochoid fillet swept by the rack's rounded tip corner.
//!
//! One half-tooth, from the tooth tip centre outward to mid tooth-space:
//!
//! ```text
//! tip arc  ->  involute flank  ->  trochoid fillet  ->  root arc
//! ```
//!
//! `theta` throughout is the magnitude of the angle from the **tooth
//! centreline**: 0 at the tooth centre, `π/z` at mid tooth-space.
//!
//! # What must not be "simplified" out
//!
//! Two things here look like they could be tidier and must not be:
//!
//! - The flank continues **below the base circle** to its true intersection with
//!   the trochoid. Clamping it at the base circle and bridging the remainder —
//!   the obvious-looking approach — leaves a visible 0.3 mm step on undercut
//!   gears. [`Gear::with_legacy_clamp`] reproduces that fault deliberately, as a
//!   negative test fixture.
//! - The fillet fit cap is `ρ_max = w_tip·cos α_t / (2(1 − sin α_t))`. The
//!   plausible-looking `w_tip / (2 cos α_t)` is wrong and silently shrinks the
//!   fillet on every profile-shifted gear.

use crate::involute::{inv, inv_from_roll};
use crate::note::{key, Note};
use crate::params::{guard, Clamps, GearParams};
use crate::solve::{brent, newton_bracketed, Tol};

/// Bracket-expansion settings for the undercut junction search.
///
/// Search heuristics, not geometry: they only decide how the bracket is grown
/// before a guaranteed bracketed solve takes over. Any values that find a
/// bracket give the same root.
mod search {
    /// Growth factor when walking outward to find where the fillet crosses the
    /// base circle.
    pub const BASE_CROSS_GROWTH: f64 = 1.6;
    /// Growth factor when walking outward to find the flank/fillet crossing.
    pub const CROSSING_GROWTH: f64 = 1.4;
    /// Additive nudge so the walk escapes `s = 0`.
    pub const CROSSING_NUDGE: f64 = 1e-6;
    /// Maximum expansion steps before declaring no bracket exists.
    pub const MAX_STEPS: u32 = 200;
    /// Samples used to scan the fillet for a centreline crossing (severed tooth).
    pub const SEVER_SCAN_SAMPLES: usize = 2000;
    /// Samples per section when estimating arc length for point allocation.
    pub const LENGTH_SAMPLES: usize = 60;
    /// Minimum share of the total point budget any one section receives.
    pub const MIN_SECTION_SHARE: f64 = 0.004;
    /// Minimum points per section, whatever the budget.
    pub const MIN_SECTION_POINTS: usize = 3;
}

/// Which curve a half-profile section is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    TipArc,
    Involute,
    Trochoid,
    RootArc,
}

/// A generated gear cross-section.
///
/// Every field is in millimetres or radians. Construction never fails: degenerate
/// input is clamped and recorded in [`Gear::clamps`].
#[derive(Clone, Debug)]
pub struct Gear {
    pub params: GearParams,
    pub clamps: Clamps,

    /// Helix angle, radians.
    pub beta: f64,
    /// Normal pressure angle, radians (after the minimum-angle guard).
    pub alpha_n: f64,
    /// Transverse pressure angle, radians.
    pub alpha_t: f64,
    /// Transverse module.
    pub mt: f64,
    /// Pitch (reference) radius.
    pub r: f64,
    /// Base radius.
    pub rb: f64,

    /// Cutter tip depth below the rolling line.
    pub bd: f64,
    /// Root (minor) radius.
    pub rf: f64,
    /// Transverse tooth thickness at the pitch circle.
    pub st: f64,
    /// Half tooth angle at the pitch circle.
    pub psi_p: f64,
    /// Half tooth angle at the base circle.
    pub psi_b: f64,

    /// Cutter tip radius (transverse).
    pub rho: f64,
    /// Depth of the tip-round centre below the rolling line.
    pub bc: f64,
    /// Lateral offset of the tip-round centre.
    pub ac: f64,

    /// Tip (major) radius, after the pointed-tooth cap.
    pub ra: f64,
    /// Involute roll parameter at the tip. NaN if severed.
    pub u_tip: f64,
    /// Half angular width of the tip arc.
    pub theta_a: f64,

    /// Signed distance from the base tangent point to where the rack's straight
    /// flank runs out. **`l < 0` is exactly the undercut condition.**
    pub l: f64,
    pub undercut: bool,
    /// Roll parameter at the flank/fillet junction. NaN if severed.
    pub u_j: f64,
    /// Rack travel parameter at the flank/fillet junction.
    pub s_j: f64,
    /// Radius at the junction.
    pub r_j: f64,
    /// Angle where the fillet meets the root circle.
    pub theta0: f64,
    /// Half the angular pitch, `π/z`.
    pub half_pitch: f64,
    /// Tooth count used for the tooth **form**.
    ///
    /// Normally `params.teeth`. For a virtual spur gear built by
    /// [`Gear::virtual_spur`] it is the fractional ISO count `z / cos³β`, which
    /// is why it is an `f64` while `params.teeth` stays a whole number — a real
    /// gear has an integer tooth count, a virtual one need not.
    pub z: f64,
    /// True when undercut has removed the tooth entirely (DESIGN.md; the
    /// profile is truncated at the centreline so it stays a simple closed curve).
    pub severed: bool,
}

/// Spread `n` points across a profile's sections in proportion to their arc
/// length, so no section is starved.
///
/// Shared by [`Gear`] and [`crate::ring::Ring`]. The two have different section
/// *geometry* — one's tooth points outward and the other's inward — but this
/// distribution is the same arithmetic on both, and it was written twice: the
/// ring's copy carried its own anonymous `MIN_SHARE`, `MIN_POINTS` and
/// `LENGTH_SAMPLES` in a function body, so changing the named constants here
/// would silently have left the ring on the old values. The values agreed; the
/// arrangement was one edit away from not.
///
/// `sample` is asked for a section twice: once coarsely to measure its length,
/// once at the share it earns. Joints are de-duplicated, so consecutive sections
/// meet at a single point.
pub(crate) fn allocate_by_arc_length(
    sections: &[Section],
    n: usize,
    sample: impl Fn(Section, usize) -> Vec<(f64, f64)>,
) -> Vec<(f64, f64)> {
    // Arc length in polar coordinates: `√(dr² + (r̄ dθ)²)` along each step.
    let lengths: Vec<f64> = sections
        .iter()
        .map(|&s| {
            let pts = sample(s, search::LENGTH_SAMPLES);
            pts.windows(2)
                .map(|w| {
                    let dr = w[1].0 - w[0].0;
                    let dt = (w[1].0 + w[0].0) / 2.0 * (w[1].1 - w[0].1);
                    f64::hypot(dr, dt)
                })
                .sum()
        })
        .collect();
    // A section of zero or non-finite length is not a section, and must not
    // reach the shares: one `NaN` length makes `total` `NaN`, every share
    // `NaN`, and `(NaN) as usize` **zero** — so every section silently falls
    // back to `MIN_SECTION_POINTS` and a 600-point request returns seven. That
    // is how a ring with no fillet came to draw as a straight-sided polygon
    // (§12). Dropping the section is the honest reading: it earns no points
    // because it has no length.
    let live: Vec<(Section, f64)> = sections
        .iter()
        .zip(&lengths)
        .filter(|(_, &w)| w.is_finite() && w > 0.0)
        .map(|(&s, &w)| (s, w))
        .collect();
    if live.is_empty() {
        return Vec::new();
    }

    let total: f64 = live.iter().map(|&(_, w)| w).sum();
    let shares: Vec<f64> = live
        .iter()
        .map(|&(_, w)| w.max(total * search::MIN_SECTION_SHARE))
        .collect();
    let share_total: f64 = shares.iter().sum();

    let mut out: Vec<(f64, f64)> = Vec::new();
    for (&(section, _), share) in live.iter().zip(&shares) {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let count = ((share / share_total) * n as f64) as usize;
        let pts = sample(section, count.max(search::MIN_SECTION_POINTS));
        // Consecutive sections share their joint; keep one copy of it.
        let skip = usize::from(!out.is_empty());
        out.extend_from_slice(&pts[skip..]);
    }
    out
}

impl Gear {
    #[must_use]
    pub fn new(params: GearParams) -> Self {
        Self::build(params, false)
    }

    /// Reproduces the pre-fix behaviour: the flank clamped at the base circle,
    /// leaving a step where the fillet should meet it.
    ///
    /// Retained **only** so the test suite can demonstrate it still detects that
    /// fault. Never use it to generate real geometry.
    #[must_use]
    pub fn with_legacy_clamp(params: GearParams) -> Self {
        Self::build(params, true)
    }

    /// The **virtual spur gear**: this gear's tooth form seen in the normal
    /// plane.
    ///
    /// A helical tooth bends as its *normal* section, not its transverse one.
    /// ISO 6336-3 handles that by rating an equivalent spur gear with
    ///
    /// ```text
    /// z_n = z / cos³β        module m_n, pressure angle α_n, same shift
    /// ```
    ///
    /// where one power of `cos β` comes from the section being taken obliquely
    /// and two from the radius of curvature of the pitch ellipse that section
    /// cuts.
    ///
    /// The count is fractional, which is exactly why [`Gear::z`] exists. Nothing
    /// else in the construction cares: the tooth *form* is a continuous function
    /// of `z`. What is **not** meaningful on the result is [`Gear::profile`],
    /// which replicates a whole number of teeth around a real gear — this object
    /// exists to be measured, not drawn.
    ///
    /// # Spur gears are not a special case
    ///
    /// At `β = 0` this is the identity, and **by construction rather than by a
    /// branch**: `cos 0` is exactly 1 so `z_n = z`, and `helix_angle` is already
    /// zero so the parameters are unchanged. The rebuild therefore reproduces
    /// the same gear bit for bit. Note that `GearParams::pressure_angle` is
    /// *already* the normal pressure angle, so it is carried across untouched —
    /// passing `self.alpha_n.to_degrees()` instead would round-trip through
    /// degrees and cost the exactness for nothing.
    #[must_use]
    pub fn virtual_spur(&self) -> Self {
        let beta = self.params.helix_angle.to_radians();
        let params = GearParams {
            helix_angle: 0.0,
            ..self.params
        };
        Self::build_with_z(params, false, self.z / beta.cos().powi(3))
    }

    fn build(params: GearParams, legacy_clamp: bool) -> Self {
        let z = f64::from(params.teeth);
        Self::build_with_z(params, legacy_clamp, z)
    }

    #[allow(clippy::too_many_lines)]
    fn build_with_z(params: GearParams, legacy_clamp: bool, z: f64) -> Self {
        let mut clamps = Clamps::default();
        let m = params.module;
        let x = params.profile_shift;

        // ---- pressure angle, guarded -----------------------------------
        let mut an = params.pressure_angle.to_radians();
        if an <= guard::MIN_PRESSURE_ANGLE_DEG.to_radians() {
            an = guard::MIN_PRESSURE_ANGLE_DEG.to_radians();
            clamps.push(Note::new(key::CLAMP_PRESSURE_ANGLE_RAISED).number(
                "degrees",
                guard::MIN_PRESSURE_ANGLE_DEG,
                1,
            ));
        }
        let beta = params.helix_angle.to_radians();

        // ---- normal -> transverse --------------------------------------
        let mt = m / beta.cos();
        let alpha_t = (an.tan() / beta.cos()).atan();
        let r = mt * z / 2.0;
        let rb = r * alpha_t.cos();
        let (ca, sa) = (alpha_t.cos(), alpha_t.sin());

        // ---- depth: RADIAL, so plain x ---------------------------------
        let mut bd = m * (params.dedendum - x);
        if bd < guard::MIN_CUTTER_DEPTH_MODULES * m {
            bd = guard::MIN_CUTTER_DEPTH_MODULES * m;
            clamps.push(Note::new(key::CLAMP_DEDENDUM_RAISED));
        }
        if bd > guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r {
            bd = guard::MAX_CUTTER_DEPTH_FRACTION_OF_R * r;
            clamps.push(Note::new(key::CLAMP_DEDENDUM_CAPPED));
        }
        let rf = r - bd;

        // ---- thickness: uses x + x_s (the thickness modification) -------
        let x_thick = x + params.thickness_shift();
        let mut st = m * (std::f64::consts::PI / 2.0 + 2.0 * x_thick * an.tan()) / beta.cos();
        let st_max =
            guard::MAX_TOOTH_THICKNESS_FRACTION_OF_PITCH * 2.0 * r * std::f64::consts::PI / z;
        if st <= guard::MIN_TOOTH_THICKNESS_MODULES * m {
            st = guard::MIN_TOOTH_THICKNESS_MODULES * m;
            clamps.push(Note::new(key::CLAMP_TOOTH_THICKNESS_RAISED));
        }
        if st > st_max {
            st = st_max;
            clamps.push(Note::new(key::CLAMP_TOOTH_THICKNESS_CAPPED));
        }
        let psi_p = st / (2.0 * r);
        let psi_b = psi_p + inv(alpha_t);

        // ---- cutter tip radius, capped so the rounds fit the tooth space
        let w_roll = std::f64::consts::PI * mt - st; // rack tooth width at the rolling line
        let w_tip = w_roll - 2.0 * bd * alpha_t.tan(); // ... and at the tip line
                                                       // NOT w_tip / (2 cos a): that form is wrong and silently shrinks the
                                                       // fillet on every profile-shifted gear.
        let rho_fit = if w_tip > 0.0 {
            w_tip * ca / (2.0 * (1.0 - sa))
        } else {
            0.0
        };
        let mut rho = params.root_radius * m / beta.cos();
        let rho_cap =
            (guard::FILLET_FRACTION_OF_MAX * bd).min(guard::FILLET_FRACTION_OF_MAX * rho_fit);
        if rho > rho_cap {
            rho = rho_cap.max(guard::MIN_FILLET_MODULES * m);
            clamps.push(Note::new(key::CLAMP_FILLET_CAPPED).number("radius", rho, 4));
        }
        let rho = rho.max(guard::MIN_FILLET_MODULES * m);
        let bc = bd - rho;
        let ac = st / 2.0 + bc * alpha_t.tan() + rho / ca;

        // ---- tip radius: RADIAL, so plain x; capped at the pointed tooth
        let mut ra = r + m * (params.addendum + x);
        // Pointed tooth: psi_b = inv_from_roll(u). Monotone in u, so bracketed.
        let u_point = newton_bracketed(
            |u| psi_b - inv_from_roll(u),
            |u| -(u * u) / (1.0 + u * u), // d/du [u - atan u] = u^2/(1+u^2)
            0.0,
            POINTED_TOOTH_MAX_ROLL,
            (3.0 * psi_b).cbrt(),
            Tol::default(),
        );
        if let Some(u_point) = u_point {
            let ra_point = rb * f64::hypot(1.0, u_point);
            if ra > ra_point {
                ra = ra_point;
                clamps.push(Note::new(key::CLAMP_TIP_CAPPED_POINTED).number("radius", ra, 4));
            }
        }
        let ra = ra.max(rb * (1.0 + guard::TIP_ABOVE_BASE_FRACTION));
        let u_tip = (((ra / rb).powi(2) - 1.0).max(0.0)).sqrt();

        // ---- flank / fillet junction ------------------------------------
        let l = r * sa - bc / sa - rho;
        let undercut = l < 0.0;

        let mut g = Self {
            params,
            clamps,
            beta,
            alpha_n: an,
            alpha_t,
            mt,
            r,
            rb,
            bd,
            rf,
            st,
            psi_p,
            psi_b,
            rho,
            bc,
            ac,
            ra,
            u_tip,
            theta_a: psi_b - inv_from_roll(u_tip),
            l,
            undercut,
            u_j: 0.0,
            s_j: 0.0,
            r_j: 0.0,
            theta0: ac / r,
            half_pitch: std::f64::consts::PI / z,
            z,
            severed: false,
        };

        let (u_j, s_j) = g.solve_junction();
        g.u_j = u_j;
        g.s_j = s_j;
        if legacy_clamp {
            g.u_j = l.max(0.0) / rb;
            g.s_j = -bc / alpha_t.tan();
        }
        g.r_j = rb * f64::hypot(1.0, g.u_j);
        g.check_severed();
        g
    }

    // ---------------------------------------------------------------- //
    //  primitive curves: (radius, |angle from tooth centre|)
    // ---------------------------------------------------------------- //

    /// The involute flank at roll parameter `u`.
    #[must_use]
    pub fn involute_at(&self, u: f64) -> (f64, f64) {
        (self.rb * f64::hypot(1.0, u), self.psi_b - inv_from_roll(u))
    }

    /// The trochoid fillet at rack travel `s`. `s = 0` puts the cutter corner at
    /// the root.
    #[must_use]
    pub fn trochoid_at(&self, s: f64) -> (f64, f64) {
        let d = f64::hypot(s, self.bc);
        let k = 1.0 + self.rho / d;
        let (xf, yf) = (k * s, self.r - k * self.bc);
        (f64::hypot(xf, yf), xf.atan2(yf) - (s - self.ac) / self.r)
    }

    // ---------------------------------------------------------------- //

    /// Where the involute flank meets the trochoid fillet.
    ///
    /// **Not undercut** (`l >= 0`): the rack's straight flank ends exactly where
    /// its tip round begins, so the curves meet tangentially at a point available
    /// in closed form. No iteration.
    ///
    /// **Undercut** (`l < 0`): the round has eaten past the flank's limit and the
    /// two curves genuinely *cross*. Solved for, not assumed — clamping here is
    /// what used to leave a step in the profile. The crossing is a real corner:
    /// that is the undercut notch, and it is correct geometry.
    fn solve_junction(&self) -> (f64, f64) {
        let s_tan = -self.bc / self.alpha_t.tan();
        if !self.undercut {
            return (self.l / self.rb, s_tan);
        }

        let r_of = |s: f64| self.trochoid_at(s).0;

        // Walk outward until the fillet has climbed past the base circle.
        let mut s_lo = s_tan.min(-f64::MIN_POSITIVE);
        let mut found = false;
        for _ in 0..search::MAX_STEPS {
            if r_of(s_lo) > self.rb {
                found = true;
                break;
            }
            s_lo *= search::BASE_CROSS_GROWTH;
        }
        if !found {
            return (0.0, s_tan); // fillet never reaches the base circle
        }
        let Some(s_b) = brent(|s| r_of(s) - self.rb, s_lo, 0.0, Tol::default()) else {
            return (0.0, s_tan);
        };

        // Angular gap between fillet and the extended involute at the same radius.
        let gap = |s: f64| {
            let (r, th) = self.trochoid_at(s);
            let u = (((r / self.rb).powi(2) - 1.0).max(0.0)).sqrt();
            th - (self.psi_b - inv_from_roll(u))
        };

        // gap < 0 at the base circle; march out for a sign change.
        let mut s_far = s_b;
        let mut crossed = false;
        for _ in 0..search::MAX_STEPS {
            s_far = s_far * search::CROSSING_GROWTH - search::CROSSING_NUDGE;
            if gap(s_far) > 0.0 {
                crossed = true;
                break;
            }
        }
        if !crossed {
            return (0.0, s_b);
        }

        let Some(s_j) = brent(gap, s_far, s_b, Tol::default()) else {
            return (0.0, s_b);
        };
        let r_j = self.trochoid_at(s_j).0;
        ((((r_j / self.rb).powi(2) - 1.0).max(0.0)).sqrt(), s_j)
    }

    /// Detect a tooth cut away entirely by undercut.
    ///
    /// If the fillet reaches the tooth centreline, the two fillets bounding one
    /// tooth have overlapped: the cutter removed the whole tooth and anything
    /// beyond is detached. The profile is truncated at the centreline so it stays
    /// a valid simple closed curve, and the condition is reported rather than
    /// silently producing a self-intersecting outline.
    ///
    /// Any code touching the flank must check [`Gear::severed`] first — `u_j`
    /// and `u_tip` are NaN in this state and there are only two sections.
    fn check_severed(&mut self) {
        let n = search::SEVER_SCAN_SAMPLES;
        let mut min_th = f64::INFINITY;
        let mut min_i = 0usize;
        for i in 0..n {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / (n - 1) as f64;
            let s = self.s_j + t * (0.0 - self.s_j);
            let th = self.trochoid_at(s).1;
            if th < min_th {
                min_th = th;
                min_i = i;
            }
        }
        if min_th >= 0.0 || min_i >= n - 1 {
            return;
        }
        #[allow(clippy::cast_precision_loss)]
        let s_at = |i: usize| self.s_j + (i as f64 / (n - 1) as f64) * (0.0 - self.s_j);
        let Some(s_c) = brent(|s| self.trochoid_at(s).1, s_at(min_i), 0.0, Tol::default()) else {
            return;
        };
        self.severed = true;
        self.s_j = s_c;
        self.u_j = f64::NAN;
        self.ra = self.trochoid_at(s_c).0;
        self.r_j = self.ra;
        self.theta_a = 0.0;
        self.u_tip = f64::NAN;
        self.clamps.push(Note::new(key::CLAMP_TOOTH_SEVERED));
    }

    // ---------------------------------------------------------------- //

    /// The half-profile sections, ordered tip -> mid tooth-space.
    #[must_use]
    pub fn sections(&self) -> Vec<Section> {
        if self.severed {
            vec![Section::Trochoid, Section::RootArc]
        } else {
            vec![
                Section::TipArc,
                Section::Involute,
                Section::Trochoid,
                Section::RootArc,
            ]
        }
    }

    fn sample_section(&self, section: Section, n: usize) -> Vec<(f64, f64)> {
        let n = n.max(2);
        #[allow(clippy::cast_precision_loss)]
        let lerp = |a: f64, b: f64, i: usize| a + (b - a) * (i as f64 / (n - 1) as f64);
        (0..n)
            .map(|i| match section {
                Section::TipArc => (self.ra, lerp(0.0, self.theta_a.max(0.0), i)),
                Section::Involute => self.involute_at(lerp(self.u_tip, self.u_j, i)),
                Section::Trochoid => self.trochoid_at(lerp(self.s_j, 0.0, i)),
                Section::RootArc => (self.rf, lerp(self.theta0, self.half_pitch, i)),
            })
            .collect()
    }

    /// `(radius, theta)` from the tooth tip centre to mid tooth-space, spaced by
    /// arc length so no section is starved of points.
    #[must_use]
    pub fn half_profile(&self, n: usize) -> (Vec<f64>, Vec<f64>) {
        allocate_by_arc_length(&self.sections(), n, |s, k| self.sample_section(s, k))
            .into_iter()
            .unzip()
    }

    /// The closed cross-section as `[x, y]` points, counter-clockwise, with the
    /// first tooth centred on +X.
    #[must_use]
    pub fn profile(&self, per_tooth: usize) -> Vec<[f64; 2]> {
        crate::eccentric::Eccentric::new(self.params).outline(per_tooth)
    }
}

/// Upper bracket for the pointed-tooth roll parameter.
///
/// `u = tan α_r`, so this corresponds to a roll angle of about 88.9°. A tooth
/// whose flanks converge only that far out is long past pointed; the bound is a
/// bracket end, not a design limit.
pub(crate) const POINTED_TOOTH_MAX_ROLL: f64 = 50.0;
