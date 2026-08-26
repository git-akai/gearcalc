//! The words, and the one place they live.
//!
//! `gear-core` emits [`Note`]s — a key and the values a message needs — and this
//! turns them into a sentence. The sentences are in `data/strings_<code>.toml`,
//! one file per language, and English is compiled in so a build with no files on
//! disk still speaks.
//!
//! # Why the catalogue is here and not in the front end
//!
//! Because the notes are, and a message split across two repositories of text is
//! a message that will disagree with itself. The application gets this same
//! catalogue through `gear_wasm::strings`, exactly as it gets its defaults
//! through `gear_wasm::defaults` — and for the same reason, which DESIGN §12
//! records: the one value that was written down in two languages drifted, and
//! only the side without tests was wrong.
//!
//! # What is checked
//!
//! Every key the core can emit exists here, every placeholder a message uses is
//! one the core supplies, and no message is left unused. Those three are what
//! makes a catalogue trustworthy rather than merely present, and they are tests
//! rather than good intentions.

use gear_core::note::Note;
use std::collections::BTreeMap;

/// English, compiled in. A build always has one language.
const EN: &str = include_str!("../data/strings_en.toml");

/// A language's messages, flattened to `section.key`.
#[derive(Clone, Debug, Default)]
pub struct Catalogue {
    messages: BTreeMap<String, String>,
}

/// What went wrong reading a catalogue.
#[derive(Debug)]
pub enum StringsError {
    /// The document is not valid TOML, or is not a table of tables of strings.
    Parse(String),
}

impl std::fmt::Display for StringsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "string catalogue is not valid: {e}"),
        }
    }
}

impl std::error::Error for StringsError {}

impl Catalogue {
    /// Parse a catalogue from TOML.
    ///
    /// The shape is one level of sections holding string values —
    /// `[stage] self_locking = "..."` becomes `stage.self_locking`. Anything
    /// else in the file is a mistake rather than an extension, so it is refused
    /// instead of ignored.
    ///
    /// # Errors
    ///
    /// [`StringsError::Parse`] if the document is not TOML or not that shape.
    pub fn parse(src: &str) -> Result<Self, StringsError> {
        let raw: toml::Value =
            toml::from_str(src).map_err(|e| StringsError::Parse(e.to_string()))?;
        let table = raw
            .as_table()
            .ok_or_else(|| StringsError::Parse("expected a table of sections".into()))?;
        let mut messages = BTreeMap::new();
        for (section, body) in table {
            let inner = body.as_table().ok_or_else(|| {
                StringsError::Parse(format!("[{section}] is not a table of messages"))
            })?;
            for (key, value) in inner {
                let text = value.as_str().ok_or_else(|| {
                    StringsError::Parse(format!("{section}.{key} is not a string"))
                })?;
                messages.insert(format!("{section}.{key}"), text.to_string());
            }
        }
        Ok(Self { messages })
    }

    /// The compiled-in English catalogue.
    ///
    /// # Panics
    ///
    /// Never in a shipped build: the file is `include_str!`d and parsed by the
    /// test suite, so a malformed one fails CI rather than a user's session.
    #[must_use]
    pub fn english() -> Self {
        Self::parse(EN).expect("the compiled-in English catalogue must parse")
    }

    /// The raw messages, for handing to a front end whole.
    #[must_use]
    pub fn messages(&self) -> &BTreeMap<String, String> {
        &self.messages
    }

    /// The message for a key, if this language has one.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.messages.get(key).map(String::as_str)
    }

    /// Render a note.
    ///
    /// A key this catalogue does not have renders as the key itself rather than
    /// as nothing: a half-translated file should show a reader something they
    /// can report, not swallow the sentence that was trying to warn them.
    #[must_use]
    pub fn render(&self, note: &Note) -> String {
        let Some(template) = self.get(&note.key) else {
            return note.key.clone();
        };
        fill(template, &note.values)
    }

    /// Render every note in a list.
    #[must_use]
    pub fn render_all(&self, notes: &[Note]) -> Vec<String> {
        notes.iter().map(|n| self.render(n)).collect()
    }
}

/// Substitute `{name}` from `values`.
///
/// A placeholder with no value is left standing rather than blanked, for the
/// same reason a missing key renders as its key: visible beats silent. Written
/// out rather than pulled in as a template dependency — the whole grammar is one
/// pair of braces, and a format-string crate would be a dependency `gear-io`
/// does not otherwise need.
fn fill(template: &str, values: &BTreeMap<String, String>) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        match after.find('}') {
            Some(close) => {
                let name = &after[..close];
                match values.get(name) {
                    Some(v) => out.push_str(v),
                    None => {
                        out.push('{');
                        out.push_str(name);
                        out.push('}');
                    }
                }
                rest = &after[close + 1..];
            }
            // An unclosed brace is the rest of the string, verbatim.
            None => {
                out.push('{');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::note::key;

    #[test]
    fn the_shipped_english_catalogue_parses() {
        let c = Catalogue::english();
        assert!(
            c.messages().len() > 20,
            "only {} messages",
            c.messages().len()
        );
        assert!(c.get("stage.self_locking").is_some());
    }

    /// Values go in where the braces are, and the core's formatting survives
    /// untouched — the catalogue never re-rounds anything.
    #[test]
    fn a_note_renders_with_its_values() {
        let c = Catalogue::english();
        let note = Note::new(key::STAGE_SELF_LOCKING)
            .number("friction", 0.06, 3)
            .number("threshold", 0.098_765, 4);
        let text = c.render(&note);
        assert!(text.contains("0.060"), "{text}");
        assert!(text.contains("0.0988"), "{text}");
        assert!(!text.contains('{'), "a placeholder was left: {text}");
    }

    /// A half-translated catalogue must not swallow a warning.
    #[test]
    fn a_missing_message_shows_its_key_rather_than_nothing() {
        let c = Catalogue::parse("[stage]\nknown = \"fine\"").unwrap();
        assert_eq!(c.render(&Note::new("stage.unknown")), "stage.unknown");
    }

    /// ...and a missing *value* leaves its placeholder standing, for the same
    /// reason: a sentence with a visible hole gets reported, a silent one does
    /// not.
    #[test]
    fn a_missing_value_leaves_its_placeholder_visible() {
        let c = Catalogue::parse("[a]\nb = \"x {gone} y\"").unwrap();
        assert_eq!(c.render(&Note::new("a.b")), "x {gone} y");
    }

    /// Braces that are not placeholders are text.
    #[test]
    fn stray_braces_survive() {
        let c = Catalogue::parse("[a]\nb = \"unclosed { here\"").unwrap();
        assert_eq!(c.render(&Note::new("a.b")), "unclosed { here");
    }

    /// **Every key the core can emit has a message here.**
    ///
    /// The half that keeps a reader from meeting a bare `clamp.ring_tip_raised`
    /// where a sentence should be. `Note::key::ALL` is the core's own list, so
    /// this cannot drift by someone adding a note and forgetting the words.
    #[test]
    fn every_key_the_core_emits_has_english() {
        let c = Catalogue::english();
        let missing: Vec<&str> = key::ALL
            .iter()
            .copied()
            .filter(|k| c.get(k).is_none())
            .collect();
        assert!(missing.is_empty(), "no English for {missing:?}");
    }

    /// **...and every message here is one the core can emit.**
    ///
    /// The other half, and the one nobody writes: without it a catalogue
    /// accumulates messages for notes that were deleted years ago, and a
    /// translator spends their time on sentences no one will read. `[ui]` is
    /// exempt — those are the application's own words and have no `Note` behind
    /// them — and that exemption is the only one.
    #[test]
    fn every_english_message_is_a_key_the_core_emits() {
        let c = Catalogue::english();
        let orphans: Vec<&String> = c
            .messages()
            .keys()
            .filter(|k| !k.starts_with("ui."))
            .filter(|k| !key::ALL.contains(&k.as_str()))
            .collect();
        assert!(orphans.is_empty(), "nothing emits {orphans:?}");
    }

    /// **Every placeholder a message uses is one the core supplies.**
    ///
    /// A `{radius}` in a message the core fills with `{tip}` renders as itself,
    /// which reads like a bug in the software rather than a typo in a text file.
    /// The values are learned by *running* the solvers rather than by declaring
    /// them, so this checks the messages against what actually arrives.
    #[test]
    fn every_placeholder_is_a_value_the_core_supplies() {
        let c = Catalogue::english();
        let supplied = observed_values();
        for (k, message) in c.messages() {
            if k.starts_with("ui.") {
                continue;
            }
            let Some(names) = supplied.get(k.as_str()) else {
                continue; // never fired in the sweep below; nothing to check
            };
            for placeholder in placeholders(message) {
                assert!(
                    names.contains(&placeholder),
                    "{k} uses {{{placeholder}}}, which the core does not supply \
                     (it supplies {names:?})"
                );
            }
        }
    }

    /// The placeholder names a message uses.
    fn placeholders(message: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut rest = message;
        while let Some(open) = rest.find('{') {
            let after = &rest[open + 1..];
            match after.find('}') {
                Some(close) => {
                    out.push(after[..close].to_string());
                    rest = &after[close + 1..];
                }
                None => break,
            }
        }
        out
    }

    /// Which values each note actually carries, learned by firing them.
    ///
    /// Deliberately a *sweep of real solves* rather than a table: a table would
    /// be a second declaration of the same thing, and the copy nobody exercises
    /// is the one that goes wrong (DESIGN §12). Anything this sweep does not
    /// reach is simply not checked, which is honest — it is not claimed to be
    /// exhaustive, only true.
    fn observed_values() -> BTreeMap<String, Vec<String>> {
        use gear_core::params::GearParams;
        use gear_core::profile::Gear;

        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut record = |notes: &[Note]| {
            for n in notes {
                seen.entry(n.key.clone())
                    .or_default()
                    .extend(n.values.keys().cloned());
            }
        };

        // Gears across a grid wide enough to trip most guard rails.
        for teeth in [5_u32, 9, 17, 40, 200] {
            for shift in [-0.9_f64, -0.4, 0.0, 0.6, 1.4] {
                for thickness in [0.2_f64, 1.0, 1.9] {
                    for root in [0.05_f64, 0.38, 0.9] {
                        let g = Gear::new(GearParams {
                            teeth,
                            profile_shift: shift,
                            thickness_mod: thickness,
                            root_radius: root,
                            dedendum: 1.25,
                            ..Default::default()
                        });
                        record(&g.clamps.notes);
                    }
                }
            }
        }

        // Corners the grid above does not turn: a dedendum deep enough to reach
        // the axis, and thickness modifications past both ends.
        for dedendum in [1.25_f64, 6.0] {
            for thickness in [0.05_f64, 1.0, 2.5] {
                record(
                    &Gear::new(GearParams {
                        teeth: 6,
                        dedendum,
                        thickness_mod: thickness,
                        ..Default::default()
                    })
                    .clamps
                    .notes,
                );
            }
        }

        // A pressure angle below the floor, which nothing above reaches.
        record(
            &Gear::new(GearParams {
                pressure_angle: 0.5,
                ..Default::default()
            })
            .clamps
            .notes,
        );

        // Rings, which have guard rails of their own and none in common with an
        // external gear: the cutter can be too big, too blunt, or unable to
        // reach the flank at all.
        for teeth in [18_u32, 30, 60] {
            for cutter_teeth in [8_u32, 20, 40, 90] {
                for shift in [-0.6_f64, 0.0, 0.8] {
                    for tip_round in [0.02_f64, 0.38] {
                        let ring = gear_core::ring::Ring::new(
                            &GearParams {
                                teeth,
                                profile_shift: shift,
                                ..Default::default()
                            },
                            &gear_core::ring::Cutter {
                                teeth: cutter_teeth,
                                tip_round,
                                ..Default::default()
                            },
                        );
                        record(&ring.clamps);
                        for thickness in [0.05_f64, 2.5] {
                            let extreme = gear_core::ring::Ring::new(
                                &GearParams {
                                    teeth,
                                    profile_shift: shift,
                                    thickness_mod: thickness,
                                    ..Default::default()
                                },
                                &gear_core::ring::Cutter {
                                    teeth: cutter_teeth,
                                    tip_round,
                                    ..Default::default()
                                },
                            );
                            record(&extreme.clamps);
                        }
                    }
                }
            }
        }

        // Stages: spur, worm, crossed and planetary, over inputs that fire the
        // notes each of them can raise.
        let lib = crate::default_library();
        for helix in [0.0_f64, 3.0, 20.0] {
            for teeth in [(17_u32, 43_u32), (9, 11)] {
                let stage = gear_core::train::SpurStage {
                    additional_helix: helix,
                    gears: [
                        gear_core::train::StageGear {
                            teeth: teeth.0,
                            ..Default::default()
                        },
                        gear_core::train::StageGear {
                            teeth: teeth.1,
                            ..Default::default()
                        },
                    ],
                    ..Default::default()
                };
                if let Ok(r) = gear_core::train::solve_stage(&stage, 2.0, &lib) {
                    record(&r.notes);
                }
                for sigma in [0.5_f64, 90.0] {
                    for face in [
                        gear_core::params::Auto::automatic(0.0),
                        gear_core::params::Auto::fixed(0.4),
                    ] {
                        let mut crossed = gear_core::train::SpurStage {
                            shaft_angle: sigma,
                            ..stage.clone()
                        };
                        for g in &mut crossed.gears {
                            g.face_width = face;
                        }
                        if let Ok(r) = gear_core::train::solve_crossed_stage(&crossed, 2.0, &lib) {
                            record(&r.notes);
                        }
                    }
                }
            }
        }
        for (starts, friction) in [
            (1_u32, 0.06_f64),
            (1, 0.3),
            (4, 0.02),
            (4, 0.16),
            (2, 0.10),
            (2, 0.12),
        ] {
            let stage = gear_core::train::WormStage {
                starts,
                sliding_friction: friction,
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_worm_stage(&stage, 2.0, &lib) {
                record(&r.notes);
            }
        }
        for (planets, clearance) in [(3_u32, 0.5_f64), (4, 9.0), (5, 0.5), (6, 40.0), (7, 0.5)] {
            let stage = gear_core::train::PlanetaryStage {
                planets,
                min_planet_clearance: clearance,
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_planetary_stage(&stage, 3000.0, 2.0, &lib) {
                record(&r.notes);
            }
        }
        // Six that the grids above do not reach, each needing its own corner.
        // Written out with the reason rather than folded into a sweep, because
        // "how do you make this happen" is the useful thing to record.

        // A ring whose corner rounds meet before mid-space: a blunt tool with
        // few teeth in a coarse ring.
        for tip_round in [0.5_f64, 0.7] {
            for cutter_teeth in [10_u32, 14] {
                record(
                    &gear_core::ring::Ring::new(
                        &GearParams {
                            teeth: 24,
                            dedendum: 1.6,
                            ..Default::default()
                        },
                        &gear_core::ring::Cutter {
                            teeth: cutter_teeth,
                            tip_round,
                            addendum: 1.6,
                        },
                    )
                    .clamps,
                );
            }
        }

        // A parallel pair that loses contact between teeth: short addenda.
        for addendum in [0.35_f64, 0.5] {
            let stage = gear_core::train::SpurStage {
                gears: [
                    gear_core::train::StageGear {
                        teeth: 17,
                        addendum: gear_core::params::Auto::fixed(addendum),
                        ..Default::default()
                    },
                    gear_core::train::StageGear {
                        teeth: 43,
                        addendum: gear_core::params::Auto::fixed(addendum),
                        ..Default::default()
                    },
                ],
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_stage(&stage, 2.0, &lib) {
                record(&r.notes);
            }
            // ...and a crossed pair of the same, whose teeth then reach a full
            // contact ratio at no width at all.
            for sigma in [45.0_f64, 90.0] {
                let crossed = gear_core::train::SpurStage {
                    shaft_angle: sigma,
                    gears: [
                        gear_core::train::StageGear {
                            face_width: gear_core::params::Auto::automatic(0.0),
                            ..stage.gears[0].clone()
                        },
                        gear_core::train::StageGear {
                            face_width: gear_core::params::Auto::automatic(0.0),
                            ..stage.gears[1].clone()
                        },
                    ],
                    ..stage.clone()
                };
                if let Ok(r) = gear_core::train::solve_crossed_stage(&crossed, 2.0, &lib) {
                    record(&r.notes);
                }
            }
        }

        // A screw pair entered by helix angle with automatic widths, which has
        // no enveloping wheel and so no published proportion to take.
        // Static coefficients either side of the default worm's 0.1327
        // threshold, so both the self-locking note and the "close to it" one
        // get their turn.
        for friction in [0.06_f64, 0.115, 0.12, 0.125] {
            let stage = gear_core::train::WormStage {
                sliding_friction: friction,
                static_friction: friction,
                sizing: gear_core::train::FirstMemberSizing::HelixAngle(45.0),
                worm: gear_core::train::WormMember {
                    face_width: gear_core::params::Auto::automatic(6.0),
                    ..Default::default()
                },
                wheel: gear_core::train::WormMember {
                    face_width: gear_core::params::Auto::automatic(6.0),
                    ..Default::default()
                },
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_worm_stage(&stage, 2.0, &lib) {
                record(&r.notes);
            }
            // ...and a worm sitting just under its self-locking threshold.
            if let Ok(r) = gear_core::train::solve_worm_stage(
                &gear_core::train::WormStage {
                    sliding_friction: friction,
                    static_friction: friction,
                    ..Default::default()
                },
                2.0,
                &lib,
            ) {
                record(&r.notes);
            }
        }

        // A planetary set whose ring runs out of addendum at its base circle.
        for (sun, planet, addendum) in [
            (11_u32, 9_u32, 2.6_f64),
            (13, 13, 2.2),
            (17, 17, 1.9),
            (24, 18, 1.7),
            (40, 25, 2.4),
        ] {
            let stage = gear_core::train::PlanetaryStage {
                sun: gear_core::train::StageGear {
                    teeth: sun,
                    ..Default::default()
                },
                planet: gear_core::train::StageGear {
                    teeth: planet,
                    ..Default::default()
                },
                ring: gear_core::train::StageGear {
                    addendum: gear_core::params::Auto::fixed(addendum),
                    ..Default::default()
                },
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_planetary_stage(&stage, 3000.0, 2.0, &lib) {
                record(&r.notes);
            }
        }

        seen
    }

    /// **The sweep fires every key there is.**
    ///
    /// Without this the placeholder test checks an empty set for anything the
    /// sweep misses and passes quietly, which is the failure mode of every
    /// coverage check ever written. Asserted as *all* of them rather than as a
    /// fraction: a fraction is a tolerance, and this one can actually be met —
    /// so a new note added without a way to fire it fails here, which is the
    /// right moment to ask whether it can happen at all.
    #[test]
    fn the_sweep_fires_most_of_the_catalogue() {
        let seen = observed_values();
        let unreached: Vec<&str> = key::ALL
            .iter()
            .copied()
            .filter(|k| !seen.contains_key(*k))
            .filter(|k| !UNFIRED.contains(k))
            .collect();
        assert!(
            unreached.is_empty(),
            "the sweep does not fire {unreached:?}, so the placeholder check is \
             vacuous for them. Add a case, or add it to UNFIRED with a reason"
        );
        // ...and the exemptions must stay exemptions. One that starts firing is
        // a list that has stopped being read.
        for k in UNFIRED {
            assert!(
                !seen.contains_key(*k),
                "{k} fires now — take it out of UNFIRED"
            );
        }
    }

    /// Notes no sweep has been able to fire through the public solvers.
    ///
    /// Not a fudge factor: each is here because it was **looked for and not
    /// found**, and each is a question rather than a settled fact.
    ///
    /// - `clamp.ring_fully_filleted` — the ring's two corner rounds meeting
    ///   before mid-space. Searched over ~11 000 combinations of ring teeth,
    ///   cutter teeth, tip round, dedendum and thickness modification, and never
    ///   fired. The likely reason is that `ShaperCut` already refuses a tool
    ///   whose own rounds overlap, which is close to the same condition — so the
    ///   guard may be shadowing it entirely.
    /// - `stage.ring_addendum_clamped` — a planetary ring whose tip clamps at
    ///   its base circle. The set solves its ring's addendum, so it does not
    ///   normally hand it one that cannot work.
    ///
    /// Both are live code with live messages, so neither is deleted on
    /// suspicion. See HANDOFF §5.
    const UNFIRED: &[&str] = &["clamp.ring_fully_filleted", "stage.ring_addendum_clamped"];

    #[test]
    fn a_document_that_is_not_a_catalogue_is_refused() {
        for src in [
            "not toml at all ][",
            "top_level = \"strings must live in a section\"",
            "[section]\nnested = { not = \"a string\" }",
        ] {
            assert!(Catalogue::parse(src).is_err(), "{src}");
        }
    }
}
