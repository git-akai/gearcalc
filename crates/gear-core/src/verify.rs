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

use crate::tooth::Tooth;

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
pub fn cutter_sdf(g: &Tooth, px: f64, py: f64, shift: f64) -> f64 {
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
pub fn rack_travel_range(g: &Tooth) -> (f64, f64) {
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
pub fn check_cut(g: &Tooth, profile_points: usize) -> CutReport {
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
pub fn fillet_envelope_error(g: &Tooth, fillet_points: usize, path_points: usize) -> f64 {
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
pub fn sdf_matches_polyline(g: &Tooth, arc_points: usize, samples: usize) -> f64 {
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

// -------------------------------------------------------------------------- //
//  the same idea for a ring, whose cutter is a pinion
// -------------------------------------------------------------------------- //

/// How far a ring's generated profile sits from the boundary its cutter leaves.
#[derive(Clone, Copy, Debug)]
pub struct RingCutReport {
    /// Worst distance, mm, between the simulated cut and the analytic profile.
    ///
    /// A **distance**, not an angular gap at equal radius, because a ring's
    /// fillet is stationary in radius at its crown — the two fillets meet there
    /// — so radius is a useless coordinate in exactly the place the two curves
    /// are closest. Comparing by it reported 18 µm of "error" that was entirely
    /// the coordinate's fault.
    pub worst_distance: f64,
    /// Radii that had a cutter point to compare against.
    pub samples: usize,
}

/// Simulate the cut and compare it with [`crate::ring::Ring`]'s analytic
/// profile.
///
/// # Why this is the gate rather than another test
///
/// Everything the ring module does is *constructive*: an involute with a flipped
/// sign, a trochoid from a rolling corner, a junction from the line of action, a
/// phase from an offset involute. Each piece has been checked against something,
/// but a construction can be right in every part and wrong in how the parts are
/// placed. Simulating the cut asks the one question that covers all of it —
/// **is this the shape that tool would leave?**
///
/// The cutter is swept through the rolling motion and every point of its
/// boundary transformed into the ring's frame. At each radius the tooth's
/// boundary is the *smallest* angle any cutter point ever reached, since
/// anything beyond that was machined away. That envelope is what the analytic
/// profile is compared against.
///
/// Nothing here consults `Ring`'s flank, fillet or junction — only the cutter
/// and the rolling, which is the point.
#[must_use]
pub fn ring_cut_envelope(ring: &crate::ring::Ring, radii: usize, phases: usize) -> Vec<(f64, f64)> {
    // Two circular pitches either side. One is not enough: a cutter tooth's
    // engagement with one space lasts longer than a pitch of travel, and cutting
    // it short leaves the ring's flank near its tip ungenerated — which showed
    // up as the envelope being 0.1 mm wide there. Four gives the same answer as
    // two, so two is converged.
    ring_cut_envelope_spans(ring, radii, phases, 2.0)
}

/// The same, sweeping `spans` circular pitches of travel either side of the
/// deepest cut.
#[must_use]
pub fn ring_cut_envelope_spans(
    ring: &crate::ring::Ring,
    radii: usize,
    phases: usize,
    spans: f64,
) -> Vec<(f64, f64)> {
    use crate::involute::inv;
    use std::f64::consts::PI;

    let cut = &ring.cut;
    let r_bc = cut.cutter_radius * ring.alpha_t.cos();
    let rho = cut.tip_round;
    let r_tip = cut.corner_radius + rho;

    // **The cutter's tooth comes from the cutter.** Deriving it as
    // `π m_t − ring_tooth` — the tool that would complement an unshifted ring at
    // reference centres — made this simulation share the model's assumption, so
    // it agreed to 2.7 µm on a shifted ring whose cutter was 0.44 mm out of
    // place. A check built from the thing under test measures nothing. docs/corrections.md.
    let cutter_tooth = cut.cutter_tooth;
    let half_angle_at = |radius: f64| {
        let alpha = (r_bc / radius).acos();
        cutter_tooth / (2.0 * cut.cutter_radius) + inv(ring.alpha_t) - inv(alpha)
    };
    let t_g = (((cut.corner_radius / r_bc).powi(2) - 1.0).max(0.0)).sqrt();
    let r_tan = r_bc * f64::hypot(1.0, t_g + rho / r_bc);
    // The corner's centre sits on the offset involute at its OWN radius, not at
    // the flank's tangency radius — the two differ by the round.
    let theta_g = half_angle_at(cut.corner_radius) - rho / r_bc;

    // One cutter tooth's boundary, on the flank that cuts this side: the
    // involute up to where the round takes over, then the round, then the tip.
    let polar = |radius: f64, angle: f64| (radius * angle.sin(), radius * angle.cos());
    let mut boundary: Vec<(f64, f64)> = Vec::new();
    const FLANK: usize = 600;
    let r_low = r_bc * 1.000_001;
    for i in 0..=FLANK {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f64 / FLANK as f64;
        let radius = r_low + (r_tan - r_low) * t;
        boundary.push(polar(radius, half_angle_at(radius)));
    }
    let (cx, cy) = polar(cut.corner_radius, theta_g);
    let (tx, ty) = polar(r_tan, half_angle_at(r_tan));
    let start = (tx - cx).atan2(ty - cy);
    let end = (0.0 - cx).atan2(r_tip - cy);
    const ROUND: usize = 300;
    for i in 0..=ROUND {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f64 / ROUND as f64;
        let a = start + (end - start) * t;
        boundary.push((cx + rho * a.sin(), cy + rho * a.cos()));
    }
    const TIP: usize = 120;
    let tip_angle = (cx + rho * end.sin()).atan2(cy + rho * end.cos());
    for i in 0..=TIP {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f64 / TIP as f64;
        boundary.push(polar(r_tip, tip_angle * (1.0 - t)));
    }

    // Sweep the cutter and keep the smallest angle reached at each radius.
    let span = spans * PI * ring.mt;
    let mut envelope = vec![f64::INFINITY; radii];
    for j in 0..=phases {
        #[allow(clippy::cast_precision_loss)]
        let s = -span + 2.0 * span * (j as f64 / phases as f64);
        // Rolling is on the operating circles, which are the reference ones only
        // when the cut sits at reference centres.
        let phi = s / cut.cutter_operating_radius;
        let rotation = (s - cut.phase) / cut.workpiece_operating_radius;
        // The corner sits at −θ_g from the cutter's tooth centreline, not +θ_g:
        // it is on the flank *facing* the ring's tooth. Mirroring the tooth and
        // turning the other way keeps the corner where `corner_centre_at` puts
        // it while moving the flank to the side that actually does the cutting.
        let (sin_t, cos_t) = (phi + theta_g).sin_cos();
        for &(px, py) in &boundary {
            let px = -px;
            let x = px * cos_t + py * sin_t;
            let y = cut.centre_distance + (py * cos_t - px * sin_t);
            let radius = f64::hypot(x, y);
            if radius <= ring.ra || radius >= ring.rf {
                continue;
            }
            // Every space is the same space: a cutter tooth several pitches
            // round from the contact is cutting an identical one, so the angle
            // folds into a single pitch. Without that fold the sweep only ever
            // touches a sliver of the profile, which is how this was caught.
            let pitch = 2.0 * ring.half_pitch;
            let mut angle = (x.atan2(y) - rotation) % pitch;
            if angle > ring.half_pitch {
                angle -= pitch;
            } else if angle < -ring.half_pitch {
                angle += pitch;
            }
            let angle = angle.abs();
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bin = (((radius - ring.ra) / (ring.rf - ring.ra)) * radii as f64) as usize;
            let bin = bin.min(radii - 1);
            if angle < envelope[bin] {
                envelope[bin] = angle;
            }
        }
    }

    envelope
        .iter()
        .enumerate()
        .filter(|(_, a)| a.is_finite())
        .map(|(bin, &a)| {
            #[allow(clippy::cast_precision_loss)]
            let radius = ring.ra + (ring.rf - ring.ra) * (bin as f64 + 0.5) / radii as f64;
            (radius, a)
        })
        .collect()
}

/// Compare that envelope with the analytic profile.
///
/// Every simulated point is measured to the nearest point of a dense sampling of
/// the analytic half-tooth. That is the whole comparison: two curves, one
/// distance.
///
/// # What it does not yet see
///
/// The simulated cutter is its involute flank, its tip corner round and its tip
/// — **not its own fillet**, below its base circle. So the sweep cannot show
/// what happens beneath [`crate::ring::Ring::generation_limit`], where the
/// cutter's involute has run out and its fillet region is what passes. In that
/// band the sweep has nothing to say and the agreement it reports is not
/// evidence either way.
///
/// That band is small on ordinary designs — 0.08 mm on a 43-tooth ring cut by a
/// 20-tooth cutter — and `Ring` flags it rather than pretending otherwise. But
/// it is the honest edge of this gate, and closing it means giving the simulated
/// cutter a fillet of its own.
#[must_use]
pub fn check_ring_cut(ring: &crate::ring::Ring, radii: usize, phases: usize) -> RingCutReport {
    // A dense analytic reference, section by section so none is starved.
    const DENSE: usize = 3000;
    let mut reference: Vec<(f64, f64)> = Vec::with_capacity(2 * DENSE);
    for i in 0..=DENSE {
        #[allow(clippy::cast_precision_loss)]
        let t = i as f64 / DENSE as f64;
        reference.extend(
            [
                Some(ring.involute_at(ring.u_tip + (ring.u_j - ring.u_tip) * t)),
                // Nothing to compare against where no fillet was cut; the flank
                // reference above already runs to the root circle there.
                ring.fillet
                    .map(|f| ring.trochoid_at(f.s_j + (f.s_root - f.s_j) * t)),
            ]
            .into_iter()
            .flatten()
            .map(|(r, th)| (r * th.sin(), r * th.cos())),
        );
    }

    let mut worst_distance = 0.0_f64;
    let envelope = ring_cut_envelope(ring, radii, phases);
    let samples = envelope.len();
    for (radius, angle) in envelope {
        if angle >= ring.half_pitch {
            continue;
        }
        let (x, y) = (radius * angle.sin(), radius * angle.cos());
        let near = reference
            .iter()
            .map(|&(rx, ry)| f64::hypot(x - rx, y - ry))
            .fold(f64::INFINITY, f64::min);
        worst_distance = worst_distance.max(near);
    }

    RingCutReport {
        worst_distance,
        samples,
    }
}

// ---------------------------------------------------------------------------
// The loaded flank's phase, measured off the generated outlines
// ---------------------------------------------------------------------------

/// A gear's tooth half-width against radius, read off its **drawn outline**.
///
/// Above the fillet an external tooth's outline is two involute flanks and a tip
/// arc, so at any radius in that band the largest angular departure from the
/// tooth centreline *is* the tooth's half-width there. Below the fillet the root
/// arc reaches mid-space and would report the space as material, which is why
/// the band starts at the base circle and the caller stays inside it.
///
/// Nothing here knows what an involute is. It reads points.
fn halfwidth_table(outline: &[[f64; 2]], teeth: u32, bins: usize) -> (f64, f64, Vec<f64>) {
    let pitch = std::f64::consts::TAU / f64::from(teeth);
    let (mut lo, mut hi) = (f64::MAX, 0.0_f64);
    for p in outline {
        let r = p[0].hypot(p[1]);
        lo = lo.min(r);
        hi = hi.max(r);
    }
    let mut table = vec![0.0_f64; bins];
    for p in outline {
        let r = p[0].hypot(p[1]);
        let th = p[1].atan2(p[0]);
        let d = (th - (th / pitch).round() * pitch).abs();
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let i = (((r - lo) / (hi - lo)) * (bins - 1) as f64).round() as usize;
        let i = i.min(bins - 1);
        table[i] = table[i].max(d);
    }
    // A bin no point landed in takes its neighbour's value, so the lookup is
    // defined across the band rather than reading zero where the sampling was
    // sparse.
    for i in 1..bins {
        if table[i] == 0.0 {
            table[i] = table[i - 1];
        }
    }
    (lo, hi, table)
}

/// Whether a point in the gear's own frame is inside its material, by the table
/// above. Points outside the band are reported outside.
fn inside(point: [f64; 2], teeth: u32, band: (f64, f64), table: &[f64], from: f64) -> bool {
    let r = point[0].hypot(point[1]);
    if r < from || r > band.1 {
        return false;
    }
    let pitch = std::f64::consts::TAU / f64::from(teeth);
    let th = point[1].atan2(point[0]);
    let d = (th - (th / pitch).round() * pitch).abs();
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = ((((r - band.0) / (band.1 - band.0)) * (table.len() - 1) as f64).round() as usize)
        .min(table.len() - 1);
    d < table[i]
}

/// The rotation that brings an external pair's flanks into contact at centre
/// distance `a`, radians at gear 2 — measured by **placing the two drawn
/// outlines** and closing them until they touch.
///
/// This exists to check [`crate::mesh::Mesh::loaded_flank_phase`] against
/// something that shares no code with it. That quantity is derived from the
/// backlash law, which comes from tooth thicknesses and `inv α`; this comes from
/// points on a curve and a containment test. If the two agree, the symmetry
/// argument behind the derivation — that a centre-distance change opens both
/// flanks equally, so loading one takes up half the play — is not an assumption
/// either of them makes.
///
/// Gear 1 sits at the origin with a tooth centred on the line of centres; gear 2
/// at `[a, 0]`, rotated until first contact. Returns `None` if the pair never
/// touches within the search, which for a sane pair means the inputs are wrong.
///
/// `sign` picks which flank is loaded: `+1` closes one way, `-1` the other. The
/// two answers straddle the zero-backlash position by half the play each, which
/// is the whole content of the claim and is why the caller asks for both.
#[must_use]
pub fn contact_phase_from_outlines(
    g1: &Tooth,
    g2: &Tooth,
    a: f64,
    sign: f64,
    per_tooth: usize,
) -> Option<f64> {
    // Through the assembly, because drawing a whole gear is the assembly's
    // job — a `Tooth` is one tooth's form and no longer pretends otherwise.
    let o1 = crate::gear::Gear::new(g1.params).profile(per_tooth);
    let o2 = crate::gear::Gear::new(g2.params).profile(per_tooth);
    // The table's radial resolution is the floor on what this can resolve — a
    // bin is a band of radius over which the half-width is taken as constant,
    // and the flank's slope turns that into an angular error. Tied to the point
    // count so asking for a finer measurement actually gets one.
    let (lo1, hi1, table) = halfwidth_table(&o1, g1.params.teeth, per_tooth.max(64) * 8);
    // Stay above the fillet, where the outline is flank and tip only.
    let from = g1.rb.max(lo1 + 0.05 * (hi1 - lo1));

    // Only gear 2's teeth that can reach gear 1 matter, and only their flanks.
    // Everything else is thousands of points that can never touch.
    // Whatever gear 2 does, a local point at radius `r` can come no closer to
    // gear 1's centre than `a - r`, so anything that cannot reach gear 1's tip
    // can never touch it whatever the rotation. That is most of the outline.
    let candidates: Vec<[f64; 2]> = o2
        .iter()
        .filter(|p| {
            let r = p[0].hypot(p[1]);
            r > g2.rb && a - r < hi1
        })
        .copied()
        .collect();
    if candidates.is_empty() {
        return None;
    }

    let touches = |phi: f64| {
        let (s, c) = phi.sin_cos();
        candidates.iter().any(|p| {
            let x = a + c * p[0] - s * p[1];
            let y = c * p[1] + s * p[0];
            inside([x, y], g1.params.teeth, (lo1, hi1), &table, from)
        })
    };

    // Where gear 2 can sit at all, found rather than assumed. Both gears are
    // drawn with a tooth centred on their own zero, and whether that leaves a
    // tooth or a space pointing at the mate depends on the tooth count's
    // **parity** — so the free placement is scanned for over one pitch instead
    // of being reasoned about. Over a pitch a meshing pair interferes almost
    // everywhere; the free band is the play, and it is what this is measuring.
    let pitch = std::f64::consts::TAU / f64::from(g2.params.teeth);
    const SCAN: usize = 400;
    #[allow(clippy::cast_precision_loss)]
    let free_seed = (0..=SCAN)
        .map(|i| pitch * (i as f64 / SCAN as f64) - pitch / 2.0)
        .find(|phi| !touches(*phi))?;

    // ...then closed from there until it interferes. **Half** a pitch, not a
    // whole one: a whole pitch is a symmetry of the gear and maps it onto
    // itself, so it would report the starting placement again and bracket
    // nothing. Half is far more than any play.
    let (mut free, mut hit) = (free_seed, free_seed + sign * pitch / 2.0);
    if !touches(hit) {
        return None;
    }
    for _ in 0..60 {
        let mid = 0.5 * (free + hit);
        if touches(mid) {
            hit = mid;
        } else {
            free = mid;
        }
    }
    Some(0.5 * (free + hit))
}
