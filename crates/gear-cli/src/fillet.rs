//! **One member's root under both bending sets, and its exact outline**, for
//! `tools/fillet_bem.py`, which solves the loaded tooth by a boundary-element
//! method sharing no code with `gear_core::strength` and holds the peak fillet
//! stress it finds against what the crate rates.
//!
//! ```text
//! gear-cli fillet grid                                  every tooth the BEM record holds, rated
//! gear-cli fillet <external|ring> z α x ρ mate [outline]
//! ```
//!
//! A tooth is rated as a train rates it, unshared, at the highest point of
//! single-pair contact against a mate of `mate` teeth on the same rack at
//! zero backlash: the default set (Dolan–Broghamer on the Lewis section, by
//! the section rule) and ISO 6336-3's (`Y_F·Y_S` on the tangent section). `ρ`
//! is the rack's tip round for an external tooth and the shaper's for a
//! ring, in modules; the mate of a ring is an external pinion. Spur, module 1.
//!
//! `outline` adds the half tooth from the tip's centre to mid-space, `+x`
//! side, each curve sampled on its own parameter with its unit tangent, in
//! the gear's frame (`y` along the tooth's centreline, origin on the axis) —
//! for a ring that is the frame before `ToothOutline`'s flip, its tip at the
//! smaller radius. The BEM interpolates between samples; it reads nothing
//! else of the crate's geometry.

use gear_core::contact::{ContactPath, LoadSharing};
use gear_core::mesh::{Mesh, MeshKind};
use gear_core::ring::{self, Cutter, Ring};
use gear_core::strength::{
    bending_section_by, bending_section_on_path, CriticalSection, RootSection, RootStressModel,
    ToothOutline, Unrated,
};
use gear_core::{GearParams, Tooth};

/// Which kind of member, and so which tool cut it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// Cut by a rack whose tip round is the case's `ρ`.
    External,
    /// Cut by the default shaper with its tip round set to the case's `ρ`.
    Ring,
}

impl Kind {
    fn name(self) -> &'static str {
        match self {
            Self::External => "external",
            Self::Ring => "ring",
        }
    }

    fn parse(s: &str) -> Option<Self> {
        match s {
            "external" => Some(Self::External),
            "ring" => Some(Self::Ring),
            _ => None,
        }
    }
}

/// One tooth: kind, teeth, normal pressure angle (degrees), shift, tool tip
/// round (modules) and the mate's teeth.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub kind: Kind,
    pub teeth: u32,
    pub alpha_deg: f64,
    pub shift: f64,
    pub round: f64,
    pub mate: u32,
}

/// The external grid: the research round's, so the two records compare
/// tooth for tooth — six counts from a small pinion to a near-rack, the three
/// offered angles, unshifted and shifted, and the rack round from the
/// default down to a near-sharp root. Mate 43.
const EXTERNAL_TEETH: [u32; 6] = [12, 17, 30, 60, 150, 1000];
const EXTERNAL_ALPHA: [f64; 3] = [14.5, 20.0, 25.0];
const EXTERNAL_SHIFT: [f64; 2] = [0.0, 0.5];
const EXTERNAL_ROUND: [f64; 5] = [0.38, 0.25, 0.1, 0.03, 0.01];
const EXTERNAL_MATE: u32 = 43;

/// The ring grid: counts a 17-tooth pinion meshes inside at both angles a
/// shaper cuts rings at here (the default cutter's tip does not reach at
/// 14.5°, `matrix.rs`), unshifted and shifted, and the shaper's round from its
/// default down.
const RING_TEETH: [u32; 4] = [40, 60, 100, 200];
const RING_ALPHA: [f64; 2] = [20.0, 25.0];
const RING_SHIFT: [f64; 2] = [0.0, 0.5];
const RING_ROUND: [f64; 3] = [0.2, 0.1, 0.03];
const RING_MATE: u32 = 17;

/// Every tooth the BEM record holds, in its order.
pub fn grid() -> Vec<Case> {
    let mut v = Vec::new();
    for teeth in EXTERNAL_TEETH {
        for alpha_deg in EXTERNAL_ALPHA {
            for shift in EXTERNAL_SHIFT {
                for round in EXTERNAL_ROUND {
                    v.push(Case {
                        kind: Kind::External,
                        teeth,
                        alpha_deg,
                        shift,
                        round,
                        mate: EXTERNAL_MATE,
                    });
                }
            }
        }
    }
    for teeth in RING_TEETH {
        for alpha_deg in RING_ALPHA {
            for shift in RING_SHIFT {
                for round in RING_ROUND {
                    v.push(Case {
                        kind: Kind::Ring,
                        teeth,
                        alpha_deg,
                        shift,
                        round,
                        mate: RING_MATE,
                    });
                }
            }
        }
    }
    v
}

/// `fillet grid`, or one case with or without its outline.
pub fn run(args: &[String]) {
    if args.get(1).map(String::as_str) == Some("grid") {
        for c in grid() {
            report(c, false);
        }
        return;
    }
    match parse(args) {
        Some(c) => report(asked(c), args.get(7).map(String::as_str) == Some("outline")),
        None => {
            println!("usage: fillet grid | fillet <external|ring> z alpha x rho mate [outline]")
        }
    }
}

/// The case the arguments name, or `None` where one is missing or does not
/// read.
fn parse(args: &[String]) -> Option<Case> {
    Some(Case {
        kind: Kind::parse(args.get(1)?)?,
        teeth: args.get(2)?.parse().ok()?,
        alpha_deg: args.get(3)?.parse().ok()?,
        shift: args.get(4)?.parse().ok()?,
        round: args.get(5)?.parse().ok()?,
        mate: args.get(6)?.parse().ok()?,
    })
}

/// **A case as the harness is asked it**, read against the table every
/// entry reads a gear through (`gear_core::input`): the member, its mate
/// (under `mate`) and, for a ring, the shaper whose round it is (under
/// `cutter`) — refused naming the field.
fn asked(c: Case) -> Case {
    let member = params(c.teeth, c.alpha_deg, c.shift, c.round).check();
    let mate = params(c.mate, c.alpha_deg, 0.0, c.round)
        .check()
        .map_err(|e| e.within("mate"));
    let cutter = match c.kind {
        Kind::External => Ok(()),
        Kind::Ring => ring_cutter(c.round).check().map_err(|e| e.within("cutter")),
    };
    if let Err(e) = member.and(mate).and(cutter) {
        crate::refuse_for(&e);
    }
    c
}

/// The shaper a ring case is cut by: the default, its tip round the case's.
fn ring_cutter(round: f64) -> Cutter {
    Cutter {
        tip_round: round,
        ..Cutter::default()
    }
}

fn params(teeth: u32, alpha_deg: f64, shift: f64, round: f64) -> GearParams {
    GearParams {
        teeth,
        pressure_angle: alpha_deg,
        profile_shift: shift,
        root_radius: round,
        ..Default::default()
    }
}

/// The case's heading, then its figures, one line each.
fn report(c: Case, outline: bool) {
    print!(
        "case {} z {} alpha {} x {} rho {} mate {}",
        c.kind.name(),
        c.teeth,
        c.alpha_deg,
        c.shift,
        c.round,
        c.mate
    );
    let mate = Tooth::new(params(c.mate, c.alpha_deg, 0.0, c.round));
    match c.kind {
        Kind::External => {
            let g = Tooth::new(params(c.teeth, c.alpha_deg, c.shift, c.round));
            let eps = Mesh::new(&g, &mate, MeshKind::External)
                .ok()
                .and_then(|m| ContactPath::new(&g, mate.flank_ends(), &m))
                .map(|p| p.contact_ratio);
            member(
                &g,
                eps,
                [g.ra, g.rf, g.half_pitch],
                Some(g.undercut),
                1.0,
                outline,
            );
        }
        Kind::Ring => {
            let r = Ring::cut_by(
                &params(c.teeth, c.alpha_deg, c.shift, c.round),
                &ring_cutter(c.round),
            );
            let eps = ring::mesh_with(&r, &mate).map(|m| m.contact_ratio);
            member(&r, eps, [r.ra, r.rf, r.half_pitch], None, -1.0, outline);
        }
    }
}

/// The rest of a case: its contact ratio, both sets, the load, and the
/// outline if asked. `undercut` is a rack-cut tooth's, and a ring has none.
/// `flip` takes `ToothOutline`'s frame back to the gear's: `−1` on a ring,
/// whose `y` the trait negates.
fn member<T: ToothOutline>(
    g: &T,
    eps: Option<f64>,
    radii: [f64; 3],
    undercut: Option<bool>,
    flip: f64,
    outline: bool,
) {
    let Some(eps) = eps else {
        println!(" | no_mesh");
        return;
    };
    let [ra, rf, half_pitch] = radii;
    println!(
        " | eps {eps:.12e} r_a {ra:.12e} r_f {rf:.12e} half_pitch {half_pitch:.12e} usable {} undercut {}",
        g.is_usable(),
        undercut.map_or_else(|| "-".to_string(), |u| u.to_string())
    );
    let db = bending_section_on_path(g, eps, 0.0, LoadSharing::None).map(|(s, _)| s);
    let iso = bending_section_by(g, eps, CriticalSection::TangentAngle);
    match &db {
        Ok(s) => set_line("db", s, RootStressModel::DolanBroghamer, flip),
        Err(Unrated::NoSection) => println!("db none no_section"),
        Err(Unrated::Compressed) => println!("db none compressed"),
    }
    match &iso {
        Some(s) => set_line("iso", s, RootStressModel::Iso6336, flip),
        None => println!("iso none no_section"),
    }
    // The load both sets are read at: the same point, the highest of
    // single-pair contact, or the tip below a contact ratio of one.
    if let Some(s) = db.as_ref().ok().or(iso.as_ref()) {
        let [px, py] = s.load_point;
        let [cx, cy] = s.load_line_crossing;
        let (dx, dy) = (cx - px, cy - py);
        let len = dx.hypot(dy);
        println!(
            "load x {px:.16e} y {:.16e} dx {:.16e} dy {:.16e}",
            flip * py,
            dx / len,
            flip * dy / len
        );
    }
    if outline && g.is_usable() {
        print_outline(g, ra, rf, half_pitch, flip);
    }
}

/// One set's section, every figure the comparison reads.
fn set_line(name: &str, s: &RootSection, model: RootStressModel, flip: f64) {
    let said = |x: Option<f64>| x.map_or_else(|| "none".to_string(), |v| format!("{v:.12e}"));
    println!(
        "{name} on {} s_Fn {:.12e} h_Fe {:.12e} alpha_Fen {:.12e} Y_F {:.12e} axial {:.12e} \
         correction {} factor {} rho_f {:.12e} rho_F {:.12e} q_s {:.12e} tangency_x {:.12e} tangency_y {:.12e}",
        if s.tangency_on_flank { "flank" } else { "fillet" },
        s.root_chord,
        s.moment_arm,
        s.load_angle,
        s.form_factor,
        s.axial_compression,
        said(s.stress_correction(model)),
        said(s.bending_factor(model)),
        s.min_fillet_curvature,
        s.fillet_curvature,
        s.notch_parameter,
        s.tangency[0],
        flip * s.tangency[1],
    );
}

/// Samples along each curve, evenly in its own parameter. On a fillet a
/// millimetre long they fall under a micron apart, where a cubic through two
/// of them with their tangents departs from the curve by about `h⁴/(384 ρ³)`:
/// under 1e-9 mm at the tightest round the grid cuts, 0.01 modules.
const SAMPLES: usize = 2001;

/// The half tooth, tip centre to mid-space: `tip`, `flank`, `fillet`, `root`,
/// each as `piece <name> <n>` and `n` lines `x y tx ty`.
fn print_outline<T: ToothOutline>(g: &T, ra: f64, rf: f64, half_pitch: f64, flip: f64) {
    // A usable outline has a fillet; one without has no outline to print.
    let Some(ends) = g.fillet() else {
        return;
    };
    let Some((lo, hi)) = g.flank_bracket() else {
        return;
    };
    let (u_tip, u_junction) = if g.tip_at_high_roll() {
        (hi, lo)
    } else {
        (lo, hi)
    };
    let unflip = |(p, t): ([f64; 2], [f64; 2])| {
        let len = t[0].hypot(t[1]);
        ([p[0], flip * p[1]], [t[0] / len, flip * t[1] / len])
    };
    let angle = |p: [f64; 2]| p[0].atan2(p[1]);
    let arc = |r: f64, a0: f64, a1: f64| {
        (0..SAMPLES)
            .map(|i| {
                let a = a0 + (a1 - a0) * i as f64 / (SAMPLES - 1) as f64;
                ([r * a.sin(), r * a.cos()], [a.cos(), -a.sin()])
            })
            .collect::<Vec<_>>()
    };
    let along = |a: f64, b: f64, f: &dyn Fn(f64) -> ([f64; 2], [f64; 2])| {
        (0..SAMPLES)
            .map(|i| unflip(f(a + (b - a) * i as f64 / (SAMPLES - 1) as f64)))
            .collect::<Vec<_>>()
    };
    let flank = along(u_tip, u_junction, &|u| g.flank_at(u));
    let fillet = along(ends.junction, ends.root, &|s| g.fillet_at(s));
    let tip_corner = angle(flank[0].0);
    let root_start = angle(fillet[SAMPLES - 1].0);
    let mut pieces = Vec::new();
    if tip_corner > 0.0 {
        pieces.push(("tip", arc(ra, 0.0, tip_corner)));
    }
    pieces.push(("flank", flank));
    pieces.push(("fillet", fillet));
    if root_start < half_pitch {
        pieces.push(("root", arc(rf, root_start, half_pitch)));
    }
    for (name, pts) in pieces {
        println!("piece {name} {}", pts.len());
        for ([x, y], [tx, ty]) in pts {
            println!("{x:.16e} {y:.16e} {tx:.16e} {ty:.16e}");
        }
    }
}
