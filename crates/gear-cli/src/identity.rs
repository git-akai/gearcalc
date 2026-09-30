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
//!   `train/arrangements.rs` builds): the whole `TrainResult` with all its
//!   load cases, the motion report, the groupings and each case's flow, and
//!   each case relieved to the train's mobility;
//! - each preset and one-part fixture asked alone (`solve_alone`), and two
//!   external pairs whose tips are held off their mates' flanks;
//! - **the graph's edits**: every offer at every piece of every preset and of
//!   a few multi-part trains (`EDITED_CHAINS`), where the join, the insert and
//!   the hold act across parts, and there every split of a body two parts
//!   share; each made, previewed and solved (whole for a sample fixed by each
//!   edit's own digest, as a digest for the rest), and each labelled by the
//!   edit, so an offer added or removed moves only its own lines;
//! - a grid of gears over module, tooth count, shift, helix, pressure angle,
//!   addendum and root radius: each gear and its outline, its mesh with a
//!   fixed mate, its span and over-pins measurements, its admissible ranges,
//!   and the same as a ring; DXF text on a sub-grid;
//! - crossed-axis screw pairs: the pair, its path of contact, efficiency and
//!   locking;
//! - **the structure the solve reads**, before any number: how each train
//!   falls into parts, its mesh groups, and per part each mesh's frame, who
//!   decides each shift and how a search groups what it moves
//!   (`Shape::structure`) — over every train above and every train the
//!   seeded walk of offered edits visits (`train::sweep`), so a change to how
//!   the structure is read shows even where no number it feeds moves.
//!
//! Not in the golden corpus: its output is megabytes of bits that only a
//! comparison between two builds reads.

use gear_core::contact::Drive;
use gear_core::mesh::{Mesh, MeshKind};
use gear_core::metrology::{self, PinCount};
use gear_core::ring::{self, Cutter, Ring};
use gear_core::screw::{Screw, ScrewParams};
use gear_core::train::arrangements::Preset;
use gear_core::train::{preview, solve_alone, solve_train, Target, Train};
use gear_core::{auto, Gear, GearParams, MaterialLibrary};

/// The drive a preset asked alone is loaded with: the torque (N·m) and speed
/// (rpm) every `gear-cli graph` and `kinematics` fixture's forward case uses,
/// so a preset alone is asked what it is asked there.
const TORQUE_NM: f64 = 2.0;
const SPEED_RPM: f64 = 3000.0;

/// Edited trains per train whose solve is printed whole rather than as a
/// digest: a sample, since each is some thousands of lines.
const SOLVED_WHOLE_PER_PRESET: usize = 8;

/// The multi-part trains whose edits are recorded as well as each preset's
/// alone, by their `gear-cli graph` names: a fixed-axis part either side of
/// an epicyclic one, a crossed part ahead of a compound set, a layshaft ahead
/// of a Wolfrom, and the chain of three.
const EDITED_CHAINS: [&str; 5] = [
    "spur then planetary",
    "planetary then spur",
    "worm then compound",
    "layshaft then wolfrom",
    "spur then layshaft then compound",
];

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

/// Every train `gear-cli graph` and `gear-cli kinematics` solve, named.
fn fixtures() -> impl Iterator<Item = (String, Train)> {
    crate::graph::fixtures()
        .into_iter()
        .map(|(n, t)| (format!("graph {n}"), t))
        .chain(
            crate::kinematics::fixtures()
                .into_iter()
                .map(|(n, t)| (format!("kinematics {n}"), t)),
        )
}

fn trains(lib: &MaterialLibrary) {
    for (name, train) in fixtures() {
        println!("== train {name}");
        guarded(&name, || {
            let solved = solve_train(&train, lib);
            println!("{solved:#?}");
            println!("motion {:#?}", train.motion_report());
            println!("groupings {:#?}", train.groupings());
            if let Ok(r) = &solved {
                println!("flows {:#?}", train.flows(r));
            }
            for case in 0..train.load_cases.len() {
                let mut relieved = train.clone();
                let done = relieved.relieve_case(case, None, lib);
                println!(
                    "relieved case {case}: {done:?} {:#?}",
                    relieved.load_cases[case]
                );
            }
        });
    }
}

fn alone(lib: &MaterialLibrary) {
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
        )
        .chain(held_externals());
    for (name, shape) in shapes {
        println!("== alone {name}");
        guarded(&name, || {
            println!(
                "{:#?}",
                solve_alone(&Train::alone(&shape, TORQUE_NM, SPEED_RPM), lib)
            );
        });
    }
}

/// **External pairs whose tips are held off their mates' flanks**
/// (`MemberGear::no_tip_past_mate_flank`), which no preset reaches at its
/// defaults: a small-pinion pair searched, where the search tops the tips
/// to buy efficiency, and a pair typed at a long addendum.
fn held_externals() -> Vec<(String, gear_core::train::Shape)> {
    use gear_core::train::arrangements::pair;
    let mut searched = pair([9, 37]);
    searched.set_search(true);
    let mut long = pair([12, 29]);
    for m in &mut long.members {
        m.gear.addendum = 1.6;
        m.gear.no_sharp_tip = false;
    }
    vec![
        ("pair 9/37 searched".to_string(), searched),
        ("pair 12/29 at 1.6 m of addendum".to_string(), long),
    ]
}

/// Every piece of a train an offer can be asked at, ground among the bodies.
fn targets(train: &Train) -> Vec<Target> {
    let s = &train.shape;
    let mut bodies: Vec<usize> = s.bodies.iter().map(|b| b.body).collect();
    bodies.push(gear_core::kinematics::GROUND);
    bodies.sort_unstable();
    bodies.dedup();
    std::iter::once(Target::Train)
        .chain((0..s.members.len()).map(Target::Member))
        .chain((0..s.meshes.len()).map(Target::Mesh))
        .chain(bodies.into_iter().map(Target::Body))
        .chain((0..s.axes.len()).map(Target::Axis))
        .chain((0..s.distances.len()).map(Target::Distance))
        .chain((0..s.couplings.len()).map(Target::Coupling))
        .collect()
}

/// FNV-1a, 64-bit: a digest fixed by its definition, not by the standard
/// library's hasher, so two builds on two toolchains agree on it.
fn fnv1a(text: &str) -> u64 {
    const OFFSET: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;
    text.bytes()
        .fold(OFFSET, |h, b| (h ^ u64::from(b)).wrapping_mul(PRIME))
}

/// Each preset as `gear-cli graph` has it alone: every offer at every piece;
/// every offer made and previewed, and the edited train solved — printed
/// whole for a fixed sample, and as a digest of the whole for the rest, so
/// any bit that moves in any edited train is seen and the sample says where.
///
/// The presets run on threads of their own, each into its own text, printed
/// in order: the output is the same as run one after another.
fn edits(lib: &MaterialLibrary) {
    let fixtures: Vec<_> = crate::graph::fixtures()
        .into_iter()
        .enumerate()
        .filter(|(k, (name, _))| *k < Preset::ALL.len() || EDITED_CHAINS.contains(&name.as_str()))
        .map(|(_, f)| f)
        .collect();
    let texts: Vec<String> = std::thread::scope(|scope| {
        let running: Vec<_> = fixtures
            .iter()
            .map(|(name, train)| scope.spawn(move || edits_of(name, train, lib)))
            .collect();
        running
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| "PANIC in an edits thread\n".into())
            })
            .collect()
    });
    for t in texts {
        print!("{t}");
    }
}

/// One change [`edits_of`] makes to a train: an offered edit, or a part's
/// end of a shared body split off, which is the train's and no piece offers.
enum Change {
    Offered(gear_core::train::Edit),
    Split { part: usize, body: usize },
}

/// What a change is called in the record: the edit itself, an insert by the
/// preset it lays in rather than the whole shape it carries.
fn label(offer: &gear_core::train::Offer) -> String {
    match (&offer.edit, offer.preset) {
        (gear_core::train::Edit::Insert { at, .. }, Some(p)) => {
            format!("Insert {{ {p:?} at {at:?} }}")
        }
        (edit, _) => format!("{edit:?}"),
    }
}

/// [`edits`] for one train. Each change is labelled by what it does, listed
/// once however many pieces offer it, and printed whole where its label's
/// digest is among the smallest — a sample no other change's arrival moves.
fn edits_of(name: &str, train: &Train, lib: &MaterialLibrary) -> String {
    use std::fmt::Write;
    let mut out = String::new();
    let caught = |f: &mut dyn FnMut() -> String| {
        std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
            .unwrap_or_else(|_| "PANIC\n".to_string())
    };
    let _ = writeln!(out, "== offers {name}");
    let mut changes: Vec<(String, Change)> = Vec::new();
    for at in targets(train) {
        match std::panic::catch_unwind(|| train.offers(at)) {
            Ok(offers) => {
                let _ = writeln!(out, "at {at:?}: {offers:#?}");
                for o in offers.into_iter().filter(|o| o.refused.is_none()) {
                    let l = label(&o);
                    if !changes.iter().any(|(k, _)| *k == l) {
                        changes.push((l, Change::Offered(o.edit)));
                    }
                }
            }
            Err(_) => {
                let _ = writeln!(out, "PANIC in offers at {at:?}");
            }
        }
    }
    // Every end of a body two parts share, split off from each part.
    let parts = train.parts();
    for (p, part) in parts.iter().enumerate() {
        for b in part.shape.bodies.iter().map(|b| b.body) {
            let shared = parts
                .iter()
                .filter(|q| q.shape.bodies.iter().any(|x| x.body == b))
                .count();
            if b != gear_core::kinematics::GROUND && shared > 1 {
                changes.push((
                    format!("split part {p} body {b}"),
                    Change::Split { part: p, body: b },
                ));
            }
        }
    }
    let mut digests: Vec<u64> = changes.iter().map(|(l, _)| fnv1a(l)).collect();
    digests.sort_unstable();
    let cut = digests
        .get(SOLVED_WHOLE_PER_PRESET.min(digests.len()).saturating_sub(1))
        .copied()
        .unwrap_or(0);
    for (l, change) in &changes {
        let _ = writeln!(out, "== edit {name}: {l}");
        let whole = fnv1a(l) <= cut;
        out += &caught(&mut || {
            let mut text = String::new();
            let mut edited = train.clone();
            let made = match change {
                Change::Offered(edit) => edited.edit(edit.clone()),
                Change::Split { part, body } => {
                    edited.split(*part, *body);
                    Ok(())
                }
            };
            let _ = writeln!(text, "made {made:?}");
            let _ = writeln!(
                text,
                "preview {:#?}",
                preview(train, made.map(|()| &edited), lib)
            );
            if made.is_ok() {
                let solved = format!("{:#?}", solve_train(&edited, lib));
                if whole {
                    let _ = writeln!(text, "solved {solved}");
                } else {
                    let _ = writeln!(text, "solved fnv1a {:016x}", fnv1a(&solved));
                }
            }
            text
        });
    }
    out
}

/// A gear's measurements: span over its best tooth count, and over two and
/// three pins of the diameter midway in the range its spaces seat.
fn measured(g: &Gear) {
    let t = g.mean();
    println!("best span {:?}", metrology::best_span(t));
    println!("best span around {:?}", metrology::best_span_around(g));
    let pins = metrology::pin_diameter_range_around(g);
    println!("pin range {pins:?}");
    if let Some((lo, hi)) = pins {
        let d = 0.5 * (lo + hi);
        println!(
            "over two pins {:?}",
            metrology::over_pins(t, d, PinCount::Two)
        );
        println!(
            "over three pins {:?}",
            metrology::over_pins(t, d, PinCount::Three)
        );
    }
}

/// A mate for the external meshes, and a ring's worth of teeth more than the
/// pinion for the internal ones.
const MATE: u32 = 43;
const RING_MORE: u32 = 30;
/// Points per tooth of a sampled profile.
const PER_TOOTH: usize = 8;
/// The chord tolerance, mm, every grid outline is drawn to: fifty times the
/// default, which keeps a 70-tooth ring at module 2.5 to some hundreds of
/// vertices (a chord's count goes as the inverse root of the tolerance). The
/// sub-grid with DXF text is drawn at the default, through the writer.
const OUTLINE_TOLERANCE: f64 = 50.0 * gear_core::outline::DEFAULT_CHORD_TOLERANCE;

/// The gear grid: every axis that selects a different branch of the
/// generation — undercut and not, spur and helical, three rack angles, the
/// basic rack's proportions — on both sides of the workpiece, at two modules.
fn gears() {
    const MODULE: [f64; 2] = [1.0, 2.5];
    const TEETH: [u32; 3] = [9, 17, 40];
    const SHIFT: [f64; 3] = [-0.3, 0.0, 0.5];
    const HELIX: [f64; 2] = [0.0, 20.0];
    const PRESSURE: [f64; 3] = [14.5, 20.0, 25.0];
    let default = GearParams::default();
    // The default rack, and a stub tooth with a sharper root round.
    let racks = [(default.addendum, default.root_radius), (0.8, 0.2)];
    let dxf = gear_io::DxfOptions {
        reference_circles: true,
        ..gear_io::DxfOptions::default()
    };
    // What a preset's mesh is opened by, so the operating mesh is read at the
    // distance a train would run it.
    let opened_by = Preset::Spur.build().distances[0].clearance.manual;
    for module in MODULE {
        for (addendum, root_radius) in racks {
            for teeth in TEETH {
                for profile_shift in SHIFT {
                    for helix_angle in HELIX {
                        for pressure_angle in PRESSURE {
                            let p = GearParams {
                                module,
                                teeth,
                                profile_shift,
                                helix_angle,
                                pressure_angle,
                                addendum,
                                root_radius,
                                ..default
                            };
                            // DXF text on the spur gears of the default rack:
                            // the writer's formatting, once per kind of outline.
                            let dxf = (helix_angle == 0.0 && (addendum, root_radius) == racks[0])
                                .then_some(&dxf);
                            gear(&p, opened_by, dxf);
                            ring(&p, dxf);
                        }
                    }
                }
            }
        }
    }
}

fn gear(p: &GearParams, opened_by: f64, dxf: Option<&gear_io::DxfOptions>) {
    let name = format!("{p:?}");
    println!("== gear {name}");
    guarded(&name, || {
        let g = Gear::new(*p);
        println!("{g:#?}");
        println!(
            "profile {}",
            drawn(g.profile(PER_TOOTH, gear_core::input::Budget::DEFAULT))
        );
        println!(
            "outline {}",
            drawn(g.outline(OUTLINE_TOLERANCE, gear_core::input::Budget::DEFAULT))
        );
        println!("admissible {:#?}", auto::admissible_ranges(p, p.dedendum));
        measured(&g);
        if let Some(o) = dxf {
            println!("dxf\n{}", drawn_text(gear_io::gear_to_dxf(&g, o)));
        }
        let other = Gear::new(GearParams {
            teeth: MATE,
            helix_angle: -p.helix_angle,
            ..*p
        });
        let mesh = Mesh::new(g.mean(), other.mean(), MeshKind::External);
        println!("external mesh {mesh:#?}");
        if let Ok(m) = mesh {
            let a = m.a_w + opened_by;
            println!("opened: {:#?}", m.at(a));
            println!("backlash {:?}", m.backlash(a));
        }
    });
}

/// A drawing as the record prints it: its points, or the refusal that
/// stands in for them.
fn drawn<T: std::fmt::Debug>(r: Result<T, gear_core::input::Refused>) -> String {
    r.map_or_else(|e| format!("refused: {e}"), |v| format!("{v:?}"))
}

/// A drawing's text, or the refusal that stands in for it.
fn drawn_text(r: Result<String, gear_core::input::Refused>) -> String {
    r.unwrap_or_else(|e| format!("refused: {e}"))
}

fn ring(p: &GearParams, dxf: Option<&gear_io::DxfOptions>) {
    let name = format!("{p:?}");
    println!("== ring {name}");
    guarded(&name, || {
        let rp = GearParams {
            teeth: p.teeth + RING_MORE,
            ..*p
        };
        let r = Ring::cut_by(&rp, &Cutter::default());
        println!("{r:#?}");
        println!(
            "profile {}",
            drawn(r.profile(PER_TOOTH, gear_core::input::Budget::DEFAULT))
        );
        println!(
            "outline {}",
            drawn(r.outline(OUTLINE_TOLERANCE, gear_core::input::Budget::DEFAULT))
        );
        if let Some(o) = dxf {
            println!("dxf\n{}", drawn_text(gear_io::ring_to_dxf(&r, o)));
        }
        let pinion = Gear::new(*p);
        println!("ring mesh {:#?}", ring::mesh_with(&r, pinion.mean()));
    });
}

/// Crossed-axis pairs: a worm at one and two starts and a crossed helical
/// pair, at a right angle and at 45°. Friction is the worm preset's mesh's.
fn screws() {
    const PAIRS: [(u32, u32); 3] = [(1, 40), (2, 41), (17, 23)];
    const SHAFT_DEG: [f64; 2] = [90.0, 45.0];
    // Samples along the path for its integrated figures: enough that the
    // integrals are smooth, few enough to stay quick; identity only asks
    // that the same count gives the same bits.
    const SAMPLES: usize = 64;
    let friction = Preset::Worm.build().meshes[0].sliding_friction;
    for (starts, wheel_teeth) in PAIRS {
        for shaft in SHAFT_DEG {
            let params = ScrewParams {
                starts,
                wheel_teeth,
                shaft_angle_rad: shaft.to_radians(),
                ..ScrewParams::default()
            };
            let name = format!("{params:?}");
            println!("== screw {name}");
            guarded(&name, || {
                let s = Screw::new(&params);
                println!("{s:#?}");
                let Ok(s) = s else { return };
                for drive in [Drive::Forward, Drive::Backward] {
                    println!("efficiency {drive:?} {:?}", s.efficiency(friction, drive));
                }
                println!("locking {:?}", s.locking_friction());
                // Tips a standard addendum, one module, past each pitch circle.
                let tip = |d: f64| 0.5 * d + params.normal_module;
                let path =
                    s.path_of_contact(tip(s.worm_pitch_diameter), tip(s.wheel_pitch_diameter));
                println!("path {path:#?}");
                if let Some(path) = path {
                    for drive in [Drive::Forward, Drive::Backward] {
                        println!(
                            "path efficiency {drive:?} {:?}",
                            path.efficiency(&s, friction, drive, SAMPLES)
                        );
                    }
                    println!("path locking {:?}", path.locking_friction(&s, SAMPLES));
                    println!("single pair {:?}", path.single_pair_bounds(&s));
                    println!("axial travel {:?}", path.axial_travel(&s));
                }
            });
        }
    }
}

/// **The structure of every train**: the fixtures [`trains`] solves, then
/// every train the seeded walk visits, each labelled by the steps that made
/// it.
fn structures() {
    let walked = gear_core::train::sweep::visited()
        .into_iter()
        .map(|(walk, steps, t)| (format!("{walk}: {steps:?}"), t));
    for (name, train) in fixtures().chain(walked) {
        println!("== structure {name}");
        guarded(&name, || {
            println!("mesh groups {:?}", train.shape.mesh_groups());
            for (p, part) in train.parts().iter().enumerate() {
                println!(
                    "part {p}: members {:?} meshes {:?} distances {:?} axes {:?} couplings {:?}",
                    part.members, part.meshes, part.distances, part.axes, part.couplings
                );
                println!("{:#?}", part.shape.structure());
            }
        });
    }
}

pub fn run() {
    let lib = gear_io::default_library();
    trains(&lib);
    alone(&lib);
    edits(&lib);
    gears();
    screws();
    structures();
}
