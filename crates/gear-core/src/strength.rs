//! Tooth bending geometry: the critical root section and the form factor.
//!
//! # Why there is no table here
//!
//! Lewis form factors are tabulated because, historically, nobody could compute
//! the real root geometry. We can: `profile` generates the exact trochoid,
//! undercut included, and it is validated against a simulation of the cutter
//! that would make it. So the form factor is **measured off the profile** rather
//! than looked up.
//!
//! That is the whole argument for this module. Undercut, profile shift and tooth
//! thickness modification need no special cases, because they change the profile
//! and we measured the profile. A tabulated factor cannot do that — the tables
//! are indexed by tooth count and shift alone, and stop at the standard rack.
//!
//! # The construction
//!
//! The **30° tangent method** (Hofer): the critical section is the chord between
//! the two points on the root fillet where the tangent makes 30° to the tooth
//! centreline. Along the fillet that angle sweeps monotonically from near zero at
//! the flank junction to 90° where it meets the root circle, so the tangency
//! point exists, is unique, and is bracketed by construction.
//!
//! Everything is done in **tooth coordinates**: `y` along the tooth centreline
//! pointing outward, `x` across it, origin at the gear axis.

use crate::contact::ContactPath;
use crate::mesh::{Mesh, MeshKind};
use crate::profile::Gear;
use crate::solve::{brent, Tol};

/// How the critical root section is located.
///
/// These are two answers to the same question, and they disagree most exactly
/// where it matters — on undercut teeth.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CriticalSection {
    /// The ISO/AGMA 30° tangent (Hofer).
    ///
    /// **Retained but not the default.** Kept because it is what ISO 6336 and
    /// AGMA 2101 specify, so it is the setting to return to for a
    /// standards-comparable number, and because
    /// [`StressConcentration::Iso6336`] is a fit calibrated against *this*
    /// construction.
    ///
    /// Its weakness is that it is **independent of where the load acts**, which
    /// is precisely the property the cantilever model is supposed to have. The
    /// two constructions converge as teeth get larger — at z=60 the 30° tangents
    /// cross the centreline within 0.04% of the load point — and diverge where
    /// the geometry is worst, crossing 12% below it at z=9.
    TangentAngle,
    /// The Lewis inscribed parabola — the original construction.
    ///
    /// A cantilever whose outline is a parabola with its vertex at the load has
    /// uniform bending stress along its length. Inscribing the largest such
    /// parabola, vertex at the point where the load line crosses the tooth
    /// centreline, and taking the tangency with the fillet, finds where the real
    /// tooth is weakest *relative to that uniform-strength shape*.
    ///
    /// Unlike the 30° tangent this **follows the load point**, which is the
    /// property the cantilever model is supposed to have. It is consistently
    /// more conservative: the tangency sits higher up the fillet, the section is
    /// narrower, and `Y_F` comes out 2–14% larger, most on undercut teeth.
    ///
    /// # Divergence from the standards
    ///
    /// This is **the default here and it is not what ISO 6336 or AGMA 2101
    /// specify.** The reasons for diverging, and the reasons to be careful:
    ///
    /// - It is the original construction; the 30° tangent is a later
    ///   simplification adopted for ease of calculation. This project computes
    ///   the exact profile, so the simplification buys nothing.
    /// - Experimental single-tooth-bending work reports measured critical
    ///   locations *above* the 30° prediction, with that prediction at the edge
    ///   of the observed range — the direction the parabola moves the section.
    ///   (The authors note large test deformations may contribute, so this is
    ///   support rather than proof.)
    /// - It is the more conservative of the two, everywhere.
    ///
    /// Against that: it changes ranking very little — Spearman ρ = 0.993 against
    /// the 30° tangent over 1521 designs, with identical gradient direction
    /// wherever a parameter moves the answer by 1% or more — so the choice is
    /// principled rather than consequential. And **`Y_S` is calibrated against
    /// the 30° construction**, so pairing the two mixes conventions; where the
    /// parabola leaves the fillet the pairing is refused outright, see
    /// [`RootSection::stress_correction`].
    ///
    /// For a number to compare against a published ISO or AGMA rating, switch to
    /// [`CriticalSection::TangentAngle`].
    #[default]
    LewisParabola,
}

/// The angle the critical-section tangent makes with the tooth centreline.
///
/// 30° is the Hofer construction adopted by ISO 6336 for external gears. It is a
/// convention, not a derivation — the true peak stress location depends on the
/// fillet shape — and it is named here rather than buried as a literal so the
/// 60° internal-gear variant can be added beside it.
pub const TANGENT_ANGLE_DEG: f64 = 30.0;

/// The critical root section and the load that acts on it.
///
/// Lengths in millimetres, angles in radians. Coordinates are in the tooth frame
/// described in the module documentation, so they can be drawn directly.
#[derive(Clone, Copy, Debug)]
pub struct RootSection {
    /// Rack travel parameter at the tangency point.
    pub s: f64,
    /// Tooth root chord thickness at the critical section, `s_Fn`.
    pub root_chord: f64,
    /// Bending moment arm from the critical section to where the load line
    /// crosses the tooth centreline, `h_Fe`.
    pub moment_arm: f64,
    /// Load application angle, `α_Fen`: the angle between the load direction and
    /// the perpendicular to the tooth centreline.
    pub load_angle: f64,
    /// Radius of curvature of the fillet at the critical section, `ρ_F`.
    pub fillet_curvature: f64,
    /// Tooth form factor `Y_F`.
    pub form_factor: f64,
    /// Notch parameter `q_s = s_Fn / (2 ρ_F)`, the input to stress correction.
    pub notch_parameter: f64,
    /// Which construction located this section.
    pub method: CriticalSection,
    /// True when the inscribed parabola touched the involute flank rather than
    /// the fillet. Expected on larger teeth; see [`CriticalSection::LewisParabola`].
    pub tangency_on_flank: bool,
    /// Parabola parameter `p` in `x² = 4p(y_v − y)`, for drawing the inscribed
    /// parabola. Only meaningful for [`CriticalSection::LewisParabola`].
    pub parabola_p: Option<f64>,

    /// Tangency point on the `+x` side, for drawing.
    pub tangency: [f64; 2],
    /// Unit tangent to the fillet at the tangency point, `+x` side.
    ///
    /// Carried rather than left to be reconstructed from the 30° angle: the
    /// angle alone does not fix which way the line leans, and a drawing that
    /// rebuilt it got the sign wrong while the tangency point was correct.
    /// Anything depicting the construction should use this.
    pub tangent_direction: [f64; 2],
    /// Load application point on the flank, for drawing.
    pub load_point: [f64; 2],
    /// Where the load line crosses the centreline, for drawing.
    pub load_line_crossing: [f64; 2],
}

/// A point on the fillet and the curve's tangent there, in tooth coordinates.
///
/// The derivative is analytic. Writing the trochoid as a rotation of the
/// generating-frame point makes it fall out in two lines:
///
/// ```text
/// X = u cos φ − v sin φ        X' = u' cos φ − v' sin φ − φ' Y
/// Y = v cos φ + u sin φ        Y' = u' sin φ + v' cos φ + φ' X
/// ```
///
/// with `u = k s`, `v = r − k b_c`, `φ = (s − a_c)/r`.
fn fillet_point_and_tangent(g: &Gear, s: f64) -> ([f64; 2], [f64; 2]) {
    let d = f64::hypot(s, g.bc);
    let k = 1.0 + g.rho / d;
    let dk = -g.rho * s / (d * d * d);

    let u = k * s;
    let du = k + s * dk;
    let v = g.r - k * g.bc;
    let dv = -dk * g.bc;

    let phi = (s - g.ac) / g.r;
    let dphi = 1.0 / g.r;
    let (c, sn) = (phi.cos(), phi.sin());

    let x = u * c - v * sn;
    let y = v * c + u * sn;
    let dx = du * c - dv * sn - dphi * y;
    let dy = du * sn + dv * c + dphi * x;
    ([x, y], [dx, dy])
}

/// Second derivative by central difference on the analytic first derivative.
///
/// Only the fillet's radius of curvature needs it, and that feeds the empirical
/// stress-concentration factor rather than the form factor, so a difference is
/// proportionate here where it would not be for the tangent itself.
fn fillet_curvature_radius(g: &Gear, s: f64) -> f64 {
    let h = 1e-6 * g.params.module.max(1e-9);
    let (_, t0) = fillet_point_and_tangent(g, s - h);
    let (_, t1) = fillet_point_and_tangent(g, s + h);
    let (_, t) = fillet_point_and_tangent(g, s);
    let ddx = (t1[0] - t0[0]) / (2.0 * h);
    let ddy = (t1[1] - t0[1]) / (2.0 * h);
    let speed = f64::hypot(t[0], t[1]);
    let cross = (t[0] * ddy - t[1] * ddx).abs();
    if cross < f64::MIN_POSITIVE {
        f64::INFINITY
    } else {
        speed.powi(3) / cross
    }
}

/// A point on the involute flank and its tangent, in tooth coordinates.
///
/// The parabola construction has to search the flank as well as the fillet: on
/// anything but a small or undercut tooth the largest inscribed parabola touches
/// the *flank*. For the rack limit that tangency sits 0.54 module above where
/// the fillet ends, so a fillet-only search finds nothing at all.
fn flank_point_and_tangent(g: &Gear, u: f64) -> ([f64; 2], [f64; 2]) {
    let root = f64::hypot(1.0, u);
    let r = g.rb * root;
    let th = g.psi_b - (u - u.atan());
    let (st, ct) = (th.sin(), th.cos());

    let dr = g.rb * u / root;
    let dth = -(u * u) / (1.0 + u * u);
    (
        [r * st, r * ct],
        [dr * st + r * ct * dth, dr * ct - r * st * dth],
    )
}

/// A point on the involute flank and the direction of the load there.
///
/// The load acts along the involute normal, which is the line from the contact
/// point to the base-circle tangency point — that is what "the line of action"
/// means, and it is exact rather than an approximation to the pressure angle.
fn flank_point_and_load_direction(g: &Gear, roll: f64) -> ([f64; 2], [f64; 2]) {
    let (r, th) = g.involute_at(roll);
    let p = [r * th.sin(), r * th.cos()];
    // The generating tangent point sits `roll` radians back around the base
    // circle from the involute's own angular position.
    let tangent_angle = g.psi_b - roll;
    let t = [g.rb * tangent_angle.sin(), g.rb * tangent_angle.cos()];
    let (dx, dy) = (p[0] - t[0], p[1] - t[1]);
    let len = f64::hypot(dx, dy);
    if len < f64::MIN_POSITIVE {
        return (p, [1.0, 0.0]);
    }
    (p, [dx / len, dy / len])
}

/// The critical root section for a load applied at a given roll parameter.
///
/// Returns `None` when the gear has no usable flank — a severed tooth, or a
/// fillet on which the 30° tangent does not exist.
#[must_use]
pub fn root_section(g: &Gear, load_roll: f64) -> Option<RootSection> {
    root_section_with(g, load_roll, CriticalSection::default())
}

/// The critical root section, locating it by the chosen construction.
///
/// Returns `None` when the gear has no usable flank — a severed tooth, or a
/// fillet on which the construction has no solution.
#[must_use]
pub fn root_section_with(g: &Gear, load_roll: f64, method: CriticalSection) -> Option<RootSection> {
    if g.severed || !g.u_j.is_finite() || !load_roll.is_finite() {
        return None;
    }

    // The load has to be resolved first either way: the parabola's vertex sits
    // where the load line crosses the centreline.
    let (load_point, dir) = flank_point_and_load_direction(g, load_roll);
    if dir[0].abs() < 1e-12 {
        return None;
    }
    let crossing = [0.0, load_point[1] + (-load_point[0] / dir[0]) * dir[1]];
    let vertex = crossing[1];

    let s = match method {
        // Along the fillet the tangent angle to the centreline sweeps from near
        // zero at the junction to 90° at the root circle, so this is monotone
        // and the bracket is the fillet itself.
        CriticalSection::TangentAngle => {
            let target = TANGENT_ANGLE_DEG.to_radians().tan();
            brent(
                |s| {
                    let (_, t) = fillet_point_and_tangent(g, s);
                    t[0].abs() - target * t[1].abs()
                },
                g.s_j,
                0.0,
                Tol::default(),
            )?
        }
        // Tangency of the parabola x² = 4p(y_v − y) with the tooth outline.
        // Requiring the point to lie on the parabola and the slopes to match
        // eliminates p and leaves one equation:
        //     X·Y' + 2 X' (y_v − Y) = 0
        //
        // Searched on the fillet first and then the flank, because which one it
        // touches depends on the tooth: small and undercut teeth touch the
        // fillet, larger ones the flank.
        CriticalSection::LewisParabola => {
            let condition = |q: [f64; 2], t: [f64; 2]| q[0] * t[1] + 2.0 * t[0] * (vertex - q[1]);
            let on_fillet = brent(
                |s| {
                    let (q, t) = fillet_point_and_tangent(g, s);
                    condition(q, t)
                },
                g.s_j,
                0.0,
                Tol::default(),
            );
            match on_fillet {
                Some(s) => s,
                None => {
                    let u = brent(
                        |u| {
                            let (q, t) = flank_point_and_tangent(g, u);
                            condition(q, t)
                        },
                        g.u_j,
                        g.u_tip,
                        Tol::default(),
                    )?;
                    return finish(g, method, u, true, load_point, dir, crossing, vertex);
                }
            }
        }
    };

    finish(g, method, s, false, load_point, dir, crossing, vertex)
}

/// Assemble the result once the tangency parameter is known, whichever curve it
/// was found on.
#[allow(clippy::too_many_arguments)]
fn finish(
    g: &Gear,
    method: CriticalSection,
    param: f64,
    on_flank: bool,
    load_point: [f64; 2],
    load_dir: [f64; 2],
    crossing: [f64; 2],
    vertex: f64,
) -> Option<RootSection> {
    let s = param;
    let (tangency, raw_tangent) = if on_flank {
        flank_point_and_tangent(g, param)
    } else {
        fillet_point_and_tangent(g, param)
    };
    let root_chord = 2.0 * tangency[0].abs();
    // Orient it up the tooth (towards the tip) so the direction is unambiguous.
    let len = f64::hypot(raw_tangent[0], raw_tangent[1]);
    let sign = if raw_tangent[1] < 0.0 { -1.0 } else { 1.0 };
    let tangent_direction = [sign * raw_tangent[0] / len, sign * raw_tangent[1] / len];

    let moment_arm = crossing[1] - tangency[1];
    let parabola_p = match method {
        CriticalSection::LewisParabola => {
            Some(-tangency[0] * raw_tangent[0] / (2.0 * raw_tangent[1]))
        }
        CriticalSection::TangentAngle => None,
    };
    let _ = vertex;
    // cos α_Fen is the share of the load acting across the tooth; the load
    // direction's x-component is exactly that.
    let load_angle = load_dir[0].abs().clamp(-1.0, 1.0).acos();

    let m = g.params.module;
    let form_factor =
        6.0 * (moment_arm / m) * load_angle.cos() / ((root_chord / m).powi(2) * g.alpha_n.cos());

    Some(RootSection {
        s,
        // Curvature is a fillet property; on a flank tangency the involute's own
        // curvature is what the notch sees.
        notch_parameter: root_chord
            / (2.0
                * if on_flank {
                    g.rb * param
                } else {
                    fillet_curvature_radius(g, s)
                }),
        tangency_on_flank: on_flank,
        method,
        parabola_p,
        root_chord,
        moment_arm,
        load_angle,
        fillet_curvature: fillet_curvature_radius(g, s),
        form_factor,
        tangency,
        tangent_direction,
        load_point,
        load_line_crossing: crossing,
    })
}

/// The critical root section with the load at the tooth tip.
///
/// This is the standalone-gear case: without a mating gear there is no outer
/// point of single-pair contact, and tip loading is both the conservative choice
/// and the one the classical tabulated factors assume.
#[must_use]
pub fn tip_load_section(g: &Gear) -> Option<RootSection> {
    root_section(g, g.u_tip)
}

/// How root stress concentration is accounted for.
///
/// Kept as an explicit choice rather than folded into the stress calculation so
/// that a run can be repeated without it. The form factor is measured off exact
/// geometry and is checkable against a closed-form limit; a stress correction is
/// an empirical fit, and being able to separate the two is what makes it
/// possible to tell a geometry error from an over-fitted correction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum StressConcentration {
    /// ISO 6336-3.
    ///
    /// ```text
    /// Y_S = (1.2 + 0.13 L) · q_s^(1 / (1.21 + 2.3/L))
    /// L   = s_Fn / h_Fe          q_s = s_Fn / (2 ρ_F)
    /// ```
    ///
    /// Chosen over Dolan–Broghamer, the 1942 photoelastic fit that AGMA carries,
    /// for a reason specific to this project: **it is written in terms of the
    /// geometry we already measure.** `s_Fn`, `h_Fe` and `ρ_F` all come off the
    /// generated profile, so undercut, profile shift and thickness modification
    /// flow into the correction the same way they flow into the form factor.
    /// Dolan–Broghamer is presented as charts indexed by tooth count and shift,
    /// which is exactly the dependence this project set out to avoid.
    ///
    /// It remains an empirical fit. Published comparisons put Dolan–Broghamer
    /// within about 8% of finite-element results; a genuinely geometry-exact
    /// notch stress needs FEA or a critical-distance method, and the latter is
    /// material-dependent, so neither belongs in a high-level design tool.
    #[default]
    Iso6336,
    /// No correction: report the form factor alone.
    ///
    /// This is the control case. If a stress figure looks wrong, comparing the
    /// two says whether the geometry or the fit is responsible.
    None,
}

/// Upper bound on the notch parameter for which the ISO fit is stated.
///
/// Outside it the formula still evaluates, and [`RootSection::stress_correction`]
/// still returns a value — but [`RootSection::notch_parameter_in_range`] reports
/// false so a caller can say so. **Confirm against ISO 6336-3 before relying on
/// this bound**; it is the figure quoted in secondary sources, not one I have
/// read from the standard.
pub const NOTCH_PARAMETER_RANGE: std::ops::Range<f64> = 1.0..8.0;

impl RootSection {
    /// The stress correction factor `Y_S` under the chosen model.
    ///
    /// Returns `None` when the correction has no meaning for this section:
    /// [`StressConcentration::Iso6336`] is a *notch* factor, and its input `ρ_F`
    /// is the notch radius. When the inscribed parabola touches the involute
    /// flank there is no notch at the critical section, so the fit has no valid
    /// input.
    ///
    /// This is not a formality. Evaluating it anyway substitutes the involute's
    /// own curvature for `ρ_F`, which on a large tooth is enormous — `q_s` falls
    /// from 1.81 to 0.048 across the seam at z=150→151 and the corrected factor
    /// **jumps 17%** while `Y_F` itself moves by 0.03%. A design tool with a
    /// cliff in the middle of its parameter space is worse than one that admits
    /// the combination is undefined.
    ///
    /// The notch parameter is **clamped** into the range the fit is stated for
    /// before being used, so an out-of-range gear gets the value at the boundary
    /// rather than an extrapolation. The unclamped figure stays available in
    /// [`RootSection::notch_parameter`], because it is worth seeing even when it
    /// cannot be used: it is largely set by the cutter tip radius and the tooth
    /// space, neither of which the designer can freely choose.
    ///
    /// Note the direction of the error. `Y_S` rises with `q_s`, so clamping a
    /// sharper-than-stated notch **under-predicts** the stress. That is
    /// unconservative, which is why [`RootSection::notch_parameter_in_range`]
    /// exists and should be surfaced rather than swallowed.
    #[must_use]
    pub fn stress_correction(&self, model: StressConcentration) -> Option<f64> {
        match model {
            StressConcentration::None => Some(1.0),
            StressConcentration::Iso6336 if self.tangency_on_flank => None,
            StressConcentration::Iso6336 => {
                let l = self.root_chord / self.moment_arm;
                let q = self
                    .notch_parameter
                    .clamp(NOTCH_PARAMETER_RANGE.start, NOTCH_PARAMETER_RANGE.end);
                Some((1.2 + 0.13 * l) * q.powf(1.0 / (1.21 + 2.3 / l)))
            }
        }
    }

    /// Whether the notch parameter falls where the ISO fit is stated.
    #[must_use]
    pub fn notch_parameter_in_range(&self) -> bool {
        NOTCH_PARAMETER_RANGE.contains(&self.notch_parameter)
    }

    /// `Y_F · Y_S`: the full geometry factor multiplying `F_t / (b m)`.
    ///
    /// `None` where the correction is undefined; see
    /// [`RootSection::stress_correction`].
    #[must_use]
    pub fn bending_factor(&self, model: StressConcentration) -> Option<f64> {
        Some(self.form_factor * self.stress_correction(model)?)
    }
}

// ------------------------------------------------------------------ load ---

/// What a mesh is carrying.
///
/// # Why the load is stored as the normal force
///
/// A torque can be turned into a tangential force at the reference pitch
/// circle, at the operating pitch circle, or anywhere else, and the three
/// differ — which is a reliable source of quiet factor-of-`cos α` errors.
///
/// The force along the **line of action** has no such ambiguity. Power crosses
/// an involute mesh along the common tangent to the base circles, so the exact
/// lever arm is the *base* radius:
///
/// ```text
/// T = F_n · r_b        (exact, not a convention)
/// ```
///
/// Every other force follows from it by projection — `F_t = F_n cos α_t` at the
/// reference circle, `F_n cos α_w` at the operating circle — so storing `F_n`
/// makes each conversion explicit at the point of use.
#[derive(Clone, Copy, Debug)]
pub struct Load {
    /// Force along the line of action, N.
    pub normal_force: f64,
    /// Face width, mm.
    pub face_width: f64,
}

impl Load {
    /// From a torque in **N·m** on this gear, and a face width in mm.
    ///
    /// The 1000 converts N·m to N·mm, because every length in this crate is
    /// millimetres.
    #[must_use]
    pub fn from_torque(g: &Gear, torque: f64, face_width: f64) -> Self {
        Self {
            normal_force: 1000.0 * torque / g.rb,
            face_width,
        }
    }

    /// Tangential force at the **reference** pitch circle, N.
    ///
    /// This is the force the form factor is normalised against, and the one ISO
    /// 6336 calls `F_t`.
    #[must_use]
    pub fn tangential(&self, g: &Gear) -> f64 {
        self.normal_force * g.alpha_t.cos()
    }

    /// The torque this load represents on a given gear, N·m.
    #[must_use]
    pub fn torque(&self, g: &Gear) -> f64 {
        self.normal_force * g.rb / 1000.0
    }
}

/// Tooth root bending stress, MPa.
///
/// ```text
/// σ_F = F_t / (b · m_n) · Y_F · Y_S
/// ```
///
/// `F_t` in newtons and `b`, `m` in millimetres give N/mm² = MPa directly.
///
/// The **normal** module is used, not the transverse one: `Y_F` is measured in
/// the normal plane, where the tooth form actually lives, so pairing it with
/// `m_t` would double-count the helix angle.
///
/// Returns `None` when the stress correction is undefined for this section —
/// see [`RootSection::stress_correction`]. That is not a failure to compute; it
/// is the model declining to apply a notch factor where there is no notch.
#[must_use]
pub fn bending_stress(
    section: &RootSection,
    g: &Gear,
    load: &Load,
    model: StressConcentration,
) -> Option<f64> {
    let factor = section.bending_factor(model)?;
    Some(load.tangential(g) / (load.face_width * g.params.module) * factor)
}

/// Hertzian contact stress along the path of contact, MPa.
#[derive(Clone, Copy, Debug)]
pub struct ContactStress {
    /// At the pitch point, where the relative radius is largest.
    pub at_pitch_point: f64,
    /// At the worse of the two single-pair contact boundaries.
    pub at_single_pair: f64,
    /// The higher of the two, which is what a design is rated on.
    pub worst: f64,
    /// Position of the worst point on the line of action, mm from the pitch
    /// point.
    pub worst_position: f64,
    /// Relative radius of curvature at the worst point, mm. Reported because it
    /// is what the number is really driven by.
    pub relative_radius: f64,
}

/// Exact Hertzian line contact for a meshing pair.
///
/// At a point `ξ` from the pitch point the two flanks are locally cylinders of
/// radius `ρ₁ = r_b1 tan α_w + ξ` and `ρ₂ = r_b2 tan α_w − ξ`, so
///
/// ```text
/// 1/ρ = 1/ρ₁ ± 1/ρ₂                 + external, − internal
/// σ_H = √( (F_n / b) · (1/ρ) · E* / π )
/// ```
///
/// `e_star` is the effective contact modulus `E*`, from
/// [`crate::material::contact_modulus`]. It is passed as a number rather than
/// taken from two materials so this stays a statement about mechanics.
///
/// # Which points are checked
///
/// `ρ₁ + ρ₂` is constant along the path, so the relative radius `ρ₁ρ₂/(ρ₁+ρ₂)`
/// is largest where the two are equal and falls away toward **both** ends. The
/// worst single-pair point is therefore whichever boundary of the single-pair
/// zone lies further from that balance point.
///
/// **Both boundaries are evaluated, not just the inner one.** DESIGN.md §4.7
/// says to take the inner point of single-pair contact, "usually the pinion's
/// worst case" — but "usually" is doing real work there. The balance point sits
/// at `(r_b2 − r_b1) tan α_w / 2`, so it is on the recess side when gear 1 is
/// the pinion and on the approach side when gear 1 is the wheel. Checking only
/// the inner boundary would therefore make the answer depend on which gear the
/// caller happened to label 1 — for the same physical mesh. Contact stress is a
/// property of the *pair*; a test asserts that swapping the labels leaves it
/// unchanged.
///
/// Returns `None` if the geometry puts a contact point outside both flanks,
/// which cannot happen for a mesh [`ContactPath`] accepted.
#[must_use]
pub fn contact_stress(
    path: &ContactPath,
    mesh: &Mesh,
    load: &Load,
    e_star: f64,
) -> Option<ContactStress> {
    // r_b1 + r_b2 = a_w cos α_w, so the two tangent lengths sum to a_w sin α_w
    // and ρ₁ + ρ₂ is constant along the path — a useful invariant to test.
    let sz = f64::from(mesh.z1) + f64::from(mesh.z2);
    let rb1 = path.base_radius_1;
    let rb2 = mesh.a_w * f64::from(mesh.z2) / sz * mesh.alpha_w.cos();

    let at = |xi: f64| -> Option<(f64, f64)> {
        let rho1 = rb1 * mesh.alpha_w.tan() + xi;
        let rho2 = rb2 * mesh.alpha_w.tan() - xi;
        if rho1 <= 0.0 || rho2 <= 0.0 {
            return None;
        }
        let inv_rho = match mesh.kind {
            MeshKind::External => 1.0 / rho1 + 1.0 / rho2,
            MeshKind::Internal => 1.0 / rho1 - 1.0 / rho2,
        };
        if inv_rho <= 0.0 {
            return None;
        }
        let sigma = ((load.normal_force / load.face_width) * inv_rho * e_star
            / std::f64::consts::PI)
            .sqrt();
        Some((sigma, 1.0 / inv_rho))
    };

    let (pitch, r_pitch) = at(0.0)?;
    let lo = path.lowest_single_pair();
    let hi = path.highest_single_pair();
    let (s_lo, r_lo) = at(lo)?;
    let (s_hi, r_hi) = at(hi)?;

    let (single, r_single, xi_single) = if s_lo >= s_hi {
        (s_lo, r_lo, lo)
    } else {
        (s_hi, r_hi, hi)
    };
    let (worst, relative_radius, worst_position) = if single >= pitch {
        (single, r_single, xi_single)
    } else {
        (pitch, r_pitch, 0.0)
    };

    Some(ContactStress {
        at_pitch_point: pitch,
        at_single_pair: single,
        worst,
        worst_position,
        relative_radius,
    })
}

/// Minimum face width for a bending stress, mm.
///
/// `σ_F ∝ 1/b`, so `b_min = b · σ_F / σ_allow`.
///
/// **The `b` cancels.** Whatever face width the stress was evaluated at, the
/// answer is the same — which is the invariant worth testing, because it is the
/// one that catches a stress that did not actually scale the way it should.
#[must_use]
pub fn min_face_width_bending(stress: f64, evaluated_at: f64, allowable: f64) -> f64 {
    evaluated_at * stress / allowable
}

/// Minimum face width for a contact stress, mm.
///
/// `σ_H ∝ 1/√b`, so `b_min = b · (σ_H / σ_allow)²`. The square is the whole
/// difference from the bending case, and it is why contact usually governs the
/// face width of a lightly loaded gear while bending governs a heavily loaded
/// one.
#[must_use]
pub fn min_face_width_contact(stress: f64, evaluated_at: f64, allowable: f64) -> f64 {
    evaluated_at * (stress / allowable).powi(2)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::GearParams;

    #[test]
    fn analytic_tangent_matches_a_finite_difference() {
        let g = Gear::new(GearParams::default());
        let h = 1e-7;
        let mut worst = 0.0_f64;
        for i in 1..20 {
            let s = g.s_j * f64::from(i) / 20.0;
            let (_, t) = fillet_point_and_tangent(&g, s);
            let (a, _) = fillet_point_and_tangent(&g, s - h);
            let (b, _) = fillet_point_and_tangent(&g, s + h);
            let fd = [(b[0] - a[0]) / (2.0 * h), (b[1] - a[1]) / (2.0 * h)];
            let scale = f64::hypot(t[0], t[1]).max(1.0);
            worst = worst.max(((t[0] - fd[0]).abs() + (t[1] - fd[1]).abs()) / scale);
        }
        assert!(worst < 1e-6, "analytic tangent disagrees by {worst:.3e}");
    }

    #[test]
    fn tangency_really_is_at_thirty_degrees() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 12,
                profile_shift: -0.2,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                profile_shift: 0.4,
                ..Default::default()
            },
            GearParams {
                teeth: 25,
                helix_angle: 20.0,
                ..Default::default()
            },
        ] {
            let g = Gear::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let (_, t) = fillet_point_and_tangent(&g, sec.s);
            let angle = (t[0].abs()).atan2(t[1].abs()).to_degrees();
            assert!(
                (angle - TANGENT_ANGLE_DEG).abs() < 1e-6,
                "z={}: tangent at {angle}°, expected {TANGENT_ANGLE_DEG}°",
                p.teeth
            );
        }
    }

    #[test]
    fn tangency_point_lies_on_the_fillet_between_root_and_junction() {
        for teeth in [9u32, 17, 30, 60] {
            let g = Gear::new(GearParams {
                teeth,
                ..Default::default()
            });
            let sec = root_section(&g, g.u_tip).unwrap();
            assert!(
                sec.s <= 0.0 && sec.s >= g.s_j,
                "s={} outside [{}, 0]",
                sec.s,
                g.s_j
            );
            let r = f64::hypot(sec.tangency[0], sec.tangency[1]);
            assert!(r >= g.rf - 1e-9 && r <= g.r_j + 1e-9, "tangency at r={r}");
        }
    }

    /// The load line must actually pass through the contact point and the
    /// base-circle tangent point — that is what makes it the line of action.
    #[test]
    fn load_line_is_tangent_to_the_base_circle() {
        let g = Gear::new(GearParams {
            teeth: 23,
            ..Default::default()
        });
        let (p, d) = flank_point_and_load_direction(&g, g.u_tip);
        // distance from the gear centre to the load line
        let dist = (p[0] * d[1] - p[1] * d[0]).abs();
        assert!(
            (dist - g.rb).abs() < 1e-9,
            "load line passes {dist} from the centre, base radius is {}",
            g.rb
        );
    }

    #[test]
    fn form_factor_falls_as_tooth_count_rises() {
        let mut last = f64::INFINITY;
        for teeth in [12u32, 17, 25, 40, 80, 150] {
            let g = Gear::new(GearParams {
                teeth,
                ..Default::default()
            });
            let y = root_section(&g, g.u_tip).unwrap().form_factor;
            assert!(y < last, "z={teeth}: Y_F {y} did not fall below {last}");
            last = y;
        }
    }

    #[test]
    fn positive_profile_shift_thickens_the_root_and_lowers_the_form_factor() {
        let mut last_chord = 0.0_f64;
        let mut last_yf = f64::INFINITY;
        for xi in [-4i32, -2, 0, 2, 4] {
            let g = Gear::new(GearParams {
                teeth: 20,
                profile_shift: f64::from(xi) * 0.1,
                ..Default::default()
            });
            let sec = root_section(&g, g.u_tip).unwrap();
            assert!(
                sec.root_chord > last_chord,
                "x={xi}: root chord did not grow"
            );
            assert!(sec.form_factor < last_yf, "x={xi}: Y_F did not fall");
            last_chord = sec.root_chord;
            last_yf = sec.form_factor;
        }
    }

    /// The drawn tangent leans **inward**: climbing the fillet toward the tip,
    /// the tooth narrows. A diagram that reconstructed the line from the 30°
    /// angle alone drew its mirror image, with the tangency point still correct,
    /// so this pins the sign as well as the angle.
    #[test]
    fn tangent_direction_leans_towards_the_centreline_going_up() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                profile_shift: 0.3,
                ..Default::default()
            },
        ] {
            let g = Gear::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let d = sec.tangent_direction;
            assert!(
                d[1] > 0.0,
                "z={}: tangent should point up the tooth",
                p.teeth
            );
            assert!(
                d[0] < 0.0,
                "z={}: on the +x side the tooth narrows going up, so the tangent \
                 must lean inward; got {d:?}",
                p.teeth
            );
            let angle = d[0].abs().atan2(d[1].abs()).to_degrees();
            assert!((angle - TANGENT_ANGLE_DEG).abs() < 1e-6, "angle {angle}");
            assert!(
                (f64::hypot(d[0], d[1]) - 1.0).abs() < 1e-12,
                "not a unit vector"
            );
        }
    }

    #[test]
    fn stress_correction_can_be_switched_off_for_comparison() {
        let g = Gear::new(GearParams::default());
        let sec = root_section(&g, g.u_tip).unwrap();
        assert!((sec.stress_correction(StressConcentration::None).unwrap() - 1.0).abs() < 1e-15);
        assert!(
            (sec.bending_factor(StressConcentration::None).unwrap() - sec.form_factor).abs()
                < 1e-15,
            "with no correction the bending factor must be the form factor alone"
        );
    }

    /// The correction rises as the fillet sharpens — that is the whole content
    /// of a notch factor, and it is checkable without trusting the coefficients.
    #[test]
    fn sharper_fillets_raise_the_stress_correction() {
        let mut last = 0.0_f64;
        for root_radius in [0.38_f64, 0.30, 0.20, 0.10, 0.05] {
            let g = Gear::new(GearParams {
                root_radius,
                ..Default::default()
            });
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let ys = sec.stress_correction(StressConcentration::Iso6336).unwrap();
            assert!(
                ys > last,
                "rho={root_radius}: Y_S {ys} did not exceed {last} for a blunter fillet"
            );
            assert!(ys > 1.0, "a notch cannot reduce stress");
            last = ys;
        }
    }

    #[test]
    fn notch_parameter_is_the_ratio_the_iso_fit_expects() {
        let g = Gear::new(GearParams::default());
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        let want = sec.root_chord / (2.0 * sec.fillet_curvature);
        assert!((sec.notch_parameter - want).abs() < 1e-12);
        assert!(
            sec.notch_parameter_in_range(),
            "q_s = {}",
            sec.notch_parameter
        );
    }

    /// The parabola must genuinely be tangent: the tangency point lies on it,
    /// and the slopes agree. Solved through an eliminated parameter, so both
    /// conditions are worth re-checking against the recovered `p`.
    #[test]
    fn lewis_parabola_is_tangent_to_the_fillet() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 9,
                ..Default::default()
            },
            GearParams {
                teeth: 12,
                profile_shift: -0.3,
                ..Default::default()
            },
            GearParams {
                teeth: 60,
                profile_shift: 0.3,
                ..Default::default()
            },
        ] {
            let g = Gear::new(p);
            let sec = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
            let pp = sec.parabola_p.unwrap();
            let vertex = sec.load_line_crossing[1];
            let (q, t) = fillet_point_and_tangent(&g, sec.s);

            // on the parabola
            let on = q[0] * q[0] - 4.0 * pp * (vertex - q[1]);
            assert!(
                on.abs() < 1e-9,
                "z={}: point off the parabola by {on:.2e}",
                p.teeth
            );
            // slopes agree
            let parabola_slope = -q[0] / (2.0 * pp);
            let fillet_slope = t[1] / t[0];
            assert!(
                (parabola_slope - fillet_slope).abs() < 1e-7,
                "z={}: slope {parabola_slope} vs {fillet_slope}",
                p.teeth
            );
        }
    }

    /// The parabola is the more conservative construction at every tooth count,
    /// and the gap does **not** close: the two converge to *different* rack
    /// limits (2.063 against 2.159), because they are different constructions
    /// rather than an approximation and its exact form.
    #[test]
    fn parabola_is_consistently_more_conservative() {
        for teeth in [9u32, 17, 60, 300, 1000] {
            let g = Gear::new(GearParams {
                teeth,
                ..Default::default()
            });
            let a = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
            let b = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
            assert!(
                b.root_chord < a.root_chord,
                "z={teeth}: parabola section not narrower"
            );
            assert!(
                b.form_factor > a.form_factor,
                "z={teeth}: parabola Y_F {} not above tangent {}",
                b.form_factor,
                a.form_factor
            );
        }
    }

    /// Which curve the parabola touches depends on the tooth, and getting this
    /// wrong is how the first implementation failed: a fillet-only search finds
    /// no solution at all on large teeth.
    #[test]
    fn parabola_touches_the_fillet_on_small_teeth_and_the_flank_on_large() {
        let small = Gear::new(GearParams {
            teeth: 17,
            ..Default::default()
        });
        let small_sec =
            root_section_with(&small, small.u_tip, CriticalSection::LewisParabola).unwrap();
        assert!(!small_sec.tangency_on_flank, "z=17 should touch the fillet");

        let large = Gear::new(GearParams {
            teeth: 1000,
            ..Default::default()
        });
        let large_sec =
            root_section_with(&large, large.u_tip, CriticalSection::LewisParabola).unwrap();
        assert!(large_sec.tangency_on_flank, "z=1000 should touch the flank");
    }

    /// Unlike the 30° tangent, the parabola construction follows the load point.
    /// That is the property the cantilever model is meant to have.
    #[test]
    fn only_the_parabola_moves_with_the_load_point() {
        let g = Gear::new(GearParams {
            teeth: 20,
            ..Default::default()
        });
        let low = g.u_tip * 0.6;
        let a1 = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        let a2 = root_section_with(&g, low, CriticalSection::TangentAngle).unwrap();
        assert!(
            (a1.s - a2.s).abs() < 1e-12,
            "the 30 degree section must not move"
        );

        let b1 = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
        let b2 = root_section_with(&g, low, CriticalSection::LewisParabola).unwrap();
        assert!(
            (b1.s - b2.s).abs() > 1e-6,
            "the parabola section must follow the load"
        );
    }

    /// The clamp is tested on the formula directly, by setting the parameter,
    /// because the interesting cases are at the extremes.
    ///
    /// It is not a hypothetical guard: see
    /// `large_teeth_with_a_sharp_cutter_leave_the_stated_range`.
    #[test]
    fn notch_parameter_is_clamped_for_the_fit_but_reported_raw() {
        let g = Gear::new(GearParams::default());
        let base = root_section(&g, g.u_tip).unwrap();

        let mut sharp = base;
        sharp.notch_parameter = 50.0; // far past the stated range
        assert!(!sharp.notch_parameter_in_range());
        assert!(
            (sharp.notch_parameter - 50.0).abs() < 1e-12,
            "the raw value must survive for reporting"
        );

        let mut sharper = base;
        sharper.notch_parameter = 500.0;
        // Both clamp to the same q_s, so the correction stops rising.
        let a = sharp
            .stress_correction(StressConcentration::Iso6336)
            .unwrap();
        let b = sharper
            .stress_correction(StressConcentration::Iso6336)
            .unwrap();
        assert!((a - b).abs() < 1e-12, "clamp is not holding: {a} vs {b}");

        let mut blunt = base;
        blunt.notch_parameter = 0.1;
        assert!(!blunt.notch_parameter_in_range());
        let at_floor = blunt
            .stress_correction(StressConcentration::Iso6336)
            .unwrap();
        let mut at_one = base;
        at_one.notch_parameter = 1.0;
        assert!(
            (at_floor
                - at_one
                    .stress_correction(StressConcentration::Iso6336)
                    .unwrap())
            .abs()
                < 1e-12
        );
    }

    /// On small and medium gears the notch parameter stays inside the ISO
    /// range whatever the cutter, because `ρ_F` there is governed by the
    /// trochoid rather than by the cutter tip radius: shrinking the corner from
    /// 0.38 to 0.005 module moves `q_s` only from 1.62 to 2.37 at z=17.
    #[test]
    fn ordinary_gears_keep_the_notch_parameter_in_range() {
        for root_radius in [0.38_f64, 0.2, 0.05, 0.005] {
            for teeth in [9u32, 17, 60] {
                let g = Gear::new(GearParams {
                    teeth,
                    root_radius,
                    ..Default::default()
                });
                let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
                assert!(
                    sec.notch_parameter_in_range(),
                    "z={teeth} rho={root_radius}: q_s = {} left the stated range",
                    sec.notch_parameter
                );
            }
        }
    }

    /// Large teeth are the exception, and they are why the clamp exists. On a
    /// flat tooth the trochoid no longer dominates the root curvature, so a
    /// sharp cutter carries straight through into `q_s`.
    ///
    /// The clamp then **under**-predicts the stress, so this is exactly the case
    /// a caller must be told about rather than have silently corrected.
    #[test]
    fn large_teeth_with_a_sharp_cutter_leave_the_stated_range() {
        let g = Gear::new(GearParams {
            teeth: 300,
            root_radius: 0.05,
            ..Default::default()
        });
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        assert!(
            sec.notch_parameter > NOTCH_PARAMETER_RANGE.end,
            "expected q_s past the range, got {}",
            sec.notch_parameter
        );
        assert!(!sec.notch_parameter_in_range());
        // and the reported value is the real one, not the clamped one
        assert!(sec.notch_parameter > 10.0);
    }

    /// The ISO correction is a notch factor. Where the parabola leaves the
    /// fillet there is no notch, and the combination must say so rather than
    /// substitute the involute's curvature — which produced a 17% cliff at
    /// z=150→151 while the form factor moved 0.03%.
    #[test]
    fn iso_correction_is_undefined_on_a_flank_tangency() {
        let g = Gear::new(GearParams {
            teeth: 1000,
            ..Default::default()
        });
        let sec = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
        assert!(sec.tangency_on_flank);
        assert!(sec
            .stress_correction(StressConcentration::Iso6336)
            .is_none());
        assert!(sec.bending_factor(StressConcentration::Iso6336).is_none());
        // the uncorrected factor is still perfectly well defined
        assert!(sec.bending_factor(StressConcentration::None).is_some());
    }

    #[test]
    fn a_severed_tooth_has_no_root_section() {
        let g = Gear::new(GearParams {
            teeth: 3,
            profile_shift: -0.5,
            ..Default::default()
        });
        assert!(g.severed);
        assert!(root_section(&g, 0.5).is_none());
    }

    // ------------------------------------------------------------ load ----

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

    #[test]
    fn torque_and_normal_force_are_exact_inverses() {
        let g = Gear::new(GearParams {
            teeth: 23,
            module: 2.0,
            helix_angle: 15.0,
            ..Default::default()
        });
        let load = Load::from_torque(&g, 4.5, 10.0);
        assert!((load.torque(&g) - 4.5).abs() < 1e-12);

        // The tangential force at the reference circle is the projection, and
        // it must equal torque / pitch radius independently.
        let expected_ft = 1000.0 * 4.5 / g.r;
        assert!(
            (load.tangential(&g) - expected_ft).abs() < 1e-9,
            "{} vs {expected_ft}",
            load.tangential(&g)
        );
    }

    #[test]
    fn bending_stress_scales_the_way_the_cantilever_model_says() {
        let g = Gear::new(GearParams {
            teeth: 25,
            ..Default::default()
        });
        let sec = root_section(&g, g.u_tip).unwrap();
        let s = |t: f64, b: f64| {
            bending_stress(
                &sec,
                &g,
                &Load::from_torque(&g, t, b),
                StressConcentration::None,
            )
            .unwrap()
        };

        let base = s(1.0, 10.0);
        // Linear in load...
        assert!((s(2.0, 10.0) - 2.0 * base).abs() < 1e-9);
        // ...and inversely proportional to face width.
        assert!((s(1.0, 20.0) - base / 2.0).abs() < 1e-9);
        assert!(base > 0.0);
    }

    /// `b_min` must not depend on the `b` the stress was evaluated at. It is the
    /// invariant that catches a stress which did not actually scale with face
    /// width — the failure a single spot check would sail past.
    #[test]
    fn minimum_face_width_is_independent_of_the_face_width_used() {
        let (g1, g2, mesh) = pair(19, 31);
        let path = ContactPath::new(&g1, &g2, &mesh).unwrap();
        let sec = root_section(&g1, path.roll_at(path.highest_single_pair())).unwrap();

        let (mut bend, mut cont) = (Vec::new(), Vec::new());
        for b in [1.0, 5.0, 12.5, 100.0] {
            let load = Load::from_torque(&g1, 3.0, b);
            let sf = bending_stress(&sec, &g1, &load, StressConcentration::None).unwrap();
            let sh = contact_stress(&path, &mesh, &load, 100_000.0).unwrap();
            bend.push(min_face_width_bending(sf, b, 200.0));
            cont.push(min_face_width_contact(sh.worst, b, 800.0));
        }
        for v in &bend {
            assert!(
                (v - bend[0]).abs() < 1e-9,
                "bending b_min drifted: {bend:?}"
            );
        }
        for v in &cont {
            assert!(
                (v - cont[0]).abs() < 1e-9,
                "contact b_min drifted: {cont:?}"
            );
        }
        assert!(bend[0] > 0.0 && cont[0] > 0.0);
    }

    /// Hertz, reached a second way: through the contact half-width.
    ///
    /// `b_h = √(4F'R/πE*)` and `p_max = 2F'/(π b_h)` is the textbook line-contact
    /// pair. Eliminating `b_h` gives `√(F'E*/πR)`, so agreement checks the
    /// algebra in `contact_stress` against a route that shares none of it.
    #[test]
    fn contact_stress_matches_the_half_width_route() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, &g2, &mesh).unwrap();
        let load = Load::from_torque(&g1, 2.0, 8.0);
        let e_star = 113_000.0;
        let cs = contact_stress(&path, &mesh, &load, e_star).unwrap();

        let f_prime = load.normal_force / load.face_width;
        let r = cs.relative_radius;
        let half_width = (4.0 * f_prime * r / (std::f64::consts::PI * e_star)).sqrt();
        let p_max = 2.0 * f_prime / (std::f64::consts::PI * half_width);
        assert!(
            (cs.worst - p_max).abs() / p_max < 1e-12,
            "{} vs {p_max}",
            cs.worst
        );
        assert!(half_width > 0.0 && half_width < r, "implausible half width");
    }

    /// `ρ₁ + ρ₂ = a_w sin α_w` everywhere on the path — the two flanks' local
    /// radii are complementary, which is what makes the relative radius peak at
    /// the pitch point and fall toward both ends.
    #[test]
    fn the_two_local_radii_sum_to_the_line_of_action() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, &g2, &mesh).unwrap();
        let sz = f64::from(mesh.z1) + f64::from(mesh.z2);
        let rb2 = mesh.a_w * f64::from(mesh.z2) / sz * mesh.alpha_w.cos();
        let want = mesh.a_w * mesh.alpha_w.sin();

        for i in 0..=20 {
            #[allow(clippy::cast_precision_loss)]
            let t = i as f64 / 20.0;
            let xi = -path.approach + t * (path.approach + path.recess);
            let rho1 = path.base_radius_1 * mesh.alpha_w.tan() + xi;
            let rho2 = rb2 * mesh.alpha_w.tan() - xi;
            assert!((rho1 + rho2 - want).abs() < 1e-9);
        }
    }

    /// Contact stress belongs to the *pair*, so it cannot depend on which gear
    /// the caller labelled 1. Checking only the inner single-pair boundary — as
    /// DESIGN.md §4.7 originally prescribed — breaks this, because the relative
    /// radius peaks on the recess side for a pinion and the approach side for a
    /// wheel.
    #[test]
    fn contact_stress_does_not_depend_on_which_gear_is_called_first() {
        for (za, zb) in [(17u32, 43u32), (13, 60), (25, 25), (43, 17), (60, 13)] {
            let (g1, g2, mesh) = pair(za, zb);
            let path = ContactPath::new(&g1, &g2, &mesh).unwrap();

            let m_rev = Mesh::new(&g2, &g1, MeshKind::External).unwrap();
            let path_rev = ContactPath::new(&g2, &g1, &m_rev).unwrap();

            // Same physical mesh and same transmitted power, so the same load
            // along the line of action.
            let load = Load::from_torque(&g1, 2.0, 8.0);
            let load_rev = Load {
                normal_force: load.normal_force,
                face_width: load.face_width,
            };

            let a = contact_stress(&path, &mesh, &load, 113_000.0).unwrap();
            let b = contact_stress(&path_rev, &m_rev, &load_rev, 113_000.0).unwrap();

            assert!(
                (a.worst - b.worst).abs() / a.worst < 1e-12,
                "z={za}/{zb}: {} vs {} when the labels are swapped",
                a.worst,
                b.worst
            );
            assert!((a.relative_radius - b.relative_radius).abs() < 1e-9);
            // The worst point is the same physical place. Its sign flips with
            // the labels, since ξ is measured toward gear 1's tip — except on a
            // symmetric mesh, where both single-pair boundaries are equally bad
            // and the tie-break picks the same one in both frames.
            assert!(
                (a.worst_position.abs() - b.worst_position.abs()).abs() < 1e-9,
                "z={za}/{zb}: worst at {} vs {}",
                a.worst_position,
                b.worst_position
            );
        }
    }

    #[test]
    fn contact_stress_is_worst_off_the_pitch_point_and_softens_with_a_softer_pair() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, &g2, &mesh).unwrap();
        let load = Load::from_torque(&g1, 2.0, 8.0);

        let steel = contact_stress(&path, &mesh, &load, 113_000.0).unwrap();
        // The single-pair point has the smaller relative radius, so it governs.
        assert!(steel.at_single_pair > steel.at_pitch_point);
        assert!((steel.worst - steel.at_single_pair).abs() < 1e-12);

        // A compliant pair spreads the contact and drops the pressure, as √E*.
        let poly = contact_stress(&path, &mesh, &load, 1_700.0).unwrap();
        assert!(poly.worst < steel.worst);
        let ratio = steel.worst / poly.worst;
        assert!(
            (ratio - (113_000.0f64 / 1_700.0).sqrt()).abs() < 1e-9,
            "contact stress should go as sqrt(E*), got ratio {ratio}"
        );
    }
}
