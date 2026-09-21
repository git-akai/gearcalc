//! The words, and the one place they live.
//!
//! `gear-core` emits [`Note`]s — a key and the values a message needs — and this
//! turns them into a sentence. The sentences are in `data/strings_<code>.toml`,
//! one file per language, all of them compiled in: a language nobody can select
//! is not shipped, and a file loaded from disk at run time would be a fourth
//! place a message could go missing.
//!
//! # Why the catalogue is here and not in the front end
//!
//! Because the notes are, and a message split across two repositories of text is
//! a message that will disagree with itself. The application gets this same
//! catalogue through `gear_wasm::strings`, exactly as it gets its defaults
//! through `gear_wasm::defaults` — and for the same reason, which docs/corrections.md
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

/// A language the application can be read in.
///
/// The **native** name is what appears in the picker, because a reader looking
/// for their own language is looking for the word they call it by — a list that
/// says "German" is a list for people who already read English.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Language {
    /// BCP 47 tag: what a caller asks for, and what a preference is stored as.
    pub code: &'static str,
    /// The language's name in itself.
    pub name: &'static str,
    /// ...and in English, shown beside it.
    ///
    /// A reader looking for their own language wants the word they call it by,
    /// which is why [`Self::name`] leads. But a reader who has landed in a
    /// script they cannot read needs a way *back*, and four names none of which
    /// they recognise is not one — so English rides along as the common index.
    pub english_name: &'static str,
    source: &'static str,
}

impl Language {
    /// The language a BCP 47 tag should be read in.
    ///
    /// **Matching lives here rather than in the browser** for the reason the
    /// list does: a tag like `zh-TW` names a script this file knows about and a
    /// front end would have to be told about separately. Three steps, widest
    /// last:
    ///
    /// 1. the tag names a shipped language outright — `de`, `zh-Hant`;
    /// 2. it is Chinese, where the *script* is what decides and the region
    ///    usually implies it: Taiwan, Hong Kong and Macau read traditional,
    ///    everywhere else simplified;
    /// 3. its primary subtag matches one — `de-CH` and `de-AT` are both German
    ///    here, which is a claim about this catalogue's vocabulary rather than
    ///    about the languages.
    ///
    /// Anything else is English, because a preference nobody can honour should
    /// leave a working application. Tags are compared case-insensitively: they
    /// are conventionally cased but not required to be.
    #[must_use]
    pub fn resolve(tag: &str) -> &'static Self {
        let tag = tag.to_ascii_lowercase();
        let mut parts = tag.split('-');
        let primary = parts.next().unwrap_or_default();
        let rest: Vec<&str> = parts.collect();

        let by_code = |want: &str| {
            LANGUAGES
                .iter()
                .find(|l| l.code.eq_ignore_ascii_case(want))
                .unwrap_or(&LANGUAGES[0])
        };
        if let Some(exact) = LANGUAGES
            .iter()
            .find(|l| l.code.to_ascii_lowercase() == tag)
        {
            return exact;
        }
        if primary == "zh" {
            let traditional = rest
                .iter()
                .any(|p| matches!(*p, "hant" | "tw" | "hk" | "mo"));
            return by_code(if traditional { "zh-Hant" } else { "zh-Hans" });
        }
        LANGUAGES
            .iter()
            .find(|l| l.code.split('-').next() == Some(primary))
            .unwrap_or(&LANGUAGES[0])
    }
}

/// Every language shipped. English is first and is the fallback.
pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
        english_name: "English",
        source: include_str!("../data/strings_en.toml"),
    },
    Language {
        code: "de",
        name: "Deutsch",
        english_name: "German",
        source: include_str!("../data/strings_de.toml"),
    },
    Language {
        code: "pt",
        name: "Português",
        english_name: "Portuguese",
        source: include_str!("../data/strings_pt.toml"),
    },
    Language {
        code: "zh-Hans",
        name: "简体中文",
        english_name: "Chinese (Simplified)",
        source: include_str!("../data/strings_zh-Hans.toml"),
    },
    Language {
        code: "zh-Hant",
        name: "繁體中文",
        english_name: "Chinese (Traditional)",
        source: include_str!("../data/strings_zh-Hant.toml"),
    },
];

/// English, compiled in. A build always has one language.
const EN: &str = LANGUAGES[0].source;

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

    /// The catalogue for a language tag, **filled in from English**.
    ///
    /// An unknown tag gives English rather than an error: a language preference
    /// is not an engineering input, and a reader who arrives with one this build
    /// does not ship should get a working application rather than a refusal.
    ///
    /// Every key English has, this has — a translation that falls behind shows
    /// the English sentence rather than a bare key, which is the failure a
    /// reader can actually act on. A test holds every shipped file to the full
    /// key set, so the fallback is a safety net rather than the plan.
    ///
    /// # Panics
    ///
    /// Never in a shipped build, for the reason [`Self::english`] gives: every
    /// file is parsed by the test suite.
    #[must_use]
    pub fn for_language(tag: &str) -> Self {
        let mut base = Self::english();
        let lang = Language::resolve(tag);
        if lang.code == LANGUAGES[0].code {
            return base;
        }
        let translated = Self::parse(lang.source).expect("every compiled-in catalogue must parse");
        base.messages.extend(translated.messages);
        base
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
        assert!(c.get("mesh.self_locking").is_some());
    }

    /// **Every shipped language says everything English says, and nothing more.**
    ///
    /// A translation is allowed to fall behind at run time — `for_language`
    /// fills the gap from English, because a reader would rather meet an English
    /// sentence than a bare key. But falling behind is not allowed to *ship*: a
    /// missing key here is a message nobody will ever see in that language and
    /// nobody will ever notice, and an invented one is a message the code cannot
    /// reach. Both are caught here rather than discovered by a reader.
    ///
    /// Placeholders are held to the same standard. A translator may move
    /// `{name}` wherever the grammar wants it; inventing one silently produces
    /// a sentence with a brace in it, and dropping one silently loses the number
    /// the sentence existed to carry.
    #[test]
    fn every_language_carries_the_whole_catalogue() {
        let en = Catalogue::english();
        let placeholders = |text: &str| -> std::collections::BTreeSet<String> {
            let mut out = std::collections::BTreeSet::new();
            let mut rest = text;
            while let Some(open) = rest.find('{') {
                let after = &rest[open + 1..];
                let Some(close) = after.find('}') else { break };
                out.insert(after[..close].to_string());
                rest = &after[close + 1..];
            }
            out
        };

        for lang in LANGUAGES {
            let c = Catalogue::parse(lang.source)
                .unwrap_or_else(|e| panic!("{} does not parse: {e}", lang.code));
            let (mine, theirs): (Vec<_>, Vec<_>) = (
                en.messages().keys().collect(),
                c.messages().keys().collect(),
            );
            assert_eq!(
                mine, theirs,
                "{} does not have exactly English's keys",
                lang.code
            );
            for (key, english) in en.messages() {
                let translated = &c.messages()[key];
                assert_eq!(
                    placeholders(english),
                    placeholders(translated),
                    "{}: {key} does not use the same placeholders as English",
                    lang.code
                );
                assert!(
                    !translated.trim().is_empty(),
                    "{}: {key} is empty",
                    lang.code
                );
            }
        }
    }

    /// A browser's tag reaches the language it names, or English.
    #[test]
    fn a_language_tag_resolves_to_the_catalogue_it_names() {
        for (tag, want) in [
            ("en", "en"),
            ("EN-gb", "en"),
            ("de", "de"),
            ("de-CH", "de"),
            ("de-AT", "de"),
            ("pt", "pt"),
            ("pt-BR", "pt"),
            ("pt-PT", "pt"),
            // Chinese is decided by script, and the region usually implies it.
            ("zh", "zh-Hans"),
            ("zh-CN", "zh-Hans"),
            ("zh-Hans", "zh-Hans"),
            ("zh-SG", "zh-Hans"),
            ("zh-TW", "zh-Hant"),
            ("zh-HK", "zh-Hant"),
            ("zh-Hant", "zh-Hant"),
            ("zh-Hant-TW", "zh-Hant"),
            // ...and anything this build does not ship reads in English.
            ("fr", "en"),
            ("xx-Nowhere", "en"),
            ("", "en"),
        ] {
            assert_eq!(Language::resolve(tag).code, want, "tag {tag}");
        }
    }

    /// An unknown tag is answered, not refused, and a known one is translated.
    #[test]
    fn an_unknown_language_falls_back_rather_than_failing() {
        let en = Catalogue::english();
        for tag in ["", "en", "xx-Nowhere", "de-CH"] {
            let c = Catalogue::for_language(tag);
            assert_eq!(
                c.messages().keys().collect::<Vec<_>>(),
                en.messages().keys().collect::<Vec<_>>(),
                "{tag} should still answer with every key"
            );
        }
        assert_eq!(Catalogue::for_language("xx").messages(), en.messages());
        // A file that is still a copy of English would fall back invisibly, so
        // the gate is on **coverage** rather than on one chosen word: some
        // messages legitimately survive translation unchanged — "Material" in
        // Portuguese, "mm" everywhere — and picking a key that happened to be
        // one of them failed a translation that was fine.
        for lang in LANGUAGES.iter().filter(|l| l.code != "en") {
            let c = Catalogue::for_language(lang.code);
            let differ = en
                .messages()
                .iter()
                .filter(|(k, v)| c.get(k) != Some(v.as_str()))
                .count();
            let total = en.messages().len();
            assert!(
                differ * 4 >= total * 3,
                "{}: only {differ} of {total} messages differ from English — \
                 that is a copy, not a translation",
                lang.code
            );
        }
    }

    /// Values go in where the braces are, and the core's formatting survives
    /// untouched — the catalogue never re-rounds anything.
    #[test]
    fn a_note_renders_with_its_values() {
        let c = Catalogue::english();
        let note = Note::new(key::MESH_SELF_LOCKING)
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
    /// is the one that goes wrong (docs/corrections.md). Anything this sweep does not
    /// reach is simply not checked, which is honest — it is not claimed to be
    /// exhaustive, only true.
    fn observed_values() -> BTreeMap<String, Vec<String>> {
        use gear_core::params::GearParams;
        use gear_core::tooth::Tooth;

        let mut seen: BTreeMap<String, Vec<String>> = BTreeMap::new();
        /// Every note a stage result carries, wherever it carries it: the
        /// stage's own, each member's and each mesh's. A sweep that walked the
        /// fields it thought of missed the meshes the moment they got a list.
        trait EveryNote {
            fn every_note(&self) -> Vec<Note>;
        }
        fn every_note_of(r: &gear_core::train::StageResult) -> Vec<Note> {
            let mut out: Vec<Note> = match r {
                gear_core::train::StageResult::Shape(s) => s.notes.clone(),
            };
            out.extend(r.members().iter().flat_map(|g| g.notes.iter().cloned()));
            out.extend(r.meshes().iter().flat_map(|m| m.notes.iter().cloned()));
            out
        }
        impl EveryNote for gear_core::train::StageResult {
            fn every_note(&self) -> Vec<Note> {
                every_note_of(self)
            }
        }
        impl EveryNote for gear_core::train::TrainResult {
            fn every_note(&self) -> Vec<Note> {
                let mut out: Vec<Note> = self.cases.iter().flat_map(|c| c.notes.clone()).collect();
                out.extend(self.stages.iter().flat_map(every_note_of));
                out
            }
        }
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
                        let g = Tooth::new(GearParams {
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
                    &Tooth::new(GearParams {
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
            &Tooth::new(GearParams {
                pressure_angle: 0.5,
                ..Default::default()
            })
            .clamps
            .notes,
        );

        // Gear gears, whose per-tooth troubles are reported by the
        // assembly rather than by any one `Tooth` — a tooth that is undercut or
        // pointed while its neighbours are not.
        for (shift, amplitude) in [(0.0_f64, 0.3_f64), (-1.0, 0.3), (1.4, 0.3), (0.5, 0.5)] {
            let e = gear_core::gear::Gear::new(GearParams {
                teeth: 24,
                profile_shift: shift,
                angular_shift: amplitude,
                ..Default::default()
            });
            record(&e.per_tooth_clamps().notes);
        }

        // Rings, which have guard rails of their own and none in common with an
        // external gear: the cutter can be too big, too blunt, or unable to
        // reach the flank at all.
        for teeth in [18_u32, 30, 60] {
            for cutter_teeth in [8_u32, 20, 40, 90] {
                for shift in [-0.6_f64, 0.0, 0.8] {
                    for tip_round in [0.02_f64, 0.38] {
                        let ring = gear_core::ring::Ring::cut_by(
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
                            let extreme = gear_core::ring::Ring::cut_by(
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
        // notes each of them can raise. Every one of them is the one shape;
        // the names here say which preset the case is about.
        let lib = crate::default_library();
        let solve_spur = |stage: &gear_core::train::PairStage,
                          loads: &gear_core::train::StageLoads,
                          lib: &gear_core::material::MaterialLibrary| {
            gear_core::train::solve_any(&gear_core::train::Stage::spur(stage.clone()), loads, lib)
        };
        let solve_crossed = solve_spur;
        let solve_worm = |stage: &gear_core::train::PairStage,
                          loads: &gear_core::train::StageLoads,
                          lib: &gear_core::material::MaterialLibrary| {
            gear_core::train::solve_any(&gear_core::train::Stage::worm(stage.clone()), loads, lib)
        };
        for helix in [0.0_f64, 3.0, 20.0] {
            for teeth in [(17_u32, 43_u32), (9, 11)] {
                let stage = gear_core::train::PairStage {
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
                }
                .with_additional_helix(helix);
                if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                    record(&r.every_note());
                }
                for sigma in [0.5_f64, 90.0] {
                    for face in [
                        gear_core::params::Auto::automatic(0.0),
                        gear_core::params::Auto::fixed(0.4),
                    ] {
                        let mut crossed = gear_core::train::PairStage {
                            shaft_angle: sigma,
                            ..stage.clone()
                        };
                        for g in &mut crossed.gears {
                            g.face_width = face;
                        }
                        if let Ok(r) =
                            solve_crossed(&crossed, &gear_core::train::StageLoads::just(2.0), &lib)
                        {
                            record(&r.every_note());
                        }
                    }
                }
            }
        }
        // Load sharing switched on, which is off by default and so unreachable
        // from the grid above. The tall tooth is the point: a **high contact
        // ratio** design — the addendum a real one is given — puts the virtual
        // ratio past 2, where the ramp has no single-pair zone left to work
        // with and the stage says so. A standard tooth cannot reach it at any
        // helix, which is why this case carries an addendum rather than an
        // angle.
        for (teeth, addendum) in [(17_u32, 1.0_f64), (60, 1.35)] {
            let gear = gear_core::train::StageGear {
                teeth,
                addendum,
                profile_shift: gear_core::params::Auto::fixed(0.0),
                ..Default::default()
            };
            let stage = gear_core::train::PairStage {
                load_sharing: gear_core::contact::LoadSharing::LinearRamp,
                gears: [gear.clone(), gear],
                ..Default::default()
            };
            if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
            }
        }
        // **A rim thinner than ISO 6336-3 will rate.** Half a tooth deep is
        // past the backup ratio below which the clause says a design shall be
        // avoided — an ordinary input rather than a contrived one, and still
        // rated, which is the point of the message: the figure is given and the
        // reader is told where it stands.
        for rim in [Some(1.0_f64), None] {
            let gear = gear_core::train::StageGear {
                teeth: 23,
                rim_thickness: rim,
                ..Default::default()
            };
            let stage = gear_core::train::PairStage {
                gears: [gear.clone(), gear],
                ..Default::default()
            };
            if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
            }
        }
        // A **sharp** rack on many teeth, and a blunt one on few: the extremes
        // of the fillet the notch factor is read off, kept in the sweep because
        // the clamps a cutter raises live out here even though the `Y_S` band
        // that first brought this case no longer belongs to any stage.
        for (teeth, root_radius) in [(300_u32, 0.0_f64), (17, 0.38)] {
            let gear = gear_core::train::StageGear {
                teeth,
                root_radius,
                ..Default::default()
            };
            let stage = gear_core::train::PairStage {
                gears: [gear.clone(), gear],
                ..Default::default()
            };
            if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
            }
        }
        // **Both ends of a tooth pushed past what it can carry.** A tall
        // addendum on a small pinion comes to a point, so the tip-width bound
        // cuts it down.
        {
            let gear = |teeth: u32| gear_core::train::StageGear {
                teeth,
                addendum: 1.6,
                min_tip_width: 0.4,
                ..Default::default()
            };
            let stage = gear_core::train::PairStage {
                gears: [gear(17), gear(43)],
                ..Default::default()
            };
            if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
                // A bound that moved a gear's own number rides that gear.
                for g in r.members() {
                    record(&g.notes);
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
            let mut stage = gear_core::train::PairStage {
                sliding_friction: friction,
                ..gear_core::train::PairStage::worm()
            };
            stage.gears[0].teeth = starts;
            if let Ok(r) = solve_worm(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
            }
        }
        // A planet's root is loaded both ways whatever the drive does, so the
        // correction switched on is what fires the "applied" half of the pair;
        // switched off — every other case here — fires the disclosure.
        if let Ok(r) = gear_core::train::solve_any_with(
            &gear_core::train::Stage::planetary(gear_core::train::PlanetaryStage::default()),
            &gear_core::train::StageLoads::just(2.0),
            &lib,
            gear_core::train::Reversal { correct: true },
        ) {
            record(&r.every_note());
            // ...and what the members themselves say, which is where a note
            // about one gear belongs.
            for g in r.members() {
                record(&g.notes);
            }
        }
        for (planets, clearance) in [(3_u32, 0.5_f64), (4, 9.0), (5, 0.5), (6, 40.0), (7, 0.5)] {
            let stage = gear_core::train::PlanetaryStage {
                planets,
                min_planet_clearance: clearance,
                ..Default::default()
            };
            if let Ok(r) = gear_core::train::solve_any(
                &gear_core::train::Stage::planetary(stage),
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
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
                    &gear_core::ring::Ring::cut_by(
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
            let stage = gear_core::train::PairStage {
                gears: [
                    gear_core::train::StageGear {
                        teeth: 17,
                        addendum,
                        ..Default::default()
                    },
                    gear_core::train::StageGear {
                        teeth: 43,
                        addendum,
                        ..Default::default()
                    },
                ],
                ..Default::default()
            };
            if let Ok(r) = solve_spur(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
            }
            // ...and a crossed pair of the same, whose teeth then reach a full
            // contact ratio at no width at all.
            for sigma in [45.0_f64, 90.0] {
                let crossed = gear_core::train::PairStage {
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
                if let Ok(r) =
                    solve_crossed(&crossed, &gear_core::train::StageLoads::just(2.0), &lib)
                {
                    record(&r.every_note());
                }
            }
        }

        // A screw pair entered by helix angle with automatic widths, which has
        // no enveloping wheel and so no published proportion to take.
        // Static coefficients either side of the default worm's 0.1327
        // threshold, so both the self-locking note and the "close to it" one
        // get their turn.
        for friction in [0.06_f64, 0.115, 0.12, 0.125] {
            let stage = gear_core::train::PairStage {
                sliding_friction: friction,
                static_friction: friction,
                ..gear_core::train::PairStage::worm()
            }
            .with_first_helix(45.0);
            if let Ok(r) = solve_crossed(&stage, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
            }
            // ...the optimiser asked of a crossed mesh, whose search has its
            // own way of finding nothing.
            if let Ok(r) = solve_crossed(
                &gear_core::train::PairStage {
                    optimisation: gear_core::train::Optimisation {
                        enabled: true,
                        ..gear_core::train::Optimisation::default()
                    },
                    ..stage.clone()
                },
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
            }
            // ...and a worm sitting just under its self-locking threshold.
            if let Ok(r) = solve_worm(
                &gear_core::train::PairStage {
                    sliding_friction: friction,
                    static_friction: friction,
                    ..gear_core::train::PairStage::worm()
                },
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
            }
        }

        // **An optimiser with nothing to choose.** A hula stage at a one-tooth
        // difference opens to about 45° of operating pressure angle to clear
        // itself and sits at `ε ≈ 1.02`: every split in the admissible interval
        // is refused, so the search has no answer and the shifts stay where they
        // were. Indistinguishable from a search that agreed, until it said so
        // (`docs/corrections.md`; the audit's record F58, F82).
        {
            let mut hula = gear_core::train::HulaStage::default();
            hula.optimisation.enabled = true;
            for (g, z) in hula.gears.iter_mut().zip([18u32, 19, 19, 20]) {
                g.teeth = z;
            }
            if let Ok(r) = gear_core::train::solve_any(
                &gear_core::train::Stage::hula(hula),
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
            }
        }

        // **A centre distance no admissible shifts reach**, and the negative
        // clearance that comes of asking for one well inside what the teeth
        // allow. A 9/37 pair told to run at 23.00 mm has its shifts pinned at
        // the pinion's undercut floor, which puts the pair at 23.4433 — so it
        // runs 0.44 mm *inside* its own zero-backlash distance and cannot be
        // assembled at all. Both used to be silent.
        for distance in [23.0_f64, 23.46, 25.0] {
            let mut sp = gear_core::train::PairStage {
                centre_distance: gear_core::params::Auto::fixed(distance),
                ..Default::default()
            };
            sp.gears[0].teeth = 9;
            sp.gears[1].teeth = 37;
            if let Ok(r) = solve_spur(&sp, &gear_core::train::StageLoads::just(2.0), &lib) {
                record(&r.every_note());
            }
        }

        // **The other end of the same question: a pair that cannot be driven
        // forward.** A very steep first member — a 3° helix, so an 87° lead
        // angle — locks *forwards* at µ ≈ 0.049 while back-driving perfectly
        // well at 0.45 efficiency. It is the case `Directional::locked` was made
        // directional for, and it is reachable from the front end: it is what
        // `gear-cli crossed 17 23 90` shows at its 9°/81° split.
        //
        // Frictions either side of that threshold, so the "cannot be driven
        // forward" note and the "close to it" one both get their turn.
        for friction in [0.02_f64, 0.045, 0.06, 0.10] {
            if let Ok(r) = solve_crossed(
                &gear_core::train::PairStage {
                    sliding_friction: friction,
                    static_friction: friction,
                    ..gear_core::train::PairStage::worm()
                }
                .with_first_helix(3.0),
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
            }
        }

        // A planetary set whose ring's tip clamps at its base circle.
        //
        // **These used to raise the addendum and none of them solved.** Five
        // sets were built with a ring addendum of 1.7 to 2.6 modules, every one
        // of them failed with *the teeth never come into contact*, and the
        // `if let Ok` below swallowed it — so the note was declared unfirable
        // "because it was looked for and not found" when the looking had never
        // happened. `docs/corrections.md`'s own heading: *a test that never
        // meets the case it is about passes against the fault.*
        //
        // A ring's tip is `r − m(h_a − x)` and its clamp is that tip reaching
        // the base circle, so what drives it there is a **short** addendum
        // against a **negative** shift, not a tall one. Swept: it fires on 441
        // of 1,482 sets that solve.
        let mut ring_cases_solved = 0;
        for (sun, planet, addendum, shift) in [
            (11_u32, 13_u32, 0.5_f64, -1.0_f64),
            (11, 13, 0.7, -1.4),
            (17, 18, 0.5, -1.6),
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
                    // **And its tooth count**, which the version before this
                    // left at `StageGear`'s own 17 while setting the sun and the
                    // planet — a ring that does not close the set it is in.
                    teeth: sun + 2 * planet,
                    addendum,
                    profile_shift: gear_core::params::Auto::fixed(shift),
                    ..Default::default()
                },
                ..Default::default()
            };
            match gear_core::train::solve_any(
                &gear_core::train::Stage::planetary(stage),
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                Ok(r) => {
                    record(&r.every_note());
                    for g in r.members() {
                        record(&g.notes);
                    }
                    ring_cases_solved += 1;
                }
                // The other two cannot be assembled at all — no planet shift
                // closes them — and are refused for it, which is the model's
                // to say and not this sweep's to hide.
                Err(e) => assert!(
                    matches!(e, gear_core::train::TrainError::NoCommonDistance),
                    "the ring case {sun}/{planet} fails for a reason other than assembly: {e}"
                ),
            }
        }
        assert!(ring_cases_solved >= 1, "the looking has to happen");

        // ---- the whole train ---------------------------------------- //
        //
        // The notes no stage can fire, because they are facts about the shaft
        // line: a load case that is either held somewhere or held nowhere.
        // Fired through `solve_train` for the same reason the clamps are fired
        // through the geometry — the case has to be live, not merely
        // constructible.
        {
            use gear_core::train::{LoadCase, PairStage, ShaftRef, Stage, Train};
            let (start, end) = (
                ShaftRef::Of { stage: 0, shaft: 1 },
                ShaftRef::Of { stage: 0, shaft: 2 },
            );
            let train = |stages| {
                Train::chained(
                    stages,
                    vec![
                        LoadCase::ultimate(start, end, 2.0, 3000.0),
                        // A load from the end that nothing is asked to hold:
                        // the start declared free beside it.
                        LoadCase {
                            loads: vec![
                                gear_core::train::Load::given(end, 5.0, 0.0),
                                gear_core::train::Load::declared(
                                    start,
                                    gear_core::train::LoadRole::Free,
                                ),
                            ],
                            ..LoadCase::back_driving(start, end, 5.0)
                        },
                    ],
                )
            };
            // A load nothing reacts...
            if let Ok(r) =
                gear_core::train::solve_train(&train(vec![Stage::spur(PairStage::default())]), &lib)
            {
                record(&r.every_note());
            }
            // ...and the same load against a worm that cannot be back-driven.
            if let Ok(r) = gear_core::train::solve_train(
                &train(vec![Stage::worm(PairStage {
                    sliding_friction: 0.3,
                    static_friction: 0.3,
                    ..PairStage::worm()
                })]),
                &lib,
            ) {
                record(&r.every_note());
            }
            // **The three ways a case can fail to be one**: no load at all;
            // a load whose speed nobody gave, on a train with a degree of
            // freedom left; and a given torque of nought, the only one given,
            // so no load does any work and nothing drives.
            {
                use gear_core::params::Auto;
                use gear_core::train::{Load, LoadRole};
                let mut t = train(vec![Stage::spur(PairStage::default())]);
                t.load_cases = vec![
                    LoadCase {
                        loads: Vec::new(),
                        ..LoadCase::ultimate(start, end, 2.0, 3000.0)
                    },
                    LoadCase {
                        loads: vec![
                            Load {
                                at: start,
                                role: LoadRole::Load,
                                torque: Auto::fixed(2.0),
                                speed: Auto::automatic(0.0),
                            },
                            Load::declared(end, LoadRole::Reacted),
                        ],
                        ..LoadCase::ultimate(start, end, 2.0, 3000.0)
                    },
                    LoadCase {
                        loads: vec![
                            Load::given(start, 0.0, 3000.0),
                            Load::declared(end, LoadRole::Reacted),
                        ],
                        ..LoadCase::ultimate(start, end, 2.0, 3000.0)
                    },
                ];
                if let Ok(r) = gear_core::train::solve_train(&t, &lib) {
                    record(&r.every_note());
                }
            }
            // A ratio asked to decide the helix that no helix reaches: three
            // base pitches of overlap on a two-millimetre face.
            {
                let mut narrow = PairStage {
                    overlap: gear_core::params::Auto::fixed(3.0),
                    ..PairStage::default()
                };
                for g in &mut narrow.gears {
                    g.face_width = gear_core::params::Auto::fixed(2.0);
                }
                if let Ok(r) = solve_spur(&narrow, &gear_core::train::StageLoads::just(2.0), &lib) {
                    record(&r.every_note());
                }
            }
            // A ratio given as a floor under an automatic width, on straight
            // teeth — where no width buys any overlap.
            {
                let mut straight = PairStage {
                    overlap: gear_core::params::Auto::fixed(1.2),
                    ..PairStage::default()
                };
                straight.gears[0].face_width = gear_core::params::Auto::automatic(5.0);
                if let Ok(r) = solve_spur(&straight, &gear_core::train::StageLoads::just(2.0), &lib)
                {
                    record(&r.every_note());
                }
            }
            // An automatic face width with every rating switched off.
            let no_source = gear_core::train::StageGear {
                face_width: gear_core::params::Auto::automatic(0.0),
                face_sources: gear_core::train::FaceSources {
                    bending: gear_core::train::ByKind {
                        ultimate: false,
                        fatigue: false,
                    },
                    contact: gear_core::train::ByKind {
                        ultimate: false,
                        fatigue: false,
                    },
                },
                ..Default::default()
            };
            if let Ok(r) = solve_spur(
                &PairStage {
                    gears: [no_source.clone(), no_source],
                    ..PairStage::default()
                },
                &gear_core::train::StageLoads::just(2.0),
                &lib,
            ) {
                record(&r.every_note());
                for g in r.members() {
                    record(&g.notes);
                }
            }
        }

        // ---- the errors -------------------------------------------- //
        //
        // Every reason a result does not exist, fired from the geometry that
        // produces it rather than by constructing the variant. Building the
        // enum by hand would check that a key has a message and nothing else;
        // reaching it through the model also checks that the case is *live* —
        // the same standard the clamps above are held to, and the reason a note
        // nothing can fire has to be named in `UNFIRED` with its evidence.
        {
            use gear_core::note::Explain;
            let g = |p| gear_core::Tooth::new(p);
            let d = gear_core::GearParams::default();
            let mut err = |n: gear_core::note::Note| record(std::slice::from_ref(&n));

            // A mesh: different racks, a ring no bigger than its pinion, shifts
            // that leave the involute domain, and centres inside the base
            // circles.
            let big = gear_core::GearParams { module: 2.0, ..d };
            let pair = |a, b, k| gear_core::Mesh::new(&g(a), &g(b), k).map(|_| ());
            if let Err(e) = pair(d, big, gear_core::MeshKind::External) {
                err(e.note());
            }
            let small_ring = gear_core::GearParams { teeth: 9, ..d };
            if let Err(e) = pair(d, small_ring, gear_core::MeshKind::Internal) {
                err(e.note());
            }
            let very_thin = gear_core::GearParams {
                profile_shift: -2.0,
                ..d
            };
            if let Err(e) = pair(very_thin, very_thin, gear_core::MeshKind::External) {
                err(e.note());
            }
            if let Ok(m) = gear_core::Mesh::new(&g(d), &g(d), gear_core::MeshKind::External) {
                if let Err(e) = m.backlash(1.0) {
                    err(e.note());
                }
            }

            // Metrology: no measurable span, and pins that miss the flank, sit
            // on the root, fall through, or ride out past the base circle.
            // A one-tooth gear at a steep helix has no `k` whose contact lands
            // on the usable flank, which is the honest answer rather than a
            // number nobody could measure.
            for p in [
                gear_core::GearParams {
                    teeth: 1,
                    pressure_angle: 10.0,
                    profile_shift: -0.5,
                    helix_angle: 70.0,
                    ..d
                },
                gear_core::GearParams { teeth: 3, ..d },
            ] {
                if let Err(e) = gear_core::metrology::best_span(&g(p)) {
                    err(e.note());
                }
            }
            for (p, dia) in [
                (d, 0.2_f64),
                (d, 1.75),
                (d, 4.0),
                (gear_core::GearParams { teeth: 5, ..d }, 3.0),
                (gear_core::GearParams { teeth: 200, ..d }, 0.05),
                (
                    gear_core::GearParams {
                        teeth: 1,
                        pressure_angle: 10.0,
                        profile_shift: -0.5,
                        ..d
                    },
                    1.75,
                ),
            ] {
                if let Err(e) =
                    gear_core::metrology::over_pins(&g(p), dia, gear_core::metrology::PinCount::Two)
                {
                    err(e.note());
                }
            }

            // A screw pair: nothing positive, a worm too thin for its starts, a
            // shaft angle the wheel cannot take, a first member that is a disc,
            // and parallel axes.
            use gear_core::screw::{Screw, ScrewParams};
            for (starts, dia, sigma, module) in [
                (1_u32, 7.0_f64, 90.0_f64, 0.0_f64),
                (9, 3.0, 90.0, 1.0),
                (1, 7.0, 179.999, 1.0),
                (1, 7.0, 0.0, 1.0),
            ] {
                let sp = ScrewParams {
                    normal_module: module,
                    normal_pressure_angle_rad: 20.0_f64.to_radians(),
                    shaft_angle_rad: sigma.to_radians(),
                    starts,
                    wheel_teeth: 40,
                    worm_pitch_diameter: dia,
                    profile_shifts: [0.0; 2],
                };
                if let Err(e) = Screw::new(&sp) {
                    err(e.note());
                }
            }
            err(gear_core::screw::ScrewError::FirstMemberIsADisc.note());

            // A train: no contact, an unknown material, a tooth with no root
            // section left to rate, and no stages at all.
            use gear_core::train::{Train, TrainError};
            err(TrainError::NoContact.note());
            // **...and a member with no teeth**, which is the one refusal a
            // wiring has that a *design* can reach: `teeth` is a `u32` and
            // nothing stops a designer typing zero. It was answered with "the
            // tooth is too undercut to have a root section", which describes a
            // tooth that exists.
            {
                let mut st = gear_core::train::PairStage::default();
                st.gears[1].teeth = 0;
                let out = gear_core::train::solve_any(
                    &gear_core::train::Stage::spur(st),
                    &gear_core::train::StageLoads::just(1.0),
                    &lib,
                );
                assert!(
                    matches!(out, Err(TrainError::Wiring(_))),
                    "a member with no teeth is no mechanism, whatever else is wrong"
                );
                if let Err(e) = out {
                    err(e.note());
                }
            }
            // **...and a set whose centre distances no shift can bring
            // together, fired from the model rather than built by hand.** At a
            // 17-tooth sun and 17-tooth planets only `z_ring ∈ [48, 54]` admits
            // any planet shift at all, so 80 is genuinely impossible. It used
            // to say *the teeth never contact*, at a reader who would then go
            // and look at the teeth.
            {
                let mut set = gear_core::train::PlanetaryStage::default();
                set.sun.teeth = 17;
                set.planet.teeth = 17;
                set.ring.teeth = 80;
                let out = gear_core::train::solve_any(
                    &gear_core::train::Stage::planetary(set),
                    &gear_core::train::StageLoads::just(1.0),
                    &lib,
                );
                assert!(
                    matches!(out, Err(TrainError::NoCommonDistance)),
                    "this set is the one that cannot be assembled"
                );
                if let Err(e) = out {
                    err(e.note());
                }
            }
            err(TrainError::UnknownMaterial("nothing by that name".into()).note());
            err(TrainError::NoRootSection.note());
            if let Err(e) = gear_core::train::solve_train(
                &Train::chained(
                    Vec::new(),
                    vec![gear_core::train::LoadCase::ultimate(
                        gear_core::train::ShaftRef::Ground,
                        gear_core::train::ShaftRef::Ground,
                        2.0,
                        3000.0,
                    )],
                ),
                &lib,
            ) {
                err(e.note());
            }
            // **A load case that names a shaft it cannot enter by, and one
            // that enters between two stages** — both fired from the model,
            // on a two-pair chain: a load on the held ground, and a load on
            // the shaft the two pairs share.
            {
                use gear_core::train::{
                    Load, LoadCase, LoadRole, PairStage, ShaftRef, Stage, Train,
                };
                let (start, end) = (
                    ShaftRef::Of { stage: 0, shaft: 1 },
                    ShaftRef::Of { stage: 1, shaft: 2 },
                );
                let at = |port| {
                    Train::chained(
                        vec![
                            Stage::spur(PairStage::default()),
                            Stage::spur(PairStage::default()),
                        ],
                        vec![LoadCase {
                            loads: vec![
                                Load::given(port, 2.0, 3000.0),
                                Load::declared(start, LoadRole::Reacted),
                                Load::declared(end, LoadRole::Reacted),
                            ],
                            ..LoadCase::ultimate(start, end, 2.0, 3000.0)
                        }],
                    )
                };
                for port in [ShaftRef::Ground, ShaftRef::Of { stage: 1, shaft: 1 }] {
                    let out = gear_core::train::solve_train(&at(port), &lib);
                    assert!(
                        matches!(
                            out,
                            Err(TrainError::LoadPort { case: 0 }
                                | TrainError::LoadShared { case: 0 })
                        ),
                        "a load at {port:?} is refused by name, not solved: {out:?}"
                    );
                    if let Err(e) = out {
                        err(e.note());
                    }
                }
            }
            // **Every way the train's own conditions can fail to give one
            // motion**, each fired from the model on a set whose shafts are
            // sun 1, carrier 2, ring 3: the carrier and the ring both held
            // (the sun cannot turn); a constraint on a shaft no stage has;
            // and a chain whose tooth counts multiply past `i128`.
            {
                use gear_core::train::{
                    LoadCase, PairStage, PlanetaryStage, ShaftConstraint, ShaftRef, Stage,
                    StageGear, Train,
                };
                let set = |constraints| {
                    let mut t = Train::chained(
                        vec![Stage::planetary(PlanetaryStage::default())],
                        vec![LoadCase::ultimate(
                            ShaftRef::Of { stage: 0, shaft: 1 },
                            ShaftRef::Of { stage: 0, shaft: 2 },
                            2.0,
                            3000.0,
                        )],
                    );
                    t.constraints = constraints;
                    t
                };
                let huge = |teeth| StageGear {
                    teeth,
                    ..StageGear::default()
                };
                let wide = Train {
                    stages: (0..6)
                        .map(|k| {
                            Stage::spur(PairStage {
                                gears: [huge(4_000_000_000 + k), huge(4_000_000_001 + k)],
                                ..PairStage::default()
                            })
                        })
                        .collect(),
                    ..set(Vec::new())
                };
                let trains = [
                    (
                        set(vec![
                            ShaftConstraint::held(0, 2),
                            ShaftConstraint::held(0, 3),
                        ]),
                        "overdetermined",
                    ),
                    (set(vec![ShaftConstraint::held(7, 1)]), "no such shaft"),
                    (wide, "overflow"),
                ];
                for (train, what) in trains {
                    let e = gear_core::train::solve_train(&train, &lib).expect_err(what);
                    assert!(
                        matches!(
                            e,
                            TrainError::Overdetermined { .. }
                                | TrainError::NoSuchShaft { .. }
                                | TrainError::Overflow
                                | TrainError::InStage { .. }
                        ),
                        "{what}: {e:?}"
                    );
                    err(e.note());
                }
            }

            // **A distance no tips can clear**: a planocentric asked for a
            // far-side gap wider than any distance a pinion still inside its
            // ring can give.
            {
                let mut shape = gear_core::train::arrangements::planocentric(40, 41);
                shape.distances[0].tip_clearance = 1000.0;
                match gear_core::train::solve_any(
                    &gear_core::train::Stage::Shape(Box::new(shape)),
                    &gear_core::train::StageLoads::just(2.0),
                    &lib,
                ) {
                    Err(e) => {
                        assert!(
                            matches!(e, gear_core::train::TrainError::TipsUnclearable { .. }),
                            "{e:?}"
                        );
                        err(e.note());
                    }
                    Ok(_) => panic!("no distance gives a metre of tip gap"),
                }
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
    ///   before mid-space. Re-searched over **71,750** combinations of ring
    ///   teeth, cutter teeth, tip round, cutter addendum, thickness modification,
    ///   profile shift and module, and it never fires. (The first search was
    ///   ~11,000 and this is the one the entry now rests on — evidence has a
    ///   date, and a crate that has changed underneath it deserves a fresh
    ///   look.) The likely reason is that `ShaperCut` already refuses a tool
    ///   whose own rounds overlap, which is close to the same condition — so the
    ///   guard may be shadowing it entirely.
    ///
    /// It is live code with a live message, so it is not deleted on suspicion.
    ///
    /// # One left this list, and how it got on it
    ///
    /// `ring_addendum_clamped` was here on the reading that "the set
    /// solves its ring's addendum, so it does not normally hand it one that
    /// cannot work". It fires on **441 of the 1,482 sets** a sweep of tooth
    /// counts, addenda, shifts, cutters and thickness modifications can solve.
    ///
    /// What put it here is worth more than the note. The sweep *did* have five
    /// cases aimed at it — and every one of them failed to solve, with *the
    /// teeth never come into contact*, inside an `if let Ok(…)` that said
    /// nothing. So "looked for and not found" was true of the sentence and false
    /// of the code: the looking never happened. They raised the ring's addendum,
    /// where a ring's tip is `r − m(h_a − x)` and reaches its base circle on a
    /// **short** addendum against a negative shift — and they left the ring's
    /// tooth count at `StageGear`'s own 17 while setting the sun's and the
    /// planet's, which is a ring that does not close the set it is in.
    ///
    /// *A case that cannot solve is not a case*, and an `if let Ok` around one
    /// is how it stays that way quietly.
    /// - `error.train_no_power_flow` — a stage with no self-consistent power
    ///   flow. The site is live (`train::shape`, the **forward** flow) and the
    ///   message is right; what no design reaches is that flow refusing.
    ///
    ///   `flow::solve` is asked twice by a stage: once forward at unit speed
    ///   and unit torque, and once backward with the output as the input. Only
    ///   the first is a refusal; the second is a `map_or(0.0, …)`, because a
    ///   stage that cannot be back-driven is self-locking and that is an
    ///   *answer* rather than a refusal (`docs/reference.md#the-stage`). A
    ///   genuinely driving input appears never to refuse: the flow is held to
    ///   Pennestrì's closed form on every arrangement (`flow::tests`), and
    ///   that form was swept over **1.1 million** combinations — sun 1…119
    ///   against ring 1…249, `η₀` from 0.999 down to 0.3, all six arrangements
    ///   — with no refusal at all, which
    ///   `planetary::tests::a_driving_input_always_has_a_flow` holds in the
    ///   crate rather than in a note. The one mesh that can lock, the
    ///   crossed-axis one, does not refuse the flow either: a mesh with no
    ///   efficiency in a direction **holds** — its driver presses the flanks
    ///   and its driven side takes nothing — which is a flow with an
    ///   efficiency of nought rather than none (`train::flow`).
    ///
    ///   So it is an exemption with a reason rather than a hole: the ways the
    ///   flow can refuse are an input that does not turn, which the motion
    ///   solve has already refused as undetermined, and no assignment of
    ///   directions confirming itself, which is the back-driven case the other
    ///   call site already treats as an answer.
    /// - `error.train_stage_undetermined` — a stage whose boundary leaves
    ///   its motion undetermined. A train no longer produces one: a boundary
    ///   that is a family is a stage with no figures of its own rather than
    ///   a refusal, and a boundary that contradicts itself is named at the
    ///   hold that closed it (`Train::boundaries`). What still raises it is
    ///   `Wiring::unit_motion` asked directly — a lone stage asked for a
    ///   backward load through `solve_any` with two of its ports free, which
    ///   the harness can do and no panel can. Live, and kept for the
    ///   harness.
    const UNFIRED: &[&str] = &[
        "clamp.ring_fully_filleted",
        "error.train_no_power_flow",
        "error.train_stage_undetermined",
    ];

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
