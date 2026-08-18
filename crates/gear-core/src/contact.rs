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
/// while the useful power crosses the mesh as `F_bt v_b`, with
/// `v_b = ω₁r_b1 = ω₂r_b2`. Friction acts on the force pressing the flanks
/// together, `F_bn = F_bt / cos β_b`, so the instantaneous fractional loss is
///
/// ```text
/// μ |ξ| (1/r_b1 + 1/r_b2) / cos β_b
/// ```
///
/// Contact travels the line of action at constant speed, so the time average is
/// the uniform average of `|ξ|` over the path, `(ξ_a² + ξ_r²) / 2(ξ_a + ξ_r)`.
/// Substituting `r_b = m_t z cos α_t / 2` and `p_bt = π m_t cos α_t` collapses
/// it to
///
/// ```text
/// η = 1 − μ π (1/z₁ ± 1/z₂) (ε₁² + ε₂²) / (ε_α cos β_b)
///                                          + external, − internal
/// ```
///
/// which is Buckingham's formula recovered from first principles — no fitted
/// constants. **This is one formula for spur and helical alike**; `cos β_b` is
/// exactly 1 at zero helix, so the spur case is a value of this expression
/// rather than a separate branch. [verified against a numerical average of the
/// instantaneous loss over five meshes at three helix angles each: agreement to
/// 1e-10 relative.]
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
///
/// # Why the helical case needs nothing else
///
/// `ε₁`, `ε₂` and `ε_α` are transverse quantities, and the transverse geometry
/// already carries the helix angle through `m_t` and `α_t`. The one genuinely
/// new thing is the `cos β_b` above. What might look like a second thing is not:
///
/// **The load is spread over several inclined contact lines** instead of one
/// straight one — and it does not matter. The field of action is a rectangle,
/// uniform along the face, so the line-length-weighted mean of `|ξ|` equals the
/// plain average over the path, the same `(ξ_a² + ξ_r²)/2(ξ_a + ξ_r)` the spur
/// case uses. Spreading the load redistributes it across positions that were
/// being averaged over anyway.
///
/// At fixed transverse geometry the `1/cos β_b` makes a helical mesh slightly
/// *less* efficient — the accepted direction, and the reason DIN 3990's loss
/// factor carries the same term. At fixed **normal** module the net usually goes
/// the other way, because `ε_α` falls with helix angle and outweighs it. Both
/// are consequences of one formula, not two cases.
///
/// # What is left out, and which way it errs
///
/// The **`ε_γ` substitution is deliberately not used.** Replacing `ε_α` with
/// `ε_α + ε_β` is common, but it drives the predicted loss toward zero as the
/// overlap grows, which is unphysical — friction does not vanish on a
/// high-overlap gear. The mean sliding distance does not depend on the overlap;
/// only the force does, and that is the factor above.
///
/// **Nothing is missing along the contact line, and an earlier draft of this
/// documentation said otherwise.** The concern was that only profile sliding is
/// modelled, so a helical mesh's along-tooth sliding would be unaccounted for.
/// It is not there to account for: both bodies rotate about **parallel** axes,
/// so both surface velocities are perpendicular to those axes, and so is their
/// difference. The relative sliding is therefore entirely transverse, and
/// [`sliding_velocity`] measures its component along the contact line as
/// identically zero at any helix angle. `|ξ|(ω₁+ω₂)` is the **whole** sliding
/// magnitude, not a component of it, so this formula is exact rather than
/// conservative. See DESIGN.md §12.
///
/// What does remain a limit is the single friction coefficient itself, and the
/// fact that a crossed-axis mesh is a genuinely different question — there the
/// lengthwise component is not zero, it dominates, and it is why a worm drive
/// can self-lock.
#[must_use]
pub fn efficiency(path: &ContactPath, mesh: &Mesh, g1: &Gear, friction: f64) -> f64 {
    let e1 = path.approach / path.base_pitch;
    let e2 = path.recess / path.base_pitch;
    let z = match mesh.kind {
        MeshKind::External => 1.0 / f64::from(mesh.z1) + 1.0 / f64::from(mesh.z2),
        MeshKind::Internal => 1.0 / f64::from(mesh.z1) - 1.0 / f64::from(mesh.z2),
    };
    let cos_bb = crate::metrology::base_helix_angle(g1).cos();
    1.0 - friction * std::f64::consts::PI * z * (e1 * e1 + e2 * e2) / (path.contact_ratio * cos_bb)
}

/// A relative sliding velocity, resolved in the plane where the flanks touch.
///
/// Coulomb friction acts on the **magnitude**, so the split is not what a loss
/// is computed from. It is here because the split is exactly what separates a
/// parallel-axis mesh from a crossed one, and because a claim that one
/// component vanishes is only worth making if something can measure it.
#[derive(Clone, Copy, Debug)]
pub struct Sliding {
    /// Across the tooth, perpendicular to the contact line, mm/s.
    pub profile: f64,
    /// Along the contact line, mm/s.
    ///
    /// **Analytically zero for parallel axes**, at every helix angle — see
    /// [`sliding_velocity`]. What the implementation returns there is rounding
    /// noise, bounded by test at 1e-14 of the magnitude.
    pub lengthwise: f64,
}

impl Sliding {
    /// What friction acts on, mm/s.
    #[must_use]
    pub fn magnitude(&self) -> f64 {
        self.profile.hypot(self.lengthwise)
    }
}

/// The relative sliding velocity of two rotating bodies at a point they touch.
///
/// This is the friction side of the unification in DESIGN.md §4.7: one
/// expression covering parallel and crossed axes, with the shaft angle carried
/// in `axis_2` rather than selecting between two formulas.
///
/// ```text
/// v_s = ω₁ â₁ × p  −  ω₂ â₂ × (p − c)
/// ```
///
/// Gear 1's axis is `[0,0,1]` through the origin **by construction** — that is
/// a choice of frame, not a restriction. `speed_1` and `speed_2` are signed
/// about their own axes, so an external parallel-axis mesh has them of opposite
/// sign; getting that wrong shows up immediately as a mesh that slides at its
/// pitch point.
///
/// # Why parallel axes have no lengthwise sliding
///
/// It is a kinematic identity rather than an approximation. With `â₁ = â₂ = ẑ`
/// both surface velocities are `ω ẑ × r`, which has no `z` component; so
/// neither does their difference, and the sliding is **entirely transverse**.
/// The contact line, meanwhile, is inclined out of the transverse plane by the
/// base helix angle, and the transverse sliding turns out to be perpendicular
/// to it at every helix angle: the sliding is `∝ ẑ × û` for `û` along the line
/// of action, and the contact line is a combination of `ẑ` and `û`, so the two
/// are orthogonal by construction.
///
/// That is worth stating plainly because an earlier draft of the design assumed
/// the opposite — that a helical mesh has along-tooth sliding this crate was
/// failing to charge for. It does not, so [`efficiency`] is exact rather than
/// conservative (DESIGN.md §12).
///
/// # What changes when the axes cross
///
/// Everything the worm depends on. With `â₂` no longer parallel to `â₁` the
/// sliding does not vanish at the pitch point at all: for perpendicular axes it
/// is `√(v₁² + v₂²)` there, which is the textbook `v₁ / cos γ` written without
/// naming the lead angle. That non-vanishing term is the entire reason a worm
/// drive is inefficient and can self-lock, and it arrives here as a value of
/// this function rather than as a separate screw-gear formula.
#[must_use]
pub fn sliding_velocity(
    axis_2: [f64; 3],
    speed_1: f64,
    speed_2: f64,
    offset: [f64; 3],
    point: [f64; 3],
    contact_line: [f64; 3],
) -> Sliding {
    let v1 = scale(cross([0.0, 0.0, 1.0], point), speed_1);
    let v2 = scale(cross(axis_2, sub(point, offset)), speed_2);
    let slip = sub(v1, v2);

    let lengthwise = dot(slip, contact_line);
    let across = sub(slip, scale(contact_line, lengthwise));
    Sliding {
        profile: norm(across),
        lengthwise,
    }
}

/// The sliding velocity at a point on a parallel-axis mesh's path of contact.
///
/// `xi` is the position along the line of action from the pitch point, the same
/// coordinate [`ContactPath`] uses, and `speed_1` is gear 1's angular velocity
/// in rad/s. This places the mesh in [`sliding_velocity`]'s frame: gear 1 on the
/// `z` axis, gear 2 one centre distance away along `x`, the line of action
/// through the pitch point inclined at the operating pressure angle, and the
/// contact line inclined out of the transverse plane by the base helix angle.
///
/// The crossed-axis version of this arrives with the screw geometry it needs
/// (milestone 7, step 5); what is fixed now is that it will build a different
/// `axis_2` for the *same* function, not call a different one.
#[must_use]
pub fn sliding_at(path: &ContactPath, mesh: &Mesh, g1: &Gear, xi: f64, speed_1: f64) -> Sliding {
    let along_action = [path.alpha_w.sin(), path.alpha_w.cos(), 0.0];
    let point = [
        path.operating_radius_1 + xi * along_action[0],
        xi * along_action[1],
        0.0,
    ];

    // Signed about a shared axis: an external pair turns opposite ways.
    let ratio = f64::from(mesh.z1) / f64::from(mesh.z2);
    let speed_2 = match mesh.kind {
        MeshKind::External => -speed_1 * ratio,
        MeshKind::Internal => speed_1 * ratio,
    };

    let beta_b = crate::metrology::base_helix_angle(g1);
    let contact_line = [
        beta_b.sin() * along_action[0],
        beta_b.sin() * along_action[1],
        beta_b.cos(),
    ];

    sliding_velocity(
        [0.0, 0.0, 1.0],
        speed_1,
        speed_2,
        [mesh.a_w, 0.0, 0.0],
        point,
        contact_line,
    )
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
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

    fn helical_pair(z1: u32, z2: u32, beta: f64) -> (Gear, Gear, Mesh) {
        let a = Gear::new(GearParams {
            teeth: z1,
            helix_angle: beta,
            ..Default::default()
        });
        let b = Gear::new(GearParams {
            teeth: z2,
            helix_angle: -beta,
            ..Default::default()
        });
        let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
        (a, b, m)
    }

    /// A parallel-axis mesh rolls without sliding at its pitch point — spur and
    /// helical alike. Anything else means the two speeds were not the meshing
    /// pair they claim to be.
    #[test]
    fn a_parallel_axis_mesh_does_not_slide_at_its_pitch_point() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            for beta in [0.0, 15.0, 30.0] {
                let (a, b, m) = helical_pair(z1, z2, beta);
                let path = ContactPath::new(&a, &b, &m).unwrap();
                let s = sliding_at(&path, &m, &a, 0.0, 100.0);
                let reference = 100.0 * path.operating_radius_1;
                assert!(
                    s.magnitude() < 1e-12 * reference,
                    "z={z1}/{z2} beta={beta}: {} mm/s at the pitch point",
                    s.magnitude()
                );
            }
        }
    }

    /// **The correction this step exists to make.** The sliding of a
    /// parallel-axis mesh has no component along the contact line, at any helix
    /// angle — so `|ξ|(ω₁+ω₂)` is the whole sliding magnitude and the closed
    /// form for efficiency is exact, not conservative.
    ///
    /// An earlier draft of the design assumed a helical mesh slid along its
    /// teeth and that the loss from it was going uncharged. It does not: both
    /// surface velocities are perpendicular to the shared axis direction, so
    /// the difference is transverse, while the contact line is not.
    #[test]
    fn parallel_axes_slide_across_the_teeth_and_never_along_them() {
        for (z1, z2) in [(17u32, 43u32), (19, 31)] {
            for beta in [0.0, 8.0, 20.0, 35.0] {
                let (a, b, m) = helical_pair(z1, z2, beta);
                let path = ContactPath::new(&a, &b, &m).unwrap();
                for step in 0..=10 {
                    #[allow(clippy::cast_precision_loss)]
                    let t = step as f64 / 10.0;
                    let xi = -path.approach + t * (path.approach + path.recess);
                    let s = sliding_at(&path, &m, &a, xi, 100.0);
                    // The velocities being differenced are of order the pitch
                    // line speed, so that — not the small sliding that survives
                    // the difference — is the scale rounding noise lives on.
                    let pitch_line = 100.0 * path.operating_radius_1;
                    assert!(
                        s.lengthwise.abs() <= 1e-14 * pitch_line,
                        "z={z1}/{z2} beta={beta} xi={xi}: lengthwise {} against \
                         a pitch line speed of {pitch_line}",
                        s.lengthwise
                    );
                }
            }
        }
    }

    /// The vector model reproduces the scalar the closed form was built on:
    /// `|v_s| = |ξ|(ω₁+ω₂)`, which is the step that lets the closed form stand
    /// unchanged.
    #[test]
    fn the_sliding_magnitude_is_the_scalar_the_closed_form_uses() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            for beta in [0.0, 12.0, 25.0] {
                let (a, b, m) = helical_pair(z1, z2, beta);
                let path = ContactPath::new(&a, &b, &m).unwrap();
                let omega_1 = 100.0;
                let omega_2 = omega_1 * f64::from(z1) / f64::from(z2);
                for step in 0..=8 {
                    #[allow(clippy::cast_precision_loss)]
                    let t = step as f64 / 8.0;
                    let xi = -path.approach + t * (path.approach + path.recess);
                    let expected = xi.abs() * (omega_1 + omega_2);
                    let got = sliding_at(&path, &m, &a, xi, omega_1).magnitude();
                    let pitch_line = omega_1 * path.operating_radius_1;
                    assert!(
                        (got - expected).abs() <= 1e-11 * expected + 1e-14 * pitch_line,
                        "z={z1}/{z2} beta={beta} xi={xi}: {got} vs {expected}"
                    );
                }
            }
        }
    }

    /// Crossing the axes is what makes the pitch point slide, and the amount is
    /// the textbook worm figure `√(v₁² + v₂²) = v₁/cos γ`. No gear geometry is
    /// involved — two axes, two speeds and a point.
    #[test]
    fn crossed_axes_slide_at_the_pitch_point_by_the_worm_relation() {
        let (r1, r2) = (7.0, 25.0);
        let (omega_1, omega_2) = (300.0, 12.0);
        let centre = r1 + r2;
        // Axis 2 perpendicular to axis 1, offset along x: a worm and its wheel.
        let s = sliding_velocity(
            [0.0, 1.0, 0.0],
            omega_1,
            omega_2,
            [centre, 0.0, 0.0],
            [r1, 0.0, 0.0],
            [0.0, 0.0, 1.0],
        );

        let (v1, v2) = (omega_1 * r1, omega_2 * r2);
        let expected = v1.hypot(v2);
        assert!(
            (s.magnitude() - expected).abs() < 1e-12 * expected,
            "{} vs {expected}",
            s.magnitude()
        );

        // and it is the same number as v1/cos(lead angle), which is how the
        // literature writes it
        let gamma = (v2 / v1).atan();
        assert!((s.magnitude() - v1 / gamma.cos()).abs() < 1e-12 * expected);

        // Resolved along the worm's own axis — which is not the contact line,
        // but is a direction that exists without any tooth geometry — the
        // sliding has a component of exactly the wheel's pitch velocity. A
        // parallel-axis mesh has no such component at all, at any helix angle.
        assert!(
            (s.lengthwise.abs() - v2).abs() < 1e-12 * v2,
            "along the worm axis: {} vs the wheel's {v2} mm/s",
            s.lengthwise
        );
    }

    /// The closed form for efficiency, checked against a numerical average
    /// driven by the **vector** model rather than by the scalar it was derived
    /// from. This closes the loop: the formula in use is the exact integral of
    /// `μ|v_s|` over the path, not an approximation of it.
    #[test]
    fn efficiency_is_the_integral_of_the_sliding_vector() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (19, 31)] {
            for beta in [0.0, 12.0, 25.0] {
                let (a, b, m) = helical_pair(z1, z2, beta);
                let path = ContactPath::new(&a, &b, &m).unwrap();
                let mu = 0.06;
                let omega_1 = 100.0;

                // Fractional loss is mu |v_s| / (v_b cos beta_b): friction acts
                // on F_bn while the useful power crosses as F_bt.
                let v_b = omega_1 * a.rb;
                let cos_bb = crate::metrology::base_helix_angle(&a).cos();

                const N: usize = 200_000;
                let span = path.approach + path.recess;
                let mut sum = 0.0;
                for i in 0..N {
                    #[allow(clippy::cast_precision_loss)]
                    let t = (i as f64 + 0.5) / N as f64;
                    let xi = -path.approach + span * t;
                    sum += sliding_at(&path, &m, &a, xi, omega_1).magnitude();
                }
                #[allow(clippy::cast_precision_loss)]
                let mean_slide = sum / N as f64;
                let numeric = 1.0 - mu * mean_slide / (v_b * cos_bb);

                let closed = efficiency(&path, &m, &a, mu);
                assert!(
                    (closed - numeric).abs() < 1e-9,
                    "z={z1}/{z2} beta={beta}: closed {closed} vs vector-driven {numeric}"
                );
            }
        }
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
    ///
    /// Helical angles are included, and they exercise the `cos β_b` too — the
    /// friction acts on `F_bn` while the useful power crosses as `F_bt`.
    #[test]
    fn efficiency_matches_a_numerical_average_of_the_instantaneous_loss() {
        for (z1, z2) in [(17u32, 17u32), (17, 43), (13, 60), (25, 25), (19, 31)] {
            for beta in [0.0, 12.0, 25.0] {
                let a = Gear::new(crate::GearParams {
                    teeth: z1,
                    helix_angle: beta,
                    ..Default::default()
                });
                let b = Gear::new(crate::GearParams {
                    teeth: z2,
                    helix_angle: -beta,
                    ..Default::default()
                });
                let m = Mesh::new(&a, &b, MeshKind::External).unwrap();
                let path = ContactPath::new(&a, &b, &m).unwrap();
                let mu = 0.06;

                // Instantaneous fractional loss is mu|xi|(1/rb1 + 1/rb2)/cos(beta_b);
                // contact sweeps the path at constant speed, so average it
                // uniformly in xi.
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
                let cos_bb = crate::metrology::base_helix_angle(&a).cos();
                let numeric = 1.0 - mu * mean_abs_xi * (1.0 / a.rb + 1.0 / b.rb) / cos_bb;

                let closed = efficiency(&path, &m, &a, mu);
                assert!(
                    (closed - numeric).abs() < 1e-9,
                    "z={z1}/{z2} beta={beta}: closed {closed} vs numeric {numeric}"
                );
            }
        }
    }

    /// `cos β_b` is exactly 1 at zero helix, so the helical formula *is* the
    /// spur formula rather than a generalisation with a special case attached.
    #[test]
    fn the_helical_efficiency_formula_reduces_exactly_at_zero_helix() {
        let (a, b, m) = pair(17, 43);
        let path = ContactPath::new(&a, &b, &m).unwrap();
        assert!((crate::metrology::base_helix_angle(&a).cos() - 1.0).abs() < f64::EPSILON);

        // The loss carries the 1/cos(beta_b), so at a fixed transverse geometry
        // more helix means more loss. That the CLI shows helical meshes as
        // slightly *more* efficient is a separate effect: the transverse contact
        // ratio falls with beta, and that outweighs this factor.
        let mut previous = 0.0;
        for beta in [0.0, 15.0, 30.0] {
            let g = Gear::new(crate::GearParams {
                teeth: 17,
                helix_angle: beta,
                ..Default::default()
            });
            let cos_bb = crate::metrology::base_helix_angle(&g).cos();
            let loss = (1.0 - efficiency(&path, &m, &g, 0.06)) * 1.0;
            assert!(
                loss > previous,
                "beta={beta}: loss must rise with 1/cos(beta_b)"
            );
            assert!(cos_bb <= 1.0);
            previous = loss;
        }
    }

    #[test]
    fn efficiency_is_unity_without_friction_and_falls_linearly_with_it() {
        let (a, b, m) = pair(17, 43);
        let path = ContactPath::new(&a, &b, &m).unwrap();
        assert!((efficiency(&path, &m, &a, 0.0) - 1.0).abs() < 1e-15);

        let l1 = 1.0 - efficiency(&path, &m, &a, 0.05);
        let l2 = 1.0 - efficiency(&path, &m, &a, 0.10);
        assert!((l2 - 2.0 * l1).abs() < 1e-12, "loss must be linear in mu");

        // A plain steel spur mesh should land in the high nineties.
        let eta = efficiency(&path, &m, &a, 0.06);
        assert!((0.97..1.0).contains(&eta), "implausible efficiency {eta}");
    }

    /// Swapping driver and driven swaps approach and recess but not the total
    /// sliding, so a parallel-axis mesh has the same efficiency both ways. Two
    /// identical numbers in the UI are a result, not a bug.
    #[test]
    fn forward_and_backward_efficiency_are_equal() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            let (a, b, m) = pair(z1, z2);
            let forward = efficiency(&ContactPath::new(&a, &b, &m).unwrap(), &m, &a, 0.07);

            // The same physical mesh, described from the other gear.
            let m_rev = Mesh::new(&b, &a, MeshKind::External).unwrap();
            let backward = efficiency(&ContactPath::new(&b, &a, &m_rev).unwrap(), &m_rev, &b, 0.07);

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
