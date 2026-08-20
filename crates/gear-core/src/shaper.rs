//! Generation by a **pinion-shaped cutter**, of which the rack is the limit.
//!
//! An internal gear cannot be cut by a rack — the tool has to be a pinion, run
//! in mesh with the workpiece — and the fillet it leaves is swept by a corner
//! going round a circle rather than along a line. That is the one genuinely new
//! curve milestone 8 needs.
//!
//! It is new, but it is not *separate*. A rack is a shaper with infinitely many
//! teeth, so this module is the general construction and [`crate::profile`]'s
//! trochoid is its limit — measured, not asserted: the tests here drive the
//! cutter's tooth count up and watch the difference fall away as `1/z_c`.
//!
//! ```text
//! external gear, rack cutter     z_c → ∞      profile::Gear::trochoid_at
//! external gear, shaper          z_c finite   here, MeshKind::External
//! internal gear, shaper          z_c finite   here, MeshKind::Internal
//! ```
//!
//! # Why the fillet point is where it is
//!
//! Both cases use the same one-line construction, and it is worth stating
//! because it looks like a coincidence and is not. The cutter's tip corner is a
//! circle of radius `ρ` about a centre `C` that the rolling carries around. The
//! fillet is the envelope of that circle, so the fillet point lies on the common
//! normal — and **the common normal of a rolling pair passes through the pitch
//! point**. So the fillet point is simply `C` pushed a further `ρ` along the line
//! from the pitch point `P`:
//!
//! ```text
//! F = P + (1 + ρ/|C − P|)(C − P)
//! ```
//!
//! `profile::Gear::trochoid_at` is that expression with `C − P = (s, −b_c)`, the
//! rack's corner centre. Here `C` goes round a circle instead. Nothing else
//! differs.
//!
//! # One sign, not two cases
//!
//! `σ = +1` for an external workpiece and `−1` for an internal one, and it
//! appears in exactly three places: the centre distance `a = r + σ r_c`, the
//! side of the cutter its tip points from, and the sense the workpiece turns.
//! An internal gear's root is *outside* its pitch circle and its material is
//! *outside* its teeth, which is the whole of what `σ` says.

use crate::involute::inv;
use crate::mesh::MeshKind;
use crate::profile::Gear;

/// A workpiece being cut by a pinion-shaped cutter.
///
/// Radii are transverse and in mm; the workpiece frame matches
/// [`crate::profile::Gear`]'s, with a tooth centred on `+y` and angles measured
/// from it.
#[derive(Clone, Copy, Debug)]
pub struct ShaperCut {
    /// The workpiece's pitch radius.
    pub workpiece_radius: f64,
    /// The cutter's pitch radius. A rack is this going to infinity.
    pub cutter_radius: f64,
    /// Radius of the cutter's tip-round **centres**, from the cutter's axis.
    pub corner_radius: f64,
    /// The cutter's tip corner round.
    pub tip_round: f64,
    /// The cutter's own tooth thickness at its reference pitch circle, mm.
    ///
    /// Kept so that anything reconstructing the tool — the cut simulation above
    /// all — can take it from the tool rather than inferring it from the
    /// workpiece. Inferring it is what made that simulation unable to see a
    /// misplaced cutter: it derived the same wrong tooth the model did, agreed
    /// with itself to 2.7 µm, and said nothing. §12.
    pub cutter_tooth: f64,
    /// Operating pressure angle of the cut, radians.
    ///
    /// The reference angle when the cut is at reference centres; larger when the
    /// tool sits further out. Every base-tangent length the ring's junction and
    /// generation limit are built from is `a sin α_w`, so this is the angle those
    /// relations need rather than `α_t`.
    pub alpha_w: f64,
    /// Centre distance the cut actually happens at, mm.
    ///
    /// Equal to `r_w + σ r_c` only when neither member is shifted. A shifted
    /// workpiece is cut by the same tool placed further out or further in, and
    /// that displacement is the whole of what its shift *is* for a shaper — see
    /// [`crate::mesh::operating_geometry`].
    pub centre_distance: f64,
    /// The pitch radii the pair actually rolls on, mm.
    ///
    /// Rolling happens on the **operating** circles, whose radii are the
    /// reference ones scaled by `a / a_ref`; the two coincide exactly when the
    /// cut is at reference centres. A rack is the one case where a shift does
    /// *not* move them, because a rack's pitch line is a machine setting rather
    /// than a consequence of two tooth counts.
    pub workpiece_operating_radius: f64,
    pub cutter_operating_radius: f64,
    /// Lateral phase of the corner, in the same units and meaning as
    /// [`crate::profile::Gear::ac`] — arc length along the pitch circle.
    pub phase: f64,
    /// External or internal workpiece.
    pub kind: MeshKind,
}

/// Everything needed to describe one cut.
#[derive(Clone, Copy, Debug)]
pub struct CutParams {
    /// Transverse module, mm — shared, since the pair must roll together.
    pub module_t: f64,
    /// Transverse pressure angle, radians. Shared.
    pub alpha_t: f64,
    /// The workpiece's pitch radius, mm.
    pub workpiece_radius: f64,
    /// The workpiece's tooth thickness at its reference pitch circle, as an arc,
    /// mm.
    pub workpiece_tooth: f64,
    /// The **cutter's own** tooth thickness at its reference pitch circle, mm.
    ///
    /// Taken rather than derived as `π m_t − workpiece_tooth`, which is only the
    /// cutter that would complement an *unshifted* workpiece at reference
    /// centres. Deriving it meant a shifted workpiece was described as having
    /// been cut by a thicker tool than exists, and — because the cut simulation
    /// derived the cutter the same way — nothing could see it. §12.
    pub cutter_tooth: f64,
    /// Centre distance the cut happens at, mm.
    pub centre_distance: f64,
    /// The cutter's pitch radius, mm.
    ///
    /// Taken rather than derived from a tooth count so a **virtual** cutter can
    /// be described: rating a helical ring means working on its virtual spur
    /// section, where both members carry `z / cos³β` teeth and neither count is a
    /// whole number.
    pub cutter_radius: f64,
    /// The cutter's tip radius, mm.
    pub cutter_tip_radius: f64,
    /// The cutter's tip corner round, mm.
    pub tip_round: f64,
    pub kind: MeshKind,
}

impl ShaperCut {
    /// Build a cut from the cutter and workpiece that make it.
    ///
    /// # Errors
    ///
    /// `None` if the cutter has no teeth, or its corner falls inside its own
    /// base circle — which is not a cutter, it is a disc.
    #[must_use]
    pub fn new(p: &CutParams) -> Option<Self> {
        if !p.cutter_radius.is_finite() || p.cutter_radius <= 0.0 || !p.module_t.is_finite() {
            return None;
        }
        let cutter_radius = p.cutter_radius;
        let corner_radius = p.cutter_tip_radius - p.tip_round;
        if corner_radius.is_nan() || corner_radius <= 0.0 {
            return None;
        }
        let angle = Self::corner_angle(
            cutter_radius,
            corner_radius,
            p.cutter_tooth,
            p.alpha_t,
            p.tip_round,
        )?;
        // Rolling is on the operating circles, and both are the reference radii
        // scaled by the same `a / a_ref` — so a shift enters the kinematics as
        // one factor rather than as a second construction. The phase is an arc
        // on those circles, so it scales with them.
        let sigma = match p.kind {
            MeshKind::External => 1.0,
            MeshKind::Internal => -1.0,
        };
        let a_ref = p.workpiece_radius + sigma * cutter_radius;
        if !a_ref.is_finite() || a_ref == 0.0 || !p.centre_distance.is_finite() {
            return None;
        }
        let scale = p.centre_distance / a_ref;
        if scale <= 0.0 {
            return None;
        }
        // r_b1 + r_b2 = a cos α_w is what fixes the operating angle, and the
        // base radii do not move: cos α_w = a_ref cos α_t / a.
        let cos_aw = p.alpha_t.cos() / scale;
        if !(-1.0..=1.0).contains(&cos_aw) {
            return None;
        }
        Some(Self {
            workpiece_radius: p.workpiece_radius,
            cutter_radius,
            cutter_tooth: p.cutter_tooth,
            alpha_w: cos_aw.acos(),
            centre_distance: p.centre_distance,
            workpiece_operating_radius: p.workpiece_radius * scale,
            cutter_operating_radius: cutter_radius * scale,
            corner_radius,
            tip_round: p.tip_round,
            phase: scale * Self::phase_from(p.module_t, cutter_radius, angle),
            kind: p.kind,
        })
    }
}

impl ShaperCut {
    /// `+1` for an external workpiece, `−1` for an internal one.
    fn sigma(&self) -> f64 {
        match self.kind {
            MeshKind::External => 1.0,
            MeshKind::Internal => -1.0,
        }
    }

    /// Reference centre distance, mm — where the cut would sit with neither
    /// member shifted.
    #[must_use]
    pub fn reference_centre_distance(&self) -> f64 {
        self.workpiece_radius + self.sigma() * self.cutter_radius
    }

    /// The cutter's tip-round centre, in the fixed frame, at pitch-line travel
    /// `s`.
    ///
    /// `s` is arc length rolled, so it means exactly what the rack's `s` means
    /// and the two can be compared directly. The cutter has turned
    /// `s / r_c` at that point.
    #[must_use]
    pub fn corner_centre_at(&self, s: f64) -> (f64, f64) {
        let sigma = self.sigma();
        let (sin_phi, cos_phi) = (s / self.cutter_operating_radius).sin_cos();
        (
            self.corner_radius * sin_phi,
            self.centre_distance - sigma * self.corner_radius * cos_phi,
        )
    }

    /// The trochoid fillet at pitch-line travel `s`, as `(radius, angle from the
    /// tooth centreline)` — the same pair [`Gear::trochoid_at`] returns.
    #[must_use]
    pub fn trochoid_at(&self, s: f64) -> (f64, f64) {
        // The pitch point sits on the OPERATING circle, and the workpiece turns
        // by arc over that same radius. Identical to the reference radius
        // whenever the cut is at reference centres.
        let r = self.workpiece_operating_radius;
        let (cx, cy) = self.corner_centre_at(s);
        // From the pitch point to the corner centre; the fillet is a further ρ
        // along it, because the contact normal runs through the pitch point.
        let (dx, dy) = (cx, cy - r);
        let d = f64::hypot(dx, dy);
        let k = 1.0 + self.tip_round / d;
        let (fx, fy) = (k * dx, r + k * dy);
        // ...then into the workpiece frame. **No `σ` here**, and that is a
        // result rather than an oversight: an internal workpiece does roll the
        // other way relative to its cutter, but the cutter also turns the other
        // way for increasing `s`, because its corner points outward instead of
        // inward. The two reversals cancel, and the workpiece turns
        // `(s − phase)/r` either way.
        let rotation = (s - self.phase) / r;
        (f64::hypot(fx, fy), fx.atan2(fy) - rotation)
    }

    /// The fillet point **and its tangent**, in Cartesian workpiece coordinates.
    ///
    /// `(x, y)` with the tooth centred on `+y`, the same frame
    /// [`crate::strength`] inscribes its parabola in, and the tangent is the
    /// derivative with respect to travel `s` — not normalised, since only its
    /// direction is used.
    ///
    /// # Analytic, deliberately
    ///
    /// A difference quotient would do for the *curvature* — which feeds an
    /// empirical notch factor — but not for the tangent, which the critical
    /// section is located by. So this differentiates the construction rather
    /// than sampling it, and it comes out as the same rotating-frame pattern
    /// [`crate::profile::Gear`]'s rack fillet uses:
    ///
    /// ```text
    /// X = u cos φ − v sin φ        X′ = u′ cos φ − v′ sin φ − φ′ Y
    /// Y = v cos φ + u sin φ        Y′ = u′ sin φ + v′ cos φ + φ′ X
    /// ```
    ///
    /// with `(u, v)` the fillet point in the *fixed* frame and `φ` the
    /// workpiece's own rotation. That the rack and the shaper share the pattern
    /// is not a coincidence: both are a curve carried round by a rolling frame,
    /// and only the curve differs — a corner going round a circle here, along a
    /// line there.
    #[must_use]
    pub fn trochoid_point_and_tangent(&self, s: f64) -> ([f64; 2], [f64; 2]) {
        let r = self.workpiece_operating_radius;
        let sigma = self.sigma();
        let rc = self.corner_radius;

        // The corner centre and its velocity. `φ_c = s / r′_c`, so the cutter
        // turns `1/r′_c` per unit travel.
        let phi_c = s / self.cutter_operating_radius;
        let (sin_c, cos_c) = phi_c.sin_cos();
        let w = 1.0 / self.cutter_operating_radius;
        let (cx, cy) = (rc * sin_c, self.centre_distance - sigma * rc * cos_c);
        let (dcx, dcy) = (rc * w * cos_c, sigma * rc * w * sin_c);

        // From the pitch point to the corner centre, then a further ρ along it.
        let (dx, dy) = (cx, cy - r);
        let d = f64::hypot(dx, dy);
        let dd = (dx * dcx + dy * dcy) / d;
        let k = 1.0 + self.tip_round / d;
        let dk = -self.tip_round * dd / (d * d);

        let (u, v) = (k * dx, r + k * dy);
        let (du, dv) = (dk * dx + k * dcx, dk * dy + k * dcy);

        // ...and into the workpiece frame, which has turned `(s − phase)/r`.
        let phi = (s - self.phase) / r;
        let dphi = 1.0 / r;
        let (c, sn) = (phi.cos(), phi.sin());

        let x = u * c - v * sn;
        let y = v * c + u * sn;
        let dxx = du * c - dv * sn - dphi * y;
        let dyy = du * sn + dv * c + dphi * x;
        ([x, y], [dxx, dyy])
    }

    /// Where the cutter's tip-round centre sits, as an angle from the cutter's
    /// own tooth centreline.
    ///
    /// # The offset of an involute is another involute
    ///
    /// The round is tangent to the flank, so its centre lies a distance `ρ` from
    /// the flank along the normal. That locus is not some new curve: an
    /// involute's normal is the tangent line to its base circle, and the
    /// arc-length along that base circle *is* the involute's angular origin. So
    /// stepping back `ρ` along the normal lands on **the involute of the same
    /// base circle with its origin shifted by `ρ/r_b`** — exact, and no solve.
    ///
    /// The corner centre is then that curve at radius `r_g = r_ac − ρ`:
    ///
    /// ```text
    /// θ_g = s_c/2r_c + inv α_t − inv α_g − ρ/r_bc,     cos α_g = r_bc/r_g
    /// ```
    ///
    /// `s_c` is the cutter's tooth thickness at its pitch circle. The three
    /// terms after it are the tooth narrowing as it climbs to the tip, and the
    /// round's inset.
    fn corner_angle(
        cutter_radius: f64,
        corner_radius: f64,
        tooth: f64,
        alpha_t: f64,
        rho: f64,
    ) -> Option<f64> {
        let base = cutter_radius * alpha_t.cos();
        if corner_radius <= base {
            return None;
        }
        let alpha_g = (base / corner_radius).acos();
        let angle = tooth / (2.0 * cutter_radius) + inv(alpha_t) - inv(alpha_g) - rho / base;
        // A negative angle means the round's centre has crossed the cutter's own
        // tooth centreline: the two corner rounds would overlap, so the tip is
        // narrower than the rounds asked for and this is not a tool. Refused
        // rather than clamped, because which of the round, the addendum and the
        // tooth count to give up is the designer's call.
        if angle < 0.0 {
            return None;
        }
        Some(angle)
    }

    /// The travel at which the corner is at the workpiece's tooth centreline —
    /// the phase [`Self::trochoid_at`] measures from.
    ///
    /// The cutter's tooth fills the workpiece's space, so its centreline sits
    /// half a circular pitch from the workpiece's tooth centreline, and the
    /// corner is `r_c θ_g` of arc back from that. Rolling preserves arc length,
    /// which is why the cutter's angle converts straight into the workpiece's
    /// travel.
    fn phase_from(module_t: f64, cutter_radius: f64, corner_angle: f64) -> f64 {
        std::f64::consts::PI * module_t / 2.0 - cutter_radius * corner_angle
    }

    /// The cutter that a rack-generated [`Gear`] would be, if the rack had
    /// `cutter_teeth` teeth instead of infinitely many.
    ///
    /// Everything is matched to the rack it replaces: the same pitch circle, the
    /// same depth of cut, the same corner round and the same phase. That is what
    /// makes the convergence test a test rather than a demonstration.
    ///
    /// # Errors
    ///
    /// `None` if the cutter is too small to reach the workpiece's root.
    #[must_use]
    pub fn equivalent_to_rack(g: &Gear, cutter_teeth: u32, kind: MeshKind) -> Option<Self> {
        if cutter_teeth == 0 {
            return None;
        }
        let cutter_radius = f64::from(cutter_teeth) * g.mt / 2.0;
        let sigma = match kind {
            MeshKind::External => 1.0,
            MeshKind::Internal => -1.0,
        };
        let centre = g.r + sigma * cutter_radius;
        // Reach the same *depth* the rack does. A ring's dedendum goes outward,
        // so its root is one dedendum beyond the pitch circle rather than one
        // inside it — and the corner then sits at the same place either way,
        // `r_c + dedendum − ρ` from the cutter's axis. That the two cases
        // collapse to one expression is a consequence of measuring depth rather
        // than radius, not a coincidence worth relying on silently.
        let dedendum = g.r - g.rf;
        let root = g.r - sigma * dedendum;
        let corner_radius = sigma * (centre - root) - g.rho;
        if corner_radius.is_nan() || corner_radius <= 0.0 {
            return None;
        }
        // The cutter's tooth fills the workpiece's space, so its thickness at
        // the pitch circle is what the workpiece's tooth leaves over.
        let workpiece_tooth = 2.0 * g.r * (g.psi_b - crate::involute::inv(g.alpha_t));
        let cutter_tooth = std::f64::consts::PI * g.mt - workpiece_tooth;
        let angle =
            Self::corner_angle(cutter_radius, corner_radius, cutter_tooth, g.alpha_t, g.rho)?;
        // Reference centres, deliberately: this cutter stands in for a *rack*,
        // and a rack's pitch line is a machine setting rather than a consequence
        // of two tooth counts, so a shift does not move where it rolls. The
        // operating radii are therefore the reference ones and the scale is 1,
        // which is what keeps the `z_c → ∞` convergence a test of the curve
        // rather than of the placement.
        Some(Self {
            workpiece_radius: g.r,
            cutter_radius,
            cutter_tooth,
            alpha_w: g.alpha_t,
            centre_distance: centre,
            workpiece_operating_radius: g.r,
            cutter_operating_radius: cutter_radius,
            corner_radius,
            tip_round: g.rho,
            phase: Self::phase_from(g.mt, cutter_radius, angle),
            kind,
        })
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::GearParams;

    /// **The analytic tangent against a difference quotient.**
    ///
    /// The critical section is *located* by this tangent, so it has to be
    /// differentiated rather than sampled — and a construction that claims to be
    /// analytic should be checked against the thing it replaces.
    #[test]
    fn the_trochoid_tangent_matches_a_finite_difference() {
        for kind in [MeshKind::External, MeshKind::Internal] {
            for cutter_teeth in [15u32, 20, 40] {
                let g = Gear::new(GearParams {
                    teeth: 43,
                    root_radius: 0.2,
                    ..Default::default()
                });
                let Some(cut) = ShaperCut::equivalent_to_rack(&g, cutter_teeth, kind) else {
                    continue;
                };
                let mut worst = 0.0_f64;
                for i in 0..=20 {
                    #[allow(clippy::cast_precision_loss)]
                    let s = -0.6 + 1.2 * (i as f64 / 20.0);
                    // Balanced, not guessed: a central difference's error is
                    // truncation `~h²` plus cancellation `~ε|f|/h`, minimised
                    // near `(ε|f|)^(1/3)` — about 1e-5 for coordinates of tens
                    // of mm. At 1e-7 the cancellation alone is 1.4e-7, which is
                    // the quotient's floor rather than the tangent's error.
                    let h = 1e-5;
                    let (a, _) = cut.trochoid_point_and_tangent(s - h);
                    let (b, _) = cut.trochoid_point_and_tangent(s + h);
                    let (_, t) = cut.trochoid_point_and_tangent(s);
                    let numeric = [(b[0] - a[0]) / (2.0 * h), (b[1] - a[1]) / (2.0 * h)];
                    // Compare directions, since only the direction is used.
                    let na = f64::hypot(numeric[0], numeric[1]);
                    let ta = f64::hypot(t[0], t[1]);
                    let cross = (numeric[0] * t[1] - numeric[1] * t[0]).abs() / (na * ta);
                    worst = worst.max(cross);
                }
                // 1.2e-9 is the quotient's measured floor at this `h`; the sharp
                // check on the tangent itself is the rack limit below, where two
                // independent analytic derivations meet.
                assert!(
                    worst < 1e-8,
                    "{kind:?} z_c={cutter_teeth}: tangent direction off by {worst}"
                );
            }
        }
    }

    /// The Cartesian point this returns is the polar one `trochoid_at` returns —
    /// the two are the same construction expressed two ways, so a disagreement
    /// would mean the derivative was taken of a different curve.
    #[test]
    fn the_tangent_and_the_polar_form_describe_one_curve() {
        for kind in [MeshKind::External, MeshKind::Internal] {
            let g = Gear::new(GearParams {
                teeth: 43,
                root_radius: 0.2,
                ..Default::default()
            });
            let Some(cut) = ShaperCut::equivalent_to_rack(&g, 20, kind) else {
                continue;
            };
            for i in 0..=10 {
                #[allow(clippy::cast_precision_loss)]
                let s = -0.5 + 1.0 * (i as f64 / 10.0);
                let (radius, angle) = cut.trochoid_at(s);
                let ([x, y], _) = cut.trochoid_point_and_tangent(s);
                assert!((f64::hypot(x, y) - radius).abs() < 1e-12);
                assert!((x.atan2(y) - angle).abs() < 1e-12);
            }
        }
    }

    /// **And in the `z_c → ∞` limit it is the rack's own analytic tangent.**
    ///
    /// The rack is the shaper's limit for the fillet *curve* (tested below), so
    /// it must be the limit for the fillet's tangent too — and the rack's tangent
    /// is derived independently, in `strength.rs`, from a different
    /// parameterisation. Two analytic derivations meeting is worth more than
    /// either meeting a difference quotient.
    #[test]
    fn the_trochoid_tangent_tends_to_the_racks() {
        let g = Gear::new(GearParams {
            teeth: 43,
            root_radius: 0.2,
            ..Default::default()
        });
        let mut previous = f64::INFINITY;
        for cutter_teeth in [1_000u32, 10_000, 100_000] {
            let cut = ShaperCut::equivalent_to_rack(&g, cutter_teeth, MeshKind::External).unwrap();
            let mut worst = 0.0_f64;
            for i in 0..=10 {
                #[allow(clippy::cast_precision_loss)]
                let s = -0.4 + 0.8 * (i as f64 / 10.0);
                let (_, shaper) = cut.trochoid_point_and_tangent(s);
                let (_, rack) = crate::strength::fillet_point_and_tangent(&g, s);
                let ns = f64::hypot(shaper[0], shaper[1]);
                let nr = f64::hypot(rack[0], rack[1]);
                worst = worst.max((shaper[0] * rack[1] - shaper[1] * rack[0]).abs() / (ns * nr));
            }
            assert!(
                worst < previous,
                "z_c={cutter_teeth}: {worst} did not improve on {previous}"
            );
            previous = worst;
        }
        assert!(previous < 1e-4, "the limit is not reached: {previous}");
    }

    /// A gear whose cutter round is small enough to survive being turned into a
    /// pinion cutter.
    ///
    /// The default 0.38 modules is the **rack's** figure: a rack tooth is wide
    /// at its tip and takes it comfortably. A pinion cutter's tooth narrows
    /// toward its tip, so below a few dozen teeth two 0.38 rounds no longer fit
    /// on it and `ShaperCut` refuses the tool. That refusal is correct, and
    /// these tests use a tool that exists.
    fn gear(teeth: u32, shift: f64) -> Gear {
        Gear::new(GearParams {
            teeth,
            profile_shift: shift,
            root_radius: 0.15,
            ..Default::default()
        })
    }

    /// **The rack is the shaper with infinitely many teeth, and this measures
    /// it.** Drive the cutter's tooth count up and the shaper-cut fillet must
    /// converge on the rack-cut one — first order in `1/z_c`, since that is the
    /// order at which a circle of radius `r_c` departs from its tangent line.
    ///
    /// This is the gate for treating the two as one construction rather than
    /// two. Without it, "the rack is a limit" is a remark.
    #[test]
    fn the_shaper_converges_on_the_rack_as_its_teeth_grow() {
        for (teeth, shift) in [(17u32, 0.0), (17, 0.3), (43, -0.2), (9, 0.5)] {
            let g = gear(teeth, shift);
            let mut previous = f64::INFINITY;
            for power in 2..7 {
                let z_c = 10_u32.pow(power);
                let cut = ShaperCut::equivalent_to_rack(&g, z_c, MeshKind::External).unwrap();

                // Compare over the fillet's own travel range.
                let mut worst: f64 = 0.0;
                for i in 0..=20 {
                    #[allow(clippy::cast_precision_loss)]
                    let t = i as f64 / 20.0;
                    let s = g.s_j * (1.0 - t);
                    let (r_rack, a_rack) = g.trochoid_at(s);
                    let (r_shaper, a_shaper) = cut.trochoid_at(s);
                    worst = worst
                        .max((r_shaper - r_rack).abs())
                        .max((a_shaper - a_rack).abs() * g.r);
                }
                assert!(
                    worst < previous,
                    "z={teeth} x={shift} z_c={z_c}: {worst} did not improve on {previous}"
                );
                previous = worst;
            }
            // What is left is the geometry, not the arithmetic, and it can be
            // predicted rather than tolerated: a circle of radius `r_c` departs
            // from its tangent line by `s²/2r_c`, so that is the residue a
            // finite cutter must leave over a fillet spanning `s_j`. Comparing
            // against it rather than against a chosen number is what makes this
            // a statement about the limit.
            let cutter_radius = f64::from(10_u32.pow(6)) * g.mt / 2.0;
            let expected = g.s_j * g.s_j / (2.0 * cutter_radius);
            assert!(
                previous < 3.0 * expected,
                "z={teeth} x={shift}: {previous} mm apart at a million teeth, \
                 against a curvature term of {expected}"
            );
        }
    }

    /// **The derived phase against the rack's, which is verified code.**
    ///
    /// `profile.rs` places the rack's corner at `a_c = s_t/2 + b_c tan α_t +
    /// ρ/cos α_t`, measured from the workpiece's tooth centreline. The shaper's
    /// phase is derived a different way — the cutter's own tooth thickness, its
    /// narrowing to the tip, and the round's inset, all in angle — so agreement
    /// in the limit is a check on the derivation rather than a restatement.
    ///
    /// It is worth its own test even though the convergence test above would
    /// also fail without it: there the phase error would arrive mixed into a
    /// distance, and a claim about the phase should be measured as one.
    #[test]
    fn the_derived_phase_converges_on_the_racks_corner_offset() {
        for (teeth, shift) in [(17u32, 0.0), (17, 0.3), (43, -0.2), (60, 0.0)] {
            let g = gear(teeth, shift);
            let mut previous = f64::INFINITY;
            for power in 2..6 {
                let z_c = 10_u32.pow(power);
                let cut = ShaperCut::equivalent_to_rack(&g, z_c, MeshKind::External).unwrap();
                let error = (cut.phase - g.ac).abs();
                assert!(
                    error < previous,
                    "z={teeth} x={shift} z_c={z_c}: phase error {error} did not improve \
                     on {previous}"
                );
                previous = error;
            }
            assert!(
                previous < 1e-4,
                "z={teeth} x={shift}: phase still {previous} mm from the rack's {}",
                g.ac
            );
        }
    }

    /// The convergence is first order in `1/z_c`: ten times the teeth, a tenth
    /// of the error. Anything else would mean the two constructions differ by
    /// more than the curvature of the pitch line.
    #[test]
    fn the_convergence_is_first_order_in_the_cutter_tooth_count() {
        let g = gear(17, 0.0);
        let error = |z_c: u32| {
            let cut = ShaperCut::equivalent_to_rack(&g, z_c, MeshKind::External).unwrap();
            let mut worst: f64 = 0.0;
            for i in 0..=20 {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f64 / 20.0;
                let s = g.s_j * (1.0 - t);
                worst = worst.max((cut.trochoid_at(s).0 - g.trochoid_at(s).0).abs());
            }
            worst
        };
        let (a, b) = (error(1_000), error(10_000));
        let order = (a / b).log10();
        assert!(
            (order - 1.0).abs() < 0.05,
            "error fell by 10^{order} for a ten-fold cutter, expected 10^1"
        );
    }

    /// **The envelope condition, which is what makes this a cut rather than a
    /// curve.** A fillet point must lie *on* the cutter's corner circle at its
    /// own phase and *outside* it at every other — otherwise the tool would have
    /// removed it.
    ///
    /// This is the check that pins the rolling sense, and it is applied to the
    /// internal case as much as the external: get `σ` wrong anywhere and the
    /// workpiece turns the wrong way, putting the fillet inside the cutter half
    /// a tooth later.
    #[test]
    fn every_fillet_point_is_touched_by_the_cutter_and_cut_by_no_other_phase() {
        for kind in [MeshKind::External, MeshKind::Internal] {
            for (teeth, z_c) in [(43u32, 17u32), (60, 25), (31, 13)] {
                let g = gear(teeth, 0.0);
                let cut = ShaperCut::equivalent_to_rack(&g, z_c, kind).unwrap();

                for i in 0..=10 {
                    #[allow(clippy::cast_precision_loss)]
                    let t = i as f64 / 10.0;
                    let s0 = g.s_j * (1.0 - t) * 0.9;
                    let (radius, angle) = cut.trochoid_at(s0);
                    // The fillet point, back in the fixed frame at travel s0.
                    let rotation = (s0 - cut.phase) / g.r;
                    let world = angle + rotation;
                    let (fx, fy) = (radius * world.sin(), radius * world.cos());

                    let (cx, cy) = cut.corner_centre_at(s0);
                    let own = f64::hypot(fx - cx, fy - cy);
                    assert!(
                        (own - cut.tip_round).abs() < 1e-9,
                        "{kind:?} z={teeth}/{z_c}: the point is {own} from its own corner, \
                         not {}",
                        cut.tip_round
                    );

                    // Now sweep the cutter past it. The workpiece turns too, so
                    // the point moves with it.
                    let span = std::f64::consts::PI * g.mt;
                    for j in -40..=40 {
                        #[allow(clippy::cast_precision_loss)]
                        let s = s0 + span * f64::from(j) / 40.0;
                        let turned = (s - s0) / g.r;
                        let (px, py) = (
                            radius * (world + turned).sin(),
                            radius * (world + turned).cos(),
                        );
                        let (qx, qy) = cut.corner_centre_at(s);
                        let gap = f64::hypot(px - qx, py - qy) - cut.tip_round;
                        assert!(
                            gap > -1e-9,
                            "{kind:?} z={teeth}/{z_c}: the cutter would have removed this \
                             point at s={s}, by {}",
                            -gap
                        );
                    }
                }
            }
        }
    }

    /// An internal workpiece's fillet climbs *outward*: its root is beyond its
    /// pitch circle, which is the whole of what the sign means.
    #[test]
    fn an_internal_fillet_lies_outside_the_pitch_circle() {
        let g = gear(60, 0.0);
        let cut = ShaperCut::equivalent_to_rack(&g, 20, MeshKind::Internal).unwrap();
        let (r0, _) = cut.trochoid_at(0.0);
        assert!(
            r0 > g.r,
            "a ring's fillet is outside its pitch circle: {r0} against {}",
            g.r
        );
        // ...and at the deepest point it reaches the root the cutter was set to.
        let expected = 2.0 * g.r - g.rf;
        assert!(
            (r0 - expected).abs() < 1e-9,
            "reached {r0}, expected the mirrored root {expected}"
        );
    }

    #[test]
    fn a_cutter_too_small_to_reach_the_root_is_refused() {
        let g = gear(17, 0.0);
        assert!(ShaperCut::equivalent_to_rack(&g, 0, MeshKind::External).is_none());
    }
}
