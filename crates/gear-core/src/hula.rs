//! The hula drive: two internal meshes sharing one crank offset.
//!
//! Four gears in two pairs. Gears 1 and 4 sit on the fixed axis — 1 grounded, 4
//! the output — while gears 2 and 3 ride a body carried on an eccentric crank.
//! Every pair here is therefore separated by the *same* distance, the crank's
//! offset, and that one shared number is what makes the drive a drive rather
//! than two independent meshes.
//!
//! # The ratio is integer arithmetic
//!
//! Willis on each mesh, with the crank as the carrier of both:
//!
//! ```text
//! ω₄/ω_c = 1 − z₁z₃/(z₂z₄)        so      R = z₂z₄ / D,   D = z₂z₄ − z₁z₃
//! ```
//!
//! `D` is an **integer**, and it is the whole design rule. `|D| = 1` gives a
//! ratio of order `z²`; `D = 0` means the output cannot turn at all, which is a
//! refusal rather than a large number; and the mixed arrangements — a wobble
//! body with one external face and one internal one — land at `|D| ≈ 2z` and
//! hence at a ratio of order `z/2`. Nothing here is approximated, so the ratio
//! is reported from the two products rather than from a float.
//!
//! # Which member is a ring is not an input
//!
//! The two axes of a pair are one crank offset apart — a fraction of a module —
//! so a pair can only be an *internal* one, and the ring is whichever member has
//! more teeth. Every arrangement of the four counts therefore describes itself,
//! and the sixteen a designer can write from `z, z±1` are one code path rather
//! than a topology to be named. That is [`crate::mesh::MeshKind`]'s signed
//! convention doing the work again: a ring is a gear with a negative tooth
//! count, and here it is one with the larger one.
//!
//! # One offset, and what is left over
//!
//! A pair has two profile shifts, and only their **difference** reaches either
//! quantity this module solves for. The operating pressure angle takes the
//! difference through [`crate::mesh::operating_geometry`], and so, after the
//! tips are written out, does the far-side gap:
//!
//! ```text
//! C = a_ref − m(h_ring + h_pinion) − m·Σx + e            Σx = x_pinion − x_ring
//! ```
//!
//! Both tips move together with the sum, so it cancels. **The offset therefore
//! decides the difference, and the sum is free** — one spare number per mesh
//! that no geometry claims, which is where an efficiency-optimal distribution
//! belongs when one arrives.
//!
//! So this module has exactly three unknowns — the offset, and one split per
//! mesh — and each of them names what decides it ([`Offset`], [`Split`]). A
//! system where every unknown carries its own source cannot be over- or
//! under-determined, which is why there is no constraint count here to check and
//! no solve order to choose.
//!
//! # Why the far-side gap is the constraint
//!
//! At one tooth of difference a ring and its pinion very nearly fill each other.
//! With ordinary proportions their tip circles *overlap* on the side away from
//! the mesh — at `h = 0.8` and no shift the gap is `−0.6 m` — and a wobble body
//! cannot orbit through that. The gap is what the shift is spent on, and it is
//! the reason a drive of this kind runs at operating pressure angles no ordinary
//! pair would.
//!
//! The two conditions that bite *inside* the mesh — a tip reaching past a flank —
//! are [`crate::ring::mesh_with`]'s, and belong to the pair rather than to the
//! arrangement, so they are asked there rather than restated here.

use crate::plane::BasicRack;
use crate::solve::{newton_bracketed, Tol};

/// The four tooth counts, in the order the drive is read: the grounded gear,
/// the two that ride the wobble body, then the output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Teeth(pub [u32; 4]);

/// One mesh, by role rather than by index.
///
/// Built by [`Teeth::pair`], which decides which member is the ring by counting
/// teeth — see the module note. The indices point back into [`Teeth`], so a
/// caller can put a solved shift where it belongs without knowing the
/// arrangement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pair {
    pub ring: usize,
    pub pinion: usize,
}

/// A reduction, as the two products it comes from.
///
/// Kept as integers because it *is* integers: a drive whose denominator is 1
/// reduces by exactly `z₂z₄`, and rounding that to a float and back is how a
/// ratio starts disagreeing with the teeth it was counted from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ratio {
    /// `z₂z₄` — the product across the wobble body.
    pub numerator: i64,
    /// `D = z₂z₄ − z₁z₃`. Its size is the whole design rule; its sign is the
    /// output's direction.
    pub denominator: i64,
}

impl Ratio {
    /// Input revolutions per output revolution. Negative means the output turns
    /// against the crank.
    #[must_use]
    pub fn value(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }
}

/// What decides the crank offset — the one distance both meshes run at.
///
/// The offset is the stage's own quantity rather than either mesh's, which is
/// why it is named once here and not twice in the pairs.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Offset {
    /// Given, in mm.
    Given(f64),
    /// From the clearance: the tightest mesh sits exactly at the minimum, and
    /// the other one has more than it asked for. Which is which is an outcome,
    /// not an input — see [`Layout::binding`].
    Clearance,
}

/// What decides how a mesh's profile shift is divided between its two members.
///
/// The **difference** is not free: it is what the crank offset is, and what the
/// far-side gap depends on. This names the one freedom a mesh has left, and
/// names it by what fixes it rather than by which member "gets" the shift — so
/// neither member is privileged and a design can be entered from either end.
///
/// What the shape is for beyond the two arms below: with the difference already
/// spent, the split is exactly the freedom a distribution rule needs, so a rule
/// that chooses it — for efficiency, say — is one more arm here rather than
/// another way to describe a gear.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    feature = "typescript",
    derive(ts_rs::TS),
    ts(export, export_to = "core/")
)]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Split {
    /// The ring's shift is given; the pinion's follows from the offset.
    Ring(f64),
    /// The pinion's shift is given; the ring's follows.
    Pinion(f64),
}

/// A drive as its inputs describe it.
#[derive(Clone, Copy, Debug)]
pub struct Set {
    pub teeth: Teeth,
    /// Normal module of each mesh, mm. Two of them, because the pairs need not
    /// share one — only the offset they run at.
    pub module: [f64; 2],
    /// Normal pressure angle, degrees. Shared, as every stage here shares it.
    pub pressure_angle: f64,
    /// Helix angle, degrees. Shared.
    pub helix_angle: f64,
    /// Addendum of each gear, in modules, in [`Teeth`]'s order.
    pub addendum: [f64; 4],
    /// The smallest far-side gap any mesh may run at, mm.
    ///
    /// Always a **minimum** and always checked, whether or not the offset is
    /// taken from it: a given offset that fails it is reported rather than
    /// refused, because it describes a drive that could be built and would
    /// foul, which is a thing a designer is owed the number for.
    pub clearance: f64,
    pub offset: Offset,
    pub split: [Split; 2],
}

/// What a drive's geometry came to.
#[derive(Clone, Copy, Debug)]
pub struct Layout {
    /// The crank offset, mm — the centre distance of both meshes.
    pub offset: f64,
    /// Operating pressure angle of each mesh, radians.
    pub alpha_w: [f64; 2],
    /// Profile shift of each gear, in [`Teeth`]'s order.
    pub shift: [f64; 4],
    /// Far-side tip gap of each mesh, mm.
    pub clearance: [f64; 2],
    /// Which mesh sits at the clearance minimum, when the offset came from it.
    /// `None` when the offset was given, or when neither mesh is what held it
    /// back.
    pub binding: Option<usize>,
    pub ratio: Ratio,
}

/// Why a drive has no geometry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    /// `z₂z₄ = z₁z₃`. The two meshes step by the same amount and cancel, so the
    /// output cannot turn: a reduction of infinity is not a large number.
    Locked,
    /// A pair whose members have the same tooth count is not a mesh — its two
    /// axes would be coincident, and the crank has nowhere to sit.
    Coaxial(usize),
    /// The offset is below the pair's base-circle limit, where no operating
    /// pressure angle exists.
    OffsetTooSmall(usize),
    /// No offset in the involute domain gives this mesh the clearance asked
    /// for.
    ClearanceUnreachable(usize),
}

impl Teeth {
    /// The pair a mesh is, ring first.
    ///
    /// # Errors
    ///
    /// [`Error::Coaxial`] when the two counts are equal.
    pub fn pair(self, mesh: usize) -> Result<Pair, Error> {
        let (a, b) = (mesh * 2, mesh * 2 + 1);
        match self.0[a].cmp(&self.0[b]) {
            std::cmp::Ordering::Greater => Ok(Pair { ring: a, pinion: b }),
            std::cmp::Ordering::Less => Ok(Pair { ring: b, pinion: a }),
            std::cmp::Ordering::Equal => Err(Error::Coaxial(mesh)),
        }
    }

    /// `R = z₂z₄ / (z₂z₄ − z₁z₃)`.
    ///
    /// # Errors
    ///
    /// [`Error::Locked`] when the denominator is zero.
    pub fn ratio(self) -> Result<Ratio, Error> {
        let z = self.0.map(i64::from);
        let numerator = z[1] * z[3];
        let denominator = numerator - z[0] * z[2];
        (denominator != 0)
            .then_some(Ratio {
                numerator,
                denominator,
            })
            .ok_or(Error::Locked)
    }
}

/// One mesh reduced to the quantities the offset and the gap are written in.
#[derive(Clone, Copy, Debug)]
struct Geometry {
    rack: BasicRack,
    /// `z_pinion − z_ring`, and so always negative.
    sum_z: f64,
    /// Reference centre distance, mm.
    a_ref: f64,
    /// `h_ring + h_pinion`, in modules. Only the sum reaches the gap.
    addendum_sum: f64,
}

impl Geometry {
    fn of(set: &Set, mesh: usize, pair: Pair) -> Self {
        let rack = BasicRack::new(set.module[mesh], set.pressure_angle, set.helix_angle);
        let sum_z = f64::from(set.teeth.0[pair.pinion]) - f64::from(set.teeth.0[pair.ring]);
        Self {
            rack,
            sum_z,
            a_ref: rack.mt * sum_z.abs() / 2.0,
            addendum_sum: set.addendum[pair.ring] + set.addendum[pair.pinion],
        }
    }

    /// The offset a pair runs at, at an operating pressure angle.
    fn offset_at(&self, alpha_w: f64) -> f64 {
        self.a_ref * self.rack.alpha_t.cos() / alpha_w.cos()
    }

    /// ...and the operating pressure angle it runs at, at an offset. The same
    /// relation read backwards, which is why it is here rather than solved for.
    fn alpha_w_at(&self, offset: f64) -> Option<f64> {
        let cos_w = self.a_ref * self.rack.alpha_t.cos() / offset;
        (offset > 0.0 && cos_w <= 1.0).then(|| cos_w.acos())
    }

    /// `Σx = x_pinion − x_ring`, from [`crate::mesh::operating_geometry`]'s
    /// relation solved for the shift rather than for the angle.
    fn sum_x_at(&self, alpha_w: f64) -> f64 {
        self.sum_z * (crate::inv(alpha_w) - crate::inv(self.rack.alpha_t))
            / (2.0 * self.rack.alpha_n.tan())
    }

    /// The far-side gap between the two tip circles.
    ///
    /// `a_ref` is transverse and the two tooth terms are normal, which is not a
    /// slip: an addendum and a shift are what the *tool* cuts, across the tooth.
    fn clearance_at(&self, alpha_w: f64) -> f64 {
        self.a_ref - self.rack.mn * (self.addendum_sum + self.sum_x_at(alpha_w))
            + self.offset_at(alpha_w)
    }

    /// `dC/dα_w`, analytic.
    ///
    /// `d(inv α)/dα = tan²α` gives the shift term and `de/dα_w = e tan α_w` the
    /// offset term. `sum_z` is negative, so **both terms are positive** across
    /// the whole domain: the gap is strictly increasing in the operating
    /// pressure angle, the root is unique, and Newton cannot be led astray.
    fn d_clearance(&self, alpha_w: f64) -> f64 {
        let t = alpha_w.tan();
        -self.rack.mn * self.sum_z * t * t / (2.0 * self.rack.alpha_n.tan())
            + self.offset_at(alpha_w) * t
    }

    /// The smallest offset that gives this mesh `wanted` of gap.
    ///
    /// Returns the domain floor when the mesh already has more than it asked
    /// for there — the mesh is then not what holds the drive open, and saying
    /// so is the difference between "no solution" and "not this one's problem".
    fn offset_for_clearance(&self, wanted: f64) -> Option<f64> {
        // The involute domain, from the base-circle limit to the tangent's
        // asymptote. Both ends are excluded, and both are approached rather
        // than reached, so the bracket is the domain rather than a guess at it.
        let (lo, hi) = (1e-9, std::f64::consts::FRAC_PI_2 - 1e-9);
        if self.clearance_at(lo) >= wanted {
            return Some(self.offset_at(lo));
        }
        let alpha_w = newton_bracketed(
            |a| self.clearance_at(a) - wanted,
            |a| self.d_clearance(a),
            lo,
            hi,
            self.rack.alpha_t,
            Tol::default(),
        )?;
        Some(self.offset_at(alpha_w))
    }
}

/// Solve a drive: one offset, and the four shifts that let both meshes run at
/// it.
///
/// # Errors
///
/// Every arm of [`Error`]. Each is a drive that cannot exist rather than a
/// solver that gave up, and each names which mesh could not be made to work.
pub fn solve(set: &Set) -> Result<Layout, Error> {
    let ratio = set.teeth.ratio()?;
    let pairs = [set.teeth.pair(0)?, set.teeth.pair(1)?];
    let geometry = [
        Geometry::of(set, 0, pairs[0]),
        Geometry::of(set, 1, pairs[1]),
    ];

    // The offset. Where it comes from the clearance, each mesh names the
    // smallest that will do and the larger of the two is what both must run at
    // — so the mesh that asked for more sits exactly at the minimum and the
    // other has slack. Taking the maximum is the whole of that argument: the
    // gap rises with the offset in both meshes, so nothing is given up by
    // opening out to the one that needs it.
    let (offset, binding) = match set.offset {
        Offset::Given(e) => (e, None),
        Offset::Clearance => {
            let wants = [
                geometry[0]
                    .offset_for_clearance(set.clearance)
                    .ok_or(Error::ClearanceUnreachable(0))?,
                geometry[1]
                    .offset_for_clearance(set.clearance)
                    .ok_or(Error::ClearanceUnreachable(1))?,
            ];
            let held_by = usize::from(wants[1] > wants[0]);
            (wants[held_by], Some(held_by))
        }
    };

    let mut alpha_w = [0.0; 2];
    let mut clearance = [0.0; 2];
    let mut shift = [0.0; 4];
    for (mesh, geo) in geometry.iter().enumerate() {
        let a_w = geo.alpha_w_at(offset).ok_or(Error::OffsetTooSmall(mesh))?;
        alpha_w[mesh] = a_w;
        clearance[mesh] = geo.clearance_at(a_w);
        // The difference is the offset's; the sum is whatever the split says.
        let sum_x = geo.sum_x_at(a_w);
        let (ring, pinion) = (pairs[mesh].ring, pairs[mesh].pinion);
        match set.split[mesh] {
            Split::Ring(x) => {
                shift[ring] = x;
                shift[pinion] = x + sum_x;
            }
            Split::Pinion(x) => {
                shift[pinion] = x;
                shift[ring] = x - sum_x;
            }
        }
    }

    Ok(Layout {
        offset,
        alpha_w,
        shift,
        clearance,
        binding,
        ratio,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// The drive the rest of the tests vary: the arrangement that reduces by
    /// `z²`, at the proportions a one-tooth-difference pair needs.
    fn set(teeth: [u32; 4]) -> Set {
        Set {
            teeth: Teeth(teeth),
            module: [1.0, 1.0],
            pressure_angle: 20.0,
            helix_angle: 0.0,
            addendum: [0.8; 4],
            clearance: 0.5,
            offset: Offset::Clearance,
            split: [Split::Pinion(0.0); 2],
        }
    }

    fn n_squared(n: u32) -> [u32; 4] {
        [n + 1, n, n - 1, n]
    }

    /// **The ratio, against the closed forms rather than against itself.**
    ///
    /// Every arrangement a designer can write from `z` and `z ± 1` with one
    /// tooth of difference in each mesh — sixteen of them — checked against the
    /// expression each was derived to be, at four tooth counts. The expressions
    /// come from Willis and from integrating the rolling pitch circles
    /// (`tools/hula_kinematics.py`), neither of which is the two products this
    /// module reports, so agreeing is evidence rather than a tautology.
    ///
    /// The families are the design rule made visible: the four that cancel are
    /// **locked**, the four with `|D| = 1` reduce by order `z²`, and the eight
    /// mixed arrangements — a wobble body carrying one external face and one
    /// internal one — cannot exceed about `z/2` however the counts are chosen.
    #[test]
    fn the_ratio_is_the_two_products() {
        #[allow(clippy::type_complexity)]
        let cases: [([i32; 4], Option<fn(f64) -> f64>); 16] = [
            ([1, 0, -1, 0], Some(|n| n * n)),
            ([-1, 0, 1, 0], Some(|n| n * n)),
            ([0, -1, 0, 1], Some(|n| -(n * n - 1.0))),
            ([0, 1, 0, -1], Some(|n| -(n * n - 1.0))),
            (
                [0, 1, 0, 1],
                Some(|n| (n + 1.0) * (n + 1.0) / (2.0 * n + 1.0)),
            ),
            ([-1, 0, 0, 1], Some(|n| (n + 1.0) / 2.0)),
            ([0, 1, -1, 0], Some(|n| (n + 1.0) / 2.0)),
            ([-1, 0, -1, 0], Some(|n| n * n / (2.0 * n - 1.0))),
            ([1, 0, 1, 0], Some(|n| -n * n / (2.0 * n + 1.0))),
            ([0, -1, 1, 0], Some(|n| -(n - 1.0) / 2.0)),
            ([1, 0, 0, -1], Some(|n| -(n - 1.0) / 2.0)),
            (
                [0, -1, 0, -1],
                Some(|n| -(n - 1.0) * (n - 1.0) / (2.0 * n - 1.0)),
            ),
            ([-1, 0, 0, -1], None),
            ([0, -1, -1, 0], None),
            ([0, 1, 1, 0], None),
            ([1, 0, 0, 1], None),
        ];

        for n in [6_u32, 12, 18, 30] {
            for (offsets, expected) in cases {
                let z = offsets.map(|o| u32::try_from(i64::from(n) + i64::from(o)).unwrap());
                let got = Teeth(z).ratio();
                match expected {
                    None => assert_eq!(got, Err(Error::Locked), "{z:?} should cancel"),
                    Some(f) => {
                        let want = f(f64::from(n));
                        let have = got.unwrap().value();
                        assert!(
                            (have - want).abs() < 1e-9 * want.abs().max(1.0),
                            "{z:?}: ratio {have} but the closed form says {want}"
                        );
                    }
                }
            }
        }
    }

    /// A denominator of zero is a drive whose meshes step by the same amount and
    /// cancel. The output cannot turn, which is a refusal and not a large
    /// number — and four of the sixteen arrangements are exactly this, so it is
    /// an ordinary mistake rather than an exotic one.
    #[test]
    fn a_drive_whose_meshes_cancel_is_refused() {
        assert_eq!(Teeth([17, 18, 18, 17]).ratio(), Err(Error::Locked));
        assert_eq!(solve(&set([17, 18, 18, 17])).unwrap_err(), Error::Locked);
    }

    #[test]
    fn a_pair_of_equal_counts_is_not_a_mesh() {
        assert_eq!(Teeth([18, 18, 17, 18]).pair(0), Err(Error::Coaxial(0)));
        assert_eq!(
            solve(&set([18, 18, 17, 18])).unwrap_err(),
            Error::Coaxial(0)
        );
    }

    /// Which member is the ring is read off the counts, in either order.
    #[test]
    fn the_larger_member_of_a_pair_is_its_ring() {
        assert_eq!(
            Teeth([19, 18, 17, 18]).pair(0).unwrap(),
            Pair { ring: 0, pinion: 1 }
        );
        assert_eq!(
            Teeth([19, 18, 17, 18]).pair(1).unwrap(),
            Pair { ring: 3, pinion: 2 }
        );
    }

    /// **The two meshes run at one offset**, checked by the other route.
    ///
    /// The solve reaches each mesh's operating pressure angle from the offset
    /// through an arc cosine; this reads the solved shifts forwards through
    /// [`crate::mesh::operating_geometry`], which goes the other way round via
    /// `inv⁻¹`. Agreeing to machine precision is the two directions closing on
    /// each other, which is what "both meshes run at the crank's offset" has to
    /// mean if it means anything.
    #[test]
    fn the_two_meshes_run_at_one_offset() {
        for teeth in [n_squared(18), [19, 18, 18, 17], [17, 18, 19, 18]] {
            let s = set(teeth);
            let l = solve(&s).unwrap();
            for mesh in 0..2 {
                let pair = s.teeth.pair(mesh).unwrap();
                let rack = BasicRack::new(s.module[mesh], s.pressure_angle, s.helix_angle);
                let sum_z = f64::from(teeth[pair.pinion]) - f64::from(teeth[pair.ring]);
                let sum_x = l.shift[pair.pinion] - l.shift[pair.ring];
                let (alpha_w, _, a_w) = crate::mesh::operating_geometry(
                    rack.mt,
                    rack.alpha_t,
                    rack.alpha_n,
                    sum_z,
                    sum_x,
                )
                .unwrap();
                assert!(
                    (a_w - l.offset).abs() < 1e-12,
                    "{teeth:?} mesh {mesh}: {a_w} vs the crank's {}",
                    l.offset
                );
                assert!((alpha_w - l.alpha_w[mesh]).abs() < 1e-12);
            }
        }
    }

    /// The mesh that asked for the most sits exactly at the minimum; the other
    /// has more than it asked for. Neither is short.
    ///
    /// **Every drive here has two meshes that differ**, and that is the whole
    /// point of the list: a drive whose pairs share a module and a tooth
    /// difference has two identical meshes, so the larger requirement and the
    /// smaller are the same number and taking either passes. An axis nobody
    /// turns is an axis nobody tests, and this one decides whether a mesh is
    /// left fouling.
    #[test]
    fn the_binding_mesh_sits_at_the_clearance_minimum() {
        let drives = [
            // unequal modules: the pairs want different offsets
            Set {
                module: [1.3, 1.0],
                ..set(n_squared(18))
            },
            Set {
                module: [1.0, 1.4],
                ..set(n_squared(18))
            },
            // unequal tooth differences, which is the other way they can differ
            set([20, 18, 17, 18]),
            // ...and unequal addenda, which move the gap and nothing else
            Set {
                addendum: [1.0, 0.8, 0.6, 0.8],
                ..set(n_squared(18))
            },
            // the symmetric case, where either answer is the same answer
            set(n_squared(18)),
        ];
        for s in drives {
            let l = solve(&s).unwrap();
            let held = l.binding.unwrap();
            assert!(
                (l.clearance[held] - s.clearance).abs() < 1e-9,
                "{:?}: the binding mesh should be at {} but has {}",
                s.teeth,
                s.clearance,
                l.clearance[held]
            );
            assert!(
                l.clearance[1 - held] >= l.clearance[held] - 1e-9,
                "{:?}: the other mesh should have slack, not less",
                s.teeth
            );
            for (mesh, c) in l.clearance.iter().enumerate() {
                assert!(
                    *c >= s.clearance - 1e-9,
                    "{:?}: mesh {mesh} is below the minimum at {c}",
                    s.teeth
                );
            }
        }
    }

    /// The gap rises with the offset, in both meshes and over the whole domain.
    ///
    /// It is what makes the root unique and taking the larger of the two
    /// requirements safe — open the drive out for the mesh that needs it and the
    /// other one gains too. Sampled here; the analytic statement is
    /// [`Geometry::d_clearance`], whose two terms are both positive.
    #[test]
    fn the_clearance_rises_with_the_offset() {
        let s = set(n_squared(18));
        let mut last = [f64::NEG_INFINITY; 2];
        for step in 1..200 {
            let offset = 0.4 + f64::from(step) * 0.01;
            let l = solve(&Set {
                offset: Offset::Given(offset),
                ..s
            });
            let Ok(l) = l else { continue };
            for (mesh, (now, previous)) in l.clearance.iter().zip(last.iter_mut()).enumerate() {
                assert!(*now > *previous, "mesh {mesh} fell back at offset {offset}");
                *previous = *now;
            }
        }
        assert!(last[0].is_finite() && last[1].is_finite(), "nothing solved");
    }

    /// **The split moves neither the offset nor the gap.**
    ///
    /// The freedom this module leaves over is real: only the difference of a
    /// mesh's two shifts reaches its operating pressure angle or its far-side
    /// gap, so the sum can be moved anywhere without disturbing either. Gated
    /// bit-exactly, because it is the property an efficiency-optimal
    /// distribution will stand on — if the split moved the clearance, choosing
    /// it for efficiency would quietly close the drive up.
    #[test]
    fn the_split_moves_neither_offset_nor_clearance() {
        let base = set(n_squared(18));
        let a = solve(&base).unwrap();
        for split in [Split::Ring(0.0), Split::Ring(-1.25), Split::Pinion(0.4)] {
            let b = solve(&Set {
                split: [split; 2],
                ..base
            })
            .unwrap();
            assert_eq!(a.offset, b.offset);
            assert_eq!(a.clearance, b.clearance);
            assert_eq!(a.alpha_w, b.alpha_w);
            for mesh in 0..2 {
                let pair = base.teeth.pair(mesh).unwrap();
                let d = |l: &Layout| l.shift[pair.pinion] - l.shift[pair.ring];
                assert!(
                    (d(&a) - d(&b)).abs() < 1e-12,
                    "the difference is the offset's and must not move"
                );
            }
        }
    }

    /// The split says which member is given, and either member can be the one.
    #[test]
    fn either_member_may_be_the_given_one() {
        let base = set(n_squared(18));
        let l = solve(&Set {
            split: [Split::Ring(0.25), Split::Pinion(-0.1)],
            ..base
        })
        .unwrap();
        assert!((l.shift[base.teeth.pair(0).unwrap().ring] - 0.25).abs() < 1e-12);
        assert!((l.shift[base.teeth.pair(1).unwrap().pinion] + 0.1).abs() < 1e-12);
    }

    #[test]
    fn a_given_offset_is_the_offset() {
        let l = solve(&Set {
            offset: Offset::Given(0.9),
            ..set(n_squared(18))
        })
        .unwrap();
        assert!((l.offset - 0.9).abs() < 1e-12);
        assert_eq!(l.binding, None, "nothing held a given offset back");
    }

    /// **A two-tooth difference needs no new code.**
    ///
    /// Nothing here counts to one: the difference enters only through the
    /// reference centre distance, so an arrangement built from `z, z ± 2` solves
    /// through the same expressions and reduces by `z²/4` — a quarter of the
    /// one-tooth drive, which is the arithmetic of `D = 4` rather than a
    /// separate model.
    #[test]
    fn a_two_tooth_difference_needs_no_new_code() {
        let teeth = [20_u32, 18, 16, 18];
        let r = Teeth(teeth).ratio().unwrap();
        assert_eq!((r.numerator, r.denominator), (324, 4));
        assert!((r.value() - 81.0).abs() < 1e-12);
        let l = solve(&set(teeth)).unwrap();
        assert!(l.offset > 0.0 && l.clearance.iter().all(|c| *c >= 0.5 - 1e-9));
    }
}
