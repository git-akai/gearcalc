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
//! # The shape can change, and a file written before it does will be refused
//!
//! Loudly, by the parser, naming the field and the line — which is the only
//! behaviour worth having: a document read with a field quietly defaulted is a
//! document that describes a different gearbox. Two changes so far:
//!
//! - `working_depth` became `Auto` (`{ auto, manual }`) when its default moved
//!   from a fixed module to the gear's own dedendum. A file written before that
//!   has `working_depth = 1.0`; `working_depth = { auto = false, manual = 1.0 }`
//!   preserves what it meant, and `{ auto = true, manual = 1.0 }` takes the new
//!   behaviour.
//! - A worm stage gained `thickness_mod`, which a pre-existing file will not
//!   have. `thickness_mod = 1.0` is the standard tooth.
//! - A spur stage gained `load_sharing`, which a pre-existing file will not
//!   have. It is the one shape change so far that **defaults rather than
//!   refusing**, and deliberately: `load_sharing = "none"` is both the default
//!   and what every file written before it meant, so there is no reading of an
//!   older document that this gets wrong. A field whose absence is unambiguous
//!   is not a document describing a different gearbox.
//! - `friction` became `sliding_friction`, and every stage gained a
//!   `static_friction` beside it (`static_friction_sun_planet` and
//!   `static_friction_planet_ring` on a planetary set). Rename the field and add
//!   `static_friction = 0.16` — or the sliding value, to keep a pre-existing
//!   file's answers, since one coefficient throughout is what it used to mean.
//!
//! - **A worm stage became a pair.** `kind = "worm"` now carries exactly the
//!   fields a `kind = "spur"` stage does — `gears` with the full member inputs
//!   in place of `worm`/`wheel`, `gears[0].teeth` for `starts` and
//!   `gears[1].teeth` for `wheel_teeth` — plus `axial_clearance`, which every
//!   pair now has (`0.0` on a spur stage means what its absence meant). A
//!   spur stage's `additional_helix = 15.0` became one reading of the sizing,
//!   `sizing = { auto = false, manual = { additional_helix = 15.0 } }`, beside
//!   the worm's `helix_angle` and `pitch_diameter` readings. A worm file
//!   written before this had no shifts or addenda to carry; `profile_shift =
//!   { auto = false, manual = 0.0 }` on the worm and automatic on the wheel is
//!   the convention it meant.
//! - **The train's loads became a list.** `input_speed`, `input_torque`,
//!   `back_driving_torque`, `operating_torque` and `actuation` are gone, and
//!   `[[train.load_cases]]` holds any number of loads, each with a `kind`
//!   (`ultimate` or `fatigue`), `enabled`, a `port` (`start` or `end`), whether
//!   it is `reacted` at the far end, its `torque` and `speed` at that port, and
//!   a `duty` a fatigue case is counted over. What a pre-existing file meant is
//!   three cases: `kind = "ultimate"`, `port = "start"`, `reacted = true` at
//!   the input torque and speed; `kind = "ultimate"`, `port = "end"`,
//!   `reacted = false` at the back-driving torque; and `kind = "fatigue"`,
//!   `port = "start"`, `reacted = true` at the operating torque, with the
//!   actuation as its `duty` — an intermittent one gaining `at = "end"`, the
//!   port its range was always measured at, and a continuous one losing its
//!   `operating_speed` to the case's own `speed`.
//! - **The helix moved onto the members.** A pair's `sizing` — one of three
//!   readings of its size — is gone; every member of every stage carries
//!   `helix_angle = { auto, manual }`, a pair carries `pitch_diameter =
//!   { auto, manual }` for its first member, and a planetary set's and a hula
//!   stage's stage-level `helix_angle` went with it. At most one reading is
//!   given: `sizing = { auto = false, manual = { helix_angle = 45.0 } }` is
//!   `helix_angle = { auto = false, manual = 45.0 }` on the first member, an
//!   `additional_helix = a` is `Σ/2 + a` there, and a `pitch_diameter = d` is
//!   the stage's `pitch_diameter`; a set's `helix_angle = β` is
//!   `helix_angle = { auto = false, manual = β }` on its sun. Every reading
//!   automatic shares the shaft angle evenly, which at zero is a spur pair.
//!   Every stage with a line contact also gained `overlap = { auto, manual }`,
//!   the axial contact ratio; `{ auto = true, manual = 1.0 }` is what an
//!   older file meant.
//!
//! - **A planetary set's `arrangement` moved onto the train.** Which shaft is
//!   held and which driven is a fact about how the train is wired, so the
//!   stage no longer carries `arrangement = { input, fixed }` and a file that
//!   still does is refused by name. It is `[[train.constraints]]` now — one
//!   per shaft, `at = { kind = "of", stage = 0, shaft = 1 }` with
//!   `constraint = "driven"`, `"held"` or `"free"` — where a set's shafts are
//!   numbered sun 1, carrier 2, ring 3 in its wiring. A file with no
//!   constraints at all means what it always meant: each stage's conventions,
//!   with the first stage's input driven. So `{ input = "sun", fixed =
//!   "ring" }` is nothing to write, and `{ input = "sun", fixed = "carrier" }`
//!   is one line, shaft 2 held: a hold the train writes on a stage replaces
//!   the stage's conventional holds on that stage, so holding the carrier
//!   *instead of* the ring needs no word about the ring. (For a while it
//!   needed three lines — the ring written free as well — and a file that
//!   still says so means the same thing.) A drive replaces the conventional
//!   drive the same way. `[[train.couplings]]` arrived beside it, empty
//!   meaning the chain.
//! - **The stage kinds are one shape.** `kind = "spur"`, `"worm"` and
//!   `"planetary"` are gone and a file that still says one is refused by
//!   name; every such stage is `kind = "shape"` now, and says what it is
//!   made of: `[[train.stages.axes]]` (each `count = N`, and `carried_by =
//!   <shaft>` where a carrier carries it), `[[train.stages.shafts]]` (each
//!   `axis = i`, numbered from one as the constraints number them),
//!   `[[train.stages.members]]` (each with its `shaft`, `module`,
//!   `thickness_mod`, its `gear` table, a `ring` table naming the cutter
//!   where it is one, and a `pitch_diameter` reading), `[[train.stages.
//!   meshes]]` (`a`, `b`, and the two frictions) and `[[train.stages.
//!   distances]]` (`axes = [i, j]`, `angle`, `worm`, `distance`, `clearance`,
//!   the tolerances and `axial_clearance`). A spur pair is two ground axes
//!   with one mesh; a set is a central axis with a planet axis carried by
//!   its second shaft, `count` planets, two meshes and one distance; a worm
//!   is a pair whose distance is at `angle = 90` with `worm = true`. The
//!   panel writes these from its presets, and a person can write any shape
//!   the graph admits. (The hula stage became a preset of the same shape
//!   since.)
//! - **A member's `thickness_mod` is `{ auto, manual }`.** It was a plain
//!   number per member with nothing tying a mesh's two together; it is
//!   given on one member of each mesh and automatic on the other, which
//!   follows the mesh's rule — the two sum to 2 across an external mesh, a
//!   ring takes its pinion's — and relief keeps at most one of a mesh's two
//!   given. A file that writes a plain number is refused; one that writes
//!   both members of a mesh given has one relieved on load.
//! - **A distance carries `tip_clearance`**, the least far-side tip gap an
//!   internal mesh on it may run at, mm: an automatic distance is what the
//!   shifts leave or what the tips need, whichever is larger. Absent means
//!   zero — the tips must not cross and nothing more — so a file written
//!   before it reads as it did.
//! - **A load case's `port` may name a shaft.** `"start"` and `"end"` still
//!   mean the chain's two ends — the first stage's input and the last
//!   stage's output under the constraints in force — and a third spelling,
//!   `port = { at = { kind = "of", stage = 1, shaft = 2 } }`, names any
//!   shaft a stage lists as a port, in the same reference a constraint uses.
//!   A duty's `at` takes the same three. Nothing an older file wrote
//!   changed meaning.
//!
//! No compatibility shim, deliberately. Accepting both shapes means carrying two
//! readers for one format and testing both forever, and the thing that would go
//! wrong — a file loading with a field defaulted rather than read — is exactly
//! what a refusal prevents.
//!
//! **And the refusal is enforced, not assumed.** For a while this note promised
//! it and the parser delivered only half: a field that went *missing* was
//! refused, but serde reads past an *unknown* field by default, so a field that
//! had been **removed** from the shape would have been dropped in silence and
//! the file would have loaded describing a different gearbox — the very fault
//! above. Every struct the document is made of says `deny_unknown_fields`
//! now, and `a_field_the_shape_no_longer_has_is_refused_and_named` holds it
//! from both ends (`docs/corrections.md`).
//!
//! # What *is* adjusted on import
//!
//! A file can say what the panel cannot: a crossed pair with its axial
//! contact ratio given, a pair with its distance and both shifts and a helix
//! pinned at once. An input that stands given and is read by nothing is the
//! thing this tool refuses to have (`docs/rationale.md#what-a-stage-owes-relief`),
//! so every stage read is **relieved** exactly as a stage in the panel is after
//! any change — by the core, with nothing just touched — and the reader says
//! whether that moved anything ([`Imported::adjusted`]). A hand-edited value
//! is never changed; only which toggles stand, and only where the file asked
//! for a contradiction or for an input the stage has no use for. The
//! precedent: a file is *adjusted to what the tool can honour*, once, on the
//! way in, and the reader is told in one sentence rather than left to find
//! a box that does nothing.
//!
//! # What is *not* checked on import
//!
//! A stage names its materials by name, and the library that has them is the
//! user's own. Import therefore does not verify that they exist: the file is
//! valid, the library is simply a different document, and the train solve
//! already reports an unknown material by name where the user can act on it. The
//! alternative — refusing the import — would lose a whole train over one
//! material that a library import could supply a moment later.

use gear_core::train::Train;
use serde::{Deserialize, Serialize};

/// A geartrain as it is exchanged: a name, and the train's inputs.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrainDocument {
    /// The tab's name. Not unique, per the specification.
    pub name: String,
    pub train: Train,
}

/// What went wrong reading a geartrain document.
#[derive(Debug)]
pub enum TrainError {
    /// The document is not valid TOML, or does not match the geartrain schema.
    Parse(toml::de::Error),
    /// The document could not be written back out.
    Serialise(toml::ser::Error),
    /// The document parsed but describes no stages, so there is no train.
    NoStages,
}

impl std::fmt::Display for TrainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "geartrain file is not valid: {e}"),
            Self::Serialise(e) => write!(f, "geartrain could not be written: {e}"),
            Self::NoStages => write!(
                f,
                "geartrain file contains no stages, so there is nothing to solve"
            ),
        }
    }
}

impl std::error::Error for TrainError {}

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
pub struct Imported {
    pub document: TrainDocument,
    /// Whether any stage was relieved on the way in — a toggle the file had
    /// given that the stage cannot honour, turned back automatic (the module
    /// documentation, *What is adjusted on import*). The values are the
    /// file's own throughout.
    pub adjusted: bool,
}

/// Parse a geartrain from TOML, relieved of anything it asks for that no stage
/// can honour.
///
/// # Errors
///
/// [`TrainError::Parse`] if the document is not a geartrain, or
/// [`TrainError::NoStages`] if it describes a train with no stages — which
/// parses happily and then has nothing to solve.
pub fn from_toml(src: &str) -> Result<Imported, TrainError> {
    let mut document: TrainDocument = toml::from_str(src).map_err(TrainError::Parse)?;
    if document.train.stages.is_empty() {
        return Err(TrainError::NoStages);
    }
    let mut adjusted = false;
    for stage in &mut document.train.stages {
        let relieved = stage.relieved(None);
        if relieved.toggles() != stage.toggles() {
            adjusted = true;
            *stage = relieved;
        }
    }
    Ok(Imported { document, adjusted })
}

/// Write a geartrain as TOML, in the same shape the reader accepts.
///
/// # Errors
///
/// [`TrainError::Serialise`] if the document cannot be encoded, which would be a
/// defect rather than a runtime condition.
pub fn to_toml(doc: &TrainDocument) -> Result<String, TrainError> {
    let body = toml::to_string_pretty(doc).map_err(TrainError::Serialise)?;
    Ok(format!("{HEADER}\n{body}"))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::params::Auto;
    use gear_core::train::{
        Constraint, Coupling, Duty, HulaStage, Load, LoadCase, PairStage, PlanetaryStage, Port,
        ShaftConstraint, ShaftRef, Stage,
    };

    /// One of every preset, so the `kind` tag and every preset's layout are
    /// exercised in both directions and none can quietly stop round-tripping.
    fn document() -> TrainDocument {
        TrainDocument {
            name: "Test train".into(),
            train: Train {
                // One of every preset at every port, and both duties, so the
                // tags and the nested tables are exercised in both directions.
                load_cases: vec![
                    LoadCase::ultimate(0.25, 12_000.0),
                    LoadCase {
                        enabled: false,
                        ..LoadCase::back_driving(0.1)
                    },
                    LoadCase {
                        duty: Duty::Continuous {
                            runtime_hours: 1000.0,
                        },
                        ..LoadCase::fatigue(0.2, 9600.0)
                    },
                    LoadCase {
                        loads: vec![Load::given(Port::End, 0.05, 100.0)],
                        ..LoadCase::fatigue(0.05, 100.0)
                    },
                    // ...and one written at a shaft by reference — the
                    // hula stage's output gear, which is also `end` — so the
                    // third spelling of a port round-trips.
                    LoadCase {
                        loads: vec![
                            Load::given(Port::At(ShaftRef::Of { stage: 4, shaft: 4 }), 0.05, 100.0),
                            // ...and a derived load beside it, so an
                            // `{ auto, manual }` on a load round-trips too.
                            Load::derived(Port::Start),
                        ],
                        duty: Duty::Intermittent {
                            range_degrees: 90.0,
                            at: Port::At(ShaftRef::Of { stage: 0, shaft: 1 }),
                            actuations: 50,
                            reversing: true,
                        },
                        ..LoadCase::fatigue(0.05, 100.0)
                    },
                ],
                reversed_bending: false,
                stages: vec![
                    Stage::spur(
                        PairStage {
                            ..PairStage::default()
                        }
                        .with_additional_helix(15.0),
                    ),
                    Stage::worm(PairStage::worm()),
                    Stage::worm(
                        PairStage {
                            ..PairStage::worm()
                        }
                        .with_first_helix(45.0),
                    ),
                    Stage::planetary(PlanetaryStage::default()),
                    Stage::hula(HulaStage::default()),
                ],
                // One coupling and one of each constraint, so the tagged
                // `ShaftRef` and the `Constraint` values are exercised both
                // ways — a set at stage 3 with its carrier held instead of its
                // ring, and its ring said free.
                couplings: vec![Coupling {
                    a: ShaftRef::Of { stage: 0, shaft: 2 },
                    b: ShaftRef::Of { stage: 1, shaft: 1 },
                }],
                constraints: vec![
                    ShaftConstraint::held(3, 2),
                    ShaftConstraint {
                        at: ShaftRef::Of { stage: 3, shaft: 3 },
                        constraint: Constraint::Free,
                    },
                    ShaftConstraint::driven(0, 1),
                ],
            },
        }
    }

    /// **What we write, we read back — every field of it.**
    ///
    /// The property the whole feature rests on, and the one a format can fail
    /// silently: a dropped field reappears as a default, which looks like a
    /// value rather than like a loss. Compared through the serialised form
    /// rather than field by field, so a field added later is covered without
    /// anyone remembering to add it here.
    #[test]
    fn a_train_of_every_stage_kind_round_trips_unchanged() {
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
        // One tag a stage, one a load case — and one on each end of a
        // coupling, on each constraint's shaft, and on each port a load case
        // names by reference, since a `ShaftRef` says which kind of place it
        // names.
        assert_eq!(
            text.matches("kind = ").count(),
            5 + 5 + 2 + 3 + 2,
            "one tag a stage, a load case, a coupling end, a constraint and a named port:\n{text}"
        );
        for kind in ["shape", "ultimate", "fatigue"] {
            assert!(text.contains(&format!("kind = \"{kind}\"")), "no {kind}");
        }
        // ...and the shapes say what they are made of, by name.
        for table in ["axes", "shafts", "members", "meshes", "distances"] {
            assert!(
                text.contains(&format!("[[train.stages.{table}]]")),
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
            .replace("speed = 12000.0", "speed = 3000.0 # slowed down by hand");
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

    /// A train with no stages parses as TOML and is not a train. Refused here
    /// rather than downstream, where it would arrive as an empty result.
    #[test]
    fn a_train_without_stages_is_refused() {
        let mut doc = document();
        doc.train.stages.clear();
        let text = toml::to_string_pretty(&doc).unwrap();
        assert!(matches!(from_toml(&text), Err(TrainError::NoStages)));
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
        // ...on a stage.
        let stale = text.replacen(
            "kind = \"shape\"",
            "kind = \"shape\"\nsomething_old = 1.0",
            1,
        );
        match from_toml(&stale) {
            Err(TrainError::Parse(e)) => {
                assert!(e.to_string().contains("something_old"), "{e}")
            }
            other => panic!("a stale field must be a parse error, not {other:?}"),
        }
        // ...and on the train itself.
        let stale = text.replacen(
            "reversed_bending",
            "an_old_flag = true\nreversed_bending",
            1,
        );
        match from_toml(&stale) {
            Err(TrainError::Parse(e)) => assert!(e.to_string().contains("an_old_flag"), "{e}"),
            other => panic!("a stale field must be a parse error, not {other:?}"),
        }
    }

    /// Nonsense is refused with the parser's own message rather than a panic or
    /// a default-filled train.
    #[test]
    fn a_document_that_is_not_a_geartrain_is_refused() {
        for src in [
            "this is not toml",
            "name = \"no train in here\"",
            "[train]\nload_cases = \"heavy\"",
        ] {
            assert!(matches!(from_toml(src), Err(TrainError::Parse(_))), "{src}");
        }
    }

    /// An automatic input stays automatic across the file. It is two fields
    /// rather than one — the toggle and the value it falls back to — and losing
    /// the toggle would turn a solved dimension into whatever was last seeded.
    #[test]
    fn an_automatic_input_survives_as_a_toggle_and_a_value() {
        let mut doc = document();
        if let Some(s) = doc.train.stages[0].as_shape_mut() {
            s.distances[0].distance = Auto::fixed(31.5);
            s.members[0].gear.face_width = Auto::automatic(4.0);
        }
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap().document;
        let s = back.train.stages[0]
            .as_shape()
            .expect("stage 1 came back a different kind");
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
    fn a_file_asking_for_what_no_stage_honours_is_adjusted_and_says_so() {
        let mut doc = document();
        doc.train.stages[1]
            .as_shape_mut()
            .expect("stage 2 is the worm")
            .overlap = Auto::fixed(1.5);
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap();
        assert!(back.adjusted);
        let w = back.document.train.stages[1]
            .as_shape()
            .expect("stage 2 came back a different kind");
        assert!(w.overlap.auto, "a crossed pair's ratio cannot stand given");
        assert!((w.overlap.manual - 1.5).abs() < 1e-12, "the number is kept");
        // ...and a second read of the adjusted document adjusts nothing.
        assert!(
            !from_toml(&to_toml(&back.document).unwrap())
                .unwrap()
                .adjusted
        );
    }
}
