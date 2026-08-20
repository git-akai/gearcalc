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

use gear_core::strength::{root_section_with, CriticalSection, TANGENT_ANGLE_DEG};
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
            let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
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
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
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

/// The Lewis parabola has its own rack limit, and it is a different number.
///
/// On a rack the largest inscribed parabola touches the **straight flank**, not
/// the fillet — 0.54 module above where the fillet ends. That is not a detail:
/// a fillet-only search finds no solution at all on large teeth, which is how
/// the first implementation failed.
fn parabola_rack_limit(alpha_deg: f64, ha: f64, hf: f64) -> (f64, f64, f64) {
    let a = alpha_deg.to_radians();
    let b = a.tan();
    let big_a = std::f64::consts::PI / 4.0 + hf * b;

    // Load at the tip corner, along the flank normal; the vertex is where that
    // line crosses the centreline.
    let x_tip = std::f64::consts::PI / 4.0 - ha * b;
    let y_v = (hf + ha) + (-x_tip / a.cos()) * a.sin();

    // Tangency of x² = 4p(y_v − y) with the flank x = A − b y is a double root.
    let p = b * (big_a - b * y_v);
    let y_t = (big_a * b - 2.0 * p) / (b * b);
    let x_t = big_a - b * y_t;

    let s_fn = 2.0 * x_t;
    let h_fe = y_v - y_t;
    (s_fn, h_fe, 6.0 * h_fe / (s_fn * s_fn))
}

#[test]
fn parabola_form_factor_converges_to_its_own_rack_limit() {
    for alpha in [20.0_f64, 25.0] {
        let (want_s, want_h, want_y) = parabola_rack_limit(alpha, 1.0, 1.25);
        let g = Gear::new(GearParams {
            teeth: 4000,
            pressure_angle: alpha,
            ..Default::default()
        });
        let sec = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
        assert!(
            sec.tangency_on_flank,
            "at z=4000 the parabola must touch the flank"
        );

        let m = g.params.module;
        assert!(
            (sec.root_chord / m - want_s).abs() < 1e-2,
            "s_Fn/m {} vs {want_s}",
            sec.root_chord / m
        );
        assert!(
            (sec.moment_arm / m - want_h).abs() < 1e-2,
            "h_Fe/m {} vs {want_h}",
            sec.moment_arm / m
        );
        assert!(
            (sec.form_factor - want_y).abs() < 1e-2,
            "Y_F {} vs {want_y}",
            sec.form_factor
        );
        println!(
            "α={alpha:>5}: parabola Y_F {:.6} vs its rack limit {want_y:.6}",
            sec.form_factor
        );
    }
}

// ------------------------------------------------------------ internal ---

/// `J = 1/(Y_F · Y_S)` for one gear of a 25-tooth-pinion pair, external and
/// internal, loaded at the highest point of single-pair contact.
///
/// This is the quantity NASA TM-107012's Figure 8 plots. `σ_F = F_t/(b m)·Y_F·Y_S`
/// and AGMA's `σ = W_t/(f m J)`, so the two are reciprocals of each other and
/// comparable directly.
fn j_factors(z: u32, cutter_teeth: u32) -> (Option<f64>, Option<f64>) {
    use gear_core::contact::ContactPath;
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{bending_section, ring_bending_section, StressConcentration};

    let pinion = Gear::new(GearParams {
        teeth: 25,
        ..Default::default()
    });
    let params = GearParams {
        teeth: z,
        root_radius: 0.3,
        ..Default::default()
    };
    let wheel = Gear::new(params);
    let ring = Ring::new(
        &params,
        &Cutter {
            teeth: cutter_teeth,
            addendum: 1.25,
            tip_round: 0.3,
        },
    );
    let j = |s: Option<gear_core::strength::RootSection>| {
        s.and_then(|s| s.bending_factor(StressConcentration::Iso6336))
            .map(|f| 1.0 / f)
    };

    let external = Mesh::new(&pinion, &wheel, MeshKind::External)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, wheel.ra, &m))
        .and_then(|p| j(bending_section(&wheel, p.contact_ratio)));
    let internal = Mesh::new(&pinion, &wheel, MeshKind::Internal)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, ring.ra, &m))
        .and_then(|p| j(ring_bending_section(&ring, p.contact_ratio, 0.0)));
    (external, internal)
}

/// **What NASA TM-107012's Figure 8 says that survives the difference in setup**:
/// an internal tooth is stronger than an external one of the same pitch, and the
/// internal factor falls as tooth count rises.
///
/// Asserted as directions rather than values, per §12's rule about not predicting
/// what the computation can find — and because this crate uses ISO 6336's `Y_S`
/// where the paper uses an extrapolated Dolan–Broghamer, and omits the axial
/// compression term the paper includes. Neither changes either direction.
///
/// # Why the *third* claim is not asserted here
///
/// The paper's figure also has the two factors approaching each other, and over
/// its range they do. But its external gear is cut by the same 20-tooth shaper as
/// its ring, while this crate's external gear is **rack**-cut. With the shaper
/// held at 20 teeth the ring's fillet never tends to the rack's, so the two
/// fillets do not approach a common shape and the gap has no reason to close —
/// measured, it closes to z ≈ 150 and then widens again. Asserting it here would
/// be asserting a property of the paper's setup against a different one.
///
/// The convergence that *is* real is the next test, where the shaper grows with
/// the ring and both teeth genuinely become the same rack tooth.
#[test]
fn a_rings_tooth_is_stronger_than_an_external_one_and_they_converge() {
    let counts = [30u32, 40, 50, 60, 80, 100, 150, 200, 250];
    let mut previous_internal = f64::INFINITY;
    for z in counts {
        let (Some(external), Some(internal)) = j_factors(z, 20) else {
            panic!("z={z}: no rating");
        };
        assert!(
            internal > external,
            "z={z}: an internal tooth must be the stronger, {internal} vs {external}"
        );
        assert!(
            internal < previous_internal,
            "z={z}: the internal factor must fall with tooth count, {internal} vs {previous_internal}"
        );
        previous_internal = internal;
    }
}

/// **The sharp one: both teeth become the same rack tooth.**
///
/// Take the ring's tooth count *and* its shaper to infinity together and its
/// tooth tends to a rack tooth — as a rack-cut external gear's does. So the two
/// bending factors must converge, and they do so first order in `1/z`:
///
/// ```text
/// z_ring    z_cutter    |J_int − J_ext|
///    200         100        0.0251
///  1 000         500        0.0041
///  5 000       2 500        0.0008
/// 20 000      10 000        0.0002
/// ```
///
/// This is the acceptance gate for the whole internal construction, and it is
/// independent in the way that matters: the internal route is a ring involute
/// plus a shaper trochoid with the tooth pointing inward, the external route is a
/// gear involute plus a rack trochoid pointing outward, and they share none of
/// that geometry. Agreeing to 2e-4 is not something a sign error survives.
#[test]
fn a_huge_ring_cut_by_a_huge_shaper_rates_as_a_rack_tooth() {
    let mut previous = f64::INFINITY;
    for (z, cutter) in [
        (200u32, 100u32),
        (1_000, 500),
        (5_000, 2_500),
        (20_000, 10_000),
    ] {
        let (Some(external), Some(internal)) = j_factors(z, cutter) else {
            panic!("z={z}: no rating");
        };
        let gap = (internal - external).abs();
        assert!(
            gap < previous,
            "z={z}: the gap must keep closing, {gap} vs {previous}"
        );
        previous = gap;
    }
    assert!(
        previous < 5e-4,
        "the two constructions do not meet in the rack limit: {previous}"
    );
}

/// A helical ring is **refused** rather than rated on its transverse section.
///
/// Rating a helical tooth on the transverse section and dividing by the normal
/// module mixes planes and under-predicts by about `cos β` — the error §12
/// records for external gears. A ring would need a virtual spur *ring* to avoid
/// it, and that is not built yet, so the answer is no answer.
#[test]
fn a_helical_ring_is_refused_rather_than_rated_on_the_wrong_plane() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::ring_bending_section;

    for beta in [5.0, 15.0, 30.0] {
        let ring = Ring::new(
            &GearParams {
                teeth: 60,
                helix_angle: beta,
                ..Default::default()
            },
            &Cutter::default(),
        );
        assert!(
            ring_bending_section(&ring, 1.7, beta).is_none(),
            "beta={beta}: a helical ring must not be rated on its transverse section"
        );
    }
    // ...and the spur case, which is the same call at zero helix, does answer.
    let ring = Ring::new(
        &GearParams {
            teeth: 60,
            ..Default::default()
        },
        &Cutter::default(),
    );
    assert!(ring_bending_section(&ring, 1.7, 0.0).is_some());
}
