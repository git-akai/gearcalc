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
                if let Some(c) = ca
                    .contact_stress
                    .zip(cb.contact_stress)
                    .and_then(|(x, y)| ratio(x, y))
                {
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

/// **A mate loaded low on its flank is not rated negative.** Against a
/// long-addendum member the unshifted 300/300 pair runs past `ε_n = 2`, so the
/// unshared load point `d = ε − 1` sits low on the other member's flank. It
/// used to report −1.30 MPa and a negative minimum width, with no note, on a
/// mate whose standard root the long tip reached into: a path through flank
/// interference, whose load point was off the usable flank. With the mate's
/// root deep enough for the tip, the path stays on usable flank, both members
/// rate positive, and the mesh says it is past `ε_n = 2`.
#[test]
fn a_mate_loaded_low_on_its_flank_is_not_rated_negative() {
    let mut s = pair([300, 300]);
    s.members[0].gear.addendum = 1.9;
    s.members[0].gear.dedendum = 2.15;
    s.members[1].gear.dedendum = 2.15;
    let r = solve_train(&Train::alone(&s, 10.0, 100.0), &test_library()).unwrap();
    assert_eq!(r.meshes[0].flank_interference, [false, false]);
    bending_is_absent_or_positive(&r, "300/300 at h_a 1.9");
    assert!(r
        .members
        .iter()
        .all(|g| g.cases.iter().all(|c| c.bending_stress.is_some())));
    assert!(
        r.meshes[0]
            .notes
            .iter()
            .any(|n| n.is(crate::note::key::MESH_LOAD_SHARING_OUT_OF_BAND)),
        "no band note at ε {}",
        r.meshes[0].contact_ratio.unwrap()
    );
}

/// **A pair whose tips stay on usable flank rates every member.** Since the
/// path of contact ends at the usable flanks, a load point off every section
/// is reached only through flank interference: over the grid below, every
/// pair that solves with no flagged mesh has both members rated, and the
/// grid does reach unrated members with one. The 5/40 pinion at β 20°, x 1
/// with its tip held to its width is the case that tests the section rule:
/// its flank's candidate is compressed (`Y_F` 4.06 against an axial term of
/// 4.34) and must not mask its fillet's, which rates.
#[test]
fn only_flank_interference_leaves_a_member_unrated() {
    let lib = test_library();
    let (mut clean, mut unrated_with) = (0, 0);
    for (z0, z1) in [
        (5_u32, 40_u32),
        (7, 300),
        (9, 100),
        (12, 300),
        (20, 40),
        (40, 300),
    ] {
        for beta in [0.0, 20.0, 35.0] {
            for addendum in [1.0, 1.6, 2.2] {
                for x in [-0.8, 0.0, 1.0] {
                    for sharp in [true, false] {
                        let mut s = pair([z0, z1]).with_first_helix(beta);
                        s.members[0].gear.addendum = addendum;
                        s.members[1].gear.dedendum = addendum + 0.25;
                        s.members[0].gear.profile_shift = crate::params::Auto::fixed(x);
                        s.members[1].gear.profile_shift = crate::params::Auto::fixed(0.0);
                        for m in &mut s.members {
                            m.gear.no_sharp_tip = sharp;
                            m.gear.no_undercut = false;
                        }
                        let Ok(r) = solve_train(&Train::alone(&s, 10.0, 100.0), &lib) else {
                            continue;
                        };
                        let unrated = (0..2).any(|i| {
                            r.members[i]
                                .cases
                                .iter()
                                .all(|c| c.bending_stress.is_none())
                        });
                        if r.meshes[0].flank_interference.contains(&true) {
                            unrated_with += usize::from(unrated);
                        } else {
                            clean += 1;
                            assert!(
                                !unrated,
                                "{z0}/{z1} β {beta} h_a {addendum} x {x}: unrated on usable flank"
                            );
                        }
                    }
                }
            }
        }
    }
    assert!(
        clean > 100 && unrated_with > 0,
        "{clean} clean, {unrated_with} unrated"
    );
}

/// **A member with no rated section costs its own rating, not the train's**,
/// and says why. Reached only through flank interference
/// (`only_flank_interference_leaves_a_member_unrated`): the 7-tooth pinion
/// at `x` −0.8 against 300 at 20° is one, every section it has compressed at
/// its load point; the train was refused whole there once (a root-section
/// error that blamed undercut).
#[test]
fn a_member_with_no_root_section_is_unrated_and_the_train_solves() {
    let mut s = pair([7, 300]).with_first_helix(20.0);
    s.members[0].gear.profile_shift = crate::params::Auto::fixed(-0.8);
    s.members[1].gear.profile_shift = crate::params::Auto::fixed(0.0);
    for m in &mut s.members {
        m.gear.no_undercut = false;
    }
    let r = solve_train(&Train::alone(&s, 10.0, 100.0), &test_library())
        .unwrap_or_else(|e| panic!("refused whole: {e}"));
    assert!(r.meshes[0].flank_interference.contains(&true));
    bending_is_absent_or_positive(&r, "7/300 at x -0.8");
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
        // Its sections exist and are compressed, and it says that rather
        // than that it has none.
        assert!(
            r.members[i]
                .notes
                .iter()
                .any(|n| n.is(crate::note::key::GEAR_BENDING_UNRATED_COMPRESSED)),
            "member {i}: {:?}",
            r.members[i].notes
        );
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

/// The grid's arrangements ([`super::testing::shapes`]), and a
/// planocentric at a wider tooth difference.
fn every_arrangement() -> Vec<super::Shape> {
    let mut out = super::testing::shapes();
    out.push(super::arrangements::planocentric(52, 56));
    out
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
/// is given, and the flank that sized the width holds it exactly**, on every
/// preset, in every case: the automatic width is the largest any rating
/// asks, and the stress reported at a width is the allowable scaled by the
/// ask (`σ_a √(b_ask / b)`), so where a member's width is its own contact
/// ask the stress is the allowable to the bit and `gear.contact_above_allowable`
/// is silent — where the scaled stress had come out an ulp either side of it.
/// The bending default the same way: a width its own bending ask set carries
/// the bending allowable to the bit.
#[test]
fn with_contact_sizing_on_every_flank_holds_its_allowable() {
    use super::{allowable, ByKind, FaceSources, Rating};
    let lib = test_library();
    let (mut judged, mut exact) = (0, 0);
    for p in Preset::ALL {
        let mut s = p.build();
        for m in &mut s.members {
            m.gear.face_sources = FaceSources {
                bending: ByKind::of(|_| true),
                contact: ByKind::of(|_| true),
            };
        }
        let t = Train::alone(&s, 2.0, 100.0);
        let r = solve_train(&t, &lib).unwrap_or_else(|e| panic!("{p:?}: {e:?}"));
        for g in &r.members {
            assert!(
                !g.notes.iter().any(|n| n.is("gear.contact_above_allowable")),
                "{p:?}: {:?}",
                g.notes
            );
            for (c, case) in g.cases.iter().zip(&t.load_cases) {
                // A member sized by a line contact: the one kind of contact
                // a width answers to.
                let (Some(ask), Some(stress), Some(width)) =
                    (c.min_face_width.contact, c.contact_stress, g.face_width)
                else {
                    continue;
                };
                let a = allowable(&g.material, Rating::Contact { aspect: 0.0 }, case.kind).unwrap();
                assert!(stress <= a, "{p:?}: {stress} MPa against {a} at {width} mm");
                judged += 1;
                if ask == width {
                    assert_eq!(stress.to_bits(), a.to_bits(), "{p:?}: sized, so at {a}");
                    exact += 1;
                }
            }
        }
    }
    assert!(
        judged > 0 && exact > 0,
        "{judged} judged, {exact} sized exactly"
    );
}

/// **A stress one ulp over its allowable fires the note; the allowable
/// itself does not** — the comparison is strict, and needs no tolerance
/// because a sized width carries its allowable exactly.
#[test]
fn the_contact_note_fires_one_ulp_over_and_not_at_the_allowable() {
    use super::{contact_notes, CaseKind, Rated, Widths};
    let a = 845.68;
    let rated = |s: f64| Rated {
        case: 0,
        kind: CaseKind::Fatigue,
        bending_stress: None,
        contact_stress: Some(s),
        min_face_width: Widths {
            bending: None,
            contact: None,
        },
        contact_worst: Some((s, a)),
        contact_unjudged: false,
    };
    let fires = |s: f64| {
        contact_notes(&[rated(s)])
            .iter()
            .any(|n| n.is("gear.contact_above_allowable"))
    };
    assert!(!fires(a), "at the allowable");
    assert!(fires(f64::from_bits(a.to_bits() + 1)), "one ulp over");
    assert!(!fires(f64::from_bits(a.to_bits() - 1)), "one ulp under");
}

/// **An automatic width's box is bounded where it is read** (stage1-exit
/// item 3). On a line contact it is not read: a layshaft at boxes of nought
/// solves, its engaged ratio sized by its ratings and its idle ratios — which
/// no case loads — not sized, where a box of nought there was a face of
/// nothing the solve refused as if the teeth missed (`NoContact`, at the
/// base this ran on `FlankInterference`). On a point contact the box is the
/// width, so a crossed pair's box of nought is refused by its field. Boxes of
/// the default solve both (the control).
#[test]
fn an_automatic_widths_box_is_bounded_where_it_is_read() {
    use super::{ByKind, FaceSources, DEFAULT_FACE_WIDTH};
    let lib = test_library();
    let with_box = |preset: Preset, width: f64| {
        let mut s = preset.build();
        for m in &mut s.members {
            m.gear.face_width = Auto::automatic(width);
            m.gear.face_sources = FaceSources {
                bending: ByKind::of(|_| true),
                contact: ByKind::of(|_| true),
            };
        }
        solve_train(&Train::alone(&s, 2.0, 100.0), &lib)
    };
    let r = with_box(Preset::Layshaft, 0.0).unwrap_or_else(|e| panic!("{e:?}"));
    let (sized, not): (Vec<_>, Vec<_>) = r.members.iter().partition(|g| g.face_width.is_some());
    assert!(
        sized.len() >= 4 && !not.is_empty(),
        "{} / {}",
        sized.len(),
        not.len()
    );
    assert!(sized.iter().all(|g| g.face_width.is_some_and(|w| w > 0.0)));
    let refused = with_box(Preset::Crossed, 0.0)
        .err()
        .map(|e| crate::note::Explain::note(&e));
    assert!(
        refused.as_ref().is_some_and(|n| n
            .values
            .get("field")
            .is_some_and(|f| f.ends_with("gear.face_width.manual"))),
        "{refused:?}"
    );
    for preset in [Preset::Layshaft, Preset::Crossed] {
        assert!(with_box(preset, DEFAULT_FACE_WIDTH).is_ok(), "{preset:?}");
    }
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
        m.gear.face_width = Auto::automatic(super::DEFAULT_FACE_WIDTH);
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

/// **A tip asked to be held reaches past nothing, exactly, and is never cut
/// into a negative tooth**: the train's graph built as the solve builds it
/// (`Shape::built`: the shifts it settles on, each distance where its tips
/// hold it), each held tip compared with the conjugate of its mate's
/// junction directly (`shape::tip_hold::tips_check`), and every member held
/// by its mates at a positive addendum. (It built at the plan's distances,
/// not the ones the tips opened out, and a train that solved then read as
/// one whose graph does not build.)
pub(super) fn held_tips_reach_past_nothing(
    t: &Train,
    r: &super::TrainResult,
) -> Result<(), String> {
    let shape = &t.shape.shared();
    let built = shape
        .built()
        .map_err(|e| format!("solved, but its graph does not build: {e}"))?;
    let found = super::shape::tip_hold::tips_check(shape, &built);
    if !found.is_empty() {
        return Err(found.join("; "));
    }
    for (i, g) in r.members.iter().enumerate() {
        let held = g
            .notes
            .iter()
            .any(|n| n.is(crate::note::key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK));
        if held && g.addendum <= 0.0 {
            return Err(format!("member {i} held to an addendum of {}", g.addendum));
        }
    }
    Ok(())
}

/// **Every tip held, on every preset and arrangement**: each shipped design
/// with the bound turned on for every member, external ones included, and
/// solved. Each member is held exactly or says it cannot be
/// ([`held_tips_reach_past_nothing`]), and at least one is each, so the law
/// reaches both arms.
#[test]
fn every_tip_held_reaches_past_nothing() {
    let mut held = 0;
    for (name, mut t) in default_trains() {
        for m in &mut t.shape.members {
            m.gear.no_tip_past_mate_flank = true;
        }
        let Ok(r) = solve_train(&t, &test_library()) else {
            continue;
        };
        held_tips_reach_past_nothing(&t, &r).unwrap_or_else(|e| panic!("{name}: {e}"));
        held += r
            .members
            .iter()
            .filter(|g| {
                g.notes
                    .iter()
                    .any(|n| n.is(crate::note::key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK))
            })
            .count();
    }
    assert!(held > 0, "nothing was held: the law is vacuous");
    // The unholdable arm: a 7-tooth pinion at x −0.8 against 300, whose
    // wheel's tip reaches past the pinion's flank at any length.
    let mut s = pair([7, 300]);
    s.members[0].gear.profile_shift = Auto::fixed(-0.8);
    s.members[0].gear.no_undercut = false;
    s.members[1].gear.no_tip_past_mate_flank = true;
    let t = Train::alone(&s, 2.0, 100.0);
    let r = solve_train(&t, &test_library()).expect("the deep pair solves");
    assert!(
        r.members[1]
            .notes
            .iter()
            .any(|n| n.is(crate::note::key::GEAR_TIP_CANNOT_CLEAR_MATE_FLANK)),
        "the wheel says it cannot be held: {:?}",
        r.members[1].notes
    );
    held_tips_reach_past_nothing(&t, &r).unwrap();
}

/// Every preset, every arrangement of a set, a Ravigneaux, the hula and a
/// worm beside a pair, each alone as shipped.
pub(super) fn default_trains() -> Vec<(String, Train)> {
    use super::arrangements::{hula, planetary, ravigneaux, worm_and_pair};
    use crate::planetary::{Arrangement, PlanetaryShaft};
    let mut trains: Vec<(String, Train)> = Preset::ALL
        .iter()
        .map(|p| (format!("{p:?}"), Train::alone(&p.build(), 2.0, 3000.0)))
        .collect();
    let set = planetary(12, 30, 72, 3);
    for input in PlanetaryShaft::ALL {
        for fixed in PlanetaryShaft::ALL {
            if input != fixed {
                let arrangement = Arrangement { input, fixed };
                trains.push((
                    format!("{arrangement:?}"),
                    Train::alone(&set, 2.0, 3000.0).arranged_as(arrangement),
                ));
            }
        }
    }
    for (name, shape) in [
        ("ravigneaux", ravigneaux([18, 30], [22, 18], 62, 3)),
        ("hula", hula([65, 61, 57, 61], [1.0, 1.0])),
        ("worm and pair", worm_and_pair((1, 40), (17, 43))),
    ] {
        trains.push((name.into(), Train::alone(&shape, 2.0, 3000.0)));
    }
    trains
}

/// **No default design reaches a flank past its usable end.** Every preset,
/// every arrangement of a set, a Ravigneaux, the hula, a worm beside a pair
/// and four small-pinion pairs, solved alone as shipped and searched: no
/// mesh reports flank interference and none says so. A full-depth ring's
/// tip reaches below its planet's form circle, so a default that interferes
/// is a default nobody would cut.
#[test]
fn no_default_design_raises_flank_interference() {
    // As shipped, and searched: the search chooses shifts that push a tip
    // against its mate's flank, where the hold then tops it. Small pinions
    // on parallel axes besides, where an external pair's interference bites.
    let mut trains = default_trains();
    for teeth in [[9_u32, 37], [9, 20], [12, 29], [7, 30]] {
        trains.push((
            format!("pair {teeth:?}"),
            Train::alone(&pair(teeth), 2.0, 3000.0),
        ));
    }
    let searched: Vec<(String, Train)> = trains
        .iter()
        .map(|(name, t)| {
            let mut t = t.clone();
            t.shape.set_search(true);
            (format!("{name}, searched"), t)
        })
        .collect();
    trains.extend(searched);
    let mut fouled = Vec::new();
    for (name, t) in &trains {
        let r = solve_train(t, &test_library()).unwrap_or_else(|e| panic!("{name}: {e}"));
        for (k, m) in r.meshes.iter().enumerate() {
            let said = m
                .notes
                .iter()
                .any(|n| n.is(crate::note::key::MESH_FLANK_INTERFERENCE));
            if m.flank_interference.contains(&true) || said {
                fouled.push(format!("{name} mesh {k}: {:?}", m.flank_interference));
            }
        }
    }
    assert!(fouled.is_empty(), "defaults that interfere: {fouled:#?}");
}
/// **A point contact holds a tip as a line contact does**: a crossed pair
/// and a worm at 1.8 modules of addendum reach past each other's usable
/// flanks as typed, and with the bound on every tip that reaches is held,
/// through [`crate::screw::CrossedPath::contact_radius_at`], and none does.
#[test]
fn a_crossed_mesh_holds_its_tips() {
    for p in [Preset::Crossed, Preset::Worm] {
        let mut s = p.build();
        for m in &mut s.members {
            m.gear.addendum = 1.8;
            m.gear.no_sharp_tip = false;
            m.gear.no_tip_past_mate_flank = false;
        }
        let typed = solve_train(&Train::alone(&s, 2.0, 100.0), &test_library()).unwrap();
        assert_eq!(typed.meshes[0].flank_interference, [true, true], "{p:?}");
        for m in &mut s.members {
            m.gear.no_tip_past_mate_flank = true;
        }
        let t = Train::alone(&s, 2.0, 100.0);
        let r = solve_train(&t, &test_library()).unwrap();
        assert_eq!(r.meshes[0].flank_interference, [false, false], "{p:?}");
        held_tips_reach_past_nothing(&t, &r).unwrap_or_else(|e| panic!("{p:?}: {e}"));
        let held = r
            .members
            .iter()
            .filter(|g| {
                g.notes
                    .iter()
                    .any(|n| n.is(crate::note::key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK))
            })
            .count();
        assert!(held > 0, "{p:?}: nothing was held");
    }
}

/// **A contact rated at any pressure angle the table admits is a figure, or
/// a refusal that names why** (T01.7). Every arrangement, every member at
/// 45°, 60°, 75°, 85°, 89° and 89.9° — the whole of the steep half of
/// `(0°, 90°)`: the train either solves, every member's every case a
/// finite contact stress (nought only on a mesh that carries nothing) and a
/// finite bending stress or none, where a line contact's member says so
/// (`gear.bending_unrated`, or `gear.bending_unrated_compressed` where the
/// load compresses the root it would bend) and a member on point contacts
/// alone has none
/// by decision; or it is refused, by a catalogue key. The line and point
/// contacts answer steep flanks alike: a contact rated, a root said.
#[test]
fn a_steep_flank_is_rated_or_refused_by_name() {
    use crate::note::{key, Explain};
    let lib = test_library();
    let (mut rated, mut refused) = (0, 0);
    for (name, shape) in super::sweep::arrangements() {
        for alpha in [45.0, 60.0, 75.0, 85.0, 89.0, 89.9] {
            let mut t = super::sweep::cased(vec![shape.clone()]);
            for m in &mut t.shape.members {
                m.pressure_angle = Auto::fixed(alpha);
            }
            let r = match solve_train(&t, &lib) {
                Ok(r) => r,
                Err(e) => {
                    assert!(e.note().key.starts_with("error."), "{name} {alpha}: {e:?}");
                    refused += 1;
                    continue;
                }
            };
            let s = t.shape.indexed();
            for (i, g) in r.members.iter().enumerate() {
                let line = s.meshes_of(i).iter().any(|&k| {
                    s.distance_of(k)
                        .is_some_and(|d| s.distances[d].angle == 0.0)
                });
                for c in &g.cases {
                    let at = format!("{name} {alpha}° member {i} case {}", c.case);
                    assert!(
                        c.contact_stress.is_some_and(|s| s.is_finite() && s >= 0.0),
                        "{at}"
                    );
                    match c.bending_stress {
                        Some(b) => assert!(b.is_finite() && b >= 0.0, "{at}: {b}"),
                        None if line => assert!(
                            g.notes.iter().any(|n| {
                                n.is(key::GEAR_BENDING_UNRATED)
                                    || n.is(key::GEAR_BENDING_UNRATED_COMPRESSED)
                            }),
                            "{at}: no bending, and nothing said"
                        ),
                        None => {}
                    }
                }
            }
            rated += 1;
        }
    }
    assert_eq!(rated + refused, 6 * super::sweep::arrangements().len());
    assert!(rated > refused, "{rated} rated, {refused} refused");
}

/// **An axis distance's two axes are a pair, not an order** (T14.6):
/// swapping them on every distance of every arrangement changes nothing the
/// train reports — no member, mesh, distance, axis, path or case figure, and
/// no note. The axial float in particular is read along the mesh's own
/// first member and the worm's axis, never along whichever axis a distance
/// happens to list first; the worm, whose float is 0.04 mm, is among them.
#[test]
fn a_distance_read_either_way_round_is_the_same_distance() {
    let lib = test_library();
    let mut asked = 0;
    for (name, shape) in super::sweep::arrangements() {
        let t = super::sweep::cased(vec![shape.clone()]);
        let mut swapped = t.clone();
        for d in &mut swapped.shape.distances {
            d.axes = [d.axes[1], d.axes[0]];
        }
        let (a, b) = (solve_train(&t, &lib), solve_train(&swapped, &lib));
        let seen = |r: &super::TrainResult| {
            format!(
                "{:?}",
                (
                    &r.members,
                    &r.meshes,
                    &r.distances,
                    &r.axes,
                    &r.paths,
                    &r.cases,
                    r.parts
                        .iter()
                        .map(|p| (&p.cases, &p.notes))
                        .collect::<Vec<_>>()
                )
            )
        };
        match (a, b) {
            (Ok(a), Ok(b)) => assert_eq!(seen(&a), seen(&b), "{name}"),
            (a, b) => panic!("{name}: {:?} against {:?}", a.err(), b.err()),
        }
        asked += 1;
    }
    assert_eq!(asked, super::sweep::arrangements().len());
}

/// **Every mesh's efficiency is a fraction for every friction the table
/// admits** (T01.8): every arrangement with both coefficients of every mesh
/// at 0, 0.06, 0.5, 2, 5 and 100 — line and point contacts alike, the worm
/// and the crossed pair among them — every mesh's efficiency either way in
/// `[0, 1]`. A negative coefficient, which drove a spur mesh to 1.004 and a
/// worm's to 1.63, is refused where it enters (`crate::input::MESH`).
#[test]
fn a_mesh_efficiency_is_a_fraction_at_every_friction_admitted() {
    let lib = test_library();
    let mut read = 0;
    for (name, shape) in super::sweep::arrangements() {
        for mu in [0.0, 0.06, 0.5, 2.0, 5.0, 100.0] {
            let mut t = super::sweep::cased(vec![shape.clone()]);
            for m in &mut t.shape.meshes {
                m.sliding_friction = mu;
                m.static_friction = mu;
            }
            let r = solve_train(&t, &lib).unwrap_or_else(|e| panic!("{name} at {mu}: {e}"));
            for (k, m) in r.meshes.iter().enumerate() {
                for e in [m.efficiency.forward, m.efficiency.backward] {
                    assert!((0.0..=1.0).contains(&e), "{name} at µ {mu}, mesh {k}: {e}");
                    read += 1;
                }
            }
        }
        let mut t = super::sweep::cased(vec![shape.clone()]);
        t.shape.meshes[0].sliding_friction = -0.02;
        assert!(
            matches!(solve_train(&t, &lib), Err(super::TrainError::Input(ref r)) if r.field == "shape.meshes.0.sliding_friction"),
            "{name}: a negative friction refused by its field"
        );
    }
    assert!(read > 6 * 2 * super::sweep::arrangements().len());
}

/// **Ultimate contact is judged on every solid the table admits**, and said
/// unjudged only for the one true cause, an ultimate figure at break: a
/// steel pair whose Poisson's ratio is −0.5 (auxetic), 0.05 and 0.17 (the
/// line's first yield at the surface, which the depth search once began
/// below) and ½ (the incompressible end, which a second bound on ν once
/// refused) is rated for contact at its ultimate case and says nothing of
/// judging it; the same steel read as a figure at break says so.
#[test]
fn ultimate_contact_is_judged_on_every_admitted_solid() {
    use crate::material::{Measure, Overrides};
    use crate::note::key;
    let mut lib = test_library();
    let rated = |lib: &crate::material::MaterialLibrary, nu: f64| {
        let mut s = pair([17, 43]);
        for m in &mut s.members {
            m.gear.material_overrides = Overrides {
                poissons_ratio: Some(nu),
                ..Overrides::default()
            };
        }
        let r = solve_train(&Train::alone(&s, 2.0, 100.0), lib).unwrap();
        r.members.iter().any(|g| {
            g.notes
                .iter()
                .any(|n| n.is(key::GEAR_CONTACT_ULTIMATE_UNJUDGED))
        })
    };
    let mut asked = 0;
    for nu in [-0.5, 0.05, 0.17, 0.3, 0.5] {
        assert!(!rated(&lib, nu), "ν {nu}: ultimate contact unjudged");
        asked += 1;
    }
    assert_eq!(asked, 5);
    for m in &mut lib.materials {
        m.ultimate_measure = Measure::Break;
    }
    assert!(rated(&lib, 0.3), "a figure at break is said unjudged");
}
