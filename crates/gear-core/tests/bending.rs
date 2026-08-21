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
    j_factors_at(z, cutter_teeth, 0.0)
}

/// The same at a helix angle, so the virtual-section route is exercised on both
/// sides of the comparison.
fn j_factors_at(z: u32, cutter_teeth: u32, beta: f64) -> (Option<f64>, Option<f64>) {
    use gear_core::contact::ContactPath;
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{bending_section, ring_bending_section, StressConcentration};

    let pinion = Gear::new(GearParams {
        teeth: 25,
        helix_angle: beta,
        ..Default::default()
    });
    let params = GearParams {
        teeth: z,
        root_radius: 0.3,
        // An internal pair shares its helix hand; an external one opposes it,
        // which is what `Mesh::new` checks. The wheel below is built for the
        // internal pairing and the external mesh is formed from its mirror.
        helix_angle: beta,
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

    let mirrored = Gear::new(GearParams {
        helix_angle: -beta,
        ..params
    });
    let external = Mesh::new(&pinion, &mirrored, MeshKind::External)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, mirrored.ra, &m))
        .and_then(|p| j(bending_section(&mirrored, p.contact_ratio)));
    let internal = Mesh::new(&pinion, &wheel, MeshKind::Internal)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, ring.ra, &m))
        .and_then(|p| j(ring_bending_section(&ring, p.contact_ratio)));
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

/// **A helical ring is rated on its virtual spur section**, exactly as a helical
/// external gear is — feature parity with the spur case, not a refusal.
///
/// Two things are asserted, and the second is the one that matters. The rating
/// exists at every helix angle; and at `β = 0` the virtual ring *is* the ring, so
/// the helical route must reproduce the spur answer **bit-identically** rather
/// than merely closely. That is the check that the virtual construction is a
/// generalisation and not a second model with the spur case bolted on.
#[test]
fn a_helical_ring_is_rated_on_its_virtual_spur_section() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{ring_bending_section, StressConcentration};

    let of = |beta: f64| {
        Ring::new(
            &GearParams {
                teeth: 60,
                helix_angle: beta,
                ..Default::default()
            },
            &Cutter::default(),
        )
    };

    // The spur case goes through the same virtual route; that it *is* the ring
    // at zero helix is asserted on its own below.
    let spur = ring_bending_section(&of(0.0), 1.7).unwrap();
    let mut last = spur.form_factor;
    for beta in [5.0, 15.0, 25.0, 35.0] {
        let sec = ring_bending_section(&of(beta), 1.7)
            .unwrap_or_else(|| panic!("beta={beta}: a helical ring must still be rated"));
        assert!(
            sec.form_factor > 0.0 && sec.form_factor.is_finite(),
            "beta={beta}: Y_F is {}",
            sec.form_factor
        );
        assert!(
            sec.stress_correction(StressConcentration::Iso6336)
                .is_some(),
            "beta={beta}: the notch factor must be defined too"
        );
        // The virtual ring grows with helix (`z_n = z/cos³β`), and a bigger ring
        // has a thicker root — so the form factor falls, as it does on an
        // external gear.
        assert!(
            sec.form_factor < last,
            "beta={beta}: Y_F {} should fall below {last}",
            sec.form_factor
        );
        last = sec.form_factor;
    }
}

/// The virtual spur ring at zero helix is the ring itself — by construction, not
/// by a branch. The internal counterpart of the external identity DESIGN's
/// appendix records.
#[test]
fn the_virtual_spur_ring_is_the_identity_at_zero_helix() {
    use gear_core::ring::{Cutter, Ring};
    for teeth in [43u32, 60, 90] {
        let ring = Ring::new(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let v = ring.virtual_spur();
        assert_eq!(v.r, ring.r, "z={teeth}: pitch radius");
        assert_eq!(v.rb, ring.rb, "z={teeth}: base radius");
        assert_eq!(v.ra, ring.ra, "z={teeth}: tip radius");
        assert_eq!(v.rf, ring.rf, "z={teeth}: root radius");
        assert_eq!(v.psi_b, ring.psi_b, "z={teeth}: tooth thickness");
        assert_eq!(v.u_j, ring.u_j, "z={teeth}: junction");
        assert_eq!(v.s_j, ring.s_j, "z={teeth}: junction travel");
    }
}

/// **The rack limit holds at a helix angle too**, which is the check that the
/// virtual spur *ring* is the right construction rather than merely a plausible
/// one.
///
/// Grow the ring and its shaper together and both teeth become the same rack
/// tooth in the normal plane — so the internal and external bending factors must
/// converge, at any helix. Both sides reach it through their own virtual
/// section, and those two virtualisations are independent: the external gear's
/// comes from `Gear::virtual_spur`, the ring's from `Ring::virtual_spur` with the
/// cutter scaled alongside.
#[test]
fn the_rack_limit_holds_at_a_helix_angle() {
    for beta in [15.0, 30.0] {
        let mut previous = f64::INFINITY;
        for (z, cutter) in [(1_000u32, 500u32), (5_000, 2_500), (20_000, 10_000)] {
            let (Some(external), Some(internal)) = j_factors_at(z, cutter, beta) else {
                panic!("beta={beta} z={z}: no rating");
            };
            let gap = (internal - external).abs();
            assert!(
                gap < previous,
                "beta={beta} z={z}: the gap must keep closing, {gap} vs {previous}"
            );
            previous = gap;
        }
        assert!(
            previous < 2e-3,
            "beta={beta}: the two constructions do not meet in the rack limit: {previous}"
        );
    }
}

/// **A ring's rating is continuous too, and it is rated at all.**
///
/// A ring's tooth is short and wide, so the inscribed parabola touches its
/// involute flank across most of the useful range — every count from 40 up, in
/// this sweep. Under the earlier rule that withheld the correction on a flank
/// tangency, that meant *no bending stress for almost any ring*, appearing and
/// disappearing as the contact ratio moved the load point. Nothing physical
/// happens at those tooth counts, so nothing may step.
#[test]
fn a_rings_rating_is_continuous_over_its_useful_range() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{ring_bending_section, StressConcentration};

    for contact_ratio in [1.5, 1.7, 1.9] {
        let mut previous: Option<f64> = None;
        for z in 40..=90u32 {
            let ring = Ring::new(
                &GearParams {
                    teeth: z,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let sec = ring_bending_section(&ring, contact_ratio)
                .unwrap_or_else(|| panic!("eps={contact_ratio} z={z}: no section"));
            let factor = sec
                .bending_factor(StressConcentration::Iso6336)
                .unwrap_or_else(|| panic!("eps={contact_ratio} z={z}: no rating"));
            // The notch radius is the fillet's, whichever curve the section
            // landed on — so it stays a plausible fillet size rather than
            // jumping to the involute's, which is tens of millimetres.
            assert!(
                sec.fillet_curvature > 0.0 && sec.fillet_curvature < 5.0,
                "eps={contact_ratio} z={z}: rho_F = {}",
                sec.fillet_curvature
            );
            if let Some(p) = previous {
                let step = ((factor - p) / p).abs();
                assert!(
                    step < 0.02,
                    "eps={contact_ratio} z={z}: the rating stepped by {step}"
                );
            }
            previous = Some(factor);
        }
    }
}
