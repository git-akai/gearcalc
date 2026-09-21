//! **What every preset and arrangement says about motion and torque, before
//! any of it moves.**
//!
//! This was the characterisation the geartrain refactor was measured against:
//! the plan (`geartrain-refactor-plan.md`) replaced three hand-written
//! kinematic models — `planetary::power`, the hula stage's mapping onto it, and
//! `solve_train`'s chain walk — with one graph, and the only honest way to know
//! whether that reproduced them was to have written them down first. It is
//! the record the graph is still held to, in the golden corpus.
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
//! - per shaft of an epicyclic preset — speed and torque in every case,
//!   **including the shaft that is not a gear**;
//! - per member — its speed, its speed *against the frame of its mesh*, its
//!   torque and its cycles, in every case;
//! - per mesh — efficiency both ways, the contact ratio its loss is read over,
//!   and whether it hunts.
//!
//! # Why the members and the meshes are read through the shape
//!
//! `StageResult::members()` and `::meshes()` answer for any stage, and
//! everything below reads through them so that this harness cannot be the
//! place an arrangement is forgotten. Which members a mesh joins and in whose
//! frame, and what each shaft is called, are read off the shape ([`member_role`])
//! rather than off a result type of the arrangement's own — the two readings
//! that were per type went with the types.

use gear_core::train::{
    solve_train, Duty, LoadCase, PairStage, PlanetaryStage, ShaftConstraint, ShaftRef, Stage,
    StageGear, StageResult, Train, TrainResult,
};

/// The loads every fixture is rated for, between two shafts: one from each,
/// reacted at the other, and a fatigue case counted over a continuous duty.
///
/// **Both ports, because torque distributes differently from each.** Which way
/// a stage is driven decides where `η₀` multiplies in an epicyclic set and
/// which flank a screw pair presses, so a characterisation taken from one end
/// records half the model.
fn loads(input: ShaftRef, output: ShaftRef) -> Vec<LoadCase> {
    vec![
        LoadCase::ultimate(input, output, 2.0, 3000.0),
        LoadCase::back_driving(input, output, 0.6),
        LoadCase {
            duty: Duty::Continuous {
                runtime_hours: 1000.0,
            },
            ..LoadCase::fatigue(input, output, 2.0, 2400.0)
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
    Stage::spur(s)
}

/// A set's shaft by name, in its wiring's order: ground, sun, carrier, ring,
/// planet.
fn member(s: &str) -> usize {
    match s {
        "sun" => 1,
        "carrier" => 2,
        _ => 3,
    }
}

/// **An arrangement as a train states it**: a lone set with one shaft held
/// — one line, since a hold on a stage replaces the stage's conventional
/// hold, so holding the carrier releases the ring without a word about it —
/// and its loads between the other two, the input named first. The set
/// itself carries no arrangement; what drives it is a load, and nothing
/// more.
fn arranged(input: &str, fixed: &str) -> Train {
    let output = ["sun", "carrier", "ring"]
        .into_iter()
        .find(|s| *s != input && *s != fixed)
        .expect("three shafts, two named");
    let at = |s: &str| ShaftRef::Of {
        stage: 0,
        shaft: member(s),
    };
    let mut t = Train::chained(vec![set()], loads(at(input), at(output)));
    t.constraints = vec![ShaftConstraint::held(0, member(fixed))];
    t
}

/// A default epicyclic set. What drives it and what holds it is the train's
/// to say ([`arranged`]); alone, it is solved sun in and ring held.
fn set() -> Stage {
    Stage::planetary(PlanetaryStage::default())
}

/// **A set whose two centre distances no planet shift can bring together.**
///
/// 17-tooth sun, 17-tooth planets: only `z_ring ∈ [48, 54]` admits any solution
/// at all ([`gear_core::planetary`]), so 80 is genuinely impossible rather than
/// merely unconverged. Its **kinematics are perfectly well defined** — Willis
/// needs the tooth counts and nothing else — and the fixture is here to record
/// that the tool currently reports none of them.
fn unclosed() -> Stage {
    let mut p = PlanetaryStage::default();
    p.sun.teeth = 17;
    p.planet.teeth = 17;
    p.ring.teeth = 80;
    Stage::planetary(p)
}

/// **Every fixture, and why each is here.**
///
/// A table rather than a `match`, for the reason `COMMANDS` is one: a fixture
/// added with no row is a fixture the corpus cannot record.
fn fixtures() -> Vec<(String, Train)> {
    // A chain of these stages, loaded between its two ends.
    let train = |stages: Vec<Stage>| {
        let mut t = Train::chained(stages, Vec::new());
        let (input, output) = t
            .boundaries()
            .ok()
            .and_then(|b| t.ends(&b))
            .expect("a chain fixture has two ends");
        t.load_cases = loads(input, output);
        t
    };
    let mut out = vec![
        // The two parallel-axis readings: a spur pair has no axial overlap and
        // a helical one does, and neither changes the kinematics — which is
        // itself a thing the graph must reproduce rather than assume.
        ("pair".to_string(), train(vec![pair(17, 43, 0.0)])),
        ("helical".to_string(), train(vec![pair(17, 43, 20.0)])),
        // A worm: the one preset whose two directions genuinely differ, and the
        // one that can refuse to be driven at all.
        (
            "worm".to_string(),
            train(vec![Stage::worm(PairStage::worm())]),
        ),
    ];
    // **All six arrangements**, because which shaft is held is the whole of
    // what an epicyclic set's ratio and efficiency depend on, and the plan
    // moves that decision from the stage to the train. Six rows here is what
    // says whether the move cost anything.
    for input in ["sun", "carrier", "ring"] {
        for fixed in ["sun", "carrier", "ring"] {
            if input != fixed {
                out.push((format!("set-{input}-{fixed}"), arranged(input, fixed)));
            }
        }
    }
    out.push((
        "hula".to_string(),
        train(vec![Stage::hula(gear_core::train::HulaStage::default())]),
    ));
    // **The arrangements the shape reaches with no code of their own**
    // (`gear_core::train::arrangements`), each under its textbook boundary,
    // which the shape's convention — first shaft driven, first ring held —
    // gives some of and the constraints the rest. `tools/train_kinematics.py`
    // derives the same speeds from rigid-body velocities.
    {
        use gear_core::train::arrangements as arr;
        let shape = |s| Stage::Shape(Box::new(s));
        // Three ratios on one layshaft, the second engaged: 17/43 in, then
        // 31/29 out, the idlers turning free.
        out.push((
            "layshaft".to_string(),
            train(vec![shape(arr::layshaft(
                (17, 43),
                &[(19, 41), (31, 29), (43, 17)],
                1,
            ))]),
        ));
        // A worm feeding a spur pair in one stage: a point contact and a
        // line contact on two distances, the stage locking where the worm
        // does.
        out.push((
            "wormpair".to_string(),
            train(vec![shape(arr::worm_and_pair((1, 40), (17, 43)))]),
        ));
        // A Wolfrom under its convention: carrier in, first ring held, second
        // ring out — 61 : 1 on a one-tooth difference.
        out.push((
            "wolfrom".to_string(),
            train(vec![shape(arr::wolfrom(18, [60, 61], 3))]),
        ));
        // A stepped planet under its convention: sun in, first ring held,
        // second ring out — the compound reduction.
        out.push((
            "stepped".to_string(),
            train(vec![shape(arr::stepped(24, [18, 17], [60, 59], 3))]),
        ));
        // A planocentric: carrier in, ring held by convention, the planet's
        // own turn out.
        out.push((
            "planocentric".to_string(),
            train(vec![shape(arr::planocentric(30, 33))]),
        ));
        // Meshed planets under their convention: sun in, ring held, carrier
        // out, turning against the sun.
        out.push((
            "meshed-planets".to_string(),
            train(vec![shape(arr::meshed_planets(24, [18, 18], 96, 3))]),
        ));
        // A Ravigneaux, three ways. It has two degrees of freedom, so one
        // member is held and one driven: the small sun in with the ring held
        // by convention (first gear); the large sun in with the ring held
        // (reverse, through the planet–planet mesh); and the small sun in
        // with the large sun held, the ring running free. The carrier (2)
        // is the output each time.
        let ravigneaux = || arr::ravigneaux([18, 30], [22, 18], 62, 3);
        out.push((
            "ravigneaux-small-sun".to_string(),
            train(vec![shape(ravigneaux())]),
        ));
        // Loaded at the large sun with the ring held: the small sun and the
        // carrier are both free, and the carrier is the output a designer
        // means — so every case is written between the large sun and the
        // carrier, and declares the small sun free, which is how a case says
        // a port turns and carries nothing.
        out.push(("ravigneaux-large-sun".to_string(), {
            let at = |shaft| ShaftRef::Of { stage: 0, shaft };
            let (small_sun, carrier, large_sun) = (1, 2, 3);
            let mut t =
                Train::chained(vec![shape(ravigneaux())], loads(at(large_sun), at(carrier)));
            for c in &mut t.load_cases {
                c.loads.push(gear_core::train::Load::declared(
                    at(small_sun),
                    gear_core::train::LoadRole::Free,
                ));
            }
            t
        }));
        // The small sun in with the large sun held: the ring runs free,
        // the carrier is the output, and the hold on the large sun replaces
        // the convention's hold on the ring.
        out.push(("ravigneaux-ring-free".to_string(), {
            let at = |shaft| ShaftRef::Of { stage: 0, shaft };
            let mut t = Train::chained(vec![shape(ravigneaux())], loads(at(1), at(2)));
            t.constraints = vec![ShaftConstraint::held(0, 3)];
            t
        }));
    }
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
            set(),
            Stage::worm(PairStage::worm()),
        ]),
    ));
    // **A reversing stage, in front of another and behind one.** An epicyclic
    // set with its carrier held has a negative ratio, and no shipped fixture
    // ever put one next to a second stage — so the two places the sign leaked
    // into a size were both outside the change detector. A set that could not
    // be followed by anything at all, and a backlash 23.5 % light, are what
    // that cost; these two rows are what keeps them caught.
    // A set with its carrier held reverses. Ahead of a pair it is coupled
    // onward by its ring — the chain's coupling is to the carrier, which the
    // hold takes back — and loaded at its sun; behind one it is entered by
    // its sun through the coupling and leaves by the ring.
    out.push(("set-then-pair".to_string(), {
        let of = |stage, shaft| ShaftRef::Of { stage, shaft };
        let mut t = Train::chained(vec![set(), pair(17, 43, 0.0)], Vec::new());
        t.hold(of(0, 2));
        t.couple(of(0, 3), of(1, 1));
        t.load_cases = loads(of(0, 1), of(1, 2));
        t
    }));
    out.push(("pair-then-set".to_string(), {
        let of = |stage, shaft| ShaftRef::Of { stage, shaft };
        let mut t = Train::chained(vec![pair(17, 43, 0.0), set()], Vec::new());
        t.hold(of(1, 2));
        t.load_cases = loads(of(0, 1), of(1, 3));
        t
    }));
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
    // **A train that is one condition short, and one that asks two things
    // of a shaft** — recorded for what each answers, since each used to
    // answer with the wiring sentence. The first is a set with its ring
    // released: no figure of its own, every case one speed short under the
    // line, and the ring free. The second holds the carrier and the ring
    // both, so the sun cannot turn, named at the hold that closed it.
    out.push(("ring-released".to_string(), {
        let mut t = arranged("sun", "ring");
        t.release(ShaftRef::Of { stage: 0, shaft: 3 });
        t
    }));
    out.push(("conflict".to_string(), {
        let mut t = arranged("sun", "carrier");
        t.hold(ShaftRef::Of { stage: 0, shaft: 3 });
        t
    }));
    out
}

/// One shaft's `(case, speed, torque)` per load case.
type ShaftLine = Vec<(usize, f64, f64)>;

/// **Every shaft's speed and torque per case**, named as the harness names
/// shafts: a shape's from its own per-shaft cases, a hula stage's from its
/// three.
fn shaft_cases(stage: &Stage, s: &StageResult) -> Vec<(String, ShaftLine)> {
    match s {
        StageResult::Shape(r) => {
            let w = stage.wiring();
            (1..w.shafts.len())
                .map(|i| {
                    (
                        named(
                            std::slice::from_ref(stage),
                            gear_core::train::ShaftRef::Of { stage: 0, shaft: i },
                            w.shafts[i],
                        ),
                        r.cases
                            .iter()
                            .map(|c| (c.case, c.speeds[i], c.torques[i]))
                            .collect(),
                    )
                })
                .collect()
        }
    }
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
    let m = match train.motion() {
        Ok(_) => train.motion_report().expect("a motion has a report"),
        Err(e) => {
            println!("  graph    no motion: {e:?}");
            return;
        }
    };
    let by_ref = |r: gear_core::train::ShaftRef| -> String {
        m.shafts
            .iter()
            .find(|s| s.at == r)
            .map_or_else(|| format!("{r:?}"), |s| named(&train.stages, s.at, s.label))
    };
    // A quotient that is not there is a family where the answer is one, and
    // an output that does not turn where it is not.
    let quotient = |r: &Option<gear_core::train::Exact>| -> String {
        match r {
            Some(x) => x.text.clone(),
            None if m.free.is_empty() => "does not turn".to_string(),
            None => "a family".to_string(),
        }
    };
    println!(
        "  graph    mobility {} degree(s), {} given{}{}   total ratio {}",
        m.mobility,
        m.constrained,
        if m.untouched.is_empty() {
            String::new()
        } else {
            format!("  ({} shaft(s) untouched)", m.untouched.len())
        },
        if m.free.is_empty() {
            String::new()
        } else {
            format!(
                "  free: {}",
                m.free
                    .iter()
                    .map(|&r| by_ref(r))
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
        quotient(&m.total),
    );
    for (k, r) in m.ratios.iter().enumerate() {
        println!("    stage {}  ratio {}", k + 1, quotient(r));
    }
    for s in &m.shafts {
        let terms: String = s
            .terms
            .iter()
            .map(|t| format!(" + {} × ω({})", t.coefficient.text, by_ref(t.per)))
            .collect();
        println!(
            "    shaft {:<9} of {:<7} speed {}{terms}",
            named(&train.stages, s.at, s.label),
            match s.at {
                gear_core::train::ShaftRef::Ground => "the train".to_string(),
                gear_core::train::ShaftRef::Of { stage, .. } => format!("stage {}", stage + 1),
            },
            s.speed.text,
        );
    }
}

/// One fixture, end to end.
fn report(name: &str, train: &Train, r: &TrainResult) {
    println!("== {name} ==");
    // Every path: between every two open bodies, the two ends first.
    for p in &r.paths {
        println!(
            "  path {:<5} -> {:<5} ratio {:>14.6}   efficiency {:>10.6} / {:<10.6} %   backlash {:>10.6} / {:<10.6} deg",
            port(p.from),
            port(p.to),
            p.ratio,
            100.0 * p.efficiency.forward,
            100.0 * p.efficiency.backward,
            p.backlash.forward.nominal,
            p.backlash.backward.nominal,
        );
    }
    if r.paths.is_empty() {
        println!("  no path: the train's holds leave its motion a family");
    }
    // Every shaft of every case: what it is in the case and what it
    // carries — the loads as given or derived, the reactions found.
    for c in &r.cases {
        let input = &train.load_cases[c.case];
        println!(
            "  case {}   {}{}",
            c.case + 1,
            input
                .loads
                .iter()
                .map(entry)
                .collect::<Vec<_>>()
                .join("  +"),
            if c.solved { "" } else { "   (not solved)" }
        );
        for n in &c.notes {
            println!("    ! {}", n.key);
        }
        for s in &c.shafts {
            println!(
                "    {:<8} {:<26} {:>10}  torque {:>12.6}",
                format!("{:?}", s.role).to_lowercase(),
                named(&train.stages, s.at, s.label),
                s.speed
                    .map_or_else(|| "-".to_string(), |v| format!("{v:.4} rpm")),
                s.torque
            );
        }
    }
    for (k, s) in r.stages.iter().enumerate() {
        println!(
            "  stage {}  ratio {:>14.6}   efficiency {:>10.6} / {:<10.6} %   backlash {:>10.6} / {:<10.6} deg",
            k + 1,
            crate::or_nan(s.ratio()),
            100.0 * crate::ways_or_nan(s.efficiency()).forward,
            100.0 * crate::ways_or_nan(s.efficiency()).backward,
            crate::play_or_nan(s.backlash()).forward.nominal,
            crate::play_or_nan(s.backlash()).backward.nominal,
        );
        // What the teeth pass over what comes in, both ways, and what one
        // more tooth on each member would make the ratio — the two figures
        // Phase 7 added, recorded so their path is known to be walked.
        if let Some(shape) = s.as_shape() {
            println!(
                "    power through the teeth {:>10.6} / {:<10.6}   one more tooth on each member: {}",
                crate::ways_or_nan(shape.circulation).forward,
                crate::ways_or_nan(shape.circulation).backward,
                shape
                    .ratio_per_tooth
                    .iter()
                    .flatten()
                    .map(|r| r.map_or_else(|| "locked".to_string(), |r| format!("{r:.6}")))
                    .collect::<Vec<_>>()
                    .join(" ")
            );
        }
        for (label, cases) in shaft_cases(&train.stages[k], s) {
            for (case, speed, torque) in cases {
                println!(
                    "    shaft {label:<9} case {}  speed {:>14.6}  torque {:>14.6}",
                    case + 1,
                    speed,
                    torque,
                );
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

/// What to call a shaft here — the harness's English, which the core does not
/// have. A member's shaft is named after the member's role where the shape
/// gives it one, so the corpus reads as it did.
pub fn named(stages: &[Stage], at: ShaftRef, label: gear_core::train::ShaftLabel) -> String {
    use gear_core::train::ShaftLabel;
    let stage = match at {
        ShaftRef::Ground => None,
        ShaftRef::Of { stage, .. } => stages.get(stage),
    };
    match (label, stage) {
        (ShaftLabel::Ground, _) => "ground".into(),
        (ShaftLabel::Carrier { .. }, _) => "carrier".into(),
        (ShaftLabel::Member { member }, Some(Stage::Shape(shape))) => member_role(shape, member),
        (ShaftLabel::Member { member }, None) => format!("member {}", member + 1),
    }
}

/// **A member's name, as the shape reads it** ([`Shape::member_names`]),
/// in the harness's English: a pair's two members are "first" and
/// "second", a gear with no role but its number is "member n", and a role
/// the shape numbers keeps its number.
fn member_role(shape: &gear_core::train::shape::Shape, member: usize) -> String {
    use gear_core::train::shape::MemberRole;
    let names = shape.member_names();
    let name = names[member];
    let word = match name.role {
        MemberRole::Gear | MemberRole::Worm | MemberRole::Wheel => {
            return if shape.members.len() == 2 {
                ["first", "second"][member].to_string()
            } else {
                format!("member {}", member + 1)
            };
        }
        MemberRole::Sun => "sun",
        MemberRole::Planet => "planet",
        MemberRole::Ring => "ring",
    };
    match name.ordinal {
        None => word.into(),
        Some(n) => format!("{word} {n}"),
    }
}

/// One entry of a case as the harness prints it: a load's two figures at
/// its shaft, or the word a declared port carries.
pub fn entry(l: &gear_core::train::Load) -> String {
    match l.role {
        gear_core::train::LoadRole::Load => format!(
            "{:>10.4} Nm / {:>9.2} rpm at {:<5}",
            l.torque.manual,
            l.speed.manual,
            port(l.at)
        ),
        gear_core::train::LoadRole::Reacted => format!("reacted at {:<5}", port(l.at)),
        gear_core::train::LoadRole::Free => format!("free at {:<5}", port(l.at)),
    }
}

/// A shaft as the harness prints it: `stage.shaft`, one-based both ways as
/// the front end counts.
pub fn port(p: ShaftRef) -> String {
    match p {
        ShaftRef::Ground => "ground".into(),
        ShaftRef::Of { stage, shaft } => format!("{}.{shaft}", stage + 1),
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
