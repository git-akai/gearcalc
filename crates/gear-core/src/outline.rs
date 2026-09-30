//! The gear outline as a CAD-ready closed path.
//!
//! Two things distinguish this from [`Gear::profile`](crate::gear::Gear::profile), which returns plain
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

/// A closed parametric curve, `t ∈ [0, 1]` with `f(1) = f(0)`, as a polyline
/// within `tolerance` of it. Split into quarters first, so the whole loop's
/// chord — of zero length — is never the one measured.
pub(crate) fn closed_curve<F>(f: &F, tolerance: f64) -> Vec<Vertex>
where
    F: Fn(f64) -> (f64, f64),
{
    let mut out = Vec::new();
    for q in 0..4 {
        let (t0, t1) = (f64::from(q) / 4.0, f64::from(q + 1) / 4.0);
        subdivide(f, t0, t1, tolerance, 0, &mut out);
    }
    // Each piece emits its end and never its start, so the loop's start is
    // its last vertex, once.
    out
}

/// A reference curve drawn beside an outline: a circle where the curve is one,
/// and a closed polyline where it is not.
///
/// It is what a drawing needs of a gear's tip and root without knowing what a
/// gear is: an eccentric gear's tip is no circle, and a writer handed four
/// radii can only draw a concentric one.
#[derive(Clone, Debug, PartialEq)]
pub enum Envelope {
    Circle { centre: [f64; 2], radius: f64 },
    Closed(Vec<Vertex>),
}

impl Envelope {
    /// A circle about the axis.
    #[must_use]
    pub fn circle(radius: f64) -> Self {
        Self::Circle {
            centre: [0.0, 0.0],
            radius,
        }
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
        // A space the tool closed has no root arc: the fillets meet on its
        // centreline, and an empty section adds no vertex.
        if self.tool.closes {
            return;
        }
        if displace.is_none() {
            if side < 0.0 {
                // One bulged vertex and the arc's own end: the fillet's walk
                // never emits its start, so without the end the bulge would
                // land on the chord to the first fillet sample.
                let s = pt(self.rf, -self.half_pitch);
                out.push(Vertex {
                    x: s.0,
                    y: s.1,
                    bulge: bulge_for(self.half_pitch - self.theta0),
                });
                let e = pt(self.rf, -self.theta0);
                out.push(Vertex::line(e.0, e.1));
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
        // Mid-space is the previous tooth's last vertex, so the walk in starts
        // after it.
        if side < 0.0 {
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

            // 3. flank, minus side: u from the junction out to the tip. A tooth
            //    that ends at its tip on the fillet has no flank, and an empty
            //    section adds no vertex: the involute's point at the tip is not
            //    the fillet's.
            let flank = |t: f64| self.involute_at(self.u_j + t * (self.u_tip - self.u_j));
            let has_flank = self.u_j < self.u_tip;
            let l_minus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, -th)
            };
            if has_flank {
                subdivide(&l_minus, 0.0, 1.0, tol, 0, out);
            }

            // 4. tip arc, across the tooth. Exact. A pointed tip has none: the
            //    flanks meet on the centreline, where the walk already is.
            if self.theta_a > 0.0 {
                if let Some(last) = out.last_mut() {
                    last.bulge = bulge_for(2.0 * self.theta_a);
                }
                let tip = pt(self.ra, self.theta_a);
                out.push(Vertex::line(tip.0, tip.1));
            }

            // 5. flank, plus side: back down from the tip to the junction
            let l_plus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, th)
            };
            if has_flank {
                subdivide(&l_plus, 1.0, 0.0, tol, 0, out);
            }

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
    ///
    /// # Errors
    ///
    /// [`crate::input::Refused::past_memory`], naming `teeth`, where the
    /// outline is more vertices than the machine's memory holds.
    pub fn outline(&self, chord_tolerance: f64) -> Result<Vec<Vertex>, crate::input::Refused> {
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
        // Each tooth's vertices are made apart and then given room: the
        // first sizes the rest, which a ring's all match.
        let mut one: Vec<Vertex> = Vec::new();

        for k in 0..z {
            one.clear();
            let base = pitch * f64::from(k);
            let pt = |r: f64, th: f64| {
                let a = base + th;
                (r * a.cos(), r * a.sin())
            };

            // 1. root arc, from mid tooth-space round to where the fillet
            //    starts: one bulged vertex and the arc's own end. A fully
            //    filleted root has no arc, and an empty section adds no vertex.
            let start = pt(self.rf, -self.half_pitch);
            one.push(Vertex {
                x: start.0,
                y: start.1,
                bulge: bulge_for(root_arc),
            });
            if root_arc > 0.0 {
                let end = pt(self.rf, -theta_root);
                one.push(Vertex::line(end.0, end.1));
            }

            // 2. fillet, minus side, climbing inward from the root. Absent when
            //    the cut generated none: the flank then starts at the root
            //    circle and there is no curve to walk.
            let fillet = |t: f64| {
                self.fillet.map_or_else(
                    || self.involute_at(self.u_j),
                    |f| self.trochoid_at(f.phi_root + t * (f.phi_j - f.phi_root)),
                )
            };
            if self.fillet.is_some() {
                let f_minus = |t: f64| {
                    let (r, th) = fillet(t);
                    pt(r, -th)
                };
                subdivide(&f_minus, 0.0, 1.0, tol, 0, &mut one);
            }

            // 3. flank, minus side, on inward to the tip
            let flank = |t: f64| self.involute_at(self.u_j + t * (self.u_tip - self.u_j));
            let l_minus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, -th)
            };
            subdivide(&l_minus, 0.0, 1.0, tol, 0, &mut one);

            // 4. tip arc, across the tooth. Exact.
            if let Some(last) = one.last_mut() {
                last.bulge = bulge_for(2.0 * theta_tip);
            }
            let tip = pt(self.ra, theta_tip);
            one.push(Vertex::line(tip.0, tip.1));

            // 5. flank, plus side, back out toward the root
            let l_plus = |t: f64| {
                let (r, th) = flank(t);
                pt(r, th)
            };
            subdivide(&l_plus, 1.0, 0.0, tol, 0, &mut one);

            // 6. fillet, plus side, out to where the root arc resumes
            if self.fillet.is_some() {
                let f_plus = |t: f64| {
                    let (r, th) = fillet(t);
                    pt(r, th)
                };
                subdivide(&f_plus, 1.0, 0.0, tol, 0, &mut one);
            }

            // 7. the run out to mid tooth-space opens the next tooth, so only
            //    its bulge is recorded here.
            if let Some(last) = one.last_mut() {
                last.bulge = bulge_for(root_arc);
            }
            let wanted = if k == 0 {
                one.len().saturating_mul(z as usize)
            } else {
                one.len()
            };
            // Room already there is a no-op; room short grows as `Vec` grows.
            out.try_reserve(wanted)
                .map_err(|_| crate::input::Refused::past_memory("teeth", f64::from(z)))?;
            out.extend_from_slice(&one);
        }

        Ok(out)
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
    /// The profile must lie within the tolerance asked for of the outline,
    /// spans and arcs included, and the outline inside the ring's annulus.
    /// That is what makes it exportable rather than merely plottable.
    #[test]
    fn a_rings_outline_tracks_its_profile() {
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
            let v = g.outline(tol).unwrap();
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
                let t = i as f64 / DENSE as f64;
                dense.push((g.ra, -g.involute_at(g.u_tip).1 * (1.0 - 2.0 * t)));
                dense.push(g.involute_at(g.u_tip + (g.u_j - g.u_tip) * t));
                let f = g.fillet.expect("this ring is cut with a fillet");
                dense.push(g.trochoid_at(f.phi_j + (f.phi_root - f.phi_j) * t));
                let space = g.trochoid_at(f.phi_root).1;
                dense.push((g.rf, space + (g.half_pitch - space) * t));
            }
            let cartesian: Vec<(f64, f64)> = dense
                .iter()
                .flat_map(|&(r, th)| [(r, th), (r, -th)])
                .map(|(r, th)| (r * th.cos(), r * th.sin()))
                .collect();

            // Every reference point within the tolerance of the outline, each
            // arc read as the arc its chord and bulge define — the law the
            // external gear is held to. Measuring the outline's vertices
            // against the profile instead cannot see a span between them.
            let cartesian: Vec<[f64; 2]> = cartesian.iter().map(|&(x, y)| [x, y]).collect();
            let worst = deviation(&v, &cartesian);
            assert!(
                worst <= tol,
                "z={teeth}: the outline strays {worst} mm from the profile"
            );

            // ...and stays inside its own annulus.
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
        let coarse = crate::gear::Gear::new(g.params)
            .outline(1e-2)
            .unwrap()
            .len();
        let fine = crate::gear::Gear::new(g.params)
            .outline(1e-5)
            .unwrap()
            .len();
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
    /// of the subdivision under test. A ring is held to the same law in
    /// `a_rings_outline_tracks_its_profile`.
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
        let v = gear.outline(tol).unwrap();
        let per_tooth = 10 * (v.len() / (g.params.teeth as usize).max(1)).max(20);
        let all = gear.profile(per_tooth).unwrap();
        let truth = &all[..per_tooth.min(all.len())];

        deviation(&v, truth)
    }

    /// How far the furthest reference point lies from a closed outline, each
    /// point measured to its nearest span. The outline covers the whole gear
    /// and the reference may cover one tooth, so each point is matched to the
    /// nearest span rather than the two being walked in step.
    ///
    /// **A bulged span is the arc its chord and bulge define**, as CAD reads
    /// it: the distance is to that circle within the arc's angular extent, and
    /// to the nearer end outside it. Reading every arc as concentric with the
    /// axis, as this once did, cannot see an arc bulged onto the wrong chord —
    /// it passed one centred 22 mm off the axis.
    fn deviation(v: &[Vertex], truth: &[[f64; 2]]) -> f64 {
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
        let to_arc = |p: [f64; 2], a: [f64; 2], b: [f64; 2], bulge: f64| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let chord = dx.hypot(dy);
            // Centre to the left of travel by `c/2 · (1 − b²)/(2b)`; the arc
            // sweeps `4 atan b` from `a`, counter-clockwise when positive.
            let h = chord / 2.0 * (1.0 - bulge * bulge) / (2.0 * bulge);
            let c = [
                (a[0] + b[0]) / 2.0 - dy / chord * h,
                (a[1] + b[1]) / 2.0 + dx / chord * h,
            ];
            let radius = (a[0] - c[0]).hypot(a[1] - c[1]);
            let sweep = 4.0 * bulge.atan();
            let angle = |q: [f64; 2]| (q[1] - c[1]).atan2(q[0] - c[0]);
            // Where `p` sits along the sweep, from 0 at `a`.
            let along = (angle(p) - angle(a) + std::f64::consts::PI)
                .rem_euclid(std::f64::consts::TAU)
                - std::f64::consts::PI;
            if (along / sweep) >= 0.0 && (along / sweep) <= 1.0 {
                ((p[0] - c[0]).hypot(p[1] - c[1]) - radius).abs()
            } else {
                (p[0] - a[0])
                    .hypot(p[1] - a[1])
                    .min((p[0] - b[0]).hypot(p[1] - b[1]))
            }
        };

        let mut worst = 0.0_f64;
        for &p in truth {
            let mut nearest = f64::INFINITY;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let (pa, pb) = ([a.x, a.y], [b.x, b.y]);
                nearest = nearest.min(if a.bulge == 0.0 {
                    to_segment(p, pa, pb)
                } else {
                    to_arc(p, pa, pb, a.bulge)
                });
            }
            worst = worst.max(nearest);
        }
        worst
    }

    /// **The outline meets the tolerance it was given.**
    ///
    /// An absolute claim, so it sees a tolerance that moved — where the two
    /// relative tests it replaced could not — and the bound is the tolerance
    /// itself rather than a multiple chosen above what was measured.
    ///
    /// # What it was, and why
    ///
    /// It used to allow three times the tolerance, and 1.4 at a
    /// ten-thousandth, having measured this across nine gears:
    ///
    /// ```text
    ///                    1e-2   1e-3   1e-4
    ///   z9  undercut     2.51   1.81   1.11
    ///   z17 undercut     2.70   1.84   0.99
    ///   z43              0.63   0.93   0.99
    /// ```
    ///
    /// and put it down to the midpoint sagitta under-reading a re-entrant
    /// undercut flank. It was the root arc bulged onto the wrong chord
    /// (T04.1): the fillet's first span was drawn as part of a circle through
    /// its far end, and the measure — which read every arc as concentric with
    /// the axis — reported that as a deviation that shrank with the tolerance
    /// only because the fillet's first span did. With every arc measured as the
    /// arc it is, and every arc ending where it should, no gear here strays
    /// past the tolerance.
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
                for t in [1e-2_f64, 1e-3, 1e-4] {
                    let r = worst_deviation(&g, t) / t;
                    assert!(
                        r <= 1.0,
                        "z{teeth} x{shift}: asked for {t} mm and the outline \
                         strays {r} times it"
                    );
                }
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
        let a = crate::gear::Gear::new(g.params)
            .outline(1e-2)
            .unwrap()
            .len();
        let b = crate::gear::Gear::new(g.params)
            .outline(1e-4)
            .unwrap()
            .len();
        let c = crate::gear::Gear::new(g.params)
            .outline(1e-6)
            .unwrap()
            .len();
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
            for v in crate::gear::Gear::new(g.params).outline(1e-3).unwrap() {
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
            let v = crate::gear::Gear::new(g.params).outline(1e-3).unwrap();
            assert!(
                v.len().is_multiple_of(teeth as usize),
                "z={teeth}: {} vertices is not a whole number of teeth",
                v.len()
            );
        }
    }

    /// Outlines of every kind the walkers draw, with the tip and root radii
    /// each may carry an arc at: external, helical, undercut, severed, pointed,
    /// ended at its tip on the fillet, eccentric, and rings.
    fn every_kind_of_outline() -> Vec<(String, Vec<Vertex>, Vec<f64>)> {
        use crate::ring::{Cutter, Ring};
        let external = |p: GearParams| {
            let gear = crate::gear::Gear::new(p);
            let radii = gear.distinct().flat_map(|t| [t.ra, t.rf]).collect();
            (format!("{p:?}"), gear.outline(1e-3).unwrap(), radii)
        };
        let with = |f: fn(&mut GearParams)| {
            let mut p = GearParams::default();
            f(&mut p);
            p
        };
        let mut cases = vec![
            external(GearParams::default()),
            external(with(|p| p.helix_angle = 25.0)),
            external(with(|p| {
                p.teeth = 9;
                p.profile_shift = -0.3;
            })),
            // severed
            external(with(|p| {
                p.teeth = 5;
                p.profile_shift = -0.5;
                p.pressure_angle = 14.5;
            })),
            // pointed
            external(with(|p| {
                p.teeth = 9;
                p.profile_shift = 0.9;
            })),
            // ended at its tip on the fillet
            external(with(|p| p.profile_shift = -1.5)),
            // eccentric
            external(with(|p| {
                p.teeth = 24;
                p.angular_shift = 0.25;
            })),
            external(with(|p| {
                p.teeth = 9;
                p.profile_shift = -0.3;
                p.angular_shift = 1.0;
            })),
            // the space closed by the rack's tooth
            external(closed_space()),
        ];
        for teeth in [43u32, 90] {
            let ring = Ring::cut_by(
                &GearParams {
                    teeth,
                    ..Default::default()
                },
                &Cutter::default(),
            );
            cases.push((
                format!("ring z{teeth}"),
                ring.outline(1e-3).unwrap(),
                vec![ring.ra, ring.rf],
            ));
        }
        cases
    }

    /// A gear whose rack's tooth comes to a point before the depth asked:
    /// z20 at 30° with a sharp corner closes at `h_f = π/(4 tan 30°)` = 1.360.
    fn closed_space() -> GearParams {
        GearParams {
            teeth: 20,
            pressure_angle: 30.0,
            dedendum: 1.6,
            root_radius: 0.0,
            ..GearParams::default()
        }
    }

    /// **A space the rack's tooth closed has no root arc.** Its fillets meet on
    /// the space's centreline at the root, so the outline goes from one to the
    /// next with no arc at the root radius between them. Held at 95 % of the
    /// round that fits, the tool left a root arc 2.6e-11 mm long in every space
    /// — a span CAD refuses — and exactly at the fit, one a rounding long and
    /// of either sign.
    #[test]
    fn a_closed_space_has_no_root_arc() {
        for p in [
            closed_space(),
            GearParams {
                helix_angle: 25.0,
                ..closed_space()
            },
            GearParams {
                angular_shift: 0.1,
                ..closed_space()
            },
        ] {
            let gear = crate::Gear::new(p);
            assert!(gear.mean().tool.closes, "{p:?}: the space did not close");
            let v = gear.outline(1e-3).unwrap();
            let root = gear.distinct().map(|t| t.rf).fold(f64::MAX, f64::min);
            for (i, a) in v.iter().enumerate() {
                assert!(
                    a.bulge == 0.0 || (a.x.hypot(a.y) - root).abs() > 1e-9 * root,
                    "{p:?}: span {i} is a root arc"
                );
            }
            for t in gear.distinct() {
                assert!(t.theta0 == t.half_pitch, "{p:?}: θ0 {} ≠ π/z", t.theta0);
            }
        }
    }

    /// **Every arc is a tip or root arc about the axis.** Each bulged span is
    /// read back from its chord and bulge alone, as CAD reads it: its ends and
    /// its midpoint lie on one circle about the axis, that circle is a tip or a
    /// root, and the bulge is `tan(Δθ/4)` of the angle its ends subtend there.
    ///
    /// Three points rather than a centre because a span a fraction of a micron
    /// long has a centre its rounding cannot place, and a midpoint it can. The
    /// check this replaces read the start vertex only, and passed an arc bulged
    /// onto the chord to the first fillet sample: centred 22.4 mm off the axis
    /// on z17, 47.6 mm on a z43 ring.
    #[test]
    fn every_arc_is_a_tip_or_root_arc_about_the_axis() {
        for (name, v, radii) in every_kind_of_outline() {
            let size = radii.iter().copied().fold(0.0, f64::max);
            let tol = 1e-9 * size;
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                if a.bulge == 0.0 {
                    continue;
                }
                let (ra, rb) = (a.x.hypot(a.y), b.x.hypot(b.y));
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let sagitta = a.bulge * dx.hypot(dy) / 2.0;
                let len = dx.hypot(dy);
                // The arc's midpoint: off the chord's middle, to the right of
                // travel for a counter-clockwise (positive) bulge.
                let mid = [
                    (a.x + b.x) / 2.0 + dy / len * sagitta,
                    (a.y + b.y) / 2.0 - dx / len * sagitta,
                ];
                let rm = mid[0].hypot(mid[1]);
                assert!(
                    (ra - rb).abs() < tol && (ra - rm).abs() < tol,
                    "{name}: span {i} is not one circle about the axis: ends at {ra}, {rb}, middle at {rm}"
                );
                assert!(
                    radii.iter().any(|r| (r - ra).abs() < tol),
                    "{name}: span {i} is an arc at r={ra}, neither a tip nor a root"
                );
                let subtends = (a.x * b.y - a.y * b.x).atan2(a.x * b.x + a.y * b.y);
                assert!(
                    (a.bulge - (subtends / 4.0).tan()).abs() < 1e-9,
                    "{name}: span {i} bulges {} for {subtends} rad",
                    a.bulge
                );
            }
        }
    }

    /// **No vertex is repeated.** Every section ends on its own vertex and
    /// starts on the one before, so two consecutive vertices at one point mean
    /// a section was emitted twice or an empty one was emitted at all: a
    /// zero-length segment in the export.
    #[test]
    fn no_two_consecutive_vertices_coincide() {
        for (name, v, radii) in every_kind_of_outline() {
            let size = radii.iter().copied().fold(0.0, f64::max);
            for i in 0..v.len() {
                let (a, b) = (v[i], v[(i + 1) % v.len()]);
                let gap = (b.x - a.x).hypot(b.y - a.y);
                assert!(
                    gap > 1e-12 * size,
                    "{name}: vertices {i} and {} are {gap:e} mm apart",
                    (i + 1) % v.len()
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
            .unwrap()
            .len();
        for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let v = crate::gear::Gear::new(g.params).outline(t).unwrap();
            assert_eq!(
                v.len(),
                want,
                "tolerance {t} should fall back to the default"
            );
        }
    }

    /// An unreachably tight tolerance must not explode the vertex count: every
    /// span subdivides to the floor and stops there.
    ///
    /// **And the floor is where it says it is.** The count here is
    /// `spans × 2^depth` (measured: 278 579, 557 107, 1 113 925, 2 223 583 at
    /// depths 12 to 15), so it is held to depth 14 within a factor of `√2`
    /// either side, half-way to the neighbouring depths: a stop moved one
    /// level either way fails it. The depth is written here, not read from
    /// `MAX_SUBDIVISION_DEPTH` — a safety stop is a chosen number, and this is
    /// what notices it being changed by accident. `SPANS` is the default gear's
    /// span count, the tooth's business. A count, not a clock, so the test
    /// fails on the depth and not on a busy machine.
    #[test]
    fn an_unreachable_tolerance_stays_bounded() {
        const SPANS: f64 = 68.0;
        const DEPTH: u32 = 14;
        let g = Tooth::new(GearParams::default());
        let n = crate::gear::Gear::new(g.params)
            .outline(1e-18)
            .unwrap()
            .len();
        let expected = SPANS * f64::from(1u32 << DEPTH);
        let ratio = n as f64 / expected;
        assert!(
            (std::f64::consts::FRAC_1_SQRT_2..std::f64::consts::SQRT_2).contains(&ratio),
            "{n} vertices at the subdivision floor, {ratio:.3} of {SPANS} spans x 2^{DEPTH}: \
             the stop has moved off depth {DEPTH} (MAX_SUBDIVISION_DEPTH is {MAX_SUBDIVISION_DEPTH})"
        );
    }
}
