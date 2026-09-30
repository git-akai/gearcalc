//! **Every value a geartrain file holds, set to what describes nothing, is
//! refused naming it — or read, and solved, without a panic.**
//!
//! Each arrangement's document, as the panel starts it (cased between its
//! chain's two ends), is written, and every number in it is replaced in
//! turn: each float by a not-a-number, each infinity, nought, minus one and
//! a huge value; each integer by nought, 99, and the length of every list
//! the document holds and one past it — so every index is asked at its
//! list's length, one past it and 99. Each file is read ([`from_toml`]) and,
//! where it is read, solved; a panic anywhere fails the law. A value the
//! reader refuses is refused by the field's own path: the refusal's note
//! carries `field`, and it is the path that was changed. A structural
//! refusal ([`Train::validate`]'s graph invariants) names its piece instead,
//! and is accepted as such.
//!
//! Every float not finite is refused, whatever field it is in, since no
//! field of a train means anything at a not-a-number or an infinity; and
//! every integer is refused by name at one of its replacements — so every
//! number the file holds has a row in the table (`gear_core::input`).

#![allow(clippy::unwrap_used, clippy::panic)]

use gear_core::note::key;
use gear_core::train::sweep::{arrangements, cased};
use gear_core::train::Train;
use gear_io::train::{from_toml, to_toml, DocumentError, TrainDocument};
use std::panic::{catch_unwind, AssertUnwindSafe};

/// Every leaf of `v` that is a number, by its path — the table keys and
/// array indices from the root, joined by `.` — and whether it is a float.
fn leaves(v: &toml::Value, at: &str, out: &mut Vec<(String, bool)>) {
    let join = |k: &str| {
        if at.is_empty() {
            k.to_owned()
        } else {
            format!("{at}.{k}")
        }
    };
    match v {
        toml::Value::Integer(_) => out.push((at.to_owned(), false)),
        toml::Value::Float(_) => out.push((at.to_owned(), true)),
        toml::Value::Array(a) => {
            for (i, x) in a.iter().enumerate() {
                leaves(x, &join(&i.to_string()), out);
            }
        }
        toml::Value::Table(t) => {
            for (k, x) in t {
                leaves(x, &join(k), out);
            }
        }
        _ => {}
    }
}

/// `v` with the leaf at `path` replaced by `with`.
fn set(v: &mut toml::Value, path: &str, with: toml::Value) {
    let mut at = v;
    for step in path.split('.') {
        at = match at {
            toml::Value::Table(t) => t.get_mut(step).unwrap(),
            toml::Value::Array(a) => &mut a[step.parse::<usize>().unwrap()],
            _ => panic!("{path}: no {step}"),
        };
    }
    *at = with;
}

/// The length of every list the train holds — what an index is asked at.
fn lengths(t: &Train) -> Vec<i64> {
    let s = &t.shape;
    let mut n = vec![
        s.axes.len(),
        s.bodies.len(),
        s.members.len(),
        s.meshes.len(),
        s.distances.len(),
        s.couplings.len(),
        t.load_cases.len(),
    ];
    n.extend(t.load_cases.iter().map(|c| c.loads.len()));
    let mut out: Vec<i64> = n
        .into_iter()
        .flat_map(|k| [k, k + 1])
        .map(|k| i64::try_from(k).unwrap())
        .collect();
    out.extend([0, 99]);
    out.sort_unstable();
    out.dedup();
    out
}

/// The keys of the structural refusals — a graph that describes no train —
/// which name their piece rather than a field.
const STRUCTURAL: &[&str] = &[
    key::ERROR_TRAIN_MALFORMED_CARRIED_BY,
    key::ERROR_TRAIN_MALFORMED_CARRIED_BY_CYCLE,
    key::ERROR_TRAIN_MALFORMED_DISTANCE_TWICE,
    key::ERROR_TRAIN_MALFORMED_RING_FIRST,
    key::ERROR_TRAIN_MALFORMED_DISTANCE_OFF_FRAME,
    key::ERROR_TRAIN_MALFORMED_NUMBER_GAP,
];

/// The message of a panic.
fn message(e: &(dyn std::any::Any + Send)) -> String {
    e.downcast_ref::<String>()
        .cloned()
        .or_else(|| e.downcast_ref::<&str>().map(ToString::to_string))
        .unwrap_or_default()
}

/// How many replacements came to what.
#[derive(Default)]
struct Count {
    asked: usize,
    /// Read, and solved (to an answer or the train's own failure).
    solved: usize,
    /// Refused by the value's row, naming its path.
    named: usize,
    /// Refused as a graph that describes no train.
    structural: usize,
    /// Not finite, and refused.
    not_finite: usize,
}

/// What one arrangement's document came to under every replacement: the
/// panics, and the refusals that did not name the field replaced.
fn sweep(name: &str, train: Train, n: &mut Count) -> Vec<String> {
    let lib = gear_io::default_library();
    let doc = TrainDocument {
        name: name.to_owned(),
        train,
    };
    let text = to_toml(&doc).unwrap();
    let root: toml::Value = toml::from_str(&text).unwrap();
    let mut all = Vec::new();
    leaves(&root, "", &mut all);
    let indices = lengths(&doc.train);
    let floats = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, 0.0, -1.0, 1e300];
    let mut faults = Vec::new();
    for (path, float) in &all {
        if path == "format" {
            continue;
        }
        let with: Vec<toml::Value> = if *float {
            floats.iter().map(|&x| toml::Value::Float(x)).collect()
        } else {
            indices.iter().map(|&k| toml::Value::Integer(k)).collect()
        };
        // Whether some replacement of this leaf was refused naming it: every
        // integer is, at nought, 99 or past its list, where it has a row.
        let mut has_a_row = false;
        for value in with {
            n.asked += 1;
            let mut doc = root.clone();
            set(&mut doc, path, value.clone());
            let text = toml::to_string(&doc).unwrap();
            let said = format!("{name}: {path} = {value}");
            let read = catch_unwind(AssertUnwindSafe(|| from_toml(&text)));
            let read = match read {
                Err(e) => {
                    faults.push(format!("{said}: reading panicked: {}", message(&*e)));
                    continue;
                }
                Ok(r) => r,
            };
            let not_finite = value.as_float().is_some_and(|x| !x.is_finite());
            match read {
                Ok(imported) => {
                    if not_finite {
                        faults.push(format!("{said}: read, not refused"));
                    }
                    let train = imported.document.train;
                    match catch_unwind(AssertUnwindSafe(|| {
                        gear_core::train::solve_train(&train, &lib)
                    })) {
                        Err(e) => {
                            faults.push(format!("{said}: solving panicked: {}", message(&*e)));
                        }
                        Ok(_) => n.solved += 1,
                    }
                }
                Err(DocumentError::Parse(e)) => {
                    // An integer past what its type holds is the parser's,
                    // which names the key; nothing here is past a u32.
                    faults.push(format!("{said}: refused by the parser: {e}"));
                }
                Err(e) => {
                    let note = e.note().unwrap();
                    let named = note.values.get("field").map(String::as_str);
                    let structural = STRUCTURAL.contains(&note.key.as_str());
                    if named == Some(path.as_str()) {
                        has_a_row = true;
                        n.named += 1;
                        n.not_finite += usize::from(not_finite);
                    } else if structural && !not_finite {
                        n.structural += 1;
                    } else {
                        faults.push(format!(
                            "{said}: refused as {} {:?}, not naming the field",
                            note.key, note.values
                        ));
                    }
                }
            }
        }
        if !has_a_row {
            faults.push(format!(
                "{name}: {path}: nothing refused by name — a number with no row"
            ));
        }
    }
    faults
}

#[test]
fn every_value_a_file_holds_is_refused_by_name_or_read_without_a_panic() {
    std::panic::set_hook(Box::new(|_| {}));
    let mut n = Count::default();
    let mut faults = Vec::new();
    let mut floats = 0;
    for (name, shape) in arrangements() {
        let train = cased(vec![shape]);
        // Each document reads and solves as written, or the replacements
        // ask nothing.
        let text = to_toml(&TrainDocument {
            name: name.clone(),
            train: train.clone(),
        })
        .unwrap();
        let mut all = Vec::new();
        leaves(&toml::from_str(&text).unwrap(), "", &mut all);
        floats += all.iter().filter(|(_, f)| *f).count();
        from_toml(&text).unwrap_or_else(|e| panic!("{name} as written: {e}"));
        faults.extend(sweep(&name, train, &mut n));
    }
    let _ = std::panic::take_hook();
    assert!(
        faults.is_empty(),
        "{} of {} replacements:\n{}",
        faults.len(),
        n.asked,
        faults.join("\n")
    );
    // Every branch ran, and every not-finite float was refused by name.
    eprintln!(
        "asked {}: solved {}, refused by name {} ({} not finite), structurally {}",
        n.asked, n.solved, n.named, n.not_finite, n.structural
    );
    assert_eq!(
        n.not_finite,
        3 * floats,
        "a not-finite value not refused by name"
    );
    assert_eq!(n.asked, n.solved + n.named + n.structural);
    assert!(n.solved > 0 && n.named > 0 && n.structural > 0);
}

/// **The file trap, refused by name** (carried item 1): a file whose gear
/// has a helix of ±90°, a pressure angle of 0° or 90°, a thickness
/// coefficient of 0 or 2, a module of nought — or any of them not a number
/// — read the graph alone before the table, then built a tooth of it and
/// indexed its empty flank (`&r[1..]`). Each is refused where the file is
/// read, naming the field; the angle just inside each bound reads.
#[test]
fn the_file_trap_is_refused_by_name() {
    let lib = gear_io::default_library();
    let doc = TrainDocument {
        name: "trap".into(),
        train: cased(vec![gear_core::train::Preset::Spur.build()]),
    };
    let root: toml::Value = toml::from_str(&to_toml(&doc).unwrap()).unwrap();
    let at = "train.shape.members.0";
    let given = |v: f64| {
        let mut t = toml::Table::new();
        t.insert("auto".into(), toml::Value::Boolean(false));
        t.insert("manual".into(), toml::Value::Float(v));
        toml::Value::Table(t)
    };
    let cases: [(&str, f64, bool); 13] = [
        ("gear.helix_angle", 90.0, false),
        ("gear.helix_angle", -90.0, false),
        ("gear.helix_angle", f64::NAN, false),
        ("gear.helix_angle", 89.999, true),
        ("pressure_angle", 90.0, false),
        ("pressure_angle", 0.0, false),
        ("pressure_angle", 89.999, true),
        ("pressure_angle", 0.001, true),
        ("thickness_mod", 2.0, false),
        ("thickness_mod", 0.0, false),
        ("thickness_mod", 1.999, true),
        ("module", 0.0, false),
        ("module", f64::NAN, false),
    ];
    let mut refused = 0;
    for (field, v, reads) in cases {
        let mut file = root.clone();
        set(&mut file, &format!("{at}.{field}"), given(v));
        let text = toml::to_string(&file).unwrap();
        match from_toml(&text) {
            Ok(imported) if reads => {
                let r = catch_unwind(AssertUnwindSafe(|| {
                    gear_core::train::solve_train(&imported.document.train, &lib)
                }));
                assert!(r.is_ok(), "{field} = {v}: the solve panicked");
            }
            Err(e) if !reads => {
                let note = e.note().unwrap();
                assert_eq!(note.values["field"], format!("{at}.{field}.manual"), "{v}");
                refused += 1;
            }
            other => panic!("{field} = {v}: {:?}", other.map(|_| "read")),
        }
    }
    assert_eq!(refused, 9);
}
