//! The material library as a human-readable TOML document.
//!
//! The spec asks for a preloaded library that can be exported, hand-edited and
//! imported again. That makes TOML the format and round-tripping the property
//! that matters: whatever the tool writes, it must read back unchanged.
//!
//! The default library is compiled in from `data/materials_default.toml`, so a
//! fresh install has values in every field and the calculator can produce a
//! number before the user has sourced anything. Those defaults are a starting
//! point, not an authority — see the file's own header for which of its numbers
//! are measured and which are estimates.

use gear_core::material::MaterialLibrary;

/// The library shipped with the tool.
const DEFAULT_TOML: &str = include_str!("../data/materials_default.toml");

/// What went wrong reading a library document.
#[derive(Debug)]
pub enum MaterialError {
    /// The document is not valid TOML, or does not match the material schema.
    Parse(toml::de::Error),
    /// The library could not be written back out.
    Serialise(toml::ser::Error),
    /// The document parsed but is not a usable library.
    Empty,
    /// Two materials share a name, so a selection by name would be ambiguous.
    DuplicateName(String),
    /// A figure no material has — not finite, or outside its row
    /// ([`gear_core::input::MATERIAL`]) — named by its path in the file.
    Implausible(gear_core::input::Refused),
}

impl MaterialError {
    /// The catalogue's note, where the refusal has one: a figure no
    /// material has. The rest are the parser's words or this module's.
    #[must_use]
    pub fn note(&self) -> Option<gear_core::note::Note> {
        use gear_core::note::Explain;
        match self {
            Self::Implausible(r) => Some(r.note()),
            _ => None,
        }
    }
}

impl std::fmt::Display for MaterialError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Parse(e) => write!(f, "material library is not valid: {e}"),
            Self::Serialise(e) => write!(f, "material library could not be written: {e}"),
            Self::Empty => write!(f, "material library contains no materials"),
            Self::DuplicateName(n) => {
                write!(f, "material library contains two materials named {n:?}")
            }
            Self::Implausible(r) => write!(f, "material library is not valid: {r}"),
        }
    }
}

impl std::error::Error for MaterialError {}

/// Parse a material library from TOML.
///
/// Rejects an empty library and duplicate names rather than accepting them:
/// both would surface much later as a material that cannot be selected, or one
/// that silently shadows another. Every figure is read against its row
/// ([`MaterialLibrary::check`]), named by its path in the file where it
/// describes no material.
pub fn from_toml(src: &str) -> Result<MaterialLibrary, MaterialError> {
    let lib: MaterialLibrary = toml::from_str(src).map_err(MaterialError::Parse)?;
    lib.check().map_err(MaterialError::Implausible)?;

    if lib.is_empty() {
        return Err(MaterialError::Empty);
    }
    for (i, m) in lib.materials.iter().enumerate() {
        if lib.materials[..i].iter().any(|p| p.name == m.name) {
            return Err(MaterialError::DuplicateName(m.name.clone()));
        }
    }
    Ok(lib)
}

/// Write a material library as TOML, in the same shape the reader accepts.
pub fn to_toml(lib: &MaterialLibrary) -> Result<String, MaterialError> {
    toml::to_string_pretty(lib).map_err(MaterialError::Serialise)
}

/// The default library, parsed.
///
/// Panics only if the compiled-in document is malformed, which a test in this
/// module rules out at build time.
///
/// # Panics
/// If `data/materials_default.toml` does not parse — a build-time defect, not a
/// runtime condition.
#[must_use]
#[expect(
    clippy::expect_used,
    reason = "the compiled-in document is parsed by this module's tests"
)]
pub fn default_library() -> MaterialLibrary {
    from_toml(DEFAULT_TOML).expect("compiled-in default material library must parse")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::material::{Basis, Family, Measure};

    #[test]
    fn the_default_library_parses_and_holds_the_expected_materials() {
        let lib = default_library();
        assert_eq!(
            lib.names(),
            [
                "4340 Steel",
                "4340 Hardened Steel",
                "Brass C360",
                "POM Delrin 100P",
                "PA6",
                "PA6 GF30",
                "PA GF50",
                "PA GF70",
            ]
        );
    }

    /// **A root loaded both ways on fully reversed data is judged at the
    /// stored figure, bit for bit.** The steels' and the brass's figures are
    /// rotating-beam endurances and POM's is ASTM D671's, all at `R = −1`:
    /// already the reversed case, so taking 0.7 of them counts the reversal
    /// twice. It did: only `gear-cli train toggles` caught a change to the
    /// fraction, and the suite was silent.
    #[test]
    fn a_reversed_root_on_fully_reversed_data_is_judged_at_the_stored_figure() {
        use gear_core::train::{CaseKind, Reversal};
        let lib = default_library();
        let reversed = [
            "4340 Steel",
            "4340 Hardened Steel",
            "Brass C360",
            "POM Delrin 100P",
        ];
        for m in &lib.materials {
            let is_reversed = reversed.contains(&m.name.as_str());
            assert_eq!(
                m.fatigue_load_ratio == Some(gear_core::material::LoadRatio::Reversed),
                is_reversed,
                "{}: its fatigue figure's ratio",
                m.name
            );
            let judged = Reversal { correct: true }.bending_allowable(m, CaseKind::Fatigue, true);
            if is_reversed {
                assert_eq!(
                    judged.to_bits(),
                    m.fatigue_allowable.value.to_bits(),
                    "{}: judged at {judged} against {}",
                    m.name,
                    m.fatigue_allowable.value
                );
            } else {
                // A figure that does not say is read one way: ISO's fraction.
                assert_eq!(
                    judged.to_bits(),
                    (m.fatigue_allowable.value * gear_core::material::REVERSED_BENDING_FRACTION)
                        .to_bits(),
                    "{}",
                    m.name
                );
            }
            // A replaced figure keeps none of the library's ratio.
            let own = m.overridden(&gear_core::material::Overrides {
                fatigue_allowable: Some(m.fatigue_allowable.value),
                ..Default::default()
            });
            assert_eq!(own.fatigue_load_ratio, None, "{}", m.name);
            assert_eq!(own.fatigue_allowable.note, None, "{}", m.name);
        }
    }

    /// **Every flank allowable is derived from what the library states, and
    /// none is borrowed.** The steels' come from their hardness on ISO
    /// 6336-5's through-hardened alloy line, at a grade a design can change;
    /// nothing publishes one for the brass, POM or the polyamides, so they
    /// have none rather than a number from elsewhere.
    #[test]
    fn every_flank_allowable_is_derived_and_none_is_borrowed() {
        use gear_core::material::{HardnessLine, QualityGrade};
        for m in &default_library().materials {
            assert!(
                m.contact_fatigue_allowable.is_none(),
                "{}: nothing is published",
                m.name
            );
            match &m.contact_estimate {
                Some(e) => {
                    assert_eq!(m.class, Family::Steel, "{}", m.name);
                    assert_eq!(e.line, HardnessLine::ThroughHardenedAlloySteel);
                    assert_eq!(e.grade, QualityGrade::Mq, "{}", m.name);
                    assert!(e.hardness.note.is_some(), "{}: hardness unsourced", m.name);
                    // The grade moves the figure and nothing else does.
                    let at = |grade| {
                        m.overridden(&gear_core::material::Overrides {
                            contact_grade: Some(grade),
                            ..Default::default()
                        })
                        .contact_fatigue_allowable
                        .unwrap()
                        .value
                    };
                    assert!(at(QualityGrade::Ml) < at(QualityGrade::Mq));
                    assert!(at(QualityGrade::Mq) < at(QualityGrade::Me));
                }
                None => assert_ne!(m.class, Family::Steel, "{}", m.name),
            }
        }
        // The figures the notes quote.
        let hardened = default_library();
        let hardened = hardened.get("4340 Hardened Steel").unwrap();
        let mq = hardened.flank_endurance().unwrap().value;
        assert!((mq - 845.68).abs() < 1e-9, "{mq}");
    }

    #[test]
    fn the_default_library_round_trips_through_toml_unchanged() {
        // The property the export/import feature rests on: what we write, we
        // read back identically. Hand-editing is the whole point of the format,
        // so a lossy write would corrupt a user's library silently.
        let lib = default_library();
        let text = to_toml(&lib).unwrap();
        let back = from_toml(&text).unwrap();
        assert_eq!(lib, back);
    }

    /// **Every shipped material is one the boundary admits** — the rows a
    /// file's are read against ([`MaterialLibrary::check`]) — and sits
    /// inside the band real engineering materials do: a Poisson's ratio
    /// from 0.2, and a cyclic allowable strictly below the static one. The
    /// band is a sanity check of this library's data, not a physical limit,
    /// so it is the test's and not the reader's.
    #[test]
    fn every_material_carries_physically_sane_values() {
        let lib = default_library();
        lib.check().unwrap();
        for m in &lib.materials {
            let name = &m.name;
            let nu = m.poissons_ratio.value;
            assert!((0.2..0.5).contains(&nu), "{name}: Poisson's ratio {nu}");
            assert!(
                m.fatigue_allowable.value < m.ultimate_allowable.value,
                "{name}: fatigue allowable is not below ultimate"
            );
        }
    }

    /// **A figure no material has is refused by its path in the file**, and
    /// the same figure given as a member's replacement is refused by its
    /// path in the train: for every shipped material and every figure, nought,
    /// minus one, not a number, and a Poisson's ratio at 0.7 — past the
    /// incompressible limit, where the shear modulus goes negative — each
    /// refused naming the field; the limit itself, ν = ½, admitted in both.
    #[test]
    fn a_figure_no_material_has_is_refused_by_its_field() {
        use gear_core::input::Refused;
        use gear_core::material::Overrides;
        use gear_core::train::arrangements as arr;
        use gear_core::train::{solve_train, Train, TrainError};
        type Figure = (
            &'static str,
            fn(&mut gear_core::Material) -> &mut f64,
            fn(&mut Overrides) -> &mut Option<f64>,
        );
        let figures: [Figure; 5] = [
            ("density", |m| &mut m.density.value, |o| &mut o.density),
            (
                "elastic_modulus",
                |m| &mut m.elastic_modulus.value,
                |o| &mut o.elastic_modulus,
            ),
            (
                "poissons_ratio",
                |m| &mut m.poissons_ratio.value,
                |o| &mut o.poissons_ratio,
            ),
            (
                "ultimate_allowable",
                |m| &mut m.ultimate_allowable.value,
                |o| &mut o.ultimate_allowable,
            ),
            (
                "fatigue_allowable",
                |m| &mut m.fatigue_allowable.value,
                |o| &mut o.fatigue_allowable,
            ),
        ];
        let shipped = default_library();
        let mut refused = 0;
        for (i, material) in shipped.materials.iter().enumerate() {
            for (name, in_library, in_overrides) in figures {
                let wrong: &[f64] = if name == "poissons_ratio" {
                    &[-1.0, f64::NAN, 0.7]
                } else {
                    &[0.0, -1.0, f64::NAN]
                };
                for &x in wrong {
                    // In the file.
                    let mut lib = shipped.clone();
                    *in_library(&mut lib.materials[i]) = x;
                    let text = to_toml(&lib).unwrap();
                    match from_toml(&text) {
                        Err(MaterialError::Implausible(Refused { field, .. })) => {
                            assert_eq!(field, format!("material.{i}.{name}.value"), "{x}");
                        }
                        other => panic!("{name} = {x}: refused by its field, not {other:?}"),
                    }
                    // As a member's replacement, on a pair of this material.
                    let mut s = arr::pair([17, 43]);
                    s.members[0].gear.material.clone_from(&material.name);
                    *in_overrides(&mut s.members[0].gear.material_overrides) = Some(x);
                    let t = Train::alone(&s, 2.0, 100.0);
                    match solve_train(&t, &shipped) {
                        Err(TrainError::Input(Refused { field, .. })) => assert_eq!(
                            field,
                            format!("shape.members.0.gear.material_overrides.{name}"),
                            "{x}"
                        ),
                        other => panic!("override {name} = {x}: refused, not {:?}", other.err()),
                    }
                    refused += 2;
                }
            }
            // The incompressible limit is a solid.
            let mut lib = shipped.clone();
            lib.materials[i].poissons_ratio.value = 0.5;
            assert!(from_toml(&to_toml(&lib).unwrap()).is_ok());
        }
        assert_eq!(refused, shipped.len() * figures.len() * 3 * 2);
    }

    /// Every entry describes one state, and says which. That is what replaced
    /// the old paired dry/conditioned numbers, and it is only honest if the
    /// condition is actually filled in.
    #[test]
    fn every_entry_names_the_condition_its_numbers_describe() {
        for m in &default_library().materials {
            assert!(!m.condition.is_empty(), "{}: no condition", m.name);
        }
        // The polyamides are the conditioned state, and say so.
        for name in ["PA6", "PA6 GF30", "PA GF50", "PA GF70"] {
            let m = default_library();
            let m = m.get(name).unwrap();
            assert!(
                m.condition.contains("conditioned"),
                "{name}: {}",
                m.condition
            );
            // ...and the dry figure it replaced is preserved rather than lost.
            assert!(
                m.elastic_modulus
                    .note
                    .as_deref()
                    .is_some_and(|n| n.contains("Dry as moulded")),
                "{name}: the dry modulus should survive in the note"
            );
        }
    }

    #[test]
    fn glass_filled_grades_report_break_and_ductile_grades_report_yield() {
        let lib = default_library();
        for name in ["4340 Steel", "Brass C360", "POM Delrin 100P", "PA6"] {
            assert_eq!(
                lib.get(name).unwrap().ultimate_measure,
                Measure::Yield,
                "{name}"
            );
        }
        for name in ["PA6 GF30", "PA GF50", "PA GF70"] {
            assert_eq!(
                lib.get(name).unwrap().ultimate_measure,
                Measure::Break,
                "{name}"
            );
        }
    }

    #[test]
    fn glass_content_raises_stiffness_and_density_across_the_family() {
        // A consistency check across the polyamide family rather than on any
        // one entry: these came from three separate datasheets, and if a column
        // were misread the monotonicity would break.
        let lib = default_library();
        let pa = ["PA6", "PA6 GF30", "PA GF50", "PA GF70"].map(|n| lib.get(n).unwrap());

        for w in pa.windows(2) {
            assert!(
                w[1].elastic_modulus.value > w[0].elastic_modulus.value,
                "{} is not stiffer than {}",
                w[1].name,
                w[0].name
            );
            assert!(
                w[1].density.value > w[0].density.value,
                "{} density",
                w[1].name
            );
        }
    }

    #[test]
    fn every_value_that_is_not_a_datasheet_reading_explains_itself() {
        // The discipline that makes an estimated library safe to ship: a number
        // that is not straight off a datasheet must say what it is.
        for m in &default_library().materials {
            for (prop, v) in [
                ("density", &m.density),
                ("elastic_modulus", &m.elastic_modulus),
                ("poissons_ratio", &m.poissons_ratio),
                ("ultimate_allowable", &m.ultimate_allowable),
                ("fatigue_allowable", &m.fatigue_allowable),
            ]
            .into_iter()
            .chain(
                m.contact_fatigue_allowable
                    .as_ref()
                    .map(|v| ("contact_fatigue_allowable", v)),
            )
            .chain(
                m.contact_estimate
                    .as_ref()
                    .map(|e| ("hardness", &e.hardness)),
            ) {
                if v.basis != Basis::Datasheet {
                    assert!(
                        v.note.is_some(),
                        "{}: {prop} has basis {} but no note",
                        m.name,
                        v.basis.as_str()
                    );
                }
            }
            assert!(!m.source.is_empty(), "{}: no source", m.name);
            assert!(!m.grade.is_empty(), "{}: no grade", m.name);
        }
    }

    #[test]
    fn the_polyamide_estimate_rule_the_file_states_is_the_rule_it_uses() {
        // The header of materials_default.toml promises that polyamide fatigue
        // allowables are a uniform 0.30 × ultimate. That uniformity is the only
        // thing making the entries comparable with each other, so a hand-edit
        // that breaks it should fail loudly rather than quietly produce a
        // library whose estimates no longer mean the same thing.
        let lib = default_library();
        for m in lib
            .materials
            .iter()
            .filter(|m| m.class == Family::Polyamide)
        {
            {
                let (ult, fat) = (m.ultimate_allowable.value, m.fatigue_allowable.value);
                let ratio = fat / ult;
                assert!(
                    (ratio - 0.30).abs() < 1e-9,
                    "{}: fatigue/ultimate is {ratio:.4}, not the stated 0.30",
                    m.name
                );
            }
            assert_eq!(m.fatigue_allowable.basis, Basis::Estimated, "{}", m.name);
        }
    }

    #[test]
    fn a_library_with_duplicate_names_is_rejected() {
        let one = r#"
            [[material]]
            name = "X"
            class = "steel"
            grade = "g"
            condition = "c"
            source = "s"
            density = { value = 7850.0, basis = "datasheet" }
            elastic_modulus = { value = 190000.0, basis = "datasheet" }
            poissons_ratio = { value = 0.29, basis = "datasheet" }
            ultimate_allowable = { value = 470.0, basis = "datasheet" }
            ultimate_measure = "yield"
            fatigue_allowable = { value = 330.0, basis = "datasheet" }
        "#;
        assert!(from_toml(one).is_ok());

        let two = format!("{one}\n{one}");
        assert!(matches!(
            from_toml(&two),
            Err(MaterialError::DuplicateName(n)) if n == "X"
        ));
    }

    #[test]
    fn an_empty_library_is_rejected_rather_than_returned() {
        assert!(matches!(from_toml(""), Err(MaterialError::Empty)));
    }
}
