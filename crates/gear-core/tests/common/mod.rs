//! One parameter grid, shared by every integration test that needs one.
//!
//! # Why this exists
//!
//! There were three of these, one per test file, each a nest of `for` loops over
//! its own hand-chosen lists. They were not the same lists, and between them
//! they turned five of a gear's eleven inputs and left six — `module`,
//! `addendum`, `dedendum`, `thickness_mod`, `angular_shift`, `index_offset` — at
//! their defaults in every profile law the crate asserts. The module was `1.0`
//! everywhere.
//!
//! That is the standing trap of `docs/corrections.md`, met in the oldest tests
//! rather than the newest: **a test that never leaves a control at its default
//! never tests the control**, and *turning it in one context does not cover the
//! others*. Three copies is also three places a coverage gap can hide, and the
//! gap is invisible from inside any one of them.
//!
//! # How it is used
//!
//! An axis left alone is a single value — the default — so a test declares the
//! axes it needs and pays for nothing else:
//!
//! ```ignore
//! for p in Grid::new().teeth(&[9, 17, 40]).shifts(&[-0.3, 0.0, 0.5]).build() { … }
//! ```
//!
//! Cost is the product of the axes turned, so the caller can see it at the call
//! site. That is deliberate: the reason the old grids never grew a `module` axis
//! is that adding one meant editing a five-deep loop nest in three files and
//! multiplying a runtime nobody could see from the test.

#![allow(dead_code)] // each test file uses a different part of this

use gear_core::GearParams;

/// A cross product of parameter axes. Any axis not set holds
/// [`GearParams::default`]'s value, so the empty grid is one ordinary gear.
#[derive(Clone, Debug)]
pub struct Grid {
    module: Vec<f64>,
    teeth: Vec<u32>,
    profile_shift: Vec<f64>,
    pressure_angle: Vec<f64>,
    helix_angle: Vec<f64>,
    addendum: Vec<f64>,
    dedendum: Vec<f64>,
    root_radius: Vec<f64>,
    thickness_mod: Vec<f64>,
    angular_shift: Vec<f64>,
    index_offset: Vec<f64>,
}

impl Default for Grid {
    fn default() -> Self {
        let d = GearParams::default();
        Self {
            module: vec![d.module],
            teeth: vec![d.teeth],
            profile_shift: vec![d.profile_shift],
            pressure_angle: vec![d.pressure_angle],
            helix_angle: vec![d.helix_angle],
            addendum: vec![d.addendum],
            dedendum: vec![d.dedendum],
            root_radius: vec![d.root_radius],
            thickness_mod: vec![d.thickness_mod],
            angular_shift: vec![d.angular_shift],
            index_offset: vec![d.index_offset],
        }
    }
}

macro_rules! axis {
    ($name:ident, $ty:ty) => {
        #[must_use]
        pub fn $name(mut self, v: &[$ty]) -> Self {
            assert!(!v.is_empty(), concat!(stringify!($name), " axis is empty"));
            self.$name = v.to_vec();
            self
        }
    };
}

impl Grid {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    axis!(module, f64);
    axis!(teeth, u32);
    axis!(pressure_angle, f64);
    axis!(helix_angle, f64);
    axis!(addendum, f64);
    axis!(dedendum, f64);
    axis!(root_radius, f64);
    axis!(thickness_mod, f64);
    axis!(angular_shift, f64);
    axis!(index_offset, f64);

    /// Named `shifts` rather than `profile_shift` because every call site reads
    /// better for it, and because `profile_shift` is also what a *thickness*
    /// modification becomes — see [`Grid::thickness_mod`].
    #[must_use]
    pub fn shifts(mut self, v: &[f64]) -> Self {
        assert!(!v.is_empty(), "shift axis is empty");
        self.profile_shift = v.to_vec();
        self
    }

    /// Every combination, in a stable order so a failure names a reproducible
    /// case.
    #[must_use]
    pub fn build(&self) -> Vec<GearParams> {
        let mut out = Vec::new();
        for &module in &self.module {
            for &teeth in &self.teeth {
                for &profile_shift in &self.profile_shift {
                    for &pressure_angle in &self.pressure_angle {
                        for &helix_angle in &self.helix_angle {
                            for &addendum in &self.addendum {
                                for &dedendum in &self.dedendum {
                                    for &root_radius in &self.root_radius {
                                        for &thickness_mod in &self.thickness_mod {
                                            for &angular_shift in &self.angular_shift {
                                                for &index_offset in &self.index_offset {
                                                    out.push(GearParams {
                                                        module,
                                                        teeth,
                                                        profile_shift,
                                                        pressure_angle,
                                                        helix_angle,
                                                        addendum,
                                                        dedendum,
                                                        root_radius,
                                                        thickness_mod,
                                                        angular_shift,
                                                        index_offset,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        out
    }
}

// ------------------------------------------------------------- the axes ---
//
// Shared axis values, so "the awkward tooth counts" means the same thing in
// every file that asks for them. Named for what makes them awkward rather than
// listed at each call site, which is how three files came to sweep three
// different ideas of "a range of tooth counts".

/// Small enough to undercut and sever, large enough to be rack-like.
pub const AWKWARD_TEETH: &[u32] = &[3, 5, 7, 9, 12, 17, 23, 40, 80];

/// Both signs, past the undercut threshold at one end and toward pointed at the
/// other.
pub const AWKWARD_SHIFTS: &[f64] = &[-0.5, -0.3, 0.0, 0.3, 0.6, 0.9];

/// The three pressure angles in common use, spanning the range over which the
/// admissible shift interval swings by more than a factor of two.
pub const PRESSURE_ANGLES: &[f64] = &[14.5, 20.0, 25.0];

/// Both hands and zero, because a spur gear is the helical case's value rather
/// than a branch beside it.
pub const HELIX_ANGLES: &[f64] = &[0.0, 15.0, -30.0];

/// A sharp rack, a small round, and the ISO 53 basic rack.
pub const ROOT_RADII: &[f64] = &[0.0, 0.2, 0.38];

/// Sub-millimetre to coarse. Every length in the crate is homogeneous of degree
/// one in this and every angle is invariant, which is a law worth turning the
/// axis for — see `geometry_laws::every_length_scales_with_the_module`.
pub const MODULES: &[f64] = &[0.5, 1.0, 3.7, 12.0];

/// Either side of nominal. `k = 1` is the unmodified rack.
pub const THICKNESS_MODS: &[f64] = &[0.7, 1.0, 1.3];

/// Dedenda, in modules: shallow, the ISO 53 rack, and two deep enough that a
/// rack's tooth comes to a point before it reaches them at ordinary pressure
/// angles (h_f > π/(4 tan α_n) − x_s, 2.16 at 20°).
pub const DEDENDA: &[f64] = &[0.3, 1.25, 2.0, 3.0];

// ---------------------------------------------------------------------------
// Reading a drawn outline back as points, and asking whether it is simple.
// ---------------------------------------------------------------------------

/// The circle a bulged span `a → b` lies on: `(centre, radius, included angle)`.
///
/// From the chord and the bulge alone — the DXF definition, `bulge = tan(θ/4)`,
/// positive counter-clockwise — so it shares nothing with how the outline was
/// drawn. `None` for a straight span.
pub fn arc_of(a: [f64; 2], b: [f64; 2], bulge: f64) -> Option<([f64; 2], f64, f64)> {
    if bulge == 0.0 {
        return None;
    }
    let theta = 4.0 * bulge.atan();
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let chord = f64::hypot(dx, dy);
    // Signed distance from the chord's midpoint to the centre, to the left of
    // travel: `c/2 · (1 − b²)/(2b)`.
    let h = chord / 2.0 * (1.0 - bulge * bulge) / (2.0 * bulge);
    let (nx, ny) = (-dy / chord, dx / chord);
    let centre = [(a[0] + b[0]) / 2.0 + nx * h, (a[1] + b[1]) / 2.0 + ny * h];
    let radius = f64::hypot(a[0] - centre[0], a[1] - centre[1]);
    Some((centre, radius, theta))
}

/// A closed outline as points, each bulged span replaced by `per_arc` chords
/// along its true arc.
pub fn flatten(outline: &[gear_core::Vertex], per_arc: usize) -> Vec<[f64; 2]> {
    let n = outline.len();
    let mut out = Vec::with_capacity(n * 2);
    for i in 0..n {
        let v = outline[i];
        let w = outline[(i + 1) % n];
        let (a, b) = ([v.x, v.y], [w.x, w.y]);
        out.push(a);
        if let Some((c, radius, theta)) = arc_of(a, b, v.bulge) {
            let start = (a[1] - c[1]).atan2(a[0] - c[0]);
            for k in 1..per_arc {
                #[allow(clippy::cast_precision_loss)]
                let t = start + theta * k as f64 / per_arc as f64;
                out.push([c[0] + radius * t.cos(), c[1] + radius * t.sin()]);
            }
        }
    }
    out
}

/// How many pairs of non-adjacent segments of a closed polyline cross.
///
/// Proper crossings only — each segment's ends strictly either side of the
/// other. A vertex repeated to rounding (within `1e-12` of the outline's size)
/// is read once, since two spans meeting through a zero-length one would
/// otherwise "cross" by an ulp; whether an outline repeats a vertex is a law
/// of its own. Segments are swept in order of their least `x`, so only pairs
/// whose extents overlap are compared: the count is exact and the cost is near
/// linear on an outline, where crossings can only be local.
pub fn crossings(points: &[[f64; 2]]) -> usize {
    let size = points
        .iter()
        .map(|p| f64::hypot(p[0], p[1]))
        .fold(0.0, f64::max);
    let mut kept: Vec<[f64; 2]> = Vec::with_capacity(points.len());
    for &p in points {
        let near = |q: &[f64; 2]| f64::hypot(p[0] - q[0], p[1] - q[1]) <= 1e-12 * size;
        if !kept.last().is_some_and(near) {
            kept.push(p);
        }
    }
    while kept.len() > 1
        && f64::hypot(
            kept[0][0] - kept[kept.len() - 1][0],
            kept[0][1] - kept[kept.len() - 1][1],
        ) <= 1e-12 * size
    {
        kept.pop();
    }
    let points = &kept;
    let n = points.len();
    let seg = |i: usize| (points[i], points[(i + 1) % n]);
    let cross = |o: [f64; 2], a: [f64; 2], b: [f64; 2]| {
        (a[0] - o[0]) * (b[1] - o[1]) - (a[1] - o[1]) * (b[0] - o[0])
    };
    let mut order: Vec<usize> = (0..n).collect();
    let lo_x = |i: usize| seg(i).0[0].min(seg(i).1[0]);
    let hi_x = |i: usize| seg(i).0[0].max(seg(i).1[0]);
    order.sort_by(|&a, &b| lo_x(a).total_cmp(&lo_x(b)));
    let mut count = 0;
    for (k, &i) in order.iter().enumerate() {
        let (p, q) = seg(i);
        for &j in &order[k + 1..] {
            if lo_x(j) > hi_x(i) {
                break;
            }
            let adjacent = (i + 1) % n == j || (j + 1) % n == i;
            if adjacent {
                continue;
            }
            let (r, s) = seg(j);
            let (d1, d2) = (cross(p, q, r), cross(p, q, s));
            let (d3, d4) = (cross(r, s, p), cross(r, s, q));
            if d1 * d2 < 0.0 && d3 * d4 < 0.0 {
                count += 1;
            }
        }
    }
    count
}
