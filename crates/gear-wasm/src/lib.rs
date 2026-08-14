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

/// Derived geometry and metrology for one gear.
///
/// Lengths are millimetres, angles degrees, composite errors micrometres.
#[derive(Serialize)]
pub struct GearSummary {
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

    GearSummary {
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
    fn dxf_comes_out_whole() {
        let dxf = export_dxf_impl(REQ).unwrap();
        assert!(dxf.starts_with("0\nSECTION"));
        assert!(dxf.trim_end().ends_with("EOF"));
        assert!(dxf.contains("LWPOLYLINE") && dxf.contains("GEAR_PROFILE"));
    }
}
