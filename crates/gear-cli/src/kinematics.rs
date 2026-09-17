//! **What every stage kind says about motion and torque, before any of it
//! moves.**
//!
//! This is the characterisation the geartrain refactor is measured against.
//! The plan (`geartrain-refactor-plan.md`) replaces three hand-written
//! kinematic models — `planetary::power`, the hula stage's mapping onto it, and
//! `solve_train`'s chain walk — with one graph, and the only honest way to know
//! whether that reproduced them is to have written them down first.
//!
//! # What it prints, and what it deliberately does not
//!
//! Motion, torque, loss and play. **No geometry at all**: no shift, no stress,
//! no centre distance, no contact ratio beyond the one number a mesh's loss
//! depends on. Those are recorded by `train`, `planetstage`, `hula` and the
//! rest, and printing them twice would be a second copy to keep in step.
//!
//! What is here is exactly the set of facts a graph over shafts and meshes has
//! to produce:
//!
//! - per train — total ratio, efficiency and backlash both ways, and where each
//!   load case ends up;
//! - per stage — ratio, efficiency and backlash both ways;
//! - per shaft of an epicyclic kind — speed and torque in every case,
//!   **including the shaft that is not a gear**;
//! - per member — its speed, its speed *against the frame of its mesh*, its
//!   torque and its cycles, in every case;
//! - per mesh — efficiency both ways, the contact ratio its loss is read over,
//!   and whether it hunts.
//!
//! # Why the members and the meshes are read kind-independently
//!
//! `StageResult::members()` and `::meshes()` already answer for any kind, and
//! everything below reads through them so that this harness cannot be the place
//! a kind is forgotten. The one thing it cannot ask that way is **which members
//! a mesh joins and in whose frame** — no accessor reports it, which is the gap
//! the graph closes — so a mesh is identified here by its position in the
//! stage's own order, and the shafts of the two epicyclic kinds are read
//! through their own result types. Both of those readings go when the graph
//! arrives, and their going is the point.

use gear_core::train::{
    solve_train, Duty, LoadCase, PairStage, PlanetaryStage, Port, Stage, StageGear, StageResult,
    Train, TrainResult,
};

/// The loads every fixture is rated for: one from each port, held at the far
/// end, and a fatigue case counted over a continuous duty.
///
/// **Both ports, because torque distributes differently from each.** Which way
/// a stage is driven decides where `η₀` multiplies in an epicyclic set and
/// which flank a screw pair presses, so a characterisation taken from one end
/// records half the model.
fn loads() -> Vec<LoadCase> {
    vec![
        LoadCase::ultimate(2.0, 3000.0),
        LoadCase {
            port: Port::End,
            reacted: true,
            ..LoadCase::ultimate(0.6, 0.0)
        },
        LoadCase {
            duty: Duty::Continuous {
                runtime_hours: 1000.0,
            },
            ..LoadCase::fatigue(2.0, 2400.0)
        },
    ]
}

/// A member with an automatic face width, so nothing in a fixture is sized by a
/// number typed here.
fn gear(teeth: u32) -> StageGear {
    StageGear {
        teeth,
        face_width: gear_core::params::Auto::automatic(0.0),
        ..StageGear::default()
    }
}

/// A pair at these tooth counts and this helix.
fn pair(z1: u32, z2: u32, helix: f64) -> Stage {
    let mut s = PairStage {
        gears: [gear(z1), gear(z2)],
        ..PairStage::default()
    };
    if helix != 0.0 {
        s = s.with_first_helix(helix);
    }
    Stage::Spur(s)
}

/// An epicyclic set driven by one shaft with another held.
fn set(input: &str, fixed: &str) -> Stage {
    use gear_core::planetary::{Arrangement, PlanetaryShaft};
    let shaft = |s: &str| match s {
        "sun" => PlanetaryShaft::Sun,
        "carrier" => PlanetaryShaft::Carrier,
        _ => PlanetaryShaft::Ring,
    };
    Stage::Planetary(Box::new(PlanetaryStage {
        arrangement: Arrangement {
            input: shaft(input),
            fixed: shaft(fixed),
        },
        ..PlanetaryStage::default()
    }))
}

/// **A set whose two centre distances no planet shift can bring together.**
///
/// 17-tooth sun, 17-tooth planets: only `z_ring ∈ [48, 54]` admits any solution
/// at all ([`gear_core::planetary`]), so 80 is genuinely impossible rather than
/// merely unconverged. Its **kinematics are perfectly well defined** — Willis
/// needs the tooth counts and nothing else — and the fixture is here to record
/// that the tool currently reports none of them.
fn unclosed() -> Stage {
    let Stage::Planetary(mut p) = set("sun", "ring") else {
        unreachable!("set() builds a planetary stage")
    };
    p.sun.teeth = 17;
    p.planet.teeth = 17;
    p.ring.teeth = 80;
    Stage::Planetary(p)
}

/// **Every fixture, and why each is here.**
///
/// A table rather than a `match`, for the reason `COMMANDS` is one: a fixture
/// added with no row is a fixture the corpus cannot record.
fn fixtures() -> Vec<(String, Train)> {
    let train = |stages: Vec<Stage>| Train {
        load_cases: loads(),
        reversed_bending: false,
        stages,
    };
    let mut out = vec![
        // The two parallel-axis readings: a spur pair has no axial overlap and
        // a helical one does, and neither changes the kinematics — which is
        // itself a thing the graph must reproduce rather than assume.
        ("pair".to_string(), train(vec![pair(17, 43, 0.0)])),
        ("helical".to_string(), train(vec![pair(17, 43, 20.0)])),
        // A worm: the one kind whose two directions genuinely differ, and the
        // one that can refuse to be driven at all.
        (
            "worm".to_string(),
            train(vec![Stage::Worm(PairStage::worm())]),
        ),
    ];
    // **All six arrangements**, because which shaft is held is the whole of
    // what an epicyclic set's ratio and efficiency depend on, and the plan
    // moves that decision from the stage to the train. Six rows here is what
    // says whether the move cost anything.
    for input in ["sun", "carrier", "ring"] {
        for fixed in ["sun", "carrier", "ring"] {
            if input != fixed {
                out.push((
                    format!("set-{input}-{fixed}"),
                    train(vec![set(input, fixed)]),
                ));
            }
        }
    }
    out.push(("hula".to_string(), train(vec![Stage::Hula(Box::default())])));
    // Chains, because the accumulation is the third place the same kinematics
    // is written: two pairs, and a chain with an epicyclic set in the middle of
    // it so that a stage with three shafts sits between two with two.
    out.push((
        "chain".to_string(),
        train(vec![pair(17, 43, 0.0), pair(13, 31, 15.0)]),
    ));
    out.push((
        "mixed".to_string(),
        train(vec![
            pair(17, 43, 0.0),
            set("sun", "ring"),
            Stage::Worm(PairStage::worm()),
        ]),
    ));
    // **A train that does not close, recorded as it currently answers.** A
    // ratio needs tooth counts and topology; neither of these fixtures has
    // anything wrong with its kinematics. The first says what a set with no
    // admissible planet shift reports, and the second says what it costs the
    // stage beside it — a pair that closes perfectly well and loses its ratio
    // to a neighbour that does not. Both are the fault
    // `geartrain-refactor-plan.md` opens with, and they are here so that fixing
    // it is a diff in this file rather than an assertion about one.
    out.push(("unclosed".to_string(), train(vec![unclosed()])));
    out.push((
        "chain-unclosed".to_string(),
        train(vec![pair(17, 43, 0.0), unclosed()]),
    ));
    out
}

/// The shafts an epicyclic kind reports, by name, or nothing for a kind whose
/// shafts are its members.
///
/// **Transitional.** It is the one reading below that has to know what kind it
/// is holding, and it is there because no accessor reports a stage's shafts —
/// only its gears. The graph's `shafts()` replaces it.
fn shafts(s: &StageResult) -> Option<([&'static str; 3], &[gear_core::train::ShaftsCase])> {
    if let Some(p) = s.as_planetary() {
        return Some((["sun", "carrier", "ring"], &p.cases));
    }
    s.as_hula()
        .map(|h| (["grounded", "crank", "output"], h.cases.as_slice()))
}

/// **What the graph says**, from tooth counts and topology alone.
///
/// Printed for every fixture whether or not the geometry solved, because it
/// needs none of it: a train whose stage cannot be built still turns at a
/// ratio, and the two rows under `no answer:` below are the whole of the fault
/// the refactor opens on, recorded so that fixing it is a diff here.
///
/// The ratios are **exact** — a quotient of tooth counts written as one, so a
/// reduction of exactly 49 says 49 and an arrangement whose meshes cancel says
/// it does not turn rather than printing a very large number.
fn graph(train: &Train) {
    match train.motion() {
        Ok(m) => {
            println!(
                "  graph    mobility {} degree(s){}   total ratio {}",
                m.mobility.degrees,
                if m.mobility.untouched.is_empty() {
                    String::new()
                } else {
                    format!("  ({} shaft(s) untouched)", m.mobility.untouched.len())
                },
                m.total
                    .map_or_else(|| "does not turn".to_string(), |r| r.to_string()),
            );
            for (k, r) in m.ratios.iter().enumerate() {
                println!(
                    "    stage {}  ratio {}",
                    k + 1,
                    r.map_or_else(|| "does not turn".to_string(), |r| r.to_string())
                );
            }
            for s in &m.shafts {
                println!(
                    "    shaft {:<9} of {:<7} speed {}",
                    s.label,
                    s.stage
                        .map_or_else(|| "the train".to_string(), |k| format!("stage {}", k + 1)),
                    s.speed,
                );
            }
        }
        Err(e) => println!("  graph    no motion: {e:?}"),
    }
}

/// One fixture, end to end.
fn report(name: &str, train: &Train, r: &TrainResult) {
    println!("== {name} ==");
    println!(
        "  total    ratio {:>14.6}   efficiency {:>10.6} / {:<10.6} %   backlash {:>10.6} / {:<10.6} deg",
        r.total_ratio,
        100.0 * r.total_efficiency.forward,
        100.0 * r.total_efficiency.backward,
        r.backlash.forward.nominal,
        r.backlash.backward.nominal,
    );
    for c in &r.cases {
        let input = &train.load_cases[c.case];
        println!(
            "  case {}   {:>10.4} Nm / {:>9.2} rpm at {:<5}  ->  {:>10.4} Nm / {:>9.2} rpm at {:<5}  held at {}",
            c.case + 1,
            input.torque,
            input.speed,
            port(input.port),
            c.delivered_torque,
            c.delivered_speed,
            port(c.delivered_at),
            c.reacted_at
                .map_or_else(|| "-".to_string(), |k| (k + 1).to_string()),
        );
    }
    for (k, s) in r.stages.iter().enumerate() {
        println!(
            "  stage {}  ratio {:>14.6}   efficiency {:>10.6} / {:<10.6} %   backlash {:>10.6} / {:<10.6} deg",
            k + 1,
            s.ratio(),
            100.0 * s.efficiency().forward,
            100.0 * s.efficiency().backward,
            s.backlash().forward.nominal,
            s.backlash().backward.nominal,
        );
        if let Some((names, cases)) = shafts(s) {
            for (i, label) in names.iter().enumerate() {
                for c in cases {
                    println!(
                        "    shaft {label:<9} case {}  speed {:>14.6}  torque {:>14.6}",
                        c.case + 1,
                        c.speeds[i],
                        c.torques[i],
                    );
                }
            }
        }
        for (i, g) in s.members().iter().enumerate() {
            println!("    member {}  z {:<4}", i + 1, g.params.teeth);
            for c in &g.cases {
                println!(
                    "      case {}  speed {:>14.6}  vs frame {:>14.6}  torque {:>14.6}  cycles {}",
                    c.case + 1,
                    c.speed,
                    c.speed_against_carrier,
                    c.torque,
                    c.cycles.map_or_else(
                        || "-".to_string(),
                        |n| format!("{:.6e} / {:.6e}", n.bending, n.contact)
                    ),
                );
            }
        }
        for (j, m) in s.meshes().iter().enumerate() {
            println!(
                "    mesh {}  efficiency {:>10.6} / {:<10.6} %  contact ratio {:>8.4}  {}",
                j + 1,
                100.0 * m.efficiency.forward,
                100.0 * m.efficiency.backward,
                m.contact_ratio,
                if m.coprime {
                    "coprime"
                } else {
                    "shares a factor"
                },
            );
        }
    }
    graph(train);
    println!();
}

fn port(p: Port) -> &'static str {
    match p {
        Port::Start => "start",
        Port::End => "end",
    }
}

/// Every fixture, or the one named.
pub fn run(which: Option<&str>) {
    let lib = gear_io::default_library();
    let all = fixtures();
    let mut found = false;
    for (name, train) in &all {
        if which.is_some_and(|w| w != name) {
            continue;
        }
        found = true;
        match solve_train(train, &lib) {
            Ok(r) => report(name, train, &r),
            // A fixture that refuses is recorded as refusing — but only the
            // *geometry* refused, so the graph still answers underneath it.
            // The two `unclosed` fixtures are the whole of the fault the
            // refactor opens on, in one place: the `no answer` line is the
            // train's, and the `graph` block beneath it is what the train
            // already knows and does not report.
            Err(e) => {
                println!("== {name} ==\n  no answer: {e}");
                graph(train);
                println!();
            }
        }
    }
    if !found {
        println!(
            "no such fixture. try one of: {}",
            all.iter()
                .map(|(n, _)| n.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
}
