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

use crate::profile::Gear;
use crate::solve::{brent, Tol};

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
    if g.severed || !g.u_j.is_finite() || !load_roll.is_finite() {
        return None;
    }

    // Along the fillet the tangent angle to the centreline sweeps from near zero
    // at the junction to 90° at the root circle, so this is monotone and the
    // bracket is the fillet itself.
    let target = TANGENT_ANGLE_DEG.to_radians().tan();
    let angle_excess = |s: f64| {
        let (_, t) = fillet_point_and_tangent(g, s);
        t[0].abs() - target * t[1].abs()
    };
    let s = brent(angle_excess, g.s_j, 0.0, Tol::default())?;

    let (tangency, raw_tangent) = fillet_point_and_tangent(g, s);
    let root_chord = 2.0 * tangency[0].abs();
    // Orient it up the tooth (towards the tip) so the direction is unambiguous.
    let len = f64::hypot(raw_tangent[0], raw_tangent[1]);
    let sign = if raw_tangent[1] < 0.0 { -1.0 } else { 1.0 };
    let tangent_direction = [sign * raw_tangent[0] / len, sign * raw_tangent[1] / len];

    let (load_point, dir) = flank_point_and_load_direction(g, load_roll);
    // Where the load line crosses the tooth centreline (x = 0).
    if dir[0].abs() < 1e-12 {
        return None;
    }
    let t = -load_point[0] / dir[0];
    let crossing = [0.0, load_point[1] + t * dir[1]];

    let moment_arm = crossing[1] - tangency[1];
    // cos α_Fen is the share of the load acting across the tooth; the load
    // direction's x-component is exactly that.
    let load_angle = dir[0].abs().clamp(-1.0, 1.0).acos();

    let m = g.params.module;
    let form_factor =
        6.0 * (moment_arm / m) * load_angle.cos() / ((root_chord / m).powi(2) * g.alpha_n.cos());

    Some(RootSection {
        s,
        notch_parameter: root_chord / (2.0 * fillet_curvature_radius(g, s)),
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
    #[must_use]
    pub fn stress_correction(&self, model: StressConcentration) -> f64 {
        match model {
            StressConcentration::None => 1.0,
            StressConcentration::Iso6336 => {
                let l = self.root_chord / self.moment_arm;
                (1.2 + 0.13 * l) * self.notch_parameter.powf(1.0 / (1.21 + 2.3 / l))
            }
        }
    }

    /// Whether the notch parameter falls where the ISO fit is stated.
    #[must_use]
    pub fn notch_parameter_in_range(&self) -> bool {
        NOTCH_PARAMETER_RANGE.contains(&self.notch_parameter)
    }

    /// `Y_F · Y_S`: the full geometry factor multiplying `F_t / (b m)`.
    #[must_use]
    pub fn bending_factor(&self, model: StressConcentration) -> f64 {
        self.form_factor * self.stress_correction(model)
    }
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
            let sec = root_section(&g, g.u_tip).unwrap();
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
            let sec = root_section(&g, g.u_tip).unwrap();
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
        assert!((sec.stress_correction(StressConcentration::None) - 1.0).abs() < 1e-15);
        assert!(
            (sec.bending_factor(StressConcentration::None) - sec.form_factor).abs() < 1e-15,
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
            let sec = root_section(&g, g.u_tip).unwrap();
            let ys = sec.stress_correction(StressConcentration::Iso6336);
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
        let sec = root_section(&g, g.u_tip).unwrap();
        let want = sec.root_chord / (2.0 * sec.fillet_curvature);
        assert!((sec.notch_parameter - want).abs() < 1e-12);
        assert!(
            sec.notch_parameter_in_range(),
            "q_s = {}",
            sec.notch_parameter
        );
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
}
