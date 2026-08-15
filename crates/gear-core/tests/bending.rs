//! The bending construction, checked against a limit computed independently.
//!
//! Comparing a form factor to a remembered table value proves only that two
//! sources agree, and a misremembered table is worse than no table. Instead this
//! derives the **rack limit** — the tooth a standard rack cuts as z goes to
//! infinity, whose geometry is straight flanks and a circular fillet — in closed
//! form, and checks the generated gear converges to it.
//!
//! That comparison shares no code with the implementation: the reference below
//! never mentions an involute, a trochoid, or `gear_core::strength`.

#![allow(clippy::unwrap_used)]

use gear_core::strength::{root_section, TANGENT_ANGLE_DEG};
use gear_core::{Gear, GearParams};

/// Closed-form critical section of a rack-cut tooth: `(s_Fn/m, h_Fe/m, Y_F)`.
///
/// Straight flanks at `α` to the centreline, circular fillet of radius `ρ`
/// tangent to both the flank and the root line, load at the tip corner along the
/// flank normal. `y` is measured up from the root line.
fn rack_limit(alpha_deg: f64, ha: f64, hf: f64, rho: f64) -> (f64, f64, f64) {
    let a = alpha_deg.to_radians();
    let ta = a.tan();

    let x_c = std::f64::consts::PI / 4.0 + (hf - rho) * ta + rho / a.cos();
    let y_c = rho;

    // On the fillet the direction from the centre sweeps from (0,-1) at the root
    // line to -(cos α, sin α) at the flank tangency; at direction angle τ the
    // tangent makes (90° − τ) with the centreline.
    let tau = (90.0 - TANGENT_ANGLE_DEG).to_radians();
    let x_t = x_c - rho * tau.sin();
    let y_t = y_c - rho * tau.cos();
    let s_fn = 2.0 * x_t;

    let x_tip = std::f64::consts::PI / 4.0 - ha * ta;
    let y_tip = hf + ha;
    let (dx, dy) = (a.cos(), a.sin());
    let y_cross = y_tip + (-x_tip / dx) * dy;
    let h_fe = y_cross - y_t;

    let y_f = 6.0 * h_fe * a.cos() / (s_fn * s_fn * a.cos());
    (s_fn, h_fe, y_f)
}

/// The generated gear must approach the rack as its tooth count grows, and keep
/// approaching it — a curve that levels off somewhere else is the failure this
/// is looking for.
#[test]
fn form_factor_converges_to_the_rack_limit() {
    for (alpha, rho) in [(20.0_f64, 0.38_f64), (25.0, 0.3), (14.5, 0.2)] {
        let (want_s, want_h, want_y) = rack_limit(alpha, 1.0, 1.25, rho);

        let mut previous_gap = f64::INFINITY;
        for teeth in [60u32, 250, 1000, 4000] {
            let g = Gear::new(GearParams {
                teeth,
                pressure_angle: alpha,
                root_radius: rho,
                ..Default::default()
            });
            let sec = root_section(&g, g.u_tip).unwrap();
            let gap = (sec.form_factor - want_y).abs();
            assert!(
                gap < previous_gap,
                "α={alpha} ρ={rho} z={teeth}: Y_F {} moved away from the rack limit {want_y}",
                sec.form_factor
            );
            previous_gap = gap;
        }

        // At four thousand teeth the gear is a rack to within a fraction of a
        // percent, and every ingredient must match, not just the result.
        let g = Gear::new(GearParams {
            teeth: 4000,
            pressure_angle: alpha,
            root_radius: rho,
            ..Default::default()
        });
        let sec = root_section(&g, g.u_tip).unwrap();
        let m = g.params.module;
        assert!(
            (sec.root_chord / m - want_s).abs() < 5e-3,
            "α={alpha}: s_Fn/m {} vs rack {want_s}",
            sec.root_chord / m
        );
        assert!(
            (sec.moment_arm / m - want_h).abs() < 5e-3,
            "α={alpha}: h_Fe/m {} vs rack {want_h}",
            sec.moment_arm / m
        );
        assert!(
            (sec.form_factor - want_y).abs() < 5e-3,
            "α={alpha}: Y_F {} vs rack {want_y}",
            sec.form_factor
        );
        println!(
            "α={alpha:>5} ρ={rho}: Y_F {:.6} vs rack limit {want_y:.6}",
            sec.form_factor
        );
    }
}
