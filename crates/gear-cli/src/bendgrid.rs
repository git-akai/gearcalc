//! **The default bending rating over a grid, every figure at full
//! precision**, for `tools/bending_gate.py`, which rebuilds each one from the
//! tooth's stated proportions and shares no code with `gear_core::strength`.
//!
//! Each row is one member rated as a train rates it: the section and share
//! `bending_section_on_path` returns and the stress `bending_stress` makes of
//! them, Dolan–Broghamer's model, no rim. The contact ratio is an input of the
//! bending model, so the grid states it (in the normal plane, `ε_αn`) rather
//! than taking it from a mate; `ε_αn ≥ 2` rows under the ramp are where the
//! held section and one searched afresh part.
//!
//! The grid is written here and again in the gate, which refuses a record
//! that is not exactly this product.

use gear_core::contact::LoadSharing;
use gear_core::strength::{bending_section_on_path, bending_stress, Load, RootStressModel};
use gear_core::{GearParams, Tooth};

/// One block of the grid: every combination of its lists.
struct Block {
    teeth: &'static [u32],
    alpha_deg: &'static [f64],
    shift: &'static [f64],
    helix_deg: &'static [f64],
    /// Addendum, dedendum, rack tip radius, thickness factor.
    proportions: &'static [(f64, f64, f64, f64)],
    eps_n: &'static [f64],
    short_of_tip: &'static [f64],
}

const BLOCKS: &[Block] = &[
    // Finite tooth counts, the three offered pressure angles, shifts either
    // side of zero, spur and helical, at the default proportions.
    Block {
        teeth: &[9, 12, 17, 25, 40, 70, 150, 400],
        alpha_deg: &[14.5, 20.0, 25.0],
        shift: &[-0.3, 0.0, 0.5],
        helix_deg: &[0.0, 15.0, 30.0],
        proportions: &[(1.0, 1.25, 0.38, 1.0)],
        eps_n: &[1.3, 1.7, 2.3],
        short_of_tip: &[0.0],
    },
    // Other proportions, a thinned tooth, and contact ending short of the tip.
    Block {
        teeth: &[12, 25],
        alpha_deg: &[20.0],
        shift: &[0.0, 0.3],
        helix_deg: &[0.0, 20.0],
        proportions: &[(1.25, 1.5, 0.25, 1.0), (1.0, 1.25, 0.2, 0.96)],
        eps_n: &[1.5, 2.2],
        short_of_tip: &[0.0, 0.2],
    },
];

const SHARING: [(LoadSharing, &str); 2] = [
    (LoadSharing::None, "none"),
    (LoadSharing::LinearRamp, "ramp"),
];

/// The torque on the member, N·m, and the face, mm, every stress is taken at.
const TORQUE: f64 = 10.0;
const FACE: f64 = 10.0;

pub fn run() {
    println!(
        "torque {TORQUE} face {FACE} module {}",
        GearParams::default().module
    );
    let mut row = 0;
    for b in BLOCKS {
        for &teeth in b.teeth {
            for &alpha in b.alpha_deg {
                for &shift in b.shift {
                    for &helix in b.helix_deg {
                        for &(addendum, dedendum, root_radius, thickness_mod) in b.proportions {
                            let g = Tooth::new(GearParams {
                                teeth,
                                pressure_angle: alpha,
                                profile_shift: shift,
                                helix_angle: helix,
                                addendum,
                                dedendum,
                                root_radius,
                                thickness_mod,
                                ..Default::default()
                            });
                            let cos_bb = g.base_helix_angle().cos();
                            for &eps_n in b.eps_n {
                                let eps = eps_n * cos_bb * cos_bb;
                                for &short in b.short_of_tip {
                                    for (sharing, name) in SHARING {
                                        print!(
                                            "row {row} z {teeth} alpha {alpha} beta {helix} x {shift} \
                                             h_a {addendum} h_f {dedendum} rho {root_radius} k {thickness_mod} \
                                             eps_a {eps:e} short {short} sharing {name}"
                                        );
                                        rated(&g, eps, short, sharing);
                                        row += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    println!("rows {row}");
}

/// The rest of one row: the virtual member's tool and form, then its rating
/// or `none`. The tool is printed to the bit, since the gate takes a round
/// the tooth capped as the one it was cut by; everything else to 13 digits.
fn rated(g: &Tooth, eps: f64, short: f64, sharing: LoadSharing) {
    let v = g.virtual_spur();
    let clamps: Vec<&str> = v.clamps.notes.iter().map(|n| n.key.as_str()).collect();
    print!(
        " | z_n {:e} rho_mm {:e} b_d {:e} u_j {:.12e} u_tip {:.12e} clamps {}",
        v.z,
        v.rho,
        v.bd,
        v.u_j,
        v.u_tip,
        if clamps.is_empty() {
            "-".to_string()
        } else {
            clamps.join(",")
        }
    );
    let Some((s, share)) = bending_section_on_path(g, eps, short, sharing) else {
        println!(" | none");
        return;
    };
    let force = Load::new(TORQUE, FACE).tangential(g);
    let stress =
        bending_stress(&s, force, FACE, RootStressModel::DolanBroghamer, None).map(|f| f * share);
    println!(
        " | on {} s_Fn {:.12e} h_Fe {:.12e} alpha_Fen {:.12e} load_x {:.12e} load_y {:.12e} \
         y_t {:.12e} rho_f {:.12e} Y_F {:.12e} axial {:.12e} K_f {} factor {} share {:.12e} sigma_F {}",
        if s.tangency_on_flank { "flank" } else { "fillet" },
        s.root_chord,
        s.moment_arm,
        s.load_angle,
        s.load_point[0],
        s.load_point[1],
        s.tangency[1],
        s.min_fillet_curvature,
        s.form_factor,
        s.axial_compression,
        said(s.stress_correction(RootStressModel::DolanBroghamer)),
        said(s.bending_factor(RootStressModel::DolanBroghamer)),
        share,
        said(stress),
    );
}

/// A figure the model may have no reading of, printed as `none` where it has
/// none.
fn said(x: Option<f64>) -> String {
    x.map_or_else(|| "none".to_string(), |v| format!("{v:.12e}"))
}
