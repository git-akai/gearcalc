//! Measurements you can actually take on a finished gear: span over teeth,
//! measurement over pins or balls, and the cutter tip width.
//!
//! These are what go on a drawing, so the emphasis is on reporting *whether a
//! measurement is takeable* as much as on the number itself. A span that
//! contacts below the form circle, or a pin that bottoms out in the root, is not
//! a measurement — and returning a plausible number for one is worse than
//! returning nothing.
//!
//! Per DESIGN.md §4.6, only **nominal** values are produced. Minimum and maximum
//! need a tooth thickness tolerance from JGMA 1103-01, which is not available;
//! the result types carry the space for it so adding the data later is a data
//! change rather than a redesign.

use crate::involute::{inv, inv_inverse};
use crate::profile::Gear;

/// Base helix angle: `sin β_b = sin β · cos α_n`.
///
/// The angle of the helix measured on the base cylinder, which is what projects
/// a transverse base-circle arc into the normal plane.
#[must_use]
pub fn base_helix_angle(g: &Gear) -> f64 {
    (g.beta.sin() * g.alpha_n.cos()).asin()
}

/// Why a measurement cannot be taken.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementError {
    /// No span over any number of teeth contacts the usable flank.
    NoValidSpan,
    /// The pin contacts outside the usable flank — below the form circle or
    /// beyond the tip.
    PinContactOffFlank,
    /// The pin sits on the root diameter instead of the flanks.
    PinBottomsOut,
    /// The pin is small enough to fall between the flanks entirely: its centre
    /// would sit inside the base circle, where there is no involute to touch.
    PinTooSmall,
    /// The pin is large enough that contact runs past the base circle on the
    /// other side — there is no involute point where it could touch.
    PinTooLarge,
}

/// Span over `k` teeth ("base tangent length"), mm.
#[derive(Clone, Copy, Debug)]
pub struct Span {
    /// Teeth spanned.
    pub teeth_spanned: u32,
    /// Nominal span, mm.
    pub nominal: f64,
    /// Radius at which the measuring faces touch the flank.
    pub contact_radius: f64,
    /// Tolerance band, once a tooth thickness tolerance source exists.
    pub limits: Option<(f64, f64)>,
}

/// Span over an explicit number of teeth.
///
/// Derived from first principles rather than quoted: the span is a chord along
/// the base tangent, so it is `(k−1)` base pitches plus one base tooth
/// thickness, all projected into the normal plane by `cos β_b`.
///
/// ```text
/// W_k = cos β_b · r_b · [ 2π(k−1)/z + s_t/r + 2 inv α_t ]
/// ```
///
/// For a standard rack this reduces exactly to the familiar
/// `W_k = m cos αₙ [π(k−0.5) + z inv α_t] + 2 x m sin αₙ`, and the test suite
/// checks it does. Writing it in the general form means profile shift and
/// thickness modification are handled without a special case, since both are
/// already inside `s_t`.
#[must_use]
pub fn span_over_teeth(g: &Gear, k: u32) -> Span {
    let z = f64::from(g.params.teeth);
    let bb = base_helix_angle(g);
    let nominal = bb.cos()
        * g.rb
        * (2.0 * std::f64::consts::PI * f64::from(k.saturating_sub(1)) / z
            + g.st / g.r
            + 2.0 * inv(g.alpha_t));
    // The configuration is symmetric about the radius through the middle of the
    // spanned group, so each measuring face touches half a span from the base
    // tangent point.
    let half = nominal / (2.0 * bb.cos());
    Span {
        teeth_spanned: k,
        nominal,
        contact_radius: f64::hypot(g.rb, half),
        limits: None,
    }
}

/// The span a metrologist would actually use: the one whose contact lands
/// nearest the pitch circle while staying on the usable flank.
///
/// The usual approach is a rounded empirical formula. Here the admissible range
/// is computed directly — contact must lie between the form radius and the tip —
/// and the best of it chosen. It reports failure instead of returning a number
/// that cannot be measured, which for very few or very many teeth is the honest
/// answer.
///
/// # Errors
///
/// [`MeasurementError::NoValidSpan`] when no `k` contacts the usable flank.
pub fn best_span(g: &Gear) -> Result<Span, MeasurementError> {
    if g.severed {
        return Err(MeasurementError::NoValidSpan);
    }
    let mut best: Option<Span> = None;
    for k in 1..=g.params.teeth {
        let s = span_over_teeth(g, k);
        if s.contact_radius < g.r_j || s.contact_radius > g.ra {
            continue;
        }
        let better = best
            .as_ref()
            .is_none_or(|b| (s.contact_radius - g.r).abs() < (b.contact_radius - g.r).abs());
        if better {
            best = Some(s);
        }
    }
    best.ok_or(MeasurementError::NoValidSpan)
}

/// How many pins the measurement uses, and hence which geometry applies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PinCount {
    /// Two opposed pins: the maximum distance across their outer surfaces.
    Two,
    /// Three pins, as for thread measurement over wires: two pins in adjacent
    /// spaces form a datum, and the measurement is perpendicular from that datum
    /// to the third pin on the far side.
    Three,
}

/// Measurement over pins or balls.
#[derive(Clone, Copy, Debug)]
pub struct OverPins {
    pub pin_diameter: f64,
    pub pin_count: PinCount,
    /// Nominal measurement, mm.
    pub nominal: f64,
    /// Radius of the pin centres.
    pub pin_centre_radius: f64,
    /// Radius at which the pin touches the flank.
    pub contact_radius: f64,
    pub limits: Option<(f64, f64)>,
}

/// Radius of the pin centres, and the radius at which the pin touches the flank.
///
/// The involute angle at the pin centre satisfies
///
/// ```text
/// inv φ = ψ_b + d_p / (2 r_b cos β_b) − π/z
/// ```
///
/// The `cos β_b` is there because the pin contacts in the **normal** plane,
/// while the rest of the expression is transverse — which is the correct
/// treatment for balls in a helical gear, and the reason a normal-module
/// formula gives the wrong answer there.
///
/// # Errors
///
/// [`MeasurementError::PinTooSmall`] if the pin centre would fall inside the
/// base circle, [`MeasurementError::PinTooLarge`] if contact would.
pub fn pin_geometry(g: &Gear, pin_diameter: f64) -> Result<(f64, f64), MeasurementError> {
    let z = f64::from(g.params.teeth);
    let bb = base_helix_angle(g);
    let target = g.psi_b + pin_diameter / (2.0 * g.rb * bb.cos()) - std::f64::consts::PI / z;
    let phi = inv_inverse(target).ok_or(MeasurementError::PinTooSmall)?;
    let r_m = g.rb / phi.cos();

    // The contact point lies on the involute normal through the pin centre, at
    // the pin's radius from it: unwrapped length r_b·tan φ minus d_p/2.
    let u_contact = phi.tan() - pin_diameter / (2.0 * g.rb);
    if u_contact <= 0.0 {
        return Err(MeasurementError::PinTooLarge);
    }
    Ok((r_m, g.rb * f64::hypot(1.0, u_contact)))
}

/// Measurement over two or three pins or balls.
///
/// All four combinations of pin count and tooth-count parity are closed form.
/// For three pins the two adjacent pin centres sit `2π/z` apart, so their common
/// outer tangent is perpendicular to their bisector at `r_M cos(π/z) + d_p/2`.
/// For **odd** `z` a tooth space lies exactly opposite that bisector; for **even**
/// `z` the nearest sits `±π/z` away, which is where the two formulas diverge.
///
/// # Errors
///
/// Returns [`MeasurementError`] when the pin cannot make a valid measurement:
/// contact off the usable flank, or the pin bottoming out on the root diameter.
pub fn over_pins(
    g: &Gear,
    pin_diameter: f64,
    pin_count: PinCount,
) -> Result<OverPins, MeasurementError> {
    let (r_m, contact_radius) = pin_geometry(g, pin_diameter)?;

    if r_m - pin_diameter / 2.0 <= g.rf {
        return Err(MeasurementError::PinBottomsOut);
    }
    if contact_radius < g.r_j || contact_radius > g.ra {
        return Err(MeasurementError::PinContactOffFlank);
    }

    let z = f64::from(g.params.teeth);
    let even = g.params.teeth.is_multiple_of(2);
    let pi = std::f64::consts::PI;
    let nominal = match (pin_count, even) {
        (PinCount::Two, true) => 2.0 * r_m + pin_diameter,
        (PinCount::Two, false) => 2.0 * r_m * (pi / (2.0 * z)).cos() + pin_diameter,
        (PinCount::Three, true) => 2.0 * r_m * (pi / z).cos() + pin_diameter,
        (PinCount::Three, false) => r_m * (1.0 + (pi / z).cos()) + pin_diameter,
    };

    Ok(OverPins {
        pin_diameter,
        pin_count,
        nominal,
        pin_centre_radius: r_m,
        contact_radius,
        limits: None,
    })
}

/// The cutter's tip width in the **normal** plane, mm.
///
/// This is the sharp rack tip width, ignoring the tip round. Reported in the
/// normal plane so it is independent of helix angle, which is what a
/// normal-module tool definition implies.
#[must_use]
pub fn cutter_tip_width(g: &Gear) -> f64 {
    let p = &g.params;
    std::f64::consts::PI * p.module
        - g.st * g.beta.cos()
        - 2.0 * p.module * (p.dedendum - p.profile_shift) * g.alpha_n.tan()
}

impl std::fmt::Display for MeasurementError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Self::NoValidSpan => "no span contacts the usable flank on this gear",
            Self::PinContactOffFlank => "the pin touches outside the usable flank",
            Self::PinBottomsOut => "the pin sits on the root diameter, not the flanks",
            Self::PinTooSmall => "the pin is too small: it falls between the flanks into the root",
            Self::PinTooLarge => "the pin is too large: contact would fall below the base circle",
        };
        f.write_str(s)
    }
}

impl std::error::Error for MeasurementError {}
