//! **No `Display` in the core holds a word** (T02.1): every reason a person
//! reads is a note, whose words are the catalogue's, so an `impl Display`
//! under `src/` writes no string literal with a letter in it outside a
//! placeholder. Read off the source, as a reviewer would.
//!
//! One is a name rather than a sentence, and says so: `jgma::Class` prints
//! the standard's own designation of a grade, `Fine 5` or `Standard 7`.

#![allow(clippy::unwrap_used)]

use std::path::Path;

/// `(file, type)` of a `Display` that prints a proper name, not English.
const NAMES: &[(&str, &str)] = &[("jgma.rs", "Class")];

/// The `impl … Display for <type>` blocks of `text`, each its type and body.
fn displays(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = text[at..].find("Display for ") {
        let start = at + i + "Display for ".len();
        let ty: String = text[start..]
            .chars()
            .take_while(|c| c.is_alphanumeric() || *c == '_')
            .collect();
        let open = start + text[start..].find('{').unwrap();
        let (mut depth, mut end) = (0, open);
        for (k, c) in text[open..].char_indices() {
            match c {
                '{' => depth += 1,
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        end = open + k;
                        break;
                    }
                }
                _ => {}
            }
        }
        out.push((ty, text[open..=end].to_string()));
        at = end;
    }
    out
}

/// The string literals in `body` that hold a letter outside `{…}`.
fn words(body: &str) -> Vec<String> {
    let code: String = body
        .lines()
        .map(|l| l.split("//").next().unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n");
    let mut out = Vec::new();
    let mut rest = code.as_str();
    while let Some(i) = rest.find('"') {
        let after = &rest[i + 1..];
        let Some(j) = after.find('"') else { break };
        let literal = &after[..j];
        let mut depth = 0;
        let outside: String = literal
            .chars()
            .filter(|&c| {
                match c {
                    '{' => depth += 1,
                    '}' => depth -= 1,
                    _ => return depth == 0,
                }
                false
            })
            .collect();
        if outside.chars().any(char::is_alphabetic) {
            out.push(literal.to_string());
        }
        rest = &after[j + 1..];
    }
    out
}

fn sources(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    for e in std::fs::read_dir(dir).unwrap() {
        let p = e.unwrap().path();
        if p.is_dir() {
            sources(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

#[test]
fn no_display_in_the_core_holds_a_word() {
    let mut files = Vec::new();
    sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut files,
    );
    let (mut read, mut faults) = (0, Vec::new());
    for f in &files {
        let text = std::fs::read_to_string(f).unwrap();
        let name = f.file_name().unwrap().to_string_lossy().to_string();
        for (ty, body) in displays(&text) {
            read += 1;
            if NAMES.contains(&(name.as_str(), ty.as_str())) {
                continue;
            }
            for w in words(&body) {
                faults.push(format!("{name}: Display for {ty} writes {w:?}"));
            }
        }
    }
    assert!(faults.is_empty(), "{}", faults.join("\n"));
    // TrainError, MeshError, MeasurementError, EditRefused, Refused, Never,
    // Note, Ratio and Class at least.
    assert!(read >= 9, "only {read} Display impls read");
}

/// The gate catches a sentence, and passes a placeholder and a name it
/// is told of — its own near misses.
#[test]
fn the_gate_reads_a_sentence_and_not_a_placeholder() {
    let planted = r#"impl std::fmt::Display for Bad {
        fn fmt(&self, f: &mut Formatter) -> Result { write!(f, "no root {x}") }
    }
    impl std::fmt::Display for Good {
        fn fmt(&self, f: &mut Formatter) -> Result { write!(f, "{}/{}", self.a, self.b) }
    }"#;
    let found: Vec<_> = displays(planted)
        .into_iter()
        .map(|(ty, body)| (ty, words(&body)))
        .collect();
    assert_eq!(
        found[0],
        ("Bad".to_string(), vec!["no root {x}".to_string()])
    );
    assert_eq!(found[1], ("Good".to_string(), Vec::<String>::new()));
}
