//! Laws of a figure that was not computed (T02.2) and of given speeds
//! judged by rank (T02.3).
//!
//! - **Nought is a lock and nothing else**: a path's efficiency is absent
//!   where its flow was refused, and nought only where a mesh on the train
//!   locks that way.
//! - **A loop of meshes is named as one**: twin countershafts and a pair
//!   doubled close a loop a torque goes round, which the case says as
//!   `train.mesh_loop`, never as a load held at both ends.
//! - **Doubling a mesh never lowers a path's play**: it is absent, or no
//!   less than the single mesh's.
//! - **Given speeds by rank**: two that agree with the ratio solve; two
//!   that disagree name the second; a count of speeds short is never 0 and
//!   never a sentinel.

#![allow(clippy::unwrap_used)]

use super::arrangements::{self as arr, Builder, Preset};
use super::sweep::cased;
use super::testing::grid;
use super::{solve_train, test_library, Load, LoadCase, Train, TrainResult};
use crate::note::key;
use crate::params::Auto;

/// The input 17 to a 43 on each layshaft, the 19 on each to the output 41:
/// input 1, output 2, the layshafts 3 and 4. `twins` layshafts, one or two.
fn countershafts(twins: usize) -> Train {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let input = b.body(centre);
    let output = b.body(centre);
    let driving = b.gear(input, 17);
    let driven = b.gear(output, 41);
    for _ in 0..twins {
        let lay = b.axis();
        let shaft = b.body(lay);
        let a = b.gear(shaft, 43);
        b.mesh(driving, a);
        let c = b.gear(shaft, 19);
        b.mesh(driven, c);
        b.distance([centre, lay]);
    }
    cased(vec![b.build()])
}

/// The layshaft with its output pair `pairs` times over, between the same
/// two bodies: input 1, output 2, layshaft 3.
fn doubled_output(pairs: usize) -> Train {
    let mut b = Builder::new(1.0);
    let centre = b.axis();
    let lay = b.axis();
    let input = b.body(centre);
    let output = b.body(centre);
    let shaft = b.body(lay);
    let driving = b.gear(input, 17);
    let a = b.gear(shaft, 43);
    b.mesh(driving, a);
    for _ in 0..pairs {
        let z = b.gear(output, 41);
        let c = b.gear(shaft, 19);
        b.mesh(z, c);
    }
    b.distance([centre, lay]);
    cased(vec![b.build()])
}

fn path(r: &TrainResult, from: usize, to: usize) -> &super::PathReport {
    r.paths
        .iter()
        .find(|p| (p.from, p.to) == (from, to))
        .unwrap()
}

/// **Twin countershafts are a loop of meshes, not a lock** (T02.2,
/// kinematics-flow#2): the same ratio as one layshaft; an efficiency that
/// is absent or the single branch's, never nought; a play absent or no less
/// than one branch's; and the case says the loop, naming its four meshes.
#[test]
fn twin_countershafts_are_a_mesh_loop_and_never_read_as_locked() {
    let lib = test_library();
    let one = solve_train(&countershafts(1), &lib).unwrap();
    let two = solve_train(&countershafts(2), &lib).unwrap();
    let (p1, p2) = (path(&one, 1, 2), path(&two, 1, 2));
    assert!((p1.ratio - 5.458_204).abs() < 1e-6, "{}", p1.ratio);
    assert!((p2.ratio - p1.ratio).abs() < 1e-12);
    let single = p1.efficiency.forward.unwrap();
    assert!((single - 0.968_084).abs() < 1e-6, "{single}");
    for e in [p2.efficiency.forward, p2.efficiency.backward] {
        assert!(
            e.is_none_or(|e| (e - single).abs() < 1e-9),
            "twin efficiency {e:?}"
        );
    }
    let b1 = p1.backlash.forward.unwrap().nominal;
    assert!(
        p2.backlash.forward.is_none_or(|b| b.nominal >= b1 - 1e-12),
        "{:?} against {b1}",
        p2.backlash.forward
    );
    let mut named = 0;
    for c in &two.cases {
        assert!(
            !c.notes.iter().any(|n| n.is(key::TRAIN_LOAD_SHARED)),
            "{:?}",
            c.notes
        );
        let looped = c.notes.iter().find(|n| n.is(key::TRAIN_MESH_LOOP)).unwrap();
        assert_eq!(looped.values["meshes"], "1, 2, 3, 4");
        named += 1;
    }
    assert_eq!(named, 2);
}

/// **A pair doubled loses none of its play** (T02.2): the doubled output
/// mesh's path reads no less play than the single mesh's — absent, or at
/// least it — where it once read 69 % less.
#[test]
fn a_doubled_mesh_never_lowers_the_play() {
    let lib = test_library();
    let one = solve_train(&doubled_output(1), &lib).unwrap();
    let two = solve_train(&doubled_output(2), &lib).unwrap();
    let (p1, p2) = (path(&one, 1, 2), path(&two, 1, 2));
    for (single, doubled) in [
        (p1.backlash.forward, p2.backlash.forward),
        (p1.backlash.backward, p2.backlash.backward),
    ] {
        let single = single.unwrap();
        assert!(
            doubled.is_none_or(|d| d.nominal >= single.nominal - 1e-12),
            "{doubled:?} against {single:?}"
        );
    }
    assert!(p2.efficiency.forward.is_none_or(|e| e > 0.0));
}

/// **Doubling any mesh of any preset never lowers a path's play** (T02.2):
/// each mesh's first member given a second mate on its mate's body; where
/// the doubled train still has the path, its play is absent or no less. A
/// doubled mesh closes a loop, whose play is not computed: absent, today,
/// on every preset.
#[test]
fn doubling_any_mesh_never_lowers_the_play() {
    let lib = test_library();
    let mut compared = 0;
    for preset in Preset::ALL {
        let t = cased(vec![preset.build()]);
        let Ok(r) = solve_train(&t, &lib) else {
            continue;
        };
        for k in 0..t.shape.meshes.len() {
            let m = t.shape.meshes[k];
            let on = t.shape.members[m.b].body;
            let mut u = t.clone();
            if u.edit(super::Edit::AddGear {
                mate: m.a,
                on: super::Place::Body(on),
                ring: t.shape.members[m.b].ring.is_some(),
            })
            .is_err()
            {
                continue;
            }
            let Ok(s) = solve_train(&u, &lib) else {
                continue;
            };
            for p in &r.paths {
                let Some(q) = s.paths.iter().find(|q| (q.from, q.to) == (p.from, p.to)) else {
                    continue;
                };
                for (was, now) in [
                    (p.backlash.forward, q.backlash.forward),
                    (p.backlash.backward, q.backlash.backward),
                ] {
                    compared += 1;
                    let (Some(was), Some(now)) = (was, now) else {
                        continue;
                    };
                    assert!(
                        now.minimum.unwrap()
                            >= was.minimum.unwrap() - 1e-9 * was.minimum.unwrap().abs().max(1.0),
                        "{preset:?} mesh {k}: {now:?} below {was:?}"
                    );
                }
            }
        }
    }
    assert!(compared > 10, "only {compared} compared");
}

/// **A path's figure is absent only where its flow was refused, and
/// nought only where it was not** (T02.2): over the grid, a path with no
/// efficiency is one a case loading it could not divide (`load_shared` or
/// `mesh_loop` on that case), and a path at nought is one whose cases
/// solved — a lock, never a refusal read as one.
#[test]
fn an_absent_efficiency_is_a_refused_flow_and_nought_a_lock() {
    let lib = test_library();
    let (mut absent, mut noughts, mut paths) = (0, 0, 0);
    let refused =
        |n: &crate::note::Note| n.is(key::TRAIN_LOAD_SHARED) || n.is(key::TRAIN_MESH_LOOP);
    let mut trains: Vec<(String, Train)> =
        grid().iter().map(|e| (e.name.clone(), e.train())).collect();
    trains.push(("twin countershafts".into(), countershafts(2)));
    for (name, t) in &trains {
        let Ok(r) = solve_train(t, &lib) else {
            continue;
        };
        for p in &r.paths {
            let on_path: Vec<_> = r
                .cases
                .iter()
                .filter(|c| {
                    let loads = &t.load_cases[c.case].loads;
                    loads.iter().any(|l| l.at == p.from) && loads.iter().any(|l| l.at == p.to)
                })
                .collect();
            for x in [p.efficiency.forward, p.efficiency.backward] {
                match x {
                    None => {
                        assert!(
                            on_path.iter().any(|c| c.notes.iter().any(refused)),
                            "{name}: {}→{} absent with no refused flow",
                            p.from,
                            p.to
                        );
                        absent += 1;
                    }
                    Some(0.0) => {
                        assert!(
                            !on_path.iter().any(|c| c.notes.iter().any(refused)),
                            "{name}: {}→{} nought where the flow was refused",
                            p.from,
                            p.to
                        );
                        noughts += 1;
                    }
                    Some(_) => {}
                }
            }
            paths += 1;
        }
    }
    assert!(
        paths > 30 && absent > 0 && noughts > 0,
        "paths {paths}, absent {absent}, noughts {noughts}"
    );
}

/// A pair 17/43, loaded at 1 with `speeds` given at 1 and 2.
fn pair_given(speeds: [f64; 2]) -> Train {
    Train::chained(vec![arr::pair([17, 43])], |_| {
        vec![LoadCase {
            loads: vec![
                Load::given(1, 2.0, speeds[0]),
                Load {
                    speed: Auto::fixed(speeds[1]),
                    ..Load::derived(2)
                },
            ],
            ..LoadCase::ultimate(1, 2, 2.0, speeds[0])
        }]
    })
}

/// **Two speeds that agree are one condition said twice** (T02.3): a pair
/// given 4300 and −1700 rpm, which 17/43 turns at exactly, solves; given
/// −1699 at the far end it is refused naming that end; never a count of
/// `u32::MAX` speeds short.
#[test]
fn given_speeds_are_judged_by_rank() {
    let lib = test_library();
    let r = solve_train(&pair_given([4300.0, -1700.0]), &lib).unwrap();
    assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);
    let at = |b: usize| r.cases[0].shaft(b).unwrap().speed.unwrap();
    assert_eq!((at(1), at(2)), (4300.0, -1700.0));

    let r = solve_train(&pair_given([4300.0, -1699.0]), &lib).unwrap();
    assert!(!r.cases[0].solved);
    let over = r.cases[0]
        .notes
        .iter()
        .find(|n| n.is(key::TRAIN_CASE_OVERDETERMINED))
        .unwrap();
    assert_eq!(over.values["body"], "2");
}

/// A set with its ring released, loaded at its sun, given `speeds` at the
/// sun, the carrier and the ring where each is `Some`.
fn set_given(speeds: [Option<f64>; 3]) -> Train {
    let mut t = Train::chained(vec![arr::planetary(12, 30, 72, 3)], |_| Vec::new());
    t.edit(super::Edit::Release(3)).unwrap();
    let mut loads = vec![Load::given(1, 2.0, speeds[0].unwrap())];
    for (b, s) in [(2, speeds[1]), (3, speeds[2])] {
        loads.push(match s {
            Some(s) => Load {
                speed: Auto::fixed(s),
                ..Load::derived(b)
            },
            None => Load::declared(b, super::LoadRole::Reacted),
        });
    }
    t.load_cases = vec![LoadCase {
        loads,
        ..LoadCase::ultimate(1, 2, 2.0, 700.0)
    }];
    t
}

/// **A set given all three speeds solves where they agree** (T02.3): with
/// its ring released the set has two freedoms, and Willis at 12/72 turns
/// the sun at 700 for a carrier at 100 and a ring at rest — once read as
/// `short: 0`. A ring given 1 rpm disagrees and is named; the sun alone
/// is one speed short, said as one.
#[test]
fn a_set_given_every_speed_solves_where_they_agree() {
    let lib = test_library();
    let r = solve_train(&set_given([Some(700.0), Some(100.0), Some(0.0)]), &lib).unwrap();
    assert!(r.cases[0].solved, "{:?}", r.cases[0].notes);

    let r = solve_train(&set_given([Some(700.0), Some(100.0), Some(1.0)]), &lib).unwrap();
    let over = r.cases[0]
        .notes
        .iter()
        .find(|n| n.is(key::TRAIN_CASE_OVERDETERMINED))
        .unwrap();
    assert_eq!(over.values["body"], "3");

    let r = solve_train(&set_given([Some(700.0), None, None]), &lib).unwrap();
    let short = r.cases[0]
        .notes
        .iter()
        .find(|n| n.is(key::TRAIN_CASE_UNDERDETERMINED))
        .unwrap();
    assert_eq!(short.values["short"], "1");
}

/// **No note says a count of nought short, or a sentinel's** (T02.3), over
/// the grid and the fixtures above.
#[test]
fn no_count_short_is_nought_or_a_sentinel() {
    let lib = test_library();
    let mut trains: Vec<Train> = grid().iter().map(super::sweep::Entry::train).collect();
    trains.push(set_given([Some(700.0), None, None]));
    trains.push(pair_given([4300.0, -1699.0]));
    let mut read = 0;
    for t in &trains {
        let Ok(r) = solve_train(t, &lib) else {
            continue;
        };
        for n in r.cases.iter().flat_map(|c| &c.notes) {
            if let Some(short) = n.values.get("short") {
                let short: u64 = short.parse().unwrap();
                assert!(short > 0 && short < u64::from(u32::MAX), "{n:?}");
                read += 1;
            }
        }
    }
    assert!(read >= 1, "no count short was read");
}

/// One unit in the last place of a double.
fn ulp(x: f64) -> f64 {
    let a = x.abs();
    a.next_up() - a
}

/// **A given speed agrees to the precision it was given in, and no
/// further** (T02.3, rule 2): 1000 rpm in with its output given as the
/// double `1000 · (−17/43)` makes, and that ±1 ulp, solve; a disagreement
/// ten times what the two speeds carry — `ulp(out) + 17/43 · ulp(in)` — is
/// refused at the output; and the near miss a reader at single precision
/// would pass, the output read through an `f32`, is refused.
#[test]
fn a_given_speed_agrees_to_its_own_precision() {
    let lib = test_library();
    let solves = |out: f64| {
        let r = solve_train(&pair_given([1000.0, out]), &lib).unwrap();
        r.cases[0].solved
    };
    let out = 1000.0 * (-17.0 / 43.0);
    assert!(solves(out), "its own double");
    assert!(
        solves(out.next_up()) && solves(out.next_down()),
        "a ulp either way"
    );
    let carried = ulp(out) + 17.0 / 43.0 * ulp(1000.0);
    assert!(!solves(out + 10.0 * carried), "ten times what they carry");
    assert!(!solves(out - 10.0 * carried), "ten times what they carry");
    #[allow(clippy::cast_possible_truncation)]
    let single = f64::from(out as f32);
    assert!(single != out, "the plant moves the figure");
    assert!(!solves(single), "a figure read at single precision");
}

/// **A count that is whole reads whole** (T02.3 with the cycles): two
/// meshed-planet sets in a chain, driven at 2400 rpm for 1000 hours, whose
/// planets turn at −800/3 rpm against their carrier, count 48,000,000
/// cycles — never one
/// more for a speed rounded above its true size — and every count is a
/// whole number.
#[test]
fn a_whole_count_of_cycles_reads_whole() {
    let lib = test_library();
    let mut t = Train::chained(
        vec![Preset::MeshedPlanets.build(), Preset::MeshedPlanets.build()],
        |_| Vec::new(),
    );
    let mut case = LoadCase {
        duty: super::Duty::Continuous {
            runtime_hours: 1000.0,
        },
        ..t.fresh_case(super::CaseKind::Fatigue, 2.0, 2400.0)
    };
    case.enabled = true;
    t.load_cases = vec![case];
    let r = solve_train(&t, &lib).unwrap();
    let mut counts = Vec::new();
    for g in &r.members {
        for c in &g.cases {
            if let Some(n) = c.cycles {
                assert_eq!(n.bending, n.bending.round(), "{n:?}");
                counts.push(n.bending);
            }
        }
    }
    assert!(counts.contains(&48_000_000.0), "{counts:?}");
    assert!(!counts.contains(&48_000_001.0), "{counts:?}");
}
