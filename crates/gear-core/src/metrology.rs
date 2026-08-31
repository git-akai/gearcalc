//! Measurements you can actually take on a finished gear: span over teeth,
//! measurement over pins or balls, and the cutter tip width.
//!
//! These are what go on a drawing, so the emphasis is on reporting *whether a
//! measurement is takeable* as much as on the number itself. A span that
//! contacts below the form circle, or a pin that bottoms out in the root, is not
//! a measurement — and returning a plausible number for one is worse than
//! returning nothing.
//!
//! Per docs/reference.md#metrology, only **nominal** values are produced. Minimum and maximum
//! need a tooth thickness tolerance from JGMA 1103-01, which is not available;
//! the result types carry the space for it so adding the data later is a data
//! change rather than a redesign.

use crate::involute::{inv, inv_inverse};
use crate::tooth::Tooth;

/// This gear's base helix angle, radians — [`crate::plane::base_helix_angle`]
/// read off a [`Tooth`].
///
/// The identity lives in [`crate::plane`]; what this adds is which two of a
/// gear's angles go into it, which is the part a call site would otherwise have
/// to remember.
#[must_use]
pub fn base_helix_angle(g: &Tooth) -> f64 {
    crate::plane::base_helix_angle(g.beta, g.alpha_n)
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
    ///
    /// External gears only — a ring's space narrows *outward*, so there is no
    /// pin too small to seat in one.
    PinTooSmall,
    /// The pin is too large to seat: contact would run past the base circle, or —
    /// in a ring, whose space narrows outward — the centre itself would.
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
pub fn span_over_teeth(g: &Tooth, k: u32) -> Span {
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
pub fn best_span(g: &Tooth) -> Result<Span, MeasurementError> {
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

/// Span over `k` teeth starting at tooth `j`, on a gear whose teeth may differ.
///
/// # The span is a distance between two flank seats, and always was
///
/// A span is a chord along a common tangent to the base circle, and the distance
/// between two involutes of one base circle measured along any such tangent is
/// `r_b` times the difference of their origin angles — whichever tangent. So the
/// reading is
///
/// ```text
/// W = r_b cos β_b · [ flank(j+k−1, +1) − flank(j, −1) ]
/// ```
///
/// and with `flank(i, ±1) = seat_i ± ψ_i` and `seat_i = 2πi/z + λ(ψ̄ − ψ_i)`
/// that is
///
/// ```text
/// W = r_b cos β_b · [ 2π(k−1)/z + (1+λ) ψ_j + (1−λ) ψ_{j+k−1} ]
/// ```
///
/// Two things fall out that are worth stating. The evenly cut gear is this with
/// every `ψ` equal, which gives back `2π(k−1)/z + 2ψ_b` and so
/// [`span_over_teeth`] exactly — a **value**, not a limit. And **λ reaches a
/// span**, where it reaches neither the flanks nor the commanded centre
/// distance: a span is measured between flanks of *different teeth*, and the
/// indexing offset is precisely what moves one relative to another.
///
/// # Where the caliper sits
///
/// The value does not depend on which common tangent is used, but the *contact
/// radii* do — the faces slide along the flanks as the caliper turns. There is
/// therefore a family of placements, and validity is that **some** placement
/// puts both contacts on usable flank. Since a contact radius rises with its
/// unwrapped length and the two lengths sum to the span, that is an
/// intersection of two intervals and needs no search.
#[must_use]
pub fn span_over_teeth_at(gear: &crate::gear::Gear, j: usize, k: u32) -> Option<Span> {
    if k == 0 {
        return None;
    }
    let z = gear.teeth();
    let last = (j + k as usize - 1) % z;
    let mean = gear.mean();
    let bb = base_helix_angle(mean);

    // **Grouped so the cancellation happens first.** Taking the difference of two
    // *accumulated* seats — `flank_seat(last) − flank_seat(j)` — is arithmetically
    // the same and numerically is not: `τ·k/z − τ/z` and `τ(k−1)/z` differ by an
    // ulp or two, which is enough to give an evenly cut gear two different ends
    // to a range whose true width is exactly zero. The mean-seat term `ψ̄` cancels
    // outright, and the λ term is a difference of two `ψ` that is *exactly* zero
    // when the teeth agree (`docs/corrections.md`).
    let lam = mean.params.index_offset;
    let (psi_first, psi_last) = (gear.tooth(j).0.psi_b, gear.tooth(last).0.psi_b);
    let sweep = std::f64::consts::TAU * f64::from(k - 1) / z as f64
        + lam * (psi_first - psi_last)
        + psi_first
        + psi_last;
    let nominal = mean.rb * bb.cos() * sweep;
    if !nominal.is_finite() || nominal <= 0.0 {
        return None;
    }

    // The two unwrapped lengths sum to the span; each contact radius rises with
    // its own. So the placements that keep a contact on usable flank are an
    // interval in that length, and both must hold at once.
    let total = sweep;
    let roll_at = |radius: f64| ((radius / mean.rb).powi(2) - 1.0).max(0.0).sqrt();
    let usable = |t: &Tooth| (roll_at(t.r_j), roll_at(t.ra));
    let (a_lo, a_hi) = usable(gear.tooth(j).0);
    let (b_lo, b_hi) = usable(gear.tooth(last).0);
    // `u_a ∈ [a_lo, a_hi]` and `total − u_a ∈ [b_lo, b_hi]`.
    let lo = a_lo.max(total - b_hi);
    let hi = a_hi.min(total - b_lo);
    if lo > hi {
        return None;
    }
    // Report the placement nearest the symmetric one, which is what a
    // metrologist centres on and what an evenly cut gear gives exactly.
    let u_a = (total / 2.0).clamp(lo, hi);

    Some(Span {
        teeth_spanned: k,
        nominal,
        contact_radius: mean.rb * f64::hypot(1.0, u_a),
        limits: None,
    })
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
pub fn pin_geometry(g: &Tooth, pin_diameter: f64) -> Result<(f64, f64), MeasurementError> {
    pin_seat(
        std::f64::consts::PI / f64::from(g.params.teeth) - g.psi_b,
        g.rb,
        base_helix_angle(g),
        pin_diameter,
        1.0,
    )
}

/// Where a pin sits in a tooth space, for **either** kind of gear.
///
/// Returns `(pin centre radius, contact radius)`. `sigma` is `+1` for an external
/// gear and `−1` for a ring — the same sign convention [`crate::mesh::MeshKind`]
/// uses, and the whole of the difference between the two.
///
/// # One relation, read from either side
///
/// An involute's *offset* is another involute of the same base circle, so the
/// perpendicular distance from a point to a flank is `r_b` times the difference
/// of their origin angles. A pin resting on both flanks of a space has its centre
/// on the bisector at `θ = π/z`, and its own origin angle is `θ ∓ inv φ` — minus
/// for an external gear, plus for a ring, because a ring's tooth *gains* angle
/// outward. Setting that distance to `d_p/2`:
///
/// ```text
/// inv φ_M = σ ( ψ_b + d_p / (2 r_b cos β_b) − π/z )
/// u_c     = tan φ_M − σ d_p / (2 r_b)
/// ```
///
/// The signs say something physical. An external gear's space narrows *inward*,
/// so a larger pin rides higher and touches **below** its own centre. A ring's
/// space narrows *outward*, so a larger pin sits deeper — at smaller radius —
/// and touches **above** its centre. Both fall out of `σ` rather than being
/// separate cases.
fn pin_seat(
    half_space: f64,
    rb: f64,
    beta_b: f64,
    pin_diameter: f64,
    sigma: f64,
) -> Result<(f64, f64), MeasurementError> {
    // `half_space` is half the angular width of the space **at the base circle**,
    // which is what the pin actually sits in. For an evenly cut gear that is
    // `π/z − ψ_b` — the tooth taken out of the pitch — and it was written that
    // way here. Taken as an argument instead, because on a gear whose teeth are
    // not all the same thickness a space is bounded by two *different* teeth and
    // has no expression in `z` and one `ψ_b`. The evenly cut gear is the value
    // where the two teeth agree.
    let target = sigma * (pin_diameter / (2.0 * rb * beta_b.cos()) - half_space);
    // A negative target means the pin centre would have to sit inside the base
    // circle, where there is no involute to touch — but *which* pin fault that is
    // depends on the kind. On an external gear the space narrows inward, so it is
    // a pin too **small** to bridge the flanks; inside a ring the space narrows
    // outward, so the same failure is a pin too **large** to reach a seat. Same
    // arithmetic, opposite diagnosis, and reporting the external one for a ring
    // sends the designer the wrong way.
    let phi = inv_inverse(target).ok_or(if sigma > 0.0 {
        MeasurementError::PinTooSmall
    } else {
        MeasurementError::PinTooLarge
    })?;
    let r_m = rb / phi.cos();

    // The contact point lies on the involute normal through the pin centre, at
    // the pin's radius from it: unwrapped length r_b·tan φ, less the pin.
    let u_contact = phi.tan() - sigma * pin_diameter / (2.0 * rb);
    if u_contact <= 0.0 {
        return Err(MeasurementError::PinTooLarge);
    }
    Ok((r_m, rb * f64::hypot(1.0, u_contact)))
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
    g: &Tooth,
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
    let pi = std::f64::consts::PI;

    // **One measurement, not four.** The four cases — two pins or three, odd `z`
    // or even — were a `match` returning four expressions, each correct and each
    // a separate place to be wrong. They are the same measurement taken over
    // different seats, so what they share is geometry rather than arithmetic:
    // the pins are equal-diameter circles at known places, and the caliper reads
    // the distance between two parallel planes touching them.
    //
    // A space sits at `(2j+1)π/z` from the first tooth's centreline, so seat `j`
    // has that angular position and a radius of `r_m`.
    let at = |j: f64| {
        let c = (2.0 * j + 1.0) * pi / z;
        [r_m * c.cos(), r_m * c.sin()]
    };
    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    let unit = |v: [f64; 2]| {
        let n = f64::hypot(v[0], v[1]);
        [v[0] / n, v[1] / n]
    };

    let nominal = match pin_count {
        // Across two pins: the planes are perpendicular to the line joining the
        // centres, so the reading is that distance plus one pin. The seat
        // nearest half a turn away is exactly opposite for even `z` and half a
        // pitch off it for odd, which `round` picks — so the parity is a value
        // of the expression rather than a branch on it.
        PinCount::Two => {
            let (a, b) = (at(0.0), at((z / 2.0).round()));
            f64::hypot(a[0] - b[0], a[1] - b[1]) + pin_diameter
        }
        // Two **adjacent** pins make the datum: equal diameters, so the plane
        // resting on them is parallel to the line joining their centres. The
        // third seat is the one nearest opposite their bisector — exactly
        // opposite for odd `z`, half a pitch off for even.
        PinCount::Three => {
            let (p1, p2) = (at(0.0), at(1.0));
            let p3 = at(((z + 1.0) / 2.0).round());
            let along = [p2[0] - p1[0], p2[1] - p1[1]];
            let n = unit([along[1], -along[0]]);
            let n = if dot(n, p1) < 0.0 { [-n[0], -n[1]] } else { n };
            (dot(p1, n) - dot(p3, n)).abs() + pin_diameter
        }
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

/// The span a metrologist would use, and how far it varies around the
/// revolution.
///
/// One `k` for the whole gear — a caliper is set once and carried round — so the
/// admissible counts are intersected over every starting tooth rather than
/// chosen per tooth. The `k` picked is the one whose contact lands nearest the
/// pitch circle, averaged over the revolution, which is [`best_span`]'s rule
/// read across all the positions instead of one.
///
/// Returns the span at each starting tooth's `[smallest, largest]`. An evenly
/// cut gear's two ends are the **same bits**, so a caller can report a range
/// unconditionally and have an ordinary gear read as a single number.
///
/// # Errors
///
/// [`MeasurementError::NoValidSpan`] when no `k` is measurable at *every*
/// position. That is stricter than asking per tooth, and deliberately: a span
/// that can only be taken at some angular positions is not a measurement of the
/// gear.
pub fn best_span_around(gear: &crate::gear::Gear) -> Result<(Span, [f64; 2]), MeasurementError> {
    let z = gear.teeth();
    let mean = gear.mean();
    let mut best: Option<(Span, [f64; 2], f64)> = None;

    for k in 1..=gear.mean().params.teeth {
        let mut lo = f64::MAX;
        let mut hi = f64::MIN;
        let mut worst_offset = 0.0_f64;
        let mut at_first = None;
        let mut every = true;
        for j in 0..z {
            match span_over_teeth_at(gear, j, k) {
                Some(s) => {
                    lo = lo.min(s.nominal);
                    hi = hi.max(s.nominal);
                    worst_offset = worst_offset.max((s.contact_radius - mean.r).abs());
                    if j == 0 {
                        at_first = Some(s);
                    }
                }
                None => {
                    every = false;
                    break;
                }
            }
        }
        if !every {
            continue;
        }
        let Some(s) = at_first else { continue };
        let better = best.as_ref().is_none_or(|(_, _, w)| worst_offset < *w);
        if better {
            best = Some((s, [lo, hi], worst_offset));
        }
    }

    best.map(|(s, range, _)| (s, range))
        .ok_or(MeasurementError::NoValidSpan)
}

/// Measurement over pins at one angular position, on a gear whose teeth may
/// differ.
///
/// The seats are read from the gear rather than assumed identical: a pin sits in
/// the space bounded by two *different* teeth, so its centre radius is that
/// space's own. The reading is then the same plane-to-plane geometry
/// [`over_pins`] uses, which is why that one already generalises — it was
/// written in vectors rather than in `z`.
///
/// `start` chooses which space the first pin sits in, so sweeping it is what
/// makes the measurement a range.
///
/// # Errors
///
/// The same as [`over_pins`], for whichever seat cannot take the pin.
pub fn over_pins_at(
    gear: &crate::gear::Gear,
    pin_diameter: f64,
    pin_count: PinCount,
    start: usize,
) -> Result<OverPins, MeasurementError> {
    let z = gear.teeth();
    let mean = gear.mean();
    let bb = base_helix_angle(mean);

    // Where the pin in the space after tooth `i` sits: its own centre radius,
    // and the angle its centre sits at.
    // The index is deliberately **not** wrapped here. `Gear` wraps it for the
    // tooth lookups, and wrapping it first would turn an offset of one space
    // into one of `z − 1` — the same angle, and not the same `cos`. That last
    // ulp reaches the screen as a range on a gear that has none.
    let seat = |i: usize| -> Result<([f64; 2], f64), MeasurementError> {
        let (r_m, contact) = pin_seat(gear.space_half_angle(i), mean.rb, bb, pin_diameter, 1.0)?;
        let t = gear.tooth(i).0;
        if r_m - pin_diameter / 2.0 <= t.rf {
            return Err(MeasurementError::PinBottomsOut);
        }
        if contact < t.r_j || contact > t.ra {
            return Err(MeasurementError::PinContactOffFlank);
        }
        // Relative to the first pin's space, so every angle in the measurement
        // is a *difference* and the pitch terms cancel exactly.
        let c = gear.space_centre_delta(start, i);
        Ok(([r_m * c.cos(), r_m * c.sin()], contact))
    };

    let dot = |a: [f64; 2], b: [f64; 2]| a[0] * b[0] + a[1] * b[1];
    // The space nearest half a turn away, and the one nearest opposite a pair's
    // bisector — integer arithmetic, so the parity is a value of the expression
    // rather than a branch on it, and there is no float to truncate.
    let half = z.div_ceil(2);
    let opposite_pair = (z + 1).div_ceil(2);

    let (p1, contact) = seat(start)?;
    let nominal = match pin_count {
        PinCount::Two => {
            let (p2, _) = seat(start + half)?;
            f64::hypot(p1[0] - p2[0], p1[1] - p2[1]) + pin_diameter
        }
        PinCount::Three => {
            let (p2, _) = seat(start + 1)?;
            let (p3, _) = seat(start + opposite_pair)?;
            let along = [p2[0] - p1[0], p2[1] - p1[1]];
            let n = f64::hypot(along[1], -along[0]);
            let n = [along[1] / n, -along[0] / n];
            let n = if dot(n, p1) < 0.0 { [-n[0], -n[1]] } else { n };
            (dot(p1, n) - dot(p3, n)).abs() + pin_diameter
        }
    };

    Ok(OverPins {
        pin_diameter,
        pin_count,
        nominal,
        pin_centre_radius: f64::hypot(p1[0], p1[1]),
        contact_radius: contact,
        limits: None,
    })
}

/// The cutter's tip width in the **normal** plane, mm.
///
/// This is the sharp rack tip width, ignoring the tip round. Reported in the
/// normal plane so it is independent of helix angle, which is what a
/// normal-module tool definition implies.
#[must_use]
pub fn cutter_tip_width(g: &Tooth) -> f64 {
    let p = &g.params;
    std::f64::consts::PI * p.module
        - g.st * g.beta.cos()
        - 2.0 * p.module * (p.dedendum - p.profile_shift) * g.alpha_n.tan()
}

impl crate::note::Explain for MeasurementError {
    /// Why the measurement cannot be taken, as a key and its values.
    fn note(&self) -> crate::note::Note {
        use crate::note::key;
        crate::note::Note::new(match self {
            Self::NoValidSpan => key::ERROR_MEASURE_NO_VALID_SPAN,
            Self::PinContactOffFlank => key::ERROR_MEASURE_PIN_OFF_FLANK,
            Self::PinBottomsOut => key::ERROR_MEASURE_PIN_BOTTOMS_OUT,
            Self::PinTooSmall => key::ERROR_MEASURE_PIN_TOO_SMALL,
            Self::PinTooLarge => key::ERROR_MEASURE_PIN_TOO_LARGE,
        })
    }
}

/// English, for the CLI and for `Debug`. **Not** what the browser renders — see
/// [`MeasurementError::note`].
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

// ------------------------------------------------------ internal gears ---

/// Measurement **between** pins or balls, for a ring.
///
/// The internal counterpart of [`OverPins`]. The distinction is not cosmetic: on
/// an external gear the pins stand proud and you measure *across their outer
/// surfaces*, so the pin diameter **adds**; inside a ring they seat in opposing
/// spaces and you measure *between their inner surfaces*, so it **subtracts**.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
pub struct BetweenPins {
    pub pin_diameter: f64,
    /// Nominal measurement, mm.
    pub nominal: f64,
    /// Radius of the pin centres.
    pub pin_centre_radius: f64,
    /// Radius at which the pin touches the flank.
    pub contact_radius: f64,
    /// Space for the tolerance band once JGMA 1103-01 is available (docs/reference.md#metrology), as on
    /// [`OverPins`].
    pub limits: Option<(f64, f64)>,
}

/// Measurement between two pins or balls seated in a ring's opposing spaces.
///
/// # Why two pins and not three
///
/// Three pins exist for an external gear because an odd tooth count leaves no
/// space diametrically opposite, and a micrometer needs a stable flat datum;
/// two adjacent pins provide one. Inside a bore neither problem arises — the
/// odd-count offset is the same `cos(π/2z)` correction, and a bore gauge needs
/// no datum — so three-pin internal measurement is not a practice anyone takes,
/// and inventing its geometry would be describing a measurement rather than
/// reporting one.
///
/// # Errors
///
/// [`MeasurementError`] when the pin cannot make a valid measurement: contact off
/// the usable flank, or a pin so large it reaches the root circle.
pub fn between_pins(
    ring: &crate::ring::Ring,
    pin_diameter: f64,
) -> Result<BetweenPins, MeasurementError> {
    let (r_m, contact_radius) = pin_seat(
        std::f64::consts::PI / f64::from(ring.teeth) - ring.psi_b,
        ring.rb,
        ring.base_helix_angle(),
        pin_diameter,
        -1.0,
    )?;

    // A ring's root circle is *outside* its teeth, so "bottoming out" is the pin
    // reaching outward into the root rather than inward.
    if r_m + pin_diameter / 2.0 >= ring.rf {
        return Err(MeasurementError::PinBottomsOut);
    }
    // The usable flank runs from the tip — the ring's *smallest* radius — out to
    // where the cutter handed over to the fillet.
    let form_radius = ring.involute_at(ring.u_j).0;
    if contact_radius < ring.ra || contact_radius > form_radius {
        return Err(MeasurementError::PinContactOffFlank);
    }

    let z = f64::from(ring.teeth);
    let pi = std::f64::consts::PI;
    // Even tooth counts put a space exactly opposite; odd ones leave the nearest
    // half a pitch away, which is the same correction an external gear takes.
    let across = if ring.teeth.is_multiple_of(2) {
        2.0 * r_m
    } else {
        2.0 * r_m * (pi / (2.0 * z)).cos()
    };

    Ok(BetweenPins {
        pin_diameter,
        nominal: across - pin_diameter,
        pin_centre_radius: r_m,
        contact_radius,
        limits: None,
    })
}
