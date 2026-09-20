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
    solve_train, Duty, LoadCase, PairStage, PlanetaryStage, Port, ShaftConstraint, ShaftRef, Stage,
    StageGear, StageResult, Train, TrainResult,
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
        LoadCase::back_driving(0.6),
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
    Stage::spur(s)
}

/// **An arrangement as a train's constraints**: which of a set's shafts is
/// driven and which held, for the set at stage `k`. The set itself carries no
/// arrangement any more — that is a fact about the train, and this is the
/// shape a file writes it in.
fn arranged(k: usize, input: &str, fixed: &str) -> Vec<ShaftConstraint> {
    // The set's shafts, in its wiring's order: ground, sun, carrier, ring,
    // planet. One line each: a hold or a drive on a stage replaces the
    // stage's convention of that kind, so holding the carrier releases the
    // ring without a word about it.
    let shaft = |s: &str| match s {
        "sun" => 1,
        "carrier" => 2,
        _ => 3,
    };
    vec![
        ShaftConstraint::driven(k, shaft(input)),
        ShaftConstraint::held(k, shaft(fixed)),
    ]
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
    let train = |stages: Vec<Stage>| Train {
        load_cases: loads(),
        reversed_bending: false,
        stages,
        couplings: Vec::new(),
        constraints: Vec::new(),
    };
    // ...and one told what to hold and drive, which is how every arrangement
    // but the conventional one is stated now.
    let asked = |stages: Vec<Stage>, constraints: Vec<ShaftConstraint>| Train {
        constraints,
        ..train(stages)
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
                out.push((
                    format!("set-{input}-{fixed}"),
                    asked(vec![set()], arranged(0, input, fixed)),
                ));
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
        // With the large sun driven the small sun and the carrier are both
        // free, and `end` is the first of them — the small sun. The carrier
        // is the output a designer means, so every case loads it by
        // reference — a derived load where it reacts, the given one where
        // it drives — and loads the small sun with a torque of nought, which
        // is how a case says a port turns and carries nothing.
        out.push(("ravigneaux-large-sun".to_string(), {
            let mut t = asked(
                vec![shape(ravigneaux())],
                vec![ShaftConstraint::driven(0, 3)],
            );
            let at = |shaft| Port::At(ShaftRef::Of { stage: 0, shaft });
            let (small_sun, carrier) = (1, 2);
            t.load_cases = t
                .load_cases
                .iter()
                .map(|c| {
                    let mut c = c.clone();
                    for l in &mut c.loads {
                        if l.at == Port::End {
                            l.at = at(carrier);
                        }
                    }
                    if !c.loads.iter().any(|l| l.at == at(carrier)) {
                        c.loads.push(gear_core::train::Load::derived(at(carrier)));
                    }
                    c.loads.push(gear_core::train::Load {
                        at: at(small_sun),
                        torque: gear_core::params::Auto::fixed(0.0),
                        speed: gear_core::params::Auto::automatic(0.0),
                    });
                    c
                })
                .collect();
            t
        }));
        out.push((
            "ravigneaux-ring-free".to_string(),
            asked(vec![shape(ravigneaux())], vec![ShaftConstraint::held(0, 3)]),
        ));
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
    // A set with its carrier held reverses, and in a chain the set at
    // stage `k` is *driven by the coupling*, so only its held shaft is stated;
    // the drive belongs to the first stage's input.
    out.push((
        "set-then-pair".to_string(),
        asked(
            vec![set(), pair(17, 43, 0.0)],
            arranged(0, "sun", "carrier"),
        ),
    ));
    // The set at stage 1 is driven by the coupling, so only its held shaft
    // is stated; the ring it releases follows by rule.
    out.push((
        "pair-then-set".to_string(),
        asked(
            vec![pair(17, 43, 0.0), set()],
            vec![ShaftConstraint::held(1, 2)],
        ),
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
    // **A train that is one condition short, one that has two drives, and
    // one that asks two things of a shaft** — recorded for what each answers,
    // since each used to answer with the wiring sentence. The first is a set
    // with its ring released: no rating, and a *family* under the line, every
    // shaft's speed per turn of the free one. The second drives the sun and
    // the carrier together, which is one motion — the whole set turns as one
    // — and no arrangement to rate. The third holds the carrier and the ring
    // both, so the sun cannot turn.
    let free = |shaft| ShaftConstraint {
        at: ShaftRef::Of { stage: 0, shaft },
        constraint: gear_core::train::Constraint::Free,
    };
    out.push((
        "ring-released".to_string(),
        asked(vec![set()], vec![free(3)]),
    ));
    out.push((
        "two-drives".to_string(),
        asked(
            vec![set()],
            vec![
                ShaftConstraint::driven(0, 1),
                ShaftConstraint::driven(0, 2),
                free(3),
            ],
        ),
    ));
    out.push((
        "conflict".to_string(),
        asked(
            vec![set()],
            vec![ShaftConstraint::held(0, 2), ShaftConstraint::held(0, 3)],
        ),
    ));
    // **The same chain with its loads written at shafts by reference** —
    // the shaft `end` resolves to, which with the carrier held is the set's
    // *ring*, and the pair's first member for `start`. The fixture above and
    // this one must print the same figures, and both are recorded so that
    // the two spellings cannot drift. (A first draft wrote the carrier here
    // by hand and was refused: the carrier is held. That is the reason the
    // names exist.)
    out.push(("pair-then-set-named".to_string(), {
        let mut t = out
            .iter()
            .find(|(name, _)| name == "pair-then-set")
            .map(|(_, t)| t.clone())
            .expect("the fixture above");
        let boundaries = t.boundaries().expect("a chain has boundaries");
        t.load_cases = t
            .load_cases
            .iter()
            .map(|c| {
                let mut c = c.clone();
                for l in &mut c.loads {
                    l.at = Port::At(t.port_shaft(&boundaries, l.at));
                }
                c
            })
            .collect();
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
    println!(
        "  total    ratio {:>14.6}   efficiency {:>10.6} / {:<10.6} %   backlash {:>10.6} / {:<10.6} deg",
        crate::or_nan(r.total_ratio),
        100.0 * crate::ways_or_nan(r.total_efficiency).forward,
        100.0 * crate::ways_or_nan(r.total_efficiency).backward,
        crate::play_or_nan(r.backlash).forward.nominal,
        crate::play_or_nan(r.backlash).backward.nominal,
    );
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
                .map(|l| format!(
                    "{:>10.4} Nm / {:>9.2} rpm at {:<5}",
                    l.torque.manual,
                    l.speed.manual,
                    port(l.at)
                ))
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
                    .map(|r| format!("{r:.6}"))
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

/// A port as the harness prints it: the chain's two names as words, and a
/// named shaft as `stage.shaft`, one-based both ways as the front end counts.
pub fn port(p: Port) -> String {
    match p {
        Port::Start => "start".into(),
        Port::End => "end".into(),
        Port::At(ShaftRef::Ground) => "ground".into(),
        Port::At(ShaftRef::Of { stage, shaft }) => format!("{}.{shaft}", stage + 1),
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
