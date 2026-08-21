//! Development harness for `gear-core`.
//!
//! Exists so the mathematics can be driven and inspected without a browser.
//!
//! ```text
//! gear-cli show   [z] [x]     print the derived geometry of one gear
//! gear-cli sweep              scan a parameter grid for clamps and undercut
//! gear-cli materials          the material library, with each value's basis
//! gear-cli strength [z1] [z2] [torque] [material] [helix]
//!                             a worked mesh: bending, contact, efficiency
//! gear-cli train              a two-stage geartrain, end to end
//! gear-cli crossed [z1] [z2] [shaft angle]
//!                             a crossed gear pair, swept over the helix split
//! gear-cli planetstage [z_sun] [z_planet] [z_ring] [N] [helix]
//!                             a planetary stage, end to end
//! gear-cli planetary [z_sun] [z_planet] [N] [x_sun] [x_ring]
//!                             the ring counts that can be made to work, and
//!                             the planet shift each of them needs
//! ```

mod diagram;
mod matrix;

use gear_core::{Gear, GearParams};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("sweep") => sweep(),
        Some("dump") => dump(),
        Some("bending") => bending_report(),
        Some("matrix") => matrix_report(),
        Some("loadcase") => loadcase_report(),
        Some("materials") => materials(),
        Some("train") => train_report(args.get(1).map(String::as_str) == Some("mixed")),
        Some("crossed") => crossed_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(23),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(90.0),
        ),
        Some("wormstage") => worm_stage_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(2.0),
        ),
        Some("worm") => worm_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(40),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(7.0),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(90.0),
        ),
        Some("strength") => strength_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(43),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(2.0),
            args.get(4).map_or("4340 Hardened Steel", String::as_str),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ),
        Some("dxf") => dxf(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1e-3),
        ),
        Some("planetstage") => planetary_stage_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(24),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(18),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(60),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(3),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ),
        Some("planetary") => planetary_report(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(3),
            args.get(4).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(5).and_then(|s| s.parse().ok()).unwrap_or(0.0),
        ),
        Some("verify") => verify(
            args.get(1)
                .and_then(|s| s.parse().ok())
                .unwrap_or(usize::MAX),
        ),
        Some("show") | None => {
            let teeth = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17);
            let x = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0);
            show(GearParams {
                teeth,
                profile_shift: x,
                ..Default::default()
            });
        }
        Some(other) => eprintln!("unknown command {other:?}; try `show` or `sweep`"),
    }
}

/// A two-stage geartrain, end to end — milestone 6's gate.
fn train_report(mixed: bool) {
    use gear_core::params::Auto;
    use gear_core::train::{
        solve_train, Actuation, SpurStage, Stage, StageGear, StageResult, Train, WormStage,
    };

    let lib = gear_io::default_library();
    let auto_width = |teeth: u32| StageGear {
        teeth,
        face_width: Auto::automatic(0.0),
        ..StageGear::default()
    };
    let train = Train {
        input_speed: 3000.0,
        input_torque: 2.0,
        actuation: Actuation::Continuous {
            operating_percent: 80.0,
            runtime_hours: 1000.0,
        },
        stages: if mixed {
            vec![
                Stage::Spur(SpurStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..SpurStage::default()
                }),
                Stage::Worm(WormStage {
                    wheel: gear_core::train::WormMember {
                        material: "Brass C360".into(),
                        ..gear_core::train::WormMember::default()
                    },
                    ..WormStage::default()
                }),
            ]
        } else {
            vec![
                Stage::Spur(SpurStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..SpurStage::default()
                }),
                Stage::Spur(SpurStage {
                    helix_angle: 15.0,
                    gears: [auto_width(13), auto_width(31)],
                    ..SpurStage::default()
                }),
            ]
        },
    };

    let r = match solve_train(&train, &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("train did not solve: {e}");
            return;
        }
    };

    println!(
        "train  in {:.0} rpm / {:.3} Nm   ->   out {:.1} rpm / {:.3} Nm",
        train.input_speed, train.input_torque, r.output_speed, r.output_torque
    );
    println!(
        "       total ratio {:.4}:1   total efficiency {:.3} % forward / {:.3} % backward{}",
        r.total_ratio,
        100.0 * r.total_efficiency.forward,
        100.0 * r.total_efficiency.backward,
        if r.total_efficiency.self_locking() {
            "  (cannot be back-driven)"
        } else {
            ""
        }
    );
    println!(
        "       backlash at the output shaft  {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.forward.nominal, r.backlash.forward.minimum, r.backlash.forward.maximum
    );
    println!(
        "       backlash at the input shaft   {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.backward.nominal, r.backlash.backward.minimum, r.backlash.backward.maximum
    );

    for (k, s) in r.stages.iter().enumerate() {
        match (&train.stages[k], s) {
            (Stage::Spur(st), StageResult::Spur(res)) => print_spur_stage(k, st, res),
            (Stage::Worm(st), StageResult::Worm(res)) => print_worm_stage(k, st, res),
            _ => println!("\nstage {}: kind and result disagree", k + 1),
        }
    }
}

fn print_spur_stage(k: usize, st: &gear_core::train::SpurStage, s: &gear_core::train::SpurResult) {
    println!(
        "\nstage {}  spur  z {}/{}  beta {} deg  ratio {:.4}  a_w {:.4} mm{}",
        k + 1,
        st.gears[0].teeth,
        st.gears[1].teeth,
        st.helix_angle,
        s.ratio,
        s.centre_distance,
        if s.coprime { "  coprime" } else { "" }
    );
    println!(
        "  contact ratio  transverse {:.4}   overlap {:.4}   total {:.4}{}",
        s.contact_ratios.transverse,
        s.contact_ratios.overlap,
        s.contact_ratios.total,
        if st.helix_angle != 0.0 && !s.contact_ratios.has_full_axial_overlap() {
            "   <- no full axial overlap"
        } else {
            ""
        }
    );
    println!(
        "  efficiency {:.3} % forward / {:.3} % backward",
        100.0 * s.efficiency.forward,
        100.0 * s.efficiency.backward
    );
    println!(
        "  {:<6} {:>8} {:>8} {:>10} {:>10} {:>10} {:>9} {:>12}",
        "gear", "x", "b mm", "torque Nm", "sigma_F", "sigma_H", "rpm", "cycles"
    );
    for (i, g) in s.gears.iter().enumerate() {
        println!(
            "  {:<6} {:>8.4} {:>8.3} {:>10.4} {:>6.1} MPa {:>5.1} MPa {:>9.1} {:>12.3e}",
            i + 1,
            g.profile_shift,
            g.face_width,
            g.torque,
            g.bending_stress.unwrap_or(f64::NAN),
            g.contact_stress,
            g.speed,
            g.tooth_cycles
        );
    }
    for n in &s.notes {
        println!("  note: {n}");
    }
}

fn print_worm_stage(k: usize, st: &gear_core::train::WormStage, s: &gear_core::train::WormResult) {
    println!(
        "\nstage {}  worm  z {}/{}  ratio {:.4}  a {:.4} mm  lead angle {:.4} deg",
        k + 1,
        st.starts,
        st.wheel_teeth,
        s.ratio,
        s.centre_distance,
        s.lead_angle
    );
    println!(
        "  efficiency  forward {:.3} %  backward {:.3} %{}",
        100.0 * s.efficiency.forward,
        100.0 * s.efficiency.backward,
        if s.efficiency.self_locking() {
            "  (self-locking)"
        } else {
            ""
        }
    );
    println!(
        "  contact {:.1} MPa   patch {:.4} x {:.4} mm   sliding {:.1} mm/s",
        s.contact.max_pressure, s.contact.patch_length, s.contact.patch_width, s.sliding_velocity
    );
    println!(
        "  {:<6} {:>8} {:>10} {:>9} {:>12}   material",
        "member", "b mm", "torque Nm", "rpm", "cycles"
    );
    for (name, m) in ["worm", "wheel"].iter().zip(&s.members) {
        println!(
            "  {name:<6} {:>8.3} {:>10.4} {:>9.1} {:>12.3e}   {}",
            m.face_width, m.torque, m.speed, m.tooth_cycles, m.material.name
        );
    }
    println!("  bending not reported, flank type ZI - see DESIGN.md 4.5.1");
    for n in &s.notes {
        println!("  note: {n}");
    }
}

/// A worked mesh end to end: load, bending, contact, efficiency, face width.
///
/// This is the whole of milestone 5 in one view, and the first thing that
/// actually consumes the material library. Both gears are rated, because the
/// pinion is not automatically the worse one — it sees the higher contact
/// stress but the wheel may have the weaker root.
fn strength_report(z1: u32, z2: u32, torque: f64, material_name: &str, helix: f64) {
    use gear_core::contact::{efficiency, ContactPath, Drive};
    use gear_core::material::contact_modulus;
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::metrology::base_helix_angle;
    use gear_core::strength::{
        bending_section, bending_stress, contact_stress, min_face_width_bending,
        min_face_width_contact, Load, StressConcentration, PARALLEL_AXES,
    };

    let lib = gear_io::default_library();
    let Some(mat) = lib.get(material_name) else {
        eprintln!("no material named {material_name:?}; try `gear-cli materials`");
        return;
    };

    // Meshing helical gears have equal and opposite hands.
    let g1 = Gear::new(GearParams {
        teeth: z1,
        helix_angle: helix,
        ..Default::default()
    });
    let g2 = Gear::new(GearParams {
        teeth: z2,
        helix_angle: -helix,
        ..Default::default()
    });
    let Ok(mesh) = Mesh::new(&g1, &g2, MeshKind::External) else {
        eprintln!("z={z1}/{z2} cannot mesh");
        return;
    };
    let Some(path) = ContactPath::new(&g1, g2.ra, &mesh) else {
        eprintln!("z={z1}/{z2} has no usable path of contact");
        return;
    };

    // A face width to evaluate at. Any value does: the minimum face widths
    // below are independent of it, which is asserted in the test suite.
    const B: f64 = 10.0;
    let load = Load::new(torque, B);

    println!(
        "mesh   z {z1}/{z2}  module {}  a_w {:.4} mm",
        g1.params.module, mesh.a_w
    );
    println!(
        "       operating pressure angle {:.3} deg  contact ratio {:.4}",
        mesh.alpha_w.to_degrees(),
        path.contact_ratio
    );
    if helix != 0.0 {
        println!(
            "helix  beta {helix} deg  base helix beta_b {:.3} deg  virtual teeth {:.2}/{:.2}",
            base_helix_angle(&g1).to_degrees(),
            g1.virtual_spur().z,
            g2.virtual_spur().z
        );
    }
    println!(
        "load   {torque} Nm on gear 1  ->  F_n {:.1} N along the line of action",
        load.transverse_line_of_action(&g1)
    );
    println!(
        "       F_t {:.1} N at the reference circle,  face width {B} mm",
        load.tangential(&g1)
    );
    println!(
        "material  {}  [{}]",
        mat.name,
        if mat.weakest_basis().is_measured() {
            "all values measured"
        } else {
            "contains estimates - see `gear-cli materials`"
        }
    );
    println!(
        "       E {:.0} MPa   nu {:.2}   ultimate {:.1} MPa   fatigue {:.1} MPa",
        mat.elastic_modulus.value,
        mat.poissons_ratio.value,
        mat.ultimate_allowable.value,
        mat.fatigue_allowable.value
    );
    println!();

    // --- bending, each gear at its own highest point of single-pair contact
    println!("bending");
    println!(
        "  {:<6} {:>8} {:>8} {:>9} {:>10} {:>10}",
        "gear", "Y_F", "Y_S", "sigma_F", "b_min fat", "b_min ult"
    );
    let reversed = Mesh::new(&g2, &g1, MeshKind::External).ok();
    for (label, g, p) in [
        (1u32, &g1, Some(path)),
        (
            2,
            &g2,
            reversed.and_then(|m| ContactPath::new(&g2, g1.ra, &m)),
        ),
    ] {
        let Some(p) = p else {
            println!("  {label:<6} no contact path");
            continue;
        };
        let Some(sec) = bending_section(g, p.contact_ratio) else {
            println!("  {label:<6} no root section (severed tooth?)");
            continue;
        };
        let load_g = load.across_mesh(&g1, g);
        let ys = sec.stress_correction(StressConcentration::Iso6336);
        let Some(sf) = bending_stress(&sec, g, &load_g, StressConcentration::Iso6336) else {
            println!(
                "  {label:<6} {:>8.4} {:>8} {:>9} - stress correction undefined (tangency on the flank)",
                sec.form_factor, "-", "-"
            );
            continue;
        };
        println!(
            "  {label:<6} {:>8.4} {:>8.4} {:>7.1} MPa {:>8.3} mm {:>8.3} mm",
            sec.form_factor,
            ys.unwrap_or(1.0),
            sf,
            min_face_width_bending(sf, B, mat.fatigue_allowable.value),
            min_face_width_bending(sf, B, mat.ultimate_allowable.value),
        );
        if !sec.notch_parameter_in_range() {
            println!(
                "         note: notch parameter q_s = {:.2} is outside the ISO fit's range, so",
                sec.notch_parameter
            );
            println!("               Y_S was clamped and the stress is UNDER-predicted");
        }
    }

    // --- contact, shared by the pair
    let e_star = contact_modulus(mat, mat);
    if let Some(cs) = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star) {
        println!("\ncontact   E* {e_star:.0} MPa (like on like)");
        println!("  at the pitch point        {:>7.1} MPa", cs.at_pitch_point);
        println!(
            "  at single-pair contact    {:>7.1} MPa   <- governs",
            cs.at_single_pair
        );
        println!("  relative radius           {:>7.3} mm", cs.relative_radius);
        println!(
            "  b_min against fatigue     {:>7.3} mm",
            min_face_width_contact(cs.worst, B, mat.fatigue_allowable.value)
        );
        println!(
            "  b_min against ultimate    {:>7.3} mm",
            min_face_width_contact(cs.worst, B, mat.ultimate_allowable.value)
        );
    }

    // --- efficiency
    println!("\nefficiency (equal in both directions for a parallel-axis mesh)");
    for mu in [0.02, 0.04, 0.06, 0.10] {
        println!(
            "  mu {mu:.2}   {:.3} %",
            100.0 * efficiency(&path, &mesh, &g1, mu, Drive::Forward)
        );
    }
}

/// The shipped material library, with each value's provenance.
///
/// The `basis` column is the point of this view: the library deliberately
/// contains estimates as well as measurements, and the difference should be
/// visible at a glance rather than buried in the TOML.
fn materials() {
    let lib = gear_io::default_library();
    println!(
        "{:<20} {:>8} {:>9} {:>6} {:>10} {:>9}",
        "material", "rho", "E", "nu", "ultimate", "fatigue"
    );
    println!(
        "{:<20} {:>8} {:>9} {:>6} {:>10} {:>9}",
        "", "kg/m3", "MPa", "", "MPa", "MPa"
    );
    println!("{}", "-".repeat(67));

    for m in &lib.materials {
        // A single letter per value, so a row reads as a confidence pattern:
        // `d` datasheet, `D` derived, `c` chart, `e` estimated.
        let tag = |v: &gear_core::material::Value| match v.basis {
            gear_core::material::Basis::Overridden => 'o',
            gear_core::material::Basis::Datasheet => 'd',
            gear_core::material::Basis::Derived => 'D',
            gear_core::material::Basis::Chart => 'c',
            gear_core::material::Basis::Estimated => 'e',
        };
        println!(
            "{:<20} {:>7.0}{} {:>8.0}{} {:>5.2}{} {:>9.1}{} {:>8.1}{}",
            m.name,
            m.density.value,
            tag(&m.density),
            m.elastic_modulus.value,
            tag(&m.elastic_modulus),
            m.poissons_ratio.value,
            tag(&m.poissons_ratio),
            m.ultimate_allowable.value,
            tag(&m.ultimate_allowable),
            m.fatigue_allowable.value,
            tag(&m.fatigue_allowable),
        );
    }

    println!("\nbasis: d datasheet   D derived   c read off a chart   e estimated   o overridden");
    println!("each entry is one material in one state; the condition names it\n");

    for m in &lib.materials {
        println!("{}  [{}]", m.name, m.grade);
        println!("  condition: {}", m.condition);
        println!("  source:    {}", m.source);
        for (label, v) in [
            ("density", &m.density),
            ("modulus", &m.elastic_modulus),
            ("poisson", &m.poissons_ratio),
            ("ultimate", &m.ultimate_allowable),
            ("fatigue", &m.fatigue_allowable),
        ] {
            if let Some(note) = &v.note {
                println!("  {label:<9} {note}");
            }
        }
        println!();
    }
}

fn show(p: GearParams) {
    let g = Gear::new(p);
    println!(
        "module {}  z {}  alpha {}  x {:+}  beta {}  k {}",
        p.module, p.teeth, p.pressure_angle, p.profile_shift, p.helix_angle, p.thickness_mod
    );
    println!("  pitch radius   {:12.6}", g.r);
    println!("  base radius    {:12.6}", g.rb);
    println!("  tip radius     {:12.6}", g.ra);
    println!("  root radius    {:12.6}", g.rf);
    println!("  tooth thick.   {:12.6}  (transverse, at pitch)", g.st);
    println!("  fillet radius  {:12.6}", g.rho);
    println!(
        "  L              {:12.6}  ({})",
        g.l,
        if g.undercut {
            "UNDERCUT"
        } else {
            "no undercut"
        }
    );
    println!("  junction r     {:12.6}", g.r_j);
    println!("  severed        {:12}", g.severed);
    if g.clamps.any() {
        println!("  clamps:");
        for n in &g.clamps.notes {
            println!("    - {n}");
        }
    }
    let pts = g.profile(400);
    println!("  profile points {:12}", pts.len());
}

fn sweep() {
    let mut total = 0u32;
    let mut undercut = 0u32;
    let mut severed = 0u32;
    let mut clamped = 0u32;
    for z in 3..=60u32 {
        for xi in -6..=10i32 {
            for alpha in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0] {
                    let g = Gear::new(GearParams {
                        teeth: z,
                        profile_shift: f64::from(xi) * 0.1,
                        pressure_angle: alpha,
                        helix_angle: beta,
                        ..Default::default()
                    });
                    total += 1;
                    undercut += u32::from(g.undercut);
                    severed += u32::from(g.severed);
                    clamped += u32::from(g.clamps.any());
                }
            }
        }
    }
    println!("{total} cases: {undercut} undercut, {severed} severed, {clamped} clamped");
}

/// Emit the same case grid as `tools/dump_ref.py` so the port can be compared
/// point by point against the Python reference it was derived from.
fn dump() {
    for z in [3u32, 5, 8, 9, 11, 13, 17, 23, 31, 47, 60] {
        for x in [-0.5_f64, -0.2, 0.0, 0.2, 0.5, 0.9] {
            for an in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0, -30.0] {
                    for rr in [0.0_f64, 0.38] {
                        let g = Gear::new(GearParams {
                            module: 1.0,
                            pressure_angle: an,
                            teeth: z,
                            profile_shift: x,
                            helix_angle: beta,
                            addendum: 1.0,
                            dedendum: 1.25,
                            root_radius: rr,
                            thickness_mod: 1.0,
                        });
                        println!(
                            "S\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{:.17e}\t{}\t{}\t{}",
                            g.r, g.rb, g.ra, g.rf, g.st, g.rho, g.bc, g.ac, g.l,
                            g.u_j, g.s_j, g.r_j, g.theta0, g.theta_a, g.psi_b,
                            u8::from(g.undercut), u8::from(g.severed), g.clamps.notes.len()
                        );
                        let (r, th) = g.half_profile(200);
                        let j = |v: &Vec<f64>| {
                            v.iter()
                                .map(|x| format!("{x:.17e}"))
                                .collect::<Vec<_>>()
                                .join(",")
                        };
                        println!("R\t{}", j(&r));
                        println!("T\t{}", j(&th));
                    }
                }
            }
        }
    }
}

/// Two-sided cutter verification over a parameter grid.
fn verify(limit: usize) {
    use gear_core::verify::{check_cut, fillet_envelope_error, sdf_matches_polyline};
    let mut n = 0usize;
    let mut worst_pen = 0.0_f64;
    let mut worst_dev = 0.0_f64;
    let mut worst_env = 0.0_f64;
    let mut worst_sdf = 0.0_f64;
    let (mut pen_case, mut dev_case) = (String::new(), String::new());
    'outer: for z in [3u32, 5, 8, 9, 11, 13, 17, 23, 31, 47] {
        for xi in [-5i32, -2, 0, 2, 5, 9] {
            for an in [14.5_f64, 20.0, 25.0] {
                for beta in [0.0_f64, 20.0] {
                    for rr in [0.0_f64, 0.25, 0.38] {
                        let p = GearParams {
                            teeth: z,
                            profile_shift: f64::from(xi) * 0.1,
                            pressure_angle: an,
                            helix_angle: beta,
                            root_radius: rr,
                            ..Default::default()
                        };
                        let g = Gear::new(p);
                        let rep = check_cut(&g, 150);
                        let env = fillet_envelope_error(&g, 150, 4000);
                        let sdf = sdf_matches_polyline(&g, 400, 4000);
                        let tag = format!(
                            "z={z} x={:+.1} a={an} b={beta} rho={rr}",
                            f64::from(xi) * 0.1
                        );
                        if rep.penetration > worst_pen {
                            worst_pen = rep.penetration;
                            pen_case = tag.clone();
                        }
                        if rep.deviation > worst_dev {
                            worst_dev = rep.deviation;
                            dev_case = tag;
                        }
                        worst_env = worst_env.max(env);
                        worst_sdf = worst_sdf.max(sdf);
                        n += 1;
                        if n >= limit {
                            break 'outer;
                        }
                    }
                }
            }
        }
    }
    println!("{n} cases");
    println!("  worst penetration        {worst_pen:.6e} mm   {pen_case}");
    println!("  worst deviation          {worst_dev:.6e} mm   {dev_case}");
    println!("  worst fillet envelope    {worst_env:.6e} mm");
    println!("  worst sdf vs polyline    {worst_sdf:.6e} mm");
}

/// Write a DXF to stdout, for inspecting or importing into CAD.
fn dxf(teeth: u32, x: f64, tol: f64) {
    let g = Gear::new(GearParams {
        teeth,
        profile_shift: x,
        ..Default::default()
    });
    print!(
        "{}",
        gear_io::gear_to_dxf(
            &g,
            &gear_io::DxfOptions {
                chord_tolerance: tol,
                reference_circles: true,
            }
        )
    );
}

/// Emit an HTML page showing the bending construction, for visual checking.
fn bending_report() {
    let cases = [
        (9u32, 0.0),
        (13, 0.0),
        (17, 0.0),
        (17, 0.5),
        (30, 0.0),
        (60, 0.0),
        (12, -0.3),
        (12, 0.4),
    ];
    println!("<h2>The construction, tooth by tooth</h2>");
    println!(
        "<p class=\"lede\">Load at the tooth tip. \
        Grey dashed: tip, pitch, base and root circles. \
        <span class=\"k tangent\">Amber</span>: the 30° tangents and their tangency points. \
        <span class=\"k chord\">Red</span>: the critical section chord s_Fn. \
        <span class=\"k loadline\">Blue</span>: the load line, from the contact point to the \
        centreline. <span class=\"k arm\">Green</span>: the moment arm h_Fe.</p>"
    );
    println!("<div class=\"grid\">");
    for (teeth, profile_shift) in cases {
        let p = GearParams {
            teeth,
            profile_shift,
            ..Default::default()
        };
        println!("<figure>{}", diagram::tooth_diagram(p, 210.0));
        println!(
            "<figcaption>{}</figcaption></figure>",
            diagram::tooth_caption(p)
        );
    }
    println!("</div>");
    println!("<h2>Form factor against tooth count</h2>");
    println!("{}", diagram::form_factor_chart(660.0, 380.0));
}

/// Precision study of the bending-model matrix. Text, so the numbers can be
/// read before anything is drawn from them.
fn matrix_report() {
    let pop = matrix::population();
    println!("population: {} designs\n", pop.len());

    println!("== 1. continuity ==");
    println!(
        "{:<24} {:>18} {:>10} {:>22}",
        "model", "worst z step", "at z", "worst x step (fine)"
    );
    for m in matrix::MATRIX {
        let (step, at) = matrix::continuity_in_tooth_count(m);
        let xs = matrix::continuity_in_profile_shift(m, 120);
        println!(
            "{:<24} {:>17.4}% {:>10} {:>21.5}%",
            m.name(),
            100.0 * step,
            at,
            100.0 * xs
        );
    }

    println!("\n== 2. rank agreement (Spearman) ==");
    for i in 0..matrix::MATRIX.len() {
        for j in (i + 1)..matrix::MATRIX.len() {
            let (r, n) = matrix::rank_correlation(matrix::MATRIX[i], matrix::MATRIX[j], &pop);
            println!(
                "  {:<22} vs {:<22} rho = {r:.6}  (n={n})",
                matrix::MATRIX[i].name(),
                matrix::MATRIX[j].name()
            );
        }
    }

    for thresh in [0.0_f64, 0.002, 0.01] {
        println!(
            "\n== 3. gradient sign agreement, ignoring effects below {:.1}% ==",
            100.0 * thresh
        );
        println!(
            "  {:<48} {:>7} {:>7} {:>7}",
            "", "shift", "fillet", "dedend"
        );
        for i in 0..matrix::MATRIX.len() {
            for j in (i + 1)..matrix::MATRIX.len() {
                let g =
                    matrix::gradient_agreement(matrix::MATRIX[i], matrix::MATRIX[j], &pop, thresh);
                println!(
                    "  {:<22} vs {:<22} {:6.1}% {:6.1}% {:6.1}%",
                    matrix::MATRIX[i].name(),
                    matrix::MATRIX[j].name(),
                    100.0 * g[0],
                    100.0 * g[1],
                    100.0 * g[2]
                );
            }
        }
    }

    println!("\n== 4. divergence across the matrix, by tooth count (x=0, rho=0.38) ==");
    println!("{:>6} {:>12}", "z", "spread");
    for teeth in [9u32, 12, 17, 25, 40, 70, 120, 250] {
        let p = GearParams {
            teeth,
            ..Default::default()
        };
        if let Some(d) = matrix::divergence(p) {
            println!("{teeth:>6} {:>11.2}%", 100.0 * d);
        }
    }
    println!("\n   worst divergence over the whole population:");
    let mut worst = (0.0_f64, GearParams::default());
    for p in &pop {
        if let Some(d) = matrix::divergence(*p) {
            if d > worst.0 {
                worst = (d, *p);
            }
        }
    }
    println!(
        "   {:.2}% at z={} x={:+.1} rho={} alpha={}",
        100.0 * worst.0,
        worst.1.teeth,
        worst.1.profile_shift,
        worst.1.root_radius,
        worst.1.pressure_angle
    );
}

/// Compare the three load cases on a few ordinary meshes.
///
/// (a) worst case  -- load at the tip, this tooth carrying everything
/// (b) HPSTC       -- load at the highest point of single-pair contact
/// (c) shared      -- worst point of the mesh cycle with load sharing applied
fn loadcase_report() {
    use gear_core::contact::{ContactPath, LoadSharing};
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::strength::{root_section, StressConcentration};

    let meshes = [
        ("pinion 17 : 17", 17u32, 17u32, 0.0_f64),
        ("pinion 17 : 43", 17, 43, 0.0),
        ("pinion 13 : 60", 13, 60, 0.0),
        ("pinion 25 : 25", 25, 25, 0.0),
        ("pinion 12 : 30, x=+0.4", 12, 30, 0.4),
        ("pinion 20 : 20, x=-0.2", 20, 20, -0.2),
    ];

    println!(
        "{:<24} {:>6} {:>9} {:>9} {:>9} {:>8} {:>8}",
        "mesh", "eps", "(a) tip", "(b) HPSTC", "(c) shared", "b vs a", "c vs a"
    );

    for (name, z1, z2, x) in meshes {
        let p1 = GearParams {
            teeth: z1,
            profile_shift: x,
            ..Default::default()
        };
        let p2 = GearParams {
            teeth: z2,
            profile_shift: -x,
            ..Default::default()
        };
        let (g1, g2) = (Gear::new(p1), Gear::new(p2));
        let Ok(m) = Mesh::new(&g1, &g2, MeshKind::External) else {
            println!("{name}: mesh failed");
            continue;
        };
        let Some(path) = ContactPath::new(&g1, g2.ra, &m) else {
            println!("{name}: path failed");
            continue;
        };

        // The bending factor is proportional to stress for a fixed torque, so
        // ratios of (factor x load fraction) are ratios of stress.
        let factor = |roll: f64| {
            root_section(&g1, roll).and_then(|s| s.bending_factor(StressConcentration::Iso6336))
        };

        let Some(a) = factor(path.roll_at(path.tip())) else {
            println!("{name}: tip factor failed");
            continue;
        };
        let Some(b) = factor(path.roll_at(path.highest_single_pair())) else {
            println!("{name}: hpstc factor failed");
            continue;
        };

        // (c): the worst point of the whole cycle once sharing is applied.
        let mut c = 0.0_f64;
        for i in 0..=400 {
            let t = f64::from(i) / 400.0;
            let xi = -path.approach + t * (path.approach + path.recess);
            if let Some(f) = factor(path.roll_at(xi)) {
                c = c.max(f * path.load_fraction(xi, LoadSharing::LinearRamp));
            }
        }

        println!(
            "{name:<24} {:>6.3} {a:>9.4} {b:>9.4} {c:>9.4} {:>7.1}% {:>7.1}%",
            path.contact_ratio,
            100.0 * (b - a) / a,
            100.0 * (c - a) / a
        );
    }
}

/// A worm pair, end to end: geometry, sliding, and both drive directions.
fn worm_report(starts: u32, wheel_teeth: u32, worm_diameter: f64, shaft_angle_deg: f64) {
    use gear_core::contact::{Directional, Drive};
    use gear_core::mesh::Member;
    use gear_core::screw::{Screw, ScrewParams};

    let params = ScrewParams {
        starts,
        wheel_teeth,
        worm_pitch_diameter: worm_diameter,
        shaft_angle: shaft_angle_deg.to_radians(),
        ..Default::default()
    };
    let s = match Screw::new(&params) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("cannot build that pair: {e:?}");
            return;
        }
    };

    println!(
        "worm   z {}/{}  module {}  alpha_n {:.1} deg  shaft angle {:.1} deg",
        starts,
        wheel_teeth,
        params.normal_module,
        params.normal_pressure_angle.to_degrees(),
        shaft_angle_deg
    );
    println!(
        "       ratio {:.4}:1   centre distance {:.4} mm",
        s.ratio, s.centre_distance
    );
    println!();
    println!("geometry");
    println!(
        "  lead angle          worm {:8.4} deg    wheel {:8.4} deg",
        s.lead_angle.to_degrees(),
        s.wheel_lead_angle.to_degrees()
    );
    println!(
        "  helix angle         worm {:8.4} deg    wheel {:8.4} deg",
        s.worm_helix_angle.to_degrees(),
        s.wheel_helix_angle.to_degrees()
    );
    println!(
        "  pitch diameter      worm {:8.4} mm     wheel {:8.4} mm",
        s.worm_pitch_diameter, s.wheel_pitch_diameter
    );
    println!(
        "  lead {:.4} mm   axial module {:.5} mm",
        s.lead, s.axial_module
    );
    println!(
        "  sliding at the pitch point   {:.4} x the worm's pitch line speed",
        s.sliding_ratio
    );

    println!();
    println!("efficiency          worm driving   wheel driving");
    for mu in [0.0, 0.02, 0.04, 0.06, 0.10] {
        let e = Directional::of(|d| s.efficiency(mu, d));
        let back = if e.self_locking() {
            "  self-locking".to_string()
        } else {
            format!("{:12.3} %", e.backward * 100.0)
        };
        println!("  mu {mu:.2}        {:10.3} % {back}", e.forward * 100.0);
    }
    let threshold = s.self_locking_friction();
    println!("  self-locks at mu >= {threshold:.4}   (cos alpha_n tan gamma)");

    // Contact is the strength figure a worm stage reports. There is deliberately
    // no bending stress here; DESIGN.md §4.5.1 says why.
    let lib = gear_io::default_library();
    let (worm_material, wheel_material) = ("4340 Hardened Steel", "Brass C360");
    let (Some(m1), Some(m2)) = (lib.get(worm_material), lib.get(wheel_material)) else {
        return;
    };
    let e_star = gear_core::material::contact_modulus(m1, m2);
    let mu = 0.06;
    let torque_in = 2.0;
    let torque_out = torque_in * s.ratio * s.efficiency(mu, Drive::Forward);

    println!();
    println!("contact   {worm_material} on {wheel_material},  E* {e_star:.0} MPa");
    println!("  worm torque {torque_in:.3} Nm  ->  wheel torque {torque_out:.3} Nm at mu {mu}");
    let (flat, sharp) = match s.contact_curvatures() {
        Some(c) => c,
        None => {
            println!("  the flanks do not touch at a point");
            return;
        }
    };
    println!("  relative curvature   along {flat:.6} /mm   across {sharp:.6} /mm");
    if let Some(c) = s.contact(torque_out, Member::Second, mu, e_star) {
        println!(
            "  patch  {:.4} x {:.4} mm   (rated on the wheel's torque)",
            c.semi_major() * 2.0,
            c.semi_minor() * 2.0
        );
        println!("  peak pressure               {:.1} MPa", c.max_pressure);
        println!(
            "  sliding speed at 3000 rpm   {:.1} mm/s",
            s.sliding_ratio * 3000.0 / 60.0 * std::f64::consts::TAU * s.worm_pitch_diameter / 2.0
        );
    }
    println!("  bending                     not reported - see DESIGN.md 4.5.1");
    println!(
        "  flank type   ZI (involute helicoid). A ZN worm's contact stress is\n\
         {:24}1-15 % lower, rising with lead angle - see DESIGN.md 4.5.1",
        ""
    );
}

/// A worm stage end to end: geometry, both directions, contact and backlash.
fn worm_stage_report(starts: u32, wheel_teeth: u32, worm_diameter: f64, torque: f64) {
    use gear_core::train::{solve_worm_stage, WormMember, WormStage};

    let stage = WormStage {
        starts,
        wheel_teeth,
        sizing: gear_core::train::FirstMemberSizing::PitchDiameter(worm_diameter),
        wheel: WormMember {
            material: "Brass C360".into(),
            ..WormMember::default()
        },
        ..WormStage::default()
    };
    let lib = gear_io::default_library();
    let r = match solve_worm_stage(&stage, torque, &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot solve that stage: {e}");
            return;
        }
    };

    println!(
        "worm stage  z {starts}/{wheel_teeth}  module {}  ratio {:.4}:1  a {:.4} mm",
        stage.module, r.ratio, r.centre_distance
    );
    println!(
        "  lead angle {:.4} deg   wheel helix {:.4} deg   lead {:.4} mm",
        r.lead_angle, r.wheel_helix_angle, r.lead
    );
    println!();
    println!("  member      torque Nm   face mm   d mm      material");
    for (name, m) in ["worm", "wheel"].iter().zip(&r.members) {
        println!(
            "  {name:<10} {:9.4} {:9.3} {:9.4}   {}",
            m.torque, m.face_width, m.pitch_diameter, m.material.name
        );
    }
    println!();
    println!(
        "  efficiency   forward {:.3} %   backward {:.3} %{}",
        r.efficiency.forward * 100.0,
        r.efficiency.backward * 100.0,
        if r.efficiency.self_locking() {
            "  (self-locking)"
        } else {
            ""
        }
    );
    println!(
        "  contact      {:.1} MPa   patch {:.4} x {:.4} mm",
        r.contact.max_pressure, r.contact.patch_length, r.contact.patch_width
    );
    println!(
        "  backlash     at the wheel {:.5} deg (min {:.5}, max {:.5})   at the worm {:.5} deg",
        r.backlash.forward.nominal,
        r.backlash.forward.minimum,
        r.backlash.forward.maximum,
        r.backlash.backward.nominal
    );
    println!("  bending      not reported - see DESIGN.md 4.5.1");
    println!(
        "  flank type   ZI (involute helicoid); a ZN worm's contact stress is\n\
         {:15}1-15 % lower, rising with lead angle - see DESIGN.md 4.5.1",
        ""
    );
    for note in &r.notes {
        println!("  ! {note}");
    }
}

/// The ring tooth counts a sun and planet pair admits, and what each costs in
/// planet shift.
///
/// Prints every candidate rather than picking one, because the choice is a
/// designer's: the geometric ideal needs no shift but rarely spaces the planets
/// evenly, and the one that does costs a shift. Both facts are on the same row.
fn planetary_report(sun: u32, planet: u32, planets: u32, sun_shift: f64, ring_shift: f64) {
    use gear_core::planetary::{ring_candidates, shift_bracket, solve, Rack, Set, Teeth};

    let module = 1.0;
    let set = Set {
        rack: Rack::new(module, 20.0, 0.0),
        teeth: Teeth {
            sun,
            planet,
            ring: 0,
        },
        planets,
        sun_shift,
        ring_shift,
        // A planet's tip diameter at a standard addendum, which is what the
        // clearance column is measured against.
        planet_tip_diameter: module * (f64::from(planet) + 2.0),
    };

    println!(
        "planetary  z_sun {sun}  z_planet {planet}  N {planets}  \
         x_sun {sun_shift}  x_ring {ring_shift}  module {module}  alpha 20 deg"
    );
    println!(
        "ideal ring (needs no planet shift): {}",
        Teeth::ideal_ring(sun, planet)
    );

    // Everything the involute domain admits, whatever shift it costs. The range
    // is deliberately wide: this is the "what is possible" listing.
    let all = ring_candidates(
        &set,
        (f64::NEG_INFINITY, f64::INFINITY),
        4 * (sun + 2 * planet),
    );
    if all.is_empty() {
        println!("\nno ring tooth count admits a solution for that sun and planet");
        return;
    }

    println!(
        "\n{:>6} {:>10} {:>12} {:>10} {:>9} {:>7} {:>7} {:>11}",
        "z_ring", "x_planet", "c2c mm", "residual", "a_w sun", "even", "simult", "clearance"
    );
    for (ring, l) in &all {
        let clearance = l
            .planet_clearance
            .map_or_else(|| "     n/a".to_string(), |c| format!("{c:8.3}"));
        println!(
            "{ring:>6} {:>10.4} {:>12.6} {:>10.1e} {:>9.3} {:>7} {:>7} {clearance:>11}",
            l.planet_shift,
            l.centre_distance,
            l.residual,
            l.alpha_w_sun.to_degrees(),
            if l.equal_spacing { "yes" } else { "no" },
            if l.simultaneous_meshing { "yes" } else { "no" },
        );
    }

    // The bracket is why the list stops where it does, so say so with numbers.
    let widest = all.last().map(|(z, _)| *z).unwrap_or(0);
    let beyond = Set {
        teeth: Teeth {
            ring: widest + 1,
            ..set.teeth
        },
        ..set
    };
    print!("\nwhy it stops: z_ring {} ", widest + 1);
    match shift_bracket(&beyond) {
        None => println!("has no admissible planet shift at all"),
        Some((lo, hi)) => {
            let inside = solve(&beyond).is_some();
            println!(
                "admits x_planet in [{lo:.4}, {hi:.4}] but {}",
                if inside {
                    "was excluded by the sweep limit"
                } else {
                    "no shift in it equalises the two centre distances"
                }
            );
        }
    }
}

/// A planetary stage, end to end, in all six arrangements.
fn planetary_stage_report(sun: u32, planet: u32, ring: u32, planets: u32, helix: f64) {
    use gear_core::planetary::{Arrangement, Member};
    use gear_core::train::{solve_planetary_stage, PlanetaryStage, StageGear};

    let lib = gear_io::default_library();
    let base = PlanetaryStage {
        helix_angle: helix,
        planets,
        sun: StageGear {
            teeth: sun,
            ..StageGear::default()
        },
        planet: StageGear {
            teeth: planet,
            ..StageGear::default()
        },
        ring: StageGear {
            teeth: ring,
            profile_shift: gear_core::Auto::fixed(0.0),
            ..StageGear::default()
        },
        ..PlanetaryStage::default()
    };

    println!(
        "planetary stage  z {sun}/{planet}/{ring}  N={planets}  helix {helix} deg  \
         module {}  alpha {} deg",
        base.module, base.pressure_angle
    );
    let all = [Member::Sun, Member::Carrier, Member::Ring];
    let name = |m: Member| match m {
        Member::Sun => "sun",
        Member::Carrier => "carrier",
        Member::Ring => "ring",
    };

    let mut shown = false;
    for &input in &all {
        for &fixed in &all {
            if input == fixed {
                continue;
            }
            let stage = PlanetaryStage {
                arrangement: Arrangement { input, fixed },
                ..base.clone()
            };
            match solve_planetary_stage(&stage, 3000.0, 2.0, &lib) {
                Err(e) => println!("  {:>7} in, {:>7} held: {e}", name(input), name(fixed)),
                Ok(r) => {
                    if !shown {
                        println!(
                            "\ncommon centre distance {:.6} mm (residual {:.1e})  \
                             planet shift {:+.4}",
                            r.centre_distance_nominal,
                            r.planet.shift_residual,
                            r.planet.profile_shift
                        );
                        println!(
                            "eps_a  sun-planet {:.3}   planet-ring {:.3}   \
                             eta_0 {:.4}   even spacing {}   planet gap {}",
                            r.sun_planet.contact_ratios.transverse,
                            r.planet_ring.contact_ratios.transverse,
                            r.fixed_carrier_efficiency.forward,
                            r.equal_spacing,
                            r.planet_clearance
                                .map_or_else(|| "n/a".into(), |g| format!("{g:.3} mm"))
                        );
                        println!(
                            "sigma_H  sun-planet {:.1} MPa   planet-ring {:.1} MPa",
                            r.sun_planet.contact_stress, r.planet_ring.contact_stress
                        );
                        println!(
                            "sigma_F  sun {}   planet {} (reversed, allowable {:.1} MPa)   ring {}",
                            r.sun
                                .bending_stress
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.planet
                                .gear
                                .bending_stress
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.planet.reversed_allowable.value,
                            r.ring
                                .bending_stress
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                        );
                        println!(
                            "\n{:>9} {:>9} {:>9} {:>12} {:>11}",
                            "input", "held", "output", "ratio", "efficiency"
                        );
                        shown = true;
                    }
                    println!(
                        "{:>9} {:>9} {:>9} {:>12.4} {:>10.3} %",
                        name(input),
                        name(fixed),
                        name(r.output),
                        r.ratio,
                        r.efficiency.forward * 100.0
                    );
                }
            }
        }
    }
    if !shown {
        return;
    }
    let stage = base;
    if let Ok(r) = solve_planetary_stage(&stage, 3000.0, 2.0, &lib) {
        println!();
        for note in &r.notes {
            println!("note: {note}");
        }
    }
}

/// A crossed gear pair, swept over how the shaft angle divides between the two
/// members.
///
/// The split is the design freedom a worm does not have: a worm's diameter is
/// chosen and its lead angle follows, while two gears have their diameters fixed
/// by tooth count and helix, so `β₁` is what there is to choose. Nothing else
/// about the pair changes — it is the same screw geometry either way.
fn crossed_report(z1: u32, z2: u32, shaft_angle: f64) {
    use gear_core::train::{solve_worm_stage, FirstMemberSizing, WormStage};

    let lib = gear_io::default_library();
    println!(
        "crossed gear pair  z {z1}/{z2}  shaft angle {shaft_angle} deg  module 1  alpha 20 deg  \
         mu 0.06"
    );
    println!(
        "\n{:>7} {:>7} {:>9} {:>9} {:>10} {:>10} {:>11} {:>10}",
        "beta1", "beta2", "d1 mm", "d2 mm", "a mm", "slide/v1", "eta fwd", "sigma_H"
    );
    let mut any = false;
    for i in 0..=10 {
        #[allow(clippy::cast_precision_loss)]
        let beta1 = shaft_angle * (i as f64 / 10.0);
        let stage = WormStage {
            shaft_angle,
            starts: z1,
            wheel_teeth: z2,
            sizing: FirstMemberSizing::HelixAngle(beta1),
            ..WormStage::default()
        };
        let Ok(g) = stage.geometry() else {
            println!("{beta1:>7.1} {:>7} — no such pair", shaft_angle - beta1);
            continue;
        };
        match solve_worm_stage(&stage, 2.0, &lib) {
            Err(e) => println!(
                "{beta1:>7.1} {:>7.1}  {e}",
                g.wheel_helix_angle.to_degrees()
            ),
            Ok(r) => {
                any = true;
                println!(
                    "{beta1:>7.1} {:>7.1} {:>9.4} {:>9.4} {:>10.4} {:>10.4} {:>10.3} % {:>9.1}",
                    g.wheel_helix_angle.to_degrees(),
                    g.worm_pitch_diameter,
                    g.wheel_pitch_diameter,
                    g.centre_distance,
                    g.sliding_ratio,
                    r.efficiency.forward * 100.0,
                    r.contact.max_pressure
                );
            }
        }
    }
    if any {
        println!(
            "\nA worm is the same geometry with the first member's diameter chosen instead \
             of its helix;\nthe split above is the freedom two gears have and a worm does not."
        );
    }
}
