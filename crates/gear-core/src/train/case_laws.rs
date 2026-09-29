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
        at: b,
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
            differ(
                &mut out,
                format!("{at} contact"),
                x.contact_stress,
                y.contact_stress,
            );
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
                at: b,
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
