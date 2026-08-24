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

/// Derived geometry and metrology for one gear.
///
/// Lengths are millimetres, angles degrees, composite errors micrometres.
#[derive(Serialize)]
pub struct GearSummary {
    /// Every bound on this gear's inputs.
    ///
    /// The core type is serialised directly rather than mirrored here: a mirror
    /// is one more place a limit could be written down, and there is meant to be
    /// exactly one. See `gear_core::auto::admissible_ranges`.
    pub ranges: gear_core::auto::Ranges,
    /// The four reference circles as **radii**, mm — what a drawing is built
    /// from, and what the viewport scales by.
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    /// ...and as **diameters**, mm, which is how a gear is specified, measured
    /// and called out on a drawing.
    ///
    /// Both are served rather than one derived from the other on the far side,
    /// because doubling a number is arithmetic and arithmetic belongs here: the
    /// UI displays what Rust computed and nothing else (DESIGN §1).
    pub pitch_diameter: f64,
    pub base_diameter: f64,
    pub tip_diameter: f64,
    pub root_diameter: f64,
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
        ranges: gear_core::auto::admissible_ranges(&g.params, req.working_depth),
        pitch_radius: g.r,
        base_radius: g.rb,
        tip_radius: g.ra,
        root_radius: g.rf,
        pitch_diameter: 2.0 * g.r,
        base_diameter: 2.0 * g.rb,
        tip_diameter: 2.0 * g.ra,
        root_diameter: 2.0 * g.rf,
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

/// Everything the UI asks about one **internal** gear.
///
/// A separate request from [`GearRequest`], for the reason the worm stage got a
/// separate result: a ring's answers are a different shape. It has no span or
/// over-pins measurement in the sense the metrology module means, no strength
/// rating yet, and it has something an external gear does not — the cutter that
/// shaped it, without which its fillet is undefined.
#[derive(Deserialize)]
pub struct RingRequest {
    pub params: GearParams,
    /// Pin or ball diameter for the between-pins measurement, mm.
    #[serde(default)]
    pub pin_diameter: Option<f64>,
    #[serde(default)]
    pub cutter: CutterRef,
    #[serde(default)]
    pub chord_tolerance: Option<f64>,
    #[serde(default = "yes")]
    pub reference_circles: bool,
}

/// The pinion cutter, as the UI sends it.
#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct CutterRef {
    pub teeth: u32,
    /// Addendum, in modules.
    pub addendum: f64,
    /// Tip corner round, in modules.
    pub tip_round: f64,
}

impl Default for CutterRef {
    fn default() -> Self {
        let c = gear_core::ring::Cutter::default();
        Self {
            teeth: c.teeth,
            addendum: c.addendum,
            tip_round: c.tip_round,
        }
    }
}

impl CutterRef {
    fn to_cutter(self) -> gear_core::ring::Cutter {
        gear_core::ring::Cutter {
            teeth: self.teeth,
            addendum: self.addendum,
            tip_round: self.tip_round,
        }
    }
}

/// How the space between two ring teeth closes.
///
/// Three cases rather than the two a boolean allowed: a cut that generates no
/// fillet at all is not "a root arc between the fillets", and saying so put a
/// description of a fillet next to a drawing that had none.
#[derive(Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RootForm {
    /// The fillets from the two flanks meet before mid-space: no flat at all.
    FullyFilleted,
    /// A flat at the root circle between the two fillets.
    RootArc,
    /// No fillet was cut — the flank runs to the root circle. The reason is in
    /// [`RingSummary::clamps`].
    NoFillet,
}

/// What the UI shows for a ring.
#[derive(Serialize)]
pub struct RingSummary {
    pub teeth: u32,
    pub transverse_module: f64,
    pub transverse_pressure_angle: f64,
    /// Pitch, base, tip and root radii, mm. The tip is **inside** the pitch
    /// circle and the root outside it. These are what the viewport draws with.
    pub pitch_radius: f64,
    pub base_radius: f64,
    pub tip_radius: f64,
    pub root_radius: f64,
    /// The same four as diameters, mm — how a ring is specified and gauged.
    pub pitch_diameter: f64,
    pub base_diameter: f64,
    pub tip_diameter: f64,
    pub root_diameter: f64,
    /// Radius at which the flank hands over to the fillet, mm.
    ///
    /// `None` when the cut generated no fillet: there is then no handover, and
    /// reporting the root radius here said there was one.
    pub junction_radius: Option<f64>,
    /// How the tooth space closes.
    pub root_form: RootForm,
    /// Where a drawing shades the rim out to, mm. A convention with no
    /// engineering meaning — see [`gear_core::ring::Ring::rim_radius`].
    pub rim_radius: f64,
    /// The lowest radius this cutter can generate as an involute, mm. Below it
    /// the cutter's own involute has run out.
    pub generation_limit: f64,
    /// Whether the tip stays above that limit.
    pub fully_generated: bool,
    /// The fewest teeth this design could have had and still cleared its own
    /// base circle: `2 h_a cos β / (1 − cos α_t)`, rounded up.
    ///
    /// Reported because it is the constraint that actually bites on internal
    /// gears, it moves with the addendum, pressure angle and helix, and a
    /// designer meeting it by accident should be told which margin they are on.
    pub smallest_tooth_count: u32,
    /// Measurement **between** two pins or balls, the internal counterpart of
    /// the gear tab's over-pins. Two pins only, and
    /// [`gear_core::metrology::between_pins`] says why.
    pub between_pins: Maybe<PinsOut>,
    pub clamps: Vec<String>,
}

fn ring_of(req: &RingRequest) -> gear_core::ring::Ring {
    gear_core::ring::Ring::new(&req.params, &req.cutter.to_cutter())
}

fn smallest_tooth_count(params: &GearParams) -> u32 {
    let beta = params.helix_angle.to_radians();
    let alpha_t = (params.pressure_angle.to_radians().tan() / beta.cos()).atan();
    let threshold = 2.0 * params.addendum * beta.cos() / (1.0 - alpha_t.cos());
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let out = threshold.ceil().max(1.0) as u32;
    out
}

fn parse_ring(input: &str) -> Result<RingRequest, String> {
    serde_json::from_str(input).map_err(|e| format!("bad ring request: {e}"))
}

fn solve_ring_impl(input: &str) -> Result<String, String> {
    let req = parse_ring(input)?;
    let g = ring_of(&req);
    let summary = RingSummary {
        teeth: g.teeth,
        transverse_module: g.mt,
        transverse_pressure_angle: g.alpha_t.to_degrees(),
        pitch_radius: g.r,
        base_radius: g.rb,
        tip_radius: g.ra,
        root_radius: g.rf,
        pitch_diameter: 2.0 * g.r,
        base_diameter: 2.0 * g.rb,
        tip_diameter: 2.0 * g.ra,
        root_diameter: 2.0 * g.rf,
        junction_radius: g.fillet.map(|_| g.involute_at(g.u_j).0),
        rim_radius: g.rim_radius(),
        root_form: match g.fillet {
            None => RootForm::NoFillet,
            Some(f) if f.s_root != 0.0 => RootForm::FullyFilleted,
            Some(_) => RootForm::RootArc,
        },
        generation_limit: g.generation_limit(),
        fully_generated: g.fully_generated(),
        smallest_tooth_count: smallest_tooth_count(&req.params),
        between_pins: match req.pin_diameter {
            Some(d) => Maybe::from(metrology::between_pins(&g, d).map(|p| PinsOut {
                nominal: p.nominal,
                pin_centre_radius: p.pin_centre_radius,
                contact_radius: p.contact_radius,
            })),
            None => Maybe::Unavailable {
                unavailable: "no pin diameter given".into(),
            },
        },
        clamps: g.clamps.clone(),
    };
    serde_json::to_string(&summary).map_err(|e| format!("could not encode result: {e}"))
}

fn ring_profile_impl(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, String> {
    let req = parse_ring(input)?;
    Ok(ring_of(&req)
        .profile(points_per_tooth)
        .into_iter()
        .flatten()
        .collect())
}

fn export_ring_dxf_impl(input: &str) -> Result<String, String> {
    let req = parse_ring(input)?;
    Ok(gear_io::ring_to_dxf(
        &ring_of(&req),
        &gear_io::DxfOptions {
            chord_tolerance: req
                .chord_tolerance
                .unwrap_or(gear_core::outline::DEFAULT_CHORD_TOLERANCE),
            reference_circles: req.reference_circles,
        },
    ))
}

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

fn import_train_impl(toml_text: &str) -> Result<String, String> {
    let doc = gear_io::train::from_toml(toml_text).map_err(|e| e.to_string())?;
    serde_json::to_string(&doc).map_err(|e| e.to_string())
}

fn export_train_impl(document_json: &str) -> Result<String, String> {
    let doc: gear_io::TrainDocument =
        serde_json::from_str(document_json).map_err(|e| e.to_string())?;
    gear_io::train::to_toml(&doc).map_err(|e| e.to_string())
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
/// Derived geometry for one internal gear. JSON in, JSON out.
///
/// # Errors
///
/// A malformed request.
#[wasm_bindgen]
pub fn solve_ring(input: &str) -> Result<String, JsError> {
    solve_ring_impl(input).map_err(|e| JsError::new(&e))
}

/// A ring's closed outline as flat `[x, y, x, y, ...]`, for the viewport.
///
/// # Errors
///
/// A malformed request.
#[wasm_bindgen]
pub fn ring_profile(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, JsError> {
    ring_profile_impl(input, points_per_tooth).map_err(|e| JsError::new(&e))
}

/// A ring's bore as DXF.
///
/// # Errors
///
/// A malformed request.
#[wasm_bindgen]
pub fn export_ring_dxf(input: &str) -> Result<String, JsError> {
    export_ring_dxf_impl(input).map_err(|e| JsError::new(&e))
}

/// Everything a fresh tab starts at.
///
/// **These are engineering numbers, so they live here rather than in
/// TypeScript.** They used to be written down in both places, and the two
/// copies drifted: the gear tab's cutter carried `tip_round = 0.38`, which is
/// the *rack's* figure. A 20-tooth shaper's tip is only 0.377 modules wide, so
/// no such tool exists — every ring the UI built was cut by a cutter that
/// generates no fillet, and the viewport drew the result as a straight-sided
/// polygon. The core's own default has been 0.2 all along, with a comment
/// saying why. See `docs/DESIGN.md` §12.
///
/// Serving them across the boundary is what makes that class of drift
/// impossible rather than merely fixed.
#[derive(Serialize)]
pub struct Defaults {
    pub gear: GearTabDefaults,
    /// A fresh geartrain, with one spur stage in it.
    pub train: gear_core::train::Train,
    /// One of each stage kind, for the "add stage" menu. Three, as the
    /// specification lists them: a crossed gear pair is **not** a fourth — it is
    /// a spur stage with its shafts at an angle (§4.5.1).
    pub spur_stage: gear_core::train::Stage,
    pub worm_stage: gear_core::train::Stage,
    pub planetary_stage: gear_core::train::Stage,
}

/// What a new gear tab holds. The values are the specification's, and the
/// tooth count is deliberately *not* the core's own default of 17.
#[derive(Serialize)]
pub struct GearTabDefaults {
    pub params: GearParams,
    pub cutter: CutterRef,
    /// Pin or ball diameter for the over-pins measurement, mm.
    pub pin_diameter: f64,
    /// Export accuracy, mm.
    pub chord_tolerance: f64,
    pub reference_circles: bool,
}

fn defaults_impl() -> Result<String, String> {
    use gear_core::train::{Actuation, PlanetaryStage, SpurStage, Stage, Train, WormStage};

    // The tab starts with an automatic face width, where the core's own
    // default is a plain 10 mm. Both are right for their caller: the CLI and
    // the tests want a fixed number they can reason about, and a designer
    // opening the panel wants to see the width the rating asks for. Seeded at
    // 5 mm so the field has something to fall back to when the toggle is
    // turned off.
    let ui_gear = |g: &gear_core::train::StageGear| gear_core::train::StageGear {
        face_width: gear_core::params::Auto::automatic(5.0),
        ..g.clone()
    };
    let spur = {
        let d = SpurStage::default();
        SpurStage {
            gears: [ui_gear(&d.gears[0]), ui_gear(&d.gears[1])],
            ..d
        }
    };
    let planetary = {
        let d = PlanetaryStage::default();
        PlanetaryStage {
            sun: ui_gear(&d.sun),
            planet: ui_gear(&d.planet),
            ring: ui_gear(&d.ring),
            ..d
        }
    };

    let defaults = Defaults {
        gear: GearTabDefaults {
            params: GearParams {
                // The specification's default tooth count, not the core's.
                teeth: 9,
                ..GearParams::default()
            },
            cutter: CutterRef::default(),
            pin_diameter: 1.75,
            chord_tolerance: gear_core::outline::DEFAULT_CHORD_TOLERANCE,
            reference_circles: true,
        },
        train: Train {
            input_speed: 30_000.0,
            input_torque: 0.1,
            actuation: Actuation::default(),
            stages: vec![Stage::Spur(spur.clone())],
        },
        spur_stage: Stage::Spur(spur),
        worm_stage: Stage::Worm(WormStage::default()),
        planetary_stage: Stage::Planetary(Box::new(planetary)),
    };
    serde_json::to_string(&defaults).map_err(|e| format!("could not encode defaults: {e}"))
}

/// The values a fresh tab starts at, as JSON.
///
/// # Errors
///
/// Only if the defaults cannot be encoded, which would be a build-time defect.
#[wasm_bindgen]
pub fn defaults() -> Result<String, JsError> {
    defaults_impl().map_err(|e| JsError::new(&e))
}

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

/// Import a geartrain: TOML text in, `{ name, train }` JSON out.
///
/// The same arrangement as the material library, and for the same reason: the
/// TOML never reaches TypeScript, so exactly one parser exists and it is the
/// tested one. A malformed file comes back as the parser's own complaint, which
/// names the line.
///
/// The train's **inputs** are what the file holds; everything derived is
/// recomputed by `solve_train` once the tab exists. A stage may name a material
/// this library does not have — that is not an import failure, and `solve_train`
/// reports it by name.
///
/// # Errors
///
/// A document that is not a geartrain, or one with no stages.
#[wasm_bindgen]
pub fn import_train(toml_text: &str) -> Result<String, JsError> {
    import_train_impl(toml_text).map_err(|e| JsError::new(&e))
}

/// Export a geartrain: `{ name, train }` JSON in, TOML text out, ready for a
/// download.
///
/// # Errors
///
/// A malformed document, which would be a defect on this side of the boundary.
#[wasm_bindgen]
pub fn export_train(document_json: &str) -> Result<String, JsError> {
    export_train_impl(document_json).map_err(|e| JsError::new(&e))
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

    /// **A geartrain survives the round trip to a file and back — as answers,
    /// not just as bytes.**
    ///
    /// Comparing the two documents would only show that `serde` is consistent
    /// with itself. What a user cares about is that the imported train *is* the
    /// train they exported, so the check solves both and compares the results:
    /// ratio, efficiency, backlash and every stage's numbers. A field silently
    /// dropped on the way out reappears as a default, which looks like a value
    /// rather than like a loss — and would move an answer here.
    #[test]
    fn a_geartrain_survives_export_and_import_as_the_same_answers() {
        // Start from the defaults the UI hands out, so the tested path is the
        // one a user actually takes, and give it one of every stage kind.
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let crossed = {
            let mut c = d["spur_stage"].clone();
            c["shaft_angle"] = serde_json::json!(90.0);
            c
        };
        let document = serde_json::json!({
            "name": "Elevation drive",
            "train": {
                "input_speed": 12_000.0,
                "input_torque": 0.25,
                "actuation": { "continuous": { "operating_percent": 80.0, "runtime_hours": 1000.0 } },
                // Every stage kind, and a crossed pair too — which is a spur
                // stage with its shafts at an angle, not a kind of its own.
                "stages": [d["spur_stage"], crossed, d["worm_stage"], d["planetary_stage"]],
            }
        });

        let toml_text = export_train_impl(&document.to_string()).unwrap();
        let back: serde_json::Value =
            serde_json::from_str(&import_train_impl(&toml_text).unwrap()).unwrap();
        assert_eq!(back["name"].as_str(), Some("Elevation drive"));

        let solve = |doc: &serde_json::Value| {
            let req = serde_json::json!({ "train": doc["train"], "materials": null });
            let out = solve_train_impl(&req.to_string()).unwrap();
            serde_json::from_str::<serde_json::Value>(&out).unwrap()
        };
        assert_eq!(
            solve(&document),
            solve(&back),
            "the imported train answers differently from the one exported"
        );

        // ...and the file a person opens says what it is.
        assert!(toml_text.starts_with("# Geartrain."));
        assert!(toml_text.contains("name = \"Elevation drive\""));
    }

    /// A file that is not a geartrain comes back as a reason, not a panic and
    /// not a default-filled train — the UI shows the text verbatim.
    #[test]
    fn a_bad_geartrain_file_comes_back_as_a_reason() {
        let e = import_train_impl("name = \"nothing here\"").unwrap_err();
        assert!(e.contains("not valid"), "{e}");

        let empty = serde_json::json!({
            "name": "no stages",
            "train": { "input_speed": 1.0, "input_torque": 1.0,
                       "actuation": { "intermittent": { "range_degrees": 25.0, "actuations": 10 } },
                       "stages": [] }
        });
        let text = export_train_impl(&empty.to_string()).unwrap();
        let e = import_train_impl(&text).unwrap_err();
        assert!(e.contains("no stages"), "{e}");
    }

    /// **The tool the UI ships with is one that can cut.**
    ///
    /// A shaper's tip is narrow — 0.377 modules on a 20-tooth cutter at a 1.25
    /// addendum — so it cannot carry two 0.38-module corner rounds, and asking
    /// it to generates no fillet at all. That figure is the *rack's*, and for a
    /// while it was the gear tab's default: every ring the UI drew had its
    /// flank running to a sharp root.
    ///
    /// This asserts the property rather than the number, so a future default is
    /// free to be different and not free to be uncuttable.
    #[test]
    fn the_shipped_cutter_generates_a_fillet() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let cutter = &d["gear"]["cutter"];

        // At a tooth count that cutter can actually reach around.
        let req = format!(
            r#"{{"params":{{"teeth":60,"module":1.0,"pressure_angle":20.0,
               "helix_angle":0.0,"profile_shift":0.0,"addendum":1.0,"dedendum":1.25,
               "root_radius":0.38,"thickness_mod":1.0}},"cutter":{cutter}}}"#
        );
        let v: serde_json::Value = serde_json::from_str(&solve_ring_impl(&req).unwrap()).unwrap();

        assert_ne!(
            v["root_form"].as_str(),
            Some("no_fillet"),
            "the shipped cutter generates no fillet: {:?}",
            v["clamps"]
        );
        let junction = v["junction_radius"]
            .as_f64()
            .expect("a generated fillet has a junction with the flank");
        assert!(
            junction > v["tip_radius"].as_f64().unwrap()
                && junction < v["root_radius"].as_f64().unwrap(),
            "the junction is on the tooth, between tip and root"
        );
    }

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
        // One state per entry, and the polyamides are quoted conditioned.
        assert!((pa6["elastic_modulus"]["value"].as_f64().unwrap() - 1000.0).abs() < 1e-9);
        assert!(pa6["condition"].as_str().unwrap().contains("conditioned"));
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

    /// A train with two kinds of stage in it, which is what the `kind` tag on
    /// each stage is for. The worm result has a different shape from the spur
    /// one, and both have to survive the same boundary.
    /// A ring crosses the boundary: its own request shape, its own summary, an
    /// outline the viewport can draw and a DXF the CAD can read.
    #[test]
    fn a_ring_crosses_the_boundary() {
        let req = r#"{"params":{"teeth":60,"module":1.0,"pressure_angle":20.0,
            "helix_angle":0.0,"profile_shift":0.0,"addendum":1.0,"dedendum":1.25,
            "root_radius":0.38,"thickness_mod":1.0},
            "cutter":{"teeth":20,"addendum":1.25,"tip_round":0.2}}"#;

        let v: serde_json::Value = serde_json::from_str(&solve_ring_impl(req).unwrap()).unwrap();
        let (tip, pitch, root) = (
            v["tip_radius"].as_f64().unwrap(),
            v["pitch_radius"].as_f64().unwrap(),
            v["root_radius"].as_f64().unwrap(),
        );
        assert!(tip < pitch && pitch < root, "a ring's radii run inward");
        assert!(v["junction_radius"].as_f64().unwrap() > tip);
        // The constraint that actually bites on internal gears, reported.
        assert_eq!(v["smallest_tooth_count"].as_u64().unwrap(), 34);

        // **At the density that was asked for.** `> 200` passed while the
        // outline was collapsing to seven points a tooth, because 60 teeth of
        // rubbish still clears 200: the number to assert is points *per tooth*
        // against the number requested, not a total that any failure also meets.
        let outline = ring_profile_impl(req, 60).unwrap();
        assert!(outline.len().is_multiple_of(2));
        let per_tooth = outline.len() as f64 / 2.0 / 60.0;
        assert!(
            (50.0..=70.0).contains(&per_tooth),
            "asked for 60 points a tooth and got {per_tooth}"
        );
        assert!(
            outline.iter().all(|v| v.is_finite()),
            "the viewport cannot draw a NaN"
        );

        let dxf = export_ring_dxf_impl(req).unwrap();
        assert!(dxf.contains("LWPOLYLINE"), "no polyline in the DXF");
        assert!(
            dxf.ends_with("EOF\r\n") || dxf.ends_with("EOF\n"),
            "truncated DXF"
        );

        // **The between-pins measurement crosses too, and subtracts.**
        let with_pin = req.replace(
            r#""cutter":{"teeth":20,"addendum":1.25,"tip_round":0.2}"#,
            r#""cutter":{"teeth":20,"addendum":1.25,"tip_round":0.2},"pin_diameter":1.8"#,
        );
        let p: serde_json::Value =
            serde_json::from_str(&solve_ring_impl(&with_pin).unwrap()).unwrap();
        let bp = &p["between_pins"];
        let nominal = bp["nominal"]
            .as_f64()
            .expect("a 60-tooth ring admits a 1.8 mm pin");
        let centre = bp["pin_centre_radius"].as_f64().unwrap();
        // Between inner surfaces, so the pin *subtracts* — the opposite of the
        // gear tab's over-pins, and the sign a reader should be able to check.
        assert!((nominal - (2.0 * centre - 1.8)).abs() < 1e-9);
        assert!(nominal < 2.0 * centre);
        // ...and without a pin diameter it says so rather than inventing one.
        assert!(v["between_pins"]["unavailable"].is_string());

        // **A shifted ring must come back shifted.** The gear tab has always
        // sent `profile_shift` for an internal gear and `Ring::new` used to drop
        // it on the floor, so the box moved nothing — the sort of gap only an
        // end-to-end check finds, because every layer was individually happy.
        let shifted = req.replace("\"profile_shift\":0.0", "\"profile_shift\":0.3");
        let w: serde_json::Value =
            serde_json::from_str(&solve_ring_impl(&shifted).unwrap()).unwrap();
        for key in ["tip_radius", "root_radius"] {
            let (before, after) = (v[key].as_f64().unwrap(), w[key].as_f64().unwrap());
            assert!(
                after > before,
                "{key} must move outward under a positive shift: {before} -> {after}"
            );
        }
        // ...and the circles a shift cannot move stay put, exactly.
        assert_eq!(v["pitch_radius"], w["pitch_radius"]);
        assert_eq!(v["base_radius"], w["base_radius"]);
        assert!(
            ring_profile_impl(&shifted, 60).unwrap().len() > 200,
            "a shifted ring still has an outline"
        );
    }

    /// **A planetary stage crosses the boundary with nothing added for it.**
    ///
    /// `Stage` is a tagged enum and `Train` already carried a `Vec` of them, so
    /// the new kind needed no entry point of its own. That is the claim worth
    /// checking rather than assuming — "it should just work" is exactly what
    /// turns out to be false at a serde boundary.
    ///
    /// The assertions look for the shape a planetary result has and no other kind
    /// does: three shafts instead of two, a *solved* planet shift, and two meshes
    /// each with their own answers.
    #[test]
    fn a_planetary_stage_crosses_the_boundary_with_its_own_shape() {
        let req = r#"{"train":{
            "input_speed": 3000.0,
            "input_torque": 2.0,
            "actuation": { "continuous": { "operating_percent": 80.0, "runtime_hours": 1000.0 } },
            "stages": [
              {"kind":"planetary",
               "module":1.0,"pressure_angle":20.0,"helix_angle":0.0,
               "friction_sun_planet":0.06,"friction_planet_ring":0.06,
               "thickness_mod":1.0,"planets":3,
               "arrangement":{"input":"sun","fixed":"ring"},
               "clearance":0.02,"tolerance_plus":0.02,"tolerance_minus":0.02,
               "min_planet_clearance":0.3,
               "cutter":{"teeth":20,"addendum":1.25,"tip_round":0.2},
               "sun":{"teeth":24,"profile_shift":{"auto":false,"manual":0.0},"working_depth":1.0,"addendum":{"auto":false,"manual":1.0},"min_tip_width":0.1,"dedendum":1.25,"root_radius":0.38,"face_width":{"auto":true,"manual":0.0},"auto_face_from_bending":true,"auto_face_from_contact":true,"material":"4340 Hardened Steel"},
               "planet":{"teeth":18,"profile_shift":{"auto":false,"manual":0.0},"working_depth":1.0,"addendum":{"auto":false,"manual":1.0},"min_tip_width":0.1,"dedendum":1.25,"root_radius":0.38,"face_width":{"auto":true,"manual":0.0},"auto_face_from_bending":true,"auto_face_from_contact":true,"material":"4340 Hardened Steel"},
               "ring":{"teeth":60,"profile_shift":{"auto":false,"manual":0.0},"working_depth":1.0,"addendum":{"auto":false,"manual":1.0},"min_tip_width":0.1,"dedendum":1.25,"root_radius":0.38,"face_width":{"auto":true,"manual":0.0},"auto_face_from_bending":true,"auto_face_from_contact":true,"material":"4340 Hardened Steel"}
              }
            ]}}"#;

        let out = solve_train_impl(req).expect("a planetary train should solve");
        let v: serde_json::Value = serde_json::from_str(&out).unwrap();
        let stage = &v["stages"][0];
        assert_eq!(stage["kind"], "planetary");

        // Ring held, sun driving: the classical 1 + z_r/z_s.
        assert!((v["total_ratio"].as_f64().unwrap() - 3.5).abs() < 1e-12);
        assert_eq!(stage["output"], "carrier");
        assert_eq!(stage["arrangement"]["fixed"], "ring");

        // Three shafts, the held one exactly still, and the torques balancing.
        let speeds = stage["speeds"].as_array().unwrap();
        assert_eq!(speeds.len(), 3);
        assert_eq!(speeds[2].as_f64().unwrap(), 0.0, "the ring is held");
        assert!((speeds[0].as_f64().unwrap() - 3000.0).abs() < 1e-9);
        let sum: f64 = stage["torques"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_f64().unwrap())
            .sum();
        assert!(sum.abs() < 1e-9, "torques must balance, got {sum}");

        // The planet's shift is *solved*, not sent: 24 + 2x18 = 60 is the ideal
        // ring, so it comes back as exactly zero with a closed residual.
        assert!(stage["planet"]["profile_shift"].as_f64().unwrap().abs() < 1e-12);
        assert!(stage["planet"]["shift_residual"].as_f64().unwrap() < 1e-12);
        assert_eq!(stage["planet"]["fully_reversed"], true);
        // Its reduced allowable carries provenance across, like every other
        // material figure (§6.3).
        assert_eq!(stage["planet"]["reversed_allowable"]["basis"], "derived");
        assert!(stage["planet"]["reversed_allowable"]["note"].is_string());

        // Two meshes with their own answers, and every member rated.
        for mesh in ["sun_planet", "planet_ring"] {
            assert!(
                stage[mesh]["contact_ratios"]["transverse"]
                    .as_f64()
                    .unwrap()
                    > 1.0
            );
            assert!(stage[mesh]["contact_stress"].as_f64().unwrap() > 0.0);
        }
        for who in ["sun", "ring"] {
            assert!(
                stage[who]["bending_stress"].as_f64().unwrap() > 0.0,
                "{who} must be rated"
            );
        }
        assert!(stage["planet"]["gear"]["bending_stress"].as_f64().unwrap() > 0.0);
        assert_eq!(stage["equal_spacing"], true);
        // What the stage *assumes* has to come across too — here, equal load
        // sharing between planets, which no calculation can establish.
        let notes = stage["notes"].as_array().unwrap();
        assert!(
            notes
                .iter()
                .any(|n| n.as_str().unwrap_or("").contains("share the load equally")),
            "the load-sharing assumption must be reported: {notes:?}"
        );
        // ...and the output-shaft backlash is a real figure now, not a placeholder.
        assert!(stage["backlash"]["forward"]["nominal"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn a_mixed_train_crosses_the_boundary_with_both_shapes_intact() {
        let req = r#"{"train":{
            "input_speed": 3000.0,
            "input_torque": 2.0,
            "actuation": { "continuous": { "operating_percent": 80.0, "runtime_hours": 1000.0 } },
            "stages": [
              {"kind":"spur",
               "module":1.0,"pressure_angle":20.0,"additional_helix":0.0,"friction":0.06,
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
               ]},
              {"kind":"worm",
               "module":1.0,"pressure_angle":20.0,"shaft_angle":90.0,"friction":0.06,
               "starts":1,"sizing":{"pitch_diameter":7.0},"wheel_teeth":40,
               "centre_distance":{"auto":true,"manual":0.0},
               "clearance":0.02,"tolerance_plus":0.02,"tolerance_minus":0.02,
               "axial_clearance":0.04,
               "worm":{"face_width":{"auto":false,"manual":10.0},"material":"4340 Hardened Steel"},
               "wheel":{"face_width":{"auto":true,"manual":10.0},"material":"Brass C360"}}
            ]}}"#;

        let v: serde_json::Value = serde_json::from_str(&solve_train_impl(req).unwrap()).unwrap();
        let want = (43.0 / 17.0) * 40.0;
        assert!((v["total_ratio"].as_f64().unwrap() - want).abs() < 1e-9);

        // Each stage says what it is, and carries its own shape.
        assert_eq!(v["stages"][0]["kind"], "spur");
        assert_eq!(v["stages"][1]["kind"], "worm");
        assert!(
            v["stages"][0]["gears"][0]["bending_stress"]
                .as_f64()
                .unwrap()
                > 0.0
        );

        let worm = &v["stages"][1];
        assert!(
            worm["gears"].is_null(),
            "a worm stage has members, not gears"
        );
        assert!(worm["contact"]["max_pressure"].as_f64().unwrap() > 0.0);
        let eff = &worm["efficiency"];
        assert!(eff["backward"].as_f64().unwrap() < eff["forward"].as_f64().unwrap());
        // ...while the spur stage puts the same number in both, which is the
        // point of reporting it directionally everywhere rather than only where
        // it differs.
        let spur_eff = &v["stages"][0]["efficiency"];
        assert_eq!(spur_eff["forward"], spur_eff["backward"]);
        // And the train reports both totals, plus backlash at each end.
        assert!(v["total_efficiency"]["backward"].as_f64().unwrap() > 0.0);
        assert!(v["backlash"]["forward"]["nominal"].as_f64().unwrap() > 0.0);
        assert!(v["backlash"]["backward"]["nominal"].as_f64().unwrap() > 0.0);
        // The sliding speed could only be filled once the shaft line was known.
        assert!(worm["sliding_velocity"].as_f64().unwrap() > 0.0);
        assert!(worm["members"][1]["speed"].as_f64().unwrap() > 0.0);
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
              {"kind":"spur",
               "module":1.0,"pressure_angle":20.0,"additional_helix":0.0,"friction":0.06,
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
