//! SVG diagrams of the bending construction, for looking at.
//!
//! A form factor is a number that is easy to compute wrongly and hard to check
//! by staring at. Drawing the construction — the fillet, the 30° tangents, the
//! critical chord, the load line and the moment arm — makes a wrong one obvious
//! at a glance in a way that a passing assertion does not.
//!
//! The drawing comes from the same code the product uses, so what is on screen
//! is what the calculation saw.
//!
//! Elements carry classes rather than colours so the embedding page can theme
//! them.

use gear_core::strength::{root_section_with, CriticalSection, RootSection};
use gear_core::{Gear, GearParams};
use std::fmt::Write as _;

/// One tooth in tooth coordinates: `y` along the centreline, `x` across it.
fn tooth_outline(g: &Gear, n: usize) -> Vec<[f64; 2]> {
    let (r, th) = g.half_profile(n);
    let mut pts: Vec<[f64; 2]> = r
        .iter()
        .zip(&th)
        .rev()
        .map(|(r, t)| [-r * t.sin(), r * t.cos()])
        .collect();
    pts.extend(
        r.iter()
            .zip(&th)
            .skip(1)
            .map(|(r, t)| [r * t.sin(), r * t.cos()]),
    );
    pts
}

fn path_of(pts: &[[f64; 2]]) -> String {
    let mut d = String::new();
    for (i, p) in pts.iter().enumerate() {
        let _ = write!(
            d,
            "{}{:.4} {:.4}",
            if i == 0 { "M" } else { "L" },
            p[0],
            p[1]
        );
        d.push(' ');
    }
    d
}

/// Draw one tooth with the bending construction over it.
///
/// The view is placed in tooth coordinates and flipped in `y`, since SVG counts
/// downward and a gear tooth is read pointing up.
#[allow(clippy::too_many_lines)]
pub fn tooth_diagram(p: GearParams, width: f64) -> String {
    let g = Gear::new(p);
    let outline = tooth_outline(&g, 900);
    let Some(sec) = root_section_with(&g, g.u_tip, CriticalSection::TangentAngle) else {
        return format!(
            r#"<svg viewBox="0 0 100 60" class="diagram"><text x="50" y="30" class="label" \
               text-anchor="middle">z={} x={}: no root section (severed)</text></svg>"#,
            p.teeth, p.profile_shift
        );
    };

    // Frame the tooth with a little room for the annotations.
    let x_max = outline.iter().fold(0.0_f64, |a, q| a.max(q[0].abs())) * 1.25;
    let y_lo = g.rf - 0.35 * p.module;
    let y_hi = sec.load_line_crossing[1].max(g.ra) + 0.25 * p.module;
    let h = y_hi - y_lo;
    let w = 2.0 * x_max;
    let scale = width / w;

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg viewBox="{:.4} {:.4} {:.4} {:.4}" width="{:.1}" height="{:.1}" class="diagram">"#,
        -x_max,
        -y_hi,
        w,
        h,
        width,
        h * scale
    );
    // Flip y so the tooth points up.
    let _ = write!(s, r#"<g transform="scale(1,-1)">"#);
    let sw = |k: f64| format!("{:.5}", k / scale);

    // reference circles, as arcs across the view
    for (r, cls) in [
        (g.ra, "ref"),
        (g.r, "ref-pitch"),
        (g.rb, "ref"),
        (g.rf, "ref"),
    ] {
        if r > 0.0 {
            let _ = write!(
                s,
                r#"<circle cx="0" cy="0" r="{r:.4}" class="{cls}" stroke-width="{}"/>"#,
                sw(1.0)
            );
        }
    }

    // the tooth itself
    let _ = write!(
        s,
        r#"<path d="{}" class="profile" stroke-width="{}"/>"#,
        path_of(&outline),
        sw(1.6)
    );

    // tooth centreline
    let _ = write!(
        s,
        r#"<line x1="0" y1="{:.4}" x2="0" y2="{:.4}" class="axis" stroke-width="{}"/>"#,
        y_lo,
        y_hi,
        sw(0.8)
    );

    // The 30 degree tangents, both sides. The direction comes from the solver
    // rather than being rebuilt from the angle: the angle alone does not fix
    // which way the line leans, and rebuilding it drew the mirror image.
    let reach = 0.9 * p.module;
    for side in [-1.0_f64, 1.0] {
        let tx = side * sec.tangency[0].abs();
        let ty = sec.tangency[1];
        let (dx, dy) = (side * sec.tangent_direction[0], sec.tangent_direction[1]);
        let n = f64::hypot(dx, dy);
        let _ = write!(
            s,
            r#"<line x1="{:.4}" y1="{:.4}" x2="{:.4}" y2="{:.4}" class="tangent" stroke-width="{}"/>"#,
            tx - dx / n * reach,
            ty - dy / n * reach,
            tx + dx / n * reach,
            ty + dy / n * reach,
            sw(1.0)
        );
        let _ = write!(
            s,
            r#"<circle cx="{tx:.4}" cy="{ty:.4}" r="{:.4}" class="marker"/>"#,
            0.035 * p.module
        );
    }

    // the critical section chord, s_Fn
    let _ = write!(
        s,
        r#"<line x1="{:.4}" y1="{:.4}" x2="{:.4}" y2="{:.4}" class="chord" stroke-width="{}"/>"#,
        -sec.tangency[0].abs(),
        sec.tangency[1],
        sec.tangency[0].abs(),
        sec.tangency[1],
        sw(2.0)
    );

    // the load line, from the contact point through the centreline
    let _ = write!(
        s,
        r#"<line x1="{:.4}" y1="{:.4}" x2="{:.4}" y2="{:.4}" class="loadline" stroke-width="{}"/>"#,
        sec.load_point[0],
        sec.load_point[1],
        sec.load_line_crossing[0],
        sec.load_line_crossing[1],
        sw(1.2)
    );
    let _ = write!(
        s,
        r#"<circle cx="{:.4}" cy="{:.4}" r="{:.4}" class="marker-load"/>"#,
        sec.load_point[0],
        sec.load_point[1],
        0.045 * p.module
    );

    // The Lewis parabola: vertex at the end of the moment arm, tangent to the
    // tooth. Drawn alongside the 30 degree construction so the difference — and
    // where it matters — is visible rather than argued.
    if let Some(par) = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola) {
        if let Some(pp) = par.parabola_p {
            let vertex = par.load_line_crossing[1];
            let x_end = par.tangency[0].abs() * 1.45;
            let mut d = String::new();
            let steps = 90;
            for i in 0..=steps {
                #[allow(clippy::cast_precision_loss)]
                let t = -1.0 + 2.0 * (i as f64) / f64::from(steps);
                let xx = t * x_end;
                let yy = vertex - xx * xx / (4.0 * pp);
                let _ = write!(d, "{}{xx:.4} {yy:.4} ", if i == 0 { "M" } else { "L" });
            }
            let _ = write!(
                s,
                r#"<path d="{d}" class="parabola" stroke-width="{}"/>"#,
                sw(1.1)
            );
            for side in [-1.0_f64, 1.0] {
                let _ = write!(
                    s,
                    r#"<circle cx="{:.4}" cy="{:.4}" r="{:.4}" class="marker-par"/>"#,
                    side * par.tangency[0].abs(),
                    par.tangency[1],
                    0.035 * p.module
                );
            }
        }
    }

    // the moment arm h_Fe, along the centreline
    let _ = write!(
        s,
        r#"<line x1="0" y1="{:.4}" x2="0" y2="{:.4}" class="arm" stroke-width="{}"/>"#,
        sec.tangency[1],
        sec.load_line_crossing[1],
        sw(2.5)
    );

    let _ = write!(s, "</g></svg>");
    s
}

/// A caption of the numbers behind one diagram.
pub fn tooth_caption(p: GearParams) -> String {
    let g = Gear::new(p);
    let parabola = root_section_with(&g, g.u_tip, CriticalSection::LewisParabola);
    match root_section_with(&g, g.u_tip, CriticalSection::TangentAngle) {
        Some(RootSection {
            root_chord,
            moment_arm,
            form_factor,
            fillet_curvature,
            ..
        }) => format!(
            "z={} x={:+.2}{}<br>30°: s_Fn/m {:.3}, h_Fe/m {:.3}, ρ_F/m {:.3}, <b>Y_F {:.3}</b>\
             <br>parabola: <b>Y_F {}</b>{}",
            p.teeth,
            p.profile_shift,
            if g.undercut { ", undercut" } else { "" },
            root_chord / p.module,
            moment_arm / p.module,
            fillet_curvature / p.module,
            form_factor,
            parabola.map_or_else(|| "—".to_string(), |q| format!("{:.3}", q.form_factor)),
            parabola.map_or_else(String::new, |q| format!(
                " ({:+.1}%, touches the {})",
                100.0 * (q.form_factor - form_factor) / form_factor,
                if q.tangency_on_flank {
                    "flank"
                } else {
                    "fillet"
                }
            )),
        ),
        None => format!("z={} x={:+.2} — no root section", p.teeth, p.profile_shift),
    }
}

/// Form factor against tooth count, one line per profile shift.
pub fn form_factor_chart(width: f64, height: f64) -> String {
    let shifts = [-0.3_f64, 0.0, 0.3, 0.6];
    let teeth: Vec<u32> = (10..=120).step_by(2).collect();

    let mut series: Vec<(f64, Vec<[f64; 2]>)> = Vec::new();
    for x in shifts {
        let pts: Vec<[f64; 2]> = teeth
            .iter()
            .filter_map(|&z| {
                let g = Gear::new(GearParams {
                    teeth: z,
                    profile_shift: x,
                    ..Default::default()
                });
                root_section_with(&g, g.u_tip, CriticalSection::TangentAngle)
                    .map(|s| [f64::from(z), s.form_factor])
            })
            .collect();
        series.push((x, pts));
    }

    let (x0, x1) = (10.0, 120.0);
    let (y0, y1) = (1.6_f64, 4.2_f64);
    let (ml, mr, mt, mb) = (46.0, 96.0, 12.0, 34.0);
    let px = |v: f64| ml + (v - x0) / (x1 - x0) * (width - ml - mr);
    let py = |v: f64| mt + (y1 - v) / (y1 - y0) * (height - mt - mb);

    let mut s = String::new();
    let _ = write!(
        s,
        r#"<svg viewBox="0 0 {width:.0} {height:.0}" width="{width:.0}" height="{height:.0}" class="chart">"#
    );

    for g in [2.0_f64, 2.5, 3.0, 3.5, 4.0] {
        let _ = write!(
            s,
            r#"<line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" class="grid"/>
               <text x="{:.1}" y="{:.1}" class="tick" text-anchor="end">{g:.1}</text>"#,
            px(x0),
            py(g),
            px(x1),
            py(g),
            px(x0) - 6.0,
            py(g) + 4.0
        );
    }
    for z in [10.0_f64, 20.0, 40.0, 60.0, 80.0, 100.0, 120.0] {
        let _ = write!(
            s,
            r#"<text x="{:.1}" y="{:.1}" class="tick" text-anchor="middle">{z:.0}</text>"#,
            px(z),
            height - mb + 16.0
        );
    }
    let _ = write!(
        s,
        r#"<text x="{:.1}" y="{:.1}" class="axis-label" text-anchor="middle">tooth count z</text>"#,
        (px(x0) + px(x1)) / 2.0,
        height - 6.0
    );
    let _ = write!(
        s,
        r#"<text transform="translate(13,{:.1}) rotate(-90)" class="axis-label" text-anchor="middle">form factor Y_F</text>"#,
        (py(y0) + py(y1)) / 2.0
    );

    for (i, (x, pts)) in series.iter().enumerate() {
        let d: String = pts
            .iter()
            .enumerate()
            .map(|(j, p)| {
                format!(
                    "{}{:.2} {:.2} ",
                    if j == 0 { "M" } else { "L" },
                    px(p[0]),
                    py(p[1])
                )
            })
            .collect();
        let _ = write!(s, r#"<path d="{d}" class="series s{i}"/>"#);
        if let Some(last) = pts.last() {
            let _ = write!(
                s,
                r#"<text x="{:.1}" y="{:.1}" class="series-label s{i}">x = {x:+.1}</text>"#,
                px(last[0]) + 8.0,
                py(last[1]) + 4.0
            );
        }
    }

    let _ = write!(s, "</svg>");
    s
}
