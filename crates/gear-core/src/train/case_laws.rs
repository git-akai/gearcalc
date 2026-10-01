//! Laws of a load case against its own mirror and against the paths it
//! walks, over the train's one grid ([`super::testing`]).
//!
//! - **The mirror**: every given torque and speed negated is the same
//!   mechanism turning the other way, so every body's torque and speed
//!   negate and nothing a tooth sees moves.
//! - **A path is the case through it**: a path's efficiency and circulation
//!   are what a case loading its one end and reacting its other comes to,
//!   read off that case's own rows; and a path's backward figures are the
//!   reverse path's forward ones.

#![allow(clippy::unwrap_used)]

use super::testing::{grid, SPEED_RPM, TORQUE_NM};
use super::{solve_train, test_library, Duty, LoadCase, Train, TrainResult};

/// Two figures equal to a relative `1e-9`: the flow and the rating are
/// linear in the load's sign, so a mirror moves each by roundings alone.
fn same(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1e-12)
}

/// `t` with every case replaced by the mirror law's four between its
/// chain's two ends: an ultimate load, the same load back-driving from the
/// far end held still, and a fatigue case over an intermittent duty each
/// way round (the second reversing).
fn with_mirror_cases(t: &mut Train) {
    let Some((a, b)) = t.chain_ends() else {
        return;
    };
    let intermittent = |reversing: bool| Duty::Intermittent {
        range_degrees: 25.0,
        at: Some(b),
        actuations: 1000,
        reversing,
    };
    t.load_cases = vec![
        LoadCase::ultimate(a, b, TORQUE_NM, SPEED_RPM),
        LoadCase::back_driving(a, b, TORQUE_NM),
        LoadCase {
            duty: intermittent(false),
            ..LoadCase::fatigue(a, b, TORQUE_NM, SPEED_RPM)
        },
        LoadCase {
            duty: intermittent(true),
            ..LoadCase::fatigue(a, b, TORQUE_NM, SPEED_RPM)
        },
    ];
}

/// `t` turned the other way: every load's torque and speed negated.
fn mirrored(t: &Train) -> Train {
    let mut m = t.clone();
    for c in &mut m.load_cases {
        for l in &mut c.loads {
            l.torque.manual = -l.torque.manual;
            l.speed.manual = -l.speed.manual;
        }
    }
    m
}

/// `what` said into `out` where `a` and `b` differ.
fn differ(out: &mut Vec<String>, what: String, a: f64, b: f64) {
    if !same(a, b) {
        out.push(format!("{what}: {a} vs {b}"));
    }
}

/// Every difference between a solve and its mirror's, each named.
fn mirror_differences(r: &TrainResult, m: &TrainResult) -> Vec<String> {
    let mut out = Vec::new();
    for (c, (x, y)) in r.cases.iter().zip(&m.cases).enumerate() {
        if x.solved != y.solved {
            out.push(format!("case {c}: solved {} vs {}", x.solved, y.solved));
            continue;
        }
        for (p, q) in x.bodies.iter().zip(&y.bodies) {
            differ(
                &mut out,
                format!("case {c} body {} torque", p.at),
                p.torque,
                -q.torque,
            );
            match (p.speed, q.speed) {
                (Some(u), Some(v)) => {
                    differ(&mut out, format!("case {c} body {} speed", p.at), u, -v)
                }
                (u, v) if u.is_some() != v.is_some() => {
                    out.push(format!("case {c} body {}: speed {u:?} vs {v:?}", p.at));
                }
                _ => {}
            }
        }
    }
    for (i, (g, h)) in r.members.iter().zip(&m.members).enumerate() {
        for (x, y) in g.cases.iter().zip(&h.cases) {
            let at = format!("member {i} case {}", x.case);
            differ(
                &mut out,
                format!("{at} |torque|"),
                x.torque.abs(),
                y.torque.abs(),
            );
            differ(&mut out, format!("{at} on body"), x.on_body, -y.on_body);
            differ(&mut out, format!("{at} speed"), x.speed, -y.speed);
            differ(
                &mut out,
                format!("{at} speed against carrier"),
                x.speed_against_carrier,
                -y.speed_against_carrier,
            );
            if x.contact_stress.is_some() != y.contact_stress.is_some() {
                out.push(format!("{at} contact: one sized, one not"));
            }
            if let (Some(a), Some(b)) = (x.contact_stress, y.contact_stress) {
                differ(&mut out, format!("{at} contact"), a, b);
            }
            match (x.bending_stress, y.bending_stress) {
                (Some(u), Some(v)) => differ(&mut out, format!("{at} bending"), u, v),
                (u, v) if u.is_some() != v.is_some() => {
                    out.push(format!("{at}: bending {u:?} vs {v:?}"));
                }
                _ => {}
            }
            match (x.cycles, y.cycles) {
                (Some(u), Some(v)) => {
                    differ(
                        &mut out,
                        format!("{at} bending cycles"),
                        u.bending,
                        v.bending,
                    );
                    differ(
                        &mut out,
                        format!("{at} contact cycles"),
                        u.contact,
                        v.contact,
                    );
                }
                (u, v) if u.is_some() != v.is_some() => {
                    out.push(format!("{at}: cycles {u:?} vs {v:?}"));
                }
                _ => {}
            }
        }
    }
    for (k, (a, b)) in r.meshes.iter().zip(&m.meshes).enumerate() {
        for (x, y) in a.cases.iter().zip(&b.cases) {
            differ(
                &mut out,
                format!("mesh {k} case {} power through", x.case),
                x.power_through,
                y.power_through,
            );
        }
    }
    for (p, q) in r.paths.iter().zip(&m.paths) {
        let at = format!("path {}→{}", p.from, p.to);
        differ(
            &mut out,
            format!("{at} forward"),
            p.efficiency.forward,
            q.efficiency.forward,
        );
        differ(
            &mut out,
            format!("{at} backward"),
            p.efficiency.backward,
            q.efficiency.backward,
        );
    }
    if r.paths.len() != m.paths.len() {
        out.push(format!("{} paths vs {}", r.paths.len(), m.paths.len()));
    }
    out
}

/// **A mirrored case rates as the case.** On every train of the grid,
/// under an ultimate, a back-driving and two intermittent fatigue cases,
/// every given torque and speed negated: each case solves or not as it
/// did; each body's torque and speed negate; each member's torque in size,
/// its stresses and its cycles are unchanged and its speeds negate; each
/// mesh passes the same power; each path's efficiency is unchanged. A
/// load held still takes its direction from the given torques, so the
/// back-driving case is where a rule that reads a still load's torque
/// without its sign turns the flow the wrong way.
#[test]
fn a_mirrored_case_rates_as_the_case() {
    let lib = test_library();
    let (mut compared, mut solved) = (0, 0);
    let mut failures: Vec<String> = Vec::new();
    for e in grid() {
        let mut t = e.train();
        with_mirror_cases(&mut t);
        let r = solve_train(&t, &lib).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        let m = solve_train(&mirrored(&t), &lib)
            .unwrap_or_else(|err| panic!("{} mirrored: {err}", e.name));
        failures.extend(
            mirror_differences(&r, &m)
                .into_iter()
                .map(|d| format!("{}: {d}", e.name)),
        );
        compared += r.cases.len();
        solved += r.cases.iter().filter(|c| c.solved).count();
    }
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
    // Every train of the grid carries the four cases, and all but the
    // back-driving ones solve on every train whose paths back-drive.
    assert_eq!(compared, 4 * grid().len());
    assert!(
        solved >= 3 * grid().len(),
        "only {solved} of {compared} solved"
    );
}

/// **An intermittent sweep counts what running through the same turns
/// counts.** A fatigue case over a sweep of the far end, and the same case
/// run continuously for exactly the time the far end takes to turn through
/// that sweep at the case's speed: every member's cycles agree, both being
/// the one count of the same turns rounded up once, to within the one
/// cycle a rounding of the duration can move. On every train of the grid
/// whose far end turns — so a member's turns against its carrier, a
/// difference of two signed turns, are counted the same way by both.
#[test]
fn an_intermittent_sweep_counts_what_running_through_it_counts() {
    const RANGE_DEG: f64 = 25.0;
    const ACTUATIONS: u32 = 1000;
    let lib = test_library();
    let mut compared = 0;
    for e in grid() {
        let mut t = e.train();
        let Some((a, b)) = t.chain_ends() else {
            continue;
        };
        let swept = LoadCase {
            duty: Duty::Intermittent {
                range_degrees: RANGE_DEG,
                at: Some(b),
                actuations: ACTUATIONS,
                reversing: false,
            },
            ..LoadCase::fatigue(a, b, TORQUE_NM, SPEED_RPM)
        };
        t.load_cases = vec![swept.clone()];
        let r = solve_train(&t, &lib).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        let Some(far) = r.cases[0]
            .shaft(b)
            .and_then(|x| x.speed)
            .filter(|s| *s != 0.0)
        else {
            continue;
        };
        let revolutions = RANGE_DEG / 360.0 * f64::from(ACTUATIONS);
        t.load_cases = vec![LoadCase {
            duty: Duty::Continuous {
                runtime_hours: revolutions / (far.abs() * 60.0),
            },
            ..swept
        }];
        let c = solve_train(&t, &lib).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        for (i, (x, y)) in r.members.iter().zip(&c.members).enumerate() {
            let (Some(u), Some(v)) = (x.cases[0].cycles, y.cases[0].cycles) else {
                panic!("{}: member {i} counts no cycles", e.name);
            };
            assert!(
                (u.bending - v.bending).abs() <= 1.0,
                "{}: member {i}: swept {} vs running {}",
                e.name,
                u.bending,
                v.bending
            );
            compared += 1;
        }
    }
    assert!(compared > grid().len(), "only {compared} members compared");
}

/// **A path's figures are the case through it.** On every train of the
/// grid, two cases between the chain's two ends, one each way. Each
/// path's forward efficiency is the power its case delivers at the far
/// end over the power it takes in, read off the case's own body rows —
/// where the path breaks away, since one that cannot start reads nought
/// whatever it would do turning — and its forward circulation is the
/// power its case passes through every mesh, summed. And each path's
/// backward figures are the reverse path's forward ones.
#[test]
fn a_paths_figures_are_the_case_through_it() {
    let lib = test_library();
    let (mut read, mut moving, mut reversed) = (0, 0, 0);
    let mut failures: Vec<String> = Vec::new();
    for e in grid() {
        let mut t = e.train();
        let Some((a, b)) = t.chain_ends() else {
            continue;
        };
        t.load_cases = vec![
            LoadCase::ultimate(a, b, TORQUE_NM, SPEED_RPM),
            LoadCase::ultimate(b, a, TORQUE_NM, SPEED_RPM),
        ];
        let r = solve_train(&t, &lib).unwrap_or_else(|err| panic!("{}: {err}", e.name));
        let path = |from: usize, to: usize| r.paths.iter().find(|p| (p.from, p.to) == (from, to));
        let mut differ = |what: &str, x: f64, y: f64| {
            if !same(x, y) {
                failures.push(format!("{}: {what}: {x} vs {y}", e.name));
            }
        };
        for (case, (from, to)) in [(a, b), (b, a)].into_iter().enumerate() {
            let (Some(p), c) = (path(from, to), &r.cases[case]) else {
                continue;
            };
            if !c.solved {
                continue;
            }
            let power = |at: usize| {
                let x = c.shaft(at).unwrap();
                (x.torque * x.speed.unwrap()).abs()
            };
            if p.efficiency.forward > 0.0 {
                differ(
                    &format!("{from}→{to} efficiency against its case"),
                    p.efficiency.forward,
                    power(to) / power(from),
                );
                moving += 1;
            }
            let through: f64 = r.meshes.iter().map(|m| m.cases[case].power_through).sum();
            differ(
                &format!("{from}→{to} circulation against its case"),
                p.circulation.forward,
                through,
            );
            read += 1;
        }
        if let (Some(p), Some(q)) = (path(a, b), path(b, a)) {
            differ(
                "backward efficiency",
                p.efficiency.backward,
                q.efficiency.forward,
            );
            differ(
                "backward circulation",
                p.circulation.backward,
                q.circulation.forward,
            );
            for (x, y) in [
                (p.backlash.backward.nominal, q.backlash.forward.nominal),
                (p.backlash.backward.minimum, q.backlash.forward.minimum),
                (p.backlash.backward.maximum, q.backlash.forward.maximum),
            ] {
                differ("backward backlash", x, y);
            }
            reversed += 1;
        }
    }
    assert!(
        failures.is_empty(),
        "{} differences:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(read > grid().len(), "only {read} cases read");
    assert!(moving > grid().len(), "only {moving} efficiencies read");
    assert!(
        reversed > grid().len() / 2,
        "only {reversed} paths reversed"
    );
}

// ------------------------------------------------ what a case must say ---

/// Whether a case's notes carry `key`.
fn says(c: &super::TrainCase, key: &str) -> bool {
    c.notes.iter().any(|n| n.key == key)
}

/// A train with one case, a fatigue load between its chain's two ends.
fn one_fatigue_case(mut t: Train) -> Train {
    let (a, b) = t.chain_ends().unwrap();
    t.load_cases = vec![LoadCase::fatigue(a, b, TORQUE_NM, SPEED_RPM)];
    t
}

/// A set with its ring released and held still by its case — a load
/// given no speed and no torque at the ring, the sun driven, the carrier
/// reacted: the ring is an open port no motion of the case turns.
fn ring_held_by_its_case() -> Train {
    use super::arrangements::Preset;
    use super::testing::cased;
    use super::{Edit, Load, LoadRole};
    let mut t = cased(vec![Preset::Planetary.build()]);
    let (sun, carrier, ring) = (t.port(0, 1), t.port(0, 2), t.port(0, 3));
    t.edit(Edit::Release(ring)).unwrap();
    t.load_cases = vec![LoadCase {
        loads: vec![
            Load::given(sun, TORQUE_NM, SPEED_RPM),
            Load {
                speed: crate::params::Auto::fixed(0.0),
                ..Load::derived(ring)
            },
            Load::declared(carrier, LoadRole::Reacted),
        ],
        ..LoadCase::fatigue(sun, carrier, TORQUE_NM, SPEED_RPM)
    }];
    t
}

/// **A sweep is measured at an open port that turns, or the case says
/// why** (audit T13.5). Every preset, cased between its chain's two ends,
/// and a set whose ring its case holds still ([`ring_held_by_its_case`]),
/// its fatigue sweep put at every body — ground and one past the last
/// included: the solve refuses the case by the key that names a sweep at
/// no open port, or the case says its sweep stands still, or every member
/// counts cycles. It never panics, and each of the three is met. Ground and
/// one past the last are no body a case can name, and the input table
/// refuses them first, by the field (`input::DUTY`).
#[test]
fn a_sweep_at_any_body_counts_refuses_or_says_why() {
    use super::arrangements::Preset;
    use super::testing::cased;
    let lib = test_library();
    let (mut refused, mut still, mut counted) = (0, 0, 0);
    let mut failures: Vec<String> = Vec::new();
    let trains = Preset::ALL
        .into_iter()
        .map(|p| (format!("{p:?}"), one_fatigue_case(cased(vec![p.build()]))))
        .chain([(
            "a ring held by its case".to_owned(),
            ring_held_by_its_case(),
        )]);
    for (p, base) in trains {
        for at in 0..=base.max_body() + 1 {
            let mut t = base.clone();
            t.load_cases[0].duty = Duty::intermittent(Some(at));
            let solved = std::panic::catch_unwind(|| solve_train(&t, &lib));
            let context = format!("{p}, sweep at {at}");
            match solved {
                Err(_) => failures.push(format!("{context}: panicked")),
                Ok(Err(e)) => {
                    let key = crate::note::Explain::note(&e).key;
                    let outside = at == 0 || at > base.max_body();
                    if key == "error.train_duty_port" && !outside {
                        refused += 1;
                    } else if !(outside && key == "error.input_out_of_range") {
                        failures.push(format!("{context}: refused as {key}"));
                    }
                }
                Ok(Ok(r)) if says(&r.cases[0], "train.duty_at_still") => {
                    if r.members.iter().any(|m| m.cases[0].cycles.is_some()) {
                        failures.push(format!("{context}: still, yet counted"));
                    }
                    still += 1;
                }
                Ok(Ok(r)) => {
                    if !r.cases[0].solved {
                        continue;
                    }
                    let zero = r
                        .members
                        .iter()
                        .any(|m| m.cases[0].cycles.is_none_or(|c| c.bending <= 0.0));
                    if zero {
                        failures.push(format!("{context}: a member counts nothing, unsaid"));
                    }
                    counted += 1;
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        refused > 0 && still > 0 && counted > Preset::ALL.len(),
        "refused {refused}, still {still}, counted {counted}"
    );
}

/// **Two entries of one case at one body are refused** by the key that
/// names the body (audit T13.5): two given speeds at one shaft summed to
/// twice either, and of two torques the last silently won.
#[test]
fn two_entries_of_one_case_at_one_body_are_refused() {
    use super::arrangements::Preset;
    use super::testing::cased;
    use super::{Load, LoadRole};
    let lib = test_library();
    let mut t = cased(vec![Preset::Spur.build()]);
    let (given, reacted) = (
        Load::given(1, TORQUE_NM, SPEED_RPM),
        Load::declared(2, LoadRole::Reacted),
    );
    // The two entries side by side, and apart with another between — a
    // check of neighbours alone sees only the first — and two of different
    // roles at one body, which a check of like roles alone would pass.
    let at_the_load = Load::declared(1, LoadRole::Reacted);
    let mut refused = 0;
    for loads in [
        vec![given, given, reacted],
        vec![given, reacted, given],
        vec![given, at_the_load, reacted],
    ] {
        t.load_cases = vec![LoadCase {
            loads,
            ..LoadCase::ultimate(1, 2, TORQUE_NM, SPEED_RPM)
        }];
        let key = solve_train(&t, &lib)
            .map(|_| ())
            .map_err(|e| crate::note::Explain::note(&e));
        assert!(
            matches!(&key, Err(n) if n.key == "error.train_duplicate_entry"
                && n.values.get("body").map(String::as_str) == Some("1")),
            "{:?}: {key:?}",
            t.load_cases[0].loads
        );
        refused += 1;
    }
    assert_eq!(refused, 3);
    // One entry fewer is the case it was meant to be.
    t.load_cases[0].loads.remove(1);
    assert!(solve_train(&t, &lib).unwrap().cases[0].solved);
}

/// **A case switched off fails alone** (audit T13.5): an entry at a body
/// no load can enter by — a body of the train that is no open port, a
/// planet — in a case that is off leaves it unsolved with the refusal as
/// its note, and the train and its other case solve as they did; switched
/// on, it refuses the train by name.
#[test]
fn a_switched_off_case_at_no_port_fails_alone() {
    use super::arrangements::Preset;
    use super::testing::cased;
    use super::Load;
    let lib = test_library();
    let t = cased(vec![Preset::Planetary.build()]);
    let ports: Vec<usize> = t.open_ports().iter().map(|p| p.body).collect();
    let no_port = (1..=t.max_body())
        .find(|b| !ports.contains(b))
        .expect("a planetary set has a body that is no port");
    let alone = solve_train(&t, &lib).unwrap();
    let mut u = t.clone();
    u.load_cases.push(LoadCase {
        enabled: false,
        loads: vec![Load::given(no_port, TORQUE_NM, SPEED_RPM)],
        ..LoadCase::ultimate(1, 2, TORQUE_NM, SPEED_RPM)
    });
    let r = solve_train(&u, &lib).unwrap_or_else(|e| panic!("the train refused: {e}"));
    let off = &r.cases[t.load_cases.len()];
    assert!(!off.solved && says(off, "error.train_load_port"), "{off:?}");
    for (x, y) in alone.cases.iter().zip(&r.cases) {
        assert_eq!(
            format!("{x:?}"),
            format!("{y:?}"),
            "a case that is off moved another"
        );
    }
    u.load_cases.last_mut().unwrap().enabled = true;
    assert!(solve_train(&u, &lib).is_err(), "switched on, it refuses");
}

/// **A sweep with no entry to follow is unset, and the case says so**
/// (audit T13.5): a case with no entries switched to an intermittent duty,
/// then given a load and a reaction, counts no cycles and says its sweep
/// is unset — where it was measured at ground and counted nought, saying
/// nothing.
#[test]
fn a_sweep_with_no_entry_to_follow_is_unset_and_said() {
    use super::arrangements::Preset;
    use super::testing::cased;
    use super::{Load, LoadRole};
    let lib = test_library();
    let mut t = cased(vec![Preset::Spur.build()]);
    t.load_cases = vec![LoadCase {
        loads: Vec::new(),
        duty: Duty::Continuous {
            runtime_hours: 1000.0,
        },
        ..LoadCase::fatigue(1, 2, TORQUE_NM, SPEED_RPM)
    }];
    t.set_duty(0, true).unwrap();
    t.load_cases[0].loads = vec![
        Load::given(1, TORQUE_NM, SPEED_RPM),
        Load::declared(2, LoadRole::Reacted),
    ];
    let r = solve_train(&t, &lib).unwrap();
    assert!(says(&r.cases[0], "train.duty_unset"), "{:?}", r.cases[0]);
    assert!(r.members.iter().all(|m| m.cases[0].cycles.is_none()));
}

/// **A fresh case's figures are what their basis says** (audit T13.6):
/// the ultimate case a stall torque, positive and finite, at a running
/// speed, positive and finite; the fatigue case a fifth of that torque —
/// a running load rather than the peak — at the same speed; and
/// `LoadCase::fresh` builds each kind at its figures, switched on, loaded
/// at its input and reacted at its output, the sweep measured there.
#[test]
fn a_fresh_cases_figures_are_their_basis() {
    use super::{CaseKind, Load, LoadRole};
    let (stall, speed) = CaseKind::Ultimate.fresh_figures();
    let (running, same) = CaseKind::Fatigue.fresh_figures();
    assert!(stall > 0.0 && stall.is_finite(), "{stall}");
    assert!(speed > 0.0 && speed.is_finite(), "{speed}");
    assert_eq!(running, stall / 5.0, "a fifth of the stall");
    assert_eq!(same, speed, "one speed");
    for kind in CaseKind::BOTH {
        let (torque, speed) = kind.fresh_figures();
        let c = LoadCase::fresh(kind, 1, 2);
        assert_eq!(c.kind, kind);
        assert!(c.enabled);
        assert_eq!(
            c.loads,
            vec![
                Load::given(1, torque, speed),
                Load::declared(2, LoadRole::Reacted)
            ]
        );
        assert_eq!(c.duty, Duty::intermittent(Some(2)));
    }
}

/// **Switched to intermittent, a case's sweep is seeded at its reaction**
/// (`Train::set_duty`): the body a designer's reaction names is where the
/// output's travel is known, whichever order the case lists its entries
/// in; with no reaction, at its first entry; with none, unset.
#[test]
fn a_sweep_is_seeded_at_the_cases_reaction() {
    use super::arrangements::Preset;
    use super::testing::cased;
    use super::{Load, LoadRole};
    let (load, reaction) = (
        Load::given(1, TORQUE_NM, SPEED_RPM),
        Load::declared(2, LoadRole::Reacted),
    );
    let mut seeded = 0;
    for (loads, at) in [
        (vec![load, reaction], Some(2)),
        (vec![reaction, load], Some(2)),
        (vec![load], Some(1)),
        (Vec::new(), None),
    ] {
        let mut t = cased(vec![Preset::Spur.build()]);
        t.load_cases = vec![LoadCase {
            loads: loads.clone(),
            duty: Duty::Continuous {
                runtime_hours: 1000.0,
            },
            ..LoadCase::fatigue(1, 2, TORQUE_NM, SPEED_RPM)
        }];
        t.set_duty(0, true).unwrap();
        assert_eq!(t.load_cases[0].duty, Duty::intermittent(at), "{loads:?}");
        seeded += 1;
    }
    assert_eq!(seeded, 4);
}
