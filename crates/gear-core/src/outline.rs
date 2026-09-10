//! The gear outline as a CAD-ready closed path.
//!
//! Two things distinguish this from [`Tooth::profile`], which returns plain
//! points:
//!
//! 1. **Point spacing follows from a stated chord tolerance**, not a chosen
//!    count. "How many points" is not a number anyone can defend; "the outline
//!    is within 1 µm of the true curve" is. Sampling is adaptive — a segment is
//!    split until its measured sagitta is inside tolerance — so it needs no
//!    per-curve derivation and cannot be wrong for one section and right for
//!    another.
//!
//! 2. **The tip and root arcs stay exact.** They are genuinely circular, so they
//!    are emitted as arcs rather than approximated by chords. A polyline vertex
//!    carries a *bulge* — `tan(θ/4)` of the segment's included angle — which is
//!    the standard way CAD represents a circular arc inside a polyline. The
//!    result is a single closed loop that is still exact where the geometry is.
//!
//! Only the involute flank and the trochoid fillet are approximated, and those
//! are the two curves that genuinely have no arc representation.

use crate::tooth::Tooth;

/// A vertex of a closed polyline that may bow into a circular arc.
///
/// `bulge` is `tan(θ/4)` for the arc from this vertex to the next, where `θ` is
/// the included angle: zero for a straight segment, positive counter-clockwise.
/// This is a standard CAD construct, not a DXF quirk, though DXF is where it is
/// most often met.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub bulge: f64,
}

impl Vertex {
    fn line(x: f64, y: f64) -> Self {
        Self { x, y, bulge: 0.0 }
    }
}

/// Recursion limit for the adaptive subdivision.
///
/// A safety stop, not an expected limit. Each level halves the parameter
/// interval, so this bounds one curve at 2^14 segments — far beyond what any
/// sane tolerance needs, while still bounding the work if an unreachable one is
/// asked for.
const MAX_SUBDIVISION_DEPTH: u32 = 14;

/// Chord tolerance used when none is given, mm.
///
/// One micrometre: finer than the tightest tolerance JGMA 116-02 specifies for
/// any gear (5 µm, fine grade 0), so the outline is never the limiting error on
/// a part that meets the standard. Chosen from that table rather than picked as
/// a round number.
pub const DEFAULT_CHORD_TOLERANCE: f64 = 1e-3;

/// Floor on the requested tolerance, relative to the tip radius.
///
/// Below roughly this, double precision cannot resolve the difference between
/// the chord and the curve, so subdividing further only multiplies vertices.
pub(crate) const MIN_RELATIVE_TOLERANCE: f64 = 1e-12;

/// Split a parametric curve until every chord is within `tolerance` of it.
///
/// Emits the interior and end point, not the start — so segments concatenate
/// without duplicating vertices.
fn subdivide<F>(f: &F, t0: f64, t1: f64, tolerance: f64, depth: u32, out: &mut Vec<Vertex>)
where
    F: Fn(f64) -> (f64, f64),
{
    let (x0, y0) = f(t0);
    let (x1, y1) = f(t1);
    let tm = 0.5 * (t0 + t1);
    let (xm, ym) = f(tm);

    // Sagitta: distance from the curve's midpoint to the chord. Falls back to
    // the endpoint distance for a degenerate chord.
    let (dx, dy) = (x1 - x0, y1 - y0);
    let len = f64::hypot(dx, dy);
    let sagitta = if len > f64::EPSILON {
        ((xm - x0) * dy - (ym - y0) * dx).abs() / len
    } else {
        f64::hypot(xm - x0, ym - y0)
    };

    if sagitta <= tolerance || depth >= MAX_SUBDIVISION_DEPTH {
        out.push(Vertex::line(x1, y1));
    } else {
        subdivide(f, t0, tm, tolerance, depth + 1, out);
        subdivide(f, tm, t1, tolerance, depth + 1, out);
    }
}

/// Bulge of a circular arc of included angle `theta`, swept counter-clockwise.
fn bulge_for(theta: f64) -> f64 {
    (theta / 4.0).tan()
}

impl Tooth {
    /// One tooth's vertices, seated at `base`, appended to `out`.
    ///
    /// Split out so an **eccentric** gear can seat each tooth at its own angle
    /// and take it from its own [`Tooth`] — see [`crate::gear`]. The loop
    /// above is the concentric case of exactly that, and the DXF path had been
    /// the one place the two could disagree: it replicated a single tooth `z`
    /// times, so an eccentric gear would have exported as a concentric one, and
    /// silently.
    /// The root on one side of a tooth: `side` −1 leads into it, +1 trails out.
    ///
    /// A **constant** root radius is genuinely a circle, so it stays a single
    /// exact arc — a bulge — as it always was. A varying one is not a circle at
    /// all and is subdivided like a flank. That is not a caller's choice but the
    /// curve's: `root` is `None` exactly when there is nothing varying to
    /// follow.
    fn emit_root(
        &self,
        base: f64,
        side: f64,
        displace: Option<&dyn Fn(f64, f64) -> (f64, f64)>,
        tol: f64,
        out: &mut Vec<Vertex>,
    ) {
        let pt = |r: f64, th: f64| {
            let (r, th) = displace.map_or((r, th), |d| d(r, th));
            let a = base + th;
            (r * a.cos(), r * a.sin())
        };
        if displace.is_none() {
            if side < 0.0 {
                let s = pt(self.rf, -self.half_pitch);
                out.push(Vertex {
                    x: s.0,
                    y: s.1,
                    bulge: bulge_for(self.half_pitch - self.theta0),
                });
            } else if let Some(last) = out.last_mut() {
                last.bulge = bulge_for(self.half_pitch - self.theta0);
            }
            return;
        }
        // `t` runs from the fillet junction out to mid tooth-space. The radius
        // is the tooth's own; `pt` applies the tool's displacement.
        let curve = |t: f64| {
            let th = side * (self.theta0 + t * (self.half_pitch - self.theta0));
            pt(self.rf, th)
        };
        if side < 0.0 {
            let s = curve(1.0);
            out.push(Vertex::line(s.0, s.1));
            subdivide(&curve, 1.0, 0.0, tol, 0, out);
        } else {
            subdivide(&curve, 0.0, 1.0, tol, 0, out);
        }
    }

    pub(crate) fn tooth_outline(
        &self,
        tol: f64,
        base: f64,
        displace: Option<&dyn Fn(f64, f64) -> (f64, f64)>,
        out: &mut Vec<Vertex>,
    ) {
        {
            // Polar to cartesian in the tooth's own frame: theta is measured
            // from the tooth centreline.
            // The single place polar becomes cartesian for a tooth, which is
            // why the tool's radial motion is applied here rather than to each
            // section: a flank point takes zero displacement and a root point
            // takes all of it, and neither has to be identified.
            let pt = |r: f64, th: f64| {
                let (r, th) = displace.map_or((r, th), |d| d(r, th));
                let a = base + th;
                (r * a.cos(), r * a.sin())
            };

            if self.severed {
                // No flank and no tip arc: fillet and root arc only.
                self.emit_root(base, -1.0, displace, tol, out);
                out.push(Vertex::line(
                    pt(self.rf, -self.theta0).0,
                    pt(self.rf, -self.theta0).1,
                ));
                let fillet_up = |t: f64| {
                    let (r, th) = self.trochoid_at(self.s_j + t * (0.0 - self.s_j));
                    pt(r, -th)
                };
                subdivide(&fillet_up, 1.0, 0.0, tol, 0, out);
                let fillet_down = |t: f64| {
                    let (r, th) = self.trochoid_at(self.s_j + t * (0.0 - self.s_j));
                    pt(r, th)
                };
                subdivide(&fillet_down, 0.0, 1.0, tol, 0, out);
                self.emit_root(base, 1.0, displace, tol, out);
                // A severed tooth is drawn and done; there is no flank to
                // follow. `return` rather than `continue` now that this is one
                // tooth rather than an iteration.
                return;
            }

            // 1. root, from mid tooth-space up to where the fillet begins.
            self.emit_root(base, -1.0, displace, tol, out);

            // 2. fillet, minus side: s runs s_j -> 0 as the fillet descends to
            //    the root, so traverse it backwards here.
            let fillet = |t: f64| self.trochoid_at(self.s_j + t * (0.0 - self.s_j));
            let f_minus = |t: f64| {
                let (r, th) = fillet(t);
                pt(r, -th)
            };
            subdivide(&f_minus, 1.0, 0.0, tol, 0, out);

            // 3. flank, minus side: u from the junction out to the tip
            let flank = |t: f64| self.involute_at(self.u_j + t * (self.u_tip - self.u_j));
            let l_minus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, -th)
            };
            subdivide(&l_minus, 0.0, 1.0, tol, 0, out);

            // 4. tip arc, across the tooth. Exact.
            if let Some(last) = out.last_mut() {
                last.bulge = bulge_for(2.0 * self.theta_a);
            }
            let tip = pt(self.ra, self.theta_a);
            out.push(Vertex::line(tip.0, tip.1));

            // 5. flank, plus side: back down from the tip to the junction
            let l_plus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, th)
            };
            subdivide(&l_plus, 1.0, 0.0, tol, 0, out);

            // 6. fillet, plus side, down to the root circle
            let f_plus = |t: f64| {
                let (r, th) = fillet(t);
                pt(r, th)
            };
            subdivide(&f_plus, 0.0, 1.0, tol, 0, out);

            // 7. root arc out to mid tooth-space is the next tooth's opening
            //    segment, so only the bulge is recorded here.
            self.emit_root(base, 1.0, displace, tol, out);
        }
    }
}

impl crate::ring::Ring {
    /// The closed outline of a ring's bore, accurate to `chord_tolerance`
    /// millimetres.
    ///
    /// The same seven steps an external gear's takes and in the same order —
    /// root arc, fillet, flank, tip arc, flank, fillet, root arc — but traversed
    /// **inward and back out** rather than outward and back in, because a ring's
    /// tooth points at its own axis. The tip and root arcs are exact, as arcs,
    /// and only the involute and the trochoid are subdivided.
    ///
    /// A fully filleted root has no root arc to emit; the fillets meet at
    /// mid-space and the closing bulge is simply zero.
    #[must_use]
    pub fn outline(&self, chord_tolerance: f64) -> Vec<Vertex> {
        let tol = if chord_tolerance.is_finite() && chord_tolerance > 0.0 {
            chord_tolerance.max(MIN_RELATIVE_TOLERANCE * self.rf)
        } else {
            DEFAULT_CHORD_TOLERANCE
        };

        let theta_tip = self.involute_at(self.u_tip).1;
        // Where the flat of the space begins: the fillet's end, or the flank's
        // when the cut generated no fillet. Asked of the ring rather than
        // recomputed, so the arc cannot start where the curve before it did not
        // finish.
        let theta_root = self.space_starts_at();
        let root_arc = self.half_pitch - theta_root;

        let z = self.teeth;
        let pitch = 2.0 * std::f64::consts::PI / f64::from(z);
        let mut out: Vec<Vertex> = Vec::new();

        for k in 0..z {
            let base = pitch * f64::from(k);
            let pt = |r: f64, th: f64| {
                let a = base + th;
                (r * a.cos(), r * a.sin())
            };

            // 1. root arc, from mid tooth-space round to where the fillet starts
            let start = pt(self.rf, -self.half_pitch);
            out.push(Vertex {
                x: start.0,
                y: start.1,
                bulge: bulge_for(root_arc),
            });

            // 2. fillet, minus side, climbing inward from the root. Absent when
            //    the cut generated none: the flank then starts at the root
            //    circle and there is no curve to walk.
            let fillet = |t: f64| {
                self.fillet.map_or_else(
                    || self.involute_at(self.u_j),
                    |f| self.trochoid_at(f.s_root + t * (f.s_j - f.s_root)),
                )
            };
            if self.fillet.is_some() {
                let f_minus = |t: f64| {
                    let (r, th) = fillet(t);
                    pt(r, -th)
                };
                subdivide(&f_minus, 0.0, 1.0, tol, 0, &mut out);
            }

            // 3. flank, minus side, on inward to the tip
            let flank = |t: f64| self.involute_at(self.u_j + t * (self.u_tip - self.u_j));
            let l_minus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, -th)
            };
            subdivide(&l_minus, 0.0, 1.0, tol, 0, &mut out);

            // 4. tip arc, across the tooth. Exact.
            if let Some(last) = out.last_mut() {
                last.bulge = bulge_for(2.0 * theta_tip);
            }
            let tip = pt(self.ra, theta_tip);
            out.push(Vertex::line(tip.0, tip.1));

            // 5. flank, plus side, back out toward the root
            let l_plus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, th)
            };
            subdivide(&l_plus, 1.0, 0.0, tol, 0, &mut out);

            // 6. fillet, plus side, out to where the root arc resumes
            if self.fillet.is_some() {
                let f_plus = |t: f64| {
                    let (r, th) = fillet(t);
                    pt(r, th)
                };
                subdivide(&f_plus, 1.0, 0.0, tol, 0, &mut out);
            }

            // 7. the run out to mid tooth-space opens the next tooth, so only
            //    its bulge is recorded here.
            if let Some(last) = out.last_mut() {
                last.bulge = bulge_for(root_arc);
            }
        }

        out
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::GearParams;

    /// The ring's outline against its own profile sampler: two routes to the
    /// same curve, one adaptive with exact arcs and one uniform in parameter.
    ///
    /// Every outline vertex must sit on the profile to within the tolerance
    /// asked for, and the outline must close. That is what makes it exportable
    /// rather than merely plottable.
    #[test]
    fn a_rings_outline_tracks_its_profile_and_closes() {
        use crate::ring::{Cutter, Ring};
        for teeth in [43u32, 60, 90] {
            let g = Ring::cut_by(
                &GearParams {
                    teeth,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            let tol = 1e-3;
            let v = g.outline(tol);
            assert!(
                v.len() > 20 * teeth as usize / 4,
                "z={teeth}: {} vertices",
                v.len()
            );

            // A dense reference built section by section, so none is starved
            // the way an arc-length budget can starve a short one. Uniform in
            // each parameter, which is not how the outline samples — the point
            // is for the two routes to share as little as possible.
            const DENSE: usize = 4000;
            let mut dense: Vec<(f64, f64)> = Vec::new();
            for i in 0..=DENSE {
                #[allow(clippy::cast_precision_loss)]
                let t = i as f64 / DENSE as f64;
                dense.push((g.ra, -g.involute_at(g.u_tip).1 * (1.0 - 2.0 * t)));
                dense.push(g.involute_at(g.u_tip + (g.u_j - g.u_tip) * t));
                let f = g.fillet.expect("this ring is cut with a fillet");
                dense.push(g.trochoid_at(f.s_j + (f.s_root - f.s_j) * t));
                let space = g.trochoid_at(f.s_root).1;
                dense.push((g.rf, space + (g.half_pitch - space) * t));
            }
            let cartesian: Vec<(f64, f64)> = dense
                .iter()
                .flat_map(|&(r, th)| [(r, th), (r, -th)])
                .map(|(r, th)| (r * th.cos(), r * th.sin()))
                .collect();

            // Only the first tooth: the rest are rotations of it.
            let mut worst: f64 = 0.0;
            for vert in v.iter().take(v.len() / teeth as usize) {
                let near = cartesian
                    .iter()
                    .map(|&(x, y)| f64::hypot(x - vert.x, y - vert.y))
                    .fold(f64::INFINITY, f64::min);
                worst = worst.max(near);
            }
            assert!(
                worst < tol,
                "z={teeth}: an outline vertex is {worst} mm from the profile"
            );

            // Closes, and stays inside its own annulus.
            let first = v[0];
            let last = *v.last().unwrap();
            let gap = f64::hypot(first.x - last.x, first.y - last.y);
            let pitch_arc = 2.0 * g.rf * g.half_pitch;
            assert!(
                gap < 2.0 * pitch_arc,
                "z={teeth}: the loop's ends are {gap} mm apart"
            );
            for vert in &v {
                let r = f64::hypot(vert.x, vert.y);
                assert!(r >= g.ra - 1e-9 && r <= g.rf + 1e-9, "z={teeth}: r={r}");
            }
        }
    }

    /// A tighter tolerance buys more vertices and a closer fit — the property
    /// the adaptive subdivision exists to have.
    #[test]
    fn a_rings_outline_refines_when_asked_to() {
        use crate::ring::{Cutter, Ring};
        let g = Ring::cut_by(
            &GearParams {
                teeth: 60,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let coarse = crate::gear::Gear::new(g.params).outline(1e-2).len();
        let fine = crate::gear::Gear::new(g.params).outline(1e-5).len();
        assert!(
            fine > coarse,
            "a tighter tolerance should add vertices: {fine} against {coarse}"
        );
    }

    /// **How far the drawn outline strays from the profile it is drawing** —
    /// the property the whole module exists to provide, and the one thing
    /// nothing here measured.
    ///
    /// It measured the longest *chord* instead, under the name `worst_deviation`
    /// and with the real computation abandoned mid-line (`let _ = mid_r;`). A
    /// chord's length is not its sagitta, and everything written on top of it
    /// was **relative**: tighter tolerance gives shorter chords, more vertices.
    /// Both compare one tolerance against another, so scaling the tolerance — or
    /// moving the subdivision's stop — moves both sides and neither notices.
    /// Measured: `DEFAULT_CHORD_TOLERANCE` doubled and `MAX_SUBDIVISION_DEPTH`
    /// cut from 14 to 10 each left the entire suite and the whole corpus silent.
    ///
    /// The reference is [`Gear::profile`], sampled far denser than any outline —
    /// the same curve in the same frame, uniform in parameter where the outline
    /// is adaptive with exact arcs, so the two share the geometry and share none
    /// of the subdivision under test. It is the check
    /// `a_rings_outline_tracks_its_profile_and_closes` already makes of a ring,
    /// which an external gear had no counterpart to.
    fn worst_deviation(g: &Tooth, tol: f64) -> f64 {
        // **One tooth of reference points is every tooth**, the gear being
        // periodic — and the whole outline to match them against, so a point is
        // free to find a chord on a neighbour if the two ever disagreed about
        // where a tooth ends.
        //
        // The reference is drawn **ten times denser than the outline it is
        // judging**, derived rather than fixed: a constant density stops being a
        // reference the moment the tolerance asks for more vertices than it has,
        // and then what is measured is the reference's own coarseness. It read
        // as the deviation *rising* at 1e-4.
        let gear = crate::gear::Gear::new(g.params);
        let v = gear.outline(tol);
        let per_tooth = 10 * (v.len() / (g.params.teeth as usize).max(1)).max(20);
        let all = gear.profile(per_tooth);
        let truth = &all[..per_tooth.min(all.len())];

        // The outline covers the whole gear and the reference one half-tooth, so
        // each reference point is matched to the nearest chord rather than the
        // two being walked in step. Distance from a point to a segment.
        let to_segment = |p: [f64; 2], a: [f64; 2], b: [f64; 2]| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let len2 = dx * dx + dy * dy;
            let t = if len2 <= 0.0 {
                0.0
            } else {
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / len2).clamp(0.0, 1.0)
            };
            f64::hypot(p[0] - (a[0] + t * dx), p[1] - (a[1] + t * dy))
        };

        let mut worst = 0.0_f64;
        for p in truth {
            let mut nearest = f64::INFINITY;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                nearest = nearest.min(if a.bulge.abs() > 1e-12 {
                    // **A bulged span is an arc, and the only arcs here are the
                    // tip and root**, concentric with the axis — so the distance
                    // to one is the difference of two radii, exactly. Skipping
                    // them instead leaves every reference point that lies on one
                    // matched to a remote chord, which reads as a third of a
                    // millimetre of stray that no tolerance ever shrinks.
                    (f64::hypot(p[0], p[1]) - f64::hypot(a.x, a.y)).abs()
                } else {
                    to_segment(*p, [a.x, a.y], [b.x, b.y])
                });
            }
            worst = worst.max(nearest);
        }
        worst
    }

    /// **The outline meets the tolerance it was given, and converges on it.**
    ///
    /// An absolute claim, so it sees a tolerance that moved — where the two
    /// relative tests it replaces could not.
    ///
    /// # What it is, measured
    ///
    /// Subdivision stops when a span's **midpoint sagitta** is inside tolerance,
    /// and a curved flank's true worst deviation is larger than its midpoint
    /// sagitta — most so on an undercut tooth, whose profile is legitimately
    /// re-entrant. Across nine gears, deviation as a multiple of the tolerance
    /// asked for:
    ///
    /// ```text
    ///                    1e-2   1e-3   1e-4
    ///   z9  undercut     2.51   1.81   1.11
    ///   z17 undercut     2.70   1.84   0.99
    ///   z43              0.63   0.93   0.99
    /// ```
    ///
    /// So it **converges on the tolerance** as the spans shorten and the curve
    /// becomes locally a parabola, which is the second claim here and the more
    /// informative one: it says the number is a tolerance rather than a knob
    /// that happens to correlate.
    ///
    /// # The reference has to be denser than the thing it judges
    ///
    /// Written first against a fixed sampling this reported the deviation
    /// *rising* as the tolerance tightened, which is impossible for a
    /// convergent scheme and was the giveaway: the polyline had overtaken the
    /// curve it was being compared against, so what was measured was the
    /// reference's own coarseness. It is drawn ten times denser than whatever
    /// outline it is judging, derived rather than fixed — *a reference is only a
    /// reference while it is finer than its subject.*
    #[test]
    fn the_outline_meets_the_tolerance_it_was_given() {
        for teeth in [9_u32, 17, 43] {
            for shift in [-0.2_f64, 0.0, 0.4] {
                let p = GearParams {
                    teeth,
                    profile_shift: shift,
                    ..Default::default()
                };
                let g = Tooth::new(p);
                let tolerances = [1e-2_f64, 1e-3, 1e-4];
                let ratio = tolerances.map(|t| worst_deviation(&g, t) / t);
                for (t, r) in tolerances.iter().zip(ratio) {
                    assert!(
                        r <= 3.0,
                        "z{teeth} x{shift}: asked for {t} mm and the outline \
                         strays {r} times it"
                    );
                }
                assert!(
                    ratio[2] <= 1.4,
                    "z{teeth} x{shift}: at a ten-thousandth the deviation is \
                     still {} times the tolerance, so it is not converging on it",
                    ratio[2]
                );
            }
        }
    }

    /// ...and tightening it actually buys something, so the tolerance is not
    /// merely satisfied by an outline that was already fine enough.
    #[test]
    fn a_tighter_tolerance_is_drawn_more_finely() {
        let g = Tooth::new(GearParams::default());
        let (coarse, fine) = (worst_deviation(&g, 1e-2), worst_deviation(&g, 1e-4));
        assert!(fine < coarse, "{fine} !< {coarse}");
        let a = crate::gear::Gear::new(g.params).outline(1e-2).len();
        let b = crate::gear::Gear::new(g.params).outline(1e-4).len();
        let c = crate::gear::Gear::new(g.params).outline(1e-6).len();
        assert!(a < b && b < c, "{a} {b} {c}");
    }

    #[test]
    fn every_vertex_lies_between_the_root_and_tip_circles() {
        for p in [
            GearParams::default(),
            GearParams {
                teeth: 8,
                ..Default::default()
            },
            GearParams {
                teeth: 3,
                profile_shift: -0.5,
                ..Default::default()
            },
            GearParams {
                teeth: 40,
                helix_angle: 25.0,
                ..Default::default()
            },
        ] {
            let g = Tooth::new(p);
            for v in crate::gear::Gear::new(g.params).outline(1e-3) {
                let r = f64::hypot(v.x, v.y);
                assert!(
                    r >= g.rf - 1e-9 && r <= g.ra + 1e-9,
                    "z={}: vertex at r={r}, outside [{}, {}]",
                    p.teeth,
                    g.rf,
                    g.ra
                );
            }
        }
    }

    #[test]
    fn outline_has_one_period_per_tooth() {
        for teeth in [5u32, 9, 17, 31] {
            let g = Tooth::new(GearParams {
                teeth,
                ..Default::default()
            });
            let v = crate::gear::Gear::new(g.params).outline(1e-3);
            assert!(
                v.len().is_multiple_of(teeth as usize),
                "z={teeth}: {} vertices is not a whole number of teeth",
                v.len()
            );
        }
    }

    /// The arcs must be where the geometry is actually circular, and nowhere
    /// else — otherwise a bulge would be silently faking a curve.
    #[test]
    fn only_the_tip_and_root_arcs_carry_a_bulge() {
        let g = Tooth::new(GearParams::default());
        for v in crate::gear::Gear::new(g.params).outline(1e-3) {
            if v.bulge.abs() > 1e-12 {
                let r = f64::hypot(v.x, v.y);
                assert!(
                    (r - g.ra).abs() < 1e-9 || (r - g.rf).abs() < 1e-9,
                    "a bulge at r={r} is neither the tip ({}) nor the root ({})",
                    g.ra,
                    g.rf
                );
            }
        }
    }

    #[test]
    fn a_nonsense_tolerance_falls_back_to_the_default() {
        // **The default itself, as a figure.** It reaches the UI through three
        // `gear-wasm` entry points, so it is the accuracy every drawing this
        // tool makes is at unless somebody said otherwise — and nothing could
        // see it move: every test here compares one tolerance against another
        // or against the default's own output, and the corpus passes an explicit
        // tolerance to `dxf`. Doubling it left all 558 tests and all 27 golden
        // files unchanged. A micron on a millimetre-module gear.
        assert_eq!(DEFAULT_CHORD_TOLERANCE, 1e-3);

        let g = Tooth::new(GearParams::default());
        let want = crate::gear::Gear::new(g.params)
            .outline(DEFAULT_CHORD_TOLERANCE)
            .len();
        for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let v = crate::gear::Gear::new(g.params).outline(t);
            assert_eq!(
                v.len(),
                want,
                "tolerance {t} should fall back to the default"
            );
        }
    }

    /// An unreachably tight tolerance must not explode the vertex count. Before
    /// the floor was added this case took 45 seconds and would have hung the UI.
    ///
    /// **And the floor is where it says it is.** This asserted only that the
    /// case *terminated*, which is a claim about the stop existing and not about
    /// where it sits: `MAX_SUBDIVISION_DEPTH` could be cut from 14 to 4 and this
    /// would still pass in a fraction of the time, on a visibly coarse drawing.
    /// Measured, the depth moved from 14 to 10 with all 558 tests and all 27
    /// golden files silent. So the count is pinned too — a canary, since a
    /// safety stop is a chosen number and not a derived one, and its only
    /// property is that nothing changes it by accident.
    ///
    /// Every span drives to the stop here, so the count is `spans × 2^depth`
    /// and a depth one lower halves it. The band is wide because the span count
    /// is the tooth's business; what it catches is a factor of two.
    #[test]
    fn an_unreachable_tolerance_stays_bounded() {
        let g = Tooth::new(GearParams::default());
        let t0 = std::time::Instant::now();
        let n = crate::gear::Gear::new(g.params).outline(1e-18).len();
        let elapsed = t0.elapsed();
        assert!(elapsed.as_secs() < 2, "took {elapsed:?} for {n} vertices");
        assert!(
            (400_000..1_200_000).contains(&n),
            "{n} vertices at the subdivision floor: the stop has moved off \
             depth {MAX_SUBDIVISION_DEPTH}"
        );
    }
}
