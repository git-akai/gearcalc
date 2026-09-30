//! **The solve is homogeneous in length.** Scale the module and every input
//! stated in millimetres by `s`, and every torque by `s³` so every stress
//! stays put: then every dimensionless figure — ratio, efficiency, contact
//! ratio, stress, every angle, every note's key and every refusal — is the
//! unscaled solve's, every length is `s` times it, every curvature `1/s`
//! times and every torque `s³` times.
//!
//! The law is read off the whole result rather than a chosen list: the pretty
//! `Debug` of each solve is walked into one token per leaf, keyed by its field
//! path. Text tokens (keys, flags, variants, refusals) must match exactly.
//! **Every numeric leaf must be classified** in [`CLASSES`] by the power of
//! `s` its unit gives it, so a figure the law does not know fails rather than
//! passing on whatever power it happens to show — a length pinned to a fixed
//! millimetre value is a length that did not scale.
//!
//! What it does not change: any default's unit. The presets keep their
//! millimetre figures; the law scales them with the module, so it tests the
//! model's homogeneity and not what the defaults should be.

use super::arrangements::Preset;
use super::{solve_train, test_library, Duty, LoadCase, Train};
use crate::params::Auto;

/// The scales: a decade either side; the smallest module the input table
/// admits ([`crate::input::SMALLEST_MODULE`]), where an absolute floor in
/// the tips' sizing once refused a watch's planocentric; and one whose
/// torque, `s³` the unscaled one's, stays inside the table's figures
/// ([`crate::input::CEILING`], 2¹²⁷ ≈ 1.7e38 against 1e36).
const SCALES: [f64; 4] = [crate::input::SMALLEST_MODULE, 0.1, 10.0, 1e12];

/// **How far a figure may move between the two solves**, in roundings.
///
/// A figure scaled by `s` is computed from operands scaled by `s`, and the two
/// solves round differently, so each carries some roundings of relative size
/// `ε` on its operands — not on itself: a figure taken as a difference (a
/// play, a tip margin, a far gap) keeps the operands' absolute error while
/// its own value shrinks. So the allowance is
///
/// ```text
/// |v_s − s^k v₁|  ≤  ROUNDINGS · ε · κ · s^k · (|v₁| + O_k)
/// ```
///
/// with `κ` the train's condition figure — its largest length over its
/// smallest millimetre input, how far a relative error in the one can be
/// magnified in a difference down to the other. `κ` is a heuristic, not a
/// derived condition number: no error analysis of the solve bounds it, and
/// a figure whose amplification exceeds that ratio would breach the
/// allowance without being wrong. `O_k` is the scale of the
/// figure's operands ([`operand_scale`]). `ROUNDINGS` counts the roundings
/// one figure passes through between an input and the output: a root solve
/// to `2ε` ([`crate::solve`]), a trigonometric evaluation, a difference, each
/// some few, over a handful of steps; 16 covers that. The law reports the
/// count it needed: 0.5, at the head it was written on.
const ROUNDINGS: f64 = 16.0;

/// What a figure's unit makes of `s`.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Class {
    /// Scales as `s^k`: 0 dimensionless, 1 a length, −1 a curvature, 3 a
    /// torque.
    Power(i32),
    /// Formatted text holding a number; compared nowhere, since a scaled
    /// length reads differently at a fixed count of decimals. The number
    /// behind each is a figure this law checks where the result holds it.
    NoteValue,
}

/// **Every numeric leaf of a solved train, by the power of `s` its unit
/// gives it.** A pattern is a suffix of the leaf's path with list elements
/// dropped; `*` matches any one name. The longest matching pattern decides.
const CLASSES: &[(&str, Class)] = &[
    // Lengths, mm.
    ("distances/clearance", Class::Power(1)),
    ("distances/nominal", Class::Power(1)),
    ("distances/running", Class::Power(1)),
    ("layout/clearance", Class::Power(1)),
    ("members/face_width", Class::Power(1)),
    ("members/recommended_face_width", Class::Power(1)),
    ("members/pitch_diameter", Class::Power(1)),
    ("members/lead", Class::Power(1)),
    ("params/module", Class::Power(1)),
    ("min_face_width/*", Class::Power(1)),
    ("contact/patch_length", Class::Power(1)),
    ("contact/patch_width", Class::Power(1)),
    ("contact/worst_position", Class::Power(1)),
    ("cases/sliding_velocity", Class::Power(1)),
    ("point/axial_travel", Class::Power(1)),
    ("point/face_width_for_continuity", Class::Power(1)),
    ("tips/far_gap", Class::Power(1)),
    // Curvatures, 1/mm.
    ("contact/curvature_across", Class::Power(-1)),
    ("contact/curvature_along", Class::Power(-1)),
    // Torques, N·m, scaled as s³ with the loads.
    ("bodies/torque", Class::Power(3)),
    ("cases/torque", Class::Power(3)),
    ("cases/on_body", Class::Power(3)),
    ("cases/torques", Class::Power(3)),
    // Angles: degrees where a designer reads one (helix, lead, pressure,
    // backlash), radians in a play row and a tip margin.
    ("helix_angle", Class::Power(0)),
    ("lead_angle", Class::Power(0)),
    ("pressure_angle", Class::Power(0)),
    ("operating_pressure_angle", Class::Power(0)),
    ("backlash/*", Class::Power(0)),
    ("backlash/*/*", Class::Power(0)),
    ("meshes/row_play", Class::Power(0)),
    ("tips/tip_margin", Class::Power(0)),
    // Ratios, efficiencies and fractions of the power in.
    ("paths/ratio", Class::Power(0)),
    ("paths/per_tooth", Class::Power(0)),
    ("reads/turns", Class::Power(0)),
    ("efficiency/*", Class::Power(0)),
    ("circulation/*", Class::Power(0)),
    ("locking_friction/*", Class::Power(0)),
    ("meshes/contact_ratio", Class::Power(0)),
    ("contact_ratios/*", Class::Power(0)),
    ("meshes/sliding_ratio", Class::Power(0)),
    ("cases/power_through", Class::Power(0)),
    // Coefficients in modules, and the ranges they may take.
    ("addendum", Class::Power(0)),
    ("dedendum", Class::Power(0)),
    ("root_radius", Class::Power(0)),
    ("thickness_mod", Class::Power(0)),
    ("profile_shift", Class::Power(0)),
    ("angular_shift", Class::Power(0)),
    ("index_offset", Class::Power(0)),
    ("ranges/*/min", Class::Power(0)),
    ("ranges/*/max", Class::Power(0)),
    ("ranges/*/bound/*", Class::Power(0)),
    ("ranges/profile_shift/*", Class::Power(0)),
    // Stresses, MPa: unmoved, since the torque scales as the section.
    ("cases/bending_stress", Class::Power(0)),
    ("cases/contact_stress", Class::Power(0)),
    ("contact/max_pressure", Class::Power(0)),
    ("contact/at_pitch_point", Class::Power(0)),
    // Speeds and cycle counts: inputs the law leaves alone.
    ("cases/speed", Class::Power(0)),
    ("cases/speeds", Class::Power(0)),
    ("cases/speed_against_carrier", Class::Power(0)),
    ("bodies/speed", Class::Power(0)),
    ("cycles/*", Class::Power(0)),
    // The material's constants, which no length enters.
    ("material/*/value", Class::Power(0)),
    // ...and the hardness a flank estimate is read from.
    ("material/contact_estimate/hardness/value", Class::Power(0)),
    // Indices and counts: must not move at all.
    ("case", Class::Power(0)),
    ("bodies/at", Class::Power(0)),
    ("paths/from", Class::Power(0)),
    ("paths/to", Class::Power(0)),
    ("layout/axis", Class::Power(0)),
    ("layout/count", Class::Power(0)),
    ("params/teeth", Class::Power(0)),
    ("ranges/teeth/min", Class::Power(0)),
    ("axes/count/min", Class::Power(0)),
    ("axes/count/max", Class::Power(0)),
    ("distances/sized_by", Class::Power(0)),
    // The angle between two carried axes, from a triangle of lengths.
    ("distances/stagger", Class::Power(0)),
    // A note's values.
    ("values/*", Class::NoteValue),
];

/// The class of a leaf's path, by its longest matching pattern.
fn class_of(path: &str) -> Option<Class> {
    let names: Vec<&str> = path.split('/').filter(|n| *n != "[]").collect();
    CLASSES
        .iter()
        .filter(|(pattern, _)| {
            let p: Vec<&str> = pattern.split('/').collect();
            p.len() <= names.len()
                && p.iter()
                    .zip(&names[names.len() - p.len()..])
                    .all(|(a, b)| *a == "*" || a == b)
        })
        .max_by_key(|(pattern, _)| pattern.split('/').count())
        .map(|(_, c)| *c)
}

/// The scale of a figure's operands, beside the figure itself: a
/// dimensionless figure is made of order-one operands (angles in radians,
/// ratios), a length of the train's lengths, a torque of the train's torques;
/// a curvature is its own operand.
fn operand_scale(k: i32, largest: &[(i32, f64)]) -> f64 {
    match k {
        0 => 1.0,
        -1 => 0.0,
        _ => largest
            .iter()
            .find(|(j, _)| *j == k)
            .map_or(0.0, |(_, v)| *v),
    }
}

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

/// The inputs [`scaled`] multiplies, with the power of `s` each takes: every
/// millimetre input of the train, found by reading every `f64` of `Shape`,
/// `Axis`, `Member`, `MemberGear`, `Distance` and `Load` for its unit, and
/// the torque. The rest are coefficients in modules (addendum, dedendum,
/// root radius, working depth, a cutter's), angles, counts, frictions,
/// speeds and material figures.
const INPUTS: [(&str, i32); 13] = [
    ("min_planet_clearance", 1),
    ("module", 1),
    ("pitch_diameter", 1),
    ("face_width", 1),
    ("min_tip_width", 1),
    ("rim_thickness", 1),
    ("distance", 1),
    ("clearance", 1),
    ("tip_clearance", 1),
    ("tolerance_plus", 1),
    ("tolerance_minus", 1),
    ("axial_clearance", 1),
    ("torque", 3),
];

/// The train with every input in [`INPUTS`] scaled by its power of `s`,
/// except `keep`, left as it was.
fn scaled(train: &Train, s: f64, keep: Option<&str>) -> Train {
    let f = |name: &str| {
        let k = INPUTS
            .iter()
            .find(|(n, _)| *n == name)
            .map_or(0, |(_, k)| *k);
        if keep == Some(name) {
            1.0
        } else {
            s.powi(k)
        }
    };
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
            l.torque.manual *= f("torque");
        }
    }
    t
}

/// **How much a flank junction near its base circle magnifies a rounding**,
/// where the path of contact is cut there.
///
/// A mesh whose tip reaches past a flank's usable end, or is held to meet
/// it, has its path end at that member's junction, `ρ_j = r_b u_j` along the line of action. Near the
/// base circle the involute is a cusp: its radius moves as `r_b u du`, so a
/// junction located to a rounding `ε r_b` in position is located to
/// `ε / u_j` in roll — a derivative, not a solver's tolerance — and every
/// figure read off the cut path carries that factor. `u_j` itself resolves
/// to no better than `√ε` there (`r − r_b = r_b u²/2`), which bounds the
/// factor. One on every train whose paths no junction cuts; a ring's
/// junction is its generation limit, well off its base circle, and is not
/// counted.
fn cusp(train: &Train, r: &super::TrainResult) -> f64 {
    let mut worst: f64 = 1.0;
    for (k, mesh) in r.meshes.iter().enumerate() {
        let m = train.shape.meshes[k];
        for (side, i) in [m.a, m.b].into_iter().enumerate() {
            // Cut there: reached past, or met exactly by a mate's tip held
            // to it (`MemberGear::no_tip_past_mate_flank`).
            let mate = [m.b, m.a][side];
            let held = r.members[mate]
                .notes
                .iter()
                .any(|n| n.is(crate::note::key::GEAR_ADDENDUM_HELD_TO_MATE_FLANK));
            if !(mesh.flank_interference[side] || held) || train.shape.members[i].ring.is_some() {
                continue;
            }
            let u = crate::Tooth::new(r.members[i].params).u_j.abs();
            worst = worst.max(1.0 / u.max(f64::EPSILON.sqrt()));
        }
    }
    worst
}

/// The smallest millimetre input the train is given, other than zero: the
/// smallest length a difference in the solve comes down to.
fn smallest_input(train: &Train) -> f64 {
    let s = &train.shape;
    let axes = s.axes.iter().map(|a| a.min_planet_clearance);
    let members = s.members.iter().flat_map(|m| {
        [
            m.module.manual,
            m.pitch_diameter.manual,
            m.gear.face_width.manual,
            m.gear.min_tip_width,
            m.gear.rim_thickness.unwrap_or(0.0),
        ]
    });
    let distances = s.distances.iter().flat_map(|d| {
        [
            d.distance.manual,
            d.clearance.manual,
            d.tip_clearance,
            d.tolerance_plus,
            d.tolerance_minus,
            d.axial_clearance,
        ]
    });
    axes.chain(members)
        .chain(distances)
        .map(f64::abs)
        .filter(|v| *v > 0.0)
        .fold(f64::INFINITY, f64::min)
}

/// The loads every train is asked under: the three cases `gear-cli
/// kinematics` gives each of its fixtures (a drive from the input, a
/// back-driving load from the output, a fatigue duty), so the law covers
/// what the corpus records.
const TORQUE_NM: f64 = 2.0;
const SPEED_RPM: f64 = 3000.0;
const BACK_DRIVING_NM: f64 = 0.6;
const FATIGUE_SPEED_RPM: f64 = 2400.0;
const RUNTIME_HOURS: f64 = 1000.0;

/// A given running distance: the spur preset's reference distance opened by
/// this many modules — more than the clearance, so the shifts must absorb
/// it, and well inside what a shift can reach.
const GIVEN_OPENING_MODULES: f64 = 0.3;
/// A rim of two modules under a tooth of two and a quarter: thin enough that
/// ISO 6336-3's rim factor de-rates the root, so the rim is read.
const RIM_MODULES: f64 = 2.0;
/// A planet gap asked of the planetary preset wider than its three planets
/// leave (a few millimetres at module 1), so the layout's gap is read and
/// its note fires.
const PLANET_GAP_MM: f64 = 10.0;

/// Each preset, and the arrangements no preset lays out, asked alone under
/// the three cases above.
fn trains() -> Vec<(String, Train)> {
    let mut shapes = super::testing::arrangements();
    // Three inputs no preset reads at its defaults, given so the law reaches
    // them: a running distance, a thin rim, and a planet gap.
    let mut given = Preset::Spur.build();
    let module = given.members[0].module.manual;
    let teeth: u32 = given.members.iter().map(|m| m.gear.teeth).sum();
    given.distances[0].distance =
        Auto::fixed(module * (0.5 * f64::from(teeth) + GIVEN_OPENING_MODULES));
    given.members[0].gear.rim_thickness = Some(RIM_MODULES * module);
    shapes.push(("spur, distance and rim given".into(), given));
    let mut gap = Preset::Planetary.build();
    for a in &mut gap.axes {
        a.min_planet_clearance = PLANET_GAP_MM;
    }
    shapes.push(("planetary, planet gap asked".into(), gap));
    shapes
        .into_iter()
        .map(|(name, shape)| {
            let mut t = Train::alone(&shape, TORQUE_NM, SPEED_RPM);
            let (input, output) = (t.load_cases[0].loads[0].at, t.load_cases[0].loads[1].at);
            t.load_cases = vec![
                LoadCase::ultimate(input, output, TORQUE_NM, SPEED_RPM),
                LoadCase::back_driving(input, output, BACK_DRIVING_NM),
                LoadCase {
                    duty: Duty::Continuous {
                        runtime_hours: RUNTIME_HOURS,
                    },
                    ..LoadCase::fatigue(input, output, TORQUE_NM, FATIGUE_SPEED_RPM)
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
    /// The most roundings any figure that held needed, and where.
    needed: (f64, String),
    /// Trains the unscaled solve refused, which the law then says little of.
    refused: Vec<String>,
}

/// The law over every train at every scale, with every input scaled but
/// `keep`, and `plant` applied to each scaled solve's text — the way a
/// negative control changes the result without changing the core.
fn breaches(keep: Option<&str>, plant: &dyn Fn(String, f64) -> String) -> Breaches {
    let lib = test_library();
    let mut b = Breaches {
        found: Vec::new(),
        checked: 0,
        needed: (0.0, String::new()),
        refused: Vec::new(),
    };
    for (name, train) in trains() {
        let solved = solve_train(&train, &lib);
        if let Err(e) = &solved {
            b.refused.push(format!("{name}: {e:?}"));
        }
        let base = leaves(&plant(format!("{solved:#?}"), 1.0));
        // κ and the operand scales, from the unscaled train.
        let mut largest: Vec<(i32, f64)> = Vec::new();
        for leaf in &base {
            if let (Some(Class::Power(k)), Ok(v)) = (class_of(&leaf.path), leaf.text.parse::<f64>())
            {
                if v.is_finite() {
                    match largest.iter_mut().find(|(j, _)| *j == k) {
                        Some((_, m)) => *m = m.max(v.abs()),
                        None => largest.push((k, v.abs())),
                    }
                }
            }
        }
        let kappa = operand_scale(1, &largest) / smallest_input(&train)
            * solved.as_ref().map_or(1.0, |r| cusp(&train, r));
        for s in SCALES {
            let t = scaled(&train, s, keep);
            let at = leaves(&plant(format!("{:#?}", solve_train(&t, &lib)), s));
            let here = format!("{name} at s = {s}");
            if base.len() != at.len() {
                b.found.push(format!(
                    "{here}: {} leaves against {}",
                    at.len(),
                    base.len()
                ));
                continue;
            }
            for (l1, ls) in base.iter().zip(&at) {
                if l1.path != ls.path {
                    b.found
                        .push(format!("{here}: {} where {} was", ls.path, l1.path));
                    break;
                }
                let numbers = (l1.text.parse::<f64>(), ls.text.parse::<f64>());
                let class = class_of(&l1.path);
                let (Ok(v1), Ok(vs)) = numbers else {
                    if class != Some(Class::NoteValue) && l1.text != ls.text {
                        b.found
                            .push(format!("{here}: {} {} vs {}", l1.path, ls.text, l1.text));
                    }
                    continue;
                };
                let k = match class {
                    Some(Class::Power(k)) => k,
                    Some(Class::NoteValue) => continue,
                    None => {
                        b.found
                            .push(format!("{here}: {} is not classified", l1.path));
                        continue;
                    }
                };
                b.checked += 1;
                if !(v1.is_finite() && vs.is_finite()) {
                    if v1.to_bits() != vs.to_bits() {
                        b.found.push(format!("{here}: {} {vs} vs {v1}", l1.path));
                    }
                    continue;
                }
                let unit =
                    f64::EPSILON * kappa * s.powi(k) * (v1.abs() + operand_scale(k, &largest));
                let roundings = (vs - s.powi(k) * v1).abs() / unit;
                if roundings > ROUNDINGS {
                    b.found.push(format!(
                        "{here}: {} is {vs:e}, s^{k} times {v1:e} is {:e} ({roundings:.0} roundings)",
                        l1.path,
                        s.powi(k) * v1
                    ));
                } else if roundings > b.needed.0 {
                    b.needed = (roundings, format!("{} ({here})", l1.path));
                }
            }
        }
    }
    b
}

fn unchanged(text: String, _: f64) -> String {
    text
}

#[test]
fn every_preset_is_homogeneous_in_length() {
    let b = breaches(None, &unchanged);
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
        "{} figures; the most roundings any needed: {:.2} at {}",
        b.checked, b.needed.0, b.needed.1
    );
}

/// **The law can fail**: each input left unscaled in turn — the known break,
/// a millimetre default that does not follow the module — and the law says
/// so. Every input in [`INPUTS`] is read by some train in [`trains`], so none
/// may leave it silent.
#[test]
fn each_input_left_unscaled_breaks_it() {
    let mut silent = Vec::new();
    for (name, _) in INPUTS {
        let b = breaches(Some(name), &unchanged);
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

/// **A length pinned to a fixed millimetre value fails**, even where it is
/// pinned at every scale alike and so scales, consistently, as `s⁰`: the far
/// gap of every internal mesh reported as one millimetre.
#[test]
fn a_length_pinned_to_a_millimetre_value_breaks_it() {
    let pinned = |text: String, _: f64| {
        text.lines()
            .map(|l| match l.split_once("far_gap: ") {
                Some((head, _)) => format!("{head}far_gap: 1.0,"),
                None => l.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n")
    };
    let b = breaches(None, &pinned);
    assert!(
        b.found.iter().any(|f| f.contains("far_gap")),
        "a pinned far gap broke nothing:\n{}",
        b.found.join("\n")
    );
    // And nothing but the pinned figure moved.
    assert!(b.found.iter().all(|f| f.contains("far_gap")));
}
