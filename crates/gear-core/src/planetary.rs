//! Planetary layout: the shift that makes two centre distances agree, and the
//! checks a set of planets has to pass.
//!
//! A planetary stage has one constraint no other stage has. The sun and the ring
//! are **coaxial**, so the sun–planet centre distance and the planet–ring centre
//! distance are the same distance measured twice, and nothing in the tooth counts
//! makes them equal. Only `z_ring = z_sun + 2 z_planet` does it for free; every
//! other combination has to be brought into line, and the free variable is the
//! planet's profile shift.
//!
//! # Why the solve is safe
//!
//! ```text
//! g(x_p) = a_ext(x_s + x_p) − a_int(x_p − x_r)
//! ```
//!
//! `a_ext` **increases** with the planet's shift and `a_int` **decreases** with
//! it — the external sum grows while the internal one, divided by a negative
//! tooth sum, drives its operating pressure angle down. So `g` is strictly
//! increasing, its root is unique, and Newton cannot be led astray.
//!
//! That the two move opposite ways is the whole reason a solution exists, and it
//! is worth seeing why the internal one reverses: `z_p − z_r` is negative, so the
//! same `+2 Σx tan α_n / Σz` that raises `inv α_w` on an external pair lowers it
//! on an internal one. It is the signed convention of [`crate::mesh`] doing the
//! work, not a special case.
//!
//! # And why it is bracketed rather than merely seeded
//!
//! Both meshes need `inv α_w ≥ 0`; below that the base circles would have to
//! overlap and there is no such pair. That bounds `x_p` from both sides in closed
//! form, and the bound is not academic — for a 17-tooth sun and 17-tooth planets
//! only `z_ring ∈ [48, 54]` admits any solution at all. The rest are genuinely
//! impossible rather than merely unconverged, and the difference is what lets the
//! UI say *why*.
//!
//! # What is not here
//!
//! Radial assembly — whether a planet can be brought in sideways past the ring's
//! teeth. It is a swept-motion question, not a comparison of tip circles, and
//! `docs/DESIGN.md` §4.11 records what happened to the attempt that treated it as
//! one. Efficiency is §4.5.2 and belongs with the stage.

use crate::mesh::operating_geometry;
use crate::solve::{newton_bracketed, Tol};

/// The reference rack a planetary set shares.
///
/// All three members are cut by it, so the module, both pressure angles and the
/// helix live here once rather than three times — the same reason a stage owns
/// them rather than its gears (§3.2).
#[derive(Clone, Copy, Debug)]
pub struct Rack {
    /// Transverse module, mm.
    pub mt: f64,
    /// Transverse pressure angle, radians.
    pub alpha_t: f64,
    /// Normal pressure angle, radians.
    pub alpha_n: f64,
}

impl Rack {
    /// From the inputs as the UI holds them: normal module, and both angles in
    /// degrees.
    #[must_use]
    pub fn new(module: f64, pressure_angle_deg: f64, helix_angle_deg: f64) -> Self {
        let beta = helix_angle_deg.to_radians();
        let alpha_n = pressure_angle_deg.to_radians();
        Self {
            mt: module / beta.cos(),
            alpha_t: (alpha_n.tan() / beta.cos()).atan(),
            alpha_n,
        }
    }
}

/// The three tooth counts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Teeth {
    pub sun: u32,
    pub planet: u32,
    pub ring: u32,
}

impl Teeth {
    /// Signed tooth sums of the two meshes: sun–planet, then planet–ring.
    ///
    /// The second is negative, because the ring is member 2 of an internal pair
    /// (see [`crate::mesh::MeshKind::sign`]). Everything below is one expression
    /// for both meshes because of it.
    fn sums(self) -> (f64, f64) {
        (
            f64::from(self.sun) + f64::from(self.planet),
            f64::from(self.planet) - f64::from(self.ring),
        )
    }

    /// The ring tooth count that needs no planet shift at all.
    ///
    /// `z_s + 2 z_p` puts the planet exactly halfway, so both centre distances
    /// are their reference values and agree without help. It is the sanity check
    /// the whole construction has to pass.
    #[must_use]
    pub fn ideal_ring(sun: u32, planet: u32) -> u32 {
        sun + 2 * planet
    }
}

/// A solved planetary layout.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Layout {
    /// The planet thickness shift that makes the two centre distances agree.
    pub planet_shift: f64,
    /// The common centre distance, mm — sun-to-planet and planet-to-ring, which
    /// are now the same number.
    pub centre_distance: f64,
    /// Operating pressure angle of the sun–planet mesh, radians.
    pub alpha_w_sun: f64,
    /// ...and of the planet–ring mesh.
    pub alpha_w_ring: f64,
    /// Residual `|a_ext − a_int|` at the returned shift, mm.
    ///
    /// Reported rather than asserted. It is the one number that says the solve
    /// actually closed, and a caller that wants to trust the layout can look at
    /// it instead of trusting this module.
    pub residual: f64,
    /// Planets can be spaced evenly around the sun: `(z_s + z_r) mod N = 0`.
    pub equal_spacing: bool,
    /// Every planet meshes at the same phase — `N | z_s` and `N | z_r`.
    ///
    /// Given equal spacing either implies the other. Rarely true, and its being
    /// false is not a fault: it means the planets engage staggered, which is
    /// usually preferable.
    pub simultaneous_meshing: bool,
    /// Gap between the tip circles of adjacent planets, mm.
    ///
    /// `2 a sin(π/N) − d_a,planet`. Negative means they overlap and the set
    /// cannot be built. `None` for a single planet, which has no neighbour.
    pub planet_clearance: Option<f64>,
}

/// A planetary set as its inputs describe it.
///
/// Gathered rather than passed loose because the two entry points below want the
/// same six things, and a six-argument call is a place to transpose two floats
/// silently. The same reason [`crate::shaper::CutParams`] exists.
#[derive(Clone, Copy, Debug)]
pub struct Set {
    pub rack: Rack,
    pub teeth: Teeth,
    /// How many planets. One is legal — it has no neighbour to clear.
    pub planets: u32,
    /// Thickness shift of the sun, `x + x_s`.
    pub sun_shift: f64,
    /// ...and of the ring, acting on its **space** (see [`crate::ring::Ring`]).
    pub ring_shift: f64,
    /// The planet's tip diameter, mm — needed only for planet-to-planet
    /// clearance, which is the one check that cares how big a planet is rather
    /// than how many teeth it has.
    pub planet_tip_diameter: f64,
}

/// The shifts a planet could have, from the involute domain alone.
///
/// Both meshes need `inv α_w ≥ 0`, and each bounds the planet's shift from one
/// side:
///
/// ```text
/// x_p ≥ −inv α_t (z_s + z_p) / (2 tan α_n) − x_s        external
/// x_p ≤  x_r + inv α_t (z_r − z_p) / (2 tan α_n)        internal
/// ```
///
/// Returns `None` when the two cross, which is a set that cannot exist at any
/// planet shift.
#[must_use]
pub fn shift_bracket(set: &Set) -> Option<(f64, f64)> {
    let (sum_ext, sum_int) = set.teeth.sums();
    let reach = crate::inv(set.rack.alpha_t) / (2.0 * set.rack.alpha_n.tan());
    let lo = -reach * sum_ext - set.sun_shift;
    // `sum_int` is negative, so this is the *upper* bound; writing it with the
    // signed sum keeps it the same expression as the line above rather than a
    // mirrored one.
    let hi = set.ring_shift - reach * sum_int;
    (lo.is_finite() && hi.is_finite() && hi > lo).then_some((lo, hi))
}

/// `da_w/dΣx`, analytic.
///
/// From `a_w = a_ref cos α_t / cos α_w`, `d(inv α)/dα = tan²α` and
/// `d(inv α_w)/dΣx = 2 tan α_n / Σz`, the chain rule collapses to
///
/// ```text
/// da_w/dΣx = 2 a_w tan α_n / (Σz tan α_w)
/// ```
///
/// which carries its own sign: `Σz < 0` for an internal pair, so the same
/// expression makes its centre distance *fall* as its shift sum rises.
fn d_centre_distance(a_w: f64, alpha_w: f64, alpha_n: f64, sum_z: f64) -> f64 {
    2.0 * a_w * alpha_n.tan() / (sum_z * alpha_w.tan())
}

/// Pull a bracket endpoint inward until the residual there is a number.
///
/// The bracket's ends sit *exactly* on the involute domain's boundary, where
/// `inv α_w = 0`. Whether that arithmetic lands on zero or on −1e-17 is a matter
/// of rounding in the tooth counts, so on some sets an endpoint falls a hair
/// outside a domain it is meant to touch — and the solve was refused for a root
/// sitting comfortably inside. A 24/16 set with four planets lost `z_ring = 57`
/// exactly that way, leaving a hole in a run that is supposed to be contiguous.
///
/// Halving toward a point known to be inside finds the last representable point
/// in the domain. There is no tolerance to choose: it stops when the answer
/// becomes a number, and the iteration bound is the same exhaust-the-mantissa
/// bound [`Tol`] uses rather than a tuning parameter.
fn pull_in(g: &impl Fn(f64) -> f64, from: f64, toward: f64) -> Option<f64> {
    let mut x = from;
    for _ in 0..Tol::default().max_iter {
        if g(x).is_finite() {
            return Some(x);
        }
        x = 0.5 * (x + toward);
    }
    None
}

/// Solve for the planet shift that makes the two centre distances equal.
///
/// # Errors
///
/// `None` when no planet shift can serve — either the involute domain admits
/// none ([`shift_bracket`]), or the residual does not change sign across it,
/// which for a strictly increasing `g` means the root lies outside the domain.
/// A tooth count that cannot work is the common case, not an exceptional one:
/// most `z_ring` values fail here.
#[must_use]
pub fn solve(set: &Set) -> Option<Layout> {
    let (rack, teeth, planets) = (&set.rack, set.teeth, set.planets);
    let (sun_shift, ring_shift) = (set.sun_shift, set.ring_shift);
    let (sum_ext, sum_int) = teeth.sums();
    if teeth.ring <= teeth.planet || teeth.planet == 0 || teeth.sun == 0 {
        return None;
    }

    // The two centre distances, each as a function of the planet's shift.
    let ext = |x_p: f64| {
        operating_geometry(
            rack.mt,
            rack.alpha_t,
            rack.alpha_n,
            sum_ext,
            sun_shift + x_p,
        )
    };
    let int = |x_p: f64| {
        operating_geometry(
            rack.mt,
            rack.alpha_t,
            rack.alpha_n,
            sum_int,
            x_p - ring_shift,
        )
    };

    let g = |x_p: f64| match (ext(x_p), int(x_p)) {
        (Some((_, _, a_e)), Some((_, _, a_i))) => a_e - a_i,
        // Outside the involute domain the residual has no value, and returning a
        // number here would let the solver walk into it. `newton_bracketed`
        // rejects a non-finite endpoint, which is the right answer.
        _ => f64::NAN,
    };
    let dg = |x_p: f64| match (ext(x_p), int(x_p)) {
        (Some((aw_e, _, a_e)), Some((aw_i, _, a_i))) => {
            d_centre_distance(a_e, aw_e, rack.alpha_n, sum_ext)
                - d_centre_distance(a_i, aw_i, rack.alpha_n, sum_int)
        }
        _ => f64::NAN,
    };

    let (lo, hi) = shift_bracket(set)?;
    // The endpoints are on the domain boundary, so bring each just inside before
    // handing them over — see `pull_in`.
    let mid = 0.5 * (lo + hi);
    if !g(mid).is_finite() {
        return None;
    }
    let (lo, hi) = (pull_in(&g, lo, mid)?, pull_in(&g, hi, mid)?);
    // Newton from zero shift, which is where the answer sits for the ideal ring
    // and near it for its neighbours; the maintained bracket makes the seed a
    // convenience rather than a requirement.
    let x_p = newton_bracketed(g, dg, lo, hi, 0.0_f64.clamp(lo, hi), Tol::default())?;

    let (alpha_w_sun, _, a_e) = ext(x_p)?;
    let (alpha_w_ring, _, a_i) = int(x_p)?;

    let equal_spacing = planets > 0 && (teeth.sun + teeth.ring) % planets == 0;
    let simultaneous_meshing = planets > 0 && teeth.sun % planets == 0 && teeth.ring % planets == 0;
    let planet_clearance = (planets > 1).then(|| {
        2.0 * a_e * (std::f64::consts::PI / f64::from(planets)).sin() - set.planet_tip_diameter
    });

    Some(Layout {
        planet_shift: x_p,
        centre_distance: a_e,
        alpha_w_sun,
        alpha_w_ring,
        residual: (a_e - a_i).abs(),
        equal_spacing,
        simultaneous_meshing,
        planet_clearance,
    })
}

/// Every ring tooth count that can be made to work, ascending.
///
/// # Why the search is provably complete
///
/// The required planet shift is **strictly increasing in `z_ring`**: a bigger
/// ring needs the planet pushed further out. So the admissible counts are a
/// contiguous run, and sweeping until the required shift passes `max_shift`
/// cannot skip a solution — which is what makes this a search rather than a
/// sample. `limit` bounds the sweep for a caller that wants one; the monotonicity
/// is what makes the answer complete rather than the limit.
#[must_use]
pub fn ring_candidates(set: &Set, shift_range: (f64, f64), limit: u32) -> Vec<(u32, Layout)> {
    let mut out = Vec::new();
    // `set.teeth.ring` is the one field this ignores: it is what the sweep varies.
    for ring in (set.teeth.planet + 1)..=limit {
        let candidate = Set {
            teeth: Teeth { ring, ..set.teeth },
            ..*set
        };
        let Some(layout) = solve(&candidate) else {
            continue;
        };
        if layout.planet_shift < shift_range.0 {
            continue;
        }
        // Monotone in `z_ring`, so once the required shift passes the top of the
        // range every larger ring does too.
        if layout.planet_shift > shift_range.1 {
            break;
        }
        out.push((ring, layout));
    }
    out
}

// ------------------------------------------------------------- kinematics ---

/// One of the three shafts a planetary set presents.
///
/// The planets themselves are not on this list: they have no shaft of their own,
/// and their speed is a consequence of the other three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum Member {
    Sun,
    Carrier,
    Ring,
}

impl Member {
    /// Index into the `[sun, carrier, ring]` arrays below.
    const fn index(self) -> usize {
        match self {
            Self::Sun => 0,
            Self::Carrier => 1,
            Self::Ring => 2,
        }
    }

    /// The member that is neither of these two.
    ///
    /// A planetary set has three shafts and exactly two are chosen — one driven,
    /// one held — so the third is not a choice at all.
    fn other(a: Self, b: Self) -> Option<Self> {
        [Self::Sun, Self::Carrier, Self::Ring]
            .into_iter()
            .find(|&m| m != a && m != b)
            .filter(|_| a != b)
    }
}

/// Which shaft drives and which is held.
///
/// **This is an addition to the specification's field list** (§8.1). The
/// specification names only "Driven By", which picks one shaft of three and
/// leaves the arrangement undetermined: a sun-driven set behaves quite
/// differently with the ring held than with the carrier held. Naming the held
/// shaft as well is what makes the six modes of §4.5.2 reachable.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct Arrangement {
    pub input: Member,
    pub fixed: Member,
}

/// What the three shafts do, and what it costs to make them do it.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Power {
    /// Angular speeds `[sun, carrier, ring]`, in whatever unit the input was
    /// given. The held shaft is exactly zero.
    pub speeds: [f64; 3],
    /// Torques `[sun, carrier, ring]`, in whatever unit the input was given.
    /// They sum to zero — the set is in equilibrium, and the held shaft's torque
    /// is the reaction it carries.
    pub torques: [f64; 3],
    /// Speed reduction, input over output. Negative when the output turns the
    /// other way.
    pub ratio: f64,
    /// Mechanical efficiency, `|T_out ω_out| / |T_in ω_in|`.
    pub efficiency: f64,
    /// The member the other two leave over.
    pub output: Member,
    /// Sign of the rolling power — which way power crosses the meshes in the
    /// carrier's frame. `+1` when the sun leads the carrier under a driving
    /// torque, `−1` when it trails.
    pub rolling_power_sign: f64,
}

/// The basic, carrier-fixed ratio `i₀ = −z_ring / z_sun`.
///
/// Negative because with the carrier held the sun and ring turn opposite ways —
/// the planet reverses the sense once and the internal mesh does not reverse it
/// again. Everything below is written in terms of this one number, which is what
/// makes the six modes one piece of algebra rather than six.
#[must_use]
pub fn basic_ratio(teeth: Teeth) -> f64 {
    -(f64::from(teeth.ring) / f64::from(teeth.sun))
}

/// Solve the whole set: speeds, torques, ratio and efficiency, in one go.
///
/// # The method
///
/// Pennestrì–Freudenstein, as §4.5.2 sets it out. Two linear relations carry
/// everything:
///
/// ```text
/// ω_s + (i₀ − 1) ω_c − i₀ ω_r = 0             Willis — kinematics
/// T_s : T_c : T_r = 1 : −(1 − k) : −k         k = i₀ η₀^w — equilibrium with loss
/// ```
///
/// The first is Willis's equation rearranged so all three speeds appear
/// symmetrically; the second is torque equilibrium with the mesh loss folded in
/// through `η₀^w`. Both are written once with the member's *index* selecting a
/// coefficient, so no mode is a special case of any other.
///
/// **Efficiency must not be taken mesh by mesh in the fixed frame.** The meshes
/// slide at their speeds relative to the *carrier*, not to ground, which is why
/// `η₀` — the fixed-carrier efficiency — is the quantity that enters. A set whose
/// two meshes are each 99 % efficient can be far worse than 98 % overall, and can
/// self-lock; that is a real property of the arrangement, not an error.
///
/// # The sign of the rolling power
///
/// `w = sgn(T_s (ω_s − ω_c))` decides whether `η₀` multiplies or divides, and it
/// depends on a torque that is itself being solved for. Rather than assume it,
/// both values are tried and the self-consistent one kept — there are only two,
/// and consistency picks between them.
///
/// # Errors
///
/// `None` when the same member is both driven and held, when the tooth counts
/// make a relation degenerate, or when neither sign of the rolling power is
/// self-consistent.
#[must_use]
pub fn power(
    teeth: Teeth,
    arrangement: Arrangement,
    input_speed: f64,
    input_torque: f64,
    fixed_carrier_efficiency: f64,
) -> Option<Power> {
    let output = Member::other(arrangement.input, arrangement.fixed)?;
    let (i, f, o) = (
        arrangement.input.index(),
        arrangement.fixed.index(),
        output.index(),
    );
    let i0 = basic_ratio(teeth);
    if !i0.is_finite() || !fixed_carrier_efficiency.is_finite() {
        return None;
    }

    // Willis, with the held shaft at zero: one equation, one unknown.
    let willis = [1.0, i0 - 1.0, -i0];
    if willis[o].abs() < f64::MIN_POSITIVE {
        return None;
    }
    let mut speeds = [0.0; 3];
    speeds[i] = input_speed;
    speeds[f] = 0.0;
    speeds[o] = -willis[i] * input_speed / willis[o];

    // ...then equilibrium, for each candidate sign of the rolling power.
    for w in [1.0, -1.0] {
        let k = i0 * fixed_carrier_efficiency.powf(w);
        let shares = [1.0, -(1.0 - k), -k];
        if shares[i].abs() < f64::MIN_POSITIVE {
            continue;
        }
        let sun_torque = input_torque / shares[i];
        let torques = [
            sun_torque * shares[0],
            sun_torque * shares[1],
            sun_torque * shares[2],
        ];
        // The sign this branch assumed has to be the sign it produces.
        let rolling = torques[0] * (speeds[0] - speeds[1]);
        if rolling != 0.0 && rolling.signum() != w {
            continue;
        }
        let input_power = (input_torque * input_speed).abs();
        let efficiency = if input_power > 0.0 {
            (torques[o] * speeds[o]).abs() / input_power
        } else {
            // No power in, so no loss to measure. The lossless kinematic answer
            // is the honest one rather than a division by zero.
            fixed_carrier_efficiency.powf(w).abs().min(1.0)
        };
        return Some(Power {
            speeds,
            torques,
            ratio: input_speed / speeds[o],
            efficiency,
            output,
            rolling_power_sign: w,
        });
    }
    None
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    fn teeth() -> Teeth {
        Teeth {
            sun: 24,
            planet: 18,
            ring: 60,
        }
    }

    /// Every arrangement of driven and held shaft, six in all.
    fn arrangements() -> Vec<Arrangement> {
        let all = [Member::Sun, Member::Carrier, Member::Ring];
        let mut out = Vec::new();
        for &input in &all {
            for &fixed in &all {
                if input != fixed {
                    out.push(Arrangement { input, fixed });
                }
            }
        }
        assert_eq!(out.len(), 6);
        out
    }

    /// **The three classical ratios, arrived at rather than written down.**
    ///
    /// Each falls out of the one Willis relation with a different shaft held, so
    /// agreeing with the textbook forms says the relation is right — and none of
    /// the three is a special case in the code.
    #[test]
    fn the_classical_ratios_come_out_of_one_relation() {
        let t = teeth();
        let (zs, zr) = (f64::from(t.sun), f64::from(t.ring));

        // Ring held, sun driving: the reduction is 1 + z_r/z_s.
        let p = power(
            t,
            Arrangement {
                input: Member::Sun,
                fixed: Member::Ring,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, Member::Carrier);
        assert!((p.ratio - (1.0 + zr / zs)).abs() < 1e-12, "{}", p.ratio);

        // Sun held, ring driving: 1 + z_s/z_r.
        let p = power(
            t,
            Arrangement {
                input: Member::Ring,
                fixed: Member::Sun,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, Member::Carrier);
        assert!((p.ratio - (1.0 + zs / zr)).abs() < 1e-12, "{}", p.ratio);

        // Carrier held: the sun and ring turn opposite ways, ratio −z_r/z_s.
        let p = power(
            t,
            Arrangement {
                input: Member::Sun,
                fixed: Member::Carrier,
            },
            1000.0,
            1.0,
            1.0,
        )
        .unwrap();
        assert_eq!(p.output, Member::Ring);
        assert!((p.ratio - (-zr / zs)).abs() < 1e-12, "{}", p.ratio);
        assert!(p.ratio < 0.0, "a fixed carrier reverses the output");
    }

    /// A set built from lossless meshes is lossless, in **all six** arrangements
    /// and exactly — so the loss term enters only through `η₀` and nothing else
    /// leaks.
    #[test]
    fn a_lossless_set_is_lossless_in_every_arrangement() {
        for a in arrangements() {
            let p = power(teeth(), a, 1500.0, 3.0, 1.0).unwrap();
            assert!(
                (p.efficiency - 1.0).abs() < 1e-12,
                "{a:?}: efficiency {}",
                p.efficiency
            );
            // ...and the power balance closes exactly.
            let out = p.torques[p.output.index()] * p.speeds[p.output.index()];
            assert!((out.abs() - (3.0 * 1500.0f64).abs()).abs() < 1e-9, "{a:?}");
        }
    }

    /// The three torques sum to zero in every arrangement — the set is in
    /// equilibrium, and the held shaft's torque is the reaction it carries.
    /// Getting this wrong is how a loss term ends up creating power.
    #[test]
    fn the_torques_are_in_equilibrium() {
        for a in arrangements() {
            for eta in [1.0, 0.98, 0.9] {
                let p = power(teeth(), a, 1500.0, 3.0, eta).unwrap();
                let sum: f64 = p.torques.iter().sum();
                assert!(
                    sum.abs() < 1e-9 * p.torques[0].abs().max(1.0),
                    "{a:?} eta={eta}: torques sum to {sum}"
                );
            }
        }
    }

    /// **Loss costs power, never makes it** — in every arrangement. The sign
    /// convention that is easy to get backwards, and the one a planetary punishes
    /// hardest, because `η₀` appears as a *power*.
    #[test]
    fn friction_never_pays() {
        for a in arrangements() {
            let ideal = power(teeth(), a, 1500.0, 3.0, 1.0).unwrap();
            let mut last = ideal.efficiency;
            for eta in [0.99, 0.97, 0.94, 0.9] {
                let p = power(teeth(), a, 1500.0, 3.0, eta).unwrap();
                assert!(
                    p.efficiency <= 1.0,
                    "{a:?} eta={eta}: efficiency {} exceeds one",
                    p.efficiency
                );
                assert!(
                    p.efficiency < last,
                    "{a:?} eta={eta}: {} did not fall below {last}",
                    p.efficiency
                );
                // The output speed is kinematic and loss cannot touch it.
                assert!((p.ratio - ideal.ratio).abs() < 1e-12);
                last = p.efficiency;
            }
        }
    }

    /// **With the carrier held, the answer must be exactly `η₀`.**
    ///
    /// A fixed-carrier set *is* two ordinary meshes in series — that is what
    /// `η₀` means — so this is the one arrangement whose efficiency is known in
    /// advance, and it comes out of the general algebra rather than being
    /// short-circuited. `|k/i₀| = η₀^w` exactly, with nothing left over.
    #[test]
    fn a_held_carrier_gives_exactly_the_fixed_carrier_efficiency() {
        for eta0 in [1.0, 0.99, 0.98, 0.9, 0.75] {
            for input in [Member::Sun, Member::Ring] {
                let p = power(
                    teeth(),
                    Arrangement {
                        input,
                        fixed: Member::Carrier,
                    },
                    1000.0,
                    4.0,
                    eta0,
                )
                .unwrap();
                assert!(
                    (p.efficiency - eta0).abs() < 1e-12,
                    "{input:?} eta0={eta0}: got {}",
                    p.efficiency
                );
            }
        }
    }

    /// **And with the ring held, the published closed form.**
    ///
    /// `η = (1 − i₀ η₀) / (1 − i₀)` for a sun-in, carrier-out set — derived
    /// independently of the code, which reaches it through the torque shares. Note
    /// what it says: the answer is **above** `η₀`, because only part of the power
    /// passes through the meshes at all. That is the result a mesh-by-mesh
    /// calculation gets wrong in the optimistic direction for some arrangements
    /// and the pessimistic direction for others.
    #[test]
    fn a_held_ring_matches_the_published_closed_form() {
        let t = teeth();
        let i0 = basic_ratio(t);
        for eta0 in [1.0, 0.99, 0.98, 0.95] {
            let p = power(
                t,
                Arrangement {
                    input: Member::Sun,
                    fixed: Member::Ring,
                },
                1000.0,
                4.0,
                eta0,
            )
            .unwrap();
            let want = (1.0 - i0 * eta0) / (1.0 - i0);
            assert!(
                (p.efficiency - want).abs() < 1e-12,
                "eta0={eta0}: {} vs {want}",
                p.efficiency
            );
            assert!(
                p.efficiency >= eta0,
                "eta0={eta0}: a carrier-output set should beat its meshes"
            );
        }
    }

    /// **A coupled planetary is not as efficient as its meshes.**
    ///
    /// The result worth surfacing, and the reason §4.5.2 refuses a mesh-by-mesh
    /// calculation: the meshes slide at their speeds relative to the *carrier*, so
    /// as the ratio grows the recirculating power grows with it and the overall
    /// efficiency falls well below `η₀` — from meshes that never change.
    #[test]
    fn a_high_ratio_set_loses_more_than_its_meshes_do() {
        let eta0 = 0.98;
        let mut previous = f64::INFINITY;
        // Growing the sun against a fixed ring raises the carrier-output ratio.
        for sun in [60u32, 40, 30, 24, 20, 18] {
            let t = Teeth {
                sun,
                planet: 18,
                ring: 60,
            };
            let p = power(
                t,
                Arrangement {
                    input: Member::Carrier,
                    fixed: Member::Ring,
                },
                1000.0,
                5.0,
                eta0,
            )
            .unwrap();
            assert!(p.efficiency < 1.0);
            assert!(
                p.efficiency < previous,
                "z_sun={sun}: efficiency {} did not fall below {previous}",
                p.efficiency
            );
            previous = p.efficiency;
        }
        // Even so, a carrier-output set stays *above* its mesh product: it is the
        // sun-or-ring-output arrangements that fall below. Stated as a comparison
        // because the direction is the whole point.
        let t = teeth();
        let carrier_out = power(
            t,
            Arrangement {
                input: Member::Sun,
                fixed: Member::Ring,
            },
            1000.0,
            5.0,
            eta0,
        )
        .unwrap();
        let ring_out = power(
            t,
            Arrangement {
                input: Member::Sun,
                fixed: Member::Carrier,
            },
            1000.0,
            5.0,
            eta0,
        )
        .unwrap();
        assert!(
            carrier_out.efficiency > ring_out.efficiency,
            "{} vs {}",
            carrier_out.efficiency,
            ring_out.efficiency
        );
    }

    /// The held shaft never turns, and the shaft that is neither driven nor held
    /// is the one reported as the output.
    #[test]
    fn the_held_shaft_is_still_and_the_third_is_the_output() {
        for a in arrangements() {
            let p = power(teeth(), a, 1234.0, 7.0, 0.98).unwrap();
            assert_eq!(p.speeds[a.fixed.index()], 0.0);
            assert_eq!(p.speeds[a.input.index()], 1234.0);
            assert_ne!(p.output, a.input);
            assert_ne!(p.output, a.fixed);
        }
    }

    /// Driving and holding the same shaft is not an arrangement.
    #[test]
    fn a_shaft_cannot_be_both_driven_and_held() {
        for m in [Member::Sun, Member::Carrier, Member::Ring] {
            assert!(power(
                teeth(),
                Arrangement { input: m, fixed: m },
                1000.0,
                1.0,
                0.98
            )
            .is_none());
        }
    }

    fn rack() -> Rack {
        Rack::new(1.0, 20.0, 0.0)
    }

    /// A set with three planets, nothing shifted but the planet, and a planet
    /// small enough that clearance never bites.
    fn set_of(sun: u32, planet: u32, ring: u32) -> Set {
        Set {
            rack: rack(),
            teeth: Teeth { sun, planet, ring },
            planets: 3,
            sun_shift: 0.0,
            ring_shift: 0.0,
            planet_tip_diameter: 0.0,
        }
    }

    fn solve_at(sun: u32, planet: u32, ring: u32) -> Option<Layout> {
        solve(&set_of(sun, planet, ring))
    }

    /// **The check the whole construction has to pass.** `z_r = z_s + 2 z_p` puts
    /// the planet exactly halfway between sun and ring, so both centre distances
    /// are already their reference values and the required shift is zero — not
    /// nearly zero.
    #[test]
    fn the_ideal_ring_needs_no_planet_shift() {
        for (sun, planet) in [(17u32, 17u32), (20, 25), (13, 31), (40, 15)] {
            let ring = Teeth::ideal_ring(sun, planet);
            let l = solve_at(sun, planet, ring).unwrap();
            assert!(
                l.planet_shift.abs() < 1e-12,
                "z={sun}/{planet}/{ring}: shift {} should be zero",
                l.planet_shift
            );
            // ...and the common centre distance is then the reference one.
            let a_ref = rack().mt * f64::from(sun + planet) / 2.0;
            assert!((l.centre_distance - a_ref).abs() < 1e-12);
        }
    }

    /// The solve closes: the two centre distances agree to machine precision, at
    /// every ring count that admits a solution at all.
    #[test]
    fn the_two_centre_distances_agree() {
        let mut solved = 0;
        for ring in 40..=70 {
            if let Some(l) = solve_at(17, 17, ring) {
                assert!(l.residual < 1e-12, "z_r={ring}: residual {} mm", l.residual);
                solved += 1;
            }
        }
        assert!(solved >= 5, "only {solved} ring counts solved");
    }

    /// **The analytic derivative, against central differences.**
    ///
    /// `dg/dx_p` is what makes Newton safe here rather than merely fast, so it is
    /// checked against something that shares none of its algebra.
    #[test]
    fn the_derivative_matches_central_differences() {
        let r = rack();
        for ring in [49u32, 51, 52, 53] {
            let teeth = Teeth {
                sun: 17,
                planet: 17,
                ring,
            };
            let (sum_ext, sum_int) = teeth.sums();
            // Each mesh's centre distance and operating angle at planet shift x.
            // The sun and ring are unshifted here, so each mesh's shift sum *is*
            // x — with the internal one entering negatively, as its sign says.
            let at = |sum_z: f64, sum_x: f64| {
                operating_geometry(r.mt, r.alpha_t, r.alpha_n, sum_z, sum_x).unwrap()
            };
            let g = |x: f64| at(sum_ext, x).2 - at(sum_int, x).2;
            let dg = |x: f64| {
                let (aw_e, _, a_e) = at(sum_ext, x);
                let (aw_i, _, a_i) = at(sum_int, x);
                d_centre_distance(a_e, aw_e, r.alpha_n, sum_ext)
                    - d_centre_distance(a_i, aw_i, r.alpha_n, sum_int)
            };
            for x in [-0.3, -0.1, 0.0, 0.2, 0.4] {
                let h = 1e-6;
                let numeric = (g(x + h) - g(x - h)) / (2.0 * h);
                let analytic = dg(x);
                assert!(
                    (analytic - numeric).abs() < 1e-6 * analytic.abs().max(1.0),
                    "z_r={ring} x={x}: analytic {analytic} vs numeric {numeric}"
                );
            }
        }
    }

    /// **The required planet shift is strictly increasing in `z_ring`** — which
    /// is what makes the ring search provably complete rather than a sample.
    #[test]
    fn the_required_shift_is_monotone_in_ring_teeth() {
        let mut last = f64::NEG_INFINITY;
        let mut seen = 0;
        for ring in 40..=70 {
            if let Some(l) = solve_at(17, 17, ring) {
                assert!(
                    l.planet_shift > last,
                    "z_r={ring}: {} is not above {last}",
                    l.planet_shift
                );
                last = l.planet_shift;
                seen += 1;
            }
        }
        assert!(seen >= 5);
    }

    /// **Most ring counts are genuinely impossible, and the bracket is what says
    /// so.** For a 17-tooth sun and 17-tooth planets only 48…54 admits any
    /// planet shift at all; outside that the involute domain is empty, which is a
    /// different statement from "the solver did not converge".
    #[test]
    fn only_a_contiguous_run_of_ring_counts_is_admissible() {
        let admissible: Vec<u32> = (40..=70)
            .filter(|&z| solve_at(17, 17, z).is_some())
            .collect();
        assert_eq!(admissible, (48..=54).collect::<Vec<u32>>());
    }

    /// **And the run has no holes in it, on any set.**
    ///
    /// Monotonicity says the admissible counts are contiguous, so a gap is a bug
    /// by construction — which is the only reason this is checkable without
    /// knowing the answers. It found one: a 24/16 set with four planets lost
    /// `z_ring = 57`, because that bracket's endpoint rounded a hair outside the
    /// involute domain it was meant to sit exactly on. The test above only
    /// sweeps 17/17, where the rounding happens to fall the other way — a
    /// one-configuration check for a property that is about all of them.
    #[test]
    fn the_admissible_run_has_no_holes() {
        for (sun, planet) in [
            (17u32, 17u32),
            (18, 18),
            (24, 16),
            (13, 31),
            (40, 15),
            (20, 25),
            (9, 21),
            (31, 13),
        ] {
            let ideal = Teeth::ideal_ring(sun, planet);
            let found: Vec<u32> = (planet + 1..=2 * ideal)
                .filter(|&z| solve_at(sun, planet, z).is_some())
                .collect();
            assert!(!found.is_empty(), "z={sun}/{planet}: nothing admissible");
            let (first, last) = (found[0], *found.last().unwrap());
            assert_eq!(
                found,
                (first..=last).collect::<Vec<u32>>(),
                "z={sun}/{planet}: the admissible run has a hole in it"
            );
            // The ideal ring must be in it — it is the one that needs no shift.
            assert!(found.contains(&ideal), "z={sun}/{planet}: {ideal} missing");
        }
    }

    /// The worked example of DESIGN.md §4.8, to the digits recorded there.
    #[test]
    fn the_worked_example_reproduces() {
        for (ring, want) in [
            (48u32, -0.6684),
            (49, -0.4807),
            (51, 0.0),
            (52, 0.2480),
            (54, 0.6862),
        ] {
            let l = solve_at(17, 17, ring).unwrap();
            assert!(
                (l.planet_shift - want).abs() < 5e-5,
                "z_r={ring}: {} vs {want}",
                l.planet_shift
            );
        }
    }

    /// Equal spacing and simultaneous meshing are arithmetic on the tooth
    /// counts, and the example is a case where the first holds and the second
    /// does not — three planets divide `z_s + z_r` but not `z_s = 17`.
    #[test]
    fn the_layout_checks_read_the_tooth_counts() {
        let l = solve_at(17, 17, 52).unwrap();
        assert!(l.equal_spacing, "(17+52)/3 = 23");
        assert!(!l.simultaneous_meshing, "3 does not divide 17");

        let l = solve_at(17, 17, 51).unwrap();
        assert!(!l.equal_spacing, "(17+51)/3 is not whole");

        // A set where both hold: 3 divides sun, ring and their sum.
        let l = solve_at(18, 18, 54).unwrap();
        assert!(l.equal_spacing && l.simultaneous_meshing);
    }

    /// Planet clearance is a gap, so it goes negative when the planets overlap —
    /// reported rather than refused, because which of the tooth counts, the
    /// planet count and the module to give up is the designer's call.
    #[test]
    fn planet_clearance_is_a_gap_and_can_be_negative() {
        let sized = |planets: u32| Set {
            planets,
            planet_tip_diameter: 19.0,
            ..set_of(17, 17, 52)
        };

        let roomy = solve(&sized(3)).unwrap().planet_clearance.unwrap();
        assert!(roomy > 0.0, "three 19 mm planets fit: {roomy}");

        let crowded = solve(&sized(8)).unwrap().planet_clearance.unwrap();
        assert!(crowded < 0.0, "eight of them cannot: {crowded}");

        // One planet has no neighbour to clear.
        assert!(solve(&sized(1)).unwrap().planet_clearance.is_none());
    }

    /// The ring search returns exactly the counts whose required shift lands in
    /// the range asked for, and stops rather than sampling past it.
    #[test]
    fn the_ring_search_returns_the_run_inside_the_shift_range() {
        let found = ring_candidates(
            &Set {
                planet_tip_diameter: 19.0,
                ..set_of(17, 17, 0)
            },
            (0.0, 0.5),
            80,
        );
        let counts: Vec<u32> = found.iter().map(|(z, _)| *z).collect();
        // From the worked table: 51 needs exactly 0, 52 needs +0.2480 and 53
        // +0.4831, all inside the range; 54 needs +0.6862 and is out, so the
        // sweep stops there rather than sampling on.
        assert_eq!(counts, vec![51, 52, 53], "got {counts:?}");
        for (_, l) in &found {
            assert!((0.0..=0.5).contains(&l.planet_shift));
            assert!(l.residual < 1e-12);
        }

        // Which of them a designer wants is a *second* question, and the one
        // §4.8's worked example answers: only 52 also spaces three planets
        // evenly. Keeping the two apart is deliberate — the search reports what
        // is possible, and the layout checks say what is desirable.
        let even: Vec<u32> = found
            .iter()
            .filter(|(_, l)| l.equal_spacing)
            .map(|(z, _)| *z)
            .collect();
        assert_eq!(even, vec![52], "the worked example selects 52");
    }

    /// Shifting the sun and the ring is not the same as shifting neither: the
    /// planet has to take up the difference, and only the difference reaches it.
    #[test]
    fn sun_and_ring_shifts_move_the_planet_the_way_they_should() {
        let base = solve_at(17, 17, 52).unwrap().planet_shift;

        // A more positive sun already widens the external mesh, so the planet
        // needs less of its own shift...
        let sunny = solve(&Set {
            sun_shift: 0.2,
            ..set_of(17, 17, 52)
        })
        .unwrap()
        .planet_shift;
        assert!(sunny < base, "{sunny} should be below {base}");

        // ...and a more positive ring widens its space, which the planet follows.
        let ringy = solve(&Set {
            ring_shift: 0.2,
            ..set_of(17, 17, 52)
        })
        .unwrap()
        .planet_shift;
        assert!(ringy > base, "{ringy} should be above {base}");
    }
}
