//! What the solve wants a person to read, kept separate from the words it is
//! read in.
//!
//! # Why a key and not a sentence
//!
//! A note is generated where the physics is — "this pair loses contact between
//! teeth" is a fact about a contact ratio, and only the code that computed the
//! contact ratio knows to say it. But the *sentence* is not engineering, and
//! `gear-core` has no business holding English any more than it has business
//! holding CSS. So the core emits a [`Note`]: a stable key naming what happened,
//! and the values the sentence needs.
//!
//! The words live in `gear-io`'s string catalogue, one file per language, and
//! reach the browser through the same door the defaults do — see DESIGN §12 for
//! what happened the one time a value was written down in two languages.
//!
//! # Rounding is engineering, so the core still does it
//!
//! Values arrive **already formatted**. Whether a contact ratio is worth three
//! decimals or one is a judgement about what the number means, and it belongs
//! next to the model rather than in a translator's file — a locale may decide
//! how to write `1.234`, but not whether the fourth digit is worth printing.
//! [`Note::number`] takes the decimals explicitly for that reason: there is no
//! default, so the choice is made every time and can be read at the call site.
//!
//! # What this replaces, and what it fixed on the way
//!
//! `Vec<String>` everywhere, which had a second cost beyond the words: a
//! consumer that wanted to *act* on a note had nothing to match on but its text.
//! The planetary solve did exactly that — `clamps.iter().any(|c| c.contains("tip
//! radius raised"))` — which is a sentence doing a symbol's job, and would have
//! broken silently the first time anybody improved the wording or translated it.

use std::collections::BTreeMap;

/// Every message key the core can emit.
///
/// Constants rather than literals at the call sites, for two reasons. A typo in
/// a key is invisible at runtime — the catalogue has no message for it, so it
/// renders as the key itself, which looks like a deliberate identifier — and
/// `ALL` gives the string catalogue something exact to be checked against.
/// `gear-io` holds both halves of that check: every key here has a message, and
/// every message here has a key.
pub mod key {
    /// `clamp.cutter_no_tip_corner`
    pub const CLAMP_CUTTER_NO_TIP_CORNER: &str = "clamp.cutter_no_tip_corner";
    /// `clamp.cutter_teeth_reduced`
    pub const CLAMP_CUTTER_TEETH_REDUCED: &str = "clamp.cutter_teeth_reduced";
    /// `clamp.dedendum_capped`
    pub const CLAMP_DEDENDUM_CAPPED: &str = "clamp.dedendum_capped";
    /// `clamp.dedendum_raised`
    pub const CLAMP_DEDENDUM_RAISED: &str = "clamp.dedendum_raised";
    /// `clamp.fillet_capped`
    pub const CLAMP_FILLET_CAPPED: &str = "clamp.fillet_capped";
    /// `clamp.pressure_angle_raised`
    pub const CLAMP_PRESSURE_ANGLE_RAISED: &str = "clamp.pressure_angle_raised";
    /// `clamp.ring_flank_fillet_gap`
    pub const CLAMP_RING_FLANK_FILLET_GAP: &str = "clamp.ring_flank_fillet_gap";
    /// `clamp.ring_flank_ungenerated`
    pub const CLAMP_RING_FLANK_UNGENERATED: &str = "clamp.ring_flank_ungenerated";
    /// `clamp.ring_fully_filleted`
    pub const CLAMP_RING_FULLY_FILLETED: &str = "clamp.ring_fully_filleted";
    /// `clamp.ring_space_capped`
    pub const CLAMP_RING_SPACE_CAPPED: &str = "clamp.ring_space_capped";
    /// `clamp.ring_space_raised`
    pub const CLAMP_RING_SPACE_RAISED: &str = "clamp.ring_space_raised";
    /// `clamp.ring_tip_raised`
    pub const CLAMP_RING_TIP_RAISED: &str = "clamp.ring_tip_raised";
    /// `clamp.tip_capped_pointed`
    pub const CLAMP_TIP_CAPPED_POINTED: &str = "clamp.tip_capped_pointed";

    /// `clamp.tooth_severed`
    pub const CLAMP_TOOTH_SEVERED: &str = "clamp.tooth_severed";
    /// `clamp.tooth_undercut`
    pub const CLAMP_TOOTH_UNDERCUT: &str = "clamp.tooth_undercut";
    /// `clamp.tooth_thickness_capped`
    pub const CLAMP_TOOTH_THICKNESS_CAPPED: &str = "clamp.tooth_thickness_capped";
    /// `clamp.tooth_thickness_raised`
    pub const CLAMP_TOOTH_THICKNESS_RAISED: &str = "clamp.tooth_thickness_raised";
    /// `stage.crossed_contact_ratio_below_one`
    pub const STAGE_CROSSED_CONTACT_RATIO_BELOW_ONE: &str = "stage.crossed_contact_ratio_below_one";
    /// `stage.crossed_face_width_as_entered`
    pub const STAGE_CROSSED_FACE_WIDTH_AS_ENTERED: &str = "stage.crossed_face_width_as_entered";
    /// `stage.low_mesh_efficiency`
    pub const STAGE_LOW_MESH_EFFICIENCY: &str = "stage.low_mesh_efficiency";
    /// `stage.near_self_locking`
    pub const STAGE_NEAR_SELF_LOCKING: &str = "stage.near_self_locking";
    /// `stage.overlap_below_one`
    pub const STAGE_OVERLAP_BELOW_ONE: &str = "stage.overlap_below_one";
    /// `stage.planet_clearance_below_minimum`
    pub const STAGE_PLANET_CLEARANCE_BELOW_MINIMUM: &str = "stage.planet_clearance_below_minimum";
    /// `stage.planets_not_evenly_spaced`
    pub const STAGE_PLANETS_NOT_EVENLY_SPACED: &str = "stage.planets_not_evenly_spaced";
    /// `stage.planets_share_load_equally`
    pub const STAGE_PLANETS_SHARE_LOAD_EQUALLY: &str = "stage.planets_share_load_equally";
    /// `stage.proportions_not_applicable`
    pub const STAGE_PROPORTIONS_NOT_APPLICABLE: &str = "stage.proportions_not_applicable";
    /// `stage.ring_addendum_clamped`
    pub const STAGE_RING_ADDENDUM_CLAMPED: &str = "stage.ring_addendum_clamped";
    /// `stage.self_locking`
    pub const STAGE_SELF_LOCKING: &str = "stage.self_locking";
    /// `stage.transverse_contact_ratio_below_one`
    pub const STAGE_TRANSVERSE_CONTACT_RATIO_BELOW_ONE: &str =
        "stage.transverse_contact_ratio_below_one";

    /// Every key above, for the catalogue coverage tests.
    pub const ALL: &[&str] = &[
        CLAMP_CUTTER_NO_TIP_CORNER,
        CLAMP_CUTTER_TEETH_REDUCED,
        CLAMP_DEDENDUM_CAPPED,
        CLAMP_DEDENDUM_RAISED,
        CLAMP_FILLET_CAPPED,
        CLAMP_PRESSURE_ANGLE_RAISED,
        CLAMP_RING_FLANK_FILLET_GAP,
        CLAMP_RING_FLANK_UNGENERATED,
        CLAMP_RING_FULLY_FILLETED,
        CLAMP_RING_SPACE_CAPPED,
        CLAMP_RING_SPACE_RAISED,
        CLAMP_RING_TIP_RAISED,
        CLAMP_TIP_CAPPED_POINTED,
        CLAMP_TOOTH_SEVERED,
        CLAMP_TOOTH_SEVERED,
        CLAMP_TOOTH_UNDERCUT,
        CLAMP_TOOTH_THICKNESS_CAPPED,
        CLAMP_TOOTH_THICKNESS_RAISED,
        STAGE_CROSSED_CONTACT_RATIO_BELOW_ONE,
        STAGE_CROSSED_FACE_WIDTH_AS_ENTERED,
        STAGE_LOW_MESH_EFFICIENCY,
        STAGE_NEAR_SELF_LOCKING,
        STAGE_OVERLAP_BELOW_ONE,
        STAGE_PLANET_CLEARANCE_BELOW_MINIMUM,
        STAGE_PLANETS_NOT_EVENLY_SPACED,
        STAGE_PLANETS_SHARE_LOAD_EQUALLY,
        STAGE_PROPORTIONS_NOT_APPLICABLE,
        STAGE_RING_ADDENDUM_CLAMPED,
        STAGE_SELF_LOCKING,
        STAGE_TRANSVERSE_CONTACT_RATIO_BELOW_ONE,
    ];
}

/// One thing worth saying about a result, as a key and its values.
///
/// Render it with `gear_io::strings`, or in the browser from the catalogue
/// `gear_wasm::strings` serves.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Note {
    /// Stable identifier for the message. Never shown to anyone.
    pub key: String,
    /// Values the message interpolates, already formatted. `BTreeMap` so the
    /// serialised form is stable and two equal notes compare equal.
    pub values: BTreeMap<String, String>,
}

impl Note {
    /// A note with no values.
    #[must_use]
    pub fn new(key: &str) -> Self {
        Self {
            key: key.to_string(),
            values: BTreeMap::new(),
        }
    }

    /// Add a number, formatted to `decimals` places.
    ///
    /// The decimals are not optional and have no default: how precisely a
    /// quantity deserves to be quoted is part of what the quantity means.
    #[must_use]
    pub fn number(mut self, name: &str, value: f64, decimals: usize) -> Self {
        self.values
            .insert(name.to_string(), format!("{value:.decimals$}"));
        self
    }

    /// Add an integer.
    #[must_use]
    pub fn count(mut self, name: &str, value: u32) -> Self {
        self.values.insert(name.to_string(), value.to_string());
        self
    }

    /// Add a value that is already text — a material name, a member's number.
    #[must_use]
    pub fn text(mut self, name: &str, value: impl Into<String>) -> Self {
        self.values.insert(name.to_string(), value.into());
        self
    }

    /// Whether this is the named message, whatever language it is read in.
    ///
    /// The reason the key exists at all: consumers that need to *act* on a note
    /// ask this instead of searching its text.
    #[must_use]
    pub fn is(&self, key: &str) -> bool {
        self.key == key
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Rounding stays in the core, and the call site names the precision.
    #[test]
    fn a_number_is_formatted_where_it_is_computed() {
        let n = Note::new("x").number("ratio", 1.23456, 3);
        assert_eq!(n.values["ratio"], "1.235");
        assert_eq!(Note::new("x").number("r", 1.23456, 1).values["r"], "1.2");
        // Zero decimals is a choice, not an accident, and survives as one.
        assert_eq!(Note::new("x").number("r", 1.6, 0).values["r"], "2");
    }

    /// The point of the key: a consumer matches on it, never on words.
    #[test]
    fn a_note_is_recognised_by_key_rather_than_by_its_words() {
        let n = Note::new("ring_addendum_clamped").count("teeth", 40);
        assert!(n.is("ring_addendum_clamped"));
        assert!(!n.is("ring_addendum_clamp"));
    }
}
