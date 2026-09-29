//! Laws of the rating layer: what a stress is judged against, and what an
//! input that should move one figure moves.

#![allow(clippy::unwrap_used)]

use super::arrangements::{pair, Preset};
use super::{solve_train, test_library, CaseKind, Train};
use crate::params::Auto;

/// Every preset alone, each member at a given width so a factor on the load
/// moves the stresses rather than the width they are taken at.
fn at_given_width(shape: &super::Shape, width: f64) -> super::Shape {
    let mut s = shape.clone();
    for m in &mut s.members {
        m.gear.face_width = Auto::fixed(width);
    }
    s
}

fn relative(a: f64, b: f64) -> f64 {
    (a - b).abs() / b.abs()
}

/// `y / x`, where a mesh that carries nothing — a layshaft's idle ratio —
/// must still carry nothing.
fn ratio(x: f64, y: f64) -> Option<f64> {
    if x == 0.0 {
        assert_eq!(y, 0.0, "a load on a mesh that carried none");
        None
    } else {
        Some(y / x)
    }
}

/// A solve of `shape` alone at `torque`, every case under application
/// factor `k`.
fn with_factor(shape: &super::Shape, torque: f64, k: f64) -> Option<super::TrainResult> {
    let mut t = Train::alone(shape, torque, 100.0);
    for c in &mut t.load_cases {
        c.application_factor = k;
    }
    solve_train(&t, &test_library()).ok()
}

/// **An application factor multiplies the rated force and nothing else.**
/// Bending is linear in the force, so it scales by `K_A`; a line contact's
/// pressure goes as the root of the force and a point contact's as the cube
/// root, so contact scales by `√K_A` or `∛K_A`. Both minimum widths scale by
/// `K_A`. The torque a member reports is the torque entered, unscaled. On
/// every preset.
#[test]
fn an_application_factor_scales_bending_by_itself_and_contact_by_its_root() {
    const K: f64 = 1.25;
    const TOL: f64 = 1e-12;
    let mut rated = 0;
    for p in Preset::ALL {
        let s = at_given_width(&p.build(), 10.0);
        let (Some(one), Some(more)) = (with_factor(&s, 2.0, 1.0), with_factor(&s, 2.0, K)) else {
            panic!("{p:?} does not solve");
        };
        for (a, b) in one.members.iter().zip(&more.members) {
            for (ca, cb) in a.cases.iter().zip(&b.cases) {
                assert_eq!(
                    ca.torque, cb.torque,
                    "{p:?}: the torque entered is reported"
                );
                assert_eq!(ca.bending_stress.is_some(), cb.bending_stress.is_some());
                if let (Some(x), Some(y)) = (ca.bending_stress, cb.bending_stress) {
                    if let Some(r) = ratio(x, y) {
                        assert!(relative(r, K) < TOL, "{p:?}: bending ×{r}");
                        rated += 1;
                    }
                }
                if let Some(c) = ratio(ca.contact_stress, cb.contact_stress) {
                    assert!(
                        relative(c, K.sqrt()) < TOL || relative(c, K.cbrt()) < TOL,
                        "{p:?}: contact ×{c}"
                    );
                }
                for (x, y) in [
                    (ca.min_face_width.bending, cb.min_face_width.bending),
                    (ca.min_face_width.contact, cb.min_face_width.contact),
                ] {
                    if let Some(r) = x.zip(y).and_then(|(x, y)| ratio(x, y)) {
                        assert!(relative(r, K) < TOL, "{p:?}: width ×{r}");
                    }
                }
            }
        }
    }
    assert!(
        rated > 0,
        "no bending stress was compared: the law is vacuous"
    );
}

/// **The canary pair at `K_A = 1.25` fails its 10 mm face.** The unshifted
/// 17/43 pair at 2 N·m, its flank judged at 750 MPa, asks 8.483 mm of the
/// pinion for fatigue contact at `K_A = 1`; a light application factor asks
/// 1.25 times that, past the 10 mm the preset gives it. (`gear-cli strength
/// 17 43 2.0` builds its pair with the library's default tooth rather than
/// the preset's.) The flank figure is pinned by override: the finding is
/// about the factor, and the library's estimate is its own question.
#[test]
fn the_canary_at_a_light_application_factor_asks_more_than_its_face() {
    let mut s = pair([17, 43]);
    for m in &mut s.members {
        m.gear.profile_shift = Auto::fixed(0.0);
        m.gear.material_overrides.contact_fatigue_allowable = Some(750.0);
    }
    let fatigue = |k: f64| {
        let r = with_factor(&s, 2.0, k).unwrap();
        let t = Train::alone(&s, 2.0, 100.0);
        let i = t
            .load_cases
            .iter()
            .position(|c| c.kind == CaseKind::Fatigue)
            .unwrap();
        r.members[0].cases[i].min_face_width.contact.unwrap()
    };
    let (one, light) = (fatigue(1.0), fatigue(1.25));
    assert!((one - 8.483).abs() < 5e-4, "K_A 1: {one}");
    assert!((light / one - 1.25).abs() < 1e-12, "K_A 1.25: {light}");
    assert!(light > 10.0, "K_A 1.25 asks {light}, within the face");
}

/// Every bending figure a result reports is absent or a positive number —
/// a stress, and the minimum width inverted from it — or zero on a member
/// that carries nothing (a layshaft's idle pair).
fn bending_is_absent_or_positive(r: &super::TrainResult, label: &str) -> usize {
    let mut rated = 0;
    for (i, g) in r.members.iter().enumerate() {
        for c in &g.cases {
            for (what, v) in [
                ("stress", c.bending_stress),
                ("min width", c.min_face_width.bending),
            ] {
                if let Some(v) = v {
                    assert!(
                        v.is_finite() && (v > 0.0 || (v == 0.0 && c.torque == 0.0)),
                        "{label} member {i}: bending {what} {v} at {} N·m",
                        c.torque
                    );
                    rated += 1;
                }
            }
        }
    }
    rated
}

/// Whether member `i` says its bending went unrated.
fn says_unrated(r: &super::TrainResult, i: usize) -> bool {
    r.members[i]
        .notes
        .iter()
        .any(|n| n.is(crate::note::key::GEAR_BENDING_UNRATED))
}

/// **A mate loaded low on its flank is not rated negative.** Against a
/// long-addendum member the unshifted 300/300 pair runs at `ε_α` 2.75, so the
/// unshared load point `d = ε − 1` sits low on the other member's flank:
/// the load line crosses the centreline below the section, the moment arm
/// shrinks, `Y_F` falls below the axial term and Dolan–Broghamer's factor goes
/// negative. It used to report −1.30 MPa and a negative minimum width, with
/// no note (−0.26 MPa at 2 N·m). The model has no reading there, so the
/// member is unrated and says so, and the mesh says it is past `ε_n = 2`
/// whatever the sharing model.
#[test]
fn a_mate_loaded_low_on_its_flank_is_unrated_not_negative() {
    let mut s = pair([300, 300]);
    s.members[0].gear.addendum = 1.9;
    s.members[0].gear.dedendum = 2.15;
    let r = solve_train(&Train::alone(&s, 10.0, 100.0), &test_library()).unwrap();
    bending_is_absent_or_positive(&r, "300/300 at h_a 1.9");
    assert!(r.members[1]
        .cases
        .iter()
        .all(|c| c.bending_stress.is_none()));
    assert!(says_unrated(&r, 1), "{:?}", r.members[1].notes);
    assert!(
        r.meshes[0]
            .notes
            .iter()
            .any(|n| n.is(crate::note::key::MESH_LOAD_SHARING_OUT_OF_BAND)),
        "no band note at ε {}",
        r.meshes[0].contact_ratio
    );
}

/// **A member with no root section costs its own rating, not the train's.**
/// At `h_a` 2.2 with sharp tips allowed, a 300/300 pair keeps no rated
/// section on one member in its one mesh; the train was refused whole
/// (`NoRootSection`, which blamed undercut).
#[test]
fn a_member_with_no_root_section_is_unrated_and_the_train_solves() {
    let mut s = pair([300, 300]);
    s.members[0].gear.addendum = 2.2;
    s.members[0].gear.no_sharp_tip = false;
    let r = solve_train(&Train::alone(&s, 10.0, 100.0), &test_library())
        .unwrap_or_else(|e| panic!("refused whole: {e}"));
    bending_is_absent_or_positive(&r, "300/300 at h_a 2.2");
    let unrated: Vec<usize> = (0..2)
        .filter(|&i| {
            r.members[i]
                .cases
                .iter()
                .all(|c| c.bending_stress.is_none())
        })
        .collect();
    assert!(!unrated.is_empty(), "both members rated");
    for i in unrated {
        assert!(says_unrated(&r, i), "member {i}: {:?}", r.members[i].notes);
    }
}

/// **A pointed tooth under load sharing is a number.** The 12/40 pair at 25°
/// with sharp tips allowed, `h_a` 1.6 and the pinion at `x` 0.5: the sweep
/// sampled the wheel's apex, where the Lewis section is `0/0`, and the `NaN`
/// latched as its worst (`Some(NaN)`). Below `ε_n = 2` sharing may only
/// relieve the unshared figure, by the fraction the seeded sweep leaves.
#[test]
fn a_pointed_tooth_under_load_sharing_is_a_number() {
    use crate::contact::LoadSharing;
    let build = |sharing| {
        let mut s = pair([12, 40]);
        s.set_load_sharing(sharing);
        for m in &mut s.members {
            m.gear.addendum = 1.6;
            m.gear.no_sharp_tip = false;
            m.pressure_angle = Auto::fixed(25.0);
        }
        s.members[0].gear.profile_shift = Auto::fixed(0.5);
        solve_train(&Train::alone(&s, 50.0, 3000.0), &test_library()).unwrap()
    };
    let (alone, shared) = (build(LoadSharing::None), build(LoadSharing::LinearRamp));
    assert!(bending_is_absent_or_positive(&shared, "12/40 shared") > 0);
    bending_is_absent_or_positive(&alone, "12/40 alone");
    for (a, b) in alone.members.iter().zip(&shared.members) {
        for (ca, cb) in a.cases.iter().zip(&b.cases) {
            if let (Some(x), Some(y)) = (ca.bending_stress, cb.bending_stress) {
                assert!(y / x <= 1.002, "sharing raised {x} to {y}");
            }
        }
    }
}

/// The presets, the hula and the arrangements no button reaches.
fn every_arrangement() -> Vec<super::Shape> {
    use super::arrangements as arr;
    Preset::ALL
        .into_iter()
        .map(Preset::build)
        .chain([
            arr::hula([65, 61, 57, 61], [1.0, 1.0]),
            arr::ravigneaux([18, 30], [22, 18], 62, 3),
            arr::worm_and_pair((1, 40), (17, 43)),
            arr::planocentric(52, 56),
        ])
        .collect()
}

/// **Every bending figure is absent or positive**, on every preset and
/// arrangement with one member's addendum swept 1.0–1.8, sharp tips allowed
/// on it, under both sharing models. The sweep carries `ε_α` through 1.5–3.2
/// and teeth to a point; a fuzz of 20,000 trains found 36 `NaN`, 10 negative
/// and 2 `−∞` bending results.
#[test]
fn every_bending_figure_is_absent_or_positive() {
    use crate::contact::LoadSharing;
    let lib = test_library();
    let (mut solved, mut rated) = (0, 0);
    for (a, base) in every_arrangement().into_iter().enumerate() {
        for i in 0..base.members.len() {
            for addendum in [1.0, 1.2, 1.4, 1.6, 1.8] {
                for sharing in [LoadSharing::None, LoadSharing::LinearRamp] {
                    let mut s = base.clone();
                    s.set_load_sharing(sharing);
                    s.members[i].gear.addendum = addendum;
                    s.members[i].gear.no_sharp_tip = false;
                    let Ok(r) = solve_train(&Train::alone(&s, 2.0, 100.0), &lib) else {
                        continue;
                    };
                    solved += 1;
                    rated += bending_is_absent_or_positive(
                        &r,
                        &format!("arrangement {a}, member {i} at h_a {addendum}, {sharing:?}"),
                    );
                }
            }
        }
    }
    assert!(solved > 0 && rated > 0, "the law is vacuous");
    eprintln!("{solved} trains solved, {rated} bending figures");
}

/// The canary pair, unshifted, both members at `width` mm.
fn canary(width: f64) -> super::Shape {
    let mut s = at_given_width(&pair([17, 43]), width);
    for m in &mut s.members {
        m.gear.profile_shift = Auto::fixed(0.0);
    }
    s
}

/// Every member's `(bending, contact)` minimum widths, case by case.
fn widths(s: &super::Shape) -> Vec<(Option<f64>, Option<f64>)> {
    let r = solve_train(&Train::alone(s, 2.0, 100.0), &test_library()).unwrap();
    r.members
        .iter()
        .flat_map(|g| {
            g.cases
                .iter()
                .map(|c| (c.min_face_width.bending, c.min_face_width.contact))
        })
        .collect()
}

/// **Each allowable moves its own width and nothing else.** A root limit and
/// a flank limit are two figures; overriding the fatigue (root) figure used to
/// move the contact width too, because contact was judged against it: on the
/// canary at 1088 MPa the bending width went 0.8953 -> 0.6172 mm and the
/// contact width with it.
#[test]
fn each_allowable_moves_its_own_width_and_nothing_else() {
    let base = canary(10.0);
    let before = widths(&base);
    let mut root = base.clone();
    root.members[0].gear.material_overrides.fatigue_allowable = Some(1088.0);
    let after = widths(&root);
    let mut moved = 0;
    for ((b0, c0), (b1, c1)) in before.iter().zip(&after) {
        assert_eq!(
            c0.map(f64::to_bits),
            c1.map(f64::to_bits),
            "the root figure moved a contact width"
        );
        moved += usize::from(b0 != b1);
    }
    assert!(moved > 0, "the root figure moved no bending width");

    let mut flank = base.clone();
    flank.members[0]
        .gear
        .material_overrides
        .contact_fatigue_allowable = Some(1088.0);
    let after = widths(&flank);
    let mut moved = 0;
    for ((b0, c0), (b1, c1)) in before.iter().zip(&after) {
        assert_eq!(
            b0.map(f64::to_bits),
            b1.map(f64::to_bits),
            "the flank figure moved a bending width"
        );
        moved += usize::from(c0 != c1);
    }
    assert!(moved > 0, "the flank figure moved no contact width");
}

/// **With contact sizing on, every flank holds its allowable at the width it
/// is given**, on every preset, in every case: the automatic width is the
/// largest any rating asks, so no line contact is left above its own figure.
#[test]
fn with_contact_sizing_on_every_flank_holds_its_allowable() {
    use super::{allowable, ByKind, FaceSources, Rating};
    let lib = test_library();
    let mut judged = 0;
    for p in Preset::ALL {
        let mut s = p.build();
        for m in &mut s.members {
            // The box a member no rating sizes keeps: a layshaft's idle pair.
            m.gear.face_width = Auto::automatic(10.0);
            m.gear.face_sources = FaceSources {
                bending: ByKind::of(|_| true),
                contact: ByKind::of(|_| true),
            };
        }
        let t = Train::alone(&s, 2.0, 100.0);
        let r = solve_train(&t, &lib).unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        for g in &r.members {
            for (c, case) in g.cases.iter().zip(&t.load_cases) {
                // A member sized by a line contact: the one kind of contact
                // a width answers to.
                if c.min_face_width.contact.is_none() {
                    continue;
                }
                let a = allowable(&g.material, Rating::Contact { aspect: 0.0 }, case.kind).unwrap();
                assert!(
                    c.contact_stress <= a * (1.0 + 1e-9),
                    "{p:?}: {} MPa against {a} at {} mm",
                    c.contact_stress,
                    g.face_width
                );
                judged += 1;
            }
        }
    }
    assert!(judged > 0, "no flank was judged: the law is vacuous");
}

/// **The canary runs past its flank's endurance, and says so.** At the
/// bending-sized default width its pinion's contact stress is well above any
/// published contact endurance for 46 HRC 4340; contact sizing is off by
/// default, so the member says the stress exceeds the allowable.
#[test]
fn the_canary_says_its_flank_is_past_its_allowable() {
    let mut s = pair([17, 43]);
    for m in &mut s.members {
        m.gear.profile_shift = Auto::fixed(0.0);
        m.gear.face_width = Auto::automatic(0.0);
    }
    let r = solve_train(&Train::alone(&s, 2.0, 100.0), &test_library()).unwrap();
    assert!(
        r.members[0]
            .notes
            .iter()
            .any(|n| n.is("gear.contact_above_allowable")),
        "{:?}",
        r.members[0].notes
    );
}
