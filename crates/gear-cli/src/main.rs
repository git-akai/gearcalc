//! Development harness for `gear-core`.
//!
//! Exists so the mathematics can be driven and inspected without a browser.
//!
//! ```text
//! gear-cli show   [z] [x]     print the derived geometry of one gear
//! gear-cli sweep              scan a parameter grid for clamps and undercut
//! gear-cli materials          the material library, with each value's basis
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
        Some("dxf") => dxf(
            args.get(1).and_then(|s| s.parse().ok()).unwrap_or(17),
            args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0.0),
            args.get(3).and_then(|s| s.parse().ok()).unwrap_or(1e-3),
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
            gear_core::material::Basis::Datasheet => 'd',
            gear_core::material::Basis::Derived => 'D',
            gear_core::material::Basis::Chart => 'c',
            gear_core::material::Basis::Estimated => 'e',
        };
        println!(
            "{:<20} {:>7.0}{} {:>8.0}{} {:>5.2}{} {:>9.1}{} {:>8.1}{}",
            m.name,
            m.density.get(),
            tag(&m.density),
            m.elastic_modulus.get(),
            tag(&m.elastic_modulus),
            m.poissons_ratio.get(),
            tag(&m.poissons_ratio),
            m.ultimate_allowable.get(),
            tag(&m.ultimate_allowable),
            m.fatigue_allowable.get(),
            tag(&m.fatigue_allowable),
        );
    }

    println!("\nbasis: d datasheet   D derived   c read off a chart   e estimated");
    println!("values are the conditioned state where the material has one\n");

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
        let Some(path) = ContactPath::new(&g1, &g2, &m) else {
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
