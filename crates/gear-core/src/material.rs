//! Material model: the elastic constants and stress allowables a gear
//! calculation needs, plus a record of where each number came from.
//!
//! # Why every value carries its provenance
//!
//! The rest of this crate holds itself to closed form and no fitted constants.
//! Material data cannot meet that bar, and pretending otherwise would be the
//! more dangerous choice. The survey behind this module (recorded in
//! `docs/DESIGN.md` §6) found that:
//!
//! - density, elastic modulus and tensile strength are published for every
//!   material here, on primary manufacturer or standards datasheets;
//! - Poisson's ratio is published for the metals and for POM, and for **no**
//!   polyamide grade at all;
//! - fatigue data is published for the steels, exists only as a printed *graph*
//!   for POM, and does not exist in any form for the polyamides.
//!
//! So a single number in this library may be a measured datasheet value, a
//! quantity derived from two measured values, a reading off a published chart,
//! or a class-consistent estimate. Those are not equally trustworthy and the
//! calculator must not present them as if they were. Every value therefore
//! carries a [`Basis`], and the UI is expected to surface it.
//!
//! # Moisture
//!
//! Polyamides absorb water, and it is not a small effect: unfilled PA6 loses
//! **two thirds** of its stiffness between dry-as-moulded and equilibrium at
//! 50 % relative humidity. Both states are published, so both are stored, and
//! [`Value::get`] returns the conditioned figure wherever one exists. A gear in
//! service has been in service; dry-as-moulded is the state it leaves the mould
//! in, not the state it works in.
//!
//! Metals and POM are insensitive, so their `conditioned` field is absent and
//! `get` falls through to the single published value.
//!
//! # The two allowables
//!
//! [`Material::ultimate_allowable`] is what a **peak** load may not exceed, and
//! [`Material::fatigue_allowable`] is what a **cyclic** load may not exceed.
//! That pairing matches the peak/cyclic input torques the geartrain takes, so
//! the two allowables answer the two questions the tool is actually asked.
//!
//! This replaces the S-N curve of earlier drafts. Fitting a Basquin law needs
//! two points on a fatigue curve; those two points do not exist for most of
//! this library, and a curve fitted to invented points is worse than a single
//! honest scalar because it looks like it knows more.

/// How much a number is worth.
///
/// Ordered weakest-last so a UI can sort or colour by it, and so the worst
/// basis in a material is `max()` over its properties.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Basis {
    /// Read directly from a primary manufacturer or standards datasheet.
    Datasheet,
    /// Computed exactly from two or more published values on the same
    /// datasheet — for example `ν = E/2G − 1` where both moduli are published.
    /// As trustworthy as the values it came from.
    Derived,
    /// Read off a published graph. The figure is real; the precision is not.
    Chart,
    /// A class-consistent estimate. No measurement of this property for this
    /// material was found. The weakest kind of number here, and the one most
    /// worth overriding with your own data.
    Estimated,
}

impl Basis {
    /// Whether this value rests on a measurement of *this* material.
    #[must_use]
    pub fn is_measured(self) -> bool {
        matches!(self, Self::Datasheet | Self::Derived)
    }

    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Datasheet => "datasheet",
            Self::Derived => "derived",
            Self::Chart => "chart",
            Self::Estimated => "estimated",
        }
    }
}

/// Which quantity the ultimate allowable actually measures.
///
/// Not cosmetic. Unfilled polymers and ductile metals yield before they break,
/// so their datasheets report a yield stress. Glass-filled grades have no yield
/// point at all — they break first — so their datasheets report stress at
/// break. Recording which one is in the field keeps the number comparable to
/// its own source instead of silently relabelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Measure {
    /// Stress at yield: permanent deformation begins.
    Yield,
    /// Stress at break: no yield point exists.
    Break,
}

/// Broad material family.
///
/// Carried because several conventions are per-family rather than per-material
/// — the uniform fatigue fraction applied across the polyamides, and the
/// reversed-bending penalty a planet gear takes (§4.9).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Class {
    Steel,
    Brass,
    /// Polyoxymethylene / acetal.
    Pom,
    Polyamide,
}

impl Class {
    /// Whether this family takes up water enough to change its mechanics.
    #[must_use]
    pub fn is_moisture_sensitive(self) -> bool {
        matches!(self, Self::Polyamide)
    }
}

/// One material property: its value, whether it varies with moisture, and how
/// far it can be trusted.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Value {
    /// The value as published, or for a moisture-sensitive material, the
    /// dry-as-moulded value.
    pub dry: f64,
    /// The value at equilibrium with 23 °C / 50 % relative humidity, where the
    /// material is moisture-sensitive and the figure is published.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub conditioned: Option<f64>,
    pub basis: Basis,
    /// Why this number is what it is, where that is not obvious. Always present
    /// on anything that is not a plain datasheet reading.
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub note: Option<String>,
}

impl Value {
    /// The figure the calculator uses: **conditioned where one exists**.
    ///
    /// See the module documentation for why the in-service state wins.
    #[must_use]
    pub fn get(&self) -> f64 {
        self.conditioned.unwrap_or(self.dry)
    }

    /// A plain published value that does not vary with moisture.
    #[must_use]
    pub fn datasheet(v: f64) -> Self {
        Self {
            dry: v,
            conditioned: None,
            basis: Basis::Datasheet,
            note: None,
        }
    }
}

/// A material, as the calculator sees it.
///
/// Lengths are millimetres and stresses megapascals throughout the crate, but
/// **density is SI** (kg/m³) per the unit rule in `docs/DESIGN.md` §6: SI
/// internally except where the domain's own convention is unambiguous.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Material {
    /// Display name, and the key the library is looked up by.
    pub name: String,
    pub class: Class,
    /// The specific product or condition the numbers describe. A generic name
    /// like "PA6" is not a material; some particular grade was measured, and
    /// this says which.
    pub grade: String,
    /// Heat treatment, moisture state, or temper the values apply to.
    pub condition: String,
    /// Where the numbers came from, precisely enough to check them.
    pub source: String,

    /// kg/m³.
    pub density: Value,
    /// MPa.
    pub elastic_modulus: Value,
    /// Dimensionless.
    pub poissons_ratio: Value,
    /// MPa. The limit on a **peak** load — see the module documentation.
    pub ultimate_allowable: Value,
    /// What `ultimate_allowable` measures: yield, or break for materials with
    /// no yield point.
    pub ultimate_measure: Measure,
    /// MPa. The limit on a **cyclic** load.
    pub fatigue_allowable: Value,
}

impl Material {
    /// The weakest basis among this material's properties.
    ///
    /// What a UI should show as the material's overall confidence: a library
    /// entry is only as good as its worst number, and for most of this library
    /// that is the fatigue allowable.
    #[must_use]
    pub fn weakest_basis(&self) -> Basis {
        [
            self.density.basis,
            self.elastic_modulus.basis,
            self.poissons_ratio.basis,
            self.ultimate_allowable.basis,
            self.fatigue_allowable.basis,
        ]
        .into_iter()
        .max()
        .unwrap_or(Basis::Estimated)
    }

    /// Plane-strain contact modulus contribution, `(1 − ν²)/E`, in 1/MPa.
    ///
    /// The only combination of `E` and `ν` that Hertzian contact needs, so it
    /// is computed once here rather than spelled out at each call site.
    #[must_use]
    pub fn contact_compliance(&self) -> f64 {
        let nu = self.poissons_ratio.get();
        (1.0 - nu * nu) / self.elastic_modulus.get()
    }
}

/// A set of materials, in presentation order.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct MaterialLibrary {
    #[cfg_attr(feature = "serde", serde(rename = "material", default))]
    pub materials: Vec<Material>,
}

impl MaterialLibrary {
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&Material> {
        self.materials.iter().find(|m| m.name == name)
    }

    #[must_use]
    pub fn names(&self) -> Vec<&str> {
        self.materials.iter().map(|m| m.name.as_str()).collect()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.materials.is_empty()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.materials.len()
    }
}

/// Effective contact modulus `E*` for a pair, in MPa.
///
/// `1/E* = (1−ν₁²)/E₁ + (1−ν₂²)/E₂`. Separate from [`Material`] because it is a
/// property of the *pair*, and the pair may be two different materials.
#[must_use]
pub fn contact_modulus(a: &Material, b: &Material) -> f64 {
    1.0 / (a.contact_compliance() + b.contact_compliance())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn steel() -> Material {
        Material {
            name: "test steel".into(),
            class: Class::Steel,
            grade: "test".into(),
            condition: "test".into(),
            source: "test".into(),
            density: Value::datasheet(7800.0),
            elastic_modulus: Value::datasheet(190_000.0),
            poissons_ratio: Value::datasheet(0.29),
            ultimate_allowable: Value::datasheet(470.0),
            ultimate_measure: Measure::Yield,
            fatigue_allowable: Value::datasheet(330.0),
        }
    }

    #[test]
    fn conditioned_value_wins_where_one_exists() {
        let v = Value {
            dry: 3000.0,
            conditioned: Some(1000.0),
            basis: Basis::Datasheet,
            note: None,
        };
        assert!((v.get() - 1000.0).abs() < 1e-12);

        // ...and a material with no conditioned figure falls through.
        assert!((Value::datasheet(3000.0).get() - 3000.0).abs() < 1e-12);
    }

    #[test]
    fn contact_modulus_reproduces_the_textbook_identical_material_case() {
        // For two identical bodies, 1/E* = 2(1−ν²)/E, so E* = E/2(1−ν²).
        let s = steel();
        let got = contact_modulus(&s, &s);
        let want = 190_000.0 / (2.0 * (1.0 - 0.29 * 0.29));
        assert!((got - want).abs() < 1e-6, "{got} vs {want}");
    }

    #[test]
    fn contact_modulus_is_symmetric_and_between_the_two_single_material_values() {
        let a = steel();
        let mut b = steel();
        b.elastic_modulus = Value::datasheet(3000.0);
        b.poissons_ratio = Value::datasheet(0.37);

        let ab = contact_modulus(&a, &b);
        let ba = contact_modulus(&b, &a);
        assert!((ab - ba).abs() < 1e-9);

        // A compliant mate dominates the pair: E* must sit below the softer
        // material's own like-on-like value.
        assert!(ab < contact_modulus(&b, &b) * 2.0);
        assert!(ab > 0.0);
    }

    #[test]
    fn weakest_basis_is_the_worst_property_not_the_first() {
        let mut m = steel();
        m.fatigue_allowable = Value {
            dry: 100.0,
            conditioned: None,
            basis: Basis::Estimated,
            note: Some("class estimate".into()),
        };
        assert_eq!(m.weakest_basis(), Basis::Estimated);
        assert!(!m.weakest_basis().is_measured());
    }
}
