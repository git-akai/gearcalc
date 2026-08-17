//! The WebAssembly boundary.
//!
//! Deliberately thin. Because outputs are a pure function of inputs, the whole
//! surface is a handful of `JSON in -> JSON out` calls: there is no state to
//! synchronise, no lifecycle and no callbacks.
//!
//! **No engineering calculation belongs on the other side of this boundary.**
//! TypeScript formats numbers for display; every number it formats came from
//! here. That rule is what keeps the Rust test suite meaningful — otherwise
//! logic migrates into the view layer, where nothing tests it.
//!
//! Anything that can fail returns a *reason* rather than a number. A span that
//! cannot be measured, a pin that bottoms out, a tolerance class the standard
//! does not cover — each comes back as an explanation the UI can show, because
//! a plausible-looking number for an impossible measurement is worse than none.

use gear_core::jgma;
use gear_core::metrology::{self, PinCount};
use gear_core::{Gear, GearParams};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Everything the UI asks about one gear.
#[derive(Deserialize)]
pub struct GearRequest {
    pub params: GearParams,
    /// Pin or ball diameter for the over-pins measurement, mm.
    #[serde(default)]
    pub pin_diameter: Option<f64>,
    /// Tolerance class, as `{ "scale": "fine" | "standard", "grade": n }`.
    #[serde(default)]
    pub tolerance_class: Option<ClassRef>,
    /// Maximum deviation of the exported outline from the true curve, mm.
    #[serde(default)]
    pub chord_tolerance: Option<f64>,
    /// Include the pitch, base, tip and root circles in the DXF.
    #[serde(default = "yes")]
    pub reference_circles: bool,
    /// Depth, in modules, at which the undercut question is asked.
    #[serde(default = "one")]
    pub working_depth: f64,
}

fn one() -> f64 {
    1.0
}

fn yes() -> bool {
    true
}

#[derive(Clone, Deserialize, Serialize)]
pub struct ClassRef {
    pub scale: String,
    pub grade: u8,
}

impl ClassRef {
    fn to_class(&self) -> Option<jgma::Class> {
        let scale = match self.scale.as_str() {
            "fine" => jgma::Scale::Fine,
            "standard" => jgma::Scale::Standard,
            _ => return None,
        };
        Some(jgma::Class {
            scale,
            grade: self.grade,
        })
    }

    fn from_class(c: jgma::Class) -> Self {
        Self {
            scale: c.scale.as_str().to_string(),
            grade: c.grade,
        }
    }
}

/// A value, or the reason there is not one.
#[derive(Serialize)]
#[serde(untagged)]
pub enum Maybe<T> {
    Value(T),
    Unavailable { unavailable: String },
}

impl<T> Maybe<T> {
    fn from<E: std::fmt::Display>(r: Result<T, E>) -> Self {
        match r {
            Ok(v) => Self::Value(v),
            Err(e) => Self::Unavailable {
                unavailable: e.to_string(),
            },
        }
    }
}

#[derive(Serialize)]
pub struct SpanOut {
    pub teeth_spanned: u32,
    pub nominal: f64,
    pub contact_radius: f64,
}

#[derive(Serialize)]
pub struct PinsOut {
    pub nominal: f64,
    pub pin_centre_radius: f64,
    pub contact_radius: f64,
}

#[derive(Serialize)]
pub struct ToleranceOut {
    pub class: ClassRef,
    pub tooth_to_tooth: f64,
    pub total: f64,
}

/// The profile shifts this gear can be built at, and the thresholds inside them.
///
/// Sent so the UI can validate against the *real* bound rather than the
/// specification's fixed `|x| ≤ 2`, which is loose above and both loose and
/// tight below depending on pressure angle — see
/// [`gear_core::auto::admissible_profile_shift`]. The project rule applies here
/// as much as anywhere: the bounds are computed in Rust and merely compared in
/// TypeScript.
#[derive(Serialize)]
pub struct ShiftRangeOut {
    pub min: f64,
    pub max: f64,
    pub undercut: f64,
    pub sharp_rack_undercut: f64,
    pub pointed: Option<f64>,
}

/// A bound the geometry imposes, either side absent where it is unbounded.
#[derive(Serialize)]
pub struct BoundOut {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

/// Every input range this gear's own geometry decides.
///
/// Sent so the UI can bound a field by what can exist rather than by
/// convention — see `gear_core::auto::admissible_ranges`. The fields absent here
/// are the ones whose bounds do not vary (`m > 0`, `z ≥ 1`, `0 < α < 90°`,
/// `|β| < 90°`, `0 < k < 2`), which the UI carries as constants.
#[derive(Serialize)]
pub struct RangesOut {
    pub profile_shift: ShiftRangeOut,
    pub addendum: BoundOut,
    pub dedendum: BoundOut,
    pub root_radius: BoundOut,
}

/// Derived geometry and metrology for one gear.
///
/// Lengths are millimetres, angles degrees, composite errors micrometres.
#[derive(Serialize)]
pub struct GearSummary {
    pub ranges: RangesOut,
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    pub tooth_thickness: f64,
    pub fillet_radius: f64,
    pub transverse_pressure_angle: f64,
    pub cutter_tip_width: f64,
    pub undercut: bool,
    pub severed: bool,
    /// Guards that altered the requested geometry. Empty is the normal case.
    pub clamps: Vec<String>,

    pub span: Maybe<SpanOut>,
    pub over_two_pins: Maybe<PinsOut>,
    pub over_three_pins: Maybe<PinsOut>,

    /// Classes the standard actually covers for this gear.
    pub available_classes: Vec<ClassRef>,
    pub tolerance: Maybe<ToleranceOut>,
}

fn summarise(g: &Gear, req: &GearRequest) -> GearSummary {
    let pitch_diameter = 2.0 * g.r;
    let available = jgma::available_classes(g.params.module, pitch_diameter);

    let chosen = req
        .tolerance_class
        .as_ref()
        .and_then(ClassRef::to_class)
        .or_else(|| jgma::default_class(g.params.module, pitch_diameter));

    let tolerance = match chosen {
        Some(c) => match jgma::lookup(c, g.params.module, pitch_diameter) {
            Some(e) => Maybe::Value(ToleranceOut {
                class: ClassRef::from_class(c),
                tooth_to_tooth: e.tooth_to_tooth,
                total: e.total,
            }),
            None => Maybe::Unavailable {
                unavailable: format!(
                    "JGMA 116-02 has no {c} entry for module {} at {pitch_diameter:.3} mm",
                    g.params.module
                ),
            },
        },
        None => Maybe::Unavailable {
            unavailable: format!(
                "JGMA 116-02 does not cover module {} at {pitch_diameter:.3} mm",
                g.params.module
            ),
        },
    };

    let pins = |count| match req.pin_diameter {
        Some(d) => Maybe::from(metrology::over_pins(g, d, count).map(|p| PinsOut {
            nominal: p.nominal,
            pin_centre_radius: p.pin_centre_radius,
            contact_radius: p.contact_radius,
        })),
        None => Maybe::Unavailable {
            unavailable: "no pin diameter given".to_string(),
        },
    };

    let rg = gear_core::auto::admissible_ranges(&g.params, req.working_depth);
    let bound = |b: gear_core::auto::Bound| BoundOut {
        min: b.min,
        max: b.max,
    };

    GearSummary {
        ranges: RangesOut {
            profile_shift: ShiftRangeOut {
                min: rg.profile_shift.min,
                max: rg.profile_shift.max,
                undercut: rg.profile_shift.undercut,
                sharp_rack_undercut: rg.profile_shift.sharp_rack_undercut,
                pointed: rg.profile_shift.pointed,
            },
            addendum: bound(rg.addendum),
            dedendum: bound(rg.dedendum),
            root_radius: bound(rg.root_radius),
        },
        pitch_radius: g.r,
        base_radius: g.rb,
        tip_radius: g.ra,
        root_radius: g.rf,
        tooth_thickness: g.st,
        fillet_radius: g.rho,
        transverse_pressure_angle: g.alpha_t.to_degrees(),
        cutter_tip_width: metrology::cutter_tip_width(g),
        undercut: g.undercut,
        severed: g.severed,
        clamps: g.clamps.notes.clone(),
        span: Maybe::from(metrology::best_span(g).map(|s| SpanOut {
            teeth_spanned: s.teeth_spanned,
            nominal: s.nominal,
            contact_radius: s.contact_radius,
        })),
        over_two_pins: pins(PinCount::Two),
        over_three_pins: pins(PinCount::Three),
        available_classes: available.into_iter().map(ClassRef::from_class).collect(),
        tolerance,
    }
}

// The work lives in plain-Rust functions so it is testable on the host.
// `JsError` cannot even be constructed off a wasm target, so wrapping the logic
// in it directly would make the error paths untestable — which is exactly where
// a calculation engine most needs tests.

fn parse(input: &str) -> Result<GearRequest, String> {
    serde_json::from_str(input).map_err(|e| format!("bad gear request: {e}"))
}

fn solve_gear_impl(input: &str) -> Result<String, String> {
    let req = parse(input)?;
    let g = Gear::new(req.params);
    serde_json::to_string(&summarise(&g, &req)).map_err(|e| format!("could not encode result: {e}"))
}

fn gear_profile_impl(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, String> {
    let req = parse(input)?;
    Ok(Gear::new(req.params)
        .profile(points_per_tooth)
        .into_iter()
        .flat_map(|p| [p[0], p[1]])
        .collect())
}

fn export_dxf_impl(input: &str) -> Result<String, String> {
    let req = parse(input)?;
    let g = Gear::new(req.params);
    Ok(gear_io::gear_to_dxf(
        &g,
        &gear_io::DxfOptions {
            chord_tolerance: req
                .chord_tolerance
                .unwrap_or(gear_core::outline::DEFAULT_CHORD_TOLERANCE),
            reference_circles: req.reference_circles,
        },
    ))
}

/// Derived geometry and metrology for one gear.
#[wasm_bindgen]
pub fn solve_gear(input: &str) -> Result<String, JsError> {
    solve_gear_impl(input).map_err(|e| JsError::new(&e))
}

/// The closed cross-section as a flat `[x0, y0, x1, y1, ...]` array, ready for
/// a canvas path. Flat rather than nested to keep the crossing cheap.
#[wasm_bindgen]
pub fn gear_profile(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, JsError> {
    gear_profile_impl(input, points_per_tooth).map_err(|e| JsError::new(&e))
}

/// The gear as a DXF drawing, ready to be handed to the browser as a download.
#[wasm_bindgen]
pub fn export_dxf(input: &str) -> Result<String, JsError> {
    export_dxf_impl(input).map_err(|e| JsError::new(&e))
}

/// A geartrain, plus optionally the material library to rate it against.
#[derive(Deserialize)]
pub struct TrainRequest {
    pub train: gear_core::train::Train,
    /// The library to use. Omitted means the one the tool ships with, which is
    /// the common case — the UI only sends this once the user has imported or
    /// edited a library of their own.
    #[serde(default)]
    pub materials: Option<gear_core::MaterialLibrary>,
}

fn solve_train_impl(input: &str) -> Result<String, String> {
    let req: TrainRequest =
        serde_json::from_str(input).map_err(|e| format!("bad train request: {e}"))?;
    let lib = req.materials.unwrap_or_else(gear_io::default_library);
    let out = gear_core::train::solve_train(&req.train, &lib).map_err(|e| e.to_string())?;
    serde_json::to_string(&out).map_err(|e| format!("could not encode result: {e}"))
}

fn default_materials_impl() -> Result<String, String> {
    serde_json::to_string(&gear_io::default_library()).map_err(|e| e.to_string())
}

fn import_materials_impl(toml_text: &str) -> Result<String, String> {
    let lib = gear_io::from_toml(toml_text).map_err(|e| e.to_string())?;
    serde_json::to_string(&lib).map_err(|e| e.to_string())
}

fn export_materials_impl(library_json: &str) -> Result<String, String> {
    let lib: gear_core::MaterialLibrary =
        serde_json::from_str(library_json).map_err(|e| e.to_string())?;
    gear_io::to_toml(&lib).map_err(|e| e.to_string())
}

/// Derived results for a whole geartrain.
///
/// The third of the three entry points DESIGN.md §1 planned. Like the others it
/// is JSON in, JSON out, with no state held across the boundary: the UI owns the
/// inputs and this recomputes everything from them on each change.
#[wasm_bindgen]
pub fn solve_train(input: &str) -> Result<String, JsError> {
    solve_train_impl(input).map_err(|e| JsError::new(&e))
}

/// The material library the tool ships with, as JSON.
///
/// Includes each value's `basis` and `note`, because the UI is expected to show
/// which numbers are measured and which are estimates — see `docs/DESIGN.md`
/// §6.1. Dropping that on the floor would present a class estimate with the
/// same authority as a datasheet reading.
#[wasm_bindgen]
pub fn default_materials() -> Result<String, JsError> {
    default_materials_impl().map_err(|e| JsError::new(&e))
}

/// Import a material library: TOML text in, JSON out.
///
/// The TOML never reaches TypeScript — the browser reads a file as text and
/// hands it straight here, so exactly one parser exists and it is the tested
/// one. A malformed library returns the parser's own complaint, which names the
/// line, rather than a generic failure.
#[wasm_bindgen]
pub fn import_materials(toml_text: &str) -> Result<String, JsError> {
    import_materials_impl(toml_text).map_err(|e| JsError::new(&e))
}

/// Export a material library: JSON in, TOML text out, ready for a download.
#[wasm_bindgen]
pub fn export_materials(library_json: &str) -> Result<String, JsError> {
    export_materials_impl(library_json).map_err(|e| JsError::new(&e))
}

/// Version of the core, so the UI can show what it is actually running.
#[wasm_bindgen]
#[must_use]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    const REQ: &str = r#"{"params":{"module":1.0,"pressure_angle":20.0,"teeth":17,
        "profile_shift":0.2,"helix_angle":0.0,"addendum":1.0,"dedendum":1.25,
        "root_radius":0.38,"thickness_mod":1.0},"pin_diameter":1.75}"#;

    #[test]
    fn round_trips_json() {
        let v: serde_json::Value = serde_json::from_str(&solve_gear_impl(REQ).unwrap()).unwrap();
        assert!((v["pitch_radius"].as_f64().unwrap() - 8.5).abs() < 1e-12);
        assert_eq!(v["undercut"].as_bool(), Some(false));
        // metrology came through
        assert!(v["span"]["nominal"].as_f64().unwrap() > 0.0);
        assert!(v["over_two_pins"]["nominal"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn unavailable_measurements_explain_themselves() {
        // no pin diameter given
        let no_pin = r#"{"params":{"module":1.0,"pressure_angle":20.0,"teeth":17,
            "profile_shift":0.0,"helix_angle":0.0,"addendum":1.0,"dedendum":1.25,
            "root_radius":0.38,"thickness_mod":1.0}}"#;
        let v: serde_json::Value = serde_json::from_str(&solve_gear_impl(no_pin).unwrap()).unwrap();
        assert!(v["over_two_pins"]["unavailable"].is_string());

        // a pin that cannot measure this gear says why
        let bad_pin = REQ.replace("1.75", "0.05");
        let v: serde_json::Value =
            serde_json::from_str(&solve_gear_impl(&bad_pin).unwrap()).unwrap();
        let msg = v["over_two_pins"]["unavailable"].as_str().unwrap();
        assert!(msg.contains("too small"), "unhelpful message: {msg}");
    }

    #[test]
    fn tolerance_class_defaults_and_lists_what_is_available() {
        let v: serde_json::Value = serde_json::from_str(&solve_gear_impl(REQ).unwrap()).unwrap();
        // module 1, d = 17 mm: both scales apply, fine grade 0 is the default
        assert_eq!(v["tolerance"]["class"]["scale"], "fine");
        assert_eq!(v["tolerance"]["class"]["grade"], 0);
        assert!(v["tolerance"]["tooth_to_tooth"].as_f64().unwrap() > 0.0);
        let classes = v["available_classes"].as_array().unwrap();
        assert!(classes.iter().any(|c| c["scale"] == "standard"));
    }

    #[test]
    fn rejects_malformed_input_instead_of_panicking() {
        assert!(solve_gear_impl("{ not json").is_err());
        assert!(solve_gear_impl(r#"{"params":{"module":1.0}}"#).is_err());
        assert!(export_dxf_impl("{}").is_err());
    }

    #[test]
    fn profile_is_flat_pairs() {
        let v = gear_profile_impl(REQ, 200).unwrap();
        assert!(v.len().is_multiple_of(2) && v.len() > 100);
    }

    #[test]
    fn the_material_library_crosses_the_boundary_with_its_provenance_intact() {
        let json = default_materials_impl().unwrap();
        let v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let materials = v["material"].as_array().unwrap();
        assert_eq!(materials.len(), 8);

        let pa6 = materials.iter().find(|m| m["name"] == "PA6").unwrap();
        // The conditioned value must survive: it is the one the tool uses.
        assert!((pa6["elastic_modulus"]["conditioned"].as_f64().unwrap() - 1000.0).abs() < 1e-9);
        // ...and so must the honesty about where a number came from.
        assert_eq!(pa6["poissons_ratio"]["basis"], "estimated");
        assert!(pa6["poissons_ratio"]["note"].is_string());
        assert_eq!(pa6["elastic_modulus"]["basis"], "datasheet");
    }

    #[test]
    fn a_material_library_survives_export_and_reimport() {
        // The user-facing loop: ship defaults, export to a file, edit, import.
        let original = default_materials_impl().unwrap();
        let toml_text = export_materials_impl(&original).unwrap();
        let reimported = import_materials_impl(&toml_text).unwrap();

        let a: serde_json::Value = serde_json::from_str(&original).unwrap();
        let b: serde_json::Value = serde_json::from_str(&reimported).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn a_two_stage_train_crosses_the_boundary() {
        // The shape the UI will send: a train, and no library, meaning "use the
        // one you ship with".
        let req = r#"{"train":{
            "input_speed": 3000.0,
            "input_torque": 2.0,
            "actuation": { "continuous": { "operating_percent": 80.0, "runtime_hours": 1000.0 } },
            "stages": [
              {"module":1.0,"pressure_angle":20.0,"helix_angle":0.0,"friction":0.06,
               "thickness_mod":1.0,
               "centre_distance":{"auto":true,"manual":0.0},
               "clearance":0.02,"tolerance_plus":0.02,"tolerance_minus":0.02,
               "gears":[
                 {"teeth":17,"profile_shift":{"auto":true,"manual":0.0},"working_depth":1.0,
                  "addendum":{"auto":false,"manual":1.0},"min_tip_width":0.1,
                  "dedendum":1.25,"root_radius":0.38,
                  "face_width":{"auto":true,"manual":0.0},
                  "auto_face_from_bending":true,"auto_face_from_contact":true,
                  "material":"4340 Hardened Steel"},
                 {"teeth":43,"profile_shift":{"auto":true,"manual":0.0},"working_depth":1.0,
                  "addendum":{"auto":false,"manual":1.0},"min_tip_width":0.1,
                  "dedendum":1.25,"root_radius":0.38,
                  "face_width":{"auto":true,"manual":0.0},
                  "auto_face_from_bending":true,"auto_face_from_contact":true,
                  "material":"4340 Hardened Steel"}
               ]}
            ]}}"#;

        let v: serde_json::Value = serde_json::from_str(&solve_train_impl(req).unwrap()).unwrap();
        assert!((v["total_ratio"].as_f64().unwrap() - 43.0 / 17.0).abs() < 1e-12);
        assert!(v["output_torque"].as_f64().unwrap() > 2.0);

        let g0 = &v["stages"][0]["gears"][0];
        // The automatic face width came back, and so did the cycle count.
        assert!(g0["face_width"].as_f64().unwrap() > 0.0);
        assert!(g0["tooth_cycles"].as_f64().unwrap() > 0.0);
        assert!((g0["speed"].as_f64().unwrap() - 3000.0).abs() < 1e-9);
        // Spur stage: the overlap ratio is exactly zero, not merely small.
        assert_eq!(
            v["stages"][0]["contact_ratios"]["overlap"]
                .as_f64()
                .unwrap(),
            0.0
        );
    }

    #[test]
    fn a_train_that_cannot_be_solved_says_why() {
        let bad = r#"{"train":{"input_speed":1.0,"input_torque":1.0,
            "actuation":{"intermittent":{"range_degrees":25.0,"actuations":1000}},
            "stages":[]}}"#;
        assert!(solve_train_impl(bad).unwrap_err().contains("no stages"));
        assert!(solve_train_impl("{ not json").is_err());
    }

    #[test]
    fn a_broken_material_file_explains_itself_rather_than_panicking() {
        // The message must name what is wrong, not merely report failure: this
        // is what the user sees after hand-editing their own library.
        let err = import_materials_impl("[[material]]\nname = ").unwrap_err();
        assert!(err.contains("not valid"), "unhelpful message: {err}");

        assert!(import_materials_impl("")
            .unwrap_err()
            .contains("no materials"));
        assert!(export_materials_impl("{ not json").is_err());
    }

    #[test]
    fn dxf_comes_out_whole() {
        let dxf = export_dxf_impl(REQ).unwrap();
        assert!(dxf.starts_with("0\nSECTION"));
        assert!(dxf.trim_end().ends_with("EOF"));
        assert!(dxf.contains("LWPOLYLINE") && dxf.contains("GEAR_PROFILE"));
    }
}
