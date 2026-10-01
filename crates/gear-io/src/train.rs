//! A geartrain as a human-readable TOML document.
//!
//! The specification asks that a geartrain can be exported to a text file and
//! imported back into a new tab, and that the file is the only place anything is
//! kept: nothing is saved between runs except by an explicit export.
//!
//! # Inputs only
//!
//! The document holds exactly what [`Train`] holds, which is exactly the inputs
//! — no ratio, no stresses, no efficiencies. That is docs/rationale.md#inputs-are-the-only-state showing up
//! in the file format: outputs are a pure function of the inputs, so writing
//! them down would create a second copy that can disagree with the first. A file
//! that says a stage's efficiency is 98.741 % is a file that will one day be
//! wrong about it. Everything derived is recomputed on import, which is also why
//! the file stays small enough to read and diff.
//!
//! # The name is part of the document
//!
//! A tab has a name and [`Train`] does not, so the document carries one. It is
//! kept at the top of the file rather than recovered from the filename, which a
//! browser download and a subsequent rename would both destroy.
//!
//! # The format
//!
//! A document states its format at its root, `format = 1` ([`FORMAT`]),
//! then its `name`, then `[train]`: `load_cases`, `reversed_bending`, `held`
//! and `[train.shape]`, the train's one graph — `axes`, `bodies`,
//! `members`, `meshes`, `distances` and `couplings`, each table what the
//! core's type of that name holds. Every field is required, and the writer
//! writes every one. An optional input — a ring's cutter, a rim thickness,
//! a replaced material figure — is written where it is given and left out
//! where it is not, TOML having no null. A field the format does not have
//! is refused by name, and so is one it has that a file leaves out: a
//! document read with a field quietly filled in describes a gearbox nobody
//! wrote down.
//!
//! A file of another format, or one that states none, is refused by the
//! format it names and pointed at `gear-cli convert` ([`convert`]). That
//! reads a file written before the format was numbered — its train as
//! stages or as one graph — once, at what such a file meant, and writes
//! the current format. Each change the format has been through, and what a
//! file written before it meant, is a row of
//! `docs/reference.md#geartrain-file-formats`. One reader for the current
//! format and one frozen converter for the files before it, rather than a
//! reader for every format: two readers for one format is two to test for
//! ever.
//!
//! # What *is* adjusted on import
//!
//! A file can say what the panel cannot: a crossed pair with its axial
//! contact ratio given, a pair with its distance and both shifts and a helix
//! pinned at once. An input that stands given and is read by nothing is the
//! thing this tool refuses to have (`docs/rationale.md#what-a-stage-owes-relief`),
//! so every train read is **relieved** exactly as the panel's is after any
//! change — by the core, with nothing just touched — and the reader says
//! whether that moved anything ([`Imported::adjusted`]). A hand-edited value
//! is never changed; only which toggles stand, and only where the file asked
//! for a contradiction or for an input the graph has no use for. The
//! precedent: a file is *adjusted to what the tool can honour*, once, on the
//! way in, and the reader is told in one sentence rather than left to find
//! a box that does nothing.
//!
//! Two more adjustments are made on the way in, each said the same way: a
//! mesh written (ring, gear) is turned round to (gear, ring), the order its
//! kind is read in, which changes nothing a mesh owns
//! ([`gear_core::train::Shape::order_meshes`]); and in a file before the
//! format was numbered, an automatic width's box of nought reads as 10 mm
//! ([`convert`]).
//!
//! # What is *not* checked on import
//!
//! A member names its material by name, and the library that has them is the
//! user's own. Import therefore does not verify that they exist: the file is
//! valid, the library is simply a different document, and the train solve
//! already reports an unknown material by name where the user can act on it. The
//! alternative — refusing the import — would lose a whole train over one
//! material that a library import could supply a moment later.

use gear_core::note::{key, Note};
use gear_core::train::{Shape, Train};
use serde::{Deserialize, Serialize};

mod unversioned;

/// **The format this tool writes and reads**, stated at a document's root.
/// A change to what a file means takes the next number, and [`convert`] the
/// step from the one before.
pub const FORMAT: u32 = 1;

/// A geartrain as it is exchanged: a name, and the train's inputs. The
/// format it is written at is the file's, stated by [`to_toml`] and held by
/// [`from_toml`], and not a field of the document.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "io/")
)]
pub struct TrainDocument {
    /// The tab's name. Not unique, per the specification.
    pub name: String,
    pub train: Train,
}

/// A document as the file holds it: its format, then the document.
#[derive(Serialize)]
struct Written<'a> {
    format: u32,
    name: &'a str,
    train: &'a Train,
}

/// A document of the current format as the reader takes it: the format is
/// held before this is read, and is a field of nothing after.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Read {
    #[allow(dead_code)]
    format: serde::de::IgnoredAny,
    name: String,
    train: Train,
}

/// What went wrong reading or writing a geartrain document.
#[derive(Debug)]
pub enum DocumentError {
    /// The document is not valid TOML, or does not match the geartrain schema.
    Parse(toml::de::Error),
    /// The document could not be written back out.
    Serialise(toml::ser::Error),
    /// The document states a format other than [`FORMAT`] — the one it
    /// names — or none, having been written before the format was numbered.
    Format(Option<i64>),
    /// The document parses but describes no train: a number in it no field
    /// holds, or a graph that breaks an invariant input must keep
    /// ([`Train::validate`]). Carries the core's refusal, whose note names
    /// the field by its path in the file, or the piece.
    Malformed(gear_core::train::TrainError),
}

impl DocumentError {
    /// The catalogue's note, where the refusal has one: a file of another
    /// format, or a graph that describes no train. A parse error is the
    /// parser's own words, which name the line.
    #[must_use]
    pub fn note(&self) -> Option<Note> {
        use gear_core::note::Explain;
        let current = FORMAT.to_string();
        match self {
            Self::Format(None) => {
                Some(Note::new(key::ERROR_TRAIN_FILE_UNVERSIONED).text("current", current))
            }
            Self::Format(Some(found)) => Some(
                Note::new(key::ERROR_TRAIN_FILE_FORMAT)
                    .text("found", found.to_string())
                    .text("current", current),
            ),
            Self::Malformed(e) => Some(e.note()),
            Self::Parse(_) | Self::Serialise(_) => None,
        }
    }
}

impl std::fmt::Display for DocumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (self, self.note()) {
            (Self::Parse(e), _) => write!(f, "geartrain file is not valid: {e}"),
            (Self::Serialise(e), _) => write!(f, "geartrain could not be written: {e}"),
            (_, Some(note)) => {
                let words = crate::strings::Catalogue::english().render(&note);
                write!(f, "geartrain file is not valid: {words}")
            }
            (_, None) => write!(f, "geartrain file is not valid"),
        }
    }
}

impl std::error::Error for DocumentError {}

/// The header written above an exported geartrain.
///
/// Comments survive a round trip only in the sense that TOML ignores them; this
/// is re-emitted on every export rather than preserved from the file that was
/// read. It exists because "human read-able" was the requirement, and a reader
/// meeting one of these files cold should not have to guess whether the numbers
/// in it are inputs or results.
const HEADER: &str = "\
# Geartrain.
#
# Inputs only — every ratio, stress, efficiency and backlash is recomputed from
# these, so nothing here can go stale. Units: mm, degrees, rpm, N·m.
#
# Materials are named; the library that defines them is a separate document.
# Editing this file by hand is expected.
";

/// What reading a geartrain came to: the document, and whether reading it
/// changed it.
#[derive(Clone, Debug, Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "io/")
)]
pub struct Imported {
    pub document: TrainDocument,
    /// Whether the train was relieved on the way in — a toggle the file had
    /// given that the graph cannot honour, turned back automatic (the module
    /// documentation, *What is adjusted on import*). The values are the
    /// file's own throughout.
    pub adjusted: bool,
}

/// The format a document states at its root, `None` where it states none.
/// Every other field is left for the reader, which knows that format.
fn format_of(src: &str) -> Result<Option<i64>, DocumentError> {
    #[derive(Deserialize)]
    struct Header {
        format: Option<i64>,
    }
    toml::from_str::<Header>(src)
        .map(|h| h.format)
        .map_err(DocumentError::Parse)
}

/// Parse a geartrain from TOML, relieved of anything it asks for that the
/// graph cannot honour.
///
/// # Errors
///
/// [`DocumentError::Format`] if the document is not of the current format,
/// [`DocumentError::Parse`] if it is not a geartrain, and
/// [`DocumentError::Malformed`] if its graph describes none. An empty train
/// is a train — its cases wait for a preset — and reads as written.
pub fn from_toml(src: &str) -> Result<Imported, DocumentError> {
    match format_of(src)? {
        Some(found) if found == i64::from(FORMAT) => {}
        found => return Err(DocumentError::Format(found)),
    }
    let Read {
        name, mut train, ..
    } = toml::from_str(src).map_err(DocumentError::Parse)?;
    let turned = train.shape.order_meshes();
    validated(&train)?;
    Ok(relieved(TrainDocument { name, train }, turned))
}

/// **A train as a file holds it, validated** ([`Train::validate`]): a
/// refused value named by its path in the file, under `train`.
fn validated(train: &Train) -> Result<(), DocumentError> {
    use gear_core::train::TrainError;
    train.validate().map_err(|e| {
        DocumentError::Malformed(match e {
            TrainError::Input(r) => TrainError::Input(r.within("train")),
            other => other,
        })
    })
}

/// A document read, relieved of anything it asks for that nothing can
/// honour: the graph as the panel relieves it — every group of inputs that
/// argue is a part's, so relieving the one graph relieves each part — and
/// every load case the same. `turned` says a mesh was written (gear, ring)
/// on the way in ([`gear_core::train::shape::Shape::order_meshes`]), which is
/// an adjustment too.
fn relieved(mut document: TrainDocument, turned: bool) -> Imported {
    let mut adjusted = turned;
    let shape = &mut document.train.shape;
    let relieved = shape.relieved(None);
    if relieved.toggles() != shape.toggles() {
        adjusted = true;
        *shape = relieved;
    }
    // ...and every load case the same: a file that gives a speed at each
    // end of a pair has asked for a contradiction, and the last gives way.
    // A train whose graph cannot be asked is left as written; the solve
    // refuses it by name.
    for case in 0..document.train.load_cases.len() {
        if let Ok(true) = document.train.relieve_case_toggles(case, None) {
            adjusted = true;
        }
    }
    Imported { document, adjusted }
}

/// **A file written before the format was numbered, read as the current
/// format** — what `gear-cli convert` does, once, to a file [`from_toml`]
/// refuses as unnumbered. Its train is stages or one graph; stages become
/// the graph ([`gear_core::train::graph::graph_of`]), a body a join could
/// not make coaxial numbered after every body the file names. What the file
/// leaves out is read at what such a file meant ([`unversioned`]), and what
/// it held and loaded is kept as written; the document is then relieved as
/// any other is. A file of the current format reads as [`from_toml`] reads
/// it.
///
/// # Errors
///
/// [`DocumentError::Format`] if the document states a format other than the
/// current one, [`DocumentError::Parse`] if it is not a geartrain of its
/// time — or holds its train both as stages and as one graph, or neither —
/// and [`DocumentError::Malformed`] if the graph describes none.
pub fn convert(src: &str) -> Result<Imported, DocumentError> {
    match format_of(src)? {
        None => {}
        Some(found) if found == i64::from(FORMAT) => return from_toml(src),
        found => return Err(DocumentError::Format(found)),
    }
    let old: unversioned::Document = toml::from_str(src).map_err(DocumentError::Parse)?;
    let widened = old.reads_a_box_of_nought();
    let train = old.train;
    let load_cases: Vec<gear_core::train::LoadCase> =
        train.load_cases.into_iter().map(Into::into).collect();
    let shape = match (train.stages, train.shape) {
        (Some(stages), None) => {
            let stages: Vec<Shape> = stages.into_iter().map(Into::into).collect();
            // Every body the file names, so a body the graph adds is past them.
            let named = load_cases
                .iter()
                .flat_map(|c| {
                    c.loads.iter().map(|l| l.at).chain(match c.duty {
                        gear_core::train::Duty::Intermittent { at, .. } => at,
                        gear_core::train::Duty::Continuous { .. } => None,
                    })
                })
                .chain(train.held.iter().copied())
                .chain(stages.iter().map(Shape::max_body))
                .max()
                .unwrap_or(0); // absence: a file naming no body names ground
            gear_core::train::graph::graph_of(&stages, named + 1).shape
        }
        (None, Some(shape)) => shape.into(),
        _ => {
            return Err(DocumentError::Parse(serde::de::Error::custom(
                "a train holds `stages` or `shape`, one of the two",
            )))
        }
    };
    let mut train = Train {
        load_cases,
        reversed_bending: train.reversed_bending,
        shape,
        held: train.held,
    };
    let turned = train.shape.order_meshes();
    validated(&train)?;
    Ok(relieved(
        TrainDocument {
            name: old.name,
            train,
        },
        turned || widened,
    ))
}

/// Write a geartrain as TOML, at the current format, in the shape the
/// reader accepts.
///
/// # Errors
///
/// [`DocumentError::Serialise`] if the document cannot be encoded, which would
/// be a defect rather than a runtime condition.
pub fn to_toml(doc: &TrainDocument) -> Result<String, DocumentError> {
    let written = Written {
        format: FORMAT,
        name: &doc.name,
        train: &doc.train,
    };
    let body = toml::to_string_pretty(&written).map_err(DocumentError::Serialise)?;
    Ok(format!("{HEADER}\n{body}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::params::Auto;
    use gear_core::train::arrangements as arr;
    use gear_core::train::{Duty, Load, LoadCase, LoadRole, Shape};

    /// One of every preset, so the `kind` tag and every preset's layout are
    /// exercised in both directions and none can quietly stop round-tripping.
    fn document() -> TrainDocument {
        // Chained, so a body two stages share is written once under one
        // number; a set at stage 3 with its carrier held instead of its
        // ring, and its ring said free, so both constraint values are
        // exercised both ways.
        let mut train = Train::chained(
            vec![
                arr::pair([17, 43]).with_additional_helix(15.0),
                arr::worm(1, 40),
                arr::worm(1, 40).with_first_helix(45.0),
                arr::planetary(12, 30, 72, 3),
                gear_core::train::arrangements::hula([65, 61, 57, 61], [1.0, 1.0]),
            ],
            |t| {
                let (start, end) = (t.port(0, 1), t.port(4, 4));
                // One of every case at every port, and both duties, so the
                // tags and the nested tables are exercised in both directions.
                vec![
                    LoadCase::ultimate(start, end, 0.25, 12_000.0),
                    LoadCase {
                        enabled: false,
                        ..LoadCase::back_driving(start, end, 0.1)
                    },
                    LoadCase {
                        duty: Duty::Continuous {
                            runtime_hours: 1000.0,
                        },
                        ..LoadCase::fatigue(start, end, 0.2, 9600.0)
                    },
                    // ...and every role an entry can carry — a load with a
                    // derived figure, a reaction and a free port — so each
                    // spelling round-trips.
                    LoadCase {
                        loads: vec![
                            Load::given(end, 0.05, 100.0),
                            Load::derived(start),
                            Load::declared(t.port(3, 3), LoadRole::Free),
                        ],
                        duty: Duty::Intermittent {
                            range_degrees: 90.0,
                            at: Some(start),
                            actuations: 50,
                            reversing: true,
                        },
                        ..LoadCase::fatigue(start, end, 0.05, 100.0)
                    },
                ]
            },
        );
        train.held = vec![train.port(3, 2)];
        TrainDocument {
            name: "Test train".into(),
            train,
        }
    }

    /// **A file that lists a ring first reads as the one that lists it
    /// second** (T05.8): on every preset with an internal mesh, each such
    /// mesh written (ring, gear) reads back to the shape written (gear,
    /// ring), said as adjusted, and the two solve to the same motion and the
    /// same paths. At the base a turned mesh was refused (`RingFirst`), and
    /// solved as written it gave a set −5 where it is 7 (audit T05.8). Two
    /// rings in mesh, and a ring across crossed axes, are refused by their
    /// own keys.
    #[test]
    fn a_ring_listed_first_reads_as_listed_second() {
        use gear_core::note::Explain;
        use gear_core::train::{solve_train, Preset};
        let lib = crate::default_library();
        let read = |train: &Train| {
            from_toml(
                &to_toml(&TrainDocument {
                    name: "turned".into(),
                    train: train.clone(),
                })
                .unwrap(),
            )
        };
        let mut checked = 0;
        for preset in Preset::ALL {
            let train = Train::alone(&preset.build(), 2.0, 3000.0);
            let rings: Vec<usize> = (0..train.shape.meshes.len())
                .filter(|&k| train.shape.members[train.shape.meshes[k].b].ring.is_some())
                .collect();
            if rings.is_empty() {
                continue;
            }
            checked += 1;
            let mut turned = train.clone();
            for &k in &rings {
                let m = &mut turned.shape.meshes[k];
                std::mem::swap(&mut m.a, &mut m.b);
            }
            let straight = read(&train).unwrap();
            let back = read(&turned).unwrap_or_else(|e| panic!("{preset:?}: {e:?}"));
            assert!(back.adjusted, "{preset:?}: a turned mesh is an adjustment");
            assert_eq!(
                format!("{:?}", back.document.train.shape),
                format!("{:?}", straight.document.train.shape)
            );
            let [a, b] = [&straight, &back].map(|d| solve_train(&d.document.train, &lib));
            assert_eq!(
                format!("{:?}", a.map(|r| (r.cases, r.paths))),
                format!("{:?}", b.map(|r| (r.cases, r.paths))),
                "{preset:?}"
            );
        }
        assert_eq!(checked, 5, "every epicyclic preset has a ring");

        let mut two = Train::alone(&arr::planetary(12, 30, 72, 3), 2.0, 3000.0);
        let ring = two.shape.meshes[1].b;
        let planet = two.shape.meshes[1].a;
        two.shape.members[planet].ring = two.shape.members[ring].ring;
        let refused = |r: Result<Imported, DocumentError>| match r {
            Err(DocumentError::Malformed(e)) => e.note().key,
            other => panic!("{other:?}"),
        };
        assert_eq!(
            refused(read(&two)),
            gear_core::note::key::ERROR_TRAIN_MALFORMED_TWO_RINGS
        );
        let mut crossed = Train::alone(&arr::pair([17, 43]), 2.0, 3000.0);
        crossed.shape.members[1].ring = Some(gear_core::ring::Cutter::default());
        crossed.shape.distances[0].angle = 90.0;
        assert_eq!(
            refused(read(&crossed)),
            gear_core::note::key::ERROR_TRAIN_MALFORMED_RING_CROSSED
        );
    }

    /// **What we write, we read back — every field of it.**
    ///
    /// The property the whole feature rests on, and the one a format can fail
    /// silently: a dropped field reappears as a default, which looks like a
    /// value rather than like a loss. Compared through the serialised form
    /// rather than field by field, so a field added later is covered without
    /// anyone remembering to add it here.
    #[test]
    fn a_train_of_every_preset_round_trips_unchanged() {
        let doc = document();
        let text = to_toml(&doc).unwrap();
        let back = from_toml(&text).unwrap();
        assert!(
            !back.adjusted,
            "a document the tool wrote needs no adjusting"
        );
        let back = back.document;
        assert_eq!(
            toml::to_string_pretty(&doc).unwrap(),
            toml::to_string_pretty(&back).unwrap()
        );
        // ...and again, so an export of an import is stable rather than merely
        // equal once.
        assert_eq!(text, to_toml(&back).unwrap());
    }

    /// The file is meant to be read and edited by a person, so the things a
    /// person needs are checked: the header, the name at the top, and a
    /// `kind` on every stage and every load case rather than meaning carried
    /// by position.
    #[test]
    fn the_document_says_what_it_is() {
        let text = to_toml(&document()).unwrap();
        assert!(text.starts_with("# Geartrain."), "no header:\n{text}");
        assert!(text.contains("Inputs only"));
        assert!(text.contains("name = \"Test train\""));
        // **One tag a load case, and nothing else has one**: a stage is a
        // shape and says what it is made of rather than naming a type, and
        // a body is a number of the train's.
        assert_eq!(
            text.matches("kind = ").count(),
            4,
            "one tag a load case, and one nowhere else:\n{text}"
        );
        for kind in ["ultimate", "fatigue"] {
            assert!(text.contains(&format!("kind = \"{kind}\"")), "no {kind}");
        }
        // ...and the graph says what it is made of, by name.
        for table in ["axes", "bodies", "members", "meshes", "distances"] {
            assert!(
                text.contains(&format!("[[train.shape.{table}]]")),
                "no {table}"
            );
        }
    }

    /// A hand-edited file is the point of the format, so an edit must reach the
    /// answer. Nothing here trusts the writer: the value is changed in the text.
    #[test]
    fn an_edit_to_the_text_survives_the_read() {
        let text = to_toml(&document())
            .unwrap()
            .replace("manual = 12000.0", "manual = 3000.0 # slowed down by hand");
        let back = from_toml(&text).unwrap().document;
        assert!((back.train.load_cases[0].speed() - 3000.0).abs() < 1e-12);
    }

    /// A train with no load cases is a shaft line and round-trips as one: the
    /// empty list is written and read back, rather than dropped and defaulted.
    #[test]
    fn a_train_without_load_cases_round_trips() {
        let mut doc = document();
        doc.train.load_cases.clear();
        let text = to_toml(&doc).unwrap();
        assert!(text.contains("load_cases = []"), "{text}");
        let back = from_toml(&text).unwrap().document;
        assert!(back.train.load_cases.is_empty());
    }

    /// **A sweep written at a body the train does not have is refused by
    /// name, never read as one** (audit T13.5): a fatigue case's sweep at
    /// body 99 is refused where the file is read, naming its field, or
    /// where it is solved, as a sweep at no open port — and nothing panics,
    /// where the count read the speed of body 99. An unset sweep, left out
    /// of the file, reads back unset.
    #[test]
    fn a_sweep_at_no_body_is_refused_by_name() {
        use gear_core::train::TrainError;
        let mut doc = document();
        let fatigue = 3;
        doc.train.load_cases[fatigue].duty = Duty::intermittent(Some(99));
        let text = to_toml(&doc).unwrap();
        match from_toml(&text) {
            Err(e) => assert!(e.to_string().contains("at"), "{e}"),
            Ok(read) => {
                let lib = crate::default_library();
                let solved = std::panic::catch_unwind(|| {
                    gear_core::train::solve_train(&read.document.train, &lib)
                });
                assert_eq!(
                    solved.expect("no panic").err(),
                    Some(TrainError::DutyPort {
                        case: fatigue,
                        body: 99
                    })
                );
            }
        }
        doc.train.load_cases[fatigue].duty = Duty::intermittent(None);
        let text = to_toml(&doc).unwrap();
        let back = from_toml(&text).unwrap().document;
        assert_eq!(
            back.train.load_cases[fatigue].duty,
            Duty::intermittent(None)
        );
    }

    /// A train with no stages is a train, and round-trips as one: a designer
    /// who removed the last stage and saved keeps the cases.
    #[test]
    fn an_empty_train_reads_as_written() {
        let mut doc = document();
        doc.train.shape = Shape::default();
        doc.train.held.clear();
        let text = to_toml(&doc).unwrap();
        let back = from_toml(&text).unwrap().document;
        assert!(back.train.parts().is_empty());
        assert_eq!(back.train.load_cases.len(), doc.train.load_cases.len());
    }

    /// **An automatic width's box of nought, in a file of stages, reads as
    /// the width the format's gears were born with (10 mm)**, said as an
    /// adjustment: the panel of the time left an automatic box at nought,
    /// and nothing read it while a load sized the gear. Only an automatic
    /// box is read so: a *given* width of nought is a gear with no face,
    /// refused by its field as in any file.
    #[test]
    fn a_staged_files_automatic_box_of_nought_reads_as_the_default() {
        let old = include_str!("../tests/data/elevation_drive_staged.toml");
        let given = "face_width]\nauto = false\nmanual = 10.0";
        let zero = "face_width]\nauto = true\nmanual = 0.0";
        assert_eq!(old.matches(zero).count(), 1, "one box of nought");
        assert!(old.contains(given));
        let converted = convert(old).unwrap();
        // The member whose box was nought: the one that moves when that box
        // is written otherwise, all else alike.
        let seven =
            convert(&old.replacen(zero, "face_width]\nauto = true\nmanual = 7.0", 1)).unwrap();
        assert!(
            !seven.adjusted,
            "with no box of nought, nothing is adjusted"
        );
        let widths = |d: &Imported| -> Vec<Auto<f64>> {
            let m = &d.document.train.shape.members;
            m.iter().map(|m| m.gear.face_width).collect()
        };
        let moved: Vec<usize> = (0..widths(&converted).len())
            .filter(|&i| widths(&converted)[i] != widths(&seven)[i])
            .collect();
        assert_eq!(moved.len(), 1, "{moved:?}");
        assert_eq!(widths(&seven)[moved[0]], Auto::automatic(7.0));
        // ...reads exactly the frozen literal, whatever the core's default
        // is, and says it was adjusted.
        assert_eq!(
            widths(&converted)[moved[0]],
            Auto::automatic(unversioned::BOX_OF_NOUGHT_READ_AS)
        );
        assert!(converted.adjusted, "a box read as a width is an adjustment");
        assert!(
            widths(&converted).contains(&Auto::fixed(10.0)),
            "a given width is as written"
        );
        let nought = old.replacen(given, "face_width]\nauto = false\nmanual = 0.0", 1);
        match convert(&nought) {
            Err(DocumentError::Malformed(e)) => {
                use gear_core::note::Explain;
                let n = e.note();
                assert!(
                    n.values
                        .get("field")
                        .is_some_and(|f| f.ends_with("gear.face_width.manual")),
                    "{n:?}"
                );
            }
            other => panic!("a given width of nought: {:?}", other.map(|_| ())),
        }
    }

    /// **A file written as stages is refused as unnumbered, and converts to
    /// the train a chain builds now.** The file is one the tool wrote before
    /// the train was one graph — the harness's elevation drive, a pair, a
    /// worm and a set — and the reader points at the converter rather than
    /// loading a different gearbox. Converted, it is three parts, holds and
    /// loads what it did, reads back unchanged, and turns at the ratio the
    /// tool recorded of it then (`tools/golden/trainfile.txt` at the time).
    /// Adjusted only where it left an automatic width's box at nought
    /// (`a_staged_files_automatic_box_of_nought_reads_as_the_default`).
    #[test]
    fn a_file_written_as_stages_is_refused_by_name_and_converts() {
        let old = include_str!("../tests/data/elevation_drive_staged.toml");
        match from_toml(old) {
            Err(e @ DocumentError::Format(None)) => {
                assert!(e.to_string().contains("gear-cli convert"), "{e}");
            }
            other => panic!("a file of stages must be refused, not {other:?}"),
        }
        let converted = convert(old).unwrap();
        // Relieved of nothing: its one adjustment is the boxes of nought.
        assert!(converted.adjusted, "its automatic boxes of nought are read");
        let train = &converted.document.train;
        assert_eq!(
            train
                .parts()
                .iter()
                .map(|p| p.members.len())
                .collect::<Vec<_>>(),
            vec![2, 2, 3]
        );
        assert_eq!(train.held, vec![5], "the set's ring, as the file held it");
        // What the file never wrote is what a file of its time meant: each
        // case at `K_A` = 1, each tip held off its mates' flanks.
        assert_eq!(
            train
                .load_cases
                .iter()
                .map(|c| c.application_factor)
                .collect::<Vec<_>>(),
            vec![1.0; 4]
        );
        assert!(train
            .shape
            .members
            .iter()
            .all(|m| m.gear.no_tip_past_mate_flank));
        let text = to_toml(&converted.document).unwrap();
        assert_eq!(text, to_toml(&from_toml(&text).unwrap().document).unwrap());
        let r = gear_core::train::solve_train(train, &crate::default_library()).unwrap();
        let ratio = r.total().unwrap().ratio;
        assert!((ratio - 708.235_294_118).abs() < 1e-8, "{ratio}");
    }

    /// **A carrier cycle is refused where it enters, naming the field.**
    /// A pair whose first axis is carried by the body on it, and one whose
    /// two axes each carry the other, read as files, both come back as the
    /// refusal [`Train::validate`] gives — the one [`Train::check`] reads —
    /// and a file of stages whose first axis carries itself converts to the
    /// same refusal; none walks the carriers round for ever.
    #[test]
    fn a_carrier_cycle_in_a_file_is_refused_and_named() {
        use gear_core::note::Explain;
        use gear_core::train::{Invariant, Preset};
        let spur = |carriers: [usize; 2]| {
            let mut t = Train::chained(vec![Preset::Spur.build()], |_| {
                vec![LoadCase::ultimate(1, 2, 2.0, 3000.0)]
            });
            t.shape.axes[0].carried_by = carriers[0];
            t.shape.axes[1].carried_by = carriers[1];
            to_toml(&TrainDocument {
                name: "cycle".into(),
                train: t,
            })
            .unwrap()
        };
        for (carriers, key, invariant) in [
            (
                [1, 0],
                "error.train_malformed_carried_by",
                Invariant::CarriedByNothing(0),
            ),
            (
                [2, 1],
                "error.train_malformed_carried_by_cycle",
                Invariant::CarriedInACycle(0),
            ),
        ] {
            match from_toml(&spur(carriers)) {
                Err(DocumentError::Malformed(e)) => {
                    assert_eq!(e.note().key, key, "{carriers:?}");
                    assert_eq!(
                        e,
                        gear_core::train::TrainError::Malformed(invariant),
                        "{carriers:?}"
                    );
                }
                other => panic!("{carriers:?}: a carrier cycle must be refused, not {other:?}"),
            }
        }
        let staged = include_str!("../tests/data/elevation_drive_staged.toml").replacen(
            "carried_by = 0",
            "carried_by = 1",
            1,
        );
        match convert(&staged) {
            Err(DocumentError::Malformed(e)) => {
                assert_eq!(e.note().key, "error.train_malformed_carried_by");
            }
            other => panic!("a self-carried stage must be refused, not {other:?}"),
        }
    }

    /// **A field the shape no longer has is refused, not dropped.**
    ///
    /// The module note promises that a file written before a shape change is
    /// refused loudly, and for a while that was true only of a field that went
    /// *missing*: serde ignores an unknown field by default, so a field that
    /// had been **removed** — an arrangement that moved to the train, say —
    /// would have been read past in silence and the file would have described
    /// a different gearbox. Every struct the document is made of now says
    /// `deny_unknown_fields`, and this is the case that holds it: a stage with
    /// a field nobody has, and a train with one, are both parse errors that
    /// name the field.
    #[test]
    fn a_field_the_shape_no_longer_has_is_refused_and_named() {
        let text = to_toml(&document()).unwrap();
        // ...on a stage — which is a shape, and a `kind` on one is itself a
        // field the shape no longer has, so the tag an older file wrote is
        // the case this starts from.
        let stale = text.replacen("[train.shape]", "[train.shape]\nsomething_old = 1.0", 1);
        match from_toml(&stale) {
            Err(DocumentError::Parse(e)) => {
                assert!(e.to_string().contains("something_old"), "{e}")
            }
            other => panic!("a stale field must be a parse error, not {other:?}"),
        }
        // ...the tag a stage used to carry included, and the three inputs
        // that were the stage's until they became its meshes' and axes'.
        for (field, line) in [
            ("kind", "kind = \"shape\""),
            ("load_sharing", "load_sharing = \"linear_ramp\""),
            ("min_planet_clearance", "min_planet_clearance = 0.3"),
            ("optimisation", "optimisation = { enabled = true }"),
        ] {
            let stale = text.replacen("[train.shape]", &format!("[train.shape]\n{line}"), 1);
            match from_toml(&stale) {
                Err(DocumentError::Parse(e)) => assert!(e.to_string().contains(field), "{e}"),
                other => panic!("a stage's old {field} must be a parse error, not {other:?}"),
            }
        }
        // ...a module written as the plain number it used to be, where it
        // is stated on one member of a group and followed by the rest now.
        let mut value: toml::Value = toml::from_str(&text).unwrap();
        value["train"]["shape"]["members"][0]["module"] = toml::Value::Float(1.0);
        match from_toml(&toml::to_string(&value).unwrap()) {
            Err(DocumentError::Parse(e)) => assert!(e.to_string().contains("module"), "{e}"),
            other => panic!("a plain module must be a parse error, not {other:?}"),
        }
        // ...and on the train itself.
        let stale = text.replacen(
            "reversed_bending",
            "an_old_flag = true\nreversed_bending",
            1,
        );
        match from_toml(&stale) {
            Err(DocumentError::Parse(e)) => assert!(e.to_string().contains("an_old_flag"), "{e}"),
            other => panic!("a stale field must be a parse error, not {other:?}"),
        }
    }

    /// Every key any table of `v` holds.
    fn keys(v: &toml::Value, out: &mut std::collections::BTreeSet<String>) {
        match v {
            toml::Value::Table(t) => {
                for (k, x) in t {
                    out.insert(k.clone());
                    keys(x, out);
                }
            }
            toml::Value::Array(a) => a.iter().for_each(|x| keys(x, out)),
            _ => {}
        }
    }

    /// `key` taken out of every table of `v` where `keep` says it may go.
    fn strip(v: &mut toml::Value, key: &str, goes: &dyn Fn(&toml::Value) -> bool) {
        match v {
            toml::Value::Table(t) => {
                if t.get(key).is_some_and(goes) {
                    t.remove(key);
                }
                t.iter_mut().for_each(|(_, x)| strip(x, key, goes));
            }
            toml::Value::Array(a) => a.iter_mut().for_each(|x| strip(x, key, goes)),
            _ => {}
        }
    }

    /// **Every field the writer writes, the reader requires.** A field a file
    /// leaves out is refused by name rather than filled in: a filled-in field
    /// is a document describing a gearbox nobody wrote down. Each key the
    /// written document holds is taken out of every table it is in, and the
    /// read must fail naming it. An `Option` is written by its absence —
    /// TOML has no null — so a ring's cutter is not asked, and nor is a
    /// duty's variant, whose absence the parser names as the duty; a
    /// sweep's `at` is asked, since a load's `at` shares its name.
    #[test]
    fn a_document_missing_any_field_is_refused_and_named() {
        let text = to_toml(&document()).unwrap();
        let value: toml::Value = toml::from_str(&text).unwrap();
        let mut all = std::collections::BTreeSet::new();
        keys(&value, &mut all);
        let by_absence = ["ring", "intermittent", "continuous"];
        let mut read_past = Vec::new();
        let mut asked = 0;
        for key in all.iter().filter(|k| !by_absence.contains(&k.as_str())) {
            let mut v = value.clone();
            strip(&mut v, key, &|_| true);
            asked += 1;
            match from_toml(&toml::to_string(&v).unwrap()) {
                Err(e) if e.to_string().contains(key.as_str()) => {}
                Err(e) => read_past.push(format!("{key}: refused, not naming it: {e}")),
                Ok(_) => read_past.push(format!("{key}: read")),
            }
        }
        assert!(
            read_past.is_empty(),
            "fields a file may leave out:\n{}",
            read_past.join("\n")
        );
        // Every key the written document holds, but the three above.
        assert_eq!(asked, 74, "fields asked");
    }

    /// **An addendum is a number.** It was `{ auto, manual }` once, and the
    /// reader that still took that table took any key beside `manual` too.
    #[test]
    fn an_addendum_written_as_a_table_is_refused() {
        let text = to_toml(&document()).unwrap();
        for table in [
            "{ auto = true, manual = 1.0 }",
            "{ manual = 1.0, anything = 3 }",
        ] {
            let stale = text.replacen("addendum = 1.0", &format!("addendum = {table}"), 1);
            assert_ne!(stale, text);
            match from_toml(&stale) {
                Err(e) => assert!(e.to_string().contains("addendum"), "{e}"),
                Ok(_) => panic!("an addendum of {table} must be refused"),
            }
        }
    }

    /// **A document says its format, and the reader holds a file to it.** A
    /// file that states none, or states another, is refused by the version
    /// it names and pointed at `gear-cli convert`, rather than read as if
    /// the fields it has meant what they mean now.
    #[test]
    fn a_document_of_another_format_is_refused_and_pointed_at_convert() {
        let text = to_toml(&document()).unwrap();
        let line = text
            .lines()
            .find(|l| l.starts_with("format = "))
            .expect("the writer writes its format");
        for (stale, names) in [
            (text.replacen(&format!("{line}\n"), "", 1), "format"),
            (text.replacen(line, "format = 999", 1), "999"),
        ] {
            match from_toml(&stale) {
                Err(e) => {
                    let e = e.to_string();
                    assert!(e.contains(names) && e.contains("gear-cli convert"), "{e}");
                }
                Ok(_) => panic!("a file without the current format must be refused"),
            }
        }
    }

    /// **A file written as stages converts by what a file of its time
    /// meant, not by today's defaults.** Each field such a file could leave
    /// out is taken out wherever it stands at the value every file that
    /// left it out meant; the file converts to the train the full one does.
    /// The values are the converter's own, written down once: a default the
    /// tool moves later does not move what an old file said.
    #[test]
    fn a_staged_file_leaving_out_what_it_could_converts_as_the_full_one() {
        let full = include_str!("../tests/data/elevation_drive_staged.toml");
        let mut sparse: toml::Value = toml::from_str(full).unwrap();
        let auto = |manual: f64| {
            let mut t = toml::Table::new();
            t.insert("auto".into(), true.into());
            t.insert("manual".into(), manual.into());
            toml::Value::Table(t)
        };
        let mut left_out = 0;
        for (key, meant) in [
            ("carried_by", toml::Value::Integer(0)),
            ("min_planet_clearance", 0.3.into()),
            ("pressure_angle", auto(20.0)),
            ("overlap", auto(1.0)),
            ("min_contact_ratio", 1.2.into()),
            ("load_sharing", "none".into()),
            ("search", false.into()),
            ("tip_clearance", 0.0.into()),
            ("couplings", toml::Value::Array(Vec::new())),
            ("no_undercut", true.into()),
            ("no_sharp_tip", true.into()),
            ("material_overrides", toml::Value::Table(toml::Table::new())),
            ("role", "load".into()),
            ("reversed_bending", false.into()),
        ] {
            let before = toml::to_string(&sparse).unwrap();
            strip(&mut sparse, key, &|v| *v == meant);
            left_out += usize::from(toml::to_string(&sparse).unwrap() != before);
        }
        assert_eq!(left_out, 14, "fields the file leaves out");
        let sparse = toml::to_string(&sparse).unwrap();
        let written = |src: &str| to_toml(&convert(src).unwrap().document).unwrap();
        let (sparse, full) = (written(&sparse), written(full));
        let moved: Vec<_> = sparse
            .lines()
            .zip(full.lines())
            .filter(|(a, b)| a != b)
            .collect();
        assert!(sparse == full, "converted apart: {moved:?}");
    }

    /// Nonsense is refused with the parser's own message rather than a panic or
    /// a default-filled train.
    #[test]
    fn a_document_that_is_not_a_geartrain_is_refused() {
        for src in [
            "this is not toml",
            "format = 1\nname = \"no train in here\"",
            "format = 1\n[train]\nload_cases = \"heavy\"",
        ] {
            assert!(
                matches!(from_toml(src), Err(DocumentError::Parse(_))),
                "{src}"
            );
        }
    }

    /// An automatic input stays automatic across the file. It is two fields
    /// rather than one — the toggle and the value it falls back to — and losing
    /// the toggle would turn a solved dimension into whatever was last seeded.
    #[test]
    fn an_automatic_input_survives_as_a_toggle_and_a_value() {
        let mut doc = document();
        {
            let part = doc.train.parts()[0].clone();
            let s = &mut doc.train.shape;
            s.distances[part.distances[0]].distance = Auto::fixed(31.5);
            s.members[part.members[0]].gear.face_width = Auto::automatic(4.0);
        }
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap().document;
        let s = &back.train.part_shapes()[0];
        let d = s.distances[0].distance;
        assert!(!d.auto && (d.manual - 31.5).abs() < 1e-12);
        let w = s.members[0].gear.face_width;
        assert!(w.auto && (w.manual - 4.0).abs() < 1e-12);
    }

    /// **A file asking for what no stage can honour is adjusted on the way
    /// in, and says so.** A crossed pair's axial contact ratio given — which
    /// the panel cannot produce and the solve reads on parallel shafts only —
    /// comes back automatic with its number kept, and `adjusted` is the one
    /// sentence the reader is owed. Every value the file gave is untouched,
    /// and a file that asks for nothing impossible is not adjusted.
    #[test]
    fn a_file_asking_for_what_nothing_honours_is_adjusted_and_says_so() {
        let mut doc = document();
        let mesh = doc.train.parts()[1].meshes[0];
        doc.train.shape.meshes[mesh].overlap = Auto::fixed(1.5);
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap();
        assert!(back.adjusted);
        let w = &back.document.train.part_shapes()[1];
        assert!(
            w.meshes[0].overlap.auto,
            "a crossed pair's ratio cannot stand given"
        );
        assert!(
            (w.meshes[0].overlap.manual - 1.5).abs() < 1e-12,
            "the number is kept"
        );
        // ...and a second read of the adjusted document adjusts nothing.
        assert!(
            !from_toml(&to_toml(&back.document).unwrap())
                .unwrap()
                .adjusted
        );
        // A load case the same: a speed given at each end of a pair — one
        // degree of freedom — is one too many, and the last gives way with
        // its number kept.
        let mut doc = document();
        doc.train = Train::chained(vec![arr::pair([17, 43]).with_additional_helix(15.0)], |t| {
            vec![LoadCase::ultimate(t.port(0, 1), t.port(0, 2), 1.0, 1000.0)]
        });
        // The reaction at the pair's second gear made a load with both
        // figures given.
        let (first, second) = (doc.train.port(0, 1), doc.train.port(0, 2));
        doc.train.load_cases[0].loads = vec![
            gear_core::train::Load::given(first, 0.25, 12_000.0),
            gear_core::train::Load::given(second, 1.0, 100.0),
        ];
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap();
        assert!(back.adjusted);
        let loads = &back.document.train.load_cases[0].loads;
        assert!(!loads[0].speed.auto && loads[1].speed.auto, "{loads:?}");
        assert!(
            (loads[1].speed.manual - 100.0).abs() < 1e-12,
            "the number is kept"
        );
        assert!(
            !from_toml(&to_toml(&back.document).unwrap())
                .unwrap()
                .adjusted
        );
    }
}
