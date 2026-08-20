//! Two gears in mesh: centre distance, operating pressure angle, backlash.
//!
//! Everything here is closed form except one involute inversion.
//!
//! # The backlash law
//!
//! Backlash is computed exactly, not linearised:
//!
//! ```text
//! j_t = 2 a' ( inv α' − inv α_w )
//! ```
//!
//! where `α_w` is the zero-backlash operating pressure angle and `α'` the actual
//! one at centre distance `a'`. This is worth dwelling on, because the textbook
//! `j_t ≈ 2 Δa tan α_w` is only its first-order expansion:
//!
//! - it is **exact**, verified to 3e-16 mm against a direct computation of tooth
//!   thicknesses at the operating pitch circles;
//! - it is zero at `a' = a_w` by construction;
//! - every source of backlash — profile shift, thickness modification, clearance,
//!   tolerance — enters through just `α_w` and `α'`. One formula, not four cases.
//!
//! # Thickness modification enters through the shift, not separately
//!
//! The sum that sets `α_w` is over `x + x_s`, the *thickness* shift of §4.1, not
//! over `x` alone. When both gears use `k = 1` this is the textbook formula; when
//! a mating pair satisfies `k₁ + k₂ = 2` the `x_s` terms cancel and the centre
//! distance is provably unmoved. Both fall out of one expression rather than
//! needing a special case.

use crate::involute::{inv, inv_inverse};
use crate::profile::Gear;

/// Whether the second member is an external gear or an internal (ring) gear.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshKind {
    External,
    /// Gear 2 is the ring. Its tooth count and shift enter with the opposite
    /// sign, which is the only difference in every formula here.
    Internal,
}

impl MeshKind {
    /// The sign gear 2's tooth count, shift and radii carry: `+1` external,
    /// `−1` internal.
    ///
    /// **This is the whole of the difference between the two kinds**, and it is
    /// returned as a number rather than matched on at each site so that every
    /// relation below can be written once. A ring is a gear whose tooth count,
    /// shift and radius of curvature are negative — the same convention that
    /// lets Hertzian contact treat a concave surface as a negative radius, which
    /// is why [`crate::hertz`] needed no internal case either.
    ///
    /// Writing the two kinds out separately instead is not merely repetition. It
    /// is how this crate carried a wrong internal relative curvature, in two
    /// independent ways at once: the hand-written branch scaled `r_b2` by
    /// `z₁ + z₂` where an internal pair needs `z₂ − z₁`, and wrote
    /// `ρ₂ = r_b2 tan α_w − ξ` where it needs `+ ξ`. Together they were wrong by
    /// **50 % at the pitch point** of a 17/51 pair and by enough to return a
    /// *negative* relative curvature on a 25/41 — reported as "no contact" for
    /// an ordinary internal mesh. Neither was reachable, because
    /// [`ContactPath::new`](crate::contact::ContactPath::new) admitted no
    /// internal mesh; both would have gone live the moment it did.
    /// See `docs/DESIGN.md` §12.
    #[must_use]
    pub const fn sign(self) -> f64 {
        match self {
            Self::External => 1.0,
            Self::Internal => -1.0,
        }
    }
}

/// Why a requested mesh has no real geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeshError {
    /// The two gears cannot mesh at all: different module, pressure angle, or
    /// incompatible helix.
    Incompatible,
    /// An internal mesh needs the ring to have more teeth than the pinion.
    RingTooSmall,
    /// The requested profile shifts drive the operating pressure angle below
    /// zero — the base circles would have to overlap. Not a numerical failure:
    /// there is no such gear pair.
    OutsideInvoluteDomain,
    /// The actual centre distance is below the base-circle limit.
    CentreDistanceTooSmall,
}

/// A meshing pair, with the derived operating geometry.
#[derive(Clone, Copy, Debug)]
pub struct Mesh {
    pub kind: MeshKind,
    /// Transverse pressure angle of the (shared) reference rack.
    pub alpha_t: f64,
    /// Normal pressure angle.
    pub alpha_n: f64,
    /// Transverse module, shared by both members.
    ///
    /// Kept so the reference geometry can be reached without the gears: a base
    /// radius is `m_t z / 2 · cos α_t`, which owes nothing to the centre distance
    /// and so nothing to the involute inversion that produces `alpha_w`.
    pub mt: f64,
    /// Reference centre distance, before any profile shift.
    pub a_ref: f64,
    /// Zero-backlash operating pressure angle, radians.
    pub alpha_w: f64,
    /// Zero-backlash centre distance.
    pub a_w: f64,
    /// Tooth counts. `z2` is the ring for an internal mesh.
    pub z1: u32,
    pub z2: u32,
}

impl Mesh {
    /// Build the operating geometry of a pair.
    ///
    /// # Errors
    ///
    /// Returns [`MeshError`] when the pair cannot mesh, or when the requested
    /// shifts put the operating pressure angle outside the involute domain.
    pub fn new(g1: &Gear, g2: &Gear, kind: MeshKind) -> Result<Self, MeshError> {
        let (p1, p2) = (&g1.params, &g2.params);
        let compatible = (p1.module - p2.module).abs() < 1e-12
            && (p1.pressure_angle - p2.pressure_angle).abs() < 1e-12
            && match kind {
                // External helical gears mesh with opposite hands, so the helix
                // angles are equal and opposite; spur gears have both at zero.
                MeshKind::External => (p1.helix_angle + p2.helix_angle).abs() < 1e-9,
                // Internal pairs share the same hand.
                MeshKind::Internal => (p1.helix_angle - p2.helix_angle).abs() < 1e-9,
            };
        if !compatible {
            return Err(MeshError::Incompatible);
        }
        if kind == MeshKind::Internal && p2.teeth <= p1.teeth {
            return Err(MeshError::RingTooSmall);
        }

        // The sums are over the THICKNESS shift, x + x_s, and gear 2 enters both
        // with the sign of its kind. For an internal pair that makes each sum
        // the negative of the difference the textbook writes — and since only
        // the RATIO sx/sz reaches the operating pressure angle, the two negatives
        // cancel and one expression serves both kinds.
        let s = kind.sign();
        let sx = x_thick(g1) + s * x_thick(g2);
        let sz = f64::from(p1.teeth) + s * f64::from(p2.teeth);

        let alpha_t = g1.alpha_t;
        let alpha_n = g1.alpha_n;
        // A centre distance is a distance, so the reference one is the magnitude;
        // `sz` keeps its sign for the radii below, where it is needed.
        let a_ref = g1.mt * sz.abs() / 2.0;

        let inv_aw = inv(alpha_t) + 2.0 * sx * alpha_n.tan() / sz;
        let alpha_w = inv_inverse(inv_aw).ok_or(MeshError::OutsideInvoluteDomain)?;
        let a_w = a_ref * alpha_t.cos() / alpha_w.cos();

        Ok(Self {
            kind,
            alpha_t,
            alpha_n,
            mt: g1.mt,
            a_ref,
            alpha_w,
            a_w,
            z1: p1.teeth,
            z2: p2.teeth,
        })
    }

    /// Operating pressure angle at an actual centre distance.
    ///
    /// # Errors
    ///
    /// [`MeshError::CentreDistanceTooSmall`] if the base circles cannot reach.
    pub fn pressure_angle_at(&self, a_actual: f64) -> Result<f64, MeshError> {
        let c = self.a_ref * self.alpha_t.cos() / a_actual;
        if !(-1.0..=1.0).contains(&c) {
            return Err(MeshError::CentreDistanceTooSmall);
        }
        Ok(c.acos())
    }

    /// Transverse circumferential backlash at an actual centre distance, mm.
    ///
    /// Positive is play; negative means the teeth interfere. Exact — see the
    /// module documentation.
    ///
    /// # Errors
    ///
    /// [`MeshError::CentreDistanceTooSmall`] if the base circles cannot reach.
    pub fn backlash(&self, a_actual: f64) -> Result<f64, MeshError> {
        let ap = self.pressure_angle_at(a_actual)?;
        Ok(2.0 * a_actual * (inv(ap) - inv(self.alpha_w)))
    }

    /// Angular backlash seen at gear 1 or gear 2, radians.
    ///
    /// # Errors
    ///
    /// [`MeshError::CentreDistanceTooSmall`] if the base circles cannot reach.
    pub fn angular_backlash(&self, a_actual: f64, at_gear: Member) -> Result<f64, MeshError> {
        let j = self.backlash(a_actual)?;
        // Play is a magnitude at whichever member is asked, so both the tooth
        // sum and the member's own count enter by magnitude. Which member turns
        // which way is a kinematic question and not this one.
        let z = match at_gear {
            Member::First => f64::from(self.z1),
            Member::Second => f64::from(self.z2),
        };
        Ok(j * self.tooth_sum().abs() / (a_actual * z))
    }

    /// Gear 2's tooth count, signed: negative when gear 2 is a ring.
    ///
    /// See [`MeshKind::sign`] for why the sign lives on the tooth count rather
    /// than in a branch at each formula.
    #[must_use]
    pub fn signed_z2(&self) -> f64 {
        self.kind.sign() * f64::from(self.z2)
    }

    /// `z₁ + z₂` with gear 2 signed — the tooth sum every relation is scaled by.
    ///
    /// Negative for an internal pair, where its magnitude is the `z₂ − z₁` the
    /// textbooks write.
    #[must_use]
    pub fn tooth_sum(&self) -> f64 {
        f64::from(self.z1) + self.signed_z2()
    }

    /// Operating pitch radii of the two members, mm — gear 2's **signed**.
    ///
    /// Gear 2's radius is negative for a ring, which is the same convention that
    /// makes it a concave surface to [`crate::hertz`].
    #[must_use]
    pub fn operating_radii(&self) -> (f64, f64) {
        let sz = self.tooth_sum().abs();
        (
            self.a_w * f64::from(self.z1) / sz,
            self.a_w * self.signed_z2() / sz,
        )
    }

    /// Base radii of the two members, mm — gear 2's **signed**.
    ///
    /// # Reached through the reference geometry, deliberately
    ///
    /// A base radius is `r cos α_t` *and* `r' cos α_w`; the two are equal because
    /// a profile shift moves `r'` and `α_w` together and leaves the product
    /// alone. They are not equally good to compute. `α_w` comes out of the
    /// involute inversion, so the operating route carries that solve's residual
    /// while `m_t z / 2 · cos α_t` carries none — and it is what
    /// [`Gear::rb`](crate::Gear) itself is, bit for bit.
    ///
    /// An earlier revision reached gear 1's this way and gear 2's through `a_w`,
    /// which left one pair of curvatures built from two different routes and
    /// `r_b1 + r_b2 = a_w cos α_w` true only to an ulp. Both through the
    /// reference geometry makes that identity exact, which is what
    /// [`Self::curvature_radii`]'s constant-sum property rests on.
    #[must_use]
    pub fn base_radii(&self) -> (f64, f64) {
        let c = self.alpha_t.cos();
        let r = |z: f64| self.mt * z / 2.0 * c;
        (r(f64::from(self.z1)), r(self.signed_z2()))
    }

    /// Radii of curvature of the two involutes at a position `ξ` on the line of
    /// action, mm — gear 2's **signed**.
    ///
    /// `ξ` is measured from the pitch point, positive toward gear 1's tip.
    ///
    /// # The one relation, and the sign that carries the kind
    ///
    /// Each member's radius of curvature is the distance from the contact point
    /// to its own base tangent point, so
    ///
    /// ```text
    /// ρ₁ = r_b1 tan α_w + ξ            ρ₂ = r_b2 tan α_w − ξ
    /// ```
    ///
    /// With `r_b2` signed this is **one pair of expressions for both kinds**, and
    /// it reproduces each textbook relation exactly:
    ///
    /// ```text
    /// ρ₁ + ρ₂ = a_w sin α_w            external — the sum is constant
    /// ρ₂ − ρ₁ = a_w sin α_w            internal — the DIFFERENCE is constant
    /// ```
    ///
    /// both of which are the single statement `ρ₁ + ρ₂ = σ · a_w sin α_w` in the
    /// signed values. The internal case needs `ρ₂ = |r_b2| tan α_w + ξ` in
    /// unsigned terms — note the `+ ξ` — and getting that from a hand-written
    /// branch instead is what this crate previously got wrong.
    #[must_use]
    pub fn curvature_radii(&self, xi: f64) -> (f64, f64) {
        let (rb1, rb2) = self.base_radii();
        let t = self.alpha_w.tan();
        (rb1 * t + xi, rb2 * t - xi)
    }

    /// Transverse relative curvature `1/ρ₁ + 1/ρ₂` at `ξ`, per mm.
    ///
    /// One expression for both kinds: gear 2's `ρ` is negative for a ring, so the
    /// sum *is* the difference an internal pair needs. `None` where either flank
    /// has run inside its base circle, or where the relative curvature is not
    /// positive — an internal pair whose members are too close in size has no
    /// contact to press.
    #[must_use]
    pub fn relative_curvature(&self, xi: f64) -> Option<f64> {
        let (rho1, rho2) = self.curvature_radii(xi);
        let s = self.kind.sign();
        // Each ρ must be positive in its own sense: gear 1's outright, gear 2's
        // after its sign is taken off.
        if rho1 <= 0.0 || s * rho2 <= 0.0 {
            return None;
        }
        let inv_rho = 1.0 / rho1 + 1.0 / rho2;
        (inv_rho > 0.0).then_some(inv_rho)
    }

    /// Speed ratio, output over input, ignoring sign.
    #[must_use]
    pub fn ratio(&self) -> f64 {
        f64::from(self.z2) / f64::from(self.z1)
    }
}

/// Which member of a pair a quantity refers to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Member {
    First,
    Second,
}

/// The thickness shift of a gear: `x + x_s`.
///
/// Thickness quantities take this; radial quantities take plain `x`. Keeping the
/// distinction in one named function is what stops the two being confused.
fn x_thick(g: &Gear) -> f64 {
    g.params.profile_shift + g.params.thickness_shift()
}

impl std::fmt::Display for MeshError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::Incompatible => "the gears cannot mesh: module, pressure angle or helix differ",
            Self::RingTooSmall => "an internal mesh needs more teeth on the ring than the pinion",
            Self::OutsideInvoluteDomain => {
                "no such gear pair: the profile shifts require the base circles to overlap"
            }
            Self::CentreDistanceTooSmall => "centre distance is below the base-circle limit",
        };
        f.write_str(s)
    }
}

impl std::error::Error for MeshError {}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::GearParams;

    fn pair(z1: u32, z2: u32, x1: f64, x2: f64) -> (Gear, Gear) {
        (
            Gear::new(GearParams {
                teeth: z1,
                profile_shift: x1,
                ..Default::default()
            }),
            Gear::new(GearParams {
                teeth: z2,
                profile_shift: x2,
                ..Default::default()
            }),
        )
    }

    /// **The one relation both kinds obey, in signed values.**
    ///
    /// Each member's radius of curvature is its distance from the contact point
    /// to its own base tangent point, so the two must satisfy
    ///
    /// ```text
    /// rho_1 + rho_2 = sigma . a_w sin(alpha_w)
    /// ```
    ///
    /// at *every* position on the line of action — a constant sum for an
    /// external pair and, once gear 2's is signed, the constant **difference**
    /// an internal pair needs. This is the property that lets one expression
    /// serve both, so it is asserted along the whole path rather than at the
    /// pitch point, where a sign error in the `xi` term cancels.
    #[test]
    fn curvature_radii_obey_one_signed_relation_along_the_whole_path() {
        for (kind, z1, z2) in [
            (MeshKind::External, 17u32, 43u32),
            (MeshKind::External, 13, 60),
            (MeshKind::Internal, 17, 51),
            (MeshKind::Internal, 20, 60),
            (MeshKind::Internal, 31, 34),
        ] {
            let (a, b) = pair(z1, z2, 0.0, 0.0);
            let m = Mesh::new(&a, &b, kind).unwrap();
            let want = kind.sign() * m.a_w * m.alpha_w.sin();
            for step in -8..=8 {
                let xi = f64::from(step) * 0.35;
                let (rho1, rho2) = m.curvature_radii(xi);
                assert!(
                    (rho1 + rho2 - want).abs() < 1e-12 * m.a_w,
                    "{kind:?} {z1}/{z2} at xi={xi}: rho1+rho2 = {}, want {want}",
                    rho1 + rho2
                );
            }
        }
    }

    /// A ring's radius of curvature is **negative**, and that is the whole of
    /// what makes it internal.
    ///
    /// Stated separately from the relation above because the relation would also
    /// hold with both signs flipped, and it is the sign itself that
    /// [`crate::hertz`] reads as "concave".
    #[test]
    fn only_a_ring_curves_the_other_way() {
        let (p, r) = pair(17, 51, 0.0, 0.0);
        let internal = Mesh::new(&p, &r, MeshKind::Internal).unwrap();
        let (rho1, rho2) = internal.curvature_radii(0.0);
        assert!(rho1 > 0.0, "the pinion is convex: {rho1}");
        assert!(rho2 < 0.0, "the ring must be concave: {rho2}");
        assert!(internal.signed_z2() < 0.0);
        assert!(internal.tooth_sum() < 0.0);
        // ...while the centre distance is a distance, and stays one.
        assert!(internal.a_w > 0.0);
        assert!((internal.a_w - 17.0).abs() < 1e-12);

        let (a, b) = pair(17, 51, 0.0, 0.0);
        let external = Mesh::new(&a, &b, MeshKind::External).unwrap();
        let (e1, e2) = external.curvature_radii(0.0);
        assert!(e1 > 0.0 && e2 > 0.0, "both flanks convex: {e1}, {e2}");
    }

    /// **The internal relative curvature, against the relation derived the other
    /// way — and against the error that was there before.**
    ///
    /// DESIGN.md §4.11 gives the internal pair's conjugate relation with both
    /// distances positive:
    ///
    /// ```text
    /// rho_2 - rho_1 = a_w sin(alpha_w),     rho_2 = |r_b2| tan(alpha_w) + xi
    /// ```
    ///
    /// Note the **plus** `xi`. The branch this replaced wrote `minus`, and also
    /// scaled `r_b2` by `z1 + z2` where an internal pair needs `z2 - z1`. The two
    /// errors cancel exactly at `xi = 0` and nowhere else, which is why this
    /// sweeps and why the old check — made at the pitch point — passed.
    #[test]
    fn internal_relative_curvature_matches_the_conjugate_relation_off_the_pitch_point() {
        for (z1, z2) in [(17u32, 51u32), (20, 60), (25, 41)] {
            let (p, r) = pair(z1, z2, 0.0, 0.0);
            let m = Mesh::new(&p, &r, MeshKind::Internal).unwrap();
            let t = m.alpha_w.tan();
            // Independently: base radii straight from each gear, positive, and
            // the relation read as a difference.
            let (rb1, rb2) = (p.rb, r.rb);
            assert!(
                ((rb2 - rb1) * t - m.a_w * m.alpha_w.sin()).abs() < 1e-12,
                "the pair's own relation must hold first"
            );

            for step in -6..=6 {
                let xi = f64::from(step) * 0.4;
                let want = 1.0 / (rb1 * t + xi) - 1.0 / (rb2 * t + xi);
                let got = m.relative_curvature(xi).unwrap();
                assert!(
                    (got - want).abs() < 1e-15 * want.abs(),
                    "{z1}/{z2} at xi={xi}: {got} vs {want}"
                );
            }
        }
    }

    /// **An internal mesh presses a convex flank against a concave one, so its
    /// relative curvature is lower than the external pair of the same teeth.**
    ///
    /// A law rather than a number, and the cheapest independent check on the sign
    /// convention: `1/ρ₁ − 1/ρ₂ < 1/ρ₁ + 1/ρ₂` for any positive pair, so a ring
    /// must come out *softer* at every position on the path. Getting gear 2's
    /// sign backwards inverts this, and no self-consistent test of the internal
    /// pair alone would notice.
    #[test]
    fn an_internal_mesh_is_less_curved_than_the_external_pair_of_the_same_teeth() {
        for (z1, z2) in [(17u32, 51u32), (20, 60), (25, 41)] {
            let (a, b) = pair(z1, z2, 0.0, 0.0);
            let internal = Mesh::new(&a, &b, MeshKind::Internal).unwrap();
            let external = Mesh::new(&a, &b, MeshKind::External).unwrap();
            for step in -4..=4 {
                let xi = f64::from(step) * 0.4;
                let (i, e) = (
                    internal.relative_curvature(xi).unwrap(),
                    external.relative_curvature(xi).unwrap(),
                );
                assert!(
                    i > 0.0 && i < e,
                    "{z1}/{z2} at xi={xi}: internal {i} should be positive and below external {e}"
                );
            }
        }
    }

    /// A base radius owes nothing to the centre distance, so the two routes to
    /// it must agree — and the reference route is the one that does not carry the
    /// involute inversion's residual.
    #[test]
    fn base_radii_agree_with_each_gears_own() {
        for (kind, z1, z2, x1, x2) in [
            (MeshKind::External, 17u32, 43u32, 0.0, 0.0),
            (MeshKind::External, 17, 43, 0.3, -0.15),
            (MeshKind::Internal, 17, 51, 0.0, 0.0),
            (MeshKind::Internal, 17, 51, 0.25, 0.4),
        ] {
            let (a, b) = pair(z1, z2, x1, x2);
            let m = Mesh::new(&a, &b, kind).unwrap();
            let (rb1, rb2) = m.base_radii();
            assert_eq!(
                rb1, a.rb,
                "gear 1's base radius must be its own, bit for bit"
            );
            assert_eq!(rb2, kind.sign() * b.rb, "and gear 2's, up to its sign");
            // r_b1 + r_b2 = a_w cos(alpha_w) is then exact rather than close.
            assert!(
                (rb1 + rb2 - kind.sign() * m.a_w * m.alpha_w.cos()).abs() < 1e-12,
                "{kind:?} {z1}/{z2}: {} vs {}",
                rb1 + rb2,
                kind.sign() * m.a_w * m.alpha_w.cos()
            );
        }
    }

    #[test]
    fn unshifted_pair_meshes_at_the_reference_centre_distance() {
        let (a, b) = pair(17, 23, 0.0, 0.0);
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        assert!((m.a_w - 20.0).abs() < 1e-12, "a_w = {}", m.a_w);
        assert!((m.alpha_w - m.alpha_t).abs() < 1e-14);
    }

    #[test]
    fn positive_shift_opens_the_centre_distance() {
        let (a, b) = pair(17, 23, 0.3, 0.2);
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        assert!(m.a_w > 20.0 && m.a_w < 20.6, "a_w = {}", m.a_w);
        // and the operating pressure angle rises with it
        assert!(m.alpha_w > m.alpha_t);
    }

    #[test]
    fn backlash_is_zero_at_the_zero_backlash_centre_distance() {
        for (z1, z2, x1, x2) in [(17, 23, 0.0, 0.0), (13, 41, 0.4, -0.2), (9, 9, 0.5, 0.5)] {
            let (a, b) = pair(z1, z2, x1, x2);
            let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
            assert!(m.backlash(m.a_w).unwrap().abs() < 1e-12);
        }
    }

    /// The textbook `j ≈ 2 Δa tan α_w` must appear as the small-Δa limit of the
    /// exact law. If it does not, one of the two is wrong.
    #[test]
    fn exact_backlash_reduces_to_the_textbook_linearisation() {
        let (a, b) = pair(17, 23, 0.2, 0.1);
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        for da in [1e-4_f64, 1e-3, 1e-2] {
            let exact = m.backlash(m.a_w + da).unwrap();
            let linear = 2.0 * da * m.alpha_w.tan();
            // second-order agreement: the error must shrink quadratically
            let rel = (exact - linear).abs() / linear;
            assert!(
                rel < 20.0 * da,
                "da={da}: exact {exact:.9}, linear {linear:.9}, rel {rel:.2e}"
            );
        }
    }

    /// The property relied on downstream: a mating pair with `k1 + k2 = 2` has
    /// the same centre distance as an unmodified pair.
    #[test]
    fn paired_thickness_modification_does_not_move_the_centre_distance() {
        let plain = Mesh::new(
            &Gear::new(GearParams {
                teeth: 17,
                ..Default::default()
            }),
            &Gear::new(GearParams {
                teeth: 23,
                ..Default::default()
            }),
            MeshKind::External,
        )
        .unwrap();
        for k in [0.6_f64, 0.9, 1.3, 1.7] {
            let m = Mesh::new(
                &Gear::new(GearParams {
                    teeth: 17,
                    thickness_mod: k,
                    ..Default::default()
                }),
                &Gear::new(GearParams {
                    teeth: 23,
                    thickness_mod: 2.0 - k,
                    ..Default::default()
                }),
                MeshKind::External,
            )
            .unwrap();
            assert!(
                (m.a_w - plain.a_w).abs() < 1e-12,
                "k={k}: centre distance moved from {} to {}",
                plain.a_w,
                m.a_w
            );
        }
    }

    /// An unpaired thickness modification is not a no-op — it shows up as
    /// backlash at the unmodified centre distance. Same formula, no special case.
    #[test]
    fn unpaired_thickness_modification_appears_as_backlash() {
        let plain = Mesh::new(
            &Gear::new(GearParams {
                teeth: 17,
                ..Default::default()
            }),
            &Gear::new(GearParams {
                teeth: 23,
                ..Default::default()
            }),
            MeshKind::External,
        )
        .unwrap();
        // thin gear 1 only: k < 1 removes material, so there is play
        let thin = Mesh::new(
            &Gear::new(GearParams {
                teeth: 17,
                thickness_mod: 0.9,
                ..Default::default()
            }),
            &Gear::new(GearParams {
                teeth: 23,
                ..Default::default()
            }),
            MeshKind::External,
        )
        .unwrap();
        let j = thin.backlash(plain.a_w).unwrap();
        assert!(j > 0.0, "thinning a tooth should open backlash, got {j}");
    }

    #[test]
    fn internal_mesh_centre_distance_is_the_difference() {
        let sun = Gear::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        let ring = Gear::new(GearParams {
            teeth: 51,
            ..Default::default()
        });
        let m = Mesh::new(&sun, &ring, MeshKind::Internal).unwrap();
        assert!((m.a_w - 17.0).abs() < 1e-12, "a_w = {}", m.a_w);
    }

    #[test]
    fn impossible_pairs_are_rejected_rather_than_fudged() {
        let a = Gear::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        let b = Gear::new(GearParams {
            teeth: 23,
            module: 2.0,
            ..Default::default()
        });
        assert_eq!(
            Mesh::new(&a, &b, MeshKind::External).unwrap_err(),
            MeshError::Incompatible
        );

        let small = Gear::new(GearParams {
            teeth: 40,
            ..Default::default()
        });
        let ring = Gear::new(GearParams {
            teeth: 20,
            ..Default::default()
        });
        assert_eq!(
            Mesh::new(&small, &ring, MeshKind::Internal).unwrap_err(),
            MeshError::RingTooSmall
        );

        // Large negative shifts push the operating pressure angle below zero.
        let a = Gear::new(GearParams {
            teeth: 17,
            profile_shift: -1.9,
            ..Default::default()
        });
        let b = Gear::new(GearParams {
            teeth: 23,
            profile_shift: -1.9,
            ..Default::default()
        });
        assert_eq!(
            Mesh::new(&a, &b, MeshKind::External).unwrap_err(),
            MeshError::OutsideInvoluteDomain
        );
    }
}
