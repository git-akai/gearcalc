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
//! reach the browser through the same door the defaults do — see docs/corrections.md for
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
    /// `stage.face_width_no_source`
    pub const STAGE_FACE_WIDTH_NO_SOURCE: &str = "stage.face_width_no_source";
    /// `stage.load_sharing_out_of_band`
    pub const STAGE_LOAD_SHARING_OUT_OF_BAND: &str = "stage.load_sharing_out_of_band";

    // ---- the train, as a whole ------------------------------------ //
    //
    // What a stage cannot say, because it is a fact about the shaft line: an
    // input clamped against the peak it is measured from, and where — or
    // whether — a back-driving load is reacted at all.
    /// `train.operating_torque_clamped`
    pub const TRAIN_OPERATING_TORQUE_CLAMPED: &str = "train.operating_torque_clamped";
    /// `train.operating_speed_clamped`
    pub const TRAIN_OPERATING_SPEED_CLAMPED: &str = "train.operating_speed_clamped";
    /// `train.back_driving_reacted_at`
    pub const TRAIN_BACK_DRIVING_REACTED_AT: &str = "train.back_driving_reacted_at";
    /// `train.back_driving_not_reacted`
    pub const TRAIN_BACK_DRIVING_NOT_REACTED: &str = "train.back_driving_not_reacted";

    /// Every key above, for the catalogue coverage tests.
    // ---- errors --------------------------------------------------- //
    //
    // The reason a result does not exist, said the same way a clamp says what
    // it altered. These were `Display` impls writing English inside
    // `gear-core` — the last place the no-words rule was broken, and broken
    // in the messages a user sees when something has gone wrong, which are
    // the ones that most need translating (`docs/corrections.md`).
    /// `error.mesh_incompatible`
    pub const ERROR_MESH_INCOMPATIBLE: &str = "error.mesh_incompatible";
    /// `error.mesh_ring_too_small`
    pub const ERROR_MESH_RING_TOO_SMALL: &str = "error.mesh_ring_too_small";
    /// `error.mesh_outside_involute_domain`
    pub const ERROR_MESH_OUTSIDE_INVOLUTE_DOMAIN: &str = "error.mesh_outside_involute_domain";
    /// `error.mesh_centre_distance_too_small`
    pub const ERROR_MESH_CENTRE_DISTANCE_TOO_SMALL: &str = "error.mesh_centre_distance_too_small";
    /// `error.measure_no_valid_span`
    pub const ERROR_MEASURE_NO_VALID_SPAN: &str = "error.measure_no_valid_span";
    /// `error.measure_pin_off_flank`
    pub const ERROR_MEASURE_PIN_OFF_FLANK: &str = "error.measure_pin_off_flank";
    /// `error.measure_pin_bottoms_out`
    pub const ERROR_MEASURE_PIN_BOTTOMS_OUT: &str = "error.measure_pin_bottoms_out";
    /// `error.measure_pin_too_small`
    pub const ERROR_MEASURE_PIN_TOO_SMALL: &str = "error.measure_pin_too_small";
    /// `error.measure_pin_too_large`
    pub const ERROR_MEASURE_PIN_TOO_LARGE: &str = "error.measure_pin_too_large";
    /// `error.screw_not_positive`
    pub const ERROR_SCREW_NOT_POSITIVE: &str = "error.screw_not_positive";
    /// `error.screw_worm_too_thin`
    pub const ERROR_SCREW_WORM_TOO_THIN: &str = "error.screw_worm_too_thin";
    /// `error.screw_shaft_angle_impossible`
    pub const ERROR_SCREW_SHAFT_ANGLE_IMPOSSIBLE: &str = "error.screw_shaft_angle_impossible";
    /// `error.screw_first_member_is_a_disc`
    pub const ERROR_SCREW_FIRST_MEMBER_IS_A_DISC: &str = "error.screw_first_member_is_a_disc";
    /// `error.screw_axes_are_parallel`
    pub const ERROR_SCREW_AXES_ARE_PARALLEL: &str = "error.screw_axes_are_parallel";
    /// `error.train_no_contact`
    pub const ERROR_TRAIN_NO_CONTACT: &str = "error.train_no_contact";
    /// `error.train_unknown_material`
    pub const ERROR_TRAIN_UNKNOWN_MATERIAL: &str = "error.train_unknown_material";
    /// `error.train_no_root_section`
    pub const ERROR_TRAIN_NO_ROOT_SECTION: &str = "error.train_no_root_section";
    /// `error.train_empty`
    pub const ERROR_TRAIN_EMPTY: &str = "error.train_empty";
    /// `error.gear_no_mate`
    pub const ERROR_GEAR_NO_MATE: &str = "error.gear_no_mate";
    /// `error.gear_concentric_has_no_profile`
    pub const ERROR_GEAR_CONCENTRIC_HAS_NO_PROFILE: &str = "error.gear_concentric_has_no_profile";
    /// `error.gear_no_pin_diameter`
    pub const ERROR_GEAR_NO_PIN_DIAMETER: &str = "error.gear_no_pin_diameter";

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
        CLAMP_TOOTH_UNDERCUT,
        CLAMP_TOOTH_THICKNESS_CAPPED,
        CLAMP_TOOTH_THICKNESS_RAISED,
        STAGE_CROSSED_CONTACT_RATIO_BELOW_ONE,
        STAGE_FACE_WIDTH_NO_SOURCE,
        TRAIN_OPERATING_TORQUE_CLAMPED,
        TRAIN_OPERATING_SPEED_CLAMPED,
        TRAIN_BACK_DRIVING_REACTED_AT,
        TRAIN_BACK_DRIVING_NOT_REACTED,
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
        STAGE_LOAD_SHARING_OUT_OF_BAND,
        ERROR_MESH_INCOMPATIBLE,
        ERROR_MESH_RING_TOO_SMALL,
        ERROR_MESH_OUTSIDE_INVOLUTE_DOMAIN,
        ERROR_MESH_CENTRE_DISTANCE_TOO_SMALL,
        ERROR_MEASURE_NO_VALID_SPAN,
        ERROR_MEASURE_PIN_OFF_FLANK,
        ERROR_MEASURE_PIN_BOTTOMS_OUT,
        ERROR_MEASURE_PIN_TOO_SMALL,
        ERROR_MEASURE_PIN_TOO_LARGE,
        ERROR_SCREW_NOT_POSITIVE,
        ERROR_SCREW_WORM_TOO_THIN,
        ERROR_SCREW_SHAFT_ANGLE_IMPOSSIBLE,
        ERROR_SCREW_FIRST_MEMBER_IS_A_DISC,
        ERROR_SCREW_AXES_ARE_PARALLEL,
        ERROR_TRAIN_NO_CONTACT,
        ERROR_TRAIN_UNKNOWN_MATERIAL,
        ERROR_TRAIN_NO_ROOT_SECTION,
        ERROR_TRAIN_EMPTY,
        ERROR_GEAR_NO_MATE,
        ERROR_GEAR_CONCENTRIC_HAS_NO_PROFILE,
        ERROR_GEAR_NO_PIN_DIAMETER,
    ];
}

/// One thing worth saying about a result, as a key and its values.
///
/// Render it with `gear_io::strings`, or in the browser from the catalogue
/// `gear_wasm::strings` serves.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
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

/// Something that can say why it happened, in the catalogue's currency.
///
/// Errors used to answer that question in English, through `Display`, from
/// inside a crate whose standing rule is that it holds no words
/// (`docs/corrections.md`). This is the same door [`Note`] already gave clamps
/// and stage notes: one channel for every reason a person reads, so the
/// catalogue check covers all of them and a translator has one file.
///
/// `Display` stays on each of these, for the CLI and for `Debug`. What changed
/// is which of the two the browser sees.
pub trait Explain {
    /// Why, as a key and the values its sentence needs.
    fn note(&self) -> Note;
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
