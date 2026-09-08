//! The normal plane and the transverse plane, and the two identities that
//! carry an angle between them.
//!
//! A helical gear is specified in the plane the **tool** works in — the normal
//! plane, across the tooth — and meshes in the plane the **gear** turns in, the
//! transverse one. Two angles differ between those planes, and each is a
//! one-line identity that this crate needs almost everywhere:
//!
//! ```text
//! tan α_t = tan α_n / cos β            the pressure angle, in the plane of rotation
//! sin β_b = sin β · cos α_n            the helix angle, measured on the base cylinder
//! ```
//!
//! # Why they are functions
//!
//! Because they were written out **nineteen times**. `atan(tan α_n / cos β)`
//! appeared in ten places outside the tests and `asin(sin β · cos α_n)` in nine,
//! five of those in `screw.rs` alone — each correct, each one edit away from not
//! being. That is the oldest pattern in `docs/corrections.md`: *one idea written
//! down twice is a place where two answers can differ, and the copy nothing
//! exercises is the one that is wrong.* The internal relative curvature was
//! wrong in two independent ways at once while it was a hand-written branch, and
//! `MeshKind::sign` was written out three times before it became a number.
//!
//! Nothing here is new mathematics and no answer moves: these are the
//! expressions that were already at each site, named once. That is the whole
//! point — a refactor that changes a digit is a different change, and this one
//! is gated on the digits not moving.

/// Transverse pressure angle from the normal one, radians.
///
/// ```text
/// tan α_t = tan α_n / cos β
/// ```
///
/// A spur gear is `β = 0`, where `cos β` is exactly 1 and this is the identity —
/// by construction rather than by a branch, which is why no call site tests for
/// it.
#[must_use]
pub fn transverse_pressure_angle(normal_pressure_angle: f64, helix_angle: f64) -> f64 {
    (normal_pressure_angle.tan() / helix_angle.cos()).atan()
}

/// Base helix angle from the reference one, radians.
///
/// ```text
/// sin β_b = sin β · cos α_n
/// ```
///
/// The angle the helix makes on the **base** cylinder, which is what projects a
/// transverse base-circle arc into the normal plane. It is the factor that turns
/// a transverse curvature, force or face width into the normal-plane one the
/// contact actually sees, and it is exactly zero for a spur gear.
#[must_use]
pub fn base_helix_angle(helix_angle: f64, normal_pressure_angle: f64) -> f64 {
    (helix_angle.sin() * normal_pressure_angle.cos()).asin()
}

/// Base pitch, mm — the distance between successive flanks measured along the
/// line of action.
///
/// ```text
/// p_b = π m cos α
/// ```
///
/// **Both arguments must be in the same plane**, and saying so is the whole of
/// what this signature can do about it: `π m_t cos α_t` is the transverse base
/// pitch and `π m_n cos α_n` the normal one, and they differ by `cos β`. Which
/// is wanted depends on what is being counted in them — a contact ratio divides
/// a transverse path, while a bending section measures a **virtual spur gear**,
/// whose own transverse plane *is* the normal plane, so it passes the pair it
/// has and gets the normal pitch by construction.
///
/// It was written out at six sites, which is the shape the two identities above
/// had at nineteen. Nothing here is new and no answer moves — the multiplication
/// is in the order every site already had it, so the refactor is gated on the
/// digits not changing.
#[must_use]
pub fn base_pitch(module: f64, pressure_angle: f64) -> f64 {
    std::f64::consts::PI * module * pressure_angle.cos()
}

/// The **basic rack** two meshing gears share: one module, and its pressure
/// angle in both planes.
///
/// Three numbers that always travel together and are always derived the same
/// way, named once for the same reason the two identities above are — a third
/// copy of `m/cos β` and `transverse_pressure_angle` was about to be written
/// when this moved here from `planetary`, which is how that pattern starts.
///
/// It lives in this module rather than beside a mesh because its whole content
/// is one rack expressed in two planes, which is this module's subject. It is
/// **not** [`crate::tooth::Rack`]: that is the *tool*, two lengths and a round,
/// and this is the reference the tooth counts are measured against.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BasicRack {
    /// Normal module, mm — the plane the **tool** works in, and so the one an
    /// addendum, a dedendum and a profile shift are counted in.
    ///
    /// Carried rather than dropped because it is the input this was built from,
    /// and a caller that needs it otherwise carries a second copy beside the
    /// rack — which is the duplication this type exists to end, one field
    /// further down.
    pub mn: f64,
    /// Transverse module, mm — the plane the **gear** turns in, and so the one
    /// a radius and a centre distance are measured in.
    pub mt: f64,
    /// Transverse pressure angle, radians.
    pub alpha_t: f64,
    /// Normal pressure angle, radians.
    pub alpha_n: f64,
}

impl BasicRack {
    /// From the inputs as the UI holds them: normal module, and both angles in
    /// degrees.
    #[must_use]
    pub fn new(module: f64, pressure_angle_deg: f64, helix_angle_deg: f64) -> Self {
        let beta = helix_angle_deg.to_radians();
        let alpha_n = pressure_angle_deg.to_radians();
        Self {
            mn: module,
            mt: module / beta.cos(),
            alpha_t: transverse_pressure_angle(alpha_n, beta),
            alpha_n,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A spur gear is not a special case: at `β = 0` both identities reduce
    /// **exactly**, which is what lets every call site take the general form.
    #[test]
    fn a_spur_gear_is_the_identity_to_the_bit() {
        for alpha_deg in [14.5_f64, 20.0, 25.0, 30.0] {
            let alpha = alpha_deg.to_radians();
            assert_eq!(transverse_pressure_angle(alpha, 0.0), alpha);
            assert_eq!(base_helix_angle(0.0, alpha), 0.0);
        }
    }

    /// The relation the two are used together for: `cos α_t cos β_b = cos α_n
    /// cos β`, which is what makes the transverse and normal routes to backlash
    /// agree exactly rather than nearly (`mesh::tests`).
    #[test]
    fn the_two_planes_are_bridged_by_the_standard_identity() {
        for alpha_deg in [14.5_f64, 20.0, 25.0] {
            for beta_deg in [0.0_f64, 12.0, -20.0, 35.0, 60.0] {
                let (alpha, beta) = (alpha_deg.to_radians(), beta_deg.to_radians());
                let alpha_t = transverse_pressure_angle(alpha, beta);
                let beta_b = base_helix_angle(beta, alpha);
                assert!(
                    (alpha_t.cos() * beta_b.cos() - alpha.cos() * beta.cos()).abs() < 1e-15,
                    "α={alpha_deg} β={beta_deg}"
                );
            }
        }
    }

    /// Both are odd in the helix angle's hand, or independent of it — a gear's
    /// hand cannot change the size of an angle it makes with its own axis.
    #[test]
    fn the_hand_of_the_helix_does_not_change_a_magnitude() {
        let alpha = 20.0_f64.to_radians();
        for beta_deg in [12.0_f64, 30.0, 47.5] {
            let beta = beta_deg.to_radians();
            assert_eq!(
                transverse_pressure_angle(alpha, beta),
                transverse_pressure_angle(alpha, -beta)
            );
            assert_eq!(
                base_helix_angle(beta, alpha),
                -base_helix_angle(-beta, alpha)
            );
        }
    }
}
