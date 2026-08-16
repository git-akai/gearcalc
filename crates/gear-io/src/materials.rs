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
        }
    }
}

impl std::error::Error for MaterialError {}

/// Parse a material library from TOML.
///
/// Rejects an empty library and duplicate names rather than accepting them:
/// both would surface much later as a material that cannot be selected, or one
/// that silently shadows another.
pub fn from_toml(src: &str) -> Result<MaterialLibrary, MaterialError> {
    let lib: MaterialLibrary = toml::from_str(src).map_err(MaterialError::Parse)?;

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
pub fn default_library() -> MaterialLibrary {
    from_toml(DEFAULT_TOML).expect("compiled-in default material library must parse")
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use gear_core::material::{Basis, Class, Measure};

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

    #[test]
    fn every_material_carries_physically_sane_values() {
        for m in &default_library().materials {
            let name = &m.name;
            assert!(m.density.get() > 0.0, "{name}: density");
            assert!(m.elastic_modulus.get() > 0.0, "{name}: modulus");

            // Outside (-1, 0.5) a material has a negative bulk or shear
            // modulus. Real engineering materials sit well inside that.
            let nu = m.poissons_ratio.get();
            assert!((0.2..0.5).contains(&nu), "{name}: Poisson's ratio {nu}");

            // A cyclic allowable at or above the peak allowable would mean
            // fatigue never governs, which is never true.
            assert!(
                m.fatigue_allowable.get() < m.ultimate_allowable.get(),
                "{name}: fatigue allowable is not below ultimate"
            );
            assert!(m.fatigue_allowable.get() > 0.0, "{name}: fatigue allowable");
        }
    }

    #[test]
    fn moisture_sensitivity_matches_the_material_class() {
        for m in &default_library().materials {
            // Only polyamides should carry a second moisture state, and every
            // polyamide should carry one for modulus and strength. Getting this
            // backwards would silently apply dry values to a gear in service.
            let expected = m.class.is_moisture_sensitive();
            assert_eq!(
                m.elastic_modulus.conditioned.is_some(),
                expected,
                "{}: modulus moisture state",
                m.name
            );
            assert_eq!(
                m.ultimate_allowable.conditioned.is_some(),
                expected,
                "{}: strength moisture state",
                m.name
            );

            if expected {
                // Water plasticises: conditioned must be the weaker state.
                assert!(
                    m.elastic_modulus.conditioned.unwrap() < m.elastic_modulus.dry,
                    "{}: conditioned modulus is not below dry",
                    m.name
                );
                assert!(m.class == Class::Polyamide);
            }
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
    fn glass_content_raises_stiffness_and_closes_the_moisture_gap() {
        // A consistency check across the polyamide family rather than on any
        // one entry: these came from three separate datasheets, and if a column
        // were misread the monotonicity would break.
        let lib = default_library();
        let pa = ["PA6", "PA6 GF30", "PA GF50", "PA GF70"].map(|n| lib.get(n).unwrap());

        for w in pa.windows(2) {
            assert!(
                w[1].elastic_modulus.dry > w[0].elastic_modulus.dry,
                "{} is not stiffer than {}",
                w[1].name,
                w[0].name
            );
            assert!(w[1].density.dry > w[0].density.dry, "{} density", w[1].name);

            // More glass, less polymer, less water uptake.
            let gap = |m: &gear_core::Material| {
                1.0 - m.elastic_modulus.conditioned.unwrap() / m.elastic_modulus.dry
            };
            assert!(gap(w[1]) < gap(w[0]), "{} moisture gap", w[1].name);
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
            ] {
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
        for m in lib.materials.iter().filter(|m| m.class == Class::Polyamide) {
            for (state, ult, fat) in [
                ("dry", m.ultimate_allowable.dry, m.fatigue_allowable.dry),
                (
                    "conditioned",
                    m.ultimate_allowable.conditioned.unwrap(),
                    m.fatigue_allowable.conditioned.unwrap(),
                ),
            ] {
                let ratio = fat / ult;
                assert!(
                    (ratio - 0.30).abs() < 1e-9,
                    "{} {state}: fatigue/ultimate is {ratio:.4}, not the stated 0.30",
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
            density = { dry = 7850.0, basis = "datasheet" }
            elastic_modulus = { dry = 190000.0, basis = "datasheet" }
            poissons_ratio = { dry = 0.29, basis = "datasheet" }
            ultimate_allowable = { dry = 470.0, basis = "datasheet" }
            ultimate_measure = "yield"
            fatigue_allowable = { dry = 330.0, basis = "datasheet" }
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
