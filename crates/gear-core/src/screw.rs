//! Crossed-axis screw gearing — the worm drive and the crossed-helical pair.
//!
//! A worm drive is not a separate kind of gearing. It is a screw pair with few
//! starts, a small lead angle and (usually) a throated wheel, and the same
//! mathematics covers a crossed-helical pair with none of those properties
//! (DESIGN.md §4.5.1). So there is one module, and the worm stage and a
//! crossed-axis spur stage will both use it.
//!
//! Everything here is evaluated **at the pitch point**, where the two pitch
//! cylinders touch. That is enough for the lead angle, the ratio, the sliding
//! and the efficiency, and it is where the classical screw-gear results live.
//!
//! # The frame
//!
//! One right-handed frame carries the whole derivation. At the pitch point:
//!
//! ```text
//! x̂   the common normal to the two pitch cylinders — along the centre line
//! ŷ   member 1's surface velocity direction
//! ẑ   member 1's axis direction
//! ```
//!
//! `ŷ` and `ẑ` span the common tangent plane, and member 2's axis lies in that
//! plane, turned from `ẑ` by the shaft angle `Σ`. Two directions in it do all
//! the work: `û₂`, member 2's surface velocity direction, and `ĥ`, the tooth
//! trace both members share.
//!
//! # Lead angle — exact, and the place people iterate
//!
//! `tan γ = L/(π d)` with `L = z p_x = z π m_x`, and `m_x = m_n / cos γ` — which
//! depends on `γ`, so the obvious reading is a fixed point to be iterated.
//! Substituting once removes it:
//!
//! ```text
//! sin γ = z m_n / d
//! ```
//!
//! No solve, and it holds for **both** members with their own `z` and `d`, which
//! is what makes the wheel's diameter fall out rather than needing the axial
//! module as an intermediate.
//!
//! # Where the shaft angle enters
//!
//! Helix angles add: `Σ = β₁ + β₂` for a same-hand pair, and `β = 90° − γ`, so
//! `γ₁ + γ₂ = 180° − Σ`. The velocity ratio at the pitch point follows from
//! requiring the two tooth traces to coincide there, which forces the sliding to
//! lie **along** the trace:
//!
//! ```text
//! v₂/v₁ = sin γ₁ / sin γ₂
//! ```
//!
//! and the transmission ratio `z₂/z₁` then falls out of `sin γ = z m_n/d` on both
//! members, rather than being imposed.
//!
//! # Why parallel axes are not a case of this
//!
//! At `Σ = 0` the sliding at the pitch point is zero — two pitch cylinders with
//! parallel axes roll on each other — so the friction direction is undefined and
//! the efficiency below is `0/0`. That is not a discontinuity to paper over: a
//! parallel-axis mesh genuinely has no sliding at its pitch point, and its whole
//! loss comes from sliding **along the path of contact**, which is
//! [`crate::contact::efficiency`]'s integral. The two are different physical
//! regimes rather than two branches of one formula, and [`Screw::new`] refuses
//! `Σ = 0` for that reason. What *is* unified across both — and is the point of
//! DESIGN §4.7 — is the sliding vector, the Hertzian contact, and the geometry;
//! not this pitch-point shortcut.

use crate::contact::{cross, dot, norm, scale, sub, Drive};
use crate::mesh::Member;

/// What a screw pair is made of.
///
/// Angles are radians here, as everywhere inside the crate; degrees are a UI
/// boundary concern.
#[derive(Clone, Copy, Debug)]
pub struct ScrewParams {
    /// Normal module, mm. Shared by both members.
    pub normal_module: f64,
    /// Normal pressure angle, radians. Shared.
    pub normal_pressure_angle: f64,
    /// Shaft angle `Σ`, radians. 90° is the ordinary worm drive.
    pub shaft_angle: f64,
    /// Starts on the worm, `z₁`.
    pub starts: u32,
    /// Teeth on the wheel, `z₂`.
    pub wheel_teeth: u32,
    /// The worm's pitch diameter, mm. This is a free input — it is what sets
    /// the lead angle, and with it the efficiency and whether the drive can be
    /// back-driven at all.
    pub worm_pitch_diameter: f64,
}

impl Default for ScrewParams {
    fn default() -> Self {
        Self {
            normal_module: 1.0,
            normal_pressure_angle: 20.0_f64.to_radians(),
            shaft_angle: std::f64::consts::FRAC_PI_2,
            starts: 1,
            wheel_teeth: 17,
            worm_pitch_diameter: 7.0,
        }
    }
}

/// A screw pair's geometry at its pitch point. All closed form.
#[derive(Clone, Copy, Debug)]
pub struct Screw {
    /// Worm lead angle `γ₁`, radians — from the transverse plane.
    pub lead_angle: f64,
    /// Wheel lead angle `γ₂`, radians. `γ₁ + γ₂ = 180° − Σ`.
    pub wheel_lead_angle: f64,
    /// Worm helix angle `β₁ = 90° − γ₁`, radians — from the axis.
    pub worm_helix_angle: f64,
    /// Wheel helix angle `β₂ = Σ − β₁`, radians. The specification lists this as
    /// an output "calculated from worm helix angle + axis angle", which is what
    /// `Σ = β₁ + β₂` says.
    pub wheel_helix_angle: f64,
    /// Lead — how far a point on the thread advances per worm revolution, mm.
    pub lead: f64,
    /// Axial module of the worm, `m_x = m_n / cos γ₁`, mm.
    pub axial_module: f64,
    pub worm_pitch_diameter: f64,
    pub wheel_pitch_diameter: f64,
    pub centre_distance: f64,
    /// `z₂/z₁`, which is also `ω₁/ω₂`.
    pub ratio: f64,
    /// Sliding speed at the pitch point, as a multiple of the worm's own pitch
    /// line speed. `1/cos γ₁` for the ordinary 90° drive.
    pub sliding_ratio: f64,
    /// Normal pressure angle, carried through because the force balance needs
    /// it and nothing else here does.
    pub normal_pressure_angle: f64,
    /// Shaft angle `Σ`, radians.
    pub shaft_angle: f64,
}

/// Why a screw pair could not be built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScrewError {
    /// A module, diameter or tooth count was not positive.
    NotPositive,
    /// `z₁ m_n ≥ d₁`: the thread would have to wrap at 90° or more. The worm is
    /// too thin for that many starts at that module.
    WormTooThin,
    /// The shaft angle leaves the wheel with no usable lead angle.
    ShaftAngleImpossible,
    /// Parallel axes. There is no sliding at the pitch point, so this model has
    /// nothing to say; see the module documentation.
    AxesAreParallel,
    /// The first member's helix angle reaches 90°: its teeth would run
    /// circumferentially and its pitch diameter is unbounded. That is a disc, not
    /// a gear.
    ///
    /// The opposite end of [`Self::WormTooThin`], and it has to be caught where
    /// the *helix angle* is still known: by the time a diameter has been derived
    /// from it, `cos 90°` is 6e-17 rather than zero, so the diameter is merely
    /// enormous — 2.8e17 mm on a 17-tooth member — and every check downstream
    /// finds it perfectly finite.
    FirstMemberIsADisc,
}

impl Screw {
    /// Build the pitch-point geometry.
    ///
    /// # Errors
    ///
    /// [`ScrewError`] when the inputs describe something that cannot exist.
    pub fn new(p: &ScrewParams) -> Result<Self, ScrewError> {
        // Predicates rather than negated comparisons, so a NaN is refused here
        // instead of propagating into a lead angle.
        let positive = |v: f64| v.is_finite() && v > 0.0;
        if !positive(p.normal_module)
            || !positive(p.worm_pitch_diameter)
            || p.starts == 0
            || p.wheel_teeth == 0
            || !p.normal_pressure_angle.is_finite()
            || !p.shaft_angle.is_finite()
        {
            return Err(ScrewError::NotPositive);
        }

        // sin γ = z m_n / d — exact, no iteration. The bound is what "the thread
        // cannot wrap past 90°" means numerically.
        let sin_gamma_1 = f64::from(p.starts) * p.normal_module / p.worm_pitch_diameter;
        if sin_gamma_1 >= 1.0 {
            return Err(ScrewError::WormTooThin);
        }
        let lead_angle = sin_gamma_1.asin();

        let worm_helix_angle = std::f64::consts::FRAC_PI_2 - lead_angle;
        let wheel_helix_angle = p.shaft_angle - worm_helix_angle;
        let wheel_lead_angle = std::f64::consts::FRAC_PI_2 - wheel_helix_angle;
        let sin_gamma_2 = wheel_lead_angle.sin();
        if sin_gamma_2 <= 0.0 {
            return Err(ScrewError::ShaftAngleImpossible);
        }

        // The same law on the wheel, with its own tooth count.
        let wheel_pitch_diameter = f64::from(p.wheel_teeth) * p.normal_module / sin_gamma_2;

        // Sliding at the pitch point, per unit of the worm's pitch line speed.
        // v_s = (1 − k cos Σ) ŷ + k sin Σ ẑ with k = v₂/v₁ = sin γ₁ / sin γ₂.
        let k = sin_gamma_1 / sin_gamma_2;
        let sliding_ratio = (1.0 - 2.0 * k * p.shaft_angle.cos() + k * k).sqrt();
        if sliding_ratio <= 0.0 {
            return Err(ScrewError::AxesAreParallel);
        }

        let axial_module = p.normal_module / lead_angle.cos();
        Ok(Self {
            lead_angle,
            wheel_lead_angle,
            worm_helix_angle,
            wheel_helix_angle,
            lead: f64::from(p.starts) * std::f64::consts::PI * axial_module,
            axial_module,
            worm_pitch_diameter: p.worm_pitch_diameter,
            wheel_pitch_diameter,
            centre_distance: 0.5 * (p.worm_pitch_diameter + wheel_pitch_diameter),
            ratio: f64::from(p.wheel_teeth) / f64::from(p.starts),
            sliding_ratio,
            normal_pressure_angle: p.normal_pressure_angle,
            shaft_angle: p.shaft_angle,
        })
    }

    /// `v₂/v₁` at the pitch point — the surface speeds, not the shaft speeds.
    #[must_use]
    pub fn velocity_ratio(&self) -> f64 {
        self.lead_angle.sin() / self.wheel_lead_angle.sin()
    }

    /// Mesh efficiency, both drive directions, from a force balance at the
    /// pitch point.
    ///
    /// # Derived, not quoted
    ///
    /// With the normal force taken as 1, the force member 1 puts on member 2 is
    /// `n̂ + μ v̂_s`: a normal component along the flank normal, and friction
    /// along the sliding direction, which is the direction member 1's surface
    /// moves relative to member 2's. Projecting that onto each member's own
    /// velocity direction gives the two tangential forces, and
    ///
    /// ```text
    /// η = P_out/P_in = v₂ (F·û₂) / v₁ (F·ŷ)
    /// ```
    ///
    /// The flank normal is `n̂ = cos α_n p̂ + sin α_n x̂`, where `p̂` is the
    /// in-plane direction perpendicular to the tooth trace. **The `x̂` part never
    /// survives**: both `ŷ` and `û₂` lie in the tangent plane, so the separating
    /// force does no work and drops out of both projections — which is why the
    /// pressure angle enters only as `cos α_n`.
    ///
    /// At `Σ = 90°` this reproduces the classical screw-gear pair
    ///
    /// ```text
    /// η_worm_driving  = (cos α_n − μ tan γ) / (cos α_n + μ cot γ)
    /// η_wheel_driving = (cos α_n − μ cot γ) / (cos α_n + μ tan γ)
    /// ```
    ///
    /// [verified to 1e-14 against both], but it is not restricted to 90° — the
    /// shaft angle is carried in `û₂` and in the sliding direction.
    ///
    /// # Why the two directions differ, when no parallel-axis mesh does
    ///
    /// Reversing the drive puts the load on the other flank, which flips the
    /// normal force's in-plane component — but **not** the sliding direction,
    /// which is fixed by the rotation senses. So friction that had been resisting
    /// the motion now sits on the other side of the balance. A parallel-axis
    /// mesh has no such asymmetry to flip: its sliding reverses across the pitch
    /// point and averages out, which is why §4.5 gets two identical numbers.
    ///
    /// Self-locking is the same statement carried to its end: when friction alone
    /// exceeds what the flank can push back with, the numerator of the backward
    /// case changes sign and `η_wheel_driving ≤ 0`.
    #[must_use]
    pub fn efficiency(&self, friction: f64, drive: Drive) -> f64 {
        let flank = match drive {
            Drive::Forward => Flank::Driving,
            Drive::Backward => Flank::BackDriving,
        };
        let (on_1, on_2) = self.tangential_per_normal(friction, flank);
        let k = self.velocity_ratio();
        match drive {
            Drive::Forward => k * on_2 / on_1,
            Drive::Backward => on_1 / (k * on_2),
        }
    }

    /// The coefficient of friction at which the drive stops being back-driveable
    /// — `cos α_n tan γ` for a right-angle worm.
    ///
    /// **Reported rather than compared against silently**, because it is the
    /// number a designer actually wants: a worm sized just past it is relying on
    /// a friction coefficient nobody measured.
    #[must_use]
    pub fn self_locking_friction(&self) -> f64 {
        self.normal_pressure_angle.cos() * self.lead_angle.sin() / self.slide_on(1)
    }

    /// The sliding direction resolved on member `which`'s velocity direction.
    fn slide_on(&self, which: u8) -> f64 {
        let k = self.velocity_ratio();
        let cos_sigma = self.shaft_angle.cos();
        if which == 1 {
            (1.0 - k * cos_sigma) / self.sliding_ratio
        } else {
            (cos_sigma - k) / self.sliding_ratio
        }
    }

    /// Each member's tangential force per unit normal force, from the balance
    /// described on [`Self::efficiency`]. Back-driving loads the other flank,
    /// which flips the normal term and leaves the friction term alone.
    fn tangential_per_normal(&self, friction: f64, flank: Flank) -> (f64, f64) {
        let sign = match flank {
            Flank::Driving => 1.0,
            Flank::BackDriving => -1.0,
        };
        let cos_alpha = self.normal_pressure_angle.cos();
        (
            sign * cos_alpha * self.lead_angle.sin() + friction * self.slide_on(1),
            sign * cos_alpha * self.wheel_lead_angle.sin() + friction * self.slide_on(2),
        )
    }

    /// The force pressing the flanks together, N, for a torque quoted on one
    /// member.
    ///
    /// `torque` is in Nm, as everywhere at this crate's boundary, and `on` says
    /// which member it is the torque *of* — [`Member::First`] the worm,
    /// [`Member::Second`] the wheel. This is the **loaded** normal force, from
    /// the same balance the efficiency comes from, so it already carries the
    /// friction that a frictionless projection would miss. In a worm mesh that
    /// is not a small correction.
    ///
    /// # Which member the torque is quoted on changes the answer's *direction*
    ///
    /// Hold the **input** torque fixed and more friction gives a **lower** flank
    /// load: friction itself carries part of the tangential force, so less
    /// normal force is needed to balance the same input. Hold the **output**
    /// torque fixed — what the wheel must actually deliver — and more friction
    /// gives a **higher** one, because the useful part of the flank force has
    /// been eaten into and the flank must be pressed harder to make it up.
    ///
    /// Both are correct answers to different questions, and it is an easy one to
    /// get backwards. A rating wants the second: size the contact on the
    /// **wheel** torque, which is the conservative direction and the one worm
    /// gearing is conventionally rated on.
    #[must_use]
    pub fn normal_force(&self, torque: f64, on: Member, friction: f64) -> f64 {
        let (per_normal_1, per_normal_2) = self.tangential_per_normal(friction, Flank::Driving);
        match on {
            Member::First => 2000.0 * torque / self.worm_pitch_diameter / per_normal_1,
            Member::Second => 2000.0 * torque / self.wheel_pitch_diameter / per_normal_2.abs(),
        }
    }

    /// The relative principal curvatures at the pitch point, 1/mm.
    ///
    /// See [`pitch_point_curvatures`], which this hands its own geometry to.
    #[must_use]
    pub fn contact_curvatures(&self) -> Option<(f64, f64)> {
        pitch_point_curvatures(
            self.shaft_angle,
            self.normal_pressure_angle,
            self.worm_helix_angle,
            self.worm_pitch_diameter / 2.0,
            self.wheel_pitch_diameter / 2.0,
        )
    }

    /// The Hertzian contact patch at the pitch point, for a torque on the worm.
    ///
    /// `e_star` is the effective contact modulus, from
    /// [`crate::material::contact_modulus`]. This is the strength figure a worm
    /// stage reports: **there is deliberately no bending stress**, and DESIGN
    /// §4.5.1 says why at length.
    #[must_use]
    pub fn contact(
        &self,
        torque: f64,
        on: Member,
        friction: f64,
        e_star: f64,
    ) -> Option<crate::hertz::EllipticalContact> {
        let (flat, sharp) = self.contact_curvatures()?;
        crate::hertz::elliptical_contact(
            flat,
            sharp,
            self.normal_force(torque, on, friction),
            e_star,
        )
    }
}

/// Which flank carries the load — that is, which way the drive is being pushed.
#[derive(Clone, Copy, Debug)]
enum Flank {
    Driving,
    BackDriving,
}

/// The two relative principal curvatures at a crossed pair's pitch point, 1/mm,
/// flatter one first — ready for [`crate::hertz::elliptical_contact`].
///
/// Written as a free function rather than a method so that it can be asked for
/// the **parallel** case too, `Σ = 0`, which [`Screw::new`] refuses. That case
/// is not idle: it is the check that this construction agrees with the
/// line-contact relative curvature `strength` has been using all along.
///
/// # Each flank is a cylinder, and which way it points is the whole question
///
/// An involute helicoid is a **developable** surface — the tangent surface of
/// its base helix — so one of its principal curvatures is exactly zero, along
/// the straight ruling that lies on the flank. Each flank is therefore locally a
/// cylinder: curvature `1/ρ_n` across the tooth, nothing along the ruling.
///
/// So point contact is not a different mechanism from line contact. It is two
/// cylinders whose axes cross. For parallel axes the two rulings **coincide** —
/// that is what a contact line is — and the relative curvature in that direction
/// is exactly zero. Cross the shafts and the rulings separate, the flatter
/// relative curvature lifts off zero, and the line shortens into an ellipse.
///
/// # Finding the rulings without trigonometric bookkeeping
///
/// The ruling of member `i` lies in the common tangent plane (it is on the
/// flank) and makes the base helix angle with that member's own axis. Both
/// conditions are met by one construction: **project the axis onto the tangent
/// plane and normalise**. That the projection's length then comes out as
/// `cos β_b` is not arranged, it is a consequence — `â·n̂ = ± sin β_b` falls out
/// of the frame — and a test asserts it rather than the code assuming it.
///
/// The skew between the two rulings is then just the angle between two vectors,
/// and at `Σ = 0` they are the *same* vector, so the skew is zero to the bit and
/// the flat curvature comes back exactly zero.
///
/// # What this assumes about the worm
///
/// That both members are involute helicoids — the **ZI** worm, which is exactly
/// a helical gear with few teeth and a large helix angle. That is the type
/// §4.5.1's "worm and crossed-helical are the same mathematics" claim is true
/// of, and the only one consistent with a crate that builds everything from the
/// involute. A ZA worm (straight-sided in the axial section) or a ZN has a
/// different flank form and would need its own curvature, which is a separate
/// piece of work rather than a factor on this one.
///
/// # Errors
///
/// `None` if a radius or angle is not usable, or if the flanks turn out not to
/// touch at a point.
#[must_use]
pub fn pitch_point_curvatures(
    shaft_angle: f64,
    normal_pressure_angle: f64,
    helix_angle_1: f64,
    pitch_radius_1: f64,
    pitch_radius_2: f64,
) -> Option<(f64, f64)> {
    let usable = |v: f64| v.is_finite() && v > 0.0;
    if !usable(pitch_radius_1) || !usable(pitch_radius_2) || !normal_pressure_angle.is_finite() {
        return None;
    }

    let beta_1 = helix_angle_1;
    let beta_2 = shaft_angle - beta_1;
    let (rho_n_1, beta_b_1) = flank_curvature(pitch_radius_1, beta_1, normal_pressure_angle)?;
    let (rho_n_2, beta_b_2) = flank_curvature(pitch_radius_2, beta_2, normal_pressure_angle)?;
    let _ = (beta_b_1, beta_b_2);

    let (ruling_1, ruling_2) = rulings(shaft_angle, normal_pressure_angle, beta_1)?;

    // The angle between two directions. `atan2` of the cross and dot rather than
    // an `acos`, which loses its precision exactly where the rulings are nearly
    // parallel — the case this whole model is built around.
    let skew = norm(cross(ruling_1, ruling_2)).atan2(dot(ruling_1, ruling_2));

    crate::hertz::relative_curvatures((1.0 / rho_n_1, 0.0), (1.0 / rho_n_2, 0.0), skew)
}

/// The two flanks' rulings at the pitch point — their zero-curvature directions.
///
/// The frame is the one the module documentation sets out: `x̂` along the centre
/// line, `ŷ` member 1's surface velocity, `ẑ` its axis. `p̂` is the in-plane
/// direction across the tooth and `n̂ = cos α_n p̂ + sin α_n x̂` the common flank
/// normal.
///
/// Each ruling is that member's axis with its `n̂` component removed. Nothing
/// here asserts the base helix angle; that it comes out is the check.
fn rulings(shaft_angle: f64, alpha_n: f64, beta_1: f64) -> Option<([f64; 3], [f64; 3])> {
    let (sin_b1, cos_b1) = beta_1.sin_cos();
    let p_hat = [0.0, cos_b1, -sin_b1];
    let (sin_a, cos_a) = alpha_n.sin_cos();
    let n_hat = [sin_a, cos_a * p_hat[1], cos_a * p_hat[2]];

    let (sin_sigma, cos_sigma) = shaft_angle.sin_cos();
    Some((
        project_out([0.0, 0.0, 1.0], n_hat)?,
        project_out([0.0, sin_sigma, cos_sigma], n_hat)?,
    ))
}

/// A flank's radius of curvature across the tooth at the pitch point, and its
/// base helix angle.
///
/// `ρ_n = ρ_t / cos β_b` with `ρ_t = r sin α_t` — the transverse profile radius
/// seen in the normal plane. This is the same relation [`crate::strength`] uses
/// for line contact, deliberately: a crossed mesh at zero shaft angle has to
/// return the number the parallel path already returns, and it can only do that
/// if both are measuring the same curvature.
fn flank_curvature(radius: f64, helix: f64, alpha_n: f64) -> Option<(f64, f64)> {
    let cos_beta = helix.cos();
    if cos_beta == 0.0 {
        return None;
    }
    let alpha_t = (alpha_n.tan() / cos_beta).atan();
    let beta_b = (helix.sin() * alpha_n.cos()).asin();
    let rho_t = radius * alpha_t.sin();
    let rho_n = rho_t / beta_b.cos();
    if !(rho_n.is_finite() && rho_n > 0.0) {
        return None;
    }
    Some((rho_n, beta_b))
}

/// The part of `v` that survives projection onto the plane normal to `n`,
/// normalised.
fn project_out(v: [f64; 3], n: [f64; 3]) -> Option<[f64; 3]> {
    let in_plane = sub(v, scale(n, dot(v, n)));
    let length = norm(in_plane);
    // A zero-length projection means the axis is along the flank normal, which
    // is not a surface this model has anything to say about.
    if length.is_nan() || length <= 0.0 {
        return None;
    }
    Some(scale(in_plane, 1.0 / length))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::contact::sliding_velocity;
    use crate::contact::Directional;
    use std::f64::consts::PI;

    fn worm(starts: u32, teeth: u32, d1: f64) -> Screw {
        Screw::new(&ScrewParams {
            starts,
            wheel_teeth: teeth,
            worm_pitch_diameter: d1,
            ..Default::default()
        })
        .unwrap()
    }

    /// **The check that ties the crossed-axis construction to the verified
    /// parallel one.** At zero shaft angle the two flanks' rulings coincide, so
    /// the flatter relative curvature is exactly zero and the sharper one must
    /// be the very curvature `strength` has been raising to a stress all along.
    ///
    /// Rather than compare curvatures — which would only be comparing two
    /// spellings of the same algebra — this takes the number the whole way to a
    /// contact stress and puts it against the verified line-contact path.
    #[test]
    fn parallel_axes_return_the_line_contact_the_verified_path_already_gives() {
        use crate::contact::ContactPath;
        use crate::mesh::{Mesh, MeshKind};
        use crate::profile::Gear;
        use crate::strength::{contact_stress, Load, PARALLEL_AXES};
        use crate::GearParams;

        for (z1, z2, beta_deg) in [(17u32, 43u32, 0.0), (17, 43, 20.0), (13, 60, 30.0)] {
            let g1 = Gear::new(GearParams {
                teeth: z1,
                helix_angle: beta_deg,
                ..Default::default()
            });
            let g2 = Gear::new(GearParams {
                teeth: z2,
                helix_angle: -beta_deg,
                ..Default::default()
            });
            let mesh = Mesh::new(&g1, &g2, MeshKind::External).unwrap();
            let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
            let load = Load::new(2.0, 10.0);
            let e_star = 113_000.0;
            let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star).unwrap();

            let sum_z = f64::from(mesh.z1) + f64::from(mesh.z2);
            let r1 = mesh.a_w * f64::from(mesh.z1) / sum_z;
            let r2 = mesh.a_w * f64::from(mesh.z2) / sum_z;
            let (flat, sharp) = pitch_point_curvatures(0.0, g1.alpha_n, g1.beta, r1, r2).unwrap();

            assert_eq!(
                flat, 0.0,
                "z={z1}/{z2} beta={beta_deg}: parallel axes must be exactly line contact"
            );

            let f_prime = load.transverse_line_of_action(&g1) / load.face_width;
            let sigma = (f_prime * sharp * e_star / PI).sqrt();
            assert!(
                (sigma - cs.at_pitch_point).abs() < 1e-12 * sigma,
                "z={z1}/{z2} beta={beta_deg}: {sigma} vs the verified {}",
                cs.at_pitch_point
            );
        }
    }

    /// The rulings are built by projecting each axis onto the common tangent
    /// plane, with no trigonometry about helix angles anywhere in it. That the
    /// result makes the **base** helix angle with its own axis is therefore a
    /// consequence to be checked, not an assumption to be trusted.
    #[test]
    fn the_rulings_make_the_base_helix_angle_with_their_own_axes() {
        let alpha_n = 20.0_f64.to_radians();
        for sigma_deg in [0.0, 45.0, 90.0, 120.0] {
            for beta_1_deg in [0.0, 20.0, 60.0, 82.0] {
                let sigma = f64::to_radians(sigma_deg);
                let beta_1 = f64::to_radians(beta_1_deg);
                let (g1, g2) = rulings(sigma, alpha_n, beta_1).unwrap();

                let beta_b1 = (beta_1.sin() * alpha_n.cos()).asin();
                let beta_b2 = ((sigma - beta_1).sin() * alpha_n.cos()).asin();
                assert!(
                    (dot(g1, [0.0, 0.0, 1.0]).abs() - beta_b1.cos()).abs() < 1e-14,
                    "Σ={sigma_deg} β₁={beta_1_deg}: worm ruling"
                );
                let axis_2 = [0.0, sigma.sin(), sigma.cos()];
                assert!(
                    (dot(g2, axis_2).abs() - beta_b2.cos()).abs() < 1e-14,
                    "Σ={sigma_deg} β₁={beta_1_deg}: wheel ruling"
                );
            }
        }
    }

    /// Crossing the shafts is what turns the line into an ellipse: the flatter
    /// relative curvature lifts off zero and keeps rising, so the patch keeps
    /// shortening. This is the geometry behind `1/R_L` being *the* parameter of
    /// the contact unification.
    #[test]
    fn crossing_the_shafts_lifts_the_flat_curvature_off_zero() {
        let alpha_n = 20.0_f64.to_radians();
        let beta_1 = 82.0_f64.to_radians();
        let mut previous = -1.0;
        for sigma_deg in [0.0, 1.0, 5.0, 15.0, 45.0, 90.0] {
            let (flat, sharp) =
                pitch_point_curvatures(f64::to_radians(sigma_deg), alpha_n, beta_1, 3.5, 20.0)
                    .unwrap();
            assert!(flat <= sharp, "the flatter direction comes first");
            assert!(
                flat > previous,
                "Σ={sigma_deg}: flat curvature {flat} did not exceed {previous}"
            );
            previous = flat;
        }
        assert!(previous > 0.0, "a right-angle pair is a point contact");
    }

    /// A worm pair presses a real ellipse, and the load that presses it is the
    /// loaded normal force rather than a frictionless projection.
    #[test]
    fn a_worm_presses_an_ellipse_and_carries_the_loaded_normal_force() {
        let s = worm(1, 40, 7.0);
        let torque = 2.0;

        // Frictionless, the balance is F_t/(cos α_n sin γ) exactly.
        let tangential = 2000.0 * torque / s.worm_pitch_diameter;
        let ideal = tangential / (s.normal_pressure_angle.cos() * s.lead_angle.sin());
        assert!((s.normal_force(torque, Member::First, 0.0) - ideal).abs() < 1e-12 * ideal);

        // Which torque is held fixed decides which way friction moves the flank
        // load, and it is an easy one to get backwards. Holding the *input*
        // fixed, friction carries part of the tangential force, so the flanks
        // need pressing less hard; holding the *output* fixed, friction has
        // eaten into the useful part and they must be pressed harder.
        assert!(
            s.normal_force(torque, Member::First, 0.06)
                < s.normal_force(torque, Member::First, 0.0),
            "at fixed input torque, friction lowers the flank load"
        );
        assert!(
            s.normal_force(torque, Member::Second, 0.06)
                > s.normal_force(torque, Member::Second, 0.0),
            "at fixed output torque, friction raises it — the rating direction"
        );

        let c = s.contact(torque, Member::Second, 0.06, 113_000.0).unwrap();
        assert!(c.semi_major().is_finite() && c.semi_minor() > 0.0);
        assert!(
            c.semi_major() > c.semi_minor(),
            "the patch should be an ellipse, not a circle: {} by {}",
            c.semi_major(),
            c.semi_minor()
        );
        assert!(
            c.max_pressure > 0.0 && c.max_pressure.is_finite(),
            "pressure {}",
            c.max_pressure
        );
        // A point contact concentrates load far harder than a line would: the
        // same normal force spread along a 10 mm face at this curvature.
        let line = (s.normal_force(torque, Member::Second, 0.06) / 10.0
            * s.contact_curvatures().unwrap().1
            * 113_000.0
            / PI)
            .sqrt();
        assert!(
            c.max_pressure > 2.0 * line,
            "point contact {} should dominate a line {line}",
            c.max_pressure
        );
    }

    /// `sin γ = z m_n/d` is claimed to be the once-substituted form of
    /// `tan γ = z m_x/d` with `m_x = m_n/cos γ`. Check that the answer really is
    /// the fixed point the iterative reading would have converged to.
    #[test]
    fn the_lead_angle_is_the_fixed_point_the_iteration_would_have_found() {
        for (starts, d1) in [(1u32, 7.0), (2, 7.0), (4, 12.0), (1, 30.0), (9, 40.0)] {
            let s = worm(starts, 30, d1);
            let m_x = 1.0 / s.lead_angle.cos();
            let residual = s.lead_angle.tan() - f64::from(starts) * m_x / d1;
            assert!(
                residual.abs() < 1e-15,
                "starts={starts} d={d1}: tan γ − z m_x/d = {residual}"
            );
        }
    }

    /// The same law governs the wheel, with its own tooth count — which is what
    /// lets the wheel diameter come straight out instead of via the axial
    /// module.
    #[test]
    fn the_lead_angle_law_holds_on_the_wheel_too() {
        for sigma_deg in [90.0, 70.0, 110.0, 45.0] {
            let s = Screw::new(&ScrewParams {
                starts: 2,
                wheel_teeth: 41,
                worm_pitch_diameter: 9.0,
                shaft_angle: f64::to_radians(sigma_deg),
                ..Default::default()
            })
            .unwrap();
            let implied = 41.0 * 1.0 / s.wheel_pitch_diameter;
            assert!(
                (s.wheel_lead_angle.sin() - implied).abs() < 1e-14,
                "Σ={sigma_deg}: sin γ₂ {} vs z₂m_n/d₂ {implied}",
                s.wheel_lead_angle.sin()
            );
        }
    }

    /// The transmission ratio is not imposed anywhere — it has to fall out of
    /// the two diameters and the two lead angles. If it does not, the geometry
    /// is not self-consistent.
    #[test]
    fn the_ratio_falls_out_of_the_geometry_rather_than_being_imposed() {
        for (starts, teeth, d1, sigma) in [
            (1u32, 30u32, 7.0, 90.0),
            (2, 41, 9.0, 90.0),
            (4, 60, 20.0, 75.0),
            (3, 23, 14.0, 100.0),
        ] {
            let s = Screw::new(&ScrewParams {
                starts,
                wheel_teeth: teeth,
                worm_pitch_diameter: d1,
                shaft_angle: f64::to_radians(sigma),
                ..Default::default()
            })
            .unwrap();
            // ω₁/ω₂ = (v₁/r₁)/(v₂/r₂) = (r₂/r₁)/(v₂/v₁)
            let from_geometry =
                (s.wheel_pitch_diameter / s.worm_pitch_diameter) / s.velocity_ratio();
            assert!(
                (from_geometry - s.ratio).abs() < 1e-12 * s.ratio,
                "z={starts}/{teeth} Σ={sigma}: {from_geometry} vs {}",
                s.ratio
            );
        }
    }

    /// Helix angles add to the shaft angle — the relation the specification
    /// leans on when it makes the wheel's helix angle an output.
    #[test]
    fn helix_angles_add_to_the_shaft_angle() {
        for sigma_deg in [90.0, 60.0, 120.0, 46.0] {
            let sigma = f64::to_radians(sigma_deg);
            let s = Screw::new(&ScrewParams {
                shaft_angle: sigma,
                worm_pitch_diameter: 11.0,
                ..Default::default()
            })
            .unwrap();
            assert!((s.worm_helix_angle + s.wheel_helix_angle - sigma).abs() < 1e-15);
            assert!(
                (s.lead_angle + s.wheel_lead_angle - (PI - sigma)).abs() < 1e-15,
                "γ₁ + γ₂ should be 180° − Σ"
            );
        }
    }

    /// The pitch-point sliding, against the vector kinematics of step 4 — which
    /// knows nothing about screw gearing and builds the velocities from two
    /// axes and two speeds.
    #[test]
    fn the_sliding_matches_the_vector_kinematics_it_should() {
        for sigma_deg in [90.0, 70.0, 110.0] {
            let sigma = f64::to_radians(sigma_deg);
            let s = Screw::new(&ScrewParams {
                starts: 2,
                wheel_teeth: 41,
                worm_pitch_diameter: 9.0,
                shaft_angle: sigma,
                ..Default::default()
            })
            .unwrap();

            let r1 = s.worm_pitch_diameter / 2.0;
            let r2 = s.wheel_pitch_diameter / 2.0;
            let omega_1 = 100.0;
            // Signed about its own axis. As with an external parallel-axis
            // pair, the two members turn opposite ways about their axes; the
            // sign is what makes the surfaces move together rather than into
            // each other, and getting it wrong is visible in the magnitude at
            // any shaft angle but 90°.
            let omega_2 = -omega_1 / s.ratio;

            // Axis 1 is z; axis 2 is turned out of it by the shaft angle, with
            // the centre distance along x.
            let v = sliding_velocity(
                [0.0, sigma.sin(), sigma.cos()],
                omega_1,
                omega_2,
                [s.centre_distance, 0.0, 0.0],
                [r1, 0.0, 0.0],
                [0.0, 0.0, 1.0],
            );
            let expected = s.sliding_ratio * omega_1 * r1;
            assert!(
                (v.magnitude() - expected).abs() < 1e-10 * expected,
                "Σ={sigma_deg}: kinematics {} vs geometry {expected} (r₂={r2})",
                v.magnitude()
            );
        }
    }

    /// A 90° drive must reproduce the classical closed forms exactly — they are
    /// what this derivation is answerable to.
    #[test]
    fn the_ninety_degree_case_reproduces_the_classical_screw_formulas() {
        for (starts, d1) in [(1u32, 7.0), (2, 9.0), (4, 12.0), (1, 25.0)] {
            let s = worm(starts, 41, d1);
            let cos_alpha = s.normal_pressure_angle.cos();
            let (tan_g, cot_g) = (s.lead_angle.tan(), 1.0 / s.lead_angle.tan());
            for mu in [0.0, 0.02, 0.05, 0.1, 0.2] {
                let e = Directional::of(|d| s.efficiency(mu, d));
                let forward = (cos_alpha - mu * tan_g) / (cos_alpha + mu * cot_g);
                let backward = (cos_alpha - mu * cot_g) / (cos_alpha + mu * tan_g);
                assert!(
                    (e.forward - forward).abs() < 1e-14,
                    "z₁={starts} d={d1} mu={mu}: forward {} vs {forward}",
                    e.forward
                );
                assert!(
                    (e.backward - backward).abs() < 1e-14,
                    "z₁={starts} d={d1} mu={mu}: backward {} vs {backward}",
                    e.backward
                );
            }
        }
    }

    /// Energy has to balance at any shaft angle: what goes in, minus what comes
    /// out, is exactly the friction force times the sliding speed. This is the
    /// check that the force balance is a balance and not an assembly of
    /// plausible terms.
    #[test]
    fn the_power_that_does_not_come_out_is_the_friction_loss() {
        for sigma_deg in [90.0, 65.0, 115.0, 50.0] {
            let s = Screw::new(&ScrewParams {
                starts: 2,
                wheel_teeth: 37,
                worm_pitch_diameter: 10.0,
                shaft_angle: f64::to_radians(sigma_deg),
                ..Default::default()
            })
            .unwrap();
            for mu in [0.0, 0.03, 0.08, 0.15] {
                let e = Directional::of(|d| s.efficiency(mu, d));
                // Per unit normal force and unit worm pitch line speed.
                let cos_alpha = s.normal_pressure_angle.cos();
                let k = s.velocity_ratio();
                let slide_on_1 = (1.0 - k * s.shaft_angle.cos()) / s.sliding_ratio;
                let power_in = (cos_alpha * s.lead_angle.sin() + mu * slide_on_1) * 1.0;
                let power_out = e.forward * power_in;
                let loss = mu * s.sliding_ratio;
                assert!(
                    (power_in - power_out - loss).abs() < 1e-13 * power_in,
                    "Σ={sigma_deg} mu={mu}: in−out {} vs μ|v_s| {loss}",
                    power_in - power_out
                );
            }
        }
    }

    /// Frictionless means lossless, in both directions and at any shaft angle —
    /// exactly, not nearly.
    #[test]
    fn a_frictionless_screw_pair_loses_nothing() {
        for sigma_deg in [90.0, 60.0, 120.0] {
            let s = Screw::new(&ScrewParams {
                shaft_angle: f64::to_radians(sigma_deg),
                worm_pitch_diameter: 8.0,
                ..Default::default()
            })
            .unwrap();
            let e = Directional::of(|d| s.efficiency(0.0, d));
            assert!((e.forward - 1.0).abs() < 1e-15);
            assert!((e.backward - 1.0).abs() < 1e-15);
            assert!(!e.self_locking());
        }
    }

    /// Self-locking is a sign change, and the threshold is `μ ≥ cos α_n tan γ`.
    /// The interesting part is that it is exact: at the threshold the backward
    /// efficiency is zero rather than merely small.
    #[test]
    fn self_locking_begins_exactly_where_the_closed_form_says() {
        for (starts, d1) in [(1u32, 7.0), (1, 20.0), (2, 9.0), (4, 12.0)] {
            let s = worm(starts, 41, d1);
            let threshold = s.normal_pressure_angle.cos() * s.lead_angle.tan();
            assert!(
                (s.self_locking_friction() - threshold).abs() < 1e-14,
                "z₁={starts} d={d1}: threshold {} vs cos α_n tan γ {threshold}",
                s.self_locking_friction()
            );

            let at = Directional::of(|d| s.efficiency(threshold, d));
            assert!(
                at.backward.abs() < 1e-15,
                "at the threshold the backward efficiency should vanish: {}",
                at.backward
            );
            assert!(at.self_locking(), "the threshold itself counts as locked");

            let below = Directional::of(|d| s.efficiency(threshold * 0.9, d));
            assert!(below.backward > 0.0 && !below.self_locking());
            let above = Directional::of(|d| s.efficiency(threshold * 1.1, d));
            assert!(above.backward < 0.0 && above.self_locking());
        }
    }

    /// The direction dependence itself: a worm is worse to back-drive than to
    /// drive, and a single-start worm on a small diameter — a low lead angle —
    /// is where that gap becomes self-locking.
    #[test]
    fn back_driving_is_always_the_worse_direction_and_low_lead_angles_lock() {
        let mu = 0.06;
        let steep = worm(4, 41, 9.0);
        let shallow = worm(1, 41, 25.0);
        assert!(
            steep.lead_angle > shallow.lead_angle,
            "more starts on a smaller worm is the steeper thread"
        );

        for s in [steep, shallow] {
            let e = Directional::of(|d| s.efficiency(mu, d));
            assert!(
                e.backward < e.forward,
                "back-driving must be the worse direction: {} vs {}",
                e.backward,
                e.forward
            );
        }
        assert!(
            !Directional::of(|d| steep.efficiency(mu, d)).self_locking(),
            "a steep thread back-drives"
        );
        assert!(
            Directional::of(|d| shallow.efficiency(mu, d)).self_locking(),
            "a 1-start worm on a 25 mm diameter should self-lock at mu={mu}"
        );
    }

    /// A worm drive's efficiency is dominated by the lead angle, not by the
    /// friction coefficient — which is the design fact the number exists to
    /// show.
    #[test]
    fn efficiency_rises_with_the_lead_angle() {
        let mu = 0.05;
        let mut previous = 0.0;
        for (starts, d1) in [(1u32, 30.0), (1, 12.0), (2, 12.0), (4, 12.0), (6, 12.0)] {
            let s = worm(starts, 60, d1);
            let e = s.efficiency(mu, Drive::Forward);
            assert!(
                e > previous,
                "z₁={starts} d={d1}: γ={:.2}° gave {e}, below the previous {previous}",
                s.lead_angle.to_degrees()
            );
            previous = e;
        }
        // Six starts on a 12 mm worm is a 30° lead angle, which is about as
        // steep as a worm is built; it lands near 89 % at this friction.
        assert!(
            previous > 0.85,
            "a steep multi-start worm should be efficient"
        );
    }

    #[test]
    fn impossible_pairs_are_refused() {
        // z₁ m_n >= d₁: the thread cannot wrap at ninety degrees or more.
        assert_eq!(
            Screw::new(&ScrewParams {
                starts: 9,
                worm_pitch_diameter: 8.0,
                ..Default::default()
            })
            .unwrap_err(),
            ScrewError::WormTooThin
        );
        assert_eq!(
            Screw::new(&ScrewParams {
                worm_pitch_diameter: 0.0,
                ..Default::default()
            })
            .unwrap_err(),
            ScrewError::NotPositive
        );
        // Parallel axes leave the wheel with no lead angle at all.
        assert!(Screw::new(&ScrewParams {
            shaft_angle: 0.0,
            ..Default::default()
        })
        .is_err());
    }
}
