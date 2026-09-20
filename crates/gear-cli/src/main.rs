//! Development harness for `gear-core`.
//!
//! Exists so the mathematics can be driven and inspected without a browser.
//!
//! ```text
//! gear-cli help          every subcommand, its arguments and its defaults
//! ```
//!
//! **The list is [`COMMANDS`], and `help` prints it.** It used to be written out
//! here as well, and `docs/state.md` pointed at this comment as the exhaustive
//! one — "next to the code it describes, where it cannot fall out of step with
//! the commands it lists". It had fallen out of step by eight of twenty-one.
//! Proximity is not a mechanism; a table that *is* the dispatch is.

mod diagram;
mod kinematics;
mod matrix;

use gear_core::{GearParams, Tooth};

/// The English catalogue, for turning a [`Note`](gear_core::note::Note) into a
/// sentence.
///
/// The harness has no locale to choose from and does not want one — it exists to
/// show what the core computed. Built per call rather than cached: this is a
/// development tool printing a handful of lines, and a `OnceLock` here would be
/// machinery in place of a parse that costs nothing.
/// A worm stage at the preset with these counts and this worm diameter — what
/// every worm command here builds from.
fn worm_stage(starts: u32, wheel_teeth: u32, worm_diameter: f64) -> gear_core::train::PairStage {
    use gear_core::train::PairStage;
    let mut stage = PairStage::worm().with_first_diameter(worm_diameter);
    stage.gears[0].teeth = starts;
    stage.gears[1].teeth = wheel_teeth;
    stage
}

/// **A stage of two members and one mesh, read as the pair it is** — the
/// harness's own view of a shape's result, so the commands that build pairs
/// print them by the names a pair has. Nothing in the core has this shape
/// any more; a stage's result is a shape's ([`gear_core::train::shape`]).
struct Pair<'a> {
    ratio: f64,
    centre_distance: f64,
    clearance: f64,
    mesh: &'a gear_core::train::MeshReport,
    gears: [&'a gear_core::train::GearResult; 2],
    notes: &'a [gear_core::note::Note],
}

/// The pair a shape's result is, where it is one.
fn pair(r: &gear_core::train::StageResult) -> Option<Pair<'_>> {
    let s = r.as_shape()?;
    if s.members.len() != 2 || s.meshes.len() != 1 {
        return None;
    }
    let d = s.distances.first()?;
    Some(Pair {
        ratio: s.ratio,
        centre_distance: d.running,
        clearance: d.clearance,
        mesh: &s.meshes[0],
        gears: [&s.members[0], &s.members[1]],
        notes: &s.notes,
    })
}

/// **A stage of three members and two meshes, read as the set it is** — sun,
/// planet, ring; sun–planet, planet–ring — for the commands that build one.
struct SetView<'a> {
    ratio: f64,
    efficiency: gear_core::contact::Directional<f64>,
    centre_distance: f64,
    centre_distance_nominal: [f64; 2],
    /// How far the two meshes disagree about the one distance, each opened
    /// by the clearance its own way.
    residual: f64,
    /// The two meshes in series: `η₀`.
    eta0: f64,
    sun: &'a gear_core::train::GearResult,
    planet: &'a gear_core::train::GearResult,
    ring: &'a gear_core::train::GearResult,
    sun_planet: &'a gear_core::train::MeshReport,
    planet_ring: &'a gear_core::train::MeshReport,
    layout: Option<&'a gear_core::train::shape::LayoutReport>,
    notes: &'a [gear_core::note::Note],
}

/// The set a shape's result is, where it is one.
fn set_view(r: &gear_core::train::StageResult) -> Option<SetView<'_>> {
    let s = r.as_shape()?;
    if s.members.len() != 3 || s.meshes.len() != 2 {
        return None;
    }
    let d = s.distances.first()?;
    let (n0, n1) = (*d.nominal.first()?, *d.nominal.get(1)?);
    Some(SetView {
        ratio: s.ratio,
        efficiency: s.efficiency,
        centre_distance: d.running,
        centre_distance_nominal: [n0, n1],
        residual: ((n0 + d.clearance) - (n1 - d.clearance)).abs(),
        eta0: s.meshes[0].efficiency.forward * s.meshes[1].efficiency.forward,
        sun: &s.members[0],
        planet: &s.members[1],
        ring: &s.members[2],
        sun_planet: &s.meshes[0],
        planet_ring: &s.meshes[1],
        layout: s.layouts.first(),
        notes: &s.notes,
    })
}

/// A set through the shape, as the commands here build one.
fn solve_set(
    stage: &gear_core::train::PlanetaryStage,
    loads: &gear_core::train::StageLoads,
    lib: &gear_core::material::MaterialLibrary,
) -> Result<gear_core::train::StageResult, gear_core::train::TrainError> {
    gear_core::train::solve_any(
        &gear_core::train::Stage::planetary(stage.clone()),
        loads,
        lib,
    )
}

/// **A stage of four members on a crank, read as the hula stage it is** —
/// the grounded gear, the two on the wobble body, the output; two meshes at
/// one crank offset — for the commands that build one.
struct HulaView<'a> {
    ratio: f64,
    /// `z₂z₄` and `D = z₂z₄ − z₁z₃`, the two products the reduction is
    /// written in: counted here from the teeth, since the graph's exact ratio
    /// is the same quotient.
    ratio_products: [i64; 2],
    /// The crank offset at zero backlash on the first mesh, and as run.
    offset_nominal: f64,
    offset: f64,
    /// Which mesh's tips sized the offset, where the tips did.
    binding_mesh: Option<usize>,
    efficiency: gear_core::contact::Directional<f64>,
    /// The two meshes alone, crank held: their efficiencies multiplied.
    fixed_carrier_efficiency: f64,
    backlash: gear_core::contact::Directional<gear_core::train::Backlash>,
    /// Speeds and torques per case on the grounded gear's shaft, the crank
    /// and the output — the shape's shafts 3, 1 and 2.
    cases: Vec<(usize, [f64; 3], [f64; 3])>,
    /// Each member with whether it is a ring, in the stage's order.
    gears: Vec<(&'a gear_core::train::GearResult, bool)>,
    meshes: &'a [gear_core::train::MeshReport],
    notes: &'a [gear_core::note::Note],
}

/// The hula stage a shape's result is.
fn hula_view<'a>(
    stage: &'a gear_core::train::Stage,
    r: &'a gear_core::train::StageResult,
) -> Option<HulaView<'a>> {
    let shape = stage.as_shape()?;
    let s = r.as_shape()?;
    if s.members.len() != 4 || s.meshes.len() != 2 {
        return None;
    }
    let d = s.distances.first()?;
    let z: Vec<i64> = shape
        .members
        .iter()
        .map(|m| i64::from(m.gear.teeth))
        .collect();
    let numerator = z[1] * z[3];
    Some(HulaView {
        ratio: s.ratio,
        ratio_products: [numerator, numerator - z[0] * z[2]],
        offset_nominal: *d.nominal.first()?,
        offset: d.running,
        binding_mesh: d.sized_by,
        efficiency: s.efficiency,
        fixed_carrier_efficiency: s.meshes.iter().map(|m| m.efficiency.forward).product(),
        backlash: s.backlash,
        cases: s
            .cases
            .iter()
            .map(|c| {
                (
                    c.case,
                    [c.speeds[3], c.speeds[1], c.speeds[2]],
                    [c.torques[3], c.torques[1], c.torques[2]],
                )
            })
            .collect(),
        gears: s
            .members
            .iter()
            .zip(&shape.members)
            .map(|(g, m)| (g, m.ring.is_some()))
            .collect(),
        meshes: &s.meshes,
        notes: &s.notes,
    })
}

/// A hula stage through the shape under its own arrangement — crank driven,
/// grounded gear held, output out — whatever its counts make the rings.
fn solve_hula(
    stage: &gear_core::train::HulaStage,
    loads: &gear_core::train::StageLoads,
    lib: &gear_core::material::MaterialLibrary,
) -> Result<(gear_core::train::Stage, gear_core::train::StageResult), gear_core::train::TrainError>
{
    let as_stage = gear_core::train::Stage::hula(stage.clone());
    let boundary = gear_core::train::StageBoundary::holding(5, &[3], 1, 2);
    let r = gear_core::train::solve_any(&as_stage, &loads.clone().under(boundary), lib)?;
    Ok((as_stage, r))
}

/// A pair through the shape, as the commands here build one.
fn solve_pair(
    stage: &gear_core::train::PairStage,
    kind: gear_core::train::PairKind,
    loads: &gear_core::train::StageLoads,
    lib: &gear_core::material::MaterialLibrary,
) -> Result<gear_core::train::StageResult, gear_core::train::TrainError> {
    let as_stage = match kind {
        gear_core::train::PairKind::Spur => gear_core::train::Stage::spur(stage.clone()),
        gear_core::train::PairKind::Worm => gear_core::train::Stage::worm(stage.clone()),
    };
    gear_core::train::solve_any(&as_stage, loads, lib)
}

/// The mesh a pair reports, checked to be the point contact these commands
/// built a crossed pair for.
fn point<'a>(r: &Pair<'a>) -> &'a gear_core::train::MeshReport {
    assert!(
        r.mesh.point.is_some(),
        "this command builds crossed pairs, which report a point contact"
    );
    r.mesh
}

/// The transverse figures a line contact reports — the commands that read them
/// are the ones that built a parallel pair.
fn line<'a>(r: &Pair<'a>) -> &'a gear_core::train::LineContact {
    transverse(r.mesh)
}

/// The same, of any mesh a command built on parallel shafts.
fn transverse(m: &gear_core::train::MeshReport) -> &gear_core::train::LineContact {
    m.line
        .as_ref()
        .expect("this command builds parallel-axis meshes, which report a line contact")
}

fn words() -> gear_io::strings::Catalogue {
    gear_io::strings::Catalogue::english()
}

/// A mesh efficiency in both directions, with any direction it refuses to be
/// driven in **named**.
///
/// One helper because there were four sites, each printing `(self-locking)` or
/// `(cannot be back-driven)` off `Directional::self_locking`, and none of them
/// able to say the other thing — a pair that cannot be driven *forward* showed
/// only `0.000 %` in the forward column, which reads as arithmetic rather than
/// as a statement about the mechanism. See `Directional::locked`.
fn both_ways(e: gear_core::contact::Directional<f64>) -> String {
    let locked = e.locked();
    let said = match (locked.forward, locked.backward) {
        (true, true) => "  (turns neither way)",
        (true, false) => "  (cannot be driven forward)",
        (false, true) => "  (cannot be back-driven)",
        (false, false) => "",
    };
    format!(
        "{:.3} % forward / {:.3} % backward{said}",
        100.0 * e.forward,
        100.0 * e.backward
    )
}

/// One subcommand: what it is called, what it takes, what it does, and how to
/// run it.
///
/// **The table below is the dispatch**, so the list and the code cannot
/// disagree. They had: `docs/state.md` pointed at this module's own comment as
/// the exhaustive list, "next to the code it describes, where it cannot fall out
/// of step with the commands it lists" — and it listed thirteen of twenty-one.
/// Proximity is not a mechanism. The same table shape, for the same reason, is
/// what the front end's gear kinds and stage presets became.
struct Command {
    name: &'static str,
    /// The arguments as a reader would type them, defaults in the summary.
    args: &'static str,
    summary: &'static str,
    run: fn(&[String]),
    /// How `tools/check_golden.sh` records what this prints.
    record: Record,
    /// Slow enough that a working-tree run may skip it. About a minute between
    /// the five that are.
    slow: bool,
}

/// How a command's output is kept, so that a change in it is a diff.
///
/// **A field rather than a list in the shell script**, which is where it started
/// and where it would have rotted: a command added here with no line added there
/// is invisible, which is the same shape as the eight commands this module's own
/// comment had stopped listing. The script asks the binary now
/// (`gear-cli --golden-cases`), so the corpus cannot be missing a command that
/// exists.
enum Record {
    /// In full, at these invocations. More than one where a single call would
    /// leave a whole regime uncovered.
    Cases(&'static [&'static str]),
    /// As a digest of the output, with the reason. A checked-in file nobody
    /// reads a diff of is a change detector that detects nothing.
    Digest(&'static str, &'static str),
    /// Not by `check_golden.sh` at all, and by what instead.
    Elsewhere(&'static str),
}

/// A positional argument, or its default. Written out forty times before this
/// existed, which made adding one a matter of copying the incantation.
fn arg<T: std::str::FromStr>(args: &[String], n: usize, default: T) -> T {
    args.get(n).and_then(|s| s.parse().ok()).unwrap_or(default)
}

/// ...and one that is genuinely optional, where absent is not a default value
/// but a different question.
fn opt<T: std::str::FromStr>(args: &[String], n: usize) -> Option<T> {
    args.get(n).and_then(|s| s.parse().ok())
}

const COMMANDS: &[Command] = &[
    Command {
        name: "show",
        args: "[z] [x]",
        summary: "one gear's derived geometry (17, 0)",
        run: |a| {
            show(GearParams {
                teeth: arg(a, 1, 17),
                profile_shift: arg(a, 2, 0.0),
                ..Default::default()
            });
        },
        record: Record::Cases(&["show 17 0.2", "show 9 -0.3"]),
        slow: false
    },
    Command {
        name: "sweep",
        args: "",
        summary: "scan a parameter grid for clamps, undercut and severing",
        run: |_| sweep(),
        record: Record::Cases(&["sweep"]),
        slow: false
    },
    Command {
        name: "dump",
        args: "",
        summary: "every sampled profile point, for an external check",
        run: |_| dump(),
        record: Record::Digest("dump", "10.9 MB of raw profile points; every figure in it is one another command derives and prints in ten"),
        slow: false
    },
    Command {
        name: "materials",
        args: "",
        summary: "the material library, with each value's basis",
        run: |_| materials(),
        record: Record::Cases(&["materials"]),
        slow: false
    },
    Command {
        name: "strength",
        args: "[z1] [z2] [torque] [material] [helix] [rim]",
        summary: "a worked mesh: bending, contact, efficiency (17, 43, 2 N·m)",
        run: |a| {
            strength_report(
                arg(a, 1, 17),
                arg(a, 2, 43),
                arg(a, 3, 2.0),
                a.get(4).map_or("4340 Hardened Steel", String::as_str),
                arg(a, 5, 0.0),
                opt(a, 6),
            );
        },
        record: Record::Cases(&["strength 17 43 2.0"]),
        slow: false
    },
    Command {
        name: "bending",
        args: "",
        summary: "the bending construction drawn tooth by tooth — the body of docs/bending-check.html",
        run: |_| bending_report(),
        record: Record::Elsewhere("its output *is* `docs/bending-check.html`, checked verbatim by `tools/check_figures.py`"),
        slow: false
    },
    Command {
        name: "matrix",
        args: "",
        summary: "the bending-model matrix on external teeth and on rings — the ISO comparison",
        run: |_| matrix_report(),
        record: Record::Cases(&["matrix"]),
        slow: true
    },
    Command {
        name: "loadcase",
        args: "",
        summary: "a stage's two load cases side by side",
        run: |_| loadcase_report(),
        record: Record::Cases(&["loadcase"]),
        slow: false
    },
    Command {
        name: "shifts",
        args: "[z1] [z2] | epicyclic",
        summary: "the shifts a pair loses least at, free and against a given centre distance (9, 37); `epicyclic` asks the two presets that choose more than two",
        run: |a| {
            if a.get(1).map(String::as_str) == Some("epicyclic") {
                epicyclic_shifts_report();
            } else {
                shifts_report(arg(a, 1, 9), arg(a, 2, 37));
            }
        },
        // **Two pairs, because the documented table has two rows** — and an
        // epicyclic case because the corpus had never walked the crate's one
        // optimiser at all: no `gear-cli` command set `Optimisation::enabled`,
        // so every answer it chooses was outside the change detector. That is
        // the fault `docs/corrections.md` records of a load from the end, met
        // again, and it is worth covering **each preset that searches** rather
        // than the one whose table is documented.
        record: Record::Cases(&["shifts 9 37", "shifts 17 43", "shifts epicyclic"]),
        slow: false
    },
    Command {
        name: "train",
        args: "[mixed|held|toggles]",
        summary: "a two-stage geartrain, end to end; `mixed` puts a worm stage in it, `held` back-drives that worm harder than the drive does, `toggles` turns on every optional control",
        run: |a| train_report(a.get(1).map(String::as_str)),
        record: Record::Cases(&["train", "train mixed", "train held", "train toggles"]),
        slow: false
    },
    Command {
        name: "kinematics",
        args: "[fixture]",
        summary: "motion, torque, loss and play alone, for every preset and every epicyclic arrangement — what a graph over shafts and meshes has to reproduce",
        run: |a| kinematics::run(a.get(1).map(String::as_str)),
        record: Record::Cases(&["kinematics"]),
        slow: false
    },
    Command {
        name: "trainfile",
        args: "[path]",
        summary: "a train to TOML and back, both answers compared",
        run: |a| train_file_report(a.get(1).map(String::as_str)),
        record: Record::Cases(&["trainfile"]),
        slow: false
    },
    Command {
        name: "dxf",
        args: "[z] [x] [chord tolerance]",
        summary: "a gear exported to DXF, on stdout (17, 0, 1e-3)",
        run: |a| dxf(arg(a, 1, 17), arg(a, 2, 0.0), arg(a, 3, 1e-3)),
        record: Record::Cases(&["dxf 17 0.2 0.001"]),
        slow: false
    },
    Command {
        name: "worm",
        args: "[starts] [z_wheel] [d_worm] [shaft angle]",
        summary: "a worm pair, both directions (1, 40, 7 mm, 90°)",
        run: |a| worm_report(arg(a, 1, 1), arg(a, 2, 40), arg(a, 3, 7.0), arg(a, 4, 90.0)),
        record: Record::Cases(&["worm 1 40 7 90"]),
        slow: false
    },
    Command {
        name: "wormstage",
        args: "[starts] [z_wheel] [d_worm] [torque]",
        summary: "a worm stage, end to end (1, 40, 7 mm, 2 N·m)",
        run: |a| {
            worm_stage_report(arg(a, 1, 1), arg(a, 2, 40), arg(a, 3, 7.0), arg(a, 4, 2.0));
        },
        record: Record::Cases(&["wormstage 1 40 7 2"]),
        slow: false
    },
    Command {
        name: "crossed",
        args: "[z1] [z2] [shaft angle]",
        summary: "a crossed pair, swept over the helix split (17, 23, 90°)",
        run: |a| crossed_report(arg(a, 1, 17), arg(a, 2, 23), arg(a, 3, 90.0)),
        record: Record::Cases(&["crossed 17 23 90", "crossed 17 43 5"]),
        slow: false
    },
    Command {
        name: "planetary",
        args: "[z_sun] [z_planet] [N] [x_sun] [x_ring]",
        summary: "every ring count that can work, and the planet shift each needs (17, 17, 3)",
        run: |a| {
            planetary_report(
                arg(a, 1, 17),
                arg(a, 2, 17),
                arg(a, 3, 3),
                arg(a, 4, 0.0),
                arg(a, 5, 0.0),
            );
        },
        record: Record::Cases(&["planetary 17 17 3"]),
        slow: false
    },
    Command {
        name: "planetstage",
        args: "[z_sun] [z_planet] [z_ring] [N] [helix]",
        summary: "a planetary stage in all six arrangements (12, 30, 72, 3)",
        run: |a| {
            planetary_stage_report(
                arg(a, 1, 12),
                arg(a, 2, 30),
                arg(a, 3, 72),
                arg(a, 4, 3),
                arg(a, 5, 0.0),
            );
        },
        record: Record::Cases(&["planetstage 12 30 72 3", "planetstage 24 18 60 3"]),
        slow: false
    },
    Command {
        name: "hula",
        args: "[N] [clearance] [m_outer] [m_inner] [cutter teeth]",
        summary: "a hula stage: the offset, the shifts it takes, and what the teeth then do (18, 0.5)",
        run: |a| {
            hula_report(
                arg(a, 1, 18),
                arg(a, 2, 0.5),
                arg(a, 3, 1.0),
                arg(a, 4, 1.0),
                opt(a, 5),
            );
        },
        record: Record::Cases(&["hula 18 0.2"]),
        slow: false
    },
    Command {
        name: "hulaband",
        args: "[N] [clearance in modules]",
        summary: "one reduction at every tooth difference, to see what the difference of one costs (18, 0.30)",
        run: |a| hula_band(arg(a, 1, 18), arg(a, 2, 0.30)),
        record: Record::Cases(&["hulaband 18"]),
        slow: true
    },
    Command {
        name: "meshsweep",
        args: "[z_ring] [z_pinion] [ring addendum] [pinion addendum]",
        summary: "roll an ordinary internal pair through a tooth — the control (40, 20)",
        run: |a| {
            mesh_sweep(arg(a, 1, 40), arg(a, 2, 20), arg(a, 3, 1.0), arg(a, 4, 1.0));
        },
        record: Record::Cases(&["meshsweep 60 20 0.8"]),
        slow: true
    },
    Command {
        name: "hulasweep",
        args: "[N] [clearance] [mesh]",
        summary: "the same roll on a hula pair, where the tip circles cross (18, 0.2)",
        run: |a| hula_sweep(arg(a, 1, 18), arg(a, 2, 0.2), arg(a, 3, 0)),
        record: Record::Cases(&["hulasweep 18 0.25"]),
        slow: true
    },
    Command {
        name: "verify",
        args: "[cases]",
        summary: "the two-sided cutter check over a parameter grid (all of it)",
        run: |a| verify(arg(a, 1, usize::MAX)),
        record: Record::Cases(&["verify 100"]),
        slow: true
    },
];

fn help() {
    let width = COMMANDS.iter().map(|c| c.name.len() + c.args.len()).max();
    println!("gear-cli — drive the mathematics without a browser\n");
    for c in COMMANDS {
        let call = format!("{} {}", c.name, c.args);
        println!(
            "  {call:<width$}   {}",
            c.summary,
            width = width.unwrap_or(0) + 1
        );
    }
}

/// What `tools/check_golden.sh` should run and how to keep each one.
///
/// `speed \t keep \t invocation \t why`, one per line. A flag rather than a
/// subcommand so that "every subcommand is recorded" stays a statement with no
/// exception carved out of it for the one that lists them.
///
/// **A command kept elsewhere is on this list too**, with `keep = none` and its
/// reason, so the check can end by saying what it did *not* record and why. A
/// coverage claim that omits its own exceptions is the shape of the fault this
/// whole table replaced.
fn golden_cases() {
    for c in COMMANDS {
        let speed = if c.slow { "slow" } else { "fast" };
        match c.record {
            Record::Cases(cases) => {
                for case in cases {
                    println!("{speed}\tfull\t{case}\t");
                }
            }
            Record::Digest(case, why) => println!("{speed}\tdigest\t{case}\t{why}"),
            Record::Elsewhere(why) => println!("{speed}\tnone\t{}\t{why}", c.name),
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    // No argument at all is `show`, which is the cheapest useful thing here and
    // was the default before this table existed.
    let name = args.first().map_or("show", String::as_str);
    if matches!(name, "help" | "-h" | "--help") {
        help();
        return;
    }
    if name == "--golden-cases" {
        golden_cases();
        return;
    }
    match COMMANDS.iter().find(|c| c.name == name) {
        Some(c) => (c.run)(&args),
        None => {
            eprintln!("unknown command {name:?}\n");
            help();
        }
    }
}

/// A hula stage, from the arrangement down to what the teeth do.
///
/// Drives the shape under the hula's own arrangement rather than assembling
/// the parts itself:
/// the stage is where an arrangement becomes gears, and a harness that built its own
/// would be a second answer to the same question — which is how the two start
/// disagreeing.
fn hula_report(n: u32, clearance: f64, m_outer: f64, m_inner: f64, cutter_teeth: Option<u32>) {
    use gear_core::train::{HulaStage, StageLoads};

    let lib = gear_io::default_library();
    let teeth = [n + 1, n, n - 1, n];
    let mut stage = HulaStage {
        module: [m_outer, m_inner],
        clearance,
        ..HulaStage::default()
    };
    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
        gear.teeth = count;
    }
    // **A shaper has to be smaller than the ring it cuts**, and this command
    // builds rings far smaller than the ones the stage ships with: `hula 18`
    // asks for a 19-tooth ring, which the stocked default would not fit inside.
    // So the default stands where it fits and gives way to five teeth under the
    // ring where it does not — the sizing the comment here has always claimed
    // and never did, since the count it worked out was discarded unused.
    for (mesh, cutter) in stage.cutter.iter_mut().enumerate() {
        let ring = teeth[mesh * 2].max(teeth[mesh * 2 + 1]);
        let stocked = cutter.teeth;
        cutter.teeth = cutter_teeth.unwrap_or_else(|| stocked.min(ring.saturating_sub(5)).max(4));
    }

    let (as_stage, solved) = match solve_hula(&stage, &StageLoads::at(2.0, 1000.0), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("that stage has no geometry: {e}");
            return;
        }
    };
    let Some(result) = hula_view(&as_stage, &solved) else {
        eprintln!("that stage is not a hula stage");
        return;
    };

    println!(
        "hula  z {}/{}/{}/{}  module {m_outer}/{m_inner}  alpha {} deg  clearance {clearance} mm",
        teeth[0], teeth[1], teeth[2], teeth[3], stage.pressure_angle
    );
    println!(
        "  ratio {} / {} = {:+.4}   crank offset {:.6} mm (running {:.6}){}",
        result.ratio_products[0],
        result.ratio_products[1],
        result.ratio,
        result.offset_nominal,
        result.offset,
        match result.binding_mesh {
            Some(m) => format!("   held open by mesh {}", m + 1),
            None => String::new(),
        }
    );
    println!(
        "  speeds  crank {:.1}  wobble {:+.3}  output {:+.4} rpm",
        result.cases[0].1[1], result.gears[1].0.cases[0].speed, result.gears[3].0.cases[0].speed
    );
    println!(
        "  efficiency {:.3} % forward, {:.3} % back-driven{}   (the two meshes alone, crank held: {:.4})",
        result.efficiency.forward * 100.0,
        result.efficiency.backward * 100.0,
        if result.efficiency.backward == 0.0 {
            "  (self-locking)"
        } else {
            ""
        },
        result.fixed_carrier_efficiency
    );
    println!(
        "  backlash at the output {:.6} deg (min {:.6}, max {:.6})   at the crank {:.4} deg",
        result.backlash.forward.nominal,
        result.backlash.forward.minimum,
        result.backlash.forward.maximum,
        result.backlash.backward.nominal
    );

    for (index, mesh) in result.meshes.iter().enumerate() {
        let members = &result.gears[index * 2..index * 2 + 2];
        println!(
            "\n  mesh {}  {}   alpha_w {:.3} deg   shaper z{}",
            index + 1,
            members
                .iter()
                .map(|(g, ring)| format!(
                    "{} z{} x{:+.4}",
                    if *ring { "ring" } else { "pinion" },
                    g.params.teeth,
                    g.profile_shift
                ))
                .collect::<Vec<_>>()
                .join("  "),
            transverse(mesh).operating_pressure_angle,
            stage.cutter[index].teeth
        );
        println!(
            "    far-side gap {:.4} mm   contact ratio {:.4}",
            mesh.tips.map_or(f64::NAN, |t| t.far_gap),
            transverse(mesh).contact_ratios.transverse
        );
        println!(
            "    backlash {:.5} / {:.5} deg   flank interference: pinion {}  ring {}   \
             tip {} ({:+.4} deg)",
            mesh.backlash[0].nominal,
            mesh.backlash[1].nominal,
            mesh.flank_interference[0],
            mesh.flank_interference[1],
            mesh.tips.is_some_and(|t| t.tip_interference),
            mesh.tips.map_or(0.0, |t| t.tip_margin)
        );
        for note in &mesh.notes {
            println!("    ! {}", words().render(note));
        }
        // What the teeth are worth, which a worm stage needs as much as
        // the geometry: the reduction multiplies the mesh loss, and it multiplies
        // the torque on the way as well — the output pair carries the whole of it.
        println!(
            "    sigma_H {:.1} MPa at the pitch point   rho {:.4} mm",
            mesh.cases[0].contact.at_pitch_point,
            1.0 / mesh.cases[0].contact.curvature_across
        );
        for (gear, _) in members {
            println!(
                "    z{:<4} T {:>10.4} Nm  b {:>7.3} mm  sigma_F {:>8}  sigma_H {:>7.1} MPa",
                gear.params.teeth,
                gear.cases[0].torque,
                gear.face_width,
                gear.cases[0]
                    .bending_stress
                    .map_or_else(|| "—".to_string(), |s| format!("{s:.1}")),
                gear.cases[0].contact_stress,
            );
            for note in gear.clamps.iter().chain(&gear.notes) {
                println!("    ! z{}: {}", gear.params.teeth, words().render(note));
            }
        }
    }
    for note in result.notes {
        println!("  ! {}", words().render(note));
    }
}

/// An outline as plain points, with its **arcs expanded rather than chorded**.
///
/// A tip and a root are exact arcs carried as vertex bulges, so reading only the
/// vertices replaces each with its chord — which on a 19 mm tip sits six microns
/// inside the true surface. That is small, and it is four times the interference
/// this harness was built to measure.
fn flatten(v: &[gear_core::Vertex]) -> Vec<(f64, f64)> {
    const PER_ARC: usize = 64;
    let mut out = Vec::with_capacity(v.len() * 2);
    for i in 0..v.len() {
        let (a, b) = (v[i], v[(i + 1) % v.len()]);
        out.push((a.x, a.y));
        if a.bulge.abs() < 1e-15 {
            continue;
        }
        // `bulge = tan(θ/4)`, so the included angle, the radius and the centre
        // all follow; the sign of the bulge carries the sense through `R`.
        let theta = 4.0 * a.bulge.atan();
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let chord = dx.hypot(dy);
        if chord < 1e-15 {
            continue;
        }
        let radius = chord / (2.0 * (theta / 2.0).sin());
        let h = radius * (theta / 2.0).cos();
        let centre = (
            (a.x + b.x) / 2.0 - dy / chord * h,
            (a.y + b.y) / 2.0 + dx / chord * h,
        );
        let start = (a.y - centre.1).atan2(a.x - centre.0);
        let r = radius.abs();
        for k in 1..PER_ARC {
            let t = start + theta * (k as f64) / (PER_ARC as f64);
            out.push((centre.0 + r * t.cos(), centre.1 + r * t.sin()));
        }
    }
    out
}

/// A closed outline, as segments bucketed by the angle they span about the
/// origin.
///
/// **Containment, not a radius table.** An earlier version of this compared a
/// point's radius against the bore's radius interpolated at the same angle,
/// which is not a distance between two curves and is not even defined where a
/// fillet doubles back in angle. It reported a seventieth of a millimetre of
/// interference on a pair that has none, which is how it was caught: the control
/// is not decoration.
///
/// A radial ray from a point stays in one angular bucket, so the crossing test
/// that decides inside from outside is exact and costs one bucket. The distance
/// wants a few buckets either side, since the nearest segment need not span the
/// point's own angle.
struct Boundary {
    segments: Vec<[f64; 4]>,
    buckets: Vec<Vec<usize>>,
}

impl Boundary {
    const BUCKETS: usize = 3600;

    fn new(points: &[(f64, f64)]) -> Self {
        let mut segments = Vec::with_capacity(points.len());
        for i in 0..points.len() {
            let (a, b) = (points[i], points[(i + 1) % points.len()]);
            segments.push([a.0, a.1, b.0, b.1]);
        }
        let mut buckets = vec![Vec::new(); Self::BUCKETS];
        let index = Self::bucket_of;
        for (i, s) in segments.iter().enumerate() {
            // The angular span of a segment, the short way round: a segment of a
            // gear outline never spans half a turn.
            let (t0, t1) = (s[1].atan2(s[0]), s[3].atan2(s[2]));
            let mut d = (t1 - t0).rem_euclid(std::f64::consts::TAU);
            if d > std::f64::consts::PI {
                d -= std::f64::consts::TAU;
            }
            // Clamped into range first, so the conversion below is exact and
            // the lint is waived for a value that cannot be out of it.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let steps = ((d.abs() / std::f64::consts::TAU) * Self::BUCKETS as f64)
                .ceil()
                .clamp(1.0, Self::BUCKETS as f64) as usize;
            for k in 0..=steps.max(1) {
                let t = t0 + d * (k as f64) / (steps.max(1) as f64);
                buckets[index(t)].push(i);
            }
        }
        // **Once per bucket.** A short segment samples into the same bucket at
        // both ends, and a segment counted twice by the crossing test turns an
        // odd number of crossings into an even one — which is to say, turns
        // inside into outside for every point on that ray.
        for b in &mut buckets {
            b.sort_unstable();
            b.dedup();
        }
        Self { segments, buckets }
    }

    /// The bucket an angle falls in.
    ///
    /// `rem_euclid` puts it in `[0, τ)` and the scale puts it in
    /// `[0, BUCKETS)`, so the conversion is in range by construction; the
    /// remainder catches the one value that rounds up to the end.
    fn bucket_of(theta: f64) -> usize {
        let scaled = (theta.rem_euclid(std::f64::consts::TAU) / std::f64::consts::TAU)
            * Self::BUCKETS as f64;
        // Clamped into range first, as above.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let bucket = scaled.trunc().clamp(0.0, Self::BUCKETS as f64 - 1.0) as usize;
        bucket
    }

    /// Positive inside the outline, negative outside, in millimetres.
    fn clearance(&self, p: (f64, f64)) -> f64 {
        let theta = p.1.atan2(p.0);
        let r = p.0.hypot(p.1);
        // Inside or out: a ray straight out from the centre through `p` crosses
        // the outline an odd number of times beyond `p` exactly when `p` is
        // within it.
        let (c, s) = (theta.cos(), theta.sin());
        let mut crossings = 0;
        for &i in &self.buckets[Self::bucket_of(theta)] {
            let g = self.segments[i];
            // Cross the ray x = r' (c, s), r' > r, with the segment.
            let (a, b) = ((g[0], g[1]), (g[2], g[3]));
            let (na, nb) = (a.0 * -s + a.1 * c, b.0 * -s + b.1 * c);
            if (na > 0.0) == (nb > 0.0) {
                continue;
            }
            let t = na / (na - nb);
            let hit = (a.0 + t * (b.0 - a.0)) * c + (a.1 + t * (b.1 - a.1)) * s;
            if hit > r {
                crossings += 1;
            }
        }
        let inside = crossings % 2 == 1;

        let mut least = f64::INFINITY;
        let here = Self::bucket_of(theta);
        for k in 0..=60usize {
            let b = (here + Self::BUCKETS + k - 30) % Self::BUCKETS;
            for &i in &self.buckets[b] {
                let g = self.segments[i];
                let (dx, dy) = (g[2] - g[0], g[3] - g[1]);
                let len2 = dx * dx + dy * dy;
                let t = if len2 > 0.0 {
                    (((p.0 - g[0]) * dx + (p.1 - g[1]) * dy) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                let d = (p.0 - (g[0] + t * dx)).hypot(p.1 - (g[1] + t * dy));
                least = least.min(d);
            }
        }
        if inside {
            least
        } else {
            -least
        }
    }
}

/// Roll an internal pair through one tooth and report where the flanks touch.
///
/// **A contact ratio counts tooth pairs on the line of action; it does not say
/// whether the rest of the teeth are clear of each other.** The two questions
/// are separate, and at one tooth of difference the second is the one in doubt.
///
/// The assembly phase is swept rather than assumed, because an assembly that has
/// a phase is one that can be built, and the outlines' own conventions are not
/// this harness's business.
fn roll_pair(ring: &gear_core::ring::Ring, pinion: &gear_core::Gear, a: f64, title: &str) {
    const TOLERANCE: f64 = 1e-5;
    let pin_pts = flatten(&pinion.outline(TOLERANCE));
    let bore = Boundary::new(&flatten(&ring.outline(TOLERANCE)));

    let (z1, z2) = (f64::from(pinion.mean().params.teeth), f64::from(ring.teeth));
    // Only the points that could reach the bore are worth testing: everything
    // well inside the ring's tip circle is clear whatever the phase.
    let near: Vec<(f64, f64)> = pin_pts
        .iter()
        .copied()
        .filter(|(x, y)| (x + a).hypot(*y) > ring.ra - 1.0 || x.hypot(*y) > pinion.mean().ra - 1.0)
        .collect();

    let worst = |psi: f64, phi: f64| -> (f64, f64) {
        let (c1, s1) = (phi.cos(), phi.sin());
        let ring_angle = phi * z1 / z2 + psi;
        let (cr, sr) = ((-ring_angle).cos(), (-ring_angle).sin());
        let mut least = f64::INFINITY;
        let mut at = 0.0;
        for (x, y) in &near {
            let (fx, fy) = (x * c1 - y * s1 + a, x * s1 + y * c1);
            let p = (fx * cr - fy * sr, fx * sr + fy * cr);
            let gap = bore.clearance(p);
            if gap < least {
                least = gap;
                at = fy.atan2(fx);
            }
        }
        (least, at)
    };

    let pitch = std::f64::consts::TAU / z2;
    let mut best = (f64::NEG_INFINITY, 0.0);
    for i in 0..600 {
        let psi = pitch * f64::from(i) / 600.0;
        let g = worst(psi, 0.0).0;
        if g > best.0 {
            best = (g, psi);
        }
    }
    let psi = best.1;

    println!("{title}   centre distance {a:.4} mm");
    println!(
        "  best assembly phase {:.4} deg of a {:.3} deg ring pitch, clear by {:+.5} mm there",
        psi.to_degrees(),
        pitch.to_degrees(),
        best.0
    );

    let mut overall = (f64::INFINITY, 0.0);
    for step in 0..180 {
        let phi = (std::f64::consts::TAU / z1) * f64::from(step) / 180.0;
        let (gap, at) = worst(psi, phi);
        if gap < overall.0 {
            overall = (gap, at);
        }
    }
    println!(
        "  rolled through one tooth: least clearance {:+.5} mm at {:.1} deg from the line of centres{}",
        overall.0,
        overall.1.to_degrees(),
        if overall.0 < -10.0 * TOLERANCE {
            "   THE TEETH FOUL"
        } else {
            "   (touching, as a mesh does)"
        }
    );
}

/// The same reduction at every tooth difference, and what each one can reach.
///
/// **A reduction does not say how to get it.** `R = z²/d²`, so `z = d·z₀` gives
/// `z₀²` at any difference `d` — the same ratio, the same pitch diameters and
/// the same crank offset, reached with `d` times the teeth at a `d`th of the
/// module. Whether that matters is the question this answers, and the answer is
/// not small.
///
/// For each difference it searches what a designer would: the addendum, the
/// shaper, and the division of each mesh's shift. What it reports is the best
/// design that is *admissible* — contact continuous, no interference of any of
/// the three kinds, nothing clamped — which is the only kind worth comparing.
///
/// # Which shifts these are
///
/// Not the basic ones, and not
/// [`contact::efficient_split`](gear_core::contact::efficient_split)'s either.
///
/// The **sum** of a mesh's two shifts is never free here: the crank offset is
/// what it is, and that is the arrangement's defining constraint. Only the
/// division is left, and it is searched rather than solved — because the
/// unconstrained optimum is not a design. On these stages the mesh loses least
/// at a division of `+2.85`, `+2.05`, `−2.55` for one, two and four teeth of
/// difference, and **none of the three is admissible**: contact has gone
/// discontinuous or the tips have fouled long before. Every row below sits
/// instead on the constraints, at `ε ≈ 1.00` with the tip margin at zero, which
/// is where a bound answers rather than an optimum.
///
/// So the closed form is not the tool for this arrangement. It finds where the
/// loss is stationary, and here the loss is still falling when the geometry runs
/// out — the useful statement is which bound stops it, and that is what these
/// rows are.
fn hula_band(z0: u32, clearance_in_modules: f64) {
    use gear_core::train::{HulaStage, StageLoads};

    let lib = gear_io::default_library();

    println!(
        "hula, reduction {} : 1 — the same ratio at every tooth difference\n",
        z0 * z0
    );
    println!(
        "{:>3} {:>6} {:>7} {:>5} {:>6} {:>7} {:>10} {:>8} {:>8} {:>7} {:>9}",
        "d", "z", "module", "h_a", "shaper", "x", "meshes", "stage", "alpha_w", "eps", "backlash"
    );
    for d in 1..=9u32 {
        let n = z0 * d;
        let module = 1.0 / f64::from(d);
        let teeth = [n + d, n, n - d, n];
        let mut best: Option<(
            f64,
            u32,
            f64,
            gear_core::train::Stage,
            gear_core::train::StageResult,
        )> = None;
        for addendum in [0.8, 0.7, 0.6, 0.5, 0.4] {
            for cutter in [10u32, 14, 20, 28] {
                if cutter + 2 >= n {
                    continue;
                }
                for i in -30..=40 {
                    let x = f64::from(i) * 0.05;
                    let mut stage = HulaStage {
                        module: [module; 2],
                        clearance: clearance_in_modules * module,
                        running_clearance: gear_core::params::Auto::fixed(0.02 * module),
                        tolerance_plus: 0.02 * module,
                        tolerance_minus: 0.02 * module,
                        ..HulaStage::default()
                    };
                    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
                        gear.teeth = count;
                        gear.addendum = addendum;
                    }
                    // **The ring's shift is the one given**, and the pinion's
                    // follows to the crank the tips size — the member the
                    // kind took as given when it was handed both, and what a
                    // shape reads off the toggles: a mesh with both members
                    // given reaches nothing.
                    for mesh in 0..2 {
                        let (a, b) = (mesh * 2, mesh * 2 + 1);
                        let ring = if teeth[a] > teeth[b] { a } else { b };
                        stage.gears[ring].profile_shift = gear_core::params::Auto::fixed(x);
                    }
                    for c in &mut stage.cutter {
                        c.teeth = cutter;
                    }
                    let Ok((as_stage, r)) = solve_hula(&stage, &StageLoads::at(2.0, 1000.0), &lib)
                    else {
                        continue;
                    };
                    // **The whole question**, through the one method that asks
                    // it. This read `tips.is_some_and(|t| t.clear())` and so
                    // asked only about the tips crossing once the other two
                    // conditions moved onto the mesh report — which let fouling
                    // candidates win four of these rows.
                    let admissible =
                        r.meshes().iter().all(|m| {
                            transverse(m).contact_ratios.transverse >= 1.0 && m.teeth_clear()
                        }) && r.members().iter().all(|g| g.as_asked());
                    if !admissible {
                        continue;
                    }
                    if best.as_ref().is_none_or(|(_, _, _, _, b)| {
                        r.efficiency().forward > b.efficiency().forward
                    }) {
                        best = Some((x, cutter, addendum, as_stage, r));
                    }
                }
            }
        }
        match best {
            None => println!("{d:>3} {n:>6} {module:>7.3}   nothing admissible"),
            Some((x, cutter, h, as_stage, r)) => {
                let v = hula_view(&as_stage, &r).expect("a hula stage");
                println!(
                    "{d:>3} {n:>6} {module:>7.3} {h:>5.1} {cutter:>6} {x:>+7.2} {:>9.4}% {:>7.2}% {:>8.2} {:>7.4} {:>9.5}",
                    v.fixed_carrier_efficiency * 100.0,
                    v.efficiency.forward * 100.0,
                    transverse(&v.meshes[0]).operating_pressure_angle,
                    transverse(&v.meshes[0]).contact_ratios.transverse,
                    v.backlash.forward.nominal
                );
            }
        }
    }
}

/// The control: an ordinary internal pair, whose answer is known.
///
/// Standard proportions, no shift, at its own zero-backlash centre distance. Its
/// flanks touch and nothing penetrates, so whatever this reports is the
/// harness's own error and the figure everything else is read against.
fn mesh_sweep(z_ring: u32, z_pinion: u32, ring_addendum: f64, pinion_addendum: f64) {
    use gear_core::ring::{mesh_with, Cutter, Ring};
    let params = |teeth: u32, addendum: f64| GearParams {
        teeth,
        addendum,
        ..GearParams::default()
    };
    let cutter = Cutter {
        teeth: z_ring.saturating_sub(5).max(6),
        ..Cutter::default()
    };
    let ring = Ring::cut_by(&params(z_ring, ring_addendum), &cutter);
    let pinion = gear_core::Gear::new(params(z_pinion, pinion_addendum));
    let Some(m) = mesh_with(&ring, pinion.mean()) else {
        eprintln!("z{z_ring} and z{z_pinion} do not mesh");
        return;
    };
    roll_pair(
        &ring,
        &pinion,
        m.centre_distance,
        &format!(
            "control  ring z{z_ring}  pinion z{z_pinion}   contact ratio {:.4}   alpha_w {:.2} deg   troch {} inv {} tip {} ({:+.4} deg)",
            m.contact_ratio,
            m.alpha_w.to_degrees(),
            m.trochoid_interference,
            m.involute_interference,
            m.tip_interference,
            m.tip_margin.to_degrees()
        ),
    );
}

/// The same roll, on the hula pair the **stage** produces.
///
/// Through the shape rather than a pair built by hand, because the offset
/// answers to the tips as well as to the far-side gap and a harness rolling
/// a pair before that bound was applied would be measuring one nobody builds.
fn hula_sweep(n: u32, clearance: f64, mesh_index: usize) {
    use gear_core::ring::Ring;
    use gear_core::train::{HulaStage, StageLoads};

    let lib = gear_io::default_library();
    let teeth = [n + 1, n, n - 1, n];
    let mut stage = HulaStage {
        clearance,
        ..HulaStage::default()
    };
    for (gear, count) in stage.gears.iter_mut().zip(teeth) {
        gear.teeth = count;
    }
    let (as_stage, solved) = match solve_hula(&stage, &StageLoads::at(2.0, 1000.0), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("that stage has no geometry: {e}");
            return;
        }
    };
    let Some(result) = hula_view(&as_stage, &solved) else {
        eprintln!("that stage is not a hula stage");
        return;
    };
    let (a, b) = (mesh_index * 2, mesh_index * 2 + 1);
    let (ring_i, pinion_i) = if result.gears[a].1 { (a, b) } else { (b, a) };
    let params = |i: usize| GearParams {
        module: stage.module[mesh_index],
        teeth: result.gears[i].0.params.teeth,
        profile_shift: result.gears[i].0.profile_shift,
        addendum: stage.gears[i].addendum,
        dedendum: stage.gears[i].dedendum,
        thickness_mod: stage.thickness_mod[mesh_index],
        ..GearParams::default()
    };
    let ring = Ring::cut_by(&params(ring_i), &stage.cutter[mesh_index]);
    let pinion = gear_core::Gear::new(params(pinion_i));
    let m = &result.meshes[mesh_index];
    roll_pair(
        &ring,
        &pinion,
        result.offset_nominal,
        &format!(
            "hula mesh {}  ring z{} x{:+.4}  pinion z{} x{:+.4}   gap asked {clearance} got {:.4} mm   alpha_w {:.2} deg   tip {} ({:+.4} deg)",
            mesh_index + 1,
            result.gears[ring_i].0.params.teeth,
            result.gears[ring_i].0.profile_shift,
            result.gears[pinion_i].0.params.teeth,
            result.gears[pinion_i].0.profile_shift,
            m.tips.map_or(f64::NAN, |t| t.far_gap),
            transverse(m).operating_pressure_angle,
            m.tips.is_some_and(|t| t.tip_interference),
            m.tips.map_or(0.0, |t| t.tip_margin)
        ),
    );
}

/// A two-stage geartrain, end to end — milestone 6's gate.
/// Write a geartrain out as TOML, read it back, and solve both.
///
/// The point is the *comparison*, not the file: an export that loses a field
/// produces a train that still solves, just to different numbers, and only
/// putting the two answers side by side shows it. With a path, the file is left
/// there to be looked at and hand-edited; without one it is written to a
/// temporary file and removed.
fn train_file_report(path: Option<&str>) {
    use gear_core::params::Auto;
    use gear_core::train::{
        solve_train, Duty, Load, LoadCase, PairStage, PlanetaryStage, Port, Stage, StageGear, Train,
    };
    use gear_io::TrainDocument;

    let lib = gear_io::default_library();
    let doc = TrainDocument {
        name: "Elevation drive".to_string(),
        train: Train {
            // Both case kinds, both ports, both duties: everything the document can
            // carry for a load, so the round trip is asked of all of it.
            load_cases: vec![
                LoadCase::ultimate(2.0, 3000.0),
                LoadCase::back_driving(0.5),
                LoadCase {
                    duty: Duty::Continuous {
                        runtime_hours: 1000.0,
                    },
                    ..LoadCase::fatigue(2.0, 2400.0)
                },
                LoadCase {
                    loads: vec![Load::given(Port::End, 0.2, 30.0)],
                    enabled: false,
                    ..LoadCase::fatigue(0.2, 30.0)
                },
            ],
            reversed_bending: false,
            stages: vec![
                Stage::spur(
                    PairStage {
                        gears: [
                            StageGear {
                                teeth: 17,
                                face_width: Auto::automatic(0.0),
                                ..StageGear::default()
                            },
                            StageGear {
                                teeth: 43,
                                ..StageGear::default()
                            },
                        ],
                        ..PairStage::default()
                    }
                    .with_additional_helix(15.0),
                ),
                Stage::worm(PairStage::worm()),
                Stage::planetary(PlanetaryStage::default()),
            ],
            couplings: Vec::new(),
            constraints: Vec::new(),
        },
    };

    let text = match gear_io::train::to_toml(&doc) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("could not write the geartrain: {e}");
            return;
        }
    };
    let back = match gear_io::train::from_toml(&text) {
        Ok(d) => d.document,
        Err(e) => {
            eprintln!("could not read it back: {e}");
            return;
        }
    };

    let written = path.unwrap_or("/tmp/gear-cli-train.toml");
    if std::fs::write(written, &text).is_ok() {
        println!(
            "wrote {written}  ({} lines, {} bytes)",
            text.lines().count(),
            text.len()
        );
    }
    if path.is_none() {
        let _ = std::fs::remove_file(written);
    }
    println!("name   {:?} -> {:?}", doc.name, back.name);
    println!(
        "stages {} -> {}",
        doc.train.stages.len(),
        back.train.stages.len()
    );

    let (a, b) = (
        solve_train(&doc.train, &lib),
        solve_train(&back.train, &lib),
    );
    match (a, b) {
        (Ok(a), Ok(b)) => {
            println!("\n  quantity                 exported            re-imported   same");
            let end = |r: &gear_core::train::TrainResult, t: &Train| -> (f64, f64) {
                let b = t.boundaries().expect("boundaries");
                r.cases[0]
                    .shaft(t.port_shaft(&b, Port::End))
                    .map_or((0.0, 0.0), |s| (s.speed.unwrap_or(0.0), s.torque))
            };
            let (a_end, b_end) = (end(&a, &doc.train), end(&b, &back.train));
            let rows: [(&str, f64, f64); 5] = [
                ("total ratio", a.total_ratio, b.total_ratio),
                ("output speed rpm", a_end.0, b_end.0),
                ("output torque Nm", a_end.1, b_end.1),
                (
                    "efficiency forward",
                    a.total_efficiency.forward,
                    b.total_efficiency.forward,
                ),
                (
                    "backlash out deg",
                    a.backlash.forward.nominal,
                    b.backlash.forward.nominal,
                ),
            ];
            let mut all = true;
            for (name, x, y) in rows {
                // Bit-identical or not at all: these come from the same
                // arithmetic on numbers that either survived the file or did
                // not. A tolerance here would hide exactly what is being asked.
                let same = x.to_bits() == y.to_bits();
                all &= same;
                println!(
                    "  {name:<22} {x:>16.9} {y:>16.9}   {}",
                    if same { "yes" } else { "NO" }
                );
            }
            println!(
                "\n  {}",
                if all {
                    "the file is the train: every figure identical to the last bit"
                } else {
                    "SOMETHING WAS LOST IN THE FILE - see the rows marked NO"
                }
            );
        }
        (a, b) => println!("a train did not solve: {:?} / {:?}", a.err(), b.err()),
    }
}

/// **What choosing a pair's shifts for efficiency is worth**, and what it costs.
///
/// The body of `docs/reference.md#efficiency-parallel-axes`'s first table, and
/// the only command that drives `auto::shifts_for_efficiency` — so it is also
/// what puts the crate's one search inside the golden corpus.
///
/// The distance sweep is the second half and is the law rather than the table: a
/// pair told to run at the distance the free search chose must come back with
/// the gears the free search chose. It did not, and the audit's record (F52) says what
/// that cost.
fn shifts_report(z1: u32, z2: u32) {
    use gear_core::params::Auto;
    use gear_core::train::{Optimisation, PairKind, PairStage, StageGear, StageLoads};

    let lib = gear_io::default_library();
    let stage = |on: bool, at: Option<f64>| PairStage {
        gears: [z1, z2].map(|teeth| StageGear {
            teeth,
            ..StageGear::default()
        }),
        centre_distance: at.map_or(Auto::automatic(0.0), Auto::fixed),
        optimisation: Optimisation {
            enabled: on,
            ..Optimisation::default()
        },
        ..PairStage::default()
    };
    let solved = |on: bool, at: Option<f64>| {
        solve_pair(&stage(on, at), PairKind::Spur, &StageLoads::just(2.0), &lib)
    };

    println!("pair z {z1}/{z2}  module 1  alpha 20 deg  mu 0.06\n");
    println!(
        "{:<34} {:>9} {:>9} {:>9} {:>9} {:>10}",
        "", "x1", "x2", "sum", "eps", "eta fwd"
    );
    let row = |name: &str, r: &Pair| {
        println!(
            "{name:<34} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>9.3} %",
            r.gears[0].profile_shift,
            r.gears[1].profile_shift,
            r.gears[0].profile_shift + r.gears[1].profile_shift,
            line(r).contact_ratios.transverse,
            100.0 * r.mesh.efficiency.forward
        );
    };
    let (Ok(floor), Ok(best)) = (solved(false, None), solved(true, None)) else {
        eprintln!("that pair has no answer");
        return;
    };
    let (Some(floor), Some(best)) = (pair(&floor), pair(&best)) else {
        eprintln!("that stage is not a pair");
        return;
    };
    row("least shift that clears undercut", &floor);
    row("least loss", &best);
    println!(
        "\n  the trade: {:.2} points of efficiency for {:.2} of contact ratio",
        100.0 * (best.mesh.efficiency.forward - floor.mesh.efficiency.forward),
        line(&floor).contact_ratios.transverse - line(&best).contact_ratios.transverse
    );

    // **The same question asked the other way.** A given centre distance fixes
    // the shift sum and leaves the division; at the distance the free search
    // itself chose, the two must agree.
    println!("\ncentre distance given, shifts chosen to reach it");
    println!(
        "{:<34} {:>9} {:>9} {:>9} {:>9} {:>10}",
        "a mm", "x1", "x2", "sum", "eps", "eta fwd"
    );
    let free_at = best.centre_distance;
    // **What the stage has to say about the distance**, printed under the row it
    // is about. This is how a distance no admissible shifts reach comes to be in
    // the change detector rather than only in a test (F55) — and most rows say
    // nothing, which is the point.
    let said = |r: &Pair| {
        for n in r.notes {
            if n.is(gear_core::note::key::STAGE_CENTRE_DISTANCE_NOT_REACHED)
                || n.is(gear_core::note::key::STAGE_CLEARANCE_NEGATIVE)
            {
                println!("{:>36}{}", "", words().render(n));
            }
        }
    };
    let tight = f64::from(z1 + z2) / 2.0 + floor.clearance;
    for k in 0..=6 {
        let a = tight + (free_at - tight) * f64::from(k) / 6.0;
        match solved(true, Some(a)) {
            Ok(r) => {
                let p = pair(&r).expect("a pair");
                row(&format!("{a:.4}"), &p);
                said(&p);
            }
            Err(e) => println!("{a:.4}: {e}"),
        }
    }

    // **And the same distances with the optimiser off**, which is the plainest
    // thing a designer does and was the one combination no recorded case
    // walked: a distance given by hand with nothing asked to move.
    //
    // It is a different rule, not a different answer to the same one. With the
    // shifts free the stage closes the pair a clearance *inside* the distance
    // given, so the designer gets both the housing and the play; with nothing
    // free there is nothing to absorb it, the shifts stay at their undercut
    // floor, and the clearance is not read at all
    // (`solve_spur_stage`, where the centre distance is settled). It was the
    // one combination no recorded case walked.
    println!("\n...and with the shift optimiser off");
    println!(
        "{:<34} {:>9} {:>9} {:>9} {:>9} {:>10}",
        "a mm", "x1", "x2", "sum", "eps", "eta fwd"
    );
    for k in 0..=3 {
        let a = tight + (free_at - tight) * f64::from(k) / 3.0;
        match solved(false, Some(a)) {
            Ok(r) => {
                let p = pair(&r).expect("a pair");
                row(&format!("{a:.4}"), &p);
                said(&p);
            }
            Err(e) => println!("{a:.4}: {e}"),
        }
    }
}

/// **What the two epicyclic presets choose, and whether their tools leave it.**
///
/// A set and a hula stage each search more than one shift, against a constraint
/// their planet or their crank closes. The figure beside each is the one that
/// went missing: **is the part the search chose the part its tool cuts?** A ring
/// is asked of its cutter rather than of a rack, and until it was asked at all
/// the search walked past the shift where its space stops being the space asked
/// for — 26 of 30 sets swept came back with a ring the cutter had to alter.
fn epicyclic_shifts_report() {
    use gear_core::params::Auto;
    use gear_core::train::{solve_any, HulaStage, Optimisation, PlanetaryStage, Stage, StageLoads};

    let lib = gear_io::default_library();
    let on = Optimisation {
        enabled: true,
        ..Optimisation::default()
    };

    // **The hula stage belongs here too.** It is the third kind that chooses
    // shifts, and the one whose optimiser can find *nothing* to choose — at a
    // one-tooth difference every split is refused, which looks exactly like a
    // search that agreed until the stage says otherwise (`docs/corrections.md`, and
    // the audit's record F58, F82).
    // Without a row here that path is one the change detector cannot see, which
    // this project has now recorded eight times.
    println!("hula stages  z, by tooth difference\n");
    println!(
        "{:<12} {:>9} {:>9} {:>11}   what the optimiser found",
        "d", "x mesh 1", "x mesh 2", "eta0 fwd"
    );
    for d in [1u32, 3, 6] {
        let n = 18 * d;
        let mut hula = gear_core::train::HulaStage {
            module: [1.0 / f64::from(d); 2],
            clearance: 0.30 / f64::from(d),
            ..gear_core::train::HulaStage::default()
        };
        hula.optimisation = on;
        for (g, z) in hula.gears.iter_mut().zip([n, n + d, n + d, n + 2 * d]) {
            g.teeth = z;
        }
        match solve_hula(&hula, &StageLoads::at(2.0, 1000.0), &lib) {
            Err(e) => println!("{d:<12} {e}"),
            Ok((as_stage, r)) => {
                let v = hula_view(&as_stage, &r).expect("a hula stage");
                println!(
                    "{d:<12} {:>9.4} {:>9.4} {:>10.4} %   {}",
                    v.gears[1].0.profile_shift,
                    v.gears[3].0.profile_shift,
                    v.fixed_carrier_efficiency * 100.0,
                    v.notes
                        .iter()
                        .find(|n| n.is(gear_core::note::key::STAGE_OPTIMISER_FOUND_NOTHING))
                        .map_or("a shift to choose", |_| "nothing admissible"),
                );
            }
        }
    }

    println!("\nepicyclic sets  z_sun/z_planet, ring = sun + 2 planet\n");
    println!(
        "{:<12} {:>9} {:>9} {:>9} {:>11} {:>16}",
        "z_s/z_p", "x sun", "x planet", "x ring", "eta0 fwd", "cut as asked"
    );
    for (sun, planet) in [(11u32, 18u32), (13, 25), (17, 17), (24, 18), (31, 21)] {
        let mut set = PlanetaryStage {
            optimisation: on,
            ..PlanetaryStage::default()
        };
        set.sun.teeth = sun;
        set.planet.teeth = planet;
        set.ring.teeth = sun + 2 * planet;
        set.sun.profile_shift = Auto::automatic(0.0);
        set.ring.profile_shift = Auto::automatic(0.0);
        match solve_any(&Stage::planetary(set), &StageLoads::just(2.0), &lib) {
            Ok(r) => {
                let members = r.members();
                let meshes = r.meshes();
                println!(
                    "{:<12} {:>9.4} {:>9.4} {:>9.4} {:>10.4} % {:>16}",
                    format!("{sun}/{planet}"),
                    members[0].profile_shift,
                    members[1].profile_shift,
                    members[2].profile_shift,
                    // `η₀`: the two meshes in series, which is what the search
                    // maximises.
                    100.0 * meshes[0].efficiency.forward * meshes[1].efficiency.forward,
                    // A clamp on any member is the tool declining to make what was
                    // asked for, which is what the search may not choose.
                    members.iter().all(|g| g.clamps.is_empty())
                );
            }
            Err(e) => println!("{sun}/{planet}: {e}"),
        }
    }

    println!("\nhula stages  N teeth of difference\n");
    println!(
        "{:<12} {:>9} {:>9} {:>11} {:>16}",
        "N", "x mesh 1", "x mesh 2", "eta fwd", "cut as asked"
    );
    for n in [12u32, 18, 30] {
        let mut stage = HulaStage {
            optimisation: on,
            ..HulaStage::default()
        };
        for (gear, count) in stage.gears.iter_mut().zip([n + 1, n, n - 1, n]) {
            gear.teeth = count;
        }
        match solve_hula(&stage, &StageLoads::at(2.0, 1000.0), &lib) {
            Ok((_, r)) => {
                let members = r.members();
                println!(
                    "{:<12} {:>9.4} {:>9.4} {:>10.4} % {:>16}",
                    n,
                    members[1].profile_shift,
                    members[2].profile_shift,
                    100.0 * r.efficiency().forward,
                    members.iter().all(|g| g.clamps.is_empty())
                );
            }
            Err(e) => println!("{n}: {e}"),
        }
    }
}

fn train_report(mode: Option<&str>) {
    use gear_core::params::Auto;
    use gear_core::train::{solve_train, Duty, LoadCase, PairStage, Port, Stage, StageGear, Train};

    let lib = gear_io::default_library();
    let auto_width = |teeth: u32| StageGear {
        teeth,
        face_width: Auto::automatic(0.0),
        ..StageGear::default()
    };
    let train = Train {
        // **Three trains, because a load from the far end has three regimes**
        // and the corpus has to walk all of them. `train` reacts none of it;
        // `train mixed` reacts a load the drive still outweighs; `train held` is
        // the worm holding more than it is driving, which is what a self-locking
        // worm is chosen to do and is the only regime in which the case from the
        // end is the larger one.
        //
        // Every train this harness shipped set this to zero, so
        // `tools/check_golden.sh` recorded a path nothing ever walked: a
        // self-locking worm's wheel reported 2.2e307 N·m and an epicyclic set's
        // ring 6 % low, and the corpus could not have shown either. Then the
        // load it did walk was one the drive outweighed, so the corpus still
        // could not show a stage rated at `η_forward` of what it was holding
        // (`docs/corrections.md`).
        //
        // **`toggles` reverses the duty**, which is the switch that lets a
        // reversed root reach a member at all — and holds a load from the end
        // at the far port, which is the other thing a case can be asked.
        load_cases: vec![
            LoadCase::ultimate(2.0, 3000.0),
            {
                let torque = match mode {
                    Some("held") => 400.0,
                    Some("mixed") => 0.6,
                    Some("toggles") => 5.0,
                    _ => 0.0,
                };
                if mode == Some("toggles") {
                    LoadCase::back_driving_held(torque)
                } else {
                    LoadCase::back_driving(torque)
                }
            },
            LoadCase {
                duty: if mode == Some("toggles") {
                    Duty::Intermittent {
                        range_degrees: 90.0,
                        at: Port::End,
                        actuations: 600_000,
                        reversing: true,
                    }
                } else {
                    Duty::Continuous {
                        runtime_hours: 1000.0,
                    }
                },
                ..LoadCase::fatigue(
                    2.0,
                    if mode == Some("toggles") {
                        3000.0
                    } else {
                        2400.0
                    },
                )
            },
        ],
        // **Every optional control, engaged.** The corpus turned three of a
        // gear's eleven and left the rest at their defaults, so the constants
        // behind them were outside the change detector: perturbing
        // `REVERSED_BENDING_FRACTION` or the load-sharing ramp moved **no
        // recorded output and no test**. That is the fault
        // `docs/corrections.md` records of a back-driving load and of the
        // optimiser, met a third time — *an opt-in the harness never switches on
        // is a path the detector cannot see.*
        //
        // The helix is 30° so the **virtual** contact ratio passes 2, which is
        // where the sharing ramp's own constants finally reach an answer: below
        // it the governing point is the single-pair boundary, where the share is
        // exactly one and the ramp is decorative.
        reversed_bending: mode == Some("toggles"),
        stages: if mode == Some("toggles") {
            let toggled = |teeth: u32, undercut: bool, sharp: bool| StageGear {
                teeth,
                no_undercut: undercut,
                no_sharp_tip: sharp,
                rim_thickness: Some(3.0),
                material_overrides: gear_core::material::Overrides {
                    fatigue_allowable: Some(420.0),
                    ..gear_core::material::Overrides::default()
                },
                face_width: Auto::automatic(0.0),
                ..StageGear::default()
            };
            vec![
                Stage::spur(
                    PairStage {
                        load_sharing: gear_core::contact::LoadSharing::LinearRamp,
                        gears: [toggled(17, false, true), toggled(43, true, false)],
                        ..PairStage::default()
                    }
                    .with_additional_helix(30.0),
                ),
                Stage::spur(PairStage {
                    load_sharing: gear_core::contact::LoadSharing::LinearRamp,
                    gears: [toggled(13, true, false), toggled(31, false, true)],
                    ..PairStage::default()
                }),
            ]
        } else if matches!(mode, Some("mixed" | "held")) {
            vec![
                Stage::spur(PairStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..PairStage::default()
                }),
                Stage::worm(PairStage::worm()),
            ]
        } else {
            vec![
                Stage::spur(PairStage {
                    gears: [auto_width(17), auto_width(43)],
                    ..PairStage::default()
                }),
                Stage::spur(
                    PairStage {
                        gears: [auto_width(13), auto_width(31)],
                        ..PairStage::default()
                    }
                    .with_additional_helix(15.0),
                ),
            ]
        },
        couplings: Vec::new(),
        constraints: Vec::new(),
    };

    let r = match solve_train(&train, &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("train did not solve: {e}");
            return;
        }
    };

    println!(
        "train  total ratio {:.4}:1   total efficiency {}",
        r.total_ratio,
        both_ways(r.total_efficiency)
    );
    println!(
        "       backlash at the output shaft  {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.forward.nominal, r.backlash.forward.minimum, r.backlash.forward.maximum
    );
    println!(
        "       backlash at the input shaft   {:.5} deg  (min {:.5}, max {:.5})",
        r.backlash.backward.nominal, r.backlash.backward.minimum, r.backlash.backward.maximum
    );
    print_train_cases(&train, &r);

    for (k, s) in r.stages.iter().enumerate() {
        // One report, and which contact it is decides the rows — the same
        // rows for every line contact and for every point.
        match pair(s) {
            Some(res) => match res.mesh.line {
                Some(line) => print_line_pair(k, kind_name(&train.stages[k]), &res, &line),
                None => print_point_pair(k, kind_name(&train.stages[k]), &res, res.mesh),
            },
            None => println!("\nstage {}: not a pair", k + 1),
        }
    }
}

/// Every load case of a train: what was applied where, what arrived at the far
/// end, and what the shaft line had to say about it.
fn print_train_cases(train: &gear_core::train::Train, r: &gear_core::train::TrainResult) {
    use gear_core::train::{CaseKind, Duty};
    let port = kinematics::port;
    for c in &r.cases {
        let input = &train.load_cases[c.case];
        let duty = match (input.kind, input.duty) {
            (CaseKind::Ultimate, _) => String::new(),
            (
                CaseKind::Fatigue,
                Duty::Intermittent {
                    range_degrees,
                    at,
                    actuations,
                    reversing,
                },
            ) => format!(
                "   {range_degrees} deg at {} x {actuations}{}",
                port(at),
                if reversing { ", reversing" } else { "" }
            ),
            (CaseKind::Fatigue, Duty::Continuous { runtime_hours }) => {
                format!("   {runtime_hours} h")
            }
        };
        println!(
            "       case {}  {:<8} {}{duty}{}",
            c.case + 1,
            match input.kind {
                CaseKind::Ultimate => "ultimate",
                CaseKind::Fatigue => "fatigue",
            },
            input
                .loads
                .iter()
                .map(|l| format!(
                    "{}{:.3} Nm / {}{:.0} rpm at {}",
                    if l.torque.auto { "~" } else { "" },
                    l.torque.manual,
                    if l.speed.auto { "~" } else { "" },
                    l.speed.manual,
                    port(l.at)
                ))
                .collect::<Vec<_>>()
                .join(" + "),
            if c.solved { "" } else { "   (not solved)" }
        );
        for n in &c.notes {
            println!("              note: {}", words().render(n));
        }
        // Every shaft: what it is in this case and what it carries.
        for s in &c.shafts {
            println!(
                "              {:<8} {:<24} {:>12}  {:>12.4} Nm",
                format!("{:?}", s.role).to_lowercase(),
                kinematics::named(&train.stages, s.at, s.label),
                s.speed
                    .map_or_else(|| "-".to_string(), |v| format!("{v:.2} rpm")),
                s.torque
            );
        }
    }
}

/// What every load case does to one member, one row per case — the same rows
/// for a line contact's members and a point's, which report no bending.
fn print_gear_cases(cases: &[gear_core::train::GearCase]) {
    for c in cases {
        println!(
            "      case {}  T {:>10.4} Nm  {:>9.1} rpm  sigma_F {:>8}  sigma_H {:>8.1} MPa  cycles {}",
            c.case + 1,
            c.torque,
            c.speed,
            c.bending_stress
                .map_or_else(|| "-".to_string(), |s| format!("{s:.1}")),
            c.contact_stress,
            c.cycles.map_or_else(
                || "-".to_string(),
                |n| format!("{:.3e} / {:.3e}", n.bending, n.contact)
            ),
        );
    }
}

/// A pair with its shafts parallel: line contact, a bending rating.
fn print_line_pair(k: usize, kind: &str, s: &Pair, line: &gear_core::train::LineContact) {
    let mesh = s.mesh;
    let helix = s.gears[0].helix_angle;
    let ratios = &line.contact_ratios;
    println!(
        "\nstage {}  {}  z {}/{}  beta {} deg  ratio {:.4}  a_w {:.4} mm{}",
        k + 1,
        kind,
        s.gears[0].params.teeth,
        s.gears[1].params.teeth,
        helix,
        s.ratio,
        s.centre_distance,
        if mesh.coprime { "  coprime" } else { "" }
    );
    println!(
        "  contact ratio  transverse {:.4}   overlap {:.4}   total {:.4}{}",
        ratios.transverse,
        ratios.overlap,
        ratios.total,
        if helix != 0.0 && !ratios.has_full_axial_overlap() {
            "   <- no full axial overlap"
        } else {
            ""
        }
    );
    println!(
        "  efficiency {:.3} % forward / {:.3} % backward",
        100.0 * mesh.efficiency.forward,
        100.0 * mesh.efficiency.backward
    );
    // One pressure, printed once per case. The pair shares a patch, a normal
    // force and an `E*`, so there is no second number to print per gear — what
    // a gear has of its own is the allowable, and therefore `b_min`.
    println!(
        "  contact at the pitch point  sigma_H {} MPa by case   rho {:.3} mm",
        mesh.cases
            .iter()
            .map(|c| format!("{:.1}", c.contact.at_pitch_point))
            .collect::<Vec<_>>()
            .join(" / "),
        1.0 / mesh.cases[0].contact.curvature_across
    );
    println!("  {:<6} {:>8} {:>8}", "gear", "x", "b mm");
    for (i, g) in s.gears.iter().enumerate() {
        println!(
            "  {:<6} {:>8.4} {:>8.3}",
            i + 1,
            g.profile_shift,
            g.face_width
        );
        print_gear_cases(&g.cases);
    }
    for n in s.mesh.notes.iter().chain(s.notes) {
        println!("  note: {}", words().render(n));
    }
}

/// The pair's kind, as a designer names it — the word the harness prints for
/// a pair: what the shape's one distance
/// says it is sized as.
fn kind_name(stage: &gear_core::train::Stage) -> &'static str {
    match stage.as_shape().and_then(|s| s.distances.first()) {
        Some(d) if d.worm => "worm",
        _ => "spur",
    }
}

/// A pair with its shafts crossed: point contact, two efficiencies.
fn print_point_pair(k: usize, kind: &str, s: &Pair, m: &gear_core::train::MeshReport) {
    println!(
        "\nstage {}  {}  z {}/{}  ratio {:.4}  a {:.4} mm  lead angle {:.4} deg",
        k + 1,
        kind,
        s.gears[0].params.teeth,
        s.gears[1].params.teeth,
        s.ratio,
        s.centre_distance,
        s.gears[0].lead_angle
    );
    println!("  efficiency  {}", both_ways(m.efficiency));
    println!(
        "  contact  {} MPa by case   patch {:.4} x {:.4} mm   sliding {} mm/s by case",
        m.cases
            .iter()
            .map(|c| format!("{:.1}", c.contact.max_pressure))
            .collect::<Vec<_>>()
            .join(" / "),
        m.cases[0].contact.patch_length,
        m.cases[0].contact.patch_width,
        m.cases
            .iter()
            .map(|c| format!("{:.1}", c.sliding_velocity))
            .collect::<Vec<_>>()
            .join(" / "),
    );
    println!("  {:<6} {:>8}   material", "member", "b mm");
    let names = match kind {
        "worm" => ["worm", "wheel"],
        _ => ["1", "2"],
    };
    for (name, g) in names.iter().zip(&s.gears) {
        println!("  {name:<6} {:>8.3}   {}", g.face_width, g.material.name);
        print_gear_cases(&g.cases);
    }
    println!("  bending not reported, flank type ZI - see docs/reference.md#crossed-axes");
    for n in s.mesh.notes.iter().chain(s.notes) {
        println!("  note: {}", words().render(n));
    }
}

/// A worked mesh end to end: load, bending, contact, efficiency, face width.
///
/// This is the whole of milestone 5 in one view, and the first thing that
/// actually consumes the material library. Both gears are rated, because the
/// pinion is not automatically the worse one — it sees the higher contact
/// stress but the wheel may have the weaker root.
fn strength_report(
    z1: u32,
    z2: u32,
    torque: f64,
    material_name: &str,
    helix: f64,
    rim: Option<f64>,
) {
    use gear_core::contact::{efficiency, ContactPath, Drive};
    use gear_core::material::contact_modulus;
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::metrology::base_helix_angle;
    use gear_core::strength::{
        bending_section, bending_stress, contact_stress, min_face_width_bending,
        min_face_width_contact, Load, RimSupport, RootStressModel, PARALLEL_AXES,
    };

    let lib = gear_io::default_library();
    let Some(mat) = lib.get(material_name) else {
        eprintln!("no material named {material_name:?}; try `gear-cli materials`");
        return;
    };

    // Meshing helical gears have equal and opposite hands.
    let g1 = Tooth::new(GearParams {
        teeth: z1,
        helix_angle: helix,
        ..Default::default()
    });
    let g2 = Tooth::new(GearParams {
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
        "  {:<6} {:>8} {:>8} {:>8} {:>9} {:>10} {:>10}",
        "gear", "Y_F", "-axial", "K_f", "sigma_F", "b_min fat", "b_min ult"
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
        let ys = sec.stress_correction(RootStressModel::DolanBroghamer);
        // `Y_B` only where a rim was named on the command line: a rim nobody
        // described rates at 1 and is not the same claim as a thick one.
        let rim_support = rim.map(|s| RimSupport::external(s, g.ra - g.rf));
        let Some(sf) = bending_stress(
            &sec,
            load_g.tangential(g),
            load_g.face_width,
            RootStressModel::DolanBroghamer,
            rim_support,
        ) else {
            println!(
                "  {label:<6} {:>8.4} {:>8.4} {:>8} - notch factor undefined",
                sec.form_factor, sec.axial_compression, "-"
            );
            continue;
        };
        println!(
            "  {label:<6} {:>8.4} {:>8.4} {:>8.4} {:>7.1} MPa {:>8.3} mm {:>8.3} mm",
            sec.form_factor,
            sec.axial_compression,
            ys.unwrap_or(1.0),
            sf,
            min_face_width_bending(sf, B, mat.fatigue_allowable.value),
            min_face_width_bending(sf, B, mat.ultimate_allowable.value),
        );
        // `Y_B`, printed only where a rim was named: on a gear with none it is
        // exactly 1, and a column of ones is noise.
        if let Some(r) = rim_support {
            println!("         Y_B {:.4} (s_R/h_t {:.3})", r.factor(), r.ratio());
            if !r.in_range() {
                println!(
                    "         note: ISO 6336-3 says a backup ratio at or below 0.5 shall be avoided"
                );
            }
        }
    }

    // --- contact. One pressure at any instant, two ratings: each gear is judged
    // where its own dedendum carries the load alone.
    let e_star = contact_modulus(mat, mat);
    if let Some(cs) = contact_stress(&path, &mesh, &g1, PARALLEL_AXES, &load, e_star) {
        println!("\ncontact   E* {e_star:.0} MPa (like on like)");
        println!("  at the pitch point        {:>7.1} MPa", cs.at_pitch_point);
        for i in 0..2 {
            println!(
                "  gear {} single-pair        {:>7.1} MPa   -> rated at {:>7.1} MPa",
                i + 1,
                cs.at_single_pair[i],
                cs.governing(i)
            );
        }
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
    let g = Tooth::new(p);
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
            println!("    - {}", words().render(n));
        }
    }
    let pts = gear_core::gear::Gear::new(g.params).profile(400);
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
                    // **Every axis a gear has**, not the four this turned. It
                    // swept `z`, `x`, `α` and `β` and left the rest at their
                    // defaults — so the eccentric feature, the tool's round and
                    // the thickness modification were outside the change
                    // detector entirely, on a command whose whole job is to be
                    // the parameter grid. That is `tests/common/mod.rs`'s own
                    // finding — *an axis nobody turns is an axis nobody tests* —
                    // met in the harness rather than in the tests.
                    //
                    // Two values an axis rather than a range: this counts
                    // clamps, and what it is for is that every combination is
                    // reached, not that any one of them is resolved.
                    for dx in [0.0_f64, 0.15] {
                        for offset in [0.0_f64, 0.25] {
                            for k in [1.0_f64, 0.9] {
                                for rho in [0.38_f64, 0.0] {
                                    let g = Tooth::new(GearParams {
                                        teeth: z,
                                        profile_shift: f64::from(xi) * 0.1,
                                        pressure_angle: alpha,
                                        helix_angle: beta,
                                        angular_shift: dx,
                                        index_offset: offset,
                                        thickness_mod: k,
                                        root_radius: rho,
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
                        let g = Tooth::new(GearParams {
                            module: 1.0,
                            pressure_angle: an,
                            teeth: z,
                            profile_shift: x,
                            angular_shift: 0.0,
                            index_offset: 0.0,
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
                        let g = Tooth::new(p);
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
    let g = Tooth::new(GearParams {
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
    // **Both kinds of member, through one harness.** A ring is the same four
    // studies rather than a second set beside them, which is the only way the
    // two answers can be compared at all — and comparing them is the point,
    // because the crate rates a ring by the construction it chose for an
    // external tooth.
    for member in [matrix::Member::External, matrix::Member::Internal] {
        let pop = matrix::population_for(member);
        println!(
            "\n################ {} ({} designs, tangent at {:.0}°) ################\n",
            member.name().to_uppercase(),
            pop.len(),
            member.tangent_angle()
        );

        println!("== 1. continuity ==");
        println!(
            "{:<24} {:>18} {:>10} {:>22}",
            "model", "worst z step", "at z", "worst x step (fine)"
        );
        let fine_at = match member {
            matrix::Member::External => 120,
            matrix::Member::Internal => 60,
        };
        for m in matrix::MATRIX {
            let (step, at) = matrix::continuity_in_tooth_count(m, member);
            let xs = matrix::continuity_in_profile_shift(m, member, fine_at);
            println!(
                "{:<24} {:>17.4}% {:>10} {:>21.5}%",
                m.name(member),
                100.0 * step,
                at,
                100.0 * xs
            );
        }

        println!("\n== 2. rank agreement (Spearman) ==");
        for i in 0..matrix::MATRIX.len() {
            for j in (i + 1)..matrix::MATRIX.len() {
                let (r, n) =
                    matrix::rank_correlation(matrix::MATRIX[i], matrix::MATRIX[j], member, &pop);
                println!(
                    "  {:<22} vs {:<22} rho = {r:.6}  (n={n})",
                    matrix::MATRIX[i].name(member),
                    matrix::MATRIX[j].name(member)
                );
            }
        }

        for thresh in [0.0_f64, 0.01] {
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
                    let g = matrix::gradient_agreement(
                        matrix::MATRIX[i],
                        matrix::MATRIX[j],
                        member,
                        &pop,
                        thresh,
                    );
                    println!(
                        "  {:<22} vs {:<22} {:6.1}% {:6.1}% {:6.1}%",
                        matrix::MATRIX[i].name(member),
                        matrix::MATRIX[j].name(member),
                        100.0 * g[0],
                        100.0 * g[1],
                        100.0 * g[2]
                    );
                }
            }
        }

        println!("\n== 4. divergence across the matrix, by tooth count (x=0) ==");
        println!("{:>6} {:>12}", "z", "spread");
        let counts: &[u32] = match member {
            matrix::Member::External => &[9, 12, 17, 25, 40, 70, 120, 250],
            matrix::Member::Internal => &[40, 48, 60, 72, 90, 120, 160, 220],
        };
        for &teeth in counts {
            let p = GearParams {
                teeth,
                ..Default::default()
            };
            if let Some(d) = matrix::divergence(member, p) {
                println!("{teeth:>6} {:>11.2}%", 100.0 * d);
            }
        }
        let mut worst = (0.0_f64, GearParams::default());
        for p in &pop {
            if let Some(d) = matrix::divergence(member, *p) {
                if d > worst.0 {
                    worst = (d, *p);
                }
            }
        }
        println!(
            "   worst over the population: {:.2}% at z={} x={:+.1} alpha={}",
            100.0 * worst.0,
            worst.1.teeth,
            worst.1.profile_shift,
            worst.1.pressure_angle
        );

        // **5. Where the default parts from the standard's construction.** The
        // crate rates on the inscribed parabola; ISO specifies the tangent. This
        // is the size of that choice, on the number a designer actually feels.
        println!(
            "\n== 5. parabola against the {:.0}° tangent ==",
            member.tangent_angle()
        );
        let d = matrix::parting(member, &pop);
        #[allow(clippy::cast_precision_loss)]
        let pct = |k: usize| 100.0 * k as f64 / d.n as f64;
        println!("  designs rated by both        {}", d.n);
        println!(
            "  parabola tangency on flank   {} ({:.1}%)",
            d.on_flank,
            pct(d.on_flank)
        );
        // **The spread is printed rather than left to be subtracted.**
        // `docs/state.md` quotes it as evidence for which bending model is the
        // default, and quoted it by taking the difference of the two ends by
        // hand — so it was a figure this harness did not print, in a table this
        // harness is named as the source of. A number a document derives from
        // output is a number that drifts on its own schedule.
        println!(
            "  Y_F   parabola/tangent       {:.3} .. {:.3}   mean {:.3}   spread {:.3}",
            d.form[0],
            d.form[1],
            d.form[2],
            d.form[1] - d.form[0]
        );
        println!(
            "  Y_F·K_f / Y_F·Y_S            {:.3} .. {:.3}   mean {:.3}   spread {:.3}",
            d.factor[0],
            d.factor[1],
            d.factor[2],
            d.factor[1] - d.factor[0]
        );
        println!(
            "  mean q_s                     {:.3} parabola, {:.3} tangent",
            d.notch[0], d.notch[1]
        );
        println!(
            "  q_s outside Y_S's band       {} parabola ({:.1}%), {} tangent ({:.1}%)",
            d.notch_out[0],
            pct(d.notch_out[0]),
            d.notch_out[1],
            pct(d.notch_out[1])
        );
    }

    // Which end of a ring's tooth is the thick one — the question the cantilever
    // picture rests on, and one worth measuring rather than asserting.
    println!("\n== 6. a ring tooth along its generated flank (tip -> root) ==");
    for teeth in [40u32, 90, 160] {
        let t = matrix::ring_flank_thickness(
            GearParams {
                teeth,
                ..Default::default()
            },
            4,
        );
        let cells: Vec<String> = t
            .iter()
            .map(|(r, w)| format!("r {r:.3} w {w:.4}"))
            .collect();
        println!("  z={teeth:<4} {}", cells.join("   "));
    }

    // **The fillet radius the notch factor is fed.** The crate reads it at the
    // junction when the parabola tangency is on the flank; the source the ring
    // model comes from defines it as the minimum over the whole fillet. `q_s`
    // is linear in it, so the gap between the two is the gap in `q_s`.
    println!("\n== 7. fillet radius: at the junction vs the fillet's minimum ==");
    println!(
        "{:<9} {:>5} {:>7} {:>12} {:>10} {:>9} {:>9}",
        "member", "z", "x", "at junction", "minimum", "ratio", "min at"
    );
    for member in [matrix::Member::External, matrix::Member::Internal] {
        let counts: &[u32] = match member {
            matrix::Member::External => &[17, 40, 100],
            matrix::Member::Internal => &[40, 90, 160],
        };
        for &teeth in counts {
            for shift in [-0.3_f64, 0.0, 0.3] {
                let p = GearParams {
                    teeth,
                    profile_shift: shift,
                    ..Default::default()
                };
                if let Some((j, m, at)) = matrix::fillet_radius_readings(member, p, 2000) {
                    println!(
                        "{:<9} {teeth:>5} {shift:>+7.1} {j:>12.4} {m:>10.4} {:>9.3} {:>9.2}",
                        member.name(),
                        j / m,
                        at
                    );
                }
            }
        }
    }
}

/// Compare the three load cases on a few ordinary meshes.
///
/// (a) worst case  -- load at the tip, this tooth carrying everything
/// (b) HPSTC       -- load at the highest point of single-pair contact
/// (c) shared      -- worst point of the mesh cycle with load sharing applied
fn loadcase_report() {
    use gear_core::contact::{ContactPath, LoadSharing};
    use gear_core::mesh::{Mesh, MeshKind};
    use gear_core::strength::{root_section, RootStressModel};

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
        let (g1, g2) = (Tooth::new(p1), Tooth::new(p2));
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
            root_section(&g1, roll).and_then(|s| s.bending_factor(RootStressModel::DolanBroghamer))
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
    use gear_core::mesh::MeshSide;
    use gear_core::screw::{Screw, ScrewParams};

    let params = ScrewParams {
        starts,
        wheel_teeth,
        worm_pitch_diameter: worm_diameter,
        shaft_angle_rad: shaft_angle_deg.to_radians(),
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
        params.normal_pressure_angle_rad.to_degrees(),
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
        s.lead_angle_rad.to_degrees(),
        s.wheel_lead_angle_rad.to_degrees()
    );
    println!(
        "  helix angle         worm {:8.4} deg    wheel {:8.4} deg",
        s.worm_helix_angle_rad.to_degrees(),
        s.wheel_helix_angle_rad.to_degrees()
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
    // **Both columns can lock**, and the column that reads `locked` is the
    // finding rather than the formatting: this table used to test only the wheel
    // and print a bare `0.000 %` where the worm could not drive, which is what
    // `gear-cli crossed 17 23 90` does at a 9°/81° split.
    for mu in [0.0, 0.02, 0.04, 0.06, 0.10] {
        let e = Directional::of(|d| s.efficiency(mu, d));
        let locked = e.locked();
        let column = |v: f64, locked: bool| {
            if locked {
                "      locked".to_string()
            } else {
                format!("{:11.3} %", v * 100.0)
            }
        };
        println!(
            "  mu {mu:.2}       {} {}",
            column(e.forward, locked.forward),
            column(e.backward, locked.backward)
        );
    }
    // Both thresholds, since both directions have one. Forwards it is usually
    // absurd or negative — a worm you can turn — and saying so is the point:
    // the figure a designer checks is the one for the direction they care about,
    // and quoting only the backward one made that choice for them.
    let threshold = s.locking_friction();
    let at = |v: f64| {
        if v > 0.0 {
            format!("mu >= {v:.4}")
        } else {
            "no friction locks it this way".to_string()
        }
    };
    println!("  locks driving forward at {}", at(threshold.forward));
    println!(
        "  locks back-driving at    {}   (cos alpha_n tan gamma)",
        at(threshold.backward)
    );

    // **A worm sized to reach a given centre distance** — F39's fourth item.
    // A screw stage has no profile shift, so its size is the only thing inside
    // it free to absorb one, and the distance has a *minimum* in that size, so
    // a target above it is reached by two different worms. The branch is the one
    // the designer's own number is on, which is what makes the answers below
    // move smoothly instead of jumping.
    {
        let least =
            Screw::least_distance_lead_angle(starts, wheel_teeth, shaft_angle_deg.to_radians());
        println!();
        if let Some(least) = least {
            println!(
                "  centre distance is least at lead angle {:.4} deg  (d1 {:.4} mm) \
                 — above it two worms reach the same distance",
                least.to_degrees(),
                f64::from(starts.max(1)) / least.sin()
            );
        }
        // **The wheel's shift absorbs a distance before the worm's size does.**
        // At the preset the worm's shift is pinned and the wheel's is free, so
        // a given distance moves the wheel's shift by the rack law; pin the
        // wheel's too, and the size is what is left.
        let base = gear_core::train::PairStage {
            shaft_angle: shaft_angle_deg,
            ..worm_stage(starts, wheel_teeth, worm_diameter)
        };
        // The screw geometry as the shape settles it — the one chooser.
        let geometry = |stage: &gear_core::train::PairStage| {
            gear_core::train::shape::Shape::from(stage).screw(0)
        };
        if let Ok(g0) = geometry(&base) {
            println!("  a mm given, the wheel's shift absorbs it");
            for step in 0..3 {
                let target = g0.centre_distance + 0.5 * f64::from(step);
                let mut stage = base.clone();
                stage.centre_distance =
                    gear_core::params::Auto::fixed(target + stage.clearance.manual);
                match geometry(&stage) {
                    Err(e) => println!("  {target:9.4}  {e:?}"),
                    Ok(s) => println!(
                        "  {target:9.4}  x2 {:+8.4}   d1 {:8.4} mm   ran at {:9.4}",
                        s.shift_sum, s.worm_pitch_diameter, s.centre_distance
                    ),
                }
            }
            println!("  a mm given, both shifts pinned, worm sized to reach it");
            let base = {
                let mut pinned = base.clone();
                pinned.gears[1].profile_shift = gear_core::params::Auto::fixed(0.0);
                pinned
            };
            for step in 0..5 {
                let target = g0.centre_distance + 0.5 * f64::from(step);
                let mut stage = base.clone().size_free();
                stage.centre_distance =
                    gear_core::params::Auto::fixed(target + stage.clearance.manual);
                match geometry(&stage) {
                    Err(e) => println!("  {target:9.4}  {e:?}"),
                    Ok(s) => println!(
                        "  {target:9.4}  d1 {:8.4} mm   lead angle {:7.4} deg   ran at {:9.4}",
                        s.worm_pitch_diameter,
                        s.lead_angle_rad.to_degrees(),
                        s.centre_distance
                    ),
                }
            }
        }
    }

    // Contact is the strength figure a worm stage reports. There is deliberately
    // no bending stress here; docs/reference.md#crossed-axes says why.
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
    if let Some(c) = s.contact(torque_out, MeshSide::Second, mu, Drive::Forward, e_star) {
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
    println!("  bending                     not reported - see docs/reference.md#crossed-axes");
    println!(
        "  flank type   ZI (involute helicoid). A ZN worm's contact stress is\n\
         {:24}1-15 % lower, rising with lead angle - see docs/reference.md#crossed-axes",
        ""
    );
}

/// A worm stage end to end: geometry, both directions, contact and backlash.
fn worm_stage_report(starts: u32, wheel_teeth: u32, worm_diameter: f64, torque: f64) {
    use gear_core::train::{PairKind, StageLoads};

    let stage = worm_stage(starts, wheel_teeth, worm_diameter);
    let lib = gear_io::default_library();
    let solved = match solve_pair(&stage, PairKind::Worm, &StageLoads::just(torque), &lib) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("cannot solve that stage: {e}");
            return;
        }
    };
    let r = pair(&solved).expect("a worm stage is a pair");

    println!(
        "worm stage  z {starts}/{wheel_teeth}  module {}  ratio {:.4}:1  a {:.4} mm",
        stage.module, r.ratio, r.centre_distance
    );
    let m = point(&r);
    println!(
        "  lead angle {:.4} deg   wheel helix {:.4} deg   lead {:.4} mm",
        r.gears[0].lead_angle,
        r.gears[1].helix_angle,
        r.gears[0].lead.unwrap_or(f64::INFINITY)
    );
    println!();
    println!("  member      torque Nm   face mm   d mm      material");
    for (name, m) in ["worm", "wheel"].iter().zip(&r.gears) {
        println!(
            "  {name:<10} {:9.4} {:9.3} {:9.4}   {}",
            m.cases[0].torque, m.face_width, m.pitch_diameter, m.material.name
        );
    }
    println!();
    println!("  efficiency   {}", both_ways(m.efficiency));
    println!(
        "  contact      {:.1} MPa   patch {:.4} x {:.4} mm",
        m.cases[0].contact.max_pressure,
        m.cases[0].contact.patch_length,
        m.cases[0].contact.patch_width
    );
    println!(
        "  backlash     at the wheel {:.5} deg (min {:.5}, max {:.5})   at the worm {:.5} deg",
        m.backlash[1].nominal, m.backlash[1].minimum, m.backlash[1].maximum, m.backlash[0].nominal
    );
    println!("  bending      not reported - see docs/reference.md#crossed-axes");
    println!(
        "  flank type   ZI (involute helicoid); a ZN worm's contact stress is\n\
         {:15}1-15 % lower, rising with lead angle - see docs/reference.md#crossed-axes",
        ""
    );
    for note in m.notes.iter().chain(r.notes) {
        println!("  ! {}", words().render(note));
    }
}

/// The ring tooth counts a sun and planet pair admits, and what each costs in
/// planet shift.
///
/// Prints every candidate rather than picking one, because the choice is a
/// designer's: the geometric ideal needs no shift but rarely spaces the planets
/// evenly, and the one that does costs a shift. Both facts are on the same row.
///
/// Through the shape, one ring count at a time — the planet closing the set
/// as it closes any automatic distance — rather than through a ring search
/// of its own: the sweep is the same walk with the same closure, and the
/// row a count does not admit says why in the stage's own words.
fn planetary_report(sun: u32, planet: u32, planets: u32, sun_shift: f64, ring_shift: f64) {
    use gear_core::params::Auto;
    use gear_core::train::{solve_any, PlanetaryStage, Stage, StageLoads, TrainError};

    let lib = gear_io::default_library();
    let module = 1.0;
    let ideal = sun + 2 * planet;
    println!(
        "planetary  z_sun {sun}  z_planet {planet}  N {planets}  \
         x_sun {sun_shift}  x_ring {ring_shift}  module {module}  alpha 20 deg"
    );
    println!("ideal ring (needs no planet shift): {ideal}");

    // Every count the closure admits, from a ring too small to hold the
    // planets up to four times the ideal — the "what is possible" listing.
    // The counts a set can reach are asked at zero backlash; a running
    // clearance moves every row's shift by the same small amount.
    let at = |ring: u32| -> Result<gear_core::train::StageResult, TrainError> {
        let mut set = PlanetaryStage {
            module,
            planets,
            clearance: Auto::fixed(0.0),
            ..PlanetaryStage::default()
        };
        set.sun.teeth = sun;
        set.planet.teeth = planet;
        set.ring.teeth = ring;
        set.sun.profile_shift = Auto::fixed(sun_shift);
        set.ring.profile_shift = Auto::fixed(ring_shift);
        set.planet.profile_shift = Auto::automatic(0.0);
        // A listing of what the counts admit, not a design: the shifts are
        // taken as typed, not raised off undercut.
        for g in [&mut set.sun, &mut set.planet, &mut set.ring] {
            g.no_undercut = false;
        }
        solve_any(&Stage::planetary(set), &StageLoads::just(2.0), &lib)
    };
    let mut rows = Vec::new();
    let mut below: Option<(u32, TrainError)> = None;
    let mut refused: Option<(u32, TrainError)> = None;
    for ring in (planet + 1)..=(4 * ideal) {
        match at(ring) {
            Ok(r) => rows.push((ring, r)),
            Err(e) => {
                if rows.is_empty() {
                    below = Some((ring, e));
                } else {
                    refused = Some((ring, e));
                    break;
                }
            }
        }
    }
    if rows.is_empty() {
        println!("\nno ring tooth count admits a solution for that sun and planet");
        return;
    }
    if let Some((ring, e)) = below {
        println!("\nwhy it starts: z_ring {ring}: {e}");
    }

    println!(
        "\n{:>6} {:>10} {:>12} {:>10} {:>9} {:>7} {:>7} {:>11}",
        "z_ring", "x_planet", "c2c mm", "residual", "a_w sun", "even", "simult", "clearance"
    );
    for (ring, r) in &rows {
        let s = r.as_shape().expect("a set is a shape");
        let d = &s.distances[0];
        let layout = s.layouts.first();
        let clearance = layout.map_or_else(
            || "     n/a".to_string(),
            |l| format!("{:8.3}", l.clearance),
        );
        println!(
            "{ring:>6} {:>10.4} {:>12.6} {:>10.1e} {:>9.3} {:>7} {:>7} {clearance:>11}",
            s.members[1].profile_shift,
            d.running,
            (d.nominal[0] - d.nominal[1]).abs(),
            transverse(&s.meshes[0]).operating_pressure_angle,
            if layout.and_then(|l| l.equal_spacing) == Some(true) {
                "yes"
            } else {
                "no"
            },
            if layout.and_then(|l| l.simultaneous_meshing) == Some(true) {
                "yes"
            } else {
                "no"
            },
        );
    }

    // Why the list stops where it does, in the stage's own words.
    match refused {
        Some((ring, e)) => println!("\nwhy it stops: z_ring {ring}: {e}"),
        None => println!("\nwhy it stops: the sweep's own limit, four times the ideal ring"),
    }
}

/// A planetary stage, end to end, in all six arrangements.
fn planetary_stage_report(sun: u32, planet: u32, ring: u32, planets: u32, helix: f64) {
    use gear_core::planetary::{Arrangement, PlanetaryShaft};
    use gear_core::train::{PlanetaryStage, StageGear};

    let lib = gear_io::default_library();
    let base = PlanetaryStage {
        planets,
        sun: StageGear {
            teeth: sun,
            helix_angle: gear_core::params::Auto::fixed(helix),
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
    let all = [
        PlanetaryShaft::Sun,
        PlanetaryShaft::Carrier,
        PlanetaryShaft::Ring,
    ];
    let name = |m: PlanetaryShaft| match m {
        PlanetaryShaft::Sun => "sun",
        PlanetaryShaft::Carrier => "carrier",
        PlanetaryShaft::Ring => "ring",
    };

    let mut shown = false;
    for &input in &all {
        for &fixed in &all {
            if input == fixed {
                continue;
            }
            // The arrangement is what the set is *asked*, not what it is:
            // the same stage, six boundaries.
            let asked = PlanetaryStage::boundary_for(Arrangement { input, fixed });
            let output = all
                .iter()
                .copied()
                .find(|&m| m != input && m != fixed)
                .unwrap_or(input);
            match solve_set(
                &base,
                &gear_core::train::StageLoads::at(2.0, 3000.0).under(asked),
                &lib,
            ) {
                Err(e) => println!("  {:>7} in, {:>7} held: {e}", name(input), name(fixed)),
                Ok(solved) => {
                    let r = set_view(&solved).expect("a set");
                    if !shown {
                        println!(
                            "\nrunning centre distance {:.6} mm (zero-backlash {:.6} sun-planet, \
                             {:.6} planet-ring; residual {:.1e})  shifts {:+.4} / {:+.4} / {:+.4}",
                            r.centre_distance,
                            r.centre_distance_nominal[0],
                            r.centre_distance_nominal[1],
                            r.residual,
                            r.sun.profile_shift,
                            r.planet.profile_shift,
                            r.ring.profile_shift,
                        );
                        println!(
                            "eps_a  sun-planet {:.3}   planet-ring {:.3}   \
                             eta_0 {:.4}   even spacing {}   planet gap {}",
                            transverse(r.sun_planet).contact_ratios.transverse,
                            transverse(r.planet_ring).contact_ratios.transverse,
                            r.eta0,
                            r.layout
                                .and_then(|l| l.equal_spacing)
                                .map_or_else(|| "n/a".into(), |b| b.to_string()),
                            r.layout
                                .map_or_else(|| "n/a".into(), |l| format!("{:.3} mm", l.clearance))
                        );
                        // **The internal mesh, asked what an internal mesh is
                        // asked.** A set never put the question until the tip
                        // room moved onto the mesh report, and a full-depth ring
                        // fails one of the three as a matter of course — so this
                        // is the row that puts it in the corpus. Its counterpart
                        // for the sun-planet mesh is `None` and is not printed:
                        // an external pair has no such question.
                        // **Both meshes now**, because a tip reaching past a
                        // flank's usable end is every mesh's question and had
                        // been asked of the internal one alone.
                        for (name, m) in
                            [("sun-planet", r.sun_planet), ("planet-ring", r.planet_ring)]
                        {
                            println!(
                                "{name} flank interference  member 1 {}  member 2 {}{}",
                                m.flank_interference[0],
                                m.flank_interference[1],
                                m.tips.map_or_else(String::new, |t| format!(
                                    "   tips {} ({:+.4} deg of pinion)",
                                    t.tip_interference, t.tip_margin
                                ))
                            );
                        }
                        println!(
                            "sigma_H at pitch  sun-planet {:.1} MPa   planet-ring {:.1} MPa",
                            r.sun_planet.cases[0].contact.at_pitch_point,
                            r.planet_ring.cases[0].contact.at_pitch_point
                        );
                        println!(
                            "sigma_F  sun {}   planet {}   ring {}",
                            r.sun.cases[0]
                                .bending_stress
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.planet.cases[0]
                                .bending_stress
                                .map_or_else(|| "-".into(), |v| format!("{v:.1} MPa")),
                            r.ring.cases[0]
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
                        name(output),
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
    let stage = base.clone();
    if let Ok(solved) = solve_set(&stage, &gear_core::train::StageLoads::at(2.0, 3000.0), &lib) {
        let r = set_view(&solved).expect("a set");
        println!();
        for (which, mesh) in [("sun-planet", r.sun_planet), ("planet-ring", r.planet_ring)] {
            for note in &mesh.notes {
                println!("note: {which}: {}", words().render(note));
            }
        }
        for note in r.notes {
            println!("note: {}", words().render(note));
        }
    }

    // **A given centre distance**, which the set had no input for until F39's
    // third item. Two equations instead of one, both closed form, so the
    // iteration disappears and one shift is left free — and the row that says
    // it worked is `residual`, the layout's own measure of whether the two
    // meshes closed at the distance asked for.
    //
    // Here rather than nowhere because a path the harness never walks is a path
    // the change detector cannot see, which this project has recorded six times.
    let free = solve_set(&base, &gear_core::train::StageLoads::at(2.0, 3000.0), &lib);
    if let Ok(free) = free {
        let free = set_view(&free).expect("a set");
        println!("\ncentre distance given, shifts chosen to reach it");
        println!(
            "{:<12} {:>9} {:>9} {:>9} {:>11} {:>9}",
            "a mm", "x_sun", "x_planet", "x_ring", "residual", "eta_0"
        );
        for step in -2..=2 {
            let asked = free.centre_distance + 0.1 * f64::from(step);
            let mut stage = base.clone();
            stage.centre_distance = gear_core::Auto::fixed(asked);
            match solve_set(&stage, &gear_core::train::StageLoads::at(2.0, 3000.0), &lib) {
                Err(e) => println!("{asked:<12.4} {e}"),
                Ok(solved) => {
                    let r = set_view(&solved).expect("a set");
                    println!(
                        "{:<12.4} {:>9.4} {:>9.4} {:>9.4} {:>11.1e} {:>9.4}",
                        r.centre_distance,
                        r.sun.profile_shift,
                        r.planet.profile_shift,
                        r.ring.profile_shift,
                        r.residual,
                        r.eta0
                    );
                }
            }
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
    use gear_core::train::{PairKind, StageLoads};

    let lib = gear_io::default_library();
    println!(
        "crossed gear pair  z {z1}/{z2}  shaft angle {shaft_angle} deg  module 1  alpha 20 deg  \
         mu 0.06"
    );
    // The worm preset's members and frictions, entered as a gear pair: the
    // faces fixed, since nothing sizes a crossed gear pair's automatic face
    // and this table is about the split; both shifts pinned at zero so the
    // geometry is the pure one the split describes; and a gear's root round
    // on the first member, since a worm's thread has none and these are gears.
    let base = {
        let mut stage = worm_stage(z1, z2, 7.0);
        stage.shaft_angle = shaft_angle;
        for g in &mut stage.gears {
            g.face_width = gear_core::params::Auto::fixed(10.0);
            g.profile_shift = gear_core::params::Auto::fixed(0.0);
            g.root_radius = gear_core::train::StageGear::default().root_radius;
        }
        stage
    };
    println!(
        "\n{:>7} {:>7} {:>9} {:>9} {:>10} {:>10} {:>11} {:>10} {:>13}",
        "beta1", "beta2", "d1 mm", "d2 mm", "a mm", "slide/v1", "eta fwd", "sigma_H", "epsilon"
    );
    let mut any = false;
    for i in 0..=10 {
        #[allow(clippy::cast_precision_loss)]
        let beta1 = shaft_angle * (i as f64 / 10.0);
        let stage = base.clone().with_first_helix(beta1);
        let Ok(g) = gear_core::train::shape::Shape::from(&stage).screw(0) else {
            println!("{beta1:>7.1} {:>7} — no such pair", shaft_angle - beta1);
            continue;
        };
        match solve_pair(&stage, PairKind::Spur, &StageLoads::just(2.0), &lib) {
            Err(e) => println!(
                "{beta1:>7.1} {:>7.1}  {e}",
                g.wheel_helix_angle_rad.to_degrees()
            ),
            Ok(solved) => {
                let r = pair(&solved).expect("a pair");
                any = true;
                // **What a locked row says, in words**, printed under it rather
                // than left to the reader to infer from a blank efficiency.
                //
                // It is also what puts `mesh.forward_locking` in the golden
                // corpus. A note fired only by the string sweep is a note the
                // change detector cannot see, which is this project's
                // sixth-recorded *opt-in the harness never switches on* — and
                // the case is already here, so nothing had to be contrived.
                // **`locked` rather than `0.000 %`.** The steepest split here
                // cannot be driven forward at all, and printing its efficiency
                // as a number reads as arithmetic rather than as the statement
                // that this end cannot turn that one. This row is the case
                // `Directional::locked` was made directional for.
                let m = point(&r);
                let eta = if m.efficiency.locked().forward {
                    format!("{:>10} ", "locked")
                } else {
                    format!("{:>10.3} %", m.efficiency.forward * 100.0)
                };
                println!(
                    "{beta1:>7.1} {:>7.1} {:>9.4} {:>9.4} {:>10.4} {:>10.4} {eta} {:>9.1} {:>13}",
                    g.wheel_helix_angle_rad.to_degrees(),
                    g.worm_pitch_diameter,
                    g.wheel_pitch_diameter,
                    g.centre_distance,
                    g.sliding_ratio,
                    m.cases[0].contact.max_pressure,
                    if m.point.is_some() && m.contact_ratio > 0.0 {
                        format!("{:.9}", m.contact_ratio)
                    } else {
                        "—".to_string()
                    }
                );
                for n in m.notes.iter().filter(|n| {
                    n.is(gear_core::note::key::MESH_FORWARD_LOCKING)
                        || n.is(gear_core::note::key::MESH_SELF_LOCKING)
                }) {
                    println!("{:>16}{}", "", words().render(n));
                }
            }
        }
    }
    if any {
        println!(
            "\nA worm is the same geometry with the first member's diameter chosen instead \
             of its helix;\nthe split above is the freedom two gears have and a worm does not."
        );
    }

    // **The shifts chosen for least loss, at the even split** — the one
    // search, on the crossed mesh's own objective. Here so the optimiser's
    // crossed answer is in the change detector, which is the ninth time this
    // audit has had to put an opt-in the harness never switched on into it.
    let even = gear_core::train::PairStage {
        optimisation: gear_core::train::Optimisation {
            enabled: true,
            ..gear_core::train::Optimisation::default()
        },
        ..base.clone()
    }
    .with_first_helix(shaft_angle / 2.0);
    let mut free = even.clone();
    for g in &mut free.gears {
        g.profile_shift = gear_core::params::Auto::automatic(0.0);
    }
    println!(
        "\nshifts chosen for least loss at the even split ({:.1}/{:.1} deg)",
        shaft_angle / 2.0,
        shaft_angle / 2.0
    );
    println!(
        "{:<34} {:>9} {:>9} {:>9} {:>9} {:>10}",
        "", "x1", "x2", "a mm", "eps", "eta fwd"
    );
    for (name, stage) in [
        ("least shift that clears undercut", &even),
        ("least loss", &free),
    ] {
        match solve_pair(stage, PairKind::Spur, &StageLoads::just(2.0), &lib).map(|solved| {
            let r = pair(&solved).expect("a pair");
            (
                r.gears[0].profile_shift,
                r.gears[1].profile_shift,
                r.centre_distance,
                r.mesh.contact_ratio,
                r.mesh.efficiency.forward,
            )
        }) {
            Ok((x1, x2, a, eps, eta)) => println!(
                "{name:<34} {:>9.4} {:>9.4} {:>9.4} {:>9.4} {:>9.3} %",
                x1,
                x2,
                a,
                eps,
                100.0 * eta
            ),
            Err(e) => println!("{name:<34} {e}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Every subcommand is recorded, or says where instead.**
    ///
    /// The table is the dispatch, so this cannot be a check that the two agree —
    /// there is only one of them now. What it checks is the thing that replaced
    /// the drift: that adding a command cannot leave the golden corpus quietly
    /// short of it, because the decision is a field and the field has no
    /// default.
    #[test]
    fn every_command_says_how_its_output_is_kept() {
        for c in COMMANDS {
            match c.record {
                Record::Cases(cases) => {
                    assert!(!cases.is_empty(), "{}: Cases with no case", c.name);
                    for case in cases {
                        assert!(
                            case.split_whitespace().next() == Some(c.name),
                            "{}: golden case {case:?} invokes something else",
                            c.name
                        );
                    }
                }
                Record::Digest(case, why) => {
                    assert!(case.split_whitespace().next() == Some(c.name));
                    assert!(!why.is_empty(), "{}: a digest needs its reason", c.name);
                }
                Record::Elsewhere(why) => {
                    assert!(!why.is_empty(), "{}: needs to say where instead", c.name);
                }
            }
        }
    }

    /// Names are unique, and `help` can lay them out.
    ///
    /// Trivial, and it is here because a table is exactly the shape that grows a
    /// duplicate on a copied row — at which point the second one is unreachable
    /// and nothing says so.
    #[test]
    fn command_names_are_unique() {
        let mut seen: Vec<&str> = COMMANDS.iter().map(|c| c.name).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(before, seen.len(), "a command name is listed twice");
    }
}
