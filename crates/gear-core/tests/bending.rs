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
use gear_core::{GearParams, Tooth};

mod common;
use common::Grid;

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
            let g = Tooth::new(GearParams {
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
        let g = Tooth::new(GearParams {
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
        let g = Tooth::new(GearParams {
            teeth: 4000,
            pressure_angle: alpha,
            ..Default::default()
        });
        // The construction's own section, the largest inscribed parabola.
        let sec = gear_core::strength::root_section_rated(
            &g,
            g.u_tip,
            CriticalSection::LewisParabola,
            gear_core::strength::RootStressModel::FormFactorOnly,
        )
        .unwrap();
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
    use gear_core::strength::RootStressModel;

    let pinion = Tooth::new(GearParams {
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
    let wheel = Tooth::new(params);
    let ring = Ring::cut_by(
        &params,
        &Cutter {
            teeth: cutter_teeth,
            addendum: 1.25,
            tip_round: 0.3,
        },
    );
    // The second member's section, loaded where its contact ends: below its
    // tip where the pinion's usable flank ends first.
    fn on_path<T: gear_core::strength::ToothOutline>(
        g: &T,
        p: &ContactPath,
    ) -> Option<gear_core::strength::RootSection> {
        gear_core::strength::bending_section_on_path(
            g,
            p.contact_ratio,
            p.short_of_tip()[1],
            gear_core::contact::LoadSharing::None,
        )
        .ok()
        .map(|(s, _)| s)
    }
    let j = |s: Option<gear_core::strength::RootSection>| {
        s.and_then(|s| s.bending_factor(RootStressModel::Iso6336))
            .map(|f| 1.0 / f)
    };

    let mirrored = Tooth::new(GearParams {
        helix_angle: -beta,
        ..params
    });
    let external = Mesh::new(&pinion, &mirrored, MeshKind::External)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, mirrored.flank_ends(), &m))
        .and_then(|p| j(on_path(&mirrored, &p)));
    let internal = Mesh::new(&pinion, &wheel, MeshKind::Internal)
        .ok()
        .and_then(|m| ContactPath::new(&pinion, ring.flank_ends(), &m))
        .and_then(|p| j(on_path(&ring, &p)));
    (external, internal)
}

/// **What NASA TM-107012's Figure 8 says that survives the difference in setup**:
/// an internal tooth is stronger than an external one of the same pitch, and the
/// internal factor falls as tooth count rises.
///
/// Asserted as directions rather than values, per docs/corrections.md's rule about not predicting
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
    use gear_core::strength::{bending_section, RootStressModel};

    let of = |beta: f64| {
        Ring::cut_by(
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
    let spur = bending_section(&of(0.0), 1.7).unwrap();
    let mut last = spur.form_factor;
    for beta in [5.0, 15.0, 25.0, 35.0] {
        let sec = bending_section(&of(beta), 1.7)
            .unwrap_or_else(|| panic!("beta={beta}: a helical ring must still be rated"));
        assert!(
            sec.form_factor > 0.0 && sec.form_factor.is_finite(),
            "beta={beta}: Y_F is {}",
            sec.form_factor
        );
        assert!(
            sec.stress_correction(RootStressModel::Iso6336).is_some(),
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
        let ring = Ring::cut_by(
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
        assert_eq!(
            v.fillet.map(|f| f.phi_j),
            ring.fillet.map(|f| f.phi_j),
            "z={teeth}: junction travel"
        );
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
/// comes from `Tooth::virtual_spur`, the ring's from `Ring::virtual_spur` with the
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
    use gear_core::strength::{bending_section, RootStressModel};

    for contact_ratio in [1.5, 1.7, 1.9] {
        let mut previous: Option<f64> = None;
        for z in 40..=90u32 {
            let ring = Ring::cut_by(
                &GearParams {
                    teeth: z,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let sec = bending_section(&ring, contact_ratio)
                .unwrap_or_else(|| panic!("eps={contact_ratio} z={z}: no section"));
            let factor = sec
                .bending_factor(RootStressModel::Iso6336)
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

/// **A generated root fillet is not a circle of the cutter's tip radius**, and
/// the difference is far too large to shrug at.
///
/// This is worth pinning because the shortcut is tempting: a fillet *looks* like
/// a radius, drawings specify it as one, and `ρ` is right there as an input. But
/// the fillet is the **envelope** of the tool's corner as it sweeps, so it is
/// flatter than the corner itself — and `ρ_F` is what `q_s = s_Fn/(2 ρ_F)` divides
/// by, so using `ρ` instead would inflate `q_s`, and `Y_S` with it, by whatever
/// the ratio happens to be.
///
/// Measured at the critical section:
///
/// ```text
/// external, z = 17 … 150      ρ_F / ρ = 2.52 … 1.47
/// ring,     z = 43 … 150      ρ_F / ρ = 4.26 … 3.55
/// ```
///
/// A ring's is the flatter of the two, which is one of the reasons its tooth
/// rates stronger. Both are bracketed loosely here: the point is the *order*, not
/// the digits.
#[test]
fn a_generated_fillet_is_much_flatter_than_the_tool_that_cut_it() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::bending_section;

    const RHO: f64 = 0.2;

    for z in [17u32, 43, 60, 90, 150] {
        let g = Tooth::new(GearParams {
            teeth: z,
            root_radius: RHO,
            ..Default::default()
        });
        let sec = bending_section(&g, 1.8).unwrap();
        let ratio = sec.fillet_curvature / g.rho;
        assert!(
            (1.2..3.0).contains(&ratio),
            "external z={z}: rho_F/rho = {ratio}, expected the fillet to be flatter \
             than the tool but not unrecognisably so"
        );
    }

    let mut previous = f64::INFINITY;
    for z in [43u32, 60, 90, 150] {
        let ring = Ring::cut_by(
            &GearParams {
                teeth: z,
                ..Default::default()
            },
            &Cutter {
                teeth: 20,
                addendum: 1.25,
                tip_round: RHO,
            },
        );
        let sec = bending_section(&ring, 1.8).unwrap();
        let ratio = sec.fillet_curvature / RHO;
        assert!(
            (2.5..6.0).contains(&ratio),
            "ring z={z}: rho_F/rho = {ratio}"
        );
        // A bigger ring is closer to a rack, so its fillet tends back toward the
        // tool's own round — a direction, not a number.
        assert!(
            ratio < previous,
            "ring z={z}: {ratio} should be below {previous}"
        );
        previous = ratio;
    }
}

/// **The fillet's curvature is the analytic derivative, not a difference of
/// one.**
///
/// Both fillets used to differentiate their own analytic tangent numerically,
/// with a step of `1e-6` modules written out twice. The closed form has to
/// agree with that difference — but only to the difference's *own* error, which
/// is what this asserts rather than a tolerance somebody chose: a central
/// difference is `O(h²)` in truncation and `O(ε/h)` in rounding, so refining `h`
/// improves it and then makes it worse, and the closed form must sit inside that
/// bowl at every step.
///
/// Written as convergence rather than as a number: at the coarse step the two
/// differ by the truncation error, and halving `h` must shrink the gap toward
/// the rounding floor. A closed form that is *wrong* does not do that — it
/// converges on its own answer instead, and the gap stops falling.
#[test]
fn the_closed_form_fillet_curvature_is_what_a_difference_converges_on() {
    use gear_core::strength::fillet_point_and_tangent as tangent;

    for p in Grid::new()
        .teeth(&[9, 17, 40, 120])
        .shifts(&[-0.3, 0.0, 0.4])
        .pressure_angle(&[14.5, 20.0, 25.0])
        .root_radius(&[0.1, 0.25, 0.38])
        .build()
    {
        let g = Tooth::new(p);
        if g.severed || !g.s_j.is_finite() {
            continue;
        }
        // Three places along the fillet, avoiding both ends.
        for t in [0.25, 0.5, 0.75] {
            let s = g.s_j * (1.0 - t);
            let exact = gear_core::strength::fillet_curvature_radius(&g, s);
            if !exact.is_finite() {
                continue;
            }

            let difference_at = |h: f64| {
                let (_, t0) = tangent(&g, s - h);
                let (_, t1) = tangent(&g, s + h);
                let (_, tc) = tangent(&g, s);
                let ddx = (t1[0] - t0[0]) / (2.0 * h);
                let ddy = (t1[1] - t0[1]) / (2.0 * h);
                let speed = f64::hypot(tc[0], tc[1]);
                let cross = (tc[0] * ddy - tc[1] * ddx).abs();
                speed.powi(3) / cross
            };

            let coarse = (difference_at(1e-3) - exact).abs() / exact;
            let fine = (difference_at(1e-4) - exact).abs() / exact;
            // Truncation falls as h²: a hundredfold at a tenth of the step,
            // less whatever rounding has crept in. Ten is a generous floor and
            // is still far outside what a *wrong* closed form could manage,
            // since that one would not converge at all.
            assert!(
                fine * 10.0 < coarse.max(1e-14) || fine < 1e-9,
                "z={} x={} rho={} s={s}: the difference is not converging on the \
                 closed form — {coarse:e} at h=1e-3 against {fine:e} at h=1e-4",
                p.teeth,
                p.profile_shift,
                p.root_radius
            );
        }
    }
}

/// **The tangent construction takes its angle from the member**, 30° on an
/// external tooth and 60° on a ring's — ISO 6336-3:2019, 6.1 and 6.2.5.
///
/// Measured off the tangent the construction actually found rather than read
/// back off the constant it was given, which is the only version of this test
/// that can fail: `tangent_direction` is carried on the result precisely
/// because reconstructing it from the angle once got the sign wrong while the
/// tangency point was right.
///
/// A ring's 60° is the same construction on a tooth that points the other way
/// and whose fillet is concave, not a second one — which is why it arrives as a
/// number on `ToothOutline` beside the curves and the frame.
#[test]
fn the_tangent_section_is_taken_at_the_angle_its_member_asks_for() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{
        root_section_with, CriticalSection, TANGENT_ANGLE_DEG, TANGENT_ANGLE_INTERNAL_DEG,
    };

    // The angle a tangent direction makes with the tooth centreline, degrees.
    // The frame has `y` along the centreline toward the tip for both kinds of
    // member, so this is the same measurement on either.
    let from_centreline = |t: [f64; 2]| t[0].abs().atan2(t[1].abs()).to_degrees();

    for teeth in [17u32, 40, 150] {
        let g = Tooth::new(GearParams {
            teeth,
            ..Default::default()
        });
        let sec = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle)
            .unwrap_or_else(|| panic!("z={teeth}: an external tooth has a 30° tangent"));
        let got = from_centreline(sec.tangent_direction);
        assert!(
            (got - TANGENT_ANGLE_DEG).abs() < 1e-6,
            "z={teeth}: external tangent at {got}°, wanted {TANGENT_ANGLE_DEG}°"
        );
    }

    for teeth in [40u32, 60, 90] {
        let ring = Ring::cut_by(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let load = ring.u_j;
        let sec = root_section_with(&ring, load, CriticalSection::TangentAngle)
            .unwrap_or_else(|| panic!("z={teeth}: a ring has a 60° tangent"));
        let got = from_centreline(sec.tangent_direction);
        assert!(
            (got - TANGENT_ANGLE_INTERNAL_DEG).abs() < 1e-6,
            "z={teeth}: ring tangent at {got}°, wanted {TANGENT_ANGLE_INTERNAL_DEG}°"
        );
        // The two constructions are genuinely different sections, so the ring's
        // 60° chord is not the 30° one under another name.
        assert!(sec.root_chord > 0.0 && sec.root_chord.is_finite());
    }
}

/// **The inscribed parabola is still what a section defaults to**, and reading
/// ISO 6336-3 did not quietly move it.
///
/// The standard specifies the 30°/60° tangent and this crate deliberately does
/// not follow it, for the reasons `CriticalSection::LewisParabola` sets out —
/// it follows the load point, it is the more conservative of the two, and this
/// crate computes the exact profile so the tangent's simplification buys
/// nothing. Filling in the standard's *other* gaps is not a reason to adopt
/// this one, and the 60° angle added beside the 30° is an extension of the
/// construction that is **not** the default rather than a step toward making it
/// one.
///
/// Written as a test rather than a comment because the two constructions agree
/// closely on large teeth, so a default that had drifted would show up in
/// almost nothing else.
#[test]
fn the_default_critical_section_is_still_the_inscribed_parabola() {
    use gear_core::strength::{root_section, root_section_with, CriticalSection};

    assert_eq!(CriticalSection::default(), CriticalSection::LewisParabola);

    for teeth in [9u32, 17, 60] {
        let g = Tooth::new(GearParams {
            teeth,
            ..Default::default()
        });
        let plain = root_section(&g, g.u_tip).unwrap();
        assert_eq!(plain.method, CriticalSection::LewisParabola);
        let parabola = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola).unwrap();
        assert_eq!(plain.form_factor, parabola.form_factor);

        // ...and it is the more conservative of the two, which is half of why
        // it was chosen. The gap widens as the tooth gets worse.
        let tangent = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle).unwrap();
        assert!(
            parabola.form_factor > tangent.form_factor,
            "z={teeth}: the parabola should be the conservative one, {} vs {}",
            parabola.form_factor,
            tangent.form_factor
        );
    }
}

/// **The fillet is tightest at its root**, which is what lets `ρ_f` — the
/// minimum radius of curvature of the fillet curve — be read at one point
/// instead of searched for.
///
/// Dolan and Broghamer's stress concentration factor is a function of "the
/// minimum radius of the fillets", so the crate has to produce that minimum.
/// It reads it at [`ToothOutline::fillet_root`] on the physical argument that a
/// trochoid is closest to the tool's own corner radius where the corner cut
/// deepest and flattens as it sweeps up toward the flank. That is an argument,
/// not a proof, so this sweeps the whole fillet on both kinds of member and
/// checks nothing anywhere is smaller.
#[test]
fn the_fillet_is_tightest_at_its_root() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::ToothOutline;

    let check = |what: &str, g: &dyn ToothOutline| {
        let (lo, hi) = g.fillet_bracket();
        let at_root = g.fillet_curvature(g.fillet_root());
        assert!(
            at_root.is_finite() && at_root > 0.0,
            "{what}: rho_f {at_root}"
        );
        for i in 0..=2000 {
            let t = f64::from(i) / 2000.0;
            let r = g.fillet_curvature(lo + (hi - lo) * t);
            if !r.is_finite() {
                continue;
            }
            assert!(
                r >= at_root * (1.0 - 1e-9),
                "{what}: fillet radius {r} at t={t} is below the root's {at_root}"
            );
        }
        // ...and the junction is the flat end, which is the other half of why
        // reading `rho_f` there was wrong: it is the largest value, not the
        // smallest.
        let at_junction = g.fillet_curvature(g.fillet_junction());
        assert!(
            at_junction >= at_root,
            "{what}: junction {at_junction} below root {at_root}"
        );
    };

    for teeth in [9u32, 17, 40, 100, 250] {
        for shift in [-0.3_f64, 0.0, 0.4] {
            for root_radius in [0.1_f64, 0.25, 0.38] {
                let g = Tooth::new(GearParams {
                    teeth,
                    profile_shift: shift,
                    root_radius,
                    ..Default::default()
                });
                check(
                    &format!("external z={teeth} x={shift} rho={root_radius}"),
                    &g,
                );
            }
        }
    }
    for teeth in [40u32, 60, 90, 160] {
        for shift in [-0.3_f64, 0.0, 0.4] {
            let ring = Ring::cut_by(
                &GearParams {
                    teeth,
                    profile_shift: shift,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            if ring.fillet.is_some() {
                check(&format!("ring z={teeth} x={shift}"), &ring);
            }
        }
    }
}

/// **The weaker of the two inscribed parabolas is the one reported.**
///
/// Savage, Rubadeux & Coe search the involute and the trochoid and take "the
/// smaller x coordinate", which "identifies the weaker inscribed parabola" —
/// `x = s_Fn²/(4 h_Fe)`. Both candidates share a load point and so share
/// `cos α_Fen`, which makes a smaller `x` exactly a larger `Y_F`.
///
/// The crate used to take the fillet whenever it had a solution and consult the
/// flank only otherwise. This asserts the rule directly: whichever curve the
/// section came from, no tangency on the *other* curve is weaker.
/// **A load point is on the tooth**, at every contact ratio.
///
/// `d = ε_n − 1` base pitches back from the tip goes negative below a contact
/// ratio of 1 — a point past the end of the tooth. Nothing refused it: the
/// involute is a curve, so `root_section` answered there, with a moment arm the
/// tooth does not have. It over-predicted by 2.8 % at `ε_n` = 0.95 and 29.4 % at
/// 0.5, which is a conservative error and therefore
/// [no better than the other kind](../../../docs/rationale.md).
///
/// Two invariants, and the second is the one that would have caught it: the
/// section a rating is taken at must sit **on the generated flank**, and below
/// a contact ratio of 1 it must be the **tip** exactly — a pair alone for the
/// whole of its engagement is alone at the tip too, which is what
/// `ContactPath::highest_single_pair` says with its own `.min(recess)`.
#[test]
fn a_rating_is_taken_at_a_point_on_the_tooth() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{bending_section, root_section, ToothOutline};

    let check = |what: &str, member: &dyn Fn(f64) -> Option<f64>, tip: f64| {
        // Below a contact ratio of 1 every answer is the tip's, exactly.
        for eps in [0.999_f64, 0.9, 0.6, 0.2] {
            let got = member(eps)
                .unwrap_or_else(|| panic!("{what}: eps={eps} should still rate, at the tip"));
            assert!(
                (got - tip).abs() < 1e-12,
                "{what}: eps={eps} rated at {got}, not the tip's {tip}"
            );
        }
        // ...and above it the load point moves down the flank, shortening the
        // moment arm, so `Y_F` **falls** away from the tip's — monotonically,
        // and the tip's is the largest any of them can be.
        let mut last = tip;
        for eps in [1.2_f64, 1.4, 1.6] {
            let got = member(eps).unwrap_or_else(|| panic!("{what}: eps={eps} should rate"));
            assert!(
                got < last,
                "{what}: eps={eps} gave {got}, which is not below {last}"
            );
            last = got;
        }
    };

    for teeth in [13u32, 18, 40] {
        let g = Tooth::new(GearParams {
            teeth,
            addendum: 0.8,
            ..Default::default()
        });
        let v = ToothOutline::virtual_spur(&g);
        let tip = root_section(&v, ToothOutline::flank_bracket(&v).1)
            .expect("a tip section")
            .form_factor;
        check(
            &format!("external z={teeth}"),
            &|eps| bending_section(&g, eps).map(|s| s.form_factor),
            tip,
        );
    }

    for teeth in [40u32, 90] {
        let ring = Ring::cut_by(
            &GearParams {
                teeth,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let v = ToothOutline::virtual_spur(&ring);
        let tip = root_section(&v, ToothOutline::flank_bracket(&v).0)
            .expect("a tip section")
            .form_factor;
        check(
            &format!("ring z={teeth}"),
            &|eps| bending_section(&ring, eps).map(|s| s.form_factor),
            tip,
        );
    }

    // And the bound itself: a roll off either end of the flank has no section,
    // for either kind of member, rather than an extrapolated one.
    let g = Tooth::new(GearParams {
        teeth: 18,
        ..Default::default()
    });
    let v = ToothOutline::virtual_spur(&g);
    let (lo, hi) = ToothOutline::flank_bracket(&v);
    assert!(root_section(&v, hi).is_some() && root_section(&v, lo).is_some());
}

/// **Each curve offers its least Lewis measure, and the highest rated
/// governs.** The rule, on both kinds of member:
///
/// - each curve (fillet, flank) offers the section where `x²/(y_v − y)` is
///   least over the whole curve below the vertex — an interior tangency, or
///   one of the curve's ends — found here by brute force from the curves
///   alone (sampled, each dip closed by golden section, the ends compared);
/// - the fillet's is rated with the fillet's notch factor, the smooth flank's
///   with none (1);
/// - the governing section is the readable one that rates highest, and one
///   the model cannot read never masks one it can.
///
/// So a ring, whose fillet has no tangency, keeps its fillet's notch factor
/// through the fillet's end. Two cases carry the history: z 20, x 0.8, 20°,
/// h_a 1.1 loaded at the tip, where a search that took the first tangency
/// it bracketed read the fillet's while the flank's is less; and the 5/40
/// pinion at β 20°, x 1 with its tip held to its width (a virtual 6-tooth
/// spur), whose flank candidate is compressed (`Y_F` 4.06, axial 4.34) and
/// once masked its fillet's.
///
/// The measure's tolerance is rounding: it is read off coordinates as large
/// as the outline's largest, `c`, each carrying `2⁸ε·c`, and the ratio
/// `2/x + 1/(y_v − y)` of that relatively; a dip is closed to the argument's
/// rounding, where the measure is flat to `ε`.
#[test]
fn each_curve_offers_its_least_and_the_highest_rated_governs() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{root_section_with, root_sections, RootStressModel, ToothOutline};

    /// The least of `x²/(y_v − y)` on one curve, its dips and ends, and the
    /// largest coordinate seen.
    fn least(curve: &dyn Fn(f64) -> [f64; 2], lo: f64, hi: f64, vertex: f64) -> (Option<f64>, f64) {
        let ratio = |p: f64| {
            let q = curve(p);
            (q[1] < vertex && q[0] > 0.0).then(|| q[0] * q[0] / (vertex - q[1]))
        };
        let n = 800;
        let at = |i: usize| lo + (hi - lo) * i as f64 / n as f64;
        let mut scale = 0.0_f64;
        let vals: Vec<Option<f64>> = (0..=n)
            .map(|i| {
                let q = curve(at(i));
                scale = scale.max(q[0].abs()).max(q[1].abs());
                ratio(at(i))
            })
            .collect();
        // Both ends; the flank's tip end is under the vertex only by rounding,
        // and the caller asserts it is not.
        let mut out = vec![vals[0], vals[n]];
        for i in 1..n {
            let (Some(a), Some(b), Some(c)) = (vals[i - 1], vals[i], vals[i + 1]) else {
                continue;
            };
            if !(b <= a && b <= c) || (b == a && b == c) {
                continue;
            }
            let g = (5.0_f64.sqrt() - 1.0) / 2.0;
            let (mut l, mut h) = (at(i - 1), at(i + 1));
            while h - l > f64::EPSILON.sqrt() * l.abs().max(h.abs()).max(1e-3) {
                let (x1, x2) = (h - g * (h - l), l + g * (h - l));
                if ratio(x1) <= ratio(x2) {
                    h = x2;
                } else {
                    l = x1;
                }
            }
            out.push(ratio(0.5 * (l + h)));
        }
        (out.into_iter().flatten().reduce(f64::min), scale)
    }

    let model = RootStressModel::DolanBroghamer;
    let (mut checked, mut rings, mut flank_governs, mut at_an_end, mut dropped) = (0, 0, 0, 0, 0);
    let mut members: Vec<(String, Box<dyn ToothOutline>)> = Vec::new();
    for teeth in [6_u32, 9, 12, 20, 30, 40, 150] {
        for (x, alpha, h_a, rho) in [
            (0.8_f64, 25.0_f64, 1.25_f64, 0.2_f64),
            (0.5, 25.0, 1.0, 0.38),
            (0.8, 20.0, 1.1, 0.25),
            (1.0, 20.0, 0.658, 0.38),
            (0.0, 20.0, 1.0, 0.38),
            (-0.3, 14.5, 1.0, 0.3),
        ] {
            members.push((
                format!("z{teeth} x{x} {alpha}° h_a{h_a} ρ{rho}"),
                Box::new(Tooth::new(GearParams {
                    teeth,
                    profile_shift: x,
                    pressure_angle: alpha,
                    addendum: h_a,
                    root_radius: rho,
                    ..Default::default()
                })),
            ));
        }
    }
    for teeth in [40_u32, 60, 90] {
        for x in [0.0_f64, 0.4] {
            let params = GearParams {
                teeth,
                profile_shift: x,
                root_radius: 0.3,
                ..Default::default()
            };
            let cutter = Cutter {
                teeth: 20,
                addendum: 1.25,
                tip_round: 0.3,
            };
            members.push((
                format!("ring z{teeth} x{x}"),
                Box::new(Ring::cut_by(&params, &cutter)),
            ));
        }
    }
    for (label, g) in &members {
        if !g.is_usable() {
            continue;
        }
        let (ulo, uhi) = g.flank_bracket();
        let tip = if g.tip_at_high_roll() { uhi } else { ulo };
        for frac in [0.0_f64, 0.15, 0.3, 0.5] {
            let roll = tip + (if g.tip_at_high_roll() { -1.0 } else { 1.0 }) * frac * (uhi - ulo);
            let (load_point, dir) = g.load_at(roll);
            let vertex = load_point[1] + (-load_point[0] / dir[0]) * dir[1];
            let (flo, fhi) = g.fillet_bracket();
            let (fillet, c1) = least(&|p| g.fillet_at(p).0, flo, fhi, vertex);
            let (flank, c2) = least(&|p| g.flank_at(p).0, ulo, uhi, vertex);
            // The flank's tip is never strictly below the vertex, so its
            // measure there is none, and a least there is rounding's.
            let (tq, _) = g.flank_at(tip);
            assert!(
                tq[1] >= vertex - 256.0 * f64::EPSILON * tq[1].abs(),
                "{label} at {frac}: the tip stands {} below the vertex",
                vertex - tq[1]
            );
            let found = root_sections(g.as_ref(), roll, CriticalSection::LewisParabola);
            let measure = |s: &gear_core::strength::RootSection| {
                s.root_chord * s.root_chord / (4.0 * s.moment_arm)
            };
            for (name, want, on_flank) in [("fillet", fillet, false), ("flank", flank, true)] {
                let got = found.iter().find(|s| s.tangency_on_flank == on_flank);
                let (Some(want), Some(got)) = (want, got) else {
                    assert_eq!(
                        want.is_some(),
                        got.is_some(),
                        "{label} at {frac}: the {name}'s least {want:?}, the crate's {got:?}"
                    );
                    continue;
                };
                let tol = 256.0
                    * f64::EPSILON
                    * c1.max(c2)
                    * (4.0 / got.root_chord + 1.0 / got.moment_arm);
                assert!(
                    (measure(got) - want).abs() <= tol * want,
                    "{label} at {frac}: the {name} offers {}, its least is {want}",
                    measure(got)
                );
                // The notch factor is the curve's: AGMA's fit to Dolan and
                // Broghamer on the fillet, none on the smooth flank.
                let notch = got.stress_correction(model).unwrap();
                let a = got.pressure_angle_rad;
                let fillets = (0.331 - 0.436 * a)
                    + (got.root_chord / got.min_fillet_curvature).powf(0.324 - 0.492 * a)
                        * (got.root_chord / got.moment_arm).powf(0.261 + 0.545 * a);
                let want = if on_flank { 1.0 } else { fillets };
                assert!(
                    (notch - want).abs() <= 4.0 * f64::EPSILON * want,
                    "{label} at {frac}: the {name}'s section carries a notch factor of {notch}, \
                     not {want}"
                );
                let (lo, hi) = if on_flank { (ulo, uhi) } else { (flo, fhi) };
                at_an_end += usize::from(got.s == lo || got.s == hi);
                // The parabola drawn is the one through the section's point:
                // `p = x²/(4(y_v − y))`, tangent there or through an end.
                let p = got.parabola_p.unwrap();
                let through = got.root_chord * got.root_chord / (16.0 * got.moment_arm);
                assert!(
                    (p - through).abs() <= tol * through,
                    "{label} at {frac}: the {name}'s parabola p {p}, through its point {through}"
                );
            }
            // The governing section: the readable candidate rating highest.
            let rated: Vec<f64> = found
                .iter()
                .filter_map(|s| s.bending_factor(model))
                .collect();
            dropped += usize::from(rated.len() < found.len() && !rated.is_empty());
            let governing = root_section_with(g.as_ref(), roll, CriticalSection::LewisParabola);
            match rated.iter().copied().reduce(f64::max) {
                Some(best) => {
                    let g = governing.unwrap();
                    assert_eq!(g.bending_factor(model), Some(best), "{label} at {frac}");
                    flank_governs += usize::from(g.tangency_on_flank);
                }
                None => assert!(
                    governing.is_none_or(|g| g.bending_factor(model).is_none()),
                    "{label} at {frac}: no candidate is readable and the section rates"
                ),
            }
            checked += 1;
            if !g.tip_at_high_roll() {
                rings += 1;
                assert!(
                    found.iter().any(|s| !s.tangency_on_flank),
                    "{label} at {frac}: a ring offers no fillet section"
                );
            }
        }
    }
    assert!(
        checked > 100 && rings >= 24,
        "checked {checked}, rings {rings}"
    );
    assert!(
        flank_governs > 0 && at_an_end > 0 && dropped > 0,
        "the flank governs {flank_governs} times, a curve's end is its least {at_an_end}, \
         a compressed candidate is dropped {dropped}: the law must reach each"
    );
}

/// **The rating is continuous where its section changes curve or moves to an
/// end.** Each curve's candidate is its least Lewis measure, at a tangency or
/// at an end, and the least of a continuous measure over a curve is
/// continuous; the governing section is the greater of the candidates'
/// ratings. So wherever a candidate moves between its tangency and its end,
/// or the governing section changes curve, the measure and the rating may
/// turn a corner but not step. (A candidate that jumps between two local
/// minima whose measures tie keeps its `Y_F`, a function of the measure alone,
/// but not its axial term or notch factor; that is the change this watches.)
///
/// Swept over the shift on rack-cut teeth, undercut ones included, and on
/// rings, each such change is bisected to rounding and the change across it
/// measured at half-widths `h` of 1e-4 and 1e-7: continuous, it falls with
/// `h` (by 1e-3, allowed 1e-2); a step leaves it where it was.
#[test]
fn the_rating_is_continuous_where_its_section_changes_curve_or_ends() {
    use gear_core::ring::{Cutter, Ring};
    use gear_core::strength::{
        bending_section, root_sections, RootSection, RootStressModel, ToothOutline,
    };

    /// The rating, the two curves' least measures, and which section each
    /// curve offers and which governs: `(curve, at an end)`.
    type State = (f64, [Option<f64>; 2], [Option<(bool, bool)>; 3]);
    fn state<T: ToothOutline>(g: &T, eps: f64) -> Option<State> {
        let governing = bending_section(g, eps)?;
        let f = governing.bending_factor(RootStressModel::DolanBroghamer)?;
        let kind = |s: &RootSection| {
            let (lo, hi) = if s.tangency_on_flank {
                g.flank_bracket()
            } else {
                g.fillet_bracket()
            };
            (s.tangency_on_flank, s.s == lo || s.s == hi)
        };
        // The candidates at the same load point, on the virtual spur (spur
        // here, so the tooth itself).
        let (lo, hi) = g.flank_bracket();
        let tip = if g.tip_at_high_roll() { hi } else { lo };
        let eps_n = eps;
        let pitch =
            std::f64::consts::PI * g.transverse_module() * g.transverse_pressure_angle().cos();
        let roll = tip
            + (if g.tip_at_high_roll() { -1.0 } else { 1.0 }) * (eps_n - 1.0).max(0.0) * pitch
                / g.base_radius();
        let found = root_sections(g, roll, CriticalSection::LewisParabola);
        let on = |flank: bool| found.iter().find(|s| s.tangency_on_flank == flank);
        let measure = |s: &RootSection| s.root_chord * s.root_chord / (4.0 * s.moment_arm);
        Some((
            f,
            [on(false).map(measure), on(true).map(measure)],
            [
                Some(kind(&governing)),
                on(false).map(kind),
                on(true).map(kind),
            ],
        ))
    }
    let (mut between_curves, mut to_an_end, mut rings) = (0, 0, 0);
    let mut check =
        |label: &str, at: &dyn Fn(f64) -> Option<State>, lo: f64, hi: f64, ring: bool| {
            let n = 200;
            let x = |i: usize| lo + (hi - lo) * i as f64 / n as f64;
            for i in 0..n {
                let (Some(a), Some(b)) = (at(x(i)), at(x(i + 1))) else {
                    continue;
                };
                if a.2 == b.2 {
                    continue;
                }
                let (mut l, mut r) = (x(i), x(i + 1));
                for _ in 0..60 {
                    let m = 0.5 * (l + r);
                    match at(m) {
                        Some(s) if s.2 == a.2 => l = m,
                        _ => r = m,
                    }
                }
                let mid = 0.5 * (l + r);
                let across =
                    |h: f64| -> Option<(State, State)> { Some((at(mid - h)?, at(mid + h)?)) };
                let (Some((wl, wr)), Some((nl, nr))) = (across(1e-4), across(1e-7)) else {
                    continue;
                };
                let continuous = |what: &str, wide: f64, narrow: f64, size: f64| {
                    assert!(
                    narrow <= 1e-2 * wide + 256.0 * f64::EPSILON * size,
                    "{label}: {what} steps where the sections go from {:?} to {:?} at x {mid}: \
                     {wide} across 2e-4, {narrow} across 2e-7",
                    a.2,
                    b.2
                );
                };
                continuous("the rating", (wr.0 - wl.0).abs(), (nr.0 - nl.0).abs(), nl.0);
                for k in 0..2 {
                    if let (Some(p), Some(q), Some(pn), Some(qn)) =
                        (wl.1[k], wr.1[k], nl.1[k], nr.1[k])
                    {
                        continuous(
                            ["the fillet's measure", "the flank's measure"][k],
                            (q - p).abs(),
                            (qn - pn).abs(),
                            pn,
                        );
                    }
                }
                let governing_curve_changed = a.2[0].map(|g| g.0) != b.2[0].map(|g| g.0);
                if governing_curve_changed {
                    between_curves += 1;
                }
                for k in 0..3 {
                    if let (Some(p), Some(q)) = (a.2[k], b.2[k]) {
                        to_an_end += usize::from(p.0 == q.0 && p.1 != q.1);
                    }
                }
                rings += usize::from(ring);
            }
        };
    // Tip-loaded (ε 1) families stop short of the pointed limit (z9 25°: x
    // 0.5088; z12 20° h_a 1.1: 0.6214; z30 25° h_a 1.25: 0.661), where the
    // rule is singular: the flank's least sits under the vanishing tip land
    // and rates without bound as it closes (at 1e-7 of the limit, 5.7e5),
    // then gives way to the fillet's (`docs/state.md`). Loaded below the tip,
    // the vertex is below the tip corner and the limit is crossed smoothly.
    for (teeth, alpha, h_a, rho, eps, lo, hi) in [
        (
            30_u32, 25.0_f64, 1.25_f64, 0.2_f64, 1.2_f64, 0.3_f64, 1.0_f64,
        ),
        (20, 25.0, 1.25, 0.2, 1.2, 0.2, 1.0),
        (9, 25.0, 1.0, 0.38, 1.0, 0.0, 0.48),
        (12, 20.0, 1.1, 0.25, 1.0, 0.3, 0.6),
        (30, 25.0, 1.25, 0.2, 1.0, 0.3, 0.64),
        (40, 20.0, 1.0, 0.38, 1.3, -0.3, 0.8),
        (9, 20.0, 1.0, 0.38, 1.3, -0.75, 0.2),
        (7, 20.0, 1.0, 0.38, 1.2, -0.7, 0.2),
        (10, 14.5, 1.0, 0.38, 1.3, -0.6, 0.3),
    ] {
        let make = move |x: f64| {
            Tooth::new(GearParams {
                teeth,
                pressure_angle: alpha,
                addendum: h_a,
                root_radius: rho,
                profile_shift: x,
                ..Default::default()
            })
        };
        let label = format!("z{teeth} {alpha}° h_a{h_a} ρ{rho} ε{eps}");
        check(&label, &|x| state(&make(x), eps), lo, hi, false);
    }
    for teeth in [40_u32, 60, 90] {
        let cutter = Cutter {
            teeth: 20,
            addendum: 1.25,
            tip_round: 0.3,
        };
        let make = move |x: f64| {
            Ring::cut_by(
                &GearParams {
                    teeth,
                    profile_shift: x,
                    root_radius: 0.3,
                    ..Default::default()
                },
                &cutter,
            )
        };
        check(
            &format!("ring z{teeth}"),
            &|x| state(&make(x), 1.4),
            -0.3,
            0.8,
            true,
        );
    }
    eprintln!("{between_curves} changes of the governing curve, {to_an_end} of tangency and end, {rings} on rings");
    assert!(
        between_curves > 0 && to_an_end > 0,
        "{between_curves} changes of the governing curve, {to_an_end} of tangency and end: \
         the sweep must cross both"
    );
}
