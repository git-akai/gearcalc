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

use gear_core::{Gear, GearParams};
use serde::Serialize;
use wasm_bindgen::prelude::*;

/// Everything the UI needs about one gear.
///
/// Lengths are millimetres, angles are **degrees** at this boundary (radians
/// stay inside `gear-core`).
#[derive(Serialize)]
pub struct GearSummary {
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    /// Transverse tooth thickness at the pitch circle.
    pub tooth_thickness: f64,
    /// Cutter tip radius actually used, after the fit cap.
    pub fillet_radius: f64,
    /// Transverse pressure angle, degrees.
    pub transverse_pressure_angle: f64,
    /// Cutter tip width in the normal plane — helix-independent, as expected.
    pub cutter_tip_width: f64,
    pub undercut: bool,
    pub severed: bool,
    /// Guards that altered the requested geometry. Empty is the normal case;
    /// anything here means the result is not the geometry that was asked for.
    pub clamps: Vec<String>,
}

fn summarise(g: &Gear) -> GearSummary {
    let p = &g.params;
    let cutter_tip_width = std::f64::consts::PI * p.module
        - g.st * g.beta.cos()
        - 2.0 * p.module * (p.dedendum - p.profile_shift) * g.alpha_n.tan();
    GearSummary {
        pitch_radius: g.r,
        base_radius: g.rb,
        tip_radius: g.ra,
        root_radius: g.rf,
        tooth_thickness: g.st,
        fillet_radius: g.rho,
        transverse_pressure_angle: g.alpha_t.to_degrees(),
        cutter_tip_width,
        undercut: g.undercut,
        severed: g.severed,
        clamps: g.clamps.notes.clone(),
    }
}

// The real work lives in these plain-Rust functions so it is testable on the
// host. `JsError` cannot even be constructed off a wasm target, so wrapping the
// logic in it directly would make the error paths untestable — which is exactly
// where a calculation engine most needs tests.

fn parse(input: &str) -> Result<GearParams, String> {
    serde_json::from_str(input).map_err(|e| format!("bad gear parameters: {e}"))
}

fn solve_gear_impl(input: &str) -> Result<String, String> {
    let g = Gear::new(parse(input)?);
    serde_json::to_string(&summarise(&g)).map_err(|e| format!("could not encode result: {e}"))
}

fn gear_profile_impl(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, String> {
    let g = Gear::new(parse(input)?);
    Ok(g.profile(points_per_tooth)
        .into_iter()
        .flat_map(|p| [p[0], p[1]])
        .collect())
}

/// Derived geometry for one gear.
#[wasm_bindgen]
pub fn solve_gear(input: &str) -> Result<String, JsError> {
    solve_gear_impl(input).map_err(|e| JsError::new(&e))
}

/// The closed cross-section, as a flat `[x0, y0, x1, y1, ...]` array ready for
/// a canvas path. Flat rather than nested to keep the crossing cheap.
#[wasm_bindgen]
pub fn gear_profile(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, JsError> {
    gear_profile_impl(input, points_per_tooth).map_err(|e| JsError::new(&e))
}

/// Version of the core, so the UI can show what it is actually running.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_json() {
        let input = r#"{"module":1.0,"pressure_angle":20.0,"teeth":17,"profile_shift":0.2,
            "helix_angle":0.0,"addendum":1.0,"dedendum":1.25,"root_radius":0.38,
            "thickness_mod":1.0}"#;
        let out = solve_gear_impl(input).unwrap();
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert!((v["pitch_radius"].as_f64().unwrap() - 8.5).abs() < 1e-12);
        assert_eq!(v["undercut"].as_bool(), Some(false));
    }

    #[test]
    fn rejects_malformed_input_instead_of_panicking() {
        assert!(solve_gear_impl("{ not json").is_err());
        assert!(solve_gear_impl(r#"{"module":1.0}"#).is_err());
    }

    #[test]
    fn profile_is_flat_pairs() {
        let input = r#"{"module":1.0,"pressure_angle":20.0,"teeth":9,"profile_shift":0.0,
            "helix_angle":0.0,"addendum":1.0,"dedendum":1.25,"root_radius":0.38,
            "thickness_mod":1.0}"#;
        let v = gear_profile_impl(input, 200).unwrap();
        assert!(v.len().is_multiple_of(2) && v.len() > 100);
    }
}
