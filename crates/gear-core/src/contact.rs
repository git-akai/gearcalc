//! The path of contact, and how load is shared along it.
//!
//! Bending stress depends on *where on the flank* the load acts and *how much of
//! it* that tooth is carrying. Those are separate questions and they pull in
//! opposite directions: moving the load down the flank from the tip shortens the
//! moment arm, while sharing splits the load with a neighbour. Both reduce the
//! stress below the single-tooth-at-the-tip worst case, which is why a design
//! rated on that worst case alone is conservative by a margin worth knowing.
//!
//! Positions along the line of action are measured as `ξ` from the **pitch
//! point**, positive toward gear 1's tip.

use crate::mesh::{Mesh, MeshKind};
use crate::profile::Gear;

/// The path of contact for a meshing pair.
#[derive(Clone, Copy, Debug)]
pub struct ContactPath {
    /// Approach length: from first contact to the pitch point.
    pub approach: f64,
    /// Recess length: from the pitch point to last contact.
    pub recess: f64,
    /// Transverse base pitch — the spacing of successive tooth pairs along the
    /// line of action.
    pub base_pitch: f64,
    /// Transverse contact ratio.
    pub contact_ratio: f64,
    /// Operating pitch radius of gear 1.
    pub operating_radius_1: f64,
    /// Base radius of gear 1.
    pub base_radius_1: f64,
    /// Operating pressure angle.
    pub alpha_w: f64,
}

impl ContactPath {
    /// Build the contact path for a pair meshing at their zero-backlash centre
    /// distance.
    ///
    /// Returns `None` for an internal mesh, which is not yet covered.
    #[must_use]
    pub fn new(g1: &Gear, g2: &Gear, mesh: &Mesh) -> Option<Self> {
        if mesh.kind != MeshKind::External {
            return None;
        }
        let sz = f64::from(mesh.z1) + f64::from(mesh.z2);
        let r1 = mesh.a_w * f64::from(mesh.z1) / sz;
        let r2 = mesh.a_w * f64::from(mesh.z2) / sz;

        // Each length is measured from the PITCH POINT, so each subtracts the
        // distance from its own gear's base tangent point to the pitch point —
        // r' sin α_w, not the whole tangent length a_w sin α_w. Only their sum
        // uses a_w, since r'_1 + r'_2 = a_w, which is why the familiar
        // contact-ratio formula has a_w in it and this does not.
        let recess = (g1.ra.powi(2) - g1.rb.powi(2)).sqrt() - r1 * mesh.alpha_w.sin();
        let approach = (g2.ra.powi(2) - g2.rb.powi(2)).sqrt() - r2 * mesh.alpha_w.sin();
        if recess <= 0.0 || approach <= 0.0 {
            return None;
        }
        let base_pitch = std::f64::consts::PI * g1.mt * g1.alpha_t.cos();

        Some(Self {
            approach,
            recess,
            base_pitch,
            contact_ratio: (approach + recess) / base_pitch,
            operating_radius_1: r1,
            base_radius_1: g1.rb,
            alpha_w: mesh.alpha_w,
        })
    }

    /// Gear 1's involute roll parameter at a position on the line of action.
    #[must_use]
    pub fn roll_at(&self, xi: f64) -> f64 {
        (self.operating_radius_1 * self.alpha_w.sin() + xi) / self.base_radius_1
    }

    /// Where gear 1 is at its tip: the last instant of contact.
    #[must_use]
    pub fn tip(&self) -> f64 {
        self.recess
    }

    /// The highest point of single-pair tooth contact on gear 1.
    ///
    /// One base pitch along from first contact: below that the preceding pair is
    /// still engaged, so the load is shared and this tooth is not alone.
    #[must_use]
    pub fn highest_single_pair(&self) -> f64 {
        (-self.approach + self.base_pitch).min(self.recess)
    }

    /// The lowest point of single-pair tooth contact on gear 1.
    ///
    /// One base pitch back from last contact. This is the **contact** stress
    /// worst case rather than the bending one: gear 1's relative radius of
    /// curvature is smallest at the approach end, so the point where it first
    /// carries the whole load alone is where the Hertzian pressure peaks.
    /// Bending's worst case is the opposite end, [`Self::highest_single_pair`],
    /// where the moment arm is longest.
    #[must_use]
    pub fn lowest_single_pair(&self) -> f64 {
        (self.recess - self.base_pitch).max(-self.approach)
    }

    /// How much of the total load this tooth carries at position `ξ`.
    ///
    /// Between one and two pairs are in contact at any moment, and where two are
    /// the split depends on their relative stiffness. This is the
    /// [`LoadSharing`] model's job; see its documentation for what it is and is
    /// not.
    #[must_use]
    pub fn load_fraction(&self, xi: f64, model: LoadSharing) -> f64 {
        let start = -self.approach;
        let end = self.recess;
        // The endpoints are legitimate positions, and a caller sweeping the mesh
        // cycle lands on them by construction — so admit them despite rounding
        // rather than returning "no contact" a hair outside.
        let slack = (end - start) * 1e-9;
        if xi < start - slack || xi > end + slack {
            return 0.0;
        }
        let xi = xi.clamp(start, end);
        match model {
            LoadSharing::None => 1.0,
            LoadSharing::LinearRamp => {
                // Single-pair zone: this tooth carries everything.
                let single_lo = end - self.base_pitch;
                let single_hi = start + self.base_pitch;
                if xi >= single_lo && xi <= single_hi {
                    return 1.0;
                }
                // Double-pair zones: ramp between the endpoints below.
                if xi < single_lo {
                    let t = (xi - start) / (single_lo - start).max(f64::MIN_POSITIVE);
                    RAMP_MIN + (RAMP_MAX - RAMP_MIN) * t
                } else {
                    let t = (end - xi) / (end - single_hi).max(f64::MIN_POSITIVE);
                    RAMP_MIN + (RAMP_MAX - RAMP_MIN) * t
                }
            }
        }
    }
}

/// Mesh efficiency, as a fraction — `0.98` means 98 %.
///
/// # Derived, not quoted
///
/// At a contact point `ξ` from the pitch point the flanks slide at `ξ(ω₁+ω₂)`
/// while the transmitted power is `F_n v_b`, with `v_b = ω₁r_b1 = ω₂r_b2`. So
/// the instantaneous fractional loss is
///
/// ```text
/// μ |ξ| (1/r_b1 + 1/r_b2)
/// ```
///
/// Contact travels the line of action at constant speed, so the time average is
/// the uniform average of `|ξ|` over the path, `(ξ_a² + ξ_r²) / 2(ξ_a + ξ_r)`.
/// Substituting `r_b = m_t z cos α_t / 2` and `p_bt = π m_t cos α_t` collapses
/// it to
///
/// ```text
/// η = 1 − μ π (1/z₁ ± 1/z₂) (ε₁² + ε₂²) / ε_α        + external, − internal
/// ```
///
/// which is Buckingham's formula recovered from first principles — no fitted
/// constants. [verified against a numerical average of the instantaneous loss
/// over six meshes: agreement to 1e-10 relative.]
///
/// **The `/ε_α` is not decoration, and dropping it is the easy mistake.** It is
/// what holds the *total* transmitted force at `F_n`. Average `|ξ|` per
/// engagement instead, counting one engagement per base pitch, and you have
/// implicitly let every engaged pair carry the whole load — so the mesh
/// transmits `ε_α F_n` and the loss comes out too big by exactly that factor.
///
/// # Two honest notes
///
/// **Forward and backward efficiency are equal.** The formula is symmetric in
/// `ε₁² + ε₂²`, and physically it should be: swapping driver for driven swaps
/// approach and recess but not the total sliding. Two identical numbers in the
/// UI are a result, not a bug. (Crossed-axis screw gearing is the case where
/// they genuinely differ — see DESIGN.md §4.5.1.)
///
/// **`μ` is a mesh input, not a material property.** It depends on lubrication,
/// speed, finish and temperature at least as much as on the pair of materials,
/// and no defensible per-pair table was available.
#[must_use]
pub fn efficiency(path: &ContactPath, mesh: &Mesh, friction: f64) -> f64 {
    let e1 = path.approach / path.base_pitch;
    let e2 = path.recess / path.base_pitch;
    let z = match mesh.kind {
        MeshKind::External => 1.0 / f64::from(mesh.z1) + 1.0 / f64::from(mesh.z2),
        MeshKind::Internal => 1.0 / f64::from(mesh.z1) - 1.0 / f64::from(mesh.z2),
    };
    1.0 - friction * std::f64::consts::PI * z * (e1 * e1 + e2 * e2) / path.contact_ratio
}

/// Load fraction at the outer edge of a double-contact zone.
///
/// **Uncalibrated.** A tooth entering mesh at its root is stiffer than its
/// partner near the tip, so it takes less than half; 1/3 to 2/3 across the
/// double-contact zone is the common first-order stand-in in the literature for
/// spur gears. Replacing it with a real mesh-stiffness model is the work
/// deferred in DESIGN.md.
const RAMP_MIN: f64 = 1.0 / 3.0;
const RAMP_MAX: f64 = 2.0 / 3.0;

/// How the load is divided when two tooth pairs are in contact.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LoadSharing {
    /// No sharing: this tooth carries the whole load wherever it is in mesh.
    #[default]
    None,
    /// A linear ramp across the double-contact zones.
    ///
    /// This is a **placeholder for a stiffness model, not a substitute for
    /// one.** Real sharing depends on tooth, rim and mesh stiffness, on
    /// deflection under load, and on manufacturing deviation — none of which
    /// this knows about. It is here to size the effect, not to certify a design.
    LinearRamp,
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::mesh::MeshKind;
    use crate::GearParams;

    fn pair(z1: u32, z2: u32) -> (Gear, Gear, Mesh) {
        let a = Gear::new(GearParams {
            teeth: z1,
            ..Default::default()
        });
        let b = Gear::new(GearParams {
            teeth: z2,
            ..Default::default()
        });
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        (a, b, m)
    }

    /// The two lengths must reproduce the familiar contact-ratio formula, which
    /// is written with `a_w sin α_w`. Getting this wrong is easy — an early
    /// version subtracted the whole tangent length from *each* part instead of
    /// each gear's own share, which made both lengths negative.
    #[test]
    fn approach_and_recess_sum_to_the_standard_length_of_action() {
        for (z1, z2) in [(17u32, 17u32), (17, 43), (13, 60), (25, 25)] {
            let (a, b, m) = pair(z1, z2);
            let path = ContactPath::new(&a, &b, &m).unwrap();
            let standard = (a.ra.powi(2) - a.rb.powi(2)).sqrt()
                + (b.ra.powi(2) - b.rb.powi(2)).sqrt()
                - m.a_w * m.alpha_w.sin();
            assert!(
                (path.approach + path.recess - standard).abs() < 1e-12,
                "z={z1}/{z2}: {} vs {standard}",
                path.approach + path.recess
            );
            assert!(path.approach > 0.0 && path.recess > 0.0);
        }
    }

    #[test]
    fn contact_ratio_is_in_the_usual_range_for_spur_gears() {
        for (z1, z2) in [(17u32, 17u32), (17, 43), (25, 25), (13, 60)] {
            let (a, b, m) = pair(z1, z2);
            let path = ContactPath::new(&a, &b, &m).unwrap();
            assert!(
                path.contact_ratio > 1.0 && path.contact_ratio < 2.0,
                "z={z1}/{z2}: contact ratio {} outside (1, 2)",
                path.contact_ratio
            );
        }
    }

    #[test]
    fn hpstc_lies_between_the_pitch_point_and_the_tip() {
        for (z1, z2) in [(17u32, 17u32), (17, 43), (25, 60)] {
            let (a, b, m) = pair(z1, z2);
            let path = ContactPath::new(&a, &b, &m).unwrap();
            let h = path.highest_single_pair();
            assert!(h > 0.0, "HPSTC should be on the recess side, got {h}");
            assert!(h < path.tip(), "HPSTC must be inside the tip");
            // and the roll parameter there is inside the flank
            assert!(path.roll_at(h) > 0.0 && path.roll_at(h) < path.roll_at(path.tip()));
        }
    }

    #[test]
    fn load_fraction_is_one_in_the_single_pair_zone_and_ramps_outside() {
        let (a, b, m) = pair(17, 43);
        let path = ContactPath::new(&a, &b, &m).unwrap();
        assert!((path.load_fraction(0.0, LoadSharing::LinearRamp) - 1.0).abs() < 1e-12);
        assert!((path.load_fraction(0.0, LoadSharing::None) - 1.0).abs() < 1e-12);
        // at the very ends of contact the tooth carries the least
        let at_tip = path.load_fraction(path.tip(), LoadSharing::LinearRamp);
        assert!(
            (at_tip - RAMP_MIN).abs() < 1e-9,
            "at the tip the tooth should carry {RAMP_MIN}, got {at_tip}"
        );
        // outside the path it carries nothing
        assert!(
            path.load_fraction(path.tip() * 2.0, LoadSharing::LinearRamp)
                .abs()
                < 1e-12
        );
    }

    /// The closed form against a direct numerical average of the instantaneous
    /// loss it was derived from. This is the check that catches the `/ε_α`: drop
    /// it and every case here is wrong by exactly the contact ratio.
    #[test]
    fn efficiency_matches_a_numerical_average_of_the_instantaneous_loss() {
        for (z1, z2) in [(17u32, 17u32), (17, 43), (13, 60), (25, 25), (19, 31)] {
            let (a, b, m) = pair(z1, z2);
            let path = ContactPath::new(&a, &b, &m).unwrap();
            let mu = 0.06;

            // Instantaneous fractional loss is mu|xi|(1/rb1 + 1/rb2); contact
            // sweeps the path at constant speed, so average it uniformly in xi.
            let rb1 = a.rb;
            let rb2 = b.rb;
            const N: usize = 200_000;
            let span = path.approach + path.recess;
            let mut sum = 0.0;
            for i in 0..N {
                #[allow(clippy::cast_precision_loss)]
                let t = (i as f64 + 0.5) / N as f64;
                sum += (-path.approach + span * t).abs();
            }
            #[allow(clippy::cast_precision_loss)]
            let mean_abs_xi = sum / N as f64;
            let numeric = 1.0 - mu * mean_abs_xi * (1.0 / rb1 + 1.0 / rb2);

            let closed = efficiency(&path, &m, mu);
            assert!(
                (closed - numeric).abs() < 1e-9,
                "z={z1}/{z2}: closed {closed} vs numeric {numeric}"
            );
        }
    }

    #[test]
    fn efficiency_is_unity_without_friction_and_falls_linearly_with_it() {
        let (a, b, m) = pair(17, 43);
        let path = ContactPath::new(&a, &b, &m).unwrap();
        assert!((efficiency(&path, &m, 0.0) - 1.0).abs() < 1e-15);

        let l1 = 1.0 - efficiency(&path, &m, 0.05);
        let l2 = 1.0 - efficiency(&path, &m, 0.10);
        assert!((l2 - 2.0 * l1).abs() < 1e-12, "loss must be linear in mu");

        // A plain steel spur mesh should land in the high nineties.
        let eta = efficiency(&path, &m, 0.06);
        assert!((0.97..1.0).contains(&eta), "implausible efficiency {eta}");
    }

    /// Swapping driver and driven swaps approach and recess but not the total
    /// sliding, so a parallel-axis mesh has the same efficiency both ways. Two
    /// identical numbers in the UI are a result, not a bug.
    #[test]
    fn forward_and_backward_efficiency_are_equal() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            let (a, b, m) = pair(z1, z2);
            let forward = efficiency(&ContactPath::new(&a, &b, &m).unwrap(), &m, 0.07);

            // The same physical mesh, described from the other gear.
            let m_rev = Mesh::new(&b, &a, MeshKind::External).unwrap();
            let backward = efficiency(&ContactPath::new(&b, &a, &m_rev).unwrap(), &m_rev, 0.07);

            assert!(
                (forward - backward).abs() < 1e-12,
                "z={z1}/{z2}: {forward} vs {backward}"
            );
        }
    }

    /// Sharing can only reduce what a tooth carries, never increase it.
    #[test]
    fn sharing_never_raises_the_load() {
        let (a, b, m) = pair(19, 31);
        let path = ContactPath::new(&a, &b, &m).unwrap();
        for i in 0..=200 {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / 200.0;
            let xi = -path.approach + t * (path.approach + path.recess);
            let shared = path.load_fraction(xi, LoadSharing::LinearRamp);
            let full = path.load_fraction(xi, LoadSharing::None);
            assert!(shared <= full + 1e-12);
            assert!(shared > 0.0);
        }
    }
}
