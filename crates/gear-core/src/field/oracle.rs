//! The differential oracle's records (`tests/data/field_oracle/`, its README), as each step's test
//! reads them: every record's inputs, the prototype's outputs, and the tolerance it states.

use serde_json::Value;

/// One record: `{id, fn, inputs, outputs, tol}`.
pub(crate) struct Record<'a> {
    pub id: &'a str,
    pub inputs: &'a Value,
    pub outputs: &'a Value,
    pub tol: &'a Value,
}

/// A module's records, in file order.
pub(crate) fn records(module: &Value) -> Vec<Record<'_>> {
    module["records"]
        .as_array()
        .expect("a module file holds a list of records")
        .iter()
        .map(|r| Record {
            id: r["id"].as_str().expect("every record has an id"),
            inputs: &r["inputs"],
            outputs: &r["outputs"],
            tol: &r["tol"],
        })
        .collect()
}

/// A number the record must hold.
pub(crate) fn num(v: &Value) -> f64 {
    v.as_f64().unwrap_or_else(|| panic!("a number, found {v}"))
}

/// A number the record may hold: `null` is the prototype's `None`.
pub(crate) fn opt(v: &Value) -> Option<f64> {
    (!v.is_null()).then(|| num(v))
}

/// The closed-form rule's bound on a value: `rel` of its size, and `rel` absolute where the value
/// passes zero (the README's tolerance rule), so a value that is zero on one piece is not held to
/// a relative bound of nothing.
pub(crate) fn closed_bound(rel: f64, oracle: f64, passes_zero: bool) -> f64 {
    if passes_zero {
        rel * oracle.abs().max(1.0)
    } else {
        rel * oracle.abs()
    }
}

/// How far the port misses the oracle, over the bound: at most 1 reproduces it.
pub(crate) fn miss(port: f64, oracle: f64, bound: f64) -> f64 {
    let d = (port - oracle).abs();
    if d == 0.0 {
        0.0
    } else {
        d / bound
    }
}

/// The worst miss seen, where, and how many values were compared.
#[derive(Default)]
pub(crate) struct Worst {
    pub ratio: f64,
    pub at: String,
    pub count: usize,
}

impl Worst {
    pub fn see(&mut self, ratio: f64, at: impl FnOnce() -> String) {
        self.count += 1;
        if ratio > self.ratio || ratio.is_nan() {
            self.ratio = ratio;
            self.at = at();
        }
    }
}

/// The version of the oracle this copy must be: `index.json`'s `version`.
pub(crate) const VERSION: u64 = 2;

/// What `index` and the module files disagree on: the version, and per module the record ids
/// the index lists against those the file holds (as sets: the index lists them sorted). Empty
/// when the copy is whole and of [`VERSION`].
pub(crate) fn index_mismatches(index: &Value, files: &[(&str, &Value)]) -> Vec<String> {
    let mut out = Vec::new();
    if index["version"].as_u64() != Some(VERSION) {
        out.push(format!("version {} is not {VERSION}", index["version"]));
    }
    let listed = index["modules"].as_object().expect("index lists modules");
    if listed.len() != files.len() {
        out.push(format!(
            "{} modules listed, {} files",
            listed.len(),
            files.len()
        ));
    }
    for (name, file) in files {
        let ids = |v: &Value| -> Vec<String> {
            let mut ids: Vec<String> = v
                .as_array()
                .map(|a| {
                    a.iter()
                        .map(|x| x.as_str().expect("an id").to_owned())
                        .collect()
                })
                .unwrap_or_default();
            ids.sort();
            ids
        };
        let held: Vec<String> = {
            let mut v: Vec<String> = records(file).iter().map(|r| r.id.to_owned()).collect();
            v.sort();
            v
        };
        if file["module"].as_str() != Some(name) {
            out.push(format!("{name}.json names module {}", file["module"]));
        }
        if ids(&listed[*name]) != held {
            out.push(format!("{name}: the index's records are not the file's"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Value {
        serde_json::from_str(text).expect("the oracle's JSON parses")
    }

    fn copy() -> (Value, Vec<(&'static str, Value)>) {
        let index = parse(include_str!("../../tests/data/field_oracle/index.json"));
        let files = vec![
            (
                "form",
                include_str!("../../tests/data/field_oracle/form.json"),
            ),
            (
                "kernel",
                include_str!("../../tests/data/field_oracle/kernel.json"),
            ),
            (
                "across",
                include_str!("../../tests/data/field_oracle/across.json"),
            ),
            (
                "compliance",
                include_str!("../../tests/data/field_oracle/compliance.json"),
            ),
            (
                "gap",
                include_str!("../../tests/data/field_oracle/gap.json"),
            ),
            (
                "matched",
                include_str!("../../tests/data/field_oracle/matched.json"),
            ),
            (
                "state",
                include_str!("../../tests/data/field_oracle/state.json"),
            ),
            (
                "phase",
                include_str!("../../tests/data/field_oracle/phase.json"),
            ),
        ];
        (
            index,
            files.into_iter().map(|(n, t)| (n, parse(t))).collect(),
        )
    }

    fn mismatches(index: &Value, files: &[(&'static str, Value)]) -> Vec<String> {
        let refs: Vec<(&str, &Value)> = files.iter().map(|(n, v)| (*n, v)).collect();
        index_mismatches(index, &refs)
    }

    /// The repository's copy is version 2, whole: every module the index lists is here and holds
    /// exactly the records the index names, 106 in all, and the index records no failure.
    #[test]
    fn the_copy_is_version_two_and_whole() {
        let (index, files) = copy();
        assert_eq!(mismatches(&index, &files), Vec::<String>::new());
        let total: usize = files.iter().map(|(_, f)| records(f).len()).sum();
        assert_eq!((files.len(), total), (8, 106));
        assert_eq!(index["errors"].as_array().map(Vec::len), Some(0));
    }

    /// Its plants: version 1's index (no `version`), an index a record short, and a file holding
    /// one record more than the index (a record added to the copy by hand) are each refused.
    #[test]
    fn the_version_check_fails_its_plants() {
        let (index, files) = copy();
        let mut v1 = index.clone();
        v1.as_object_mut().expect("an object").remove("version");
        assert_eq!(mismatches(&v1, &files).len(), 1);

        let mut short = index.clone();
        short["modules"]["form"]
            .as_array_mut()
            .expect("a list")
            .pop();
        assert_eq!(mismatches(&short, &files).len(), 1);

        let mut more = files.clone();
        let extra = more[0].1["records"][0].clone();
        more[0].1["records"]
            .as_array_mut()
            .expect("a list")
            .push(extra);
        assert_eq!(mismatches(&index, &more).len(), 1);
    }
}
