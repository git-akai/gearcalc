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
use gear_core::note::{Explain, Note};
use gear_core::train::{StageFamily, StagePreset};
use gear_core::{GearParams, Tooth};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

/// Everything the UI asks about one gear.
#[derive(Deserialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct GearRequest {
    pub params: GearParams,
    /// Pin or ball diameter for the over-pins measurement, mm.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub pin_diameter: Option<f64>,
    /// Tolerance class, as `{ "scale": "fine" | "standard", "grade": n }`.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub tolerance_class: Option<ClassRef>,
    /// Maximum deviation of the exported outline from the true curve, mm.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub chord_tolerance: Option<f64>,
    /// Include the pitch, base, tip and root circles in the DXF.
    ///
    /// `None` is "not stated", which is not the same as `false` — and saying so
    /// in the type is what lets the generated TypeScript describe the request a
    /// caller actually builds. [`REFERENCE_CIRCLES_BY_DEFAULT`] is the value.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub reference_circles: Option<bool>,
    /// Depth, in modules, at which the undercut question is asked.
    ///
    /// `None` takes [`working_depth_for`] — the gear's own dedendum. Optional
    /// for the same reason as the fields around it: the gear tab has no control
    /// for it, so it is genuinely absent rather than defaulted on the far side.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub working_depth: Option<f64>,
    /// What this gear runs against, when the answer depends on it.
    ///
    /// Only an **eccentric** gear has such an answer today — the centre distance
    /// its mesh commands around a revolution — and a concentric one has nothing
    /// that varies for a mate to be needed for. Absent means "do not ask".
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub mate: Option<MateRef>,
    /// Size the eccentricity by the **centre-distance throw** instead of the
    /// angular-shift amplitude: the amplitude is then solved so the commanded
    /// centre distance's best-fit sinusoid has this half-amplitude, mm. Signed —
    /// the sign carries to `angular_shift` (which end runs thick). Needs the
    /// mate. Absent leaves `params.angular_shift` as the input.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub eccentric_throw: Option<f64>,
}

/// The gear on the other side, as far as a commanded centre distance needs it.
///
/// Not a whole [`GearParams`]: a mate shares this gear's module, pressure angle
/// and helix by definition — a pair that did not could not mesh — so sending
/// them again would be sending a constraint that can be broken.
#[derive(Deserialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct MateRef {
    pub teeth: u32,
    #[serde(default)]
    pub profile_shift: f64,
    /// The mate is a ring, and this gear runs inside it.
    #[serde(default)]
    pub internal: bool,
}

/// The depth the undercut question is asked at when a request does not say.
///
/// **The gear's own dedendum**, which is what a stage does and for the reason a
/// stage does it: that is the depth the profile generator's `undercut` flag
/// actually answers about, so the reported threshold and the reported flag agree
/// by construction (docs/reference.md#automatic-values).
///
/// This was a fixed one module — the classical rule — on the argument that the
/// gear tab has no control for it. Having no control is a reason to *derive* the
/// value, not to pick a convention: the two differ by a quarter of a module on
/// an ordinary gear, and they differ in **sign**. A default 17-tooth gear on the
/// ISO 53 rack read "undercut below `x = −0.2443`" on the gear tab and "below
/// `+0.0057`" as a member of a stage, and the generator agrees with the second —
/// at `x = −0.1` it reports the flank undercut while the tab's own threshold
/// said it was not.
///
/// `docs/rationale.md#a-control-that-exposes-an-assumption-must-not-default-to-it`
/// records that correction; it had been applied to one of the two surfaces.
fn working_depth_for(params: &GearParams) -> f64 {
    params.dedendum
}

/// Reference circles go into a drawing unless a request says otherwise.
const REFERENCE_CIRCLES_BY_DEFAULT: bool = true;

#[derive(Clone, Deserialize, Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
#[serde(untagged)]
pub enum Maybe<T> {
    Value(T),
    Unavailable { unavailable: gear_core::note::Note },
}

impl<T> Maybe<T> {
    fn from<E: gear_core::note::Explain>(r: Result<T, E>) -> Self {
        match r {
            Ok(v) => Self::Value(v),
            Err(e) => Self::Unavailable {
                unavailable: e.note(),
            },
        }
    }
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct SpanOut {
    pub teeth_spanned: u32,
    pub nominal: f64,
    pub contact_radius: f64,
    /// `[smallest, largest]` around the revolution. **The two ends are the same
    /// bits for an ordinary gear**, so the front end can render a range
    /// unconditionally and an evenly cut gear reads as one number with no flag
    /// to check.
    pub around: [f64; 2],
}

/// A measurement over (or between) pins.
///
/// The nominal figure only. `metrology::OverPins` also carries the pin centre
/// and contact radii — they are how the measurement is *derived*, and how its
/// tangency is verified against the generated flank — but nothing displays
/// them, and a field on the wire that no one reads is a field that can go wrong
/// unnoticed.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct PinsOut {
    pub nominal: f64,
    /// `[smallest, largest]` around the revolution — identical ends for an
    /// ordinary gear, as [`SpanOut::around`].
    pub around: [f64; 2],
}

#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct ToleranceOut {
    pub class: ClassRef,
    pub tooth_to_tooth: f64,
    pub total: f64,
}

/// Derived geometry and metrology for one gear.
///
/// Lengths are millimetres, angles degrees, composite errors micrometres.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
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
    /// UI displays what Rust computed and nothing else (docs/rationale.md#the-stack).
    pub pitch_diameter: f64,
    pub base_diameter: f64,
    pub tip_diameter: f64,
    pub root_diameter: f64,
    pub tooth_thickness: f64,
    pub fillet_radius: f64,
    /// Transverse pressure angle, **degrees** — a number a designer reads, so
    /// it crosses in the unit they read it in. `gear_core` works in radians.
    pub transverse_pressure_angle: f64,
    pub cutter_tip_width: f64,
    /// Whether **any** tooth is undercut / severed — for a concentric gear that
    /// is its only tooth; for an eccentric one the short teeth can while the mean
    /// does not, and `per_tooth_clamps` names which and where.
    pub undercut: bool,
    pub severed: bool,
    /// Tool-level guards that altered the requested geometry — for an eccentric
    /// gear the shared cutter's, with per-tooth clamps in `per_tooth_clamps`.
    /// Empty is the normal case.
    pub clamps: Vec<gear_core::note::Note>,

    /// The angular-shift amplitude in force, modules. Normally the input; when
    /// the eccentricity was sized by a centre-distance throw instead, this is
    /// the value solved for — the field the UI shows in place of the throw.
    pub angular_shift: f64,

    /// What varies around the revolution. Every field is zero for an ordinary
    /// gear, so it crosses unconditionally rather than behind a flag.
    pub variation: gear_core::gear::Variation,
    /// Teeth that came out other than as drawn, and why. Empty is the normal
    /// case, for an ordinary gear and for a buildable eccentric one alike.
    pub per_tooth_clamps: gear_core::gear::PerToothClamps,
    /// The centre distance this gear's mesh commands around a revolution.
    ///
    /// Needs a mate, so it is `Unavailable` with the reason when none was sent —
    /// which is every concentric gear, since a concentric one commands a
    /// constant and there is nothing to profile.
    pub centre_profile: Maybe<gear_core::gear::CentreProfile>,

    pub span: Maybe<SpanOut>,
    pub over_two_pins: Maybe<PinsOut>,
    pub over_three_pins: Maybe<PinsOut>,
    /// The pin or ball diameters that seat on the flanks at every position
    /// round the gear — the bound the pin box is held to, as every other input
    /// has one. `None` where no pin measures this gear at all.
    pub pin_diameter_range: Option<(f64, f64)>,

    /// Classes the standard actually covers for this gear.
    pub available_classes: Vec<ClassRef>,
    pub tolerance: Maybe<ToleranceOut>,
}

fn summarise(ecc: &gear_core::gear::Gear, req: &GearRequest, params: GearParams) -> GearSummary {
    // The mean tooth is quoted from, but it is cut by the **shared** tool: an
    // eccentric gear's teeth are rebuilt to the greatest depth any of them needs
    // and the smallest tip round any of them allows, and `Gear::mean`
    // carries the same. So `fillet_radius`, `cutter_tip_width` and the root
    // radius describe a tooth the gear actually has rather than the one the raw
    // inputs would have produced. For a concentric gear the mean *is*
    // `Tooth::new(params)`, bit for bit.
    let g = ecc.mean();
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
                unavailable: Note::new("ui.gear_tolerance_no_entry")
                    .text("class", c.to_string())
                    .number("module", g.params.module, 3)
                    .number("diameter", pitch_diameter, 3),
            },
        },
        None => Maybe::Unavailable {
            unavailable: Note::new("ui.gear_tolerance_not_covered")
                .number("module", g.params.module, 3)
                .number("diameter", pitch_diameter, 3),
        },
    };

    // Span and over-pins **vary around the revolution** on an eccentric gear — a
    // metrologist reads a different value at every angular position — so each
    // crosses as a value *and* the range it takes. The two ends are the same
    // bits for an ordinary gear, so nothing branches on which kind this is: the
    // range is always there, and an evenly cut gear's is a point.
    //
    // These were withheld entirely until the ranged form existed, on the
    // reasoning that one number would read as *the* span. That was right, and
    // the answer to it is the range rather than the silence.
    let pins = |count| match req.pin_diameter {
        Some(d) => Maybe::from(metrology::over_pins_at(ecc, d, count, 0).map(|p| {
            let mut lo = f64::MAX;
            let mut hi = f64::MIN;
            for start in 0..ecc.teeth() {
                if let Ok(m) = metrology::over_pins_at(ecc, d, count, start) {
                    lo = lo.min(m.nominal);
                    hi = hi.max(m.nominal);
                }
            }
            PinsOut {
                nominal: p.nominal,
                around: [lo, hi],
            }
        })),
        None => Maybe::Unavailable {
            unavailable: Note::new("ui.gear_no_pin_diameter"),
        },
    };

    GearSummary {
        // Bounds on the input fields, against the resolved params — the mean
        // shift and the amplitude in force (solved, if the eccentricity was
        // sized by a throw), but the dedendum and root radius as typed, not the
        // shared cutter's. `admissible_ranges` closes the shift-dependent bounds
        // onto the swept interval from that amplitude (docs/reference.md#angularly-varying-profile-shift).
        ranges: gear_core::auto::admissible_ranges(
            &params,
            req.working_depth
                .unwrap_or_else(|| working_depth_for(&params)),
        ),
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
        // "Is this gear undercut / severed?" — **any** tooth, not the mean one.
        // A concentric gear has one distinct tooth, so this is `g.undercut`
        // unchanged; an eccentric gear's short teeth can undercut while the mean
        // does not, and `per_tooth_clamps` says which and where.
        undercut: ecc.distinct().any(|t| t.undercut),
        severed: ecc.distinct().any(|t| t.severed),
        clamps: g.clamps.notes.clone(),
        angular_shift: params.angular_shift,
        variation: ecc.variation(),
        per_tooth_clamps: ecc.per_tooth_clamps(),
        centre_profile: centre_profile(params, req),
        span: Maybe::from(metrology::best_span_around(ecc).map(|(s, around)| SpanOut {
            teeth_spanned: s.teeth_spanned,
            nominal: s.nominal,
            contact_radius: s.contact_radius,
            around,
        })),
        over_two_pins: pins(PinCount::Two),
        over_three_pins: pins(PinCount::Three),
        pin_diameter_range: metrology::pin_diameter_range_around(ecc),
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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct RingRequest {
    pub params: GearParams,
    /// Pin or ball diameter for the between-pins measurement, mm.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub pin_diameter: Option<f64>,
    #[serde(default)]
    pub cutter: CutterRef,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub chord_tolerance: Option<f64>,
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub reference_circles: Option<bool>,
}

/// The pinion cutter, as the UI sends it.
#[derive(Clone, Copy, Deserialize, Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
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

/// What the UI shows for a ring.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct RingSummary {
    pub teeth: u32,
    pub transverse_module: f64,
    /// Transverse pressure angle, **degrees** — a number a designer reads, so
    /// it crosses in the unit they read it in. `gear_core` works in radians.
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
    /// The pin or ball diameters that seat in this ring's spaces, as on
    /// [`GearSummary::pin_diameter_range`].
    pub pin_diameter_range: Option<(f64, f64)>,
    pub clamps: Vec<gear_core::note::Note>,
}

fn ring_of(req: &RingRequest) -> gear_core::ring::Ring {
    gear_core::ring::Ring::cut_by(&req.params, &req.cutter.to_cutter())
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
        generation_limit: g.generation_limit(),
        fully_generated: g.fully_generated(),
        smallest_tooth_count: smallest_tooth_count(&req.params),
        between_pins: match req.pin_diameter {
            Some(d) => {
                Maybe::from(metrology::between_pins(&g, d).map(|p| PinsOut {
                    nominal: p.nominal,
                    // A ring has one tooth form, so its measurement is a point.
                    around: [p.nominal, p.nominal],
                }))
            }
            None => Maybe::Unavailable {
                unavailable: Note::new("ui.gear_no_pin_diameter"),
            },
        },
        pin_diameter_range: metrology::pin_diameter_range(&metrology::Space::of_ring(&g)),
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
            reference_circles: req
                .reference_circles
                .unwrap_or(REFERENCE_CIRCLES_BY_DEFAULT),
        },
    ))
}

fn parse(input: &str) -> Result<GearRequest, String> {
    serde_json::from_str(input).map_err(|e| format!("bad gear request: {e}"))
}

/// The mate, built as an ordinary gear from the shared module, pressure angle
/// and helix plus its own tooth count and shift — and its mesh kind. `None`
/// when no mate was sent.
fn eccentric_mate(req: &GearRequest) -> Option<(Tooth, gear_core::mesh::MeshKind)> {
    let mate = req.mate.as_ref()?;
    let g = Tooth::new(GearParams {
        teeth: mate.teeth,
        profile_shift: mate.profile_shift,
        angular_shift: 0.0,
        index_offset: 0.0,
        ..req.params
    });
    let kind = if mate.internal {
        gear_core::mesh::MeshKind::Internal
    } else {
        gear_core::mesh::MeshKind::External
    };
    Some((g, kind))
}

/// `req.params`, with `angular_shift` solved from `eccentric_throw` when that is
/// the input. One downstream inversion at the boundary — every entry point runs
/// it, and everything past it is built from `angular_shift` exactly as before.
fn resolved_params(req: &GearRequest) -> Result<GearParams, String> {
    let Some(target) = req.eccentric_throw else {
        return Ok(req.params);
    };
    let (mate, kind) = eccentric_mate(req).ok_or_else(|| {
        "a centre-distance throw is commanded against a mate — set the mate's tooth count"
            .to_string()
    })?;
    let magnitude = gear_core::gear::amplitude_for_throw(
        req.params,
        &mate,
        kind,
        gear_core::mesh::MeshSide::First,
        target.abs(),
    )
    .map_err(|_| {
        format!(
            "a centre-distance throw of {:.4} mm is not reachable with this mate — a larger \
             tooth-count difference or a mean shift nearer zero each raise the throw a pair \
             can deliver",
            target.abs()
        )
    })?;
    Ok(GearParams {
        angular_shift: magnitude.copysign(target),
        ..req.params
    })
}

fn solve_gear_impl(input: &str) -> Result<String, String> {
    let req = parse(input)?;
    let params = resolved_params(&req)?;
    // One construction for both kinds: a concentric gear is the `Δx = 0`
    // degenerate of the eccentric assembly, and its `mean` is `Tooth::new`
    // verbatim. Building it here means every scalar the summary quotes is a
    // tooth the gear actually has.
    let ecc = gear_core::gear::Gear::new(params);
    serde_json::to_string(&summarise(&ecc, &req, params))
        .map_err(|e| format!("could not encode result: {e}"))
}

fn gear_profile_impl(input: &str, points_per_tooth: usize) -> Result<Vec<f64>, String> {
    let req = parse(input)?;
    Ok(gear_core::gear::Gear::new(resolved_params(&req)?)
        .profile(points_per_tooth)
        .into_iter()
        .flat_map(|p| [p[0], p[1]])
        .collect())
}

fn export_dxf_impl(input: &str) -> Result<String, String> {
    let req = parse(input)?;
    let g = Tooth::new(resolved_params(&req)?);
    Ok(gear_io::gear_to_dxf(
        &g,
        &gear_io::DxfOptions {
            chord_tolerance: req
                .chord_tolerance
                .unwrap_or(gear_core::outline::DEFAULT_CHORD_TOLERANCE),
            reference_circles: req
                .reference_circles
                .unwrap_or(REFERENCE_CIRCLES_BY_DEFAULT),
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
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct TrainRequest {
    pub train: gear_core::train::Train,
    /// The library to use. Omitted means the one the tool ships with, which is
    /// the common case — the UI only sends this once the user has imported or
    /// edited a library of their own.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub materials: Option<gear_core::MaterialLibrary>,
}

/// What a train solve came to — an answer, or why there is none.
///
/// **A refusal is data, not an exception.** A geartrain that cannot be built is
/// an ordinary thing for a designer to be holding halfway through an edit, and
/// the front end has to keep showing them every input that produced it. Sent
/// back as a value for that reason, and as a [`Note`] rather than a sentence
/// for the reason every other message here is one: the words belong to the
/// catalogue, and `TrainError`'s `Display` is English written in Rust —
/// which is what this used to hand over, making the failure the one thing the
/// application said in a language nobody chose.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct TrainOutcome {
    pub result: Option<gear_core::train::TrainResult>,
    pub failure: Option<TrainFailure>,
    /// **What every constrainable input of the train came to**, by the name
    /// relief knows it by, indexed as the graph is — what a box relief turns
    /// given is seeded from ([`relieve`]). Beside the result rather than
    /// inside it because it is the *inputs'* names lined up against the
    /// result's figures, which the core does in one place
    /// (`TrainResult::figure`) and the panel need not know at all: it hands
    /// this list back with the graph, and never learns which field is which.
    pub figures: Vec<gear_core::train::Figure>,
    /// **Every card**: the part of the train's graph it is, in its own
    /// numbering with where each of its pieces is in the graph — what the
    /// panel stands a card on — and its ports, with the label the panel names
    /// each by, so a designer is offered exactly the bodies a train may hold,
    /// drive or couple, read from the part's wiring rather than written into
    /// the front end a second time. Present on success and failure alike: it
    /// needs no geometry.
    pub topology: Vec<gear_core::train::StagePorts>,
    /// **The train's motion** — exact ratios, every body's speed, mobility —
    /// present whenever the tooth counts and topology give one, which is
    /// whether or not the geometry solved. A train mid-edit whose stage will
    /// not close still turns, and this is what says at what.
    pub motion: Option<gear_core::train::MotionReport>,
    /// **The train's centres and its axes** — two of the three groupings a
    /// list shows the graph in, derived by the core; present whether or not
    /// the train solved, since neither needs a solve.
    pub groupings: gear_core::train::Groupings,
    /// **Each case's flow** — the third grouping — the bodies in the order
    /// the case's power reaches them, the meshes carrying it, epicyclic
    /// parts as junctions and idle branches, by the graph's indices. Empty
    /// where the train did not solve.
    pub flows: Vec<Vec<gear_core::train::FlowRow>>,
    /// **What each gear is**, read off the whole graph — sun, planet, ring,
    /// worm, wheel, or a gear by its number — by the graph's index.
    pub names: Vec<gear_core::train::shape::MemberName>,
    /// **Each card's view of the result**, in the order the topology deals
    /// them ([`gear_core::train::TrainResult::cards`]) — the result laid
    /// back out in each part's own numbering by the core's one rule, so the
    /// cards do not slice it a second time. Empty where the train did not
    /// solve.
    pub cards: Vec<gear_core::train::ShapeResult>,
}

/// Why a train has no answer, and where.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct TrainFailure {
    /// The message, as a key and its already-formatted values.
    pub note: gear_core::note::Note,
    /// Which stage could not be built, **numbered from one** as the panel
    /// numbers them. `None` where the fault is the train's own rather than any
    /// one stage's — an empty train, say.
    pub stage: Option<u32>,
}

fn solve_train_impl(input: &str) -> Result<String, String> {
    use gear_core::note::Explain;
    let req: TrainRequest =
        serde_json::from_str(input).map_err(|e| format!("bad train request: {e}"))?;
    let lib = req.materials.unwrap_or_else(gear_io::default_library);
    let topology = req.train.topology();
    let motion = req.train.motion_report();
    let groupings = req.train.groupings();
    let names = req.train.shape.member_names();
    let outcome = match gear_core::train::solve_train(&req.train, &lib) {
        Ok(result) => TrainOutcome {
            figures: req
                .train
                .shape
                .toggles()
                .into_iter()
                .map(|(freedom, _)| gear_core::train::Figure {
                    freedom,
                    value: result.figure(freedom),
                })
                .collect(),
            cards: result.cards(&req.train),
            flows: req.train.flows(&result),
            groupings,
            names,
            result: Some(result),
            failure: None,
            topology,
            motion,
        },
        Err(e) => {
            let stage = match &e {
                gear_core::train::TrainError::InStage { stage, .. } => {
                    u32::try_from(*stage + 1).ok()
                }
                _ => None,
            };
            TrainOutcome {
                result: None,
                failure: Some(TrainFailure {
                    note: e.note(),
                    stage,
                }),
                figures: Vec::new(),
                topology,
                motion,
                cards: Vec::new(),
                flows: Vec::new(),
                groupings,
                names,
            }
        }
    };
    serde_json::to_string(&outcome).map_err(|e| format!("could not encode result: {e}"))
}

fn default_materials_impl() -> Result<String, String> {
    serde_json::to_string(&gear_io::default_library()).map_err(|e| e.to_string())
}

/// Every word the application shows, as `{ "section.key": "text" }`.
///
/// The catalogue crosses the boundary whole rather than the core rendering
/// sentences on this side, because a note's *words* are a display decision and
/// its *values* are not. The core sends `{ key, values }` with the numbers
/// already rounded; the front end picks a language and fills in the blanks.
///
/// This is `defaults()` again, for the same reason docs/corrections.md gives: the one
/// value that was written down in both languages drifted, and only the side
/// without tests was wrong. A string catalogue is that trap with thirty more
/// entries.
fn strings_impl(language: &str) -> Result<String, String> {
    serde_json::to_string(gear_io::strings::Catalogue::for_language(language).messages())
        .map_err(|e| e.to_string())
}

/// The languages this build ships, as JSON: `[{ code, name, english }, …]`.
///
/// The **list** crosses the boundary rather than being written in the front end,
/// for the reason `defaults` and the catalogue itself cross it: a language added
/// here would otherwise need remembering there too, and the half nobody tests is
/// the half that forgets. Each name is in its own language, because a reader
/// looking for theirs is looking for the word they call it by.
fn resolve_language_impl(tag: &str) -> String {
    gear_io::strings::Language::resolve(tag).code.to_string()
}

fn languages_impl() -> Result<String, String> {
    let list: Vec<_> = gear_io::strings::LANGUAGES
        .iter()
        .map(|l| serde_json::json!({ "code": l.code, "name": l.name, "english": l.english_name }))
        .collect();
    serde_json::to_string(&list).map_err(|e| e.to_string())
}

/// The commanded centre distance, when there is a mate to command it against.
///
/// The eccentric gear is always member 1 here: the tab offers an eccentric
/// *external* gear, so the ring, when there is one, is the mate. `params` is
/// already resolved — its `angular_shift` is the amplitude in force, whether it
/// was entered directly or solved from a centre-distance throw.
fn centre_profile(params: GearParams, req: &GearRequest) -> Maybe<gear_core::gear::CentreProfile> {
    use gear_core::mesh::MeshSide;

    let Some((other, kind)) = eccentric_mate(req) else {
        return Maybe::Unavailable {
            unavailable: Note::new("ui.gear_no_mate"),
        };
    };
    if params.angular_shift == 0.0 {
        return Maybe::Unavailable {
            unavailable: Note::new("ui.gear_concentric_has_no_profile"),
        };
    }
    match gear_core::gear::Gear::new(params).centre_profile(&other, kind, MeshSide::First) {
        Ok(p) => Maybe::Value(p),
        // `inv α_w < 0` at some tooth: no centre distance puts that tooth in the
        // mate's space at zero backlash. The shift term carries `1/Σz`, and for
        // an internal pair `Σz` is the tooth-count *difference* — so a close
        // count amplifies the amplitude and this is where it bites first. Say
        // what relieves it rather than leaving the reader to guess.
        Err(gear_core::mesh::MeshError::OutsideInvoluteDomain) => Maybe::Unavailable {
            unavailable: Note::new("ui.gear_eccentricity_exceeds_mate"),
        },
        Err(e) => Maybe::Unavailable {
            unavailable: e.note(),
        },
    }
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
    let imported = gear_io::train::from_toml(toml_text).map_err(|e| e.to_string())?;
    serde_json::to_string(&imported).map_err(|e| e.to_string())
}

fn export_train_impl(document_json: &str) -> Result<String, String> {
    let doc: gear_io::TrainDocument =
        serde_json::from_str(document_json).map_err(|e| e.to_string())?;
    gear_io::train::to_toml(&doc).map_err(|e| e.to_string())
}

/// Derived results for a whole geartrain.
///
/// The third of the three entry points docs/rationale.md#the-stack planned. Like the others it
/// is JSON in, JSON out, with no state held across the boundary: the UI owns the
/// inputs and this recomputes everything from them on each change.
#[wasm_bindgen]
pub fn solve_train(input: &str) -> Result<String, JsError> {
    solve_train_impl(input).map_err(|e| JsError::new(&e))
}

/// The material library the tool ships with, as JSON.
///
/// Includes each value's `basis` and `note`, because the UI is expected to show
/// which numbers are measured and which are estimates — see `docs/rationale.md#material-data-ships-estimates-deliberately`
/// . Dropping that on the floor would present a class estimate with the
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
/// saying why. See `docs/corrections.md`.
///
/// Serving them across the boundary is what makes that class of drift
/// impossible rather than merely fixed.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct Defaults {
    pub gear: GearTabDefaults,
    /// A fresh geartrain, with one spur stage in it.
    pub train: gear_core::train::Train,
    /// **One of each preset, for the "add stage" menu**, in the menu's
    /// order and each under its family — the core's list
    /// ([`StagePreset::ALL`]), so a preset is a variant there and a row
    /// here, never a field. A crossed pair is one of them for the menu's
    /// sake: it is a spur stage with its shafts at an angle
    /// (docs/reference.md#crossed-axes), and a worm is a distance marked as
    /// one, and neither is obvious to build from a pair.
    pub stages: Vec<StagePresetEntry>,
    /// The three families the menu groups them under, in order, each
    /// with the key of its name — the core's list ([`StageFamily::ALL`]).
    pub families: Vec<StageFamilyEntry>,
    /// The fraction a reversed root's fatigue bending allowable is taken at.
    ///
    /// Crosses so the control's own note can name it. It is
    /// [`REVERSED_BENDING_FRACTION`](gear_core::material::REVERSED_BENDING_FRACTION)
    /// and nothing else — a number the interface shows is a number Rust decided,
    /// this one included.
    pub reverse_loading_coefficient: f64,
}

/// A family as the menu groups by it: which, and called what.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct StageFamilyEntry {
    pub family: StageFamily,
    pub label: String,
}

/// A preset as the menu takes it: which, under what family, called what,
/// and the stage it starts as.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct StagePresetEntry {
    pub preset: StagePreset,
    pub family: StageFamily,
    /// The catalogue key of its name.
    pub label: String,
    pub stage: gear_core::train::Shape,
}

/// What a new gear tab holds. The values are the specification's, and the
/// tooth count is deliberately *not* the core's own default of 17.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct GearTabDefaults {
    pub params: GearParams,
    pub cutter: CutterRef,
    /// Pin or ball diameter for the over-pins measurement, mm.
    pub pin_diameter: f64,
    /// Export accuracy, mm.
    pub chord_tolerance: f64,
    pub reference_circles: bool,
    /// The centre-distance throw a gear starts at when the eccentricity is
    /// entered that way round and the geometry cannot seed it.
    ///
    /// It crosses from here for the same reason every other default does: a
    /// number the application shows is a number Rust decided. The gear tab
    /// carried this one in TypeScript, which is the class of drift that put a
    /// rack's tip round on a shaper (`docs/corrections.md`).
    pub eccentric_throw: f64,
}

fn defaults_impl() -> Result<String, String> {
    use gear_core::train::{LoadCase, Train};

    // The tab starts with an automatic face width, where the core's own
    // default is a plain 10 mm. Both are right for their caller: the CLI and
    // the tests want a fixed number they can reason about, and a designer
    // opening the panel wants to see the width the rating asks for. Seeded at
    // 5 mm so the field has something to fall back to when the toggle is
    // turned off.
    //
    // **Every preset the panel offers**, which it was not: the rule reached the
    // parallel pair and the epicyclic set, and a hula stage opened at a *fixed*
    // 10 mm while a worm's members were automatic but seeded at ten. So the same
    // panel answered the same question three ways depending on which stage a
    // designer had picked. Nothing could see it — `defaults()` is the boundary's
    // own, and no golden case reaches it. See the test below. Now one walk
    // over every member of every preset's shape, so a preset added to the
    // core's list is seeded by being on it.
    const UI_SEED: f64 = 5.0;
    let ui = |mut shape: gear_core::train::shape::Shape| {
        for m in &mut shape.members {
            m.gear.face_width = gear_core::params::Auto::automatic(UI_SEED);
        }
        shape
    };
    let spur = ui(StagePreset::Spur.build());

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
            eccentric_throw: 0.1,
        },
        train: {
            // The three loads a fresh train used to hold as fields, between
            // the pair's two gears: a peak at the first, reacted at the
            // second; a load from the second, held still, with the first
            // reacting it — through a stage that locks, nothing reaches it;
            // and a fatigue load a fifth of the peak — a running load rather
            // than the stall the ultimate case is, so a fresh tab shows the
            // two ratings answering different questions.
            Train::chained(vec![spur], |t| {
                let (input, output) = (t.port(0, 1), t.port(0, 2));
                vec![
                    LoadCase::ultimate(input, output, 0.1, 30_000.0),
                    LoadCase::back_driving(input, output, 3.0),
                    LoadCase::fatigue(input, output, 0.02, 30_000.0),
                ]
            })
        },
        stages: StagePreset::ALL
            .into_iter()
            .map(|preset| StagePresetEntry {
                preset,
                family: preset.family(),
                label: preset.label().to_string(),
                stage: ui(preset.build()),
            })
            .collect(),
        families: StageFamily::ALL
            .into_iter()
            .map(|family| StageFamilyEntry {
                family,
                label: family.label().to_string(),
            })
            .collect(),
        reverse_loading_coefficient: gear_core::material::REVERSED_BENDING_FRACTION,
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

/// The string catalogue for a language tag, as JSON. See [`strings_impl`].
///
/// An unknown tag answers with English rather than an error: a language
/// preference is not an engineering input, and a stale one stored in a browser
/// should leave a working application rather than a blank one.
///
/// # Errors
///
/// Only if the catalogue cannot be encoded, which would be a build-time defect.
#[wasm_bindgen]
pub fn strings(language: &str) -> Result<String, JsError> {
    strings_impl(language).map_err(|e| JsError::new(&e))
}

/// The languages this build ships. See [`languages_impl`].
///
/// # Errors
///
/// Only if the list cannot be encoded, which would be a build-time defect.
#[wasm_bindgen]
pub fn languages() -> Result<String, JsError> {
    languages_impl().map_err(|e| JsError::new(&e))
}

/// Which shipped language a BCP 47 tag should be read in — `zh-TW` answers
/// `zh-Hant`, `de-CH` answers `de`, anything unknown answers `en`.
///
/// Exposed so the picker can show the option actually in force, and so the
/// mapping from a browser's `navigator.language` lives beside the language list
/// rather than being written down a second time in TypeScript.
#[wasm_bindgen]
#[must_use]
pub fn resolve_language(tag: &str) -> String {
    resolve_language_impl(tag)
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

/// Import a geartrain: TOML text in, `{ document: { name, train }, adjusted }`
/// JSON out.
///
/// `adjusted` says whether any stage was relieved on the way in — a toggle
/// the file had given that no stage can honour, such as a crossed pair's
/// axial contact ratio, turned back automatic with its number kept. The panel
/// says so in one sentence; the values are the file's own throughout
/// (`gear_io::train`, *What is adjusted on import*).
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
/// A document that is not a geartrain.
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

/// **One member of a geartrain, as a gear tab would hold it.**
///
/// `{ train, materials, member }` JSON in — a train request with the
/// member's index in the train's graph — and `{ params, internal, cutter }`
/// out: the tooth the stage cut that
/// member with, every automatic value resolved and every convention applied
/// (`GearResult::params`), whether it is a ring, and the pinion cutter that
/// cut it where it is. The gear tab **adopts** the member — a word chosen so
/// it cannot be mistaken for the TOML `import_train`, which reads a document
/// this tool wrote — and shows the tooth the stage rated rather than a
/// rebuild from the inputs.
///
/// The train is solved here, in microseconds, because the tooth as built is
/// an output: a shift the stage chose, an addendum a tip width held down, a
/// helix shared out of a shaft angle. Nothing on the other side of the
/// boundary could know those, and nothing should try.
///
/// # Errors
///
/// A malformed request; a member index the train does not have; a
/// train that has no answer, with its reason; and a **worm**, which is not a
/// gear the tab can hold — a thread's proportions are its own — and which the
/// panel lists greyed rather than omitted so a reader can see why it is not
/// offered.
#[wasm_bindgen]
pub fn adopt_member(input: &str) -> Result<String, JsError> {
    adopt_member_impl(input).map_err(|e| JsError::new(&e))
}

#[derive(Deserialize)]
struct AdoptRequest {
    train: gear_core::train::Train,
    #[serde(default)]
    materials: Option<gear_core::MaterialLibrary>,
    /// The member, by the graph's index.
    member: usize,
}

/// What [`adopt_member`] answers.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct Adopted {
    /// The tooth as the stage cut it.
    pub params: GearParams,
    /// Whether the member is a ring — cut by a pinion cutter — and so the
    /// tab's *internal* kind.
    pub internal: bool,
    /// The cutter that cut it, where it is a ring.
    pub cutter: Option<CutterRef>,
}

/// What adopting a member came to — the member, or why there is none, as a
/// [`Note`] the catalogue renders, for the reason [`TrainOutcome`] gives: a
/// train that will not build is an answer a designer is regularly holding,
/// and the words belong to the catalogue.
#[derive(Serialize)]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "wasm/")
)]
pub struct AdoptOutcome {
    pub adopted: Option<Adopted>,
    pub failure: Option<Note>,
}

fn adopt_member_impl(input: &str) -> Result<String, String> {
    let req: AdoptRequest = serde_json::from_str(input).map_err(|e| e.to_string())?;
    // A member the train does not have, or a worm, is a defect on the other
    // side of the boundary — the panel lists what can be adopted — so each
    // is a refusal rather than an outcome.
    let shape = &req.train.shape;
    if req.member >= shape.members.len() {
        return Err(format!("the train has no member {}", req.member + 1));
    }
    // A worm's thread is a thread the tab cannot hold; its wheel it can.
    if shape.is_worm_thread(req.member) {
        return Err("a worm is not a gear the tab can hold".to_string());
    }
    let lib = req.materials.unwrap_or_else(gear_io::default_library);
    let outcome = match gear_core::train::solve_train(&req.train, &lib) {
        Ok(result) => {
            let cutter = shape.member_cutter(req.member).map(|c| CutterRef {
                teeth: c.teeth,
                addendum: c.addendum,
                tip_round: c.tip_round,
            });
            AdoptOutcome {
                adopted: Some(Adopted {
                    params: result.members[req.member].params,
                    internal: cutter.is_some(),
                    cutter,
                }),
                failure: None,
            }
        }
        Err(e) => AdoptOutcome {
            adopted: None,
            failure: Some(e.note()),
        },
    };
    serde_json::to_string(&outcome).map_err(|e| e.to_string())
}

/// **The train's graph with its over-determined inputs relieved.**
///
/// `{ shape, just, figures }` JSON in — the train's graph as it now stands,
/// the [`Freedom`] the designer has this moment pinned (`null` where what
/// changed was not a toggle), and what the graph's inputs last came to
/// ([`TrainOutcome::figures`]), every index the graph's — and the corrected
/// graph out, with every box relief turned given seeded from its figure.
///
/// A designer who pins a pair's distance *and* both its shifts has asked for a
/// contradiction: the three are bound by one relation, so one would have to be
/// ignored. Rather than accept an input and quietly disregard it, the first one
/// in relief order that they are not this moment pinning goes back to
/// automatic. **Every group of inputs that argue is a part's**, so relieving
/// the graph is relieving each card, whichever the designer touched.
///
/// Which inputs argue, how many may stand and which gives way first are facts
/// about the geometry, and they used to live in the panel as three functions,
/// one per stage type, restating a relation the core already enforces. **It is
/// the same relation the solve reads from the other end**, so the two have to
/// agree or a designer is offered an input the solve will disregard.
///
/// Nothing here decides a value: relief says which inputs are still being
/// read, and a box it turns given holds what it was showing — a number the
/// core computed, copied where the designer would have copied it.
///
/// # Errors
///
/// A malformed graph or freedom, which would be a defect on this side of the
/// boundary.
#[wasm_bindgen]
pub fn relieve(input: &str) -> Result<String, JsError> {
    relieve_impl(input).map_err(|e| JsError::new(&e))
}

#[derive(Deserialize)]
struct RelieveRequest {
    shape: gear_core::train::Shape,
    #[serde(default)]
    just: Option<gear_core::train::Freedom>,
    #[serde(default)]
    figures: Vec<gear_core::train::Figure>,
}

fn relieve_impl(input: &str) -> Result<String, String> {
    let req: RelieveRequest = serde_json::from_str(input).map_err(|e| e.to_string())?;
    serde_json::to_string(&req.shape.relieved_from(req.just, &req.figures))
        .map_err(|e| e.to_string())
}

/// **A load case with its over-determined figures relieved.**
///
/// `{ train, library, case, just }` JSON in — the train as it stands, its
/// materials, the case by index and the figure the designer has this moment
/// pinned (`null` where what changed was not a toggle) — and the case out,
/// with exactly the train's mobility of its speeds given and the torques one
/// statics equation short of the bodies that carry one, every figure relief
/// turned derived seeded from what the case comes to
/// ([`Train::relieve_case`]). The same relation [`relieve`] keeps on a
/// stage's geometry, kept on a case's loads: a pair with a speed at each end
/// has asked for a contradiction, and the one not this moment pinned gives
/// way. A case with fewer given than that is left short — relief never
/// invents a given — and [`solve_train`] says so on the case.
///
/// # Errors
///
/// A malformed request, or a train whose bodies cannot be counted, which
/// [`solve_train`] would refuse the same way.
#[wasm_bindgen]
pub fn relieve_case(input: &str) -> Result<String, JsError> {
    relieve_case_impl(input).map_err(|e| JsError::new(&e))
}

#[derive(Deserialize)]
struct RelieveCaseRequest {
    train: gear_core::train::Train,
    library: gear_core::MaterialLibrary,
    case: usize,
    #[serde(default)]
    just: Option<gear_core::train::CaseFreedom>,
}

fn relieve_case_impl(input: &str) -> Result<String, String> {
    let mut req: RelieveCaseRequest = serde_json::from_str(input).map_err(|e| e.to_string())?;
    req.train
        .relieve_case(req.case, req.just, &req.library)
        .map_err(|e| format!("{e:?}"))?;
    let case = req
        .train
        .load_cases
        .get(req.case)
        .ok_or_else(|| format!("no load case {}", req.case))?;
    serde_json::to_string(case).map_err(|e| e.to_string())
}

/// **A train's graph edited by the core's rules.**
///
/// `{ train, edit }` JSON in — the train as it stands and one edit — and the
/// train out. The edits are what a body's select and the panel's buttons
/// mean, each a rule the core owns rather than the panel:
///
/// - `{ "join": { "a", "b" } }` — two bodies made one ([`Train::join`]),
///   the lower number kept: a reaction a case declared at either becomes
///   an inline take-off;
/// - `{ "split": { "stage", "body" } }` — a stage's end of a body split
///   off as a body of its own ([`Train::split`]);
/// - `{ "hold": body }` — held to ground, every end of it;
/// - `{ "release": body }` — not held, a hold its stage's convention puts
///   on it written off;
/// - `{ "move_end": { "stage", "body", "to" } }` — a stage's end of a body
///   moved to another: split off where the body ran on, then held (`to`
///   ground, 0), joined (`to` another body) or left its own (`to` null)
///   ([`Train::move_end`]) — what the select beside a port means;
/// - `{ "push_stage": stage }` — appended, its bodies numbered after the
///   train's, and joined onward from the last stage's open output, every
///   case entry there carried to the new stage's output
///   ([`Train::push_stage`]);
/// - `{ "remove_stage": k }` — removed, its bodies with it where no other
///   stage has them, and the rest numbered densely again
///   ([`Train::remove_stage`]);
/// - `{ "add_case": kind }` — a fresh case of that kind between the train's
///   two ends ([`Train::fresh_case`]);
/// - `{ "duty": { "case", "intermittent" } }` — a case's duty switched,
///   seeded as a fresh case's is ([`Train::set_duty`]);
/// - `{ "stage": { "stage", "edit" } }` — one stage edited on its card
///   ([`gear_core::train::StageEdit`]: a step, a sun or a ring, an axis, a
///   mesh added or removed, a member moved to another body), a body it
///   adds numbered after the train's and one it takes off the stage leaving
///   the train where no other stage has it ([`Train::edit_stage`]);
/// - `{ "graph": edit }` — one of the graph's own edits
///   ([`gear_core::train::Edit`]: a gear at a body, a new body or a new
///   axis; a ratio on the body asked; a step; a coupling; a member, mesh,
///   axis, body or coupling removed with what goes with it; a gear moved;
///   a join, a hold, a release; a stage inserted at a body), every index
///   the graph's ([`Train::edit`]).
///
/// A refused edit is an error carrying the refusal's catalogue key
/// ([`gear_core::train::EditRefused::key`]), the words being the panel's to
/// say, and the train is returned unchanged.
///
/// # Errors
///
/// A malformed request, which would be a defect on this side of the
/// boundary.
#[wasm_bindgen]
pub fn edit_train(input: &str) -> Result<String, JsError> {
    edit_train_impl(input).map_err(|e| JsError::new(&e))
}

#[derive(Deserialize)]
#[serde(rename_all = "snake_case")]
enum TrainEdit {
    Join {
        a: usize,
        b: usize,
    },
    Split {
        stage: usize,
        body: usize,
    },
    Hold(usize),
    Release(usize),
    MoveEnd {
        stage: usize,
        body: usize,
        to: Option<usize>,
    },
    PushStage(gear_core::train::Shape),
    RemoveStage(usize),
    AddCase(gear_core::train::CaseKind),
    Duty {
        case: usize,
        intermittent: bool,
    },
    Stage {
        stage: usize,
        edit: gear_core::train::StageEdit,
    },
    Graph(gear_core::train::Edit),
}

#[derive(Deserialize)]
struct EditRequest {
    train: gear_core::train::Train,
    edit: TrainEdit,
}

fn edit_train_impl(input: &str) -> Result<String, String> {
    let EditRequest { mut train, edit } = serde_json::from_str(input).map_err(|e| e.to_string())?;
    // **A refusal crosses as its catalogue key**, which is what the panel
    // says under the card; its `Display` is English for a log.
    apply_edit(&mut train, edit).map_err(|e| e.key().to_string())?;
    serde_json::to_string(&train).map_err(|e| e.to_string())
}

/// **One edit made by the core's rules** — the one match [`edit_train`] and
/// [`preview_edit`] share, so a preview is made by the rule that would make
/// the edit.
fn apply_edit(
    train: &mut gear_core::train::Train,
    edit: TrainEdit,
) -> Result<(), gear_core::train::EditRefused> {
    match edit {
        TrainEdit::Join { a, b } => train.join(a, b),
        TrainEdit::Split { stage, body } => {
            train.split(stage, body);
        }
        TrainEdit::Hold(body) => train.hold(body),
        TrainEdit::Release(body) => train.release(body),
        TrainEdit::MoveEnd { stage, body, to } => train.move_end(stage, body, to),
        TrainEdit::PushStage(stage) => train.push_stage(stage),
        TrainEdit::RemoveStage(k) => train.remove_stage(k),
        // The figures a fresh case starts at are the shipped train's.
        TrainEdit::AddCase(kind) => {
            let (torque, speed) = match kind {
                gear_core::train::CaseKind::Ultimate => (0.1, 30_000.0),
                gear_core::train::CaseKind::Fatigue => (0.02, 30_000.0),
            };
            let case = train.fresh_case(kind, torque, speed);
            train.load_cases.push(case);
        }
        TrainEdit::Duty { case, intermittent } => train.set_duty(case, intermittent),
        TrainEdit::Stage { stage, edit } => train.edit_stage(stage, edit)?,
        TrainEdit::Graph(edit) => train.edit(edit)?,
    }
    Ok(())
}

/// **What an edit would do, before it is made.**
///
/// `{ train, materials, edit }` JSON in — the train as it stands, the
/// library it is rated against (omitted, the shipped one) and one edit, as
/// [`edit_train`] takes it — and a [`gear_core::train::Preview`] out: the
/// refusal where the edit would be refused, and otherwise what it would
/// change and what the headline path would come to, each as a key and its
/// values, with why the train would not solve after it where it would not.
/// The edit is made on a copy by the rule [`edit_train`] makes it by, and
/// both trains are solved here; nothing is kept.
///
/// # Errors
///
/// A malformed request, which would be a defect on this side of the
/// boundary.
#[wasm_bindgen]
pub fn preview_edit(input: &str) -> Result<String, JsError> {
    preview_edit_impl(input).map_err(|e| JsError::new(&e))
}

#[derive(Deserialize)]
struct PreviewRequest {
    train: gear_core::train::Train,
    #[serde(default)]
    materials: Option<gear_core::MaterialLibrary>,
    edit: TrainEdit,
}

fn preview_edit_impl(input: &str) -> Result<String, String> {
    let PreviewRequest {
        train,
        materials,
        edit,
    } = serde_json::from_str(input).map_err(|e| e.to_string())?;
    let lib = materials.unwrap_or_else(gear_io::default_library);
    let mut after = train.clone();
    let made = apply_edit(&mut after, edit);
    let preview = gear_core::train::preview(&train, made.map(|()| &after), &lib);
    serde_json::to_string(&preview).map_err(|e| e.to_string())
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

    /// **A chain of these stages as JSON**, with the three classic cases
    /// written between its two ends by the core's own constructors — a
    /// peak, a load from the far end held still, and a fatigue case — so a
    /// fixture here says which bodies it loads the way a file does, and the
    /// chain's bodies are numbered rather than assumed.
    /// The stage a preset starts as, by the preset's name on the wire.
    fn preset(d: &serde_json::Value, name: &str) -> serde_json::Value {
        d["stages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["preset"] == name)
            .unwrap_or_else(|| panic!("no preset {name}"))["stage"]
            .clone()
    }

    /// The hula arrangement — a stepped Wolfrom at one planet — which no
    /// preset offers and the boundary still has to carry, since a file may
    /// hold one and the epicyclic controls can build one.
    fn hula_stage() -> serde_json::Value {
        serde_json::to_value(gear_core::train::arrangements::hula(
            [65, 61, 57, 61],
            [1.0, 1.0],
        ))
        .unwrap()
    }

    fn train_json(
        stages: &[serde_json::Value],
        peak: (f64, f64),
        back: f64,
        fatigue: (f64, f64),
        hours: f64,
    ) -> serde_json::Value {
        use gear_core::train::{Duty, LoadCase, Shape, Train};
        let stages: Vec<Shape> = stages
            .iter()
            .map(|v| serde_json::from_value(v.clone()).expect("a stage"))
            .collect();
        let mut t = Train::chained(stages, |_| Vec::new());
        let (start, end) = t.chain_ends().expect("a chain fixture has two ends");
        let continuous = Duty::Continuous {
            runtime_hours: hours,
        };
        t.load_cases = vec![
            LoadCase {
                duty: continuous,
                ..LoadCase::ultimate(start, end, peak.0, peak.1)
            },
            LoadCase {
                duty: continuous,
                ..LoadCase::back_driving(start, end, back)
            },
            LoadCase {
                duty: continuous,
                ..LoadCase::fatigue(start, end, fatigue.0, fatigue.1)
            },
        ];
        serde_json::to_value(&t).expect("a train encodes")
    }

    const REQ: &str = r#"{"params":{"module":1.0,"pressure_angle":20.0,"teeth":17,
        "profile_shift":0.2,"helix_angle":0.0,"addendum":1.0,"dedendum":1.25,
        "root_radius":0.38,"thickness_mod":1.0},"pin_diameter":1.75}"#;

    /// **One gear, one answer, whichever surface asks it.**
    ///
    /// A gear tab and a stage member describe the same part, so the bounds they
    /// report on that part have to agree. They did not: the undercut threshold
    /// is asked at a *working depth*, a stage passed its gear's own dedendum,
    /// and the gear tab passed a fixed one module. On the default 17-tooth gear
    /// that is `x = −0.2443` against `+0.0057` — different by a quarter of a
    /// module and different in sign.
    ///
    /// The generator settles which was right: at `x = −0.1` it reports the flank
    /// undercut, so the tab's own threshold contradicted the tab's own flag.
    /// `docs/rationale.md#a-control-that-exposes-an-assumption-must-not-default-to-it`
    /// records that correction being made; it had reached one of the two
    /// surfaces.
    ///
    /// Written as an equality between the two paths rather than against either
    /// number, because the number is a consequence and the agreement is the
    /// claim. 533 tests passed against the disagreement.
    #[test]
    fn a_gear_tab_and_a_stage_member_bound_the_same_gear_alike() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut stage = preset(&d, "spur");
        // Whatever the shipped default gear is, asked of both surfaces. Read
        // rather than written down, so this cannot drift from the defaults.
        let teeth = stage["members"][0]["gear"]["teeth"].clone();
        let dedendum = stage["members"][0]["gear"]["dedendum"].clone();
        // A shift the stage will not move, so both surfaces describe one gear.
        stage["members"][0]["gear"]["profile_shift"] =
            serde_json::json!({"auto": false, "manual": 0.0});

        let train = serde_json::json!({
            "train": train_json(&[stage], (1.0, 1000.0), 0.0, (0.0, 1000.0), 1.0),
            "materials": null
        });
        let solved: serde_json::Value =
            serde_json::from_str(&solve_train_impl(&train.to_string()).unwrap()).unwrap();
        let member = &solved["result"]["members"][0]["ranges"]["profile_shift"];
        assert!(
            member.is_object(),
            "the stage's gear has no ranges: {member}"
        );

        let tab_req = serde_json::json!({
            "params": {
                "module": 1.0, "pressure_angle": 20.0, "teeth": teeth,
                "profile_shift": 0.0, "helix_angle": 0.0, "addendum": 1.0,
                "dedendum": dedendum, "root_radius": 0.38, "thickness_mod": 1.0
            }
        });
        let tab: serde_json::Value =
            serde_json::from_str(&solve_gear_impl(&tab_req.to_string()).unwrap()).unwrap();
        let tab = &tab["ranges"]["profile_shift"];

        for field in ["undercut", "sharp_rack_undercut", "min", "max"] {
            let (a, b) = (&member[field], &tab[field]);
            assert_eq!(
                a, b,
                "{field}: a stage member says {a}, the gear tab says {b} — \
                 the same gear, bounded two ways"
            );
        }
    }

    /// **A geartrain survives the round trip to a file and back — as answers,
    /// not just as bytes.**
    ///
    /// Comparing the two documents would only show that `serde` is consistent
    /// with itself. What a user cares about is that the imported train *is* the
    /// train they exported, so the check solves both and compares the results:
    /// ratio, efficiency, backlash and every stage's numbers. A field silently
    /// dropped on the way out reappears as a default, which looks like a value
    /// rather than like a loss — and would move an answer here.
    /// **A train a browser stored while an axis's carrier was an `Option`
    /// still loads**: `carried_by: null` reads as ground, and absent reads
    /// as ground, and both solve to what `0` does.
    #[test]
    fn a_stored_train_with_a_null_carrier_still_loads() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut stored = d["train"].clone();
        let want = solved(&serde_json::json!({ "train": stored }).to_string())["paths"][0]["ratio"]
            .clone();
        for axis in stored["shape"]["axes"].as_array_mut().unwrap() {
            axis["carried_by"] = serde_json::Value::Null;
        }
        let v = solved(&serde_json::json!({ "train": stored }).to_string());
        assert_eq!(v["paths"][0]["ratio"], want, "null is ground");
        for axis in stored["shape"]["axes"].as_array_mut().unwrap() {
            axis.as_object_mut().unwrap().remove("carried_by");
        }
        let v = solved(&serde_json::json!({ "train": stored }).to_string());
        assert_eq!(v["paths"][0]["ratio"], want, "absent is ground");
    }

    #[test]
    fn a_geartrain_survives_export_and_import_as_the_same_answers() {
        // Start from the defaults the UI hands out, so the tested path is the
        // one a user actually takes, and give it one of every preset.
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let crossed = {
            let mut c = preset(&d, "spur");
            c["distances"][0]["angle"] = serde_json::json!(90.0);
            c
        };
        // Every preset, and a crossed pair too — which is a spur stage with
        // its shafts at an angle, not a preset of its own.
        let document = serde_json::json!({
            "name": "Elevation drive",
            "train": train_json(
                &[preset(&d, "spur"), crossed, preset(&d, "worm"), preset(&d, "planetary")],
                (0.25, 12_000.0),
                0.1,
                (0.2, 9600.0),
                1000.0,
            )
        });

        let toml_text = export_train_impl(&document.to_string()).unwrap();
        let imported: serde_json::Value =
            serde_json::from_str(&import_train_impl(&toml_text).unwrap()).unwrap();
        assert_eq!(
            imported["adjusted"].as_bool(),
            Some(false),
            "a document the tool wrote asks for nothing it cannot honour"
        );
        let back = imported["document"].clone();
        assert_eq!(back["name"].as_str(), Some("Elevation drive"));

        let solve = |doc: &serde_json::Value| {
            let req = serde_json::json!({ "train": doc["train"], "materials": null });
            solved(&req.to_string())
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
            "train": { "load_cases": [
                { "kind": "ultimate", "enabled": true, "loads": [{ "at": 1, "torque": { "auto": false, "manual": 1.0 }, "speed": { "auto": false, "manual": 1.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 10, "reversing": false } } },
                { "kind": "ultimate", "enabled": true, "loads": [{ "at": 2, "torque": { "auto": false, "manual": 0.0 }, "speed": { "auto": false, "manual": 0.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 10, "reversing": false } } },
                { "kind": "fatigue", "enabled": true, "loads": [{ "at": 1, "torque": { "auto": false, "manual": 1.0 }, "speed": { "auto": false, "manual": 1.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 10, "reversing": false } } }
            ],
                       "shape": { "axes": [], "bodies": [], "members": [], "meshes": [], "distances": [] } }
        });
        let text = export_train_impl(&empty.to_string()).unwrap();
        let back: serde_json::Value =
            serde_json::from_str(&import_train_impl(&text).unwrap()).unwrap();
        assert!(
            back["document"]["train"]["shape"]["members"]
                .as_array()
                .unwrap()
                .is_empty(),
            "a train with no stages reads as written"
        );
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

        // A fillet was cut, said by the one field that carries it: a junction
        // exists exactly when there is a fillet to meet the flank.
        let junction = v["junction_radius"]
            .as_f64()
            .unwrap_or_else(|| panic!("the shipped cutter generates no fillet: {:?}", v["clamps"]));
        assert!(
            junction > v["tip_radius"].as_f64().unwrap()
                && junction < v["root_radius"].as_f64().unwrap(),
            "the junction is on the tooth, between tip and root"
        );
    }

    /// **A member adopted is the tooth the stage cut, with its kind and its
    /// cutter** — a planetary ring comes over internal with the set's cutter,
    /// a worm stage's wheel comes over as the helical gear it is, and the
    /// worm itself is refused. A gear tab solving the adopted parameters is
    /// then showing the tooth the stage rated, which is the whole point of
    /// adopting rather than retyping.
    #[test]
    fn a_member_adopted_is_the_tooth_the_stage_cut() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let request = |kind: &str, member: usize| {
            let mut train = d["train"].clone();
            train["shape"] = preset(&d, kind);
            serde_json::json!({ "train": train, "member": member }).to_string()
        };
        let adopt = |kind: &str, member: usize| -> serde_json::Value {
            serde_json::from_str(&adopt_member_impl(&request(kind, member)).unwrap()).unwrap()
        };

        let ring = adopt("planetary", 2);
        let a = &ring["adopted"];
        assert_eq!(a["internal"], true);
        assert_eq!(a["cutter"], preset(&d, "planetary")["members"][2]["ring"]);
        assert_eq!(
            a["params"]["teeth"],
            preset(&d, "planetary")["members"][2]["gear"]["teeth"]
        );
        assert!(ring["failure"].is_null());

        let sun = adopt("planetary", 0);
        assert_eq!(sun["adopted"]["internal"], false);
        assert!(sun["adopted"]["cutter"].is_null());
        // The planet opposes the sun's hand and the ring shares the planet's:
        // the adopted helix carries the member's own sign.
        let planet = adopt("planetary", 1);
        let (hs, hp) = (
            sun["adopted"]["params"]["helix_angle"].as_f64().unwrap(),
            planet["adopted"]["params"]["helix_angle"].as_f64().unwrap(),
        );
        assert_eq!(hs, -hp);

        let wheel = adopt("worm", 1);
        assert_eq!(wheel["adopted"]["internal"], false);
        assert!(wheel["adopted"]["params"]["helix_angle"].as_f64().unwrap() > 0.0);
        let worm = adopt_member_impl(&request("worm", 0)).unwrap_err();
        assert!(worm.contains("worm"), "{worm}");

        // ...and the gear tab, solving what it adopted, builds it as asked:
        // the stage's own guards already held every dimension, so the tab
        // has nothing to clamp and the pitch diameter is the stage's.
        let spur = adopt("spur", 0);
        let params = &spur["adopted"]["params"];
        let solved: serde_json::Value = serde_json::from_str(
            &solve_gear_impl(&serde_json::json!({ "params": params }).to_string()).unwrap(),
        )
        .unwrap();
        assert_eq!(
            solved["clamps"].as_array().map(Vec::len),
            Some(0),
            "{:?}",
            solved["clamps"]
        );
        let z = params["teeth"].as_f64().unwrap();
        let m = params["module"].as_f64().unwrap();
        assert!((solved["pitch_diameter"].as_f64().unwrap() - z * m).abs() < 1e-12);
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
        // The reason is a **note** — a key and its values — rather than a
        // sentence, so the assertion names the key. Matching prose is what
        // `docs/corrections.md` records four places doing wrongly.
        assert_eq!(
            v["over_two_pins"]["unavailable"]["key"],
            "ui.gear_no_pin_diameter"
        );

        // a pin that cannot measure this gear says why
        let bad_pin = REQ.replace("1.75", "0.05");
        let v: serde_json::Value =
            serde_json::from_str(&solve_gear_impl(&bad_pin).unwrap()).unwrap();
        assert_eq!(
            v["over_two_pins"]["unavailable"]["key"], "error.measure_pin_too_small",
            "{:?}",
            v["over_two_pins"]
        );
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

    /// A train with two presets in it. The worm's result has a point contact
    /// where the spur's has a line, and both have to survive the same boundary.
    /// A ring crosses the boundary: its own request shape, its own summary, an
    /// outline the viewport can draw and a DXF the CAD can read.
    /// **An eccentric gear crosses whole, and a concentric one says why it has
    /// no profile rather than sending a flat one.**
    ///
    /// The boundary is where a feature stops being the core's and starts being
    /// the application's, and the two things worth checking are that the
    /// variation arrives and that the *absence* of a centre-distance profile is
    /// explained rather than silent. A concentric gear commands one distance;
    /// profiling a constant would be an answer to a question nobody asked.
    #[test]
    fn an_eccentric_gear_crosses_the_boundary_with_its_variation_and_profile() {
        let req = |shift: f64, mate: &str| {
            format!(
                r#"{{"params":{{"module":1.0,"pressure_angle":20.0,"teeth":24,
                   "profile_shift":0.0,"helix_angle":0.0,"addendum":1.0,"dedendum":1.25,
                   "root_radius":0.38,"thickness_mod":1.0,"angular_shift":{shift},
                   "index_offset":0.0}}{mate}}}"#
            )
        };
        let parse = |s: &str| -> serde_json::Value {
            serde_json::from_str(&solve_gear_impl(s).expect("a solvable gear")).unwrap()
        };

        // With a mate: the profile crosses, and its range straddles the fit.
        let v = parse(&req(0.25, r#","mate":{"teeth":43,"profile_shift":0.0}"#));
        assert!((v["variation"]["eccentricity"].as_f64().unwrap() - 0.25).abs() < 1e-12);
        assert!(v["variation"]["drive_pitch_error"].as_f64().unwrap() > 0.0);
        let p = &v["centre_profile"];
        assert_eq!(
            p["commanded"].as_array().unwrap().len(),
            24,
            "one per tooth"
        );
        let (lo, hi) = (
            p["range"][0].as_f64().unwrap(),
            p["range"][1].as_f64().unwrap(),
        );
        assert!(lo < hi && (hi - lo) > 0.4, "{lo} to {hi}");
        // A crank leaves interference on one side and slack on the other.
        assert!(p["sinusoid_backlash"][0].as_f64().unwrap() < 0.0);
        assert!(p["sinusoid_backlash"][1].as_f64().unwrap() > 0.0);

        // Without one: everything else still crosses, and the profile says why.
        let v = parse(&req(0.25, ""));
        assert!(v["variation"]["eccentricity"].as_f64().unwrap() > 0.0);
        assert_eq!(
            v["centre_profile"]["unavailable"]["key"], "ui.gear_no_mate",
            "{:?}",
            v["centre_profile"]
        );

        // Concentric, with a mate: the variation is zero throughout and the
        // profile is refused for the right reason.
        let v = parse(&req(0.0, r#","mate":{"teeth":43,"profile_shift":0.0}"#));
        assert_eq!(v["variation"]["eccentricity"].as_f64().unwrap(), 0.0);
        assert_eq!(v["variation"]["drive_pitch_error"].as_f64().unwrap(), 0.0);
        assert_eq!(
            v["centre_profile"]["unavailable"]["key"], "ui.gear_concentric_has_no_profile",
            "{:?}",
            v["centre_profile"]
        );
    }

    /// **Sizing by the centre-distance throw is the profile read backwards, and
    /// nothing downstream can tell.**
    ///
    /// Send a throw instead of an amplitude; `angular_shift` comes back solved,
    /// the geometry is built from it as always, and the commanded centre
    /// distance's own sinusoid confirms the throw it was asked for. An
    /// unreachable throw is a request error, like any bad input.
    #[test]
    fn the_eccentricity_can_be_sized_by_the_centre_distance_throw() {
        let base = r#""module":1.0,"pressure_angle":20.0,"teeth":24,"profile_shift":0.1,
            "helix_angle":0.0,"addendum":1.0,"dedendum":1.25,"root_radius":0.38,
            "thickness_mod":1.0,"angular_shift":0.0,"index_offset":0.0"#;

        let req = format!(
            r#"{{"params":{{{base}}},"mate":{{"teeth":43,"profile_shift":0.0}},
               "eccentric_throw":0.2}}"#
        );
        let v: serde_json::Value =
            serde_json::from_str(&solve_gear_impl(&req).expect("a solvable gear")).unwrap();

        let dx = v["angular_shift"].as_f64().unwrap();
        assert!(dx > 0.0, "the amplitude was solved: {dx}");
        // The commanded centre distance it was sized from has that throw.
        assert!(
            (v["centre_profile"]["sinusoid"]["amplitude"]
                .as_f64()
                .unwrap()
                - 0.2)
                .abs()
                < 1e-6,
            "{:?}",
            v["centre_profile"]["sinusoid"]
        );
        // And the geometry followed: eccentricity is `m·Δx`.
        assert!((v["variation"]["eccentricity"].as_f64().unwrap() - dx).abs() < 1e-9);

        // The sign carries through.
        let neg = req.replace("\"eccentric_throw\":0.2", "\"eccentric_throw\":-0.2");
        let v: serde_json::Value = serde_json::from_str(&solve_gear_impl(&neg).unwrap()).unwrap();
        assert!(v["angular_shift"].as_f64().unwrap() < 0.0);

        // No mate, and an out-of-reach throw, are request errors.
        let no_mate = format!(r#"{{"params":{{{base}}},"eccentric_throw":0.2}}"#);
        assert!(solve_gear_impl(&no_mate).unwrap_err().contains("mate"));
        let too_big = req.replace("\"eccentric_throw\":0.2", "\"eccentric_throw\":50.0");
        assert!(solve_gear_impl(&too_big)
            .unwrap_err()
            .contains("not reachable"));
    }

    /// **An eccentric gear's scalars describe a tooth it actually has, and the
    /// per-position measurements say they vary rather than quoting the mean.**
    ///
    /// The summary is quoted from `Gear::mean`, which is cut by the same
    /// shared tool as the teeth — so `fillet_radius` and `cutter_tip_width` are
    /// the real tool's, not the 0.38-module rack the raw inputs name. Span and
    /// over-pins vary around the revolution and their ranged form is not built
    /// (docs/reference.md#angularly-varying-profile-shift), so they are withheld with the reason.
    #[test]
    fn an_eccentric_gears_summary_is_a_tooth_it_has() {
        let parse = |s: &str| -> serde_json::Value {
            serde_json::from_str(&solve_gear_impl(s).expect("a solvable gear")).unwrap()
        };
        // The reported case: high shift against a shallow dedendum forces the
        // shared cutter deeper and its tip round smaller.
        let ecc = parse(
            r#"{"params":{"module":1.0,"pressure_angle":25.0,"teeth":23,
               "profile_shift":0.2,"helix_angle":0.0,"addendum":0.8,"dedendum":1.0,
               "root_radius":0.38,"thickness_mod":1.0,"angular_shift":1.0,
               "index_offset":1.0},"pin_diameter":1.8}"#,
        );
        let fillet = ecc["fillet_radius"].as_f64().unwrap();
        assert!(
            fillet < 0.1,
            "fillet radius {fillet} is the raw 0.38 rack, not the shared tool"
        );
        // The root radius sits inside the range the variation reports.
        let (lo, hi) = (
            ecc["variation"]["root_radius"][0].as_f64().unwrap(),
            ecc["variation"]["root_radius"][1].as_f64().unwrap(),
        );
        let root = ecc["root_radius"].as_f64().unwrap();
        assert!(lo <= root && root <= hi, "{lo} ≤ {root} ≤ {hi}");
        // **The inspection data is a range now, not a silence.** It used to be
        // withheld entirely, on the reasoning that one number would read as
        // *the* span — right about the number and wrong about the remedy, which
        // is the range rather than nothing.
        for m in ["span", "over_two_pins", "over_three_pins"] {
            let around = &ecc[m]["around"];
            let (lo, hi) = (around[0].as_f64().unwrap(), around[1].as_f64().unwrap());
            assert!(
                lo < hi,
                "{m}: an eccentric gear's measurement does not vary — {:?}",
                ecc[m]
            );
            let nominal = ecc[m]["nominal"].as_f64().unwrap();
            assert!(
                lo <= nominal && nominal <= hi,
                "{m}: {lo} ≤ {nominal} ≤ {hi}"
            );
        }

        // `undercut` is "any tooth": the mean here (x = 0, z = 24) is not
        // undercut, but the short teeth at x ≈ −0.5 are — and `per_tooth_clamps`
        // names them.
        let short = parse(
            r#"{"params":{"module":1.0,"pressure_angle":20.0,"teeth":24,
               "profile_shift":0.0,"helix_angle":0.0,"addendum":1.0,"dedendum":1.25,
               "root_radius":0.38,"thickness_mod":1.0,"angular_shift":0.5,
               "index_offset":0.0}}"#,
        );
        assert_eq!(short["undercut"].as_bool(), Some(true));
        assert!(!short["per_tooth_clamps"]["teeth"]
            .as_array()
            .unwrap()
            .is_empty());

        // A concentric gear is untouched: mean is `Tooth::new(params)` verbatim,
        // and its span and pins are still measured.
        let con = parse(
            r#"{"params":{"module":1.0,"pressure_angle":25.0,"teeth":23,
               "profile_shift":0.2,"helix_angle":0.0,"addendum":0.8,"dedendum":1.0,
               "root_radius":0.38,"thickness_mod":1.0},"pin_diameter":1.8}"#,
        );
        assert!((con["fillet_radius"].as_f64().unwrap() - 0.38).abs() < 0.05);
        assert!(con["span"]["nominal"].as_f64().unwrap() > 0.0);
        assert!(con["over_two_pins"]["nominal"].as_f64().unwrap() > 0.0);
        assert_eq!(con["undercut"].as_bool(), Some(false));
    }

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
        // Between inner surfaces, so the pin *subtracts* — the opposite of the
        // gear tab's over-pins, and the sign a reader should be able to check.
        // The pin centre radius comes from the crate rather than the wire: it is
        // where the measurement is derived, and the wire carries the nominal
        // alone now.
        let centre =
            gear_core::metrology::between_pins(&ring_of(&parse_ring(&with_pin).unwrap()), 1.8)
                .unwrap()
                .pin_centre_radius;
        assert!((nominal - (2.0 * centre - 1.8)).abs() < 1e-9);
        assert!(nominal < 2.0 * centre);
        // ...and without a pin diameter it says so rather than inventing one.
        assert_eq!(
            v["between_pins"]["unavailable"]["key"],
            "ui.gear_no_pin_diameter"
        );

        // **A shifted ring must come back shifted.** The gear tab has always
        // sent `profile_shift` for an internal gear and `Ring::cut_by` used to drop
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
    /// `Shape` is a tagged enum and `Train` already carried a `Vec` of them, so
    /// the set needed no entry point of its own — and since the kinds retired
    /// into the one shape, not even a tag: a set is a shape whose planet's
    /// axis is carried. That is the claim worth checking rather than assuming
    /// — "it should just work" is exactly what turns out to be false at a
    /// serde boundary.
    ///
    /// The request is the shipped set put through `defaults`, so the fields
    /// this asserts on are the fields a front end actually sends. The
    /// assertions look for what a set has and a pair does not: five local
    /// bodies (the ground, then sun, carrier, ring, planet), a *solved* planet
    /// shift, and two meshes each with their own answers.
    #[test]
    fn a_planetary_stage_crosses_the_boundary_with_its_own_shape() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut set = preset(&d, "planetary");
        set["members"][0]["gear"]["teeth"] = 24.into();
        set["members"][1]["gear"]["teeth"] = 18.into();
        set["members"][2]["gear"]["teeth"] = 60.into();
        set["distances"][0]["clearance"] = serde_json::json!({"auto": false, "manual": 0.02});
        // **The arrangement is the train's**: a hold on the set's ring (body
        // 3), written in so many words, and the loads between its sun and
        // its carrier. The set carries none of its own.
        let mut train = train_json(&[set], (2.0, 3000.0), 0.0, (1.6, 2400.0), 1000.0);
        train["held"] = serde_json::json!([3]);
        let req = serde_json::json!({ "train": train });

        let v = solved(&req.to_string());
        // One part: the graph's order is the card's, and what the part's
        // meshes put on its bodies is the part's own.
        let stage = &v["parts"][0];

        // Ring held, sun driving: the classical 1 + z_r/z_s.
        assert!((v["paths"][0]["ratio"].as_f64().unwrap() - 3.5).abs() < 1e-12);

        // Five local bodies, the held one exactly still, and the torques
        // balancing — in the first load case, at its own speed.
        let shafts = &stage["cases"][0];
        let speeds = shafts["speeds"].as_array().unwrap();
        assert_eq!(speeds.len(), 5);
        assert_eq!(speeds[0].as_f64().unwrap(), 0.0, "the ground is still");
        assert_eq!(speeds[3].as_f64().unwrap(), 0.0, "the ring is held");
        assert!((speeds[1].as_f64().unwrap() - 3000.0).abs() < 1e-9);
        let sum: f64 = shafts["torques"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_f64().unwrap())
            .sum();
        assert!(sum.abs() < 1e-9, "torques must balance, got {sum}");

        // The planet's shift is *solved*, not sent: 24 + 2x18 = 60 is the ideal
        // ring, so what moves it is the running clearance alone — the planet
        // thinned by that much opens both meshes — and it comes back negative,
        // small, closing the distance.
        let x_p = v["members"][1]["profile_shift"].as_f64().unwrap();
        let c = v["distances"][0]["clearance"].as_f64().unwrap();
        assert!(
            c > 0.0 && x_p < 0.0 && x_p.abs() < 2.0 * c,
            "x_p {x_p} at clearance {c}"
        );
        // The one distance closes on both meshes: each runs at it, and each
        // is opened by the same clearance — outward for the sun's mesh, inward
        // for the ring's, which is what a signed clearance reads as.
        let running = v["distances"][0]["running"].as_f64().unwrap();
        for nominal in v["distances"][0]["nominal"].as_array().unwrap() {
            assert!(
                ((running - nominal.as_f64().unwrap()).abs() - c).abs() < 1e-9,
                "nominal {nominal}, running {running}, clearance {c}"
            );
        }
        // A planet's root is loaded on both flanks, and with no correction asked
        // for the stage says so rather than derating it out of sight.
        let notes = v["members"][1]["notes"]
            .as_array()
            .expect("a gear carries its own notes");
        assert!(
            notes
                .iter()
                .any(|n| n["key"] == "gear.reversed_bending_uncorrected"),
            "the planet's reversal should be disclosed on the planet: {notes:?}"
        );

        // Two meshes with their own answers, and every member rated.
        let meshes = v["meshes"].as_array().unwrap();
        assert_eq!(meshes.len(), 2);
        for (k, mesh) in meshes.iter().enumerate() {
            assert!(mesh["contact_ratio"].as_f64().unwrap() > 1.0);
            // The two cases that carry a load press the flanks; the one from
            // the end carries nothing and presses with exactly nothing.
            for (case, loaded) in [(0, true), (1, false), (2, true)] {
                let pressure = mesh["cases"][case]["contact"]["at_pitch_point"]
                    .as_f64()
                    .unwrap();
                assert_eq!(pressure > 0.0, loaded, "mesh {k} case {case}: {pressure}");
            }
        }
        for who in 0..3 {
            assert!(
                v["members"][who]["cases"][0]["bending_stress"]
                    .as_f64()
                    .unwrap()
                    > 0.0,
                "member {who} must be rated"
            );
        }
        assert_eq!(v["axes"][1]["layout"]["equal_spacing"], true);
        // What the stage *assumes* has to come across too — here, equal load
        // sharing between planets, which no calculation can establish. Crossing
        // as a key and its values, not as a sentence: the words are the string
        // catalogue's business and the front end renders them, so what has to
        // survive the boundary is the identity of the note and the number in it.
        let notes = stage["notes"].as_array().unwrap();
        let sharing = notes
            .iter()
            .find(|n| n["key"] == "stage.planets_share_load_equally")
            .unwrap_or_else(|| panic!("the load-sharing assumption must be reported: {notes:?}"));
        assert_eq!(sharing["values"]["planets"], "3");
        // ...and the play at the output is a real figure, the path's.
        assert!(
            v["paths"][0]["backlash"]["forward"]["nominal"]
                .as_f64()
                .unwrap()
                > 0.0
        );
    }

    /// **A hula stage crosses the boundary carrying its ratings.**
    ///
    /// The one preset with two internal meshes on a distance the tips size:
    /// four gears on a crank, a grounded member that is loaded while it does
    /// not turn, and a distance the report says was held open by a mesh.
    ///
    /// The request is the shipped stage put through `defaults`, so the fields
    /// this asserts on are the fields a front end actually sends — a hand-typed
    /// literal here would be a fourth copy of the boundary and would go stale
    /// the way the hand-written mirror did (`docs/corrections.md`).
    #[test]
    fn a_hula_stage_crosses_the_boundary_carrying_its_ratings() {
        let train = serde_json::json!({
            "train": train_json(&[hula_stage()], (2.0, 3000.0), 0.0, (1.0, 3000.0), 1.0)
        });
        let v = solved(&train.to_string());
        // One part: the graph's gears are the card's.
        let stage = &v;

        // Four gears, each with the rating every stage member carries.
        let gears = stage["members"].as_array().expect("four gears");
        assert_eq!(gears.len(), 4);
        for (i, rated) in gears.iter().enumerate() {
            assert!(rated["face_width"].as_f64().unwrap() > 0.0, "gear {i}");
            assert!(
                rated["cases"][0]["torque"].as_f64().unwrap().abs() > 0.0,
                "gear {i}"
            );
            assert!(rated["cases"][0]["contact_stress"].as_f64().unwrap() > 0.0);
            assert!(
                rated["material"]["name"].as_str().is_some(),
                "gear {i} was rated on a material"
            );
            // **A grounded gear is loaded**, which is the case a count taken
            // from a member's own revolutions could not state.
            assert!(
                rated["cases"][2]["cycles"]["bending"].as_f64().unwrap() > 0.0,
                "gear {i} is engaged once a crank turn at least"
            );
            // ...and an ultimate case counts nothing.
            assert!(rated["cases"][0]["cycles"].is_null(), "gear {i}");
        }
        // The grounded gear — member 2, after the two wobble gears — stands.
        assert_eq!(gears[2]["cases"][0]["speed"].as_f64().unwrap(), 0.0);

        // Two meshes, each reporting what any parallel-axis mesh reports,
        // with the room their tips have.
        for m in 0..2 {
            let mesh = &stage["meshes"][m];
            assert!(
                mesh["line"]["contact_ratios"]["transverse"]
                    .as_f64()
                    .unwrap()
                    > 0.0
            );
            assert!(mesh["line"]["operating_pressure_angle"].as_f64().unwrap() > 0.0);
            assert!(
                mesh["cases"][0]["contact"]["at_pitch_point"]
                    .as_f64()
                    .unwrap()
                    > 0.0
            );
            assert!(mesh["tips"]["far_gap"].as_f64().unwrap() > 0.0, "mesh {m}");
        }
        // The crank offset was sized by one of the meshes' tips.
        assert!(stage["distances"][0]["sized_by"].is_number());
        // The bodies are in equilibrium, and the drive says so in the
        // vocabulary a stage says anything in.
        let sum: f64 = v["parts"][0]["cases"][0]["torques"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t.as_f64().unwrap())
            .sum();
        assert!(sum.abs() < 1e-9, "torques must balance, got {sum}");
        assert!(
            v["parts"][0]["notes"].is_array(),
            "a part carries its own notes"
        );
    }

    #[test]
    fn a_mixed_train_crosses_the_boundary_with_both_shapes_intact() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut spur = preset(&d, "spur");
        spur["members"][0]["gear"]["teeth"] = 17.into();
        spur["members"][1]["gear"]["teeth"] = 43.into();
        spur["distances"][0]["clearance"] = serde_json::json!({"auto": false, "manual": 0.02});
        let mut worm = preset(&d, "worm");
        worm["members"][0]["gear"]["teeth"] = 1.into();
        worm["members"][1]["gear"]["teeth"] = 40.into();
        worm["members"][0]["pitch_diameter"] = serde_json::json!({"auto": false, "manual": 7.0});
        worm["distances"][0]["clearance"] = serde_json::json!({"auto": false, "manual": 0.02});
        let req = serde_json::json!({
            "train": train_json(&[spur, worm], (2.0, 3000.0), 0.0, (1.6, 2400.0), 1000.0)
        });

        let v = solved(&req.to_string());
        let want = (43.0 / 17.0) * 40.0;
        assert!((v["paths"][0]["ratio"].as_f64().unwrap() - want).abs() < 1e-9);

        // Both stages are the one shape, and each says which contact it has
        // by what its mesh carries: the transverse figures on parallel shafts,
        // the zone on crossed ones. Neither carries a kind: there is one. The
        // pair's mesh is the graph's first and the worm's its second; the
        // worm's thread is the graph's third gear.
        assert!(v["parts"][0]["kind"].is_null());
        assert!(v["parts"][1]["kind"].is_null());
        let (spur, worm) = (&v["meshes"][0], &v["meshes"][1]);
        assert!(spur["line"].is_object() && spur["point"].is_null());
        assert!(worm["point"].is_object() && worm["line"].is_null());
        // The sliding a line contact has at its pitch point is exactly none.
        assert_eq!(spur["sliding_ratio"], 0.0);
        assert!(worm["sliding_ratio"].as_f64().unwrap() > 1.0);
        assert!(
            v["members"][0]["cases"][0]["bending_stress"]
                .as_f64()
                .unwrap()
                > 0.0
        );

        assert!(
            v["members"][2]["pitch_diameter"].as_f64().unwrap() > 0.0,
            "a worm stage's members are gears like any other"
        );
        let mesh = worm;
        assert!(
            mesh["cases"][0]["contact"]["max_pressure"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        let eff = &mesh["efficiency"];
        assert!(eff["backward"].as_f64().unwrap() < eff["forward"].as_f64().unwrap());
        // ...while the spur stage puts the same number in both, which is the
        // point of reporting it directionally everywhere rather than only where
        // it differs.
        let spur_eff = &spur["efficiency"];
        assert_eq!(spur_eff["forward"], spur_eff["backward"]);
        // And the train reports both totals, plus backlash at each end.
        //
        // **The total backward efficiency is zero**, and that is the answer
        // rather than a missing one: a one-start worm at 8° of lead self-locks
        // against the *static* coefficient (threshold 0.1327 against 0.16), so
        // the train cannot be back-driven at all. It used to read as a positive
        // number because the whole model ran on the sliding coefficient, which
        // is the friction of a motion that never starts.
        assert_eq!(
            v["paths"][0]["efficiency"]["backward"].as_f64().unwrap(),
            0.0
        );
        assert!(v["paths"][0]["efficiency"]["forward"].as_f64().unwrap() > 0.0);
        assert!(
            v["paths"][0]["backlash"]["forward"]["nominal"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        assert!(
            v["paths"][0]["backlash"]["backward"]["nominal"]
                .as_f64()
                .unwrap()
                > 0.0
        );
        // The sliding speed is each case's own, at that case's speed.
        assert!(mesh["cases"][0]["sliding_velocity"].as_f64().unwrap() > 0.0);
        assert!(v["members"][3]["cases"][0]["speed"].as_f64().unwrap() > 0.0);
    }

    /// **Every number that crosses is a number**, or a `null` at a field that is
    /// allowed to have none.
    ///
    /// `serde_json` writes an infinity and a NaN as `null`, which is
    /// indistinguishable from an honest `None` and draws as a blank — so a
    /// figure that has gone non-finite arrives looking exactly like a figure
    /// that was never available, and the front end shows the same dash for
    /// both. That is how an automatic face width with no rating to size it came
    /// to resolve to zero, be divided by, and reach the screen as a row of
    /// blanks nobody read as wrong (`docs/corrections.md`).
    ///
    /// So the whole result is walked rather than sampled, and the **field name**
    /// is what the allowance is written against: a `null` under a name not on
    /// the list fails, and the failure names the path. Adding a name here is a
    /// deliberate act — it says this field can genuinely have no value — which
    /// is the point of it being a list rather than a rule.
    /// Fields that may honestly carry no value, and why.
    ///
    /// Shared by both surfaces, because "which fields can be empty" is one
    /// question about the boundary rather than one per entry point — and a name
    /// allowed on the geartrain and refused on a gear would be the two halves
    /// disagreeing about the same field.
    const ABSENT_IS_MEANINGFUL: &[&str] = &[
        // A section with no notch has no bending rating: a ring whose cutter
        // left no fillet is the ordinary way to get here.
        "bending",
        "bending_stress",
        // ...and a crossed pair's members have none by decision: a point
        // contact tracking across the flank is not the load a cantilever
        // formula measures (docs/rationale.md#a-worm-stage-reports-no-bending-stress).
        // An ultimate case is survived once and counts no cycles.
        "cycles",
        // A body the train fixes has no speed to report.
        "speed",
        // Nothing held the crank open at the clearance minimum.
        "binding_mesh",
        // A single planet has no neighbour to clear.
        "planet_clearance",
        // An axis nothing is replicated round has no layout.
        "layout",
        // A layout's assembly rule is known for one gear on the axis meshing
        // two central members, and is a question with no answer elsewhere.
        "equal_spacing",
        // A distance the shifts left where it was, or one the designer gave,
        // was sized by no mesh's tips.
        "sized_by",
        "simultaneous_meshing",
        // A crossed gear pair is not a worm and has no published proportions,
        // and no parallel-axis member has any either.
        "recommended_face_width",
        // A line contact has no zone as the faces leave it and a point contact
        // no transverse decomposition; and a spur gear's flank does not
        // advance, so it has no lead.
        "line",
        "point",
        "lead",
        // The train solved, so there is no failure to report.
        "failure",
        // A bound that does not exist on this geometry — see `auto::Ranges`.
        "min",
        "max",
        // ...and the shift above which the tooth would be pointed, where
        // the tooth never comes to a point anywhere it can be built.
        "pointed",
        // A material value with nothing to say beyond its number.
        "note",
        // One tooth more on a gear leaves no mechanism, or locks the path —
        // a Wolfrom's rings brought level, or a gear another stage shares a
        // body with locking that stage and the path's motion with it — and a
        // ratio of infinity is no figure.
        "per_tooth",
        // An **external** mesh's tips meet on the line of centres or not at
        // all, so the three ways an internal mesh's teeth can foul are not
        // three answers of `false` there — they are questions that do not
        // arise. `docs/corrections.md#an-absent-thing-is-not-a-zero-length-thing`
        // is the rule; a spur pair and an epicyclic set's sun-planet mesh are
        // where it is met.
        "tips",
        // Where no rating sizes a face, no width is asked for. A point
        // contact's peak pressure does not depend on the face width at all, so
        // there is nothing to invert; a crossed pair's width comes from
        // continuity instead, and says so under its own name.
        //
        // **Written as a path rather than as a field**, because `contact` is
        // also a tooth-cycle count and a face-width toggle, and allowing the
        // bare name would stop this noticing if either of those went absent.
        "min_face_width.contact",
    ];

    /// Every `null` in a result, at a field not named above.
    fn nulls(v: &serde_json::Value, path: &str) -> Vec<String> {
        fn walk(v: &serde_json::Value, path: &str, bad: &mut Vec<String>) {
            match v {
                serde_json::Value::Null => {
                    let field = path
                        .rsplit('.')
                        .find(|s| !s.starts_with('['))
                        .unwrap_or(path);
                    // A bare name matches the field wherever it appears; a
                    // dotted entry has to match the tail of the path, so an
                    // allowance can be as narrow as the case that earned it.
                    let bare = path.replace(['[', ']'], "");
                    let allowed = ABSENT_IS_MEANINGFUL.iter().any(|a| {
                        if a.contains('.') {
                            bare.ends_with(a)
                        } else {
                            *a == field
                        }
                    });
                    if !allowed {
                        bad.push(path.to_string());
                    }
                }
                serde_json::Value::Object(m) => {
                    for (k, x) in m {
                        walk(x, &format!("{path}.{k}"), bad);
                    }
                }
                serde_json::Value::Array(a) => {
                    for (i, x) in a.iter().enumerate() {
                        walk(x, &format!("{path}.[{i}]"), bad);
                    }
                }
                _ => {}
            }
        }
        let mut bad = Vec::new();
        walk(v, path, &mut bad);
        bad
    }

    /// **The numbers this boundary invents, as figures.**
    ///
    /// Every field in the panel starts at a value, and rule 1 says that value is
    /// one of the tool's own: *if a number appears in the UI, Rust computed it —
    /// a default is one of those numbers.* Most of them are `gear-core`'s and
    /// the golden corpus sees them, because `gear-cli` builds its stages from
    /// the same `Default`. **These are not.** They exist only here — the tab's
    /// tooth count, the pin, the throw, the face width a fresh panel seeds, the
    /// speed and torque a fresh train carries — and they reach a designer
    /// without passing anything that could notice them moving.
    ///
    /// Measured: changing any of the four probed left **every test, the whole
    /// corpus and the binding check silent**. So they are pinned here, which is
    /// what a canary is for — none of them is derived, and the only property
    /// they have is that nobody changes them by accident.
    ///
    /// A figure that *is* forwarded from the core is deliberately not repeated:
    /// pinning it twice would make this the second place to edit, and the corpus
    /// already holds the first.
    #[test]
    fn the_defaults_this_boundary_invents_are_the_ones_it_shipped() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();

        // The gear tab.
        assert_eq!(d["gear"]["params"]["teeth"], 9);
        assert_eq!(d["gear"]["pin_diameter"], 1.75);
        assert_eq!(d["gear"]["eccentric_throw"], 0.1);
        assert_eq!(d["gear"]["reference_circles"], true);

        // A fresh train: a small motor, and no derating nobody asked for —
        // the three cases a train used to hold as fields, in that order,
        // each between the pair's two gears with the far one declared.
        let (first, second) = (1, 2);
        let cases = d["train"]["load_cases"].as_array().unwrap();
        assert_eq!(cases.len(), 3);
        assert_eq!(cases[0]["kind"], "ultimate");
        assert_eq!(cases[0]["loads"][0]["at"], first);
        assert_eq!(cases[0]["loads"][0]["role"], "load");
        assert_eq!(cases[0]["loads"][0]["speed"]["manual"], 30_000.0);
        assert_eq!(cases[0]["loads"][0]["torque"]["manual"], 0.1);
        assert_eq!(cases[0]["loads"][1]["at"], second);
        assert_eq!(cases[0]["loads"][1]["role"], "reacted");
        assert_eq!(cases[0]["loads"].as_array().unwrap().len(), 2);
        // A load from the second gear, held still, with the first reacted.
        assert_eq!(cases[1]["kind"], "ultimate");
        assert_eq!(cases[1]["loads"][0]["at"], second);
        assert_eq!(cases[1]["loads"][0]["torque"]["manual"], 3.0);
        assert_eq!(cases[1]["loads"][0]["speed"]["manual"], 0.0);
        assert_eq!(cases[1]["loads"][1]["at"], first);
        assert_eq!(cases[1]["loads"][1]["role"], "reacted");
        assert_eq!(cases[2]["kind"], "fatigue");
        assert_eq!(cases[2]["loads"][0]["torque"]["manual"], 0.02);
        assert_eq!(cases[2]["duty"]["intermittent"]["at"], second);
        for c in cases {
            assert_eq!(c["enabled"], true);
        }
        // ...and its two bodies are the pair's two gears, numbered from one.
        let bodies = d["train"]["shape"]["bodies"].as_array().unwrap();
        assert_eq!(bodies.len(), 2);
        assert_eq!(bodies[0]["body"], 1);
        assert_eq!(bodies[1]["body"], 2);
        assert_eq!(d["train"]["reversed_bending"], false);

        // **The face width a panel seeds, on every gear of every preset it
        // offers.** It is the one number here that is not written once: the
        // core's default is a plain 10 mm and the tab wants the width the rating
        // asks for, so each gear is rebuilt with an automatic 5 mm — and a walk
        // is what says all of them were.
        let mut seeded = 0;
        let mut walk: Vec<serde_json::Value> = d["stages"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["stage"].clone())
            .collect();
        walk.push(d["train"]["shape"].clone());
        for stage in &walk {
            for m in stage["members"].as_array().unwrap() {
                let w = &m["gear"]["face_width"];
                assert_eq!(w["auto"], true, "a seeded width is not automatic");
                assert_eq!(w["manual"], 5.0, "a seeded width is not 5 mm");
                seeded += 1;
            }
        }
        // Every member of every preset, and the pair again inside the train:
        // counted off the list rather than written down, and at least the
        // ten presets' two apiece.
        let members: usize = walk
            .iter()
            .map(|s| s["members"].as_array().unwrap().len())
            .sum();
        assert_eq!(seeded, members, "a member's width went unseeded");
        assert!(seeded >= 2 * (StagePreset::ALL.len() + 1), "{seeded}");
    }

    #[test]
    fn every_number_that_crosses_is_a_number() {
        // Every preset the defaults can build, in one train, so the walk
        // covers every layout there is.
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut stages = vec![d["train"]["shape"].clone()];
        stages.extend(
            d["stages"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| e["stage"].clone()),
        );
        stages.push(hula_stage());
        // ...and the case that started this: a width with nothing to size it.
        let mut bare = stages[0].clone();
        for m in bare["members"].as_array_mut().unwrap() {
            let g = &mut m["gear"];
            g["face_width"] = serde_json::json!({ "auto": true, "manual": 6.0 });
            g["face_sources"] = serde_json::json!({
                "bending": { "ultimate": false, "fatigue": false },
                "contact": { "ultimate": false, "fatigue": false },
            });
        }
        stages.push(bare);

        for (i, stage) in stages.iter().enumerate() {
            let train = serde_json::json!({
                "train": train_json(std::slice::from_ref(stage), (2.0, 3000.0), 0.0, (1.0, 3000.0), 1.0)
            });
            let v = solved(&train.to_string());
            let bad = nulls(&v, &format!("stage{i}"));
            assert!(
                bad.is_empty(),
                "a figure crossed as null at a field that should always have one: {bad:?}"
            );
        }
    }

    /// **The same of a gear tab**, which is the other half of the boundary and
    /// has its own ways for a number to go missing: an eccentric gear withholds
    /// the measurements that vary around its revolution, a ring's flank is its
    /// shaper's, and both report bounds that need not exist.
    #[test]
    fn every_number_a_gear_reports_is_a_number() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let gear = |p: serde_json::Value| serde_json::json!({ "params": p, "pin_diameter": 1.75 });
        let mut ordinary: serde_json::Value = serde_json::from_str(REQ).unwrap();
        let base = ordinary["params"].clone();
        let mut eccentric = base.clone();
        eccentric["angular_shift"] = serde_json::json!(0.5);
        let mut undercut = base.clone();
        undercut["teeth"] = serde_json::json!(9);
        undercut["profile_shift"] = serde_json::json!(-0.3);
        ordinary["pin_diameter"] = serde_json::json!(1.75);

        for (what, req) in [
            ("as shipped", gear(d["gear"]["params"].clone())),
            ("the fixture", ordinary),
            ("eccentric", gear(eccentric)),
            ("undercut", gear(undercut)),
        ] {
            let out = solve_gear_impl(&req.to_string())
                .unwrap_or_else(|e| panic!("{what} should solve: {e}"));
            let v: serde_json::Value = serde_json::from_str(&out).unwrap();
            let bad = nulls(&v, what);
            assert!(bad.is_empty(), "{what}: {bad:?}");
            // ...and the case whose measurements vary really does vary, or this
            // is four ordinary gears wearing four names. An eccentric gear has
            // no single span: it has a range around the revolution, and both
            // ends of it are numbers this walk has just been over.
            if what == "eccentric" {
                let around = v["span"]["around"].as_array().unwrap_or_else(|| {
                    panic!("an eccentric gear's span is a range: {}", v["span"])
                });
                assert!(
                    around[0].as_f64().unwrap() < around[1].as_f64().unwrap(),
                    "and the two ends of it differ: {around:?}"
                );
            }
        }
    }

    #[test]
    fn a_two_stage_train_crosses_the_boundary() {
        // The shape the UI will send: a train, and no library, meaning "use the
        // one you ship with" — with the stage the panel seeds, at the tooth
        // counts of the regression canary.
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let mut spur = preset(&d, "spur");
        spur["members"][0]["gear"]["teeth"] = 17.into();
        spur["members"][1]["gear"]["teeth"] = 43.into();
        spur["distances"][0]["clearance"] = serde_json::json!({"auto": false, "manual": 0.02});
        let req = serde_json::json!({
            "train": train_json(&[spur], (2.0, 3000.0), 0.0, (1.6, 2400.0), 1000.0)
        });

        let v = solved(&req.to_string());
        // **Negative**, because an external pair reverses and a train's ratio
        // says so now: it is read off the graph rather than multiplied out of
        // the stage ratios, and a pair reports its own as a magnitude.
        assert!((v["paths"][0]["ratio"].as_f64().unwrap() + 43.0 / 17.0).abs() < 1e-12);
        // Every body of the case, with what it is: the pair's second member
        // is the reacted end and carries the load stepped up.
        let bodies = v["cases"][0]["bodies"].as_array().unwrap();
        let end = bodies.iter().find(|s| s["role"] == "reacted").unwrap();
        assert!(end["torque"].as_f64().unwrap().abs() > 2.0);
        assert_eq!(bodies[0]["role"], "fixed");
        assert!(bodies[0]["speed"].is_null(), "ground reports no speed");
        assert_eq!(v["cases"][0]["solved"], true);

        let g0 = &v["members"][0];
        // The automatic face width came back, and so did the cycle count.
        assert!(g0["face_width"].as_f64().unwrap() > 0.0);
        assert!(g0["cases"][2]["cycles"]["bending"].as_f64().unwrap() > 0.0);
        assert!(g0["cases"][2]["cycles"]["contact"].as_f64().unwrap() > 0.0);
        assert!((g0["cases"][0]["speed"].as_f64().unwrap() - 3000.0).abs() < 1e-9);
        // Spur stage: the overlap ratio is exactly zero, not merely small.
        assert_eq!(
            v["meshes"][0]["line"]["contact_ratios"]["overlap"]
                .as_f64()
                .unwrap(),
            0.0
        );
    }

    /// **A refusal crosses as data, and as a key rather than as a sentence.**
    ///
    /// A geartrain that will not build is an ordinary thing to be holding
    /// mid-edit, so it comes back in the payload and the panel goes on showing
    /// every input that produced it. And it comes back as a `Note`, because
    /// `TrainError`'s `Display` is English written in Rust — handing that over
    /// made the failure the one thing the application said in a language
    /// nobody chose. Only a broken boundary is still an error.
    /// The answer out of a solve that was supposed to have one.
    ///
    /// The payload carries `{ result, failure }` now, because a train that will
    /// not build is data rather than an exception — so a test that expects one
    /// to build says so here instead of reaching past a `result` that might be
    /// null and failing somewhere less obvious.
    fn solved(req: &str) -> serde_json::Value {
        let v: serde_json::Value = serde_json::from_str(&solve_train_impl(req).unwrap()).unwrap();
        assert!(
            v["failure"].is_null(),
            "this train was expected to solve: {}",
            v["failure"]
        );
        v["result"].clone()
    }

    /// **A preview is the edit, made on a copy, and nothing kept**: a gear
    /// on a new axis previews as the pieces it adds, counted, and a join of
    /// a pair's two bodies as its refusal — and the train sent is the train
    /// the request came with.
    #[test]
    fn a_preview_is_the_edit_on_a_copy() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let preview = |edit: serde_json::Value| -> serde_json::Value {
            serde_json::from_str(
                &preview_edit_impl(
                    &serde_json::json!({ "train": d["train"], "edit": edit }).to_string(),
                )
                .unwrap(),
            )
            .unwrap()
        };
        let made = preview(
            serde_json::json!({ "graph": { "add_gear": { "mate": 1, "on": "new_axis", "ring": false } } }),
        );
        assert!(made["refused"].is_null(), "{made}");
        assert_eq!(made["changes"][0]["key"], "preview.gears");
        assert_eq!(made["changes"][0]["values"]["after"], "3");
        let refused = preview(serde_json::json!({ "graph": { "join": { "a": 1, "b": 2 } } }));
        assert_eq!(
            refused["refused"]["key"],
            gear_core::train::EditRefused::OneCard.key()
        );
        assert!(refused["changes"].as_array().unwrap().is_empty());
    }

    /// **A refused edit crosses as its catalogue key**, which is what the
    /// panel says under the card — for a card's edit and the graph's. It
    /// crossed as English for as long as there were edits, and the panel,
    /// looking for a key, said nothing.
    #[test]
    fn a_refused_edit_crosses_as_its_key() {
        let d: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        for edit in [
            serde_json::json!({ "stage": { "stage": 0, "edit": { "remove_axis": { "axis": 0 } } } }),
            serde_json::json!({ "graph": { "join": { "a": 1, "b": 2 } } }),
        ] {
            let e = edit_train_impl(
                &serde_json::json!({ "train": d["train"], "edit": edit }).to_string(),
            )
            .unwrap_err();
            let keys = [
                gear_core::train::EditRefused::LastOfItsKind,
                gear_core::train::EditRefused::OneCard,
            ]
            .map(gear_core::train::EditRefused::key);
            assert!(keys.contains(&e.as_str()), "{edit}: {e}");
            assert!(
                gear_io::strings::Catalogue::for_language("en")
                    .messages()
                    .contains_key(e.as_str()),
                "{e} has words"
            );
        }
    }

    #[test]
    fn a_train_that_cannot_be_solved_says_why() {
        let bad = r#"{"train":{"load_cases": [
                { "kind": "ultimate", "enabled": true, "loads": [{ "at": 1, "torque": { "auto": false, "manual": 1.0 }, "speed": { "auto": false, "manual": 1.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 1000, "reversing": false } } },
                { "kind": "ultimate", "enabled": true, "loads": [{ "at": 2, "torque": { "auto": false, "manual": 0.0 }, "speed": { "auto": false, "manual": 0.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 1000, "reversing": false } } },
                { "kind": "fatigue", "enabled": true, "loads": [{ "at": 1, "torque": { "auto": false, "manual": 1.0 }, "speed": { "auto": false, "manual": 1.0 } }], "duty": { "intermittent": { "range_degrees": 25.0, "at": 2, "actuations": 1000, "reversing": false } } }
            ],
            "shape": { "axes": [], "bodies": [], "members": [], "meshes": [], "distances": [] }}}"#;
        let v: serde_json::Value = serde_json::from_str(&solve_train_impl(bad).unwrap()).unwrap();
        assert!(
            v["failure"].is_null() && v["result"]["members"].as_array().unwrap().is_empty(),
            "an empty train is a train with nothing rated: {v}"
        );
        assert_eq!(v["result"]["cases"].as_array().unwrap().len(), 3);
        assert_eq!(v["result"]["cases"][0]["solved"], false);

        // ...and where a stage is to blame, it is named — numbered as the panel
        // numbers them, so the reader is not left counting from zero.
        let sound: serde_json::Value = serde_json::from_str(&defaults_impl().unwrap()).unwrap();
        let pushed = edit_train_impl(
            &serde_json::json!({
                "train": sound["train"],
                "edit": { "push_stage": preset(&sound, "spur") },
            })
            .to_string(),
        )
        .unwrap();
        let mut train: serde_json::Value = serde_json::from_str(&pushed).unwrap();
        // The second part's one distance is the graph's second.
        train["shape"]["distances"][1]["distance"] =
            serde_json::json!({"auto": false, "manual": 0.0});
        let req = serde_json::json!({ "train": train }).to_string();
        let v: serde_json::Value = serde_json::from_str(&solve_train_impl(&req).unwrap()).unwrap();
        assert!(
            v["result"].is_null(),
            "a mesh at no distance is not a train"
        );
        assert_eq!(
            v["failure"]["stage"], 2,
            "the second stage is the one to fix"
        );
        assert!(
            v["failure"]["note"]["key"]
                .as_str()
                .is_some_and(|k| k.starts_with("error.")),
            "the reason should be a catalogue key, got {}",
            v["failure"]["note"]
        );

        // A boundary that broke is still an error: nothing a designer typed.
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
