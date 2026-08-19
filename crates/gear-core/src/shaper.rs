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
    /// The workpiece's tooth thickness at its pitch circle, as an arc, mm. The
    /// cutter's tooth takes the remainder of the pitch, because one fills what
    /// the other leaves.
    pub workpiece_tooth: f64,
    /// Teeth on the cutter.
    pub cutter_teeth: u32,
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
        if p.cutter_teeth == 0 || !p.module_t.is_finite() {
            return None;
        }
        let cutter_radius = f64::from(p.cutter_teeth) * p.module_t / 2.0;
        let corner_radius = p.cutter_tip_radius - p.tip_round;
        if corner_radius.is_nan() || corner_radius <= 0.0 {
            return None;
        }
        let cutter_tooth = std::f64::consts::PI * p.module_t - p.workpiece_tooth;
        let angle = Self::corner_angle(
            cutter_radius,
            corner_radius,
            cutter_tooth,
            p.alpha_t,
            p.tip_round,
        )?;
        Some(Self {
            workpiece_radius: p.workpiece_radius,
            cutter_radius,
            corner_radius,
            tip_round: p.tip_round,
            phase: Self::phase_from(p.module_t, cutter_radius, angle),
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

    /// Centre distance between workpiece and cutter, mm.
    #[must_use]
    pub fn centre_distance(&self) -> f64 {
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
        let (sin_phi, cos_phi) = (s / self.cutter_radius).sin_cos();
        (
            self.corner_radius * sin_phi,
            self.centre_distance() - sigma * self.corner_radius * cos_phi,
        )
    }

    /// The trochoid fillet at pitch-line travel `s`, as `(radius, angle from the
    /// tooth centreline)` — the same pair [`Gear::trochoid_at`] returns.
    #[must_use]
    pub fn trochoid_at(&self, s: f64) -> (f64, f64) {
        let r = self.workpiece_radius;
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
        Some(tooth / (2.0 * cutter_radius) + inv(alpha_t) - inv(alpha_g) - rho / base)
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
        Some(Self {
            workpiece_radius: g.r,
            cutter_radius,
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

    fn gear(teeth: u32, shift: f64) -> Gear {
        Gear::new(GearParams {
            teeth,
            profile_shift: shift,
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
