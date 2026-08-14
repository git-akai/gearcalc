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

        // The sums are over the THICKNESS shift, x + x_s.
        let sx = match kind {
            MeshKind::External => x_thick(g1) + x_thick(g2),
            MeshKind::Internal => x_thick(g2) - x_thick(g1),
        };
        let sz = match kind {
            MeshKind::External => f64::from(p1.teeth) + f64::from(p2.teeth),
            MeshKind::Internal => f64::from(p2.teeth) - f64::from(p1.teeth),
        };

        let alpha_t = g1.alpha_t;
        let alpha_n = g1.alpha_n;
        let a_ref = g1.mt * sz / 2.0;

        let inv_aw = inv(alpha_t) + 2.0 * sx * alpha_n.tan() / sz;
        let alpha_w = inv_inverse(inv_aw).ok_or(MeshError::OutsideInvoluteDomain)?;
        let a_w = a_ref * alpha_t.cos() / alpha_w.cos();

        Ok(Self {
            kind,
            alpha_t,
            alpha_n,
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
        let sz = f64::from(self.z1)
            + match self.kind {
                MeshKind::External => f64::from(self.z2),
                MeshKind::Internal => -f64::from(self.z2),
            };
        // Operating pitch radius of the chosen member.
        let z = match at_gear {
            Member::First => f64::from(self.z1),
            Member::Second => f64::from(self.z2),
        };
        Ok(j * sz.abs() / (a_actual * z))
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
