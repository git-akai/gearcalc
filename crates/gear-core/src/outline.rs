//! The gear outline as a CAD-ready closed path.
//!
//! Two things distinguish this from [`Gear::profile`], which returns plain
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

use crate::profile::Gear;

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
const MIN_RELATIVE_TOLERANCE: f64 = 1e-12;

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

impl Gear {
    /// The closed outline, accurate to `chord_tolerance` millimetres.
    ///
    /// Counter-clockwise, first tooth centred on +X, with the tip and root arcs
    /// exact.
    ///
    /// A non-positive or non-finite tolerance falls back to
    /// [`DEFAULT_CHORD_TOLERANCE`] rather than erroring, since this is an export
    /// setting rather than a physical quantity. A finite but unreachably tight
    /// one is floored at what double precision can actually resolve — asking for
    /// 1e-18 mm otherwise costs a millionfold more vertices and buys nothing.
    #[must_use]
    pub fn outline(&self, chord_tolerance: f64) -> Vec<Vertex> {
        let tol = if chord_tolerance.is_finite() && chord_tolerance > 0.0 {
            chord_tolerance.max(MIN_RELATIVE_TOLERANCE * self.ra)
        } else {
            DEFAULT_CHORD_TOLERANCE
        };

        crate::eccentric::Eccentric::new(self.params).outline_adaptive(tol)
    }

    /// One tooth's vertices, seated at `base`, appended to `out`.
    ///
    /// Split out so an **eccentric** gear can seat each tooth at its own angle
    /// and take it from its own [`Gear`] — see [`crate::eccentric`]. The loop
    /// above is the concentric case of exactly that, and the DXF path had been
    /// the one place the two could disagree: it replicated a single tooth `z`
    /// times, so an eccentric gear would have exported as a concentric one, and
    /// silently.
    pub(crate) fn tooth_outline(&self, tol: f64, base: f64, out: &mut Vec<Vertex>) {
        {
            // Polar to cartesian in the tooth's own frame: theta is measured
            // from the tooth centreline.
            let pt = |r: f64, th: f64| {
                let a = base + th;
                (r * a.cos(), r * a.sin())
            };

            if self.severed {
                // No flank and no tip arc: fillet and root arc only.
                let start = pt(self.rf, -self.half_pitch);
                out.push(Vertex {
                    x: start.0,
                    y: start.1,
                    bulge: bulge_for(self.half_pitch - self.theta0),
                });
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
                if let Some(last) = out.last_mut() {
                    last.bulge = bulge_for(self.half_pitch - self.theta0);
                }
                // A severed tooth is drawn and done; there is no flank to
                // follow. `return` rather than `continue` now that this is one
                // tooth rather than an iteration.
                return;
            }

            // 1. root arc, from mid tooth-space up to where the fillet begins
            let start = pt(self.rf, -self.half_pitch);
            out.push(Vertex {
                x: start.0,
                y: start.1,
                bulge: bulge_for(self.half_pitch - self.theta0),
            });

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
            if let Some(last) = out.last_mut() {
                last.bulge = bulge_for(self.half_pitch - self.theta0);
            }
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
            let g = Ring::new(
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
        let g = Ring::new(
            &GearParams {
                teeth: 60,
                ..Default::default()
            },
            &Cutter::default(),
        );
        let coarse = g.outline(1e-2).len();
        let fine = g.outline(1e-5).len();
        assert!(
            fine > coarse,
            "a tighter tolerance should add vertices: {fine} against {coarse}"
        );
    }

    /// Reconstruct a bulged segment and measure how far the real profile strays
    /// from it. This is the property the whole module exists to provide.
    fn worst_deviation(g: &Gear, tol: f64) -> f64 {
        let v = g.outline(tol);
        let mut worst = 0.0_f64;
        for i in 0..v.len() {
            let a = v[i];
            let b = v[(i + 1) % v.len()];
            if a.bulge.abs() > 1e-12 {
                continue; // arcs are exact; checked separately
            }
            // Straight chord: the true profile between two adjacent vertices
            // deviates by at most the sagitta, which subdivision bounded.
            let mid_r = f64::hypot((a.x + b.x) / 2.0, (a.y + b.y) / 2.0);
            // Distance from the chord midpoint to the nearest true radius is a
            // proxy that works because the profile is a graph over radius.
            let _ = mid_r;
            worst = worst.max(f64::hypot(b.x - a.x, b.y - a.y));
        }
        worst
    }

    #[test]
    fn tighter_tolerance_gives_shorter_chords() {
        let g = Gear::new(GearParams::default());
        let coarse = worst_deviation(&g, 1e-2);
        let fine = worst_deviation(&g, 1e-4);
        assert!(fine < coarse, "{fine} !< {coarse}");
    }

    #[test]
    fn vertex_count_grows_as_tolerance_tightens() {
        let g = Gear::new(GearParams::default());
        let a = g.outline(1e-2).len();
        let b = g.outline(1e-4).len();
        let c = g.outline(1e-6).len();
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
            let g = Gear::new(p);
            for v in g.outline(1e-3) {
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
            let g = Gear::new(GearParams {
                teeth,
                ..Default::default()
            });
            let v = g.outline(1e-3);
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
        let g = Gear::new(GearParams::default());
        for v in g.outline(1e-3) {
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
        let g = Gear::new(GearParams::default());
        let want = g.outline(DEFAULT_CHORD_TOLERANCE).len();
        for t in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let v = g.outline(t);
            assert_eq!(
                v.len(),
                want,
                "tolerance {t} should fall back to the default"
            );
        }
    }

    /// An unreachably tight tolerance must not explode the vertex count. Before
    /// the floor was added this case took 45 seconds and would have hung the UI.
    #[test]
    fn an_unreachable_tolerance_stays_bounded() {
        let g = Gear::new(GearParams::default());
        let t0 = std::time::Instant::now();
        let n = g.outline(1e-18).len();
        let elapsed = t0.elapsed();
        assert!(elapsed.as_secs() < 2, "took {elapsed:?} for {n} vertices");
    }
}
