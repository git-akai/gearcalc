//! **Everything the core answers, at full precision** — the record a
//! behaviour-neutral refactor is held to by `tools/check_identity.sh`.
//!
//! Every number is printed with `{:?}`, which for an `f64` is the shortest
//! decimal that reads back to the same bits: two outputs are equal as text
//! exactly when every float in them is bit-identical. A refusal prints as its
//! error value, so a changed refusal is a difference like a moved number.
//!
//! What it covers:
//! - every train `gear-cli graph` solves (each preset alone, each ordered pair
//!   of presets, and a chain of three) and every fixture `gear-cli
//!   kinematics` solves (the six planetary arrangements and every arrangement
//!   `train/arrangements.rs` builds), each with all its load cases, as the
//!   whole `TrainResult`;
//! - each preset and each of those arrangements asked alone (`solve_alone`);
//! - a grid of gears, external and internal, over tooth count, shift, helix
//!   and pressure angle, each with the mesh it makes with a fixed mate.
//!
//! Not in the golden corpus: its output is megabytes of bits that only a
//! comparison between two builds reads.

use gear_core::mesh::{Mesh, MeshKind};
use gear_core::ring::{self, Cutter, Ring};
use gear_core::train::arrangements::Preset;
use gear_core::train::{solve_alone, solve_train, Train};
use gear_core::{Gear, GearParams};

/// A panic is recorded as its message and the run goes on, so one broken
/// case does not hide the rest of the comparison.
fn guarded(what: &str, f: impl FnOnce() + std::panic::UnwindSafe) {
    if let Err(e) = std::panic::catch_unwind(f) {
        let msg = e
            .downcast_ref::<&str>()
            .map(ToString::to_string)
            .or_else(|| e.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        println!("PANIC in {what}: {msg}");
    }
}

fn trains(lib: &gear_core::MaterialLibrary) {
    let fixtures = crate::graph::fixtures()
        .into_iter()
        .map(|(n, t)| (format!("graph {n}"), t))
        .chain(
            crate::kinematics::fixtures()
                .into_iter()
                .map(|(n, t)| (format!("kinematics {n}"), t)),
        );
    for (name, train) in fixtures {
        println!("== train {name}");
        guarded(&name, || println!("{:#?}", solve_train(&train, lib)));
    }
}

fn alone(lib: &gear_core::MaterialLibrary) {
    let shapes = Preset::ALL
        .iter()
        .map(|p| (format!("{p:?}"), p.build()))
        .chain(
            crate::kinematics::fixtures()
                .into_iter()
                .filter_map(|(n, t)| {
                    // A fixture of one part is an arrangement the preset menu may not
                    // hold; its shape asked alone is what a preset laid in answers.
                    (t.parts().len() == 1).then(|| (n, t.shape.clone()))
                }),
        );
    for (name, shape) in shapes {
        println!("== alone {name}");
        guarded(&name, || {
            println!(
                "{:#?}",
                solve_alone(&Train::alone(&shape, 2.0, 3000.0), lib)
            );
        });
    }
}

/// The gear grid: every axis that selects a different branch of the
/// generation — undercut and not, spur and helical, three rack angles — on
/// both sides of the workpiece.
fn gears() {
    const TEETH: [u32; 3] = [9, 17, 40];
    const SHIFT: [f64; 3] = [-0.3, 0.0, 0.5];
    const HELIX: [f64; 2] = [0.0, 20.0];
    const PRESSURE: [f64; 3] = [14.5, 20.0, 25.0];
    // A mate for the external meshes, and a ring's worth of teeth more for
    // the internal ones.
    const MATE: u32 = 43;
    const RING_MORE: u32 = 30;
    const PER_TOOTH: usize = 8;
    for z in TEETH {
        for x in SHIFT {
            for beta in HELIX {
                for alpha in PRESSURE {
                    let p = GearParams {
                        teeth: z,
                        profile_shift: x,
                        helix_angle: beta,
                        pressure_angle: alpha,
                        ..GearParams::default()
                    };
                    let name = format!("z {z} x {x:?} beta {beta:?} alpha {alpha:?}");
                    println!("== gear {name}");
                    guarded(&name, || {
                        let g = Gear::new(p);
                        println!("{g:#?}");
                        println!("profile {:?}", g.profile(PER_TOOTH));
                        let mate = Gear::new(GearParams {
                            teeth: MATE,
                            helix_angle: -beta,
                            ..p
                        });
                        let mesh = Mesh::new(g.mean(), mate.mean(), MeshKind::External);
                        println!("external mesh {mesh:#?}");
                        if let Ok(m) = mesh {
                            let a = m.a_w + 0.1;
                            println!("at a_w + 0.1: {:#?}", m.at(a));
                            println!("backlash {:?}", m.backlash(a));
                        }
                    });
                    println!("== ring {name}");
                    guarded(&name, || {
                        let rp = GearParams {
                            teeth: z + RING_MORE,
                            ..p
                        };
                        let r = Ring::cut_by(&rp, &Cutter::default());
                        println!("{r:#?}");
                        println!("profile {:?}", r.profile(PER_TOOTH));
                        let pinion = Gear::new(p);
                        println!("ring mesh {:#?}", ring::mesh_with(&r, pinion.mean()));
                    });
                }
            }
        }
    }
}

pub fn run() {
    let lib = gear_io::default_library();
    trains(&lib);
    alone(&lib);
    gears();
}
