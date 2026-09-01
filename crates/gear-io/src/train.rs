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
//! No compatibility shim, deliberately. Accepting both shapes means carrying two
//! readers for one format and testing both forever, and the thing that would go
//! wrong — a file loading with a field defaulted rather than read — is exactly
//! what a refusal prevents.
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

/// Parse a geartrain from TOML.
///
/// # Errors
///
/// [`TrainError::Parse`] if the document is not a geartrain, or
/// [`TrainError::NoStages`] if it describes a train with no stages — which
/// parses happily and then has nothing to solve.
pub fn from_toml(src: &str) -> Result<TrainDocument, TrainError> {
    let doc: TrainDocument = toml::from_str(src).map_err(TrainError::Parse)?;
    if doc.train.stages.is_empty() {
        return Err(TrainError::NoStages);
    }
    Ok(doc)
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
        Actuation, FirstMemberSizing, PlanetaryStage, SpurStage, Stage, WormStage,
    };

    /// One of every stage kind, so the `kind` tag is exercised in both
    /// directions and no variant can quietly stop round-tripping.
    fn document() -> TrainDocument {
        TrainDocument {
            name: "Test train".into(),
            train: Train {
                input_speed: 12_000.0,
                input_torque: 0.25,
                back_driving_torque: 0.1,
                operating_torque: 0.2,
                reversed_bending: false,
                actuation: Actuation::Continuous {
                    operating_speed: 9600.0,
                    runtime_hours: 1000.0,
                },
                stages: vec![
                    Stage::Spur(SpurStage {
                        additional_helix: 15.0,
                        ..SpurStage::default()
                    }),
                    Stage::Worm(WormStage::default()),
                    Stage::Worm(WormStage {
                        sizing: FirstMemberSizing::HelixAngle(45.0),
                        ..WormStage::default()
                    }),
                    Stage::Planetary(Box::<PlanetaryStage>::default()),
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
    /// `kind` on every stage rather than meaning carried by position.
    #[test]
    fn the_document_says_what_it_is() {
        let text = to_toml(&document()).unwrap();
        assert!(text.starts_with("# Geartrain."), "no header:\n{text}");
        assert!(text.contains("Inputs only"));
        assert!(text.contains("name = \"Test train\""));
        assert_eq!(
            text.matches("kind = ").count(),
            4,
            "one tag a stage:\n{text}"
        );
        for kind in ["spur", "worm", "planetary"] {
            assert!(text.contains(&format!("kind = \"{kind}\"")), "no {kind}");
        }
    }

    /// A hand-edited file is the point of the format, so an edit must reach the
    /// answer. Nothing here trusts the writer: the value is changed in the text.
    #[test]
    fn an_edit_to_the_text_survives_the_read() {
        let text = to_toml(&document()).unwrap().replace(
            "input_speed = 12000.0",
            "input_speed = 3000.0 # slowed down by hand",
        );
        let back = from_toml(&text).unwrap();
        assert!((back.train.input_speed - 3000.0).abs() < 1e-12);
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

    /// Nonsense is refused with the parser's own message rather than a panic or
    /// a default-filled train.
    #[test]
    fn a_document_that_is_not_a_geartrain_is_refused() {
        for src in [
            "this is not toml",
            "name = \"no train in here\"",
            "[train]\ninput_speed = \"fast\"",
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
        if let Stage::Spur(s) = &mut doc.train.stages[0] {
            s.centre_distance = Auto::fixed(31.5);
            s.gears[0].face_width = Auto::automatic(4.0);
        }
        let back = from_toml(&to_toml(&doc).unwrap()).unwrap();
        let Stage::Spur(s) = &back.train.stages[0] else {
            panic!("stage 1 came back a different kind");
        };
        assert!(!s.centre_distance.auto && (s.centre_distance.manual - 31.5).abs() < 1e-12);
        assert!(s.gears[0].face_width.auto && (s.gears[0].face_width.manual - 4.0).abs() < 1e-12);
    }
}
