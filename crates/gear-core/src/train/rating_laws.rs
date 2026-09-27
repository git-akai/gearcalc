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
/// 17/43 pair at 2 N·m asks 8.483 mm of the pinion for fatigue contact at
/// `K_A = 1`; a light application factor asks 1.25 times that, past the
/// 10 mm the preset gives it. (`gear-cli strength 17 43 2.0` builds its pair
/// with the library's default tooth rather than the preset's, and asks
/// 8.529 mm.)
#[test]
fn the_canary_at_a_light_application_factor_asks_more_than_its_face() {
    let mut s = pair([17, 43]);
    for m in &mut s.members {
        m.gear.profile_shift = Auto::fixed(0.0);
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
