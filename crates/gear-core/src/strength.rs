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
use crate::hertz::elliptical_contact;
use crate::mesh::Mesh;
use crate::metrology::base_helix_angle;
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
///
/// # Why this survives the no-correction-factors policy
///
/// DESIGN.md §4.7 excludes the ISO correction factors — `Y_β`, `K_A`, `K_v`,
/// `K_Fβ`, `K_Fα`, `Z_ε`, `Z_β`. `Y_S` is kept, and the difference is not
/// special pleading:
///
/// - **It points the other way.** Those factors are mostly `≤ 1` for bending, so
///   omitting them is conservative. `Y_S ≥ 1` — typically 1.6 to 2.1. Dropping
///   it would report a nominal section stress well *below* the real peak, which
///   is the unconservative direction.
/// - **It is local, not population-calibrated.** It converts nominal stress at a
///   section into peak stress at a notch. Its inputs `s_Fn`, `h_Fe` and `ρ_F`
///   are measured off this gear's own generated profile, not looked up against a
///   population of test gears.
/// - **Its range is reported, not assumed.** See
///   [`RootSection::notch_parameter_in_range`]; a result that leaves the fit's
///   band says so instead of quietly returning a boundary value.
///
/// It remains a fit, and [`StressConcentration::None`] exists so any result can
/// be re-run without it.
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

/// What a gear is carrying.
///
/// # Why this stores torque and not a force
///
/// Every force in a gear mesh is a projection, and a projection is only defined
/// once you say *of what, onto which plane, at which radius*. There are at least
/// four in play here and they differ by factors of `cos α_t`, `cos α_w` and
/// `cos β_b`:
///
/// ```text
/// F_t   = 2000 T / d      tangential at the REFERENCE cylinder
///         2000 T / d'     tangential at the OPERATING cylinder
/// F_bt  = T / r_b         along the transverse line of action
/// F_bn  = F_bt / cos β_b  normal to the tooth flank
/// ```
///
/// Storing any one of them bakes a choice of radius and plane into a bare `f64`
/// that no longer says which it made. Torque does not: it is a property of the
/// shaft, invariant under every redefinition of a radius, and it is what the
/// specification takes as input and reports as output. So torque is what is
/// stored, and each projection is spelled out at the point of use, where the
/// plane it belongs to is visible.
///
/// An earlier revision stored `F_bt` under the name `normal_force`. Nothing it
/// computed was wrong, but the name asserted the normal plane while the value
/// was transverse — exactly the failure this arrangement is meant to make
/// impossible. See DESIGN.md §12.
///
/// # Sign and reference
///
/// A `Load` is quoted **against a particular gear**, since `T₁ ≠ T₂` across a
/// mesh. The accessors take that gear explicitly rather than assuming it. What
/// *is* shared by both gears is `F_bt`, by action and reaction — see
/// [`Load::transverse_line_of_action`].
#[derive(Clone, Copy, Debug)]
pub struct Load {
    /// Torque on the gear this load is quoted against, N·m.
    pub torque: f64,
    /// Face width, mm.
    pub face_width: f64,
}

impl Load {
    #[must_use]
    pub fn new(torque: f64, face_width: f64) -> Self {
        Self { torque, face_width }
    }

    /// Tangential force at the **reference** cylinder, N — ISO 6336's `F_t`.
    ///
    /// `F_t = 2000 T / d = 1000 T / r`. The 1000 converts N·m to N·mm, because
    /// every length in this crate is millimetres.
    #[must_use]
    pub fn tangential(&self, g: &Gear) -> f64 {
        1000.0 * self.torque / g.r
    }

    /// Force along the **transverse** line of action, N — `F_bt = T / r_b`.
    ///
    /// The exact lever arm for an involute is the base radius, so this relation
    /// is a geometric identity rather than a convention. It is also the one load
    /// quantity **both gears of a pair share**: action and reaction along the
    /// line of action are the same force, which is why contact stress — a
    /// property of the pair — is built on it.
    #[must_use]
    pub fn transverse_line_of_action(&self, g: &Gear) -> f64 {
        1000.0 * self.torque / g.rb
    }

    /// Force **normal to the tooth flank**, N — `F_bn = F_bt / cos β_b`.
    ///
    /// For a spur gear `β_b = 0` and this equals
    /// [`Self::transverse_line_of_action`]. For a helical gear it does not, and
    /// this is the force that actually presses the flanks together: the contact
    /// line is inclined at the base helix angle, so the transverse force is only
    /// its projection.
    #[must_use]
    pub fn normal_to_flank(&self, g: &Gear) -> f64 {
        self.transverse_line_of_action(g) / base_helix_angle(g).cos()
    }

    /// The same mesh load, re-quoted against the mating gear.
    ///
    /// `F_bt` is shared across the mesh, so `T₂ = T₁ · r_b2 / r_b1`. This is the
    /// **geometric** transfer only: efficiency losses belong to train
    /// accumulation (DESIGN.md §4.9), not here.
    ///
    /// Face width is carried across unchanged, since a `Load` describes what is
    /// being carried rather than by what.
    #[must_use]
    pub fn across_mesh(&self, from: &Gear, to: &Gear) -> Self {
        Self {
            torque: self.torque * to.rb / from.rb,
            face_width: self.face_width,
        }
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
/// # Helical gears
///
/// `F_t` is the **transverse** tangential force at the reference cylinder while
/// `m_n` is the **normal** module, and that pairing is deliberate — it is ISO
/// 6336-3's, and it is only consistent if `Y_F` is measured on the *normal*
/// section. So `section` must come from the virtual spur gear,
/// [`Gear::virtual_spur`], which is what [`bending_section`] returns.
///
/// Measuring `Y_F` on the transverse section and dividing by `m_n` — which an
/// earlier revision did — mixes the two planes and under-predicts the stress by
/// about `cos β` (6 % at 20°, 13 % at 30°). Spur gears are unaffected, since the
/// two sections coincide.
///
/// **No ISO correction factors are applied** — not `Y_β`, and not the `K` and `Z`
/// families either. This is a standing project policy, set out at the end of
/// DESIGN.md §4.7: their validated bands are narrow against modern designs, they
/// are only balanced as a complete set against `σ_Flim` values this project does
/// not have, and they buy precision at the cost of accuracy. Since `Y_β ≤ 1`,
/// leaving it out over-predicts stress — the safe direction — but it does mean a
/// helical result here is conservative against a published ISO rating by up to
/// about 25 % at high helix angle and overlap, and should not be compared to one
/// without saying so.
///
/// The notch factor `Y_S` is deliberately *not* in that category; see
/// [`StressConcentration`].
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

/// The critical section to rate a gear's bending on, loaded at the highest
/// point of single-pair contact.
///
/// One formula for spur and helical alike. The tooth bends as its **normal**
/// section, so both the form and the load point are taken on the virtual spur
/// gear ([`Gear::virtual_spur`]); for a spur gear that is the gear itself, by
/// construction rather than by a branch.
///
/// # Locating the load point without the mate
///
/// The highest point of single-pair contact is one base pitch back from the far
/// end of the path of contact, so measuring from the *tip* it depends only on
/// this gear's own geometry and the contact ratio:
///
/// ```text
/// u_load = u_tip − (ε_α − 1) · p_b / r_b
/// ```
///
/// [verified exact against the path-of-contact construction over seven meshes,
/// including reversed pairs.] That is what lets this take a scalar rather than
/// the mating gear, and it is what makes the helical case tractable: the same
/// relation applies on the virtual gear, using the **virtual** contact ratio
///
/// ```text
/// ε_αn = ε_α / cos² β_b
/// ```
///
/// Carrying the *real* gear's roll parameter across instead — which an earlier
/// revision did — puts the load at the wrong place on the flank, because the
/// virtual gear's involute is not the real one.
///
/// `transverse_contact_ratio` is `ε_α` from [`ContactPath::contact_ratio`].
#[must_use]
pub fn bending_section(g: &Gear, transverse_contact_ratio: f64) -> Option<RootSection> {
    let v = g.virtual_spur();
    // The virtual gear is a spur gear, so its transverse plane is the normal
    // plane: `v.mt` is m_n and `v.alpha_t` is α_n.
    let base_pitch = std::f64::consts::PI * v.mt * v.alpha_t.cos();
    let cos_bb = base_helix_angle(g).cos();
    let eps_n = transverse_contact_ratio / (cos_bb * cos_bb);

    let load_roll = v.u_tip - (eps_n - 1.0) * base_pitch / v.rb;
    root_section(&v, load_roll)
}

/// The lengthwise relative curvature of a parallel-axis, uncrowned mesh:
/// exactly zero.
///
/// Named rather than written as `0.0` at each call site, because it is a
/// *value of the general contact model* — the one at which the ellipse becomes
/// a line — and not a placeholder for something not yet passed. Every mesh this
/// crate builds today takes it.
pub const PARALLEL_AXES: f64 = 0.0;

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
    /// Relative radius of curvature at the worst point, mm, in the **normal**
    /// plane — the one the contact actually sees. Reported because it is what
    /// the number is really driven by. Equal to the transverse radius for a spur
    /// gear.
    pub relative_radius: f64,
}

/// Exact Hertzian contact for a meshing pair.
///
/// At a point `ξ` from the pitch point the two flanks are locally cylinders of
/// **transverse** radius `ρ₁ = r_b1 tan α_w + ξ` and `ρ₂ = r_b2 tan α_w − ξ`, so
///
/// ```text
/// 1/ρ_t = 1/ρ₁ ± 1/ρ₂               + external, − internal
/// σ_H   = max( σ_elliptical , √( (F' / ρ_n) · E* / π ) )
/// ```
///
/// `e_star` is the effective contact modulus `E*`, from
/// [`crate::material::contact_modulus`]. It is passed as a number rather than
/// taken from two materials so this stays a statement about mechanics.
///
/// # The lengthwise curvature, and why there is no second function
///
/// `lengthwise_curvature` is `1/R_L`, the relative curvature **along** the
/// contact line, in 1/mm. It is exactly zero for every mesh this crate builds
/// today — parallel axes, uncrowned flanks — and positive only for crossed axes
/// or crowning. It is the single parameter that unifies point and line contact
/// (DESIGN.md §4.7), and the reason the crossed-axis work adds an argument here
/// rather than a second function chosen by stage type.
///
/// The general elliptical solution ([`crate::hertz`]) is evaluated
/// unconditionally, and at `1/R_L = 0` its peak pressure is **exactly zero**:
/// the patch lengthens without bound and a finite load spread over it presses
/// on nothing. So the `max` above returns the line term, bit for bit, with no
/// branch to choose it. That is the acceptance gate — every existing contact
/// check and `gear-cli strength 17 43 2.0` to the last digit — and it is passed
/// by construction rather than by two routes happening to agree.
///
/// The `max` itself is not a fudge between two models; it is where the *body*
/// takes over from elasticity. A tooth has finite face width, so an ellipse
/// longer than the contact line is truncated by the tooth rather than by the
/// contact solution, and in that regime the line term — the same load spread
/// over the length that actually exists — is the physical one. The two cross
/// once. Near the crossing the truth sits slightly above both, since a
/// truncated ellipse concentrates load more than a uniform line does; that is
/// the honest limit of the expression rather than something papered over.
///
/// # Helical gears
///
/// Three things change together, and they nearly cancel:
///
/// ```text
/// ρ_n = ρ_t / cos β_b        curvature is seen in the NORMAL plane
/// F_bn = F_bt / cos β_b      the flank force, not its transverse projection
/// L    = b / cos β_b         one contact line, inclined across the face
/// ```
///
/// Substituting all three collapses to `σ_H = √((F_bt/b) · cos β_b / ρ_t ·
/// E*/π)`, so a helical mesh comes out lower than the same transverse geometry
/// by exactly `√(cos β_b)` — 3 % at β = 20°, 6 % at β = 30°. That benefit is
/// pure geometry: longer contact line and flatter normal-plane curvature. It is
/// **not** the extra benefit helical gears get from having several contact lines
/// engaged at once, which is load sharing and is deferred (DESIGN.md §4.7).
/// Assuming a single line is the conservative reading and is continuous with the
/// spur case at β = 0.
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
    g1: &Gear,
    lengthwise_curvature: f64,
    load: &Load,
    e_star: f64,
) -> Option<ContactStress> {
    // The curvatures come from the mesh, which owns the operating geometry and
    // the one signed relation both kinds obey — see `Mesh::curvature_radii`.
    // Re-deriving `r_b2` here is what previously got an internal pair wrong.

    // F_bt is shared by both gears of the pair, so which gear the load was
    // quoted against does not survive into the answer.
    let f_bt = load.transverse_line_of_action(g1);
    // The elliptical patch carries the whole flank force at a point, where the
    // line carries it per unit length; F_bn is the same force either way.
    let f_bn = load.normal_to_flank(g1);
    let cos_bb = base_helix_angle(g1).cos();

    let at = |xi: f64| -> Option<(f64, f64)> {
        let inv_rho_t = mesh.relative_curvature(xi)?;
        // F_bn / L = (F_bt/cos β_b) / (b/cos β_b) = F_bt / b, and
        // 1/ρ_n = cos β_b / ρ_t. Written out rather than pre-cancelled so the
        // two plane changes stay visible.
        let f_per_length = f_bt / load.face_width;
        let inv_rho_n = cos_bb * inv_rho_t;
        let line = (f_per_length * inv_rho_n * e_star / std::f64::consts::PI).sqrt();
        // Zero at zero lengthwise curvature, so this `max` is the line term
        // unchanged for every mesh built today. A patch that cannot exist —
        // no load, say — carries no pressure, which the line term still can.
        let elliptical = elliptical_contact(lengthwise_curvature, inv_rho_n, f_bn, e_star)
            .map_or(0.0, |c| c.max_pressure);
        Some((line.max(elliptical), 1.0 / inv_rho_n))
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
    use crate::mesh::MeshKind;
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

    /// The three force projections must be mutually consistent, and each must
    /// name the plane it is in. This is the check the old `normal_force` field
    /// could not have passed: it was transverse but called normal.
    #[test]
    fn the_force_projections_are_mutually_consistent() {
        for beta in [0.0, 15.0, 30.0] {
            let g = Gear::new(GearParams {
                teeth: 23,
                module: 2.0,
                helix_angle: beta,
                ..Default::default()
            });
            let load = Load::new(4.5, 10.0);

            // F_t = T / r, independently of anything else.
            assert!((load.tangential(&g) - 1000.0 * 4.5 / g.r).abs() < 1e-9);
            // F_bt = F_t / cos α_t — the transverse projection onto the line of
            // action.
            let f_bt = load.transverse_line_of_action(&g);
            assert!((f_bt - load.tangential(&g) / g.alpha_t.cos()).abs() < 1e-9);
            // F_bn = F_bt / cos β_b, and for a spur gear the two coincide.
            let f_bn = load.normal_to_flank(&g);
            let cos_bb = base_helix_angle(&g).cos();
            assert!((f_bn - f_bt / cos_bb).abs() < 1e-9);
            if beta == 0.0 {
                assert!(
                    (f_bn - f_bt).abs() < 1e-12,
                    "spur: normal must equal transverse"
                );
            } else {
                assert!(
                    f_bn > f_bt,
                    "beta={beta}: the flank force must exceed its projection"
                );
            }
        }
    }

    /// `F_bt` is shared across a mesh, so re-quoting the load against the other
    /// gear must leave it unchanged. That is the invariant that replaced storing
    /// the force directly.
    #[test]
    fn a_load_carried_across_a_mesh_keeps_the_same_force_on_the_line_of_action() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25)] {
            let (g1, g2, _) = pair(z1, z2);
            let l1 = Load::new(2.0, 8.0);
            let l2 = l1.across_mesh(&g1, &g2);

            assert!(
                (l1.transverse_line_of_action(&g1) - l2.transverse_line_of_action(&g2)).abs()
                    < 1e-9,
                "z={z1}/{z2}: F_bt changed across the mesh"
            );
            // Torque scales with the ratio, and the round trip is exact.
            let ratio = f64::from(z2) / f64::from(z1);
            assert!((l2.torque / l1.torque - ratio).abs() < 1e-9);
            assert!((l2.across_mesh(&g2, &g1).torque - l1.torque).abs() < 1e-12);
        }
    }

    #[test]
    fn bending_stress_scales_the_way_the_cantilever_model_says() {
        let g = Gear::new(GearParams {
            teeth: 25,
            ..Default::default()
        });
        let sec = root_section(&g, g.u_tip).unwrap();
        let s = |t: f64, b: f64| {
            bending_stress(&sec, &g, &Load::new(t, b), StressConcentration::None).unwrap()
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
        let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
        let sec = root_section(&g1, path.roll_at(path.highest_single_pair())).unwrap();

        let (mut bend, mut cont) = (Vec::new(), Vec::new());
        for b in [1.0, 5.0, 12.5, 100.0] {
            let load = Load::new(3.0, b);
            let sf = bending_stress(&sec, &g1, &load, StressConcentration::None).unwrap();
            let sh = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 100_000.0).unwrap();
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
        let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
        let load = Load::new(2.0, 8.0);
        let e_star = 113_000.0;
        let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star).unwrap();

        let f_prime = load.transverse_line_of_action(&g1) / load.face_width;
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

    /// **The acceptance gate for the contact unification** (DESIGN.md §4.7).
    ///
    /// At `1/R_L = 0` the general elliptical solution must not perturb the line
    /// result — not "agree to 1e-12", but return the identical `f64`. It can,
    /// because the elliptical patch's peak pressure is exactly zero there and
    /// the `max` therefore selects the untouched line expression. Anything less
    /// than bit equality means the line term was rewritten rather than carried
    /// across, which is the one way this step can silently move an answer.
    #[test]
    fn the_general_form_is_bit_identical_to_line_contact_at_parallel_axes() {
        for (z1, z2) in [(17u32, 43u32), (13, 60), (25, 25), (19, 31)] {
            for beta in [0.0, 15.0, 30.0] {
                let g1 = Gear::new(GearParams {
                    teeth: z1,
                    helix_angle: beta,
                    ..Default::default()
                });
                let g2 = Gear::new(GearParams {
                    teeth: z2,
                    helix_angle: -beta,
                    ..Default::default()
                });
                let mesh = Mesh::new(&g1, &g2, MeshKind::External).unwrap();
                let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
                let load = Load::new(2.0, 10.0);
                let e_star = 113_000.0;
                let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star).unwrap();

                // The line formula, spelled out in the same order the function
                // evaluates it. The duplication *is* the assertion: it says
                // this exact expression survived the unification. Reaching the
                // same number by a different order of operations would only
                // prove agreement to an ulp or so, which is not the claim —
                // that check is `contact_stress_matches_the_half_width_route`.
                let sum_z = f64::from(mesh.z1) + f64::from(mesh.z2);
                let rb2 = mesh.a_w * f64::from(mesh.z2) / sum_z * mesh.alpha_w.cos();
                let rho1 = path.base_radius_1 * mesh.alpha_w.tan() + cs.worst_position;
                let rho2 = rb2 * mesh.alpha_w.tan() - cs.worst_position;
                let inv_rho_n = base_helix_angle(&g1).cos() * (1.0 / rho1 + 1.0 / rho2);
                let f_prime = load.transverse_line_of_action(&g1) / load.face_width;
                let line = (f_prime * inv_rho_n * e_star / std::f64::consts::PI).sqrt();
                assert_eq!(
                    cs.worst, line,
                    "z={z1}/{z2} beta={beta}: the general form moved the line answer"
                );
            }
        }
    }

    /// The lengthwise curvature is a one-way parameter: it can only concentrate
    /// the load further, so the stress rises monotonically with it and returns
    /// continuously to the line value as it goes to zero. There is no jump at
    /// the parallel-axis point, which is the property that lets one function
    /// serve both regimes.
    #[test]
    fn stress_rises_monotonically_with_lengthwise_curvature_and_returns_to_the_line() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
        let load = Load::new(2.0, 10.0);
        let e_star = 113_000.0;

        let line = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star)
            .unwrap()
            .worst;

        // Coming down toward parallel axes, the answer converges on the line
        // value from above and never below it.
        let mut previous = f64::INFINITY;
        for exponent in 0..12 {
            let curvature = 10.0_f64.powi(-exponent);
            let worst = contact_stress(&path, &mesh, &g1, curvature, &load, e_star)
                .unwrap()
                .worst;
            assert!(
                worst >= line,
                "curvature {curvature}: {worst} fell below the line value {line}"
            );
            assert!(
                worst <= previous,
                "curvature {curvature}: stress must fall as the mesh flattens"
            );
            previous = worst;
        }
        assert!(
            (previous - line).abs() < 1e-9 * line,
            "at 1e-11/mm the answer should have returned to the line value: \
             {previous} vs {line}"
        );

        // And a curvature comparable with the profile's makes the point contact
        // govern outright, which is the crossed-axis regime.
        let crossed = contact_stress(&path, &mesh, &g1, 0.5, &load, e_star)
            .unwrap()
            .worst;
        assert!(
            crossed > 2.0 * line,
            "a point contact should be far worse than a line: {crossed} vs {line}"
        );
    }

    /// `ρ₁ + ρ₂ = a_w sin α_w` everywhere on the path — the two flanks' local
    /// radii are complementary, which is what makes the relative radius peak at
    /// the pitch point and fall toward both ends.
    #[test]
    fn the_two_local_radii_sum_to_the_line_of_action() {
        let (g1, g2, mesh) = pair(17, 43);
        let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
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

    // --------------------------------------------------------- helical ----

    /// A spur gear is its own normal section, so the virtual construction must
    /// be the identity there — otherwise every spur result would shift.
    #[test]
    fn the_virtual_spur_gear_of_a_spur_gear_is_itself() {
        let g = Gear::new(GearParams {
            teeth: 25,
            ..Default::default()
        });
        let v = g.virtual_spur();
        assert!((v.z - g.z).abs() < 1e-15);
        assert!((v.r - g.r).abs() < 1e-15);
        assert!((v.rb - g.rb).abs() < 1e-15);

        // Loaded at the same place, the two must agree exactly.
        let eps = 1.6;
        let pb = std::f64::consts::PI * g.mt * g.alpha_t.cos();
        let a = root_section(&g, g.u_tip - (eps - 1.0) * pb / g.rb).unwrap();
        let b = bending_section(&g, eps).unwrap();
        assert!((a.form_factor - b.form_factor).abs() < 1e-15);
    }

    /// `z_n = z / cos³β`, and the virtual gear is a genuine spur gear cut with
    /// the normal rack.
    #[test]
    fn the_virtual_spur_gear_has_the_iso_tooth_count_and_the_normal_rack() {
        for beta in [10.0, 20.0, 30.0, 45.0] {
            let g = Gear::new(GearParams {
                teeth: 30,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let b = beta.to_radians();

            assert!(
                (v.z - 30.0 / b.cos().powi(3)).abs() < 1e-12,
                "beta={beta}: z_n = {}",
                v.z
            );
            // It is a spur gear, in the normal plane, with the normal module.
            assert!(v.beta.abs() < 1e-15);
            assert!((v.alpha_t - g.alpha_n).abs() < 1e-12);
            assert!((v.mt - g.params.module).abs() < 1e-12);
            // ...and it always has more teeth than the real gear, which is why
            // a helical tooth is stronger in bending than its count suggests.
            assert!(v.z > g.z);
        }
    }

    /// Measuring the form on the transverse section — what an earlier revision
    /// did — is not the same as measuring it on the normal one. Compared at the
    /// *same* load point, so this isolates the section change from the load
    /// point change that also comes with the virtual gear.
    #[test]
    fn the_normal_section_is_what_bends_and_it_differs_from_the_transverse_one() {
        let mut previous = 0.0;
        for beta in [0.0, 15.0, 30.0] {
            let g = Gear::new(GearParams {
                teeth: 20,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let roll = 0.35;
            let transverse = root_section(&g, roll).unwrap().form_factor;
            let normal = root_section(&v, roll).unwrap().form_factor;
            let gap = (normal - transverse).abs() / transverse;

            if beta == 0.0 {
                assert!(gap < 1e-15, "spur sections must coincide exactly");
            } else {
                assert!(gap > 0.005, "beta={beta}: sections differ by only {gap:.4}");
                assert!(gap > previous, "the gap must widen with the helix angle");
            }
            previous = gap;
        }
    }

    /// The load point moves too, and it must move the way ISO says: the virtual
    /// contact ratio is `ε_α / cos² β_b`, so a helical gear is loaded further
    /// down its (virtual) flank than the transverse contact ratio alone implies.
    #[test]
    fn the_bending_load_point_uses_the_virtual_contact_ratio() {
        for beta in [0.0, 15.0, 30.0] {
            let g = Gear::new(GearParams {
                teeth: 20,
                helix_angle: beta,
                ..Default::default()
            });
            let v = g.virtual_spur();
            let eps = 1.55;
            let cos_bb = base_helix_angle(&g).cos();
            let pbn = std::f64::consts::PI * v.mt * v.alpha_t.cos();

            let want =
                root_section(&v, v.u_tip - (eps / (cos_bb * cos_bb) - 1.0) * pbn / v.rb).unwrap();
            let got = bending_section(&g, eps).unwrap();
            assert!(
                (got.form_factor - want.form_factor).abs() < 1e-15,
                "beta={beta}"
            );

            // At beta = 0 the virtual contact ratio IS the transverse one.
            if beta == 0.0 {
                let plain = root_section(&g, g.u_tip - (eps - 1.0) * pbn / g.rb).unwrap();
                assert!((got.form_factor - plain.form_factor).abs() < 1e-15);
            }
        }
    }

    /// `ε_αn = ε_α/cos²β_b` is ISO's relation, not an identity: building the
    /// virtual pair and measuring its contact ratio directly gives a slightly
    /// different number, because the virtual gear keeps the addendum in normal
    /// modules and so its tip circle is not in exact correspondence with the
    /// real one. This pins down what that modelling gap actually costs, so the
    /// approximation is a measured quantity rather than an assumption.
    #[test]
    fn the_virtual_contact_ratio_relation_barely_moves_the_answer() {
        for (beta, spread) in [(10.0, 0.0004), (20.0, 0.0012), (30.0, 0.0020)] {
            let g = Gear::new(GearParams {
                teeth: 17,
                helix_angle: beta,
                ..Default::default()
            });
            let eps = 1.5;
            let a = bending_section(&g, eps).unwrap();
            // Perturb the contact ratio by the observed disagreement between the
            // two routes and see how far the form factor moves.
            let b = bending_section(&g, eps * (1.0 + spread)).unwrap();
            let shift = (b.form_factor - a.form_factor).abs() / a.form_factor;
            assert!(
                shift < 0.005,
                "beta={beta}: a {spread:.4} contact-ratio gap moved Y_F by {shift:.5}"
            );
        }
    }

    /// Helical contact comes out below the equivalent transverse geometry by
    /// exactly `√(cos β_b)` — the combined effect of a longer contact line, a
    /// larger flank force and a flatter normal-plane curvature. Anything else
    /// means one of the three `cos β_b` factors is missing or doubled.
    #[test]
    fn helical_contact_stress_falls_by_the_square_root_of_the_base_helix_cosine() {
        let load = Load::new(2.0, 8.0);
        for beta in [10.0, 20.0, 30.0] {
            let g1 = Gear::new(GearParams {
                teeth: 17,
                helix_angle: beta,
                ..Default::default()
            });
            let g2 = Gear::new(GearParams {
                teeth: 43,
                helix_angle: -beta,
                ..Default::default()
            });
            let mesh = Mesh::new(&g1, &g2, MeshKind::External).unwrap();
            let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
            let cs = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();

            // The transverse geometry itself changes with beta (m_t grows), so
            // compare against the same mesh computed without the plane change
            // rather than against the spur mesh directly.
            let cos_bb = base_helix_angle(&g1).cos();
            let f_bt = load.transverse_line_of_action(&g1);
            // relative_radius is reported in the normal plane, ρ_n = ρ_t/cos β_b.
            let rho_t = cs.relative_radius * cos_bb;
            let transverse_only =
                ((f_bt / load.face_width) / rho_t * 113_000.0 / std::f64::consts::PI).sqrt();

            let ratio = cs.worst / transverse_only;
            assert!(
                (ratio - cos_bb.sqrt()).abs() < 1e-12,
                "beta={beta}: ratio {ratio} vs sqrt(cos beta_b) {}",
                cos_bb.sqrt()
            );
            assert!(cs.worst < transverse_only);
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
            let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();

            let m_rev = Mesh::new(&g2, &g1, MeshKind::External).unwrap();
            let path_rev = ContactPath::new(&g2, g1.ra, &m_rev).unwrap();

            // Same physical mesh and same transmitted power, so the same load
            // along the line of action.
            let load = Load::new(2.0, 8.0);
            let load_rev = load.across_mesh(&g1, &g2);

            let a = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();
            let b = contact_stress(&path_rev, &m_rev, &g2, PARALLEL_AXES, &load_rev, 113_000.0)
                .unwrap();

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
        let path = ContactPath::new(&g1, g2.ra, &mesh).unwrap();
        let load = Load::new(2.0, 8.0);

        let steel = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 113_000.0).unwrap();
        // The single-pair point has the smaller relative radius, so it governs.
        assert!(steel.at_single_pair > steel.at_pitch_point);
        assert!((steel.worst - steel.at_single_pair).abs() < 1e-12);

        // A compliant pair spreads the contact and drops the pressure, as √E*.
        let poly = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, 1_700.0).unwrap();
        assert!(poly.worst < steel.worst);
        let ratio = steel.worst / poly.worst;
        assert!(
            (ratio - (113_000.0f64 / 1_700.0).sqrt()).abs() < 1e-9,
            "contact stress should go as sqrt(E*), got ratio {ratio}"
        );
    }
}
