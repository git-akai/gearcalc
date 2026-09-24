//! **Every preset, alone and after every other, read by what the graph has
//! and never by stage** — the record the train-as-graph refactor was held
//! to (docs/rationale.md#a-train-is-one-graph-and-a-stage-is-a-part-of-it).
//!
//! # Why it prints nothing a stage owns
//!
//! The refactor deleted the stage's own figures (ratio, efficiency, backlash
//! under its own convention, one more tooth), which became paths a case
//! reports, and the train's conventional-ends row. A record that printed
//! them would have changed form at that step, and a changed record cannot
//! say whether a number moved with it. So this prints only what exists on
//! both sides:
//!
//! - per path — each case's, from its load to its reaction, with the power
//!   through its teeth and what one more tooth on each gear does to it;
//! - per case — every body's role, speed and torque;
//! - per member, numbered across the train — its size and shift, and in
//!   every case its torque, speed, cycles, stresses and least widths;
//! - per mesh, numbered across the train — its operating angle, contact
//!   ratios, efficiency both ways, play at each member, whether it hunts,
//!   and the power through it in every case;
//! - per distance — where it runs and with what clearance;
//! - the notes, by key.
//!
//! **Every phase of the refactor that should have moved no number left this
//! byte for byte**, including the one that changed how a train is stored.
//! That is what made it the arbiter; `train`, `kinematics` and the rest
//! record what they record.
//!
//! # The fixtures
//!
//! Each preset alone; each ordered pair of presets, the first's output
//! running on into the second's input as the panel's menu joins them; and a
//! spur pair, a layshaft and a compound set in a row — the train the
//! refactor's canvas drew. Every train carries the three cases a fresh
//! train's ends are given (`Train::fresh_case`), switched on: an ultimate
//! load at the input reacted at the output, the same from the output, and a
//! fatigue load at the input.

use gear_core::train::arrangements::StagePreset;
use gear_core::train::{solve_train, CaseKind, Duty, Shape, Train, TrainResult};

/// A train of presets in a row, with the three cases between its ends.
fn chain(presets: &[StagePreset]) -> Train {
    let stages: Vec<Shape> = presets.iter().map(|p| p.build()).collect();
    let mut train = Train::chained(stages, |_| Vec::new());
    let mut cases = vec![
        train.fresh_case(CaseKind::Ultimate, 2.0, 3000.0),
        {
            // From the output: the input declared free, so the case holds a
            // load only a stage that locks can hold.
            let forward = train.fresh_case(CaseKind::Ultimate, 0.6, 0.0);
            let (input, output) = (forward.loads[0].at, forward.loads[1].at);
            gear_core::train::LoadCase::back_driving(input, output, 0.6)
        },
        gear_core::train::LoadCase {
            duty: Duty::Continuous {
                runtime_hours: 1000.0,
            },
            ..train.fresh_case(CaseKind::Fatigue, 2.0, 2400.0)
        },
    ];
    for c in &mut cases {
        c.enabled = true;
    }
    train.load_cases = cases;
    train
}

fn fixtures() -> Vec<(String, Train)> {
    let name = |p: &StagePreset| format!("{p:?}").to_lowercase();
    let mut out: Vec<(String, Train)> = StagePreset::ALL
        .iter()
        .map(|p| (name(p), chain(&[*p])))
        .collect();
    for a in StagePreset::ALL {
        for b in StagePreset::ALL {
            out.push((format!("{} then {}", name(&a), name(&b)), chain(&[a, b])));
        }
    }
    out.push((
        "spur then layshaft then compound".to_string(),
        chain(&[
            StagePreset::Spur,
            StagePreset::Layshaft,
            StagePreset::Compound,
        ]),
    ));
    out
}

fn opt(v: Option<f64>, precision: usize) -> String {
    v.map_or_else(|| "-".to_string(), |v| format!("{v:.precision$}"))
}

fn report(name: &str, train: &Train, r: &TrainResult) {
    println!("== {name} ==");
    for p in &r.paths {
        println!(
            "  path b{} -> b{}  ratio {:.6}  efficiency {:.6} / {:.6} %  backlash {:.6} / {:.6} deg",
            p.from,
            p.to,
            p.ratio,
            100.0 * p.efficiency.forward,
            100.0 * p.efficiency.backward,
            p.backlash.forward.nominal,
            p.backlash.backward.nominal,
        );
        println!(
            "    power through the teeth {:.6} / {:.6}   one more tooth: {}",
            p.circulation.forward,
            p.circulation.backward,
            p.per_tooth
                .iter()
                .map(|r| opt(*r, 6))
                .collect::<Vec<_>>()
                .join(" "),
        );
    }
    for c in &r.cases {
        println!(
            "  case {}{}",
            c.case + 1,
            if c.solved { "" } else { "  (not solved)" }
        );
        for n in &c.notes {
            println!("    ! {}", n.key);
        }
        for s in &c.bodies {
            println!(
                "    b{:<3} {:<8} speed {:>14}  torque {:.6}",
                s.at,
                format!("{:?}", s.role).to_lowercase(),
                opt(s.speed, 4),
                s.torque
            );
        }
    }
    // Members and meshes numbered across the train, in stage order — the
    // order a flattened graph lists them in.
    let mut member = 0;
    let mut mesh = 0;
    let mut distance = 0;
    for (k, s) in r.by_part(train).iter().enumerate() {
        let stages = train.part_shapes();
        let shape = &stages[k];
        for g in &s.members {
            member += 1;
            println!(
                "  member {member}  z {}  m {}  alpha {}  x {:.6}  b {:.6}  beta {:.6}  d {:.6}",
                g.params.teeth,
                g.params.module,
                g.params.pressure_angle,
                g.profile_shift,
                g.face_width,
                g.helix_angle,
                g.pitch_diameter,
            );
            for n in g.notes.iter().chain(&g.clamps) {
                println!("    ! {}", n.key);
            }
            for c in &g.cases {
                println!(
                    "    case {}  T {:.6}  n {:.4}  vs frame {:.4}  sF {}  sH {:.3}  cycles {}  widths {} / {}",
                    c.case + 1,
                    c.torque,
                    c.speed,
                    c.speed_against_carrier,
                    opt(c.bending_stress, 3),
                    c.contact_stress,
                    c.cycles.map_or_else(
                        || "-".to_string(),
                        |n| format!("{:.6e} / {:.6e}", n.bending, n.contact)
                    ),
                    opt(c.min_face_width.bending, 6),
                    opt(c.min_face_width.contact, 6),
                );
            }
        }
        for (j, m) in s.meshes.iter().enumerate() {
            mesh += 1;
            let first = member - s.members.len();
            let pair = shape.meshes[j];
            let geometry = m.line.as_ref().map_or_else(
                || "point contact".to_string(),
                |l| {
                    format!(
                        "alpha_w {:.6}  eps {:.6} / {:.6} / {:.6}",
                        l.operating_pressure_angle,
                        l.contact_ratios.transverse,
                        l.contact_ratios.overlap,
                        l.contact_ratios.total
                    )
                },
            );
            println!(
                "  mesh {mesh}  members {} {}  {geometry}  efficiency {:.6} / {:.6} %  play {:.6} / {:.6} deg  {}",
                first + pair.a + 1,
                first + pair.b + 1,
                100.0 * m.efficiency.forward,
                100.0 * m.efficiency.backward,
                m.backlash[0].nominal,
                m.backlash[1].nominal,
                if m.coprime { "coprime" } else { "shares a factor" },
            );
            for n in &m.notes {
                println!("    ! {}", n.key);
            }
            println!(
                "    power through, by case: {}",
                m.cases
                    .iter()
                    .map(|c| format!("{:.6}", c.power_through))
                    .collect::<Vec<_>>()
                    .join(" / ")
            );
        }
        for d in &s.distances {
            distance += 1;
            println!(
                "  distance {distance}  running {:.6}  clearance {:.6}",
                d.running, d.clearance
            );
        }
    }
    println!();
}

pub fn run() {
    let lib = gear_io::default_library();
    for (name, train) in fixtures() {
        match solve_train(&train, &lib) {
            Ok(r) => report(&name, &train, &r),
            Err(e) => println!("== {name} ==\n  refused: {e:?}\n"),
        }
    }
}
