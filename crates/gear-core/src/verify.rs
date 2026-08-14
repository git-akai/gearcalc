//! Verification of generated geometry against the cutter that would make it.
//!
//! This is the most valuable machinery inherited from the prior work, and its
//! central idea is worth stating plainly:
//!
//! > **Bound the profile from both sides.** Penetration alone is not sufficient —
//! > an arbitrarily undersized profile passes it trivially. Only penetration
//! > *and* deviation together pin the profile down uniquely.
//!
//! - **penetration** — no gear point may lie inside the cutter at any phase
//!   (material the tool would have removed is still there);
//! - **deviation** — every generated point must be *touched* by the cutter at its
//!   closest approach (the profile sits no further from the tool than a cut could
//!   leave it).
//!
//! It lives in the library rather than in `tests/` so the CLI can sweep it over
//! thousands of cases, which is the whole reason for porting it to Rust.
//!
//! # Departures from the Python original
//!
//! The cutter is described by an **exact signed distance function** rather than a
//! discretised outline. The original computed one and then reverted to polyline
//! distances plus a point-in-polygon containment test; its own handoff notes
//! recommended reinstating the analytic form. Doing so removes the polyline chord
//! error entirely (a floor of ~3e-6 mm that masqueraded as geometry error) and
//! deletes the containment test, which was the least trustworthy step in the
//! suite.

use crate::profile::Gear;

/// Exact signed distance from a point to one cutter tooth. Negative is inside.
///
/// The tooth is convex: a wedge of three faces — two flanks and the tip flat —
/// with its two bottom corners rounded by `ρ`. That is exactly *the wedge eroded
/// by ρ, then Minkowski-summed with a disc of radius ρ*, and the eroded wedge's
/// corners are precisely the tip-round centres. So the distance is
/// `dist(p, wedge) − ρ`, needing three features instead of a discretised arc.
///
/// Coordinates are the rack frame: `y` measured from the gear centre, `x` along
/// the rack's travel.
#[must_use]
pub fn cutter_sdf(g: &Gear, px: f64, py: f64, shift: f64) -> f64 {
    let (ca, sa) = (g.alpha_t.cos(), g.alpha_t.sin());
    let pitch = std::f64::consts::PI * g.mt;
    let yb = g.rf + g.rho; // the eroded wedge's base = the tip-round centre line
    let x1 = g.ac + shift;
    let x2 = pitch - g.ac + shift;
    let dy = py - yb;

    let e1 = (px - x1) * ca + dy * sa; // left flank, inward positive
    let e2 = (x2 - px) * ca + dy * sa; // right flank
    let e3 = dy; // tip flat

    if e1 >= 0.0 && e2 >= 0.0 && e3 >= 0.0 {
        return -(g.rho + e1.min(e2).min(e3));
    }

    // left ray from (x1, yb) heading up-left; right ray from (x2, yb) up-right
    let t1 = ((px - x1) * -sa + dy * ca).max(0.0);
    let d1 = f64::hypot(px - (x1 - sa * t1), py - (yb + ca * t1));
    let t2 = ((px - x2) * sa + dy * ca).max(0.0);
    let d2 = f64::hypot(px - (x2 + sa * t2), py - (yb + ca * t2));
    let tb = ((px - x1) / (x2 - x1)).clamp(0.0, 1.0);
    let db = f64::hypot(px - (x1 + tb * (x2 - x1)), dy);

    d1.min(d2).min(db) - g.rho
}

/// Rack displacements over which tooth 0 is actually generated.
///
/// Derived from the geometry, never guessed. The fillet is cut when the rack is
/// roughly a whole pitch away from the tooth, so the obvious choice — one pitch
/// centred on the tooth — misses the moment the fillet is formed entirely and
/// reports a large false deviation. That single mistake hid two real failure
/// modes in the original suite.
#[must_use]
pub fn rack_travel_range(g: &Gear) -> (f64, f64) {
    let at = g.alpha_t;
    let tau = |u: f64| u * g.rb - g.r * at.sin();
    let mut bounds = vec![g.s_j - g.ac, -g.ac, 0.0, g.r * g.half_pitch];
    if !g.severed {
        for u in [g.u_j, g.u_tip] {
            bounds.push(tau(u) / at.cos() - g.st / 2.0);
        }
    }
    let lo = bounds.iter().copied().fold(f64::INFINITY, f64::min);
    let hi = bounds.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let pad = TRAVEL_PAD_PITCHES * std::f64::consts::PI * g.mt;
    (lo - pad, hi + pad)
}

/// Padding on the derived travel range, in pitches.
///
/// Covers the tail of the generating sweep, where a cutter tooth is still within
/// reach of the profile but no longer forming it. Convergence of the reported
/// deviation with respect to this value is asserted by
/// `travel_padding_is_sufficient`.
const TRAVEL_PAD_PITCHES: f64 = 0.6;

/// How far the gear is allowed to rotate between phase samples, radians.
///
/// This bounds **gear rotation**, not rack travel: a small gear turns far more
/// per unit of rack travel, and it is the rotation that limits accuracy. Chosen
/// by the convergence study in `phase_resolution_has_converged`, which shows the
/// reported deviation stable to well under the acceptance threshold at this step
/// and no better at half of it.
const MAX_ROTATION_STEP: f64 = 1e-3;

/// Hard cap on phase samples, so a pathological case cannot run unbounded.
const MAX_PHASES: usize = 4000;

/// Result of the two-sided cutter check. Both are in millimetres.
#[derive(Clone, Copy, Debug)]
pub struct CutReport {
    /// Deepest intrusion of the gear into the cutter. Must be 0.
    pub penetration: f64,
    /// Furthest any generated point sits from the cutter at its closest
    /// approach. Must be 0 to within sampling.
    pub deviation: f64,
}

/// Two-sided verification of a profile against its generating rack.
#[must_use]
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]
pub fn check_cut(g: &Gear, profile_points: usize) -> CutReport {
    let (r, th) = g.half_profile(profile_points);
    // +theta side only; the other is its mirror
    let px: Vec<f64> = r.iter().zip(&th).map(|(r, t)| r * t.sin()).collect();
    let py: Vec<f64> = r.iter().zip(&th).map(|(r, t)| r * t.cos()).collect();
    // the tip arc is not rack-cut, so it is exempt from the deviation bound
    let generated: Vec<bool> = th.iter().map(|t| *t > g.theta_a + 1e-12).collect();

    let pitch = std::f64::consts::PI * g.mt;
    let (lo, hi) = rack_travel_range(g);
    let rotation = (hi - lo) / g.r;
    let nphase = ((rotation / MAX_ROTATION_STEP).ceil() as usize).clamp(2, MAX_PHASES);

    let copy_lo = (lo / pitch).floor() as i32 - COPY_MARGIN;
    let copy_hi = (hi / pitch).ceil() as i32 + COPY_MARGIN;

    let n = px.len();
    let mut dist = vec![f64::INFINITY; (nphase + 1) * n];
    let mut penetration: f64 = 0.0;

    for k in 0..=nphase {
        let xi = lo + (hi - lo) * (k as f64) / (nphase as f64);
        let phi = xi / g.r;
        let (c, s) = ((-phi).cos(), (-phi).sin());
        for i in 0..n {
            let fx = px[i] * c - py[i] * s;
            let fy = px[i] * s + py[i] * c;
            let mut best = f64::INFINITY;
            for j in copy_lo..=copy_hi {
                let sh = xi + f64::from(j) * pitch;
                // Prune copies that cannot be nearest: the tooth spans one
                // pitch from `sh`, so anything further than a pitch away in x
                // is dominated by a nearer copy.
                if fx < sh - pitch || fx > sh + 2.0 * pitch {
                    continue;
                }
                let d = cutter_sdf(g, fx, fy, sh);
                if d < best {
                    best = d;
                }
            }
            if best < 0.0 {
                penetration = penetration.max(-best);
            }
            dist[k * n + i] = best;
        }
    }

    // Distance to the cutter is quadratic in phase near contact, so refine the
    // sampled minimum by parabolic interpolation rather than paying for more
    // phases.
    let mut deviation: f64 = 0.0;
    for i in 0..n {
        if !generated[i] {
            continue;
        }
        let mut kmin = 0usize;
        let mut dmin = f64::INFINITY;
        for k in 0..=nphase {
            if dist[k * n + i] < dmin {
                dmin = dist[k * n + i];
                kmin = k;
            }
        }
        let kc = kmin.clamp(1, nphase.saturating_sub(1).max(1));
        let refined = if kc < nphase {
            let (d0, d1, d2) = (
                dist[(kc - 1) * n + i],
                dist[kc * n + i],
                dist[(kc + 1) * n + i],
            );
            let denom = d0 - 2.0 * d1 + d2;
            if denom > 0.0 {
                d1 - (d2 - d0).powi(2) / (8.0 * denom)
            } else {
                d1
            }
        } else {
            dmin
        };
        deviation = deviation.max(dmin.min(refined).max(0.0));
    }

    CutReport {
        penetration,
        deviation,
    }
}

/// Extra rack copies considered on each side of the travel range.
///
/// A copy further than this cannot be the nearest feature for any point, since
/// the pruning test already rejects anything beyond one pitch.
const COPY_MARGIN: i32 = 3;

/// Check the fillet without using the envelope derivation at all.
///
/// Asserts only that every fillet point lies exactly `ρ` from the path traced by
/// the cutter's tip-round **centre**. This is deliberately independent of how the
/// trochoid was derived, and it is what proved the fillet correct while the rack
/// simulation was still misreporting.
///
/// The distance is measured to the centre path's **segments**, not to its sample
/// points: with a sharp-cornered rack (`ρ → 0`) the fillet lies *on* the centre
/// path, so point-sampling spacing would otherwise dominate the very quantity
/// being measured.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn fillet_envelope_error(g: &Gear, fillet_points: usize, path_points: usize) -> f64 {
    let mut pts = Vec::with_capacity(fillet_points);
    for i in 0..fillet_points {
        let t = i as f64 / (fillet_points - 1) as f64;
        let (r, th) = g.trochoid_at(g.s_j + t * (0.0 - g.s_j));
        pts.push((r * th.sin(), r * th.cos()));
    }

    let s_lo = (g.s_j * 3.0 - 1.0).min(-1.0);
    let s_hi = g.s_j.abs() * 3.0 + 1.0;
    let mut path = Vec::with_capacity(path_points);
    for i in 0..path_points {
        let t = i as f64 / (path_points - 1) as f64;
        let s = s_lo + t * (s_hi - s_lo);
        let phi = (s - g.ac) / g.r;
        path.push((
            s * phi.cos() - (g.r - g.bc) * phi.sin(),
            s * phi.sin() + (g.r - g.bc) * phi.cos(),
        ));
    }

    let mut worst: f64 = 0.0;
    for &(qx, qy) in &pts {
        let mut best = f64::INFINITY;
        for w in path.windows(2) {
            let (ax, ay) = w[0];
            let (bx, by) = w[1];
            let (ex, ey) = (bx - ax, by - ay);
            let len2 = ex * ex + ey * ey;
            let t = if len2 > 0.0 {
                (((qx - ax) * ex + (qy - ay) * ey) / len2).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let d = f64::hypot(qx - (ax + t * ex), qy - (ay + t * ey));
            if d < best {
                best = d;
            }
        }
        worst = worst.max((best - g.rho).abs());
    }
    worst
}

/// Verify the analytic cutter SDF against an independently built polyline of the
/// same tooth. Returns the worst disagreement in the near field.
///
/// The polyline is kept purely as this cross-check — it is no longer on the path
/// used by [`check_cut`].
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn sdf_matches_polyline(g: &Gear, arc_points: usize, samples: usize) -> f64 {
    let (ca, sa) = (g.alpha_t.cos(), g.alpha_t.sin());
    let pitch = std::f64::consts::PI * g.mt;
    let top = g.r
        + g.params.module * (g.params.addendum + g.params.profile_shift)
        + 0.25 * g.params.module;
    let yc = g.r - g.bc;

    let mut v: Vec<(f64, f64)> = Vec::with_capacity(2 * arc_points + 2);
    v.push((g.st / 2.0 - (top - g.r) * g.alpha_t.tan(), top));
    for i in 0..arc_points {
        let t = i as f64 / (arc_points - 1) as f64;
        let a = (std::f64::consts::PI + g.alpha_t)
            + t * (1.5 * std::f64::consts::PI - (std::f64::consts::PI + g.alpha_t));
        v.push((g.ac + g.rho * a.cos(), yc + g.rho * a.sin()));
    }
    for i in 0..arc_points {
        let t = i as f64 / (arc_points - 1) as f64;
        let a = 1.5 * std::f64::consts::PI
            + t * ((2.0 * std::f64::consts::PI - g.alpha_t) - 1.5 * std::f64::consts::PI);
        v.push((pitch - g.ac + g.rho * a.cos(), yc + g.rho * a.sin()));
    }
    v.push((pitch - g.st / 2.0 + (top - g.r) * g.alpha_t.tan(), top));
    let _ = (ca, sa);

    // Deterministic lattice rather than a random cloud, so the check is
    // reproducible without carrying a PRNG.
    let pad = 0.4 * g.params.module;
    let (x_lo, x_hi) = (
        v.iter().map(|p| p.0).fold(f64::INFINITY, f64::min) - pad,
        v.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max) + pad,
    );
    let (y_lo, y_hi) = (g.rf - pad, g.ra);

    let side = samples.isqrt().max(2) + 1;
    let mut worst: f64 = 0.0;
    for iy in 0..side {
        for ix in 0..side {
            let qx = x_lo + (x_hi - x_lo) * (ix as f64) / ((side - 1) as f64);
            let qy = y_lo + (y_hi - y_lo) * (iy as f64) / ((side - 1) as f64);
            let a = cutter_sdf(g, qx, qy, 0.0).abs();
            if a >= 0.5 * g.params.module {
                continue; // near field only, where the comparison is meaningful
            }
            let mut b = f64::INFINITY;
            for w in v.windows(2) {
                let (ax, ay) = w[0];
                let (bx, by) = w[1];
                let (ex, ey) = (bx - ax, by - ay);
                let len2 = ex * ex + ey * ey;
                let t = if len2 > 0.0 {
                    (((qx - ax) * ex + (qy - ay) * ey) / len2).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                b = b.min(f64::hypot(qx - (ax + t * ex), qy - (ay + t * ey)));
            }
            worst = worst.max((a - b).abs());
        }
    }
    worst
}
