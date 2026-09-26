//! **The solve is homogeneous in length.** Scale the module and every input
//! stated in millimetres by `s`, and the torque by `s³` so every stress stays
//! put: then every dimensionless figure — ratio, efficiency, contact ratio,
//! stress and safety factor, every angle, every note's key and every refusal —
//! is the unscaled solve's, and every length is `s` times it.
//!
//! The law is read off the whole result rather than a list of fields: the
//! pretty `Debug` of each solve is walked into one token per leaf, keyed by
//! its field path. Text tokens (keys, flags, variants) must match exactly —
//! except a note's formatted values, which carry lengths as text. Each
//! numeric path must scale as one power `s^k`, the same `k` at every preset,
//! case and scale; the paths named in [`EXPECTED`] must have the `k` their
//! unit gives them.
//!
//! What it does not change: any default's unit. The presets keep their
//! millimetre figures; the law scales them with the module, so it tests the
//! model's homogeneity and not what the defaults should be.

use super::arrangements::{hula, ravigneaux, worm_and_pair, Preset};
use super::{solve_train, test_library, Duty, LoadCase, Train};
use crate::params::Auto;
use std::collections::BTreeMap;

/// The two scales, a decade either side.
const SCALES: [f64; 2] = [0.1, 10.0];

/// Agreement asked of `s^k · v₁` against `v_s`, relative.
const REL: f64 = 1e-10;

/// An absolute allowance, in the figure's unscaled unit, for a figure taken
/// as a difference: a tip margin is two radii of order ten apart by 1e-4, and
/// carries their rounding, some tens of ulps of 10 (1.8e-15 each).
const FLOOR: f64 = 1e-12;

/// Below this, in the unscaled unit, a figure is numerical zero: it says
/// nothing about how its path scales, and is held only to stay near zero.
const ZERO: f64 = 1e-9;

/// Powers of `s` a figure may scale by: `s⁻¹` a curvature, `s³` a torque or a
/// power at a fixed speed.
const POWERS: std::ops::RangeInclusive<i32> = -3..=3;

/// **The powers the units fix**, by a field path's last name, or by a name
/// on the way to it where the leaf's own is generic (`forward`, `nominal`).
/// Dimensionless: every ratio, efficiency, contact ratio, stress and factor
/// (stresses stay put because the torque scales as `s³`), and every angle —
/// a backlash is quoted in degrees — and the power through a mesh, a
/// fraction of the power in. Lengths: `1`. Curvatures: `-1`. Torques: `3`.
const EXPECTED: &[(&str, i32)] = &[
    ("ratio", 0),
    ("efficiency", 0),
    ("contact_ratio", 0),
    ("contact_ratios", 0),
    ("operating_pressure_angle", 0),
    ("pressure_angle", 0),
    ("helix_angle", 0),
    ("lead_angle", 0),
    ("backlash", 0),
    ("profile_shift", 0),
    ("bending_stress", 0),
    ("contact_stress", 0),
    ("max_pressure", 0),
    ("cycles", 0),
    ("speed", 0),
    ("sliding_ratio", 0),
    ("power_through", 0),
    ("running", 1),
    ("clearance", 1),
    ("module", 1),
    ("pitch_diameter", 1),
    ("face_width", 1),
    ("patch_width", 1),
    ("patch_length", 1),
    ("min_face_width", 1),
    ("sliding_velocity", 1),
    ("curvature_along", -1),
    ("curvature_across", -1),
    ("torque", 3),
    ("on_body", 3),
];

/// Leaf names that say which end, bound or element, not what the figure is.
const GENERIC: &[&str] = &[
    "forward",
    "backward",
    "nominal",
    "minimum",
    "maximum",
    "transverse",
    "overlap",
    "total",
    "manual",
    "bending",
    "contact",
    "[]",
];

/// One leaf of a pretty `Debug`: its path of field names, and its text.
struct Leaf {
    path: String,
    text: String,
}

/// The leaves of `{:#?}`, in order. Indentation gives the nesting; an
/// element with no field name is `[]`.
fn leaves(debug: &str) -> Vec<Leaf> {
    let mut names: Vec<String> = Vec::new();
    let mut out = Vec::new();
    for line in debug.lines() {
        let depth = (line.len() - line.trim_start().len()) / 4;
        let body = line.trim().trim_end_matches(',');
        if body.is_empty() || matches!(body, "}" | "]" | ")") {
            continue;
        }
        let (name, rest) = match body.split_once(": ") {
            Some((n, r))
                if n.chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '"') =>
            {
                (n.to_string(), r)
            }
            _ => ("[]".to_string(), body),
        };
        names.truncate(depth);
        names.resize(depth, String::new());
        names.push(name);
        let opens = rest.ends_with('{') || rest.ends_with('[') || rest.ends_with('(');
        out.push(Leaf {
            path: names.join("/"),
            text: if opens {
                rest.to_string()
            } else {
                rest.trim().to_string()
            },
        });
    }
    out
}

/// Every millimetre input the train has, by name: the list [`scaled`]
/// multiplies by `s`. Found by reading every `f64` of `Shape`, `Axis`,
/// `Member`, `MemberGear`, `Distance` and `Load` for its unit; the rest are
/// coefficients in modules (addendum, dedendum, root radius, working depth, a
/// cutter's), angles, counts, frictions, speeds and material figures.
const MILLIMETRES: [&str; 13] = [
    "min_planet_clearance",
    "module",
    "pitch_diameter",
    "face_width",
    "min_tip_width",
    "rim_thickness",
    "distance",
    "clearance",
    "tip_clearance",
    "tolerance_plus",
    "tolerance_minus",
    "axial_clearance",
    "torque",
];

/// The train with every input in [`MILLIMETRES`] times `s` — the torque
/// times `s³` — except `keep`, left as it was.
fn scaled(train: &Train, s: f64, keep: Option<&str>) -> Train {
    let f = |name: &str| if keep == Some(name) { 1.0 } else { s };
    let mut t = train.clone();
    for axis in &mut t.shape.axes {
        axis.min_planet_clearance *= f("min_planet_clearance");
    }
    for m in &mut t.shape.members {
        m.module.manual *= f("module");
        m.pitch_diameter.manual *= f("pitch_diameter");
        m.gear.face_width.manual *= f("face_width");
        m.gear.min_tip_width *= f("min_tip_width");
        if let Some(r) = &mut m.gear.rim_thickness {
            *r *= f("rim_thickness");
        }
    }
    for d in &mut t.shape.distances {
        d.distance.manual *= f("distance");
        d.clearance.manual *= f("clearance");
        d.tip_clearance *= f("tip_clearance");
        d.tolerance_plus *= f("tolerance_plus");
        d.tolerance_minus *= f("tolerance_minus");
        d.axial_clearance *= f("axial_clearance");
    }
    for c in &mut t.load_cases {
        for l in &mut c.loads {
            l.torque.manual *= f("torque").powi(3);
        }
    }
    t
}

/// Each preset, and the arrangements no preset lays out, asked alone
/// under three cases: a load from each end and a fatigue duty.
fn trains() -> Vec<(String, Train)> {
    let mut shapes: Vec<(String, super::Shape)> = Preset::ALL
        .into_iter()
        .map(|p| (format!("{p:?}"), p.build()))
        .collect();
    shapes.push(("worm and pair".into(), worm_and_pair((1, 40), (17, 43))));
    shapes.push(("ravigneaux".into(), ravigneaux([18, 30], [22, 18], 62, 3)));
    shapes.push(("hula".into(), hula([65, 61, 57, 61], [1.0, 1.0])));
    // Three inputs no preset reads at its defaults, given so the law reaches
    // them: a running distance, a rim thin enough to de-rate the root, and a
    // planet gap wider than the planets leave.
    let mut given = Preset::Spur.build();
    given.distances[0].distance = Auto::fixed(30.3);
    given.members[0].gear.rim_thickness = Some(2.0);
    shapes.push(("spur, distance and rim given".into(), given));
    let mut gap = Preset::Planetary.build();
    for a in &mut gap.axes {
        a.min_planet_clearance = 10.0;
    }
    shapes.push(("planetary, planet gap asked".into(), gap));
    shapes
        .into_iter()
        .map(|(name, shape)| {
            let mut t = Train::alone(&shape, 2.0, 3000.0);
            let (input, output) = (t.load_cases[0].loads[0].at, t.load_cases[0].loads[1].at);
            t.load_cases = vec![
                LoadCase::ultimate(input, output, 2.0, 3000.0),
                LoadCase::back_driving(input, output, 0.6),
                LoadCase {
                    duty: Duty::Continuous {
                        runtime_hours: 1000.0,
                    },
                    ..LoadCase::fatigue(input, output, 2.0, 2400.0)
                },
            ];
            (name, t)
        })
        .collect()
}

/// What the law found over every train at every scale.
struct Breaches {
    /// What breaks it, one line each.
    found: Vec<String>,
    /// Figures compared.
    checked: usize,
    /// The largest relative departure from `s^k · v₁` among those that fit,
    /// and where.
    worst: (f64, String),
    /// Trains the unscaled solve refused, which the law then says little of.
    refused: Vec<String>,
}

/// The law over every train at every scale, with every millimetre input
/// scaled but `keep`.
fn breaches(keep: Option<&str>) -> Breaches {
    let lib = test_library();
    let mut found = Vec::new();
    let mut powers: BTreeMap<String, (i32, String)> = BTreeMap::new();
    let (mut checked, mut worst, mut refused) = (0, (0.0_f64, String::new()), Vec::new());
    for (name, train) in trains() {
        let solved = solve_train(&train, &lib);
        if let Err(e) = &solved {
            refused.push(format!("{name}: {e:?}"));
        }
        let base = leaves(&format!("{solved:#?}"));
        for s in SCALES {
            let t = scaled(&train, s, keep);
            let at = leaves(&format!("{:#?}", solve_train(&t, &lib)));
            let here = format!("{name} at s = {s}");
            if base.len() != at.len() {
                found.push(format!(
                    "{here}: {} leaves against {}",
                    at.len(),
                    base.len()
                ));
                continue;
            }
            // Numerical zeros wait until every path's power is known.
            let mut zeros = Vec::new();
            for (b, a) in base.iter().zip(&at) {
                if b.path != a.path {
                    found.push(format!("{here}: {} where {} was", a.path, b.path));
                    break;
                }
                let (Ok(v1), Ok(vs)) = (b.text.parse::<f64>(), a.text.parse::<f64>()) else {
                    // A note's values are formatted numbers, lengths among them.
                    if b.text != a.text && !b.path.contains("/values/") {
                        found.push(format!("{here}: {} {} vs {}", b.path, a.text, b.text));
                    }
                    continue;
                };
                checked += 1;
                if !(v1.is_finite() && vs.is_finite()) {
                    if v1.to_bits() != vs.to_bits() {
                        found.push(format!("{here}: {} {vs} vs {v1}", b.path));
                    }
                    continue;
                }
                if v1.abs() < ZERO {
                    zeros.push((b.path.clone(), v1, vs));
                    continue;
                }
                // The power of `s` the figure is nearest to.
                let off = |k: i32| (vs - s.powi(k) * v1).abs() / (s.powi(k) * v1).abs();
                let k = POWERS
                    .clone()
                    .min_by(|a, b| off(*a).total_cmp(&off(*b)))
                    .unwrap_or(0);
                let expected = s.powi(k) * v1;
                let departure = off(k);
                let fits = (vs - expected).abs() <= REL * expected.abs() + FLOOR * s.powi(k);
                if fits && departure > worst.0 {
                    worst = (departure, format!("{} ({here})", b.path));
                }
                if !fits {
                    found.push(format!(
                        "{here}: {} is {vs:e} against {v1:e}, no power of s (ratio {:e})",
                        b.path,
                        vs / v1
                    ));
                    continue;
                }
                match powers.get(&b.path) {
                    Some((j, first)) if *j != k => found.push(format!(
                        "{here}: {} scales as s^{k}, as s^{j} at {first}",
                        b.path
                    )),
                    Some(_) => {}
                    None => {
                        powers.insert(b.path.clone(), (k, here.clone()));
                    }
                }
            }
            for (path, v1, vs) in zeros {
                let k = powers.get(&path).map_or(0, |(k, _)| *k);
                if (vs - s.powi(k) * v1).abs() > ZERO * s.powi(k).max(1.0) {
                    found.push(format!("{here}: {path} is {vs:e} against {v1:e} near zero"));
                }
            }
        }
    }
    for (path, (k, first)) in &powers {
        // The leaf's own name, or where that only says which end or which
        // bound, the nearest name above it that says what the figure is.
        let expected = path
            .split('/')
            .rev()
            .skip_while(|n| GENERIC.contains(n))
            .take(1)
            .find_map(|n| EXPECTED.iter().find(|(e, _)| *e == n).map(|(_, k)| *k));
        if let Some(e) = expected {
            if e != *k {
                found.push(format!(
                    "{path} scales as s^{k} (from {first}); its unit says s^{e}"
                ));
            }
        }
    }
    Breaches {
        found,
        checked,
        worst,
        refused,
    }
}

#[test]
fn every_preset_is_homogeneous_in_length() {
    let b = breaches(None);
    assert!(b.refused.is_empty(), "refused unscaled: {:?}", b.refused);
    assert!(b.checked > 0);
    assert!(
        b.found.is_empty(),
        "{} breaches of homogeneity, first:\n{}",
        b.found.len(),
        b.found
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
    eprintln!(
        "{} figures, worst relative departure {:e} at {}",
        b.checked, b.worst.0, b.worst.1
    );
}

/// **The law can fail**: each millimetre input left unscaled in turn — the
/// known break, a millimetre default that does not follow the module — and
/// the law says so. Every input in [`MILLIMETRES`] is read by some train in
/// [`trains`], so none may leave it silent.
#[test]
fn each_millimetre_input_left_unscaled_breaks_it() {
    let mut silent = Vec::new();
    for name in MILLIMETRES {
        let b = breaches(Some(name));
        eprintln!(
            "{name} left unscaled: {} breaches{}",
            b.found.len(),
            b.found
                .first()
                .map_or(String::new(), |f| format!(", first {f}"))
        );
        if b.found.is_empty() {
            silent.push(name);
        }
    }
    assert!(
        silent.is_empty(),
        "left unscaled and nothing moved: {silent:?}"
    );
}
